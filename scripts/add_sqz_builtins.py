#!/usr/bin/env python3
"""Add sqz-inspired builtins to Metalogos builtins.rs.

Adds 15 new builtin functions + tests to src/builtins.rs:
  P1: squeeze, dedup, condense, strip, chomp, repeat, pad_left, pad_right, lines, words
  P2: toon_encode, toon_decode, ref, deref
  P3: token_count
"""

import re

BUILTINS_PATH = "/home/z/my-project/metalogos-build/src/builtins.rs"

with open(BUILTINS_PATH, "r") as f:
    content = f.read()

# ─── 1. BUILTIN_REGISTRY entries (before "];") ─────────────────────

registry_entries = """\
    // ── sqz-inspired: String/List utilities (P1) ──
    BuiltinSpec { name: "squeeze", arity: 2, category: "string" },
    BuiltinSpec { name: "dedup", arity: 1, category: "list" },
    BuiltinSpec { name: "condense", arity: 1, category: "list" },
    BuiltinSpec { name: "strip", arity: 2, category: "string" },
    BuiltinSpec { name: "chomp", arity: 1, category: "string" },
    BuiltinSpec { name: "repeat", arity: 2, category: "string" },
    BuiltinSpec { name: "pad_left", arity: 3, category: "string" },
    BuiltinSpec { name: "pad_right", arity: 3, category: "string" },
    BuiltinSpec { name: "lines", arity: 1, category: "string" },
    BuiltinSpec { name: "words", arity: 1, category: "string" },
    // ── sqz-inspired: TOON encoding (P2) ──
    BuiltinSpec { name: "toon_encode", arity: 1, category: "encoding" },
    BuiltinSpec { name: "toon_decode", arity: 1, category: "encoding" },
    // ── sqz-inspired: Content-addressed refs (P2) ──
    BuiltinSpec { name: "ref", arity: 1, category: "memory" },
    BuiltinSpec { name: "deref", arity: 1, category: "memory" },
    // ── sqz-inspired: Token awareness (P3) ──
    BuiltinSpec { name: "token_count", arity: 1, category: "string" },\
"""

old_registry_end = """\
    BuiltinSpec { name: "resolve_skill_index", arity: 1, category: "skill" },
    BuiltinSpec { name: "fit_to_budget", arity: 0, category: "skill" },
];"""

new_registry_end = """\
    BuiltinSpec { name: "resolve_skill_index", arity: 1, category: "skill" },
    BuiltinSpec { name: "fit_to_budget", arity: 0, category: "skill" },
""" + registry_entries + """
];"""

content = content.replace(old_registry_end, new_registry_end)

# ─── 2. Builtins::new() registrations (before "Builtins { funcs }") ──

new_registrations = """\
        // ── sqz-inspired: String/List utilities (P1) ──
        funcs.insert("squeeze".to_string(), builtin_squeeze as BuiltinFn);
        funcs.insert("dedup".to_string(), builtin_dedup as BuiltinFn);
        funcs.insert("condense".to_string(), builtin_condense as BuiltinFn);
        funcs.insert("strip".to_string(), builtin_strip as BuiltinFn);
        funcs.insert("chomp".to_string(), builtin_chomp as BuiltinFn);
        funcs.insert("repeat".to_string(), builtin_repeat as BuiltinFn);
        funcs.insert("pad_left".to_string(), builtin_pad_left as BuiltinFn);
        funcs.insert("pad_right".to_string(), builtin_pad_right as BuiltinFn);
        funcs.insert("lines".to_string(), builtin_lines as BuiltinFn);
        funcs.insert("words".to_string(), builtin_words as BuiltinFn);
        // ── sqz-inspired: TOON encoding (P2) ──
        funcs.insert("toon_encode".to_string(), builtin_toon_encode as BuiltinFn);
        funcs.insert("toon_decode".to_string(), builtin_toon_decode as BuiltinFn);
        // ── sqz-inspired: Content-addressed refs (P2) ──
        funcs.insert("ref".to_string(), builtin_content_ref as BuiltinFn);
        funcs.insert("deref".to_string(), builtin_content_deref as BuiltinFn);
        // ── sqz-inspired: Token awareness (P3) ──
        funcs.insert("token_count".to_string(), builtin_token_count as BuiltinFn);
"""

old_new_end = """\
        funcs.insert("assert_eq".to_string(), builtin_assert_eq as BuiltinFn);
        funcs.insert("assert_contains".to_string(), builtin_assert_contains as BuiltinFn);

        Builtins { funcs }"""

new_new_end = """\
        funcs.insert("assert_eq".to_string(), builtin_assert_eq as BuiltinFn);
        funcs.insert("assert_contains".to_string(), builtin_assert_contains as BuiltinFn);
""" + new_registrations + """\
        Builtins { funcs }"""

content = content.replace(old_new_end, new_new_end)

# ─── 3. Function implementations (append at end of file) ────────────

new_functions = r'''

// ════════════════════════════════════════════════════════════════════
// sqz-inspired builtins (P1 + P2 + P3)
// Source concept: https://github.com/ojuschugh1/sqz (ELv2 — no code copied)
// ════════════════════════════════════════════════════════════════════

// ── P1: String/List utilities ──────────────────────────────────────

/// `squeeze(s, chars)` — collapse consecutive identical characters from `chars`.
fn builtin_squeeze(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("squeeze", args, 0)?;
    let chars = expect_string_arg("squeeze", args, 1)?;
    if chars.is_empty() {
        return Ok(Value::String(s));
    }
    let char_set: std::collections::HashSet<char> = chars.chars().collect();
    let mut result = String::with_capacity(s.len());
    let mut prev: Option<char> = None;
    for c in s.chars() {
        if char_set.contains(&c) && prev == Some(c) {
            continue; // skip consecutive duplicate
        }
        result.push(c);
        prev = Some(c);
    }
    Ok(Value::String(result))
}

/// `dedup(list)` — remove duplicate elements, preserving first-occurrence order.
fn builtin_dedup(args: &[Value]) -> Result<Value, String> {
    let list = match args.get(0) {
        Some(Value::List(items)) => items.clone(),
        Some(other) => return Err(format!("dedup() expected List argument, got {}", other.type_name())),
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
                let json = mlog_value_to_json(other);
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
/// Example: ["a","a","b"] -> ["a","x2","b"]
fn builtin_condense(args: &[Value]) -> Result<Value, String> {
    let list = match args.get(0) {
        Some(Value::List(items)) => items.clone(),
        Some(other) => return Err(format!("condense() expected List argument, got {}", other.type_name())),
        None => return Err("condense() requires 1 argument".to_string()),
    };
    let mut result: Vec<Value> = Vec::new();
    let mut i = 0;
    while i < list.len() {
        let current = match &list[i] {
            Value::String(s) => s.clone(),
            other => return Err(format!(
                "condense() all elements must be String, got {} at index {}",
                other.type_name(), i
            )),
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

/// `strip(s, chars)` — remove characters from both ends of string.
fn builtin_strip(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("strip", args, 0)?;
    let chars = expect_string_arg("strip", args, 1)?;
    if chars.is_empty() {
        return Ok(Value::String(s));
    }
    let char_set: std::collections::HashSet<char> = chars.chars().collect();
    let start = s.chars().take_while(|c| char_set.contains(c)).count();
    let end = s.chars().rev().take_while(|c| char_set.contains(c)).count();
    let s_chars: Vec<char> = s.chars().collect();
    let trimmed: String = s_chars[start..s_chars.len().saturating_sub(end)].iter().collect();
    Ok(Value::String(trimmed))
}

/// `chomp(s)` — remove a single trailing newline (\n or \r\n).
fn builtin_chomp(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("chomp", args, 0)?;
    let trimmed = if s.ends_with("\r\n") {
        &s[..s.len() - 2]
    } else if s.ends_with('\n') {
        &s[..s.len() - 1]
    } else {
        &s[..]
    };
    Ok(Value::String(trimmed.to_string()))
}

/// `repeat(s, n)` — repeat string n times.
fn builtin_repeat(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("repeat", args, 0)?;
    let n = expect_float_arg("repeat", args, 1)?;
    if n < 0.0 {
        return Err("repeat() count must be non-negative".to_string());
    }
    let n_int = n as usize;
    if (n - n_int as f64).abs() > 1e-9 {
        return Err("repeat() count must be an integer".to_string());
    }
    Ok(Value::String(s.repeat(n_int)))
}

/// `pad_left(s, n, fill)` — left-pad string with fill character to length n.
fn builtin_pad_left(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("pad_left", args, 0)?;
    let n = expect_float_arg("pad_left", args, 1)?;
    let fill = expect_string_arg("pad_left", args, 2)?;
    if n < 0.0 {
        return Err("pad_left() width must be non-negative".to_string());
    }
    let n_int = n as usize;
    let fill_char = fill.chars().next().unwrap_or(' ');
    let s_len = s.chars().count();
    if s_len >= n_int {
        return Ok(Value::String(s));
    }
    let padding_len = n_int - s_len;
    let padding: String = std::iter::repeat(fill_char).take(padding_len).collect();
    Ok(Value::String(format!("{}{}", padding, s)))
}

/// `pad_right(s, n, fill)` — right-pad string with fill character to length n.
fn builtin_pad_right(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("pad_right", args, 0)?;
    let n = expect_float_arg("pad_right", args, 1)?;
    let fill = expect_string_arg("pad_right", args, 2)?;
    if n < 0.0 {
        return Err("pad_right() width must be non-negative".to_string());
    }
    let n_int = n as usize;
    let fill_char = fill.chars().next().unwrap_or(' ');
    let s_len = s.chars().count();
    if s_len >= n_int {
        return Ok(Value::String(s));
    }
    let padding_len = n_int - s_len;
    let padding: String = std::iter::repeat(fill_char).take(padding_len).collect();
    Ok(Value::String(format!("{}{}", s, padding)))
}

/// `lines(s)` — split string into list of lines (no trailing empty element).
fn builtin_lines(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("lines", args, 0)?;
    let items: Vec<Value> = s
        .split('\n')
        .map(|line| {
            // Handle \r\n: strip trailing \r from each line
            let trimmed = line.strip_suffix('\r').unwrap_or(line);
            Value::String(trimmed.to_string())
        })
        // Remove trailing empty element from split
        .collect();
    // If the last element is empty string (from trailing newline), remove it
    let mut items = items;
    if items.last().map_or(false, |v| matches!(v, Value::String(s) if s.is_empty())) {
        // Only remove if original string ended with newline
        if s.ends_with('\n') {
            items.pop();
        }
    }
    Ok(Value::List(items))
}

/// `words(s)` — split string into list of words by whitespace.
fn builtin_words(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("words", args, 0)?;
    let items: Vec<Value> = s
        .split_whitespace()
        .map(|w| Value::String(w.to_string()))
        .collect();
    Ok(Value::List(items))
}

// ── P2: TOON encoding ──────────────────────────────────────────────

/// Check if a string is a "simple" identifier (no quoting needed in TOON).
fn toon_is_simple(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Escape a string for TOON: non-ASCII -> \uXXXX, quotes -> \", backslash -> \\
fn toon_escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ if c.is_ascii() => out.push(c),
            _ => {
                let code = c as u32;
                out.push_str(&format!("\\u{{{:04x}}}", code));
            }
        }
    }
    out
}

/// Encode a Value to TOON format (recursive).
fn value_to_toon(val: &Value) -> String {
    match val {
        Value::String(s) => {
            if toon_is_simple(s) {
                format!("s\"{}\"", s)
            } else {
                format!("s\"{}\"", toon_escape_string(s))
            }
        }
        Value::Float(f) => {
            if *f == f.floor() && f.abs() < 1e15 {
                format!("{}", *f as i64)
            } else {
                format!("{}", f)
            }
        }
        Value::Bool(b) => (if *b { "true" } else { "false" }).to_string(),
        Value::Unit => "null".to_string(),
        Value::List(items) => {
            let inner: Vec<String> = items.iter().map(value_to_toon).collect();
            format!("[{}]", inner.join(","))
        }
        Value::Struct { fields, .. } => {
            let pairs: Vec<String> = fields
                .iter()
                .map(|(k, v)| {
                    let key = if toon_is_simple(k) {
                        k.clone()
                    } else {
                        format!("s\"{}\"", toon_escape_string(k))
                    };
                    format!("{}:{}", key, value_to_toon(v))
                })
                .collect();
            format!("{{{}}}", pairs.join(","))
        }
        other => format!("s\"{}\"", toon_escape_string(&format!("{}", other))),
    }
}

/// `toon_encode(value)` — encode any value to TOON (Token-Optimized Object Notation).
fn builtin_toon_encode(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("toon_encode() requires 1 argument".to_string());
    }
    let encoded = value_to_toon(&args[0]);
    Ok(Value::String(format!("TOON:{}", encoded)))
}

/// Decode a TOON string back into a Value. Recursive descent parser.
fn parse_toon_value(input: &str, pos: &mut usize) -> Result<Value, String> {
    let bytes = input.as_bytes();
    if *pos >= bytes.len() {
        return Err("toon_decode: unexpected end of input".to_string());
    }

    match bytes[*pos] {
        b't' => {
            // true
            if input[*pos..].starts_with("true") {
                *pos += 4;
                return Ok(Value::Bool(true));
            }
            Err(format!("toon_decode: invalid token at position {}", pos))
        }
        b'f' => {
            // false
            if input[*pos..].starts_with("false") {
                *pos += 5;
                return Ok(Value::Bool(false));
            }
            Err(format!("toon_decode: invalid token at position {}", pos))
        }
        b'n' => {
            // null
            if input[*pos..].starts_with("null") {
                *pos += 4;
                return Ok(Value::Unit);
            }
            Err(format!("toon_decode: invalid token at position {}", pos))
        }
        b's' => {
            // s"..." — quoted string
            if *pos + 1 >= bytes.len() || bytes[*pos + 1] != b'"' {
                return Err(format!("toon_decode: expected s\"...\" at position {}", pos));
            }
            *pos += 2; // skip s"
            let mut result = String::new();
            while *pos < bytes.len() {
                match bytes[*pos] {
                    b'"' => {
                        *pos += 1;
                        return Ok(Value::String(result));
                    }
                    b'\\' => {
                        *pos += 1;
                        if *pos >= bytes.len() {
                            return Err("toon_decode: unterminated escape".to_string());
                        }
                        match bytes[*pos] {
                            b'"' => { result.push('"'); *pos += 1; }
                            b'\\' => { result.push('\\'); *pos += 1; }
                            b'n' => { result.push('\n'); *pos += 1; }
                            b't' => { result.push('\t'); *pos += 1; }
                            b'r' => { result.push('\r'); *pos += 1; }
                            b'u' => {
                                // \u{XXXX}
                                *pos += 1;
                                if *pos >= bytes.len() || bytes[*pos] != b'{' {
                                    return Err(format!("toon_decode: expected {{ after \\u at position {}", pos));
                                }
                                *pos += 1;
                                let hex_start = *pos;
                                while *pos < bytes.len() && bytes[*pos] != b'}' {
                                    *pos += 1;
                                }
                                if *pos >= bytes.len() {
                                    return Err("toon_decode: unterminated \\u{{...}}".to_string());
                                }
                                let hex_str = &input[hex_start..*pos];
                                *pos += 1; // skip }
                                let code_point = u32::from_str_radix(hex_str, 16)
                                    .map_err(|e| format!("toon_decode: invalid unicode escape: {}", e))?;
                                if let Some(c) = char::from_u32(code_point) {
                                    result.push(c);
                                } else {
                                    return Err(format!("toon_decode: invalid unicode code point: {:x}", code_point));
                                }
                            }
                            other => {
                                result.push(other as char);
                                *pos += 1;
                            }
                        }
                    }
                    other => {
                        result.push(other as char);
                        *pos += 1;
                    }
                }
            }
            Err("toon_decode: unterminated string".to_string())
        }
        b'[' => {
            // Array
            *pos += 1;
            let mut items = Vec::new();
            while *pos < bytes.len() && bytes[*pos] != b']' {
                if bytes[*pos] == b',' {
                    *pos += 1;
                    continue;
                }
                items.push(parse_toon_value(input, pos)?);
            }
            if *pos >= bytes.len() {
                return Err("toon_decode: unterminated array".to_string());
            }
            *pos += 1; // skip ]
            Ok(Value::List(items))
        }
        b'{' => {
            // Object -> Struct
            *pos += 1;
            let mut fields = std::collections::HashMap::new();
            while *pos < bytes.len() && bytes[*pos] != b'}' {
                if bytes[*pos] == b',' {
                    *pos += 1;
                    continue;
                }
                // Parse key
                let key = if bytes[*pos] == b's' && *pos + 1 < bytes.len() && bytes[*pos + 1] == b'"' {
                    // s"key"
                    *pos += 2;
                    let mut k = String::new();
                    while *pos < bytes.len() && bytes[*pos] != b'"' {
                        if bytes[*pos] == b'\\' {
                            *pos += 1;
                            if *pos < bytes.len() {
                                k.push(bytes[*pos] as char);
                                *pos += 1;
                            }
                        } else {
                            k.push(bytes[*pos] as char);
                            *pos += 1;
                        }
                    }
                    if *pos < bytes.len() { *pos += 1; } // skip closing "
                    k
                } else {
                    // bare identifier
                    let start = *pos;
                    while *pos < bytes.len() && (bytes[*pos].is_ascii_alphanumeric() || bytes[*pos] == b'_' || bytes[*pos] == b'-') {
                        *pos += 1;
                    }
                    input[start..*pos].to_string()
                };
                // Expect ':'
                if *pos >= bytes.len() || bytes[*pos] != b':' {
                    return Err(format!("toon_decode: expected ':' after key '{}' at position {}", key, pos));
                }
                *pos += 1;
                // Parse value
                let val = parse_toon_value(input, pos)?;
                fields.insert(key, val);
            }
            if *pos >= bytes.len() {
                return Err("toon_decode: unterminated object".to_string());
            }
            *pos += 1; // skip }
            Ok(Value::Struct { type_name: "TOON".to_string(), fields })
        }
        b'-' | b'0'..=b'9' => {
            // Number
            let start = *pos;
            if bytes[*pos] == b'-' { *pos += 1; }
            while *pos < bytes.len() && (bytes[*pos].is_ascii_digit() || bytes[*pos] == b'.') {
                *pos += 1;
            }
            let num_str = &input[start..*pos];
            let f: f64 = num_str.parse()
                .map_err(|e| format!("toon_decode: invalid number '{}' at position {}: {}", num_str, start, e))?;
            Ok(Value::Float(f))
        }
        other => {
            Err(format!("toon_decode: unexpected character '{}' at position {}", other as char, pos))
        }
    }
}

/// `toon_decode(s)` — decode TOON string back to Value.
fn builtin_toon_decode(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("toon_decode", args, 0)?;
    if !s.starts_with("TOON:") {
        return Err("toon_decode: input must start with 'TOON:'".to_string());
    }
    let payload = &s[5..];
    let mut pos = 0;
    let value = parse_toon_value(payload, &mut pos)?;
    // Skip trailing whitespace
    while pos < payload.len() && payload.as_bytes()[pos] == b' ' {
        pos += 1;
    }
    if pos < payload.len() {
        return Err(format!("toon_decode: unexpected trailing data at position {}", 5 + pos));
    }
    Ok(value)
}

// ── P2: Content-addressed refs ─────────────────────────────────────

/// `ref(content)` — compute SHA-256 hash, store in KV, return hash string. Idempotent.
fn builtin_content_ref(args: &[Value]) -> Result<Value, String> {
    let content = expect_string_arg("ref", args, 0)?;
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash = hex::encode(hasher.finalize());
    let key = format!("__ref:{}", hash);
    // Only set if not already present (idempotent)
    let store = kv_store().lock().map_err(|e| format!("ref() lock error: {}", e))?;
    if !store.contains_key(&key) {
        drop(store); // release read lock
        let mut store = kv_store().lock().map_err(|e| format!("ref() lock error: {}", e))?;
        store.insert(key.clone(), content.clone());
        // Write-through to SQLite if available
        if let Ok(sqlite_guard) = kv_sqlite().lock() {
            if let Some(ref conn) = *sqlite_guard {
                let _ = conn.execute(
                    "INSERT OR IGNORE INTO kv_store (key, value) VALUES (?1, ?2)",
                    rusqlite::params![key, content],
                );
            }
        }
    }
    Ok(Value::String(hash))
}

/// `deref(hash)` — retrieve content by SHA-256 hash from ref store.
fn builtin_content_deref(args: &[Value]) -> Result<Value, String> {
    let hash = expect_string_arg("deref", args, 0)?;
    if hash.len() != 64 {
        return Err("deref: invalid hash format, expected 64-char hex string".to_string());
    }
    if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("deref: invalid hash format, expected hex characters only".to_string());
    }
    let key = format!("__ref:{}", hash);
    let store = kv_store().lock().map_err(|e| format!("deref() lock error: {}", e))?;
    match store.get(&key) {
        Some(content) => Ok(Value::String(content.clone())),
        None => Err("deref: hash not found in ref store".to_string()),
    }
}

// ── P3: Token awareness ────────────────────────────────────────────

/// `token_count(text)` — estimate token count. Cyrillic: chars/2, Latin: chars/4.
fn builtin_token_count(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("token_count", args, 0)?;
    if s.is_empty() {
        return Ok(Value::Float(0.0));
    }
    let total_chars = s.chars().count();
    let cyrillic_chars = s.chars().filter(|c| matches!(c, '\u{0400}'..='\u{04FF}')).count();
    // If >=50% Cyrillic, use /2 divisor; else /4
    let divisor = if total_chars > 0 && (cyrillic_chars as f64 / total_chars as f64) >= 0.5 {
        2.0
    } else {
        4.0
    };
    let tokens = (total_chars as f64 / divisor).ceil();
    Ok(Value::Float(tokens))
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests_sqz_builtins {
    use super::*;

    // ── P1 tests ──

    #[test]
    fn test_squeeze_basic() {
        let r = builtin_squeeze(&[Value::String("aaabbbccc".into()), Value::String("abc".into())]);
        assert_eq!(r.unwrap(), Value::String("abc".into()));
    }

    #[test]
    fn test_squeeze_partial() {
        let r = builtin_squeeze(&[Value::String("aaabbbccc".into()), Value::String("ab".into())]);
        assert_eq!(r.unwrap(), Value::String("abccc".into()));
    }

    #[test]
    fn test_squeeze_empty_chars() {
        let r = builtin_squeeze(&[Value::String("hello".into()), Value::String("".into())]);
        assert_eq!(r.unwrap(), Value::String("hello".into()));
    }

    #[test]
    fn test_squeeze_empty_string() {
        let r = builtin_squeeze(&[Value::String("".into()), Value::String("a".into())]);
        assert_eq!(r.unwrap(), Value::String("".into()));
    }

    #[test]
    fn test_dedup_basic() {
        let r = builtin_dedup(&[Value::List(vec![
            Value::Float(1.0), Value::Float(1.0), Value::Float(2.0), Value::Float(2.0), Value::Float(3.0),
        ])]);
        assert_eq!(r.unwrap(), Value::List(vec![
            Value::Float(1.0), Value::Float(2.0), Value::Float(3.0),
        ]));
    }

    #[test]
    fn test_dedup_strings() {
        let r = builtin_dedup(&[Value::List(vec![
            Value::String("a".into()), Value::String("b".into()), Value::String("a".into()), Value::String("c".into()),
        ])]);
        assert_eq!(r.unwrap(), Value::List(vec![
            Value::String("a".into()), Value::String("b".into()), Value::String("c".into()),
        ]));
    }

    #[test]
    fn test_dedup_empty() {
        let r = builtin_dedup(&[Value::List(vec![])]);
        assert_eq!(r.unwrap(), Value::List(vec![]));
    }

    #[test]
    fn test_condense_basic() {
        let r = builtin_condense(&[Value::List(vec![
            Value::String("error".into()), Value::String("error".into()), Value::String("error".into()),
            Value::String("warn".into()), Value::String("info".into()),
        ])]);
        assert_eq!(r.unwrap(), Value::List(vec![
            Value::String("error".into()), Value::String("\u{00d7}3".into()),
            Value::String("warn".into()), Value::String("info".into()),
        ]));
    }

    #[test]
    fn test_condense_repeating_groups() {
        let r = builtin_condense(&[Value::List(vec![
            Value::String("a".into()), Value::String("a".into()),
            Value::String("b".into()), Value::String("b".into()), Value::String("b".into()),
            Value::String("a".into()),
        ])]);
        assert_eq!(r.unwrap(), Value::List(vec![
            Value::String("a".into()), Value::String("\u{00d7}2".into()),
            Value::String("b".into()), Value::String("\u{00d7}3".into()),
            Value::String("a".into()),
        ]));
    }

    #[test]
    fn test_condense_single() {
        let r = builtin_condense(&[Value::List(vec![Value::String("single".into())])]);
        assert_eq!(r.unwrap(), Value::List(vec![Value::String("single".into())]));
    }

    #[test]
    fn test_strip_basic() {
        let r = builtin_strip(&[Value::String("///hello///".into()), Value::String("/".into())]);
        assert_eq!(r.unwrap(), Value::String("hello".into()));
    }

    #[test]
    fn test_strip_whitespace() {
        let r = builtin_strip(&[Value::String("  hello  ".into()), Value::String(" ".into())]);
        assert_eq!(r.unwrap(), Value::String("hello".into()));
    }

    #[test]
    fn test_strip_no_match() {
        let r = builtin_strip(&[Value::String("abc".into()), Value::String("x".into())]);
        assert_eq!(r.unwrap(), Value::String("abc".into()));
    }

    #[test]
    fn test_chomp_newline() {
        assert_eq!(builtin_chomp(&[Value::String("hello\n".into())]).unwrap(), Value::String("hello".into()));
    }

    #[test]
    fn test_chomp_crlf() {
        assert_eq!(builtin_chomp(&[Value::String("hello\r\n".into())]).unwrap(), Value::String("hello".into()));
    }

    #[test]
    fn test_chomp_no_newline() {
        assert_eq!(builtin_chomp(&[Value::String("hello".into())]).unwrap(), Value::String("hello".into()));
    }

    #[test]
    fn test_chomp_double_newline() {
        assert_eq!(builtin_chomp(&[Value::String("hello\n\n".into())]).unwrap(), Value::String("hello\n".into()));
    }

    #[test]
    fn test_repeat_basic() {
        assert_eq!(builtin_repeat(&[Value::String("-".into()), Value::Float(10.0)]).unwrap(), Value::String("----------".into()));
    }

    #[test]
    fn test_repeat_multiple() {
        assert_eq!(builtin_repeat(&[Value::String("ab".into()), Value::Float(3.0)]).unwrap(), Value::String("ababab".into()));
    }

    #[test]
    fn test_repeat_zero() {
        assert_eq!(builtin_repeat(&[Value::String("x".into()), Value::Float(0.0)]).unwrap(), Value::String("".into()));
    }

    #[test]
    fn test_repeat_negative() {
        assert!(builtin_repeat(&[Value::String("x".into()), Value::Float(-1.0)]).is_err());
    }

    #[test]
    fn test_repeat_non_integer() {
        assert!(builtin_repeat(&[Value::String("x".into()), Value::Float(2.5)]).is_err());
    }

    #[test]
    fn test_pad_left_basic() {
        assert_eq!(builtin_pad_left(&[Value::String("42".into()), Value::Float(5.0), Value::String("0".into())]).unwrap(), Value::String("00042".into()));
    }

    #[test]
    fn test_pad_left_noop() {
        assert_eq!(builtin_pad_left(&[Value::String("hello".into()), Value::Float(3.0), Value::String("x".into())]).unwrap(), Value::String("hello".into()));
    }

    #[test]
    fn test_pad_right_basic() {
        assert_eq!(builtin_pad_right(&[Value::String("name".into()), Value::Float(10.0), Value::String(".".into())]).unwrap(), Value::String("name.....".into()));
    }

    #[test]
    fn test_lines_basic() {
        assert_eq!(builtin_lines(&[Value::String("a\nb\nc".into())]).unwrap(), Value::List(vec![
            Value::String("a".into()), Value::String("b".into()), Value::String("c".into()),
        ]));
    }

    #[test]
    fn test_lines_trailing_newline() {
        assert_eq!(builtin_lines(&[Value::String("hello\nworld\n".into())]).unwrap(), Value::List(vec![
            Value::String("hello".into()), Value::String("world".into()),
        ]));
    }

    #[test]
    fn test_lines_empty() {
        assert_eq!(builtin_lines(&[Value::String("".into())]).unwrap(), Value::List(vec![]));
    }

    #[test]
    fn test_words_basic() {
        assert_eq!(builtin_words(&[Value::String("hello world".into())]).unwrap(), Value::List(vec![
            Value::String("hello".into()), Value::String("world".into()),
        ]));
    }

    #[test]
    fn test_words_extra_whitespace() {
        assert_eq!(builtin_words(&[Value::String("  a  b  c  ".into())]).unwrap(), Value::List(vec![
            Value::String("a".into()), Value::String("b".into()), Value::String("c".into()),
        ]));
    }

    #[test]
    fn test_words_empty() {
        assert_eq!(builtin_words(&[Value::String("".into())]).unwrap(), Value::List(vec![]));
    }

    // ── P2 tests: TOON ──

    #[test]
    fn test_toon_encode_string() {
        let r = builtin_toon_encode(&[Value::String("hello".into())]);
        assert_eq!(r.unwrap(), Value::String("TOON:s\"hello\"".into()));
    }

    #[test]
    fn test_toon_encode_float() {
        let r = builtin_toon_encode(&[Value::Float(42.0)]);
        assert_eq!(r.unwrap(), Value::String("TOON:42".into()));
    }

    #[test]
    fn test_toon_encode_list() {
        let r = builtin_toon_encode(&[Value::List(vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)])]);
        assert_eq!(r.unwrap(), Value::String("TOON:[1,2,3]".into()));
    }

    #[test]
    fn test_toon_encode_bool() {
        let r = builtin_toon_encode(&[Value::Bool(true)]);
        assert_eq!(r.unwrap(), Value::String("TOON:true".into()));
    }

    #[test]
    fn test_toon_encode_null() {
        let r = builtin_toon_encode(&[Value::Unit]);
        assert_eq!(r.unwrap(), Value::String("TOON:null".into()));
    }

    #[test]
    fn test_toon_encode_struct() {
        let mut fields = std::collections::HashMap::new();
        fields.insert("name".to_string(), Value::String("Alice".into()));
        fields.insert("age".to_string(), Value::Float(30.0));
        let r = builtin_toon_encode(&[Value::Struct { type_name: "Person".into(), fields }]);
        assert_eq!(r.unwrap(), Value::String("TOON:{name:s\"Alice\",age:30}".into()));
    }

    #[test]
    fn test_toon_roundtrip_string() {
        let original = Value::String("hello world".into());
        let encoded = builtin_toon_encode(&[original.clone()]).unwrap();
        let decoded = builtin_toon_decode(&[encoded]).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_toon_roundtrip_float() {
        let original = Value::Float(3.14);
        let encoded = builtin_toon_encode(&[original.clone()]).unwrap();
        let decoded = builtin_toon_decode(&[encoded]).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_toon_roundtrip_list() {
        let original = Value::List(vec![Value::Float(1.0), Value::String("ok".into()), Value::Bool(false)]);
        let encoded = builtin_toon_encode(&[original.clone()]).unwrap();
        let decoded = builtin_toon_decode(&[encoded]).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_toon_roundtrip_struct() {
        let mut fields = std::collections::HashMap::new();
        fields.insert("x".to_string(), Value::Float(1.0));
        let original = Value::Struct { type_name: "P".into(), fields };
        let encoded = builtin_toon_encode(&[original.clone()]).unwrap();
        let decoded = builtin_toon_decode(&[encoded]).unwrap();
        // Compare JSON representations (type_name differs)
        assert_eq!(mlog_value_to_json(&original), mlog_value_to_json(&decoded));
    }

    #[test]
    fn test_toon_roundtrip_cyrillic() {
        let original = Value::String("\u041f\u0440\u0438\u0432\u0435\u0442".into());
        let encoded = builtin_toon_encode(&[original.clone()]).unwrap();
        let decoded = builtin_toon_decode(&[encoded]).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_toon_decode_no_prefix() {
        let r = builtin_toon_decode(&[Value::String("invalid".into())]);
        assert!(r.is_err());
    }

    #[test]
    fn test_toon_decode_invalid() {
        let r = builtin_toon_decode(&[Value::String("TOON:{{broken".into())]);
        assert!(r.is_err());
    }

    // ── P2 tests: ref/deref ──

    #[test]
    fn test_ref_deref_roundtrip() {
        // Clear any previous state for this hash
        let content = "hello world test roundtrip";
        let hash_val = builtin_content_ref(&[Value::String(content.into())]).unwrap();
        let hash_str = match &hash_val {
            Value::String(s) => s.clone(),
            _ => panic!("expected String"),
        };
        assert_eq!(hash_str.len(), 64); // SHA-256 hex = 64 chars
        let derefed = builtin_content_deref(&[Value::String(hash_str.clone())]).unwrap();
        assert_eq!(derefed, Value::String(content.into()));
    }

    #[test]
    fn test_ref_idempotent() {
        let content = "idempotent test content";
        let h1 = builtin_content_ref(&[Value::String(content.into())]).unwrap();
        let h2 = builtin_content_ref(&[Value::String(content.into())]).unwrap();
        assert_eq!(h1, h2); // same hash
    }

    #[test]
    fn test_deref_not_found() {
        let r = builtin_content_deref(&[Value::String("a".repeat(64))]); // unlikely to exist
        assert!(r.is_err());
    }

    #[test]
    fn test_deref_invalid_format() {
        assert!(builtin_content_deref(&[Value::String("tooshort".into())]).is_err());
        assert!(builtin_content_deref(&[Value::String("zz".repeat(32).into())]).is_err()); // non-hex
    }

    // ── P3 tests: token_count ──

    #[test]
    fn test_token_count_ascii() {
        // "hello world" = 11 chars, /4 = 2.75, ceil = 3
        let r = builtin_token_count(&[Value::String("hello world".into())]);
        assert_eq!(r.unwrap(), Value::Float(3.0));
    }

    #[test]
    fn test_token_count_cyrillic() {
        // "\u041f\u0440\u0438\u0432\u0435\u0442 \u043c\u0438\u0440" = 10 chars, 100% cyrillic, /2 = 5
        let r = builtin_token_count(&[Value::String("\u041f\u0440\u0438\u0432\u0435\u0442 \u043c\u0438\u0440".into())]);
        assert_eq!(r.unwrap(), Value::Float(5.0));
    }

    #[test]
    fn test_token_count_empty() {
        let r = builtin_token_count(&[Value::String("".into())]);
        assert_eq!(r.unwrap(), Value::Float(0.0));
    }

    #[test]
    fn test_token_count_mixed() {
        // "Hello \u043c\u0438\u0440" = 9 chars, 3 cyrillic = 33%, < 50% so /4 = 2.25, ceil = 3
        let r = builtin_token_count(&[Value::String("Hello \u043c\u0438\u0440".into())]);
        assert_eq!(r.unwrap(), Value::Float(3.0));
    }
}
'''

content = content.rstrip() + "\n" + new_functions

with open(BUILTINS_PATH, "w") as f:
    f.write(content)

print("OK: builtins.rs updated with 15 new builtin functions + tests")