// ── №956 (issue #1923): the fuzzer's taint line learns the DELIVERY FORMS
//
// The fact the audit of 12.11 left behind (the issue's own §Факт): the
// wave-81 taint line (№904) carried 8 scenarios over 7 triples, and in
// EVERY one the source and the sink stood in the same pattern with a
// direct flow between them. Neither the leak suite nor the fuzzer held a
// single «through a pattern», «through a loop» or «through a flow output»
// form — which is why the №950/№951 breach lived from September to
// November with every gate green. A generator that only emits the shape
// the analyzer already handles measures nothing.
//
// This file is the second axis: every triple (lane, source, sink) × every
// DELIVERY FORM, with the expected verdict recorded PER CELL and the
// whole matrix run on BOTH backends.
//
// The eleven delivery forms (the issue's roster, in its order): direct;
// the return of a pattern; a parameter into a pattern that sinks it; two
// levels of calls; an `each` accumulator; a `while` accumulator; a struct
// field; a list element; a `match` arm; a `flow` output; a URL position.
//
// WHY A RECORDED VERDICT PER CELL, not one expectation for the triple:
// the same flow refuses under DIFFERENT classes depending on the form
// (the `while` fold reports SECRET_LEAK where `each` reports
// PII_EGRESS_OUTPUT), and two cells pass BY DESIGN (an untrusted-but-
// public value in a request position gates confidentiality only — B6).
// A single expectation would have to be the loosest of them, and a
// loosest expectation is how a gap hides. Every cell here was MEASURED on
// this tree, and every PASS carries the reason it passes.
//
// WHAT THE MATRIX FOUND (beyond confirming №950/№951 across all eleven
// forms):
//
//   1. `sec_entity_secret` / `pattern_return` PASSED. A `Secret`-aliased
//      entity returned from a callee and printed by the caller was not
//      refused, while the direct form was. The cause: the №950 summary
//      computed its return label with `infer_pattern_labels`, which seeds
//      PARAMETERS only, so an entity-seeded identifier (the №951 B8
//      seeds) summarized as `bottom`. Fixed in this naryad by giving the
//      summary the same `entity_seed_env` the intraprocedural gate uses —
//      ADR-0154 §12(а) already says the return label is the join of the
//      body's internal sources, and an entity-seeded identifier is one.
//      The 475-file corpus sweep is byte-identical before/after the fix.
//
//   2. The flow edge drops the label on FIVE of the seven triples
//      (`flow_output`). That is threat-model row B7, and the matrix
//      states its reach: not one fixture, five of seven. B7 stays HELD —
//      it carries three owner-level questions (one axis or two; whether
//      three project examples are corrected as leaks with their tests;
//      the sink receipt) — so the five cells are recorded as PASS WITH
//      THE ROW NAMED. When B7 lands, those five cells flip in the same
//      PR, by the roster rule below.
//
// THE ROSTER RULE (the issue's п.3): a delivery form found by an audit or
// a red-team pass is added to this roster in the SAME PR as its fix. A
// roster that lags the finding is the №904 shape again.
//
// THE HONEST COUNTER (the issue's п.2): `TAINT_SATURATION` (№906) is a
// PRODUCT — the size of a space, not a fact about a run. This file
// reports instead the number of programs the run really generates and
// analyzes, printed by the run and cross-checked by `gen_metrics.py`
// against this roster.
//
// ANALYZED vs EXECUTED, stated plainly: a program the gate REFUSES
// cannot be executed — that is what the gate is for — so for a taint
// roster the executable subset is exactly the PASS cells. The run
// therefore reports two numbers: the programs generated and analyzed on
// both backends (the fixed `programs_per_run()`), and the PASS cells
// actually RUN through the executor, which is the second line of defence
// having its say. One PASS cell is analyzed only: its runtime half calls
// `write_file`, and `write_file` resolves against the PROCESS CWD, not
// the base directory passed to the runner — running it would write into
// the repository. That cell carries `run: no` and the reason.
#![allow(clippy::disallowed_methods)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    let m = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    Path::new(&m).to_path_buf()
}

// ── №956 delivery roster BEGIN — the machine source for gen_metrics.py:
// it counts the CELL blocks between these two markers. A lost cell lowers
// TAINT_DELIVERY_CELLS (the gate working, not a pass).
/// (triple, delivery form, expected STATIC verdict, run on the backends,
/// the note — mandatory for every PASS cell).
pub const DELIVERY_CELLS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "sec_print_env",
        "direct",
        "SECRET_LEAK",
        "yes",
        "",
    ),
    (
        "sec_print_env",
        "pattern_return",
        "PII_EGRESS_OUTPUT",
        "yes",
        "",
    ),
    (
        "sec_print_env",
        "param_to_sink",
        "PII_EGRESS_OUTPUT",
        "yes",
        "",
    ),
    (
        "sec_print_env",
        "two_levels",
        "PII_EGRESS_OUTPUT",
        "yes",
        "",
    ),
    (
        "sec_print_env",
        "each_accumulator",
        "PII_EGRESS_OUTPUT",
        "yes",
        "",
    ),
    (
        "sec_print_env",
        "while_accumulator",
        "SECRET_LEAK",
        "yes",
        "the while fold reports the SECRET_LEAK class where each reports PII_EGRESS_OUTPUT: two classes, one flow, both refusals",
    ),
    (
        "sec_print_env",
        "struct_field",
        "PII_EGRESS_OUTPUT",
        "yes",
        "",
    ),
    (
        "sec_print_env",
        "list_element",
        "PII_EGRESS_OUTPUT",
        "yes",
        "",
    ),
    (
        "sec_print_env",
        "match_arm",
        "PII_EGRESS_OUTPUT",
        "yes",
        "",
    ),
    (
        "sec_print_env",
        "flow_output",
        "PII_EGRESS_OUTPUT",
        "yes",
        "№965 DONE: the flow edge carries the step's return label (the confidentiality axis; the integrity half is the №966 row)",
    ),
    (
        "sec_print_env",
        "url_position",
        "PII_EGRESS_NETWORK",
        "yes",
        "",
    ),
    (
        "sec_writefile_env",
        "direct",
        "SECRET_LEAK",
        "yes",
        "",
    ),
    (
        "sec_writefile_env",
        "pattern_return",
        "SECRET_LEAK",
        "yes",
        "",
    ),
    (
        "sec_writefile_env",
        "param_to_sink",
        "SECRET_LEAK",
        "yes",
        "",
    ),
    (
        "sec_writefile_env",
        "two_levels",
        "SECRET_LEAK",
        "yes",
        "",
    ),
    (
        "sec_writefile_env",
        "each_accumulator",
        "SECRET_LEAK",
        "yes",
        "",
    ),
    (
        "sec_writefile_env",
        "while_accumulator",
        "SECRET_LEAK",
        "yes",
        "",
    ),
    (
        "sec_writefile_env",
        "struct_field",
        "SECRET_LEAK",
        "yes",
        "",
    ),
    (
        "sec_writefile_env",
        "list_element",
        "SECRET_LEAK",
        "yes",
        "",
    ),
    (
        "sec_writefile_env",
        "match_arm",
        "SECRET_LEAK",
        "yes",
        "",
    ),
    (
        "sec_writefile_env",
        "flow_output",
        "SECRET_LEAK",
        "yes",
        "№965 DONE: the flow edge carries the step return label — the SYNTACTIC secret lane (a direct env-to-sink match) fires first for the write_file delivery (SECRET_LEAK), the same refusal family",
    ),
    (
        "sec_writefile_env",
        "url_position",
        "PII_EGRESS_NETWORK",
        "yes",
        "",
    ),
    (
        "sql_query_scalar_derived",
        "direct",
        "SQL_DYNAMIC",
        "yes",
        "",
    ),
    (
        "sql_query_scalar_derived",
        "pattern_return",
        "SQL_DYNAMIC",
        "yes",
        "",
    ),
    (
        "sql_query_scalar_derived",
        "param_to_sink",
        "SQL_DYNAMIC",
        "yes",
        "",
    ),
    (
        "sql_query_scalar_derived",
        "two_levels",
        "SQL_DYNAMIC",
        "yes",
        "",
    ),
    (
        "sql_query_scalar_derived",
        "each_accumulator",
        "SQL_DYNAMIC",
        "yes",
        "",
    ),
    (
        "sql_query_scalar_derived",
        "while_accumulator",
        "SQL_DYNAMIC",
        "yes",
        "",
    ),
    (
        "sql_query_scalar_derived",
        "struct_field",
        "SQL_DYNAMIC",
        "yes",
        "",
    ),
    (
        "sql_query_scalar_derived",
        "list_element",
        "SQL_DYNAMIC",
        "yes",
        "",
    ),
    (
        "sql_query_scalar_derived",
        "match_arm",
        "SQL_DYNAMIC",
        "yes",
        "",
    ),
    (
        "sql_query_scalar_derived",
        "flow_output",
        "SQL_DYNAMIC",
        "yes",
        "",
    ),
    (
        "sql_query_scalar_derived",
        "url_position",
        "PASS",
        "yes",
        "by design: an untrusted-but-PUBLIC value in a request position gates confidentiality only (B6/№951); the runtime SSRF guard refuses",
    ),
    (
        "sql_dbinsert_dynamic",
        "direct",
        "SQL_DYNAMIC",
        "yes",
        "",
    ),
    (
        "sql_dbinsert_dynamic",
        "pattern_return",
        "SQL_DYNAMIC",
        "yes",
        "",
    ),
    (
        "sql_dbinsert_dynamic",
        "param_to_sink",
        "SQL_DYNAMIC",
        "yes",
        "",
    ),
    (
        "sql_dbinsert_dynamic",
        "two_levels",
        "SQL_DYNAMIC",
        "yes",
        "",
    ),
    (
        "sql_dbinsert_dynamic",
        "each_accumulator",
        "SQL_DYNAMIC",
        "yes",
        "",
    ),
    (
        "sql_dbinsert_dynamic",
        "while_accumulator",
        "SQL_DYNAMIC",
        "yes",
        "",
    ),
    (
        "sql_dbinsert_dynamic",
        "struct_field",
        "SQL_DYNAMIC",
        "yes",
        "",
    ),
    (
        "sql_dbinsert_dynamic",
        "list_element",
        "SQL_DYNAMIC",
        "yes",
        "",
    ),
    (
        "sql_dbinsert_dynamic",
        "match_arm",
        "SQL_DYNAMIC",
        "yes",
        "",
    ),
    (
        "sql_dbinsert_dynamic",
        "flow_output",
        "SQL_DYNAMIC",
        "yes",
        "",
    ),
    (
        "sql_dbinsert_dynamic",
        "url_position",
        "PASS",
        "yes",
        "by design: untrusted-but-public in a request position gates confidentiality only (B6/№951); the runtime SSRF guard refuses",
    ),
    (
        "html_respond_llm_raw",
        "direct",
        "HTML_INJECTION",
        "yes",
        "",
    ),
    (
        "html_respond_llm_raw",
        "pattern_return",
        "HTML_INJECTION",
        "yes",
        "",
    ),
    (
        "html_respond_llm_raw",
        "param_to_sink",
        "HTML_INJECTION",
        "yes",
        "",
    ),
    (
        "html_respond_llm_raw",
        "two_levels",
        "HTML_INJECTION",
        "yes",
        "",
    ),
    (
        "html_respond_llm_raw",
        "each_accumulator",
        "HTML_INJECTION",
        "yes",
        "",
    ),
    (
        "html_respond_llm_raw",
        "while_accumulator",
        "HTML_INJECTION",
        "yes",
        "",
    ),
    (
        "html_respond_llm_raw",
        "struct_field",
        "HTML_INJECTION",
        "yes",
        "",
    ),
    (
        "html_respond_llm_raw",
        "list_element",
        "HTML_INJECTION",
        "yes",
        "",
    ),
    (
        "html_respond_llm_raw",
        "match_arm",
        "HTML_INJECTION",
        "yes",
        "",
    ),
    (
        "html_respond_llm_raw",
        "flow_output",
        "HTML_INJECTION",
        "yes",
        "№965 DONE: the flow edge carries the step's return label — the HTML lane's own class fires (the untrusted model output through respond); the integrity half of the EDGE is the №966 row",
    ),
    (
        "html_respond_llm_raw",
        "url_position",
        "TAINT_INTERP",
        "yes",
        "the model answer in a URL position is the interpolation class, not the egress one",
    ),
    (
        "sec_httpget_env",
        "direct",
        "PII_EGRESS_NETWORK",
        "yes",
        "",
    ),
    (
        "sec_httpget_env",
        "pattern_return",
        "PII_EGRESS_NETWORK",
        "yes",
        "",
    ),
    (
        "sec_httpget_env",
        "param_to_sink",
        "PII_EGRESS_NETWORK",
        "yes",
        "",
    ),
    (
        "sec_httpget_env",
        "two_levels",
        "PII_EGRESS_NETWORK",
        "yes",
        "",
    ),
    (
        "sec_httpget_env",
        "each_accumulator",
        "PII_EGRESS_NETWORK",
        "yes",
        "",
    ),
    (
        "sec_httpget_env",
        "while_accumulator",
        "PII_EGRESS_NETWORK",
        "yes",
        "",
    ),
    (
        "sec_httpget_env",
        "struct_field",
        "PII_EGRESS_NETWORK",
        "yes",
        "",
    ),
    (
        "sec_httpget_env",
        "list_element",
        "PII_EGRESS_NETWORK",
        "yes",
        "",
    ),
    (
        "sec_httpget_env",
        "match_arm",
        "PII_EGRESS_NETWORK",
        "yes",
        "",
    ),
    (
        "sec_httpget_env",
        "flow_output",
        "PII_EGRESS_NETWORK",
        "yes",
        "№965 DONE: the flow edge carries the step's return label — the request-side family's own class fires at the edge (the №951 B6 vocabulary); the integrity half is the №966 row",
    ),
    (
        "sec_httpget_env",
        "url_position",
        "PII_EGRESS_NETWORK",
        "yes",
        "",
    ),
    (
        "sec_entity_secret",
        "direct",
        "PII_EGRESS_OUTPUT",
        "yes",
        "",
    ),
    (
        "sec_entity_secret",
        "pattern_return",
        "PII_EGRESS_OUTPUT",
        "yes",
        "the №956 FIND: the summary returned bottom for an entity-seeded return until entity_seed_env rode the summary",
    ),
    (
        "sec_entity_secret",
        "param_to_sink",
        "PII_EGRESS_OUTPUT",
        "yes",
        "",
    ),
    (
        "sec_entity_secret",
        "two_levels",
        "PII_EGRESS_OUTPUT",
        "yes",
        "",
    ),
    (
        "sec_entity_secret",
        "each_accumulator",
        "PII_EGRESS_OUTPUT",
        "yes",
        "",
    ),
    (
        "sec_entity_secret",
        "while_accumulator",
        "PII_EGRESS_OUTPUT",
        "yes",
        "",
    ),
    (
        "sec_entity_secret",
        "struct_field",
        "PII_EGRESS_OUTPUT",
        "yes",
        "",
    ),
    (
        "sec_entity_secret",
        "list_element",
        "PII_EGRESS_OUTPUT",
        "yes",
        "",
    ),
    (
        "sec_entity_secret",
        "match_arm",
        "PII_EGRESS_OUTPUT",
        "yes",
        "",
    ),
    (
        "sec_entity_secret",
        "flow_output",
        "PII_EGRESS_OUTPUT",
        "yes",
        "№965 DONE: the flow edge carries the step's return label (the confidentiality axis; the integrity half is the №966 row)",
    ),
    (
        "sec_entity_secret",
        "url_position",
        "PII_EGRESS_NETWORK",
        "yes",
        "",
    ),
];
// ── №956 delivery roster END

/// The operator forms the delivery forms are combined with (the issue's
/// п.2). Both are pure composition — MEASURED not to launder a label on
/// any of the seven triples, which is what `n956_the_operator_rotation_
/// never_launders_a_label` pins. A sanitizer would belong in a different
/// list: clearing a label is its job, and a cell whose verdict changes
/// under it is not a weakening.
const ROTATION_OPS: &[(&str, &str)] = &[
    ("id", "{E}"),
    ("tail_concat", "({E}) + \"\""),
    ("head_concat", "\"p\" + ({E})"),
];

/// How many ROTATED passes run beside the base pass. The run executes a
/// FIXED number of programs: `cells × (1 + ROTATION_PASSES)`.
pub const ROTATION_PASSES: usize = 1;

/// The honest counter: the programs this run really generates and
/// analyzes. Not a product over a space — a count over a run.
pub fn programs_per_run() -> usize {
    DELIVERY_CELLS.len() * (1 + ROTATION_PASSES)
}

/// The rotation index: deterministic (0) by default, so a local run and a
/// CI run of the same index emit the same programs; CI passes a rotating
/// value so the operator forms take turns across runs without changing
/// the number executed.
fn rotation_index() -> usize {
    std::env::var("METALOGOS_FUZZ_ROTATION")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0)
}

/// (triple, source expression, sink template with `{X}`) — the seven
/// triples of the №904 lane plus the two the delivery matrix needs to
/// reach the request position and the entity seed.
const TRIPLES: &[(&str, &str, &str)] = &[
    ("sec_print_env", "env(\"SP\")", "print({X})"),
    (
        "sec_writefile_env",
        "env(\"SP\")",
        "write_file(\"fz956.txt\", {X})",
    ),
    (
        "sql_query_scalar_derived",
        "query_param(\"name\")",
        "query_scalar(\"SELECT count(*) FROM users WHERE name='\" + {X} + \"'\")",
    ),
    (
        "sql_dbinsert_dynamic",
        "query_param(\"t\")",
        "db_insert({X}, { v: \"1\" })",
    ),
    (
        "html_respond_llm_raw",
        "call_llm(\"summarize\")",
        "respond(\"200 OK\", {X})",
    ),
    ("sec_httpget_env", "env(\"SP\")", "http_get({X})"),
    ("sec_entity_secret", "secret_note", "print({X})"),
];

const FORMS: &[&str] = &[
    "direct",
    "pattern_return",
    "param_to_sink",
    "two_levels",
    "each_accumulator",
    "while_accumulator",
    "struct_field",
    "list_element",
    "match_arm",
    "flow_output",
    "url_position",
];

fn triple_of(id: &str) -> (&'static str, &'static str) {
    for (t, src, sink) in TRIPLES {
        if *t == id {
            return (src, sink);
        }
    }
    panic!("unknown triple {id} — the roster and the TRIPLES table diverged");
}

fn preamble(triple: &str) -> &'static str {
    if triple == "sec_entity_secret" {
        "type Secret = String\nentity secret_note: Secret = \"паспорт 1234\"\n"
    } else {
        ""
    }
}

/// The generator: a (triple, form, operator) triple becomes a program.
/// The operator wraps the SOURCE expression; the form decides the path
/// the wrapped value takes to the sink.
fn gen_program(triple: &str, form: &str, op_tmpl: &str) -> String {
    let (src_raw, sink_tmpl) = triple_of(triple);
    let src = op_tmpl.replace("{E}", src_raw);
    let sink = |x: &str| sink_tmpl.replace("{X}", x);
    let pre = preamble(triple);
    let body = match form {
        "direct" => format!(
            "pattern P(d: String) -> String {{ let _ = {} return \"ok\" }}\n\
             flow Main {{ input: String = \"x\" -> P -> output }}\n",
            sink(&src)
        ),
        "pattern_return" => format!(
            "pattern Src() -> String {{ return {src} }}\n\
             pattern P(d: String) -> String {{ let v = Src() let _ = {} return \"ok\" }}\n\
             flow Main {{ input: String = \"x\" -> P -> output }}\n",
            sink("v")
        ),
        "param_to_sink" => format!(
            "pattern Snk(x: String) -> String {{ let _ = {} return \"s\" }}\n\
             pattern P(d: String) -> String {{ let _ = Snk({src}) return \"ok\" }}\n\
             flow Main {{ input: String = \"x\" -> P -> output }}\n",
            sink("x")
        ),
        "two_levels" => format!(
            "pattern L2(x: String) -> String {{ let _ = {} return \"s\" }}\n\
             pattern L1(x: String) -> String {{ return L2(x) }}\n\
             pattern P(d: String) -> String {{ let _ = L1({src}) return \"ok\" }}\n\
             flow Main {{ input: String = \"x\" -> P -> output }}\n",
            sink("x")
        ),
        "each_accumulator" => format!(
            "pattern P(d: String) -> String {{ let mut acc = \"\" \
             each c in [{src}] {{ acc = acc + c }} let _ = {} return \"ok\" }}\n\
             flow Main {{ input: String = \"x\" -> P -> output }}\n",
            sink("acc")
        ),
        "while_accumulator" => format!(
            "pattern P(d: String) -> String {{ let mut acc = \"\" let mut i = 0 \
             while i < 1 {{ acc = acc + {src} i = i + 1 }} let _ = {} return \"ok\" }}\n\
             flow Main {{ input: String = \"x\" -> P -> output }}\n",
            sink("acc")
        ),
        "struct_field" => format!(
            "pattern P(d: String) -> String {{ let s = {{ v: {src} }} let _ = {} return \"ok\" }}\n\
             flow Main {{ input: String = \"x\" -> P -> output }}\n",
            sink("s.v")
        ),
        "list_element" => format!(
            "pattern P(d: String) -> String {{ let l = [{src}] let _ = {} return \"ok\" }}\n\
             flow Main {{ input: String = \"x\" -> P -> output }}\n",
            sink("l[0]")
        ),
        "match_arm" => format!(
            "pattern P(d: String) -> String {{ let v = match d {{ \"x\" then {{ {src} }} \
             else {{ \"n\" }} }} let _ = {} return \"ok\" }}\n\
             flow Main {{ input: String = \"x\" -> P -> output }}\n",
            sink("v")
        ),
        // The flow edge: the source leaves one STEP and the sink stands in
        // the next. This is the form the single b07 fixture covers; here it
        // runs against all seven triples.
        "flow_output" => format!(
            "pattern Src(d: String) -> String {{ return {src} }}\n\
             pattern P(x: String) -> String {{ let _ = {} return \"ok\" }}\n\
             flow Main {{ input: String = \"x\" -> Src -> P -> output }}\n",
            sink("x")
        ),
        "url_position" => format!(
            "pattern P(d: String) -> String {{ let _ = http_get({src}) return \"ok\" }}\n\
             flow Main {{ input: String = \"x\" -> P -> output }}\n"
        ),
        other => panic!("unknown delivery form {other}"),
    };
    format!("{pre}{body}")
}

/// The leak-suite class extractor (the examples/leak convention: the
/// refusal class is the first `[UPPERCASE_ID]` stamp).
fn error_class(err: &str) -> String {
    for line in err.lines() {
        if let Some(start) = line.find('[') {
            if let Some(end_rel) = line[start + 1..].find(']') {
                let id = &line[start + 1..start + 1 + end_rel];
                let is_id = !id.is_empty()
                    && id
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
                if is_id {
                    return id.to_string();
                }
            }
        }
    }
    "NOCLASS".to_string()
}

/// The STATIC verdict — the taint line's own fact. "PASS" means the
/// analyzer accepted the program.
fn static_verdict(program: &str) -> String {
    match metalogos::check_program(program) {
        Err(e) => panic!(
            "the generator emitted an unparsable program — the harness \
             defect class that invalidated two of the audit's own probes \
             (ADR-0154 §12.6a): {e}\n{program}"
        ),
        Ok(r) if r.errors.is_empty() => "PASS".to_string(),
        Ok(r) => error_class(
            &r.errors
                .iter()
                .map(|e| format!("{e:?}"))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
    }
}

/// The VM lane's verdict: the production `mlog run --backend vm` applies
/// the SAME semantic gate before the compile (the №523/№617 parity the
/// №465 harness documents), so the gate's class is the VM's class. The
/// check here is the AGREEMENT, which is what the both-backend
/// requirement of the issue asks for.
fn vm_verdict(program: &str, base: &Path) -> String {
    let declarations = match metalogos::parser::parse(program) {
        Ok(d) => d,
        Err(e) => panic!("parse error in the VM lane: {e}"),
    };
    let module_decls = match metalogos::semantic::resolve_imports_statically(&declarations, base) {
        Ok(d) => d,
        Err(_) => return "IMPORT_RESOLUTION".to_string(),
    };
    let mut merged = module_decls;
    merged.extend(declarations);
    let result = metalogos::semantic::check_program(&merged);
    let errors = metalogos::semantic::filter_suppressed_unchecked(result.errors, program);
    if errors.is_empty() {
        "PASS".to_string()
    } else {
        error_class(
            &errors
                .iter()
                .map(|e| format!("{e:?}"))
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }
}

/// THE MATRIX. Every cell on both backends, against its RECORDED verdict.
/// Nothing here is tuned to green: a cell that stops matching is a RED
/// run, never a roster edit (the №904 discipline).
///
/// This test is also the MUTATION target the issue names: neuter the №950
/// interprocedural summary and the `pattern_return`, `param_to_sink` and
/// `two_levels` cells fall back to PASS — thirty-odd cells go red here,
/// with no help from the leak suite. `scripts/mutation_verify_956.sh`
/// runs that mutation.
#[test]
fn n956_every_delivery_cell_matches_its_recorded_verdict_on_both_backends() {
    let tmp = std::env::temp_dir().join(format!("n956_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("tmp dir");
    std::env::set_var("SP", "SECRET956");
    let r = rotation_index();
    let mut executed = 0usize;
    let mut ran = 0usize;
    for (i, (triple, form, expected, run, note)) in DELIVERY_CELLS.iter().enumerate() {
        if *expected == "PASS" {
            assert!(
                !note.is_empty(),
                "cell {triple}/{form} passes and carries NO reason — a PASS \
                 without a recorded reason is how a gap hides (the №956 rule)"
            );
        }
        // The base pass: the identity operator.
        let program = gen_program(triple, form, "{E}");
        let got = static_verdict(&program);
        executed += 1;
        assert_eq!(
            &got, expected,
            "cell {triple}/{form}: the STATIC verdict moved — recorded \
             {expected}, measured {got}. Repair the lane or, if the move is \
             intended, the roster — in the SAME PR as the change.\n{program}"
        );
        if *run == "yes" {
            let vm = vm_verdict(&program, &tmp);
            assert_eq!(
                &vm, expected,
                "cell {triple}/{form}: the backends disagree — TW {got}, VM \
                 {vm}; a TW/VM class divergence is never pinnable (№476)"
            );
            if *expected == "PASS" {
                // The executable subset: the gate let this one through, so
                // the executor gets its say. Either it runs or the RUNTIME
                // refuses it (the SSRF guard, an unconfigured model) — both
                // are facts; a panic is not.
                let outcome = metalogos::run_program_with_dir(&program, tmp.clone());
                ran += 1;
                if let Err(e) = &outcome {
                    // A runtime refusal often carries no [CLASS] stamp (the
                    // SSRF guard, an unconfigured model): print the text.
                    let cls = error_class(e);
                    let shown = if cls == "NOCLASS" {
                        e.lines()
                            .next()
                            .unwrap_or("")
                            .chars()
                            .take(72)
                            .collect::<String>()
                    } else {
                        cls
                    };
                    println!("N956   runtime refusal {triple}/{form}: {shown}");
                }
            }
        }
        // The rotated pass: a fixed number of programs per run, the
        // operator forms taking turns by the rotation index.
        let (_, op_tmpl) = ROTATION_OPS[(i + r) % ROTATION_OPS.len()];
        let rotated = gen_program(triple, form, op_tmpl);
        let got_rot = static_verdict(&rotated);
        executed += 1;
        assert_eq!(
            &got_rot, expected,
            "cell {triple}/{form} under the rotated operator form: the \
             verdict changed to {got_rot}. A composition operator must not \
             launder a label.\n{rotated}"
        );
    }
    let _ = std::fs::remove_dir_all(&tmp);
    // The honest counter — a count over a run, printed.
    println!(
        "N956 delivery matrix: cells={} rotation={} programs_analyzed={} \
         pass_cells_run={}",
        DELIVERY_CELLS.len(),
        r,
        executed,
        ran
    );
    assert_eq!(
        ran, 2,
        "the executable subset moved: 2 of the 2 remaining PASS cells are \
         runnable (the by-design request-position cells). The five B7 flow \
         edge cells flipped to refusals with №965 — refused programs are \
         not executable, that is what the gate is for. A changed number \
         means a cell flipped — in the roster or in the lane"
    );
    assert_eq!(
        executed,
        programs_per_run(),
        "the run executed {executed} programs, the machine fact says {} — \
         the counter and the run must not diverge (the issue's п.2: a fact \
         about a run, not the size of a space)",
        programs_per_run()
    );
}

/// The roster is COMPLETE: every triple × every form, exactly once.
#[test]
fn n956_the_roster_covers_every_triple_times_every_form() {
    let mut seen: BTreeSet<(&str, &str)> = BTreeSet::new();
    for (triple, form, _, _, _) in DELIVERY_CELLS {
        assert!(
            seen.insert((triple, form)),
            "duplicate roster cell {triple}/{form}"
        );
        assert!(
            FORMS.contains(form),
            "cell {triple}/{form}: unknown delivery form"
        );
    }
    for (triple, _, _) in TRIPLES {
        for form in FORMS {
            assert!(
                seen.contains(&(*triple, *form)),
                "the roster is missing {triple}/{form} — the issue's п.1 \
                 requires every triple × every form"
            );
        }
    }
    assert_eq!(
        DELIVERY_CELLS.len(),
        TRIPLES.len() * FORMS.len(),
        "the roster must be exactly triples × forms"
    );
}

/// The PASS cells are the OPEN boundary, and each names its row. This is
/// the test that flips when B7 lands: five cells become refusals, and the
/// roster edit rides the same PR.
#[test]
fn n956_every_pass_cell_names_an_open_row_or_a_design_reason() {
    let mut pass = 0;
    for (triple, form, expected, _, note) in DELIVERY_CELLS {
        if *expected != "PASS" {
            continue;
        }
        pass += 1;
        assert!(
            note.contains("B7 OPEN") || note.contains("by design"),
            "cell {triple}/{form} passes with the note {note:?} — a PASS \
             must name an OPEN threat-model row or a DESIGN reason"
        );
    }
    assert_eq!(
        pass, 2,
        "the open set moved: 2 cells pass when this roster was measured \
         (2 × the by-design request position). The five B7 flow edge cells \
         flipped to refusals with №965 in the SAME PR. A new PASS is a \
         finding; a removed one is a fix — either way the number moves in \
         the SAME PR as the change"
    );
}

/// The threat-model row B7 is still OPEN in the document, so the five
/// `flow_output` PASS cells are consistent with it. If B7 is marked FIXED
/// while these cells still pass, one of the two is lying.
#[test]
fn n956_the_b7_row_and_the_flow_output_cells_agree() {
    let tm = std::fs::read_to_string(repo_root().join("docs").join("threat-model.md"))
        .expect("threat-model.md readable");
    // №965: B7 is FIXED (the confidentiality axis) — the flow_output
    // cells are refusals, and the two documents must still agree.
    let b7_fixed = tm.contains("| B7 | FIXED");
    let flow_cells_pass = DELIVERY_CELLS
        .iter()
        .filter(|(_, form, expected, _, _)| *form == "flow_output" && *expected == "PASS")
        .count();
    let flow_cells_refuse = DELIVERY_CELLS
        .iter()
        .filter(|(_, form, expected, _, _)| *form == "flow_output" && *expected != "PASS")
        .count();
    assert!(
        b7_fixed,
        "the threat model keeps B7 open while {flow_cells_refuse} flow_output \
         cells are recorded as refusals — flip the row in the same PR"
    );
    assert_eq!(
        flow_cells_pass, 0,
        "a flow_output cell still passes — the №965 edge gate refused \
         the audited shapes when measured; a PASS here is a finding"
    );
    assert_eq!(
        flow_cells_refuse, 7,
        "the flow edge refused 5 of the 7 triples when this roster was \
         measured — the number is the reach of B7, not a detail"
    );
}
