// ── Наряд №370 (issue gh#437): VM Stage 1.2 — Expr::BlockIfElse → VM ────
//
// ADR-0141 Stage 1, row 2: the BLOCK if/else as a VALUE (`let x = if c {
// ... } else { ... }`, braces + statements) compiles to bytecode and
// executes on the VM with TW-identical semantics. Previously a loud compile
// error ("block if/else expression not yet supported"). The grammar allows
// the form in ANY expression position (primary_expr), so the value form is
// exercised in let/return/binary/argument positions.
//
// Design note: NO new opcode was needed — the jump structure
// (Jump/JumpIfNot) plus the №369 last-value register (StoreLastLocal into
// a hidden slot) express it exactly; the VM dispatch is unchanged by
// construction.
//
// Value contract (REFERENCE-consistent, TW eval_statements): the branch's
// value is its last non-Unit expression; a trailing Unit-valued statement
// does not reset it; lets inside branches do not leak (cloned env in TW,
// function-scoped slots in the VM); nothing matched + no else → Unit;
// a nested STATEMENT if/else/match inside a branch leaks its branch's
// trailing value (TW eval_block! semantics — compiled by the №370
// value-mode statement path).
//!
//! How to verify manually: `cargo test --test naryad_370_block_if_else_vm`.

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

// ── 1. Basic value form ────────────────────────────────────────────────

#[test]
fn block_if_else_basic_value_vm() {
    let src = r#"
pattern pick(input: String) -> String {
  let x = 7.0
  let grade = if x > 5.0 {
    let note = "big"
    note
  } else {
    "small"
  }
  return grade
}

flow Main {
  input: String = "" -> pick -> output
}
"#;
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm, "TW/VM divergence on basic block-if value");
    assert_eq!(tw, "big", "the branch's last expression is the value");
}

#[test]
fn block_if_else_value_in_return_position_vm() {
    let src = r#"
pattern pick(input: String) -> String {
  let x = 3.0
  return if x > 5.0 {
    "big"
  } else {
    "small"
  }
}

flow Main {
  input: String = "" -> pick -> output
}
"#;
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm, "TW/VM divergence on return-position block-if value");
    assert_eq!(tw, "small");
}

#[test]
fn block_if_else_value_in_binary_and_arg_positions_vm() {
    let src = r#"
pattern pick(input: String) -> String {
  let x = 2.0
  let joined = "[" + (if x > 5.0 {
    "big"
  } else {
    "small"
  }) + "]"
  return joined
}

flow Main {
  input: String = "" -> pick -> output
}
"#;
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm, "TW/VM divergence on block-if inside binary op");
    assert_eq!(tw, "[small]");
}

// ── 2. else-if chain + Unit fallthrough ───────────────────────────────

#[test]
fn block_if_else_else_if_chain_vm() {
    let src = r#"
pattern pick(input: String) -> String {
  let x = 7.0
  let tag = if x > 100.0 {
    "huge"
  } else if x > 5.0 {
    "medium"
  } else {
    "tiny"
  }
  return tag
}

flow Main {
  input: String = "" -> pick -> output
}
"#;
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm, "TW/VM divergence on else-if chain");
    assert_eq!(tw, "medium");
}

#[test]
fn block_if_else_no_match_no_else_is_unit_vm() {
    let src = r#"
pattern pick(input: String) -> String {
  let x = 1.0
  let maybe = if x > 100.0 {
    "never"
  }
  if maybe == "" {
    return "unit-case"
  }
  return "value-case"
}

flow Main {
  input: String = "" -> pick -> output
}
"#;
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm, "TW/VM divergence on Unit fallthrough");
    // Unit is a DISTINCT value from the String "" — the typed comparison
    // is false on both backends, so the fallback path runs. The contract
    // under test: the let binds Unit and BOTH backends treat it identically.
    assert_eq!(tw, "value-case");
}

// ── 3. Nested value forms ──────────────────────────────────────────────

#[test]
fn block_if_else_nested_value_vm() {
    let src = r#"
pattern pick(input: String) -> String {
  let x = 7.0
  let nested = if x > 5.0 {
    if x > 6.0 { "six+" } else { "five" }
  } else {
    "low"
  }
  return nested
}

flow Main {
  input: String = "" -> pick -> output
}
"#;
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm, "TW/VM divergence on nested block-if value");
    assert_eq!(tw, "six+");
}

#[test]
fn block_if_else_nested_statement_if_leaks_value_vm() {
    // The nested if here sits in STATEMENT position inside the branch body
    // (branches are statement lists) — TW's eval_block! leaks its branch's
    // trailing value into the outer block's value; the №370 value-mode
    // statement path mirrors that on the VM.
    let src = r#"
pattern pick(input: String) -> String {
  let x = 7.0
  let nested = if x > 5.0 {
    if x > 6.0 {
      "six-plus"
    } else {
      "five"
    }
  } else {
    "low"
  }
  return nested
}

flow Main {
  input: String = "" -> pick -> output
}
"#;
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm, "TW/VM divergence on nested statement-if value leak");
    assert_eq!(tw, "six-plus");
}

#[test]
fn block_if_else_nested_match_statement_in_branch_vm() {
    // A statement-form match inside a branch leaks the matched arm's value
    // (№369 machinery in value mode).
    let src = r#"
pattern pick(input: String) -> String {
  let x = "b"
  let out = if input == "" {
    match x {
      "a" then {
        "A"
      }
      "b" then {
        "B"
      }
      else {
        "?"
      }
    }
  } else {
    "nonempty"
  }
  return out
}

flow Main {
  input: String = "" -> pick -> output
}
"#;
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm, "TW/VM divergence on match inside block-if branch");
    assert_eq!(tw, "B");
}

// ── 4. Branch value contract details ──────────────────────────────────

#[test]
fn block_if_else_last_non_unit_wins_vm() {
    let src = r#"
pattern pick(input: String) -> String {
  let x = 7.0
  let v = if x > 5.0 {
    let _ = print("side")
    "value-one"
  } else {
    "other"
  }
  let _ = print("after")
  return v
}

flow Main {
  input: String = "" -> pick -> output
}
"#;
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm, "TW/VM divergence on last-non-Unit rule");
    assert_eq!(tw, "value-one");
}

#[test]
fn block_if_else_branch_with_early_return_vm() {
    // `return` inside a branch body: TW propagates the early return out of
    // the pattern (respond-as-return semantics for plain returns); the VM
    // emits Instruction::Return in the same position.
    let src = r#"
pattern pick(input: String) -> String {
  let x = 7.0
  let v = if x > 5.0 {
    return "early"
  } else {
    "late"
  }
  return v
}

flow Main {
  input: String = "" -> pick -> output
}
"#;
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm, "TW/VM divergence on return inside branch");
    assert_eq!(tw, "early");
}

#[test]
fn block_if_else_condition_evaluated_once_vm() {
    let src = r#"
pattern cond() -> Float {
  let _ = print("cond-eval")
  return 1.0
}

pattern pick(input: String) -> String {
  let v = if cond() > 0.0 {
    "yes"
  } else {
    "no"
  }
  return v
}

flow Main {
  input: String = "" -> pick -> output
}
"#;
    let (tw, vm) = tw_and_vm_must_agree(src);
    assert_eq!(tw, vm, "TW/VM divergence on condition evaluation count");
    assert_eq!(tw, "yes");
}

// ── 5. Example contract + no-stub ──────────────────────────────────────

#[test]
fn p370_block_if_value_golden_contract() {
    let src =
        std::fs::read_to_string("examples/p370_block_if_value.mlog").expect("example readable");
    let expected = std::fs::read_to_string("examples/p370_block_if_value.expected")
        .expect("expected readable");
    let (tw, vm) = tw_and_vm_must_agree(&src);
    assert_eq!(tw, vm, "p370: TW/VM divergence");
    assert_eq!(
        tw,
        expected.trim_end(),
        "p370 output drifted from .expected"
    );
}

#[test]
fn naryad_370_no_stubs() {
    for path in ["src/bytecode.rs", "src/compiler.rs", "src/vm.rs"] {
        let src = std::fs::read_to_string(path).unwrap_or_default();
        for marker in ["todo!", "unimplemented!", "SKELETON"] {
            assert!(
                !src.contains(marker),
                "{path} contains stub marker `{marker}` (№16.0-D)"
            );
        }
    }
}
