// ── Naryad №828 (issue #1619, Wave 68 P1 — the gateway G5) ───────────
// The digest-pinning of the upstream tool schemas: at EVERY client-facing
// `tools/list` the gateway re-pins the session (SHA-256 over the
// deterministic name + description + inputSchema + `_meta` representation);
// at EVERY upstream `tools/call` the CURRENT representation is re-listed
// from the live upstream and compared against the pin. A divergence is a
// fail-closed refusal + a `mcp.call.denied` ledger record (the reason class
// «schema digest mismatch», the G3 form); a legitimate re-listing restores
// the path. A first use without a listing establishes the baseline LOUDLY
// (the `mcp.pin` ledger record) — the pin only narrows, the allowlist stays
// the only publishing control (the №297 posture).
//
// RED-before (форма №772): on the pre-№828 revision there was no pin store
// — the rug-pull (the schema changed between the listing and the call)
// forwarded silently, the negative-control test below fails there with a
// `gadget-ok` answer instead of the refusal.
//
// The mutating fixture (tests/fixtures/mcp_mutating_server.py) serves the
// gadget description «v1» on its FIRST tools/list and «v2» on every
// subsequent one — the deterministic rug-pull the two negative controls
// ride.

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

fn gateway_with_mutating_upstream() -> McpServer {
    std::env::set_var(
        "METALOGOS_MCP_UPSTREAMS",
        format!("mut=python3,{}", fixture_path("mcp_mutating_server.py")),
    );
    let server = McpServer::new(&parse(LOCAL_TOOL_SOURCE), &["local_api.send".to_string()])
        .expect("gateway with the mutating upstream");
    std::env::remove_var("METALOGOS_MCP_UPSTREAMS");
    server
}

fn call(server: &McpServer, name: &str, id: i64) -> serde_json::Value {
    server.handle_request(
        "tools/call",
        &serde_json::json!({"name": name, "arguments": {}}),
        &serde_json::json!(id),
    )
}

fn call_text(response: &serde_json::Value) -> String {
    response["result"]["content"][0]["text"]
        .as_str()
        .expect("text block")
        .to_string()
}

fn mcp_records() -> Vec<(String, String)> {
    metalogos::ledger::all_records()
        .expect("the in-process ledger")
        .iter()
        .filter(|r| {
            r.action == "mcp.call" || r.action == "mcp.call.denied" || r.action == "mcp.pin"
        })
        .map(|r| (r.action.clone(), r.scope.clone()))
        .collect()
}

/// ── Negative control (а): the rug-pull is REFUSED ──
/// The listing pins v1; the schema mutates; the call re-lists, sees v2 —
/// the identical call is refused with the typed class and the ledger
/// carries the `mcp.call.denied` record for the tool (the G3 form).
#[test]
#[allow(clippy::disallowed_methods)] // the №794 ledger-harness form
fn n828_schema_substitution_between_listing_and_call_refused() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var(
        "METALOGOS_LEDGER_KEY",
        "3f1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f7789aabbccddeeff12",
    );
    let before = mcp_records().len();
    let server = gateway_with_mutating_upstream();

    // The client-facing listing: pins v1 (the fixture's FIRST listing).
    let list = server.handle_request("tools/list", &serde_json::json!({}), &serde_json::json!(1));
    let names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(
        names.contains(&"mut.gadget"),
        "the listing must carry the upstream tool, got: {:?}",
        names
    );

    // The identical call AFTER the schema mutated: the call-time re-list
    // sees v2 — the rug-pull is refused, never forwarded.
    let r = call(&server, "mut.gadget", 2);
    assert_eq!(
        r["result"]["isError"], true,
        "the rug-pull must refuse: {}",
        r
    );
    let text = call_text(&r);
    assert!(
        text.contains("[SCHEMA_DIGEST_MISMATCH]"),
        "the refusal carries the typed class: {}",
        text
    );
    assert!(
        text.contains("schema digest mismatch")
            && text.contains("pinned")
            && text.contains("current"),
        "the refusal names the pinned and the current digests: {}",
        text
    );

    // The ledger: the denial record for the tool (the G3 form) + the pin
    // records are visible in the tape (the observability clause).
    let after = mcp_records();
    let session = &after[before..];
    assert!(
        session
            .iter()
            .any(|(action, scope)| action == "mcp.call.denied" && scope == "mut.gadget"),
        "the denied record for mut.gadget must ride the ledger: {:?}",
        session
    );
    assert!(
        session
            .iter()
            .any(|(action, scope)| action == "mcp.pin" && scope == "mut.gadget"),
        "the pin record for mut.gadget must be visible in the tape: {:?}",
        session
    );
    std::env::remove_var("METALOGOS_LEDGER_KEY");
}

/// ── Negative control (б): the legitimate re-pin restores the path ──
/// After the refusal, an explicit client listing re-pins the session to
/// the current representation — the IDENTICAL call now passes.
#[test]
#[allow(clippy::disallowed_methods)] // the №794 ledger-harness form
fn n828_repin_restores_the_path() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var(
        "METALOGOS_LEDGER_KEY",
        "3f1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f7789aabbccddeeff12",
    );
    let server = gateway_with_mutating_upstream();

    // Listing #1 pins v1; the call's re-list sees v2 — refused.
    let _ = server.handle_request("tools/list", &serde_json::json!({}), &serde_json::json!(1));
    let r = call(&server, "mut.gadget", 2);
    assert_eq!(
        r["result"]["isError"], true,
        "the pre-repin call must refuse: {}",
        r
    );

    // The explicit client listing: re-pins to v2 (the fixture's third
    // listing still answers v2 — the mutation is one-way).
    let _ = server.handle_request("tools/list", &serde_json::json!({}), &serde_json::json!(3));
    let r = call(&server, "mut.gadget", 4);
    assert_eq!(
        r["result"]["isError"], false,
        "the identical call must PASS after the legitimate re-pin: {}",
        r
    );
    assert_eq!(call_text(&r), "gadget-ok", "{}", r);

    // And the SECOND call rides the same pin — the verification is on
    // every call, the stable schema passes repeatedly.
    let r = call(&server, "mut.gadget", 5);
    assert_eq!(r["result"]["isError"], false, "{}", r);
    assert_eq!(call_text(&r), "gadget-ok");
    std::env::remove_var("METALOGOS_LEDGER_KEY");
}

/// ── The first use WITHOUT a listing establishes the baseline LOUDLY ──
/// The gateway session contract (G1: call without a prior listing) stays
/// usable; the baseline is recorded (`mcp.pin` in the tape + stderr) and
/// every later call verifies against it. The pin only NARROWS.
#[test]
#[allow(clippy::disallowed_methods)] // the №794 ledger-harness form
fn n828_first_use_without_listing_pins_loudly() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var(
        "METALOGOS_LEDGER_KEY",
        "3f1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f7789aabbccddeeff12",
    );
    let before = mcp_records().len();
    let server = gateway_with_mutating_upstream();

    // The DIRECT call (no prior listing): the call-time re-list IS the
    // first upstream listing (v1) — the baseline pins v1 and the call
    // forwards.
    let r = call(&server, "mut.gadget", 1);
    assert_eq!(
        r["result"]["isError"], false,
        "the first use must establish the baseline and forward: {}",
        r
    );
    assert_eq!(call_text(&r), "gadget-ok");

    // The baseline is in the tape.
    let session = &mcp_records()[before..];
    assert!(
        session
            .iter()
            .any(|(action, scope)| action == "mcp.pin" && scope == "mut.gadget"),
        "the first-use pin must ride the ledger: {:?}",
        session
    );

    // The SECOND call's re-list sees v2 — the pin from the first call now
    // refuses the rug-pull (the baseline has teeth).
    let r = call(&server, "mut.gadget", 2);
    assert_eq!(
        r["result"]["isError"], true,
        "the next call after the mutation must refuse: {}",
        r
    );
    assert!(call_text(&r).contains("[SCHEMA_DIGEST_MISMATCH]"), "{}", r);
    std::env::remove_var("METALOGOS_LEDGER_KEY");
}

/// ── The stable schema session: pin once, call repeatedly ──
/// The stateful fixture (a stable schema): the listing pins, three calls
/// pass — the G5 verification adds no friction to the honest session and
/// the ONE-process state contract (0, 1, 2) is untouched.
#[test]
fn n828_stable_schema_session_passes_repeatedly() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var(
        "METALOGOS_MCP_UPSTREAMS",
        format!(
            "stateful=python3,{}",
            fixture_path("mcp_stateful_server.py")
        ),
    );
    let server = McpServer::new(&parse(LOCAL_TOOL_SOURCE), &["local_api.send".to_string()])
        .expect("gateway with the stateful upstream");
    std::env::remove_var("METALOGOS_MCP_UPSTREAMS");

    let _ = server.handle_request("tools/list", &serde_json::json!({}), &serde_json::json!(1));
    let mut answers = Vec::new();
    for i in 0..3 {
        let r = call(&server, "stateful.count", 2 + i);
        assert_eq!(r["result"]["isError"], false, "{}", r);
        answers.push(call_text(&r));
    }
    assert_eq!(
        answers,
        vec!["0", "1", "2"],
        "the honest session passes through the G5 verification unchanged"
    );
}
