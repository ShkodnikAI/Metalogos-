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

/// `__trim(s)` — std-library primitive behind the std/string `trim` wrapper:
/// strips leading and trailing whitespace. The `__` prefix marks a primitive
/// used by `std/*.mlog` pattern wrappers (prefer the wrapper in user code).
pub(crate) fn builtin_trim(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("__trim", args, 0)?;
    Ok(Value::String(s.trim().to_string()))
}

// -- НАРЯД №117: additional string builtins ------------------------------
// All seven are Unicode-correct (operate on chars, not bytes),
// consistent with the existing len/substring/char_at precedent.

/// `trim_start(s)` — strips leading whitespace (Unicode-aware).
pub(crate) fn builtin_trim_start(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("trim_start", args, 0)?;
    Ok(Value::String(s.trim_start().to_string()))
}

/// `trim_end(s)` — strips trailing whitespace (Unicode-aware).
pub(crate) fn builtin_trim_end(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("trim_end", args, 0)?;
    Ok(Value::String(s.trim_end().to_string()))
}

/// `truncate(s, max_len)` — cuts the string to at most `max_len` characters
/// (char-wise) appending an ellipsis `…` when truncation happens;
/// `max_len` 0 yields the empty string.
pub(crate) fn builtin_truncate(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("truncate", args, 0)?;
    let max_len = expect_float_arg("truncate", args, 1)? as usize;
    // If max_len is 0, return empty (even though ellipsis is 1 char).
    if max_len == 0 {
        return Ok(Value::String(String::new()));
    }
    let char_count = s.chars().count();
    if char_count <= max_len {
        return Ok(Value::String(s));
    }
    // Reserve 1 char for ellipsis; if max_len < 1 this is handled above.
    let cut = max_len.saturating_sub(1);
    let truncated: String = s.chars().take(cut).collect();
    Ok(Value::String(format!("{}\u{2026}", truncated)))
}

/// `slugify(s)` — URL-safe slug: lowercase, non-alphanumerics collapsed to
/// single hyphens, leading/trailing hyphens trimmed.
pub(crate) fn builtin_slugify(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("slugify", args, 0)?;
    let mut result = String::with_capacity(s.len());
    let mut prev_was_dash = false;
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            result.push(ch.to_ascii_lowercase());
            prev_was_dash = false;
        } else if (ch == '-' || ch == '_' || ch.is_whitespace()) && !prev_was_dash {
            result.push('-');
            prev_was_dash = true;
        }
        // Non-ASCII letters (Cyrillic, CJK, accented Latin) are dropped.
        // Decision: transliteration is locale-dependent and error-prone;
        // dropping is deterministic and documented.
    }
    // Trim trailing dash if present
    let trimmed = result.trim_end_matches('-');
    Ok(Value::String(trimmed.to_string()))
}

/// `word_wrap(s, width)` — reflows text to `width` columns without breaking
/// words; errors on width 0.
pub(crate) fn builtin_word_wrap(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("word_wrap", args, 0)?;
    let width = expect_float_arg("word_wrap", args, 1)? as usize;
    if width == 0 {
        return Err("word_wrap: width must be > 0".to_string());
    }
    // Unicode-correct: count chars, not bytes. No unicode-width crate,
    // consistent with existing len/substring precedent.
    let mut result = String::with_capacity(s.len() + s.len() / 10);
    let mut line_len: usize = 0;
    for word in s.split_whitespace() {
        let word_char_len = word.chars().count();
        if line_len == 0 {
            // First word on line
            result.push_str(word);
            line_len = word_char_len;
        } else if line_len + 1 + word_char_len <= width {
            result.push(' ');
            result.push_str(word);
            line_len += 1 + word_char_len;
        } else {
            // Word doesn't fit — start new line
            result.push('\n');
            result.push_str(word);
            line_len = word_char_len;
        }
    }
    Ok(Value::String(result))
}

/// `capitalize(s)` — uppercases the first character and lowercases the rest.
pub(crate) fn builtin_capitalize(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("capitalize", args, 0)?;
    let mut chars = s.chars();
    match chars.next() {
        None => Ok(Value::String(String::new())),
        Some(first) => {
            let mut result = String::with_capacity(s.len());
            for ch in first.to_uppercase() {
                result.push(ch);
            }
            result.extend(chars);
            Ok(Value::String(result))
        }
    }
}

/// `title_case(s)` — uppercases the first character of every word
/// (previous character non-letter acts as the word boundary).
pub(crate) fn builtin_title_case(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("title_case", args, 0)?;
    let mut result = String::with_capacity(s.len());
    let mut prev_was_space = true; // capitalize first character
    for ch in s.chars() {
        if ch.is_whitespace() {
            result.push(ch);
            prev_was_space = true;
        } else if prev_was_space {
            for upper in ch.to_uppercase() {
                result.push(upper);
            }
            prev_was_space = false;
        } else {
            result.push(ch);
        }
    }
    Ok(Value::String(result))
}

/// `__replace(s, from, to)` — std-library primitive behind the std/string
/// `replace` wrapper: replaces every occurrence of `from` with `to`.
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

/// `__split(s, sep)` — std-library primitive behind the std/string `split`
/// wrapper: splits on the separator into a List of strings.
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

/// `__join(list, sep)` — std-library primitive behind the std/string `join`
/// wrapper: joins a List of strings with the separator.
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
/// Naryad #277 (proptest no-panic): the ends are counted independently, so
/// when the two strips overlap (string fully made of strip-chars, e.g.
/// `strip("&", "Ⱥ&")`), `start > len - end` and the slice PANICKED. The
/// correct contract (same as `str::trim_matches` with a set): both ends
/// consuming the whole string yields the empty string.
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
    let total = s_chars.len();
    let end_idx = total.saturating_sub(end);
    if start >= end_idx {
        // Both ends met (or crossed) — everything was stripped.
        return Ok(Value::String(String::new()));
    }
    let trimmed: String = s_chars[start..end_idx].iter().collect();
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

// ── Наряд №274 (ADR-0136): redact(text, mode) — PII/секреты как taint-санитайзер ──
//
// `redact` — единственный легальный путь снять Secret-taint ДО sink'а
// («mask before sink»). Маски детерминированные (одинаковый вход → одинаковая
// маска) и сохраняют ТИП маски + последние 4 символа, чтобы логи оставались
// диагностируемыми. Энтропийная сеть (base64/hex-прогоны) — страховка против
// форматов вне паттерн-набора; её остаточный риск честно зафиксирован в
// ADR-0136. Регулярные выражения — линейный движок `regex` (наряд №54),
// ReDoS-риска паттерны не создают.

use std::sync::OnceLock;

use regex::Regex;

pub(crate) fn builtin_redact(args: &[Value]) -> Result<Value, String> {
    // Accept both String and Secret — masking a secret in place is the
    // whole point of the builtin (the ADR-0136 "mask before sink" path).
    let text = match args.first() {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Secret(zs)) => zs.as_str().to_string(),
        Some(other) => {
            return Err(format!(
                "redact() expected String or Secret as first arg, got {}",
                other.type_name()
            ))
        }
        None => return Err("redact() requires 2 arguments: redact(text, mode)".to_string()),
    };
    let mode = expect_string_arg("redact", args, 1)?;
    redact_string(&text, &mode).map(Value::String)
}

/// Core masking routine (public for the fuzz target, №274).
/// mode: "pii" | "secrets" | "all" — anything else is a loud error.
pub fn redact_string(text: &str, mode: &str) -> Result<String, String> {
    match mode {
        "pii" | "secrets" | "all" => {}
        other => {
            return Err(format!(
                "redact() unknown mode \"{}\" — expected \"pii\", \"secrets\" or \"all\"",
                other
            ))
        }
    }
    let do_secrets = mode != "pii";
    let do_pii = mode != "secrets";
    let mut out = text.to_string();
    if do_secrets {
        out = redact_secret_patterns(&out);
    }
    if do_pii {
        out = redact_pii_patterns(&out);
    }
    // The entropy net runs LAST (secrets/all only): it catches high-entropy
    // runs that escaped the explicit pattern set. Order matters — masks
    // produced above never re-trigger it (see ADR-0136 idempotence tests).
    if do_secrets {
        out = redact_entropy_net(&out);
    }
    Ok(out)
}

/// Deterministic mask: `[REDACTED:<type>…<last4>]` — keeps logs diagnosable.
fn mask_typed(kind: &str, matched: &str) -> String {
    let tail: String = matched
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("[REDACTED:{}\u{2026}{}]", kind, tail)
}

/// Static pattern — Regex::new failure is impossible at compile time.
#[allow(clippy::expect_used)]
fn re_pem() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?s)-----BEGIN [A-Z0-9 ]{2,40}-----.*?-----END [A-Z0-9 ]{2,40}-----")
            .expect("static regex")
    })
}

/// Static pattern — Regex::new failure is impossible at compile time.
#[allow(clippy::expect_used)]
fn re_jwt() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"eyJ[A-Za-z0-9_-]{6,}\.[A-Za-z0-9_-]{6,}\.[A-Za-z0-9_-]{6,}")
            .expect("static regex")
    })
}

/// Static pattern — Regex::new failure is impossible at compile time.
#[allow(clippy::expect_used)]
fn re_bearer() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i:bearer)[ \t]+[A-Za-z0-9._~+/=-]{8,}").expect("static regex"))
}

/// Static pattern — Regex::new failure is impossible at compile time.
#[allow(clippy::expect_used)]
fn re_apikey() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // OpenAI/Anthropic-style (sk-…), AWS access key id (AKIA…), GitHub tokens (ghp_/gho_/ghu_/ghs_/ghr_).
    RE.get_or_init(|| {
        Regex::new(r"(sk-[A-Za-z0-9_-]{16,}|AKIA[0-9A-Z]{16}|gh[pousr]_[A-Za-z0-9]{20,})")
            .expect("static regex")
    })
}

/// Static pattern — Regex::new failure is impossible at compile time.
#[allow(clippy::expect_used)]
fn re_hex_run() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[0-9a-fA-F]{32,}").expect("static regex"))
}

/// Static pattern — Regex::new failure is impossible at compile time.
#[allow(clippy::expect_used)]
fn re_b64_run() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[A-Za-z0-9+/]{24,}={0,2}").expect("static regex"))
}

/// Static pattern — Regex::new failure is impossible at compile time.
#[allow(clippy::expect_used)]
fn re_email() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}")
            .expect("static regex")
    })
}

/// Static pattern — Regex::new failure is impossible at compile time.
#[allow(clippy::expect_used)]
fn re_phone_intl() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\+\d{1,3}[ \-(]{0,2}\d{2,4}[ \-)]{0,2}\d{2,4}(?:[ \-]\d{2}){1,2}")
            .expect("static regex")
    })
}

/// Static pattern — Regex::new failure is impossible at compile time.
#[allow(clippy::expect_used)]
fn re_phone_ru() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\b8[ \-(]{0,2}\d{3}[ \-)]{0,2}\d{3}[ \-]\d{2}[ \-]\d{2}\b")
            .expect("static regex")
    })
}

/// Static pattern — Regex::new failure is impossible at compile time.
#[allow(clippy::expect_used)]
fn re_card() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b(?:\d[ \-]?){12,18}\d\b").expect("static regex"))
}

/// Static pattern — Regex::new failure is impossible at compile time.
#[allow(clippy::expect_used)]
fn re_iban() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // BBAN tail may be shorter than a full 4-char group (e.g. 22-char DE IBAN).
    RE.get_or_init(|| {
        Regex::new(r"\b[A-Z]{2}\d{2}(?:[ ]?[A-Z0-9]{4}){2,7}[ ]?[A-Z0-9]{0,3}\b")
            .expect("static regex")
    })
}

/// Secret pattern set (modes "secrets" | "all").
fn redact_secret_patterns(text: &str) -> String {
    let mut out = re_pem()
        .replace_all(text, "[REDACTED:pem-block]")
        .into_owned();
    out = re_jwt()
        .replace_all(&out, |caps: &regex::Captures| mask_typed("jwt", &caps[0]))
        .into_owned();
    out = re_bearer()
        .replace_all(&out, |caps: &regex::Captures| {
            mask_typed("bearer", &caps[0])
        })
        .into_owned();
    out = re_apikey()
        .replace_all(&out, |caps: &regex::Captures| {
            let m = &caps[0];
            let kind = if m.starts_with("AKIA") {
                "AKIA"
            } else if m.starts_with("sk-") {
                "sk-"
            } else {
                // ghp_ / gho_ / ghu_ / ghs_ / ghr_ — first 4 chars are the type
                &m[..4]
            };
            mask_typed(kind, m)
        })
        .into_owned();
    out
}

/// PII pattern set (modes "pii" | "all").
fn redact_pii_patterns(text: &str) -> String {
    let mut out = re_email()
        .replace_all(text, |caps: &regex::Captures| {
            // Preserve the TLD for diagnosability: ***@***.io
            let m = &caps[0];
            let tld = m.rsplit('.').next().unwrap_or("tld");
            format!("***@***.{}", tld)
        })
        .into_owned();
    out = re_phone_intl()
        .replace_all(&out, |caps: &regex::Captures| {
            let digits = caps[0].chars().filter(|c| c.is_ascii_digit()).count();
            if (7..=15).contains(&digits) {
                "[REDACTED:phone]".to_string()
            } else {
                caps[0].to_string()
            }
        })
        .into_owned();
    out = re_phone_ru()
        .replace_all(&out, "[REDACTED:phone]")
        .into_owned();
    out = re_iban()
        .replace_all(&out, |caps: &regex::Captures| {
            let compact: String = caps[0].chars().filter(|c| !c.is_whitespace()).collect();
            if (15..=34).contains(&compact.len()) {
                mask_typed("iban", &compact)
            } else {
                caps[0].to_string()
            }
        })
        .into_owned();
    out = re_card()
        .replace_all(&out, |caps: &regex::Captures| {
            let digits: String = caps[0].chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() >= 13 && digits.len() <= 19 && luhn_valid(&digits) {
                let vendor = card_vendor(&digits);
                mask_typed(&format!("{}-card", vendor), &digits)
            } else {
                caps[0].to_string()
            }
        })
        .into_owned();
    out
}

/// Entropy net (modes "secrets" | "all"): base64/hex runs that slipped past
/// the explicit pattern set. Filter (documented in ADR-0136): a run is
/// masked only if it contains BOTH a digit AND at least one hex letter
/// ([a-fA-F]) — long identifiers, pure-digit ids and letter-only words pass.
fn redact_entropy_net(text: &str) -> String {
    let hit = |s: &str| {
        let has_digit = s.chars().any(|c| c.is_ascii_digit());
        let has_hex = s.chars().any(|c| matches!(c, 'a'..='f' | 'A'..='F'));
        has_digit && has_hex
    };
    let mut out = re_hex_run()
        .replace_all(text, |caps: &regex::Captures| {
            if hit(&caps[0]) {
                mask_typed("entropy", &caps[0])
            } else {
                caps[0].to_string()
            }
        })
        .into_owned();
    out = re_b64_run()
        .replace_all(&out, |caps: &regex::Captures| {
            if hit(&caps[0]) {
                mask_typed("entropy", &caps[0])
            } else {
                caps[0].to_string()
            }
        })
        .into_owned();
    out
}

/// Luhn checksum (card masking gate, №274): reduces false positives on
/// long digit runs that are not payment cards.
fn luhn_valid(digits: &str) -> bool {
    let sum = digits
        .chars()
        .rev()
        .filter_map(|c| c.to_digit(10))
        .enumerate()
        .map(|(i, d)| if i % 2 == 1 { d * 2 } else { d })
        .map(|d| if d > 9 { d - 9 } else { d })
        .sum::<u32>();
    sum % 10 == 0
}

/// Card vendor by prefix (deterministic, for the mask label).
fn card_vendor(digits: &str) -> &'static str {
    let p2: u16 = digits.get(..2).and_then(|s| s.parse().ok()).unwrap_or(0);
    let first = digits.chars().next().unwrap_or('0');
    match first {
        '4' => "visa",
        '3' if p2 == 34 || p2 == 37 => "amex",
        '5' if (51..=55).contains(&p2) => "mastercard",
        '2' if (22..=27).contains(&p2) => "mastercard",
        '6' if p2 == 62 => "unionpay",
        '6' if p2 == 60 || p2 == 64 || p2 == 65 => "discover",
        _ => "card",
    }
}
