// ── Built-in functions for METALOGOS M1+M2 ────────────────────────────

use crate::interpreter::Value;

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
        // String builtins (primary names)
        funcs.insert("trim".to_string(), builtin_trim as BuiltinFn);
        funcs.insert("replace".to_string(), builtin_replace as BuiltinFn);
        funcs.insert("split".to_string(), builtin_split as BuiltinFn);
        funcs.insert("join".to_string(), builtin_join as BuiltinFn);
        funcs.insert("to_upper".to_string(), builtin_upper as BuiltinFn);
        funcs.insert("to_lower".to_string(), builtin_lower as BuiltinFn);
        // Backward-compat double-underscore aliases
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
        funcs.insert("form_data".to_string(), builtin_form_data as BuiltinFn);
        funcs.insert("json_body".to_string(), builtin_json_body as BuiltinFn);

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
        Some(Value::String(s)) => Ok(Value::Float(s.len() as f64)),
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
    match haystack.find(&needle) {
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
    let s = expect_string_arg("trim", args, 0)?;
    Ok(Value::String(s.trim().to_string()))
}

fn builtin_replace(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("replace", args, 0)?;
    let old = expect_string_arg("replace", args, 1)?;
    let new = expect_string_arg("replace", args, 2)?;
    Ok(Value::String(s.replace(&old, &new)))
}

fn builtin_split(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("split", args, 0)?;
    let sep = expect_string_arg("split", args, 1)?;
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
        _ => return Err("join() requires List as first argument".to_string()),
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
    // Read from environment, wrap in opaque Secret
    match std::env::var(&key) {
        Ok(val) => Ok(Value::Secret(val)),
        Err(_) => Err(format!("env(): environment variable '{}' not found", key)),
    }
}

fn builtin_hash_password(args: &[Value]) -> Result<Value, String> {
    let password = expect_string_arg("hash_password", args, 0)?;
    // SHA-256 hash (using std — no external crypto crate needed for stub)
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    // Add a salt prefix for determinism in stub mode
    "mlog_pw_salt".hash(&mut hasher);
    password.hash(&mut hasher);
    let hash_val = hasher.finish();
    Ok(Value::Hash(format!("${:016x}", hash_val)))
}

fn builtin_verify_password(args: &[Value]) -> Result<Value, String> {
    let _password = expect_string_arg("verify_password", args, 0)?;
    let _hash = match args.get(1) {
        Some(Value::Hash(_)) => true,
        Some(other) => return Err(format!("verify_password() expected Hash as second arg, got {}", other.type_name())),
        None => return Err("verify_password() requires 2 arguments".to_string()),
    };
    // In interpreter mode, always return false (mock)
    Ok(Value::Bool(false))
}

fn builtin_encrypt(args: &[Value]) -> Result<Value, String> {
    let data = expect_string_arg("encrypt", args, 0)?;
    let _key = match args.get(1) {
        Some(Value::Secret(_)) => true,
        Some(other) => return Err(format!("encrypt() expected Secret as second arg, got {}", other.type_name())),
        None => return Err("encrypt() requires 2 arguments".to_string()),
    };
    // Mock AES-256-GCM: XOR with a repeating pattern
    let encrypted: Vec<u8> = data.bytes()
        .enumerate()
        .map(|(i, b)| b ^ (0xAE ^ (i as u8)))
        .collect();
    Ok(Value::Encrypted(encrypted))
}

fn builtin_decrypt(args: &[Value]) -> Result<Value, String> {
    let encrypted = match args.get(0) {
        Some(Value::Encrypted(data)) => data.clone(),
        Some(other) => return Err(format!("decrypt() expected Encrypted as first arg, got {}", other.type_name())),
        None => return Err("decrypt() requires 2 arguments".to_string()),
    };
    let _key = match args.get(1) {
        Some(Value::Secret(_)) => true,
        Some(other) => return Err(format!("decrypt() expected Secret as second arg, got {}", other.type_name())),
        None => return Err("decrypt() requires 2 arguments".to_string()),
    };
    // Mock AES-256-GCM decrypt: reverse the XOR
    let decrypted: String = encrypted.iter()
        .enumerate()
        .map(|(i, b)| char::from(b ^ (0xAE ^ (i as u8))))
        .collect();
    Ok(Value::String(decrypted))
}

fn builtin_generate_key(args: &[Value]) -> Result<Value, String> {
    let _ = args; // no args needed
    // Generate a mock 256-bit key (32 hex chars)
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};
    let mut hasher = DefaultHasher::new();
    "mlog_keygen".hash(&mut hasher);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    timestamp.hash(&mut hasher);
    let key_val = hasher.finish();
    Ok(Value::Secret(format!("{:032x}", key_val)))
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
