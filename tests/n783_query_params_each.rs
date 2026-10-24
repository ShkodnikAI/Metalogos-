// ── Naryad №783 (audit-23.10 A4/S-3+N-2, P1) — the loud params/each pins ──
//
// S-3: the query* family answered a NON-List params argument with
// `Vec::new()` — SILENTLY (`query("… WHERE id = ?1", user_id)` bound
// ZERO parameters and, without a mandatory placeholder, passed
// quietly). `db_execute` was loud since №553 — its wording moved into
// the `bind_params` SSOT (the `[x, y]` form named).
// N-2: the VM `each` over a NON-List answered 0 iterations silently
// while the TW refused with `[TYPE_MISMATCH] each: expected List, got
// String` — a live TW/VM divergence of the gh#1336 class.
//
// RED-before (verified on the unpatched main d0245a4, the stash-run):
// every pin here FAILED on the pre-№783 engines (the quiet Ok instead
// of the refusal / the divergence instead of the parity).
#![allow(clippy::disallowed_methods)]

fn tw_run(source: &str) -> Result<String, String> {
    metalogos::run_program(source).map(|o| o.unwrap_or_default().trim_end().to_string())
}

fn vm_run(source: &str) -> Result<String, String> {
    let program = metalogos::compile_program(source)?;
    metalogos::run_bytecode(program).map(|o| o.unwrap_or_default().trim_end().to_string())
}

fn assert_loud_both(source: &str, marker: &str, what: &str) {
    for (lane, res) in [("TW", tw_run(source)), ("VM", vm_run(source))] {
        let err = res.expect_err(&format!(
            "{what} [{lane}]: the non-List params argument must be a LOUD refusal, got Ok"
        ));
        assert!(
            err.contains(marker),
            "{what} [{lane}]: expected [{marker}] in the refusal, got: {err}"
        );
    }
}

const DB_DECL: &str = "db { url: \"sqlite::memory:\" }";

/// query(): a scalar (non-List) second argument is a LOUD refusal naming
/// the `[x, y]` form — on BOTH backends. RED-before: the call passed
/// QUIETLY (the parameters dropped).
#[test]
fn n783_query_nonlist_params_loud_both() {
    let src = format!(
        r#"
{DB_DECL}
pattern Probe(s: String) -> String {{
  let _ = query("CREATE TABLE t (a TEXT)")
  let _ = query("INSERT INTO t VALUES ('{}')", s)
  let n = query("SELECT count(*) FROM t")
  return type_of(n)
}}
flow Main {{ input: String = "x" -> Probe -> output }}
"#,
        "x"
    );
    // The INSERT carries no placeholder: the pre-№783 bind dropped `s`
    // and the INSERT went through QUIETLY (the row landed without the
    // caller's data). The refusal fires BEFORE the engine sees the SQL.
    assert_loud_both(
        &src,
        "must be a List of parameters",
        "query non-List params",
    );
}

/// query_scalar(): the same contract.
#[test]
fn n783_query_scalar_nonlist_params_loud_both() {
    let src = format!(
        r#"
{DB_DECL}
pattern Probe(_s: String) -> String {{
  let _ = query("CREATE TABLE t (a TEXT)")
  let n = query_scalar("SELECT count(*) FROM t", 42.0)
  return type_of(n)
}}
flow Main {{ input: String = "x" -> Probe -> output }}
"#,
    );
    assert_loud_both(
        &src,
        "must be a List of parameters",
        "query_scalar non-List params",
    );
}

/// query_row(): the same contract.
#[test]
fn n783_query_row_nonlist_params_loud_both() {
    let src = format!(
        r#"
{DB_DECL}
pattern Probe(_s: String) -> String {{
  let _ = query("CREATE TABLE t (a TEXT)")
  let n = query_row("SELECT a FROM t WHERE a = 'x'", "not-a-list")
  return type_of(n)
}}
flow Main {{ input: String = "x" -> Probe -> output }}
"#,
    );
    assert_loud_both(
        &src,
        "must be a List of parameters",
        "query_row non-List params",
    );
}

/// db_execute(): the №553 loudness holds; the wording rides the SSOT
/// (the `[x, y]` form named). RED-before: the OLD wording — the pin
/// matched the new shape and failed.
#[test]
fn n783_db_execute_nonlist_params_loud_both() {
    let src = format!(
        r#"
{DB_DECL}
pattern Probe(s: String) -> String {{
  let _ = db_execute("CREATE TABLE t (a TEXT)")
  let _ = db_execute("INSERT INTO t VALUES ('{}')", s)
  return "ok"
}}
flow Main {{ input: String = "x" -> Probe -> output }}
"#,
        "x"
    );
    assert_loud_both(
        &src,
        "must be a List of parameters",
        "db_execute non-List params",
    );
}

/// N-2: `each` over a NON-List — the SAME coded refusal on BOTH
/// backends (the byte-identical TW wording). RED-before: the VM lane
/// returned the empty trace (0 iterations, silent), the TW lane
/// refused — a live divergence.
#[test]
fn n783_each_nonlist_parity_both() {
    let src = r#"
pattern Probe(s: String) -> String {
  let mut tr = ""
  each x in s {
    tr = tr + "x"
  }
  return tr
}
flow Main { input: String = "abc" -> Probe -> output }
"#;
    for (lane, res) in [("TW", tw_run(src)), ("VM", vm_run(src))] {
        let err = res.expect_err(&format!(
            "each non-List [{lane}]: must be a loud refusal, got Ok (the silent 0-iterations class)"
        ));
        assert!(
            err.contains("[TYPE_MISMATCH]") && err.contains("each: expected List, got String"),
            "each non-List [{lane}]: expected the coded TW-parity refusal, got: {err}"
        );
    }
}

/// The happy-path control: `each` over a REAL List iterates identically
/// on both lanes (the №783 fix must not touch the List case).
#[test]
fn n783_each_list_control_both() {
    let src = r#"
pattern Probe(s: String) -> String {
  let items = ["a", "b", "c"]
  let mut tr = ""
  each x in items {
    tr = tr + x
  }
  return tr
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    for (lane, res) in [("TW", tw_run(src)), ("VM", vm_run(src))] {
        assert_eq!(
            res.unwrap_or_else(|e| panic!("each List [{lane}]: must succeed, got: {e}")),
            "abc",
            "each List [{lane}]: the iteration contract"
        );
    }
}

/// The happy-path control: a proper `[x, y]` List still binds TYPED on
/// both lanes (the №381/№99 contract is untouched).
#[test]
fn n783_query_list_params_control_both() {
    let src = format!(
        r#"
{DB_DECL}
pattern Probe(_s: String) -> String {{
  let _ = query("CREATE TABLE t (a TEXT)")
  let _ = query("INSERT INTO t VALUES ('x'),('y')")
  let rows = query("SELECT a FROM t WHERE a = ?1", ["x"])
  return json_get(get(rows, 0), "a")
}}
flow Main {{ input: String = "x" -> Probe -> output }}
"#,
    );
    for (lane, res) in [("TW", tw_run(&src)), ("VM", vm_run(&src))] {
        assert_eq!(
            res.unwrap_or_else(|e| panic!("List-params [{lane}]: must succeed, got: {e}")),
            "x",
            "List-params [{lane}]: the typed binding holds"
        );
    }
}
