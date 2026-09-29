// ── Наряд №510 (gh#794, Wave 21 P0) — the `!=` operator end-to-end ──
//
// Audit 28.09 (C-01, fact-check t85 60/60 PASS): `!=` compiled as `==`
// through `_ =>` wildcard arms in the compiler (rule conditions, flow
// branches, mutate rollback_if). Fixing the three wildcard sites
// surfaced the DEEPER truths this file pins:
//
//   1. The compiler wildcards were only half of C-01. The grammar's
//      `compare_condition` rule could NEVER match any comparison: the
//      greedy left `expression` consumes the compare operator (compare_expr
//      is an infix layer of the expression grammar), so `rule If (a != b)`
//      — and `rule If (a > b)`, any comparison — failed to PARSE. Only
//      `contains` worked. The parser now decomposes the expression-shaped
//      condition into Condition::Compare (the loud, reachable path).
//
//   2. The VM's mutate kept-table disagreed with the TW interpreter for
//      every operator except Lt/Le: Gt/Ge always rolled back, Eq was
//      inverted, Ne was missing (the always-keep default). The table now
//      mirrors interpreter/hooks.rs exactly.
//
// Contracts (TW and VM must agree on every one):
//   C1: rule `!=` fires when the sides differ          (flag 0 → 1)
//   C2: rule `!=` does NOT fire when the sides are equal
//   C3: rule `==` regression — unchanged Eq semantics
//   C4: flow branch `!=` taken when the field differs
//   C5: flow branch `!=` not taken — BOTH backends fail loud
//       ("no branch matched"), consistently
//   C6: rollback_if `accuracy != 0.95` with mock accuracy 0.95 → KEPT
//   C7: rollback_if `accuracy != 0.90` with mock accuracy 0.95 → ROLLED BACK
//   C8: rollback_if `accuracy > 0.99` → KEPT (the VM Gt fix; mirrors the
//       naryad №148 C1 contract on the VM side)
//   C9: rollback_if `accuracy > 0.90` → ROLLED BACK (0.95 > 0.90)
//   C10: rollback_if `accuracy == 0.95` → ROLLED BACK (roll back if equal,
//        the №206 contract — the VM Eq inversion is gone)
//
// Mock accuracy is always 0.95 (explicit METALOGOS_MOCK_LLM=1 opt-in, Н454).

use std::path::{Path, PathBuf};

fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.to_path_buf()).map_err(|e| e.to_string())
}

fn run_vm(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp.compile(declarations).map_err(|e| e.to_string())?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program).map_err(|e| e.to_string())
}

fn base() -> PathBuf {
    PathBuf::from(".")
}

fn trim(s: Result<Option<String>, String>) -> String {
    match s {
        Ok(Some(out)) => out.trim_end().to_string(),
        Ok(None) => String::new(),
        Err(e) => format!("ERR: {}", e),
    }
}

// ── C1: rule `!=` fires on difference ───────────────────────────────

#[test]
fn c1_rule_ne_fires_when_sides_differ() {
    let src = r#"
entity R { flag: Float }
entity r: R = { flag: 0.0 }
entity n0: Float = 1.0
rule If(n0 != 2.0) then r.flag = 1.0
pattern Show(x: String) -> String { return x }
flow Main { input: Float = r.flag -> Show -> output }
"#;
    let tw = trim(run_tw(src, &base()));
    let vm = trim(run_vm(src, &base()));
    assert_eq!(tw, "1", "C1 TW: the rule must fire (1.0 != 2.0)");
    assert_eq!(vm, "1", "C1 VM: the rule must fire (1.0 != 2.0)");
}

// ── C2: rule `!=` stays silent on equality ──────────────────────────

#[test]
fn c2_rule_ne_silent_when_sides_equal() {
    let src = r#"
entity R { flag: Float }
entity r: R = { flag: 0.0 }
entity n0: Float = 2.0
rule If(n0 != 2.0) then r.flag = 1.0
pattern Show(x: String) -> String { return x }
flow Main { input: Float = r.flag -> Show -> output }
"#;
    let tw = trim(run_tw(src, &base()));
    let vm = trim(run_vm(src, &base()));
    assert_eq!(
        tw, "0",
        "C2 TW: the rule must NOT fire (2.0 != 2.0 is false)"
    );
    assert_eq!(
        vm, "0",
        "C2 VM: the rule must NOT fire (2.0 != 2.0 is false)"
    );
}

// ── C3: rule `==` regression — Eq semantics unchanged ───────────────

#[test]
fn c3_rule_eq_regression_unchanged() {
    let src = r#"
entity R { flag: Float }
entity r: R = { flag: 0.0 }
entity n0: Float = 2.0
rule If(n0 == 2.0) then r.flag = 1.0
pattern Show(x: String) -> String { return x }
flow Main { input: Float = r.flag -> Show -> output }
"#;
    let tw = trim(run_tw(src, &base()));
    let vm = trim(run_vm(src, &base()));
    assert_eq!(tw, "1", "C3 TW: Eq must fire on equality");
    assert_eq!(vm, "1", "C3 VM: Eq must fire on equality");
}

// ── C4: flow branch `!=` taken ──────────────────────────────────────

#[test]
fn c4_flow_branch_ne_taken() {
    let src = r#"
entity M { urgency: Float }
entity m0: M = { urgency: 0.5 }
pattern Hot(x: M) -> String { return "hot-path" }
pattern Cold(x: M) -> String { return "cold-path" }
flow Main {
  input: M = m0 -> Hot -> output
  Hot {
    cold(x.urgency != 0.9) -> Cold
  }
}
"#;
    let tw = trim(run_tw(src, &base()));
    let vm = trim(run_vm(src, &base()));
    assert_eq!(tw, "cold-path", "C4 TW: 0.5 != 0.9 must route to Cold");
    assert_eq!(vm, "cold-path", "C4 VM: 0.5 != 0.9 must route to Cold");
}

// ── C5: flow branch `!=` not taken — loud, consistent failure ───────

#[test]
fn c5_flow_branch_ne_not_taken_fails_loud_both() {
    let src = r#"
entity M { urgency: Float }
entity m0: M = { urgency: 0.9 }
pattern Hot(x: M) -> String { return "hot-path" }
pattern Cold(x: M) -> String { return "cold-path" }
flow Main {
  input: M = m0 -> Hot -> output
  Hot {
    cold(x.urgency != 0.9) -> Cold
  }
}
"#;
    let tw = run_tw(src, &base());
    let vm = run_vm(src, &base());
    let tw_err = tw.expect_err("C5 TW: no branch matched must fail loud");
    let vm_err = vm.expect_err("C5 VM: no branch matched must fail loud");
    assert!(
        tw_err.contains("no branch matched"),
        "C5 TW: expected the no-branch error, got: {}",
        tw_err
    );
    assert!(
        vm_err.contains("no branch matched"),
        "C5 VM: expected the no-branch error, got: {}",
        vm_err
    );
}

// ── C6–C10: rollback_if operators on BOTH backends ─────────────────

fn rollback_source(cond: &str) -> String {
    format!(
        r#"
learnable pattern Classify(text: String) -> String {{
  prompt: "Classify this text"
}}
adapt Classify add_example("hello", "greeting")
mutate Classify {{
  add_example("hi", "greeting")
  rollback_if: accuracy {cond}
}}
flow Main {{ input: String = "s" -> Classify -> output }}
"#
    )
}

fn assert_rollback(cond: &str, expect_kept: bool, label: &str) {
    std::env::set_var("METALOGOS_MOCK_LLM", "1"); // Н454: explicit mock opt-in
    let src = rollback_source(cond);
    let tw = trim(run_tw(&src, &base()));
    let vm = trim(run_vm(&src, &base()));
    assert_eq!(tw, vm, "{}: TW and VM mutate messages must agree", label);
    assert!(
        tw.contains("[MUTATE] Classify: accuracy=0.95"),
        "{}: unexpected mutate message: {}",
        label,
        tw
    );
    if expect_kept {
        assert!(tw.contains("kept"), "{}: expected KEPT, got: {}", label, tw);
    } else {
        assert!(
            tw.contains("rolled back"),
            "{}: expected ROLLED BACK, got: {}",
            label,
            tw
        );
    }
}

#[test]
fn c6_rollback_ne_equal_accuracy_is_kept() {
    // Mock accuracy 0.95; `accuracy != 0.95` is false → keep.
    assert_rollback("!= 0.95", true, "C6");
}

#[test]
fn c7_rollback_ne_different_accuracy_rolls_back() {
    // Mock accuracy 0.95; `accuracy != 0.90` is true → roll back.
    assert_rollback("!= 0.90", false, "C7");
}

#[test]
fn c8_rollback_gt_high_threshold_kept() {
    // 0.95 > 0.99 is false → keep (mirrors the №148 C1 contract on the VM).
    assert_rollback("> 0.99", true, "C8");
}

#[test]
fn c9_rollback_gt_low_threshold_rolls_back() {
    // 0.95 > 0.90 is true → roll back.
    assert_rollback("> 0.90", false, "C9");
}

#[test]
fn c10_rollback_eq_equal_accuracy_rolls_back() {
    // №206: `accuracy == 0.95` → roll back IF equal → roll back here.
    assert_rollback("== 0.95", false, "C10");
}
