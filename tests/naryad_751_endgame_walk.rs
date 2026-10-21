// ── №751 (gh#1397, Wave 56): the 0.32 fixed-list movement №11 — the
// lane α ENDGAME: the machine walk of the 32-row residue + the (α)
// fixation ────────────────────────────────────────────────────────────
//
// The В56 dispatch (gh#1400) ordered the endgame walk of the residue
// (32 rows) by the live handler facts of BOTH lanes. The walk outcome:
// the honest ZERO on closures — lane α is EXHAUSTED: every remaining
// row's growth runs through the owner gates (the (β) state unions and
// the (γ) opaque kinds — the gate 7; the Secret spellings — ADR-0195
// §7 (c) parked; the data-dependent shapes; the №669 empty-List form;
// the never-Ok stubs; the true stub) or through the lane-parity fixes
// OUTSIDE the types movement (ADR-0194 Запреты — no semantics change).
//
// The walk's TWO discoveries (both lane-parity facts, both outside the
// movement — the candidate [compiler] naryads):
//
//   1. resolve_skill_index — the №717 "true-stub posture" anchor was
//      factually STALE: the handlers ARE LIVE on both lanes (TW
//      execution.rs:2537-2603, VM vm.rs:2466-2522 — the shared №466
//      group 7 validation, runtime_ops.rs:261-290) — but the lanes
//      DIVERGE: the TW type_name is format!("SkillIndex_{dept}")
//      (execution.rs:2600) while the VM's is the fixed "SkillIndex"
//      (vm.rs:2519), and the truncation field is TW-only (execution.rs:
//      2588-2598; the VM arm ends at budget — vm.rs:2515-2521). No
//      honest spelling survives the divergence (the №669 form: a
//      Struct<SkillIndex> would be fictitious on the TW lane) — the
//      row stays Unknown in the DIVERGENCE class (the ADR-0075
//      family).
//
//   2. The absent-field READ diverges across ALL structs: the TW
//      FieldAccess propagates the get_field error LOUDLY (execution.rs:
//      1743 — get_field returns Result, values.rs:386-394) while the
//      VM GetField falls back to the silent Unit (vm.rs:1232 — the
//      unwrap_or) — not in the ADR-0075 ledger; the loud-fail contract
//      (ADR-0194) candidate.
//
// The (β)/(γ)/parity DECISION PACKAGE for the owner (the gate 7):
// (β) state unions — session_poll_wake / session_take_interrupt /
//     backend_select / inspect; (γ) opaque kinds — the 9 unnamed
//     (memory_open/duplex_open/device_open/pose_make/trajectory_make/
//     goal_make/subgraph_extract/forecast_next/vision_load) + the 4
//     named-coarse (series_make/media_source_capture/secret/
//     generate_key) + reflex_load/reflex_bpe_load; the Secret
//     spellings — secret/generate_key/memory_read (ADR-0195 §7 (c));
//     the parity fixes — resolve_skill_index + the GetField loudness;
//     the data-dependent trio (call_llm_schema/config_load/find) and
//     the candle-gated reflex_generate stay semantics-gated.
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

// ── The corrected anchor: LIVE on both lanes (the stub claim is dead) ──

#[test]
fn n751_resolve_skill_index_is_live_on_both_lanes() {
    // The declared index answers the Struct on BOTH lanes (the №717
    // "true stub" anchor was factually stale — the walk's discovery 1):
    // the struct shape (tier1 List, the when_matches tier2 List of the
    // TriggerRule structs — the SAME type_name on both lanes:
    // execution.rs:2576 / vm.rs:2507) is the parity ground; the budget
    // field inserts on both lanes.
    let src = r#"
skill_index eng {
  tier 1 always {
    ["alpha_skill", "beta_skill"]
  }
  tier 2 when_matches {
    [{ skill: "market_skill", triggers: ["рынок", "актив"] }]
  }
  budget: 100 tokens
}
pattern P(_t: String) -> String {
  let idx = resolve_skill_index("eng")
  return type_of(idx) + "|" + type_of(idx.tier1) + "|"
    + type_of(idx.tier2) + "|" + type_of(get(idx.tier2, 0))
}
flow Main { input: String = "x" -> P -> output }
"#;
    assert_both(src, "Struct|List|List|Struct");
}

#[test]
fn n751_the_no_declaration_arm_refuses_loud_on_both_lanes() {
    // Without a declared index the call refuses LOUDLY naming the
    // builtin on BOTH lanes (execution.rs:2541-2546 / vm.rs:2475-2480)
    // — the error channel, not a shape (the deny_event/query №740
    // posture: the context-less refusal does not make a union).
    let src = r#"
pattern P(_t: String) -> String {
  let idx = resolve_skill_index("nodept")
  return type_of(idx)
}
flow Main { input: String = "x" -> P -> output }
"#;
    let tw = run_tw(src).expect_err("TW must refuse loudly (no declared index)");
    assert!(
        tw.contains("resolve_skill_index"),
        "TW refusal must name the builtin: {tw}"
    );
    let vm = run_vm(src).expect_err("VM must refuse loudly (no declared index)");
    assert!(
        vm.contains("resolve_skill_index"),
        "VM refusal must name the builtin: {vm}"
    );
}

// ── The divergence pins: why NO spelling survives (the honest zero) ──

#[test]
fn n751_the_type_name_and_field_set_diverge_across_lanes() {
    // The divergence ground, pinned BEHAVIORALLY: the truncation field
    // is TW-only (execution.rs:2588-2598 vs the VM arm ending at
    // budget — vm.rs:2515-2521). The SAME program: TW answers "String"
    // (the field exists), the VM answers the silent Unit (vm.rs:1232,
    // the GetField unwrap_or — the walk's discovery 2). A
    // Struct<SkillIndex> spelling would be fictitious on the TW lane
    // anyway (the type_name format!("SkillIndex_{dept}") —
    // execution.rs:2600 — vs the fixed "SkillIndex" — vm.rs:2519); the
    // row stays Unknown honest.
    let src = r#"
skill_index eng {
  tier 1 always {
    ["alpha_skill"]
  }
  budget: 100 tokens,
  truncation: whole_skill_only
}
pattern P(_t: String) -> String {
  let idx = resolve_skill_index("eng")
  return type_of(idx.truncation)
}
flow Main { input: String = "x" -> P -> output }
"#;
    let tw = run_tw(src)
        .unwrap_or_else(|e| panic!("TW run failed: {e}\n{src}"))
        .unwrap_or_default();
    assert_eq!(
        tw.trim_end(),
        "String",
        "TW carries the truncation field (execution.rs:2588-2598)\n{src}"
    );
    let vm = run_vm(src)
        .unwrap_or_else(|e| panic!("VM run failed: {e}\n{src}"))
        .unwrap_or_default();
    assert_eq!(
        vm.trim_end(),
        "Unit",
        "VM silently Units the absent field (vm.rs:1232) — the divergence pin\n{src}"
    );
}

#[test]
fn n751_the_divergence_row_stays_unknown_honest() {
    // The registry ground: the divergence class keeps the honest
    // Unknown (the closure is blocked by the lane divergence, NOT by a
    // stub — the №717 anchor class is dead; the interlock of
    // tests/naryad_730_type_closures.rs holds).
    let s = spec_of("resolve_skill_index");
    assert_eq!(s.return_type, Type::Unknown);
    assert_eq!(s.field_meta, "");
}
