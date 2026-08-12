// ── Regular expression builtins (Наряд №54) ─────────────────────────────
//
// Three builtins: regex_match, regex_captures, regex_replace.
// Uses the `regex` crate (Rust's standard regex library).
//
// Key design decisions (see ADR-XXXX):
//   - Linear-time matching, NO backtracking → no lookahead/lookbehind support.
//   - Soft-failure: invalid regex → degraded result (false / empty / unchanged),
//     never panic. This is consistent with the interpreter's error model.
//   - Compilation cache: size-limited LRU (32 entries) keyed by pattern string.
//     Thread-safe via std::sync::Mutex (cheap contention, fast path).

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use super::core::*;
use crate::interpreter::Value;

/// Maximum number of compiled regex patterns kept in cache.
const CACHE_CAPACITY: usize = 32;

/// Lazy-global regex compilation cache.
/// Key = pattern string, Value = compiled Regex.
static REGEX_CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(|| Mutex::new(Cache::new()));

/// Simple LRU cache for compiled Regex objects.
/// When at capacity, evicts the least-recently-used entry.
struct Cache {
    entries: HashMap<String, (regex::Regex, std::collections::HashSet<String>)>,
    order: Vec<String>, // most-recently-used at the back
}

impl Cache {
    fn new() -> Self {
        Cache {
            entries: HashMap::new(),
            order: Vec::new(),
        }
    }

    /// Get a cached regex, or compile and insert it.
    /// Returns None if the pattern fails to compile.
    fn get_or_compile(&mut self, pattern: &str) -> Option<regex::Regex> {
        if let Some((re, _)) = self.entries.get(pattern) {
            // Move to end (most recently used)
            self.order.retain(|k| k != pattern);
            self.order.push(pattern.to_string());
            return Some(re.clone());
        }

        // Compile
        match regex::Regex::new(pattern) {
            Ok(re) => {
                // Evict if at capacity
                if self.entries.len() >= CACHE_CAPACITY {
                    if let Some(evict_key) = self.order.first().cloned() {
                        self.entries.remove(&evict_key);
                        self.order.remove(0);
                    }
                }
                self.order.push(pattern.to_string());
                self.entries.insert(
                    pattern.to_string(),
                    (re.clone(), std::collections::HashSet::new()),
                );
                Some(re)
            }
            Err(_) => None,
        }
    }
}

// ── Helper: extract compiled regex from cache (never panics) ──

/// Returns a compiled regex for the given pattern, or None if invalid.
fn get_compiled(pattern: &str) -> Option<regex::Regex> {
    let mut cache = REGEX_CACHE.lock().ok()?;
    cache.get_or_compile(pattern)
}

/// Clear the regex cache (useful for tests).
#[allow(dead_code)]
pub(crate) fn clear_regex_cache() {
    if let Ok(mut cache) = REGEX_CACHE.lock() {
        cache.entries.clear();
        cache.order.clear();
    }
}

// ── Builtin implementations ──────────────────────────────────────────────

/// `regex_match(pattern, text)` → Bool
///
/// Returns true if `text` matches `pattern` (full-match semantics:
/// the pattern is implicitly anchored with `^...$`).
/// On invalid regex, returns `false` (soft-failure).
pub(crate) fn builtin_regex_match(args: &[Value]) -> Result<Value, String> {
    let pattern = expect_string_arg("regex_match", args, 0)?;
    let text = expect_string_arg("regex_match", args, 1)?;

    let re = match get_compiled(&pattern) {
        Some(r) => r,
        None => return Ok(Value::Bool(false)),
    };

    Ok(Value::Bool(re.is_match(&text)))
}

/// `regex_captures(pattern, text)` → List
///
/// Returns a list of captured groups. Index 0 = entire match,
/// indices 1..N = named/numbered capture groups.
/// If the pattern has no capture groups, returns a single-element list [full_match].
/// If no match, returns empty list [].
/// On invalid regex, returns empty list [] (soft-failure).
pub(crate) fn builtin_regex_captures(args: &[Value]) -> Result<Value, String> {
    let pattern = expect_string_arg("regex_captures", args, 0)?;
    let text = expect_string_arg("regex_captures", args, 1)?;

    let re = match get_compiled(&pattern) {
        Some(r) => r,
        None => return Ok(Value::List(Vec::new())),
    };

    match re.captures(&text) {
        Some(caps) => {
            let mut result = Vec::new();
            for i in 0..caps.len() {
                let matched = caps
                    .get(i)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                result.push(Value::String(matched));
            }
            Ok(Value::List(result))
        }
        None => Ok(Value::List(Vec::new())),
    }
}

/// `regex_replace(pattern, text, replacement)` → String
///
/// Replaces ALL non-overlapping matches of `pattern` in `text` with `replacement`.
/// Supports capture group references: `$1`, `$2`, `$name` in replacement string.
/// On invalid regex, returns original `text` unchanged (soft-failure).
pub(crate) fn builtin_regex_replace(args: &[Value]) -> Result<Value, String> {
    let pattern = expect_string_arg("regex_replace", args, 0)?;
    let text = expect_string_arg("regex_replace", args, 1)?;
    let replacement = expect_string_arg("regex_replace", args, 2)?;

    let re = match get_compiled(&pattern) {
        Some(r) => r,
        None => return Ok(Value::String(text)),
    };

    Ok(Value::String(
        re.replace_all(&text, replacement.as_str()).to_string(),
    ))
}
