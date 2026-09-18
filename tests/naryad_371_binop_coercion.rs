//! Наряд №371 (ADR-0141 Stage 1.3): VM binop coercion — TW parity.
//!
//! The VM's `eval_binop` must behave EXACTLY like the TW interpreter's
//! (`src/interpreter/execution.rs`):
//!   * `+` on (String, String) → concatenation with the same
//!     MAX_STRING_LENGTH (1 MB) limit;
//!   * `+` on (Float, Float) → numeric addition;
//!   * heterogeneous `+` (List+String, String+List, List+List, Bool+String,
//!     Float+String, …) → the SAME loud error message in both backends
//!     ("type mismatch in string concatenation: … (use to_string() explicitly)");
//!   * opaque types (Secret, Html, Query, Encrypted, Hash, Subgraph) cannot
//!     be concatenated — same message in both backends;
//!   * non-Add binops on non-Float operands → the SAME error message
//!     ("type mismatch in binary operation: …");
//!   * division by zero → "division by zero" in both.
//!
//! Every vector asserts TW result == VM result (outputs for Ok, exact error
//! strings for Err), mirroring the crosscheck's both-error rule.

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

/// Assert both backends agree (Ok outputs equal, or Err messages equal).
fn assert_backend_parity(name: &str, source: &str) {
    let base_dir = PathBuf::from("examples");
    let tw = run_tw(source, &base_dir);
    let vm = run_vm(source, &base_dir);
    match (&tw, &vm) {
        (Ok(a), Ok(b)) => {
            let ta = a.as_deref().map(str::trim_end).unwrap_or_default();
            let tb = b.as_deref().map(str::trim_end).unwrap_or_default();
            assert_eq!(ta, tb, "{}: TW and VM outputs diverge", name);
        }
        (Err(a), Err(b)) => {
            assert_eq!(a, b, "{}: TW and VM error messages diverge", name);
        }
        (r, v) => panic!(
            "{}: backends disagree on Ok/Err — TW={:?} VM={:?}",
            name, r, v
        ),
    }
}

/// Wrap a pattern body into a runnable program (flow Main drives the pattern
/// with the flow input, so the pattern must take one String parameter).
fn program(body: &str) -> String {
    format!(
        "pattern T(_input: String) -> String {{\n{}\n}}\nflow Main {{ input: String = \"s\" -> T -> output }}",
        body
    )
}

/// №371 regression: the golden example p118_collection_utils (unique/chunk/sort
/// builtins + string concatenation) produces IDENTICAL output on both backends
/// and matches its .expected file. The crosscheck exclusion is lifted.
#[test]
fn naryad_371_p118_regression_golden() {
    let base_dir = PathBuf::from("examples");
    let src_path = base_dir.join("p118_collection_utils.mlog");
    let source = fs::read_to_string(&src_path).expect("p118 example exists");
    let expected_path = src_path.with_extension("expected");
    let expected = fs::read_to_string(&expected_path).expect("p118 expected file exists");

    let tw = run_tw(&source, &base_dir).expect("p118 runs on TW");
    let vm = run_vm(&source, &base_dir).expect("p118 runs on VM");

    let tw_out = tw.as_deref().map(str::trim_end).unwrap_or_default();
    let vm_out = vm.as_deref().map(str::trim_end).unwrap_or_default();
    let exp = expected.trim_end();

    assert_eq!(tw_out, exp, "p118: TW output must match .expected");
    assert_eq!(vm_out, exp, "p118: VM output must match .expected (№371)");
}

/// Heterogeneous `+` errors with the SAME message in both backends.
#[test]
fn naryad_371_heterogeneous_add_errors_identically() {
    let cases: Vec<(&str, String)> = vec![
        // List + String (the headline case)
        (
            "list_plus_string",
            program("  let xs = make_list(1, 2)\n  return xs + \"x\""),
        ),
        // String + List
        (
            "string_plus_list",
            program("  let xs = make_list(1, 2)\n  return \"x\" + xs"),
        ),
        // List + List
        (
            "list_plus_list",
            program("  let a = make_list(1)\n  let b = make_list(2)\n  return a + b"),
        ),
        // Bool + String
        ("bool_plus_string", program("  return true + \"x\"")),
        // Float + String (the classic to_string() footgun)
        ("float_plus_string", program("  return 5 + \"x\"")),
    ];
    for (name, src) in cases {
        let src = src.as_str();
        let base_dir = PathBuf::from("examples");
        let tw = run_tw(src, &base_dir);
        let vm = run_vm(src, &base_dir);
        let tw_err = tw.expect_err(&format!("{}: TW must error", name));
        let vm_err = vm.expect_err(&format!("{}: VM must error", name));
        assert_eq!(
            tw_err, vm_err,
            "{}: heterogeneous '+' must error identically",
            name
        );
        assert!(
            tw_err.contains("type mismatch in string concatenation"),
            "{}: TW message must be the loud concat error, got: {}",
            name,
            tw_err
        );
    }
}

/// String concatenation parity: basic, empty strings, nested-list joins.
#[test]
fn naryad_371_string_concat_parity() {
    // Basic + empty + nested-list join feeding into '+'.
    assert_backend_parity("basic_concat", &program("  return \"a\" + \"b\""));
    assert_backend_parity("empty_concat", &program("  return \"\" + \"\""));
    assert_backend_parity("empty_left", &program("  return \"\" + \"value\""));
    assert_backend_parity(
        "nested_join_concat",
        &program("  let r = chunk(make_list(1, 2, 3, 4), 2)\n  return join(r, \"|\") + \"!\""),
    );
    // p118-style accumulation through a mutable accumulator.
    assert_backend_parity(
        "accumulator_chain",
        &program(
            "  let mut r = \"\"\n  r = r + \"a:\" + str(1) + \"|\"\n  r = r + \"b:\" + str(2)\n  return r",
        ),
    );
}

/// Float arithmetic parity, including division by zero.
#[test]
fn naryad_371_float_arith_parity() {
    assert_backend_parity("float_add", &program("  return str(1.5 + 2.25)"));
    assert_backend_parity(
        "float_chain",
        &program("  return str((10.0 - 2.0) * 3.0 / 4.0)"),
    );
    assert_backend_parity("div_zero", &program("  return str(1.0 / 0.0)"));
}

/// Non-Add binops on non-Float operands error identically.
#[test]
fn naryad_371_non_add_type_errors_identically() {
    let cases: Vec<(&str, String)> = vec![
        ("string_div", program("  return \"a\" / \"b\"")),
        ("string_sub", program("  return \"a\" - \"b\"")),
        ("bool_div", program("  return true / false")),
    ];
    for (name, src) in cases {
        let src = src.as_str();
        let base_dir = PathBuf::from("examples");
        let tw = run_tw(src, &base_dir);
        let vm = run_vm(src, &base_dir);
        let tw_err = tw.expect_err(&format!("{}: TW must error", name));
        let vm_err = vm.expect_err(&format!("{}: VM must error", name));
        assert_eq!(
            tw_err, vm_err,
            "{}: non-Add binop type error must be identical",
            name
        );
        assert!(
            tw_err.contains("type mismatch in binary operation"),
            "{}: expected the TW wording, got: {}",
            name,
            tw_err
        );
    }
}

/// The 1 MB MAX_STRING_LENGTH limit is enforced with the same message.
#[test]
fn naryad_371_string_length_limit_parity() {
    // 1,000,001 'a' chars + "x" = 1,000,002 > 1,000,000 → both error.
    let src = program("  let big = repeat(\"a\", 1000001)\n  return big + \"x\"");
    let base_dir = PathBuf::from("examples");
    let tw = run_tw(&src, &base_dir);
    let vm = run_vm(&src, &base_dir);
    let tw_err = tw.expect_err("TW must enforce the length limit");
    let vm_err = vm.expect_err("VM must enforce the length limit (№371)");
    assert_eq!(tw_err, vm_err, "length-limit error must be identical");
    assert!(
        tw_err.contains("exceeds maximum allowed 1000000"),
        "unexpected limit message: {}",
        tw_err
    );

    // Exactly at the limit (1,000,000 chars) → both succeed with equal output.
    let src_ok = program("  let big = repeat(\"a\", 999999)\n  return big + \"x\"");
    assert_backend_parity("at_limit_concat", &src_ok);
}

/// Opaque types (Secret) cannot be concatenated — same message in both
/// backends (№371 adds the check to the VM; TW already had it).
#[test]
fn naryad_371_opaque_secret_concat_errors_identically() {
    // secret(name) reads the env var `name`; provide it for this test process.
    // SAFETY: single-threaded test binary; the var name is unique to this test.
    std::env::set_var("naryad_371_secret_value", "hunter2");

    let cases: Vec<(&str, String)> = vec![
        (
            "secret_plus_string",
            program("  return secret(\"naryad_371_secret_value\") + \"x\""),
        ),
        (
            "string_plus_secret",
            program("  return \"x\" + secret(\"naryad_371_secret_value\")"),
        ),
    ];
    for (name, src) in cases {
        let src = src.as_str();
        let base_dir = PathBuf::from("examples");
        let tw = run_tw(src, &base_dir);
        let vm = run_vm(src, &base_dir);
        let tw_err = tw.expect_err(&format!("{}: TW must error", name));
        let vm_err = vm.expect_err(&format!("{}: VM must error", name));
        assert_eq!(
            tw_err, vm_err,
            "{}: opaque concatenation must error identically",
            name
        );
        assert!(
            tw_err.contains("cannot concatenate opaque type Secret"),
            "{}: expected the opaque-type error, got: {}",
            name,
            tw_err
        );
    }
}

/// `print()` inside a concatenation keeps its (quirky but stable) semantics:
/// print returns the printed string, so `print("hi") + "x"` == "hix" on BOTH
/// backends — lock the parity so future refactors notice a drift.
#[test]
fn naryad_371_print_in_concat_parity() {
    assert_backend_parity("print_concat", &program("  return print(\"hi\") + \"x\""));
}
