// -- String / text builtins ---------------------------------------------------

use crate::interpreter::Value;
use base64::Engine;

use super::core::*;

// -- Basic string operations --------------------------------------------------

pub(crate) fn builtin_upper(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("upper", args, 0)?;
    Ok(Value::String(s.to_uppercase()))
}

pub(crate) fn builtin_lower(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("lower", args, 0)?;
    Ok(Value::String(s.to_lowercase()))
}

pub(crate) fn builtin_len(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        // Unicode-aware: chars().count() returns character count, not byte count.
        // "\u{041f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}" (6 chars, 12 bytes) -> 6.0, not 12.0.
        Some(Value::String(s)) => Ok(Value::Float(s.chars().count() as f64)),
        Some(Value::List(items)) => Ok(Value::Float(items.len() as f64)),
        _ => Err("len() requires String or List argument".to_string()),
    }
}

pub(crate) fn builtin_str(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("str() requires 1 argument".to_string());
    }
    Ok(Value::String(format!("{}", args[0])))
}

pub(crate) fn builtin_contains(args: &[Value]) -> Result<Value, String> {
    let haystack = expect_string_arg("contains", args, 0)?;
    let needle = expect_string_arg("contains", args, 1)?;
    Ok(Value::Bool(haystack.contains(&needle)))
}

pub(crate) fn builtin_index_of(args: &[Value]) -> Result<Value, String> {
    let haystack = expect_string_arg("index_of", args, 0)?;
    let needle = expect_string_arg("index_of", args, 1)?;
    // Unicode-aware: return CHARACTER position, not byte offset.
    // "\u{041f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}, \u{043c}\u{0438}\u{0440}".find("\u{043c}\u{0438}\u{0440}") byte offset = 12, char offset = 8.
    // Must be consistent with substring()/char_at() which use char indices.
    let char_pos = haystack
        .char_indices()
        .find(|(byte_idx, _)| haystack[*byte_idx..].starts_with(&needle))
        .map(|(byte_idx, _)| haystack[..byte_idx].chars().count());
    match char_pos {
        Some(pos) => Ok(Value::Float(pos as f64)),
        None => Ok(Value::Float(-1.0)),
    }
}

pub(crate) fn builtin_substring(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("substring", args, 0)?;
    let start = expect_float_arg("substring", args, 1)? as usize;
    let end = expect_float_arg("substring", args, 2)? as usize;
    // Soft-failure: clamp to valid range, empty string if start >= len
    let s_len = s.chars().count();
    if start >= s_len {
        return Ok(Value::String(String::new()));
    }
    let end = if end > s_len { s_len } else { end };
    if start >= end {
        return Ok(Value::String(String::new()));
    }
    // Convert byte indices for char-based slicing
    let chars: Vec<char> = s.chars().collect();
    let result: String = chars[start..end].iter().collect();
    Ok(Value::String(result))
}

pub(crate) fn builtin_char_at(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("char_at", args, 0)?;
    let index = expect_float_arg("char_at", args, 1)? as usize;
    // Soft-failure: return empty string on out-of-bounds
    let chars: Vec<char> = s.chars().collect();
    match chars.get(index) {
        Some(ch) => Ok(Value::String(ch.to_string())),
        None => Ok(Value::String(String::new())),
    }
}

pub(crate) fn builtin_starts_with(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("starts_with", args, 0)?;
    let prefix = expect_string_arg("starts_with", args, 1)?;
    Ok(Value::Bool(s.starts_with(&prefix)))
}

pub(crate) fn builtin_ends_with(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("ends_with", args, 0)?;
    let suffix = expect_string_arg("ends_with", args, 1)?;
    Ok(Value::Bool(s.ends_with(&suffix)))
}

// -- Stdlib backing builtins (Phase 5.4) ------------------------------------
// These implement the primitives used by std/*.mlog pattern wrappers.

pub(crate) fn builtin_trim(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("__trim", args, 0)?;
    Ok(Value::String(s.trim().to_string()))
}

pub(crate) fn builtin_replace(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("__replace", args, 0)?;
    let old = expect_string_arg("__replace", args, 1)?;
    let new = expect_string_arg("__replace", args, 2)?;
    if old.is_empty() {
        // Empty pattern would insert replacement between every character -- return original
        Ok(Value::String(s))
    } else {
        Ok(Value::String(s.replace(&old, &new)))
    }
}

pub(crate) fn builtin_split(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("__split", args, 0)?;
    let sep = expect_string_arg("__split", args, 1)?;
    let items: Vec<Value> = if sep.is_empty() {
        s.chars().map(|c| Value::String(c.to_string())).collect()
    } else {
        s.split(&sep)
            .map(|part| Value::String(part.to_string()))
            .collect()
    };
    Ok(Value::List(items))
}

pub(crate) fn builtin_join(args: &[Value]) -> Result<Value, String> {
    let list = match args.first() {
        Some(Value::List(items)) => items,
        _ => return Err("__join() requires List as first argument".to_string()),
    };
    let sep = if args.len() > 1 {
        match &args[1] {
            Value::String(s) => s.clone(),
            _ => ",".to_string(),
        }
    } else {
        ",".to_string()
    };
    let parts: Vec<String> = list.iter().map(|v| format!("{}", v)).collect();
    Ok(Value::String(parts.join(&sep)))
}

// -- HTML escaping -----------------------------------------------------------

pub(crate) fn builtin_escape_html(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("escape_html", args, 0)?;
    Ok(Value::String(escape_html_chars(&s)))
}

/// HTML-escape a string (for use in templates and escape_html builtin).
pub(crate) fn escape_html_chars(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

// -- JSON escaping -----------------------------------------------------------

/// Escape a string for safe embedding inside a JSON string value.
/// Replaces: " -> \" , \\ -> \\\\ , newline -> \n , tab -> \t , carriage return -> \r
/// Usage: escape_json(text) -> String
pub(crate) fn builtin_escape_json(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("escape_json", args, 0)?;
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            _ => out.push(ch),
        }
    }
    Ok(Value::String(out))
}

// -- v0.5.0 -- New string builtins -------------------------------------------

/// `reverse(s)` -- reverse a string or list.
pub(crate) fn builtin_reverse(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => Ok(Value::String(s.chars().rev().collect())),
        Some(Value::List(items)) => {
            let mut rev = items.clone();
            rev.reverse();
            Ok(Value::List(rev))
        }
        other => Err(format!(
            "reverse() requires String or List, got {}",
            other.as_ref().map(|v| v.type_name()).unwrap_or("none")
        )),
    }
}

// -- Narjad 17: Utility builtins ---------------------------------------------

/// `base64_encode(s) -> String` -- encode a string to base64.
pub(crate) fn builtin_base64_encode(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("base64_encode", args, 0)?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(s.as_bytes());
    Ok(Value::String(encoded))
}

/// `base64_decode(s) -> String` -- decode a base64 string.
pub(crate) fn builtin_base64_decode(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("base64_decode", args, 0)?;
    match base64::engine::general_purpose::STANDARD.decode(s.as_bytes()) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(decoded) => Ok(Value::String(decoded)),
            Err(_) => Err("base64_decode(): decoded bytes are not valid UTF-8".to_string()),
        },
        Err(e) => Err(format!("base64_decode(): invalid base64: {}", e)),
    }
}

/// `escape_js(s) -> String` -- escape a string for safe insertion into JavaScript.
/// Escapes: backslash, single quote, double quote, newline, carriage return, tab,
/// line separator, paragraph separator, and NUL.
pub(crate) fn builtin_escape_js(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("escape_js", args, 0)?;
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{2028}' => out.push_str("\\u{2028}"), // line separator
            '\u{2029}' => out.push_str("\\u{2029}"), // paragraph separator
            '\0' => out.push_str("\\0"),
            _ => out.push(c),
        }
    }
    Ok(Value::String(out))
}

// -- OpenPlanter-inspired: Fuzzy matching (ADR-0063) --------------------------

/// `fuzzy_match(a, b)` -- Jaro-Winkler similarity between two strings (0.0..1.0).
/// Ported from OpenPlanter's wiki/matching.rs NameRegistry pattern.
pub(crate) fn builtin_fuzzy_match(args: &[Value]) -> Result<Value, String> {
    let a = expect_string_arg("fuzzy_match", args, 0)?;
    let b = expect_string_arg("fuzzy_match", args, 1)?;
    let score = strsim::jaro_winkler(&a, &b);
    Ok(Value::Float(score))
}

// -- Format (Naryad #17 V.3) -------------------------------------------------

/// `format(template, arg1, arg2, ...)` -- positional string interpolation.
/// Replaces `{}` placeholders in template with arguments.
/// Usage: format("Hello {}, you are {} years old", name, age)
pub(crate) fn builtin_format(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("format() requires at least 1 argument (template)".to_string());
    }
    let template = expect_string_arg("format", args, 0)?;
    let mut result = String::new();
    let mut arg_idx = 1;
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '{' && chars.peek() == Some(&'}') {
            chars.next(); // consume '}'
            if arg_idx < args.len() {
                result.push_str(&format!("{}", args[arg_idx]));
                arg_idx += 1;
            } else {
                return Err(format!(
                    "format(): not enough arguments for template (need {} more)",
                    arg_idx - 1
                ));
            }
        } else if ch == '{' && chars.peek() == Some(&'{') {
            chars.next(); // consume second '{', emit literal '{'
            result.push('{');
        } else {
            result.push(ch);
        }
    }
    Ok(Value::String(result))
}

// -- sqz-inspired builtins (P1 + P2 + P3) -----------------------------------
// Source concept: https://github.com/ojuschugh1/sqz (ELv2 -- no code copied)

// -- P1: String/List utilities -----------------------------------------------

/// `squeeze(s, chars)` -- collapse consecutive identical characters from `chars`.
pub(crate) fn builtin_squeeze(args: &[Value]) -> Result<Value, String> {
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

/// `strip(s, chars)` -- remove characters from both ends of string.
pub(crate) fn builtin_strip(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("strip", args, 0)?;
    let chars = expect_string_arg("strip", args, 1)?;
    if chars.is_empty() {
        return Ok(Value::String(s));
    }
    let char_set: std::collections::HashSet<char> = chars.chars().collect();
    let start = s.chars().take_while(|c| char_set.contains(c)).count();
    let end = s.chars().rev().take_while(|c| char_set.contains(c)).count();
    let s_chars: Vec<char> = s.chars().collect();
    let trimmed: String = s_chars[start..s_chars.len().saturating_sub(end)]
        .iter()
        .collect();
    Ok(Value::String(trimmed))
}

/// `chomp(s)` -- remove a single trailing newline (\n or \r\n).
pub(crate) fn builtin_chomp(args: &[Value]) -> Result<Value, String> {
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

/// `repeat(s, n)` -- repeat string n times.
pub(crate) fn builtin_repeat(args: &[Value]) -> Result<Value, String> {
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

/// `pad_left(s, n, fill)` -- left-pad string with fill character to length n.
pub(crate) fn builtin_pad_left(args: &[Value]) -> Result<Value, String> {
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
    let padding: String = std::iter::repeat_n(fill_char, padding_len).collect();
    Ok(Value::String(format!("{}{}", padding, s)))
}

/// `pad_right(s, n, fill)` -- right-pad string with fill character to length n.
pub(crate) fn builtin_pad_right(args: &[Value]) -> Result<Value, String> {
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
    let padding: String = std::iter::repeat_n(fill_char, padding_len).collect();
    Ok(Value::String(format!("{}{}", s, padding)))
}

/// `lines(s)` -- split string into list of lines (no trailing empty element).
pub(crate) fn builtin_lines(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("lines", args, 0)?;
    if s.is_empty() {
        return Ok(Value::List(vec![]));
    }
    let mut items: Vec<Value> = s
        .split('\n')
        .map(|line| {
            // Handle \r\n: strip trailing \r from each line
            let trimmed = line.strip_suffix('\r').unwrap_or(line);
            Value::String(trimmed.to_string())
        })
        .collect();
    // Remove trailing empty element caused by trailing newline
    if s.ends_with('\n')
        && items
            .last()
            .is_some_and(|v| matches!(v, Value::String(s) if s.is_empty()))
    {
        items.pop();
    }
    Ok(Value::List(items))
}

/// `words(s)` -- split string into list of words by whitespace.
pub(crate) fn builtin_words(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("words", args, 0)?;
    let items: Vec<Value> = s
        .split_whitespace()
        .map(|w| Value::String(w.to_string()))
        .collect();
    Ok(Value::List(items))
}

// -- P2: TOON encoding -------------------------------------------------------

/// Check if a string is a "simple" identifier (no quoting needed in TOON).
fn toon_is_simple(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
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

/// `toon_encode(value)` -- encode any value to TOON (Token-Optimized Object Notation).
pub(crate) fn builtin_toon_encode(args: &[Value]) -> Result<Value, String> {
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
            // s"..." -- quoted string
            if *pos + 1 >= bytes.len() || bytes[*pos + 1] != b'"' {
                return Err(format!(
                    "toon_decode: expected s\"...\" at position {}",
                    pos
                ));
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
                            b'"' => {
                                result.push('"');
                                *pos += 1;
                            }
                            b'\\' => {
                                result.push('\\');
                                *pos += 1;
                            }
                            b'n' => {
                                result.push('\n');
                                *pos += 1;
                            }
                            b't' => {
                                result.push('\t');
                                *pos += 1;
                            }
                            b'r' => {
                                result.push('\r');
                                *pos += 1;
                            }
                            b'u' => {
                                // \u{XXXX}
                                *pos += 1;
                                if *pos >= bytes.len() || bytes[*pos] != b'{' {
                                    return Err(format!(
                                        "toon_decode: expected {{ after \\u at position {}",
                                        pos
                                    ));
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
                                let code_point = u32::from_str_radix(hex_str, 16).map_err(|e| {
                                    format!("toon_decode: invalid unicode escape: {}", e)
                                })?;
                                if let Some(c) = char::from_u32(code_point) {
                                    result.push(c);
                                } else {
                                    return Err(format!(
                                        "toon_decode: invalid unicode code point: {:x}",
                                        code_point
                                    ));
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
                let key =
                    if bytes[*pos] == b's' && *pos + 1 < bytes.len() && bytes[*pos + 1] == b'"' {
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
                        if *pos < bytes.len() {
                            *pos += 1;
                        } // skip closing "
                        k
                    } else {
                        // bare identifier
                        let start = *pos;
                        while *pos < bytes.len()
                            && (bytes[*pos].is_ascii_alphanumeric()
                                || bytes[*pos] == b'_'
                                || bytes[*pos] == b'-')
                        {
                            *pos += 1;
                        }
                        input[start..*pos].to_string()
                    };
                // Expect ':'
                if *pos >= bytes.len() || bytes[*pos] != b':' {
                    return Err(format!(
                        "toon_decode: expected ':' after key '{}' at position {}",
                        key, pos
                    ));
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
            Ok(Value::Struct {
                type_name: "TOON".to_string(),
                fields,
            })
        }
        b'-' | b'0'..=b'9' => {
            // Number
            let start = *pos;
            if bytes[*pos] == b'-' {
                *pos += 1;
            }
            while *pos < bytes.len() && (bytes[*pos].is_ascii_digit() || bytes[*pos] == b'.') {
                *pos += 1;
            }
            let num_str = &input[start..*pos];
            let f: f64 = num_str.parse().map_err(|e| {
                format!(
                    "toon_decode: invalid number '{}' at position {}: {}",
                    num_str, start, e
                )
            })?;
            Ok(Value::Float(f))
        }
        other => Err(format!(
            "toon_decode: unexpected character '{}' at position {}",
            other as char, pos
        )),
    }
}

/// `toon_decode(s)` -- decode TOON string back to Value.
pub(crate) fn builtin_toon_decode(args: &[Value]) -> Result<Value, String> {
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
        return Err(format!(
            "toon_decode: unexpected trailing data at position {}",
            5 + pos
        ));
    }
    Ok(value)
}
