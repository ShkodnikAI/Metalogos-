// ── Naryad №793 (issue #1531, Wave 64 P1 — the gateway G1) ───────────
//
// The upstream SESSION pool: ONE process per declared upstream for the
// gateway's lifetime (METALOGOS_MCP_UPSTREAMS, read once at
// McpServer::new — the №263 discipline), the aggregated tools/list with
// the `name.` prefix, the tools/call proxy, the loud reconnect on an
// upstream death, the allowlist gate riding the declaration, the
// version negotiation (the client declares 2025-11-25; the server
// answers the client's known revision).
//
// RED-before (форма №772): the battery was run against the pre-№793
// revision (worktree @ 0294396) — every upstream pin FAILED there (the
// env var did not exist; tools/list carried no upstream tools; the
// initialize answer was hardcoded 2024-11-05).
//
// The stateful proof rides tests/fixtures/mcp_stateful_server.py: the
// in-memory counter restarts ONLY with a fresh process — three calls
// answering 0,1,2 = one live session; after `crash` the next call
// works again (the loud reconnect) and the counter is back at 0 (a
// NEW process — the old one is verifiably gone).

use metalogos_server::mcp_server::McpServer;
use std::path::PathBuf;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

const LOCAL_TOOL_SOURCE: &str = r#"
tool local_api {
  send(message: String) -> String {
    return "local:" + message
  }
}
"#;

fn fixture_path(name: &str) -> String {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .join("..")
        .join("tests")
        .join("fixtures")
        .join(name)
        .to_string_lossy()
        .to_string()
}

fn parse(src: &str) -> Vec<metalogos::ast::Declaration> {
    metalogos::parser::parse(src).expect("parses")
}

fn call_text(response: &serde_json::Value) -> String {
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .expect("text block");
    text.to_string()
}

/// ── The core pin: ONE session, THREE calls, ONE state ──
/// The counter answers 0, 1, 2 — the process survived between the
/// calls (the pre-№793 spawn-per-call form would restart at 0 forever).
#[test]
fn n793_upstream_session_state_persists_across_calls() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var(
        "METALOGOS_MCP_UPSTREAMS",
        format!(
            "stateful=python3,{}",
            fixture_path("mcp_stateful_server.py")
        ),
    );
    let server = McpServer::new(&parse(LOCAL_TOOL_SOURCE), &["local_api.send".to_string()])
        .expect("gateway with a stateful upstream");

    let list = server.handle_request("tools/list", &serde_json::json!({}), &serde_json::json!(1));
    let names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(
        names.contains(&"stateful.count"),
        "tools/list must aggregate the upstream tools with the server prefix, got: {:?}",
        names
    );
    assert!(names.contains(&"local_api.send"), "the local tools stay");

    let mut answers = Vec::new();
    for _ in 0..3 {
        let r = server.handle_request(
            "tools/call",
            &serde_json::json!({"name": "stateful.count", "arguments": {}}),
            &serde_json::json!(2),
        );
        assert_eq!(
            r["result"]["isError"], false,
            "the call must succeed: {}",
            r
        );
        answers.push(call_text(&r));
    }
    assert_eq!(
        answers,
        vec!["0", "1", "2"],
        "three calls in ONE session see ONE in-memory state (the pre-№793 spawn-per-call would answer 0,0,0)"
    );
    std::env::remove_var("METALOGOS_MCP_UPSTREAMS");
}

/// ── The death + loud reconnect pin ──
/// `crash` kills the upstream (the process exits after answering); the
/// NEXT call reconnects (a fresh process — the counter restarts at 0)
/// and the refusal of the dead-window call is the isError envelope, not
/// a hang and not a silent zero.
#[test]
fn n793_upstream_death_loud_reconnect() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var(
        "METALOGOS_MCP_UPSTREAMS",
        format!(
            "stateful=python3,{}",
            fixture_path("mcp_stateful_server.py")
        ),
    );
    let server = McpServer::new(&parse(LOCAL_TOOL_SOURCE), &["local_api.send".to_string()])
        .expect("gateway with a stateful upstream");

    // One call to prove the session is live (counter -> 0, increments to 1).
    let r = server.handle_request(
        "tools/call",
        &serde_json::json!({"name": "stateful.count", "arguments": {}}),
        &serde_json::json!(1),
    );
    assert_eq!(call_text(&r), "0");

    // Crash the upstream: the answer of THIS call is delivered (the
    // fixture answers, then exits) — the session is dead NOW.
    let r = server.handle_request(
        "tools/call",
        &serde_json::json!({"name": "stateful.crash", "arguments": {}}),
        &serde_json::json!(2),
    );
    assert_eq!(call_text(&r), "bye");

    // The reconnect: the death is detected via the stream-EOF flag (the
    // deterministic form) or the reaped exit; the attempt window between
    // the fixture's exit and the EOF processing is honest — a loud
    // isError envelope, never a hang. Retry up to 5 attempts; the FIRST
    // failure (if any) must be the loud envelope; the eventual success
    // answers 0 — a FRESH process (the old one is verifiably gone).
    let mut reconnected = None;
    for _attempt in 0..5 {
        let r = server.handle_request(
            "tools/call",
            &serde_json::json!({"name": "stateful.count", "arguments": {}}),
            &serde_json::json!(3),
        );
        if r["result"]["isError"] == false {
            reconnected = Some(call_text(&r));
            break;
        }
        let detail = r["result"]["content"][0]["text"].as_str().unwrap_or("");
        assert!(
            detail.contains("Error:") || detail.contains("[SCHEMA_DIGEST_MISMATCH]"),
            "a failed attempt must be the loud error envelope, not a hang: {}",
            r
        );
    }
    assert_eq!(
        reconnected.as_deref(),
        Some("0"),
        "the reconnect must succeed with a fresh process (the counter restarts)"
    );
    std::env::remove_var("METALOGOS_MCP_UPSTREAMS");
}

/// ── The local-first collision rule ──
/// An upstream named like the local tool's namespace: the ambiguous
/// call routes to the LOCAL tool (the rule pinned loudly at startup).
#[test]
fn n793_local_tools_win_name_collision() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var(
        "METALOGOS_MCP_UPSTREAMS",
        format!(
            "local_api=python3,{}",
            fixture_path("mcp_stateful_server.py")
        ),
    );
    let server = McpServer::new(&parse(LOCAL_TOOL_SOURCE), &["local_api.send".to_string()])
        .expect("gateway with a colliding upstream name");

    // `local_api.send` — the LOCAL tool's full_name wins.
    let r = server.handle_request(
        "tools/call",
        &serde_json::json!({"name": "local_api.send", "arguments": {"message": "hi"}}),
        &serde_json::json!(1),
    );
    assert_eq!(
        call_text(&r),
        "local:hi",
        "the LOCAL tool wins the collision (the upstream has no `send` tool and the local form answers): {}",
        r
    );
    std::env::remove_var("METALOGOS_MCP_UPSTREAMS");
}

/// ── Unknown tool stays a loud unknown-tool refusal ──
#[test]
fn n793_unknown_upstream_tool_loud_refusal() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var(
        "METALOGOS_MCP_UPSTREAMS",
        format!(
            "stateful=python3,{}",
            fixture_path("mcp_stateful_server.py")
        ),
    );
    let server = McpServer::new(&parse(LOCAL_TOOL_SOURCE), &["local_api.send".to_string()])
        .expect("gateway");
    let r = server.handle_request(
        "tools/call",
        &serde_json::json!({"name": "nosuch.tool", "arguments": {}}),
        &serde_json::json!(1),
    );
    assert!(
        r["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("Unknown tool"),
        "an unknown prefix stays the unknown-tool refusal: {}",
        r
    );
    std::env::remove_var("METALOGOS_MCP_UPSTREAMS");
}

/// ── The allowlist gate rides the DECLARATION (fail-closed) ──
/// An upstream whose argv[0] is outside METALOGOS_MCP_ALLOWLIST refuses
/// the whole gateway at construction — a config error, not a transient.
#[test]
fn n793_allowlist_gate_on_declaration_fail_closed() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var("METALOGOS_MCP_ALLOWLIST", "definitely-not-this-command");
    std::env::set_var(
        "METALOGOS_MCP_UPSTREAMS",
        format!(
            "stateful=python3,{}",
            fixture_path("mcp_stateful_server.py")
        ),
    );
    let res = McpServer::new(&parse(LOCAL_TOOL_SOURCE), &["local_api.send".to_string()]);
    let err = match res {
        Err(e) => e,
        Ok(_) => panic!("the declaration outside the allowlist must refuse the gateway"),
    };
    assert!(
        err.contains("MCP_NOT_ALLOWLISTED"),
        "the refusal carries the typed allowlist code: {}",
        err
    );
    std::env::remove_var("METALOGOS_MCP_ALLOWLIST");
    std::env::remove_var("METALOGOS_MCP_UPSTREAMS");
}

/// ── The version negotiation (the server side) ──
/// The server answers the client's requested revision when known (both
/// the modern and the legacy forms); an unknown request gets the
/// server's latest.
#[test]
fn n793_server_version_negotiation() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let server = McpServer::new(&parse(LOCAL_TOOL_SOURCE), &["local_api.send".to_string()])
        .expect("gateway");
    for (requested, expected) in [
        ("2025-11-25", "2025-11-25"),
        ("2024-11-05", "2024-11-05"),
        ("1999-01-01", "2025-11-25"),
    ] {
        let r = server.handle_request(
            "initialize",
            &serde_json::json!({"protocolVersion": requested}),
            &serde_json::json!(1),
        );
        assert_eq!(
            r["result"]["protocolVersion"], expected,
            "initialize must negotiate {} -> {}",
            requested, expected
        );
    }
}
