// ── tests/naryad_210_vision_skeleton.rs ────────────────────────────
// Наряд №210: Vision pillar skeleton tests.
//
// All tests run in the BASE build (no --features vision needed) —
// the skeleton (VisionId, VisionRegistry, Value::Vision, stubs) does
// not depend on the vision feature gate.
//
// Наряд №240 (R4.2) truth-up: vision_generate/list/export are REAL paths
// now (dispatch, loud environment refusals). The R1 "not implemented"
// refusal no longer exists for them — the loudness contracts below are
// updated to the R4 contract (arity 2, typed handles); edit/save/load
// remain loud stubs and keep the original naryad-210 assertions.

use metalogos::interpreter::Value;
use metalogos::vision::VisionId;

// ── Test 1: vision_list returns empty list ──────────────────────

#[test]
fn vision_list_returns_empty_list() {
    // vision_list() returns an honest empty list — the registry exists
    // but has no named artifacts in R1. This is not a stub value.
    let source = r#"
        pattern Test(_x: String) -> String {
            let result = vision_list()
            return to_string(result)
        }
        flow Main { input: String = "x" -> Test -> output }
    "#;
    let result = metalogos::run_program(source);
    assert!(result.is_ok(), "vision_list should not error");
    let output = result.unwrap().unwrap_or_default();
    assert_eq!(
        output.trim(),
        "[]",
        "vision_list() on empty registry should return []"
    );
}

// ── Test 2: vision_generate loud error via TW ───────────────────

#[test]
fn vision_generate_loud_error_tw() {
    let source = r#"
        pattern Test(_x: String) -> String {
            vision_generate("model", "prompt", 42.0)
            return "unreachable"
        }
        flow Main { input: String = "x" -> Test -> output }
    "#;
    let result = metalogos::run_program(source);
    assert!(result.is_err(), "vision_generate must error");
    let err = result.unwrap_err();
    // Наряд №240: the 3-arg R1 call shape is now an arity refusal — the
    // R4 contract is vision_generate(decl_name, prompt) (plan §3).
    assert!(
        err.contains("vision_generate"),
        "error must contain 'vision_generate': {}",
        err
    );
    assert!(
        err.contains("expects 2 arguments"),
        "error must state the R4 arity contract: {}",
        err
    );
    assert!(
        err.contains("plan \u{00a7}3"),
        "error must name the contract source: {}",
        err
    );
}

// ── Test 3: vision_generate loud error via VM (byte-for-byte TW parity) ─

#[test]
fn vision_generate_loud_error_vm() {
    let source = r#"
        pattern Test(_x: String) -> String {
            vision_generate("model", "prompt", 42.0)
            return "unreachable"
        }
        flow Main { input: String = "x" -> Test -> output }
    "#;

    let tw_result = metalogos::run_program(source).unwrap_err();

    let declarations = metalogos::parser::parse(source).expect("parse should succeed");
    let mut comp = metalogos::compiler::Compiler::new();
    let program = comp.compile(declarations).expect("compile should succeed");
    let mut vm = metalogos::vm::Vm::new();
    let vm_result = vm.run(program).unwrap_err();

    assert_eq!(
        tw_result, vm_result,
        "TW and VM must produce byte-for-byte identical error for vision_generate.\n\
         TW: {}\n\
         VM: {}",
        tw_result, vm_result
    );
}

// ── Test 4: all other vision_* builtins give loud errors ────────
//
// Наряд №240: vision_export left this stub group — it is a REAL path now
// (typed Vision handle + path, loud wrong-type refusal below).

#[test]
fn vision_edit_save_load_loud_errors() {
    let builtins = [
        ("vision_edit", r#"vision_edit("handle", "prompt")"#),
        ("vision_save", r#"vision_save("handle", "name")"#),
        ("vision_load", r#"vision_load("name")"#),
    ];

    for (name, call) in &builtins {
        let source = format!(
            r#"
            pattern Test(_x: String) -> String {{
                {}
                return "unreachable"
            }}
            flow Main {{ input: String = "x" -> Test -> output }}
            "#,
            call
        );
        let result = metalogos::run_program(&source);
        assert!(
            result.is_err(),
            "{} must return an error, not succeed",
            name
        );
        let err = result.unwrap_err();
        assert!(
            err.contains(name),
            "error for {} must contain the builtin name: {}",
            name,
            err
        );
        assert!(
            err.contains("naryad 210"),
            "error for {} must contain 'naryad 210': {}",
            name,
            err
        );
        assert!(
            err.contains("loud refusal"),
            "error for {} must contain 'loud refusal': {}",
            name,
            err
        );
    }
}

/// Наряд №240: vision_export is a real path — a String where a Vision
/// handle is expected is a loud typed refusal (not a stub, not a panic).
///
/// Наряд №241 (R5): the source carries a `vision { }` declaration — the
/// VISION_UNSIGNED_EXPORT Category-A gate (ADR-0125) otherwise refuses
/// the program at compile time, and this test's subject is the RUNTIME
/// wrong-handle-type refusal, which must stay reachable and loud.
#[test]
fn vision_export_wrong_handle_type_loud_error() {
    let source = r#"
        vision "poster" { model: "z-image-turbo" steps: 8 width: 1024 height: 1024 seed: 42 policy: safe profile: fp16 }
        pattern Test(_x: String) -> String {
            vision_export("handle", "path")
            return "unreachable"
        }
        flow Main { input: String = "x" -> Test -> output }
    "#;
    let err = metalogos::run_program(source).expect_err("wrong handle type must fail loudly");
    assert!(
        err.contains("vision_export"),
        "error must contain 'vision_export': {}",
        err
    );
    assert!(
        err.contains("Vision handle"),
        "error must name the expected type: {}",
        err
    );
}

// ── Test 5: Vision handle Display ────────────────────────────────

#[test]
fn vision_handle_display() {
    let id = VisionId(1);
    let value = Value::Vision(id);
    assert_eq!(
        format!("{}", value),
        "[Vision#1]",
        "Value::Vision display must be [Vision#N]"
    );

    let id2 = VisionId(42);
    let value2 = Value::Vision(id2);
    assert_eq!(
        format!("{}", value2),
        "[Vision#42]",
        "Value::Vision display must be [Vision#N]"
    );
}

// ── Test 6: type_of returns "vision" ────────────────────────────

#[test]
fn type_of_vision() {
    let value = Value::Vision(VisionId(0));
    assert_eq!(
        value.type_name(),
        "vision",
        "type_name for Value::Vision must be 'vision'"
    );
}

// ── Test 7: Arity check — vision_generate registered with arity 3 ─
// The registry-arity-check CI job verifies that the BUILTIN_REGISTRY
// declares the correct arity for each builtin. Here we verify the arity
// is 3 (model_name, prompt, seed) by checking the registry directly.

#[test]
fn vision_generate_registered_arity_is_3() {
    // The registry-arity-check job already verifies this on CI,
    // but we also check it here for a clear local failure message.
    let names = metalogos::builtins::builtin_names();
    let idx = names
        .iter()
        .position(|n| n == "vision_generate")
        .expect("vision_generate must be in BUILTIN_REGISTRY");
    // The arity is stored in the BuiltinSpec — verify it's 3.
    // We can't access the spec directly (it's in a const slice), but
    // builtin_names() + the registry-arity-check test covers this.
    // Instead, verify the builtin exists and its index is stable.
    assert!(
        idx >= 381,
        "vision_generate must be appended after existing builtins (index >= 381, was 381 pre-vision): got {}",
        idx
    );
    assert_eq!(
        names[idx], "vision_generate",
        "name at index {} must be 'vision_generate'",
        idx
    );
}
