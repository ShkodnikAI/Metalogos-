// ── tests/naryad_787_precision_loss.rs ───────────────────────────────
// №787 (gh#1503, Wave 63 P1 — audit-23.10 C6 I-c): the integer
// precision-loss boundary — integers that f64 cannot represent exactly
// (|n| > 2^53: 64-bit ids, snowflakes) were SILENTLY distorted at the
// SQLite→Value row conversion (`ValueRef::Integer(n) => Value::Float(n
// as f64)` — four sites: query_tw, query_scalar, query_row, query_vm)
// and at the JSON→Value conversion (`Number(n) => Value::Float(n
// .as_f64().unwrap_or(0.0))` — the silent `unwrap_or(0.0)` made the
// never-representable case WORSE than lossy: a plausible-looking 0.0).
//
// THE CONTRACT (the loud-failure line, ADR-0194 (c)): a lossy integer
// is a LOUD `[PRECISION_LOSS]` coded refusal naming the column (SQLite)
// or the path (JSON) and the value — never a silent round-trip
// distortion. Integers f64 represents exactly (|n| ≤ 2^53 and the exact
// powers of two beyond it, e.g. i64::MIN = -2^63) ride the UNCHANGED
// Value::Float behavior — the ≤2^53 regression surface is pinned by the
// last test. Parity: ONE contract on BOTH backends (форма №762).
//
// RED-before (форма №772, stash-verified): on the pre-№787 revision
// every loud-refusal assertion FAILS — the DB/JSON reads return the
// distorted Float (9007199254740993 reads back as 9007199254740992)
// instead of refusing.

fn tw_out(source: &str) -> Result<String, String> {
    metalogos::run_program(source).map(|o| o.unwrap_or_default().trim_end().to_string())
}

fn vm_out(source: &str) -> Result<String, String> {
    let program = metalogos::compile_program(source)?;
    metalogos::run_bytecode(program).map(|o| o.unwrap_or_default().trim_end().to_string())
}

fn assert_parity_error(source: &str, fragments: &[&str], what: &str) {
    for (backend, res) in [("TW", tw_out(source)), ("VM", vm_out(source))] {
        let err = res.expect_err(&format!(
            "{what}: {backend} must refuse loudly (the №787 contract)"
        ));
        for f in fragments {
            assert!(
                err.contains(f),
                "{what}: {backend} refusal must carry '{f}', got: {err}"
            );
        }
    }
}

fn assert_parity_output(source: &str, expected: &str, what: &str) {
    for (backend, res) in [("TW", tw_out(source)), ("VM", vm_out(source))] {
        let out = res.unwrap_or_else(|e| panic!("{what}: {backend} must succeed, got: {e}"));
        assert!(
            out.contains(expected),
            "{what}: {backend} output must contain '{expected}', got: '{out}'"
        );
    }
}

/// The audit's example verbatim: 2^53+1 stored as a SQLite INTEGER reads
/// back distorted on the pre-№787 code — now a loud refusal naming the
/// column and the value.
#[test]
fn n787_db_query_bigint_refused_loudly() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern Repro(_x: Float) -> String {
  db_execute("CREATE TABLE t (id INTEGER)", [])
  db_execute("INSERT INTO t VALUES (9007199254740993)", [])
  let row = query("SELECT id FROM t")
  return to_string(row[0].id)
}
flow Main { input: Float = 0.0 -> Repro -> output }
"#;
    assert_parity_error(
        src,
        &["[PRECISION_LOSS]", "id", "9007199254740993"],
        "n787 db query 2^53+1",
    );
}

/// query_scalar: the single-cell read is the same loud refusal.
#[test]
fn n787_db_query_scalar_bigint_refused_loudly() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern Repro(_x: Float) -> String {
  db_execute("CREATE TABLE t (v INTEGER)", [])
  db_execute("INSERT INTO t VALUES (9007199254740993)", [])
  return to_string(query_scalar("SELECT v FROM t"))
}
flow Main { input: Float = 0.0 -> Repro -> output }
"#;
    assert_parity_error(
        src,
        &["[PRECISION_LOSS]", "v", "9007199254740993"],
        "n787 query_scalar 2^53+1",
    );
}

/// query_row: the positional row read refuses too.
#[test]
fn n787_db_query_row_bigint_refused_loudly() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern Repro(_x: Float) -> String {
  db_execute("CREATE TABLE t (a INTEGER, b INTEGER)", [])
  db_execute("INSERT INTO t VALUES (1.0, 9007199254740993)", [])
  let row = query_row("SELECT a, b FROM t")
  return to_string(row[1])
}
flow Main { input: Float = 0.0 -> Repro -> output }
"#;
    assert_parity_error(
        src,
        &["[PRECISION_LOSS]", "b", "9007199254740993"],
        "n787 query_row 2^53+1",
    );
}

/// parse_json: the audit's JSON example — the object path is named, and
/// the never-representable `unwrap_or(0.0)` arm is gone.
#[test]
fn n787_parse_json_bigint_refused_loudly() {
    let src = r#"
pattern Repro(_x: Float) -> String {
  let doc = parse_json("{\"id\": 9007199254740993}")
  return to_string(doc.id)
}
flow Main { input: Float = 0.0 -> Repro -> output }
"#;
    assert_parity_error(
        src,
        &["[PRECISION_LOSS]", "$.id", "9007199254740993"],
        "n787 parse_json 2^53+1",
    );
}

/// Nested paths are named (array element + nested object).
#[test]
fn n787_parse_json_nested_path_named() {
    let src = r#"
pattern Repro(_x: Float) -> String {
  let doc = parse_json("{\"items\": [{\"ok\": 1}, {\"ok\": 2}, {\"v\": -9007199254740993}]}")
  return to_string(doc.items[2].v)
}
flow Main { input: Float = 0.0 -> Repro -> output }
"#;
    assert_parity_error(
        src,
        &["[PRECISION_LOSS]", "$.items[2].v", "9007199254740993"],
        "n787 parse_json nested path",
    );
}

/// The ≤2^53 regression surface: 2^53 itself (and everything below it)
/// reads back UNCHANGED as Float on both backends; the exact power of
/// two beyond it (i64::MIN = -2^63) is exactly representable and rides
/// the unchanged behavior too. The assertions are VALUE comparisons in
/// language — the f64 Display is not value-exact at this magnitude, the
/// exactness verdict must not depend on the printer.
#[test]
fn n787_boundary_2pow53_and_exact_powers_unchanged() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern Repro(_x: Float) -> String {
  db_execute("CREATE TABLE t (v INTEGER)", [])
  db_execute("INSERT INTO t VALUES (9007199254740992)", [])
  let a = query_scalar("SELECT v FROM t")
  let a_ok = if a == 9007199254740992.0 then "exact" else "distorted"
  db_execute("UPDATE t SET v = -9223372036854775808", [])
  let b = query_scalar("SELECT v FROM t")
  let b_ok = if b == -9223372036854775808.0 then "exact" else "distorted"
  let c_doc = parse_json("{\"small\": 42}")
  let c_ok = if c_doc.small == 42.0 then "exact" else "distorted"
  return a_ok + "|" + b_ok + "|" + c_ok
}
flow Main { input: Float = 0.0 -> Repro -> output }
"#;
    assert_parity_output(src, "exact|exact|exact", "n787 boundary");
}
