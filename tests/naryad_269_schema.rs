// ── Наряд №269: `call_llm_schema` — Structured LLM output (ADR-0133) ──
//
// Contract lines pinned here:
// 1. mlog contract "call_llm_schema → Struct → json_get" is green on BOTH
//    backends (TW and VM) against the deterministic schema-derived mock.
// 2. Invalid answer with no retry budget left = loud `LLM_SCHEMA_MISMATCH`
//    with the attempt bookkeeping (unit-level, provider-injected).
// 3. The validator is covered per supported/unsupported/rejected schema
//    construct by pure unit tests (no LLM, no network).
// 4. Retry loop: fail-once-then-valid succeeds; transport errors and
//    schema-side errors do NOT retry (ADR-0133 D3).
// 5. Truncation (unparseable JSON) is loud and names max_tokens as the likely
//    cause.
//
// Аудит-инвариант: до №269 a program asking an LLM for structured data had to
// hand-roll json_get scraping over a free-text answer — every format error was
// a silent garbage String flowing downstream (FOSVED llm_verifier pain).

#![cfg(feature = "llm")]

// Лекало naryad_253/naryad_259/naryad_261: env-переменные процесса глобальны —
// сериализуем env-чувствительные тесты одним poison-tolerant мьютексом и
// держим его всё тело теста (set + assert + restore).
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Force the deterministic schema-mock tier (same tier call_llm_schema uses
/// when no SmartRouter is installed). Restores the previous value on drop.
struct MockEnv;
impl MockEnv {
    fn set_json() -> Self {
        std::env::set_var("METALOGOS_LLM_MOCK", "json");
        MockEnv
    }
}
impl Drop for MockEnv {
    fn drop(&mut self) {
        std::env::remove_var("METALOGOS_LLM_MOCK");
    }
}

const FLAT_SCHEMA: &str =
    r#"{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}"#;

const NESTED_SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "user": {"type": "object", "properties": {"city": {"type": "string"}}, "required": ["city"]},
    "tags": {"type": "array", "items": {"type": "string"}},
    "age": {"type": "integer"}
  },
  "required": ["user", "tags"]
}"#;

const FLAT_CONTRACT: &str = r#"
pattern Extract(x: String) -> String {
  let s = call_llm_schema(x, "{\"type\":\"object\",\"properties\":{\"name\":{\"type\":\"string\"}},\"required\":[\"name\"]}")
  return json_get(s, "name")
}
flow Main {
  input: String = "Extract the user name"
  -> Extract
  -> output
}
"#;

// ── 1. The mlog contract: call_llm_schema → Struct → json_get ────────────

#[test]
fn n269_contract_tw_schema_to_struct_to_json_get() {
    let _env = lock_env();
    let _mock = MockEnv::set_json();

    let out = metalogos::run_program(FLAT_CONTRACT).expect("TW must run the schema contract");
    assert_eq!(
        out.as_deref(),
        Some("name"),
        "TW: json_get over the call_llm_schema Struct must yield the mock field value"
    );
}

#[test]
fn n269_contract_vm_parity_schema_to_struct_to_json_get() {
    let _env = lock_env();
    let _mock = MockEnv::set_json();

    let declarations = metalogos::parser::parse(FLAT_CONTRACT).expect("contract must parse");
    let program = metalogos::compiler::Compiler::new()
        .compile(declarations)
        .expect("contract must compile");
    let mut vm = metalogos::vm::Vm::new();
    let out = vm.run(program).expect("VM must run the schema contract");
    assert_eq!(
        out.as_deref(),
        Some("name"),
        "VM parity: same source, same answer as TW"
    );
}

#[test]
fn n269_contract_nested_struct_and_array_paths() {
    let _env = lock_env();
    let _mock = MockEnv::set_json();

    let src = format!(
        r#"
pattern Extract(x: String) -> String {{
  let s = call_llm_schema("p", "{schema}")
  let city = json_get(s, "user.city")
  let tag0 = json_get(s, "tags.0")
  let age = json_get(s, "age")
  return city + "/" + tag0 + "/" + str(age)
}}
flow Main {{
  input: String = "x"
  -> Extract
  -> output
}}
"#,
        schema = NESTED_SCHEMA.replace('"', "\\\"")
    );

    let out = metalogos::run_program(&src).expect("TW nested contract");
    assert_eq!(
        out.as_deref(),
        Some("city/tags/1"),
        "TW: nested object, array index and integer mock fields via json_get"
    );

    let declarations = metalogos::parser::parse(&src).expect("nested contract must parse");
    let program = metalogos::compiler::Compiler::new()
        .compile(declarations)
        .expect("nested contract must compile");
    let mut vm = metalogos::vm::Vm::new();
    let vm_out = vm.run(program).expect("VM nested contract");
    assert_eq!(
        vm_out.as_deref(),
        Some("city/tags/1"),
        "VM parity for nested contract"
    );
}

/// Static passes must accept the new builtin (no false positives from the
/// №264 immutability pass, call/arity checks or the taint tracker).
#[test]
fn n269_check_program_accepts_schema_builtin() {
    let result = metalogos::check_program(FLAT_CONTRACT).expect("parse must succeed");
    assert!(
        result.is_ok(),
        "mlog check must accept call_llm_schema programs, got: {:?}",
        result.errors
    );
}

// ── 2. Validator: every supported construct + every rejection ────────────

use metalogos::builtins::{
    check_schema_supported, mock_instance_from_schema, schema_retries_from_env,
    validate_json_against_schema,
};

fn violations_of(answer: serde_json::Value, schema: &serde_json::Value) -> Vec<String> {
    let mut v = Vec::new();
    validate_json_against_schema(&answer, schema, "$", &mut v);
    v
}

#[test]
fn n269_validator_happy_object() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {"a": {"type": "string"}, "b": {"type": "number"}},
        "required": ["a"]
    });
    let v = violations_of(
        serde_json::json!({"a": "x", "b": 1.5, "extra_not_counted_vs_missing": null}),
        &schema,
    );
    // extras ARE violations (strict-by-default) — asserted separately below;
    // here the answer is clean:
    let clean = violations_of(serde_json::json!({"a": "x", "b": 1.5}), &schema);
    assert!(
        clean.is_empty(),
        "clean answer must validate, got {:?}",
        clean
    );
    assert_eq!(v.len(), 1, "only the extra field is a violation here");
}

#[test]
fn n269_validator_missing_required() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {"a": {"type": "string"}, "b": {"type": "string"}},
        "required": ["a", "b"]
    });
    let v = violations_of(serde_json::json!({"a": "x"}), &schema);
    assert_eq!(v.len(), 1, "one missing required field");
    assert!(
        v[0].contains("missing required field 'b'"),
        "report names the field: {:?}",
        v
    );
}

#[test]
fn n269_validator_wrong_type_reports_expected_and_got() {
    let schema = serde_json::json!({"type": "object", "properties": {"age": {"type": "integer"}}});
    let v = violations_of(serde_json::json!({"age": "old"}), &schema);
    assert_eq!(v.len(), 1);
    assert!(
        v[0].contains("expected type integer") && v[0].contains("got string"),
        "{:?}",
        v
    );
}

#[test]
fn n269_validator_integer_rejects_fractional_but_number_accepts_both() {
    let int_schema =
        serde_json::json!({"type": "object", "properties": {"n": {"type": "integer"}}});
    let v = violations_of(serde_json::json!({"n": 1.5}), &int_schema);
    assert_eq!(v.len(), 1, "1.5 is not an integer");
    let ok = violations_of(serde_json::json!({"n": 2}), &int_schema);
    assert!(ok.is_empty());

    let num_schema = serde_json::json!({"type": "object", "properties": {"n": {"type": "number"}}});
    assert!(violations_of(serde_json::json!({"n": 1.5}), &num_schema).is_empty());
    assert!(violations_of(serde_json::json!({"n": 2}), &num_schema).is_empty());
}

#[test]
fn n269_validator_enum_violation_names_literals() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {"status": {"enum": ["active", "banned"]}}
    });
    let v = violations_of(serde_json::json!({"status": "suspended"}), &schema);
    assert_eq!(v.len(), 1);
    assert!(v[0].contains("not one of the enum literals"), "{:?}", v);
    assert!(violations_of(serde_json::json!({"status": "active"}), &schema).is_empty());
}

#[test]
fn n269_validator_extras_are_violations_strict_by_default() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {"a": {"type": "string"}}
    });
    let v = violations_of(serde_json::json!({"a": "x", "smuggled": 1}), &schema);
    assert_eq!(v.len(), 1);
    assert!(
        v[0].contains("unexpected field 'smuggled'") && v[0].contains("strict-by-default"),
        "{:?}",
        v
    );
}

#[test]
fn n269_validator_nested_and_array_paths_in_report() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "users": {"type": "array", "items": {"type": "object", "properties": {"id": {"type": "integer"}}, "required": ["id"]}}
        },
        "required": ["users"]
    });
    let v = violations_of(
        serde_json::json!({"users": [{"id": 1}, {"nope": 2}, {"id": "x"}]}),
        &schema,
    );
    assert_eq!(
        v.len(),
        3,
        "missing id on [1], wrong type on [2], extras on [1]: {:?}",
        v
    );
    assert!(
        v.iter().any(|s| s.contains("answer.users[1]")),
        "array path in report: {:?}",
        v
    );
    assert!(v.iter().any(|s| s.contains("answer.users[2]")), "{:?}", v);
}

#[test]
fn n269_validator_scalars_null_boolean() {
    let schema = serde_json::json!({"type": "object", "properties": {"note": {"type": "null"}, "ok": {"type": "boolean"}}});
    let v = violations_of(serde_json::json!({"note": null, "ok": true}), &schema);
    assert!(v.is_empty(), "{:?}", v);
    let v = violations_of(serde_json::json!({"note": "x", "ok": 1}), &schema);
    assert_eq!(v.len(), 2, "both type violations reported");
}

#[test]
fn n269_validator_annotations_ignored() {
    let schema = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "Person",
        "description": "An annotated schema",
        "type": "object",
        "properties": {"a": {"type": "string", "description": "the a field", "default": "x"}},
        "required": ["a"]
    });
    assert!(
        check_schema_supported(&schema).is_ok(),
        "annotations must be accepted"
    );
    let v = violations_of(serde_json::json!({"a": "x"}), &schema);
    assert!(
        v.is_empty(),
        "annotations must not affect validation: {:?}",
        v
    );
}

// ── 3. Schema-side rejection: unsupported features are loud ─────────────

#[test]
fn n269_schema_side_unsupported_keyword_rejected_with_code() {
    let schema = serde_json::json!({"type": "object", "properties": {"a": {"type": "string", "pattern": "^a"}}});
    let err = check_schema_supported(&schema).expect_err("pattern must be rejected");
    assert!(
        err.contains("LLM_SCHEMA_UNSUPPORTED_FEATURE"),
        "diagnostic code in text: {}",
        err
    );
    assert!(err.contains("'pattern'"), "names the keyword: {}", err);
    assert!(err.contains("ADR-0133"), "points at the ADR: {}", err);
}

#[test]
fn n269_schema_side_root_must_be_object() {
    let err = check_schema_supported(&serde_json::json!({"type": "string"}))
        .expect_err("string root must be rejected");
    assert!(err.contains("root schema must be"), "{}", err);

    let err = check_schema_supported(
        &serde_json::json!({"type": "object", "properties": {}, "allOf": []}),
    )
    .expect_err("allOf must be rejected");
    assert!(err.contains("'allOf'"), "{}", err);
}

#[test]
fn n269_schema_side_malformed_keywords_rejected() {
    let e = check_schema_supported(&serde_json::json!({"type": "object", "required": ["a", 5]}))
        .expect_err("required must be strings");
    assert!(e.contains("'required'"), "{}", e);
    let e = check_schema_supported(&serde_json::json!({"type": "object", "enum": []}))
        .expect_err("empty enum");
    assert!(e.contains("'enum'"), "{}", e);
}

// ── 4. Core retry loop with injected providers (no network) ─────────────

use metalogos::builtins::call_llm_schema_core;

/// Provider replaying scripted answers; counts real invocations.
struct Scripted {
    answers: Vec<Result<String, String>>,
    calls: usize,
}
impl Scripted {
    fn new(answers: Vec<Result<String, String>>) -> Self {
        Self { answers, calls: 0 }
    }

    /// Drive the core loop against this script (the closure borrows self
    /// mutably for the duration of the call — no temporary escapes).
    fn run_core(
        &mut self,
        schema: &serde_json::Value,
        schema_json: &str,
        retries: u32,
    ) -> Result<metalogos::interpreter::Value, String> {
        let me = &mut *self;
        call_llm_schema_core(
            "p",
            "i",
            schema,
            schema_json,
            retries,
            &mut |_p: &str, _i: &str| {
                me.calls += 1;
                me.answers
                    .get(me.calls - 1)
                    .cloned()
                    .unwrap_or_else(|| Err("script exhausted".to_string()))
            },
        )
    }
}

#[test]
fn n269_core_fail_once_then_valid_succeeds() {
    let schema = serde_json::from_str::<serde_json::Value>(FLAT_SCHEMA).unwrap();
    let mut scripted = Scripted::new(vec![
        Ok("not json at all".to_string()),
        Ok("{\"name\": \"Ada\"}".to_string()),
    ]);
    let out = scripted
        .run_core(&schema, FLAT_SCHEMA, 2)
        .expect("second attempt must validate");
    let s = out.get_field("name").expect("Struct field 'name'");
    assert!(
        matches!(s, metalogos::interpreter::Value::String(v) if v == "Ada"),
        "got {:?}",
        s
    );
    assert_eq!(scripted.calls, 2, "exactly one retry consumed");
}

#[test]
fn n269_core_no_retry_budget_left_loud_mismatch() {
    let schema = serde_json::from_str::<serde_json::Value>(FLAT_SCHEMA).unwrap();
    let mut scripted = Scripted::new(vec![Ok("garbage {{{".to_string())]);
    let err = scripted
        .run_core(&schema, FLAT_SCHEMA, 0)
        .expect_err("no retries left → loud error");
    assert!(
        err.contains("LLM_SCHEMA_MISMATCH"),
        "diagnostic code: {}",
        err
    );
    assert!(
        err.contains("gave up after 1 attempt(s)"),
        "attempt bookkeeping: {}",
        err
    );
    assert_eq!(scripted.calls, 1, "retries=0 → exactly one attempt");
}

#[test]
fn n269_core_truncation_names_max_tokens() {
    let schema = serde_json::from_str::<serde_json::Value>(FLAT_SCHEMA).unwrap();
    // A truncated JSON object: classic max_tokens cut-off shape.
    let mut scripted = Scripted::new(vec![Ok("{\"name\": \"Ada".to_string())]);
    let err = scripted
        .run_core(&schema, FLAT_SCHEMA, 0)
        .expect_err("truncated JSON is loud");
    assert!(err.contains("LLM_SCHEMA_MISMATCH"), "{}", err);
    assert!(err.contains("max_tokens"), "truncation hint: {}", err);
}

#[test]
fn n269_core_schema_side_error_does_not_call_provider() {
    let schema = serde_json::json!({"type": "object", "properties": {"a": {"type": "string", "format": "email"}}});
    let mut scripted = Scripted::new(vec![Ok("{\"a\": \"x\"}".to_string())]);
    let err = scripted
        .run_core(&schema, "{}", 3)
        .expect_err("format keyword unsupported");
    assert!(err.contains("LLM_SCHEMA_UNSUPPORTED_FEATURE"), "{}", err);
    assert_eq!(
        scripted.calls, 0,
        "schema errors must not burn LLM calls (no retry)"
    );
}

#[test]
fn n269_core_transport_error_propagates_without_retry() {
    let schema = serde_json::from_str::<serde_json::Value>(FLAT_SCHEMA).unwrap();
    let mut scripted = Scripted::new(vec![Err("backend down".to_string())]);
    let err = scripted
        .run_core(&schema, FLAT_SCHEMA, 5)
        .expect_err("transport error is loud");
    assert!(
        err.contains("backend down"),
        "propagated untouched: {}",
        err
    );
    assert_eq!(
        scripted.calls, 1,
        "no schema-retry on transport errors (SmartRouter owns failover)"
    );
}

#[test]
fn n269_core_validation_violations_fed_back_into_retry_prompt() {
    let mut prompts: Vec<String> = Vec::new();
    let schema2 = serde_json::json!({"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]});
    let mut calls = 0usize;
    let out = {
        let provider = &mut |p: &str, _i: &str| -> Result<String, String> {
            calls += 1;
            prompts.push(p.to_string());
            if calls == 1 {
                Ok("{\"name\": 42}".to_string()) // wrong type → MISMATCH → retry
            } else {
                Ok("{\"name\": \"Ada\"}".to_string())
            }
        };
        call_llm_schema_core("p", "i", &schema2, FLAT_SCHEMA, 2, provider).expect("retry fixes it")
    };
    assert!(out.get_field("name").is_ok());
    assert_eq!(calls, 2);
    assert!(
        prompts[1].contains("PREVIOUS ATTEMPT FAILED")
            && prompts[1].contains("expected type string"),
        "retry prompt must carry the validator report: {:?}",
        prompts[1]
    );
    assert!(
        prompts[1].contains(FLAT_SCHEMA),
        "retry prompt still carries the schema"
    );
}

// ── 5. Mock determinism + retries env ───────────────────────────────────

#[test]
fn n269_mock_instance_is_deterministic_and_valid() {
    let schema = serde_json::from_str::<serde_json::Value>(NESTED_SCHEMA).unwrap();
    let a = mock_instance_from_schema(&schema);
    let b = mock_instance_from_schema(&schema);
    assert_eq!(a, b, "mock must be deterministic");
    let mut v = Vec::new();
    validate_json_against_schema(&a, &schema, "$", &mut v);
    assert!(
        v.is_empty(),
        "the mock must satisfy its own schema: {:?}",
        v
    );
    // Enum answers with the first literal:
    let enum_schema = serde_json::json!({"type": "object", "properties": {"s": {"enum": ["b", "a"]}}, "required": ["s"]});
    assert_eq!(
        mock_instance_from_schema(&enum_schema)["s"],
        serde_json::json!("b")
    );
}

#[test]
fn n269_retries_env_defaults_and_cap() {
    let _env = lock_env();
    std::env::remove_var("METALOGOS_LLM_SCHEMA_RETRIES");
    assert_eq!(schema_retries_from_env(), 2, "default");

    std::env::set_var("METALOGOS_LLM_SCHEMA_RETRIES", "4");
    assert_eq!(schema_retries_from_env(), 4);

    std::env::set_var("METALOGOS_LLM_SCHEMA_RETRIES", "99");
    assert_eq!(schema_retries_from_env(), 10, "hard cap 10");

    std::env::set_var("METALOGOS_LLM_SCHEMA_RETRIES", "abc");
    assert_eq!(schema_retries_from_env(), 2, "invalid → default");

    std::env::set_var("METALOGOS_LLM_SCHEMA_RETRIES", "-1");
    assert_eq!(
        schema_retries_from_env(),
        2,
        "negative → default (u32 parse fails)"
    );

    std::env::remove_var("METALOGOS_LLM_SCHEMA_RETRIES");
}
