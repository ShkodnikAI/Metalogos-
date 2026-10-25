// ── Naryad №801 (wave 65, P1 — audit-23.10 C1(b) G2, the gateway pilot): the floating session label ──
//
// Contract (issue #1548, the C1 (b) owner decision 23.10 — «шлюз — флагман
// на пробу»): the agent session carries ONE floating (current) label, LIO
// style. Every UPSTREAM tool has a class in the .mlog policy:
//   * Source(label) — the tool returns data at that label; a successful
//     call JOINs the label into the session label;
//   * Sink(label) — the tool demands that clearance; the session label
//     must FLOW TO it (conf ≤ required, integrity ≥ required, the
//     consent scopes cover); a refusal is a TYPED error to the agent
//     ([SINK_CLEARANCE], the tool names + the reason), the ledger
//     records it (G3), and an OWNER GRANT (grant_issue, scope
//     `mcp:<tool>`, "once") unblocks the call EXACTLY ONCE.
// The session label is observable in every tools/call result `_meta`.
// Conservative posture: an upstream tool WITHOUT a policy entry passes
// through UNCLASSIFIED (the №793 posture; the pilot is opt-in) — the
// startup carries the loud note listing the unpoliced tools.
//
// Test isolation: the grant ledger is a PROCESS-GLOBAL in-memory table —
// each test uses its OWN upstream name (agent1..agent5), so the grant
// scope `mcp:<upstream>.send_email` never collides across the scenarios.
//
// RED-before (the №772 form): these pins ran against the pre-№801
// gateway — the `mcp_policy` declaration did not parse (McpServer::new
// refused the program), so every scenario was RED before the landing.

#![allow(clippy::disallowed_methods)]

use metalogos_server::mcp_server::McpServer;
use std::path::PathBuf;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

/// The policy program for ONE upstream name: the four tools classified +
/// the owner's grant-issuing local tool (the once-grant unblocks
/// `<upstream>.send_email`).
fn policy_source(upstream: &str) -> String {
    format!(
        r#"
mcp_policy {{
  tool "{u}.read_secret" source "private";
  tool "{u}.fetch_page" source "public, untrusted";
  tool "{u}.send_email" sink "public";
  tool "{u}.run_cmd" sink "public, trusted";
}}

tool owner_{u} {{
  unlock() -> String {{
    return grant_issue("mcp:{u}.send_email", 60.0, "once")
  }}
}}
"#,
        u = upstream
    )
}

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
    response["result"]["content"][0]["text"]
        .as_str()
        .expect("text block")
        .to_string()
}

fn make_server(upstream: &str) -> McpServer {
    std::env::set_var(
        "METALOGOS_MCP_UPSTREAMS",
        format!(
            "{}=python3,{}",
            upstream,
            fixture_path("mcp_classified_server.py")
        ),
    );
    let allowlist = format!("owner_{}.unlock", upstream);
    let server = McpServer::new(
        &parse(&policy_source(upstream)),
        &[allowlist.as_str().to_string()],
    )
    .expect("gateway with the classified upstream");
    std::env::remove_var("METALOGOS_MCP_UPSTREAMS");
    server
}

fn call(server: &McpServer, name: &str, id: i32) -> serde_json::Value {
    server.handle_request(
        "tools/call",
        &serde_json::json!({"name": name, "arguments": {}}),
        &serde_json::json!(id),
    )
}

// ── 1. read Private → send outward = REFUSED ─────────────────────────

#[test]
fn n801_read_private_then_send_refused() {
    let _g = lock_env();
    let server = make_server("agent1");

    // The Source call succeeds and JOINS "private" into the session label.
    let r = call(&server, "agent1.read_secret", 1);
    assert_eq!(r["result"]["isError"], false, "read must succeed: {r}");
    assert_eq!(call_text(&r), "SECRET-42");

    // The Sink call refuses: the session carries Private, the sink
    // demands Public. The typed error names the tool, the requirement,
    // and the session label.
    let r = call(&server, "agent1.send_email", 2);
    assert_eq!(
        r["result"]["isError"], true,
        "the sink must refuse after a private read: {r}"
    );
    let text = call_text(&r);
    assert!(
        text.contains("[SINK_CLEARANCE]"),
        "the refusal carries the stable code: {text}"
    );
    assert!(
        text.contains("agent1.send_email"),
        "the refusal names the tool: {text}"
    );
    assert!(
        text.contains("private"),
        "the refusal shows the session label: {text}"
    );
}

// ── 2. fetch Untrusted → exec = REFUSED ──────────────────────────────

#[test]
fn n801_fetch_untrusted_then_exec_refused() {
    let _g = lock_env();
    let server = make_server("agent2");

    let r = call(&server, "agent2.fetch_page", 1);
    assert_eq!(r["result"]["isError"], false, "fetch must succeed: {r}");

    // "public, trusted" sink vs the session "public, untrusted":
    // the INTEGRITY dimension refuses (untrusted < trusted).
    let r = call(&server, "agent2.run_cmd", 2);
    assert_eq!(
        r["result"]["isError"], true,
        "the trusted sink must refuse after an untrusted fetch: {r}"
    );
    let text = call_text(&r);
    assert!(
        text.contains("[SINK_CLEARANCE]"),
        "the refusal carries the stable code: {text}"
    );
    assert!(
        text.contains("untrusted"),
        "the refusal shows the session integrity: {text}"
    );
}

// ── 3. Public first → send passes ────────────────────────────────────

#[test]
fn n801_public_first_send_passes() {
    let _g = lock_env();
    let server = make_server("agent3");

    // A fresh session sits at the bottom (public, trusted) — the send
    // passes BEFORE any Source call raised the label.
    let r = call(&server, "agent3.send_email", 1);
    assert_eq!(
        r["result"]["isError"], false,
        "a fresh session may send: {r}"
    );
    assert_eq!(call_text(&r), "sent");
}

// ── 4. The owner grant unblocks EXACTLY ONCE ─────────────────────────

#[test]
fn n801_grant_unblocks_once() {
    let _g = lock_env();
    let server = make_server("agent4");

    // Raise the label first — a fresh session passes a public sink
    // WITHOUT any grant (the №793 posture); the grant only matters once
    // the label refuses.
    let r = call(&server, "agent4.read_secret", 1);
    assert_eq!(r["result"]["isError"], false, "read must succeed: {r}");

    // The owner's grant: issued through the LOCAL tool (grant_issue,
    // scope `mcp:agent4.send_email`, class once).
    let r = call(&server, "owner_agent4.unlock", 2);
    assert_eq!(r["result"]["isError"], false, "the grant must issue: {r}");

    // The FIRST send: the label refuses, the grant covers it (consumed).
    let r = call(&server, "agent4.send_email", 3);
    assert_eq!(
        r["result"]["isError"], false,
        "the once-grant unblocks the first send: {r}"
    );
    assert_eq!(call_text(&r), "sent");

    // The SECOND send: the grant is consumed — the label still refuses.
    let r = call(&server, "agent4.send_email", 4);
    assert_eq!(
        r["result"]["isError"], true,
        "the consumed once-grant must not unblock again: {r}"
    );
    let text = call_text(&r);
    assert!(
        text.contains("[SINK_CLEARANCE]"),
        "the second refusal carries the code: {text}"
    );
}

// ── 5. The session label is observable in _meta ──────────────────────

#[test]
fn n801_session_label_visible_in_meta() {
    let _g = lock_env();
    let server = make_server("agent5");

    let r = call(&server, "agent5.read_secret", 1);
    assert_eq!(r["result"]["isError"], false, "read must succeed: {r}");
    let meta = &r["result"]["_meta"];
    let label = meta["metalogos.dev/session-label"]
        .as_str()
        .expect("the session label must ride the result _meta");
    assert!(
        label.contains("private"),
        "the label must reflect the private join, got: {label}"
    );
}

// ── 6. mlog check validates the policy (the unknown class refuses) ───

#[test]
fn n801_check_refuses_unknown_class_word() {
    let bad = r#"
mcp_policy {
  tool "agent.x" source "classified";
}
"#;
    let result = metalogos::check_program(bad).expect("parses");
    assert!(!result.is_ok(), "an unknown label word must refuse");
    let hit = result
        .errors
        .iter()
        .any(|e| e.message.contains("MCP_POLICY"));
    assert!(
        hit,
        "the refusal names the policy class: {:?}",
        result.errors
    );
}

#[test]
fn n801_check_refuses_bare_local_tool_entry() {
    // The policy entries classify UPSTREAM tools (the `upstream.tool`
    // form); a bare name collides with the local-tool namespace whose
    // policy compiles from the body (№394) — the check refuses the
    // ambiguity.
    let bad = r#"
mcp_policy {
  tool "local_thing" source "private";
}
"#;
    let result = metalogos::check_program(bad).expect("parses");
    assert!(!result.is_ok(), "a bare tool name must refuse");
    let hit = result
        .errors
        .iter()
        .any(|e| e.message.contains("MCP_POLICY"));
    assert!(
        hit,
        "the refusal names the policy class: {:?}",
        result.errors
    );
}
