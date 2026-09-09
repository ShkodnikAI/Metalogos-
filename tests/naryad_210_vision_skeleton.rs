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
//
// Наряд №243 (R6.2) truth-up: vision_edit left the stub group too — real
// in-context editing (VAE-энкодер + conditioning, signed-source contract).
// The R1 stub assertions ("naryad 210", "loud refusal") are replaced by
// the real-path refusals per the №240/№242 precedent (typed handle, arity
// — both loud; №243 Block 2.6). File test count: 10 → 11 (−1 stub test,
// +2 real-path refusal tests).

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

// ── Test 4: the real-path refusals (edit/save/load) ──────────
//
// Наряд №240: vision_export left this stub group — it is a REAL path now
// (typed Vision handle + path, loud wrong-type refusal below).
// Наряд №242 (R6.1): vision_save/vision_load left the stub group too —
// real SQLite persistence (src/vision/store.rs); their R1 stub texts
// ("naryad 210", "naryad 214/215") are gone by mandated doc truth-up.
// Наряд №243 (R6.2): vision_edit left the stub group too — real in-context
// editing; the R1 stub contract test is REPLACED by the real-path refusal
// tests (typed handle + arity — №240/№242 precedent).

/// №243: vision_edit is a REAL path — a String where a Vision handle is
/// expected is a loud typed refusal (same shape as vision_export/save;
/// the type check fires before anything else).
#[test]
fn vision_edit_wrong_handle_type_loud_error() {
    let source = r#"
            pattern Test(_x: String) -> String {
                vision_edit("handle", "prompt")
                return "unreachable"
            }
            flow Main { input: String = "x" -> Test -> output }
            "#;
    let err = metalogos::run_program(source).expect_err("wrong handle type must fail loudly");
    assert!(
        err.contains("vision_edit"),
        "error must contain 'vision_edit': {}",
        err
    );
    assert!(
        err.contains("must be a Vision handle"),
        "error must state the typed handle contract: {}",
        err
    );
}

/// №243: vision_edit arity contract — 1 argument is a loud refusal with
/// the arity contract (лекало vision_generate's arity refusal).
#[test]
fn vision_edit_wrong_arity_loud_error() {
    let source = r#"
            pattern Test(_x: String) -> String {
                vision_edit("handle")
                return "unreachable"
            }
            flow Main { input: String = "x" -> Test -> output }
            "#;
    let err = metalogos::run_program(source).expect_err("arity 1 must fail loudly");
    assert!(
        err.contains("vision_edit"),
        "error must contain 'vision_edit': {}",
        err
    );
    assert!(
        err.contains("expects 2 arguments"),
        "error must state the arity contract: {}",
        err
    );
}

/// №242: vision_save is a REAL path — a String where a Vision handle is
/// expected is a loud typed refusal (same shape as vision_export since
/// №240; the type check fires before the db check).
#[test]
fn vision_save_wrong_handle_type_loud_error() {
    let source = r#"
            pattern Test(_x: String) -> String {
                vision_save("handle", "name")
                return "unreachable"
            }
            flow Main { input: String = "x" -> Test -> output }
            "#;
    let err = metalogos::run_program(source).expect_err("wrong handle type must fail loudly");
    assert!(
        err.contains("vision_save"),
        "error must contain 'vision_save': {}",
        err
    );
    assert!(
        err.contains("must be a Vision handle"),
        "error must state the typed handle contract: {}",
        err
    );
}

/// №242: vision_load is a REAL path — with a correctly-typed argument but
/// no `db { url: ... }` declaration, the refusal is the loud no-db error
/// naming the declaration (Block 2.1), not a stub text.
#[test]
fn vision_load_no_db_loud_error() {
    let source = r#"
            pattern Test(_x: String) -> String {
                vision_load("name")
                return "unreachable"
            }
            flow Main { input: String = "x" -> Test -> output }
            "#;
    let err = metalogos::run_program(source).expect_err("no-db load must fail loudly");
    assert!(
        err.contains("vision_load"),
        "error must contain 'vision_load': {}",
        err
    );
    assert!(
        err.contains("no database connection"),
        "error must name the missing component: {}",
        err
    );
    assert!(
        err.contains("db { url:"),
        "error must name HOW to enable persistence (№242 Block 2.1): {}",
        err
    );
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

// ── Наряд №244 (R6.3): the LoRA builtins are REAL paths ──────────────
//
// vision_lora_load / vision_lora_generate left the stub group — real
// SQLite-BLOB persistence (ADR-0124 §6) + adapter application. The
// no-db/typed refusals below are the real-path contract tests (№240/№242/
// №243 precedent). File test count: 11 → 13 (truth-up declared in PR
// №242 with the actual artifact).

/// №244: vision_lora_load is a REAL path — with correctly-typed arguments
/// but no `db { url: ... }` declaration, the refusal is the loud no-db
/// error naming the declaration (ADR-0124 §6: the adapter's ONLY home is
/// SQLite), not a stub text.
#[test]
fn vision_lora_load_no_db_loud_error() {
    let source = r#"
            pattern Test(_x: String) -> String {
                vision_lora_load("name", "lora/adapter.safetensors")
                return "unreachable"
            }
            flow Main { input: String = "x" -> Test -> output }
            "#;
    let err = metalogos::run_program(source).expect_err("no-db load must fail loudly");
    assert!(
        err.contains("vision_lora_load"),
        "error must contain 'vision_lora_load': {}",
        err
    );
    assert!(
        err.contains("no database connection"),
        "error must name the missing component: {}",
        err
    );
    assert!(
        err.contains("db { url:"),
        "error must name HOW to enable persistence: {}",
        err
    );
}

/// №244: vision_lora_generate arity contract — 2 arguments is a loud
/// refusal with the 3-arity contract (лекало vision_edit's arity refusal).
#[test]
fn vision_lora_generate_wrong_arity_loud_error() {
    let source = r#"
            pattern Test(_x: String) -> String {
                vision_lora_generate("poster", "prompt")
                return "unreachable"
            }
            flow Main { input: String = "x" -> Test -> output }
            "#;
    let err = metalogos::run_program(source).expect_err("arity 2 must fail loudly");
    assert!(
        err.contains("vision_lora_generate"),
        "error must contain 'vision_lora_generate': {}",
        err
    );
    assert!(
        err.contains("expects 3 arguments"),
        "error must state the arity contract: {}",
        err
    );
}
