// ── Naryad №734 (P1, compiler; Wave 51 tail) — the ADR-0194 landing ──
//
// The owner's verbatim 2026-10-20 («ADR-0194 вербатим (пакеты A/B)
// давай сделаем с твоей рекомендацией и ссылкой что куда вносить»)
// with the package-choice delegation (the В49-final lekalo) fixed
// packages A(c)+B(i) — the ADR §8 ratification record:
//
//   A(c) — the loud-fail CONTRACT on BOTH lanes: the TW cell error
//          names the column (the db_ops.rs:773 mirror), the row error
//          propagates (the silent skip is gone). Practically
//          unreachable over embedded sqlite (the ADR §2 probe record
//          remains the carrier) — verified by the batteries, not a
//          dead-branch unit test: THIS file IS the battery, and the
//          SELECT controls below exercise the loud path indirectly
//          (the happy-path parity holds; the unreachability is the
//          ADR's machine-verified claim).
//   B(i) — String-count on both: the VM mutation detect mirrors the
//          TW byte-for-byte (trim().to_uppercase(); NOT
//          SELECT/PRAGMA) — `query("INSERT…")` returns `"3"` on both
//          backends (the №474 ONE contract). THE OBSERVABLE CHANGE:
//          the VM-lane mutation result `[]` → `"3"` (the ADR §3
//          inventory: the examples/office mutations ride
//          `db_execute` — zero affected consumers).
//
// This file pins the landed contract on BOTH backends (the naryad_474
// harness form):
//   t1 — the mutation through `query()` → the String count on both
//        (THE parity pin — the ADR §2 live-probe shape, now GREEN on
//        both lanes);
//   t2 — SELECT → rows on both (the §2 control — the happy path is
//        untouched by both packages);
//   t3 — SELECT with parameters on both (the №381 regression — the
//        typed binding holds);
//   t4 — a parameterized MUTATION on both (the B(i) mechanics ride
//        `conn.execute` with the bound params — the count includes
//        the parameterized rows);
//   t5 — `db_execute` stays the ONE implementation (the №474 pin
//        cross-check — the mutation through `db_execute` returns the
//        same String shape on both).
//
// The mutation check (the naryad battery): reverting the A.3 VM
// mutation branch (the detect deleted → rows built unconditionally)
// turns t1 RED on the VM lane (the [] vs "3" mismatch) — reproduced
// on the PR branch in CI, then restored; the failure is the pin's
// teeth.
//
// The №476 rule: SQL classes stay OUT of the fuzzer corpus — the pins
// here are the naryad's own battery (the tests/ lane), not the corpus.

fn tw_out(source: &str) -> Result<String, String> {
    metalogos::run_program(source).map(|o| o.unwrap_or_default().trim_end().to_string())
}

fn vm_out(source: &str) -> Result<String, String> {
    let program = metalogos::compile_program(source)?;
    metalogos::run_bytecode(program).map(|o| o.unwrap_or_default().trim_end().to_string())
}

fn assert_parity(source: &str, expected: &str, what: &str) {
    let tw = tw_out(source).unwrap_or_else(|e| panic!("{}: TW run must succeed, got: {}", what, e));
    let vm = vm_out(source).unwrap_or_else(|e| panic!("{}: VM run must succeed, got: {}", what, e));
    assert_eq!(tw, expected, "{}: TW output", what);
    assert_eq!(vm, expected, "{}: VM output (parity)", what);
}

/// t1 — THE parity pin: the mutation through `query()` returns the
/// affected-row count as a String on BOTH backends (the ADR-0194 §2
/// live-probe shape; the VM lane was `[]` before the B(i) landing).
#[test]
fn n194_query_mutation_returns_string_count_both_backends() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern ParityProbe(_x: String) -> String {
  let _ = query("CREATE TABLE t (a TEXT)")
  let n = query("INSERT INTO t VALUES ('x'),('y'),('z')")
  return type_of(n) + "/" + n
}
flow Main { input: String = "x" -> ParityProbe -> output }
"#;
    assert_parity(
        src,
        "String/3",
        "the mutation contract (B(i)) must hold on both lanes",
    );
}

/// t2 — the §2 control: SELECT → rows on BOTH backends (the happy path
/// is untouched by the packages; the Row shape and the ordering stand).
#[test]
fn n194_query_select_rows_untouched_both_backends() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern ParityProbe(_x: String) -> String {
  let _ = query("CREATE TABLE t (a TEXT)")
  let _ = query("INSERT INTO t VALUES ('x'),('y'),('z')")
  let rows = query("SELECT a FROM t ORDER BY a")
  return type_of(rows) + "/" + json_get(get(rows, 0), "a") + "/"
    + json_get(get(rows, 2), "a")
}
flow Main { input: String = "x" -> ParityProbe -> output }
"#;
    assert_parity(
        src,
        "List/x/z",
        "the SELECT control (the §2 parity) must hold",
    );
}

/// t3 — the №381 regression: SELECT with the bound parameters on BOTH
/// backends (the typed convert_params binding holds under the landed
/// mutation detect — the SELECT path is byte-identical to the pre-№734
/// lane).
#[test]
fn n194_query_select_with_params_regression_n381() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern ParityProbe(_x: String) -> String {
  let _ = query("CREATE TABLE t (a TEXT)")
  let _ = query("INSERT INTO t VALUES ('x'),('y'),('z')")
  let rows = query("SELECT a FROM t WHERE a > ? ORDER BY a", ["x"])
  return type_of(rows) + "/" + json_get(get(rows, 0), "a")
}
flow Main { input: String = "x" -> ParityProbe -> output }
"#;
    assert_parity(
        src,
        "List/y",
        "the parameterized SELECT (the №381 regression) must hold",
    );
}

/// t4 — the parameterized MUTATION: the B(i) mechanics ride
/// `conn.execute` with the bound params — the affected count includes
/// the parameterized rows on BOTH backends.
#[test]
fn n194_query_mutation_with_params_string_count_both_backends() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern ParityProbe(_x: String) -> String {
  let _ = query("CREATE TABLE t (a TEXT)")
  let n = query("INSERT INTO t (a) VALUES (?), (?), (?)", ["x", "y", "z"])
  return type_of(n) + "/" + n
}
flow Main { input: String = "x" -> ParityProbe -> output }
"#;
    assert_parity(
        src,
        "String/3",
        "the parameterized mutation contract must hold on both lanes",
    );
}

/// t5 — the №474 cross-check: `db_execute` stays the ONE mutation
/// implementation (the String shape on both) — the query() landing did
/// NOT touch the ONE contract's own lane (the existing
/// naryad_474_db_parity.rs pin stays green; this is the in-file
/// cross-check).
#[test]
fn n194_db_execute_one_contract_stays_green() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern ParityProbe(_x: String) -> String {
  let _ = query("CREATE TABLE t (a TEXT)")
  let n = db_execute("INSERT INTO t VALUES ('x'),('y'),('z')", [])
  return type_of(n) + "/" + n
}
flow Main { input: String = "x" -> ParityProbe -> output }
"#;
    assert_parity(
        src,
        "String/3",
        "the №474 ONE contract must stay untouched",
    );
}
