// ── Structured LLM output: `call_llm_schema` + JSON-Schema subset validator ──
// Наряд №269, ADR-0133.
//
// Contract: `call_llm_schema(prompt, schema_json)` / `call_llm_schema(prompt, input, schema_json)`
// -> Value::Struct (type_name "Dict") whose top-level fields mirror the schema's
// `properties`. The backend answer must parse as JSON and validate against the
// SUPPORTED subset of JSON Schema (D1): type/properties/required/items/enum.
// Pure-annotation keywords (D1) are inert and ignored; anything else in the
// schema is a LOUD `[LLM_SCHEMA_UNSUPPORTED_FEATURE]` (retrying cannot help).
// Answer-side failures (unparseable JSON / schema violations / truncation) are
// loud `[LLM_SCHEMA_MISMATCH]` and DO trigger the retry loop (D3).
//
// Deviation from JSON-Schema defaults (D2, honest strictness): fields in the
// answer that are NOT declared in `properties` are violations — strict-by-default,
// which makes the unsupported `additionalProperties` keyword redundant.

use crate::interpreter::Value;

use super::core::expect_string_arg;

/// Keywords the validator actually enforces (ADR-0133 D1).
const SUPPORTED_KEYWORDS: &[&str] = &["type", "properties", "required", "items", "enum"];
/// Pure-annotation keywords: inert metadata, cannot weaken validation — ignored
/// silently so real-world schemas (which carry `title`/`description`/`$schema`)
/// do not bounce (ADR-0133 D1). This list is CLOSED; anything outside it errors.
const IGNORED_ANNOTATIONS: &[&str] = &[
    "$schema",
    "$id",
    "$comment",
    "title",
    "description",
    "default",
    "examples",
];

/// System directive appended to the user prompt (ADR-0133 D5).
const SCHEMA_DIRECTIVE: &str = "Respond with ONLY a single valid JSON value that conforms EXACTLY to the JSON Schema below. No prose, no markdown code fences, no comments, no trailing commas. The entire answer must be parseable by a strict JSON parser.\nJSON Schema:\n";

/// Retry feedback appended on attempts > 0 (ADR-0133 D3).
const RETRY_FEEDBACK_TAIL: &str =
    "\n\nFix every issue below and answer again with ONLY valid JSON.\n";

/// Default retry budget when `METALOGOS_LLM_SCHEMA_RETRIES` is unset/invalid.
const DEFAULT_RETRIES: u32 = 2;
/// Hard cap on retries — runaway budgets cannot spin the LLM loop forever.
const MAX_RETRIES: u32 = 10;

// ── Schema-side checks (NOT retryable) ────────────────────────────────────

/// Validate that `schema` lives inside the supported subset (ADR-0133 D1).
/// The ROOT schema must describe an object (the builtin returns a Struct).
/// Nested schemas may be object/array/string/number/integer/boolean/null.
pub fn check_schema_supported(schema: &serde_json::Value) -> Result<(), String> {
    check_schema_node(schema, true, "$")
}

fn check_schema_node(schema: &serde_json::Value, is_root: bool, path: &str) -> Result<(), String> {
    let map = match schema.as_object() {
        Some(m) => m,
        None => {
            return Err(format!(
                "call_llm_schema(): [LLM_SCHEMA_UNSUPPORTED_FEATURE] schema node at {} must be a JSON object, got {} (ADR-0133)",
                path, json_type_name(schema)
            ))
        }
    };
    for key in map.keys() {
        if SUPPORTED_KEYWORDS.contains(&key.as_str()) || IGNORED_ANNOTATIONS.contains(&key.as_str())
        {
            continue;
        }
        return Err(format!(
            "call_llm_schema(): [LLM_SCHEMA_UNSUPPORTED_FEATURE] schema keyword '{}' at {} is outside the supported subset {:?} (annotations {:?} are ignored); see ADR-0133",
            key, path, SUPPORTED_KEYWORDS, IGNORED_ANNOTATIONS
        ));
    }
    if is_root {
        let root_type = map.get("type").and_then(|t| t.as_str());
        if root_type != Some("object") {
            return Err(format!(
                "call_llm_schema(): [LLM_SCHEMA_UNSUPPORTED_FEATURE] root schema must be {{\"type\":\"object\",...}} — the builtin returns a Struct; got type {:?} at {} (ADR-0133)",
                root_type, path
            ));
        }
    }
    if let Some(t) = map.get("type") {
        if !t.is_string() {
            return Err(format!(
                "call_llm_schema(): [LLM_SCHEMA_UNSUPPORTED_FEATURE] 'type' at {} must be a string, got {} (ADR-0133)",
                path, json_type_name(t)
            ));
        }
    }
    if let Some(props) = map.get("properties") {
        let props = match props.as_object() {
            Some(p) => p,
            None => {
                return Err(format!(
                    "call_llm_schema(): [LLM_SCHEMA_UNSUPPORTED_FEATURE] 'properties' at {} must be an object, got {} (ADR-0133)",
                    path, json_type_name(props)
                ))
            }
        };
        for (name, sub) in props {
            check_schema_node(sub, false, &format!("{}.properties.{}", path, name))?;
        }
    }
    if let Some(req) = map.get("required") {
        let all_strings = req.is_array()
            && req
                .as_array()
                .is_some_and(|a| a.iter().all(|v| v.is_string()));
        if !all_strings {
            return Err(format!(
                "call_llm_schema(): [LLM_SCHEMA_UNSUPPORTED_FEATURE] 'required' at {} must be an array of strings (ADR-0133)",
                path
            ));
        }
    }
    if let Some(items) = map.get("items") {
        check_schema_node(items, false, &format!("{}.items", path))?;
    }
    if let Some(en) = map.get("enum") {
        let non_empty_array = en.is_array() && en.as_array().is_some_and(|a| !a.is_empty());
        if !non_empty_array {
            return Err(format!(
                "call_llm_schema(): [LLM_SCHEMA_UNSUPPORTED_FEATURE] 'enum' at {} must be a non-empty array (ADR-0133)",
                path
            ));
        }
    }
    Ok(())
}

// ── Answer-side validation (retryable) ────────────────────────────────────

/// Validate `value` against `schema` (already subset-checked), collecting ALL
/// violations instead of failing fast — the full report is the retry context
/// and the diagnostic text (ADR-0133 D3). Pure function, no I/O.
pub fn validate_json_against_schema(
    value: &serde_json::Value,
    schema: &serde_json::Value,
    path: &str,
    violations: &mut Vec<String>,
) {
    // enum applies to any type and takes precedence in reporting
    if let Some(en) = schema.get("enum").and_then(|e| e.as_array()) {
        if !en.iter().any(|allowed| allowed == value) {
            violations.push(format!(
                "{}: value {} is not one of the enum literals {:?}",
                display_path(path),
                short_repr(value),
                en
            ));
            return; // enum hit already describes the problem; type errors would be noise
        }
    }
    let expected = schema.get("type").and_then(|t| t.as_str());
    if let Some(t) = expected {
        if !type_matches(value, t) {
            violations.push(format!(
                "{}: expected type {}, got {} {}",
                display_path(path),
                t,
                json_type_name(value),
                short_repr(value)
            ));
            return; // deeper checks are meaningless against the wrong type
        }
    }
    match value {
        serde_json::Value::Object(map) => {
            let props = schema.get("properties").and_then(|p| p.as_object());
            if let Some(props) = props {
                for key in map.keys() {
                    if !props.contains_key(key) {
                        violations.push(format!(
                            "{}: unexpected field '{}' is not declared in schema properties (strict-by-default, ADR-0133 D2)",
                            display_path(path),
                            key
                        ));
                    }
                }
                for (name, sub) in props {
                    if let Some(v) = map.get(name) {
                        validate_json_against_schema(
                            v,
                            sub,
                            &format!("{}.{}", path, name),
                            violations,
                        );
                    }
                }
            }
            if let Some(req) = schema.get("required").and_then(|r| r.as_array()) {
                // Non-string entries were rejected schema-side; skip defensively
                // instead of panicking (house rule: no unwrap/expect in lib code).
                for name in req.iter().filter_map(|v| v.as_str()) {
                    if !map.contains_key(name) {
                        violations.push(format!(
                            "{}: missing required field '{}'",
                            display_path(path),
                            name
                        ));
                    }
                }
            }
        }
        serde_json::Value::Array(items) => {
            if let Some(items_schema) = schema.get("items") {
                for (i, item) in items.iter().enumerate() {
                    validate_json_against_schema(
                        item,
                        items_schema,
                        &format!("{}[{}]", path, i),
                        violations,
                    );
                }
            }
        }
        _ => {} // scalars: the type/enum check above is the whole contract
    }
}

fn type_matches(value: &serde_json::Value, t: &str) -> bool {
    match t {
        "string" => value.is_string(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        "number" => value.is_number(),
        // JSON has one number type; "integer" is the integral subset (ADR-0133 D1)
        "integer" => value.is_i64() || value.is_u64(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        _ => false, // schema-side check rejects unknown types before validation
    }
}

// ── Deterministic mock (ADR-0133 D4) ──────────────────────────────────────

/// Minimal valid instance derived from the schema: every declared property is
/// present, required or not; enums answer with their first literal; strings
/// answer with their own key name (top-level scalars would have no key, so
/// they fall back to "text"). Deterministic — tests assert exact values.
pub fn mock_instance_from_schema(schema: &serde_json::Value) -> serde_json::Value {
    mock_node(schema, None)
}

fn mock_node(schema: &serde_json::Value, key: Option<&str>) -> serde_json::Value {
    if let Some(en) = schema.get("enum").and_then(|e| e.as_array()) {
        if let Some(first) = en.first() {
            return first.clone();
        }
    }
    match schema.get("type").and_then(|t| t.as_str()) {
        Some("string") => serde_json::Value::String(key.unwrap_or("text").to_string()),
        Some("integer") => serde_json::json!(1),
        Some("number") => serde_json::json!(1.0),
        Some("boolean") => serde_json::Value::Bool(true),
        Some("null") => serde_json::Value::Null,
        Some("array") => {
            let items = schema
                .get("items")
                .cloned()
                .unwrap_or(serde_json::json!(null));
            serde_json::Value::Array(vec![mock_node(&items, key)])
        }
        Some("object") => {
            let mut map = serde_json::Map::new();
            if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
                for (name, sub) in props {
                    map.insert(name.clone(), mock_node(sub, Some(name)));
                }
            }
            serde_json::Value::Object(map)
        }
        _ => serde_json::Value::Null,
    }
}

// ── Core retry loop (provider-injected; unit-testable without network) ────

/// Execute the schema contract over an injectable answer provider.
/// `answer(final_prompt, input)` mirrors the SmartRouter slot signature so the
/// builtin wires `call_via_smart_router` in directly (ADR-0048 reuse).
///
/// Retry policy (ADR-0133 D3): parse/validation failures retry up to `retries`
/// extra attempts with the validator report fed back into the prompt;
/// transport errors and schema-side (`LLM_SCHEMA_UNSUPPORTED_FEATURE`) errors
/// return immediately — retrying cannot fix either.
pub fn call_llm_schema_core(
    prompt: &str,
    input: &str,
    schema: &serde_json::Value,
    schema_json: &str,
    retries: u32,
    answer: &mut dyn FnMut(&str, &str) -> Result<String, String>,
) -> Result<Value, String> {
    check_schema_supported(schema)?;

    let base_prompt = format!(
        "{}\n{}",
        prompt.trim_end(),
        schema_directive_block(schema_json)
    );
    let total_attempts = retries + 1;
    let mut last_failure = String::new();

    for attempt in 0..total_attempts {
        let final_prompt = if attempt == 0 {
            base_prompt.clone()
        } else {
            format!(
                "{}{}PREVIOUS ATTEMPT FAILED:\n{}{}",
                base_prompt, RETRY_FEEDBACK_TAIL, last_failure, RETRY_FEEDBACK_TAIL
            )
        };

        // Transport errors propagate untouched — SmartRouter already did its
        // own failover; a second layer here would double-call dead providers.
        let raw = answer(&final_prompt, input)?;

        let parsed: serde_json::Value = match serde_json::from_str(raw.trim()) {
            Ok(v) => v,
            Err(e) => {
                last_failure = format!(
                    "[LLM_SCHEMA_MISMATCH] the answer is not valid JSON: {} — if the text looks cut off mid-object, the backend hit max_tokens and truncation is the cause (large schemas need headroom)",
                    e
                );
                continue;
            }
        };

        let mut violations = Vec::new();
        validate_json_against_schema(&parsed, schema, "$", &mut violations);
        if violations.is_empty() {
            return Ok(json_to_mlog(&parsed));
        }
        last_failure = format!(
            "[LLM_SCHEMA_MISMATCH] {} violation(s): {}",
            violations.len(),
            violations.join("; ")
        );
    }

    // last_failure already carries the [LLM_SCHEMA_MISMATCH] code from the
    // parse or validation branch; append only the attempt bookkeeping.
    Err(format!(
        "call_llm_schema(): {} — gave up after {} attempt(s)",
        last_failure, total_attempts
    ))
}

// Truncation is detected structurally (unparseable JSON) and reported with the
// max_tokens hint above — there is no usage metadata on the legacy path to
// inspect, and inventing one would be dishonest precision (ADR-0133 D3).

/// Read `METALOGOS_LLM_SCHEMA_RETRIES` (default 2, hard cap 10, invalid/… → default).
pub fn schema_retries_from_env() -> u32 {
    std::env::var("METALOGOS_LLM_SCHEMA_RETRIES")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .map(|v| v.min(MAX_RETRIES))
        .unwrap_or(DEFAULT_RETRIES)
}

// ── Builtin entry point ───────────────────────────────────────────────────

/// `call_llm_schema(prompt, schema_json)` / `call_llm_schema(prompt, input, schema_json)`
/// -> Struct ("Dict"). See module docs + ADR-0133.
pub(crate) fn builtin_call_llm_schema(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() || args.len() > 3 {
        return Err(format!(
            "call_llm_schema() requires 2 or 3 arguments (prompt, schema_json | prompt, input, schema_json), got {}",
            args.len()
        ));
    }
    let prompt = expect_string_arg("call_llm_schema", args, 0)?;
    let schema_json = expect_string_arg("call_llm_schema", args, args.len() - 1)?;
    let input = match args.len() {
        3 => match &args[1] {
            Value::String(s) => s.clone(),
            other => format!("{}", other),
        },
        _ => String::new(),
    };

    let schema: serde_json::Value = serde_json::from_str(&schema_json).map_err(|e| {
        format!(
            "call_llm_schema(): [LLM_SCHEMA_UNSUPPORTED_FEATURE] schema_json is not valid JSON: {}",
            e
        )
    })?;
    let retries = schema_retries_from_env();

    call_llm_schema_core(
        &prompt,
        &input,
        &schema,
        &schema_json,
        retries,
        &mut |p, i| {
            // Same SmartRouter slot as call_llm (Наряд №4 / ADR-0048), model and
            // timeout overrides stay None — schema calls are not special there.
            if let Some(result) = crate::llm::call_via_smart_router(p, i, None, None) {
                return result;
            }
            // No SmartRouter: mock tier mirrors call_llm. In schema mode ANY mock
            // value (unset default / true / 1 / json) yields the deterministic
            // schema-derived instance — a text mock would be a guaranteed loud
            // failure, which helps nobody (ADR-0133 D4).
            // Наряд №276: non-router paths are traced HERE (one line per actual
            // provider invocation — a retried schema call produces one trace per
            // retry); the SmartRouter path traces inside SmartRouter::call.
            let mock_mode = std::env::var("METALOGOS_LLM_MOCK")
                .map(|v| v != "false" && v != "0")
                .unwrap_or(true);
            let t0 = std::time::Instant::now();
            if mock_mode {
                let res = serde_json::to_string(&mock_instance_from_schema(&schema))
                    .map_err(|e| format!("call_llm_schema(): mock generation failed: {}", e));
                crate::llm::trace_llm_call(&crate::llm::LlmTraceEvent {
                    provider_name: Some("mock"),
                    model: None,
                    input_tokens: None,
                    output_tokens: None,
                    latency_ms: t0.elapsed().as_millis() as u64,
                    status: if res.is_ok() { "ok" } else { "error" },
                    cache: "miss",
                    provider_alias: None,
                });
                return res;
            }
            let res = crate::llm::create_llm_backend()
                .call(p, i)
                .map_err(|e| format!("call_llm_schema() backend failed: {}", e));
            let model_env = std::env::var("METALOGOS_LLM_MODEL").ok();
            crate::llm::trace_llm_call(&crate::llm::LlmTraceEvent {
                provider_name: Some(crate::llm::provider_env_name()),
                model: model_env.as_deref(),
                input_tokens: None,
                output_tokens: None,
                latency_ms: t0.elapsed().as_millis() as u64,
                status: if res.is_ok() { "ok" } else { "error" },
                cache: "miss",
                provider_alias: None,
            });
            res
        },
    )
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn schema_directive_block(schema_json: &str) -> String {
    format!("{}{}", SCHEMA_DIRECTIVE, schema_json)
}

fn json_to_mlog(value: &serde_json::Value) -> Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut fields = std::collections::HashMap::new();
            for (k, v) in map {
                fields.insert(k.clone(), json_to_mlog(v));
            }
            Value::Struct {
                // "Dict" — the JSON-toolchain struct convention (json.rs), so the
                // result interops with json_get / has_field / dict_* for free.
                type_name: "Dict".to_string(),
                fields,
            }
        }
        serde_json::Value::Array(items) => Value::List(items.iter().map(json_to_mlog).collect()),
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => Value::Float(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::Null => Value::Unit,
    }
}

fn json_type_name(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn display_path(path: &str) -> String {
    if path == "$" {
        "answer".to_string()
    } else {
        format!(
            "answer.{}",
            path.trim_start_matches("$.").trim_start_matches('$')
        )
    }
}

fn short_repr(v: &serde_json::Value) -> String {
    let s = v.to_string();
    if s.len() > 60 {
        format!("{}...", &s[..60])
    } else {
        s
    }
}
