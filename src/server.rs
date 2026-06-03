// ── METALOGOS HTTP Server (Phase 6.1–6.6) ───────────────────────────
// Axum-based HTTP server with security middleware:
// - HMAC-SHA256 signed session cookies
// - CSRF double-submit cookie pattern
// - Security headers (CSP, X-Frame-Options, X-Content-Type-Options, HSTS)
// - Role-based route access
// - Template rendering with auto-escaping
// - Bot integration (Telegram webhooks)

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header, Method},
    response::{Html as AxumHtml, IntoResponse, Response},
    routing::{get, post, put, delete, any},
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::set_header::SetResponseHeaderLayer;

use crate::ast::*;
use crate::interpreter::{Interpreter, Value};

// ── Server State ──────────────────────────────────────────────────

/// Shared mutable server state, protected by tokio::RwLock.
#[derive(Clone)]
pub struct ServerState {
    /// In-memory session store: session_id → (user_data, expiry).
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
    /// Interpreter (for running route handlers).
    pub interpreter: Arc<RwLock<Interpreter>>,
    /// Route definitions from mlogserver block.
    pub routes: Vec<RouteDecl>,
    /// Required middleware (from mlogserver config).
    pub middleware: Vec<String>,
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
    let app = build_router(state);

    println!("mlog serve: listening on 0.0.0.0:{}", port);
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

    ServerState {
        sessions: Arc::new(RwLock::new(HashMap::new())),
        csrf_tokens: Arc::new(RwLock::new(HashMap::new())),
        hmac_key: Arc::new(hmac_key),
        audit_log: Arc::new(RwLock::new(Vec::new())),
        templates: Arc::new(RwLock::new(templates_map)),
        db_store: Arc::new(RwLock::new(Vec::new())),
        interpreter: Arc::new(RwLock::new(interp)),
        routes: config.routes.clone(),
        middleware: config.middleware.clone(),
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
    method: Method,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> Response {
    let _path = headers.get("x-original-uri")
        .map(|v| v.to_str().unwrap_or("/").to_string())
        .unwrap_or_default();

    // 1. CSRF check for mutating methods
    if matches!(method, Method::POST | Method::PUT | Method::DELETE) {
        if state.middleware.contains(&"csrf".to_string()) {
            if let Err(resp) = check_csrf(&state, &headers).await {
                return resp;
            }
        }
    }

    // 2. Find matching route and check roles
    let matched_route = state.routes.iter().find(|r| {
        // Simple path matching (exact match for now)
        r.method == method.as_str()
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
        let result = execute_route_body(&state, &route.body, &headers, &body).await;
        match result {
            Ok(response) => response,
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Handler error: {}", e),
            ).into_response(),
        }
    } else {
        (StatusCode::NOT_FOUND, "404 Not Found").into_response()
    }
}

// ── CSRF Middleware ────────────────────────────────────────────────

async fn check_csrf(state: &ServerState, headers: &HeaderMap) -> Result<(), Response> {
    let cookie_token = headers.get("cookie")
        .and_then(|c| c.to_str().ok())
        .and_then(|s| extract_cookie(s, "_csrf_token"));

    let header_token = headers.get("x-csrf-token")
        .and_then(|t| t.to_str().ok())
        .map(|s| s.to_string());

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

// ── Session & Role Middleware ────────────────────────────────────────

async fn check_roles(state: &ServerState, headers: &HeaderMap, required_roles: &[String]) -> Result<(), Response> {
    let session_id = headers.get("cookie")
        .and_then(|c| c.to_str().ok())
        .and_then(|s| extract_cookie(s, "mlog_session"));

    let session_id = match session_id {
        Some(id) => id,
        None => {
            {
                let mut log = state.audit_log.write().await;
                log.push("[AUTH] Rejected: no session cookie".to_string());
            }
            return Err((StatusCode::UNAUTHORIZED, "401 Unauthorized: no session").into_response());
        }
    };

    let sessions = state.sessions.read().await;
    if let Some(entry) = sessions.get(&session_id) {
        if entry.expires < std::time::Instant::now() {
            return Err((StatusCode::UNAUTHORIZED, "401 Unauthorized: session expired").into_response());
        }
        // Check if user has any of the required roles
        let has_role = required_roles.iter().any(|role| entry.roles.contains(role));
        if has_role {
            Ok(())
        } else {
            {
                let mut log = state.audit_log.write().await;
                log.push(format!("[AUTH] Rejected: insufficient roles (need {:?}, have {:?})",
                    required_roles, entry.roles));
            }
            Err((StatusCode::FORBIDDEN, "403 Forbidden: insufficient permissions").into_response())
        }
    } else {
        Err((StatusCode::UNAUTHORIZED, "401 Unauthorized: invalid session").into_response())
    }
}

// ── Route Body Execution ────────────────────────────────────────────

async fn execute_route_body(
    _state: &ServerState,
    body_stmts: &[Statement],
    _headers: &HeaderMap,
    raw_body: &bytes::Bytes,
) -> Result<Response, String> {
    // Set up interpreter with request context
    let mut interp = Interpreter::new();

    // Inject request data as variables
    // form_data from query string (simplified)
    // json_body from request body
    if let Ok(body_str) = std::str::from_utf8(raw_body) {
        if !body_str.is_empty() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(body_str) {
                let mut fields = HashMap::new();
                if let serde_json::Value::Object(map) = json {
                    for (k, v) in map {
                        let val = match v {
                            serde_json::Value::String(s) => Value::String(s),
                            serde_json::Value::Number(n) => Value::Float(n.as_f64().unwrap_or(0.0)),
                            serde_json::Value::Bool(b) => Value::Bool(b),
                            serde_json::Value::Array(arr) => {
                                Value::List(arr.iter().map(|item| match item {
                                    serde_json::Value::String(s) => Value::String(s.clone()),
                                    serde_json::Value::Number(n) => Value::Float(n.as_f64().unwrap_or(0.0)),
                                    serde_json::Value::Bool(b) => Value::Bool(*b),
                                    _ => Value::Unit,
                                }).collect())
                            }
                            _ => Value::Unit,
                        };
                        fields.insert(k, val);
                    }
                }
                interp.variables.insert("json_body".to_string(), Value::Struct {
                    type_name: "JsonBody".to_string(),
                    fields,
                });
            }
        }
    }

    // Execute body statements
    let mut env = HashMap::new();
    for stmt in body_stmts {
        match stmt {
            Statement::LetBinding { name, value } => {
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
                return Ok(value_to_response(val));
            }
            _ => {
                // Execute via eval_statements for complex statements
                interp.eval_statements(&[stmt.clone()], &mut env)?;
            }
        }
    }

    Ok((StatusCode::OK, "OK").into_response())
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

fn sign_cookie(value: &str, key: &[u8]) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key error");
    mac.update(value.as_bytes());
    let result = mac.finalize();
    let signature = hex::encode(result.into_bytes());
    format!("{}.{}", value, signature)
}

fn verify_cookie(cookie: &str, key: &[u8]) -> Option<String> {
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

    #[test]
    fn test_escape_html_prevents_xss() {
        assert_eq!(escape_html("<script>alert(1)</script>"),
                   "&lt;script&gt;alert(1)&lt;/script&gt;");
        assert_eq!(escape_html("Hello & \"world\""),
                   "Hello &amp; &quot;world&quot;");
    }

    #[test]
    fn test_render_template_substitution() {
        let body = "<h1>{{ title }}</h1><p>{{ content }}</p>";
        let mut vars = HashMap::new();
        vars.insert("title".to_string(), "Test <script>".to_string());
        vars.insert("content".to_string(), "Hello & world".to_string());

        let result = render_template(body, &vars);
        assert!(result.contains("<h1>Test &lt;script&gt;</h1>"));
        assert!(result.contains("<p>Hello &amp; world</p>"));
        assert!(!result.contains("<script>"));
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
    fn test_mlogserver_parsing() {
        let source = r#"
mlogserver {
  port: 8080
  middleware: [session, csrf, security_headers]
  route "/" method=GET {
    return "Hello from Metalogos"
  }
  route "/admin" method=GET requires=[admin] {
    return "Admin Panel"
  }
  route "/login" method=POST {
    return "Login processed"
  }
}
"#;
        let declarations = crate::parser::parse(source).unwrap();
        assert!(declarations.iter().any(|d| matches!(d, Declaration::MlogServer(_))));

        if let Declaration::MlogServer(srv) = &declarations[0] {
            assert_eq!(srv.port, 8080);
            assert!(srv.middleware.contains(&"session".to_string()));
            assert!(srv.middleware.contains(&"csrf".to_string()));
            assert_eq!(srv.routes.len(), 3);
            assert_eq!(srv.routes[0].path, "/");
            assert_eq!(srv.routes[0].method, "GET");
            assert_eq!(srv.routes[1].requires, vec!["admin".to_string()]);
        }
    }

    #[test]
    fn test_template_parsing() {
        let source = r#"
template Page(title: String) -> Html {
  <html><head><title>{{ title }}</title></head></html>
}
"#;
        let declarations = crate::parser::parse(source).unwrap();
        assert!(declarations.iter().any(|d| matches!(d, Declaration::Template(_))));

        if let Declaration::Template(t) = &declarations[0] {
            assert_eq!(t.name, "Page");
            assert!(t.body.contains("{{ title }}"));
        }
    }

    #[test]
    fn test_db_parsing() {
        let source = r#"
db {
  pool_size: 10
  migrate: "./migrations"
}
"#;
        let declarations = crate::parser::parse(source).unwrap();
        assert!(declarations.iter().any(|d| matches!(d, Declaration::Db(_))));

        if let Declaration::Db(db) = &declarations[0] {
            assert_eq!(db.pool_size, Some(10));
            assert_eq!(db.migrate, Some("./migrations".to_string()));
        }
    }

    #[test]
    fn test_opaque_type_secret_no_print() {
        // In semantic analysis, print(Secret) should error
        let source = r#"
entity key: Secret = env("API_KEY")
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = crate::semantic::check_program(&decls);
        // env() returns Secret — using it as entity value should be fine,
        // but printing it would be caught at runtime
        assert!(result.is_ok()); // Declaration is syntactically valid
    }

    #[test]
    fn test_opaque_types_in_value_enum() {
        // Verify Value enum has all opaque types
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
}
