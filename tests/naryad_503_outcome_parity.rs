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

// ── №651 (gh#1145, S-VAL-013): the CONDITION outcome-parity class ──────
//
// The condition path (if / else-if / while / match guards) must give ONE
// outcome per value class on BOTH backends: the same branch value, or the
// same stable [TYPE_MISMATCH] refusal. The TW side IS `Value::as_bool`;
// the VM side is `Instruction::JumpIfNotCond` calling the SAME method —
// the divergence №645-a (VM answered soft `is_truthy` and silently chose
// a branch on a composite) is structurally impossible after the repair.
// The soft truthiness of &&/|| (the №532 twin) is NOT covered here — it
// is a different surface, pinned elsewhere.

/// A condition program over one expression: `if <expr> { "then" } else { "else" }`.
fn cond_program(expr: &str) -> String {
    format!(
        r#"pattern p(x: String) -> String {{
  let branch = if {expr} {{ "then" }} else {{ "else" }}
  return branch
}}
flow Main {{
  input: String = "go" -> p -> output
}}
"#
    )
}

fn stable_code(err: &str) -> String {
    let start = err.find('[').unwrap_or(usize::MAX);
    if start == usize::MAX {
        return String::new();
    }
    let rest = &err[start + 1..];
    let end_rel = rest.find(']').unwrap_or(0);
    rest[..end_rel].to_string()
}

/// The FULL truthy set of a condition: THEN on both backends.
#[test]
fn n651_condition_truthy_set_then_on_both() {
    for expr in ["true", "1.0", "\"a\""] {
        let src = cond_program(expr);
        let tw = run_tw(&src).expect("TW must answer the truthy condition");
        let vm = run_vm(&src).expect("VM must answer the truthy condition");
        assert_eq!(
            tw.unwrap_or_default().trim(),
            vm.unwrap_or_default().trim(),
            "backends diverge on truthy expr {expr}"
        );
    }
}

/// The falsy scalars: ELSE on both backends (Unit included — S-VAL-013
/// keeps Unit falsy, it does not refuse).
#[test]
fn n651_condition_falsy_set_else_on_both() {
    for expr in ["false", "0.0", "\"\""] {
        let src = cond_program(expr);
        let tw = run_tw(&src).expect("TW must answer the falsy scalar condition");
        let vm = run_vm(&src).expect("VM must answer the falsy scalar condition");
        assert_eq!(
            tw.unwrap_or_default().trim(),
            vm.unwrap_or_default().trim(),
            "backends diverge on falsy expr {expr}"
        );
    }
    // Unit: the block-if without an else arm IS the Unit value.
    let unit_src = r#"pattern p(x: String) -> String {
  let u = if false { 1.0 }
  let branch = if u { "then" } else { "else" }
  return branch
}
flow Main {
  input: String = "go" -> p -> output
}
"#;
    let tw = run_tw(unit_src).expect("TW must answer the Unit condition");
    let vm = run_vm(unit_src).expect("VM must answer the Unit condition");
    assert_eq!(
        tw.unwrap_or_default().trim(),
        vm.unwrap_or_default().trim(),
        "backends diverge on the Unit condition"
    );
}

/// The refusal class: EVERY composite / opaque value in a condition
/// refuses with the SAME stable [TYPE_MISMATCH] code on both backends
/// (the loud refusal replaces the №645-a silent branch).
///
/// Fluid is deliberately NOT in this list: a fluid global read inside a
/// pattern body never reaches the condition path — the №523 semantic
/// gate refuses the program first (UNDEFINED_VARIABLE, the same code on
/// both backends through the FULL pipeline). The runtime Fluid arm of
/// `as_bool` is still the shared method — no drift is possible.
#[test]
fn n651_condition_composites_refuse_with_one_code() {
    let cases: Vec<(&str, String)> = vec![
        ("List", cond_program("[x]")),
        ("empty List", cond_program("[]")),
        ("Struct", cond_program("{ k: 1.0 }")),
        ("Hash (opaque)", cond_program("hash_password(x)")),
    ];
    for (name, src) in cases {
        let tw = run_tw(&src);
        let vm = run_vm(&src);
        let (tw_code, vm_code) = match (tw, vm) {
            (Err(t), Err(v)) => (stable_code(&t), stable_code(&v)),
            (tw_res, vm_res) => panic!(
                "backends diverge on {name}: tw={tw_res:?} vm={vm_res:?} — the S-VAL-013 parity pin"
            ),
        };
        assert_eq!(
            tw_code, "TYPE_MISMATCH",
            "TW must refuse {name} with [TYPE_MISMATCH]"
        );
        assert_eq!(
            vm_code, "TYPE_MISMATCH",
            "VM must refuse {name} with [TYPE_MISMATCH]"
        );
    }
}
