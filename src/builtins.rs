// ── Built-in functions for METALOGOS M1+M2 ────────────────────────────

use crate::interpreter::{Value, SecretString};

pub type BuiltinFn = fn(&[Value]) -> Result<Value, String>;

/// Registry of built-in functions.
pub struct Builtins {
    funcs: std::collections::HashMap<String, BuiltinFn>,
}

impl Builtins {
    pub fn new() -> Self {
        let mut funcs = std::collections::HashMap::new();

        funcs.insert("upper".to_string(), builtin_upper as BuiltinFn);
        funcs.insert("lower".to_string(), builtin_lower as BuiltinFn);
        funcs.insert("len".to_string(), builtin_len as BuiltinFn);
        funcs.insert("str".to_string(), builtin_str as BuiltinFn);
        funcs.insert("print".to_string(), builtin_print as BuiltinFn);
        funcs.insert("contains".to_string(), builtin_contains as BuiltinFn);
        funcs.insert("float".to_string(), builtin_float as BuiltinFn);
        funcs.insert("to_string".to_string(), builtin_to_string as BuiltinFn);
        funcs.insert("get".to_string(), builtin_get as BuiltinFn);
        funcs.insert("push".to_string(), builtin_push as BuiltinFn);

        // Phase 7 — environment variable access
        funcs.insert("env".to_string(), builtin_env as BuiltinFn);

        // Phase 5.3 — string operations
        funcs.insert("index_of".to_string(), builtin_index_of as BuiltinFn);
        funcs.insert("substring".to_string(), builtin_substring as BuiltinFn);
        funcs.insert("char_at".to_string(), builtin_char_at as BuiltinFn);
        funcs.insert("starts_with".to_string(), builtin_starts_with as BuiltinFn);
        funcs.insert("ends_with".to_string(), builtin_ends_with as BuiltinFn);
        funcs.insert("to_float".to_string(), builtin_to_float as BuiltinFn);

        // Fluid confidence accessor
        funcs.insert("confidence".to_string(), builtin_confidence as BuiltinFn);

        // Phase 5.4 — stdlib backing builtins (double-underscore prefix)
        funcs.insert("__trim".to_string(), builtin_trim as BuiltinFn);
        funcs.insert("__replace".to_string(), builtin_replace as BuiltinFn);
        funcs.insert("__split".to_string(), builtin_split as BuiltinFn);
        funcs.insert("__join".to_string(), builtin_join as BuiltinFn);
        funcs.insert("__abs".to_string(), builtin_abs as BuiltinFn);
        funcs.insert("__min".to_string(), builtin_min as BuiltinFn);
        funcs.insert("__max".to_string(), builtin_max as BuiltinFn);
        funcs.insert("__clamp".to_string(), builtin_clamp as BuiltinFn);
        funcs.insert("__round".to_string(), builtin_round as BuiltinFn);
        funcs.insert("__first".to_string(), builtin_first as BuiltinFn);
        funcs.insert("__last".to_string(), builtin_last as BuiltinFn);

        // Phase 6.1 — HTTP server stubs
        funcs.insert("respond".to_string(), builtin_respond as BuiltinFn);
        funcs.insert("respond_html".to_string(), builtin_respond_html as BuiltinFn);
        funcs.insert("form_data".to_string(), builtin_form_data as BuiltinFn);
        funcs.insert("json_body".to_string(), builtin_json_body as BuiltinFn);
        funcs.insert("query_param".to_string(), builtin_query_param as BuiltinFn);

        // Phase 6.2 — Template stubs
        funcs.insert("render".to_string(), builtin_render as BuiltinFn);
        funcs.insert("escape_html".to_string(), builtin_escape_html as BuiltinFn);

        // Phase 6.3 — Database stubs
        funcs.insert("query".to_string(), builtin_query as BuiltinFn);
        funcs.insert("db_execute".to_string(), builtin_db_execute as BuiltinFn);

        // Phase 6.4 — Encryption stubs
        funcs.insert("env".to_string(), builtin_env as BuiltinFn);
        funcs.insert("hash_password".to_string(), builtin_hash_password as BuiltinFn);
        funcs.insert("verify_password".to_string(), builtin_verify_password as BuiltinFn);
        funcs.insert("encrypt".to_string(), builtin_encrypt as BuiltinFn);
        funcs.insert("decrypt".to_string(), builtin_decrypt as BuiltinFn);
        funcs.insert("generate_key".to_string(), builtin_generate_key as BuiltinFn);

        // Phase 6.5 — Auth stubs
        funcs.insert("authenticate".to_string(), builtin_authenticate as BuiltinFn);
        funcs.insert("session_login".to_string(), builtin_session_login as BuiltinFn);
        funcs.insert("session_logout".to_string(), builtin_session_logout as BuiltinFn);

        // Phase 6.6 — Bot stubs
        funcs.insert("send_message".to_string(), builtin_send_message as BuiltinFn);
        funcs.insert("require".to_string(), builtin_require as BuiltinFn);

        // Definition of Done — outgoing HTTP
        funcs.insert("http_post".to_string(), builtin_http_post as BuiltinFn);

        // v0.5.0 — top-level string builtins (aliases for __* + new)
        funcs.insert("trim".to_string(), builtin_trim as BuiltinFn);
        funcs.insert("replace".to_string(), builtin_replace as BuiltinFn);
        funcs.insert("split".to_string(), builtin_split as BuiltinFn);
        funcs.insert("join".to_string(), builtin_join as BuiltinFn);
        funcs.insert("length".to_string(), builtin_length as BuiltinFn);
        funcs.insert("to_int".to_string(), builtin_to_int as BuiltinFn);
        funcs.insert("reverse".to_string(), builtin_reverse as BuiltinFn);

        // v0.5.0 — LLM call builtin
        funcs.insert("call_llm".to_string(), builtin_call_llm as BuiltinFn);

        // v0.5.0 — KV memory builtins
        funcs.insert("kv_set".to_string(), builtin_kv_set as BuiltinFn);
        funcs.insert("kv_get".to_string(), builtin_kv_get as BuiltinFn);
        funcs.insert("kv_delete".to_string(), builtin_kv_delete as BuiltinFn);
        funcs.insert("kv_exists".to_string(), builtin_kv_exists as BuiltinFn);
        funcs.insert("kv_list".to_string(), builtin_kv_list as BuiltinFn);

        // Наряд №6 — exact key-value memory (mem_set/mem_get/mem_delete)
        funcs.insert("mem_set".to_string(), builtin_mem_set as BuiltinFn);
        funcs.insert("mem_get".to_string(), builtin_mem_get as BuiltinFn);
        funcs.insert("mem_delete".to_string(), builtin_mem_delete as BuiltinFn);

        // v0.5.0 — File I/O builtins (full set)
        funcs.insert("read_file".to_string(), builtin_read_file as BuiltinFn);
        funcs.insert("write_file".to_string(), builtin_write_file as BuiltinFn);
        funcs.insert("append_file".to_string(), builtin_append_file as BuiltinFn);
        funcs.insert("delete_file".to_string(), builtin_delete_file as BuiltinFn);
        funcs.insert("file_exists".to_string(), builtin_file_exists as BuiltinFn);
        funcs.insert("list_dir".to_string(), builtin_list_dir as BuiltinFn);

        // Anthropic Claude LLM integration (Phase 7.7)
        funcs.insert("call_claude".to_string(), builtin_call_claude as BuiltinFn);

        // Наряд №4: LLM usage tracking
        funcs.insert("llm_usage".to_string(), builtin_llm_usage as BuiltinFn);

        // JSON escape utility (Phase 7.7)
        funcs.insert("escape_json".to_string(), builtin_escape_json as BuiltinFn);

        // Phase 7.7 — new builtins for department modularity
        funcs.insert("parse_json".to_string(), builtin_parse_json as BuiltinFn);
        funcs.insert("json_encode".to_string(), builtin_json_encode as BuiltinFn);
        funcs.insert("http_get".to_string(), builtin_http_get as BuiltinFn);
        funcs.insert("now".to_string(), builtin_now as BuiltinFn);

        // ADR-0049 — session memory (temporary per-session KV store)
        funcs.insert("session_set".to_string(), builtin_session_set as BuiltinFn);
        funcs.insert("session_get".to_string(), builtin_session_get as BuiltinFn);
        funcs.insert("session_clear".to_string(), builtin_session_clear as BuiltinFn);

        Builtins { funcs }
    }

    /// Look up a built-in by name.
    pub fn get(&self, name: &str) -> Option<&BuiltinFn> {
        self.funcs.get(name)
    }
}

fn builtin_upper(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("upper", args, 0)?;
    Ok(Value::String(s.to_uppercase()))
}

fn builtin_lower(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("lower", args, 0)?;
    Ok(Value::String(s.to_lowercase()))
}

fn builtin_len(args: &[Value]) -> Result<Value, String> {
    match args.get(0) {
        // Unicode-aware: chars().count() returns character count, not byte count.
        // "Привет" (6 chars, 12 bytes) → 6.0, not 12.0.
        Some(Value::String(s)) => Ok(Value::Float(s.chars().count() as f64)),
        Some(Value::List(items)) => Ok(Value::Float(items.len() as f64)),
        _ => Err("len() requires String or List argument".to_string()),
    }
}

fn builtin_str(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("str() requires 1 argument".to_string());
    }
    Ok(Value::String(format!("{}", args[0])))
}

fn builtin_print(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("print", args, 0)?;
    println!("{}", s);
    Ok(Value::String(s))
}

fn builtin_contains(args: &[Value]) -> Result<Value, String> {
    let haystack = expect_string_arg("contains", args, 0)?;
    let needle = expect_string_arg("contains", args, 1)?;
    let result = if haystack.contains(&needle) { 1.0 } else { 0.0 };
    Ok(Value::Float(result))
}

fn builtin_float(args: &[Value]) -> Result<Value, String> {
    match args.get(0) {
        Some(Value::Float(f)) => Ok(Value::Float(*f)),
        Some(Value::String(s)) => s.parse::<f64>()
            .map(Value::Float)
            .map_err(|_| format!("float() cannot parse '{}'", s)),
        _ => Err("float() requires 1 argument".to_string()),
    }
}

fn builtin_to_string(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("to_string() requires 1 argument".to_string());
    }
    // Use Value's Display impl — Float omits .0 for integers automatically
    Ok(Value::String(format!("{}", args[0])))
}

fn builtin_get(args: &[Value]) -> Result<Value, String> {
    let list = match args.get(0) {
        Some(Value::List(items)) => items,
        _ => return Err("get() requires List as first argument".to_string()),
    };
    let index = match args.get(1) {
        Some(Value::Float(f)) => *f as usize,
        _ => return Err("get() requires Float index as second argument".to_string()),
    };
    list.get(index).cloned().ok_or_else(|| format!(
        "get() index {} out of bounds (list has {} elements)",
        index, list.len()
    ))
}

fn builtin_push(args: &[Value]) -> Result<Value, String> {
    let list = match args.get(0) {
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

fn builtin_index_of(args: &[Value]) -> Result<Value, String> {
    let haystack = expect_string_arg("index_of", args, 0)?;
    let needle = expect_string_arg("index_of", args, 1)?;
    // Unicode-aware: return CHARACTER position, not byte offset.
    // "Привет, мир".find("мир") byte offset = 12, char offset = 8.
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

fn builtin_substring(args: &[Value]) -> Result<Value, String> {
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

fn builtin_char_at(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("char_at", args, 0)?;
    let index = expect_float_arg("char_at", args, 1)? as usize;
    // Soft-failure: return empty string on out-of-bounds
    let chars: Vec<char> = s.chars().collect();
    match chars.get(index) {
        Some(ch) => Ok(Value::String(ch.to_string())),
        None => Ok(Value::String(String::new())),
    }
}

fn builtin_starts_with(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("starts_with", args, 0)?;
    let prefix = expect_string_arg("starts_with", args, 1)?;
    Ok(Value::Bool(s.starts_with(&prefix)))
}

fn builtin_ends_with(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("ends_with", args, 0)?;
    let suffix = expect_string_arg("ends_with", args, 1)?;
    Ok(Value::Bool(s.ends_with(&suffix)))
}

fn builtin_to_float(args: &[Value]) -> Result<Value, String> {
    match args.get(0) {
        Some(Value::Float(f)) => Ok(Value::Float(*f)),
        Some(Value::String(s)) => Ok(s.parse::<f64>()
            .map(Value::Float)
            .unwrap_or(Value::Float(0.0))), // soft-failure: return 0.0 on parse error
        Some(Value::Bool(b)) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
        _ => Ok(Value::Float(0.0)), // soft-failure
    }
}

fn builtin_confidence(args: &[Value]) -> Result<Value, String> {
    match args.get(0) {
        Some(Value::Fluid(variants)) => {
            let best = variants.iter().map(|v| v.confidence)
                .fold(0.0_f64, f64::max);
            Ok(Value::Float(best))
        }
        Some(_) => Ok(Value::Float(1.0)), // concrete values are fully confident
        None => Err("confidence() requires 1 argument".to_string()),
    }
}

// builtin_env moved to Phase 6.4 section below

fn expect_float_arg(fn_name: &str, args: &[Value], index: usize) -> Result<f64, String> {
    if args.len() <= index {
        return Err(format!("{}() requires an argument at position {}", fn_name, index));
    }
    match &args[index] {
        Value::Float(f) => Ok(*f),
        other => Err(format!(
            "{}() expected Float argument, got {}",
            fn_name, other.type_name()
        )),
    }
}

fn expect_string_arg(fn_name: &str, args: &[Value], index: usize) -> Result<String, String> {
    if args.len() <= index {
        return Err(format!("{}() requires an argument at position {}", fn_name, index));
    }
    match &args[index] {
        Value::String(s) => Ok(s.clone()),
        other => Err(format!(
            "{}() expected String argument, got {}",
            fn_name, other.type_name()
        )),
    }
}

// ── Stdlib backing builtins (Phase 5.4) ───────────────────────────
// These implement the primitives used by std/*.mlog pattern wrappers.

fn builtin_trim(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("__trim", args, 0)?;
    Ok(Value::String(s.trim().to_string()))
}

fn builtin_replace(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("__replace", args, 0)?;
    let old = expect_string_arg("__replace", args, 1)?;
    let new = expect_string_arg("__replace", args, 2)?;
    Ok(Value::String(s.replace(&old, &new)))
}

fn builtin_split(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("__split", args, 0)?;
    let sep = expect_string_arg("__split", args, 1)?;
    let items: Vec<Value> = if sep.is_empty() {
        s.chars().map(|c| Value::String(c.to_string())).collect()
    } else {
        s.split(&sep).map(|part| Value::String(part.to_string())).collect()
    };
    Ok(Value::List(items))
}

fn builtin_join(args: &[Value]) -> Result<Value, String> {
    let list = match args.get(0) {
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

fn builtin_abs(args: &[Value]) -> Result<Value, String> {
    let f = expect_float_arg("__abs", args, 0)?;
    Ok(Value::Float(f.abs()))
}

fn builtin_min(args: &[Value]) -> Result<Value, String> {
    let a = expect_float_arg("__min", args, 0)?;
    let b = expect_float_arg("__min", args, 1)?;
    Ok(Value::Float(a.min(b)))
}

fn builtin_max(args: &[Value]) -> Result<Value, String> {
    let a = expect_float_arg("__max", args, 0)?;
    let b = expect_float_arg("__max", args, 1)?;
    Ok(Value::Float(a.max(b)))
}

fn builtin_clamp(args: &[Value]) -> Result<Value, String> {
    let val = expect_float_arg("__clamp", args, 0)?;
    let lo = expect_float_arg("__clamp", args, 1)?;
    let hi = expect_float_arg("__clamp", args, 2)?;
    Ok(Value::Float(val.clamp(lo, hi)))
}

fn builtin_round(args: &[Value]) -> Result<Value, String> {
    let f = expect_float_arg("__round", args, 0)?;
    Ok(Value::Float(f.round()))
}

fn builtin_first(args: &[Value]) -> Result<Value, String> {
    let list = match args.get(0) {
        Some(Value::List(items)) => items,
        _ => return Err("__first() requires List as first argument".to_string()),
    };
    match list.first() {
        Some(v) => Ok(v.clone()),
        None => Ok(Value::String(String::new())), // soft-failure
    }
}

fn builtin_last(args: &[Value]) -> Result<Value, String> {
    let list = match args.get(0) {
        Some(Value::List(items)) => items,
        _ => return Err("__last() requires List as first argument".to_string()),
    };
    match list.last() {
        Some(v) => Ok(v.clone()),
        None => Ok(Value::String(String::new())), // soft-failure
    }
}

// ── Phase 6.1 — HTTP server stubs ───────────────────────────
// In interpreter-only mode (mlog run), these return mock values.
// Real implementations live in server.rs for the Axum context.

fn builtin_respond(args: &[Value]) -> Result<Value, String> {
    let status_body = expect_string_arg("respond", args, 0)?;
    // Parse "200 OK" → status 200, body "OK"
    let (status, body) = parse_status_line(&status_body);
    Ok(Value::HttpResponse { status, body })
}

/// respond_html(status, html) — respond with HTML content.
/// In server context, value_to_response converts HttpResponse to Axum response.
/// The Html variant would auto-set Content-Type, but FOSVED uses respond_html("200", ...)
/// with return, so HttpResponse is the correct type here — the server sets Content-Type.
fn builtin_respond_html(args: &[Value]) -> Result<Value, String> {
    let status_str = expect_string_arg("respond_html", args, 0)?;
    let html = expect_string_arg("respond_html", args, 1)?;
    let (status, _) = parse_status_line(&status_str);
    Ok(Value::HttpResponse { status, body: html })
}

fn builtin_form_data(args: &[Value]) -> Result<Value, String> {
    let _ = args; // no args needed
    // In non-server context, return empty form data struct
    Ok(Value::Struct {
        type_name: "FormData".to_string(),
        fields: std::collections::HashMap::new(),
    })
}

fn builtin_json_body(args: &[Value]) -> Result<Value, String> {
    let _ = args; // no args needed
    // In non-server context, return empty json body struct
    Ok(Value::Struct {
        type_name: "JsonBody".to_string(),
        fields: std::collections::HashMap::new(),
    })
}

/// query_param(name) — stub that returns empty string.
/// Real implementation is handled in interpreter.rs FnCall dispatch
/// (needs access to server_query_params HashMap on the interpreter).
fn builtin_query_param(args: &[Value]) -> Result<Value, String> {
    let _name = if args.is_empty() {
        return Err("query_param() requires 1 argument (param name)".to_string());
    } else {
        match &args[0] {
            Value::String(s) => s.clone(),
            other => return Err(format!("query_param() expected String, got {}", other.type_name())),
        }
    };
    // Stub — real implementation is special-cased in interpreter FnCall dispatch
    Ok(Value::String(String::new()))
}

// ── Phase 6.2 — Template stubs ───────────────────────────

fn builtin_render(args: &[Value]) -> Result<Value, String> {
    // render(template_name, key1, val1, key2, val2, ...)
    // Simple {{ var }} substitution with auto-escaping
    // In interpreter mode, do basic string substitution
    if args.len() < 3 || (args.len() - 1) % 2 != 0 {
        return Err("render() requires template name + key/value pairs (odd count)".to_string());
    }
    let template_name = expect_string_arg("render", args, 0)?;

    // Build substitution map from remaining args (key, value pairs)
    let mut vars = std::collections::HashMap::new();
    let mut i = 1;
    while i + 1 < args.len() {
        let key = match &args[i] {
            Value::String(s) => s.clone(),
            other => return Err(format!("render() key must be String, got {}", other.type_name())),
        };
        let val = match &args[i + 1] {
            Value::String(s) => s.clone(),
            other => format!("{}", other),
        };
        vars.insert(key, val);
        i += 2;
    }

    // In interpreter mode, generate a simple HTML string from the template name and vars
    let mut html = String::from("<div class=\"template-");
    html.push_str(&escape_html_chars(&template_name));
    html.push_str("\">");
    for (key, val) in &vars {
        html.push_str(&format!("<span data-key=\"{}\">{}</span>",
            escape_html_chars(key), escape_html_chars(val)));
    }
    html.push_str("</div>");

    Ok(Value::Html(html))
}

fn builtin_escape_html(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("escape_html", args, 0)?;
    Ok(Value::String(escape_html_chars(&s)))
}

/// HTML-escape a string (for use in templates and escape_html builtin).
fn escape_html_chars(s: &str) -> String {
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

/// Parse a status line like "200 OK" into (status_code, body).
fn parse_status_line(status_body: &str) -> (u16, String) {
    let parts: Vec<&str> = status_body.splitn(2, ' ').collect();
    let status = parts.first().and_then(|s| s.parse::<u16>().ok()).unwrap_or(200);
    let body = if parts.len() > 1 { parts[1].to_string() } else { String::new() };
    (status, body)
}

// ── Phase 6.3 — Database stubs ───────────────────────────

fn builtin_query(args: &[Value]) -> Result<Value, String> {
    let sql = expect_string_arg("query", args, 0)?;
    // Wrap SQL in opaque Query value — prevents string concatenation or printing
    // In interpreter mode, store the SQL for later mock execution
    let _params = if args.len() > 1 { &args[1] } else { &Value::Unit };
    Ok(Value::Query(sql))
}

fn builtin_db_execute(args: &[Value]) -> Result<Value, String> {
    let _sql = expect_string_arg("db_execute", args, 0)?;
    // In interpreter mode, no-op (returns Unit)
    Ok(Value::Unit)
}

// ── Phase 6.4 — Encryption stubs ───────────────────────────

fn builtin_env(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("env", args, 0)?;
    match std::env::var(&key) {
        Ok(val) => Ok(Value::String(val)),
        Err(_) => Ok(Value::String(String::new())), // soft-failure: empty string if not found
    }
}

fn builtin_hash_password(args: &[Value]) -> Result<Value, String> {
    let password = expect_string_arg("hash_password", args, 0)?;
    // Argon2id with random salt — real password hashing (Phase 7.3)
    use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
    use rand::rngs::OsRng;

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default(); // Argon2id
    match argon2.hash_password(password.as_bytes(), &salt) {
        Ok(hash) => Ok(Value::Hash(hash.to_string())),
        Err(e) => Err(format!("hash_password() failed: {}", e)),
    }
}

fn builtin_verify_password(args: &[Value]) -> Result<Value, String> {
    let password = expect_string_arg("verify_password", args, 0)?;
    let hash_str = match args.get(1) {
        Some(Value::Hash(h)) => h.as_str(),
        Some(other) => return Err(format!("verify_password() expected Hash as second arg, got {}", other.type_name())),
        None => return Err("verify_password() requires 2 arguments".to_string()),
    };
    // Real Argon2id verification with constant-time comparison (Phase 7.3)
    use argon2::{Argon2, PasswordVerifier, password_hash::PasswordHash};

    let argon2 = Argon2::default();
    match PasswordHash::new(hash_str) {
        Ok(parsed_hash) => {
            // Constant-time comparison inside argon2
            match argon2.verify_password(password.as_bytes(), &parsed_hash) {
                Ok(_) => Ok(Value::Bool(true)),
                Err(argon2::password_hash::Error::Password) => Ok(Value::Bool(false)),
                Err(e) => Err(format!("verify_password() failed: {}", e)),
            }
        }
        Err(e) => Err(format!("verify_password() invalid hash format: {}", e)),
    }
}

fn builtin_encrypt(args: &[Value]) -> Result<Value, String> {
    let data = expect_string_arg("encrypt", args, 0)?;
    let key_str = match args.get(1) {
        Some(Value::Secret(zs)) => zs.as_str(),
        Some(other) => return Err(format!("encrypt() expected Secret as second arg, got {}", other.type_name())),
        None => return Err("encrypt() requires 2 arguments".to_string()),
    };
    // Real AES-256-GCM with random 96-bit nonce (Phase 7.3)
    use aes_gcm::{Aes256Gcm, AeadCore, Key};
    use aes_gcm::aead::{Aead, KeyInit, OsRng};

    let key_bytes = hex::decode(key_str)
        .map_err(|e| format!("encrypt() invalid key format (expected hex): {}", e))?;
    if key_bytes.len() != 32 {
        return Err(format!("encrypt() key must be 256-bit (64 hex chars), got {} bytes", key_bytes.len()));
    }
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bit random nonce

    match cipher.encrypt(&nonce, data.as_ref()) {
        Ok(ciphertext) => {
            // Prepend nonce to ciphertext for self-contained Encrypted value
            let mut output = nonce.to_vec();
            output.extend_from_slice(&ciphertext);
            Ok(Value::Encrypted(output))
        }
        Err(e) => Err(format!("encrypt() AES-256-GCM encryption failed: {}", e)),
    }
}

fn builtin_decrypt(args: &[Value]) -> Result<Value, String> {
    let encrypted = match args.get(0) {
        Some(Value::Encrypted(data)) => data.clone(),
        Some(other) => return Err(format!("decrypt() expected Encrypted as first arg, got {}", other.type_name())),
        None => return Err("decrypt() requires 2 arguments".to_string()),
    };
    let key_str = match args.get(1) {
        Some(Value::Secret(zs)) => zs.as_str(),
        Some(other) => return Err(format!("decrypt() expected Secret as second arg, got {}", other.type_name())),
        None => return Err("decrypt() requires 2 arguments".to_string()),
    };
    // Real AES-256-GCM decryption (Phase 7.3)
    // Encrypted format: nonce (12 bytes) || ciphertext_with_tag
    use aes_gcm::{Aes256Gcm, Key, Nonce};
    use aes_gcm::aead::{Aead, KeyInit};

    if encrypted.len() < 13 {
        // Need at least 12 (nonce) + 1 (tag minimum)
        return Err("decrypt() invalid encrypted data: too short".to_string());
    }

    let key_bytes = hex::decode(key_str)
        .map_err(|e| format!("decrypt() invalid key format (expected hex): {}", e))?;
    if key_bytes.len() != 32 {
        return Err(format!("decrypt() key must be 256-bit (64 hex chars), got {} bytes", key_bytes.len()));
    }

    let (nonce_bytes, ciphertext) = encrypted.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    match cipher.decrypt(nonce, ciphertext) {
        Ok(plaintext) => {
            match String::from_utf8(plaintext) {
                Ok(s) => Ok(Value::String(s)),
                Err(_) => Err("decrypt() decrypted data is not valid UTF-8".to_string()),
            }
        }
        Err(_) => Err("decrypt() failed: incorrect key or corrupted data".to_string()),
    }
}

fn builtin_generate_key(args: &[Value]) -> Result<Value, String> {
    let _ = args; // no args needed
    // Generate a real 256-bit random key (Phase 7.3)
    use rand::RngCore;

    let mut key_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key_bytes);
    let key_hex = hex::encode(key_bytes); // 64 hex chars
    Ok(Value::Secret(SecretString::new(key_hex)))
}

// ── Phase 6.5 — Auth stubs ───────────────────────────

fn builtin_authenticate(args: &[Value]) -> Result<Value, String> {
    let _email = expect_string_arg("authenticate", args, 0)?;
    let _password = match args.get(1) {
        Some(Value::Secret(_)) => true,
        Some(Value::String(_)) => true,
        Some(other) => return Err(format!("authenticate() expected Secret or String as password, got {}", other.type_name())),
        None => return Err("authenticate() requires 2 arguments (email, password)".to_string()),
    };
    // In interpreter mode, always fail (mock)
    Ok(Value::Unit)
}

fn builtin_session_login(args: &[Value]) -> Result<Value, String> {
    let _user_id = expect_string_arg("session_login", args, 0)?;
    // In interpreter mode, return empty session
    Ok(Value::Session(std::collections::HashMap::new()))
}

fn builtin_session_logout(args: &[Value]) -> Result<Value, String> {
    let _session = match args.get(0) {
        Some(Value::Session(_)) => true,
        Some(other) => return Err(format!("session_logout() expected Session, got {}", other.type_name())),
        None => return Err("session_logout() requires 1 argument".to_string()),
    };
    Ok(Value::Unit)
}

// ── Phase 6.6 — Bot stubs ───────────────────────────

fn builtin_send_message(args: &[Value]) -> Result<Value, String> {
    let _chat_id = match args.get(0) {
        Some(Value::String(_)) | Some(Value::Float(_)) => true,
        Some(other) => return Err(format!("send_message() expected String or Float as chat_id, got {}", other.type_name())),
        None => return Err("send_message() requires 2 arguments (chat_id, text)".to_string()),
    };
    let text = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => return Err(format!("send_message() expected String as text, got {}", other.type_name())),
        None => return Err("send_message() requires 2 arguments (chat_id, text)".to_string()),
    };
    // In interpreter mode, log to audit and return Unit
    eprintln!("[AUDIT] send_message: {}", text);
    Ok(Value::Unit)
}

// ── Outgoing HTTP (Definition of Done: http_post) ─────────────────

/// Send an HTTP POST request. Returns the response body as String.
/// Usage: http_post(url, body, content_type)
/// Usage: http_post(url, body, content_type, auth_token)        — sets Authorization: Bearer <auth_token>
/// Usage: http_post(url, body, content_type, headers_struct)    — sets headers from Struct fields
/// Наряд №12 Bug 2: Added 4th parameter for authorization headers.
fn builtin_http_post(args: &[Value]) -> Result<Value, String> {
    let url = match args.get(0) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => return Err(format!("http_post() expected String as url, got {}", other.type_name())),
        None => return Err("http_post() requires at least 1 argument (url)".to_string()),
    };

    let body = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => return Err(format!("http_post() expected String as body, got {}", other.type_name())),
        None => return Err("http_post() requires at least 2 arguments (url, body)".to_string()),
    };

    let content_type = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        _ => "application/json".to_string(),
    };


    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("http_post(): failed to create client: {}", e))?;


    let mut req = client
        .post(&url)
        .header("Content-Type", &content_type)
        .body(body);

    // Optional 4th argument: headers (Наряд №12 Bug 2)
    // String: treat as Bearer token; Struct: set headers from fields
    if let Some(headers_arg) = args.get(3) {
        match headers_arg {
            Value::String(auth_token) => {
                if !auth_token.is_empty() {
                    req = req.header("Authorization", format!("Bearer {}", auth_token));
                }
            }
            Value::Struct { fields, .. } => {
                for (key, val) in fields {
                    if let Value::String(v) = val {
                        req = req.header(key.as_str(), v.as_str());
                    }
                }
            }
            _ => {}
        }
    }

    let resp = req
        .send()
        .map_err(|e| format!("http_post() request failed: {}", e))?;

    let status = resp.status().as_u16();
    let resp_body = resp.text().unwrap_or_default();

    if status >= 400 {
        return Err(format!("http_post() returned status {}: {}", status, resp_body));
    }

    Ok(Value::String(resp_body))
}

/// Send a request to Anthropic Claude Messages API.
/// Usage: call_claude(api_key, model, system_prompt, user_message) -> String
fn builtin_call_claude(args: &[Value]) -> Result<Value, String> {
    let api_key = expect_string_arg("call_claude", args, 0)?;
    let model = expect_string_arg("call_claude", args, 1)?;
    let system_prompt = expect_string_arg("call_claude", args, 2)?;
    let user_message = expect_string_arg("call_claude", args, 3)?;

    let body = serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "system": system_prompt,
        "messages": [{"role": "user", "content": user_message}]
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("call_claude(): failed to create client: {}", e))?;

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .body(body.to_string())
        .send()
        .map_err(|e| format!("call_claude(): request failed: {}", e))?;

    let status = resp.status().as_u16();
    let resp_body = resp.text().unwrap_or_default();

    if status >= 400 {
        return Err(format!("call_claude() returned status {}: {}", status, resp_body));
    }

    // Parse response and extract content[0].text
    let parsed: serde_json::Value = serde_json::from_str(&resp_body)
        .map_err(|e| format!("call_claude(): JSON parse error: {}", e))?;

    let content = parsed["content"][0]["text"]
        .as_str()
        .unwrap_or("Claude API returned an unexpected response format")
        .to_string();

    Ok(Value::String(content))
}

/// Escape a string for safe embedding inside a JSON string value.
/// Replaces: " -> \" , \ -> \\ , newline -> \n , tab -> \t , carriage return -> \r
/// Usage: escape_json(text) -> String
fn builtin_escape_json(args: &[Value]) -> Result<Value, String> {
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

// ── Phase 7.7 — parse_json, http_get, now ────────────────────────────

/// Parse a JSON string into a Value (Struct or List).
/// Usage: parse_json(text) -> Struct|List|String|Float|Bool|Unit
fn builtin_parse_json(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("parse_json", args, 0)?;
    let parsed: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("parse_json() error: {}", e))?;
    Ok(json_value_to_mlog_value(&parsed))
}

/// Convert serde_json::Value to METALOGOS Value (same logic as interpreter's method).
fn json_value_to_mlog_value(json: &serde_json::Value) -> Value {
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
            Value::Struct { type_name: "Json".to_string(), fields }
        }
    }
}

/// Convert METALOGOS Value to serde_json::Value (reverse of json_value_to_mlog_value).
fn mlog_value_to_json(val: &Value) -> serde_json::Value {
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
fn builtin_json_encode(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("json_encode() requires 1 argument".to_string());
    }
    let json = mlog_value_to_json(&args[0]);
    let serialized = serde_json::to_string(&json)
        .map_err(|e| format!("json_encode() serialization error: {}", e))?;
    Ok(Value::String(serialized))
}

/// Send an HTTP GET request. Returns the response body as String.
/// Usage: http_get(url) -> String
fn builtin_http_get(args: &[Value]) -> Result<Value, String> {
    let url = match args.get(0) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => return Err(format!("http_get() expected String as url, got {}", other.type_name())),
        None => return Err("http_get() requires 1 argument (url)".to_string()),
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("http_get(): failed to create client: {}", e))?;

    let resp = client
        .get(&url)
        .send()
        .map_err(|e| format!("http_get() request failed: {}", e))?;

    let status = resp.status().as_u16();
    let resp_body = resp.text().unwrap_or_default();

    if status >= 400 {
        return Err(format!("http_get() returned status {}: {}", status, resp_body));
    }

    Ok(Value::String(resp_body))
}

/// Return current Unix timestamp as Float (seconds since epoch).
/// Usage: now() -> Float
fn builtin_now(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    Ok(Value::Float(now))
}

fn builtin_require(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("require() requires 1 argument (bool expression)".to_string());
    }
    match &args[0] {
        Value::Bool(true) => Ok(Value::Unit),
        Value::Bool(false) => {
            let msg = if args.len() > 1 {
                match &args[1] {
                    Value::String(s) => s.clone(),
                    other => format!("{:?}", other),
                }
            } else {
                "require assertion failed".to_string()
            };
            Err(format!("require assertion failed: {}", msg))
        }
        other => Err(format!("require() expected Bool, got {}", other.type_name())),
    }
}

// ── v0.5.0 — New string builtins ──────────────────────────

/// `length(s)` — returns the length of a string or list as Float.
fn builtin_length(args: &[Value]) -> Result<Value, String> {
    match args.get(0) {
        Some(Value::String(s)) => Ok(Value::Float(s.chars().count() as f64)),
        Some(Value::List(items)) => Ok(Value::Float(items.len() as f64)),
        other => Err(format!("length() requires String or List, got {}", other.as_ref().map(|v| v.type_name()).unwrap_or("none"))),
    }
}

/// `to_int(s)` — parse a string to an integer Float (truncates towards zero).
fn builtin_to_int(args: &[Value]) -> Result<Value, String> {
    match args.get(0) {
        Some(Value::Float(f)) => Ok(Value::Float(f.trunc())),
        Some(Value::String(s)) => {
            // Try integer parse first, then float truncation
            if let Ok(i) = s.parse::<i64>() {
                Ok(Value::Float(i as f64))
            } else if let Ok(f) = s.parse::<f64>() {
                Ok(Value::Float(f.trunc()))
            } else {
                Ok(Value::Float(0.0)) // soft-failure
            }
        }
        Some(Value::Bool(b)) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
        _ => Ok(Value::Float(0.0)), // soft-failure
    }
}

/// `reverse(s)` — reverse a string or list.
fn builtin_reverse(args: &[Value]) -> Result<Value, String> {
    match args.get(0) {
        Some(Value::String(s)) => {
            Ok(Value::String(s.chars().rev().collect()))
        }
        Some(Value::List(items)) => {
            let mut rev = items.clone();
            rev.reverse();
            Ok(Value::List(rev))
        }
        other => Err(format!("reverse() requires String or List, got {}", other.as_ref().map(|v| v.type_name()).unwrap_or("none"))),
    }
}

// ── v0.5.0 — LLM call builtin ──────────────────────────────

/// `call_llm(prompt, input)` — call the LLM backend with a prompt and input.
/// When METALOGOS_LLM_MOCK=true (default), returns "[MOCK: <prompt> | <input>]".
/// When METALOGOS_LLM_MOCK=false, calls the real LLM backend (30s timeout).
fn builtin_call_llm(args: &[Value]) -> Result<Value, String> {
    let prompt = match args.get(0) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => return Err(format!("call_llm() expected String as prompt, got {}", other.type_name())),
        None => return Err("call_llm() requires at least 1 argument (prompt)".to_string()),
    };
    let input = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => format!("{}", other),
        None => String::new(),
    };

    // Check mock mode
    let mock_mode = std::env::var("METALOGOS_LLM_MOCK")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(true); // Default: mock mode ON

    if mock_mode {
        Ok(Value::String(format!("[MOCK: {} | {}]", prompt, input)))
    } else {
        // Real LLM call
        let backend = crate::llm::create_llm_backend();
        backend.call(&prompt, &input)
            .map(Value::String)
            .map_err(|e| format!("call_llm() failed: {}", e))
    }
}

// ── v0.5.0 — KV memory builtins ────────────────────────────
// These use a thread-local KV store (in-memory by default).
// When memory { persist: "..." } is configured, they also persist to SQLite kv_store table.
// Uses a write-through cache: in-memory HashMap is always authoritative;
// SQLite is a persistence backend that mirrors the HashMap.

use std::sync::Mutex as StdMutex;

/// Global KV store — lazy_static pattern using std::sync::OnceLock (Rust 1.70+).
static KV_STORE: std::sync::OnceLock<StdMutex<std::collections::HashMap<String, String>>> = std::sync::OnceLock::new();

fn kv_store() -> &'static StdMutex<std::collections::HashMap<String, String>> {
    KV_STORE.get_or_init(|| StdMutex::new(std::collections::HashMap::new()))
}

/// Global SQLite KV persistence backend.
/// Initialized by init_kv_persist() when memory { persist: "..." } is configured.
/// Uses std::sync::Mutex (same thread model as KV_STORE).
static KV_SQLITE: std::sync::OnceLock<StdMutex<Option<rusqlite::Connection>>> = std::sync::OnceLock::new();

fn kv_sqlite() -> &'static StdMutex<Option<rusqlite::Connection>> {
    KV_SQLITE.get_or_init(|| StdMutex::new(None))
}

/// Initialize SQLite persistence for the KV store.
/// Called by Interpreter::configure_memory() when persist path is set.
/// Creates kv_store table (key TEXT PRIMARY KEY, value TEXT) in the given database.
/// Loads existing rows into the in-memory HashMap.
pub fn init_kv_persist(db_path: &str) -> Result<(), String> {
    let conn = rusqlite::Connection::open(db_path)
        .map_err(|e| format!("[kv_store] Failed to open database '{}': {}", db_path, e))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS kv_store (key TEXT PRIMARY KEY, value TEXT NOT NULL);"
    ).map_err(|e| format!("[kv_store] Failed to create table: {}", e))?;

    // Load existing KV pairs into in-memory HashMap (write-through cache warmup)
    {
        let mut stmt = conn.prepare("SELECT key, value FROM kv_store")
            .map_err(|e| format!("[kv_store] Failed to query: {}", e))?;
        let rows: Vec<(String, String)> = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).map_err(|e| format!("[kv_store] Failed to iterate: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

        // Merge into in-memory store (SQLite is authoritative on init)
        if let Ok(mut store) = kv_store().lock() {
            for (key, value) in rows {
                store.insert(key, value);
            }
        }
    } // stmt is dropped here, releasing borrow on conn

    // Store the connection globally
    let mut sqlite_guard = kv_sqlite().lock()
        .map_err(|e| format!("[kv_store] lock error: {}", e))?;
    *sqlite_guard = Some(conn);
    eprintln!("[kv_store] SQLite persistence enabled: {}", db_path);
    Ok(())
}

/// `kv_set(key, value)` — store a key-value pair.
fn builtin_kv_set(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("kv_set", args, 0)?;
    let value = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => format!("{}", other),
        None => return Err("kv_set() requires 2 arguments (key, value)".to_string()),
    };
    let mut store = kv_store().lock().map_err(|e| format!("kv_set() lock error: {}", e))?;
    store.insert(key.clone(), value.clone());
    // Write-through to SQLite if available
    if let Ok(sqlite_guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *sqlite_guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, value],
            );
        }
    }
    Ok(Value::Unit)
}

/// `kv_get(key)` — retrieve a value by key. Returns empty string if not found.
fn builtin_kv_get(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("kv_get", args, 0)?;
    let store = kv_store().lock().map_err(|e| format!("kv_get() lock error: {}", e))?;
    Ok(Value::String(store.get(&key).cloned().unwrap_or_default()))
}

/// `kv_delete(key)` — remove a key-value pair.
fn builtin_kv_delete(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("kv_delete", args, 0)?;
    let mut store = kv_store().lock().map_err(|e| format!("kv_delete() lock error: {}", e))?;
    store.remove(&key);
    // Write-through delete to SQLite if available
    if let Ok(sqlite_guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *sqlite_guard {
            let _ = conn.execute("DELETE FROM kv_store WHERE key = ?1", rusqlite::params![key]);
        }
    }
    Ok(Value::Unit)
}

/// `kv_exists(key)` — check if a key exists. Returns Bool.
fn builtin_kv_exists(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("kv_exists", args, 0)?;
    let store = kv_store().lock().map_err(|e| format!("kv_exists() lock error: {}", e))?;
    Ok(Value::Bool(store.contains_key(&key)))
}

/// `kv_list()` — list all keys. Returns List of Strings.
fn builtin_kv_list(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let store = kv_store().lock().map_err(|e| format!("kv_list() lock error: {}", e))?;
    let keys: Vec<Value> = store.keys().cloned().map(Value::String).collect();
    Ok(Value::List(keys))
}

// ── Наряд №6 — mem_set / mem_get / mem_delete (exact KV, not semantic) ─
// These are user-facing aliases for the KV store with String return types.
// mem_set returns the stored value, mem_get returns value or empty string,
// mem_delete returns the deleted value or empty string.
// They share the same global HashMap + optional SQLite backend as kv_*.

/// `mem_set(key, value)` — exact key-value write. Returns the stored value.
fn builtin_mem_set(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("mem_set", args, 0)?;
    let value = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => format!("{}", other),
        None => return Err("mem_set() requires 2 arguments (key, value)".to_string()),
    };
    let mut store = kv_store().lock().map_err(|e| format!("mem_set() lock error: {}", e))?;
    store.insert(key.clone(), value.clone());
    // Write-through to SQLite if available
    if let Ok(sqlite_guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *sqlite_guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, value],
            );
        }
    }
    Ok(Value::String(value))
}

/// `mem_get(key)` — exact key-value read (not semantic recall).
/// Returns the value or empty string if not found.
fn builtin_mem_get(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("mem_get", args, 0)?;
    let store = kv_store().lock().map_err(|e| format!("mem_get() lock error: {}", e))?;
    Ok(Value::String(store.get(&key).cloned().unwrap_or_default()))
}

/// `mem_delete(key)` — remove a key-value pair. Returns the deleted value or empty string.
fn builtin_mem_delete(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("mem_delete", args, 0)?;
    let mut store = kv_store().lock().map_err(|e| format!("mem_delete() lock error: {}", e))?;
    let removed = store.remove(&key);
    // Write-through delete to SQLite if available
    if let Ok(sqlite_guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *sqlite_guard {
            let _ = conn.execute("DELETE FROM kv_store WHERE key = ?1", rusqlite::params![key]);
        }
    }
    Ok(Value::String(removed.unwrap_or_default()))
}

// ── ADR-0049 — session memory (temporary per-session KV store) ──
// In-memory HashMap<String, HashMap<String, String>> — NOT persistent.
// Resets when mlog serve restarts (by design: session data is ephemeral).
// Unlike mem_set/mem_get (global), session_* is scoped to a specific session_id.
//
// Usage:
//   session_set(session_id, key, value)   -> String (stored value)
//   session_get(session_id, key)             -> String (value or "")
//   session_clear(session_id)                -> Unit

/// Global session store — lazy_static pattern using std::sync::OnceLock.
/// Outer key = session_id, inner key = data key, inner value = data value.
static SESSION_STORE: std::sync::OnceLock<StdMutex<std::collections::HashMap<String, std::collections::HashMap<String, String>>>> = std::sync::OnceLock::new();

fn session_store() -> &'static StdMutex<std::collections::HashMap<String, std::collections::HashMap<String, String>>> {
    SESSION_STORE.get_or_init(|| StdMutex::new(std::collections::HashMap::new()))
}

/// Reset the entire session store. Used by contract tests to verify restart behavior.
pub fn reset_session_store() {
    if let Ok(mut store) = session_store().lock() {
        store.clear();
    }
}

/// Get the number of sessions in the store. Used by contract tests.
pub fn session_store_count() -> usize {
    session_store().lock().map(|s| s.len()).unwrap_or(0)
}

/// Get the number of keys in a specific session. Used by contract tests.
pub fn session_key_count(session_id: &str) -> usize {
    session_store().lock()
        .ok()
        .and_then(|s| s.get(session_id).map(|m| m.len()))
        .unwrap_or(0)
}

/// `session_set(session_id, key, value)` — store a value scoped to a session.
/// Returns the stored value. Creates session bucket if it doesn't exist.
fn builtin_session_set(args: &[Value]) -> Result<Value, String> {
    let session_id = expect_string_arg("session_set", args, 0)?;
    let key = expect_string_arg("session_set", args, 1)?;
    let value = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => format!("{}", other),
        None => return Err("session_set() requires 3 arguments (session_id, key, value)".to_string()),
    };
    let mut store = session_store().lock().map_err(|e| format!("session_set() lock error: {}", e))?;
    store.entry(session_id).or_default().insert(key.clone(), value.clone());
    Ok(Value::String(value))
}

/// `session_get(session_id, key)` — retrieve a value from a session.
/// Returns empty string if session or key not found.
fn builtin_session_get(args: &[Value]) -> Result<Value, String> {
    let session_id = expect_string_arg("session_get", args, 0)?;
    let key = expect_string_arg("session_get", args, 1)?;
    let store = session_store().lock().map_err(|e| format!("session_get() lock error: {}", e))?;
    let value = store.get(&session_id)
        .and_then(|session| session.get(&key).cloned())
        .unwrap_or_default();
    Ok(Value::String(value))
}

/// `session_clear(session_id)` — remove all keys for a session.
/// Returns "ok". No-op if session doesn't exist.
fn builtin_session_clear(args: &[Value]) -> Result<Value, String> {
    let session_id = expect_string_arg("session_clear", args, 0)?;
    let mut store = session_store().lock().map_err(|e| format!("session_clear() lock error: {}", e))?;
    store.remove(&session_id);
    Ok(Value::String("ok".to_string()))
}

// ── v0.5.0 — File I/O builtins (sandboxed) ──────────────────
// All file operations are restricted to the working directory.
// Paths containing ".." or absolute paths are rejected (sandbox).

/// Validate that a path is safe (within working directory, no traversal).
fn sandbox_path(path: &str) -> Result<std::path::PathBuf, String> {
    let p = std::path::Path::new(path);
    // Reject absolute paths
    if p.is_absolute() {
        return Err(format!("file I/O sandbox: absolute paths not allowed: '{}'", path));
    }
    // Reject path traversal
    for component in p.components() {
        if let std::path::Component::ParentDir = component {
            return Err(format!("file I/O sandbox: path traversal ('..') not allowed: '{}'", path));
        }
    }
    Ok(std::path::PathBuf::from(path))
}

/// `read_file(path)` — read file contents as String.
/// Soft-failure: returns empty string on error (file not found, permission denied, etc.).
fn builtin_read_file(args: &[Value]) -> Result<Value, String> {
    let path = expect_string_arg("read_file", args, 0)?;
    let safe_path = match sandbox_path(&path) {
        Ok(p) => p,
        Err(_) => return Ok(Value::String(String::new())), // soft-failure on sandbox violation
    };
    match std::fs::read_to_string(&safe_path) {
        Ok(content) => Ok(Value::String(content)),
        Err(_) => Ok(Value::String(String::new())), // soft-failure
    }
}

/// `write_file(path, content)` — write string to file (overwrite).
/// Returns "ok" on success, empty string on soft-failure.
fn builtin_write_file(args: &[Value]) -> Result<Value, String> {
    let path = expect_string_arg("write_file", args, 0)?;
    let content = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => format!("{}", other),
        None => return Ok(Value::String(String::new())), // soft-failure
    };
    let safe_path = match sandbox_path(&path) {
        Ok(p) => p,
        Err(_) => return Ok(Value::String(String::new())), // soft-failure on sandbox violation
    };
    // Create parent directories if needed
    if let Some(parent) = safe_path.parent() {
        let _ = std::fs::create_dir_all(parent); // best-effort
    }
    match std::fs::write(&safe_path, &content) {
        Ok(_) => Ok(Value::String("ok".to_string())),
        Err(_) => Ok(Value::String(String::new())), // soft-failure
    }
}

/// `append_file(path, content)` — append string to file.
/// Returns "ok" on success, empty string on soft-failure.
fn builtin_append_file(args: &[Value]) -> Result<Value, String> {
    let path = expect_string_arg("append_file", args, 0)?;
    let content = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => format!("{}", other),
        None => return Ok(Value::String(String::new())), // soft-failure
    };
    let safe_path = match sandbox_path(&path) {
        Ok(p) => p,
        Err(_) => return Ok(Value::String(String::new())), // soft-failure on sandbox violation
    };
    // Create parent directories if needed
    if let Some(parent) = safe_path.parent() {
        let _ = std::fs::create_dir_all(parent); // best-effort
    }
    use std::io::Write;
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&safe_path)
    {
        Ok(mut file) => match file.write_all(content.as_bytes()) {
            Ok(_) => Ok(Value::String("ok".to_string())),
            Err(_) => Ok(Value::String(String::new())), // soft-failure
        },
        Err(_) => Ok(Value::String(String::new())), // soft-failure
    }
}

/// `delete_file(path)` — delete a file.
/// Soft-failure: returns empty string on error.
fn builtin_delete_file(args: &[Value]) -> Result<Value, String> {
    let path = expect_string_arg("delete_file", args, 0)?;
    let safe_path = match sandbox_path(&path) {
        Ok(p) => p,
        Err(_) => return Ok(Value::String(String::new())), // soft-failure
    };
    match std::fs::remove_file(&safe_path) {
        Ok(_) => Ok(Value::String("ok".to_string())),
        Err(_) => Ok(Value::String(String::new())), // soft-failure
    }
}

/// `file_exists(path)` — check if a file exists. Returns Bool.
fn builtin_file_exists(args: &[Value]) -> Result<Value, String> {
    let path = expect_string_arg("file_exists", args, 0)?;
    let safe_path = sandbox_path(&path)?;
    Ok(Value::Bool(safe_path.exists()))
}

/// `list_dir(path)` — list files in a directory. Returns List of Strings.
fn builtin_list_dir(args: &[Value]) -> Result<Value, String> {
    let path = if args.is_empty() {
        ".".to_string()
    } else {
        expect_string_arg("list_dir", args, 0)?
    };
    let safe_path = sandbox_path(&path)?;
    let entries: Vec<Value> = std::fs::read_dir(&safe_path)
        .map_err(|e| format!("list_dir('{}'): {}", path, e))?
        .filter_map(|entry| {
            entry.ok().map(|e| {
                Value::String(e.file_name().to_string_lossy().to_string())
            })
        })
        .collect();
    Ok(Value::List(entries))
}

/// Наряд №4: `llm_usage()` — returns LLM usage statistics as a Struct.
/// Returns: { total_calls: Float, total_tokens: Float, total_errors: Float, providers: List }
fn builtin_llm_usage(_args: &[Value]) -> Result<Value, String> {
    let report = crate::llm::global_llm_usage_report();

    let mut fields = std::collections::HashMap::new();
    fields.insert("total_calls".to_string(), Value::Float(report.total_calls));
    fields.insert("total_tokens".to_string(), Value::Float(report.total_tokens));
    fields.insert("total_errors".to_string(), Value::Float(report.total_errors));

    let providers: Vec<Value> = report.providers.iter().map(|p| {
        let mut pf = std::collections::HashMap::new();
        pf.insert("alias".to_string(), Value::String(p.alias.clone()));
        pf.insert("calls".to_string(), Value::Float(p.calls as f64));
        pf.insert("tokens".to_string(), Value::Float(p.tokens as f64));
        pf.insert("errors".to_string(), Value::Float(p.errors as f64));
        pf.insert("avg_latency_ms".to_string(), Value::Float(p.avg_latency_ms));
        pf.insert("health_score".to_string(), Value::Float(p.health_score));
        Value::Struct {
            type_name: "ProviderUsage".to_string(),
            fields: pf,
        }
    }).collect();
    fields.insert("providers".to_string(), Value::List(providers));

    Ok(Value::Struct {
        type_name: "LlmUsage".to_string(),
        fields,
    })
}
