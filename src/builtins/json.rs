// ── JSON / Dict builtins & helpers ────────────────────────────

use super::core::expect_string_arg;
use crate::interpreter::Value;

/// Convert Metalogos Value to serde_json::Value (inverse of server.rs json_value_to_value).
pub(crate) fn value_to_json(val: &Value) -> Result<serde_json::Value, String> {
    match val {
        Value::String(s) => Ok(serde_json::Value::String(s.clone())),
        Value::Float(f) => {
            if *f == (*f as i64) as f64 {
                Ok(serde_json::json!(*f as i64))
            } else {
                Ok(serde_json::json!(*f))
            }
        }
        Value::Bool(b) => Ok(serde_json::Value::Bool(*b)),
        Value::List(items) => {
            let arr: Vec<serde_json::Value> =
                items.iter().map(value_to_json).collect::<Result<_, _>>()?;
            Ok(serde_json::Value::Array(arr))
        }
        Value::Struct { fields, .. } => {
            let mut map = serde_json::Map::new();
            for (k, v) in fields {
                map.insert(k.clone(), value_to_json(v)?);
            }
            Ok(serde_json::Value::Object(map))
        }
        Value::Unit => Ok(serde_json::Value::Null),
        _ => Ok(serde_json::Value::Null),
    }
}

// ── Phase 7.7 — parse_json, http_get, now ────────────────────────────

/// Parse a JSON string into a Value (Struct or List).
/// Usage: parse_json(text) -> Struct|List|String|Float|Bool|Unit
pub(crate) fn builtin_parse_json(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("parse_json", args, 0)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("parse_json() error: {}", e))?;
    Ok(json_value_to_mlog_value(&parsed))
}

/// Convert serde_json::Value to METALOGOS Value (same logic as interpreter's method).
pub(crate) fn json_value_to_mlog_value(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Number(n) => Value::Float(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Null => Value::Unit,
        serde_json::Value::Array(arr) => {
            Value::List(arr.iter().map(|v| json_value_to_mlog_value(v)).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut fields = std::collections::HashMap::new();
            for (k, v) in obj {
                fields.insert(k.clone(), json_value_to_mlog_value(v));
            }
            Value::Struct {
                type_name: "Json".to_string(),
                fields,
            }
        }
    }
}

/// Convert METALOGOS Value to serde_json::Value (reverse of json_value_to_mlog_value).
pub(crate) fn mlog_value_to_json(val: &Value) -> serde_json::Value {
    match val {
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Float(f) => serde_json::json!(*f),
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Unit => serde_json::Value::Null,
        Value::List(items) => {
            serde_json::Value::Array(items.iter().map(|v| mlog_value_to_json(v)).collect())
        }
        Value::Struct { fields, .. } => {
            let mut map = serde_json::Map::new();
            for (k, v) in fields {
                map.insert(k.clone(), mlog_value_to_json(v));
            }
            serde_json::Value::Object(map)
        }
        // Opaque/internal types: convert to string representation
        other => serde_json::Value::String(format!("{}", other)),
    }
}

/// Serialize a Value to a JSON string.
/// Usage: json_encode(value) -> String
/// Supports: String, Float, Bool, Unit->null, List->array, Struct->object
pub(crate) fn builtin_json_encode(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("json_encode() requires 1 argument".to_string());
    }
    let json = mlog_value_to_json(&args[0]);
    let serialized = serde_json::to_string(&json)
        .map_err(|e| format!("json_encode() serialization error: {}", e))?;
    Ok(Value::String(serialized))
}

/// Safe field access on a struct value: returns default if field missing or not a struct.
/// Usage: json_get(obj, "field") -> Value (returns Unit if missing)
/// Usage: json_get(obj, "field", default_value) -> Value (returns default if missing)
/// Usage: json_get(obj, "nested.field.path", default) -> Value (dot-separated path)
/// Supports numeric path segments for array indexing: "items.0.title"
/// This is the P0 fix: prevents runtime crash when accessing optional JSON fields
/// like message.voice on non-voice Telegram updates.
pub(crate) fn builtin_json_get(args: &[Value]) -> Result<Value, String> {
    let obj = match args.get(0) {
        Some(v) => v,
        None => {
            return Err("json_get() requires at least 2 arguments (obj, field_path)".to_string())
        }
    };
    let path = expect_string_arg("json_get", args, 1)?;
    // Bug 2.2 fix: when no default is provided, return the found value directly
    // (not wrapped in Unit). The old code defaulted to Value::Unit which silently
    // swallowed string values and made them unusable.
    if args.len() >= 3 {
        let default_val = args[2].clone();
        // Navigate the path (dot-separated)
        let mut current = obj;
        for segment in path.split('.') {
            // Try struct field access first
            match current.get_field(segment) {
                Ok(val) => {
                    current = val;
                    continue;
                }
                Err(_) => {}
            }
            // If struct field not found, try numeric array index (Наряд №24 B4)
            if let Ok(index) = segment.parse::<usize>() {
                if let Value::List(items) = current {
                    if let Some(item) = items.get(index) {
                        current = item;
                        continue;
                    }
                }
            }
            return Ok(default_val);
        }
        // SQL NULL / missing value maps to Value::Unit; treat as "use default"
        if matches!(current, Value::Unit) {
            return Ok(default_val);
        }
        Ok(current.clone())
    } else {
        // 2-argument form: return the found value or Unit if not found
        let mut current = obj;
        for segment in path.split('.') {
            // Try struct field access first
            match current.get_field(segment) {
                Ok(val) => {
                    current = val;
                    continue;
                }
                Err(_) => {}
            }
            // If struct field not found, try numeric array index (Наряд №24 B4)
            if let Ok(index) = segment.parse::<usize>() {
                if let Value::List(items) = current {
                    if let Some(item) = items.get(index) {
                        current = item;
                        continue;
                    }
                }
            }
            return Ok(Value::Unit);
        }
        Ok(current.clone())
    }
}

/// Check if a struct value has a given field. Returns 1.0 (true) or 0.0 (false).
/// Usage: has_field(obj, "field") -> Float
/// Usage: has_field(obj, "nested.field") -> Float (dot-separated path)
pub(crate) fn builtin_has_field(args: &[Value]) -> Result<Value, String> {
    let obj = match args.get(0) {
        Some(v) => v,
        None => return Err("has_field() requires 2 arguments (obj, field_path)".to_string()),
    };
    let path = expect_string_arg("has_field", args, 1)?;

    let mut current = obj;
    let segments: Vec<&str> = path.split('.').collect();
    for (i, segment) in segments.iter().enumerate() {
        match current.get_field(segment) {
            Ok(val) => {
                if i == segments.len() - 1 {
                    return Ok(Value::Float(1.0));
                }
                current = val;
            }
            Err(_) => return Ok(Value::Float(0.0)),
        }
    }
    Ok(Value::Float(0.0))
}

// ── Dict operations (Наряд №17 В.1) ─────────────────────────────
// Dicts are represented as Value::Struct with type_name "Dict".

/// `dict_set(dict, key, value)` — set a key in a dict. Returns the modified dict.
pub(crate) fn builtin_dict_set(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("dict_set() requires 3 arguments (dict, key, value)".to_string());
    }
    let key = expect_string_arg("dict_set", args, 1)?;
    let mut fields = match &args[0] {
        Value::Struct { fields, .. } => fields.clone(),
        other => {
            return Err(format!(
                "dict_set() expected Struct as first arg, got {}",
                other.type_name()
            ))
        }
    };
    fields.insert(key, args[2].clone());
    Ok(Value::Struct {
        type_name: "Dict".to_string(),
        fields,
    })
}

/// `dict_keys(dict) -> List` — return list of keys.
pub(crate) fn builtin_dict_keys(args: &[Value]) -> Result<Value, String> {
    let fields = match &args.get(0) {
        Some(Value::Struct { fields, .. }) => fields,
        _ => return Err("dict_keys() requires 1 argument (Struct/Dict)".to_string()),
    };
    let keys: Vec<Value> = fields.keys().map(|k| Value::String(k.clone())).collect();
    Ok(Value::List(keys))
}

/// `dict_values(dict) -> List` — return list of values.
pub(crate) fn builtin_dict_values(args: &[Value]) -> Result<Value, String> {
    let fields = match &args.get(0) {
        Some(Value::Struct { fields, .. }) => fields,
        _ => return Err("dict_values() requires 1 argument (Struct/Dict)".to_string()),
    };
    let values: Vec<Value> = fields.values().cloned().collect();
    Ok(Value::List(values))
}

/// `dict_has(dict, key) -> Bool` — check if key exists.
pub(crate) fn builtin_dict_has(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("dict_has() requires 2 arguments (dict, key)".to_string());
    }
    let key = expect_string_arg("dict_has", args, 1)?;
    let has = match &args[0] {
        Value::Struct { fields, .. } => fields.contains_key(&key),
        _ => {
            return Err(format!(
                "dict_has() expected Struct as first arg, got {}",
                args[0].type_name()
            ))
        }
    };
    Ok(Value::Bool(has))
}

/// Convert serde_yaml::Value to serde_json::Value for unified config processing.
pub(crate) fn yaml_to_json_value(yaml: &serde_yaml::Value) -> serde_json::Value {
    match yaml {
        serde_yaml::Value::Null => serde_json::Value::Null,
        serde_yaml::Value::Bool(b) => serde_json::Value::Bool(*b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_json::Value::Number(i.into())
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            }
        }
        serde_yaml::Value::String(s) => serde_json::Value::String(s.clone()),
        serde_yaml::Value::Sequence(seq) => {
            serde_json::Value::Array(seq.iter().map(yaml_to_json_value).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                if let Some(key_str) = k.as_str() {
                    obj.insert(key_str.to_string(), yaml_to_json_value(v));
                }
            }
            serde_json::Value::Object(obj)
        }
        serde_yaml::Value::Tagged(tagged) => {
            // YAML tags — just convert the value, ignoring the tag
            yaml_to_json_value(&tagged.value)
        }
    }
}

/// Like json_value_to_mlog_value but with a custom type_name for the root struct.
pub(crate) fn json_value_to_mlog_value_with_type(
    json: &serde_json::Value,
    type_name: &str,
) -> Value {
    match json {
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Number(n) => Value::Float(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Null => Value::Unit,
        serde_json::Value::Array(arr) => Value::List(
            arr.iter()
                .map(|v| json_value_to_mlog_value_with_type(v, type_name))
                .collect(),
        ),
        serde_json::Value::Object(obj) => {
            let mut fields = std::collections::HashMap::new();
            for (k, v) in obj {
                fields.insert(k.clone(), json_value_to_mlog_value_with_type(v, type_name));
            }
            Value::Struct {
                type_name: type_name.to_string(),
                fields,
            }
        }
    }
}
