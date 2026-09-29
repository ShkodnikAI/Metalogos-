//! Naryad №503 (issue #786, P1 testing/vm) — the outcome-parity pins.
//! BLOCKING: the two former ok↔err classes of the №465 corpus.
//!
//! (b) entity-in-pattern — FIXED here: a pattern body referencing a
//!     top-level entity now behaves the SAME on both backends. The TW
//!     ident read falls back to the declaration globals
//!     (`self.variables` — entities/fluids, the exact set the VM models
//!     with StoreGlobal/LoadGlobalByName); local bindings shadow, writes
//!     stay local. The corpus example class_example_entity_in_pattern.mlog
//!     was removed with its pin in the same PR (the №479 ratchet) — the
//!     program lives on HERE as the regression pin.
//!
//! (a) memory_forget compile gap — NOT fixed (root-caused in the naryad
//!     report: the №272 vec-gate + the TW soft unknown-function contract
//!     + the VM loud compile). The owner decides the repair naryad; the
//!     corpus line and its example left in the same PR per the ratchet.
//!
//! Verify: cargo test --test naryad_503_outcome_parity

#![allow(clippy::disallowed_methods)]

/// The exact corpus example (removed from tests/fuzz_corpus/ with its
/// pin): the pattern body reads the entity `e0`.
const ENTITY_IN_PATTERN: &str = r#"
entity e0: String = "seed-alpha"

pattern pa(x: String) -> String {
  let a = upper(x) + "-" + e0
  let b = to_string(len(a))
  return lower(a) + ":" + b
}

flow Main {
  input: String = e0 -> pa -> output
}
"#;

fn run_tw(source: &str) -> Result<Option<String>, String> {
    let dir = std::env::temp_dir();
    metalogos::run_program_with_dir(source, dir)
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let dir = std::env::temp_dir();
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(dir);
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

/// The (b) class: the entity read inside the pattern body — BOTH
/// backends succeed with the SAME output (tw=ok/vm=ok, no divergence).
#[test]
fn n503_entity_in_pattern_both_backends_agree() {
    let tw = run_tw(ENTITY_IN_PATTERN).expect("TW must succeed on the entity-in-pattern program");
    let vm = run_vm(ENTITY_IN_PATTERN).expect("VM must succeed on the entity-in-pattern program");
    let tw = tw.unwrap_or_default();
    let vm = vm.unwrap_or_default();
    assert_eq!(
        tw.trim(),
        vm.trim(),
        "the backends must agree on the entity-in-pattern program: tw={:?} vm={:?}",
        tw,
        vm
    );
    assert!(
        tw.contains("seed-alpha-seed-alpha"),
        "the entity value must flow into the pattern result, got: {}",
        tw
    );
}

/// The read-fallback is READ-only: a local `let` still shadows the
/// entity inside the pattern body, and the entity itself is unchanged
/// after the call (the VM models the same with a local slot over the
/// global name).
#[test]
fn n503_entity_read_fallback_keeps_local_shadowing() {
    let src = r#"
entity e0: String = "global-value"

pattern pa(x: String) -> String {
  let e0 = "local-value"
  return e0 + "|" + x
}

flow Main {
  input: String = e0 -> pa -> output
}
"#;
    let tw = run_tw(src).expect("TW must succeed");
    let vm = run_vm(src).expect("VM must succeed");
    let tw = tw.unwrap_or_default();
    let vm = vm.unwrap_or_default();
    assert_eq!(tw.trim(), vm.trim(), "local shadowing must agree");
    assert!(
        tw.contains("local-value"),
        "the local binding must win inside the pattern body, got: {}",
        tw
    );
}
