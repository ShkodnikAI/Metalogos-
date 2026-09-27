// ── Naryad №474 (P0, vm/security) — the db parity (issue #722) ──────
//
// The audit 26.09 §3.2 (High) finding, fact-checked verbatim against
// the v0.26.1 revision: the VM `query_row` bound params STRINGIFIED
// (Vec<String>) and SILENTLY DROPPED unsupported values (a `filter_map`
// with `_ => None`) — the positional `$N` placeholders shifted; the
// worst case EXECUTED a query with a different semantics (a Unit first
// in the list shifted every later binding one position left). The
// error was fixed in the tree-walking backend by №99 and lived on in
// the VM — the default production backend of `mlog serve` since
// ADR-0171.
//
// This file blocks the regression on BOTH backends:
//   1. the audit's worst case: a Unit param must NOT shift the
//      positions (Unit → SQL NULL through the №381 convert_params
//      SSOT) — the same program produces the SAME result on TW and VM;
//   2. the audit's scenario verbatim (the Unit mid-list): no param
//      count error, no shifted match — parity;
//   3. `db_execute` returns the affected-row count as a String on BOTH
//      backends (the TW contract — the VM lane was raised to it);
//   4. an unsupported param value is a LOUD error naming the 1-based
//      parameter position on BOTH backends.

fn tw_out(source: &str) -> Result<String, String> {
    metalogos::run_program(source)
        .map(|o| o.unwrap_or_default().trim_end().to_string())
}

fn vm_out(source: &str) -> Result<String, String> {
    let program = metalogos::compile_program(source)?;
    metalogos::run_bytecode(program)
        .map(|o| o.unwrap_or_default().trim_end().to_string())
}

fn assert_parity(source: &str, expected: &str, what: &str) {
    let tw = tw_out(source).unwrap_or_else(|e| panic!("{}: TW run must succeed, got: {}", what, e));
    let vm = vm_out(source).unwrap_or_else(|e| panic!("{}: VM run must succeed, got: {}", what, e));
    assert_eq!(tw, expected, "{}: TW output", what);
    assert_eq!(vm, expected, "{}: VM output (parity)", what);
}

/// The audit's WORST case: a Unit FIRST in the param list — under the
/// old stringify lane the Unit was dropped, "paid" shifted onto the
/// `coupon` placeholder, and the query EXECUTED with a different
/// semantics (matching the SAVE10 row). Now Unit → SQL NULL keeps the
/// positions, `coupon = NULL` never matches, and BOTH backends agree
/// on the empty result.
#[test]
fn n474_unit_param_does_not_shift_positions_worst_case() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern AuditRepro(_x: String) -> String {
  db_execute("CREATE TABLE orders (user_id REAL, coupon TEXT, status TEXT)", [])
  db_execute("INSERT INTO orders VALUES (1.0, NULL, 'paid')", [])
  db_execute("INSERT INTO orders VALUES (2.0, 'SAVE10', 'paid')", [])
  let null_row = query_row("SELECT coupon FROM orders WHERE user_id = 1.0", [])
  let maybe_coupon = get(null_row, 0)
  let rows = query_row("SELECT user_id FROM orders WHERE coupon = ? AND status = ?", [maybe_coupon, "paid"])
  if len(rows) == 0 { return "no-row" }
  return "shifted-match"
}
flow Main { input: String = "x" -> AuditRepro -> output }
"#;
    assert_parity(src, "no-row", "the worst case must not silently match");
}

/// The audit's scenario verbatim: the Unit mid-list under the old lane
/// produced the best case (a param count error) or the worst case (a
/// shifted match) — never the correct result. Now the placeholder
/// count is preserved on both backends and the NULL comparison misses.
#[test]
fn n474_audit_scenario_unit_mid_list() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern AuditRepro(_x: String) -> String {
  db_execute("CREATE TABLE orders (user_id REAL, coupon TEXT, status TEXT)", [])
  db_execute("INSERT INTO orders VALUES (1.0, 'SAVE10', 'paid')", [])
  let null_row = query_row("SELECT NULL FROM orders LIMIT 1", [])
  let maybe_coupon = get(null_row, 0)
  let rows = query_row("SELECT user_id FROM orders WHERE user_id = ? AND coupon = ? AND status = ?", [1.0, maybe_coupon, "paid"])
  if len(rows) == 0 { return "no-row" }
  return "matched"
}
flow Main { input: String = "x" -> AuditRepro -> output }
"#;
    assert_parity(src, "no-row", "the mid-list Unit keeps the positions");
}

/// The unified `db_execute` contract: the affected-row count as a
/// String on BOTH backends (the TW form — the VM lane was raised to
/// it, №474).
#[test]
fn n474_db_execute_returns_affected_count_on_both_backends() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern CountIt(_x: String) -> String {
  db_execute("CREATE TABLE t (a TEXT)", [])
  let n1 = db_execute("INSERT INTO t VALUES ('x')", [])
  let n2 = db_execute("INSERT INTO t VALUES ('y'), ('z')", [])
  return to_string(n1) + "/" + to_string(n2)
}
flow Main { input: String = "x" -> CountIt -> output }
"#;
    assert_parity(src, "1/2", "one contract on both backends");
}

/// An unsupported param value (a List) is a LOUD error naming the
/// 1-based parameter position — on BOTH backends, never a silent drop.
#[test]
fn n474_unsupported_param_is_a_loud_positioned_error_on_both_backends() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern LoudFail(_x: String) -> String {
  db_execute("CREATE TABLE t (a TEXT)", [])
  let rows = query_row("SELECT a FROM t WHERE a = ?", [["x"]])
  return "unreachable"
}
flow Main { input: String = "x" -> LoudFail -> output }
"#;
    for (backend, res) in [("TW", tw_out(src)), ("VM", vm_out(src))] {
        let err = res.expect_err(&format!(
            "{}: a List param must be a loud error, not a silent drop",
            backend
        ));
        assert!(
            err.contains("SQL parameter $1"),
            "{}: the error must name the 1-based position, got: {}",
            backend,
            err
        );
    }
}

/// Sanity: supported types (String/Float/Bool/Unit) still bind typed
/// and the shape matches — no overcorrection (the №99 backward-compat
/// posture, now on the VM lane too).
#[test]
fn n474_supported_types_still_bind_typed() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern TypedBind(_x: String) -> String {
  db_execute("CREATE TABLE t (a REAL, b TEXT, c INTEGER)", [])
  db_execute("INSERT INTO t VALUES (3.0, 'x', 1)", [])
  let real_row = query_row("SELECT a FROM t WHERE a = ?", [3.0])
  let str_row = query_row("SELECT b FROM t WHERE b = ?", ["x"])
  let bool_row = query_row("SELECT a FROM t WHERE c = ?", [true])
  let rv = get(real_row, 0)
  let sv = get(str_row, 0)
  let bv = get(bool_row, 0)
  return to_string(rv) + "/" + sv + "/" + to_string(bv)
}
flow Main { input: String = "x" -> TypedBind -> output }
"#;
    assert_parity(src, "3/x/3", "typed binding backward-compat on both backends");
}
