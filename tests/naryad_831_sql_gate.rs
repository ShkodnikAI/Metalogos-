// ── tests/naryad_831_sql_gate.rs ────────────────────────────────────
// №831 (wave 70, P1, [security][db]) — the H-2 finding of the merged
// audit `24fde8a`: the SQL_DYNAMIC gate reads the SQL position from the
// classification SSOT (builtins_classification::SQL_ARG_POSITIONS) — 5
// carriers, not the №544 pair; the db_insert identifiers carry their own
// arm (the table literal-only + the untrusted column structure) and the
// runtime quote_identifier hardening rides db_ops.rs.
//
// RED-before evidence: the corpus negatives n29/n39/n40 COMPILED on main
// `24fde8a` (the leak-suite run before the fix — n29 query_row, n39
// query_scalar, n40 db_execute_with_grant all NOT CAUGHT); n41/n42 were
// refused under the FOREIGN data-taint class (UNTRUSTED_EGRESS_NETWORK)
// while the trusted-dynamic identifier hole stayed open. The suite
// turned GREEN in the same PR as the fix (the №465 ratchet form).

use metalogos::audit::{audit_category_a, Severity};
use metalogos::parser;

/// The SQL_DYNAMIC error findings for one program.
fn sql_dynamic_errors(source: &str) -> Vec<String> {
    let declarations = parser::parse(source).expect("parse");
    audit_category_a(&declarations, "")
        .into_iter()
        .filter(|f| f.check_id == "SQL_DYNAMIC" && f.severity == Severity::Error)
        .map(|f| f.message)
        .collect()
}

fn compile_ok(source: &str) {
    let declarations = parser::parse(source).expect("parse");
    let findings = audit_category_a(&declarations, "");
    let errs: Vec<String> = findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .map(|f| format!("[{}] {}", f.check_id, f.message))
        .collect();
    assert!(
        errs.is_empty(),
        "the program must carry NO Category-A error, got: {:?}",
        errs
    );
}

// ── the 5/5 carrier coverage (the H-2 core) ─────────────────────────

#[test]
fn query_row_non_literal_sql_refuses() {
    let errs = sql_dynamic_errors(
        "pattern P() -> String {\n  let _ = query_row(\"SELECT * FROM u WHERE n='\" + query_param(\"n\") + \"'\")\n  return \"done\"\n}",
    );
    assert!(
        errs.iter().any(|m| m.contains("query_row")),
        "query_row with a concatenated template must refuse: {:?}",
        errs
    );
}

#[test]
fn query_scalar_non_literal_sql_refuses() {
    let errs = sql_dynamic_errors(
        "pattern P() -> String {\n  let _ = query_scalar(\"SELECT count(*) FROM u WHERE n='\" + query_param(\"n\") + \"'\")\n  return \"done\"\n}",
    );
    assert!(
        errs.iter().any(|m| m.contains("query_scalar")),
        "query_scalar with a concatenated template must refuse: {:?}",
        errs
    );
}

#[test]
fn db_execute_with_grant_non_literal_sql_at_arg1_refuses() {
    let errs = sql_dynamic_errors(
        "pattern P() -> String {\n  let g = grant_issue(\"delete\", \"logs\")\n  let _ = db_execute_with_grant(g, \"DELETE FROM logs WHERE id=\" + query_param(\"id\"))\n  return \"done\"\n}",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("db_execute_with_grant") && m.contains("argument 1")),
        "db_execute_with_grant refuses at the SQL position (arg 1): {:?}",
        errs
    );
}

/// The grant POSITION is not SQL: a dynamic grant handle at arg 0 must
/// NOT trip the SQL gate (the position question, not a name question).
#[test]
fn grant_position_is_not_sql() {
    let errs = sql_dynamic_errors(
        "pattern P() -> String {\n  let g = grant_issue(\"delete\", \"logs\")\n  let _ = db_execute_with_grant(g, \"DELETE FROM logs\")\n  return \"done\"\n}",
    );
    assert!(
        errs.is_empty(),
        "a literal SQL template at arg 1 with a bound grant at arg 0 is legal: {:?}",
        errs
    );
}

/// The office forms keep passing: literal SQL through the SSOT carriers
/// (non-destructive statements — a bare DELETE literal is the n18
/// grant-gate's business, not the SQL_DYNAMIC class).
#[test]
fn literal_sql_through_every_carrier_passes() {
    compile_ok(
        "pattern P() -> String {\n  let _ = query(\"SELECT 1\")\n  let _ = query_row(\"SELECT 1\")\n  let _ = query_scalar(\"SELECT 1\")\n  let _ = db_execute(\"UPDATE t SET n = 1\")\n  let g = grant_issue(\"write\", \"t\")\n  let _ = db_execute_with_grant(g, \"UPDATE t SET n = 1\")\n  return \"done\"\n}",
    );
}

// ── the db_insert identifier arm ────────────────────────────────────

#[test]
fn db_insert_non_literal_table_refuses() {
    let errs = sql_dynamic_errors(
        "pattern P(d: String) -> String {\n  let _ = db_insert(query_param(\"t\"), { v: d })\n  return \"done\"\n}",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("db_insert") && m.contains("table name")),
        "a non-literal table identifier must refuse: {:?}",
        errs
    );
}

/// The trusted-dynamic table (a let-bound concat — NO untrusted taint)
/// refused by the identifier arm: the hole the data-taint gates miss
/// (the RED-before evidence for this exact shape rode the unit state,
/// the corpus n41 covers the untrusted variant).
#[test]
fn db_insert_trusted_dynamic_table_refuses() {
    let errs = sql_dynamic_errors(
        "pattern P() -> String {\n  let t = \"user\" + \"_v2\"\n  let _ = db_insert(t, { v: 1.0 })\n  return \"done\"\n}",
    );
    assert!(
        errs.iter().any(|m| m.contains("db_insert") && m.contains("table name")),
        "a trusted non-literal table must refuse (the identifier is substituted into the SQL text): {:?}",
        errs
    );
}

#[test]
fn db_insert_json_body_struct_refuses() {
    let errs = sql_dynamic_errors(
        "pattern P() -> String {\n  let _ = db_insert(\"users\", json_body())\n  return \"done\"\n}",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("db_insert") && m.contains("untrusted")),
        "the untrusted column structure must refuse (mass assignment): {:?}",
        errs
    );
}

/// The let-bound untrusted structure refuses too (the tracker layer).
#[test]
fn db_insert_let_bound_json_body_refuses() {
    let errs = sql_dynamic_errors(
        "pattern P() -> String {\n  let d = json_body()\n  let _ = db_insert(\"users\", d)\n  return \"done\"\n}",
    );
    assert!(
        errs.iter()
            .any(|m| m.contains("db_insert") && m.contains("untrusted")),
        "a let-bound untrusted structure must refuse: {:?}",
        errs
    );
}

/// The office forms keep passing: literal table + struct literal keys.
#[test]
fn db_insert_literal_table_and_struct_literal_pass() {
    compile_ok(
        "pattern P(topic: String) -> String {\n  let _ = db_insert(\"analysis\", { topic: topic, status: \"drafted\" })\n  let d = { a: 1.0 }\n  let _ = db_insert(\"t2\", d)\n  return \"done\"\n}",
    );
}

/// The struct-literal whitelist does not require a declared schema —
/// the grammar's keys are the static column set (the №831 choice).
#[test]
fn db_insert_struct_literal_without_schema_passes() {
    compile_ok(
        "schema s {\n  table notes {\n    title: String,\n    body: Text\n  }\n}\npattern P() -> String {\n  let _ = db_insert(\"notes\", { title: \"x\", body: \"y\" })\n  return \"done\"\n}",
    );
}
