// ── Naryad #457 (P1, server/security): the cron executor under the
//    serve-route guard + the inversion of the execution-context default ──
//
// Contract (issue #676, audit 25.09 finding 3.4 Medium):
// 1. THE HOLE: `execute_tick_call` (the cron/webhook tick executor) ran its
//    spawn_blocking WITHOUT `ServeRouteExecGuard` — the only unguarded
//    launch point left (TW routes, VM routes and MCP tools were already
//    guarded since №253). A cron tick could `env()` anything (the №259
//    gate never applied) and `exec()` was gated by the PROCESS flag
//    `METALOGOS_ALLOW_EXEC` instead of the server gate. Pinned here:
//    a tick `env()` refuses with ENV_NOT_PERMITTED even for a SET
//    variable (the gate is before the read — no existence oracle), and a
//    tick `exec()` refuses while the PROCESS flag is set, naming the
//    SERVER flag `METALOGOS_SERVE_ALLOW_EXEC`.
// 2. THE INVERSION: in serve the DEFAULT context is strict — every
//    unmarked thread is `ServeRoute` (`set_process_mode(ProcessMode::Serve)`
//    in `cmd_serve`), EXCEPT the explicit top-level registration zone
//    (`TopLevelRegistrationGuard` keeps the `Process` context for the
//    declaration-registration phase in `run_server`). A forgotten mark at
//    a NEW launch point now costs a spurious DENIAL (loud), not silent
//    process rights. The machinery is pinned directly (unit) and the
//    routes are pinned as regression-free under the inverted default
//    (TW probe; the VM denial parity is the №259 suite's contract).
// 3. THE SEMANTICS ARE UNCHANGED: the serve allowlist/escape hatch still
//    reads dynamically per call — a tick with
//    `METALOGOS_ENV_ALLOWLIST` naming the variable reads it (the «замена,
//    не AND» posture of №253/№259 is preserved verbatim).
//
// Mechanism: the SAME SSOT thread-local + guards (no second flag-hack);
// the tick surfaces are the public test_tick_call executor (the №426
// posture — the exact executor the scheduler uses).

#![cfg(feature = "server")]
// Служебное исключение (громко, по правилам репо — лекало naryad_259):
// serve-тесты обязаны держать ENV_LOCK через .await — env-переменные
// процесса читаются серверными потоками во время запроса.
#![allow(clippy::await_holding_lock)]

use metalogos::builtins::{
    current_exec_context, exec_gate, set_process_mode, ExecContext, ProcessMode,
    ServeRouteExecGuard, TopLevelRegistrationGuard,
};
use metalogos::interpreter::Value;
use metalogos::server::ServeBackend;
use std::sync::Mutex;

// Лекало naryad_259/naryad_426: env-переменные процесса глобальны —
// сериализуем тесты одного файла одним poison-tolerant мьютексом.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Drop-сторож переменной окружения: снимает переменную при выходе из
/// теста (включая панику), чтобы негативный кейс не подсадил позитивный.
struct EnvVar(&'static str);
impl EnvVar {
    fn set(value: &str) -> Self {
        std::env::set_var(Self::NAME, value);
        EnvVar(Self::NAME)
    }
}
impl EnvVar {
    const NAME: &'static str = "N457_TICK_PROBE";
}
impl Drop for EnvVar {
    fn drop(&mut self) {
        std::env::remove_var(self.0);
    }
}

/// Drop-сторож режима процесса: тесты, включающие строгий serve-дефолт,
/// обязаны вернуть Process даже при панике — иначе параллельный тест
/// соседнего файла увидит ServeRoute там, где ожидает Process.
struct RestoreProcessMode;
impl Drop for RestoreProcessMode {
    fn drop(&mut self) {
        set_process_mode(ProcessMode::Process);
    }
}

const TICK_APP: &str = r#"
pattern TickEnv(_payload: String) -> String {
  let v = env("N457_TICK_PROBE")
  if v == "n457-tick-ok" {
    return "TICK_ENV_OK"
  }
  return "TICK_ENV_MISS"
}
pattern TickExec(_payload: String) -> String {
  let out = exec("echo n457-ok")
  return "ran:" + to_string(len(out))
}
mlogserver { port: 0 }
"#;

const ROUTE_APP: &str = r#"
mlogserver {
  port: 0
  route "/envread" method=GET {
    let v = env("N457_TICK_PROBE")
    if v == "n457-tick-ok" {
      return respond("200", "ROUTE_ENV_OK")
    } else {
      return respond("200", "ROUTE_ENV_MISS")
    }
  }
}
"#;

async fn tick_env_call() -> Result<Value, String> {
    metalogos::server::test_tick_call(
        TICK_APP,
        "TickEnv",
        vec![Value::String("payload".to_string())],
    )
    .await
}

async fn tick_exec_call() -> Result<Value, String> {
    metalogos::server::test_tick_call(
        TICK_APP,
        "TickExec",
        vec![Value::String("payload".to_string())],
    )
    .await
}

async fn start_route_server(
    backend: ServeBackend,
) -> (
    u16,
    tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
) {
    let base_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    metalogos::server::run_test_server_with_backend_in_dir(ROUTE_APP, backend, base_dir)
        .await
        .expect("test server should start")
}

async fn http_get(port: u16, path: &str) -> (u16, String) {
    let url = format!("http://127.0.0.1:{}{}", port, path);
    let resp = reqwest::get(&url).await.expect("GET should succeed");
    let status = resp.status().as_u16();
    let body = resp.text().await.expect("body should be readable");
    (status, body)
}

// ── 1. Тик под guard: env() отказан даже для УСТАНОВЛЕННОЙ переменной ──

#[tokio::test]
async fn n457_cron_tick_env_denied_even_for_a_set_variable() {
    let _env = lock_env();
    let _probe = EnvVar::set("n457-tick-ok");

    let err = tick_env_call()
        .await
        .expect_err("env() in a cron tick must be denied (the №259 gate now applies)");
    assert!(
        err.contains("ENV_NOT_PERMITTED"),
        "the refusal must carry the stable №259 code, got: {}",
        err
    );
    assert!(
        err.contains("METALOGOS_SERVE_ALLOW_ENV"),
        "the refusal must name the server escape hatch, got: {}",
        err
    );
}

// ── 2. Тик под guard: exec() определяется СЕРВЕРНЫМ флагом, не процессным ──

#[tokio::test]
async fn n457_cron_tick_exec_gated_by_the_server_flag_not_the_process_flag() {
    let _env = lock_env();
    let _probe = EnvVar::set("n457-tick-ok"); // keep the probe var consistent
    let _proc_flag = EnvVar("METALOGOS_ALLOW_EXEC");
    std::env::set_var("METALOGOS_ALLOW_EXEC", "1");

    let err = tick_exec_call()
        .await
        .expect_err("the process flag must NOT open exec() in a cron tick");
    assert!(
        err.contains("EXEC_NOT_PERMITTED"),
        "the refusal must carry the stable №253 code, got: {}",
        err
    );
    assert!(
        err.contains("METALOGOS_SERVE_ALLOW_EXEC"),
        "the refusal must name the SERVER flag (the replacement posture), got: {}",
        err
    );
}

// ── 3. Семантика №259 не изменилась: allowlist читается в тике ─────────

#[tokio::test]
async fn n457_cron_tick_env_allowed_via_the_serve_allowlist() {
    let _env = lock_env();
    let _probe = EnvVar::set("n457-tick-ok");
    let _allow = EnvVar("METALOGOS_ENV_ALLOWLIST");
    std::env::set_var("METALOGOS_ENV_ALLOWLIST", "N457_TICK_PROBE");

    let v = tick_env_call()
        .await
        .expect("the allowlist must allow the tick env() read (replacement semantics intact)");
    assert!(
        matches!(&v, Value::String(s) if s == "TICK_ENV_OK"),
        "the tick must read the allowlisted variable, got: {:?}",
        v
    );
}

// ── 4. Механика инверсии: строгий дефолт + явная top-level зона ────────

#[test]
fn n457_strict_default_inversion_machinery() {
    let _env = lock_env();
    let _restore = RestoreProcessMode;

    // Process mode (the pre-№457 posture): unmarked → Process.
    set_process_mode(ProcessMode::Process);
    assert_eq!(current_exec_context(), ExecContext::Process);

    // Serve mode: an UNMARKED thread is ServeRoute — the strict default.
    set_process_mode(ProcessMode::Serve);
    assert_eq!(current_exec_context(), ExecContext::ServeRoute);

    // The explicit top-level registration zone keeps the Process context.
    {
        let _toplevel = TopLevelRegistrationGuard::new();
        assert_eq!(current_exec_context(), ExecContext::Process);

        // The explicit route guard still wins inside the top-level zone
        // (the resolution order: the explicit mark first).
        let _route = ServeRouteExecGuard::new();
        assert_eq!(current_exec_context(), ExecContext::ServeRoute);
    }
    assert_eq!(current_exec_context(), ExecContext::ServeRoute);

    // The SSOT gates agree with the machinery: exec_gate in the serve
    // mode on an unmarked thread refuses with the SERVER code.
    let err = exec_gate(current_exec_context())
        .expect_err("exec_gate must refuse the serve-route context");
    assert!(err.contains("METALOGOS_SERVE_ALLOW_EXEC"), "got: {}", err);
}

// ── 5. Роуты под инвертированным дефолтом: регрессий нет ──────────────

#[tokio::test]
async fn n457_routes_unregressed_under_the_inverted_default_tw() {
    let _env = lock_env();
    let _probe = EnvVar::set("n457-tick-ok");
    let _restore = RestoreProcessMode;
    set_process_mode(ProcessMode::Serve);

    let (port, _handle) = start_route_server(ServeBackend::Interpreter).await;

    // Without flags: denied loudly (the route guard posture — unchanged).
    let (status, body) = http_get(port, "/envread").await;
    assert_eq!(
        status, 500,
        "the route env() denial must stay loud under the strict default: {}",
        body
    );
    assert!(body.contains("ENV_NOT_PERMITTED"), "got: {}", body);

    // The escape hatch still works — dynamic, per call (№259 semantics).
    let _allow = EnvVar("METALOGOS_SERVE_ALLOW_ENV");
    std::env::set_var("METALOGOS_SERVE_ALLOW_ENV", "1");
    let (status, body) = http_get(port, "/envread").await;
    assert_eq!(status, 200, "got: {}", body);
    assert!(
        body.contains("ROUTE_ENV_OK"),
        "the escape hatch must keep working under the strict default: {}",
        body
    );
}
