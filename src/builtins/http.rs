// -- HTTP / Date / Weather / Geo builtins -----------------------------------

use crate::interpreter::Value;
use chrono::{Datelike, TimeZone, Timelike};

use super::core::*;
use super::string::escape_html_chars;

// ── Phase 6.1 — HTTP server stubs ───────────────────────────

pub(crate) fn builtin_respond(args: &[Value]) -> Result<Value, String> {
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
pub(crate) fn builtin_respond_html(args: &[Value]) -> Result<Value, String> {
    let status_str = expect_string_arg("respond_html", args, 0)?;
    let html = expect_string_arg("respond_html", args, 1)?;
    let (status, _) = parse_status_line(&status_str);
    Ok(Value::HttpResponse { status, body: html })
}

pub(crate) fn builtin_form_data(args: &[Value]) -> Result<Value, String> {
    let _ = args; // no args needed
                  // In non-server context, return empty form data struct
    Ok(Value::Struct {
        type_name: "FormData".to_string(),
        fields: std::collections::HashMap::new(),
    })
}

pub(crate) fn builtin_json_body(args: &[Value]) -> Result<Value, String> {
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
pub(crate) fn builtin_query_param(args: &[Value]) -> Result<Value, String> {
    let _name = if args.is_empty() {
        return Err("query_param() requires 1 argument (param name)".to_string());
    } else {
        match &args[0] {
            Value::String(s) => s.clone(),
            other => {
                return Err(format!(
                    "query_param() expected String, got {}",
                    other.type_name()
                ))
            }
        }
    };
    // Stub — real implementation is special-cased in interpreter FnCall dispatch
    Ok(Value::String(String::new()))
}

pub(crate) fn builtin_render(args: &[Value]) -> Result<Value, String> {
    // render(template_name, key1, val1, key2, val2, ...)
    // Simple {{ var }} substitution with auto-escaping
    // In interpreter mode, do basic string substitution
    if args.len() < 3 || !(args.len() - 1).is_multiple_of(2) {
        return Err("render() requires template name + key/value pairs (odd count)".to_string());
    }
    let template_name = expect_string_arg("render", args, 0)?;

    // Build substitution map from remaining args (key, value pairs)
    let mut vars = std::collections::HashMap::new();
    let mut i = 1;
    while i + 1 < args.len() {
        let key = match &args[i] {
            Value::String(s) => s.clone(),
            other => {
                return Err(format!(
                    "render() key must be String, got {}",
                    other.type_name()
                ))
            }
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
        html.push_str(&format!(
            "<span data-key=\"{}\">{}</span>",
            escape_html_chars(key),
            escape_html_chars(val)
        ));
    }
    html.push_str("</div>");

    Ok(Value::Html(html))
}

/// Parse a status line like "200 OK" into (status_code, body).
pub(crate) fn parse_status_line(status_body: &str) -> (u16, String) {
    let parts: Vec<&str> = status_body.splitn(2, ' ').collect();
    let status = parts
        .first()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(200);
    let body = if parts.len() > 1 {
        parts[1].to_string()
    } else {
        String::new()
    };
    (status, body)
}

// ── Наряд №71 — Retry helpers for HTTP builtins ────────────────────────

/// Configuration for HTTP retry behaviour. Parsed from an optional Struct arg.
/// {max_retries: 3.0, base_delay: 1.0}
struct RetryConfig {
    max_retries: u32,
    base_delay_secs: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_secs: 1.0,
        }
    }
}

/// Try to extract a RetryConfig from the last argument if it's a Struct
/// with a "max_retries" or "base_delay" field. Returns None if the last
/// arg is not a Struct or doesn't look like retry config.
fn parse_retry_config(args: &[Value]) -> Option<RetryConfig> {
    let last = args.last()?;
    match last {
        Value::Struct { fields, .. } => {
            let mut has_retry_field = false;
            let mut cfg = RetryConfig::default();
            for (key, val) in fields {
                match key.as_str() {
                    "max_retries" => {
                        if let Value::Float(f) = val {
                            cfg.max_retries = (*f).clamp(0.0, 10.0) as u32;
                            has_retry_field = true;
                        }
                    }
                    "base_delay" => {
                        if let Value::Float(f) = val {
                            cfg.base_delay_secs = (*f).clamp(0.1, 30.0);
                            has_retry_field = true;
                        }
                    }
                    _ => {}
                }
            }
            if has_retry_field {
                Some(cfg)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Check if an HTTP status code warrants a retry.
/// Retries on 429 (rate limit) and 5xx (server error).
/// Does NOT retry on other 4xx (client errors) — those are fatal.
fn should_retry_http(status: u16) -> bool {
    status == 429 || (500..600).contains(&status)
}

/// Send an HTTP POST request. Returns the response body as String.
/// Usage: http_post(url, body, content_type)
/// Usage: http_post(url, body, content_type, auth_token)        — sets Authorization: Bearer <auth_token>
/// Usage: http_post(url, body, content_type, headers_struct)    — sets headers from Struct fields
/// Наряд №12 Bug 2: Added 4th parameter for authorization headers.
/// Наряд №71: Optional trailing retry_config Struct {max_retries, base_delay}.
pub(crate) fn builtin_http_post(args: &[Value]) -> Result<Value, String> {
    // Наряд №71: extract retry_config from last arg if present (Struct with retry fields).
    // When no retry_config provided, max_retries=0 → no retry (backward compatible).
    let (effective_args, retry_cfg) = if let Some(cfg) = parse_retry_config(args) {
        (
            if args.len() > 1 {
                &args[..args.len() - 1]
            } else {
                args
            },
            cfg,
        )
    } else {
        (
            args,
            RetryConfig {
                max_retries: 0,
                base_delay_secs: 1.0,
            },
        )
    };

    let url = match effective_args.first() {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "http_post() expected String as url, got {}",
                other.type_name()
            ))
        }
        None => return Err("http_post() requires at least 1 argument (url)".to_string()),
    };

    let body = match effective_args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "http_post() expected String as body, got {}",
                other.type_name()
            ))
        }
        None => return Err("http_post() requires at least 2 arguments (url, body)".to_string()),
    };

    let (content_type, timeout_arg_idx) = match effective_args.get(2) {
        Some(Value::String(s)) => (s.clone(), 3),
        _ => ("application/json".to_string(), 2),
    };

    // Наряда-26 P0-1: configurable timeout (default 30s, max 300s)
    // Signatures: http_post(url, body, timeout) | http_post(url, body, ct, timeout) | http_post(url, body, ct, headers, timeout)
    let timeout_secs = if let Some(Value::Float(f)) = effective_args.get(timeout_arg_idx) {
        let t = f.clamp(1.0, 300.0) as u64;
        if *f > 300.0 {
            eprintln!("[http_post] timeout clamped from {} to 300s", f);
        }
        t
    } else {
        30
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("http_post(): failed to create client: {}", e))?;

    // Optional headers argument (index depends on whether content_type was provided)
    let headers_idx =
        if effective_args.len() > 2 && matches!(effective_args.get(2), Some(Value::String(_))) {
            // content_type was 3rd arg → headers are 4th
            3
        } else {
            // content_type was default → headers are 3rd
            2
        };
    // Only parse headers if the arg exists and is NOT a Float (which would be timeout)
    let mut extra_headers: Vec<(String, String)> = Vec::new();
    if let Some(headers_arg) = effective_args.get(headers_idx) {
        if !matches!(headers_arg, Value::Float(_)) {
            match headers_arg {
                Value::String(auth_token) => {
                    if !auth_token.is_empty() {
                        extra_headers.push((
                            "Authorization".to_string(),
                            format!("Bearer {}", auth_token),
                        ));
                    }
                }
                Value::Struct { fields, .. } => {
                    for (key, val) in fields {
                        if let Value::String(v) = val {
                            extra_headers.push((key.clone(), v.clone()));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Наряд №71 — retry loop (mirrors llm.rs::RealLlm::call pattern)
    let no_retry = retry_cfg.max_retries == 0;
    let mut last_error = String::new();

    for attempt in 0..=retry_cfg.max_retries {
        if attempt > 0 {
            let delay_secs = retry_cfg.base_delay_secs * (1u32 << (attempt - 1)) as f64;
            eprintln!(
                "[http_post] retry {}/{} after {:.1}s...",
                attempt, retry_cfg.max_retries, delay_secs
            );
            std::thread::sleep(std::time::Duration::from_secs_f64(delay_secs));
        }

        let mut req = client
            .post(&url)
            .header("Content-Type", &content_type)
            .body(body.clone());
        for (k, v) in &extra_headers {
            req = req.header(k.as_str(), v.as_str());
        }

        let resp = req.send().map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("timeout") || err_str.contains("timed out") {
                format!("ERROR: http timeout after {}s", timeout_secs)
            } else {
                format!("http_post() request failed: {}", e)
            }
        })?;

        let status = resp.status().as_u16();
        let resp_body = resp.text().unwrap_or_default();

        // Retry on 429/rate-limit and 5xx server errors; fail immediately on other 4xx
        if status >= 400 {
            if should_retry_http(status) && !no_retry && attempt < retry_cfg.max_retries {
                last_error = format!("status {}: {}", status, resp_body);
                continue;
            }
            return Err(format!(
                "http_post() returned status {}: {}",
                status, resp_body
            ));
        }

        return Ok(Value::String(resp_body));
    }

    Err(format!(
        "http_post() failed after {} retries: {}",
        retry_cfg.max_retries, last_error
    ))
}

/// Send an HTTP GET request. Returns the response body as String.
/// Usage: http_get(url) -> String
/// Usage: http_get(url, headers_struct) -> String  — sets headers from Struct fields
/// Usage: http_get(url, headers, timeout, retry_config) -> String  — Наряд №71: retry
pub(crate) fn builtin_http_get(args: &[Value]) -> Result<Value, String> {
    // Наряд №71: extract retry_config from last arg if present (Struct with retry fields).
    // When no retry_config provided, max_retries=0 → no retry (backward compatible).
    let (effective_args, retry_cfg) = if let Some(cfg) = parse_retry_config(args) {
        (
            if args.len() > 1 {
                &args[..args.len() - 1]
            } else {
                args
            },
            cfg,
        )
    } else {
        (
            args,
            RetryConfig {
                max_retries: 0,
                base_delay_secs: 1.0,
            },
        )
    };

    let url = match effective_args.first() {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "http_get() expected String as url, got {}",
                other.type_name()
            ))
        }
        None => return Err("http_get() requires 1 argument (url)".to_string()),
    };

    // Наряда-26 P0-1: configurable timeout
    // http_get(url) | http_get(url, timeout) | http_get(url, headers) | http_get(url, headers, timeout)
    let (headers_arg, timeout_secs) = match effective_args.len() {
        1 => (None, 30u64),
        2 => {
            // 2nd arg could be timeout (Float) or headers (String/Struct)
            match &effective_args[1] {
                Value::Float(f) => (None, f.clamp(1.0, 300.0) as u64),
                other => (Some(other), 30),
            }
        }
        _ => {
            // 3+ args: 2nd is headers, 3rd is timeout
            let timeout = if let Some(Value::Float(f)) = effective_args.get(2) {
                f.clamp(1.0, 300.0) as u64
            } else {
                30
            };
            (effective_args.get(1), timeout)
        }
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("http_get(): failed to create client: {}", e))?;

    // Extract extra headers into a Vec so we can rebuild the request on retry
    let mut extra_headers: Vec<(String, String)> = Vec::new();
    if let Some(ha) = headers_arg {
        match ha {
            Value::String(auth_token) => {
                if !auth_token.is_empty() {
                    extra_headers.push((
                        "Authorization".to_string(),
                        format!("Bearer {}", auth_token),
                    ));
                }
            }
            Value::Struct { fields, .. } => {
                for (key, val) in fields {
                    if let Value::String(v) = val {
                        extra_headers.push((key.clone(), v.clone()));
                    }
                }
            }
            _ => {}
        }
    }

    // Наряд №71 — retry loop (mirrors llm.rs::RealLlm::call pattern)
    let no_retry = retry_cfg.max_retries == 0;
    let mut last_error = String::new();

    for attempt in 0..=retry_cfg.max_retries {
        if attempt > 0 {
            let delay_secs = retry_cfg.base_delay_secs * (1u32 << (attempt - 1)) as f64;
            eprintln!(
                "[http_get] retry {}/{} after {:.1}s...",
                attempt, retry_cfg.max_retries, delay_secs
            );
            std::thread::sleep(std::time::Duration::from_secs_f64(delay_secs));
        }

        let mut req = client.get(&url);
        for (k, v) in &extra_headers {
            req = req.header(k.as_str(), v.as_str());
        }

        let resp = req.send().map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("timeout") || err_str.contains("timed out") {
                format!("ERROR: http timeout after {}s", timeout_secs)
            } else {
                format!("http_get() request failed: {}", e)
            }
        })?;

        let status = resp.status().as_u16();
        let resp_body = resp.text().unwrap_or_default();

        // Retry on 429/rate-limit and 5xx server errors; fail immediately on other 4xx
        if status >= 400 {
            if should_retry_http(status) && !no_retry && attempt < retry_cfg.max_retries {
                last_error = format!("status {}: {}", status, resp_body);
                continue;
            }
            return Err(format!(
                "http_get() returned status {}: {}",
                status, resp_body
            ));
        }

        return Ok(Value::String(resp_body));
    }

    Err(format!(
        "http_get() failed after {} retries: {}",
        retry_cfg.max_retries, last_error
    ))
}

/// Return current Unix timestamp as Float (seconds since epoch).
/// Usage: now() -> Float
pub(crate) fn builtin_now(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    Ok(Value::Float(now))
}

/// sleep(seconds) — block the current execution thread for the given duration.
/// Uses std::thread::sleep because the interpreter runs on a blocking thread
/// (via tokio::task::block_in_place in server.rs). Safe: does not block the
/// tokio async runtime worker. Maximum 60 seconds to prevent accidental hangs.
pub(crate) fn builtin_sleep(args: &[Value]) -> Result<Value, String> {
    let secs = match args.first() {
        Some(Value::Float(f)) => *f,
        _ => return Err("sleep() expects a numeric duration (Float)".to_string()),
    };
    const MAX_SECS: f64 = 60.0;
    if secs < 0.0 {
        return Err("sleep() duration must be non-negative".to_string());
    }
    if secs > MAX_SECS {
        return Err(format!(
            "sleep() duration capped at {} seconds (got {})",
            MAX_SECS, secs
        ));
    }
    std::thread::sleep(std::time::Duration::from_secs_f64(secs));
    Ok(Value::Unit)
}

pub(crate) fn builtin_require(args: &[Value]) -> Result<Value, String> {
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
        other => Err(format!(
            "require() expected Bool, got {}",
            other.type_name()
        )),
    }
}

pub(crate) fn builtin_http_post_multipart(args: &[Value]) -> Result<Value, String> {
    let url = expect_string_arg("http_post_multipart", args, 0)?;
    let fields = match args.get(1) {
        Some(Value::Struct { fields, .. }) => fields.clone(),
        _ => {
            return Err(
                "http_post_multipart() requires Struct as 2nd argument (fields)".to_string(),
            )
        }
    };
    let files = match args.get(2) {
        Some(Value::Struct { fields, .. }) => fields.clone(),
        _ => {
            return Err("http_post_multipart() requires Struct as 3rd argument (files)".to_string())
        }
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
            let file_bytes = std::fs::read(path).map_err(|e| {
                format!("http_post_multipart(): cannot read file '{}': {}", path, e)
            })?;
            let file_name = std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file");
            let part = reqwest::blocking::multipart::Part::bytes(file_bytes)
                .file_name(file_name.to_string());
            form = form.part(key.clone(), part);
        }
    }

    let resp = client
        .post(&url)
        .multipart(form)
        .send()
        .map_err(|e| format!("http_post_multipart(): request failed: {}", e))?;

    let status = resp.status().as_u16();
    let resp_body = resp.text().unwrap_or_default();

    if status >= 400 {
        return Err(format!(
            "http_post_multipart() returned status {}: {}",
            status, resp_body
        ));
    }

    Ok(Value::String(resp_body))
}

/// `web_search(query, num_results?) -> String` — search via SerpAPI.
/// Uses SERPAPI_KEY env var. Returns raw JSON string.
/// Usage: web_search("query") -> JSON string
/// Usage: web_search("query", 5) -> JSON string with 5 results
pub(crate) fn builtin_web_search(args: &[Value]) -> Result<Value, String> {
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

// ── v0.8.0 — format_date (enhanced) ──────────────────────
/// Weekday names: Monday=0 (converted from libc Sunday=0).
const WEEKDAY_NAMES_MON: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// Month names (1-indexed).
const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const MONTH_ABBR: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `format_date(format)` — format current time.
/// `format_date(format, timestamp)` — format given unix timestamp (Float seconds).
/// v0.8.0: supports %y %I %p %A %a %B %b %j %w %W %% in addition to %Y %m %d %H %M %S %F %T %R.
pub(crate) fn builtin_format_date(args: &[Value]) -> Result<Value, String> {
    let fmt_str = if args.is_empty() {
        "%Y-%m-%d %H:%M:%S".to_string()
    } else {
        expect_string_arg("format_date", args, 0)?
    };

    let timestamp = if args.len() >= 2 {
        match &args[1] {
            Value::Float(f) => *f,
            _ => {
                return Err(format!(
                    "format_date(): timestamp must be Float, got {}",
                    args[1].type_name()
                ))
            }
        }
    } else {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    };

    let secs = timestamp as i64;
    let dt = chrono::Local
        .timestamp_opt(secs, 0)
        .single()
        .unwrap_or_else(chrono::Local::now);

    let y = dt.year() as u32;
    let mo = dt.month();
    let d = dt.day();
    let h = dt.hour();
    let mi = dt.minute();
    let s = dt.second();
    let wday_mon = dt.weekday().num_days_from_monday();
    let day_of_year = dt.ordinal();
    let week_num = ((day_of_year as i32 + 6 - wday_mon as i32) / 7).max(1) as u32;
    let ampm = if h >= 12 { "PM" } else { "AM" };
    let h12 = if h == 0 {
        12
    } else if h > 12 {
        h - 12
    } else {
        h
    };

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
                        Some(c) => {
                            out.push('%');
                            out.push(c);
                        }
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

pub(crate) fn date_is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

pub(crate) fn date_days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if date_is_leap(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

pub(crate) fn make_date_struct(type_name: &str, pairs: Vec<(&str, Value)>) -> Value {
    let mut map = std::collections::HashMap::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), v);
    }
    Value::Struct {
        type_name: type_name.to_string(),
        fields: map,
    }
}

/// `date_parts(timestamp?)` — returns struct with all date components via libc.
pub(crate) fn builtin_date_parts(args: &[Value]) -> Result<Value, String> {
    let ts = if args.is_empty() {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    } else {
        expect_float_arg("date_parts", args, 0)?
    };
    let secs = ts as i64;
    let dt = chrono::Local
        .timestamp_opt(secs, 0)
        .single()
        .unwrap_or_else(chrono::Local::now);

    let y = dt.year() as u32;
    let mo = dt.month();
    let d = dt.day();
    let h = dt.hour();
    let mi = dt.minute();
    let s = dt.second();
    let wday_mon = dt.weekday().num_days_from_monday();
    let day_of_year = dt.ordinal();
    let week_num = ((day_of_year as i32 + 6 - wday_mon as i32) / 7).max(1) as u32;

    Ok(make_date_struct(
        "Date",
        vec![
            ("year", Value::Float(y as f64)),
            ("month", Value::Float(mo as f64)),
            ("day", Value::Float(d as f64)),
            ("hour", Value::Float(h as f64)),
            ("minute", Value::Float(mi as f64)),
            ("second", Value::Float(s as f64)),
            ("weekday", Value::Float(wday_mon as f64)),
            (
                "weekday_name",
                Value::String(WEEKDAY_NAMES_MON[wday_mon as usize].to_string()),
            ),
            (
                "month_name",
                Value::String(MONTH_NAMES[(mo - 1) as usize].to_string()),
            ),
            ("day_of_year", Value::Float(day_of_year as f64)),
            ("week_number", Value::Float(week_num as f64)),
            ("timestamp", Value::Float(ts)),
        ],
    ))
}

/// `days_between(ts1, ts2)` — absolute difference in days.
pub(crate) fn builtin_days_between(args: &[Value]) -> Result<Value, String> {
    let ts1 = expect_float_arg("days_between", args, 0)?;
    let ts2 = expect_float_arg("days_between", args, 1)?;
    Ok(Value::Float((ts1 - ts2).abs() / 86400.0))
}

/// `days_in_month(year, month)` — days in given month (1-12).
pub(crate) fn builtin_days_in_month(args: &[Value]) -> Result<Value, String> {
    let year = expect_float_arg("days_in_month", args, 0)? as i32;
    let month = expect_float_arg("days_in_month", args, 1)? as u32;
    if !(1..=12).contains(&month) {
        return Err("days_in_month() month must be 1-12".to_string());
    }
    Ok(Value::Float(date_days_in_month(year, month) as f64))
}

/// `is_leap_year(year)` — Gregorian leap year check.
pub(crate) fn builtin_is_leap_year(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(date_is_leap(
        expect_float_arg("is_leap_year", args, 0)? as i32,
    )))
}

/// `add_days(timestamp, days)` — add/subtract days to timestamp.
pub(crate) fn builtin_add_days(args: &[Value]) -> Result<Value, String> {
    let ts = expect_float_arg("add_days", args, 0)?;
    let days = expect_float_arg("add_days", args, 1)?;
    Ok(Value::Float(ts + days * 86400.0))
}

/// `add_hours(timestamp, hours)` — add/subtract hours to timestamp.
pub(crate) fn builtin_add_hours(args: &[Value]) -> Result<Value, String> {
    let ts = expect_float_arg("add_hours", args, 0)?;
    let hours = expect_float_arg("add_hours", args, 1)?;
    Ok(Value::Float(ts + hours * 3600.0))
}

/// `weekday_name(timestamp)` — full weekday name ("Monday".."Sunday").
pub(crate) fn builtin_weekday_name(args: &[Value]) -> Result<Value, String> {
    let ts = expect_float_arg("weekday_name", args, 0)?;
    let secs = ts as i64;
    let dt = chrono::Local
        .timestamp_opt(secs, 0)
        .single()
        .unwrap_or_else(chrono::Local::now);
    let wday_mon = dt.weekday().num_days_from_monday();
    Ok(Value::String(
        WEEKDAY_NAMES_MON[wday_mon as usize].to_string(),
    ))
}

/// `geo_ip(ip?)` — geolocate by IP. Uses ip-api.com (free, no key).
/// Returns Struct {ip, city, region, country, country_code, lat, lon, isp, timezone}.
pub(crate) fn builtin_geo_ip(args: &[Value]) -> Result<Value, String> {
    let ip = match args.first() {
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
    let resp = client
        .get(&url)
        .send()
        .map_err(|e| format!("geo_ip() request failed: {}", e))?;
    let body = resp.text().unwrap_or_default();
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("geo_ip() parse error: {}", e))?;
    if json.get("status").and_then(|v| v.as_str()) != Some("success") {
        let msg = json
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(format!("geo_ip() API error: {}", msg));
    }
    let g = |key: &str| -> Value {
        json.get(key)
            .map(|v| match v {
                serde_json::Value::String(s) => Value::String(s.clone()),
                serde_json::Value::Number(n) => Value::Float(n.as_f64().unwrap_or(0.0)),
                serde_json::Value::Null => Value::String(String::new()),
                _ => Value::String(v.to_string()),
            })
            .unwrap_or(Value::String(String::new()))
    };
    Ok(make_date_struct(
        "GeoLocation",
        vec![
            ("ip", g("query")),
            ("city", g("city")),
            ("region", g("regionName")),
            ("country", g("country")),
            ("country_code", g("countryCode")),
            ("lat", g("lat")),
            ("lon", g("lon")),
            ("isp", g("isp")),
            ("timezone", g("timezone")),
        ],
    ))
}

/// `geo_distance(lat1, lon1, lat2, lon2, unit?)` — haversine distance. unit: "km"(default), "mi", "nm", "m".
pub(crate) fn builtin_geo_distance(args: &[Value]) -> Result<Value, String> {
    let lat1 = expect_float_arg("geo_distance", args, 0)?;
    let lon1 = expect_float_arg("geo_distance", args, 1)?;
    let lat2 = expect_float_arg("geo_distance", args, 2)?;
    let lon2 = expect_float_arg("geo_distance", args, 3)?;
    let unit = match args.get(4) {
        Some(Value::String(s)) => s.as_str(),
        _ => "km",
    };
    let to_rad = |d: f64| d * std::f64::consts::PI / 180.0;
    let dlat = to_rad(lat2 - lat1);
    let dlon = to_rad(lon2 - lon1);
    let a = (dlat / 2.0).sin().powi(2)
        + to_rad(lat1).cos() * to_rad(lat2).cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    let km = 6371.0 * c;
    Ok(Value::Float(match unit {
        "mi" => km * 0.621371,
        "nm" => km * 0.539957,
        "m" => km * 1000.0,
        _ => km,
    }))
}

/// WMO weather codes to human-readable description.
pub(crate) fn wmo_description(code: i64) -> &'static str {
    match code {
        0 => "Clear sky",
        1 => "Mainly clear",
        2 => "Partly cloudy",
        3 => "Overcast",
        45 | 48 => "Fog",
        51 | 53 | 55 => "Drizzle",
        56 | 57 => "Freezing drizzle",
        61 | 63 | 65 => "Rain",
        66 | 67 => "Freezing rain",
        71 | 73 | 75 => "Snow fall",
        77 => "Snow grains",
        80..=82 => "Rain showers",
        85 | 86 => "Snow showers",
        95 => "Thunderstorm",
        96 | 99 => "Thunderstorm with hail",
        _ => "Unknown",
    }
}

/// Resolve city name to (lat, lon) via Open-Meteo geocoding (free, no key).
pub(crate) fn geo_resolve_city(city: &str) -> Result<(f64, f64), String> {
    let url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=en&format=json",
        urlencoding::encode(city)
    );
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("weather() client error: {}", e))?;
    let body = client
        .get(&url)
        .send()
        .map_err(|e| format!("weather() geocoding failed: {}", e))?
        .text()
        .unwrap_or_default();
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("weather() geocoding parse error: {}", e))?;
    let results = json
        .get("results")
        .and_then(|r| r.as_array())
        .ok_or_else(|| format!("weather() city not found: {}", city))?;
    if results.is_empty() {
        return Err(format!("weather() city not found: {}", city));
    }
    let first = &results[0];
    let lat = first
        .get("latitude")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let lon = first
        .get("longitude")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    Ok((lat, lon))
}

/// `weather(city_or_lat, lon?)` — current weather via Open-Meteo (FREE, no API key).
/// `weather("Minsk")` or `weather(53.9, 27.57)`.
/// Returns Struct {temp, feels_like, temp_min, temp_max, humidity, description,
///   wind_speed, wind_direction, pressure, cloud_cover, is_day, city, country}.
pub(crate) fn builtin_weather(args: &[Value]) -> Result<Value, String> {
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
        .build()
        .map_err(|e| format!("weather() client error: {}", e))?;
    let resp = client
        .get(&url)
        .send()
        .map_err(|e| format!("weather() request failed: {}", e))?;
    let status = resp.status().as_u16();
    let body = resp.text().unwrap_or_default();
    if status >= 400 {
        return Err(format!("weather() API error {}: {}", status, body));
    }
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("weather() parse error: {}", e))?;
    let cur = json
        .get("current")
        .ok_or("weather() missing 'current' in response")?;
    let gf = |key: &str| -> f64 { cur.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0) };
    let code = gf("weather_code") as i64;
    let desc = wmo_description(code).to_string();
    Ok(make_date_struct(
        "Weather",
        vec![
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
        ],
    ))
}

/// `weather_forecast(city_or_lat, lon?, days?)` — multi-day forecast via Open-Meteo (FREE, no API key).
/// `weather_forecast("Minsk", 7)` or `weather_forecast(53.9, 27.57, 3)`.
/// Default: 7 days. Max: 16 days. Returns List of DayForecast structs.
pub(crate) fn builtin_weather_forecast(args: &[Value]) -> Result<Value, String> {
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
    days = days.clamp(1, 16);
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_sum,wind_speed_10m_max,sunrise,sunset,uv_index_max&timezone=auto&forecast_days={}",
        lat, lon, days
    );
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| format!("weather_forecast() client error: {}", e))?;
    let resp = client
        .get(&url)
        .send()
        .map_err(|e| format!("weather_forecast() request failed: {}", e))?;
    let status = resp.status().as_u16();
    let body = resp.text().unwrap_or_default();
    if status >= 400 {
        return Err(format!("weather_forecast() API error {}: {}", status, body));
    }
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("weather_forecast() parse error: {}", e))?;
    let daily = json
        .get("daily")
        .ok_or("weather_forecast() missing 'daily' in response")?;
    let dates = daily
        .get("time")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let codes = daily
        .get("weather_code")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let tmax = daily
        .get("temperature_2m_max")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let tmin = daily
        .get("temperature_2m_min")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let precip = daily
        .get("precipitation_sum")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let wind = daily
        .get("wind_speed_10m_max")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let sunrise = daily
        .get("sunrise")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let sunset = daily
        .get("sunset")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let uv = daily
        .get("uv_index_max")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let gf = |arr: &[serde_json::Value], i: usize| -> f64 {
        arr.get(i).and_then(|v| v.as_f64()).unwrap_or(0.0)
    };
    let gs = |arr: &[serde_json::Value], i: usize| -> String {
        arr.get(i)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let mut result = Vec::new();
    for i in 0..dates.len() {
        let code = gf(&codes, i) as i64;
        let desc = wmo_description(code).to_string();
        let uv_val = gf(&uv, i);
        let uv_str = if uv_val < 0.0 {
            String::new()
        } else {
            format!("{:.1}", uv_val)
        };
        result.push(make_date_struct(
            "DayForecast",
            vec![
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
            ],
        ));
    }
    Ok(Value::List(result))
}
