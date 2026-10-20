// ── №740 (gh#1378, Wave 53): the 0.32 fixed-list movement №8 — the
// query* package through the №740 UNION vocabulary (lane α) ──────────
//
// The В53 dispatch (gh#1381) named lane α as the growth path №736/§6.37
// left open: the SURVIVING cause of the query* refusals was the UNION
// ground the stage-0 vocabulary (№627: PRECISE {String,Float,Bool,Unit}
// + the List</Struct< parameterized forms) could not spell. THIS
// naryad extends the vocabulary — the union spelling `arm|arm|...`
// (≥2 stage-0-clean arms, STRICTLY ASCENDING byte order — the
// canonical form; a duplicate, an unsorted or a malformed arm fails
// closed to Unknown) — and spells the three registry rows from the
// LIVE handler facts (the ADR-0194 ONE contract + the intercepts):
//
//   query        → List<Struct<Row>>|String  (the SQL-class union:
//                  SELECT/PRAGMA → the List of Row structs — the
//                  fields dynamic by construction, the type_name
//                  "Row" is the db_ops.rs:209/787 fact, no field-meta
//                  claimed, the rows-path NOT extended; mutations →
//                  the affected count as a String on BOTH lanes —
//                  db_ops.rs:212-218 TW / :744-755 VM);
//   query_scalar → Float|String|Unit          (the cell union:
//                  Null→Unit, Integer/Real→Float, Text/Blob→String —
//                  db_ops.rs:478-489);
//   query_row    → List<Float|String|Unit>    (the FIRST row's cells
//                  as a List — db_ops.rs:570-576; the element union
//                  db_ops.rs:554-564; the №669 "a List<X> spelling
//                  would be fictitious" concern dissolves because the
//                  inner is no longer a single fictitious X but the
//                  EXACT cell union).
//
// The №539 false-warn guard is satisfied BY CONSTRUCTION: the union
// names EVERY reachable arm (typing String alone would false-warn the
// PRIMARY SELECT arm — the memory_read form; the ADR-0194 §2 probe
// record). The (β) fixed-result №543 form and the (γ) Query/Row opaque
// kinds are NOT taken (the В53 dispatch gate 7 — separate owner
// verdicts). The context-less t6 pin (both lanes refuse LOUDLY) is
// unchanged — the intercept marker variant keeps handler: None, the
// dispatch surface is untouched.
//
// The goal032 counter 23 → 26/67 (the db group 3/3 — the FIRST ПДн-
// class group fully closed); the floors raised in the SAME PR (the
// №757/№613 form): the sensitive 6758 → 6965 bp, the general typed
// 6844 → 6901 bp, the parameterized 6893 → 7540 bp (the machine's own
// notes), the goal032 figure 23 → 26 in ci.yml.
#![allow(clippy::disallowed_methods)]

use metalogos::builtins::{sig_types::Type, BUILTIN_REGISTRY};
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

fn spec_of(name: &str) -> &'static metalogos::builtins::BuiltinSpec {
    BUILTIN_REGISTRY
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("the registry row {name} must exist"))
}

// ── The registry-side pins: the spellings ARE the vocabulary ─────────

#[test]
fn n740_query_rows_carry_the_union_spellings() {
    // query — the SQL-class union rows|count (Type::Union erasure).
    assert_eq!(spec_of("query").return_type, Type::Union);
    // query_scalar — the cell union (Type::Union erasure).
    assert_eq!(spec_of("query_scalar").return_type, Type::Union);
    // query_row — the union INNER under the List head (the erasure
    // keeps the List; the parameterized spelling).
    assert_eq!(spec_of("query_row").return_type, Type::List);
    assert!(spec_of("query_row").parameterized);
    // The unions are NEVER parameterized (the own-category rule).
    assert!(!spec_of("query").parameterized);
    assert!(!spec_of("query_scalar").parameterized);
    // No field-meta is claimed anywhere (the Row fields are dynamic —
    // the rows-path NOT extended; the ADR-0194 Запреты hold).
    assert_eq!(spec_of("query").field_meta, "");
    assert_eq!(spec_of("query_scalar").field_meta, "");
    assert_eq!(spec_of("query_row").field_meta, "");
    // The intercept posture: the registry rows stay handlerless (the
    // live handlers are the №26 P1-7 lane intercepts; the context-less
    // dispatch stays the loud stub — the t6 pin).
    assert!(spec_of("query_scalar").handler.is_none());
    assert!(spec_of("query_row").handler.is_none());
    // query keeps its shadowed static fallback handler (untouched).
    assert!(spec_of("query").handler.is_some());
}

#[test]
fn n740_the_union_grammar_is_fail_closed() {
    // THE movement's spellings parse as unions / parameterized:
    assert_eq!(Type::from_path("List<Struct<Row>>|String"), Type::Union);
    assert_eq!(Type::from_path("Float|String|Unit"), Type::Union);
    assert_eq!(Type::from_path("List<Float|String|Unit>"), Type::List);
    // The canonical-form armor: duplicates, unsorted arms, empty arms,
    // spaces, a malformed arm — all fail closed to Unknown.
    assert_eq!(Type::from_path("String|String"), Type::Unknown);
    assert_eq!(Type::from_path("String|Float"), Type::Unknown);
    assert_eq!(Type::from_path("Float|"), Type::Unknown);
    assert_eq!(Type::from_path("Float||Unit"), Type::Unknown);
    assert_eq!(Type::from_path("Float | String"), Type::Unknown);
    assert_eq!(Type::from_path("Float|nonsense"), Type::Unknown);
    // A top-level pipe on a non-union-clean string never launders.
    assert_eq!(Type::from_path("Grant|Secret"), Type::Union); // clean arms, ascending
    assert_eq!(Type::from_path("Secret|Grant"), Type::Unknown); // unsorted
    assert_eq!(Type::from_path("Secret|nonsense"), Type::Unknown);
}

// ── The false-warn guard: the spellings agree with the LIVE arms ─────

#[test]
fn n740_query_union_arms_match_the_live_contract_on_both_backends() {
    // t5 (the n736 posture) + the mutation arm: the SELECT arm returns
    // the List (the union's first arm), the mutation arm the String
    // count (the union's second arm) — the №474 ONE contract, both
    // lanes. The union names BOTH — no false-warn by construction.
    let src = r#"
db { url: "sqlite::memory:" }
pattern Probe(_x: String) -> String {
  let _ = query("CREATE TABLE t (a TEXT)")
  let rows = query("SELECT a FROM t")
  let is_list = type_of(rows) == "List"
  let affected = query("INSERT INTO t VALUES ('x')")
  return if is_list { type_of(affected) } else { "NOT-A-LIST" }
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    assert_both(src, "String");
}

#[test]
fn n740_query_scalar_union_arms_match_the_live_cells_on_both_backends() {
    // The t2–t4 posture: the Text cell → String, the Integer cell →
    // Float, the no-row case → Unit — ALL THREE union arms live.
    let src = r#"
db { url: "sqlite::memory:" }
pattern Probe(_x: String) -> String {
  let _ = query("CREATE TABLE t (a TEXT, b INTEGER)")
  let _ = query("INSERT INTO t VALUES ('x', 7)")
  let s1 = type_of(query_scalar("SELECT a FROM t"))
  let s2 = type_of(query_scalar("SELECT b FROM t"))
  let s3 = type_of(query_scalar("SELECT a FROM t WHERE a = 'none'"))
  return s1 + "|" + s2 + "|" + s3
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    assert_both(src, "String|Float|Unit");
}

#[test]
fn n740_query_row_union_inner_matches_the_live_list_on_both_backends() {
    // t1 (the n736 posture): the single-row SELECT → the List of the
    // row's cells (the union inner Float|String|Unit — here the Text
    // cell makes the element String).
    let src = r#"
db { url: "sqlite::memory:" }
pattern Probe(_x: String) -> String {
  let _ = query("CREATE TABLE t (a TEXT)")
  let _ = query("INSERT INTO t VALUES ('x')")
  let row = query_row("SELECT a FROM t")
  return type_of(row)
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    assert_both(src, "List");
}

#[test]
fn n740_the_context_less_t6_pin_stays_loud_on_both_backends() {
    // The t6 pin (the n736 battery): the context-less calls refuse
    // LOUDLY on both lanes — the intercepts precede the registry
    // dispatch; the intercept marker variant keeps handler: None, so
    // the undefined-function path is unchanged by the spellings.
    for name in ["query", "query_scalar", "query_row"] {
        let src = format!(
            r#"
pattern Probe(_x: String) -> String {{
  let v = {name}("SELECT 1")
  return type_of(v)
}}
flow Main {{ input: String = "x" -> Probe -> output }}
"#
        );
        for (lane, out) in [("TW", run_tw(&src)), ("VM", run_vm(&src))] {
            let err = out
                .err()
                .unwrap_or_else(|| panic!("{lane}: {name} must refuse loudly (the t6 pin)\n{src}"));
            assert!(
                err.contains(name),
                "{lane}: the refusal must name {name}: {err}\n{src}"
            );
        }
    }
}
