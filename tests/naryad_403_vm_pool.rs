// ── Naryad #403: warm VM pool — fail-closed reset contract (HTTP-level)
//
// The pool recycles `Vm` objects across serve requests. Two properties
// are pinned here, at the HTTP surface a client actually sees:
//
//   1. REUSE IS INVISIBLE: with the pool ON and capacity 1, sequential
//      requests are observationally identical to fresh per-request VMs.
//      The canary is the №381 class (shared-DB content): each request
//      INSERTs its own row into the in-memory db, then bare-DELETEs all
//      rows under a fresh grant and reports the affected count. A fresh
//      (or correctly reset) per-request connection holds EXACTLY ONE
//      row — its own. Any state leakage from the previous request makes
//      the count 2+; a leaked query param or grant makes the response
//      shape differ. (The exhaustive FIELD-level enumeration lives in
//      src/vm.rs, mod n403_reset_tests.)
//   2. FAIL-CLOSED: a request whose route execution returns Err must
//      DISCARD its VM — the pool counters prove the discard and the
//      next request proves no stale state survives.
//
// Plus TW↔VM parity on the security fixture (grant issue → granted
// irreversible execute → refused re-use of the exhausted grant) with
// the pool ON — parity is a red test, never a "documented difference".
//
// The green source MUST NOT contain `match ` in a ROUTE body (the VM
// serve contract, naryad_160 block 1) — the on_deny handler sits at
// the top level, outside the routes.

use metalogos::server::ServeBackend;

const SOURCE_N403: &str = r#"
on_deny(db) {
  match deny_reason() {
    "IRREVERSIBLE_NO_GRANT" then { print("deny:IRREVERSIBLE_NO_GRANT") }
    else { print("deny:other") }
  }
}

db { url: "sqlite::memory:" }

mlogserver {
  port: 8094
  route "/probe" method=GET {
    let name = query_param("name")
    db_execute("CREATE TABLE IF NOT EXISTS frames (id INTEGER PRIMARY KEY, path TEXT)")
    db_execute("INSERT INTO frames (path) VALUES ($1)", [name])
    let g = grant_issue("db:delete:frames", 60, "n", 1)
    let wiped = db_execute_with_grant(g, "DELETE FROM frames", [])
    let refused = db_execute_with_grant(g, "DELETE FROM frames", [])
    let snap = ledger_snapshot()
    respond("200", "name:" + name + "|wiped:" + wiped + "|refused:" + type_of(refused) + "|snap:" + type_of(snap))
  }
  route "/poison" method=GET {
    db_execute("CREATE TABLE IF NOT EXISTS frames (id INTEGER PRIMARY KEY, path TEXT)")
    let broken = db_execute("THIS IS NOT VALID SQL AT ALL")
    respond("200", "never:" + broken)
  }
}
"#;

async fn start_pooled_vm(max: usize) -> u16 {
    let (port, _handle, _pool) =
        metalogos::server::run_test_server_with_backend_pool(SOURCE_N403, ServeBackend::Vm, max)
            .await
            .expect("pooled test server should start");
    port
}

async fn get(port: u16, path: &str) -> (u16, String) {
    let url = format!("http://127.0.0.1:{}{}", port, path);
    let resp = reqwest::get(&url).await.expect("GET should succeed");
    let status = resp.status().as_u16();
    let body = resp.text().await.expect("body readable");
    (status, body)
}

/// A fresh (or correctly reset) connection holds exactly ONE row — the
/// row the CURRENT request just inserted. Any cross-request db leakage
/// makes `wiped` grow beyond 1.
async fn assert_probe_fresh(port: u16, name: &str) {
    let (st, body) = get(port, &format!("/probe?name={}", name)).await;
    assert_eq!(st, 200, "probe status for {name} (body: {body})");
    assert_eq!(
        body,
        format!("name:{name}|wiped:1|refused:Unit|snap:String"),
        "request {name} must see ONLY its own db content, its own grant state, \
         and a fresh ledger surface (no cross-request leakage through the pool)"
    );
}

#[tokio::test]
async fn n403_pool_reuse_is_invisible_and_fail_closed() {
    let port = start_pooled_vm(1).await;

    // Request 1: cold VM.
    assert_probe_fresh(port, "alice").await;
    // Request 2/3: the SAME pooled VM, reset between requests. The db
    // content, grants, ledger surface and query context must all be
    // indistinguishable from a fresh VM.
    assert_probe_fresh(port, "bob").await;
    assert_probe_fresh(port, "carol").await;

    // Fail-closed: a route execution that returns Err (invalid SQL →
    // runtime error → 500) must DISCARD its VM.
    let (st, _body) = get(port, "/poison").await;
    assert_eq!(st, 500, "the poison route must fail at runtime");

    // The discarded VM is not reused: the next probe is a cold build
    // and sees exactly its own state (never the poisoned VM's residue).
    assert_probe_fresh(port, "dave").await;
}

#[tokio::test]
async fn n403_pool_counters_show_reuse_and_discard() {
    let (port, _handle, pool) =
        metalogos::server::run_test_server_with_backend_pool(SOURCE_N403, ServeBackend::Vm, 1)
            .await
            .expect("pooled test server should start");

    assert_probe_fresh(port, "r1").await;
    let s1 = pool.stats();
    assert_eq!(s1.cold_created, 1, "first request must be a cold build");
    assert_eq!(s1.reuses, 0);
    assert_eq!(s1.idle_now, 1, "a successful request must be checked in");

    assert_probe_fresh(port, "r2").await;
    let s2 = pool.stats();
    assert_eq!(s2.reuses, 1, "the second request must REUSE the pooled VM");
    assert_eq!(s2.cold_created, 1);

    let (st, _) = get(port, "/poison").await;
    assert_eq!(st, 500);
    let s3 = pool.stats();
    assert_eq!(
        s3.discarded_error, 1,
        "the failed request's VM must be DISCARDED (fail-closed)"
    );
    assert_eq!(s3.idle_now, 0, "nothing rests in the pool after an error");

    assert_probe_fresh(port, "r3").await;
    let s4 = pool.stats();
    assert_eq!(
        s4.cold_created, 2,
        "after the discard the next request must be a fresh cold build"
    );
    assert_eq!(
        s4.discarded_reset_failed, 0,
        "no reset failures expected here"
    );
}

#[tokio::test]
async fn n403_pool_parity_with_tw_on_security_fixtures() {
    // TW: the interpreter path (never pooled — the shared-interpreter
    // reference behavior). VM: the pooled path. Identical responses are
    // the contract; a divergence is a RED test, not a documented quirk.
    let tw_port = {
        let (p, _h) =
            metalogos::server::run_test_server_with_backend(SOURCE_N403, ServeBackend::Interpreter)
                .await
                .expect("TW server");
        p
    };
    let (vm_port, _handle, _pool) =
        metalogos::server::run_test_server_with_backend_pool(SOURCE_N403, ServeBackend::Vm, 2)
            .await
            .expect("VM pooled server");

    for name in ["twvm1", "twvm2"] {
        let (st_tw, body_tw) = get(tw_port, &format!("/probe?name={}", name)).await;
        let (st_vm, body_vm) = get(vm_port, &format!("/probe?name={}", name)).await;
        assert_eq!(st_tw, 200, "TW status ({name})");
        assert_eq!(st_vm, 200, "VM(pooled) status ({name}) — body: {body_vm}");
        assert_eq!(
            body_tw, body_vm,
            "TW/VM parity on the security fixture with the pool ON ({name})"
        );
    }

    // The SECOND VM request runs on a REUSED VM — parity must hold on
    // pooled generations too (twvm2 above is already that case, pinned
    // again explicitly with a third name).
    let (st_tw, body_tw) = get(tw_port, "/probe?name=twvm3").await;
    let (st_vm, body_vm) = get(vm_port, "/probe?name=twvm3").await;
    assert_eq!(st_tw, 200);
    assert_eq!(st_vm, 200, "body: {body_vm}");
    assert_eq!(body_tw, body_vm, "pooled-generation parity");
}

#[tokio::test]
async fn n403_pool_off_unchanged_default() {
    // Pool OFF (the default): the pre-№403 per-request path must be
    // behaviorally unchanged — the same fixture, the same probes.
    let (port, _h) = metalogos::server::run_test_server_with_backend(SOURCE_N403, ServeBackend::Vm)
        .await
        .expect("VM server (no pool)");
    assert_probe_fresh(port, "off1").await;
    assert_probe_fresh(port, "off2").await;
    let (st, _b) = get(port, "/poison").await;
    assert_eq!(st, 500);
    assert_probe_fresh(port, "off3").await;
}
