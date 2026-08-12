// ── Collection builtins: list get, push, slice, zip, sort, filter, reduce, etc. ──

use crate::interpreter::Value;

use super::core::{expect_float_arg, expect_list_arg, make_struct};

pub(crate) fn builtin_get(args: &[Value]) -> Result<Value, String> {
    let list = match args.first() {
        Some(Value::List(items)) => items,
        _ => return Err("get() requires List as first argument".to_string()),
    };
    let index = match args.get(1) {
        Some(Value::Float(f)) => *f as usize,
        _ => return Err("get() requires Float index as second argument".to_string()),
    };
    list.get(index).cloned().ok_or_else(|| {
        format!(
            "get() index {} out of bounds (list has {} elements)",
            index,
            list.len()
        )
    })
}

pub(crate) fn builtin_push(args: &[Value]) -> Result<Value, String> {
    let list = match args.first() {
        Some(Value::List(items)) => items.clone(),
        _ => return Err("push() requires List as first argument".to_string()),
    };
    let item = match args.get(1) {
        Some(v) => v.clone(),
        None => return Err("push() requires a second argument (item to push)".to_string()),
    };
    let mut new_list = list;
    new_list.push(item);
    Ok(Value::List(new_list))
}

pub(crate) fn builtin_slice(args: &[Value]) -> Result<Value, String> {
    let list = match args.first() {
        Some(Value::List(items)) => items,
        _ => return Err("slice() requires List as first argument".to_string()),
    };
    let start = match args.get(1) {
        Some(Value::Float(f)) => *f as usize,
        _ => return Err("slice() requires Float start as second argument".to_string()),
    };
    let end = match args.get(2) {
        Some(Value::Float(f)) => *f as usize,
        _ => return Err("slice() requires Float end as third argument".to_string()),
    };
    // Soft-failure semantics, mirroring substring:
    //   start >= len  -> empty list
    //   end > len     -> clamp to len
    //   start >= end  -> empty list
    let list_len = list.len();
    if start >= list_len {
        return Ok(Value::List(vec![]));
    }
    let end = if end > list_len { list_len } else { end };
    if start >= end {
        return Ok(Value::List(vec![]));
    }
    Ok(Value::List(list[start..end].to_vec()))
}

/// `make_list(a, b, c, ...) -> List` — create a list from variadic arguments.
/// Eliminates race conditions from write_file/read_file workarounds for
/// returning multiple values from patterns.
/// Usage: make_list("red", "green", "blue") -> List ["red", "green", "blue"]
pub(crate) fn builtin_make_list(args: &[Value]) -> Result<Value, String> {
    Ok(Value::List(args.to_vec()))
}

/// `compact_list(items, keep_first, keep_last)` — context condensation for lists.
/// Protects the first `keep_first` and last `keep_last` items, replaces middle items
/// with a single placeholder struct { compacted: true, removed_count: N }.
/// Inspired by OpenPlanter's compact_messages() for managing long conversation histories.
pub(crate) fn builtin_compact_list(args: &[Value]) -> Result<Value, String> {
    let items = expect_list_arg("compact_list", args, 0)?;
    let keep_first = expect_float_arg("compact_list", args, 1)? as usize;
    let keep_last = expect_float_arg("compact_list", args, 2)? as usize;
    let total = items.len();
    if total <= keep_first + keep_last {
        // Nothing to compact
        return Ok(Value::List(items));
    }
    let mut result: Vec<Value> = Vec::new();
    // Keep first N
    for item in items.iter().take(keep_first) {
        result.push(item.clone());
    }
    // Insert compacted placeholder
    let removed = total - keep_first - keep_last;
    result.push(make_struct(
        "Compacted",
        vec![
            ("compacted", Value::Bool(true)),
            ("removed_count", Value::Float(removed as f64)),
        ],
    ));
    // Keep last N
    let last_start = total - keep_last;
    for item in items.iter().skip(last_start) {
        result.push(item.clone());
    }
    Ok(Value::List(result))
}

/// `zip(list_a, list_b)` — pairwise merge two lists into list of 2-element structs [{a, b}, ...]
pub(crate) fn builtin_zip(args: &[Value]) -> Result<Value, String> {
    let list_a = match args.first() {
        Some(Value::List(items)) => items,
        _ => return Err("zip() expects first argument to be a List".to_string()),
    };
    let list_b = match args.get(1) {
        Some(Value::List(items)) => items,
        _ => return Err("zip() expects second argument to be a List".to_string()),
    };
    let paired: Vec<Value> = list_a
        .iter()
        .zip(list_b.iter())
        .map(|(a, b)| Value::Struct {
            type_name: "Pair".to_string(),
            fields: [("a".to_string(), a.clone()), ("b".to_string(), b.clone())]
                .into_iter()
                .collect(),
        })
        .collect();
    Ok(Value::List(paired))
}

/// `sort_by(list, key_field, descending?)` — sort list of structs by a field name.
/// descending: 1.0 = descending, 0.0 or absent = ascending.
pub(crate) fn builtin_sort_by(args: &[Value]) -> Result<Value, String> {
    let list = match args.first() {
        Some(Value::List(items)) => items.clone(),
        _ => return Err("sort_by() expects first argument to be a List".to_string()),
    };
    let key_field = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err("sort_by() expects second argument to be a field name (String)".to_string())
        }
    };
    let descending = matches!(args.get(2), Some(Value::Float(f)) if *f != 0.0);

    let mut sorted = list;
    sorted.sort_by(|a, b| {
        let va = a
            .get_field(&key_field)
            .ok()
            .cloned()
            .unwrap_or(Value::Float(0.0));
        let vb = b
            .get_field(&key_field)
            .ok()
            .cloned()
            .unwrap_or(Value::Float(0.0));
        let fa = match va {
            Value::Float(f) => f,
            Value::String(s) => s.parse::<f64>().unwrap_or(0.0),
            _ => 0.0,
        };
        let fb = match vb {
            Value::Float(f) => f,
            Value::String(s) => s.parse::<f64>().unwrap_or(0.0),
            _ => 0.0,
        };
        if descending {
            fb.partial_cmp(&fa).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
        }
    });
    Ok(Value::List(sorted))
}

/// `filter(list, key_field, value)` — filter list of structs where field == value.
pub(crate) fn builtin_filter(args: &[Value]) -> Result<Value, String> {
    let list = match args.first() {
        Some(Value::List(items)) => items.clone(),
        _ => return Err("filter() expects first argument to be a List".to_string()),
    };
    let key_field = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err("filter() expects second argument to be a field name (String)".to_string())
        }
    };
    let filter_val = match args.get(2) {
        Some(v) => v.clone(),
        None => return Err("filter() expects three arguments".to_string()),
    };

    let filtered: Vec<Value> = list
        .into_iter()
        .filter(|item| {
            let field_val = item
                .get_field(&key_field)
                .ok()
                .cloned()
                .unwrap_or(Value::Unit);
            match (&field_val, &filter_val) {
                (Value::String(a), Value::String(b)) => a == b,
                (Value::Float(a), Value::Float(b)) => a == b,
                (Value::Bool(a), Value::Bool(b)) => a == b,
                _ => false,
            }
        })
        .collect();
    Ok(Value::List(filtered))
}

/// `reduce(list, key_field, initial)` — sum all float values of a field across list of structs.
pub(crate) fn builtin_reduce(args: &[Value]) -> Result<Value, String> {
    let list = match args.first() {
        Some(Value::List(items)) => items,
        _ => return Err("reduce() expects first argument to be a List".to_string()),
    };
    let key_field = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err("reduce() expects second argument to be a field name (String)".to_string())
        }
    };
    let initial = match args.get(2) {
        Some(Value::Float(f)) => *f,
        _ => {
            return Err("reduce() expects third argument to be an initial Float value".to_string())
        }
    };

    let mut acc = initial;
    for item in list {
        let field_val = item
            .get_field(&key_field)
            .ok()
            .cloned()
            .unwrap_or(Value::Float(0.0));
        if let Value::Float(f) = field_val {
            acc += f;
        }
    }
    Ok(Value::Float(acc))
}

/// `matches_any(text, triggers_list)` — case-insensitive substring match.
/// Returns 1.0 if ANY trigger string is found in text, 0.0 otherwise.
/// Used by skill_index tier matching (Problem A).
pub(crate) fn builtin_matches_any(args: &[Value]) -> Result<Value, String> {
    let text = match args.first() {
        Some(Value::String(s)) => s.to_lowercase(),
        _ => return Err("matches_any() expects first argument to be a String".to_string()),
    };
    let triggers = match args.get(1) {
        Some(Value::List(items)) => items,
        _ => {
            return Err(
                "matches_any() expects second argument to be a List of trigger strings".to_string(),
            )
        }
    };
    for trigger in triggers {
        if let Value::String(t) = trigger {
            if text.contains(&t.to_lowercase()) {
                return Ok(Value::Float(1.0));
            }
        }
    }
    Ok(Value::Float(0.0))
}

/// `dedup(list)` — remove duplicate elements, preserving first-occurrence order.
pub(crate) fn builtin_dedup(args: &[Value]) -> Result<Value, String> {
    let list = match args.first() {
        Some(Value::List(items)) => items.clone(),
        Some(other) => {
            return Err(format!(
                "dedup() expected List argument, got {}",
                other.type_name()
            ))
        }
        None => return Err("dedup() requires 1 argument".to_string()),
    };
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut result = Vec::new();
    for item in &list {
        let key = match item {
            Value::String(s) => s.clone(),
            Value::Float(f) => format!("{}", f),
            Value::Bool(b) => format!("{}", b),
            other => {
                // Use JSON representation for complex types
                let json = super::mlog_value_to_json(other);
                serde_json::to_string(&json).unwrap_or_else(|_| format!("{}", other))
            }
        };
        if seen.insert(key) {
            result.push(item.clone());
        }
    }
    Ok(Value::List(result))
}

/// `condense(list)` — collapse consecutive identical string elements with count.
/// Example: ["a","a","b"] -> ["a","×2","b"]
pub(crate) fn builtin_condense(args: &[Value]) -> Result<Value, String> {
    let list = match args.first() {
        Some(Value::List(items)) => items.clone(),
        Some(other) => {
            return Err(format!(
                "condense() expected List argument, got {}",
                other.type_name()
            ))
        }
        None => return Err("condense() requires 1 argument".to_string()),
    };
    let mut result: Vec<Value> = Vec::new();
    let mut i = 0;
    while i < list.len() {
        let current = match &list[i] {
            Value::String(s) => s.clone(),
            other => {
                return Err(format!(
                    "condense() all elements must be String, got {} at index {}",
                    other.type_name(),
                    i
                ))
            }
        };
        let mut count: usize = 1;
        while i + count < list.len() {
            if let Value::String(ref next) = list[i + count] {
                if next == &current {
                    count += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        result.push(Value::String(current.clone()));
        if count > 1 {
            result.push(Value::String(format!("\u{00d7}{}", count)));
        }
        i += count;
    }
    Ok(Value::List(result))
}
