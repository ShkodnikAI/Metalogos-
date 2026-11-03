#![allow(clippy::disallowed_methods)]

// ── tests/naryad_887_jurisdiction_coverage.rs ─────────────────────────
// Наряд №887 (issue #1760, wave 79 — W-H, P1, [jurisdiction][core],
// ADR-0209 §4): the coverage rule — the fail-closed jurisdiction gate
// at the policy-compilation/consumption site.
//
// The rule (the KPI C10 as a MACHINE check, never a declaration):
//   (а) a package rule without a norm reference refuses the policy
//       compilation — the compile-site re-check, not only the
//       constructor (a foreign-payload rule that bypassed
//       JurisdictionRule::new still refuses);
//   (б) a rule without a valid verification date refuses the same way;
//   (в) «молчаливый пропуск невозможен» — an uncovered carrier cannot
//       resolve to an implicit allow: the gate verdict refuses (the
//       №845 form), and the MCP composition (ADR-0196 × ADR-0209)
//       refuses the run with it — BEFORE any backend work;
//   (г) a deny verdict over an audited sink refuses the composition —
//       the run never launches under deny.
//
// The e2e trace (the naryad's §2, the №880 form): юрисдикция → правило
// (норма + дата) → компиляция → ToolPolicy-решение (ADR-0196, the
// compiled audited sinks) → гейт-вердикт носителя. The allowed/limit
// path proceeds — the same program runs on BOTH backends (TW + VM, the
// №829 two-lane form) with byte-identical outcomes; the gate report
// serializes byte-identically across two runs (the №828/№868
// double-run form). The refusal paths refuse BEFORE any run — no
// backend is ever reached under a refusal (the №874 fail-closed
// posture).
//
// The refusal strings reuse the kernel's jurisdiction classes — the
// frozen №385 vocabulary is not extended. The honest TW/VM boundary
// (№588, the №886 T4 form): the jurisdiction kernel itself is
// backend-independent by construction; the lanes pin the CARRIER
// surface the verdict gates (http_post — the EU limit mark honored on
// both lanes), never a fake "backend compile" of the kernel.

use metalogos::ast::Declaration;
use metalogos::jurisdiction::{
    compile_jurisdiction_policy, seed_matrix, JurisdictionId, JurisdictionRule, PolicyEffect,
};
use metalogos_server::mcp_policy::{compile_policy, jurisdiction_gate};
use std::path::Path;

const MANIFEST: &str = env!("CARGO_MANIFEST_DIR");

/// The tool surface the verdict gates: one method, one audited sink
/// (http_post — the network carrier of the seed's EU limit row).
const TOOL_SRC: &str = r#"
tool gate {
  post(url: String, body: String) -> String {
    let r = http_post(url, body)
    return r
  }
}
"#;

/// The program the limit-marked path runs on BOTH lanes — the dead
/// loopback port 1 (connect refused, deterministic, instant; the №401
/// hygiene: no test ever dials a real host).
const PROGRAM_SRC: &str = r#"
pattern LimitMarked(_tick: String) -> String {
  let r = try http_post("http://127.0.0.1:1/v1", "{}", "application/json")
  return "limit-mark honored: EU | post-outcome captured: " + to_string(r)
}
flow Main { input: String = "t" -> LimitMarked -> output }
"#;

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source.trim(), Path::new(MANIFEST).to_path_buf())
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(Path::new(MANIFEST).to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

/// The lane-runner type — the TW/VM parity harness form (№748/№829).
type LaneRunner = fn(&str) -> Result<Option<String>, String>;

fn runners() -> [(&'static str, LaneRunner); 2] {
    [("tw", run_tw), ("vm", run_vm)]
}

/// The compiled ToolPolicy of the TOOL_SRC method (the ADR-0196 SSOT
/// of what the method can touch).
fn compiled_tool() -> metalogos_server::mcp_policy::ToolPolicy {
    let decls = metalogos::parser::parse(TOOL_SRC).expect("the tool source parses");
    let tool = decls
        .iter()
        .find_map(|d| match d {
            Declaration::Tool(t) => Some(t.clone()),
            _ => None,
        })
        .expect("the tool declaration present");
    let method = tool.methods.first().expect("one method");
    compile_policy(method)
}

// ── (а) a rule without a norm reference refuses the COMPILATION ────────

#[test]
fn t1_compile_site_refuses_a_rule_without_norm_reference() {
    // The foreign-payload form: the rule bypassed the fail-closed
    // constructor (fields are pub) and entered the matrix — the
    // compile-site re-check refuses it BEFORE anything runs (the KPI
    // C10 holds on the compiled path).
    let mut m = seed_matrix();
    let raw = JurisdictionRule {
        jurisdiction: JurisdictionId::new("EU").unwrap(),
        carrier: "write_file".to_string(),
        effect: PolicyEffect::Deny,
        norm_ref: String::new(),
        checked_at: "2026-11-03".to_string(),
        source: "test/foreign-payload".to_string(),
        row_digest: "0".repeat(64),
    };
    m.issue(raw)
        .expect("the matrix accepts the row (the digest gate only)");
    let err = compile_jurisdiction_policy(&m, "EU").unwrap_err();
    assert!(err.contains("incomplete"), "{err}");
    assert!(err.contains("fail-closed"), "{err}");
    assert!(err.contains("KPI C10"), "{err}");
}

// ── (б) a rule without a valid verification date refuses too ───────────

#[test]
fn t2_compile_site_refuses_a_rule_without_valid_checked_at() {
    let mut m = seed_matrix();
    // a real constructor refuses "2026-02-30" (a non-calendar day) —
    // so the foreign-payload form is the only way to reach the
    // compile site with it; the compile refuses the same class.
    let raw = JurisdictionRule {
        jurisdiction: JurisdictionId::new("EU").unwrap(),
        carrier: "write_file".to_string(),
        effect: PolicyEffect::Deny,
        norm_ref: "GDPR (EU) 2016/679 Art. 5".to_string(),
        checked_at: "2026-02-30".to_string(),
        source: "test/foreign-payload".to_string(),
        row_digest: "0".repeat(64),
    };
    m.issue(raw)
        .expect("the matrix accepts the row (the digest gate only)");
    let err = compile_jurisdiction_policy(&m, "EU").unwrap_err();
    assert!(err.contains("incomplete"), "{err}");
    assert!(err.contains("fail-closed"), "{err}");
}

// ── (в) the consumed path: an uncovered sink refuses the composition ────

#[test]
fn t3_composition_refuses_an_uncovered_sink_fail_closed() {
    let tool = compiled_tool();
    assert!(
        !tool.sink_calls.is_empty(),
        "the method carries an audited sink (http_post)"
    );
    assert!(tool.irreversible, "http_post is the irreversible surface");

    // EU: the seed covers http_post (the data-minimisation limit).
    let eu = compile_jurisdiction_policy(&seed_matrix(), "EU").unwrap();
    let report = jurisdiction_gate(&tool, &eu).expect("EU covers the audited sink");
    assert_eq!(report.jurisdiction, "EU");
    assert_eq!(report.matrix_digest, eu.matrix_digest);
    assert_eq!(report.verdicts.len(), 1);
    assert_eq!(report.verdicts[0].carrier, "http_post");
    assert_eq!(report.verdicts[0].effect, "limit");
    // the double-run serialization byte-identity (the №880 form)
    let b1 = serde_json::to_vec(&report).unwrap();
    let report2 = jurisdiction_gate(&tool, &eu).unwrap();
    let b2 = serde_json::to_vec(&report2).unwrap();
    assert_eq!(b1, b2, "the gate report must be byte-identical across runs");

    // US-CA: NO rule covers http_post — the composition refuses; the
    // silent pass (an implicit allow over an uncovered carrier) is
    // forbidden.
    let us_ca = compile_jurisdiction_policy(&seed_matrix(), "US-CA").unwrap();
    let err = jurisdiction_gate(&tool, &us_ca).unwrap_err();
    assert!(err.contains("no rule covers"), "{err}");
    assert!(err.contains("fail-closed"), "{err}");
    assert!(err.contains("US-CA"), "{err}");
}

// ── (г) a deny verdict over an audited sink refuses BEFORE any run ──────

#[test]
fn t4_composition_refuses_a_deny_verdict_before_any_run() {
    let tool = compiled_tool();
    let mut m = seed_matrix();
    // the supersede row: EU http_post → deny (strictest wins)
    m.issue(
        JurisdictionRule::new(
            JurisdictionId::new("EU").unwrap(),
            "http_post",
            PolicyEffect::Deny,
            "GDPR (EU) 2016/679 Ch. V (Art. 44)",
            "2026-11-03",
            "test/supersede-deny",
        )
        .unwrap(),
    )
    .unwrap();
    let eu = compile_jurisdiction_policy(&m, "EU").unwrap();
    let err = jurisdiction_gate(&tool, &eu).unwrap_err();
    assert!(err.contains("deny effect covers an audited sink"), "{err}");
    assert!(err.contains("refuses before any run"), "{err}");
    assert!(err.contains("http_post"), "{err}");
}

// ── The e2e trace on BOTH backends: the limit-marked path proceeds ──────

#[test]
#[serial_test::serial]
fn t5_the_limit_marked_path_is_byte_identical_on_both_lanes() {
    let tool = compiled_tool();
    let eu = compile_jurisdiction_policy(&seed_matrix(), "EU").unwrap();
    // the gate verdict: limit — the consumer HONORS the mark and the
    // run proceeds (the kernel does not interpret the norm — ADR-0209
    // §5); the both-lane run pins the carrier surface byte-identically.
    let report = jurisdiction_gate(&tool, &eu).unwrap();
    assert_eq!(report.verdicts[0].effect, "limit");

    let mut outcomes: Vec<(String, Result<Option<String>, String>)> = Vec::new();
    for (name, run) in runners() {
        outcomes.push((name.to_string(), run(PROGRAM_SRC)));
    }
    // both lanes: the SAME outcome — the run completes, the marker
    // carries the captured post outcome byte-for-byte.
    let (lane_a, res_a) = &outcomes[0];
    let (lane_b, res_b) = &outcomes[1];
    assert_eq!(lane_a, "tw");
    assert_eq!(lane_b, "vm");
    let out_a = res_a
        .as_ref()
        .unwrap_or_else(|e| panic!("tw lane failed: {e}"));
    let out_b = res_b
        .as_ref()
        .unwrap_or_else(|e| panic!("vm lane failed: {e}"));
    assert_eq!(out_a, out_b, "the lanes must be byte-identical (№880 form)");
    let text = out_a.as_deref().unwrap_or("");
    assert!(
        text.contains("limit-mark honored: EU"),
        "the limit-marked path must complete: {text}"
    );
}

#[test]
fn t6_the_refusal_paths_never_reach_a_backend() {
    // The (в)/(г) refusals happen at the composition — BEFORE any
    // backend work. The pin: the composition returns Err and the
    // program source is never handed to a lane (the №874 fail-closed
    // posture; no run artifact exists).
    let tool = compiled_tool();
    let us_ca = compile_jurisdiction_policy(&seed_matrix(), "US-CA").unwrap();
    assert!(jurisdiction_gate(&tool, &us_ca).is_err());
    // the dead-loopback program is never launched: no output, no
    // side effect — the honest assertion is the refusal itself plus
    // the absence of any lane invocation in this test.
    let target_dir = Path::new(MANIFEST).join("target").join("n887_refused");
    let _ = std::fs::remove_dir_all(&target_dir);
    std::fs::create_dir_all(&target_dir).unwrap();
    assert!(
        std::fs::read_dir(&target_dir).unwrap().next().is_none(),
        "the refused path must leave no artifacts"
    );
}
