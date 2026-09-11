// ── Наряд №268: MCP stdio-клиент — mcp_call / mcp_list_tools (ADR-0132) ──
//
// Контракт-тесты против локального fixture-сервера
// (tests/fixtures/mcp_echo_server.py, конвенция p71/p76):
//
//   1. mlog-контракты "mcp_list_tools → Struct → json_get" и
//      "mcp_call → String" зелёные на ОБОИХ бэкендах (TW и VM).
//   2. Ошибки громкие с фазой и кодом: tool-not-found (-32602),
//      isError=true, JSON-RPC error (-32603), timeout, garbage в stdout,
//      обрыв потока (крэш сервера).
//   3. Гейты: exec-gate №253-А (process + serve-роут, ОБА бэкенда),
//      allowlist METALOGOS_MCP_ALLOWLIST (unset/пустая/точное совпадение).
//   4. Taint-интеграция (решение владельца: reuse UserInput):
//      reflex_train(mcp_call(...)) → UNTRUSTED_TRAINING_DATA;
//      прямой sink respond(mcp_call(...)) отклоняется статикой.
//   5. Аудит: каждый разрешённый spawn пишет `\tmcp_list_tools\t` /
//      `\tmcp_call\t` в METALOGOS_AUDIT_LOG_PATH.
//
// Аудит-инвариант: вывод внешнего инструмента — недоверенные данные
// с первого дня; отравление модели через MCP-инструмент отвергается
// статикой (как для json_body), сироты-процессы невозможны (Drop-гарант).

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

const FIXTURE: &str = "tests/fixtures/mcp_echo_server.py";

/// mlog-контракт: tools/list → List[Struct] → get → json_get.
const LIST_CONTRACT: &str = r#"
pattern Tools(x: String) -> String {
  let tools = mcp_list_tools("python3", ["tests/fixtures/mcp_echo_server.py"])
  let first = get(tools, 0)
  return json_get(first, "name") + " | " + json_get(first, "description")
}
flow Main {
  input: String = "list"
  -> Tools
  -> output
}
"#;

/// mlog-контракт: tools/call echo → text-блоки → String.
const CALL_CONTRACT: &str = r#"
pattern CallIt(x: String) -> String {
  let out = mcp_call("python3", ["tests/fixtures/mcp_echo_server.py"], "echo", "{\"text\":\"hi\"}")
  return out
}
flow Main {
  input: String = "call"
  -> CallIt
  -> output
}
"#;

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program(source)
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let program = metalogos::compiler::Compiler::new().compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

/// Включить exec-гейт на время теста; снять на выходе.
struct AllowExec;
impl AllowExec {
    fn set() -> Self {
        std::env::set_var("METALOGOS_ALLOW_EXEC", "1");
        AllowExec
    }
}
impl Drop for AllowExec {
    fn drop(&mut self) {
        std::env::remove_var("METALOGOS_ALLOW_EXEC");
    }
}

fn unset_env(keys: &[&str]) {
    for k in keys {
        std::env::remove_var(k);
    }
}

// ── 1. mlog-контракты на ОБОИХ бэкендах ──────────────────────────────

#[test]
fn c1_list_tools_contract_tw() {
    let _env = lock_env();
    let _allow = AllowExec::set();
    let out = run_tw(LIST_CONTRACT).expect("TW must run the list contract");
    let out = out.expect("flow output expected");
    assert!(
        out.contains("echo | Returns its input text back"),
        "TW output must carry tool name + description, got: {}",
        out
    );
}

#[test]
fn c1_list_tools_contract_vm() {
    let _env = lock_env();
    let _allow = AllowExec::set();
    let out = run_vm(LIST_CONTRACT).expect("VM must run the list contract");
    let out = out.expect("flow output expected");
    assert!(
        out.contains("echo | Returns its input text back"),
        "VM output must match TW (parity), got: {}",
        out
    );
}

#[test]
fn c2_call_echo_contract_tw() {
    let _env = lock_env();
    let _allow = AllowExec::set();
    let out = run_tw(CALL_CONTRACT).expect("TW must run the call contract");
    assert_eq!(
        out.expect("flow output expected").trim(),
        "echo: hi",
        "TW: text-блоки content конкатенируются в String"
    );
}

#[test]
fn c2_call_echo_contract_vm() {
    let _env = lock_env();
    let _allow = AllowExec::set();
    let out = run_vm(CALL_CONTRACT).expect("VM must run the call contract");
    assert_eq!(
        out.expect("flow output expected").trim(),
        "echo: hi",
        "VM: паритет с TW"
    );
}

// ── 2. Громкие ошибки протокола (фаза + код в тексте) ───────────────

/// Неизвестный инструмент: сервер отвечает JSON-RPC error -32602 →
/// клиент обязан назвать MCP_TOOL_NOT_FOUND, код и имя инструмента.
#[test]
fn c3_tool_not_found_is_loud() {
    let _env = lock_env();
    let _allow = AllowExec::set();
    let src = CALL_CONTRACT.replace("\"echo\"", "\"nosuchtool\"");
    let err = run_tw(&src).expect_err("unknown tool must be a loud error");
    assert!(err.contains("MCP_TOOL_NOT_FOUND"), "got: {}", err);
    assert!(err.contains("-32602"), "server code must propagate: {}", err);
    assert!(err.contains("nosuchtool"), "tool name in error: {}", err);
}

/// isError=true — это НЕ результат, а громкий отказ с текстом сервера.
#[test]
fn c4_tool_iserror_is_loud() {
    let _env = lock_env();
    let _allow = AllowExec::set();
    let src = CALL_CONTRACT.replace("\"echo\"", "\"fail\"");
    let err = run_tw(&src).expect_err("isError=true must be a loud error");
    assert!(err.contains("MCP_TOOL_ERROR"), "got: {}", err);
    assert!(err.contains("FIXTURE_FAIL"), "server detail must ride: {}", err);
}

/// JSON-RPC error (-32603) пробрасывается с кодом сервера.
#[test]
fn c5_jsonrpc_error_propagates() {
    let _env = lock_env();
    let _allow = AllowExec::set();
    let src = CALL_CONTRACT.replace("\"echo\"", "\"boom\"");
    let err = run_tw(&src).expect_err("JSON-RPC error must be loud");
    assert!(err.contains("MCP_PROTOCOL_ERROR"), "got: {}", err);
    assert!(err.contains("-32603"), "server code must propagate: {}", err);
    assert!(err.contains("phase=tools/call"), "phase named: {}", err);
}

/// Таймаут НА ФАЗУ: медленный инструмент + METALOGOS_MCP_TIMEOUT_SECS=1.
#[test]
fn c6_timeout_is_loud() {
    let _env = lock_env();
    let _allow = AllowExec::set();
    std::env::set_var("METALOGOS_MCP_TIMEOUT_SECS", "1");
    let src = CALL_CONTRACT
        .replace("\"echo\"", "\"sleep\"")
        .replace("{\"text\":\"hi\"}", "{\"seconds\":3}");
    let err = run_tw(&src).expect_err("slow tool must hit the phase timeout");
    assert!(err.contains("MCP_TIMEOUT"), "got: {}", err);
    assert!(err.contains("phase=tools/call"), "phase named: {}", err);
    unset_env(&["METALOGOS_MCP_TIMEOUT_SECS"]);
}

/// Garbage в stdout (нарушение спеки сервером) — громкий отказ фазы
/// handshake, не тишь и не паника.
#[test]
fn c7_garbage_stdout_is_loud() {
    let _env = lock_env();
    let _allow = AllowExec::set();
    std::env::set_var("MCP_FIXTURE_GARBAGE", "1");
    let err = run_tw(LIST_CONTRACT).expect_err("garbage on stdout must be loud");
    assert!(err.contains("MCP_PROTOCOL_ERROR"), "got: {}", err);
    assert!(
        err.contains("not valid JSON-RPC"),
        "diagnostic must name the framing problem: {}",
        err
    );
    assert!(err.contains("phase=handshake"), "phase named: {}", err);
    unset_env(&["MCP_FIXTURE_GARBAGE"]);
}

/// Сервер умер между получением initialize и ответом: обрыв потока —
/// громкий [MCP_IO_ERROR] phase=handshake, процесс-сирота невозможен.
#[test]
fn c8_crashed_server_is_loud() {
    let _env = lock_env();
    let _allow = AllowExec::set();
    std::env::set_var("MCP_FIXTURE_CRASH", "1");
    let err = run_tw(LIST_CONTRACT).expect_err("crashed server must be loud");
    assert!(err.contains("MCP_IO_ERROR"), "got: {}", err);
    assert!(
        err.contains("closed stdout") || err.contains("stream broken"),
        "diagnostic must name the broken stream: {}",
        err
    );
    unset_env(&["MCP_FIXTURE_CRASH"]);
}

// ── 3. Гейты ─────────────────────────────────────────────────────────

/// Exec-гейт №253-А: без METALOGOS_ALLOW_EXEC=1 spawn MCP-сервера
/// denied в процесс-контексте, код EXEC_NOT_PERMITTED.
#[test]
fn c9_exec_gate_denied_without_flag() {
    let _env = lock_env();
    unset_env(&["METALOGOS_ALLOW_EXEC"]);
    let err = run_tw(LIST_CONTRACT).expect_err("MCP spawn without flag must be denied");
    assert!(err.contains("EXEC_NOT_PERMITTED"), "got: {}", err);
    assert!(
        err.contains("METALOGOS_ALLOW_EXEC=1"),
        "error must name the exact flag: {}",
        err
    );
}

/// Allowlist: unset — не сужает (контракты c1/c2 уже зелёные при unset).
/// Пустая строка — deny all MCP с MCP_NOT_ALLOWLISTED.
#[test]
fn c10_allowlist_empty_denies_all() {
    let _env = lock_env();
    let _allow = AllowExec::set();
    std::env::set_var("METALOGOS_MCP_ALLOWLIST", "");
    let err = run_tw(LIST_CONTRACT).expect_err("empty allowlist must deny all MCP");
    assert!(err.contains("MCP_NOT_ALLOWLISTED"), "got: {}", err);
    unset_env(&["METALOGOS_MCP_ALLOWLIST"]);
}

/// Allowlist: точное совпадение argv[0]; несовпадение — отказ с именем.
#[test]
fn c10b_allowlist_exact_match() {
    let _env = lock_env();
    let _allow = AllowExec::set();
    std::env::set_var("METALOGOS_MCP_ALLOWLIST", "uvx , notpython");
    let err = run_tw(LIST_CONTRACT).expect_err("non-listed command must be denied");
    assert!(err.contains("MCP_NOT_ALLOWLISTED"), "got: {}", err);
    assert!(err.contains("python3"), "denied command named: {}", err);
    // Положительный случай: точное имя в списке (trim элементов — конвенция №259).
    std::env::set_var("METALOGOS_MCP_ALLOWLIST", "uvx , python3");
    let out = run_tw(LIST_CONTRACT).expect("listed command must pass the allowlist");
    assert!(out.expect("output").contains("echo"), "echo tool listed");
    unset_env(&["METALOGOS_MCP_ALLOWLIST"]);
}

// ── 4. Taint-интеграция (статика, без spawn) ────────────────────────

fn has_finding(source: &str, check_id: &str) -> bool {
    match metalogos::check_program(source) {
        Ok(result) => result.errors.iter().any(|e| e.message.contains(check_id)),
        Err(_) => false,
    }
}

const TAINT_TRAIN_SOURCE: &str = r#"
        reflex Classifier {
            input: embedding(2)
            layers: [dense(8, relu), dense(2, softmax)]
            labels: ["a", "b"]
            seed: 42
        }

        pattern Poison(x: String) -> String {
            let body = mcp_call("python3", ["tests/fixtures/mcp_echo_server.py"], "echo", "{\"text\":\"hi\"}")
            let data = [[body, 0.0]]
            reflex_train(Classifier, data, 10.0, "accuracy", 0.5)
            return "ok"
        }
    "#;

/// Ядро ценности №268: результат mcp_call НЕ может обучать reflex —
/// статика отвергает с UNTRUSTED_TRAINING_DATA (как для json_body;
/// категория-A sink для UserInput-родов — именно reflex_train).
#[test]
fn c11_taint_mcp_output_into_reflex_train_blocked() {
    assert!(
        has_finding(TAINT_TRAIN_SOURCE, "UNTRUSTED_TRAINING_DATA"),
        "reflex_train на MCP-выводе должен быть отвергнут статикой"
    );
}

/// Негатив: литеральные данные в reflex_train — без MCP-находки.
#[test]
fn c11b_negative_literal_data_no_finding() {
    let src = r#"
        reflex Classifier {
            input: embedding(2)
            layers: [dense(8, relu), dense(2, softmax)]
            labels: ["a", "b"]
            seed: 42
        }

        pattern Clean(x: String) -> String {
            let data = [["literal data", 0.0]]
            reflex_train(Classifier, data, 10.0, "accuracy", 0.5)
            return "ok"
        }
    "#;
    assert!(
        !has_finding(src, "UNTRUSTED_TRAINING_DATA"),
        "литеральные данные не должны триггерить UNTRUSTED_TRAINING_DATA"
    );
}

/// Граница (паритет с json_body, честно): respond(mcp_call(...)) НЕ флагается
/// — HTML_INJECTION в аудите ловит ТОЛЬКО LLM-род (№123), UserInput-роды
/// проходят в respond, как и form_data/json_body. Policy-дифференциация
/// родов — Future (ADR-0132 D3: ToolOutput заводится только с политикой).
/// Тест закрепляет: политика MCP = политика json_body, ни шире, ни уже.
#[test]
fn c12_taint_policy_parity_with_json_body() {
    let mcp_src = r#"
        pattern Direct(x: String) -> String {
            return respond("200", mcp_call("python3", ["tests/fixtures/mcp_echo_server.py"], "echo", "{\"text\":\"hi\"}"))
        }
    "#;
    let json_src = r#"
        pattern Direct(x: String) -> String {
            return respond("200", json_body())
        }
    "#;
    let mcp_has = has_finding(mcp_src, "HTML_INJECTION") || has_finding(mcp_src, "UNTRUSTED");
    let json_has = has_finding(json_src, "HTML_INJECTION") || has_finding(json_src, "UNTRUSTED");
    assert_eq!(
        mcp_has, json_has,
        "политика taint для mcp_call обязана совпадать с json_body (оба {})",
        json_has
    );
    assert!(!mcp_has, "UserInput-роды в respond не флагаются (паритет); если это изменилось — обнови и ADR-0132, и этот тест");
}

// ── 5. Аудит-лог ─────────────────────────────────────────────────────

/// Каждый разрешённый spawn — запись `\tmcp_list_tools\t` с исходом "ok".
#[test]
fn c13_audit_log_records_mcp_spawn() {
    let _env = lock_env();
    let _allow = AllowExec::set();
    let dir = tempfile::tempdir().expect("tempdir");
    let log = dir.path().join("mcp_audit.log");
    std::env::set_var("METALOGOS_AUDIT_LOG_PATH", log.to_str().expect("utf8 path"));
    let _ = run_tw(LIST_CONTRACT).expect("contract must run");
    let contents = std::fs::read_to_string(&log).expect("audit log must exist");
    assert!(
        contents.contains("\tmcp_list_tools\t"),
        "audit must record the operation, got: {}",
        contents
    );
    assert!(
        contents.contains("\tok\n") || contents.contains("\tok"),
        "successful spawn must be logged as ok, got: {}",
        contents
    );
    assert!(
        contents.contains("mcp_echo_server.py"),
        "audit detail must carry the server command, got: {}",
        contents
    );
    unset_env(&["METALOGOS_AUDIT_LOG_PATH"]);
}

// ── 6. Serve-роут-гейт (ОБА бэкенда) ─────────────────────────────────

#[cfg(feature = "server")]
mod serve_gate {
    use super::{unset_env, FIXTURE};
    use metalogos::server::ServeBackend;
    use std::sync::Mutex;

    static SERVE_ENV_LOCK: Mutex<()> = Mutex::new(());

    fn serve_lock() -> std::sync::MutexGuard<'static, ()> {
        SERVE_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    const MCP_ROUTE_SOURCE: &str = r#"
mlogserver {
  port: 8096
  route "/mcp" method=GET {
    let tools = mcp_list_tools("python3", ["tests/fixtures/mcp_echo_server.py"])
    let first = get(tools, 0)
    respond("200", json_get(first, "name"))
  }
}
"#;

    async fn start_server(
        backend: ServeBackend,
    ) -> (
        u16,
        tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    ) {
        let base_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        metalogos::server::run_test_server_with_backend_in_dir(MCP_ROUTE_SOURCE, backend, base_dir)
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

    /// Serve-роут: MCP-вызов denied по умолчанию (TW-бэкенд).
    #[tokio::test]
    async fn c14_serve_route_mcp_denied_by_default_tw() {
        let _env = serve_lock();
        std::env::remove_var("METALOGOS_ALLOW_EXEC");
        std::env::remove_var("METALOGOS_SERVE_ALLOW_EXEC");

        let (port, _handle) = start_server(ServeBackend::Interpreter).await;
        let (status, body) = http_get(port, "/mcp").await;

        assert_eq!(status, 500, "MCP в роуте без флагов должен быть denied: {}", body);
        assert!(body.contains("EXEC_NOT_PERMITTED"), "got: {}", body);
    }

    /// Serve-роут: то же на VM-бэкенде (гейт контекстный, не бэкендовый).
    #[tokio::test]
    async fn c14_serve_route_mcp_denied_by_default_vm() {
        let _env = serve_lock();
        std::env::remove_var("METALOGOS_ALLOW_EXEC");
        std::env::remove_var("METALOGOS_SERVE_ALLOW_EXEC");

        let (port, _handle) = start_server(ServeBackend::Vm).await;
        let (status, body) = http_get(port, "/mcp").await;

        assert_eq!(status, 500, "MCP в роуте без флагов должен быть denied (VM): {}", body);
        assert!(body.contains("EXEC_NOT_PERMITTED"), "got: {}", body);
    }

    /// Escape-hatch по №253-А: METALOGOS_SERVE_ALLOW_EXEC=1 разрешает —
    /// и тогда полный протокол до fixture-сервера отрабатывает в роуте.
    #[tokio::test]
    async fn c15_serve_route_mcp_allowed_with_serve_flag() {
        let _env = serve_lock();
        std::env::remove_var("METALOGOS_ALLOW_EXEC");
        std::env::set_var("METALOGOS_SERVE_ALLOW_EXEC", "1");

        let (port, _handle) = start_server(ServeBackend::Interpreter).await;
        let (status, body) = http_get(port, "/mcp").await;

        assert_eq!(status, 200, "serve-флаг разрешает MCP в роуте: {}", body);
        assert!(body.contains("echo"), "fixture tool name expected: {}", body);
        unset_env(&["METALOGOS_SERVE_ALLOW_EXEC"]);
    }

    // FIXTURE используется косвенно: путь зашит в MCP_ROUTE_SOURCE;
    // константа держит источник истины рядом с тестами.
    #[test]
    fn fixture_path_is_stable() {
        assert!(FIXTURE.starts_with("tests/fixtures/"));
    }
}
