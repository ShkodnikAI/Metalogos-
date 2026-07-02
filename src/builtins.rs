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
        funcs.insert("answer_callback_query".to_string(), builtin_answer_callback_query as BuiltinFn);
        funcs.insert("edit_message_text".to_string(), builtin_edit_message_text as BuiltinFn);
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
        funcs.insert("json_get".to_string(), builtin_json_get as BuiltinFn);
        funcs.insert("has_field".to_string(), builtin_has_field as BuiltinFn);
        funcs.insert("http_get".to_string(), builtin_http_get as BuiltinFn);
        funcs.insert("now".to_string(), builtin_now as BuiltinFn);

        // ADR-0049 — session memory (temporary per-session KV store)
        funcs.insert("session_set".to_string(), builtin_session_set as BuiltinFn);
        funcs.insert("session_get".to_string(), builtin_session_get as BuiltinFn);
        funcs.insert("session_clear".to_string(), builtin_session_clear as BuiltinFn);

        // Voice pipeline builtins (Phase 7.8)
        funcs.insert("http_post_multipart".to_string(), builtin_http_post_multipart as BuiltinFn);
        funcs.insert("whisper_transcribe".to_string(), builtin_whisper_transcribe as BuiltinFn);
        funcs.insert("tts_send".to_string(), builtin_tts_send as BuiltinFn);

        // Наряд 17: utility builtins — base64, exec, escape_js, dict operations, type_of
        funcs.insert("base64_encode".to_string(), builtin_base64_encode as BuiltinFn);
        funcs.insert("base64_decode".to_string(), builtin_base64_decode as BuiltinFn);
        funcs.insert("exec".to_string(), builtin_exec as BuiltinFn);
        funcs.insert("escape_js".to_string(), builtin_escape_js as BuiltinFn);
        funcs.insert("dict_get".to_string(), builtin_json_get as BuiltinFn); // alias
        funcs.insert("dict_set".to_string(), builtin_dict_set as BuiltinFn);
        funcs.insert("dict_keys".to_string(), builtin_dict_keys as BuiltinFn);
        funcs.insert("dict_values".to_string(), builtin_dict_values as BuiltinFn);
        funcs.insert("dict_has".to_string(), builtin_dict_has as BuiltinFn);
        funcs.insert("type_of".to_string(), builtin_type_of as BuiltinFn);
        // Наряд №17 В.3: format() — positional string interpolation
        funcs.insert("format".to_string(), builtin_format as BuiltinFn);

        // Наряд №24: git_push, web_search, make_list
        funcs.insert("git_push".to_string(), builtin_git_push as BuiltinFn);
        funcs.insert("web_search".to_string(), builtin_web_search as BuiltinFn);
        funcs.insert("make_list".to_string(), builtin_make_list as BuiltinFn);
        // time() alias for now()
        funcs.insert("time".to_string(), builtin_now as BuiltinFn);
        // format_date() — format unix timestamp or current time (enhanced v0.8.0)
        funcs.insert("format_date".to_string(), builtin_format_date as BuiltinFn);
        // v0.8.0 — Time / Date / Calendar (additional)
        funcs.insert("date_parts".to_string(), builtin_date_parts as BuiltinFn);
        funcs.insert("days_between".to_string(), builtin_days_between as BuiltinFn);
        funcs.insert("days_in_month".to_string(), builtin_days_in_month as BuiltinFn);
        funcs.insert("is_leap_year".to_string(), builtin_is_leap_year as BuiltinFn);
        funcs.insert("add_days".to_string(), builtin_add_days as BuiltinFn);
        funcs.insert("add_hours".to_string(), builtin_add_hours as BuiltinFn);
        funcs.insert("weekday_name".to_string(), builtin_weekday_name as BuiltinFn);
        // v0.8.0 — Geolocation
        funcs.insert("geo_ip".to_string(), builtin_geo_ip as BuiltinFn);
        funcs.insert("geo_distance".to_string(), builtin_geo_distance as BuiltinFn);
        // v0.8.0 — Weather (Open-Meteo, free, no API key)
        funcs.insert("weather".to_string(), builtin_weather as BuiltinFn);
        funcs.insert("weather_forecast".to_string(), builtin_weather_forecast as BuiltinFn);
        // v0.8.0 — Reminders
        funcs.insert("remind".to_string(), builtin_remind as BuiltinFn);
        funcs.insert("remind_recurring".to_string(), builtin_remind_recurring as BuiltinFn);
        funcs.insert("cancel_remind".to_string(), builtin_cancel_remind as BuiltinFn);
        funcs.insert("list_reminders".to_string(), builtin_list_reminders as BuiltinFn);
        funcs.insert("check_reminders".to_string(), builtin_check_reminders as BuiltinFn);
        // request_body() alias for json_body() — common in web frameworks
        funcs.insert("request_body".to_string(), builtin_json_body as BuiltinFn);
        // Public first/last (without __ prefix)
        funcs.insert("first".to_string(), builtin_first as BuiltinFn);
        funcs.insert("last".to_string(), builtin_last as BuiltinFn);

        // v0.8.1 — OpenHuman-inspired Human Intelligence builtins
        funcs.insert("human_create".to_string(), builtin_human_create as BuiltinFn);
        funcs.insert("human_mood".to_string(), builtin_human_mood as BuiltinFn);
        funcs.insert("human_remember".to_string(), builtin_human_remember as BuiltinFn);
        funcs.insert("human_forget".to_string(), builtin_human_forget as BuiltinFn);
        funcs.insert("human_recall".to_string(), builtin_human_recall as BuiltinFn);
        funcs.insert("human_respond".to_string(), builtin_human_respond as BuiltinFn);
        funcs.insert("human_personas".to_string(), builtin_human_personas as BuiltinFn);
        funcs.insert("human_delete".to_string(), builtin_human_delete as BuiltinFn);

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
    eprintln!("[print] {}", s);
    Ok(Value::String(s))
}

fn builtin_contains(args: &[Value]) -> Result<Value, String> {
    let haystack = expect_string_arg("contains", args, 0)?;
    let needle = expect_string_arg("contains", args, 1)?;
    Ok(Value::Bool(haystack.contains(&needle)))
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
        _ => return Err("first() requires List as first argument".to_string()),
    };
    match list.first() {
        Some(v) => Ok(v.clone()),
        None => Ok(Value::String(String::new())), // soft-failure
    }
}

fn builtin_last(args: &[Value]) -> Result<Value, String> {
    let list = match args.get(0) {
        Some(Value::List(items)) => items,
        _ => return Err("last() requires List as first argument".to_string()),
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
    // Two forms: respond("200 OK") or respond("200", "body text")
    let (status, body) = if args.len() >= 2 {
        let status_str = expect_string_arg("respond", args, 0)?;
        let status = status_str.parse::<u16>().unwrap_or(200);
        let body = match &args[1] {
            Value::String(s) => s.clone(),
            other => format!("{}", other),
        };
        (status, body)
    } else {
        let status_body = expect_string_arg("respond", args, 0)?;
        parse_status_line(&status_body)
    };
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

/// Convert Metalogos Value to serde_json::Value (inverse of server.rs json_value_to_value).
fn value_to_json(val: &Value) -> Result<serde_json::Value, String> {
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
            let arr: Vec<serde_json::Value> = items.iter()
                .map(value_to_json)
                .collect::<Result<_, _>>()?;
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

fn builtin_send_message(args: &[Value]) -> Result<Value, String> {
    // Extract and format chat_id — supports negative channel IDs (Наряд №24 B5)
    let chat_id_value: serde_json::Value = match args.get(0) {
        Some(Value::String(s)) => serde_json::Value::String(s.clone()),
        Some(Value::Float(f)) => {
            if *f == (*f as i64) as f64 {
                serde_json::json!(*f as i64)
            } else {
                serde_json::json!(*f)
            }
        }
        Some(other) => return Err(format!("send_message() expected String or Float as chat_id, got {}", other.type_name())),
        None => return Err("send_message() requires at least 2 arguments (chat_id, text)".to_string()),
    };
    let text = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => return Err(format!("send_message() expected String as text, got {}", other.type_name())),
        None => return Err("send_message() requires at least 2 arguments (chat_id, text)".to_string()),
    };

    // Try to send via Telegram API if BOT_TOKEN env var is set
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
    if bot_token.is_empty() {
        // No token — fall back to audit stub
        eprintln!("[AUDIT] send_message to {:?}: {}", chat_id_value, text);
        return Ok(Value::Unit);
    }

    // Build JSON body with optional reply_markup (3rd arg: Struct)
    let mut body = serde_json::json!({
        "chat_id": chat_id_value,
        "text": text,
    });
    if let Some(markup) = args.get(2) {
        let markup_json = value_to_json(markup)?;
        body["reply_markup"] = markup_json;
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("send_message(): client error: {}", e))?;

    let resp = client
        .post(format!("https://api.telegram.org/bot{}/sendMessage", bot_token))
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .map_err(|e| format!("send_message(): request failed: {}", e))?;

    let status = resp.status().as_u16();
    let resp_body = resp.text().unwrap_or_default();

    if status >= 400 {
        return Err(format!("send_message(): Telegram status {}: {}", status, resp_body));
    }

    Ok(Value::String(resp_body))
}

/// `answer_callback_query(callback_query_id, text?, show_alert?)` — respond to Telegram inline keyboard callback.
/// `callback_query_id` from update.callback_query.id.
/// `text` — notification text (max 200 chars). `show_alert` — 1.0 = alert popup, 0.0 = toast (default).
fn builtin_answer_callback_query(args: &[Value]) -> Result<Value, String> {
    let callback_query_id = expect_string_arg("answer_callback_query", args, 0)?;
    let text = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    };
    let show_alert = match args.get(2) {
        Some(Value::Float(f)) if *f > 0.5 => true,
        _ => false,
    };
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
    if bot_token.is_empty() {
        eprintln!("[AUDIT] answer_callback_query id={}: {}", callback_query_id, text);
        return Ok(Value::Unit);
    }
    let body = serde_json::json!({
        "callback_query_id": callback_query_id,
        "text": text,
        "show_alert": show_alert,
    });
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build().map_err(|e| format!("answer_callback_query(): client error: {}", e))?;
    let resp = client.post(format!("https://api.telegram.org/bot{}/answerCallbackQuery", bot_token))
        .header("Content-Type", "application/json")
        .body(body.to_string()).send()
        .map_err(|e| format!("answer_callback_query(): request failed: {}", e))?;
    let status = resp.status().as_u16();
    let resp_body = resp.text().unwrap_or_default();
    if status >= 400 {
        return Err(format!("answer_callback_query(): Telegram status {}: {}", status, resp_body));
    }
    Ok(Value::String(resp_body))
}

/// `edit_message_text(chat_id, message_id, text, reply_markup?)` — edit existing Telegram message.
/// Used to update inline keyboard buttons after callback.
fn builtin_edit_message_text(args: &[Value]) -> Result<Value, String> {
    let chat_id_val: serde_json::Value = match args.get(0) {
        Some(Value::String(s)) => serde_json::Value::String(s.clone()),
        Some(Value::Float(f)) => serde_json::json!(*f as i64),
        _ => return Err("edit_message_text() requires chat_id".to_string()),
    };
    let message_id = match args.get(1) {
        Some(Value::Float(f)) => *f as i64,
        _ => return Err("edit_message_text() message_id must be Float".to_string()),
    };
    let text = expect_string_arg("edit_message_text", args, 2)?;
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
    if bot_token.is_empty() {
        eprintln!("[AUDIT] edit_message_text chat_id={}: {}", chat_id_val, text);
        return Ok(Value::Unit);
    }
    let mut body = serde_json::json!({
        "chat_id": chat_id_val,
        "message_id": message_id,
        "text": text,
    });
    if let Some(markup) = args.get(3) {
        let markup_json = value_to_json(markup)?;
        body["reply_markup"] = markup_json;
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build().map_err(|e| format!("edit_message_text(): client error: {}", e))?;
    let resp = client.post(format!("https://api.telegram.org/bot{}/editMessageText", bot_token))
        .header("Content-Type", "application/json")
        .body(body.to_string()).send()
        .map_err(|e| format!("edit_message_text(): request failed: {}", e))?;
    let status = resp.status().as_u16();
    let resp_body = resp.text().unwrap_or_default();
    if status >= 400 {
        return Err(format!("edit_message_text(): Telegram status {}: {}", status, resp_body));
    }
    Ok(Value::String(resp_body))
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

    let (content_type, timeout_arg_idx) = match args.get(2) {
        Some(Value::String(s)) => (s.clone(), 3),
        _ => ("application/json".to_string(), 2),
    };

    // Наряда-26 P0-1: configurable timeout (default 30s, max 300s)
    // Signatures: http_post(url, body, timeout) | http_post(url, body, ct, timeout) | http_post(url, body, ct, headers, timeout)
    let timeout_secs = if let Some(timeout_val) = args.get(timeout_arg_idx) {
        match timeout_val {
            Value::Float(f) => {
                let t = f.clamp(1.0, 300.0) as u64;
                if *f > 300.0 {
                    eprintln!("[http_post] timeout clamped from {} to 300s", f);
                }
                t
            }
            _ => 30, // not a number → skip, treat as headers or ignore
        }
    } else {
        30
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("http_post(): failed to create client: {}", e))?;

    let mut req = client
        .post(&url)
        .header("Content-Type", &content_type)
        .body(body);

    // Optional headers argument (index depends on whether content_type was provided)
    let headers_idx = if args.len() > 2 && matches!(args.get(2), Some(Value::String(_))) {
        // content_type was 3rd arg → headers are 4th
        3
    } else {
        // content_type was default → headers are 3rd
        2
    };
    // Only parse headers if the arg exists and is NOT a Float (which would be timeout)
    if let Some(headers_arg) = args.get(headers_idx) {
        if !matches!(headers_arg, Value::Float(_)) {
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
    }

    let resp = req
        .send()
        .map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("timeout") || err_str.contains("timed out") {
                format!("ERROR: http timeout after {}s", timeout_secs)
            } else {
                format!("http_post() request failed: {}", e)
            }
        })?;

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
        .timeout(std::time::Duration::from_secs(120))
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

/// Safe field access on a struct value: returns default if field missing or not a struct.
/// Usage: json_get(obj, "field") -> Value (returns Unit if missing)
/// Usage: json_get(obj, "field", default_value) -> Value (returns default if missing)
/// Usage: json_get(obj, "nested.field.path", default) -> Value (dot-separated path)
/// Supports numeric path segments for array indexing: "items.0.title"
/// This is the P0 fix: prevents runtime crash when accessing optional JSON fields
/// like message.voice on non-voice Telegram updates.
fn builtin_json_get(args: &[Value]) -> Result<Value, String> {
    let obj = match args.get(0) {
        Some(v) => v,
        None => return Err("json_get() requires at least 2 arguments (obj, field_path)".to_string()),
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
                Ok(val) => { current = val; continue; }
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
        Ok(current.clone())
    } else {
        // 2-argument form: return the found value or Unit if not found
        let mut current = obj;
        for segment in path.split('.') {
            // Try struct field access first
            match current.get_field(segment) {
                Ok(val) => { current = val; continue; }
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
fn builtin_has_field(args: &[Value]) -> Result<Value, String> {
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

/// Send an HTTP GET request. Returns the response body as String.
/// Usage: http_get(url) -> String
/// Usage: http_get(url, headers_struct) -> String  — sets headers from Struct fields
fn builtin_http_get(args: &[Value]) -> Result<Value, String> {
    let url = match args.get(0) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => return Err(format!("http_get() expected String as url, got {}", other.type_name())),
        None => return Err("http_get() requires 1 argument (url)".to_string()),
    };

    // Наряда-26 P0-1: configurable timeout
    // http_get(url) | http_get(url, timeout) | http_get(url, headers) | http_get(url, headers, timeout)
    let (headers_arg, timeout_secs) = match args.len() {
        1 => (None, 30u64),
        2 => {
            // 2nd arg could be timeout (Float) or headers (String/Struct)
            match &args[1] {
                Value::Float(f) => (None, f.clamp(1.0, 300.0) as u64),
                other => (Some(other), 30),
            }
        }
        _ => {
            // 3+ args: 2nd is headers, 3rd is timeout
            let timeout = if let Some(Value::Float(f)) = args.get(2) {
                f.clamp(1.0, 300.0) as u64
            } else {
                30
            };
            (args.get(1), timeout)
        }
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("http_get(): failed to create client: {}", e))?;

    let mut req = client.get(&url);

    if let Some(headers_arg) = headers_arg {
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
        .map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("timeout") || err_str.contains("timed out") {
                format!("ERROR: http timeout after {}s", timeout_secs)
            } else {
                format!("http_get() request failed: {}", e)
            }
        })?;

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
    let safe_path = match sandbox_path(&path) {
        Ok(p) => p,
        Err(_) => return Ok(Value::Bool(false)), // soft-failure on sandbox violation
    };
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

// ── Phase 7.8: Voice pipeline builtins ──────────────────────────────

fn builtin_http_post_multipart(args: &[Value]) -> Result<Value, String> {
    let url = expect_string_arg("http_post_multipart", args, 0)?;
    let fields = match args.get(1) {
        Some(Value::Struct { fields, .. }) => fields.clone(),
        _ => return Err("http_post_multipart() requires Struct as 2nd argument (fields)".to_string()),
    };
    let files = match args.get(2) {
        Some(Value::Struct { fields, .. }) => fields.clone(),
        _ => return Err("http_post_multipart() requires Struct as 3rd argument (files)".to_string()),
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("http_post_multipart(): client error: {}", e))?;

    let mut form = reqwest::blocking::multipart::Form::new();

    // Add text fields
    for (key, val) in &fields {
        if let Value::String(v) = val {
            form = form.text(key.clone(), v.clone());
        }
    }

    // Add file fields
    for (key, val) in &files {
        if let Value::String(path) = val {
            let file_bytes = std::fs::read(path)
                .map_err(|e| format!("http_post_multipart(): cannot read file '{}': {}", path, e))?;
            let file_name = std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file");
            let part = reqwest::blocking::multipart::Part::bytes(file_bytes)
                .file_name(file_name.to_string());
            form = form.part(key.clone(), part);
        }
    }

    let resp = client.post(&url).multipart(form).send()
        .map_err(|e| format!("http_post_multipart(): request failed: {}", e))?;

    let status = resp.status().as_u16();
    let resp_body = resp.text().unwrap_or_default();

    if status >= 400 {
        return Err(format!("http_post_multipart() returned status {}: {}", status, resp_body));
    }

    Ok(Value::String(resp_body))
}

fn builtin_whisper_transcribe(args: &[Value]) -> Result<Value, String> {
    let file_id = expect_string_arg("whisper_transcribe", args, 0)?;
    let bot_token = expect_string_arg("whisper_transcribe", args, 1)?;
    let whisper_key = expect_string_arg("whisper_transcribe", args, 2)?;
    let provider = match args.get(3) {
        Some(Value::String(s)) => s.clone(),
        _ => "openai".to_string(),
    };

    // Step 1: Get file path from Telegram
    let tg_client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("whisper_transcribe(): client error: {}", e))?;

    let get_file_url = format!("https://api.telegram.org/bot{}/getFile?file_id={}", bot_token, file_id);
    let tg_resp = tg_client.get(&get_file_url).send()
        .map_err(|e| format!("whisper_transcribe(): Telegram getFile failed: {}", e))?;
    let tg_body: serde_json::Value = serde_json::from_str(&tg_resp.text().unwrap_or_default())
        .map_err(|e| format!("whisper_transcribe(): Telegram response parse error: {}", e))?;

    let file_path = tg_body.get("result")
        .and_then(|r| r.get("file_path"))
        .and_then(|p| p.as_str())
        .unwrap_or("")
        .to_string();

    if file_path.is_empty() {
        return Err("whisper_transcribe(): Telegram returned empty file_path".to_string());
    }

    // Step 2: Download the file
    let download_url = format!("https://api.telegram.org/file/bot{}/{}", bot_token, file_path);
    let audio_bytes = tg_client.get(&download_url).send()
        .map_err(|e| format!("whisper_transcribe(): download failed: {}", e))?
        .bytes()
        .map_err(|e| format!("whisper_transcribe(): read bytes failed: {}", e))?;

    // Step 3: Send to Whisper API
    let (api_url, auth_header, auth_value) = match provider.as_str() {
        "groq" => (
            "https://api.groq.com/openai/v1/audio/transcriptions".to_string(),
            "Authorization".to_string(),
            format!("Bearer {}", whisper_key),
        ),
        _ => ( // openai
            "https://api.openai.com/v1/audio/transcriptions".to_string(),
            "Authorization".to_string(),
            format!("Bearer {}", whisper_key),
        ),
    };

    // Use multipart form
    let whisper_client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("whisper_transcribe(): whisper client error: {}", e))?;

    let model = if provider == "groq" { "whisper-large-v3" } else { "whisper-1" };
    let mut form = reqwest::blocking::multipart::Form::new();
    form = form.text("model", model.to_string());
    let part = reqwest::blocking::multipart::Part::bytes(audio_bytes.to_vec())
        .file_name("audio.ogg");
    form = form.part("file", part);

    let whisper_resp = whisper_client.post(&api_url)
        .header(auth_header, auth_value)
        .multipart(form)
        .send()
        .map_err(|e| format!("whisper_transcribe(): whisper request failed: {}", e))?;

    let status = whisper_resp.status().as_u16();
    let whisper_body = whisper_resp.text().unwrap_or_default();

    if status >= 400 {
        return Err(format!("whisper_transcribe(): whisper API status {}: {}", status, whisper_body));
    }

    // Parse response to extract text
    let parsed: serde_json::Value = serde_json::from_str(&whisper_body)
        .map_err(|e| format!("whisper_transcribe(): whisper response parse error: {}", e))?;
    let text = parsed.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string();

    Ok(Value::String(text))
}

fn builtin_tts_send(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("tts_send", args, 0)?;
    let voice = expect_string_arg("tts_send", args, 1)?;
    let bot_token = expect_string_arg("tts_send", args, 2)?;
    let chat_id = expect_string_arg("tts_send", args, 3)?;
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return Err("tts_send(): OPENAI_API_KEY env var not set".to_string());
    }

    // Step 1: Call OpenAI TTS API
    let tts_body = serde_json::json!({
        "model": "tts-1",
        "input": text,
        "voice": voice,
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("tts_send(): client error: {}", e))?;

    let tts_resp = client.post("https://api.openai.com/v1/audio/speech")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .body(tts_body.to_string())
        .send()
        .map_err(|e| format!("tts_send(): TTS request failed: {}", e))?;

    let status = tts_resp.status().as_u16();
    if status >= 400 {
        let err_body = tts_resp.text().unwrap_or_default();
        return Err(format!("tts_send(): TTS API status {}: {}", status, err_body));
    }

    let audio_bytes = tts_resp.bytes()
        .map_err(|e| format!("tts_send(): failed to read TTS audio: {}", e))?;

    // Step 2: Send as voice note to Telegram via sendVoice
    // sendVoice (not sendAudio) displays as voice message bubble in Telegram.
    // Optional 5th arg "audio" switches back to sendAudio (audio player).
    let send_as = match args.get(4) {
        Some(Value::String(s)) if s == "audio" => "audio",
        _ => "voice",
    };
    let (field_name, endpoint) = match send_as {
        "audio" => ("audio", "sendAudio"),
        _ => ("voice", "sendVoice"),
    };
    let mut form = reqwest::blocking::multipart::Form::new();
    form = form.text("chat_id", chat_id.clone());
    let audio_part = reqwest::blocking::multipart::Part::bytes(audio_bytes.to_vec())
        .file_name("speech.ogg");
    form = form.part(field_name, audio_part);

    let tg_resp = client.post(format!("https://api.telegram.org/bot{}/{}", bot_token, endpoint))
        .multipart(form)
        .send()
        .map_err(|e| format!("tts_send(): Telegram {} failed: {}", endpoint, e))?;

    let tg_status = tg_resp.status().as_u16();
    let tg_body = tg_resp.text().unwrap_or_default();

    if tg_status >= 400 {
        return Err(format!("tts_send(): Telegram status {}: {}", tg_status, tg_body));
    }

    Ok(Value::String(tg_body))
}

// ── Наряд 17: Utility builtins ──────────────────────────────────

/// `base64_encode(s) -> String` — encode a string to base64.
fn builtin_base64_encode(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("base64_encode", args, 0)?;
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(s.as_bytes());
    Ok(Value::String(encoded))
}

/// `base64_decode(s) -> String` — decode a base64 string.
fn builtin_base64_decode(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("base64_decode", args, 0)?;
    use base64::Engine;
    match base64::engine::general_purpose::STANDARD.decode(s.as_bytes()) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(decoded) => Ok(Value::String(decoded)),
            Err(_) => Err("base64_decode(): decoded bytes are not valid UTF-8".to_string()),
        },
        Err(e) => Err(format!("base64_decode(): invalid base64: {}", e)),
    }
}

/// `exec(command) -> String` — execute a shell command and return stdout.
/// **Security**: Only available outside sandbox. Inside sandbox, always errors.
/// In server mode, command execution is disabled unless the binary is run with METALOGOS_ALLOW_EXEC=1.
fn builtin_exec(args: &[Value]) -> Result<Value, String> {
    // Security: disable in server context unless explicitly allowed
    if std::env::var("METALOGOS_ALLOW_EXEC").unwrap_or_default() != "1" {
        // Check if we're likely in server mode (has METALOGOS_PORT or METALOGOS_DB env)
        let in_server = std::env::var("METALOGOS_PORT").is_ok()
            || std::env::var("METALOGOS_DB").is_ok();
        if in_server {
            return Err("exec() is disabled in server mode. Set METALOGOS_ALLOW_EXEC=1 to enable.".to_string());
        }
    }

    let cmd = expect_string_arg("exec", args, 0)?;
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .output()
        .map_err(|e| format!("exec(): failed to run command: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("exec() command exited with {}: {}", output.status, stderr.trim()));
    }
    Ok(Value::String(stdout))
}

/// `escape_js(s) -> String` — escape a string for safe insertion into JavaScript.
/// Escapes: backslash, single quote, double quote, newline, carriage return, tab,
/// line separator, paragraph separator, and NUL.
fn builtin_escape_js(args: &[Value]) -> Result<Value, String> {
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
            '\u{2028}' => out.push_str("\\u2028"), // line separator
            '\u{2029}' => out.push_str("\\u2029"), // paragraph separator
            '\0' => out.push_str("\\0"),
            _ => out.push(c),
        }
    }
    Ok(Value::String(out))
}

/// `type_of(value) -> String` — returns the runtime type name as a String.
/// Useful for safe checking after json_get: `if type_of(x) == "Unit" { ... }`
fn builtin_type_of(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("type_of() requires 1 argument".to_string());
    }
    Ok(Value::String(args[0].type_name().to_string()))
}

// ── Наряд 24: New builtins (A3, A4, B2) ──────────────────────────────

/// `git_push(message?) -> String` — git add/commit/push via subprocess.
/// Uses GITHUB_TOKEN and GITHUB_REPO env vars for authentication.
/// Usage: git_push("commit message") -> "ok" | "nothing to commit" | error
fn builtin_git_push(args: &[Value]) -> Result<Value, String> {
    let message = match args.get(0) {
        Some(Value::String(s)) => s.clone(),
        _ => "Auto commit".to_string(),
    };

    let run = |cmd: &str, cmd_args: &[&str]| -> Result<String, String> {
        let output = std::process::Command::new(cmd)
            .args(cmd_args)
            .output()
            .map_err(|e| format!("git_push(): {} failed: {}", cmd, e))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(format!(
                "git_push(): {} exited with {}: {}",
                cmd,
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    };

    run("git", &["add", "."])?;

    // Check if there's anything to commit
    let status = run("git", &["status", "--porcelain"])?;
    if status.trim().is_empty() {
        return Ok(Value::String("nothing to commit".to_string()));
    }

    run("git", &["commit", "-m", &message])?;

    // Push using token from env
    let token = std::env::var("GITHUB_TOKEN").unwrap_or_default();
    let repo = std::env::var("GITHUB_REPO").unwrap_or_default();
    if token.is_empty() || repo.is_empty() {
        return Err("git_push(): GITHUB_TOKEN or GITHUB_REPO env var not set".to_string());
    }

    let remote = format!("https://{}@github.com/{}.git", token, repo);
    run("git", &["push", &remote, "main"])?;

    Ok(Value::String("ok".to_string()))
}

/// `web_search(query, num_results?) -> String` — search via SerpAPI.
/// Uses SERPAPI_KEY env var. Returns raw JSON string.
/// Usage: web_search("query") -> JSON string
/// Usage: web_search("query", 5) -> JSON string with 5 results
fn builtin_web_search(args: &[Value]) -> Result<Value, String> {
    let query = expect_string_arg("web_search", args, 0)?;
    let num: i32 = match args.get(1) {
        Some(Value::Float(n)) => *n as i32,
        _ => 10,
    };

    let api_key = std::env::var("SERPAPI_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return Err("web_search(): SERPAPI_KEY env var not set".to_string());
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("web_search(): client error: {}", e))?;

    let encoded_query = urlencoding::encode(&query);
    let url = format!(
        "https://serpapi.com/search.json?q={}&num={}&api_key={}&hl=ru",
        encoded_query, num, api_key
    );

    let resp = client
        .get(&url)
        .send()
        .map_err(|e| format!("web_search(): request failed: {}", e))?;

    let body = resp
        .text()
        .map_err(|e| format!("web_search(): failed to read response: {}", e))?;

    Ok(Value::String(body))
}

/// `make_list(a, b, c, ...) -> List` — create a list from variadic arguments.
/// Eliminates race conditions from write_file/read_file workarounds for
/// returning multiple values from patterns.
/// Usage: make_list("red", "green", "blue") -> List ["red", "green", "blue"]
fn builtin_make_list(args: &[Value]) -> Result<Value, String> {
    Ok(Value::List(args.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_struct(fields: Vec<(&str, Value)>) -> Value {
        let mut map = std::collections::HashMap::new();
        for (k, v) in fields {
            map.insert(k.to_string(), v);
        }
        Value::Struct { type_name: "Test".to_string(), fields: map }
    }

    fn is_string(val: &Value, expected: &str) -> bool {
        matches!(val, Value::String(s) if s == expected)
    }

    fn is_float(val: &Value, expected: f64) -> bool {
        matches!(val, Value::Float(f) if (*f - expected).abs() < 1e-9)
    }

    fn is_unit(val: &Value) -> bool {
        matches!(val, Value::Unit)
    }

    #[test]
    fn test_json_get_existing_field() {
        let obj = make_struct(vec![
            ("name", Value::String("Alice".to_string())),
            ("age", Value::Float(25.0)),
        ]);
        let result = builtin_json_get(&[obj, Value::String("name".to_string())]).unwrap();
        assert!(is_string(&result, "Alice"));
    }

    #[test]
    fn test_json_get_missing_field_returns_unit() {
        let obj = make_struct(vec![("name", Value::String("Alice".to_string()))]);
        let result = builtin_json_get(&[obj, Value::String("voice".to_string())]).unwrap();
        assert!(is_unit(&result), "expected Unit, got {:?}", result.type_name());
    }

    #[test]
    fn test_json_get_missing_field_returns_custom_default() {
        let obj = make_struct(vec![("name", Value::String("Alice".to_string()))]);
        let result = builtin_json_get(&[
            obj,
            Value::String("voice".to_string()),
            Value::String("none".to_string()),
        ]).unwrap();
        assert!(is_string(&result, "none"));
    }

    #[test]
    fn test_json_get_nested_path() {
        let inner = make_struct(vec![("file_id", Value::String("abc123".to_string()))]);
        let obj = make_struct(vec![("voice", inner)]);
        let result = builtin_json_get(&[
            obj,
            Value::String("voice.file_id".to_string()),
        ]).unwrap();
        assert!(is_string(&result, "abc123"));
    }

    #[test]
    fn test_json_get_nested_path_missing() {
        let obj = make_struct(vec![("text", Value::String("hello".to_string()))]);
        let result = builtin_json_get(&[
            obj,
            Value::String("voice.file_id".to_string()),
            Value::String("default".to_string()),
        ]).unwrap();
        assert!(is_string(&result, "default"));
    }

    #[test]
    fn test_json_get_non_struct_returns_default() {
        let obj = Value::String("not a struct".to_string());
        let result = builtin_json_get(&[
            obj,
            Value::String("field".to_string()),
            Value::Float(42.0),
        ]).unwrap();
        assert!(is_float(&result, 42.0));
    }

    #[test]
    fn test_has_field_existing() {
        let obj = make_struct(vec![("voice", Value::String("data".to_string()))]);
        let result = builtin_has_field(&[obj, Value::String("voice".to_string())]).unwrap();
        assert!(is_float(&result, 1.0));
    }

    #[test]
    fn test_has_field_missing() {
        let obj = make_struct(vec![("text", Value::String("hi".to_string()))]);
        let result = builtin_has_field(&[obj, Value::String("voice".to_string())]).unwrap();
        assert!(is_float(&result, 0.0));
    }

    #[test]
    fn test_has_field_nested() {
        let inner = make_struct(vec![("file_id", Value::String("x".to_string()))]);
        let obj = make_struct(vec![("voice", inner)]);
        let result = builtin_has_field(&[obj, Value::String("voice.file_id".to_string())]).unwrap();
        assert!(is_float(&result, 1.0));
    }

    #[test]
    fn test_json_encode_roundtrip() {
        let obj = make_struct(vec![
            ("key", Value::String("value".to_string())),
            ("num", Value::Float(42.0)),
        ]);
        let encoded = builtin_json_encode(&[obj]).unwrap();
        let decoded = builtin_parse_json(&[encoded]).unwrap();
        let key_val = decoded.get_field("key").unwrap();
        assert!(is_string(key_val, "value"));
        let num_val = decoded.get_field("num").unwrap();
        assert!(is_float(num_val, 42.0));
    }

    #[test]
    fn test_escape_json_handles_special_chars() {
        let result = builtin_escape_json(&[Value::String("hello\"world\n".to_string())]).unwrap();
        assert!(is_string(&result, "hello\\\"world\\n"));
    }
}

// ── Dict operations (Наряд №17 В.1) ─────────────────────────────
// Dicts are represented as Value::Struct with type_name "Dict".

/// `dict_set(dict, key, value)` — set a key in a dict. Returns the modified dict.
fn builtin_dict_set(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("dict_set() requires 3 arguments (dict, key, value)".to_string());
    }
    let key = expect_string_arg("dict_set", args, 1)?;
    let mut fields = match &args[0] {
        Value::Struct { fields, .. } => fields.clone(),
        other => return Err(format!("dict_set() expected Struct as first arg, got {}", other.type_name())),
    };
    fields.insert(key, args[2].clone());
    Ok(Value::Struct { type_name: "Dict".to_string(), fields })
}

/// `dict_keys(dict) -> List` — return list of keys.
fn builtin_dict_keys(args: &[Value]) -> Result<Value, String> {
    let fields = match &args.get(0) {
        Some(Value::Struct { fields, .. }) => fields,
        _ => return Err("dict_keys() requires 1 argument (Struct/Dict)".to_string()),
    };
    let keys: Vec<Value> = fields.keys().map(|k| Value::String(k.clone())).collect();
    Ok(Value::List(keys))
}

/// `dict_values(dict) -> List` — return list of values.
fn builtin_dict_values(args: &[Value]) -> Result<Value, String> {
    let fields = match &args.get(0) {
        Some(Value::Struct { fields, .. }) => fields,
        _ => return Err("dict_values() requires 1 argument (Struct/Dict)".to_string()),
    };
    let values: Vec<Value> = fields.values().cloned().collect();
    Ok(Value::List(values))
}

/// `dict_has(dict, key) -> Bool` — check if key exists.
fn builtin_dict_has(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("dict_has() requires 2 arguments (dict, key)".to_string());
    }
    let key = expect_string_arg("dict_has", args, 1)?;
    let has = match &args[0] {
        Value::Struct { fields, .. } => fields.contains_key(&key),
        _ => return Err(format!("dict_has() expected Struct as first arg, got {}", args[0].type_name())),
    };
    Ok(Value::Bool(has))
}

// ── Format (Наряд №17 В.3) ──────────────────────────────────────
/// `format(template, arg1, arg2, ...)` — positional string interpolation.
/// Replaces `{}` placeholders in template with arguments.
/// Usage: format("Hello {}, you are {} years old", name, age)
fn builtin_format(args: &[Value]) -> Result<Value, String> {
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
                return Err(format!("format(): not enough arguments for template (need {} more)", arg_idx - 1));
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

// ── v0.8.0 — format_date (enhanced) ──────────────────────
/// Weekday names: Monday=0 (converted from libc Sunday=0).
const WEEKDAY_NAMES_MON: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

/// Month names (1-indexed).
const MONTH_NAMES: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
];
const MONTH_ABBR: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun",
    "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `format_date(format)` — format current time.
/// `format_date(format, timestamp)` — format given unix timestamp (Float seconds).
/// v0.8.0: supports %y %I %p %A %a %B %b %j %w %W %% in addition to %Y %m %d %H %M %S %F %T %R.
fn builtin_format_date(args: &[Value]) -> Result<Value, String> {
    let fmt_str = if args.is_empty() {
        "%Y-%m-%d %H:%M:%S".to_string()
    } else {
        expect_string_arg("format_date", args, 0)?
    };

    let timestamp = if args.len() >= 2 {
        match &args[1] {
            Value::Float(f) => *f,
            _ => return Err(format!("format_date(): timestamp must be Float, got {}", args[1].type_name())),
        }
    } else {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    };

    let secs = timestamp as i64;
    let mut tm = libc::tm {
        tm_sec: 0, tm_min: 0, tm_hour: 0,
        tm_mday: 0, tm_mon: 0, tm_year: 0,
        tm_wday: 0, tm_yday: 0, tm_isdst: 0,
        tm_gmtoff: 0, tm_zone: std::ptr::null(),
    };
    unsafe { libc::localtime_r(&secs, &mut tm); }

    let y = (tm.tm_year + 1900) as u32;
    let mo = (tm.tm_mon + 1) as u32;
    let d = tm.tm_mday as u32;
    let h = tm.tm_hour as u32;
    let mi = tm.tm_min as u32;
    let s = tm.tm_sec as u32;
    let wday_mon = (tm.tm_wday as u32 + 6) % 7;
    let day_of_year = (tm.tm_yday + 1) as u32;
    let week_num = ((day_of_year as i32 + 6 - wday_mon as i32) / 7).max(1) as u32;
    let ampm = if h >= 12 { "PM" } else { "AM" };
    let h12 = if h == 0 { 12 } else if h > 12 { h - 12 } else { h };

    let result = match fmt_str.as_str() {
        "%F" => format!("{:04}-{:02}-{:02}", y, mo, d),
        "%T" => format!("{:02}:{:02}:{:02}", h, mi, s),
        "%R" => format!("{:02}:{:02}", h, mi),
        "%Y-%m-%d" => format!("{:04}-{:02}-{:02}", y, mo, d),
        "%d.%m.%Y" => format!("{:02}.{:02}.{:04}", d, mo, y),
        _ => {
            let mut out = String::new();
            let mut chars = fmt_str.chars().peekable();
            while let Some(ch) = chars.next() {
                if ch == '%' {
                    match chars.next() {
                        Some('Y') => out.push_str(&format!("{:04}", y)),
                        Some('y') => out.push_str(&format!("{:02}", y % 100)),
                        Some('m') => out.push_str(&format!("{:02}", mo)),
                        Some('d') => out.push_str(&format!("{:02}", d)),
                        Some('H') => out.push_str(&format!("{:02}", h)),
                        Some('I') => out.push_str(&format!("{:02}", h12)),
                        Some('M') => out.push_str(&format!("{:02}", mi)),
                        Some('S') => out.push_str(&format!("{:02}", s)),
                        Some('p') => out.push_str(ampm),
                        Some('A') => out.push_str(WEEKDAY_NAMES_MON[wday_mon as usize]),
                        Some('a') => out.push_str(&WEEKDAY_NAMES_MON[wday_mon as usize][..3]),
                        Some('B') => out.push_str(MONTH_NAMES[(mo - 1) as usize]),
                        Some('b') => out.push_str(MONTH_ABBR[(mo - 1) as usize]),
                        Some('j') => out.push_str(&format!("{:03}", day_of_year)),
                        Some('w') => out.push_str(&format!("{}", wday_mon)),
                        Some('W') => out.push_str(&format!("{:02}", week_num)),
                        Some('%') => out.push('%'),
                        Some('F') => out.push_str(&format!("{:04}-{:02}-{:02}", y, mo, d)),
                        Some('T') => out.push_str(&format!("{:02}:{:02}:{:02}", h, mi, s)),
                        Some('R') => out.push_str(&format!("{:02}:{:02}", h, mi)),
                        Some(c) => { out.push('%'); out.push(c); }
                        None => out.push('%'),
                    }
                } else {
                    out.push(ch);
                }
            }
            out
        }
    };
    Ok(Value::String(result))
}

// ── v0.8.0 — Additional Time / Date / Calendar builtins ──────────

fn date_is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn date_days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if date_is_leap(year) { 29 } else { 28 },
        _ => 30,
    }
}

fn make_date_struct(type_name: &str, pairs: Vec<(&str, Value)>) -> Value {
    let mut map = std::collections::HashMap::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), v);
    }
    Value::Struct { type_name: type_name.to_string(), fields: map }
}

/// `date_parts(timestamp?)` — returns struct with all date components via libc.
fn builtin_date_parts(args: &[Value]) -> Result<Value, String> {
    let ts = if args.is_empty() {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    } else {
        expect_float_arg("date_parts", args, 0)?
    };
    let secs = ts as i64;
    let mut tm = libc::tm {
        tm_sec: 0, tm_min: 0, tm_hour: 0,
        tm_mday: 0, tm_mon: 0, tm_year: 0,
        tm_wday: 0, tm_yday: 0, tm_isdst: 0,
        tm_gmtoff: 0, tm_zone: std::ptr::null(),
    };
    unsafe { libc::localtime_r(&secs, &mut tm); }

    let y = (tm.tm_year + 1900) as u32;
    let mo = (tm.tm_mon + 1) as u32;
    let d = tm.tm_mday as u32;
    let h = tm.tm_hour as u32;
    let mi = tm.tm_min as u32;
    let s = tm.tm_sec as u32;
    let wday_mon = (tm.tm_wday as u32 + 6) % 7;
    let day_of_year = (tm.tm_yday + 1) as u32;
    let week_num = ((day_of_year as i32 + 6 - wday_mon as i32) / 7).max(1) as u32;

    Ok(make_date_struct("Date", vec![
        ("year", Value::Float(y as f64)),
        ("month", Value::Float(mo as f64)),
        ("day", Value::Float(d as f64)),
        ("hour", Value::Float(h as f64)),
        ("minute", Value::Float(mi as f64)),
        ("second", Value::Float(s as f64)),
        ("weekday", Value::Float(wday_mon as f64)),
        ("weekday_name", Value::String(WEEKDAY_NAMES_MON[wday_mon as usize].to_string())),
        ("month_name", Value::String(MONTH_NAMES[(mo - 1) as usize].to_string())),
        ("day_of_year", Value::Float(day_of_year as f64)),
        ("week_number", Value::Float(week_num as f64)),
        ("timestamp", Value::Float(ts)),
    ]))
}

/// `days_between(ts1, ts2)` — absolute difference in days.
fn builtin_days_between(args: &[Value]) -> Result<Value, String> {
    let ts1 = expect_float_arg("days_between", args, 0)?;
    let ts2 = expect_float_arg("days_between", args, 1)?;
    Ok(Value::Float((ts1 - ts2).abs() / 86400.0))
}

/// `days_in_month(year, month)` — days in given month (1-12).
fn builtin_days_in_month(args: &[Value]) -> Result<Value, String> {
    let year = expect_float_arg("days_in_month", args, 0)? as i32;
    let month = expect_float_arg("days_in_month", args, 1)? as u32;
    if month < 1 || month > 12 {
        return Err("days_in_month() month must be 1-12".to_string());
    }
    Ok(Value::Float(date_days_in_month(year, month) as f64))
}

/// `is_leap_year(year)` — Gregorian leap year check.
fn builtin_is_leap_year(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(date_is_leap(expect_float_arg("is_leap_year", args, 0)? as i32)))
}

/// `add_days(timestamp, days)` — add/subtract days to timestamp.
fn builtin_add_days(args: &[Value]) -> Result<Value, String> {
    let ts = expect_float_arg("add_days", args, 0)?;
    let days = expect_float_arg("add_days", args, 1)?;
    Ok(Value::Float(ts + days * 86400.0))
}

/// `add_hours(timestamp, hours)` — add/subtract hours to timestamp.
fn builtin_add_hours(args: &[Value]) -> Result<Value, String> {
    let ts = expect_float_arg("add_hours", args, 0)?;
    let hours = expect_float_arg("add_hours", args, 1)?;
    Ok(Value::Float(ts + hours * 3600.0))
}

/// `weekday_name(timestamp)` — full weekday name ("Monday".."Sunday").
fn builtin_weekday_name(args: &[Value]) -> Result<Value, String> {
    let ts = expect_float_arg("weekday_name", args, 0)?;
    let secs = ts as i64;
    let mut tm = libc::tm {
        tm_sec: 0, tm_min: 0, tm_hour: 0,
        tm_mday: 0, tm_mon: 0, tm_year: 0,
        tm_wday: 0, tm_yday: 0, tm_isdst: 0,
        tm_gmtoff: 0, tm_zone: std::ptr::null(),
    };
    unsafe { libc::localtime_r(&secs, &mut tm); }
    let wday_mon = (tm.tm_wday as u32 + 6) % 7;
    Ok(Value::String(WEEKDAY_NAMES_MON[wday_mon as usize].to_string()))
}

// ── v0.8.0 — Geolocation builtins ───────────────────────────────────

/// `geo_ip(ip?)` — geolocate by IP. Uses ip-api.com (free, no key).
/// Returns Struct {ip, city, region, country, country_code, lat, lon, isp, timezone}.
fn builtin_geo_ip(args: &[Value]) -> Result<Value, String> {
    let ip = match args.get(0) {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        _ => String::new(),
    };
    let url = if ip.is_empty() {
        "http://ip-api.com/json/".to_string()
    } else {
        format!("http://ip-api.com/json/{}", ip)
    };
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("geo_ip() client error: {}", e))?;
    let resp = client.get(&url).send()
        .map_err(|e| format!("geo_ip() request failed: {}", e))?;
    let body = resp.text().unwrap_or_default();
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("geo_ip() parse error: {}", e))?;
    if json.get("status").and_then(|v| v.as_str()) != Some("success") {
        let msg = json.get("message").and_then(|v| v.as_str()).unwrap_or("unknown error");
        return Err(format!("geo_ip() API error: {}", msg));
    }
    let g = |key: &str| -> Value {
        json.get(key).map(|v| match v {
            serde_json::Value::String(s) => Value::String(s.clone()),
            serde_json::Value::Number(n) => Value::Float(n.as_f64().unwrap_or(0.0)),
            serde_json::Value::Null => Value::String(String::new()),
            _ => Value::String(v.to_string()),
        }).unwrap_or(Value::String(String::new()))
    };
    Ok(make_date_struct("GeoLocation", vec![
        ("ip", g("query")), ("city", g("city")), ("region", g("regionName")),
        ("country", g("country")), ("country_code", g("countryCode")),
        ("lat", g("lat")), ("lon", g("lon")),
        ("isp", g("isp")), ("timezone", g("timezone")),
    ]))
}

/// `geo_distance(lat1, lon1, lat2, lon2, unit?)` — haversine distance. unit: "km"(default), "mi", "nm", "m".
fn builtin_geo_distance(args: &[Value]) -> Result<Value, String> {
    let lat1 = expect_float_arg("geo_distance", args, 0)?;
    let lon1 = expect_float_arg("geo_distance", args, 1)?;
    let lat2 = expect_float_arg("geo_distance", args, 2)?;
    let lon2 = expect_float_arg("geo_distance", args, 3)?;
    let unit = match args.get(4) { Some(Value::String(s)) => s.as_str(), _ => "km" };
    let to_rad = |d: f64| d * std::f64::consts::PI / 180.0;
    let dlat = to_rad(lat2 - lat1);
    let dlon = to_rad(lon2 - lon1);
    let a = (dlat/2.0).sin().powi(2) + to_rad(lat1).cos() * to_rad(lat2).cos() * (dlon/2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    let km = 6371.0 * c;
    Ok(Value::Float(match unit {
        "mi" => km * 0.621371, "nm" => km * 0.539957, "m" => km * 1000.0, _ => km,
    }))
}

// ── v0.8.0 — Weather builtins (Open-Meteo, free, no API key) ───────

/// WMO weather codes to human-readable description.
fn wmo_description(code: i64) -> &'static str {
    match code {
        0 => "Clear sky", 1 => "Mainly clear", 2 => "Partly cloudy", 3 => "Overcast",
        45 | 48 => "Fog", 51 | 53 | 55 => "Drizzle", 56 | 57 => "Freezing drizzle",
        61 | 63 | 65 => "Rain", 66 | 67 => "Freezing rain",
        71 | 73 | 75 => "Snow fall", 77 => "Snow grains",
        80 | 81 | 82 => "Rain showers", 85 | 86 => "Snow showers",
        95 => "Thunderstorm", 96 | 99 => "Thunderstorm with hail",
        _ => "Unknown",
    }
}

/// Resolve city name to (lat, lon) via Open-Meteo geocoding (free, no key).
fn geo_resolve_city(city: &str) -> Result<(f64, f64), String> {
    let url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=en&format=json",
        urlencoding::encode(city)
    );
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build().map_err(|e| format!("weather() client error: {}", e))?;
    let body = client.get(&url).send()
        .map_err(|e| format!("weather() geocoding failed: {}", e))?
        .text().unwrap_or_default();
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("weather() geocoding parse error: {}", e))?;
    let results = json.get("results").and_then(|r| r.as_array())
        .ok_or_else(|| format!("weather() city not found: {}", city))?;
    if results.is_empty() {
        return Err(format!("weather() city not found: {}", city));
    }
    let first = &results[0];
    let lat = first.get("latitude").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let lon = first.get("longitude").and_then(|v| v.as_f64()).unwrap_or(0.0);
    Ok((lat, lon))
}

/// `weather(city_or_lat, lon?)` — current weather via Open-Meteo (FREE, no API key).
/// `weather("Minsk")` or `weather(53.9, 27.57)`.
/// Returns Struct {temp, feels_like, temp_min, temp_max, humidity, description,
///   wind_speed, wind_direction, pressure, cloud_cover, is_day, city, country}.
fn builtin_weather(args: &[Value]) -> Result<Value, String> {
    let (lat, lon, resolved_city) = if args.len() >= 2 {
        let lat = expect_float_arg("weather", args, 0)?;
        let lon = expect_float_arg("weather", args, 1)?;
        (lat, lon, String::new())
    } else {
        let city = expect_string_arg("weather", args, 0)?;
        let (lat, lon) = geo_resolve_city(&city)?;
        (lat, lon, city)
    };
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m,wind_direction_10m,surface_pressure,cloud_cover,is_day&timezone=auto",
        lat, lon
    );
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build().map_err(|e| format!("weather() client error: {}", e))?;
    let resp = client.get(&url).send()
        .map_err(|e| format!("weather() request failed: {}", e))?;
    let status = resp.status().as_u16();
    let body = resp.text().unwrap_or_default();
    if status >= 400 {
        return Err(format!("weather() API error {}: {}", status, body));
    }
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("weather() parse error: {}", e))?;
    let cur = json.get("current").ok_or("weather() missing 'current' in response")?;
    let gf = |key: &str| -> f64 {
        cur.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0)
    };
    let code = gf("weather_code") as i64;
    let desc = wmo_description(code).to_string();
    Ok(make_date_struct("Weather", vec![
        ("temp", Value::Float(gf("temperature_2m"))),
        ("feels_like", Value::Float(gf("apparent_temperature"))),
        ("temp_min", Value::Float(gf("temperature_2m"))),
        ("temp_max", Value::Float(gf("temperature_2m"))),
        ("humidity", Value::Float(gf("relative_humidity_2m"))),
        ("description", Value::String(desc)),
        ("wind_speed", Value::Float(gf("wind_speed_10m"))),
        ("wind_direction", Value::Float(gf("wind_direction_10m"))),
        ("pressure", Value::Float(gf("surface_pressure"))),
        ("cloud_cover", Value::Float(gf("cloud_cover"))),
        ("is_day", Value::Float(gf("is_day"))),
        ("city", Value::String(resolved_city)),
        ("country", Value::String(String::new())),
    ]))
}

/// `weather_forecast(city_or_lat, lon?, days?)` — multi-day forecast via Open-Meteo (FREE, no API key).
/// `weather_forecast("Minsk", 7)` or `weather_forecast(53.9, 27.57, 3)`.
/// Default: 7 days. Max: 16 days. Returns List of DayForecast structs.
fn builtin_weather_forecast(args: &[Value]) -> Result<Value, String> {
    let (lat, lon) = if args.len() >= 2 && matches!(&args[1], Value::Float(_)) {
        let la = expect_float_arg("weather_forecast", args, 0)?;
        let lo = expect_float_arg("weather_forecast", args, 1)?;
        (la, lo)
    } else {
        let city = expect_string_arg("weather_forecast", args, 0)?;
        geo_resolve_city(&city)?
    };
    let mut days: u32 = 7;
    if args.len() == 2 && matches!(&args[1], Value::Float(_)) {
        days = expect_float_arg("weather_forecast", args, 1)? as u32;
    } else if args.len() >= 3 {
        days = expect_float_arg("weather_forecast", args, 2)? as u32;
    }
    if days < 1 { days = 1; }
    if days > 16 { days = 16; }
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_sum,wind_speed_10m_max,sunrise,sunset,uv_index_max&timezone=auto&forecast_days={}",
        lat, lon, days
    );
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build().map_err(|e| format!("weather_forecast() client error: {}", e))?;
    let resp = client.get(&url).send()
        .map_err(|e| format!("weather_forecast() request failed: {}", e))?;
    let status = resp.status().as_u16();
    let body = resp.text().unwrap_or_default();
    if status >= 400 {
        return Err(format!("weather_forecast() API error {}: {}", status, body));
    }
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("weather_forecast() parse error: {}", e))?;
    let daily = json.get("daily").ok_or("weather_forecast() missing 'daily' in response")?;
    let dates = daily.get("time").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let codes = daily.get("weather_code").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let tmax = daily.get("temperature_2m_max").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let tmin = daily.get("temperature_2m_min").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let precip = daily.get("precipitation_sum").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let wind = daily.get("wind_speed_10m_max").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let sunrise = daily.get("sunrise").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let sunset = daily.get("sunset").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let uv = daily.get("uv_index_max").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let gf = |arr: &[serde_json::Value], i: usize| -> f64 {
        arr.get(i).and_then(|v| v.as_f64()).unwrap_or(0.0)
    };
    let gs = |arr: &[serde_json::Value], i: usize| -> String {
        arr.get(i).and_then(|v| v.as_str()).unwrap_or("").to_string()
    };
    let mut result = Vec::new();
    for i in 0..dates.len() {
        let code = gf(&codes, i) as i64;
        let desc = wmo_description(code).to_string();
        let uv_val = gf(&uv, i);
        let uv_str = if uv_val < 0.0 { String::new() } else { format!("{:.1}", uv_val) };
        result.push(make_date_struct("DayForecast", vec![
            ("date", Value::String(gs(&dates, i))),
            ("temp_max", Value::Float(gf(&tmax, i))),
            ("temp_min", Value::Float(gf(&tmin, i))),
            ("precipitation", Value::Float(gf(&precip, i))),
            ("weather_code", Value::Float(gf(&codes, i))),
            ("description", Value::String(desc)),
            ("wind_speed_max", Value::Float(gf(&wind, i))),
            ("sunrise", Value::String(gs(&sunrise, i))),
            ("sunset", Value::String(gs(&sunset, i))),
            ("uv_index", Value::String(uv_str)),
        ]));
    }
    Ok(Value::List(result))
}

// ── v0.8.0 — Reminders builtins ─────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct ReminderEntry {
    id: String, message: String, fire_at: f64, interval: f64,
    next_fire: f64, data: String, active: bool, created_at: f64,
}

static REMINDERS: std::sync::OnceLock<StdMutex<Vec<ReminderEntry>>> = std::sync::OnceLock::new();

fn reminders_store() -> &'static StdMutex<Vec<ReminderEntry>> {
    REMINDERS.get_or_init(|| StdMutex::new(Vec::new()))
}

/// Global SQLite persistence for reminders (same pattern as KV_SQLITE).
static REMINDERS_SQLITE: std::sync::OnceLock<StdMutex<Option<rusqlite::Connection>>> = std::sync::OnceLock::new();

fn reminders_sqlite() -> &'static StdMutex<Option<rusqlite::Connection>> {
    REMINDERS_SQLITE.get_or_init(|| StdMutex::new(None))
}

/// Initialize SQLite persistence for reminders. Called from server.rs on startup.
pub fn init_reminder_persist(db_path: &str) -> Result<(), String> {
    let conn = rusqlite::Connection::open(db_path)
        .map_err(|e| format!("[reminders] Failed to open database '{}': {}", db_path, e))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS reminders (
            id TEXT PRIMARY KEY,
            message TEXT NOT NULL,
            fire_at REAL NOT NULL,
            interval REAL NOT NULL DEFAULT 0,
            next_fire REAL NOT NULL,
            data TEXT NOT NULL DEFAULT '',
            active INTEGER NOT NULL DEFAULT 1,
            created_at REAL NOT NULL
        );"
    ).map_err(|e| format!("[reminders] Failed to create table: {}", e))?;
    // Load existing reminders into memory
    let mut stmt = conn.prepare("SELECT id, message, fire_at, interval, next_fire, data, active, created_at FROM reminders")
        .map_err(|e| format!("[reminders] Failed to query: {}", e))?;
    let rows: Vec<ReminderEntry> = stmt.query_map([], |row| {
        Ok(ReminderEntry {
            id: row.get::<_, String>(0)?,
            message: row.get::<_, String>(1)?,
            fire_at: row.get::<_, f64>(2)?,
            interval: row.get::<_, f64>(3)?,
            next_fire: row.get::<_, f64>(4)?,
            data: row.get::<_, String>(5)?,
            active: row.get::<_, i32>(6)? != 0,
            created_at: row.get::<_, f64>(7)?,
        })
    }).map_err(|e| format!("[reminders] Failed to iterate: {}", e))?
    .filter_map(|r| r.ok())
    .collect();
    if let Ok(mut store) = reminders_store().lock() {
        store.extend(rows);
    }
    let mut guard = reminders_sqlite().lock().map_err(|e| format!("[reminders] lock error: {}", e))?;
    *guard = Some(conn);
    eprintln!("[reminders] SQLite persistence enabled: {}", db_path);
    Ok(())
}

/// Write a single reminder to SQLite (write-through, called after mutations).
fn reminder_sqlite_upsert(entry: &ReminderEntry) {
    if let Ok(guard) = reminders_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO reminders (id, message, fire_at, interval, next_fire, data, active, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                rusqlite::params![entry.id, entry.message, entry.fire_at, entry.interval, entry.next_fire, entry.data, entry.active as i32, entry.created_at],
            );
        }
    }
}

fn reminder_sqlite_delete(id: &str) {
    if let Ok(guard) = reminders_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute("DELETE FROM reminders WHERE id = ?1", rusqlite::params![id]);
        }
    }
}

fn reminder_sqlite_delete_all_for_persona() {
    if let Ok(guard) = reminders_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute("DELETE FROM reminders", []);
        }
    }
}

/// `remind(message, timestamp, data?)` — one-time reminder. Returns ID.
fn builtin_remind(args: &[Value]) -> Result<Value, String> {
    let message = expect_string_arg("remind", args, 0)?;
    let fire_at = expect_float_arg("remind", args, 1)?;
    let data = match args.get(2) { Some(Value::String(s)) => s.clone(), Some(o) => format!("{}", o), None => String::new() };
    let now_ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
    let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let mut store = reminders_store().lock().map_err(|e| format!("remind() lock error: {}", e))?;
    let entry = ReminderEntry { id: id.clone(), message, fire_at, interval: 0.0, next_fire: fire_at, data, active: true, created_at: now_ts };
    reminder_sqlite_upsert(&entry);
    store.push(entry);
    Ok(Value::String(id))
}

/// `remind_recurring(message, interval_seconds, data?)` — recurring reminder. Returns ID.
fn builtin_remind_recurring(args: &[Value]) -> Result<Value, String> {
    let message = expect_string_arg("remind_recurring", args, 0)?;
    let interval = expect_float_arg("remind_recurring", args, 1)?;
    if interval <= 0.0 { return Err("remind_recurring() interval must be positive".to_string()); }
    let data = match args.get(2) { Some(Value::String(s)) => s.clone(), Some(o) => format!("{}", o), None => String::new() };
    let now_ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
    let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let mut store = reminders_store().lock().map_err(|e| format!("remind_recurring() lock error: {}", e))?;
    let entry = ReminderEntry { id: id.clone(), message, fire_at: now_ts, interval, next_fire: now_ts + interval, data, active: true, created_at: now_ts };
    reminder_sqlite_upsert(&entry);
    store.push(entry);
    Ok(Value::String(id))
}

/// `cancel_remind(id)` — cancel reminder. Returns "ok" or "not_found".
fn builtin_cancel_remind(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("cancel_remind", args, 0)?;
    let mut store = reminders_store().lock().map_err(|e| format!("cancel_remind() lock error: {}", e))?;
    for entry in store.iter_mut() {
        if entry.id == id && entry.active {
            entry.active = false;
            reminder_sqlite_upsert(entry);
            return Ok(Value::String("ok".to_string()));
        }
    }
    Ok(Value::String("not_found".to_string()))
}

/// `list_reminders()` — list all active reminders.
fn builtin_list_reminders(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let store = reminders_store().lock().map_err(|e| format!("list_reminders() lock error: {}", e))?;
    let mut result = Vec::new();
    for entry in store.iter().filter(|r| r.active) {
        let rtype = if entry.interval > 0.0 { "recurring" } else { "once" };
        let ec = entry.clone();
        result.push(make_date_struct("Reminder", vec![
            ("id", Value::String(ec.id)), ("message", Value::String(ec.message)),
            ("fire_at", Value::Float(ec.fire_at)), ("interval", Value::Float(ec.interval)),
            ("next_fire", Value::Float(ec.next_fire)), ("data", Value::String(ec.data)),
            ("created_at", Value::Float(ec.created_at)), ("type", Value::String(rtype.to_string())),
        ]));
    }
    Ok(Value::List(result))
}

/// `check_reminders()` — get due reminders. One-shot deactivated; recurring advanced.
fn builtin_check_reminders(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let now_ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
    let mut store = reminders_store().lock().map_err(|e| format!("check_reminders() lock error: {}", e))?;
    let mut due = Vec::new();
    for entry in store.iter_mut() {
        if !entry.active { continue; }
        if now_ts >= entry.next_fire {
            let rtype = if entry.interval > 0.0 { "recurring" } else { "once" };
            due.push(make_date_struct("DueReminder", vec![
                ("id", Value::String(entry.id.clone())), ("message", Value::String(entry.message.clone())),
                ("data", Value::String(entry.data.clone())), ("type", Value::String(rtype.to_string())),
                ("next_fire", Value::Float(entry.next_fire)), ("overdue_seconds", Value::Float(now_ts - entry.next_fire)),
            ]));
            if entry.interval > 0.0 { entry.next_fire += entry.interval; } else { entry.active = false; }
            reminder_sqlite_upsert(entry);
        }
    }
    Ok(Value::List(due))
}

// ── v0.8.1 — OpenHuman-inspired Human Intelligence builtins ──────────
// Inspired by https://github.com/tinyhumansai/OpenHuman — memory tree,
// persona system, mood tracking, human-like AI responses.
// All built on top of existing Metalogos primitives (KV store, call_llm).
// No external dependencies, no API keys required beyond LLM provider.

/// `human_create(name, traits)` — create or update a persona.
/// `traits` is a string describing personality: "friendly, professional, speaks Russian".
/// Stores persona in KV under `human_persona:{name}`.
/// Returns Struct {name, traits, created_at, memory_count}.
fn builtin_human_create(args: &[Value]) -> Result<Value, String> {
    let name = expect_string_arg("human_create", args, 0)?;
    let traits = expect_string_arg("human_create", args, 1)?;
    if name.is_empty() {
        return Err("human_create() name cannot be empty".to_string());
    }
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64()).unwrap_or(0.0);
    let persona_data = serde_json::json!({
        "name": name,
        "traits": traits,
        "created_at": now_ts,
        "mood": "neutral",
        "mood_intensity": 0.5,
    });
    let key = format!("human_persona:{}", name);
    let value = serde_json::to_string(&persona_data)
        .map_err(|e| format!("human_create() serialize error: {}", e))?;
    let mut store = kv_store().lock().map_err(|e| format!("human_create() lock error: {}", e))?;
    store.insert(key.clone(), value.clone());
    if let Ok(sqlite_guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *sqlite_guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, value],
            );
        }
    }
    // Count existing memories for this persona
    let mem_prefix = format!("human_mem:{}:", name);
    let mem_count = store.keys().filter(|k| k.starts_with(&mem_prefix)).count();
    Ok(make_date_struct("Persona", vec![
        ("name", Value::String(name)),
        ("traits", Value::String(traits)),
        ("created_at", Value::Float(now_ts)),
        ("memory_count", Value::Float(mem_count as f64)),
    ]))
}

/// `human_mood(persona, mood?, intensity?)` — get or set persona's emotional state.
/// With 1 arg: returns current mood as Struct {mood, intensity, updated_at}.
/// With 2+ args: sets mood. `intensity` is 0.0–1.0 (default 0.5).
/// `mood` examples: "happy", "sad", "focused", "creative", "neutral", "excited".
fn builtin_human_mood(args: &[Value]) -> Result<Value, String> {
    let persona = expect_string_arg("human_mood", args, 0)?;
    let key = format!("human_persona:{}", persona);
    let store = kv_store().lock().map_err(|e| format!("human_mood() lock error: {}", e))?;
    let mut data_str = store.get(&key).cloned().unwrap_or_default();
    drop(store);
    if data_str.is_empty() {
        return Err(format!("human_mood() persona '{}' not found. Use human_create() first.", persona));
    }
    let mut data: serde_json::Value = serde_json::from_str(&data_str)
        .map_err(|e| format!("human_mood() parse error: {}", e))?;
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64()).unwrap_or(0.0);

    // If mood argument provided — set mood
    if args.len() >= 2 {
        let mood = expect_string_arg("human_mood", args, 1)?;
        let intensity = if args.len() >= 3 {
            expect_float_arg("human_mood", args, 2)?.clamp(0.0, 1.0)
        } else {
            0.5
        };
        data["mood"] = serde_json::Value::String(mood.clone());
        data["mood_intensity"] = serde_json::Value::Number(serde_json::Number::from_f64(intensity).unwrap_or(serde_json::Number::from(0.5)));
        data["mood_updated_at"] = serde_json::json!(now_ts);
        let updated = serde_json::to_string(&data)
            .map_err(|e| format!("human_mood() serialize error: {}", e))?;
        let mut store = kv_store().lock().map_err(|e| format!("human_mood() lock error: {}", e))?;
        store.insert(key.clone(), updated.clone());
        if let Ok(sqlite_guard) = kv_sqlite().lock() {
            if let Some(ref conn) = *sqlite_guard {
                let _ = conn.execute(
                    "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                    rusqlite::params![key, updated],
                );
            }
        }
    }

    let mood = data.get("mood").and_then(|v| v.as_str()).unwrap_or("neutral").to_string();
    let intensity = data.get("mood_intensity").and_then(|v| v.as_f64()).unwrap_or(0.5);
    let updated_at = data.get("mood_updated_at").and_then(|v| v.as_f64()).unwrap_or(0.0);
    Ok(make_date_struct("Mood", vec![
        ("persona", Value::String(persona)),
        ("mood", Value::String(mood)),
        ("intensity", Value::Float(intensity)),
        ("updated_at", Value::Float(updated_at)),
    ]))
}

/// `human_remember(persona, key, content, importance?)` — store a memory in persona's memory tree.
/// `importance` is 0.0–1.0 (default 0.5). Higher importance = recalled first.
/// Stores as KV entry `human_mem:{persona}:{key}` with metadata.
fn builtin_human_remember(args: &[Value]) -> Result<Value, String> {
    let persona = expect_string_arg("human_remember", args, 0)?;
    let key = expect_string_arg("human_remember", args, 1)?;
    let content = expect_string_arg("human_remember", args, 2)?;
    if key.is_empty() {
        return Err("human_remember() key cannot be empty".to_string());
    }
    let importance = if args.len() >= 4 {
        expect_float_arg("human_remember", args, 3)?.clamp(0.0, 1.0)
    } else { 0.5 };
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64()).unwrap_or(0.0);
    let mem_data = serde_json::json!({
        "persona": persona,
        "key": key,
        "content": content,
        "importance": importance,
        "created_at": now_ts,
        "access_count": 0,
        "last_accessed": now_ts,
    });
    let store_key = format!("human_mem:{}:{}", persona, key);
    let value = serde_json::to_string(&mem_data)
        .map_err(|e| format!("human_remember() serialize error: {}", e))?;
    let mut store = kv_store().lock().map_err(|e| format!("human_remember() lock error: {}", e))?;
    store.insert(store_key.clone(), value.clone());
    if let Ok(sqlite_guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *sqlite_guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                rusqlite::params![store_key, value],
            );
        }
    }
    Ok(Value::String("ok".to_string()))
}

/// `human_forget(persona, key?)` — delete a specific memory or all memories for a persona.
/// With 2 args: deletes specific memory by key. Returns "ok" or "not_found".
/// With 1 arg: deletes ALL memories for persona. Returns count of deleted memories.
fn builtin_human_forget(args: &[Value]) -> Result<Value, String> {
    let persona = expect_string_arg("human_forget", args, 0)?;
    let prefix = format!("human_mem:{}:", persona);
    let mut store = kv_store().lock().map_err(|e| format!("human_forget() lock error: {}", e))?;

    if args.len() >= 2 {
        let key = expect_string_arg("human_forget", args, 1)?;
        let store_key = format!("human_mem:{}:{}", persona, key);
        if store.remove(&store_key).is_some() {
            if let Ok(sqlite_guard) = kv_sqlite().lock() {
                if let Some(ref conn) = *sqlite_guard {
                    let _ = conn.execute("DELETE FROM kv_store WHERE key = ?1", rusqlite::params![store_key]);
                }
            }
            Ok(Value::String("ok".to_string()))
        } else {
            Ok(Value::String("not_found".to_string()))
        }
    } else {
        // Delete all memories for this persona
        let to_remove: Vec<String> = store.keys().filter(|k| k.starts_with(&prefix)).cloned().collect();
        let count = to_remove.len();
        for k in &to_remove {
            store.remove(k);
        }
        if let Ok(sqlite_guard) = kv_sqlite().lock() {
            if let Some(ref conn) = *sqlite_guard {
                for k in &to_remove {
                    let _ = conn.execute("DELETE FROM kv_store WHERE key = ?1", rusqlite::params![k]);
                }
            }
        }
        Ok(Value::Float(count as f64))
    }
}

/// `human_recall(persona, query, limit?)` — search persona's memories by keyword match.
/// Returns List of Memory structs sorted by importance (descending), then by recency.
/// Each struct: {key, content, importance, created_at, access_count, relevance}.
fn builtin_human_recall(args: &[Value]) -> Result<Value, String> {
    let persona = expect_string_arg("human_recall", args, 0)?;
    let query = expect_string_arg("human_recall", args, 1)?;
    let limit: usize = if args.len() >= 3 {
        expect_float_arg("human_recall", args, 2)? as usize
    } else { 10 };
    let prefix = format!("human_mem:{}:", persona);
    let query_lower = query.to_lowercase();
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64()).unwrap_or(0.0);

    let store = kv_store().lock().map_err(|e| format!("human_recall() lock error: {}", e))?;
    let mut memories: Vec<(f64, f64, Value)> = Vec::new(); // (importance, recency, struct)

    for (k, v) in store.iter() {
        if !k.starts_with(&prefix) { continue; }
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(v) {
            let content = data.get("content").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
            let key_str = data.get("key").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
            // Simple relevance scoring: keyword match in content or key
            let query_words: Vec<&str> = query_lower.split_whitespace().collect();
            let mut matches = 0;
            for word in &query_words {
                if content.contains(word) || key_str.contains(word) {
                    matches += 1;
                }
            }
            let relevance = if query_words.is_empty() { 0.5 } else { matches as f64 / query_words.len() as f64 };
            if relevance < 0.01 && !query.is_empty() { continue; } // skip non-matching if query given

            let importance = data.get("importance").and_then(|v| v.as_f64()).unwrap_or(0.5);
            let created_at = data.get("created_at").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let access_count = data.get("access_count").and_then(|v| v.as_i64()).unwrap_or(0) as f64;
            let age_hours = (now_ts - created_at).max(0.0) / 3600.0;
            // Recency score: 1.0 for fresh, decays over time (half-life ~168h = 1 week)
            let recency = (0.5_f64).powf(age_hours / 168.0);
            // Composite score: 50% relevance, 30% importance, 20% recency
            let score = relevance * 0.5 + importance * 0.3 + recency * 0.2;

            let mem_key = data.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let mem_content = data.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let mem_struct = make_date_struct("Memory", vec![
                ("key", Value::String(mem_key)),
                ("content", Value::String(mem_content)),
                ("importance", Value::Float(importance)),
                ("created_at", Value::Float(created_at)),
                ("access_count", Value::Float(access_count)),
                ("relevance", Value::Float(relevance)),
                ("score", Value::Float(score)),
            ]);
            memories.push((score, recency, mem_struct));
        }
    }
    drop(store);

    // Sort by score descending, take top N
    memories.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    memories.truncate(limit);

    let results: Vec<Value> = memories.into_iter().map(|(_, _, v)| v).collect();
    Ok(Value::List(results))
}

/// `human_respond(persona, message, context?)` — generate a human-like response.
/// Uses the persona's traits, mood, and recalled memories to craft a response via LLM.
/// `context` is optional additional context (e.g., conversation history).
/// Returns the generated response as String.
fn builtin_human_respond(args: &[Value]) -> Result<Value, String> {
    let persona = expect_string_arg("human_respond", args, 0)?;
    let message = expect_string_arg("human_respond", args, 1)?;
    let context = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    };

    // Load persona data
    let persona_key = format!("human_persona:{}", persona);
    let store = kv_store().lock().map_err(|e| format!("human_respond() lock error: {}", e))?;
    let persona_data = store.get(&persona_key).cloned().unwrap_or_default();
    drop(store);

    if persona_data.is_empty() {
        return Err(format!("human_respond() persona '{}' not found. Use human_create() first.", persona));
    }

    let data: serde_json::Value = serde_json::from_str(&persona_data)
        .map_err(|e| format!("human_respond() persona parse error: {}", e))?;
    let traits = data.get("traits").and_then(|v| v.as_str()).unwrap_or("helpful assistant").to_string();
    let mood = data.get("mood").and_then(|v| v.as_str()).unwrap_or("neutral").to_string();
    let mood_intensity = data.get("mood_intensity").and_then(|v| v.as_f64()).unwrap_or(0.5);

    // Recall relevant memories
    let recall_result = builtin_human_recall(&[
        Value::String(persona.clone()),
        Value::String(message.clone()),
        Value::Float(5.0),
    ])?;
    let memories_text = match recall_result {
        Value::List(items) => {
            let mut parts = Vec::new();
            for item in &items {
                if let Value::Struct { fields, .. } = item {
                    let key = fields.get("key").map(|v| format!("{}", v)).unwrap_or_default();
                    let content = fields.get("content").map(|v| format!("{}", v)).unwrap_or_default();
                    parts.push(format!("- [{}]: {}", key, content));
                }
            }
            if parts.is_empty() { "No relevant memories found.".to_string() } else { parts.join("\n") }
        }
        _ => "No memories.".to_string(),
    };

    // Build the LLM prompt
    let system_prompt = format!(
        "You are {}, a persona with the following traits: {}. \
        Your current emotional state is '{}' with intensity {:.1}. \
        Let your mood subtly influence your tone and word choice. \
        You have access to the following memories about the user and past interactions:\n{}\
        \nRespond naturally, as a human would. Be concise but warm. Stay in character.",
        persona, traits, mood, mood_intensity, memories_text
    );

    let full_prompt = if context.is_empty() {
        format!("{}\n\nUser: {}", system_prompt, message)
    } else {
        format!("{}\n\nRecent context:\n{}\n\nUser: {}", system_prompt, context, message)
    };

    // Call LLM (reuses existing call_llm infrastructure)
    let mock_mode = std::env::var("METALOGOS_LLM_MOCK")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(true);

    let response = if mock_mode {
        format!("[{} (mood: {}): {}]", persona, mood, message)
    } else {
        let backend = crate::llm::create_llm_backend();
        backend.call(&full_prompt, "")
            .map_err(|e| format!("human_respond() LLM call failed: {}", e))?
    };

    Ok(Value::String(response))
}

/// `human_personas()` — list all created personas.
/// Returns List of PersonaSummary structs: {name, traits, mood, memory_count, created_at}.
fn builtin_human_personas(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let prefix = "human_persona:";
    let store = kv_store().lock().map_err(|e| format!("human_personas() lock error: {}", e))?;
    let mut result = Vec::new();
    for (k, v) in store.iter() {
        if !k.starts_with(prefix) { continue; }
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(v) {
            let name = data.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let traits = data.get("traits").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let mood = data.get("mood").and_then(|v| v.as_str()).unwrap_or("neutral").to_string();
            let created_at = data.get("created_at").and_then(|v| v.as_f64()).unwrap_or(0.0);
            // Count memories for this persona
            let mem_prefix = format!("human_mem:{}:", name);
            let mem_count = store.keys().filter(|mk| mk.starts_with(&mem_prefix)).count();
            result.push(make_date_struct("PersonaSummary", vec![
                ("name", Value::String(name)),
                ("traits", Value::String(traits)),
                ("mood", Value::String(mood)),
                ("memory_count", Value::Float(mem_count as f64)),
                ("created_at", Value::Float(created_at)),
            ]));
        }
    }
    Ok(Value::List(result))
}

/// `human_delete(persona)` — delete a persona and all its memories.
/// Returns Struct {deleted_memories: Float, status: String}.
fn builtin_human_delete(args: &[Value]) -> Result<Value, String> {
    let persona = expect_string_arg("human_delete", args, 0)?;
    let persona_key = format!("human_persona:{}", persona);
    let mem_prefix = format!("human_mem:{}:", persona);
    let mut store = kv_store().lock().map_err(|e| format!("human_delete() lock error: {}", e))?;

    // Delete persona
    let persona_existed = store.remove(&persona_key).is_some();
    // Delete all memories
    let to_remove: Vec<String> = store.keys().filter(|k| k.starts_with(&mem_prefix)).cloned().collect();
    let mem_count = to_remove.len();
    for k in &to_remove {
        store.remove(k);
    }
    drop(store);

    // SQLite cleanup
    if let Ok(sqlite_guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *sqlite_guard {
            let _ = conn.execute("DELETE FROM kv_store WHERE key = ?1", rusqlite::params![persona_key]);
            for k in &to_remove {
                let _ = conn.execute("DELETE FROM kv_store WHERE key = ?1", rusqlite::params![k]);
            }
        }
    }

    let status = if persona_existed { "deleted" } else { "not_found" };
    Ok(make_date_struct("DeleteResult", vec![
        ("deleted_memories", Value::Float(mem_count as f64)),
        ("status", Value::String(status.to_string())),
    ]))
}
