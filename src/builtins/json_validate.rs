// ── Наряд №286 (P2, M1): `json_validate` — валидатор ADR-0133 как standalone ──
//
// json_validate(schema_json, value_json) -> Struct{valid: Bool, errors: List<String>}
// json_validate(schema_json, value_json, strict) -> Struct{valid, errors}
//
// «Shape-before-use» (№286): данные приходят в программу не только от LLM —
// MCP tool-outputs (№268, #304), HTTP-ответы, request_body (UserInput taint),
// поля из БД. Раньше проверять их форму было нечем: валидатор ADR-0133 жил
// только внутри call_llm_schema. Теперь он извлечён в общий модуль
// (src/schema/validate.rs) и здесь оборачивается builtin'ом — НЕ ОДНОГО
// нового правила; дифференциальный тест (tests/naryad_286_json_validate.rs)
// прогоняет общий корпус схем/значений через ОБА пути и требует одинаковых
// вердиктов и одинаковых текстов нарушений.
//
// Контракт:
//   - результат Struct{"Dict"}{valid, errors}: valid — Bool, errors —
//     List<String> с путями нарушений (`value.age: expected type integer,
//     got string "33"`); пустой errors ⟺ valid;
//   - strict (третий аргумент, дефолт true): true = strict-by-default
//     ADR-0133 D2 — поля вне properties являются нарушениями (как в
//     call_llm_schema); false = необъявленные поля разрешены (opt-in
//     №286), остальные правила (type/required/items/enum, подмножество)
//     НЕ изменены;
//   - невалидный schema_json / ключевое слово вне подмножества /
//     не-object root — громкий [LLM_SCHEMA_UNSUPPORTED_FEATURE]: ЕДИНЫЙ
//     код с call_llm_schema (один и тот же check_schema_subset);
//   - невалидный value_json — громкая ошибка ПАРСИНГА, а не valid=false:
//     валидатор судит структуру, парсер — байты (конвенция parse_json
//     «loud error»); panic-свободно, без unwrap/expect (house rule);
//   - чистое ядро json_validate_core экспортировано для тестов
//     (конвенция №256, лекало canary_*_core №284).

use super::core::expect_string_arg;
use crate::interpreter::Value;

/// Результат `json_validate`: вердикт + полный список нарушений с путями.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonValidateResult {
    pub valid: bool,
    pub errors: Vec<String>,
}

/// Чистое ядро json_validate (без Value-обвязки): парсинг schema_json —
/// громкий [LLM_SCHEMA_UNSUPPORTED_FEATURE] (единый код с call_llm_schema),
/// затем subset-проверка схемы (root-object контракт ADR-0133 — общий для
/// обоих путей), затем парсинг value_json — громкая ошибка парсинга
/// (валидатор судит структуру, парсер — байты), затем валидация значения
/// ОДНИМ с call_llm_schema валидатором.
pub fn json_validate_core(
    schema_json: &str,
    value_json: &str,
    strict: bool,
) -> Result<JsonValidateResult, String> {
    let schema: serde_json::Value =
        serde_json::from_str(schema_json).map_err(|e| {
            format!(
                "json_validate(): [LLM_SCHEMA_UNSUPPORTED_FEATURE] schema_json is not valid JSON: {} (ADR-0133)",
                e
            )
        })?;
    crate::schema::validate::check_schema_subset(&schema, "json_validate")?;
    let value: serde_json::Value = serde_json::from_str(value_json)
        .map_err(|e| format!("json_validate() error: value_json is not valid JSON: {}", e))?;
    let mut violations = Vec::new();
    crate::schema::validate::validate_json(&value, &schema, "$", "value", strict, &mut violations);
    Ok(JsonValidateResult {
        valid: violations.is_empty(),
        errors: violations,
    })
}

/// `json_validate(schema_json, value_json)` / `json_validate(schema_json, value_json, strict)`
/// -> Struct ("Dict") {valid, errors}. См. модульную шапку + ADR-0133 + №286.
pub(crate) fn builtin_json_validate(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err(format!(
            "json_validate() expects 2..3 arguments (schema_json, value_json [, strict]), got {}",
            args.len()
        ));
    }
    let schema_json = expect_string_arg("json_validate", args, 0)?;
    let value_json = expect_string_arg("json_validate", args, 1)?;
    let strict = match args.get(2) {
        None => true,
        Some(Value::Bool(b)) => *b,
        Some(other) => {
            return Err(format!(
                "json_validate() expected Boolean as third arg (strict), got {}",
                other.type_name()
            ))
        }
    };
    let r = json_validate_core(&schema_json, &value_json, strict)?;
    let mut fields = std::collections::HashMap::new();
    fields.insert("valid".to_string(), Value::Bool(r.valid));
    fields.insert(
        "errors".to_string(),
        Value::List(r.errors.into_iter().map(Value::String).collect()),
    );
    Ok(Value::Struct {
        type_name: "Dict".to_string(),
        fields,
    })
}
