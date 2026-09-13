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
//
// Наряд №286: the validator (subset check + instance validation) moved to
// src/schema/validate.rs — SHARED with `json_validate` so untrusted non-LLM
// data (MCP tool-outputs, HTTP responses) is judged by the same rules with
// zero new ones. The two shims below keep the №269 public API and the
// byte-identical diagnostics (differential corpus: tests/naryad_286_json_validate.rs).

use crate::interpreter::Value;

use super::core::expect_string_arg;

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

/// Compat shim (Наряд №286 extraction): subset check with the historical
/// `call_llm_schema()` diagnostics. The validator itself now lives in
/// `crate::schema::validate` and is SHARED with `json_validate` — same
/// rules, same messages, zero new ones.
pub fn check_schema_supported(schema: &serde_json::Value) -> Result<(), String> {
    crate::schema::validate::check_schema_subset(schema, "call_llm_schema")
}

// ── Answer-side validation (retryable) ────────────────────────────────────

/// Compat shim (Наряд №286 extraction): strict-by-default validation with the
/// historical `answer` root label — the retry context of the schema loop.
/// The validator itself now lives in `crate::schema::validate` and is SHARED
/// with `json_validate` (which opts into `strict = false` explicitly).
pub fn validate_json_against_schema(
    value: &serde_json::Value,
    schema: &serde_json::Value,
    path: &str,
    violations: &mut Vec<String>,
) {
    crate::schema::validate::validate_json(value, schema, path, "answer", true, violations);
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
