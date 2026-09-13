// ── The JSON-Schema subset validator (ADR-0133), shared by both paths ────
// Extracted from src/builtins/llm_schema.rs (Наряд №269) in Наряд №286 so
// `call_llm_schema` (LLM answers) and `json_validate` (any untrusted JSON:
// MCP tool-outputs, HTTP responses, request_body) judge data with the SAME
// code. Differential contract: NOT A SINGLE new rule — the corpus in
// tests/naryad_286_json_validate.rs must produce byte-identical verdicts
// and violation texts through both paths.
//
// Caller-facing names keep their historical wording: passing
// `caller = "call_llm_schema"` reproduces the pre-extraction messages
// byte-for-byte (pinned by tests/naryad_269_schema.rs).

/// Keywords the validator actually enforces (ADR-0133 D1).
pub const SUPPORTED_KEYWORDS: &[&str] = &["type", "properties", "required", "items", "enum"];
/// Pure-annotation keywords: inert metadata, cannot weaken validation — ignored
/// silently so real-world schemas (which carry `title`/`description`/`$schema`)
/// do not bounce (ADR-0133 D1). This list is CLOSED; anything outside it errors.
pub const IGNORED_ANNOTATIONS: &[&str] = &[
    "$schema",
    "$id",
    "$comment",
    "title",
    "description",
    "default",
    "examples",
];

// ── Schema-side checks (NOT retryable) ────────────────────────────────────

/// Validate that `schema` lives inside the supported subset (ADR-0133 D1).
/// The ROOT schema must describe an object — the ADR-0133 root contract
/// («The root schema must be {"type":"object",...}»), shared by both paths:
/// `call_llm_schema` because the builtin returns a Struct, `json_validate`
/// because the shape-before-use contract validates a top-level object.
/// Nested schemas may be object/array/string/number/integer/boolean/null.
/// `caller` names the builtin in loud messages (byte-identical diagnostics
/// for the historical "call_llm_schema").
pub fn check_schema_subset(schema: &serde_json::Value, caller: &str) -> Result<(), String> {
    check_schema_node(schema, true, "$", caller)
}

fn check_schema_node(
    schema: &serde_json::Value,
    is_root: bool,
    path: &str,
    caller: &str,
) -> Result<(), String> {
    let map = match schema.as_object() {
        Some(m) => m,
        None => {
            return Err(format!(
                "{}(): [LLM_SCHEMA_UNSUPPORTED_FEATURE] schema node at {} must be a JSON object, got {} (ADR-0133)",
                caller,
                path,
                json_type_name(schema)
            ))
        }
    };
    for key in map.keys() {
        if SUPPORTED_KEYWORDS.contains(&key.as_str()) || IGNORED_ANNOTATIONS.contains(&key.as_str())
        {
            continue;
        }
        return Err(format!(
            "{}(): [LLM_SCHEMA_UNSUPPORTED_FEATURE] schema keyword '{}' at {} is outside the supported subset {:?} (annotations {:?} are ignored); see ADR-0133",
            caller, key, path, SUPPORTED_KEYWORDS, IGNORED_ANNOTATIONS
        ));
    }
    if is_root {
        let root_type = map.get("type").and_then(|t| t.as_str());
        if root_type != Some("object") {
            return Err(format!(
                "{}(): [LLM_SCHEMA_UNSUPPORTED_FEATURE] root schema must be {{\"type\":\"object\",...}} — the ROOT schema describes an object (ADR-0133 root contract); got type {:?} at {}",
                caller, root_type, path
            ));
        }
    }
    if let Some(t) = map.get("type") {
        if !t.is_string() {
            return Err(format!(
                "{}(): [LLM_SCHEMA_UNSUPPORTED_FEATURE] 'type' at {} must be a string, got {} (ADR-0133)",
                caller,
                path,
                json_type_name(t)
            ));
        }
    }
    if let Some(props) = map.get("properties") {
        let props = match props.as_object() {
            Some(p) => p,
            None => {
                return Err(format!(
                    "{}(): [LLM_SCHEMA_UNSUPPORTED_FEATURE] 'properties' at {} must be an object, got {} (ADR-0133)",
                    caller,
                    path,
                    json_type_name(props)
                ))
            }
        };
        for (name, sub) in props {
            check_schema_node(sub, false, &format!("{}.properties.{}", path, name), caller)?;
        }
    }
    if let Some(req) = map.get("required") {
        let all_strings = req.is_array()
            && req
                .as_array()
                .is_some_and(|a| a.iter().all(|v| v.is_string()));
        if !all_strings {
            return Err(format!(
                "{}(): [LLM_SCHEMA_UNSUPPORTED_FEATURE] 'required' at {} must be an array of strings (ADR-0133)",
                caller, path
            ));
        }
    }
    if let Some(items) = map.get("items") {
        check_schema_node(items, false, &format!("{}.items", path), caller)?;
    }
    if let Some(en) = map.get("enum") {
        let non_empty_array = en.is_array() && en.as_array().is_some_and(|a| !a.is_empty());
        if !non_empty_array {
            return Err(format!(
                "{}(): [LLM_SCHEMA_UNSUPPORTED_FEATURE] 'enum' at {} must be a non-empty array (ADR-0133)",
                caller, path
            ));
        }
    }
    Ok(())
}

// ── Instance-side validation ──────────────────────────────────────────────

/// Validate `value` against `schema` (already subset-checked), collecting ALL
/// violations instead of failing fast — the full report is the retry context
/// (`call_llm_schema`) or the `errors` list (`json_validate`) (ADR-0133 D3).
/// Pure function, no I/O.
///
/// `root_label` names the document root in violation paths: "answer" for the
/// LLM path (historical wording), "value" for `json_validate`. `strict`
/// toggles the ADR-0133 D2 deviation — strict-by-default (fields NOT declared
/// in `properties` are violations): `true` for `call_llm_schema` and as the
/// default of `json_validate`; `strict = false` (a `json_validate`-only
/// opt-in, №286) permits undeclared fields — every other rule (type /
/// required / items / enum, the subset itself) is UNCHANGED.
pub fn validate_json(
    value: &serde_json::Value,
    schema: &serde_json::Value,
    path: &str,
    root_label: &str,
    strict: bool,
    violations: &mut Vec<String>,
) {
    // enum applies to any type and takes precedence in reporting
    if let Some(en) = schema.get("enum").and_then(|e| e.as_array()) {
        if !en.iter().any(|allowed| allowed == value) {
            violations.push(format!(
                "{}: value {} is not one of the enum literals {:?}",
                display_path(path, root_label),
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
                display_path(path, root_label),
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
                if strict {
                    for key in map.keys() {
                        if !props.contains_key(key) {
                            violations.push(format!(
                                "{}: unexpected field '{}' is not declared in schema properties (strict-by-default, ADR-0133 D2)",
                                display_path(path, root_label),
                                key
                            ));
                        }
                    }
                }
                for (name, sub) in props {
                    if let Some(v) = map.get(name) {
                        validate_json(
                            v,
                            sub,
                            &format!("{}.{}", path, name),
                            root_label,
                            strict,
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
                            display_path(path, root_label),
                            name
                        ));
                    }
                }
            }
        }
        serde_json::Value::Array(items) => {
            if let Some(items_schema) = schema.get("items") {
                for (i, item) in items.iter().enumerate() {
                    validate_json(
                        item,
                        items_schema,
                        &format!("{}[{}]", path, i),
                        root_label,
                        strict,
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

// ── Formatting helpers (shared so both paths report identically) ──────────

pub(crate) fn json_type_name(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn display_path(path: &str, root_label: &str) -> String {
    if path == "$" {
        root_label.to_string()
    } else {
        format!(
            "{}.{}",
            root_label,
            path.trim_start_matches("$.").trim_start_matches('$')
        )
    }
}

fn short_repr(v: &serde_json::Value) -> String {
    let s = v.to_string();
    if s.len() > 60 {
        // Truncate on a char boundary: the pre-extraction byte slice
        // `&s[..60]` could PANIC on multi-byte input (a long Cyrillic value
        // in a type/enum violation). Formatting for ASCII is unchanged;
        // verdicts are unchanged (№286 extraction, honest robustness fix).
        let mut end = 60;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &s[..end])
    } else {
        s
    }
}
