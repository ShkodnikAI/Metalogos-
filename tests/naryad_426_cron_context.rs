// ── Naryad #426 (P1, vm/cron): the tick context — the program-context
//    semantics of the cron dispatch (ADR-0175) ─────────────────────────
//
// The №423 bring-up defects, each closed and pinned here against the
// SAME executor the scheduler uses (the public test surfaces —
// test_tick_call / test_tick_self_call):
//   X1  the tick binds the program's db{} — a tick INSERT lands in the
//       same database a route context sees (defect 1);
//   X2  an HTTP self-call from a tick lands on a LIVE server (defect 2
//       — the transport failure was the async-context blocking call);
//   X3  sqlite::memory: is UNIFIED — routes and ticks share the startup
//       connection, not per-context isolated DBs (defect 3);
//   X4  the schema-as-code DDL replays into every context (defect 4) —
//       the tick queries a schema-declared table with NO manual CREATE,
//       and the declaration order (schema before db) does not matter.

use metalogos::interpreter::Value;
use std::sync::Mutex;

/// The env-var mutex (the tests/naryad_261_ssrf_pack.rs posture): the
/// SSRF kill-switch is read dynamically per call, so the loopback
/// self-call test holds the lock for its whole body.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn s(v: &str) -> Value {
    Value::String(v.to_string())
}

// ── X1 + X4: the tick sees db{} AND the schema DDL (file DB) ──────────

const FILE_APP: &str = r#"
db { url: "sqlite:n426_tick_ctx.db" }
schema n426 {
  table tick_rows {
    id: Int primary_key auto_increment,
    label: String
  }
}
pattern TickWrite(_payload: String) -> String {
  let n = db_execute("INSERT INTO tick_rows (label) VALUES ('from-tick')")
  return "inserted:" + n
}
pattern TickRead(_payload: String) -> String {
  let rows = query("SELECT label FROM tick_rows")
  return "rows:" + to_string(len(rows))
}
mlogserver { port: 0 }
"#;

#[test]
fn tick_binds_db_and_replays_the_schema_ddl() {
    // A fresh file (no pre-created tables — the DDL replay is the ONLY
    // way this schema exists).
    let _ = std::fs::remove_file("n426_tick_ctx.db");
    let _ = std::fs::remove_file("n426_tick_ctx.db-wal");
    let _ = std::fs::remove_file("n426_tick_ctx.db-shm");

    // The tick WRITES through the program context (db bound + schema
    // replayed — the INSERT proves both: no table would refuse).
    let out = tokio_block_on(metalogos::server::test_tick_call(
        FILE_APP,
        "TickWrite",
        vec![s("p")],
    ))
    .expect("the tick insert succeeds (db bound, DDL replayed)");
    match out {
        Value::String(v) => assert!(v.starts_with("inserted:"), "got: {v}"),
        other => panic!("expected String, got {}", other.type_name()),
    }

    // A SECOND tick (a fresh context, a fresh connection) reads the row
    // back — the same database, not a per-context island.
    let out2 = tokio_block_on(metalogos::server::test_tick_call(
        FILE_APP,
        "TickRead",
        vec![s("p")],
    ))
    .expect("the tick read succeeds");
    match out2 {
        Value::String(v) => assert_eq!(v, "rows:1", "cross-tick visibility: {v}"),
        other => panic!("expected String, got {}", other.type_name()),
    }

    // Route-context parity: the same row is visible through a fresh
    // route-style interpreter (the run_program path reconnects the same
    // file) — the tick and the routes share the store.
    let route_src = FILE_APP.replace(
        "pattern TickRead(_payload: String) -> String {\n  let rows = query(\"SELECT label FROM tick_rows\")\n  return \"rows:\" + to_string(len(rows))\n}",
        "pattern TickRead(_payload: String) -> String {\n  let rows = query(\"SELECT label FROM tick_rows\")\n  return \"rows:\" + to_string(len(rows))\n}\nflow Main { input: String = \"\" -> TickRead -> output }",
    );
    let res = metalogos::run_program(&route_src).expect("the route-path read succeeds");
    let printed = res.unwrap_or_default();
    assert!(
        printed.contains("rows:1"),
        "the route context sees the tick's row: {printed:?}"
    );

    let _ = std::fs::remove_file("n426_tick_ctx.db");
    let _ = std::fs::remove_file("n426_tick_ctx.db-wal");
    let _ = std::fs::remove_file("n426_tick_ctx.db-shm");
}

// ── X3: sqlite::memory: is unified (the same DB, not per-context) ─────

const MEM_APP: &str = r#"
db { url: "sqlite::memory:" }
schema mem426 {
  table kv {
    id: Int primary_key auto_increment,
    k: String
  }
}
pattern MemWrite(_payload: String) -> String {
  let n = db_execute("INSERT INTO kv (k) VALUES ('shared')")
  return "inserted:" + n
}
pattern MemRead(_payload: String) -> String {
  let rows = query("SELECT k FROM kv")
  return "rows:" + to_string(len(rows))
}
mlogserver { port: 0 }
"#;

#[test]
fn memory_db_is_unified_across_tick_contexts() {
    // The №423 defect 3 workaround was file-URL-per-context; the
    // unified semantics: the startup connection is Arc-shared — a write
    // in one tick context is visible in ANOTHER tick context of the
    // SAME serve boot (two test_tick_call calls would be two boots —
    // the sequence surface is the one-boot contract).
    let results = tokio_block_on(metalogos::server::test_tick_sequence(
        MEM_APP,
        vec![
            ("MemWrite".to_string(), vec![s("p")]),
            ("MemRead".to_string(), vec![s("p")]),
        ],
    ));
    assert_eq!(results.len(), 2);
    match &results[0] {
        Ok(Value::String(v)) => assert!(v.starts_with("inserted:"), "got: {v}"),
        other => panic!("the memory insert failed: {other:?}"),
    }
    match &results[1] {
        Ok(Value::String(v)) => assert_eq!(v, "rows:1", "the memory DB is shared, not per-context: {v}"),
        other => panic!("the memory read failed: {other:?}"),
    }
}

// ── X4b: the declaration order does not matter (schema BEFORE db) ─────

const SCHEMA_FIRST_APP: &str = r#"
schema first426 {
  table early {
    id: Int primary_key auto_increment
  }
}
db { url: "sqlite::memory:" }
pattern EarlyRead(_payload: String) -> String {
  let rows = query("SELECT id FROM early")
  return "rows:" + to_string(len(rows))
}
mlogserver { port: 0 }
"#;

#[test]
fn schema_before_db_still_reaches_the_context() {
    // The startup declaration isolation used to lose this DDL silently
    // (the schema{} throwaway had no connection and the merge loop
    // swallowed the error). The replay discipline makes the ORDER
    // irrelevant.
    let out = tokio_block_on(metalogos::server::test_tick_call(
        SCHEMA_FIRST_APP,
        "EarlyRead",
        vec![s("p")],
    ))
    .expect("the early-declared table exists in the tick context");
    match out {
        Value::String(v) => assert_eq!(v, "rows:0", "got: {v}"),
        other => panic!("expected String, got {}", other.type_name()),
    }
}

// ── X2: the HTTP self-call from a tick lands on a live server ─────────

const SELF_CALL_APP: &str = r#"
pattern TickSelfCall(url: String) -> String {
  let resp = http_get(url)
  return "selfcall:" + resp
}
mlogserver {
  port: 0
  route "/ping426" method=GET {
    respond("200", "pong")
  }
}
"#;

#[test]
fn http_self_call_from_a_tick_reaches_the_live_server() {
    // The №423 defect 2: a blocking outbound call from the scheduler
    // tick failed at the transport level. The unified executor runs the
    // tick on spawn_blocking while the scheduler holds no lock — the
    // self-call loops back into the LIVE server. The loopback target
    // needs the SSRF kill-switch (№130; the env is read per call — the
    // mutex discipline makes the set/restore race-free).
    let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let prev = std::env::var("METALOGOS_HTTP_ALLOW_PRIVATE").ok();
    std::env::set_var("METALOGOS_HTTP_ALLOW_PRIVATE", "1");
    let out = tokio_block_on(metalogos::server::test_tick_self_call(
        SELF_CALL_APP,
        "TickSelfCall",
        "/ping426",
    ));
    match prev {
        Some(v) => std::env::set_var("METALOGOS_HTTP_ALLOW_PRIVATE", v),
        None => std::env::remove_var("METALOGOS_HTTP_ALLOW_PRIVATE"),
    }
    let out = out.expect("the self-call succeeds");
    match out {
        Value::String(v) => assert!(
            v.contains("pong"),
            "the tick received the route's answer: {v}"
        ),
        other => panic!("expected String, got {}", other.type_name()),
    }
}

// ── The harness ────────────────────────────────────────────────────────

/// The integration-test tokio entry (each test owns a small runtime —
/// the server surfaces are async; the bodies are blocking-safe).
fn tokio_block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
        .block_on(fut)
}
