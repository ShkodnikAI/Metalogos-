// ── №736 (gh#1371, Wave 52): the 0.32 fixed-list movement №7 — the
// query* package after the ADR-0194 ratification ─────────────────────
//
// The dispatch premise (gh#1371): the ADR-0194 ratification (№734)
// dissolved the query* refusal anchor. The machine facts on the
// wave-52 base (a21674b) resolve the movement HONESTLY:
//
//   1. The №712/№730 DIVERGENCE cause is DEAD — the №734 landing made
//      both lanes agree per SQL class (the mutation String, the
//      SELECT List; the parity battery tests/n194_query_parity.rs).
//   2. What SURVIVES for all THREE rows is the UNION ground the
//      stage-0 vocabulary (№627: PRECISE {String,Float,Bool,Unit} +
//      the List</Struct< parameterized forms) cannot spell:
//        query        the SQL-class union rows|count —
//                     SELECT/PRAGMA → the List of Row structs
//                     (db_ops.rs:199-212 TW / :757-790 VM), mutations
//                     → the String count (db_ops.rs:212-218 TW /
//                     :744-755 VM); typing String would false-warn
//                     the PRIMARY SELECT arm (the memory_read №539
//                     partial-arm form; the ADR-0194 §2 probe record).
//                     The context-less call refuses loudly (t6).
//        query_row    the handler IS LIVE (the №26 P1-7 intercepts —
//                     TW execution.rs:2483-2490, VM vm.rs:2706-2712;
//                     the №730 "planned wrapper, no handler" anchors
//                     were factually STALE, corrected in registry.rs
//                     this PR): the HETEROGENEOUS List (Null→Unit,
//                     Integer/Real→Float, Text/Blob→String —
//                     db_ops.rs:554-564); a List<X> spelling would be
//                     fictitious (the №669 graph_path form).
//        query_scalar the handler IS LIVE (the same correction —
//                     TW execution.rs:2474-2482, VM vm.rs:2439-2442):
//                     the String|Float|Unit cell union
//                     (db_ops.rs:478-489).
//
// THIS file pins the LIVE-HANDLER facts that carry the refreshed
// refusal table (the naryad_730/n194 harness form — the same programs
// on both backends):
//   t1 — query_row wired on both: SELECT → the List (type_of);
//   t2 — query_scalar wired on both: the Text column → String;
//   t3 — query_scalar the Integer column → Float (the UNION arm —
//        the reason a single precise spelling is fictitious);
//   t4 — query_scalar the no-row case → Unit (the third union arm);
//   t5 — query SELECT → List (the union's rows arm; the mutation
//        String arm is the №194 battery's t1);
//   t6 — the context-less lane: query_row/query_scalar refuse LOUDLY
//        (the undefined-function path over the no-handler stubs), and
//        query refuses LOUDLY too (the №7 intercepts precede the
//        registry dispatch on BOTH lanes — the db-not-open named
//        remedy, the №758 form; the registry's Value::Query wrapper
//        arm is shadowed by the intercepts execution.rs:2440-2442 /
//        vm.rs:2447).
//
// The movement outcome: the counter criterion (23 → N, N > 23) is
// UNACHIEVABLE honestly — the union ground needs the vocabulary/
// semantics verdict (the union spelling, the fixed-result №543 form,
// or the Query/Row opaque kinds — the №644 boundary, the owner lane).
// The gate stays 23/67 BLOCKING (only-up; nothing honest to move;
// the №757/№613 floor form has nothing to raise — the live share is
// unchanged). THIS IS THE REPORTED FACT, not a failure of the walk:
// the residue wall is the program's designed endgame frontier
// (ADR-0193 endgame (A), N=23 ratified by №735 — the ADR-0195 §7
// RECORD form).
//
// The №476 rule: SQL classes stay OUT of the fuzzer corpus — the pins
// here are the naryad's own battery (the tests/ lane), not the corpus.
#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source.trim(), repo_root())
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(repo_root());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

fn assert_both(src: &str, expected: &str) {
    let tw = run_tw(src)
        .unwrap_or_else(|e| panic!("TW run failed: {e}\n{src}"))
        .unwrap_or_default();
    assert_eq!(tw.trim_end(), expected, "TW output mismatch\n{src}");
    let vm = run_vm(src)
        .unwrap_or_else(|e| panic!("VM run failed: {e}\n{src}"))
        .unwrap_or_default();
    assert_eq!(vm.trim_end(), expected, "VM output mismatch\n{src}");
}

/// t1 — query_row IS wired on both lanes: the single-row SELECT
/// returns the List (the №26 P1-7 wiring — the stale "no handler"
/// refusal anchor corrected by this naryad).
#[test]
fn n736_query_row_is_live_list_on_both_backends() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern RowProbe(_x: String) -> String {
  let _ = query("CREATE TABLE t (a TEXT)")
  let _ = query("INSERT INTO t VALUES ('x')")
  let row = query_row("SELECT a FROM t")
  return type_of(row)
}
flow Main { input: String = "x" -> RowProbe -> output }
"#;
    assert_both(src, "List");
}

/// t2 — query_scalar IS wired on both lanes: the Text column rides
/// String (the first arm of the cell union).
#[test]
fn n736_query_scalar_text_arm_string_on_both_backends() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern ScalarProbe(_x: String) -> String {
  let _ = query("CREATE TABLE t (a TEXT)")
  let _ = query("INSERT INTO t VALUES ('x')")
  let v = query_scalar("SELECT a FROM t")
  return type_of(v)
}
flow Main { input: String = "x" -> ScalarProbe -> output }
"#;
    assert_both(src, "String");
}

/// t3 — THE UNION ARM: the Integer column rides Float (the
/// db_ops.rs:478-489 mapping) — the reason a single precise spelling
/// for query_scalar is fictitious (the t2/t3 pair is the union
/// evidence).
#[test]
fn n736_query_scalar_integer_arm_float_on_both_backends() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern ScalarProbe(_x: String) -> String {
  let _ = query("CREATE TABLE n (k INTEGER)")
  let _ = query("INSERT INTO n VALUES (7)")
  let v = query_scalar("SELECT k FROM n")
  return type_of(v)
}
flow Main { input: String = "x" -> ScalarProbe -> output }
"#;
    assert_both(src, "Float");
}

/// t4 — the third union arm: the no-row case rides Unit (the
/// db_ops.rs:497-499 None arm).
#[test]
fn n736_query_scalar_no_row_unit_on_both_backends() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern ScalarProbe(_x: String) -> String {
  let _ = query("CREATE TABLE t (a TEXT)")
  let v = query_scalar("SELECT a FROM t WHERE 1 = 0")
  return type_of(v)
}
flow Main { input: String = "x" -> ScalarProbe -> output }
"#;
    assert_both(src, "Unit");
}

/// t5 — query's SELECT arm: the List of Row structs (the ADR-0194 §2
/// probe shape — the union's rows arm; the mutation String arm is the
/// №194 battery's t1).
#[test]
fn n736_query_select_arm_list_on_both_backends() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern SelectProbe(_x: String) -> String {
  let _ = query("CREATE TABLE t (a TEXT)")
  let _ = query("INSERT INTO t VALUES ('x')")
  let rows = query("SELECT a FROM t")
  return type_of(rows)
}
flow Main { input: String = "x" -> SelectProbe -> output }
"#;
    assert_both(src, "List");
}

/// t6a — the context-less lane: query_row refuses LOUDLY (the
/// registry stub fallback — the class "stub" names the context-less
/// Err, not the db-lane shape).
#[test]
fn n736_query_row_context_less_loud_on_both_backends() {
    let src = r#"
pattern M(_t: String) -> String {
  let v = query_row("SELECT 1")
  return type_of(v)
}
flow Main { input: String = "t" -> M -> output }
"#;
    let tw = run_tw(src).expect_err("TW must refuse loudly (the stub fallback)");
    assert!(
        tw.contains("query_row"),
        "TW refusal must name query_row: {tw}"
    );
    let vm = run_vm(src).expect_err("VM must refuse loudly (the stub fallback)");
    assert!(
        vm.contains("query_row"),
        "VM refusal must name query_row: {vm}"
    );
}

/// t6b — the same for query_scalar.
#[test]
fn n736_query_scalar_context_less_loud_on_both_backends() {
    let src = r#"
pattern M(_t: String) -> String {
  let v = query_scalar("SELECT 1")
  return type_of(v)
}
flow Main { input: String = "t" -> M -> output }
"#;
    let tw = run_tw(src).expect_err("TW must refuse loudly (the stub fallback)");
    assert!(
        tw.contains("query_scalar"),
        "TW refusal must name query_scalar: {tw}"
    );
    let vm = run_vm(src).expect_err("VM must refuse loudly (the stub fallback)");
    assert!(
        vm.contains("query_scalar"),
        "VM refusal must name query_scalar: {vm}"
    );
}

/// t6c — the context-less query refuses LOUDLY too: the №7 intercepts
/// fire BEFORE the registry dispatch on BOTH lanes, so the db-not-open
/// named remedy (the №758 form — the message names the builtin) is the
/// context-less contract, NOT the registry's opaque Value::Query
/// wrapper (that static fallback arm is shadowed by the intercepts at
/// execution.rs:2440-2442 and vm.rs:2447).
#[test]
fn n736_query_context_less_loud_on_both_backends() {
    let src = r#"
pattern Q(_t: String) -> String {
  let q = query("SELECT 1")
  return type_of(q)
}
flow Main { input: String = "t" -> Q -> output }
"#;
    let tw = run_tw(src).expect_err("TW must refuse loudly (the db-not-open remedy)");
    assert!(tw.contains("query"), "TW refusal must name query: {tw}");
    let vm = run_vm(src).expect_err("VM must refuse loudly (the db-not-open remedy)");
    assert!(vm.contains("query"), "VM refusal must name query: {vm}");
}
