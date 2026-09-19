//! Наряд №399 (gh#522, P0 security): on_deny handlers survive the serve
//! startup per-declaration merge.
//!
//! Bug gh#520 (opened by the №395 dogfood, item (д)): `clone_definitions_into`
//! copied `deny_handlers` by OVERWRITE while server startup merges one
//! declaration at a time — every non-OnDeny declaration carried an empty
//! handler list and clobbered the handlers accumulated from earlier merges
//! (same failure class as the №381 db_conn clobber). In live `mlog serve`
//! a route hitting a runtime gate (GRANT_EXHAUSTED from a №390 grant)
//! failed LOUD ("Handler error: GRANT_EXHAUSTED") instead of degrading
//! through the declared on_deny(db) handler (№392 contract: fire_on_deny
//! → handled → Ok(Unit)).
//!
//! Red → green contract:
//!   * gh#520 repro on the TW backend (the office default): on_deny FIRST,
//!     then other declarations, then mlogserver — the route answers
//!     200 "b=Unit", not 500 "Handler error".
//!   * VM parity pin: the same fixture on the VM backend (the compiler
//!     lifts handlers from the full program — pin that the route path
//!     keeps degrading there too).
//!   * Non-serve pin (№392 semantics unchanged): the full pre-pass
//!     registers the handler exactly once, and the flow path of the same
//!     fixture degrades identically on both TW and VM.
//!
//! Verify: cargo test --test naryad_399_serve_on_deny

#![cfg(feature = "server")]

use metalogos::server::{run_test_server_with_backend, ServeBackend};

/// Serialize real-socket servers across the test binary (one port per run).
static SERVER_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// The gh#520 minimal repro (declaration order preserved verbatim):
/// on_deny(db) FIRST — the startup merge then runs db {}, the pattern and
/// the mlogserver merge AFTER it, each clobbering the handler list under
/// the old overwrite semantics.
const GH520_REPRO: &str = r#"
on_deny(db) { print("deny caught") }
db { url: "sqlite::memory:" }
pattern GrantDenyFlow(_tick: String) -> String {
  db_execute("CREATE TABLE IF NOT EXISTS t (id INTEGER PRIMARY KEY)")
  let g = grant_issue("db:delete:t", 60, "n", 1)
  let a = db_execute_with_grant(g, "DELETE FROM t")
  let b = db_execute_with_grant(g, "DELETE FROM t")
  return "b=" + type_of(b)
}
mlogserver {
  port: 18080
  route "/go" method=GET { return respond("200", GrantDenyFlow("x")) }
}
"#;

/// The same irreversible-deny logic without serve (flow path) — pins that
/// the fix does not change the №392 full pre-pass behavior.
const FLOW_REPRO: &str = r#"
on_deny(db) { print("deny caught") }
db { url: "sqlite::memory:" }
pattern GrantDenyFlow(_tick: String) -> String {
  db_execute("CREATE TABLE IF NOT EXISTS t (id INTEGER PRIMARY KEY)")
  let g = grant_issue("db:delete:t", 60, "n", 1)
  let a = db_execute_with_grant(g, "DELETE FROM t")
  let b = db_execute_with_grant(g, "DELETE FROM t")
  return "b=" + type_of(b)
}
flow Main { input: String = "x" -> GrantDenyFlow -> output }
"#;

async fn http_get(port: u16, path: &str) -> (u16, String) {
    let resp = reqwest::get(format!("http://127.0.0.1:{}{}", port, path))
        .await
        .expect("request must not fail at transport level");
    let status = resp.status().as_u16();
    let body = resp.text().await.expect("response body");
    (status, body)
}

// ── (4) gh#520 repro — live serve, TW backend (the office default) ─────

#[tokio::test]
async fn n399_gh520_repro_grant_exhausted_degrades_in_serve() {
    let _guard = SERVER_LOCK.lock().await;
    let (port, handle) = run_test_server_with_backend(GH520_REPRO, ServeBackend::Interpreter)
        .await
        .expect("test server must start");
    let (status, body) = http_get(port, "/go").await;
    handle.abort();
    assert_eq!(
        status, 200,
        "a typed DenyEvent must degrade through the declared on_deny(db), \
         not surface as a loud route error — body: {}",
        body
    );
    assert_eq!(
        body, "b=Unit",
        "the refused granted call must degrade to Unit (№392 contract)"
    );
}

// ── (5) VM parity pin — same fixture, VM backend ───────────────────────

#[tokio::test]
async fn n399_vm_backend_parity_on_repro_fixture() {
    let _guard = SERVER_LOCK.lock().await;
    let (port, handle) = run_test_server_with_backend(GH520_REPRO, ServeBackend::Vm)
        .await
        .expect("test server must start");
    let (status, body) = http_get(port, "/go").await;
    handle.abort();
    assert_eq!(
        status, 200,
        "VM route path must keep degrading through on_deny — body: {}",
        body
    );
    assert_eq!(body, "b=Unit", "TW↔VM parity on the repro fixture");
}

// ── (5) non-serve pins — full pre-pass, handlers exactly once ──────────

#[test]
fn n399_full_prepass_registers_handler_exactly_once() {
    let program =
        metalogos::compile_program(GH520_REPRO).expect("the VM must compile the repro fixture");
    assert_eq!(
        program.deny_handlers.len(),
        1,
        "the compiler pre-pass must register the on_deny(db) handler exactly once"
    );
}

#[test]
fn n399_flow_path_degrades_identically_on_both_backends() {
    let tw = metalogos::run_program(FLOW_REPRO)
        .expect("TW flow run must succeed")
        .unwrap_or_default();
    assert_eq!(
        tw.trim_end(),
        "b=Unit",
        "TW flow path degrades (№392 semantics)"
    );
    let program = metalogos::compile_program(FLOW_REPRO).expect("VM compile");
    let vm = metalogos::run_bytecode(program)
        .expect("VM flow run must succeed")
        .unwrap_or_default();
    assert_eq!(vm.trim_end(), "b=Unit", "TW↔VM parity outside serve");
}
