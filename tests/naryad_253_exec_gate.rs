// ── НАРЯД №253: exec-гейт serve-контекста (Вариант А владельца) ────
//
// Контракт (issue #256, docs/naryads-252-257-security-bugfix.md §253):
// 1. serve + нет флагов → громкая ошибка с кодом EXEC_NOT_PERMITTED;
// 2. serve + только METALOGOS_ALLOW_EXEC=1 → всё ещё отказ
//    (суть Варианта А: «замена», не «AND» — процесс-флаг не течёт в роуты);
// 3. serve + METALOGOS_SERVE_ALLOW_EXEC=1 → работает, запись в аудите есть;
// 4. mlog run с флагом процесса → работает как раньше (№97 без изменений);
// 5. замкнутость exec_restricted: не экспонируется программам как билтин
//    (html_render/pdf зовут её с фиксированным бинарником и аргументами из
//    кода — см. tests/n88_html_render_contract.rs и src/builtins/pdf.rs).
//
// Механизм: ServeRouteExecGuard (thread-local RAII) ставится внутри
// spawn_blocking-замыкания обоих роут-путей (TW и VM) — проверяются ОБА бэкенда.

#![cfg(feature = "server")]

use metalogos::builtins::{exec_gate, ExecContext};
use metalogos::server::ServeBackend;
use std::sync::Mutex;

// Наряд №206: env-переменные процесса глобальны — сериализуем тесты,
// которые их трогают (#[serial] здесь не используем: async-тесты, поэтому
// обычный мьютекс; ожидание под замком не держит await-конфликтов, т.к.
// каждый тест работает в собственном потоке cargo test).
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

const EXEC_ROUTE_SOURCE: &str = r#"
mlogserver {
  port: 8093
  route "/sh" method=GET {
    let out = exec("echo serve-exec-ok")
    respond("200", out)
  }
}
"#;

const PROCESS_EXEC_PROGRAM: &str = r#"
pattern Sh(input: String) -> String {
  return exec("echo proc-ok")
}
entity start: String = ""
flow Main { input: String = start -> Sh -> output }
"#;

async fn start_server(
    backend: ServeBackend,
) -> (
    u16,
    tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
) {
    let base_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    metalogos::server::run_test_server_with_backend_in_dir(EXEC_ROUTE_SOURCE, backend, base_dir)
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

// ── 1. TW: serve + нет флагов → EXEC_NOT_PERMITTED ──────────────────

#[tokio::test]
async fn tw_serve_route_exec_denied_without_flags() {
    let _env = lock_env();
    std::env::remove_var("METALOGOS_ALLOW_EXEC");
    std::env::remove_var("METALOGOS_SERVE_ALLOW_EXEC");

    let (port, _handle) = start_server(ServeBackend::Interpreter).await;
    let (status, body) = http_get(port, "/sh").await;

    assert_eq!(status, 500, "exec без флагов должен быть отказан: {}", body);
    assert!(
        body.contains("EXEC_NOT_PERMITTED"),
        "ошибка должна нести стабильный код EXEC_NOT_PERMITTED, got: {}",
        body
    );
    assert!(
        body.contains("METALOGOS_SERVE_ALLOW_EXEC=1"),
        "текст должен называть точный флаг контекста, got: {}",
        body
    );
}

// ── 2. TW: суть Варианта А — процесс-флаг НЕ применяется к роутам ──

#[tokio::test]
async fn tw_serve_route_exec_process_flag_does_not_apply_to_routes() {
    let _env = lock_env();
    // Только процесс-флаг; serve-флага нет. До №253 этот сценарий давал роутам
    // полный sh -c — теперь должен быть отказ (семантика «замена», не «AND»).
    std::env::set_var("METALOGOS_ALLOW_EXEC", "1");
    std::env::remove_var("METALOGOS_SERVE_ALLOW_EXEC");

    let (port, _handle) = start_server(ServeBackend::Interpreter).await;
    let (status, body) = http_get(port, "/sh").await;

    assert_eq!(
        status, 500,
        "METALOGOS_ALLOW_EXEC=1 не должен открывать exec в телах роутов: {}",
        body
    );
    assert!(body.contains("EXEC_NOT_PERMITTED"), "got: {}", body);
}

// ── 3. TW: serve-флаг открывает exec + запись в аудите ─────────────

#[tokio::test]
async fn tw_serve_route_exec_enabled_with_serve_flag_and_audited() {
    let _env = lock_env();
    // Процесс-флага НЕТ — работает только serve-флаг (замена, не AND).
    std::env::remove_var("METALOGOS_ALLOW_EXEC");
    std::env::set_var("METALOGOS_SERVE_ALLOW_EXEC", "1");

    let audit_path = std::env::temp_dir().join("n253_audit_tw.log");
    let _ = std::fs::remove_file(&audit_path);
    std::env::set_var("METALOGOS_AUDIT_LOG_PATH", &audit_path);

    let (port, _handle) = start_server(ServeBackend::Interpreter).await;
    let (status, body) = http_get(port, "/sh").await;

    assert_eq!(status, 200, "serve-флаг должен открыть exec: {}", body);
    assert!(
        body.contains("serve-exec-ok"),
        "stdout exec должен доехать до ответа, got: {}",
        body
    );

    let audit = std::fs::read_to_string(&audit_path).unwrap_or_default();
    assert!(
        audit.contains('\t') && audit.contains("exec") && audit.contains("echo serve-exec-ok"),
        "в аудите должна быть запись о exec, got: {:?}",
        audit
    );

    std::env::remove_var("METALOGOS_SERVE_ALLOW_EXEC");
    std::env::remove_var("METALOGOS_AUDIT_LOG_PATH");
    let _ = std::fs::remove_file(&audit_path);
}

// ── 4–5. VM-путь: паритет с TW (guard в execute_route_body_vm) ─────

#[tokio::test]
async fn vm_serve_route_exec_denied_without_flags() {
    let _env = lock_env();
    std::env::remove_var("METALOGOS_ALLOW_EXEC");
    std::env::remove_var("METALOGOS_SERVE_ALLOW_EXEC");

    let (port, _handle) = start_server(ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/sh").await;

    assert_eq!(status, 500, "VM-путь: exec без флагов отказан: {}", body);
    assert!(
        body.contains("EXEC_NOT_PERMITTED"),
        "VM-путь: код EXEC_NOT_PERMITTED в ошибке, got: {}",
        body
    );
}

#[tokio::test]
async fn vm_serve_route_exec_enabled_with_serve_flag() {
    let _env = lock_env();
    std::env::remove_var("METALOGOS_ALLOW_EXEC");
    std::env::set_var("METALOGOS_SERVE_ALLOW_EXEC", "1");

    let (port, _handle) = start_server(ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/sh").await;

    assert_eq!(status, 200, "VM-путь: serve-флаг открывает exec: {}", body);
    assert!(body.contains("serve-exec-ok"), "got: {}", body);

    std::env::remove_var("METALOGOS_SERVE_ALLOW_EXEC");
}

// ── 6. Процесс-контекст (mlog run) — №97 без изменений ─────────────

#[test]
fn process_context_uses_process_flag_as_before() {
    let _env = lock_env();

    // с флагом процесса — работает как раньше (serve-флаг не нужен)
    std::env::set_var("METALOGOS_ALLOW_EXEC", "1");
    std::env::remove_var("METALOGOS_SERVE_ALLOW_EXEC");
    let out = metalogos::run_program(PROCESS_EXEC_PROGRAM);
    assert!(
        matches!(&out, Ok(Some(s)) if s.contains("proc-ok")),
        "exec в процесс-контексте с METALOGOS_ALLOW_EXEC=1 должен работать, got: {:?}",
        out
    );

    // без флага — громкий отказ с кодом (проза №97 теперь несёт EXEC_NOT_PERMITTED)
    std::env::remove_var("METALOGOS_ALLOW_EXEC");
    let err = metalogos::run_program(PROCESS_EXEC_PROGRAM).unwrap_err();
    assert!(
        err.contains("EXEC_NOT_PERMITTED"),
        "ошибка процесс-контекста должна нести EXEC_NOT_PERMITTED, got: {}",
        err
    );
}

// ── 7. SSOT exec_gate: семантика «замена» напрямую ─────────────────

#[test]
fn exec_gate_replacement_semantics() {
    let _env = lock_env();

    // ServeRoute игнорирует процесс-флаг даже при METALOGOS_ALLOW_EXEC=1
    std::env::set_var("METALOGOS_ALLOW_EXEC", "1");
    std::env::remove_var("METALOGOS_SERVE_ALLOW_EXEC");
    assert!(exec_gate(ExecContext::ServeRoute).is_err());
    assert!(exec_gate(ExecContext::Process).is_ok());

    // serve-флаг открывает только ServeRoute-контекст
    std::env::set_var("METALOGOS_SERVE_ALLOW_EXEC", "1");
    assert!(exec_gate(ExecContext::ServeRoute).is_ok());
    // процесс-контекст при этом не обязан открываться serve-флагом
    std::env::remove_var("METALOGOS_ALLOW_EXEC");
    assert!(exec_gate(ExecContext::Process).is_err());

    std::env::remove_var("METALOGOS_SERVE_ALLOW_EXEC");
}

// ── 8. Замкнутость exec_restricted ─────────────────────────────────

#[test]
fn exec_restricted_is_not_exposed_to_programs() {
    let names = metalogos::builtins::builtin_names();
    assert!(
        !names.iter().any(|n| n.contains("exec_restricted")),
        "exec_restricted не должен быть доступен mlog-программам, registry: {:?}",
        names
    );
    // exec/exec_argv экспонированы, но за гейтом (проверено выше)
    assert!(names.contains(&"exec".to_string()));
    assert!(names.contains(&"exec_argv".to_string()));
}
