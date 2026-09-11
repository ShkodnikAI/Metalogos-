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
    extract::{connect_info::ConnectInfo, DefaultBodyLimit, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode, Uri},
    response::{Html as AxumHtml, IntoResponse, Response},
    routing::{any, delete, get, post, put},
    Router,
};
use dashmap::DashMap;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::set_header::SetResponseHeaderLayer;

use chrono::{Datelike, Timelike};

use crate::ast::*;
use crate::builtins::io::ServeRouteExecGuard;
use crate::bytecode::{CompiledRoute, Program};
use crate::compiler::Compiler;
use crate::interpreter::{Interpreter, Value};
use crate::vm::Vm;

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
                if step == 0 {
                    continue;
                }
                if value.is_multiple_of(step) {
                    return true;
                }
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
            if step == 0 {
                continue;
            }
            let bounds: Vec<&str> = range_str.split('-').collect();
            if bounds.len() == 2 {
                if let (Ok(lo), Ok(hi)) = (bounds[0].parse::<u32>(), bounds[1].parse::<u32>()) {
                    if value >= lo && value <= hi && (value - lo).is_multiple_of(step) {
                        return true;
                    }
                }
            }
            continue;
        }
        // Plain number
        if let Ok(n) = part.parse::<u32>() {
            if n == value {
                return true;
            }
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
    let dom = now.day(); // 1-31
    let month = now.month(); // 1-12
    let dow = now.weekday().num_days_from_sunday(); // 0=Sun

    cron_field_matches(parts[0], min)
        && cron_field_matches(parts[1], hour)
        && cron_field_matches(parts[2], dom)
        && cron_field_matches(parts[3], month)
        && cron_field_matches(parts[4], dow)
}

/// Percent-decode a query-string key/value (RFC 3986 semantics with the
/// form-urlencoded `+` convention — chosen and documented, Naryad #257).
///
/// Behavior table (each row pinned by `tests/naryad_257_rfc3986.rs`):
/// - unreserved / plain chars pass through unchanged;
/// - `%XX` (two hex digits) decodes to that BYTE — bytes are reassembled
///   and the result is interpreted as UTF-8, so multibyte sequences like
///   `%D0%B6` correctly yield `"ж"` (before #257 they produced mojibake:
///   each byte was pushed as a standalone `char`);
/// - invalid escapes (`%ZZ`, `%G1`) and a truncated `%` at the end pass
///   through literally — a malformed escape is preserved, not swallowed
///   (deliberate deviation from strict RFC rejection: query parsing must
///   never fail on user input);
/// - `+` decodes to space — the `application/x-www-form-urlencoded`
///   convention (what browsers send in query strings of HTML forms and
///   what axum's own form/Query tooling expects); a literal `+` must be
///   sent as `%2B`;
/// - byte sequences that are not valid UTF-8 decode lossily (U+FFFD) —
///   decoding never fails.
///
/// Single-pass by design: `%25D0%25B6` decodes to the literal `%D0%B6`
/// (double encoding requires two decode passes — standard behavior).
pub fn url_decode_fallback(s: &str) -> String {
    let mut bytes: Vec<u8> = Vec::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    bytes.push(byte);
                    continue;
                }
            }
            bytes.extend_from_slice(b"%");
            bytes.extend_from_slice(hex.as_bytes());
        } else if c == '+' {
            bytes.push(b' ');
        } else {
            let mut buf = [0u8; 4];
            bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
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

/// Shared mutable server state.
/// Наряд №29 §5.1: hot-path maps (sessions, csrf_tokens, rate_limits)
/// use DashMap (lock-free) instead of Arc<RwLock<HashMap>>.
#[derive(Clone)]
pub struct ServerState {
    /// In-memory session cache (kept for fast lookups, authoritative source is SQLite).
    pub sessions: Arc<DashMap<String, SessionEntry>>,
    /// CSRF token store for double-submit validation.
    /// Value: (session_id, created_at) — TTL enforcement (15 min, Наряд №29 §2.2) and
    /// session binding (Наряд №262: a token is accepted only from the session it was
    /// issued for; "" binds to sessionless requests). Tokens absent from this store
    /// were never issued by this process — rejected since №262 (no stateless fallback).
    pub csrf_tokens: Arc<DashMap<String, (String, std::time::Instant)>>,
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
    /// Rate-limit tracker: key → Vec<Instant> (Phase 7.4).
    /// Наряд №263: the key is the connection peer address by default (see
    /// `extract_client_ip`); bounded by MAX_RATE_KEYS — a new key at the cap
    /// counts as a full bucket (429), never silent unbounded growth.
    pub rate_limits: Arc<DashMap<String, Vec<std::time::Instant>>>,
    /// Наряд №263: requests per client per minute (mlogserver `rate_limit: N`,
    /// default 100 — DEFAULT_RATE_LIMIT_PER_MINUTE).
    pub rate_limit_per_minute: usize,
    /// Наряд №263: parsed METALOGOS_TRUSTED_PROXIES. Empty = XFF/X-Real-IP are
    /// never honored (the peer address is the rate-limit key).
    pub trusted_proxies: Arc<TrustedProxies>,
    /// Which backend to use for route execution (Наряд №40).
    pub backend: ServeBackend,
    /// Compiled VM program (Наряд №40: compiled once at startup, reused per request).
    pub vm_program: Option<Arc<Program>>,
    /// Compiled route bytecodes (Наряд №40: one per route, compiled at startup).
    pub vm_routes: Vec<CompiledRoute>,
}

#[derive(Debug, Clone)]
pub struct SessionEntry {
    pub data: HashMap<String, String>,
    pub roles: Vec<String>,
    pub expires: std::time::Instant,
}

// ── Наряд №263: rate-limit keying, trusted proxies, bounded maps ──

/// Default requests-per-client-per-minute when the mlogserver declaration does
/// not set `rate_limit: N` (Наряд №263: the pre-№263 hard-wired 100, unchanged).
pub(crate) const DEFAULT_RATE_LIMIT_PER_MINUTE: usize = 100;

/// Наряд №263 — hard caps for the hot-path state maps (pre-№263 all three grew
/// without bounds under a flood of unique keys: a cheap HTTP garbage stream of
/// fresh XFF values / peers / tokens → unbounded process memory).
/// Constant choices (deliberate, documented in REFERENCE §5.6):
/// - MAX_SESSIONS = 10 000: `sessions` is a read-side cache in front of SQLite;
///   each entry is a small role list + data map, 10k entries stay in the low
///   MiB range, while a typical legitimate deployment holds far fewer.
/// - MAX_CSRF_TOKENS = 10 000: tokens carry a 15-minute TTL (№29 §2.2) and one
///   is issued per GET under the csrf middleware — 10k covers 10k concurrent
///   browser sessions per sweep interval, far above honest traffic.
/// - MAX_RATE_KEYS = 65 536: one bucket per distinct key (peer or XFF entry);
///   2^16 bounds the IPv6-realistic worst case without evicting honest clients.
pub(crate) const MAX_SESSIONS: usize = 10_000;
pub(crate) const MAX_CSRF_TOKENS: usize = 10_000;
pub(crate) const MAX_RATE_KEYS: usize = 65_536;

/// Rate-limit window in seconds (the sliding window used by `check_rate_limit`;
/// the background sweep evicts keys whose every timestamp fell out of it).
pub(crate) const RATE_WINDOW_SECS: u64 = 60;

/// One entry of `METALOGOS_TRUSTED_PROXIES`: an exact IP or a CIDR `/NN` prefix
/// (both address families; no new crates — manual mask math, лекало
/// `is_blocked_address` №261 which hand-rolls ranges for the same reason).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TrustedProxyEntry {
    Exact(IpAddr),
    Cidr(IpAddr, u8),
}

/// Parsed `METALOGOS_TRUSTED_PROXIES` (Наряд №263).
///
/// CHOSEN SEMANTICS (documented loudly in REFERENCE §5.6 and CHANGELOG):
/// 1. Empty list (env unset) — `X-Forwarded-For` / `X-Real-IP` are NEVER
///    honored; the direct connection peer is the rate-limit key. This closes
///    the pre-№263 bypass: any client could send a fresh XFF per request and
///    make the rate limit a no-op.
/// 2. Non-empty list — headers are honored ONLY when the direct peer matches
///    an entry; then the key is the FIRST (leftmost) `X-Forwarded-For` value
///    (else `X-Real-IP`, else the peer). OPERATOR CONTRACT: a trusted proxy
///    must OVERWRITE XFF with the client address it sees; with an append-style
///    proxy the leftmost entry is client-controlled (documented residual).
#[derive(Debug, Clone, Default)]
pub struct TrustedProxies {
    entries: Vec<TrustedProxyEntry>,
}

impl TrustedProxies {
    /// Parse a comma-separated spec of exact IPs and CIDR `/NN` prefixes.
    /// Invalid entries are skipped with a loud warning (fail-open to fewer
    /// trusted proxies is safer than fail-closed to no server at startup).
    pub(crate) fn from_env_spec(spec: Option<&str>) -> Self {
        let mut entries = Vec::new();
        let Some(spec) = spec.map(str::trim).filter(|s| !s.is_empty()) else {
            return Self { entries };
        };
        for part in spec.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            match parse_trusted_proxy_entry(part) {
                Some(e) => entries.push(e),
                None => eprintln!(
                    "[WARN] METALOGOS_TRUSTED_PROXIES: skipping invalid entry {:?} (expected an IP or IP/prefix)",
                    part
                ),
            }
        }
        Self { entries }
    }

    /// True when at least one valid entry is configured (headers may be honored).
    pub(crate) fn is_configured(&self) -> bool {
        !self.entries.is_empty()
    }

    /// Does `ip` match an entry? IPv4-mapped IPv6 peers are unwrapped first
    /// (лекало `is_blocked_address` №261: `::ffff:10.0.0.1` must match 10.x).
    pub(crate) fn contains(&self, ip: IpAddr) -> bool {
        let ip = unwrap_mapped(ip);
        self.entries.iter().any(|e| ip_matches_entry(ip, e))
    }
}

fn parse_trusted_proxy_entry(part: &str) -> Option<TrustedProxyEntry> {
    if let Some((addr_str, prefix_str)) = part.split_once('/') {
        let addr: IpAddr = addr_str.trim().parse().ok()?;
        let prefix: u8 = prefix_str.trim().parse().ok()?;
        let max = match addr {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        if prefix > max {
            return None;
        }
        Some(TrustedProxyEntry::Cidr(addr, prefix))
    } else {
        Some(TrustedProxyEntry::Exact(part.parse().ok()?))
    }
}

/// Unwrap an IPv4-mapped IPv6 address to its V4 form (№261 лекало).
fn unwrap_mapped(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(ip),
        v4 => v4,
    }
}

fn ip_matches_entry(ip: IpAddr, entry: &TrustedProxyEntry) -> bool {
    match entry {
        TrustedProxyEntry::Exact(e) => unwrap_mapped(*e) == ip,
        TrustedProxyEntry::Cidr(net, prefix) => ip_in_cidr(ip, unwrap_mapped(*net), *prefix),
    }
}

/// CIDR membership by manual masking (std ships no helpers; №263 carries the
/// same no-new-deps decision as №261's octet-range checks).
/// Mixed families never match — unwrap_mapped runs before this.
fn ip_in_cidr(ip: IpAddr, net: IpAddr, prefix: u8) -> bool {
    match (ip, net) {
        (IpAddr::V4(a), IpAddr::V4(n)) => {
            if prefix > 32 {
                return false;
            }
            let mask = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix) };
            u32::from(a) & mask == u32::from(n) & mask
        }
        (IpAddr::V6(a), IpAddr::V6(n)) => {
            if prefix > 128 {
                return false;
            }
            let mask = if prefix == 0 {
                0
            } else {
                u128::MAX.checked_shl(128 - prefix as u32).unwrap_or(0)
            };
            u128::from(a) & mask == u128::from(n) & mask
        }
        _ => false,
    }
}

/// Which backend to use for route execution (Наряд №40).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServeBackend {
    /// Tree-walking interpreter (default).
    Interpreter,
    /// Stack-based bytecode VM.
    Vm,
}

// ── Public API ─────────────────────────────────────────────────────

/// Parse source, build Axum router, start server on configured port.
/// This is the entry point for `mlog serve <file>`.
pub async fn run_server(source: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let declarations = crate::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;

    // Наряд №98: enforce Category A security invariants before serving.
    // SQL_DYNAMIC, SECRET_LEAK, HTML_INJECTION are now compile-time errors.
    // Call audit_category_a directly (not check_program) — the interpreter
    // resolves imports at runtime; check_program would false-positive.
    let cat_a = crate::audit::audit_category_a(&declarations, "");
    let cat_a_errors: Vec<String> = cat_a
        .iter()
        .filter_map(|f| match f.severity {
            crate::audit::Severity::Error | crate::audit::Severity::Warning => {
                Some(format!("[{}] {}", f.check_id, f.message))
            }
            crate::audit::Severity::Info => None,
        })
        .collect();
    if !cat_a_errors.is_empty() {
        return Err(format!(
            "Category A security invariant violated:\n{}",
            cat_a_errors.join("\n")
        )
        .into());
    }

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
    // Наряд №164: default bind to 127.0.0.1 (loopback only) — never expose the
    // server to all network interfaces unless the user explicitly opts in.
    // Mirrors the SSRF/exec opt-in discipline established in наряд №143.
    let host = config
        .host
        .clone()
        .unwrap_or_else(|| "127.0.0.1".to_string());
    if host == "0.0.0.0" || host == "::" {
        eprintln!(
            "[WARN] Server binds to {} — reachable from all network interfaces. \
             Set host: \"127.0.0.1\" in mlogserver for local-only access.",
            host
        );
    }
    let mut state = build_state(config.clone(), interp).await?;

    // ── Наряд №263: loud startup surface for the new security knobs ──
    eprintln!(
        "[server] rate limit: {} req/min per client (mlogserver rate_limit field, default {})",
        state.rate_limit_per_minute, DEFAULT_RATE_LIMIT_PER_MINUTE
    );
    if state.trusted_proxies.is_configured() {
        eprintln!(
            "[server] METALOGOS_TRUSTED_PROXIES set — X-Forwarded-For/X-Real-IP honored ONLY for direct peers in the list; \
             leftmost XFF entry wins. OPERATOR CONTRACT: the proxy must OVERWRITE XFF (REFERENCE §5.6, naryad #263)"
        );
    } else {
        eprintln!(
            "[server] METALOGOS_TRUSTED_PROXIES unset — the connection peer address is the rate-limit key; \
             XFF/X-Real-IP headers are ignored (naryad #263)"
        );
    }

    // ── Наряд №40: Read METALOGOS_SERVE_BACKEND once at startup ──
    let backend = match std::env::var("METALOGOS_SERVE_BACKEND") {
        Ok(ref val) if val == "vm" => {
            // Наряд №109 / ADR-0105: explicit opt-in — surface known gaps
            eprintln!(
                "[WARN] METALOGOS_SERVE_BACKEND=vm — experimental, known                  limitations: `match` statements fail to compile, block                  if/else silently evaluates to Unit. See ADR-0105.                  Default (tree-walking) does not have these limitations."
            );
            eprintln!("[server] backend: vm (bytecode VM)");
            ServeBackend::Vm
        }
        Ok(ref val) if val == "interpreter" => {
            eprintln!("[server] backend: interpreter (tree-walking)");
            ServeBackend::Interpreter
        }
        Ok(val) => {
            eprintln!(
                "[WARN] METALOGOS_SERVE_BACKEND='{}' is unknown, falling back to interpreter",
                val
            );
            ServeBackend::Interpreter
        }
        Err(_) => {
            eprintln!("[server] backend: interpreter (default)");
            ServeBackend::Interpreter
        }
    };
    state.backend = backend;

    // ── Наряд №40: Compile routes for VM at startup (not per-request) ──
    if state.backend == ServeBackend::Vm {
        let start = std::time::Instant::now();
        let mut compiler = Compiler::new();
        let program = compiler
            .compile(declarations.clone())
            .map_err(|e| format!("VM compile error: {}", e))?;
        let compiled_routes = compiler
            .compile_routes(&config.routes)
            .map_err(|e| format!("VM route compile error: {}", e))?;
        let elapsed = start.elapsed();
        eprintln!(
            "[server] VM compilation: {} routes in {:.1} µs ({} instructions total)",
            compiled_routes.len(),
            elapsed.as_micros(),
            compiled_routes.iter().map(|r| r.code.len()).sum::<usize>()
        );

        // Build VM template with program data (patterns, learnables, etc.)
        // Note: Vm is !Send, so we cannot store it in ServerState (Arc<Vm>).
        // Instead, we store Arc<Program> and create a fresh Vm per request.
        // Vm::run() is cheap (just initializes globals/patterns from program).

        state.vm_program = Some(Arc::new(program));
        state.vm_routes = compiled_routes;
    }

    let app = build_router(state.clone());

    // v0.8.2 — Background reminder scheduler (checks every 5 seconds)
    // v0.8.3 — Extended: also checks cron jobs from OpenHuman-inspired cron_add
    let scheduler_state = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            // ── Phase 1: collect reminder + cron data under short write lock ──
            let (check_result, cron_check) = {
                let interp = scheduler_state.interpreter.write().await;
                let cr = {
                    if let Some(builtin_fn) = interp.get_builtin("check_reminders") {
                        builtin_fn(&[])
                    } else {
                        Ok(crate::interpreter::Value::List(vec![]))
                    }
                };
                let cc = {
                    if let Some(builtin_fn) = interp.get_builtin("cron_list") {
                        builtin_fn(&[])
                    } else {
                        Ok(crate::interpreter::Value::List(vec![]))
                    }
                };
                (cr, cc)
                // write lock released here
            };

            // ── Phase 2: process reminders (no lock needed) ──
            if let Ok(crate::interpreter::Value::List(items)) = check_result {
                for item in &items {
                    if let crate::interpreter::Value::Struct { fields, .. } = item {
                        let msg = fields
                            .get("message")
                            .map(|v| format!("{}", v))
                            .unwrap_or_default();
                        let rtype = fields
                            .get("type")
                            .map(|v| format!("{}", v))
                            .unwrap_or_default();
                        eprintln!(
                            "[scheduler] due {}: [{}] {}",
                            rtype,
                            msg,
                            fields
                                .get("data")
                                .map(|v| format!("{}", v))
                                .unwrap_or_default()
                        );
                    }
                }
            }

            // ── Phase 3: dispatch cron jobs (per-job write lock) ──
            if let Ok(crate::interpreter::Value::List(jobs)) = cron_check {
                for job in &jobs {
                    if let crate::interpreter::Value::Struct { fields, .. } = job {
                        let job_id = fields
                            .get("id")
                            .map(|v| format!("{}", v))
                            .unwrap_or_default();
                        let cron_expr = fields
                            .get("cron_expr")
                            .map(|v| format!("{}", v))
                            .unwrap_or_default();
                        let enabled = fields.get("enabled").map(|v| format!("{}", v))
                            == Some("1".to_string());
                        let force_run = fields.get("force_run").map(|v| format!("{}", v))
                            == Some("1".to_string());
                        let prompt = fields
                            .get("prompt")
                            .map(|v| format!("{}", v))
                            .unwrap_or_default();

                        if !enabled {
                            continue;
                        }

                        let should_fire = force_run || cron_expr_matches(&cron_expr);
                        if !should_fire {
                            continue;
                        }

                        // Short write lock: fire + mark in one hold
                        {
                            let interp = scheduler_state.interpreter.write().await;
                            eprintln!("[cron] firing: {} — {}", cron_expr, prompt);
                            if let Some(builtin_fn) = interp.get_builtin(&prompt) {
                                if let Err(e) = builtin_fn(&[]) {
                                    eprintln!("[cron] builtin '{}' error: {}", prompt, e);
                                }
                            } else if let Err(e) = interp.call_pattern(&prompt, &[]) {
                                eprintln!("[cron] pattern '{}' error: {}", prompt, e);
                            }
                            if let Some(mark_fn) = interp.get_builtin("cron_mark_fired") {
                                if let Err(e) =
                                    mark_fn(&[crate::interpreter::Value::String(job_id)])
                                {
                                    eprintln!("[cron] mark_fired error: {}", e);
                                }
                            }
                            // write lock released here
                        }
                    }
                }
            }
        }
    });

    // Наряд №29 §2.2 — Background CSRF token cleanup task (every 60s).
    // Evicts tokens older than 15 minutes from the in-memory store.
    // Наряд №263 — the same pass now also sweeps the two maps that had NO
    // cleanup at all: `rate_limits` (timestamps outside the 60-second window;
    // keys whose every timestamp fell out are removed) and `sessions`
    // (entries past their own `SessionEntry.expires` TTL — the contract
    // check_roles already enforces per-request; SQLite rows keep their own
    // cleanup path in clean_expired_sessions_db, untouched).
    let csrf_state = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            let (csrf, rate_keys, sessions) = sweep_expired_state(&csrf_state);
            if csrf > 0 {
                eprintln!("[csrf-cleanup] evicted {} expired token(s)", csrf);
            }
            if rate_keys > 0 {
                eprintln!("[rate-cleanup] evicted {} stale key bucket(s)", rate_keys);
            }
            if sessions > 0 {
                eprintln!("[session-cleanup] evicted {} expired session cache entr(ies)", sessions);
            }
        }
    });

    println!("mlog serve: listening on {}:{}", host, port);
    println!("mlog serve: scheduler active (5s interval — reminders + cron)");
    println!("mlog serve: CSRF token cleanup active (60s interval, 15-min TTL)");
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port)).await?;
    // Наряд №263: ConnectInfo is forwarded so the rate-limit key defaults to the
    // REAL connection peer address instead of client-controlled headers.
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;
    Ok(())
}

/// Start server on a random port for integration testing.
/// Returns (port, join_handle).
pub async fn run_test_server(
    source: &str,
) -> Result<
    (
        u16,
        tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    ),
    Box<dyn std::error::Error + Send + Sync>,
> {
    let declarations = crate::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;

    let server_config = declarations
        .iter()
        .find_map(|d| match d {
            Declaration::MlogServer(s) => Some(s.clone()),
            _ => None,
        })
        .ok_or("no mlogserver block")?;

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

    let state = build_state(config.clone(), interp).await?;
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    let handle = tokio::spawn(async move {
        // Наряд №263: ConnectInfo forwarded (same contract as run_server).
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });

    Ok((port, handle))
}

/// НАРЯД #207: Test server with explicit backend AND base_dir.
/// `base_dir` управляет ОБОИМИ путями резолва импортов:
///  - TW: `Interpreter::set_base_dir` (module loading, src/interpreter/modules.rs)
///  - VM: `Compiler::with_std_root` (import resolution, src/compiler.rs resolve_import)
pub async fn run_test_server_with_backend_in_dir(
    source: &str,
    backend: ServeBackend,
    base_dir: std::path::PathBuf,
) -> Result<
    (
        u16,
        tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    ),
    Box<dyn std::error::Error + Send + Sync>,
> {
    let declarations = crate::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;

    let server_config = declarations
        .iter()
        .find_map(|d| match d {
            Declaration::MlogServer(s) => Some(s.clone()),
            _ => None,
        })
        .ok_or("no mlogserver block")?;

    let mut interp = Interpreter::new();
    for decl in declarations.clone() {
        if !matches!(decl, Declaration::Flow(_)) {
            let mut tmp = Interpreter::new();
            // НАРЯД #207: use caller-supplied base_dir (not hardcoded ".")
            tmp.set_base_dir(base_dir.clone());
            let _ = tmp.run(vec![decl]);
            interp = merge_interpreter(tmp, interp);
        }
    }

    // Override port to 0 (OS-assigned)
    let mut config = server_config.clone();
    config.port = 0;

    let mut state = build_state(config.clone(), interp).await?;
    state.backend = backend;

    // НАРЯД #160: Compile routes for VM backend (same as run_server does)
    if state.backend == ServeBackend::Vm {
        // НАРЯД #207: use caller-supplied base_dir as std_root (not Compiler::new())
        let mut compiler = Compiler::with_std_root(base_dir.clone());
        let program = compiler
            .compile(declarations)
            .map_err(|e| format!("VM compile error: {}", e))?;
        let compiled_routes = compiler
            .compile_routes(&server_config.routes)
            .map_err(|e| format!("VM route compile error: {}", e))?;
        state.vm_program = Some(Arc::new(program));
        state.vm_routes = compiled_routes;
    }

    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    let handle = tokio::spawn(async move {
        // Наряд №263: ConnectInfo forwarded (same contract as run_server).
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });

    Ok((port, handle))
}

/// НАРЯД #207: Backward-compatible wrapper — прежнее поведение (CWD как base_dir).
/// `current_dir()` совпадает с семантикой `Compiler::new()` (src/compiler.rs:70),
/// поэтому ~40 существующих вызовов не меняют поведения.
pub async fn run_test_server_with_backend(
    source: &str,
    backend: ServeBackend,
) -> Result<
    (
        u16,
        tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    ),
    Box<dyn std::error::Error + Send + Sync>,
> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    run_test_server_with_backend_in_dir(source, backend, cwd).await
}

// ── Internal: Build State ──────────────────────────────────────────

pub(crate) async fn build_state(
    config: MlogServerDecl,
    interp: Interpreter,
) -> Result<ServerState, Box<dyn std::error::Error + Send + Sync>> {
    // Наряд №29 §2.1: HMAC key from env (METALOGOS_HMAC_KEY) or random fallback.
    // Never panics — random fallback logs WARNING and continues.
    let hmac_key = load_hmac_key();

    // Collect templates from interpreter
    let templates_map = interp.get_templates().clone();

    // Наряд №29 §2.3: SQLite init returns Result instead of panicking.
    let conn = rusqlite::Connection::open_in_memory()
        .map_err(|e| format!("Failed to open SQLite in-memory database: {}", e))?;
    init_session_db(&conn).map_err(|e| format!("Failed to create sessions table: {}", e))?;

    Ok(ServerState {
        sessions: Arc::new(DashMap::new()),
        csrf_tokens: Arc::new(DashMap::new()),
        hmac_key: Arc::new(hmac_key),
        audit_log: Arc::new(RwLock::new(Vec::new())),
        templates: Arc::new(RwLock::new(templates_map)),
        db_store: Arc::new(RwLock::new(Vec::new())),
        memory_persist: interp.get_memory_persist_path(),
        interpreter: Arc::new(RwLock::new(interp)),
        routes: config.routes.clone(),
        middleware: config.middleware.clone(),
        db: Arc::new(tokio::sync::Mutex::new(conn)),
        rate_limits: Arc::new(DashMap::new()),
        // Наряд №263: mlogserver `rate_limit: N`, default 100 (unchanged).
        rate_limit_per_minute: config
            .rate_limit
            .map(|n| n as usize)
            .unwrap_or(DEFAULT_RATE_LIMIT_PER_MINUTE),
        // Наряд №263: parsed once at startup (same read-once discipline as
        // METALOGOS_SERVE_BACKEND) so request handling never re-reads env.
        trusted_proxies: Arc::new(TrustedProxies::from_env_spec(
            std::env::var("METALOGOS_TRUSTED_PROXIES").ok().as_deref(),
        )),
        backend: ServeBackend::Interpreter, // set after build_state returns
        vm_program: None,
        vm_routes: Vec::new(),
    })
}

/// Наряд №255: максимальный размер тела запроса (байты) — 2 МиБ.
///
/// Осознанная константа вместо неявного дефолта axum 0.8 (~2 МБ):
/// до №255 источник истины о лимите находился в чужом крейте и молча
/// менялся бы с апгрейдом. Обоснование величины: 2 МиБ хватает для
/// JSON-тел роутов (конфиги, документы, payload'ы LLM-запросов);
/// загрузки большего размера — отдельное решение (streaming/multipart),
/// а не молчаливый рост лимита. Меняется только здесь; docs/threat-model.md
/// и REFERENCE.md называют то же число; тест `n255_body_limit` пинит
/// поведение N±1 (413 на превышение).
pub(crate) const REQUEST_BODY_LIMIT_BYTES: usize = 2 * 1024 * 1024; // 2 MiB

fn build_router(state: ServerState) -> Router {
    let mut app = Router::new();

    // Наряд №255: осознанный лимит тела запроса, зафиксирован явно.
    //
    // До №255 поведение держалось на неявном дефолте axum 0.8 (~2 МБ) —
    // источник истины о лимите находился в чужом крейте и молча изменился
    // бы с апгрейдом. 2 МиБ достаточно для JSON-тел роутов (конфиги,
    // документы, payload'ы LLM-запросов); загрузки большего размера —
    // отдельное осознанное решение (streaming/multipart), а не молчаливый
    // рост лимита вместе с зависимостью. Число задокументировано в
    // docs/threat-model.md и REFERENCE.md — меняется вместе с этой
    // константой (тест n255 пинит соответствие N±1).
    app = app.layer(DefaultBodyLimit::max(REQUEST_BODY_LIMIT_BYTES));

    // Add security headers layer (always applied)
    app = app.layer(SetResponseHeaderLayer::if_not_present(
        header::X_FRAME_OPTIONS,
        HeaderValue::from_static("DENY"),
    ));
    app = app.layer(SetResponseHeaderLayer::if_not_present(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    ));
    app = app.layer(SetResponseHeaderLayer::if_not_present(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    ));
    app = app.layer(SetResponseHeaderLayer::if_not_present(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'",
        ),
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

    // Наряд №250 (ADR-0122 #208 family — n206 VM-serve verification debt):
    // wire the DESIGNED 404 body into the router. The manual dispatch path
    // already returns ("404 Not Found") (the `else` branch below in this
    // file), but the axum router never received a fallback, so unknown paths
    // returned axum's default EMPTY-body 404 on BOTH backends (repro:
    // naryad_160 block2_vm_404_unknown_route — status 404, body ""). The
    // fallback is backend-agnostic (shared router: Interpreter AND VM), so
    // TW/VM parity is preserved (block4_tw_vs_vm_404 stays green).
    app = app.fallback(|| async { (StatusCode::NOT_FOUND, "404 Not Found").into_response() });

    app.with_state(state)
}

// ── Route Handler ──────────────────────────────────────────────────

async fn route_handler(
    State(state): State<ServerState>,
    // Наряд №263: the real connection peer (via into_make_service_with_connect_info
    // at every serve point) — the default rate-limit key, immune to header spoofing.
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    uri: Uri,
    method: Method,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> Response {
    // 0. Extract client IP for rate limiting (Наряд №263: peer-first semantics —
    //    XFF/X-Real-IP honored ONLY for direct peers in METALOGOS_TRUSTED_PROXIES).
    let client_ip = extract_client_ip(&headers, Some(peer.ip()), &state.trusted_proxies);

    // 0b. Bug 2.1 fix: parse query string from URI
    let query: std::collections::HashMap<String, String> = uri
        .query()
        .map(|q| {
            q.split('&')
                .filter_map(|pair| {
                    if pair.is_empty() {
                        return None;
                    }
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

    // 1. Rate limiting (Phase 7.4; limit configurable since Наряд №263)
    if state.middleware.contains(&"rate_limit".to_string()) {
        if let Err(resp) = check_rate_limit(&state, &client_ip, state.rate_limit_per_minute).await {
            return resp;
        }
    }

    // 2. CSRF check for mutating methods (Phase 7.4: real double-submit)
    if matches!(method, Method::POST | Method::PUT | Method::DELETE)
        && state.middleware.contains(&"csrf".to_string())
    {
        if let Err(resp) = check_csrf(&state, &headers).await {
            return resp;
        }
    }

    // 3. Session expiry check (Phase 7.4: SQLite-backed)
    // Наряд №29 §2.2: capture raw session_id for later CSRF token binding.
    let mut raw_session_id: Option<String> = None;
    if state.middleware.contains(&"session".to_string()) {
        if let Some(session_id) = extract_session_cookie(&headers) {
            // Verify HMAC signature first
            let verified = verify_cookie(&session_id, &state.hmac_key);
            if let Some(raw_id) = verified {
                if let Err(resp) = validate_session_in_db(&state, &raw_id).await {
                    return resp;
                }
                raw_session_id = Some(raw_id);
            }
        }
    }

    // 4. Find matching route by path AND method
    let matched_route = state
        .routes
        .iter()
        .find(|r| r.path == uri.path() && r.method == method.as_str());

    if let Some(route) = matched_route {
        // Role check
        if !route.requires.is_empty() && state.middleware.contains(&"session".to_string()) {
            if let Err(resp) = check_roles(&state, &headers, &route.requires).await {
                return resp;
            }
        }

        // ── Наряд №40: Dispatch to VM or interpreter based on backend ──
        let result = if state.backend == ServeBackend::Vm {
            execute_route_body_vm(&state, route, &headers, &body, &query).await
        } else {
            execute_route_body(&state, &route.body, &headers, &body, &query).await
        };
        let mut response = match result {
            Ok(response) => response,
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Handler error: {}", e),
            )
                .into_response(),
        };

        // 5. On GET with CSRF middleware, generate and set CSRF token cookie (Phase 7.4)
        // Наряд №29 §2.2: store (session_id, created_at) for TTL enforcement.
        // Наряд №262: the session_id half is now ENFORCED at validation — the token is
        // bound to the HMAC-verified session of the issuing request ("" = sessionless).
        // Наряд №263: issuance is BOUNDED — at the store cap the token is refused
        // LOUDLY with 503 instead of silent unbounded growth.
        if method == Method::GET && state.middleware.contains(&"csrf".to_string()) {
            let token = generate_csrf_token();
            let session_id_for_csrf = raw_session_id.clone().unwrap_or_default();
            if let Err(resp) =
                issue_csrf_token_capped(&state, &token, &session_id_for_csrf).await
            {
                return resp;
            }
            // Наряд №125: NO HttpOnly — JS must read this cookie for double-submit.
            let cookie_value = format!("_mlog_csrf={}; SameSite=Strict; Path=/", token);
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
    // Наряд №173: rand 0.10 — `thread_rng()` → `rng()`,
    // `fill(&mut [u8])` → `fill_bytes(&mut [u8])` (renamed in `Rng`).
    use rand::Rng;
    let mut buf = [0u8; 16];
    rand::rng().fill_bytes(&mut buf);
    hex::encode(buf)
}

#[allow(clippy::result_large_err)] // Response as Err is intentional for axum handlers
async fn check_csrf(state: &ServerState, headers: &HeaderMap) -> Result<(), Response> {
    // Read CSRF token from cookie
    let cookie_token = headers
        .get("cookie")
        .and_then(|c| c.to_str().ok())
        .and_then(|s| extract_cookie(s, "_mlog_csrf"));

    // Read CSRF token from header (X-CSRF-Token) or form field (_csrf)
    let header_token = headers
        .get("x-csrf-token")
        .and_then(|t| t.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| {
            // Also check content-type for form data with _csrf field
            headers
                .get("x-csrf-field")
                .and_then(|t| t.to_str().ok())
                .map(|s| s.to_string())
        });

    match (cookie_token, header_token) {
        (Some(cookie), Some(header)) if cookie == header => {
            // Наряд №262: STRICT server-issued validation. The former stateless
            // fallback ("token absent from the store → accept", the classic naive
            // double-submit bypass: plant any cookie + send any matching header)
            // is REMOVED — the token MUST have been issued by this process
            // (present in `csrf_tokens`, route_handler step 5). A server restart
            // honestly invalidates outstanding tokens: 403, page reloads, fresh token.
            let Some(entry) = state.csrf_tokens.get(&cookie) else {
                let mut log = state.audit_log.write().await;
                log.push(
                    "[CSRF] Rejected: token not issued by this server (stateless fallback removed, naryad #262)"
                        .to_string(),
                );
                return Err((
                    StatusCode::FORBIDDEN,
                    "403 Forbidden: CSRF token validation failed",
                )
                    .into_response());
            };

            // Наряд №29 §2.2: enforce 15-minute TTL on server-issued tokens
            // (unchanged by №262).
            let now = std::time::Instant::now();
            let ttl = std::time::Duration::from_secs(900); // 15 minutes
            let (bound_session, created_at) = entry.value().clone();
            drop(entry); // release the shard lock before the possible remove() below

            if now.duration_since(created_at) >= ttl {
                state.csrf_tokens.remove(&cookie);
                let mut log = state.audit_log.write().await;
                log.push("[CSRF] Rejected: token expired (>15 min)".to_string());
                return Err(
                    (StatusCode::FORBIDDEN, "403 Forbidden: CSRF token expired").into_response()
                );
            }

            // Наряд №262: session binding — the dead half of the (session_id, Instant)
            // tuple is now enforced. The request identity is computed exactly as at
            // issuance (route_handler step 3): the HMAC-verified raw session id from
            // the _mlog_session cookie; None when the session middleware is off, the
            // cookie is absent, or the signature fails. Liveness (expiry/DB) is NOT
            // re-checked here — that stays with the session middleware step that runs
            // after this one: binding proves WHO the token belongs to, not whether
            // the session is alive. A token issued without a session (bound to "")
            // is only valid for sessionless requests.
            let request_session = if state.middleware.contains(&"session".to_string()) {
                extract_session_cookie(headers).and_then(|c| verify_cookie(&c, &state.hmac_key))
            } else {
                None
            };
            let session_ok = match (bound_session.is_empty(), request_session.as_deref()) {
                (true, None) => true,     // sessionless token, sessionless request
                (true, Some(_)) => false, // sessionless token replayed with a session
                (false, Some(s)) => s == bound_session, // must present its own session
                (false, None) => false,   // bound token cannot prove ownership
            };
            if !session_ok {
                let mut log = state.audit_log.write().await;
                log.push(format!(
                    "[CSRF] Rejected: session binding mismatch (bound {:?}, request {:?})",
                    bound_session, request_session
                ));
                return Err((
                    StatusCode::FORBIDDEN,
                    "403 Forbidden: CSRF session binding mismatch",
                )
                    .into_response());
            }

            Ok(())
        }
        _ => {
            // Log to audit
            {
                let mut log = state.audit_log.write().await;
                log.push("[CSRF] Rejected: missing or mismatched CSRF token".to_string());
            }
            Err((
                StatusCode::FORBIDDEN,
                "403 Forbidden: CSRF token validation failed",
            )
                .into_response())
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
    headers
        .get("cookie")
        .and_then(|c| c.to_str().ok())
        .and_then(|s| extract_cookie(s, "_mlog_session"))
}

/// Resolve the client identity used as the rate-limit key (Наряд №263).
///
/// CHOSEN SEMANTICS (documented loudly in REFERENCE §5.6 and CHANGELOG):
/// 1. DEFAULT — the direct connection peer (`peer_ip`, from ConnectInfo) is the
///    key. `X-Forwarded-For` / `X-Real-IP` are IGNORED: pre-№263 the headers
///    were trusted unconditionally (`extract_client_ip` read XFF first, the
///    peer was never even wired in), so one client could put a fresh XFF value
///    on every request and always get a fresh bucket — the rate limit limited
///    only honest clients.
/// 2. `METALOGOS_TRUSTED_PROXIES` set AND the direct peer matches an entry —
///    the key is the FIRST (leftmost) `X-Forwarded-For` value (the identity the
///    nearest client presents), else `X-Real-IP`, else the peer itself.
///    OPERATOR CONTRACT: a trusted proxy must OVERWRITE XFF with the client
///    address it sees; behind an append-style proxy the leftmost entry is
///    client-controlled (documented residual — revisit: rightmost-untrusted
///    walk for multi-hop chains if a real deployment needs it).
/// 3. Peer NOT in the list — the peer address, headers never consulted.
/// `peer_ip: None` (only direct unit-test callers) → "unknown".
fn extract_client_ip(
    headers: &HeaderMap,
    peer_ip: Option<IpAddr>,
    trusted: &TrustedProxies,
) -> String {
    let peer_key = peer_ip
        .map(|i| i.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let headers_may_be_honored = peer_ip.map(|i| trusted.contains(i)).unwrap_or(false);
    if !headers_may_be_honored {
        return peer_key;
    }
    // Trusted peer: leftmost XFF entry, else X-Real-IP, else the peer.
    // Empty header values are treated as absent (a proxy sending "" must not
    // blank out the key).
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or(peer_key)
}

/// Наряд №263: the sanctioned writer of `state.sessions` — enforces MAX_SESSIONS.
///
/// Context (audit tail №263): the map is a read-side cache in front of SQLite
/// and had no cap and no cleanup; today no code path inserts into it (roles
/// come from DB lookups), so the helper is the LOUD, capped path any future
/// writer must use — and the anchor the cap test pins. Refusal text carries
/// 503 semantics (server busy), not a silent drop.
pub fn insert_session_capped(
    state: &ServerState,
    id: String,
    entry: SessionEntry,
) -> Result<(), String> {
    if !state.sessions.contains_key(&id) && state.sessions.len() >= MAX_SESSIONS {
        eprintln!(
            "[sessions] store at cap ({}) — refusing new session (naryad #263)",
            MAX_SESSIONS
        );
        return Err(
            "503 Service Unavailable: server busy — session store full, retry shortly"
                .to_string(),
        );
    }
    state.sessions.insert(id, entry);
    Ok(())
}

/// Наряд №263: the sanctioned issuer of CSRF tokens — enforces MAX_CSRF_TOKENS.
/// At the cap the issuance is refused LOUDLY (503 to the client + audit entry +
/// stderr metric) instead of the pre-№263 silent unbounded growth.
#[allow(clippy::result_large_err)] // Response as Err is intentional for axum handlers
async fn issue_csrf_token_capped(
    state: &ServerState,
    token: &str,
    session_id: &str,
) -> Result<(), Response> {
    if state.csrf_tokens.len() >= MAX_CSRF_TOKENS {
        {
            let mut log = state.audit_log.write().await;
            log.push(format!(
                "[CSRF] Rejected issuance: token store full ({} tokens) (naryad #263)",
                MAX_CSRF_TOKENS
            ));
        }
        eprintln!(
            "[csrf] token store at cap ({}) — refusing issuance (naryad #263)",
            MAX_CSRF_TOKENS
        );
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "503 Service Unavailable: server busy — CSRF token store full, retry shortly",
        )
            .into_response());
    }
    state.csrf_tokens.insert(
        token.to_string(),
        (session_id.to_string(), std::time::Instant::now()),
    );
    Ok(())
}

/// Наряд №263: one pass of the background sweep over ALL THREE hot-path maps.
/// Extends the №29 §2.2 csrf-only sweep (same 60-second cadence, one task).
/// Returns (csrf_evicted, rate_keys_evicted, sessions_evicted) for logging.
fn sweep_expired_state(state: &ServerState) -> (usize, usize, usize) {
    let now = std::time::Instant::now();

    // 1. csrf_tokens: TTL 15 minutes (№29 §2.2, unchanged).
    let csrf_ttl = std::time::Duration::from_secs(900);
    let before = state.csrf_tokens.len();
    state
        .csrf_tokens
        .retain(|_token, (_sid, created_at)| now.duration_since(*created_at) < csrf_ttl);
    let csrf_evicted = before - state.csrf_tokens.len();

    // 2. rate_limits: drop timestamps outside the 60-second sliding window;
    //    a key whose every timestamp fell out is removed entirely (pre-№263
    //    these outsider keys lived forever).
    let window = std::time::Duration::from_secs(RATE_WINDOW_SECS);
    let mut rate_evicted = 0usize;
    state.rate_limits.retain(|_key, stamps| {
        stamps.retain(|t| now.duration_since(*t) < window);
        if stamps.is_empty() {
            rate_evicted += 1;
            false
        } else {
            true
        }
    });

    // 3. sessions: TTL carried by SessionEntry.expires (the contract check_roles
    //    already enforces per-request: `entry.expires < now` = expired). No new
    //    constant — the entry's own TTL is the truth (revisit if a global
    //    session TTL policy ever lands).
    let before_sessions = state.sessions.len();
    state.sessions.retain(|_id, entry| entry.expires > now);
    let sessions_evicted = before_sessions - state.sessions.len();

    (csrf_evicted, rate_evicted, sessions_evicted)
}

// ── Rate Limiting (Phase 7.4) ─────────────────────────────────────

/// Check rate limit using sliding window. Returns Err(429) if exceeded.
#[allow(clippy::result_large_err)] // Response as Err is intentional for axum handlers
pub async fn check_rate_limit(
    state: &ServerState,
    ip: &str,
    max_per_minute: usize,
) -> Result<(), Response> {
    let now = std::time::Instant::now();
    let window_start = now - std::time::Duration::from_secs(60);

    // Наряд №263: bounded key store — a NEW key when the map is at cap counts
    // as a "full bucket" → 429 (loud audit entry + stderr metric). The
    // alternative (evict an arbitrary old key) would hand an attacker a
    // rotation primitive; silent growth was the pre-№263 memory-leak finding.
    // Existing keys keep working at the cap — the sweep reclaims stale ones.
    if !state.rate_limits.contains_key(ip) && state.rate_limits.len() >= MAX_RATE_KEYS {
        {
            let mut log = state.audit_log.write().await;
            log.push(format!(
                "[RATE_LIMIT] Rejected: key store full ({} keys) — new key treated as full bucket (naryad #263)",
                MAX_RATE_KEYS
            ));
        }
        eprintln!(
            "[rate-limit] key store at cap ({}) — refusing new key (naryad #263)",
            MAX_RATE_KEYS
        );
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "429 Too Many Requests: rate limit exceeded",
        )
            .into_response());
    }

    let mut entries = state.rate_limits.entry(ip.to_string()).or_default();
    // Remove entries outside the 60-second window
    entries.retain(|&t| t > window_start);

    if entries.len() >= max_per_minute {
        {
            let mut log = state.audit_log.write().await;
            log.push(format!(
                "[RATE_LIMIT] Rejected: {} exceeded {} req/min",
                ip, max_per_minute
            ));
        }
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "429 Too Many Requests: rate limit exceeded",
        )
            .into_response());
    }

    entries.push(now);
    Ok(())
}

// ── Session & Role Middleware ────────────────────────────────────────

#[allow(clippy::result_large_err)] // Response as Err is intentional for axum handlers
async fn check_roles(
    state: &ServerState,
    headers: &HeaderMap,
    required_roles: &[String],
) -> Result<(), Response> {
    let session_cookie = extract_session_cookie(headers);

    let raw_id = match session_cookie {
        Some(id) => {
            // Verify HMAC signature
            match verify_cookie(&id, &state.hmac_key) {
                Some(raw) => raw,
                None => {
                    let mut log = state.audit_log.write().await;
                    log.push("[AUTH] Rejected: tampered session cookie".to_string());
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        "401 Unauthorized: invalid session signature",
                    )
                        .into_response());
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
    let session_ref = state.sessions.get(&raw_id);
    if let Some(entry) = session_ref.as_deref() {
        if entry.expires < std::time::Instant::now() {
            return Err((
                StatusCode::UNAUTHORIZED,
                "401 Unauthorized: session expired",
            )
                .into_response());
        }
        let has_role = required_roles.iter().any(|role| entry.roles.contains(role));
        if has_role {
            Ok(())
        } else {
            drop(session_ref);
            let mut log = state.audit_log.write().await;
            log.push(format!(
                "[AUTH] Rejected: insufficient roles (need {:?}, have {:?})",
                required_roles,
                Vec::<String>::new()
            ));
            Err((
                StatusCode::FORBIDDEN,
                "403 Forbidden: insufficient permissions",
            )
                .into_response())
        }
    } else {
        // Fall through to SQLite check
        drop(session_ref);
        validate_session_in_db(state, &raw_id).await?;
        // If valid but not in memory cache, load from DB
        // For simplicity, reject here — session needs re-login
        Err((
            StatusCode::UNAUTHORIZED,
            "401 Unauthorized: session not found in cache",
        )
            .into_response())
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
        );",
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
#[allow(clippy::result_large_err)] // Response as Err is intentional for axum handlers
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
            Err((
                StatusCode::UNAUTHORIZED,
                "401 Unauthorized: session expired",
            )
                .into_response())
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("DB error: {}", e),
        )
            .into_response()),
    }
}

/// Delete a session from SQLite.
pub async fn delete_session_db(
    conn: &Arc<tokio::sync::Mutex<rusqlite::Connection>>,
    session_id: &str,
) -> Result<(), String> {
    let conn = conn.lock().await;
    conn.execute(
        "DELETE FROM sessions WHERE id = ?1",
        rusqlite::params![session_id],
    )
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
    let deleted = conn
        .execute(
            "DELETE FROM sessions WHERE expires_at <= ?1",
            rusqlite::params![now],
        )
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
        serde_json::Value::Array(arr) => Value::List(arr.iter().map(json_value_to_value).collect()),
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

pub(crate) async fn execute_route_body(
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
        interp.configure_memory(&MemoryDecl {
            span: Span::unknown(),
            persist: Some(persist_path.clone()),
        });
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
                if let Some(entry) = state.sessions.get(&raw_id) {
                    interp.set_server_user_roles(entry.value().roles.clone());
                }
            }
        }
    }

    // Execute body statements on a dedicated blocking thread.
    // This prevents nested tokio runtime panics when builtins like http_post()
    // use reqwest::blocking::Client (which internally creates its own tokio
    // runtime for DNS resolution / TLS). block_in_place() is NOT safe here
    // because dropping that inner runtime inside block_in_place panics with
    // "Cannot drop a runtime in a context where blocking is not allowed."
    // See ADR-0096.
    let body_stmts_owned: Vec<Statement> = body_stmts.to_vec();
    let outcome = tokio::task::spawn_blocking(
        move || -> Result<(Option<Response>, Vec<String>, String), String> {
            // Наряд №253 (Вариант А): тело роута исполняется в serve-роут-контексте —
            // exec()/exec_argv() здесь требуют METALOGOS_SERVE_ALLOW_EXEC=1
            // (процесс-флаг METALOGOS_ALLOW_EXEC на тела роутов не распространяется).
            let _serve_exec_guard = ServeRouteExecGuard::new();
            let mut env = HashMap::new();
            for stmt in &body_stmts_owned {
                match stmt {
                    Statement::LetBinding { name, value, .. } => {
                        let val = interp.eval_expr_with_env(value, &env)?;
                        env.insert(name.clone(), val);
                    }
                    Statement::Assign { name, value, .. } => {
                        let val = interp.eval_expr_with_env(value, &env)?;
                        if env.contains_key(name) {
                            env.insert(name.clone(), val);
                        }
                    }
                    Statement::Return { value: expr, .. } => {
                        let val = interp.eval_expr_with_env(expr, &env)?;
                        let entries = interp.take_audit_log();
                        let sandbox = interp
                            .get_active_sandbox()
                            .map(|sb| sb.name.clone())
                            .unwrap_or_default();
                        return Ok((Some(value_to_response(val)), entries, sandbox));
                    }
                    Statement::IfThen {
                        condition: cond,
                        body,
                        ..
                    } => {
                        let cond_val = interp.eval_expr_with_env(cond, &env)?;
                        if cond_val.as_bool().unwrap_or(false) {
                            // On a blocking thread, safe to call eval_statements directly
                            // (no block_in_place needed)
                            let result = interp.eval_statements(body, &mut env)?;
                            if !matches!(result, Value::Unit) {
                                let entries = interp.take_audit_log();
                                let sandbox = interp
                                    .get_active_sandbox()
                                    .map(|sb| sb.name.clone())
                                    .unwrap_or_default();
                                return Ok((Some(value_to_response(result)), entries, sandbox));
                            }
                        }
                    }
                    // Block-level if/else (Наряд №2 + final integration)
                    Statement::IfElseBlock {
                        condition,
                        then_body,
                        else_ifs,
                        else_body,
                        ..
                    } => {
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
                            matched.or(else_body.as_deref())
                        };
                        if let Some(stmts) = branch {
                            for s in stmts {
                                match s {
                                    Statement::Return { value: expr, .. } => {
                                        let val = interp.eval_expr_with_env(expr, &env)?;
                                        let entries = interp.take_audit_log();
                                        let sandbox = interp
                                            .get_active_sandbox()
                                            .map(|sb| sb.name.clone())
                                            .unwrap_or_default();
                                        return Ok((
                                            Some(value_to_response(val)),
                                            entries,
                                            sandbox,
                                        ));
                                    }
                                    Statement::LetBinding { name, value, .. } => {
                                        let val = interp.eval_expr_with_env(value, &env)?;
                                        env.insert(name.clone(), val);
                                    }
                                    Statement::ExprStmt { expr, .. } => {
                                        let val = interp.eval_expr_with_env(expr, &env)?;
                                        if let Value::HttpResponse { .. } = val {
                                            let entries = interp.take_audit_log();
                                            let sandbox = interp
                                                .get_active_sandbox()
                                                .map(|sb| sb.name.clone())
                                                .unwrap_or_default();
                                            return Ok((
                                                Some(value_to_response(val)),
                                                entries,
                                                sandbox,
                                            ));
                                        }
                                    }
                                    _ => {
                                        // On a blocking thread, safe to call directly
                                        interp
                                            .eval_statements(std::slice::from_ref(s), &mut env)?;
                                    }
                                }
                            }
                        }
                    }
                    // Bare expression statement — evaluate for side effects
                    Statement::ExprStmt { expr, .. } => {
                        let val = interp.eval_expr_with_env(expr, &env)?;
                        // If expression is respond("ok") or similar HttpResponse, use as route response
                        if let Value::HttpResponse { .. } = val {
                            let entries = interp.take_audit_log();
                            let sandbox = interp
                                .get_active_sandbox()
                                .map(|sb| sb.name.clone())
                                .unwrap_or_default();
                            return Ok((Some(value_to_response(val)), entries, sandbox));
                        }
                    }
                    _ => {
                        // On a blocking thread, safe to call directly
                        let result =
                            interp.eval_statements(std::slice::from_ref(stmt), &mut env)?;
                        // If the statement produced an HttpResponse (e.g., respond("ok")),
                        // use it as the route response (final integration)
                        if let Value::HttpResponse { .. } = result {
                            let entries = interp.take_audit_log();
                            let sandbox = interp
                                .get_active_sandbox()
                                .map(|sb| sb.name.clone())
                                .unwrap_or_default();
                            return Ok((Some(value_to_response(result)), entries, sandbox));
                        }
                    }
                }
            }
            // Normal completion — flush audit entries
            let entries = interp.take_audit_log();
            let sandbox = interp
                .get_active_sandbox()
                .map(|sb| sb.name.clone())
                .unwrap_or_default();
            Ok((None, entries, sandbox))
        },
    )
    .await
    .map_err(|e| format!("blocking task panicked: {}", e))??;

    // Phase 7.5: Flush interpreter audit entries to SQLite
    flush_audit_entries_to_db(state, &outcome.1, &outcome.2).await;

    if let Some(resp) = outcome.0 {
        Ok(resp)
    } else {
        Ok((StatusCode::OK, "OK").into_response())
    }
}

/// ── VM Route Execution (Наряд №40) ────────────────────────────────
///
/// VM equivalent of `execute_route_body`. Creates a fresh VM instance per
/// request (cloned from template), injects per-request server context,
/// executes compiled route bytecode, and returns the HTTP response.
///
/// **Isolation guarantee**: each request gets its own Vm with fresh stack.
/// Global state (kv_set, memory) is shared via builtins (Mutex-backed).
async fn execute_route_body_vm(
    state: &ServerState,
    route: &crate::ast::RouteDecl,
    headers: &HeaderMap,
    raw_body: &bytes::Bytes,
    query_params: &std::collections::HashMap<String, String>,
) -> Result<Response, String> {
    // Find the compiled route matching this path+method
    let compiled = state
        .vm_routes
        .iter()
        .find(|r| r.path == route.path && r.method == route.method)
        .ok_or_else(|| format!("VM: no compiled route for {} {}", route.method, route.path))?;

    if state.vm_program.is_none() {
        return Err("VM: no compiled program available".into());
    }

    // Execute compiled route bytecode on a dedicated blocking thread.
    // This prevents nested tokio runtime panics when builtins like http_post()
    // use reqwest::blocking::Client. See ADR-0096.
    // Extract user roles before entering blocking context (DashMap access).
    let user_roles = if state.middleware.contains(&"session".to_string()) {
        if let Some(session_id) = extract_session_cookie(headers) {
            if let Some(raw_id) = verify_cookie(&session_id, &state.hmac_key) {
                if let Some(entry) = state.sessions.get(&raw_id) {
                    entry.value().roles.clone()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Clone data needed inside spawn_blocking (closure must be 'static + Send)
    let program = match state.vm_program.as_ref() {
        Some(p) => p.clone(),
        None => return Err("VM: no program compiled".to_string()),
    };
    let compiled = compiled.clone();
    let raw_body = raw_body.clone();
    let query_params = query_params.clone();

    let (audit_entries, result) = tokio::task::spawn_blocking(move || {
        // Наряд №253 (Вариант А): VM-путь тела роута — тот же serve-роут-контекст,
        // exec()/exec_argv() требуют METALOGOS_SERVE_ALLOW_EXEC=1 (паритет с TW-путём).
        let _serve_exec_guard = ServeRouteExecGuard::new();
        let mut vm = Vm::new();
        vm.load_program(&program)
            .map_err(|e| format!("VM route init: {}", e))?;
        vm.clear_server_context();

        // Inject per-request server context
        if let Ok(body_str) = std::str::from_utf8(&raw_body) {
            if !body_str.is_empty() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(body_str) {
                    let value = json_value_to_value(&json);
                    vm.set_server_json_body(value);
                }
            }
        }
        if !query_params.is_empty() {
            vm.set_server_query_params(query_params.clone());
        }
        if !user_roles.is_empty() {
            vm.set_server_user_roles(user_roles);
        }

        let result = vm.execute_route_code(&compiled, &program);
        // Наряд №41 Block 2: collect audit entries before vm is dropped
        let entries = vm.take_audit_log();
        Result::<_, String>::Ok((entries, result))
    })
    .await
    .map_err(|e| format!("blocking task panicked: {}", e))??;

    // Наряд №41 Block 2: flush VM audit entries (parity with interpreter)
    flush_vm_audit_entries_to_db(state, &audit_entries).await;

    match result {
        Ok(val) => {
            // Check if the result is an HttpResponse (from respond())
            if let Value::HttpResponse { status, body } = val {
                let code = StatusCode::from_u16(status).unwrap_or(StatusCode::OK);
                return Ok((code, body).into_response());
            }
            // For other value types, convert like the interpreter does
            Ok(value_to_response(val))
        }
        Err(e) => Err(e),
    }
}

/// Flush audit entries to the SQLite audit_log table and in-memory log.
/// Shared implementation used by both interpreter and VM paths.
async fn flush_audit_entries_to_db(state: &ServerState, entries: &[String], sandbox: &str) {
    if entries.is_empty() {
        return;
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let conn = state.db.lock().await;
    for entry in entries {
        let (action, pattern, result) = parse_audit_entry(entry);
        let _ = conn.execute(
            "INSERT INTO audit_log (timestamp, action, pattern, result, sandbox) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![now, action, pattern, result, sandbox],
        );
    }
    // Also append to in-memory audit log for backward compatibility
    {
        let mut log = state.audit_log.write().await;
        for entry in entries {
            log.push(entry.clone());
        }
    }
}

/// Наряд №41 Block 2: Flush VM audit entries to the SQLite audit_log table.
/// VM parity with `flush_audit_to_db` — same DB writes, same in-memory log.
async fn flush_vm_audit_entries_to_db(state: &ServerState, entries: &[String]) {
    flush_audit_entries_to_db(state, entries, "").await;
}

/// Parse an audit entry string into (action, pattern, result) components.
/// Format: "[AUDIT] adapt PatternName: input -> output"
///         "[AUDIT] mutate PatternName: N examples, accuracy=X"
///         "[AUDIT] unsafe_html: rendered template 'name'"
fn parse_audit_entry(entry: &str) -> (String, Option<String>, Option<String>) {
    if let Some(rest) = entry.strip_prefix("[AUDIT] ") {
        let parts: Vec<&str> = rest.splitn(2, ' ').collect();
        let action = parts[0].to_string();
        let detail = if parts.len() > 1 {
            Some(parts[1].to_string())
        } else {
            None
        };

        match action.as_str() {
            "adapt" | "mutate" => {
                // Extract pattern name (first word of detail)
                let pattern = detail
                    .as_ref()
                    .and_then(|d| d.split(':').next())
                    .map(|s| s.trim().to_string());
                let result = detail
                    .as_ref()
                    .and_then(|d| d.split_once(':').map(|(_, s)| s.trim().to_string()));
                (action, pattern, result)
            }
            "unsafe_html" => {
                let pattern = detail
                    .as_ref()
                    .and_then(|d| d.split('\'').nth(1))
                    .map(|s| s.to_string());
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
        Value::Html(html) => AxumHtml(html).into_response(),
        Value::String(s) => (StatusCode::OK, s).into_response(),
        Value::Unit => StatusCode::OK.into_response(),
        other => (StatusCode::OK, format!("{}", other)).into_response(),
    }
}

// ── HMAC Helpers ───────────────────────────────────────────────────

// Note: `generate_hmac_key` is retained for tests (random key generation).
// Production code now uses `load_hmac_key` which reads METALOGOS_HMAC_KEY env var.
#[allow(dead_code)]
fn generate_hmac_key() -> Vec<u8> {
    // Наряд №173: rand 0.10 API — `rng()` + `fill_bytes`.
    use rand::Rng;
    let mut key = vec![0u8; 32];
    rand::rng().fill_bytes(&mut key);
    key
}

/// Наряд №29 §2.1 — Load HMAC signing key.
///
/// Priority:
/// 1. `METALOGOS_HMAC_KEY` env var (hex-encoded, 64 hex chars = 32 bytes).
///    Allows session cookies to survive restarts.
/// 2. Random fallback (`rand::thread_rng().gen::<[u8; 32]>()`),
///    with a WARNING log. Sessions will be invalidated on restart.
///
/// Never panics — returns a valid 32-byte key in all cases.
fn load_hmac_key() -> Vec<u8> {
    const EXPECTED_HEX_LEN: usize = 64; // 32 bytes * 2 hex chars
    const EXPECTED_BYTE_LEN: usize = 32;

    match std::env::var("METALOGOS_HMAC_KEY") {
        Ok(hex_str) => {
            let hex_str = hex_str.trim();
            if hex_str.len() != EXPECTED_HEX_LEN {
                eprintln!(
                    "[WARN] METALOGOS_HMAC_KEY has length {} (expected {} hex chars / 32 bytes) \
                     — generating random key; sessions will not survive restart",
                    hex_str.len(),
                    EXPECTED_HEX_LEN
                );
            } else {
                match hex::decode(hex_str) {
                    Ok(bytes) if bytes.len() == EXPECTED_BYTE_LEN => {
                        eprintln!(
                            "[INFO] METALOGOS_HMAC_KEY loaded from env ({} bytes)",
                            bytes.len()
                        );
                        return bytes;
                    }
                    Ok(bytes) => {
                        eprintln!(
                            "[WARN] METALOGOS_HMAC_KEY decoded to {} bytes (expected {}) \
                             — generating random key; sessions will not survive restart",
                            bytes.len(),
                            EXPECTED_BYTE_LEN
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "[WARN] METALOGOS_HMAC_KEY is not valid hex: {} \
                             — generating random key; sessions will not survive restart",
                            e
                        );
                    }
                }
            }
        }
        Err(std::env::VarError::NotPresent) => {
            eprintln!(
                "[WARN] METALOGOS_HMAC_KEY env var not set \
                 — generating random key; sessions will not survive restart"
            );
        }
        Err(std::env::VarError::NotUnicode(_)) => {
            eprintln!(
                "[WARN] METALOGOS_HMAC_KEY env var is not valid UTF-8 \
                 — generating random key; sessions will not survive restart"
            );
        }
    }

    // Fallback: generate random 32-byte key.
    // Наряд №173: rand 0.10 API — `rng()` replaces `thread_rng()`,
    // `gen::<T>()` renamed to `random::<T>()` in `RngExt` (extension trait).
    use rand::RngExt;
    let key: [u8; 32] = rand::rng().random();
    key.to_vec()
}

pub fn sign_cookie(value: &str, key: &[u8]) -> String {
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    // SHA256 HMAC accepts any key length, so new_from_slice never errors here.
    // Use a fallback to avoid panicking on the (theoretically impossible) error case.
    let mut mac = match HmacSha256::new_from_slice(key) {
        Ok(m) => m,
        Err(e) => {
            eprintln!(
                "[security] HMAC sign failed (invalid key length: {}): using unsigned value",
                e
            );
            return value.to_string();
        }
    };
    mac.update(value.as_bytes());
    let result = mac.finalize();
    let signature = hex::encode(result.into_bytes());
    format!("{}.{}", value, signature)
}

pub fn verify_cookie(cookie: &str, key: &[u8]) -> Option<String> {
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let parts: Vec<&str> = cookie.rsplitn(2, '.').collect();
    if parts.len() != 2 {
        return None;
    }
    let signature = parts[0];
    let value = parts[1];

    let mut mac = match HmacSha256::new_from_slice(key) {
        Ok(m) => m,
        Err(e) => {
            eprintln!(
                "[security] HMAC verify failed (invalid key length: {}): rejecting cookie",
                e
            );
            return None;
        }
    };
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

pub(crate) fn build_interpreter_with_server(
    srv: &MlogServerDecl,
    mut interp: Interpreter,
) -> Interpreter {
    interp = merge_templates(srv, interp);
    interp
}

pub(crate) fn merge_interpreter(from: Interpreter, mut into: Interpreter) -> Interpreter {
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
        assert_eq!(
            escape_html("<script>alert(1)</script>"),
            "&lt;script&gt;alert(1)&lt;/script&gt;"
        );
        assert_eq!(
            escape_html("Hello & \"world\""),
            "Hello &amp; &quot;world&quot;"
        );
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
        let _signed = sign_cookie(value, &key);
        let tampered = format!("{}.deadbeef", value);
        let verified = verify_cookie(&tampered, &key);
        assert!(verified.is_none());
    }

    #[test]
    fn test_opaque_types_in_value_enum() {
        let html = Value::Html("<h1>Test</h1>".to_string());
        assert_eq!(html.type_name(), "Html");
        assert_eq!(format!("{}", html), "<h1>Test</h1>");

        let secret = Value::Secret(crate::interpreter::SecretString::new(
            "my-api-key".to_string(),
        ));
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
        // Наряд №29 §2.2: value tuple is (session_id, created_at).
        state
            .csrf_tokens
            .insert(token.clone(), (String::new(), std::time::Instant::now()));

        // Simulate POST with matching cookie and header
        let mut headers = HeaderMap::new();
        headers.insert(
            "cookie",
            HeaderValue::from_str(&format!("_mlog_csrf={}", token)).unwrap(),
        );
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
        headers.insert(
            "cookie",
            HeaderValue::from_str(&format!("_mlog_csrf={}", token)).unwrap(),
        );
        headers.insert(
            "x-csrf-token",
            HeaderValue::from_str("wrong_token_value").unwrap(),
        );

        let result = check_csrf(&state, &headers).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status(), StatusCode::FORBIDDEN);
    }

    // ── Наряд №262 Tests: CSRF strict — server-issued tokens + session binding ──

    #[tokio::test]
    async fn test_262_csrf_unissued_pair_rejected() {
        // A self-made double-submit pair that was NEVER issued by this server:
        // pre-№262 the stateless fallback accepted it (naive double-submit
        // bypass — plant a cookie + send any matching header); now it must be
        // 403 with an audit entry naming the root cause.
        let state = make_test_state().await;
        let forged = "deadbeefdeadbeefdeadbeefdeadbeef";

        let mut headers = HeaderMap::new();
        headers.insert(
            "cookie",
            HeaderValue::from_str(&format!("_mlog_csrf={}", forged)).unwrap(),
        );
        headers.insert("x-csrf-token", HeaderValue::from_str(forged).unwrap());

        let result = check_csrf(&state, &headers).await;
        assert!(
            result.is_err(),
            "a token absent from csrf_tokens must be rejected"
        );
        assert_eq!(result.unwrap_err().status(), StatusCode::FORBIDDEN);
        let log = state.audit_log.read().await;
        assert!(
            log.iter().any(|e| e.contains("not issued by this server")),
            "audit must name the missing issuance: {:?}",
            *log
        );
    }

    #[tokio::test]
    async fn test_262_csrf_session_binding_match_passes() {
        // Issued for sess-A + the same session presented → passes.
        let state = make_test_state().await;
        let token = generate_csrf_token();
        state.csrf_tokens.insert(
            token.clone(),
            ("sess-A".to_string(), std::time::Instant::now()),
        );

        let signed_a = sign_cookie("sess-A", &state.hmac_key);
        let mut headers = HeaderMap::new();
        headers.insert(
            "cookie",
            HeaderValue::from_str(&format!("_mlog_csrf={}; _mlog_session={}", token, signed_a))
                .unwrap(),
        );
        headers.insert("x-csrf-token", HeaderValue::from_str(&token).unwrap());

        let result = check_csrf(&state, &headers).await;
        assert!(result.is_ok(), "issued token + its own session must pass");
    }

    #[tokio::test]
    async fn test_262_csrf_session_binding_mismatch_rejected() {
        // Issued for sess-A, presented with sess-B → 403 + audit entry.
        let state = make_test_state().await;
        let token = generate_csrf_token();
        state.csrf_tokens.insert(
            token.clone(),
            ("sess-A".to_string(), std::time::Instant::now()),
        );

        let signed_b = sign_cookie("sess-B", &state.hmac_key);
        let mut headers = HeaderMap::new();
        headers.insert(
            "cookie",
            HeaderValue::from_str(&format!("_mlog_csrf={}; _mlog_session={}", token, signed_b))
                .unwrap(),
        );
        headers.insert("x-csrf-token", HeaderValue::from_str(&token).unwrap());

        let result = check_csrf(&state, &headers).await;
        assert!(
            result.is_err(),
            "a token bound to sess-A must not pass with sess-B"
        );
        assert_eq!(result.unwrap_err().status(), StatusCode::FORBIDDEN);
        let log = state.audit_log.read().await;
        assert!(
            log.iter().any(|e| e.contains("session binding mismatch")),
            "audit must record the binding mismatch: {:?}",
            *log
        );
    }

    #[tokio::test]
    async fn test_262_csrf_bound_token_rejected_without_session() {
        // A session-bound token cannot prove ownership without its session.
        let state = make_test_state().await;
        let token = generate_csrf_token();
        state.csrf_tokens.insert(
            token.clone(),
            ("sess-A".to_string(), std::time::Instant::now()),
        );

        let mut headers = HeaderMap::new();
        headers.insert(
            "cookie",
            HeaderValue::from_str(&format!("_mlog_csrf={}", token)).unwrap(),
        );
        headers.insert("x-csrf-token", HeaderValue::from_str(&token).unwrap());

        let result = check_csrf(&state, &headers).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_262_csrf_sessionless_token_rejected_with_session() {
        // Issuance without a session binds the token to "" — replaying it WITH
        // a valid session is a binding mismatch ("no-session" token is only
        // valid without a session; pinned honest boundary of №262).
        let state = make_test_state().await;
        let token = generate_csrf_token();
        state
            .csrf_tokens
            .insert(token.clone(), (String::new(), std::time::Instant::now()));

        let signed_a = sign_cookie("sess-A", &state.hmac_key);
        let mut headers = HeaderMap::new();
        headers.insert(
            "cookie",
            HeaderValue::from_str(&format!("_mlog_csrf={}; _mlog_session={}", token, signed_a))
                .unwrap(),
        );
        headers.insert("x-csrf-token", HeaderValue::from_str(&token).unwrap());

        let result = check_csrf(&state, &headers).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_262_csrf_expired_token_rejected() {
        // TTL contract (Наряд №29 §2.2) unchanged by №262: 15 minutes.
        let state = make_test_state().await;
        let token = generate_csrf_token();
        let created = std::time::Instant::now()
            .checked_sub(std::time::Duration::from_secs(901))
            .expect("monotonic clock older than 901s required for this test");
        state
            .csrf_tokens
            .insert(token.clone(), (String::new(), created));

        let mut headers = HeaderMap::new();
        headers.insert(
            "cookie",
            HeaderValue::from_str(&format!("_mlog_csrf={}", token)).unwrap(),
        );
        headers.insert("x-csrf-token", HeaderValue::from_str(&token).unwrap());

        let result = check_csrf(&state, &headers).await;
        assert!(
            result.is_err(),
            "an expired token must be rejected even though it was issued"
        );
        assert_eq!(result.unwrap_err().status(), StatusCode::FORBIDDEN);
        let log = state.audit_log.read().await;
        assert!(
            log.iter().any(|e| e.contains("token expired")),
            "audit must record the expiry: {:?}",
            *log
        );
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
        let expired_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sessions WHERE id = 'expired-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!expired_exists);

        let valid_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sessions WHERE id = 'valid-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(valid_exists);
    }

    #[test]
    fn test_74_extract_client_ip_from_headers() {
        // Наряд №263 contract (rewritten): headers are honored ONLY when the
        // direct peer is in the trusted-proxies list; otherwise the peer wins.
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("10.0.0.1, 172.16.0.1"),
        );
        let no_trusted = TrustedProxies::default();
        let trusted_loopback =
            TrustedProxies::from_env_spec(Some("127.0.0.1"));
        let peer_loopback: Option<IpAddr> = Some("127.0.0.1".parse().unwrap());
        let peer_other: Option<IpAddr> = Some("192.168.1.5".parse().unwrap());

        // (1) No trusted proxies: XFF is IGNORED, the peer is the key...
        assert_eq!(extract_client_ip(&headers, peer_other, &no_trusted), "192.168.1.5");
        //    ...and without a peer (direct unit calls) → "unknown", as before.
        assert_eq!(extract_client_ip(&headers, None, &no_trusted), "unknown");

        // (2) Trusted peer: leftmost XFF entry wins.
        assert_eq!(
            extract_client_ip(&headers, peer_loopback, &trusted_loopback),
            "10.0.0.1"
        );

        // (3) Trusted peer without XFF: X-Real-IP, else the peer.
        let mut headers2 = HeaderMap::new();
        headers2.insert("x-real-ip", HeaderValue::from_static("192.168.1.100"));
        assert_eq!(
            extract_client_ip(&headers2, peer_loopback, &trusted_loopback),
            "192.168.1.100"
        );
        assert_eq!(
            extract_client_ip(&HeaderMap::new(), peer_loopback, &trusted_loopback),
            "127.0.0.1"
        );

        // (4) Peer NOT in the list: headers never honored even when the env is set.
        assert_eq!(
            extract_client_ip(&headers, peer_other, &trusted_loopback),
            "192.168.1.5"
        );
    }

    // ── Наряд №263: trusted-proxies parsing (no new crates, manual CIDR) ──

    #[test]
    fn test_n263_trusted_proxies_parse_table() {
        let unset = TrustedProxies::from_env_spec(None);
        assert!(!unset.is_configured());
        let empty = TrustedProxies::from_env_spec(Some(""));
        assert!(!empty.is_configured());

        // Exact IP + CIDR, mixed families, spaces tolerated.
        let list = TrustedProxies::from_env_spec(Some(" 10.0.0.1 , 10.0.0.0/8 , fc00::/7 , ::1 "));
        assert!(list.is_configured());
        assert!(list.contains("10.0.0.1".parse().unwrap()));
        assert!(list.contains("10.255.255.255".parse().unwrap()));
        assert!(!list.contains("11.0.0.1".parse().unwrap()));
        assert!(list.contains("fc00::1".parse().unwrap()));
        assert!(list.contains("fdff::1".parse().unwrap())); // fc00::/7 covers fc00–fdff
        assert!(!list.contains("fe00::1".parse().unwrap()));
        assert!(list.contains("::1".parse().unwrap()));
        assert!(!list.contains("::2".parse().unwrap()));

        // IPv4-mapped IPv6 peer unwraps to its V4 form (№261 лекало).
        assert!(list.contains("::ffff:10.1.2.3".parse().unwrap()));
        assert!(!list.contains("::ffff:11.1.2.3".parse().unwrap()));

        // /0 matches everything in-family (documented edge); /33, /129 invalid.
        let v4all = TrustedProxies::from_env_spec(Some("0.0.0.0/0"));
        assert!(v4all.contains("8.8.8.8".parse().unwrap()));
        assert!(!v4all.contains("::1".parse().unwrap())); // mixed family never matches
        let bad = TrustedProxies::from_env_spec(Some("10.0.0.0/33, banana, 10.0.0.0/129"));
        assert!(!bad.is_configured()); // every entry invalid → skipped loudly
    }

    // ── Наряд №263: bounded state maps — loud refusals, not silent growth ──

    #[tokio::test]
    async fn test_n263_rate_key_cap_new_key_blocked_existing_key_works() {
        let state = make_test_state().await;
        let now = std::time::Instant::now();
        // Fill the key store to the cap.
        for i in 0..MAX_RATE_KEYS {
            state.rate_limits.insert(format!("k{}", i), vec![now]);
        }
        assert_eq!(state.rate_limits.len(), MAX_RATE_KEYS);

        // A NEW key at the cap = "full bucket" → 429, loud audit entry.
        let result = check_rate_limit(&state, "fresh-peer", 100).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status(), StatusCode::TOO_MANY_REQUESTS);
        let log = state.audit_log.read().await;
        assert!(
            log.iter().any(|e| e.contains("key store full")),
            "the cap refusal must be loud in the audit log"
        );
        drop(log);

        // An EXISTING key keeps working at the cap (the sweep reclaims stale ones).
        let result = check_rate_limit(&state, "k0", 100).await;
        assert!(result.is_ok());
        // Memory is bounded: the map never exceeded the cap.
        assert!(state.rate_limits.len() <= MAX_RATE_KEYS);
    }

    #[tokio::test]
    async fn test_n263_session_cap_refuses_loudly_with_503_text() {
        let state = make_test_state().await;
        let entry = |ttl_secs: u64| SessionEntry {
            data: HashMap::new(),
            roles: vec!["user".to_string()],
            expires: std::time::Instant::now() + std::time::Duration::from_secs(ttl_secs),
        };
        for i in 0..MAX_SESSIONS {
            insert_session_capped(&state, format!("s{}", i), entry(3600)).unwrap();
        }
        // At the cap a NEW session is refused with the loud 503 text.
        let err =
            insert_session_capped(&state, "overflow".to_string(), entry(3600)).unwrap_err();
        assert!(err.contains("503"), "refusal must carry 503 semantics: {}", err);
        assert!(err.contains("session store full"));
        // Replacing an EXISTING id stays allowed (updates are not new entries).
        insert_session_capped(&state, "s0".to_string(), entry(3600)).unwrap();
        assert_eq!(state.sessions.len(), MAX_SESSIONS);
    }

    #[tokio::test]
    async fn test_n263_csrf_token_cap_refuses_issuance_with_503() {
        let state = make_test_state().await;
        for i in 0..MAX_CSRF_TOKENS {
            state
                .csrf_tokens
                .insert(format!("t{}", i), ("".to_string(), std::time::Instant::now()));
        }
        let result = issue_csrf_token_capped(&state, "fresh-token", "").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status(), StatusCode::SERVICE_UNAVAILABLE);
        let log = state.audit_log.read().await;
        assert!(log.iter().any(|e| e.contains("token store full")));
        drop(log);
        // The fresh token was NOT inserted (memory bounded).
        assert_eq!(state.csrf_tokens.len(), MAX_CSRF_TOKENS);
        assert!(!state.csrf_tokens.contains_key("fresh-token"));
    }

    // ── Наряд №263: the sweep now covers rate_limits and sessions too ──

    #[tokio::test]
    async fn test_n263_sweep_removes_stale_rate_keys_and_expired_sessions() {
        let state = make_test_state().await;
        let now = std::time::Instant::now();
        let old =
            now.checked_sub(std::time::Duration::from_secs(RATE_WINDOW_SECS + 30)).unwrap();

        // rate_limits: a fully-stale key (evicted) vs a live key (kept),
        // plus a half-stale key (stale timestamps trimmed, key stays).
        state.rate_limits.insert("stale-key".to_string(), vec![old]);
        state.rate_limits.insert("live-key".to_string(), vec![now]);
        state.rate_limits.insert("mixed-key".to_string(), vec![old, now]);

        // sessions: expired (evicted) vs live (kept).
        let session_entry = |expired: bool| SessionEntry {
            data: HashMap::new(),
            roles: vec![],
            expires: if expired {
                now.checked_sub(std::time::Duration::from_secs(60)).unwrap()
            } else {
                now + std::time::Duration::from_secs(3600)
            },
        };
        insert_session_capped(&state, "expired-session".to_string(), session_entry(true)).unwrap();
        insert_session_capped(&state, "live-session".to_string(), session_entry(false)).unwrap();

        // csrf_tokens: expired (evicted) vs live (kept) — the original №29 behavior.
        let backdated = now.checked_sub(std::time::Duration::from_secs(901)).unwrap();
        state.csrf_tokens.insert("expired-token".to_string(), ("".into(), backdated));
        state.csrf_tokens.insert("live-token".to_string(), ("".into(), now));

        let (csrf, rate_keys, sessions) = sweep_expired_state(&state);
        assert_eq!((csrf, rate_keys, sessions), (1, 1, 1));
        assert!(!state.rate_limits.contains_key("stale-key"));
        assert!(state.rate_limits.contains_key("live-key"));
        assert!(state.rate_limits.contains_key("mixed-key"));
        assert_eq!(state.rate_limits.get("mixed-key").unwrap().len(), 1);
        assert!(!state.sessions.contains_key("expired-session"));
        assert!(state.sessions.contains_key("live-session"));
        assert!(!state.csrf_tokens.contains_key("expired-token"));
        assert!(state.csrf_tokens.contains_key("live-token"));
    }

    // ── Наряд №263: the rate_limit declaration field drives the limit ──

    #[tokio::test]
    async fn test_n263_rate_limit_field_parses_and_wires() {
        let state = build_test_server_state(
            r#"
mlogserver {
    port: 0
    middleware: [rate_limit]
    rate_limit: 7
    route "/ok" method=GET { respond("200", "ok") }
}
"#,
        )
        .await;
        assert_eq!(state.rate_limit_per_minute, 7);

        // Absent field → the documented default 100 (pre-№263 hard-wired value).
        let default_state = build_test_server_state(
            r#"
mlogserver {
    port: 0
    middleware: [rate_limit]
    route "/ok" method=GET { respond("200", "ok") }
}
"#,
        )
        .await;
        assert_eq!(default_state.rate_limit_per_minute, DEFAULT_RATE_LIMIT_PER_MINUTE);
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
            sessions: Arc::new(DashMap::new()),
            csrf_tokens: Arc::new(DashMap::new()),
            hmac_key: Arc::new(generate_hmac_key()),
            audit_log: Arc::new(RwLock::new(Vec::new())),
            templates: Arc::new(RwLock::new(HashMap::new())),
            db_store: Arc::new(RwLock::new(Vec::new())),
            memory_persist: None,
            interpreter: Arc::new(RwLock::new(Interpreter::new())),
            routes: Vec::new(),
            middleware: vec![
                "session".to_string(),
                "csrf".to_string(),
                "rate_limit".to_string(),
            ],
            db: Arc::new(tokio::sync::Mutex::new(conn)),
            rate_limits: Arc::new(DashMap::new()),
            rate_limit_per_minute: DEFAULT_RATE_LIMIT_PER_MINUTE,
            trusted_proxies: Arc::new(TrustedProxies::default()),
            backend: ServeBackend::Interpreter,
            vm_program: None,
            vm_routes: Vec::new(),
        }
    }

    // ── Наряд №40 Tests: VM backend ──────────────────────────────

    #[tokio::test]
    async fn test_n40_backend_env_default_is_interpreter() {
        // Ensure default (no env var) is Interpreter
        std::env::remove_var("METALOGOS_SERVE_BACKEND");
        let backend = match std::env::var("METALOGOS_SERVE_BACKEND") {
            Ok(val) if val == "vm" => ServeBackend::Vm,
            Ok(val) if val == "interpreter" => ServeBackend::Interpreter,
            Ok(_) => ServeBackend::Interpreter,  // fallback
            Err(_) => ServeBackend::Interpreter, // default
        };
        assert_eq!(backend, ServeBackend::Interpreter);
    }

    #[tokio::test]
    async fn test_n40_backend_env_unknown_falls_back() {
        // "typo" → fallback to Interpreter, not panic
        let backend = match Some("typo".to_string()) {
            Some(ref val) if *val == "vm" => ServeBackend::Vm,
            Some(ref val) if *val == "interpreter" => ServeBackend::Interpreter,
            Some(_) => ServeBackend::Interpreter, // fallback on unknown
            None => ServeBackend::Interpreter,
        };
        assert_eq!(backend, ServeBackend::Interpreter);
    }

    #[tokio::test]
    async fn test_n40_crashing_route_returns_500_interpreter() {
        // Route body: divide by string (error in expression evaluation)
        let source = r#"
mlogserver {
    port: 0
    host: "127.0.0.1"
    route "/crash" method=GET {
        let x = "hello" / 3
        respond("200", "should not reach here")
    }
}
"#;
        let state = build_test_server_state(source).await;
        let response = call_route(&state, "GET", "/crash", "", "").await;
        assert_eq!(response.status(), 500);
    }

    #[tokio::test]
    async fn test_n40_ok_route_returns_200_interpreter() {
        let source = r#"
mlogserver {
    port: 0
    host: "127.0.0.1"
    route "/ok" method=GET {
        respond("200", "hello")
    }
}
"#;
        let state = build_test_server_state(source).await;
        let response = call_route(&state, "GET", "/ok", "", "").await;
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn test_n40_query_param_isolation() {
        // Two requests with different query params must get their own values.
        // This proves per-request isolation: request A's query_param("name")
        // does not leak into request B.
        let source = r#"
mlogserver {
    port: 0
    host: "127.0.0.1"
    route "/echo" method=GET {
        let name = query_param("name")
        respond("200", name)
    }
}
"#;
        let state = build_test_server_state(source).await;

        // Request A: name=Alice
        let resp_a = call_route(&state, "GET", "/echo", "name=Alice", "").await;
        assert_eq!(resp_a.status(), 200);
        let body_a = body_to_string(resp_a).await;
        assert_eq!(body_a, "Alice");

        // Request B: name=Bob
        let resp_b = call_route(&state, "GET", "/echo", "name=Bob", "").await;
        assert_eq!(resp_b.status(), 200);
        let body_b = body_to_string(resp_b).await;
        assert_eq!(body_b, "Bob");
    }

    #[tokio::test]
    async fn test_n40_kv_set_shared_between_requests() {
        // kv_set in one request must be visible in the next (shared store).
        // This confirms that global state (kv_set/kv_get) works across requests,
        // unlike local variables which are isolated.
        let source = r#"
mlogserver {
    port: 0
    host: "127.0.0.1"
    route "/set" method=GET {
        kv_set("test_key", "test_value")
        respond("200", "set")
    }
    route "/get" method=GET {
        let val = kv_get("test_key")
        respond("200", val)
    }
}
"#;
        let state = build_test_server_state(source).await;

        // Clear any previous value (via interpreter builtin call)
        {
            let interp = state.interpreter.write().await;
            if let Some(fn_kv) = interp.get_builtin("kv_delete") {
                let _ = fn_kv(&[crate::interpreter::Value::String("test_key".to_string())]);
            }
        }

        // Set
        let resp_set = call_route(&state, "GET", "/set", "", "").await;
        assert_eq!(resp_set.status(), 200);

        // Get — should see the value set by previous request
        let resp_get = call_route(&state, "GET", "/get", "", "").await;
        assert_eq!(resp_get.status(), 200);
        let body_get = body_to_string(resp_get).await;
        assert_eq!(body_get, "test_value");

        // Cleanup
        {
            let interp = state.interpreter.write().await;
            if let Some(fn_kv) = interp.get_builtin("kv_delete") {
                let _ = fn_kv(&[crate::interpreter::Value::String("test_key".to_string())]);
            }
        }
    }

    /// Helper: extract body string from a Response.
    async fn body_to_string(resp: axum::response::Response) -> String {
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap_or_default();
        String::from_utf8(bytes.to_vec()).unwrap_or_default()
    }

    /// Helper: build a minimal ServerState from mlog source for testing.
    async fn build_test_server_state(source: &str) -> ServerState {
        let declarations = crate::parser::parse(source).unwrap();
        let mut interp = Interpreter::new();
        for decl in declarations.clone() {
            match decl {
                Declaration::MlogServer(ref srv) => {
                    interp = build_interpreter_with_server(srv, interp);
                }
                Declaration::Flow(_) => {}
                _ => {
                    let mut tmp = Interpreter::new();
                    tmp.set_base_dir(std::path::PathBuf::from("."));
                    let _ = tmp.run(vec![decl]);
                    interp = merge_interpreter(tmp, interp);
                }
            }
        }
        let config = declarations
            .iter()
            .find_map(|d| match d {
                Declaration::MlogServer(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap();
        build_state(config, interp).await.unwrap()
    }

    /// Helper: simulate calling a route on the server state.
    async fn call_route(
        state: &ServerState,
        method: &str,
        path: &str,
        query: &str,
        body: &str,
    ) -> axum::response::Response {
        let _uri: Uri = format!("{}?{}", path, query).parse().unwrap();
        let method = match method {
            "GET" => Method::GET,
            "POST" => Method::POST,
            _ => Method::GET,
        };
        let headers = HeaderMap::new();
        let body_bytes = bytes::Bytes::from(body.to_string());

        let query_map: std::collections::HashMap<String, String> = if query.is_empty() {
            HashMap::new()
        } else {
            query
                .split('&')
                .filter_map(|pair| {
                    let mut parts = pair.splitn(2, '=');
                    let key = parts.next()?.to_string();
                    let val = parts.next().unwrap_or("").to_string();
                    Some((key, val))
                })
                .collect()
        };

        let result = execute_route_body(
            state,
            &state
                .routes
                .iter()
                .find(|r| r.path == path && r.method == method.as_str())
                .unwrap()
                .body,
            &headers,
            &body_bytes,
            &query_map,
        )
        .await;
        match result {
            Ok(resp) => resp,
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Handler error: {}", e),
            )
                .into_response(),
        }
    }

    /// Наряд №41 Block 1: match statement must cause compilation error, not silent stub.
    #[tokio::test]
    async fn test_n41_match_not_compilable_in_vm() {
        use crate::compiler::Compiler;
        use crate::parser;

        let source = r#"
mlogserver {
    port: 0
    host: "127.0.0.1"
    route "/match_test" method=GET {
        let x = "hello"
        match x {
            "hello" then { respond("200", "matched") }
            else { respond("200", "default") }
        }
    }
}
"#;
        let declarations = parser::parse(source).unwrap();
        let config = declarations
            .iter()
            .find_map(|d| match d {
                Declaration::MlogServer(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap();
        let compiler = Compiler::new();
        // compile_routes should return Err because route body contains match
        let result = compiler.compile_routes(&config.routes);
        assert!(
            result.is_err(),
            "compile_routes must return Err for match statement, got Ok"
        );
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("Match statement not yet supported"),
            "error message should mention Match, got: {}",
            err_msg
        );
    }

    /// Наряд №41 Block 1: routes without match still compile fine.
    #[tokio::test]
    async fn test_n41_non_match_routes_compile_in_vm() {
        use crate::compiler::Compiler;
        use crate::parser;

        let source = r#"
mlogserver {
    port: 0
    host: "127.0.0.1"
    route "/ok" method=GET {
        let x = "hello"
        respond("200", x)
    }
}
"#;
        let declarations = parser::parse(source).unwrap();
        let config = declarations
            .iter()
            .find_map(|d| match d {
                Declaration::MlogServer(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap();
        let compiler = Compiler::new();
        let result = compiler.compile_routes(&config.routes);
        assert!(
            result.is_ok(),
            "compile_routes should succeed for routes without match, got Err: {}",
            result.unwrap_err()
        );
    }

    /// Наряд №41 Block 4: Side-effect parity test — both backends produce
    /// same HTTP status for an identical route.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_n41_side_effect_parity() {
        use crate::compiler::Compiler;
        use crate::vm::Vm;

        let source = r#"
mlogserver {
    port: 0
    host: "127.0.0.1"
    route "/parity" method=GET {
        let x = query_param("x")
        if x == "crash" then { let _ = 1 / 0 }
        respond("200", "x=" + x)
    }
}
"#;
        // Build server state (interpreter backend)
        let state = build_test_server_state(source).await;

        // Compile routes for VM
        let mut compiler = Compiler::new();
        let declarations = crate::parser::parse(source).unwrap();
        let config = declarations
            .iter()
            .find_map(|d| match d {
                Declaration::MlogServer(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap();
        let compiled_routes = compiler.compile_routes(&config.routes).unwrap();
        let program = compiler.compile(declarations).unwrap();

        let compiled = &compiled_routes[0];

        // Helper: execute route via VM directly (bypasses state.vm_program check)
        async fn call_route_vm_direct(
            state: &ServerState,
            program: &crate::bytecode::Program,
            compiled: &crate::bytecode::CompiledRoute,
            query: &std::collections::HashMap<String, String>,
        ) -> Result<axum::response::Response, String> {
            // Clone data needed inside spawn_blocking (closure must be 'static + Send)
            let program = program.clone();
            let compiled = compiled.clone();
            let query = query.clone();
            let (audit_entries, result) = tokio::task::spawn_blocking(move || {
                let mut vm = Vm::new();
                vm.load_program(&program)
                    .map_err(|e| format!("VM route init: {}", e))?;
                vm.clear_server_context();
                if !query.is_empty() {
                    vm.set_server_query_params(query.clone());
                }
                let r = vm.execute_route_code(&compiled, &program);
                let entries = vm.take_audit_log();
                Result::<_, String>::Ok((entries, r))
            })
            .await
            .map_err(|e| format!("blocking task panicked: {}", e))??;
            flush_vm_audit_entries_to_db(state, &audit_entries).await;
            match result {
                Ok(val) => {
                    if let crate::interpreter::Value::HttpResponse { status, body } = val {
                        let code = StatusCode::from_u16(status).unwrap_or(StatusCode::OK);
                        Ok((code, body).into_response())
                    } else {
                        Ok(value_to_response(val))
                    }
                }
                Err(e) => Err(e),
            }
        }

        // Test 1: OK response parity
        let query_ok: std::collections::HashMap<String, String> =
            [("x".to_string(), "hello".to_string())]
                .into_iter()
                .collect();

        let interp_resp = call_route(&state, "GET", "/parity", "x=hello", "").await;
        let vm_resp = call_route_vm_direct(&state, &program, compiled, &query_ok)
            .await
            .unwrap();

        assert_eq!(
            interp_resp.status(),
            vm_resp.status(),
            "HTTP status mismatch for OK case"
        );

        // Test 2: crash response parity (both should error)
        let query_crash: std::collections::HashMap<String, String> =
            [("x".to_string(), "crash".to_string())]
                .into_iter()
                .collect();

        let interp_crash = call_route(&state, "GET", "/parity", "x=crash", "").await;
        let vm_crash = call_route_vm_direct(&state, &program, compiled, &query_crash).await;

        let interp_is_error = interp_crash.status() == 500;
        let vm_is_error = vm_crash.is_err();
        assert!(
            interp_is_error && vm_is_error,
            "Both backends should error on crash: interp_status={}, vm_is_error={}",
            interp_crash.status(),
            vm_is_error
        );
    }
}
