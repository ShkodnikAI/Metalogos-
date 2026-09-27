// ── Naryad #477 (P1, security/mcp): the mcp-serve inversion ────────────
//
// Contract (issue #725, audit 26.09 §3.5 Medium):
// 1. THE INVERSION: `mlog mcp-serve` sets the process mode to Serve at
//    the entrypoint (the №457 serve posture, `cmd_serve` precedent) —
//    `mcp-serve` with the http/sse transports is the same network server
//    executing user code as serve, so from the entrypoint on, every
//    UNMARKED thread is `ServeRoute`: env() is №259-gated, exec() is
//    №253-gated. Background tasks, resource-handler calls and future MCP
//    entry points fail LOUD instead of silently keeping process rights
//    ("protected only by the developer's memory" is what №457 removed
//    for serve; №477 removes the same shape for mcp-serve).
// 2. THE TOOL SURFACE DOES NOT DEGRADE: tool calls keep their explicit
//    `ServeRouteExecGuard` (№401, `execute_tool_method`) — `tools/call`
//    behaves EXACTLY as before №477 on every transport (stdio included):
//    env() per №259 (the serve flag / the allowlist), exec() naming
//    `METALOGOS_SERVE_ALLOW_EXEC`.
// 3. THE REGISTRATION ZONE: `McpServer::new` holds the Process context
//    for the tool-registration phase (`TopLevelRegistrationGuard`, the
//    №457 precedent) — the `tools/list` surface is unregressed under the
//    inverted default.
//
// The №477 pins follow the №457 test pattern (the machinery under the
// inverted default + the regression pins on the guarded surfaces); the
// MCP harness follows the №536 in-process `McpServer::handle_request`
// posture (the transport-agnostic request core, single-sourced).

use metalogos::builtins::{
    current_exec_context, exec_gate, set_process_mode, ExecContext, ProcessMode,
    ServeRouteExecGuard, TopLevelRegistrationGuard,
};
use metalogos::mcp_server::McpServer;
use std::sync::Mutex;

// Лекало naryad_259/naryad_457: env-переменные процесса глобальны —
// сериализуем тесты одного файла одним poison-tolerant мьютексом.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Drop-сторож режима процесса: тесты, включающие строгий serve-дефолт,
/// обязаны вернуть Process даже при панике (лекало naryad_457).
struct RestoreProcessMode;
impl Drop for RestoreProcessMode {
    fn drop(&mut self) {
        set_process_mode(ProcessMode::Process);
    }
}

/// Drop-сторож переменной окружения (лекало naryad_457): негативный
/// кейс не подсаживает позитивный.
struct EnvVar(&'static str);
impl EnvVar {
    fn set(name: &'static str, value: &str) -> Self {
        std::env::set_var(name, value);
        EnvVar(name)
    }
}
impl Drop for EnvVar {
    fn drop(&mut self) {
        std::env::remove_var(self.0);
    }
}

const PROBE_NAME: &str = "METALOGOS_MCP477_PROBE";
const PROBE_VALUE: &str = "probe-value-477";

const ENV_TOOL_SOURCE: &str = r#"
tool secrets {
  peek(key: String) -> String {
    return env(key)
  }
}
"#;

// ── 1. Инверсия: непомеченный поток внутри mcp-serve — ServeRoute ──────

#[test]
fn n477_unmarked_background_thread_is_serve_route() {
    let _env = lock_env();
    let _restore = RestoreProcessMode;

    // The exact entrypoint line №477 adds to `cmd_mcp_serve`.
    set_process_mode(ProcessMode::Serve);

    // The №457 test pattern, the BACKGROUND-TASK shape: a spawned thread
    // inherits nothing — under the inverted default it must resolve to
    // ServeRoute and its exec()/env() gates must refuse LOUD.
    let result = std::thread::spawn(|| {
        let ctx = current_exec_context();
        let exec_err = exec_gate(ctx).err();
        // №259: env() on a serve-route thread refuses even for a SET
        // variable (the gate is before the read — no existence oracle).
        let env_err = metalogos::builtins::env_gate(ctx, PROBE_NAME).err();
        (ctx, exec_err, env_err)
    })
    .join()
    .expect("the probe thread must not panic");

    assert_eq!(
        result.0,
        ExecContext::ServeRoute,
        "an unmarked background thread under the mcp-serve inversion must be ServeRoute"
    );
    let exec_err = result.1.expect("exec_gate must refuse the unmarked thread");
    assert!(
        exec_err.contains("METALOGOS_SERVE_ALLOW_EXEC"),
        "the exec refusal must name the SERVER flag, got: {}",
        exec_err
    );
    let env_err = result.2.expect("env_gate must refuse the unmarked thread");
    assert!(
        env_err.contains("ENV_NOT_PERMITTED"),
        "the env refusal must carry the №259 code, got: {}",
        env_err
    );

    // The explicit route guard still marks a thread ServeRoute inside the
    // serve process (the tool-call mechanic №401 — unchanged).
    let route_ctx = std::thread::spawn(|| {
        let _route = ServeRouteExecGuard::new();
        current_exec_context()
    })
    .join()
    .expect("the route probe thread must not panic");
    assert_eq!(route_ctx, ExecContext::ServeRoute);
}

#[test]
fn n477_registration_zone_keeps_process_context() {
    let _env = lock_env();
    let _restore = RestoreProcessMode;
    set_process_mode(ProcessMode::Serve);

    // The top-level registration zone (the №457 machinery №477 reuses):
    // inside the zone the thread keeps the Process context; the explicit
    // route guard still wins over the zone (the resolution order).
    let _toplevel = TopLevelRegistrationGuard::new();
    assert_eq!(
        current_exec_context(),
        ExecContext::Process,
        "the registration zone must keep the process context"
    );
    {
        let _route = ServeRouteExecGuard::new();
        assert_eq!(
            current_exec_context(),
            ExecContext::ServeRoute,
            "the explicit route guard wins inside the zone"
        );
    }
    assert_eq!(current_exec_context(), ExecContext::Process);
}

// ── 2. Поверхность инструментов не деградирует (stdio включительно) ───

#[test]
fn n477_tool_call_is_unregressed_under_the_inverted_default() {
    let _env = lock_env();
    let _restore = RestoreProcessMode;
    let _probe = EnvVar::set(PROBE_NAME, PROBE_VALUE);

    // The entrypoint inversion (the exact line cmd_mcp_serve runs now).
    set_process_mode(ProcessMode::Serve);

    // Registration happens INSIDE the inverted process — McpServer::new
    // holds the top-level zone internally (item 2 of the naryad).
    let decls = metalogos::parser::parse(ENV_TOOL_SOURCE).expect("parses");
    let server = McpServer::new(&decls, &["secrets.peek".to_string()]).expect("server builds");

    // Without the serve escape hatch: the SAME №259 refusal as before
    // №477 — the tool guard (№401) applies, the process rights are not
    // what the call hits, and the probe value does not leak.
    let resp = server.handle_request(
        "tools/call",
        &serde_json::json!({"name": "secrets.peek", "arguments": {"key": PROBE_NAME}}),
        &serde_json::json!(1),
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert_eq!(resp["result"]["isError"], true, "{}", resp);
    assert!(
        text.contains("ENV_NOT_PERMITTED"),
        "the refusal must carry the №259 code: {}",
        text
    );
    assert!(
        !text.contains(PROBE_VALUE),
        "the probe value must NOT leak to the MCP caller: {}",
        text
    );

    // With the loud opt-in: the tool reads it — exactly the pre-№477
    // behavior (the explicit guard keeps working; stdio does not degrade).
    let _allow = EnvVar::set("METALOGOS_SERVE_ALLOW_ENV", "1");
    let resp = server.handle_request(
        "tools/call",
        &serde_json::json!({"name": "secrets.peek", "arguments": {"key": PROBE_NAME}}),
        &serde_json::json!(2),
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert_eq!(resp["result"]["isError"], false, "{}", resp);
    assert_eq!(text, PROBE_VALUE, "the loud opt-in must allow the read");
}

// ── 3. tools/list под инвертированным дефолтом: регрессий нет ─────────

#[test]
fn n477_tools_list_unregressed_under_the_inverted_default() {
    let _env = lock_env();
    let _restore = RestoreProcessMode;
    set_process_mode(ProcessMode::Serve);

    let decls = metalogos::parser::parse(ENV_TOOL_SOURCE).expect("parses");
    let server = McpServer::new(&decls, &["secrets.peek".to_string()]).expect("server builds");

    // The registration surface answers as before: the tool is listed with
    // its compiled №316 policy in _meta.
    let resp = server.handle_request("tools/list", &serde_json::json!({}), &serde_json::json!(3));
    let tools = resp["result"]["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 1, "{}", resp);
    assert_eq!(tools[0]["name"], "secrets.peek", "{}", resp);
    assert!(
        tools[0]["_meta"][metalogos::mcp_policy::ToolPolicy::META_KEY].is_object(),
        "the compiled policy meta must be present: {}",
        resp
    );
}
