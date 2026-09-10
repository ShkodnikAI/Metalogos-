//! Наряд №240 — Vision R4.2 dispatch: не-gated тесты.
//!
//! Coverage (naryad §4.1):
//! - parse+compile of the plan-§3 example (z-image-turbo, steps 8,
//!   1024×1024, seed 42, policy safe, profile fp16 — the plan prose
//!   `profile: consumer` is outside the ADR-0124 enum, the plan §6 status
//!   fixed the example as illustrative; fp16 is the in-enum form);
//! - dispatch negatives: unknown declaration name, wrong arity, missing
//!   `MLOG_VISION_WEIGHTS_DIR` (loud exact messages are asserted);
//! - `vision_list` real handles sorted by id (empty + after insert);
//! - taint unit tests: UserInput in position 2 of vision_generate →
//!   audit-WARNING (VISION_PROMPT_USER_INPUT), literal prompt → no finding.
//!
//! No `#[ignore]` anywhere (loud-SKIP pattern where the environment may
//! legitimately differ — see the weights-env test).

use metalogos::ast::Declaration;
use metalogos::audit::Severity;
use metalogos::bytecode::{CompiledVisionPolicy, CompiledVisionProfile};
use metalogos::vision::{VisionArtifact, VisionId, VisionRegistry};

const PLAN_EXAMPLE: &str = r#"
vision "poster" {
  model: "z-image-turbo"
  steps: 8
  width: 1024
  height: 1024
  seed: 42
  policy: safe
  profile: fp16
}
"#;

// ── Block 1: dispatch pipeline (parse + compile) ─────────────────────

/// The plan-§3 example compiles and the compiled `Program` carries the
/// declaration 1:1 (name, model, steps, width, height, seed, policy,
/// profile) — the R4 contract «пример из §3 плана компилируется».
#[test]
fn plan_example_compiles_with_vision_decls_1to1() {
    let program = metalogos::compile_program(PLAN_EXAMPLE).expect("compile plan-§3 example");
    assert_eq!(program.vision_decls.len(), 1, "one vision declaration");
    let v = &program.vision_decls[0];
    assert_eq!(v.name, "poster");
    assert_eq!(v.model, "z-image-turbo");
    assert_eq!(v.steps, 8);
    assert_eq!(v.width, 1024);
    assert_eq!(v.height, 1024);
    assert_eq!(v.seed, 42);
    assert_eq!(v.policy, Some(CompiledVisionPolicy::Safe));
    assert_eq!(v.profile, CompiledVisionProfile::Fp16);
}

/// The declaration compiles to zero vision-related bytecode (лекало
/// reflex): dispatch state travels via `Program::vision_decls`, not
/// instructions.
#[test]
fn vision_declaration_emits_no_bytecode() {
    let program = metalogos::compile_program(PLAN_EXAMPLE).expect("compile");
    // main_code must contain no CallBuiltin to vision_* (declaration-only
    // program: no flow, no pattern bodies).
    let vision_calls = program.main_code.iter().filter(|i| {
        let s = format!("{:?}", i);
        s.contains("vision_")
    });
    assert_eq!(
        vision_calls.count(),
        0,
        "vision declaration must not emit bytecode in pass2"
    );
}

// ── Block 2: dispatch negatives (loud, exact messages) ───────────────

/// Unknown declaration name → loud Err listing the declared names.
#[test]
fn generate_unknown_declaration_loud_error() {
    let source = r#"
vision "poster" { model: "z-image-turbo" steps: 8 width: 1024 height: 1024 seed: 42 policy: safe profile: fp16 }
pattern Generate(sid: String) -> String {
  let v = vision_generate("nope", "a cat")
  return "ok"
}
flow Main { input: String = "x" -> Generate -> output }
"#;
    let err = metalogos::run_program(source).expect_err("unknown decl name must be a loud error");
    assert!(
        err.contains("not declared"),
        "error must name the problem: {}",
        err
    );
    assert!(
        err.contains("\"poster\""),
        "error must list the declared names: {}",
        err
    );
    assert!(
        err.contains("'nope'"),
        "error must repeat the requested name: {}",
        err
    );
}

/// Wrong arity (1 arg instead of 2) → loud Err with the R4 contract text.
#[test]
fn generate_wrong_arity_loud_error() {
    let source = r#"
vision "poster" { model: "z-image-turbo" steps: 8 width: 1024 height: 1024 seed: 42 policy: safe profile: fp16 }
pattern Generate(sid: String) -> String {
  let v = vision_generate("poster")
  return "ok"
}
flow Main { input: String = "x" -> Generate -> output }
"#;
    let err = metalogos::run_program(source).expect_err("arity 1 must fail loudly");
    assert!(
        err.contains("expects 2 arguments"),
        "error must state the arity contract: {}",
        err
    );
}

/// Missing `MLOG_VISION_WEIGHTS_DIR` → loud Err naming the env var.
///
/// Loud-SKIP: if the environment HAS the weights dir set (developer
/// machine), this negative is not exercisable — the run proceeds to the
/// real clip. We do not `#[ignore]` and we do not unset the variable
/// (unsetting a global env var in a parallel test runner is a race with
/// the env-gated tests).
#[test]
fn generate_missing_weights_env_loud_error() {
    if std::env::var_os("MLOG_VISION_WEIGHTS_DIR").is_some() {
        eprintln!(
            "LOUD SKIP: MLOG_VISION_WEIGHTS_DIR is set — missing-env negative \
             not exercisable in this environment"
        );
        return;
    }
    let source = r#"
vision "poster" { model: "z-image-turbo" steps: 8 width: 1024 height: 1024 seed: 42 policy: safe profile: fp16 }
pattern Generate(sid: String) -> String {
  let v = vision_generate("poster", "a red apple")
  return "ok"
}
flow Main { input: String = "x" -> Generate -> output }
"#;
    let err = metalogos::run_program(source).expect_err("missing env must fail loudly");
    assert!(
        err.contains("MLOG_VISION_WEIGHTS_DIR"),
        "error must name the env var: {}",
        err
    );
    assert!(
        err.contains("is not set"),
        "error must state what is wrong: {}",
        err
    );
}

/// Runtime re-check of the model (defense-in-depth): a hand-built
/// `Program` with an unknown model fails the dispatch even though compile-
/// time semantic never saw it.
#[test]
fn generate_runtime_model_recheck_loud_error() {
    use metalogos::bytecode::{CompiledVisionDecl, Program};

    // Build a Program whose vision declaration names an unknown model —
    // bypassing semantic validation (deserialized bytecode / hand-built).
    let mut program = Program {
        globals: vec![],
        patterns: vec![],
        learnables: vec![],
        rules: vec![],
        skill_indices: vec![],
        reflex_decls: vec![],
        reflex_seq_decls: vec![],
        reflex_gen_decls: vec![],
        vision_decls: vec![CompiledVisionDecl {
            name: "rogue".to_string(),
            model: "not-a-model".to_string(),
            steps: 8,
            width: 1024,
            height: 1024,
            seed: 42,
            policy: Some(CompiledVisionPolicy::Safe),
            profile: CompiledVisionProfile::Fp16,
        }],
        db_url: None,
        memory_persist_path: None,
        schema_ddl: vec![],
        main_code: vec![],
        collections_loaded: false,
    };
    // Empty main_code — the VM registers declarations at load_program time.
    // The re-check fires on dispatch; to trigger dispatch we call the
    // dispatch function directly (same function both backends use).
    let mut registry = VisionRegistry::new();
    let decls: std::collections::HashMap<String, CompiledVisionDecl> = program
        .vision_decls
        .drain(..)
        .map(|d| (d.name.clone(), d))
        .collect();
    let args = vec![
        metalogos::interpreter::Value::String("rogue".to_string()),
        metalogos::interpreter::Value::String("test".to_string()),
    ];
    let err = metalogos::builtins::vision_generate_dispatch(&decls, &mut registry, &args)
        .expect_err("unknown model must fail the runtime re-check");
    assert!(
        err.contains("unknown model"),
        "error must name the re-check: {}",
        err
    );
    assert!(
        err.contains("not-a-model"),
        "error must repeat the model id: {}",
        err
    );
    assert!(
        err.contains("z-image-turbo"),
        "error must list known models: {}",
        err
    );
    // No artifact may be inserted by a failed dispatch.
    assert!(registry.is_empty(), "failed dispatch must not insert");
}

// ── Block 2: vision_list real handles ────────────────────────────────

/// `vision_list` on an empty registry → empty list (honest answer).
#[test]
fn vision_list_empty_registry() {
    let registry = VisionRegistry::new();
    let out = metalogos::builtins::vision_list_dispatch(&registry, &[]).expect("list");
    match out {
        metalogos::interpreter::Value::List(items) => {
            assert_eq!(items.len(), 0, "empty registry → empty list");
        }
        other => panic!("expected List, got {:?}", other),
    }
}

/// `vision_list` after inserts → sorted, deterministic display handles.
#[test]
fn vision_list_after_insert_sorted_by_id() {
    let mut registry = VisionRegistry::new();
    let id0 = registry.insert(VisionArtifact {
        png_bytes: vec![1, 2, 3],
        manifest: None,
    });
    let id1 = registry.insert(VisionArtifact {
        png_bytes: vec![4, 5, 6],
        manifest: None,
    });
    assert_eq!(id0, VisionId(0));
    assert_eq!(id1, VisionId(1));
    let out = metalogos::builtins::vision_list_dispatch(&registry, &[]).expect("list");
    match out {
        metalogos::interpreter::Value::List(items) => {
            assert_eq!(items.len(), 2);
            let want0 = metalogos::interpreter::Value::String("[Vision#0]".to_string());
            let want1 = metalogos::interpreter::Value::String("[Vision#1]".to_string());
            let fmt = |v: &metalogos::interpreter::Value| format!("{}", v);
            assert!(matches!((&items[0], &want0), (a, b) if fmt(a) == fmt(b)));
            assert!(matches!((&items[1], &want1), (a, b) if fmt(a) == fmt(b)));
        }
        other => panic!("expected List, got {:?}", other),
    }
}

// ── Block 3: taint (UserInput → prompt = audit-warning) ──────────────

/// UserInput-tainted prompt (position 2) → VISION_PROMPT_USER_INPUT
/// audit-WARNING (not a Category-A error).
#[test]
fn user_input_prompt_emits_audit_warning() {
    let source = r#"
vision "poster" { model: "z-image-turbo" steps: 8 width: 1024 height: 1024 seed: 42 policy: safe profile: fp16 }
pattern Upload(form_id: String) -> String {
  let prompt = form_data("prompt")
  let v = vision_generate("poster", prompt)
  return "generated"
}
"#;
    let result = metalogos::audit_program(source).expect("audit");
    let hits: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.check_id == "VISION_PROMPT_USER_INPUT")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "exactly one VISION_PROMPT_USER_INPUT finding"
    );
    assert_eq!(
        hits[0].severity,
        Severity::Warning,
        "must be a WARNING, not a Category-A error"
    );
    assert_eq!(
        result.error_count(),
        0,
        "no Category-A error from the prompt"
    );
}

/// Literal prompt (no taint) → no VISION_PROMPT_USER_INPUT finding.
#[test]
fn literal_prompt_no_warning() {
    let source = r#"
vision "poster" { model: "z-image-turbo" steps: 8 width: 1024 height: 1024 seed: 42 policy: safe profile: fp16 }
pattern Upload(form_id: String) -> String {
  let v = vision_generate("poster", "a red apple on a wooden table")
  return "generated"
}
"#;
    let result = metalogos::audit_program(source).expect("audit");
    let hits: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.check_id == "VISION_PROMPT_USER_INPUT")
        .collect();
    assert_eq!(hits.len(), 0, "literal prompt must not be flagged");
}

/// Arg 0 (declaration name) is NOT checked — name is not data (лекало
/// n201: reflex_train arg 0 is not flagged either).
#[test]
fn user_input_in_arg0_not_flagged() {
    let source = r#"
vision "poster" { model: "z-image-turbo" steps: 8 width: 1024 height: 1024 seed: 42 policy: safe profile: fp16 }
pattern Upload(form_id: String) -> String {
  let decl_name = form_data("which")
  let v = vision_generate(decl_name, "a red apple")
  return "generated"
}
"#;
    let result = metalogos::audit_program(source).expect("audit");
    let hits: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.check_id == "VISION_PROMPT_USER_INPUT")
        .collect();
    assert_eq!(hits.len(), 0, "arg 0 (declaration name) is not data");
}

/// Sanitize override: a prompt wrapped in render() is not flagged (лекало
/// get_expr_taint: render/escape_html → Sanitized overrides args).
#[test]
fn sanitized_prompt_no_warning() {
    let source = r#"
vision "poster" { model: "z-image-turbo" steps: 8 width: 1024 height: 1024 seed: 42 policy: safe profile: fp16 }
pattern Upload(form_id: String) -> String {
  let raw = form_data("prompt")
  let safe_prompt = render(raw)
  let v = vision_generate("poster", safe_prompt)
  return "generated"
}
"#;
    let result = metalogos::audit_program(source).expect("audit");
    let hits: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.check_id == "VISION_PROMPT_USER_INPUT")
        .collect();
    assert_eq!(hits.len(), 0, "sanitized prompt must not be flagged");
}

// ── .mlog promise closure visibility (declaration-level) ─────────────

/// The full plan-§3 example parses as a single Declaration::Vision —
/// the dispatch pipeline consumes exactly this shape (no surprises between
/// parser and dispatch).
#[test]
fn plan_example_is_single_vision_declaration() {
    let decls = metalogos::parser::parse(PLAN_EXAMPLE).expect("parse");
    assert_eq!(decls.len(), 1);
    assert!(matches!(decls[0], Declaration::Vision(_)));
}

// ── Наряд №244 (Vision R6.3): taint extends to vision_lora_generate ──

/// UserInput-tainted prompt (arg 1) of `vision_lora_generate` → the SAME
/// VISION_PROMPT_USER_INPUT audit-WARNING (no new check-id in №244; the
/// prompt is recorded in the generated artifact's provenance manifest).
#[test]
fn lora_generate_user_input_prompt_emits_audit_warning() {
    let source = r#"
vision "poster" { model: "z-image-turbo" steps: 8 width: 1024 height: 1024 seed: 42 policy: safe profile: fp16 }
pattern Upload(form_id: String) -> String {
  let prompt = form_data("prompt")
  let v = vision_lora_generate("poster", prompt, "my-lora")
  return "generated"
}
"#;
    let result = metalogos::audit_program(source).expect("audit");
    let hits: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.check_id == "VISION_PROMPT_USER_INPUT")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "exactly one VISION_PROMPT_USER_INPUT finding"
    );
    assert_eq!(
        hits[0].severity,
        Severity::Warning,
        "WARNING, not Category-A"
    );
    assert_eq!(
        result.error_count(),
        0,
        "no Category-A error from the prompt"
    );
}

/// Arg 0 (declaration name) and arg 2 (adapter name) of
/// `vision_lora_generate` are NOT data — neither is flagged.
#[test]
fn lora_generate_arg0_and_arg2_not_flagged() {
    let source = r#"
vision "poster" { model: "z-image-turbo" steps: 8 width: 1024 height: 1024 seed: 42 policy: safe profile: fp16 }
pattern Upload(form_id: String) -> String {
  let decl_name = form_data("which")
  let lora_name = form_data("adapter")
  let v = vision_lora_generate(decl_name, "a red apple", lora_name)
  return "generated"
}
"#;
    let result = metalogos::audit_program(source).expect("audit");
    let hits: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.check_id == "VISION_PROMPT_USER_INPUT")
        .collect();
    assert_eq!(
        hits.len(),
        0,
        "arg 0 (decl name) and arg 2 (adapter name) are not data"
    );
}

/// A literal prompt of `vision_lora_generate` → no finding (the check is
/// taint-positional, not a blanket refusal).
#[test]
fn lora_generate_literal_prompt_no_warning() {
    let source = r#"
vision "poster" { model: "z-image-turbo" steps: 8 width: 1024 height: 1024 seed: 42 policy: safe profile: fp16 }
pattern Upload(form_id: String) -> String {
  let v = vision_lora_generate("poster", "a red apple", "my-lora")
  return "generated"
}
"#;
    let result = metalogos::audit_program(source).expect("audit");
    let hits: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.check_id == "VISION_PROMPT_USER_INPUT")
        .collect();
    assert_eq!(hits.len(), 0, "literal prompt must not be flagged");
}
