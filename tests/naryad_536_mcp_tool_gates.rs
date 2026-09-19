// ── gh#536 (the №401 live-verification findings): MCP tool-method
//    gates ─────────────────────────────────────────────────────────────
//
// Two live findings from the external MCP-HTTP verification of naryad
// #401 (2026-09-19, binary main `2618673a`) that the in-repo suite
// never pinned:
//
//   Finding 1 — env() was readable inside a tool method: the №259 gate
//   is ExecContext::ServeRoute-scoped, and tool methods executed in the
//   Process context, so `tools/call secrets.peek` returned the live
//   probe value over HTTP. Fix: tool method bodies run under the SAME
//   ServeRouteExecGuard thread-local as route bodies (№253-А mechanic)
//   — env() is №259-gated, exec() is №253-gated with serve semantics
//   (the process-level METALOGOS_ALLOW_EXEC flag does NOT apply).
//
//   Finding 2 — mcp-serve only PARSED the file, so a tool file that was
//   never `mlog check`-ed ran tool bodies unchecked on the interpreter
//   path: a tool method `db_execute("DROP TABLE " + table)` executed
//   until the SQL layer refused. Fix: every mcp-serve entrypoint
//   (stdio, http/sse, and the test harness) enforces the Category A
//   startup gate (`enforce_category_a_startup`, the №98 `run_server`
//   precedent) — IRREVERSIBLE_NO_GRANT and SQL_DYNAMIC refuse the
//   process loudly before any bind, on every transport.
//
// The probe sources below are the LIVE shapes from the verification
// report (variable name and sentinel value preserved for traceability).

use metalogos::mcp_server::{enforce_category_a_startup, McpAuth, McpServer};
use std::sync::Mutex;

// Лекало naryad_259/naryad_253: env-переменные процесса глобальны —
// сериализуем env-тесты одного файла poison-tolerant мьютексом и держим
// его ВСЁ тело теста (урок №261: позитив соседнего потока подсадил флаг
// негативному кейсу).
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

const PROBE_NAME: &str = "METALOGOS_MCP401_PROBE";
const PROBE_VALUE: &str = "probe-value-401";

const ENV_TOOL_SOURCE: &str = r#"
tool secrets {
  peek(key: String) -> String {
    return env(key)
  }
}
"#;

const EXEC_TOOL_SOURCE: &str = r#"
tool runner {
  run(cmd: String) -> String {
    return exec(cmd)
  }
}
"#;

/// The EXACT Finding-2 probe shape from the live verification: dynamic
/// destructive SQL in a tool method body.
const DYNAMIC_SQL_TOOL_SOURCE: &str = r#"
tool admin {
  wipe(table: String) -> String {
    return db_execute("DROP TABLE " + table)
  }
}
"#;

/// The literal destructive-SQL shape: refused by the same gate with the
/// №325 sink-clearance class.
const LITERAL_SQL_TOOL_SOURCE: &str = r#"
tool admin {
  wipe(table: String) -> String {
    return db_execute("DROP TABLE sessions")
  }
}
"#;

const WRITE_TOOL_SOURCE: &str = r#"
tool notify {
  send(message: String, channel: String) -> String {
    write_file("target/n536_gate_test_out.txt", "to " + channel + ": " + message)
    return "sent:" + message
  }
}
"#;

fn parse(src: &str) -> Vec<metalogos::ast::Declaration> {
    metalogos::parser::parse(src).expect("parses")
}

// ── Finding 1: env() in tool methods ──────────────────────────────────

#[test]
fn env_in_tool_method_is_refused_and_leaks_nothing() {
    let _env = lock_env();
    std::env::set_var(PROBE_NAME, PROBE_VALUE);

    let decls = parse(ENV_TOOL_SOURCE);
    let server = McpServer::new(&decls, &["secrets.peek".to_string()]).expect("server builds");
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

    std::env::remove_var(PROBE_NAME);
}

#[test]
fn env_escape_hatch_allows_tool_method_reads() {
    let _env = lock_env();
    std::env::set_var(PROBE_NAME, PROBE_VALUE);
    std::env::set_var("METALOGOS_SERVE_ALLOW_ENV", "1");

    let decls = parse(ENV_TOOL_SOURCE);
    let server = McpServer::new(&decls, &["secrets.peek".to_string()]).expect("server builds");
    let resp = server.handle_request(
        "tools/call",
        &serde_json::json!({"name": "secrets.peek", "arguments": {"key": PROBE_NAME}}),
        &serde_json::json!(2),
    );

    let text = resp["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert_eq!(resp["result"]["isError"], false, "{}", resp);
    assert_eq!(text, PROBE_VALUE, "the loud opt-in must allow the read");

    std::env::remove_var("METALOGOS_SERVE_ALLOW_ENV");
    std::env::remove_var(PROBE_NAME);
}

#[test]
fn env_allowlist_allows_exactly_the_listed_names() {
    let _env = lock_env();
    std::env::set_var(PROBE_NAME, PROBE_VALUE);
    std::env::set_var("METALOGOS_ENV_ALLOWLIST", PROBE_NAME);

    let decls = parse(ENV_TOOL_SOURCE);
    let server = McpServer::new(&decls, &["secrets.peek".to_string()]).expect("server builds");

    // Listed → allowed (точечное разрешение).
    let ok = server.handle_request(
        "tools/call",
        &serde_json::json!({"name": "secrets.peek", "arguments": {"key": PROBE_NAME}}),
        &serde_json::json!(3),
    );
    assert_eq!(ok["result"]["isError"], false, "{}", ok);

    // NOT listed → still refused (precision: the allowlist is not allow-all).
    let refused = server.handle_request(
        "tools/call",
        &serde_json::json!({"name": "secrets.peek", "arguments": {"key": "METALOGOS_OTHER_SECRET"}}),
        &serde_json::json!(4),
    );
    let text = refused["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("");
    assert_eq!(refused["result"]["isError"], true, "{}", refused);
    assert!(text.contains("ENV_NOT_PERMITTED"), "{}", text);

    std::env::remove_var("METALOGOS_ENV_ALLOWLIST");
    std::env::remove_var(PROBE_NAME);
}

#[test]
fn exec_in_tool_method_names_the_serve_flag_and_ignores_the_process_flag() {
    let _env = lock_env();
    // The №253-А "replacement, not AND" semantics, now pinned for tool
    // methods: even the process-level opt-in must NOT enable exec in a
    // tool method — the MCP caller is the untrusted party.
    std::env::set_var("METALOGOS_ALLOW_EXEC", "1");

    let decls = parse(EXEC_TOOL_SOURCE);
    let server = McpServer::new(&decls, &["runner.run".to_string()]).expect("server builds");
    let resp = server.handle_request(
        "tools/call",
        &serde_json::json!({"name": "runner.run", "arguments": {"cmd": "echo hi"}}),
        &serde_json::json!(5),
    );

    let text = resp["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert_eq!(resp["result"]["isError"], true, "{}", resp);
    assert!(
        text.contains("EXEC_NOT_PERMITTED"),
        "the refusal must carry the №253 code: {}",
        text
    );
    assert!(
        text.contains("METALOGOS_SERVE_ALLOW_EXEC"),
        "the refusal must name the serve-context flag (tool methods run \
         under serve semantics, not the process flag): {}",
        text
    );

    std::env::remove_var("METALOGOS_ALLOW_EXEC");
}

// ── Finding 2: the Category A startup gate ────────────────────────────

#[test]
fn startup_gate_refuses_the_live_dynamic_sql_probe() {
    let decls = parse(DYNAMIC_SQL_TOOL_SOURCE);
    let err = enforce_category_a_startup(&decls)
        .expect_err("the gh#536 probe must be refused at startup");
    assert!(
        err.contains("Category A security invariant violated"),
        "{}",
        err
    );
    assert!(err.contains("SQL_DYNAMIC"), "{}", err);
}

#[test]
fn startup_gate_refuses_literal_destructive_sql() {
    let decls = parse(LITERAL_SQL_TOOL_SOURCE);
    let err = enforce_category_a_startup(&decls)
        .expect_err("the literal DROP must be refused at startup");
    assert!(
        err.contains("IRREVERSIBLE_NO_GRANT"),
        "the №325 sink-clearance class is the contract: {}",
        err
    );
}

#[test]
fn startup_gate_passes_clean_tool_files() {
    // The №394 suite's tool sources (write_file sink + exec probe) and
    // the env probe must all SERVE — the gate refuses dangerous files,
    // not the tool construct itself.
    for src in [ENV_TOOL_SOURCE, EXEC_TOOL_SOURCE, WRITE_TOOL_SOURCE] {
        let decls = parse(src);
        enforce_category_a_startup(&decls)
            .unwrap_or_else(|e| panic!("clean tool file must serve: {}", e));
    }
}

#[test]
fn stdio_entrypoint_enforces_the_startup_gate() {
    // run_mcp_server gates BEFORE the stdin loop — a dirty file is
    // refused for what it IS, not for what it exposes.
    let decls = parse(DYNAMIC_SQL_TOOL_SOURCE);
    let err = metalogos::mcp_server::run_mcp_server(&decls, &["admin.wipe".to_string()])
        .expect_err("the stdio entrypoint must refuse the dirty file");
    assert!(err.contains("Category A"), "{}", err);
    assert!(err.contains("SQL_DYNAMIC"), "{}", err);
}

#[cfg(feature = "server")]
mod network {
    use super::*;

    #[tokio::test]
    async fn network_entrypoint_enforces_the_startup_gate() {
        let decls = parse(LITERAL_SQL_TOOL_SOURCE);
        // run_mcp_server_network builds its own tokio runtime — it is a
        // sync call that returns before any serving starts.
        let err = metalogos::mcp_server::run_mcp_server_network(
            &decls,
            &["admin.wipe".to_string()],
            "http",
            "127.0.0.1:0",
            &McpAuth::OpenLocal,
        )
        .expect_err("the network entrypoint must refuse the dirty file");
        assert!(err.contains("Category A"), "{}", err);
        assert!(err.contains("IRREVERSIBLE_NO_GRANT"), "{}", err);
    }

    #[tokio::test]
    async fn test_harness_enforces_the_startup_gate() {
        // The №394 integration tests walk the REAL surface through this
        // helper — the startup gate is part of that surface.
        let decls = parse(DYNAMIC_SQL_TOOL_SOURCE);
        let err = metalogos::mcp_server::run_test_mcp_server(
            &decls,
            &["admin.wipe".to_string()],
            &McpAuth::OpenLocal,
        )
        .await
        .expect_err("the test harness must refuse the dirty file");
        assert!(err.contains("Category A"), "{}", err);
    }
}
