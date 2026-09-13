// ── Наряд №286 (P2, M1): json_validate — валидатор ADR-0133 как standalone ──
//
// Контракт (issue #341, диспатч #343):
//   1. json_validate(schema_json, value_json[, strict]) -> Struct{valid, errors}:
//      тот же подмножество ADR-0133 (type/properties/required/items/enum),
//      ошибки с путями (value.age: expected type integer, got string "33");
//   2. ГЛАВНЫЙ DoD — дифференциальный корпус: один и тот же набор
//      схем/значений проходит/отклоняется ОДИНАКОВО в call_llm_schema
//      (provider-injected, без сети) и json_validate, а тексты нарушений
//      ДО СЛОВА совпадают (валидатор один — src/schema/validate.rs);
//   3. strict-семантика: дефолт true = strict-by-default ADR-0133 D2 (поля
//      вне properties — нарушения, как в call_llm_schema); strict=false —
//      необъявленные поля разрешены, остальные правила НЕ изменены;
//   4. Громко: невалидный schema_json / ключевое слово вне подмножества /
//      не-object root — [LLM_SCHEMA_UNSUPPORTED_FEATURE] (ЕДИНЫЙ код обоих
//      путей); невалидный value_json — громкая ошибка ПАРСИНГА, не
//      valid=false (валидатор судит структуру, парсер — байты);
//   5. TW/VM паритет: json_validate в mlog-программе, оба бекенда, strict
//      как с дефолтом, так и явным аргументом.
//
// Лекало: tests/naryad_269_schema.rs (№269) и tests/naryad_284_canary.rs
// (чистые ядра, конвенция №256).

use metalogos::builtins::{call_llm_schema_core, json_validate_core};

// ── Фикстуры ────────────────────────────────────────────────────────────

const FLAT: &str =
    r#"{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}"#;

const NESTED: &str = r#"{
  "type": "object",
  "properties": {
    "user": {"type": "object", "properties": {"city": {"type": "string"}}, "required": ["city"]},
    "tags": {"type": "array", "items": {"type": "string"}},
    "age": {"type": "integer"}
  },
  "required": ["user", "tags"]
}"#;

const ENUM_SCHEMA: &str =
    r#"{"type":"object","properties":{"s":{"type":"string","enum":["a","b"]}},"required":["s"]}"#;

const TYPED: &str =
    r#"{"type":"object","properties":{"age":{"type":"integer"}},"required":["age"]}"#;

fn validate_ok(schema: &str, value: &str) -> metalogos::builtins::JsonValidateResult {
    json_validate_core(schema, value, true)
        .unwrap_or_else(|e| panic!("json_validate({schema}) failed: {e}"))
}

// ── 1. Вердикты и пути ошибок ───────────────────────────────────────────

#[test]
fn n286_valid_verdict_and_empty_errors() {
    let r = validate_ok(FLAT, r#"{"name":"Ada"}"#);
    assert!(r.valid);
    assert!(r.errors.is_empty(), "{:?}", r.errors);
}

#[test]
fn n286_violation_paths_are_exact() {
    // type-нарушение на вложенном поле: путь + ожидание + факт
    let r = validate_ok(TYPED, r#"{"age":"33"}"#);
    assert!(!r.valid);
    assert_eq!(
        r.errors,
        vec![r#"value.age: expected type integer, got string "33""#.to_string()],
        "exact violation path and wording"
    );

    // missing required — путь корня
    let r = validate_ok(TYPED, r#"{}"#);
    assert_eq!(
        r.errors,
        vec!["value: missing required field 'age'".to_string()]
    );

    // strict-by-default (дефолт): поле вне properties — нарушение
    let r = validate_ok(TYPED, r#"{"age":1,"x":true}"#);
    assert_eq!(
        r.errors,
        vec![
            "value: unexpected field 'x' is not declared in schema properties (strict-by-default, ADR-0133 D2)"
                .to_string()
        ]
    );
}

#[test]
fn n286_nested_and_array_paths() {
    let r = validate_ok(NESTED, r#"{"user":{"city":42},"tags":[]}"#);
    assert!(!r.valid);
    assert_eq!(
        r.errors,
        vec!["value.user.city: expected type string, got number 42".to_string()],
        "dot-path through nested object"
    );

    let r = validate_ok(NESTED, r#"{"user":{"city":"Oslo"},"tags":["a",7]}"#);
    assert_eq!(
        r.errors,
        vec!["value.tags[1]: expected type string, got number 7".to_string()],
        "indexed path through array"
    );

    // несколько нарушений собираются ВСЕ, порядок обхода стабильный
    let r = validate_ok(NESTED, r#"{"tags":[]}"#);
    assert_eq!(r.errors.len(), 1, "missing required user");
    let r = validate_ok(NESTED, r#"{"user":{"city":"Oslo"}}"#);
    assert_eq!(r.errors.len(), 1, "missing required tags");
}

#[test]
fn n286_enum_violation_precedence() {
    let r = validate_ok(ENUM_SCHEMA, r#"{"s":"z"}"#);
    assert!(!r.valid);
    assert_eq!(r.errors.len(), 1, "enum hit short-circuits type noise");
    assert!(
        r.errors[0].starts_with(r#"value.s: value "z" is not one of the enum literals"#),
        "{}",
        r.errors[0]
    );
}

// ── 2. Громкие ошибки: парсер судит байты, валидатор — структуру ────────

#[test]
fn n286_unparseable_schema_is_loud_unsupported_feature() {
    let err = json_validate_core("{not json", "{}", true).unwrap_err();
    assert!(
        err.contains("LLM_SCHEMA_UNSUPPORTED_FEATURE"),
        "единый код с call_llm_schema: {err}"
    );
    assert!(err.contains("schema_json is not valid JSON"), "{err}");

    // ключевое слово вне подмножества — тот же код
    let err = json_validate_core(
        r#"{"type":"object","properties":{"a":{"type":"string","pattern":"^a"}}}"#,
        r#"{"a":"x"}"#,
        true,
    )
    .unwrap_err();
    assert!(err.contains("LLM_SCHEMA_UNSUPPORTED_FEATURE"), "{err}");
    assert!(err.contains("'pattern'"), "{err}");

    // root-object контракт ADR-0133 — общий для обоих путей
    let err = json_validate_core(r#"{"type":"string"}"#, "\"x\"", true).unwrap_err();
    assert!(err.contains("root schema must be"), "{err}");
}

#[test]
fn n286_unparseable_value_is_loud_parse_error_not_valid_false() {
    let err = json_validate_core(FLAT, "{broken", true).unwrap_err();
    assert!(
        err.contains("value_json is not valid JSON"),
        "громкая ошибка парсинга: {err}"
    );
}

// ── 3. strict-семантика ─────────────────────────────────────────────────

#[test]
fn n286_strict_false_permits_undeclared_fields_only() {
    // strict=false: необъявленные поля разрешены
    let r = json_validate_core(TYPED, r#"{"age":1,"extra":"ok"}"#, false).expect("core");
    assert!(r.valid, "{:?}", r.errors);

    // ...но declared-поля по-прежнему проверяются на тип
    let r = json_validate_core(TYPED, r#"{"age":"x","extra":"ok"}"#, false).expect("core");
    assert!(!r.valid);
    assert!(
        r.errors[0].contains("value.age: expected type integer"),
        "{:?}",
        r.errors
    );

    // ...required по-прежнему обязателен
    let r = json_validate_core(TYPED, r#"{"extra":"ok"}"#, false).expect("core");
    assert_eq!(
        r.errors,
        vec!["value: missing required field 'age'".to_string()]
    );

    // ...enum по-прежнему действует
    let r = json_validate_core(ENUM_SCHEMA, r#"{"s":"z","extra":1}"#, false).expect("core");
    assert!(!r.valid);

    // ...и вложенные необъявленные поля тоже разрешены
    let r = json_validate_core(
        NESTED,
        r#"{"user":{"city":"Oslo","dept":"x"},"tags":[]}"#,
        false,
    )
    .expect("core");
    assert!(r.valid, "{:?}", r.errors);

    // strict=true (дефолт) — то же значение отклоняется
    let r = json_validate_core(
        NESTED,
        r#"{"user":{"city":"Oslo","dept":"x"},"tags":[]}"#,
        true,
    )
    .expect("core");
    assert!(!r.valid);
}

// ── 4. ДИФФЕРЕНЦИАЛЬНЫЙ КОРПУС: call_llm_schema ≡ json_validate ─────────

/// (schema, value, expect_valid) — общий корпус обоих путей. Каждый кейс
/// прогоняется через json_validate_core и через call_llm_schema_core с
/// provider-инъекцией значения (без сети, retries=0): вердикты обязаны
/// совпадать, а тексты нарушений — до слова.
const CORPUS: &[(&str, &str, bool)] = &[
    (FLAT, r#"{"name":"Ada"}"#, true),
    // лишнее поле: strict-by-default обоих путей
    (FLAT, r#"{"name":"Ada","age":"33"}"#, false),
    // missing required
    (FLAT, r#"{}"#, false),
    // wrong type
    (FLAT, r#"{"name":42}"#, false),
    // nested valid
    (
        NESTED,
        r#"{"user":{"city":"Oslo"},"tags":["a","b"],"age":33}"#,
        true,
    ),
    // nested wrong type
    (NESTED, r#"{"user":{"city":42},"tags":[]}"#, false),
    // missing nested required
    (NESTED, r#"{"tags":["x"]}"#, false),
    // array item wrong type
    (NESTED, r#"{"user":{"city":"Oslo"},"tags":["a",7]}"#, false),
    // enum hit / enum miss
    (ENUM_SCHEMA, r#"{"s":"a"}"#, true),
    (ENUM_SCHEMA, r#"{"s":"z"}"#, false),
    // схема без properties: пустой объект валиден
    (r#"{"type":"object"}"#, r#"{}"#, true),
];

#[test]
fn n286_differential_corpus_verdicts_and_texts_identical() {
    for (i, (schema, value, expect_valid)) in CORPUS.iter().enumerate() {
        let parsed: serde_json::Value =
            serde_json::from_str(schema).unwrap_or_else(|e| panic!("fixture {i}: {e}"));

        // Путь 1: json_validate (strict=true — дефолт обоих путей)
        let json_res = json_validate_core(schema, value, true);

        // Путь 2: call_llm_schema (provider возвращает ТО ЖЕ значение)
        let llm_res = call_llm_schema_core("p", "i", &parsed, schema, 0, &mut |_, _| {
            Ok(value.to_string())
        });

        match (expect_valid, json_res, llm_res) {
            (true, Ok(j), Ok(_)) => {
                assert!(j.valid, "fixture {i}: json verdict must be valid");
                assert!(j.errors.is_empty(), "fixture {i}: {:?}", j.errors);
            }
            (false, Ok(j), Err(llm_err)) => {
                assert!(!j.valid, "fixture {i}: json verdict must be invalid");
                assert!(
                    llm_err.contains("LLM_SCHEMA_MISMATCH"),
                    "fixture {i}: llm path must be loud: {llm_err}"
                );
                // ТЕКСТОВАЯ ПАРИТЕТНОСТЬ: каждое нарушение json_validate
                // входит в отчёт call_llm_schema до слова. Единственное
                // ЗАДУМАННОЕ различие — корневой ярлык документа ("value" у
                // json_validate против "answer" у LLM-пути), нормализуем его.
                for e in &j.errors {
                    let want = e.replacen("value", "answer", 1);
                    assert!(e.starts_with("value"), "fixture {i}: {e}");
                    assert!(
                        llm_err.contains(want.as_str()),
                        "fixture {i}: violation text diverged:\n  json: {e}\n  llm:  {llm_err}"
                    );
                }
            }
            (true, Ok(j), Err(e)) => panic!(
                "fixture {i}: VERDICT DIVERGENCE — json says valid ({:?}), llm rejected: {e}",
                j.errors
            ),
            (true, Err(e), _) => panic!("fixture {i}: json loud on valid case: {e}"),
            (false, Ok(j), Ok(_)) => panic!(
                "fixture {i}: VERDICT DIVERGENCE — llm accepted, json flagged {:?}",
                j.errors
            ),
            (false, Err(e), _) => panic!("fixture {i}: json loud on invalid case: {e}"),
        }
    }
}

#[test]
fn n286_differential_schema_side_rejections_identical() {
    // Schema-side (не-JSON / вне подмножества / не-object root): оба пути
    // отвергают громко, ЕДИНЫМ кодом
    let bad_schemas = [
        "{not json",
        r#"{"type":"object","properties":{"a":{"type":"string","pattern":"^a"}}}"#,
        r#"{"type":"string"}"#,
        r#"{"type":"object","properties":{},"additionalProperties":false}"#,
    ];
    for (i, schema) in bad_schemas.iter().enumerate() {
        let j = json_validate_core(schema, "{}", true).unwrap_err();
        assert!(
            j.contains("LLM_SCHEMA_UNSUPPORTED_FEATURE"),
            "bad schema {i} json: {j}"
        );
        let parsed: serde_json::Value =
            serde_json::from_str(schema).unwrap_or(serde_json::Value::Null);
        let l = call_llm_schema_core("p", "i", &parsed, schema, 0, &mut |_, _| {
            Ok("{}".to_string())
        })
        .unwrap_err();
        assert!(
            l.contains("LLM_SCHEMA_UNSUPPORTED_FEATURE"),
            "bad schema {i} llm: {l}"
        );
    }
}

// ── 5. Языковой уровень: TW/VM паритет ──────────────────────────────────

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, std::path::PathBuf::from("."))
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations = metalogos::parser::parse(source).map_err(|e| format!("parse: {e}"))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(std::path::PathBuf::from("."));
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

const N286_CONTRACT: &str = r#"
pattern Check(raw: String) -> String {
  let schema = "{\"type\":\"object\",\"properties\":{\"rows\":{\"type\":\"array\",\"items\":{\"type\":\"integer\"}}},\"required\":[\"rows\"]}"
  let r = json_validate(schema, raw)
  if r.valid {
    return "ok"
  }
  return "bad"
}
flow Main {
  input: String = "{\"rows\":[1,2,3]}"
  -> Check
  -> output
}
"#;

const N286_CONTRACT_BAD: &str = r#"
pattern Check(raw: String) -> String {
  let schema = "{\"type\":\"object\",\"properties\":{\"rows\":{\"type\":\"array\",\"items\":{\"type\":\"integer\"}}},\"required\":[\"rows\"]}"
  let r = json_validate(schema, raw)
  if r.valid {
    return "ok"
  }
  return "bad"
}
flow Main {
  input: String = "{\"rows\":[1,\"x\"]}"
  -> Check
  -> output
}
"#;

#[test]
fn n286_language_contract_valid_tw_and_vm() {
    let tw = run_tw(N286_CONTRACT).expect("TW run").unwrap_or_default();
    assert_eq!(
        tw, "ok",
        "TW: shape-before-use accepts the MCP-style payload"
    );
    let vm = run_vm(N286_CONTRACT).expect("VM run").unwrap_or_default();
    assert_eq!(vm, "ok", "VM: parity with TW on valid payload");
}

#[test]
fn n286_language_contract_invalid_tw_and_vm() {
    let tw = run_tw(N286_CONTRACT_BAD)
        .expect("TW run")
        .unwrap_or_default();
    assert_eq!(tw, "bad", "TW: type violation in array rejected");
    let vm = run_vm(N286_CONTRACT_BAD)
        .expect("VM run")
        .unwrap_or_default();
    assert_eq!(vm, "bad", "VM: parity with TW on invalid payload");
}

#[test]
fn n286_language_strict_explicit_argument() {
    // strict=true явно — как дефолт; strict=false — лишнее поле разрешено
    let src_true = r#"
pattern Check(raw: String) -> String {
  let schema = "{\"type\":\"object\",\"properties\":{\"rows\":{\"type\":\"array\",\"items\":{\"type\":\"integer\"}}},\"required\":[\"rows\"]}"
  let r = json_validate(schema, raw, true)
  if r.valid {
    return "ok"
  }
  return "bad"
}
flow Main {
  input: String = "{\"rows\":[1],\"extra\":true}"
  -> Check
  -> output
}
"#;
    let src_false = src_true.replace("raw, true)", "raw, false)");

    for (src, expect, label) in [
        (
            src_true,
            "bad",
            "strict=true: extra field rejected (both backends)",
        ),
        (
            src_false.as_str(),
            "ok",
            "strict=false: extra field permitted (both backends)",
        ),
    ] {
        let tw = run_tw(src).expect("TW run").unwrap_or_default();
        assert_eq!(tw, expect, "TW {label}");
        let vm = run_vm(src).expect("VM run").unwrap_or_default();
        assert_eq!(vm, expect, "VM {label}");
    }
}

#[test]
fn n286_language_loud_errors() {
    // arity: 1 аргумент — громкая ошибка арности из реестра
    let err = run_tw(
        r#"
pattern Check(raw: String) -> String {
  let r = json_validate("{}")
  return str(r.valid)
}
flow Main { input: String = "x" -> Check -> output }
"#,
    )
    .unwrap_err();
    assert!(err.contains("2..3"), "arity pin: {err}");

    // strict не-Bool — громкая ошибка типа
    let err = run_tw(
        r#"
pattern Check(raw: String) -> String {
  let r = json_validate("{}", "{}", 1.0)
  return str(r.valid)
}
flow Main { input: String = "x" -> Check -> output }
"#,
    )
    .unwrap_err();
    assert!(
        err.contains("expected Boolean as third arg (strict)"),
        "strict must be Boolean: {err}"
    );

    // невалидный value_json — громкая ошибка парсинга (не valid=false);
    // схема валидна (subset прошёл) — громко именно про байты значения
    let err = run_tw(
        r#"
pattern Check(raw: String) -> String {
  let r = json_validate("{\"type\":\"object\",\"properties\":{\"name\":{\"type\":\"string\"}},\"required\":[\"name\"]}", "{broken")
  return str(r.valid)
}
flow Main { input: String = "x" -> Check -> output }
"#,
    )
    .unwrap_err();
    assert!(
        err.contains("value_json is not valid JSON"),
        "loud parse error: {err}"
    );
}
