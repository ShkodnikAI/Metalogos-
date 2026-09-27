// ── Naryad #484 (gh#732): the DbAccess trait + the FIRST unified pair ──
//
// The second dedup metric (ops_pair_counter.py) counts the *_tw/*_vm
// LOGIC pairs; this naryad unified the first one: `query_scalar` — the
// ONE suffix-free implementation over the `DbAccess` state trait.
//
// Pins:
// 1. `Vm: DbAccess` — the VM lane rides the trait (a static bound + a
//    live call through the real Vm);
// 2. `TwDbAccess` — the interpreter's `Mutex<Option<Connection>>` locks
//    through the adapter and drives the SAME unified function (the
//    shapes: Float, Unit on an empty result, the typed №381 bind);
// 3. the unified error texts carry the RICHER form on each axis: the
//    SQL-argument type detail (the former TW text) and the №758 named
//    remedy (the former VM text) — on BOTH lanes now;
// 4. end-to-end parity: the same query_scalar program returns the same
//    output on TW (run_program) and VM (run_bytecode).

use metalogos::db_ops::{DbAccess, TwDbAccess};
use metalogos::interpreter::Value;
use std::sync::Mutex;

fn s(v: &str) -> Value {
    Value::String(v.to_string())
}

/// Value carries no PartialEq — the db_ops discipline: compare Debug shapes.
fn val_shape(v: &Value) -> String {
    format!("{:?}", v)
}

/// A static witness that the VM implements the state trait (the rename
/// VmDbAccess → DbAccess is not a paper move: the production type is
/// bound right here).
#[test]
fn n484_vm_implements_db_access() {
    fn assert_db_access<T: DbAccess>(_: &T) {}
    let mut vm = metalogos::vm::Vm::new();
    assert_db_access(&vm);
}

#[test]
fn n484_tw_adapter_drives_the_unified_query_scalar() {
    let db: Mutex<Option<rusqlite::Connection>> = Mutex::new(None);
    {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute("CREATE TABLE t (a TEXT, b REAL)", []).unwrap();
        conn.execute("INSERT INTO t VALUES ('a', 7.0)", []).unwrap();
        *db.lock().unwrap() = Some(conn);
    }
    let mut tw = TwDbAccess::lock(&db).unwrap();

    // The plain shape.
    let out = metalogos::db_ops::query_scalar(&mut tw, &[s("SELECT b FROM t LIMIT 1")]).unwrap();
    assert_eq!(val_shape(&out), val_shape(&Value::Float(7.0)));

    // The empty result → Unit (the shared contract).
    let out =
        metalogos::db_ops::query_scalar(&mut tw, &[s("SELECT b FROM t WHERE a='zz'")]).unwrap();
    assert_eq!(val_shape(&out), val_shape(&Value::Unit));

    // The typed №381 bind: Float → REAL, not text.
    let out = metalogos::db_ops::query_scalar(
        &mut tw,
        &[s("SELECT typeof(?)"), Value::List(vec![Value::Float(3.0)])],
    )
    .unwrap();
    assert_eq!(val_shape(&out), val_shape(&s("real")));
}

#[test]
fn n484_unified_error_texts_carry_the_richer_form() {
    let db: Mutex<Option<rusqlite::Connection>> = Mutex::new(None);
    let mut tw = TwDbAccess::lock(&db).unwrap();

    // The SQL-argument error keeps the type detail (the former TW text;
    // the VM lane gained it in the unification).
    let err = metalogos::db_ops::query_scalar(&mut tw, &[Value::Float(1.0)]).unwrap_err();
    assert!(
        err.contains("expected String SQL, got Float"),
        "the unified SQL-arg error must name the offending type: {}",
        err
    );

    // The not-open error keeps the №758 named remedy (the former VM
    // text; the TW lane gained it in the unification).
    let err = metalogos::db_ops::query_scalar(&mut tw, &[s("SELECT 1")]).unwrap_err();
    assert!(
        err.contains("query_scalar() error:"),
        "the site prefix stays: {}",
        err
    );
    assert!(
        err.contains("no database connection") && err.contains("Declare db"),
        "the named remedy travels with the unified fn: {}",
        err
    );
}

/// End-to-end: the SAME query_scalar program on both backends.
#[test]
fn n484_query_scalar_parity_end_to_end() {
    let src = "\
db { url: \"sqlite::memory:\" }
pattern Probe(_x: String) -> String {
  db_execute(\"CREATE TABLE t (a TEXT, b REAL)\")
  db_execute(\"INSERT INTO t VALUES ('a', 7.0)\")
  return query_scalar(\"SELECT b FROM t LIMIT 1\")
}
flow Main { input: String = \"x\" -> Probe -> output }";

    let tw = metalogos::run_program(src).map(|o| o.unwrap_or_default().trim_end().to_string());
    let vm = metalogos::compile_program(src)
        .and_then(metalogos::run_bytecode)
        .map(|o| o.unwrap_or_default().trim_end().to_string());

    let tw = tw.unwrap_or_else(|e| panic!("TW run must succeed: {}", e));
    let vm = vm.unwrap_or_else(|e| panic!("VM run must succeed: {}", e));
    assert_eq!(tw, "7", "TW output");
    assert_eq!(vm, "7", "VM output (parity)");
}
