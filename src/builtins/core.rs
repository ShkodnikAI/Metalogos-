// ── Core helpers / utility functions used across builtins modules ──────

use crate::interpreter::Value;

// ── Argument extraction helpers ──────────────────────────────────────

pub(crate) fn expect_float_arg(fn_name: &str, args: &[Value], index: usize) -> Result<f64, String> {
    if args.len() <= index {
        return Err(format!(
            "{}() requires an argument at position {}",
            fn_name, index
        ));
    }
    match &args[index] {
        Value::Float(f) => Ok(*f),
        other => Err(format!(
            "{}() expected Float argument, got {}",
            fn_name,
            other.type_name()
        )),
    }
}

pub(crate) fn expect_string_arg(
    fn_name: &str,
    args: &[Value],
    index: usize,
) -> Result<String, String> {
    if args.len() <= index {
        return Err(format!(
            "{}() requires an argument at position {}",
            fn_name, index
        ));
    }
    match &args[index] {
        Value::String(s) => Ok(s.clone()),
        other => Err(format!(
            "{}() expected String argument, got {}",
            fn_name,
            other.type_name()
        )),
    }
}

pub(crate) fn expect_string_arg_var(
    name: &str,
    args: &[Value],
    idx: usize,
) -> Result<String, String> {
    if idx >= args.len() {
        return Err(format!("{}: expected argument at position {}", name, idx));
    }
    match &args[idx] {
        Value::String(s) => Ok(s.clone()),
        other => Err(format!(
            "{}: argument {} must be String, got {}",
            name,
            idx,
            other.type_name()
        )),
    }
}

pub(crate) fn expect_list_arg(
    name: &str,
    args: &[Value],
    idx: usize,
) -> Result<Vec<Value>, String> {
    if idx >= args.len() {
        return Err(format!("{}: expected argument at position {}", name, idx));
    }
    match &args[idx] {
        Value::List(items) => Ok(items.clone()),
        other => Err(format!(
            "{}: argument {} must be List, got {}",
            name,
            idx,
            other.type_name()
        )),
    }
}

pub(crate) fn expect_struct_json_arg(
    name: &str,
    args: &[Value],
    idx: usize,
) -> Result<String, String> {
    if idx >= args.len() {
        return Err(format!("{}: expected argument at position {}", name, idx));
    }
    let json = super::mlog_value_to_json(&args[idx]);
    match serde_json::to_string(&json) {
        Ok(s) => Ok(s),
        Err(_) => Err(format!(
            "{}: argument {} must be serializable to JSON",
            name, idx
        )),
    }
}

// ── Struct construction helper ───────────────────────────────────────

/// Helper: build a Value::Struct from a type name and a list of (key, value) pairs.
pub(crate) fn make_struct(type_name: &str, fields: Vec<(&str, Value)>) -> Value {
    let mut map = std::collections::HashMap::new();
    for (k, v) in fields {
        map.insert(k.to_string(), v);
    }
    Value::Struct {
        type_name: type_name.to_string(),
        fields: map,
    }
}

// ── Assertion builtins ───────────────────────────────────────────────

/// `assert_eq(actual, expected)` — error if two values display differently.
/// Returns the actual value on success.
pub(crate) fn builtin_assert_eq(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("assert_eq: requires 2 arguments (actual, expected)".to_string());
    }
    let actual_str = format!("{}", args[0]);
    let expected_str = format!("{}", args[1]);
    if actual_str != expected_str {
        Err(format!(
            "assert_eq failed: {} != {}",
            actual_str, expected_str
        ))
    } else {
        Ok(args[0].clone())
    }
}

/// `assert_contains(haystack, needle)` — panic if needle not found in haystack string.
pub(crate) fn builtin_assert_contains(args: &[Value]) -> Result<Value, String> {
    let haystack = format!("{}", args.first().unwrap_or(&Value::Unit));
    let needle = format!("{}", args.get(1).unwrap_or(&Value::Unit));
    if !haystack.contains(&needle) {
        Err(format!(
            "assert_contains failed: '{}' not in '{}'",
            needle,
            &haystack[..haystack.len().min(80)]
        ))
    } else {
        Ok(args[0].clone())
    }
}

// ── Type introspection builtin ───────────────────────────────────────

/// `type_of(value) -> String` — returns the runtime type name as a String.
/// Useful for safe checking after json_get: `if type_of(x) == "Unit" { ... }`
pub(crate) fn builtin_type_of(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("type_of() requires 1 argument".to_string());
    }
    Ok(Value::String(args[0].type_name().to_string()))
}
