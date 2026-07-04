// ── METALOGOS HTTP Server (Phase 6.1–7.4) ───────────────────────────
// Axum-based HTTP server with security middleware:
// - SQLite-backed session store (Phase 7.4)
// - HMAC-SHA256 signed session cookies
// - CSRF double-submit cookie pattern (Phase 7.4: real tokens)
// - Rate limiting: sliding window per IP (Phase 7.4)
// - Security headers (CSP, X-Frame-Options, X-Content-Type-Options, HSTS)
// - Role-based route access
// - Template rendering with auto-escaping
// - Bot integration (Telegram webhooks)

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, Uri, header, Method},
    response::{Html as AxumHtml, IntoResponse, Response},
    routing::{get, post, put, delete, any},
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::set_header::SetResponseHeaderLayer;

use chrono::{Datelike, Timelike};

use crate::ast::*;
use crate::interpreter::{Interpreter, Value};

/// Check if a cron field (min/hour/dom/month/dow) matches a value.
/// Supports: `*`, `*/N`, `N`, `N-M`, `N,M,O`, `N-M/S`.
fn cron_field_matches(field: &str, value: u32) -> bool {
    for part in field.split(',') {
        let part = part.trim();
        if part == "*" {
            return true;
        }
        if let Some(step_str) = part.strip_prefix("*/") {
            if let Ok(step) = step_str.parse::<u32>() {
                if step == 0 { continue; }
                if value % step == 0 { return true; }
            }
            continue;
        }
        // Handle range with optional step: N-M or N-M/S
        if part.contains('-') {
            let segments: Vec<&str> = part.split('/').collect();
            let range_str = segments[0];
            let step: u32 = if segments.len() > 1 {
                segments[1].parse().unwrap_or(1)
            } else {
                1
            };
            if step == 0 { continue; }
            let bounds: Vec<&str> = range_str.split('-').collect();
            if bounds.len() == 2 {
                if let (Ok(lo), Ok(hi)) = (bounds[0].parse::<u32>(), bounds[1].parse::<u32>()) {
                    if value >= lo && value <= hi && (value - lo) % step == 0 {
                        return true;
                    }
                }
            }
            continue;
        }
        // Plain number
        if let Ok(n) = part.parse::<u32>() {
            if n == value { return true; }
        }
    }
    false
}

/// Check if a 5-field cron expression matches the current time.
/// Fields: min hour dom month dow
/// dow: 0=Sunday (chrono), same as standard cron.
fn cron_expr_matches(expr: &str) -> bool {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() != 5 {
        return false;
    }
    let now = chrono::Local::now();
    let min = now.minute();
    let hour = now.hour();
    let dom = now.day();          // 1-31
    let month = now.month();       // 1-12
    let dow = now.weekday().num_days_from_sunday(); // 0=Sun

    cron_field_matches(parts[0], min)
        && cron_field_matches(parts[1], hour)
        && cron_field_matches(parts[2], dom)
        && cron_field_matches(parts[3], month)
        && cron_field_matches(parts[4], dow)
}

/// Simple URL percent-decode fallback (handles %XX without external crate).
fn url_decode_fallback(s: &str) -> String {
    let mut result = Vec::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push('%');
            result.extend(hex.chars());
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result.into_iter().collect()
}

// Compile-time check: ServerState must be Send + Sync for axum::State
fn _assert_state_send_sync(state: ServerState) {
    fn assert_send<T: Send>(_: &T) {}
    fn assert_sync<T: Sync>(_: &T) {}
    // Force the compiler to check Send+Sync on the actual struct, not just the name
    let _ = &state;
    assert_send(&state);
    assert_sync(&state);
}

// ── Server State ──────────────────────────────────────────────────

/// Shared mutable server state, protected by tokio::RwLock / std::sync::Mutex.
#[derive(Clone)]
pub struct ServerState {
    /// In-memory session cache (kept for fast lookouts, authoritative source is SQLite).
    pub sessions: Arc<RwLock<HashMap<String, SessionEntry>>>,
    /// CSRF token store for double-submit validation.
    pub csrf_tokens: Arc<RwLock<HashMap<String, String>>>,
    /// HMAC signing key for session cookies.
    pub hmac_key: Arc<Vec<u8>>,
    /// Audit log entries.
    pub audit_log: Arc<RwLock<Vec<String>>>,
    /// Registered templates.
    pub templates: Arc<RwLock<HashMap<String, TemplateDecl>>>,
    /// Mock DB store.
    pub db_store: Arc<RwLock<Vec<HashMap<String, Value>>>>,
    /// Memory persist path (if configured).
    pub memory_persist: Option<String>,
    /// Interpreter (for running route handlers).
    pub interpreter: Arc<RwLock<Interpreter>>,
    /// Route definitions from mlogserver block.
    pub routes: Vec<RouteDecl>,
    /// Required middleware (from mlogserver config).
    pub middleware: Vec<String>,
    /// SQLite connection for session persistence (Phase 7.4).
    pub db: Arc<tokio::sync::Mutex<rusqlite::Connection>>,
    /// Rate-limit tracker: IP → Vec<Instant> (Phase 7.4).
    pub rate_limits: Arc<RwLock<HashMap<String, Vec<std::time::Instant>>>>,
}

#[derive(Debug, Clone)]
pub struct SessionEntry {
    pub data: HashMap<String, String>,
    pub roles: Vec<String>,
    pub expires: std::time::Instant,
}

// ── Public API ─────────────────────────────────────────────────────

/// Parse source, build Axum router, start server on configured port.
/// This is the entry point for `mlog serve <file>`.
pub async fn run_server(source: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let declarations = crate::parser::parse(source)
        .map_err(|e| format!("parse error: {}", e))?;

    let mut interp = Interpreter::new();
    // Run declarations to populate templates, patterns, etc. (skip flows)
    for decl in declarations.clone() {
        match decl {
            Declaration::MlogServer(ref srv) => {
                interp = build_interpreter_with_server(srv, interp);
            }
            Declaration::Flow(_) => { /* skip flows in server mode */ }
            _ => {
                let mut tmp_interp = Interpreter::new();
                tmp_interp.set_base_dir(std::path::PathBuf::from("."));
                let _ = tmp_interp.run(vec![decl]);
                interp = merge_interpreter(tmp_interp, interp);
            }
        }
    }

    // Find MlogServer declaration
    let server_config = declarations.iter().find_map(|d| match d {
        Declaration::MlogServer(s) => Some(s.clone()),
        _ => None,
    });

    let config = match server_config {
        Some(c) => c,
        None => return Err("no mlogserver block found in source".into()),
    };

    let port = config.port;
    let state = build_state(config, interp).await;
    let app = build_router(state.clone());

    // v0.8.2 — Background reminder scheduler (checks every 5 seconds)
    // v0.8.3 — Extended: also checks cron jobs from OpenHuman-inspired cron_add
    let scheduler_state = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            // ── Reminder check (v0.8.2) ──
            let mut interp = scheduler_state.interpreter.write().await;
            let check_result = {
                let builtin_name = "check_reminders";
                if let Some(builtin_fn) = interp.get_builtin(builtin_name) {
                    builtin_fn(&[])
                } else {
                    Ok(crate::interpreter::Value::List(vec![]))
                }
            };
            if let Ok(crate::interpreter::Value::List(items)) = check_result {
                for item in &items {
                    if let crate::interpreter::Value::Struct { fields, .. } = item {
                        let msg = fields.get("message").map(|v| format!("{}", v)).unwrap_or_default();
                        let rtype = fields.get("type").map(|v| format!("{}", v)).unwrap_or_default();
                        eprintln!("[scheduler] due {}: [{}] {}", rtype, msg,
                            fields.get("data").map(|v| format!("{}", v)).unwrap_or_default());
                    }
                }
            }

            // ── Cron job dispatch (v0.8.5) ──
            let cron_check = {
                let builtin_name = "cron_list";
                if let Some(builtin_fn) = interp.get_builtin(builtin_name) {
                    builtin_fn(&[])
                } else {
                    Ok(crate::interpreter::Value::List(vec![]))
                }
            };
            if let Ok(crate::interpreter::Value::List(jobs)) = cron_check {
                for job in &jobs {
                    if let crate::interpreter::Value::Struct { fields, .. } = job {
                        let job_id = fields.get("id").map(|v| format!("{}", v)).unwrap_or_default();
                        let cron_expr = fields.get("cron_expr").map(|v| format!("{}", v)).unwrap_or_default();
                        let enabled = fields.get("enabled").map(|v| format!("{}", v)) == Some("1".to_string());
                        let force_run = fields.get("force_run").map(|v| format!("{}", v)) == Some("1".to_string());
                        let prompt = fields.get("prompt").map(|v| format!("{}", v)).unwrap_or_default();

                        if !enabled { continue; }

                        let should_fire = force_run || cron_expr_matches(&cron_expr);
                        if should_fire {
                            eprintln!("[cron] firing: {} — {}", cron_expr, prompt);
                            // Dispatch: try builtin first, then user pattern
                            if let Some(builtin_fn) = interp.get_builtin(&prompt) {
                                if let Err(e) = builtin_fn(&[]) {
                                    eprintln!("[cron] builtin '{}' error: {}", prompt, e);
                                }
                            } else if let Err(e) = interp.call_pattern(&prompt, &[]) {
                                eprintln!("[cron] pattern '{}' error: {}", prompt, e);
                            }
                            // Reset force_run, increment run_count, set last_run
                            if let Some(mark_fn) = interp.get_builtin("cron_mark_fired") {
                                if let Err(e) = mark_fn(&[crate::interpreter::Value::String(job_id)]) {
                                    eprintln!("[cron] mark_fired error: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    println!("mlog serve: listening on 0.0.0.0:{}", port);
    println!("mlog serve: scheduler active (5s interval — reminders + cron)");
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// Start server on a random port for integration testing.
/// Returns (port, join_handle).
pub async fn run_test_server(source: &str) -> Result<(u16, tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>), Box<dyn std::error::Error + Send + Sync>> {
    let declarations = crate::parser::parse(source)
        .map_err(|e| format!("parse error: {}", e))?;

    let server_config = declarations.iter().find_map(|d| match d {
        Declaration::MlogServer(s) => Some(s.clone()),
        _ => None,
    }).ok_or("no mlogserver block")?;

    let mut interp = Interpreter::new();
    for decl in declarations {
        if !matches!(decl, Declaration::Flow(_)) {
            let mut tmp = Interpreter::new();
            tmp.set_base_dir(std::path::PathBuf::from("."));
            let _ = tmp.run(vec![decl]);
            interp = merge_interpreter(tmp, interp);
        }
    }

    // Override port to 0 (OS-assigned)
    let mut config = server_config.clone();
    config.port = 0;

    let state = build_state(config.clone(), interp).await;
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await?;
    let port = listener.local_addr()?.port();

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await?;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });

    Ok((port, handle))
}

// ── Internal: Build State ──────────────────────────────────────────

async fn build_state(config: MlogServerDecl, interp: Interpreter) -> ServerState {
    // Generate HMAC key
    let hmac_key = generate_hmac_key();

    // Collect templates from interpreter
    let templates_map = interp.get_templates().clone();

    // Phase 7.4: SQLite session store (in-memory for this server instance)
    let conn = rusqlite::Connection::open_in_memory()
        .expect("Failed to open SQLite in-memory database");
    init_session_db(&conn).expect("Failed to create sessions table");

    ServerState {
        sessions: Arc::new(RwLock::new(HashMap::new())),
        csrf_tokens: Arc::new(RwLock::new(HashMap::new())),
        hmac_key: Arc::new(hmac_key),
        audit_log: Arc::new(RwLock::new(Vec::new())),
        templates: Arc::new(RwLock::new(templates_map)),
        db_store: Arc::new(RwLock::new(Vec::new())),
        memory_persist: interp.get_memory_persist_path(),
        interpreter: Arc::new(RwLock::new(interp)),
        routes: config.routes.clone(),
        middleware: config.middleware.clone(),
        db: Arc::new(tokio::sync::Mutex::new(conn)),
        rate_limits: Arc::new(RwLock::new(HashMap::new())),
    }
}

fn build_router(state: ServerState) -> Router {
    let mut app = Router::new();

    // Add security headers layer (always applied)
    app = app.layer(SetResponseHeaderLayer::if_not_present(
        header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"),
    ));
    app = app.layer(SetResponseHeaderLayer::if_not_present(
        header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"),
    ));
    app = app.layer(SetResponseHeaderLayer::if_not_present(
        header::STRICT_TRANSPORT_SECURITY, HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    ));
    app = app.layer(SetResponseHeaderLayer::if_not_present(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'"),
    ));

    // Register routes
    for route in &state.routes {
        let path = route.path.clone();
        let handler = route_handler;

        match route.method.as_str() {
            "GET" => app = app.route(&path, get(handler)),
            "POST" => app = app.route(&path, post(handler)),
            "PUT" => app = app.route(&path, put(handler)),
            "DELETE" => app = app.route(&path, delete(handler)),
            _ => app = app.route(&path, any(handler)),
        }
    }

    app.with_state(state)
}

// ── Route Handler ──────────────────────────────────────────────────

async fn route_handler(
    State(state): State<ServerState>,
    uri: Uri,
    method: Method,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> Response {
    // 0. Extract client IP for rate limiting
    let client_ip = extract_client_ip(&headers);

    // 0b. Bug 2.1 fix: parse query string from URI
    let query: std::collections::HashMap<String, String> = uri.query()
        .map(|q| {
            q.split('&')
                .filter_map(|pair| {
                    if pair.is_empty() { return None; }
                    let mut parts = pair.splitn(2, '=');
                    let key = parts.next()?;
                    let val = parts.next().unwrap_or("");
                    // URL-decode: handle %XX escapes
                    let key = url_decode_fallback(key);
                    let val = url_decode_fallback(val);
                    Some((key, val))
                })
                .collect()
        })
        .unwrap_or_default();

    // 1. Rate limiting (Phase 7.4)
    if state.middleware.contains(&"rate_limit".to_string()) {
        if let Err(resp) = check_rate_limit(&state, &client_ip, 100).await {
            return resp;
        }
    }

    // 2. CSRF check for mutating methods (Phase 7.4: real double-submit)
    if matches!(method, Method::POST | Method::PUT | Method::DELETE) {
        if state.middleware.contains(&"csrf".to_string()) {
            if let Err(resp) = check_csrf(&state, &headers).await {
                return resp;
            }
        }
    }

    // 3. Session expiry check (Phase 7.4: SQLite-backed)
    if state.middleware.contains(&"session".to_string()) {
        if let Some(session_id) = extract_session_cookie(&headers) {
            // Verify HMAC signature first
            let verified = verify_cookie(&session_id, &state.hmac_key);
            if let Some(raw_id) = verified {
                if let Err(resp) = validate_session_in_db(&state, &raw_id).await {
                    return resp;
                }
            }
        }
    }

    // 4. Find matching route by path AND method
    let matched_route = state.routes.iter().find(|r| {
        r.path == uri.path() && r.method == method.as_str()
    });

    if let Some(route) = matched_route {
        // Role check
        if !route.requires.is_empty() {
            if state.middleware.contains(&"session".to_string()) {
                if let Err(resp) = check_roles(&state, &headers, &route.requires).await {
                    return resp;
                }
            }
        }

        // Execute route handler body
        let result = execute_route_body(&state, &route.body, &headers, &body, &query).await;
        let mut response = match result {
            Ok(response) => response,
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Handler error: {}", e),
            ).into_response(),
        };

        // 5. On GET with CSRF middleware, generate and set CSRF token cookie (Phase 7.4)
        if method == Method::GET && state.middleware.contains(&"csrf".to_string()) {
            let token = generate_csrf_token();
            {
                let mut tokens = state.csrf_tokens.write().await;
                tokens.insert(token.clone(), token.clone());
            }
            let cookie_value = format!(
                "_mlog_csrf={}; HttpOnly; SameSite=Strict; Path=/",
                token
            );
            if let Ok(val) = HeaderValue::from_str(&cookie_value) {
                response.headers_mut().append(header::SET_COOKIE, val);
            }
        }

        response
    } else {
        (StatusCode::NOT_FOUND, "404 Not Found").into_response()
    }
}

// ── CSRF Middleware (Phase 7.4: real double-submit) ────────────────

/// Generate a cryptographically random CSRF token (32 hex chars).
pub fn generate_csrf_token() -> String {
    use rand::Rng;
    let mut buf = [0u8; 16];
    rand::thread_rng().fill(&mut buf[..]);
    hex::encode(buf)
}

async fn check_csrf(state: &ServerState, headers: &HeaderMap) -> Result<(), Response> {
    // Read CSRF token from cookie
    let cookie_token = headers.get("cookie")
        .and_then(|c| c.to_str().ok())
        .and_then(|s| extract_cookie(s, "_mlog_csrf"));

    // Read CSRF token from header (X-CSRF-Token) or form field (_csrf)
    let header_token = headers.get("x-csrf-token")
        .and_then(|t| t.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| {
            // Also check content-type for form data with _csrf field
            headers.get("x-csrf-field")
                .and_then(|t| t.to_str().ok())
                .map(|s| s.to_string())
        });

    match (cookie_token, header_token) {
        (Some(cookie), Some(header)) if cookie == header => Ok(()),
        _ => {
            // Log to audit
            {
                let mut log = state.audit_log.write().await;
                log.push("[CSRF] Rejected: missing or mismatched CSRF token".to_string());
            }
            Err((StatusCode::FORBIDDEN, "403 Forbidden: CSRF token validation failed").into_response())
        }
    }
}

fn extract_cookie(cookie_header: &str, name: &str) -> Option<String> {
    for pair in cookie_header.split(';') {
        let pair = pair.trim();
        if let Some(eq_pos) = pair.find('=') {
            let key = &pair[..eq_pos];
            let val = &pair[eq_pos + 1..];
            if key.trim() == name {
                return Some(val.trim().to_string());
            }
        }
    }
    None
}

/// Extract the session cookie (unsigned) from the Cookie header.
fn extract_session_cookie(headers: &HeaderMap) -> Option<String> {
    headers.get("cookie")
        .and_then(|c| c.to_str().ok())
        .and_then(|s| extract_cookie(s, "_mlog_session"))
}

/// Extract client IP from headers (x-forwarded-for or x-real-ip).
fn extract_client_ip(headers: &HeaderMap) -> String {
    headers.get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or("unknown").trim().to_string())
        .or_else(|| {
            headers.get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

// ── Rate Limiting (Phase 7.4) ─────────────────────────────────────

/// Check rate limit using sliding window. Returns Err(429) if exceeded.
pub async fn check_rate_limit(
    state: &ServerState,
    ip: &str,
    max_per_minute: usize,
) -> Result<(), Response> {
    let mut limits = state.rate_limits.write().await;
    let now = std::time::Instant::now();
    let window_start = now - std::time::Duration::from_secs(60);

    let entries = limits.entry(ip.to_string()).or_default();
    // Remove entries outside the 60-second window
    entries.retain(|&t| t > window_start);

    if entries.len() >= max_per_minute {
        {
            let mut log = state.audit_log.write().await;
            log.push(format!("[RATE_LIMIT] Rejected: {} exceeded {} req/min", ip, max_per_minute));
        }
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "429 Too Many Requests: rate limit exceeded",
        ).into_response());
    }

    entries.push(now);
    Ok(())
}

// ── Session & Role Middleware ────────────────────────────────────────

async fn check_roles(state: &ServerState, headers: &HeaderMap, required_roles: &[String]) -> Result<(), Response> {
    let session_cookie = extract_session_cookie(headers);

    let raw_id = match session_cookie {
        Some(id) => {
            // Verify HMAC signature
            match verify_cookie(&id, &state.hmac_key) {
                Some(raw) => raw,
                None => {
                    let mut log = state.audit_log.write().await;
                    log.push("[AUTH] Rejected: tampered session cookie".to_string());
                    return Err((StatusCode::UNAUTHORIZED, "401 Unauthorized: invalid session signature").into_response());
                }
            }
        }
        None => {
            let mut log = state.audit_log.write().await;
            log.push("[AUTH] Rejected: no session cookie".to_string());
            return Err((StatusCode::UNAUTHORIZED, "401 Unauthorized: no session").into_response());
        }
    };

    // Check in-memory cache first, then SQLite
    let sessions = state.sessions.read().await;
    if let Some(entry) = sessions.get(&raw_id) {
        if entry.expires < std::time::Instant::now() {
            return Err((StatusCode::UNAUTHORIZED, "401 Unauthorized: session expired").into_response());
        }
        let has_role = required_roles.iter().any(|role| entry.roles.contains(role));
        if has_role {
            Ok(())
        } else {
            drop(sessions);
            let mut log = state.audit_log.write().await;
            log.push(format!("[AUTH] Rejected: insufficient roles (need {:?}, have {:?})",
                required_roles, Vec::<String>::new()));
            Err((StatusCode::FORBIDDEN, "403 Forbidden: insufficient permissions").into_response())
        }
    } else {
        // Fall through to SQLite check
        drop(sessions);
        validate_session_in_db(&state, &raw_id).await?;
        // If valid but not in memory cache, load from DB
        // For simplicity, reject here — session needs re-login
        Err((StatusCode::UNAUTHORIZED, "401 Unauthorized: session not found in cache").into_response())
    }
}

// ── SQLite Session Store (Phase 7.4) ─────────────────────────────

/// Initialize the sessions table in SQLite.
pub fn init_session_db(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            data TEXT NOT NULL DEFAULT '{}',
            created_at INTEGER NOT NULL,
            expires_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_sessions_expires ON sessions(expires_at);

        -- Phase 7.5: Audit log table for interpreter audit entries
        CREATE TABLE IF NOT EXISTS audit_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            action TEXT NOT NULL,
            pattern TEXT,
            result TEXT,
            sandbox TEXT
        );"
    )?;
    Ok(())
}

/// Create a new session in SQLite. Returns the session ID (UUID).
pub async fn create_session_db(
    conn: &Arc<tokio::sync::Mutex<rusqlite::Connection>>,
    user_id: &str,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let expires_at = now + 24 * 3600; // 24 hours

    let conn = conn.lock().await;
    conn.execute(
        "INSERT INTO sessions (id, user_id, data, created_at, expires_at) VALUES (?1, ?2, '{}', ?3, ?4)",
        rusqlite::params![id, user_id, now, expires_at],
    ).map_err(|e| format!("Failed to create session: {}", e))?;

    Ok(id)
}

/// Validate a session against SQLite: check existence and expiry.
/// Returns Ok(()) if valid, Err(Response) if expired or not found.
pub async fn validate_session_in_db(state: &ServerState, session_id: &str) -> Result<(), Response> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let conn = state.db.lock().await;
    let result: Result<String, _> = conn.query_row(
        "SELECT id FROM sessions WHERE id = ?1 AND expires_at > ?2",
        rusqlite::params![session_id, now],
        |row| row.get(0),
    );
    drop(conn); // release lock before async ops

    match result {
        Ok(_) => Ok(()),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            let mut log = state.audit_log.write().await;
            log.push("[AUTH] Rejected: session expired or not found in DB".to_string());
            Err((StatusCode::UNAUTHORIZED, "401 Unauthorized: session expired").into_response())
        }
        Err(e) => {
            Err((StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response())
        }
    }
}

/// Delete a session from SQLite.
pub async fn delete_session_db(
    conn: &Arc<tokio::sync::Mutex<rusqlite::Connection>>,
    session_id: &str,
) -> Result<(), String> {
    let conn = conn.lock().await;
    conn.execute("DELETE FROM sessions WHERE id = ?1", rusqlite::params![session_id])
        .map_err(|e| format!("Failed to delete session: {}", e))?;
    Ok(())
}

/// Remove all expired sessions from SQLite.
pub async fn clean_expired_sessions_db(
    conn: &Arc<tokio::sync::Mutex<rusqlite::Connection>>,
) -> Result<usize, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let conn = conn.lock().await;
    let deleted = conn.execute("DELETE FROM sessions WHERE expires_at <= ?1", rusqlite::params![now])
        .map_err(|e| format!("Failed to clean expired sessions: {}", e))?;
    Ok(deleted)
}

/// Build a Set-Cookie header value for _mlog_session.
pub fn make_session_cookie_value(session_id: &str, signed: bool, hmac_key: &[u8]) -> String {
    let value = if signed {
        sign_cookie(session_id, hmac_key)
    } else {
        session_id.to_string()
    };
    format!(
        "_mlog_session={}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=86400",
        value
    )
}

// ── JSON → Value Conversion (Наряд №3) ──────────────────────────

/// Recursively convert serde_json::Value → metalogos Value.
/// Supports nested objects (→ Value::Struct), arrays, strings, numbers, bools, null.
pub fn json_value_to_value(val: &serde_json::Value) -> Value {
    match val {
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Number(n) => Value::Float(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Null => Value::Unit,
        serde_json::Value::Array(arr) => {
            Value::List(arr.iter().map(json_value_to_value).collect())
        }
        serde_json::Value::Object(map) => {
            let fields: HashMap<String, Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), json_value_to_value(v)))
                .collect();
            Value::Struct {
                type_name: "JsonObject".to_string(),
                fields,
            }
        }
    }
}

// ── Route Body Execution ────────────────────────────────────────────

async fn execute_route_body(
    state: &ServerState,
    body_stmts: &[Statement],
    _headers: &HeaderMap,
    raw_body: &bytes::Bytes,
    query_params: &std::collections::HashMap<String, String>,
) -> Result<Response, String> {
    // Set up interpreter with request context (Наряд №8: route pattern invocation fix)
    let mut interp = Interpreter::new();
    // Copy ALL program definitions (patterns, learnables, templates, struct types,
    // rules, sandboxes, namespaces, variables, db_config, db_url) from shared interpreter.
    {
        let shared = state.interpreter.read().await;
        shared.clone_definitions_into(&mut interp);
    }
    interp.set_base_dir(std::path::PathBuf::from("."));

    // Initialize memory persistence (per-request SQLite connection to shared DB)
    if let Some(ref persist_path) = state.memory_persist {
        interp.configure_memory(&MemoryDecl { persist: Some(persist_path.clone()) });
    }

    // Initialize DB connection for per-request interpreter (query() / db_execute())
    // Opens a NEW connection to the same database, so concurrent requests are safe.
    interp.reconnect_db();

    // Parse JSON body recursively and inject as json_body() server builtin (Наряд №3)
    if let Ok(body_str) = std::str::from_utf8(raw_body) {
        if !body_str.is_empty() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(body_str) {
                let value = json_value_to_value(&json);
                interp.set_server_json_body(value);
            }
        }
    }

    // Bug 2.1 fix: inject query string parameters so query_param() works
    if !query_params.is_empty() {
        interp.set_server_query_params(query_params.clone());
    }

    // Наряд №14 P2-6: inject user roles for require() builtin
    if state.middleware.contains(&"session".to_string()) {
        if let Some(session_id) = extract_session_cookie(_headers) {
            if let Some(raw_id) = verify_cookie(&session_id, &state.hmac_key) {
                let sessions = state.sessions.read().await;
                if let Some(entry) = sessions.get(&raw_id) {
                    interp.set_server_user_roles(entry.roles.clone());
                }
            }
        }
    }

    // Execute body statements
    let mut env = HashMap::new();
    for stmt in body_stmts {
        match stmt {
            Statement::LetBinding { name, value, mutable: _ } => {
                let val = interp.eval_expr_with_env(value, &env)?;
                env.insert(name.clone(), val);
            }
            Statement::Assign { name, value } => {
                let val = interp.eval_expr_with_env(value, &env)?;
                if env.contains_key(name) {
                    env.insert(name.clone(), val);
                }
            }
            Statement::Return(expr) => {
                let val = interp.eval_expr_with_env(expr, &env)?;
                // Phase 7.5: Flush interpreter audit entries to SQLite
                flush_audit_to_db(state, &mut interp).await;
                return Ok(value_to_response(val));
            }
            Statement::IfThen(cond, body) => {
                let cond_val = interp.eval_expr_with_env(cond, &env)?;
                if cond_val.as_bool().unwrap_or(false) {
                    let result = interp.eval_statements(body, &mut env)?;
                    if !matches!(result, Value::Unit) {
                        flush_audit_to_db(state, &mut interp).await;
                        return Ok(value_to_response(result));
                    }
                }
            }
            // Block-level if/else (Наряд №2 + final integration)
            Statement::IfElseBlock { condition, then_body, else_ifs, else_body } => {
                let cond_val = interp.eval_expr_with_env(condition, &env)?;
                let branch = if cond_val.as_bool().unwrap_or(false) {
                    Some(then_body.as_slice())
                } else {
                    // Check else-if chain
                    let mut matched = None;
                    for (ei_cond, ei_body) in else_ifs {
                        let ei_val = interp.eval_expr_with_env(ei_cond, &env)?;
                        if ei_val.as_bool().unwrap_or(false) {
                            matched = Some(ei_body.as_slice());
                            break;
                        }
                    }
                    matched.or_else(|| else_body.as_deref())
                };
                if let Some(stmts) = branch {
                    for s in stmts {
                        match s {
                            Statement::Return(expr) => {
                                let val = interp.eval_expr_with_env(expr, &env)?;
                                flush_audit_to_db(state, &mut interp).await;
                                return Ok(value_to_response(val));
                            }
                            Statement::LetBinding { name, value, mutable: _ } => {
                                let val = interp.eval_expr_with_env(value, &env)?;
                                env.insert(name.clone(), val);
                            }
                            Statement::ExprStmt(expr) => {
                                let val = interp.eval_expr_with_env(expr, &env)?;
                                if let Value::HttpResponse { .. } = val {
                                    flush_audit_to_db(state, &mut interp).await;
                                    return Ok(value_to_response(val));
                                }
                            }
                            _ => { interp.eval_statements(&[s.clone()], &mut env)?; }
                        }
                    }
                }
            }
            // Bare expression statement — evaluate for side effects
            Statement::ExprStmt(expr) => {
                let val = interp.eval_expr_with_env(expr, &env)?;
                // If expression is respond("ok") or similar HttpResponse, use as route response
                if let Value::HttpResponse { .. } = val {
                    flush_audit_to_db(state, &mut interp).await;
                    return Ok(value_to_response(val));
                }
            }
            _ => {
                let result = interp.eval_statements(&[stmt.clone()], &mut env)?;
                // If the statement produced an HttpResponse (e.g., respond("ok")),
                // use it as the route response (final integration)
                if let Value::HttpResponse { .. } = result {
                    flush_audit_to_db(state, &mut interp).await;
                    return Ok(value_to_response(result));
                }
            }
        }
    }

    // Phase 7.5: Flush interpreter audit entries to SQLite before response
    flush_audit_to_db(state, &mut interp).await;

    Ok((StatusCode::OK, "OK").into_response())
}

/// Phase 7.5: Write interpreter audit entries to the SQLite audit_log table.
async fn flush_audit_to_db(state: &ServerState, interp: &mut Interpreter) {
    let entries = interp.take_audit_log();
    if entries.is_empty() {
        return;
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    // Get active sandbox name for context
    let sandbox_name = interp.get_active_sandbox().map(|sb| sb.name.clone());

    let conn = state.db.lock().await;
    for entry in &entries {
        // Parse entry to extract action and pattern name
        let (action, pattern, result) = parse_audit_entry(entry);
        let _ = conn.execute(
            "INSERT INTO audit_log (timestamp, action, pattern, result, sandbox) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![now, action, pattern, result, sandbox_name.as_deref().unwrap_or("")],
        );
    }
    // Also append to in-memory audit log for backward compatibility
    {
        let mut log = state.audit_log.write().await;
        for entry in &entries {
            log.push(entry.clone());
        }
    }
}

/// Parse an audit entry string into (action, pattern, result) components.
/// Format: "[AUDIT] adapt PatternName: input -> output"
///         "[AUDIT] mutate PatternName: N examples, accuracy=X"
///         "[AUDIT] unsafe_html: rendered template 'name'"
fn parse_audit_entry(entry: &str) -> (String, Option<String>, Option<String>) {
    if let Some(rest) = entry.strip_prefix("[AUDIT] ") {
        let parts: Vec<&str> = rest.splitn(2, ' ').collect();
        let action = parts[0].to_string();
        let detail = if parts.len() > 1 { Some(parts[1].to_string()) } else { None };

        match action.as_str() {
            "adapt" | "mutate" => {
                // Extract pattern name (first word of detail)
                let pattern = detail.as_ref().and_then(|d| d.split(':').next()).map(|s| s.trim().to_string());
                let result = detail.as_ref().and_then(|d| {
                    d.splitn(2, ':').nth(1).map(|s| s.trim().to_string())
                });
                (action, pattern, result)
            }
            "unsafe_html" => {
                let pattern = detail.as_ref().and_then(|d| d.split('\'').nth(1)).map(|s| s.to_string());
                (action, pattern, None)
            }
            _ => (action, None, None),
        }
    } else {
        ("unknown".to_string(), None, None)
    }
}

fn value_to_response(val: Value) -> Response {
    match val {
        Value::HttpResponse { status, body } => {
            let code = StatusCode::from_u16(status).unwrap_or(StatusCode::OK);
            (code, body).into_response()
        }
        Value::Html(html) => {
            AxumHtml(html).into_response()
        }
        Value::String(s) => {
            (StatusCode::OK, s).into_response()
        }
        Value::Unit => {
            StatusCode::OK.into_response()
        }
        other => {
            (StatusCode::OK, format!("{}", other)).into_response()
        }
    }
}

// ── HMAC Helpers ───────────────────────────────────────────────────

fn generate_hmac_key() -> Vec<u8> {
    use rand::Rng;
    let mut key = vec![0u8; 32];
    rand::thread_rng().fill(&mut key[..]);
    key
}

pub fn sign_cookie(value: &str, key: &[u8]) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key error");
    mac.update(value.as_bytes());
    let result = mac.finalize();
    let signature = hex::encode(result.into_bytes());
    format!("{}.{}", value, signature)
}

pub fn verify_cookie(cookie: &str, key: &[u8]) -> Option<String> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let parts: Vec<&str> = cookie.rsplitn(2, '.').collect();
    if parts.len() != 2 {
        return None;
    }
    let signature = parts[0];
    let value = parts[1];

    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key error");
    mac.update(value.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());

    if signature == expected {
        Some(value.to_string())
    } else {
        None
    }
}

// ── HTML Auto-Escaping ─────────────────────────────────────────────

/// Escape HTML special characters to prevent XSS.
pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&#x27;")
}

/// Simple template rendering: replace {{ var }} with escaped values.
pub fn render_template(body: &str, vars: &HashMap<String, String>) -> String {
    let mut result = body.to_string();
    for (key, val) in vars {
        let escaped = escape_html(val);
        result = result.replace(&format!("{{{{{}}}}}", key), &escaped);
    }
    result
}

// ── Interpreter Merge ──────────────────────────────────────────────

fn build_interpreter_with_server(srv: &MlogServerDecl, mut interp: Interpreter) -> Interpreter {
    interp = merge_templates(srv, interp);
    interp
}

fn merge_interpreter(from: Interpreter, mut into: Interpreter) -> Interpreter {
    // Merge variables (borrow, don't move)
    for (k, v) in &from.variables {
        into.variables.entry(k.clone()).or_insert(v.clone());
    }
    // Merge templates
    for (k, v) in from.get_templates() {
        into.templates.entry(k.clone()).or_insert(v.clone());
    }
    // Propagate memory persist path
    if let Some(path) = from.get_memory_persist_path() {
        into.set_memory_persist_path(Some(path));
    }
    // Merge patterns, struct types, learnable patterns, rules, sandboxes, module namespaces
    from.clone_definitions_into(&mut into);
    into
}

fn merge_templates(_srv: &MlogServerDecl, interp: Interpreter) -> Interpreter {
    // Templates are added during run() already
    interp
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Phase 6 tests (unchanged) ──

    #[test]
    fn test_escape_html_prevents_xss() {
        assert_eq!(escape_html("<script>alert(1)</script>"),
                   "&lt;script&gt;alert(1)&lt;/script&gt;");
        assert_eq!(escape_html("Hello & \"world\""),
                   "Hello &amp; &quot;world&quot;");
    }

    #[test]
    fn test_hmac_cookie_signing() {
        let key = generate_hmac_key();
        let value = "session_abc123";
        let signed = sign_cookie(value, &key);
        assert!(signed.contains('.'));
        let verified = verify_cookie(&signed, &key);
        assert_eq!(verified, Some(value.to_string()));
    }

    #[test]
    fn test_hmac_tamper_detection() {
        let key = generate_hmac_key();
        let value = "session_abc123";
        let signed = sign_cookie(value, &key);
        let tampered = format!("{}.deadbeef", value);
        let verified = verify_cookie(&tampered, &key);
        assert!(verified.is_none());
    }

    #[test]
    fn test_opaque_types_in_value_enum() {
        let html = Value::Html("<h1>Test</h1>".to_string());
        assert_eq!(html.type_name(), "Html");
        assert_eq!(format!("{}", html), "[Html]");

        let secret = Value::Secret(crate::interpreter::SecretString::new("my-api-key".to_string()));
        assert_eq!(secret.type_name(), "Secret");
        assert_eq!(format!("{}", secret), "[Secret]");

        let query = Value::Query("SELECT * FROM users".to_string());
        assert_eq!(query.type_name(), "Query");
        assert_eq!(format!("{}", query), "[Query]");
    }

    // ── Phase 7.4 Contract Tests ──

    #[test]
    fn test_74_csrf_token_generation_is_random() {
        let t1 = generate_csrf_token();
        let t2 = generate_csrf_token();
        assert_ne!(t1, t2);
        assert_eq!(t1.len(), 32); // 16 bytes = 32 hex chars
        assert!(hex::decode(&t1).is_ok());
    }

    #[tokio::test]
    async fn test_74_post_without_csrf_returns_403() {
        // Simulate a POST request without CSRF cookie or header
        let state = make_test_state().await;

        let mut headers = HeaderMap::new();
        // No _mlog_csrf cookie, no x-csrf-token header
        headers.insert("cookie", HeaderValue::from_static("other=value"));

        let result = check_csrf(&state, &headers).await;
        assert!(result.is_err());
        let resp = result.unwrap_err();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_74_post_with_matching_csrf_returns_ok() {
        let state = make_test_state().await;
        let token = generate_csrf_token();

        // Store token in state (simulating cookie set on previous GET)
        {
            let mut tokens = state.csrf_tokens.write().await;
            tokens.insert(token.clone(), token.clone());
        }

        // Simulate POST with matching cookie and header
        let mut headers = HeaderMap::new();
        headers.insert("cookie", HeaderValue::from_str(
            &format!("_mlog_csrf={}", token)
        ).unwrap());
        headers.insert("x-csrf-token", HeaderValue::from_str(&token).unwrap());

        let result = check_csrf(&state, &headers).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_74_post_with_mismatched_csrf_returns_403() {
        let state = make_test_state().await;
        let token = generate_csrf_token();

        // Cookie has one token, header has different one
        let mut headers = HeaderMap::new();
        headers.insert("cookie", HeaderValue::from_str(
            &format!("_mlog_csrf={}", token)
        ).unwrap());
        headers.insert("x-csrf-token", HeaderValue::from_str("wrong_token_value").unwrap());

        let result = check_csrf(&state, &headers).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_74_expired_session_returns_401() {
        let state = make_test_state().await;

        // Create a session that's already expired
        let conn = state.db.lock().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let past = now - 3600; // 1 hour ago

        conn.execute(
            "INSERT INTO sessions (id, user_id, data, created_at, expires_at) VALUES (?1, ?2, '{}', ?3, ?4)",
            rusqlite::params!["expired-session-id", "user1", now, past],
        ).unwrap();
        drop(conn);

        let result = validate_session_in_db(&state, "expired-session-id").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_74_valid_session_returns_ok() {
        let state = make_test_state().await;

        // Create a valid session (expires in 24 hours)
        let session_id = create_session_db(&state.db, "user1").await.unwrap();

        let result = validate_session_in_db(&state, &session_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_74_nonexistent_session_returns_401() {
        let state = make_test_state().await;

        let result = validate_session_in_db(&state, "nonexistent-id").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_74_rate_limit_under_threshold_passes() {
        let state = make_test_state().await;

        // 50 requests should pass (limit is 100/min)
        for _ in 0..50 {
            let result = check_rate_limit(&state, "192.168.1.1", 100).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_74_rate_limit_exceeded_returns_429() {
        let state = make_test_state().await;

        // Fill up to limit
        for _ in 0..100 {
            let _ = check_rate_limit(&state, "192.168.1.2", 100).await;
        }

        // 101st should fail
        let result = check_rate_limit(&state, "192.168.1.2", 100).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_74_rate_limit_per_ip_isolated() {
        let state = make_test_state().await;

        // Exhaust limit for IP A
        for _ in 0..100 {
            let _ = check_rate_limit(&state, "ip-a", 100).await;
        }
        let result_a = check_rate_limit(&state, "ip-a", 100).await;
        assert!(result_a.is_err());

        // IP B should still be fine
        let result_b = check_rate_limit(&state, "ip-b", 100).await;
        assert!(result_b.is_ok());
    }

    #[tokio::test]
    async fn test_74_session_create_and_delete() {
        let state = make_test_state().await;

        let id = create_session_db(&state.db, "testuser").await.unwrap();
        assert!(!id.is_empty());

        // Verify it exists in DB
        let conn = state.db.lock().await;
        let found: Result<String, _> = conn.query_row(
            "SELECT user_id FROM sessions WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        );
        assert_eq!(found.unwrap(), "testuser");
        drop(conn);

        // Delete it
        delete_session_db(&state.db, &id).await.unwrap();

        // Verify deleted
        let conn = state.db.lock().await;
        let result: Result<String, _> = conn.query_row(
            "SELECT user_id FROM sessions WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        );
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_74_clean_expired_sessions() {
        let state = make_test_state().await;

        let conn = state.db.lock().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let past = now - 7200;

        // Insert expired session
        conn.execute(
            "INSERT INTO sessions (id, user_id, data, created_at, expires_at) VALUES (?1, ?2, '{}', ?3, ?4)",
            rusqlite::params!["expired-1", "old_user", now, past],
        ).unwrap();

        // Insert valid session
        let future = now + 86400;
        conn.execute(
            "INSERT INTO sessions (id, user_id, data, created_at, expires_at) VALUES (?1, ?2, '{}', ?3, ?4)",
            rusqlite::params!["valid-1", "current_user", now, future],
        ).unwrap();
        drop(conn);

        // Clean expired
        let deleted = clean_expired_sessions_db(&state.db).await.unwrap();
        assert_eq!(deleted, 1);

        // Verify expired is gone, valid remains
        let conn = state.db.lock().await;
        let expired_exists: bool = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE id = 'expired-1'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(expired_exists, false);

        let valid_exists: bool = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE id = 'valid-1'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(valid_exists, true);
    }

    #[test]
    fn test_74_extract_client_ip_from_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("10.0.0.1, 172.16.0.1"));
        assert_eq!(extract_client_ip(&headers), "10.0.0.1");

        let mut headers2 = HeaderMap::new();
        headers2.insert("x-real-ip", HeaderValue::from_static("192.168.1.100"));
        assert_eq!(extract_client_ip(&headers2), "192.168.1.100");

        let headers3 = HeaderMap::new();
        assert_eq!(extract_client_ip(&headers3), "unknown");
    }

    #[test]
    fn test_74_make_session_cookie_value() {
        let key = generate_hmac_key();
        let cookie = make_session_cookie_value("abc123", true, &key);
        assert!(cookie.starts_with("_mlog_session="));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("Secure"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(cookie.contains("Max-Age=86400"));
    }

    /// Helper: create a ServerState for testing (with in-memory SQLite).
    async fn make_test_state() -> ServerState {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        init_session_db(&conn).unwrap();

        ServerState {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            csrf_tokens: Arc::new(RwLock::new(HashMap::new())),
            hmac_key: Arc::new(generate_hmac_key()),
            audit_log: Arc::new(RwLock::new(Vec::new())),
            templates: Arc::new(RwLock::new(HashMap::new())),
            db_store: Arc::new(RwLock::new(Vec::new())),
            memory_persist: None,
            interpreter: Arc::new(RwLock::new(Interpreter::new())),
            routes: Vec::new(),
            middleware: vec!["session".to_string(), "csrf".to_string(), "rate_limit".to_string()],
            db: Arc::new(tokio::sync::Mutex::new(conn)),
            rate_limits: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
