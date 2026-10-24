// ── Naryad №794 (issue #1532, Wave 64 P1 — the gateway G3) ───────────
//
// The ledger entry on EVERY tools/call: allowed (`mcp.call`) or
// refused (`mcp.call.denied`), the tool in the scope, the
// reason/outcome in the detail (hash-only in the journal — the ledger
// must not become a leak channel). The session contract from the
// naryad: a 5-call session (3 allowed, 2 refused) leaves 5 signed
// records; `mlog ledger verify --expect-head` is GREEN on the export;
// a tampered record turns the verify RED.
//
// RED-before (форма №772): on the pre-№794 revision the gateway wrote
// NO ledger records — the mcp.* subset of the export was empty (the
// count pin fails) and the actions were absent.

use metalogos_server::mcp_server::McpServer;
use std::path::PathBuf;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

const TOOL_SOURCE: &str = r#"
tool calc {
  double(x: Float) -> String {
    return to_string(x * 2.0)
  }
  boom() -> String {
    return to_float("not-a-number")
  }
}
"#;

fn parse(src: &str) -> Vec<metalogos::ast::Declaration> {
    metalogos::parser::parse(src).expect("parses")
}

fn call(server: &McpServer, name: &str, args: serde_json::Value) -> serde_json::Value {
    server.handle_request(
        "tools/call",
        &serde_json::json!({"name": name, "arguments": args}),
        &serde_json::json!(1),
    )
}

/// The 5-call session contract: 3 allowed (an ok body, a failing body,
/// an unknown upstream prefix — allowed attempts with outcomes) hmm —
/// the honest split per the naryad: ALLOWED = executed (ok or body
/// error), DENIED = the gateway refused (unknown tool, not in
/// allowlist). The session: 2 ok calls + 1 body-error call (allowed) +
/// 2 refusals (denied) = 5 records, 3 `mcp.call` + 2 `mcp.call.denied`.
// №475 ratchet: the test-harness file writes (the export + the tampered
// copy + the temp dir) are the justified harness form — the
// #[allow] + the reason, mirroring the naryad_394 precedent.
#[test]
#[allow(clippy::disallowed_methods)] // the test harness: the ledger export + the tamper fixture
fn n794_five_call_session_leaves_five_signed_records() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    // The deterministic chain (the №393 reproducible-keys form).
    std::env::set_var(
        "METALOGOS_LEDGER_KEY",
        "3f1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f7789aabbccddeeff12",
    );

    // The stateful upstream rides the session (the proxy path's ledger
    // hook is exercised too — the r3 call below).
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("mcp_stateful_server.py");
    std::env::set_var(
        "METALOGOS_MCP_UPSTREAMS",
        format!("stateful=python3,{}", fixture.to_string_lossy()),
    );

    let server = McpServer::new(
        &parse(TOOL_SOURCE),
        &["calc.double".to_string(), "calc.boom".to_string()],
    )
    .expect("gateway builds");

    // The ledger is process-global — the DELTA snapshot BEFORE the calls.
    let before = metalogos::ledger::all_records()
        .expect("the in-process ledger")
        .iter()
        .filter(|r| r.action == "mcp.call" || r.action == "mcp.call.denied")
        .count();

    // The 5-call session: 3 allowed (ok / body error / upstream proxy) +
    // 2 denied (the gateway refusals).
    let r1 = call(&server, "calc.double", serde_json::json!({"x": 21.0})); // allowed, ok
    assert_eq!(r1["result"]["isError"], false, "{}", r1);
    let _r2 = call(&server, "calc.boom", serde_json::json!({})); // allowed, body error
    let _r3 = call(&server, "stateful.count", serde_json::json!({})); // allowed, proxy ok
    let r4 = call(&server, "unknown.tool", serde_json::json!({})); // denied
    assert!(r4["error"]["message"].is_string(), "{}", r4);
    let _r5 = call(&server, "totally_unknown", serde_json::json!({})); // denied
    std::env::remove_var("METALOGOS_MCP_UPSTREAMS");

    // The session's contract is the DELTA (see the snapshot above): exactly
    // 5 new mcp.* records, 3 allowed + 2 denied, and they are the LAST five.
    let records = metalogos::ledger::all_records().expect("the in-process ledger");
    let mcp: Vec<_> = records
        .iter()
        .filter(|r| r.action == "mcp.call" || r.action == "mcp.call.denied")
        .collect();
    let session: Vec<_> = mcp[before..].to_vec();
    assert_eq!(
        session.len(),
        5,
        "the 5-call session must leave exactly 5 NEW mcp.* records, got {}: {:?}",
        session.len(),
        session
            .iter()
            .map(|r| (&r.action, &r.scope))
            .collect::<Vec<_>>()
    );
    let allowed = session.iter().filter(|r| r.action == "mcp.call").count();
    let denied = session
        .iter()
        .filter(|r| r.action == "mcp.call.denied")
        .count();
    assert_eq!((allowed, denied), (3, 2));

    // Every record carries the signature + the chain linkage.
    for r in session {
        assert!(!r.sig.is_empty(), "the record must be signed");
        assert!(!r.hash.is_empty(), "the record must carry its hash");
        assert_eq!(r.actor, "mcp-gateway");
    }

    // Export → the CLI verify (the naryad's `mlog ledger verify` form).
    let head = metalogos::ledger::head_hash().expect("head hash");
    let jsonl = metalogos::ledger::export_jsonl().expect("export");
    let dir = std::env::temp_dir().join("n794-ledger-verify");
    let _ = std::fs::create_dir_all(&dir);
    let file = dir.join("session.jsonl");
    std::fs::write(&file, jsonl).expect("write the export");

    let out = std::process::Command::new(PathBuf::from(env!("CARGO_BIN_EXE_mlog")))
        .args([
            "ledger",
            "verify",
            file.to_str().unwrap(),
            "--expect-head",
            &head,
        ])
        .output()
        .expect("spawn mlog");
    assert!(
        out.status.success(),
        "the verify must be GREEN on the untampered export: {} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // Tamper ONE mcp record (flip a scope letter) → the verify RED.
    let content = std::fs::read_to_string(&file).expect("read back");
    let tampered: String = content
        .lines()
        .map(|line| {
            if line.contains("\"mcp.call\"") || line.contains("mcp.call.denied") {
                line.replace("calc", "calg")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert_ne!(content, tampered, "the tamper must change a line");
    let tampered_file = dir.join("tampered.jsonl");
    std::fs::write(&tampered_file, tampered).expect("write the tampered");

    let out = std::process::Command::new(PathBuf::from(env!("CARGO_BIN_EXE_mlog")))
        .args(["ledger", "verify", tampered_file.to_str().unwrap()])
        .output()
        .expect("spawn mlog");
    assert!(
        !out.status.success(),
        "the verify must be RED on a tampered record: {}",
        String::from_utf8_lossy(&out.stdout)
    );

    std::env::remove_var("METALOGOS_LEDGER_KEY");
}

/// The args never ride the journal in the open form: the mcp records'
/// stored fields are hashes and metadata only.
#[test]
fn n794_ledger_stores_no_open_arguments() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var(
        "METALOGOS_LEDGER_KEY",
        "3f1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f7789aabbccddeeff12",
    );
    let server =
        McpServer::new(&parse(TOOL_SOURCE), &["calc.double".to_string()]).expect("gateway builds");
    let _r = call(
        &server,
        "calc.double",
        serde_json::json!({"x": 1234.5, "secret_marker": "LEAKME-794"}),
    );
    let records = metalogos::ledger::all_records().expect("the in-process ledger");
    let last = records
        .iter()
        .rev()
        .find(|r| r.action == "mcp.call")
        .expect("the allowed record");
    let serialized = serde_json::to_string(last).expect("serialize");
    assert!(
        !serialized.contains("LEAKME-794"),
        "the arguments must never ride the journal in the open form"
    );
    assert!(
        !serialized.contains("secret_marker"),
        "even the argument NAMES stay out of the journal"
    );
    assert_eq!(last.scope, "calc.double");
    std::env::remove_var("METALOGOS_LEDGER_KEY");
}
