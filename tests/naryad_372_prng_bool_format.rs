//! Наряд №372 (ADR-0141 Stage 1.4): VM PRNG state + Bool→String formatting —
//! TW parity.
//!
//! Two contracts, both fixed by test vectors:
//!   1. PRNG: `random_seed(n)` + `random()` route through the SHARED registry
//!      (thread-local xorshift64 in `src/builtins/math.rs`) on BOTH backends —
//!      the same seed yields the SAME sequence on TW and VM (determinism is
//!      part of the contract).
//!   2. Bool→String: VM comparisons produce `Value::Bool` (not Float 1.0/0.0),
//!      so `to_string(a == b)` prints "true"/"false" exactly like TW.
//!
//! Every vector asserts TW result == VM result.

use std::fs;
use std::path::{Path, PathBuf};

/// Execute via tree-walking interpreter.
fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.to_path_buf())
}

/// Execute via bytecode VM.
fn run_vm(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp
        .compile(declarations)
        .map_err(|e| format!("compile error: {}", e))?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program).map_err(|e| e.to_string())
}

/// Assert both backends agree on the trimmed Ok output.
fn assert_parity(name: &str, source: &str) {
    let base_dir = PathBuf::from("examples");
    let tw =
        run_tw(source, &base_dir).unwrap_or_else(|e| panic!("{}: TW must succeed: {}", name, e));
    let vm =
        run_vm(source, &base_dir).unwrap_or_else(|e| panic!("{}: VM must succeed: {}", name, e));
    let tw_out = tw.as_deref().map(str::trim_end).unwrap_or_default();
    let vm_out = vm.as_deref().map(str::trim_end).unwrap_or_default();
    assert_eq!(tw_out, vm_out, "{}: TW and VM outputs diverge", name);
}

fn program(body: &str) -> String {
    format!(
        "pattern T(_input: String) -> String {{\n{}\n}}\nflow Main {{ input: String = \"s\" -> T -> output }}",
        body
    )
}

/// PRNG determinism contract: the same seed → the same sequence on BOTH
/// backends, and TW == VM on every element.
#[test]
fn naryad_372_prng_same_seed_same_sequence_both_backends() {
    // 5 elements after seed(42), then a fresh seed(7) branch — the full
    // sequence must match across backends AND the reseeding must restart
    // deterministically inside one program run.
    assert_parity(
        "prng_seed42_seq",
        &program(
            "  random_seed(42.0)\n  let mut s = \"\"\n  s = s + to_string(random()) + \",\"\n  s = s + to_string(random()) + \",\"\n  s = s + to_string(random()) + \",\"\n  s = s + to_string(random()) + \",\"\n  s = s + to_string(random())\n  return s",
        ),
    );
    assert_parity(
        "prng_seed7_seq",
        &program(
            "  random_seed(7.0)\n  let mut s = \"\"\n  s = s + to_string(random()) + \",\"\n  s = s + to_string(random()) + \",\"\n  s = s + to_string(random())\n  return s",
        ),
    );
    // Reseed inside a run: seed(42) → r1; seed(42) → r2 must equal r1.
    assert_parity(
        "prng_reseed_deterministic",
        &program(
            "  random_seed(42.0)\n  let a = random()\n  random_seed(42.0)\n  let b = random()\n  return to_string(a) + \"/\" + to_string(b) + \"/\" + to_string(a == b)",
        ),
    );
    // Zero seed — the degenerate state fallback must behave identically.
    assert_parity(
        "prng_zero_seed",
        &program("  random_seed(0.0)\n  return to_string(random()) + \",\" + to_string(random())"),
    );
}

/// The golden PRNG vector: seed(42) first element is a fixed value.
/// (Locked on BOTH backends — if the xorshift/seed_to_state math ever
/// changes, this test pins the migration loudly.)
#[test]
fn naryad_372_prng_golden_vector_locked() {
    let expected_first = "0.16258225917040392";
    let src = program("  random_seed(42.0)\n  return to_string(random())");
    let base_dir = PathBuf::from("examples");
    let tw = run_tw(&src, &base_dir).expect("TW runs");
    let vm = run_vm(&src, &base_dir).expect("VM runs");
    for (backend, out) in [("TW", tw), ("VM", vm)] {
        let s = out.unwrap_or_else(|| panic!("{}: output present", backend));
        assert_eq!(
            s.trim_end(),
            expected_first,
            "{}: seed(42).random() golden vector drifted",
            backend
        );
    }
}

/// Bool→String: comparisons produce Bool on the VM — `to_string` prints
/// "true"/"false" exactly like TW (the old VM printed "1"/"0").
#[test]
fn naryad_372_bool_string_format_parity() {
    assert_parity("bool_eq", &program("  return to_string(1.0 == 1.0)"));
    assert_parity("bool_ne", &program("  return to_string(1.0 != 1.0)"));
    assert_parity(
        "bool_string_cmp",
        &program("  return to_string(\"a\" == \"a\") + \"/\" + to_string(\"a\" != \"a\")"),
    );
    assert_parity(
        "bool_ordering",
        &program("  return to_string(2.0 > 1.0) + \"/\" + to_string(1.0 < 2.0) + \"/\" + to_string(1.0 >= 1.0) + \"/\" + to_string(1.0 <= 0.5)"),
    );
    assert_parity(
        "bool_literals",
        &program("  return to_string(true) + \"/\" + to_string(false)"),
    );
    // The reflex_math vector: comparison result feeds to_string in a concat.
    assert_parity(
        "bool_in_concat",
        &program(
            "  random_seed(42.0)\n  let a = random()\n  random_seed(42.0)\n  let b = random()\n  let equal = to_string(to_float(to_string(a)) == to_float(to_string(b)))\n  return \"a=\" + to_string(a) + \" b=\" + to_string(b) + \" equal=\" + equal",
        ),
    );
}

/// Bool results feed control flow identically: if/while on comparison results.
#[test]
fn naryad_372_bool_control_flow_parity() {
    assert_parity(
        "bool_if",
        &program("  let x = 5.0 > 3.0\n  if x { return \"yes\" }\n  return \"no\""),
    );
    assert_parity(
        "bool_while",
        &program(
            "  let mut i = 0.0\n  let mut acc = \"\"\n  while i < 3.0 {\n    acc = acc + to_string(i)\n    i = i + 1.0\n  }\n  return acc",
        ),
    );
    // Ne through the CmpNe instruction (Bool inversion on the VM).
    assert_parity(
        "bool_ne_instruction",
        &program(
            "  let mut i = 0.0\n  let mut acc = \"\"\n  while i != 3.0 {\n    acc = acc + \"x\"\n    i = i + 1.0\n  }\n  return acc + \"/\" + to_string(i != 3.0)",
        ),
    );
}

/// String predicates keep TW types: contains/starts_with via shared builtins
/// AND the legacy VM predicate instructions produce Bool.
#[test]
fn naryad_372_string_predicates_parity() {
    assert_parity(
        "contains_builtin",
        &program("  return to_string(contains(\"hello\", \"ell\")) + \"/\" + to_string(contains(\"hello\", \"xyz\"))"),
    );
    assert_parity(
        "starts_with_builtin",
        &program("  return to_string(starts_with(\"hello\", \"he\")) + \"/\" + to_string(starts_with(\"hello\", \"lo\"))"),
    );
    assert_parity(
        "ends_with_builtin",
        &program("  return to_string(ends_with(\"hello\", \"lo\")) + \"/\" + to_string(ends_with(\"hello\", \"he\"))"),
    );
}

/// №372 regression: the golden example reflex_math (math builtins + random +
/// Bool formatting) produces IDENTICAL output on both backends and matches
/// its .expected file. The crosscheck exclusion is lifted.
#[test]
fn naryad_372_reflex_math_regression_golden() {
    let base_dir = PathBuf::from("examples");
    let src_path = base_dir.join("reflex_math.mlog");
    let source = fs::read_to_string(&src_path).expect("reflex_math example exists");
    let expected = fs::read_to_string(src_path.with_extension("expected"))
        .expect("reflex_math expected file exists");

    let tw = run_tw(&source, &base_dir).expect("reflex_math runs on TW");
    let vm = run_vm(&source, &base_dir).expect("reflex_math runs on VM");

    let tw_out = tw.as_deref().map(str::trim_end).unwrap_or_default();
    let vm_out = vm.as_deref().map(str::trim_end).unwrap_or_default();
    let exp = expected.trim_end();

    assert_eq!(tw_out, exp, "reflex_math: TW output must match .expected");
    assert_eq!(
        vm_out, exp,
        "reflex_math: VM output must match .expected (№372)"
    );
}
