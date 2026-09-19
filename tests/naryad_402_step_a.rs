// ── Naryad #402 (step A): the Arc-shared program state pin ────────────
//
// The step-A change (Vm installs the program's immutable collections as
// Arc snapshots shared with `Program::shared_cache`; per-request globals
// and server context stay owned) must be BEHAVIORALLY INVISIBLE:
//
//   1. the route result is IDENTICAL across backends (TW/VM parity) and
//      identical to the pre-step-A contract (the same route shape the
//      wave-3 story exercises: destructive SQL under a grant, the
//      quota-exhaustion deny degraded by on_deny, the ledger export).
//      (The MEDIA egress segment of the wave-3 story stays covered by
//      the flow-level e2e — tests/wave3_kitchen_camera_e2e.rs; a route
//      body cannot use it on the TW backend because the per-request
//      interpreter never receives the origin declarations — a PRE-
//      EXISTING TW-serve gap, unrelated to step A, reported separately.)
//   2. per-request server context CANNOT leak: the same route hit with
//      different query params returns param-specific results — the
//      shared program state carries no request data (body/query/path are
//      injected AFTER load_program, onto the per-request Vm);
//   3. the per-request VM path still journals its own signed trail
//      (grant lifecycle + the irreversible record + the deny event) and
//      the exported chain verifies externally.
//
// The green source MUST NOT contain `match ` in a ROUTE body (the VM
// serve contract, naryad_160 block 1) — the on_deny handler sits at the
// top level, outside the routes.

use metalogos::server::ServeBackend;

const SOURCE_N402: &str = r#"
on_deny(db) {
  match deny_reason() {
    "IRREVERSIBLE_NO_GRANT" then { print("deny:IRREVERSIBLE_NO_GRANT") }
    else { print("deny:other") }
  }
}

db { url: "sqlite::memory:" }

mlogserver {
  port: 8093
  route "/archive" method=GET {
    let name = query_param("name")
    db_execute("CREATE TABLE IF NOT EXISTS frames (id INTEGER PRIMARY KEY, path TEXT)")
    db_execute("INSERT INTO frames (path) VALUES ($1)", [name])
    let g = grant_issue("db:delete:frames", 60, "n", 1)
    let n1 = db_execute_with_grant(g, "DELETE FROM frames WHERE path = $1", [name])
    let refused = db_execute_with_grant(g, "DELETE FROM frames")
    let snap = ledger_snapshot()
    let trail = ledger_export("target/n402_ledger.jsonl")
    respond("200", "deleted:" + n1 + "|refused:" + type_of(refused) + "|trail:" + type_of(trail) + "|name:" + name)
  }
}
"#;

async fn start(source: &str, backend: ServeBackend) -> u16 {
    // The JoinHandle is detached intentionally: the server task lives for
    // the whole test (the same pattern the naryad_160 harness uses).
    let (port, _handle) = metalogos::server::run_test_server_with_backend(source, backend)
        .await
        .expect("test server should start");
    port
}

async fn get(port: u16, path: &str) -> (u16, String) {
    let url = format!("http://127.0.0.1:{}{}", port, path);
    let resp = reqwest::get(&url).await.expect("GET should succeed");
    let status = resp.status().as_u16();
    let body = resp.text().await.expect("body readable");
    (status, body)
}

#[tokio::test]
async fn n402_route_identity_and_per_request_isolation() {
    let tw_port = start(SOURCE_N402, ServeBackend::Interpreter).await;
    let vm_port = start(SOURCE_N402, ServeBackend::Vm).await;

    // 1) TW/VM parity: the same request, the same observable result — the
    //    Arc-shared program state changes nothing the route can see.
    let (st_tw, body_tw) = get(tw_port, "/archive?name=alice").await;
    let (st_vm, body_vm) = get(vm_port, "/archive?name=alice").await;
    assert_eq!(st_tw, 200, "TW status (body: {body_tw})");
    assert_eq!(st_vm, 200, "VM status (body: {body_vm})");
    assert_eq!(body_tw, body_vm, "TW/VM route result identity");
    assert!(
        body_tw.starts_with("deleted:1|refused:Unit|trail:String|name:alice"),
        "the full wave-3 story runs inside a route: {body_tw}"
    );

    // 2) Per-request isolation: a different request (different query param)
    //    must see ONLY its own context — the shared program state carries
    //    no request data (the shared collections are immutable snapshots;
    //    body/query/path are injected per request AFTER load_program).
    let (st2, body2) = get(vm_port, "/archive?name=bob").await;
    assert_eq!(st2, 200, "second VM request status");
    assert!(
        body2.starts_with("deleted:1|refused:Unit|trail:String|name:bob"),
        "the second request sees its own query param, not the first's: {body2}"
    );
    assert!(
        body2.ends_with("name:bob") && body_tw.ends_with("name:alice"),
        "no cross-request leakage of the query context"
    );

    // 3) The per-request VM path journals its own signed trail; the chain
    //    (this test process's records, TW + VM runs included) verifies
    //    externally — the genesis-seq contract holds on the shared-program
    //    runtime too.
    let report = metalogos::ledger::verify_file(
        std::path::Path::new("target/n402_ledger.jsonl"),
        None,
        None,
    )
    .expect("the exported chain verifies externally");
    assert!(report.records >= 2, "the chain carries the story's events");
    let content = std::fs::read_to_string("target/n402_ledger.jsonl").expect("export readable");
    assert!(
        content.contains("grant.issued"),
        "grant lifecycle journaled"
    );
    assert!(
        content.contains("irreversible.db_execute"),
        "granted delete journaled (VM side included)"
    );
    assert!(
        content.contains("deny.IRREVERSIBLE_NO_GRANT"),
        "the quota-exhaustion deny journaled with the typed reason"
    );
}
