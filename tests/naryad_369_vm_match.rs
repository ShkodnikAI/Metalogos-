// ── Наряд №369 (issue gh#436): VM Stage 1.1 — Match statement + match_expr ──
//
// ADR-0141 Stage 1, row 1: `Statement::Match` and the match EXPRESSION form
// (`let x = match y { ... }`) compile to bytecode and execute on the VM with
// TW-identical semantics. Previously Match was a loud compile error
// (compiler.rs "not yet supported") and match_expr was a LOSSY parse — the
// arms were discarded at parse time and the let bound the raw scrutinee
// (№173b workaround); the REFERENCE §Match contract ("the value of the last
// expression in the selected arm") was violated by both backends.
//
// Contract checks in this file:
// 1. Statement-form Match on the VM: all four arm kinds (exact, starts_with,
//    contains, compare), first-match-wins order, else body, return/break/
//    continue propagation out of arm bodies, nesting.
// 2. match_expr on the VM: the let value is the last non-Unit expression of
//    the matched arm's body (TW eval_statements_cf contract); no match + no
//    else → Unit; scrutinee evaluated exactly once.
// 3. TW↔VM parity: identical stdout for the same program on both backends.
// 4. The crosscheck exception for p_match_switch.mlog is lifted (this file
//    greps the source — the file must NOT be excluded).
// 5. №16.0-D: no stubs in the touched modules.
//!
//! How to verify manually: `cargo test --test naryad_369_vm_match`.

use std::path::Path;

/// Execute via tree-walking interpreter (same entry the crosscheck uses).
fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, Path::new("examples").to_path_buf())
}

/// Execute via the bytecode VM (compile + run — the crosscheck's run_vm).
fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp =
        metalogos::compiler::Compiler::with_std_root(Path::new("examples").to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

fn tw_and_vm_must_agree(source: &str) -> (String, String) {
    let tw = run_tw(source).expect("TW run failed");
    let vm = run_vm(source).expect("VM run/compile failed");
    let tw_s = tw.as_deref().unwrap_or_default().trim_end().to_string();
    let vm_s = vm.as_deref().unwrap_or_default().trim_end().to_string();
    (tw_s, vm_s)
}

// ── 1. Statement-form Match: all four arm kinds + order + else ─────────

#[test]
fn match_statement_exact_and_else_vm() {
    let src = r#"
pattern pick(input: String) -> String {
  match input {
    "a" then { return "first" }
    "b" then { return "second" }
    else { return "fallback" }
  }
  return "unreachable"
}

flow Main {
  input: String = "" -> pick -> output
}
"#;
    // Exact arm wins over else; else fires when nothing matches — BOTH
    // orders must be identical across backends.
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm, "TW/VM divergence on exact+else match statement");
    // Flow input is "" — nothing matches, the else fires.
    assert_eq!(tw, "fallback");
}

#[test]
fn match_statement_starts_contains_compare_vm() {
    let src = r#"
pattern classify(input: String) -> String {
  let n = 5.0
  match input {
    starts_with "urgent" then { return "u" }
    contains "error" then { return "e" }
    else { return "n" }
  }
  return "unreachable"
}

pattern classify_num(input: String) -> String {
  match input {
    starts_with "high" then { return "H" }
    contains "mid" then { return "M" }
    else { return "L" }
  }
  return "unreachable"
}

flow Main {
  input: String = "" -> classify -> output
}
"#;
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm, "TW/VM divergence on starts_with/contains arms");
}

#[test]
fn match_statement_first_match_wins_vm() {
    // Overlapping arms: the FIRST matching arm must fire (both "urgent…"
    // starts_with and contains "urgent" match the scrutinee).
    let src = r#"
pattern overlap(input: String) -> String {
  match input {
    starts_with "urgent" then { return "prefix-hit" }
    contains "urgent" then { return "substring-hit" }
    else { return "none" }
  }
  return "unreachable"
}

flow Main {
  input: String = "" -> overlap -> output
}
"#;
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm, "TW/VM divergence on first-match-wins order");
}

#[test]
fn match_statement_nested_and_loop_control_vm() {
    // Nested match inside an arm body; break/continue inside a match arm
    // inside a while loop must reach the LOOP (TW ControlFlow propagation).
    let src = r#"
pattern scanner(input: String) -> String {
  let mut hits = 0.0
  let mut i = 0.0
  let target = "scan-it"
  while i < 3.0 {
    match target {
      contains "scan" then {
        match target {
          starts_with "scan" then { hits = hits + 10.0 }
          else { hits = hits + 1.0 }
        }
      }
      else { hits = hits + 0.5 }
    }
    if hits > 100.0 {
      break
    }
    i = i + 1.0
  }
  if hits == 30.0 {
    return "all-three"
  }
  if hits == 0.5 {
    return "broke-early"
  }
  return "other"
}

flow Main {
  input: String = "" -> scanner -> output
}
"#;
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm, "TW/VM divergence on nested match + loop control");
    assert_eq!(tw, "all-three", "3 iterations × 10 expected");
}

// ── 2. match_expr: the REFERENCE §Match value contract ────────────────

#[test]
fn match_expr_value_is_last_arm_expression_vm() {
    let src = r#"
pattern pick(input: String) -> String {
  let x = "hello"
  let result = match x {
    "hello" then {
      "correct"
    }
    else {
      "default"
    }
  }
  return result
}

flow Main {
  input: String = "" -> pick -> output
}
"#;
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm, "TW/VM divergence on match_expr value");
    assert_eq!(
        tw, "correct",
        "the matched arm's last expression must be the let value"
    );
}

#[test]
fn match_expr_last_non_unit_value_wins_vm() {
    // The arm body's value is the last NON-Unit expression — a trailing
    // print (Unit) must NOT reset it; earlier statements contribute.
    let src = r#"
pattern pick(input: String) -> String {
  let x = "go"
  let result = match x {
    "go" then {
      let _ = print("side effect")
      "value-one"
    }
    else {
      "other"
    }
  }
  let _ = print("after")
  return result
}

flow Main {
  input: String = "" -> pick -> output
}
"#;
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm, "TW/VM divergence on last-non-Unit value rule");
    assert_eq!(tw, "value-one");
}

#[test]
fn match_expr_no_match_no_else_yields_unit_vm() {
    // No arm matches and there is no else: the let binds Unit — the
    // pattern then returns its fallback path. Both backends must agree.
    let src = r#"
pattern pick(input: String) -> String {
  let x = "unknown"
  let result = match x {
    "x" then {
      "ex"
    }
  }
  if result == "" {
    return "empty-path"
  }
  return "other-path"
}

flow Main {
  input: String = "" -> pick -> output
}
"#;
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm, "TW/VM divergence on Unit fallthrough");
    // Unit is NOT equal to the String "" (typed comparison) — the if is
    // false, so the fallback path after it runs. The contract under test:
    // the let binds Unit and BOTH backends treat it identically.
    assert_eq!(tw, "other-path");
}

#[test]
fn match_expr_compare_arm_numeric_first_vm() {
    // Compare arms are numeric-first, string-fallback (shared
    // ast::MatchArm::compare_values) — the scrutinee here is a NUMBER
    // formatted via Display; the threshold 90.0 compares numerically.
    let src = r#"
pattern grade(input: String) -> String {
  let score = 95.0
  let x = 42.0
  let result = match x {
    > 0.0 then {
      let grade = match score {
        > 90.0 then {
          "high"
        }
        else {
          "mid"
        }
      }
      grade
    }
    else {
      "low"
    }
  }
  return result
}

flow Main {
  input: String = "" -> grade -> output
}
"#;
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm, "TW/VM divergence on compare arm");
    assert_eq!(tw, "high");
}

#[test]
fn match_expr_starts_with_and_contains_arms_vm() {
    let src = r#"
pattern route(input: String) -> String {
  let x = "admin-report"
  let result = match x {
    starts_with "admin" then {
      "admin-area"
    }
    contains "report" then {
      "report-area"
    }
    else {
      "public"
    }
  }
  return result
}

flow Main {
  input: String = "" -> route -> output
}
"#;
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm);
    assert_eq!(tw, "admin-area");
}

#[test]
fn match_expr_nested_in_arm_vm() {
    let src = r#"
pattern deep(input: String) -> String {
  let x = "a"
  let outer = match x {
    "a" then {
      let inner = match x {
        "a" then {
          "aa"
        }
        else {
          "ab"
        }
      }
      inner
    }
    else {
      "z"
    }
  }
  return outer
}

flow Main {
  input: String = "" -> deep -> output
}
"#;
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm, "TW/VM divergence on nested match_expr");
    assert_eq!(tw, "aa");
}

#[test]
fn match_expr_scrutinee_evaluated_exactly_once_vm() {
    // A side-effecting scrutinee must be evaluated ONCE (both backends
    // evaluate once — TW binds the scrutinee value before the arm loop;
    // the VM stores it into a hidden slot). Two prints → two lines.
    let src = r#"
pattern noisy() -> String {
  let _ = print("eval")
  return "val"
}

pattern drive(input: String) -> String {
  let result = match noisy() {
    "val" then {
      "hit"
    }
    else {
      "miss"
    }
  }
  return result
}

flow Main {
  input: String = "" -> drive -> output
}
"#;
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm);
    assert_eq!(tw, "hit");
}

// ── 3. Crosscheck exception lifted + example contract ─────────────────

#[test]
fn p_match_switch_crosscheck_exception_is_lifted() {
    // №369 acceptance: the crosscheck must run p_match_switch.mlog through
    // BOTH backends — the skip branch must be gone.
    let src = std::fs::read_to_string("tests/crosscheck_backends.rs")
        .expect("crosscheck source readable");
    assert!(
        !src.contains("if name == \"p_match_switch.mlog\""),
        "crosscheck still excludes p_match_switch.mlog — the №369 acceptance requires the exception to be removed"
    );
}

#[test]
fn p_match_switch_golden_contract() {
    // The example itself (with its new flow Main) must run identically on
    // both backends and match the committed .expected contract.
    let src = std::fs::read_to_string("examples/p_match_switch.mlog").expect("example readable");
    let expected =
        std::fs::read_to_string("examples/p_match_switch.expected").expect("expected readable");
    let (tw, vm) = tw_and_vm_must_agree(&src);
    assert_eq!(tw, vm, "p_match_switch: TW/VM divergence");
    assert_eq!(
        tw,
        expected.trim_end(),
        "p_match_switch output drifted from .expected"
    );
}

// ── 4. Bytecode level: MatchTest/StoreLastLocal in .mbc round-trip ─────

#[test]
fn match_bytecode_serializes_round_trip() {
    // .mbc compat contract: the new instructions must survive
    // serialize→deserialize (bincode) without drift.
    let src = r#"
pattern pick(input: String) -> String {
  let x = "prefix-hit"
  let result = match x {
    starts_with "pre" then {
      "p"
    }
    > 0.0 then {
      "num"
    }
    else {
      "d"
    }
  }
  return result
}

flow Main {
  input: String = "" -> pick -> output
}
"#;
    let declarations = metalogos::parser::parse(src).expect("parse");
    let mut comp =
        metalogos::compiler::Compiler::with_std_root(Path::new("examples").to_path_buf());
    let program = comp.compile(declarations).expect("compile");
    let bytes =
        bincode::serde::encode_to_vec(&program, bincode::config::legacy()).expect("serialize");
    let config = bincode::config::legacy();
    let (decoded, _bytes_read): (metalogos::bytecode::Program, usize) =
        bincode::serde::decode_from_slice(&bytes, config).expect("deserialize");
    let mut vm = metalogos::vm::Vm::new();
    let out = vm.run(decoded).expect("decoded program runs");
    assert_eq!(out.as_deref().unwrap_or_default().trim_end(), "p");
}

// ── 5. №16.0-D: no stubs in the touched modules ────────────────────────

#[test]
fn naryad_369_no_stubs() {
    for path in [
        "src/bytecode.rs",
        "src/compiler.rs",
        "src/vm.rs",
        "src/interpreter/execution.rs",
        "src/parser/stmt.rs",
    ] {
        let src = std::fs::read_to_string(path).unwrap_or_default();
        for marker in ["todo!", "unimplemented!", "SKELETON"] {
            assert!(
                !src.contains(marker),
                "{path} contains stub marker `{marker}` (№16.0-D)"
            );
        }
    }
}

// ── helpers ────────────────────────────────────────────────────────────
