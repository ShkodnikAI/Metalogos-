// ── №756 (gh#1408, Wave 57): the two lane divergences from the №751
// (gh#1397) (α)-walk — RESOLVED by the parity pins ────────────────────
//
// The В57 dispatch (gh#1411) ordered the [compiler] naryad on the walk's
// two discoveries. The resolution contract (both semantical, both OUTSIDE
// the types movement — the ADR-0194 Запреты hold, the type floors do not
// move):
//
//   1. resolve_skill_index — ONE shape on BOTH lanes now:
//      type_name = format!("SkillIndex_{dept}") (the TW canon; the VM's
//      fixed "SkillIndex" is gone — vm.rs run-loop arm) and the
//      truncation field carries on BOTH (the compiled field —
//      bytecode.rs CompiledSkillIndex.truncation — the compiler carries
//      si.truncation, the VM arm inserts it; was TW-only).
//
//   2. The absent-field READ is LOUD on both lanes (the ADR-0194
//      package A = (c) contract): the VM GetField silent Unit fallback
//      is REMOVED on both VM execution loops (run() and
//      execute_code_inner()) — the get_field Err propagates, the same
//      string the TW FieldAccess always propagated (values.rs get_field).
//
// The №751 interlock pins hold unchanged: the shared SHAPE probe
// (type_of = Struct) does not pin the type_name.
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

fn assert_both_err_contain(src: &str, needle: &str) {
    let tw_err = run_tw(src)
        .err()
        .unwrap_or_else(|| panic!("TW must refuse loudly\n{src}"));
    assert!(
        tw_err.contains(needle),
        "TW error must name the fact ({needle}); got: {tw_err}\n{src}"
    );
    let vm_err = run_vm(src)
        .err()
        .unwrap_or_else(|| panic!("VM must refuse loudly\n{src}"));
    assert!(
        vm_err.contains(needle),
        "VM error must name the fact ({needle}); got: {vm_err}\n{src}"
    );
}

const SKILL_INDEX_WHOLE: &str = r#"
skill_index eng {
  tier 1 always {
    ["alpha_skill", "beta_skill"]
  }
  budget: 100 tokens,
  truncation: whole_skill_only
}
pattern P(_t: String) -> Bool {
  let idx = resolve_skill_index("eng")
  return contains(str(idx), "SkillIndex_eng {")
}
flow Main { input: String = "x" -> P -> output }
"#;

// ── Divergence 1a: the type_name parity (the dept-qualified canon) ──

#[test]
fn n756_type_name_is_dept_qualified_on_both_lanes() {
    // The TW canon spelling rides BOTH lanes now (was: the VM fixed
    // "SkillIndex" — vm.rs:2519 pre-№756). The Display projection carries
    // the type_name (`SkillIndex_eng { ... }` — values.rs Display), so
    // contains(str(idx), "SkillIndex_eng {") is TRUE on both lanes and
    // the pre-№756 VM answer would fail the same probe ("SkillIndex {").
    assert_both(SKILL_INDEX_WHOLE, "true");
}

#[test]
fn n756_type_name_does_not_answer_the_fixed_form() {
    // The negative control: the fixed "SkillIndex {" spelling (the VM
    // pre-№756) must NOT surface on either lane.
    let probe = r#"
skill_index eng {
  tier 1 always {
    ["alpha_skill"]
  }
}
pattern P(_t: String) -> Bool {
  let idx = resolve_skill_index("eng")
  return contains(str(idx), "SkillIndex {")
}
flow Main { input: String = "x" -> P -> output }
"#;
    assert_both(probe, "false");
}

// ── Divergence 1b: the truncation field parity (both modes + absent) ──

#[test]
fn n756_truncation_whole_skill_only_rides_both_lanes() {
    // The declaration's truncation: whole_skill_only surfaces in the
    // Struct on BOTH lanes (was TW-only — the VM arm ended at budget).
    let src = SKILL_INDEX_WHOLE.replace(
        "return contains(str(idx), \"SkillIndex_eng {\")",
        "return contains(str(idx), \"truncation: whole_skill_only\")",
    );
    assert_both(&src, "true");
}

#[test]
fn n756_truncation_truncate_at_boundary_rides_both_lanes() {
    let src = r#"
skill_index eng {
  tier 1 always {
    ["alpha_skill"]
  },
  truncation: truncate_at_boundary
}
pattern P(_t: String) -> Bool {
  let idx = resolve_skill_index("eng")
  return contains(str(idx), "truncation: truncate_at_boundary")
}
flow Main { input: String = "x" -> P -> output }
"#;
    assert_both(src, "true");
}

#[test]
fn n756_truncation_absent_stays_absent_on_both_lanes() {
    // No truncation in the declaration → no field in the answer (the
    // Option is None on BOTH lanes — no fictitious default).
    let src = r#"
skill_index eng {
  tier 1 always {
    ["alpha_skill"]
  }
}
pattern P(_t: String) -> Bool {
  let idx = resolve_skill_index("eng")
  return contains(str(idx), "truncation")
}
flow Main { input: String = "x" -> P -> output }
"#;
    assert_both(src, "false");
}

// ── Divergence 2: the absent-field READ is loud on BOTH lanes ──

#[test]
fn n756_absent_field_read_is_loud_on_both_lanes() {
    // The ADR-0194 package A = (c) contract: the missing field is a loud
    // Err naming the field on BOTH lanes (was: the VM silent Unit —
    // the №604 class-4 silently-wrong shape). The SkillIndex struct is
    // the live carrier here (any struct shares the same get_field).
    let src = r#"
skill_index eng {
  tier 1 always {
    ["alpha_skill"]
  }
}
pattern P(_t: String) -> Float {
  let idx = resolve_skill_index("eng")
  let x = idx.nothere
  return 1.0
}
flow Main { input: String = "x" -> P -> output }
"#;
    assert_both_err_contain(src, "field 'nothere' not found on struct");
}

#[test]
fn n756_non_struct_field_read_is_loud_on_both_lanes() {
    // The same contract on the non-struct arm of get_field.
    let src = r#"
pattern P(_t: String) -> Float {
  let n = 5.0
  let x = n.nothere
  return 1.0
}
flow Main { input: String = "x" -> P -> output }
"#;
    assert_both_err_contain(src, "cannot access field 'nothere' on non-struct value");
}

#[test]
fn n756_present_field_read_still_works_on_both_lanes() {
    // The loudness must not eat the happy path: the present fields read
    // through on both lanes (the tier1 List; the budget Float).
    let src = r#"
skill_index eng {
  tier 1 always {
    ["alpha_skill", "beta_skill"]
  }
  budget: 100 tokens
}
pattern P(_t: String) -> String {
  let idx = resolve_skill_index("eng")
  return type_of(idx.tier1) + "|" + type_of(idx.budget)
}
flow Main { input: String = "x" -> P -> output }
"#;
    assert_both(src, "List|Float");
}
