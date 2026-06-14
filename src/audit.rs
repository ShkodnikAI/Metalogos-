// ── Static security analysis for METALOGOS (.mlog) programs ─────
// ADR-0057: `mlog audit <file>` — analyzes without executing.
// Checks: SECRETS, HTML_INJECTION, SQL_DYNAMIC, SANDBOX_COVERAGE,
//         RATE_LIMIT, CSRF, SECRET_LEAK, OPEN_REDIRECT.

use crate::ast::*;
use crate::parser;
use std::collections::HashMap;

/// Severity of an audit finding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A single audit finding with severity, check ID, line number, and message.
#[derive(Debug, Clone)]
pub struct AuditFinding {
    pub severity: Severity,
    pub check_id: &'static str,
    pub line: usize,
    pub message: String,
}

/// Complete audit result: list of findings + summary formatting.
#[derive(Debug, Clone)]
pub struct AuditResult {
    pub findings: Vec<AuditFinding>,
}

impl AuditResult {
    pub fn error_count(&self) -> usize {
        self.findings.iter().filter(|f| f.severity == Severity::Error).count()
    }
    pub fn warning_count(&self) -> usize {
        self.findings.iter().filter(|f| f.severity == Severity::Warning).count()
    }
    pub fn info_count(&self) -> usize {
        self.findings.iter().filter(|f| f.severity == Severity::Info).count()
    }

    /// Exit code: 0 = clean, 1 = has errors, 2 = warnings only.
    pub fn exit_code(&self) -> i32 {
        if self.error_count() > 0 {
            1
        } else if self.warning_count() > 0 {
            2
        } else {
            0
        }
    }

    /// Format all findings for console output.
    /// Example:
    ///   [ERROR] line 15: SQL injection risk — query() with non-literal SQL
    ///   [WARN]  line 23: LLM output passed to respond() — use template for XSS safety
    ///   [INFO]  line 1: server has csrf middleware ✓
    ///
    ///   Summary: 1 error, 2 warnings, 2 passed
    pub fn format(&self) -> String {
        let mut lines: Vec<String> = Vec::new();

        for f in &self.findings {
            let tag = match f.severity {
                Severity::Error => "ERROR",
                Severity::Warning => "WARN",
                Severity::Info => "INFO",
            };
            // Pad tag to 5 chars for alignment
            lines.push(format!("[{}] line {}: {}", tag, f.line, f.message));
        }

        // Summary line
        let ec = self.error_count();
        let wc = self.warning_count();
        let pc = self.info_count();
        let parts: Vec<String> = Vec::new();
        let mut parts = parts;
        if ec == 1 { parts.push("1 error".to_string()); }
        else if ec > 1 { parts.push(format!("{} errors", ec)); }
        if wc == 1 { parts.push("1 warning".to_string()); }
        else if wc > 1 { parts.push(format!("{} warnings", wc)); }
        if pc == 1 { parts.push("1 passed".to_string()); }
        else if pc > 1 { parts.push(format!("{} passed", pc)); }
        if parts.is_empty() {
            parts.push("clean".to_string());
        }
        lines.push(format!("Summary: {}", parts.join(", ")));

        lines.join("\n")
    }
}

// ── Taint tracking for data-flow analysis ───────────────────────────

/// Taint kind for tracking data provenance through variable assignments.
#[derive(Debug, Clone, Copy, PartialEq)]
enum TaintKind {
    /// Value came from call_llm() / call_claude() — untrusted HTML.
    LlmOutput,
    /// Value came from env() — a secret that must not be leaked.
    Secret,
    /// Value came from user input (form_data, json_body, query_param).
    UserInput,
    /// Value was processed through render() or escape_html() — safe for HTML output.
    Sanitized,
}

/// Per-scope taint tracker. Maps variable name to its taint kind.
struct TaintTracker {
    tainted: HashMap<String, TaintKind>,
}

impl TaintTracker {
    fn new() -> Self {
        Self { tainted: HashMap::new() }
    }

    fn taint(&mut self, name: &str, kind: TaintKind) {
        self.tainted.insert(name.to_string(), kind);
    }

    fn get_taint(&self, name: &str) -> Option<TaintKind> {
        self.tainted.get(name).copied()
    }

    /// Remove taint from a variable (e.g., after reassignment to a safe value).
    fn untaint(&mut self, name: &str) {
        self.tainted.remove(name);
    }
}

// ── Helper: find line number for a keyword in source ────────────────

/// Find the 1-based line number of the first occurrence of `keyword` in source.
fn find_line(source: &str, keyword: &str) -> usize {
    for (i, line) in source.lines().enumerate() {
        if line.contains(keyword) {
            return i + 1;
        }
    }
    1
}

// ── Secret detection patterns ───────────────────────────────────────

/// Substrings that indicate a hardcoded secret.
const SECRET_PATTERNS: &[&str] = &[
    "sk-", "sk_", "skant", "sk-ant",
    "api_key", "apikey", "API_KEY",
    "secret_key", "secretkey", "SECRET_KEY",
    "access_token", "accesstoken",
    "auth_token", "authtoken",
    "private_key", "privatekey",
    "token=", "TOKEN=",
];

/// Minimum string length to be considered a possible secret.
const SECRET_MIN_LENGTH: usize = 30;

/// Check if a string literal looks like a hardcoded secret.
fn looks_like_secret(s: &str) -> bool {
    if s.len() < SECRET_MIN_LENGTH {
        return false;
    }
    let lower = s.to_lowercase();
    for pattern in SECRET_PATTERNS {
        if lower.contains(&pattern.to_lowercase()) {
            return true;
        }
    }
    false
}

// ── AST walking helpers ─────────────────────────────────────────────

// ── (collect_fn_calls and collect_fn_call_names removed — unused helpers) ──

// ── Check: SECRETS — hardcoded secret strings ────────────────────────

/// Check for hardcoded secrets in string literals across all declarations.
fn check_secrets(declarations: &[Declaration], source: &str, findings: &mut Vec<AuditFinding>) {
    fn walk_string_exprs<'a>(expr: &'a Expr, acc: &mut Vec<&'a String>) {
        match expr {
            Expr::StringLit(s) => acc.push(s),
            Expr::FnCall(name, args) => {
                // Skip env() calls — they are the OK way to get secrets
                if name != "env" {
                    for arg in args { walk_string_exprs(arg, acc); }
                }
            }
            Expr::QualifiedCall { args, .. } => {
                for arg in args { walk_string_exprs(arg, acc); }
            }
            Expr::BinaryOp(l, _, r) => { walk_string_exprs(l, acc); walk_string_exprs(r, acc); }
            Expr::IfElse(c, t, e) => { walk_string_exprs(c, acc); walk_string_exprs(t, acc); walk_string_exprs(e, acc); }
            Expr::List(items) => { for item in items { walk_string_exprs(item, acc); } }
            Expr::FieldAccess(inner, _) => walk_string_exprs(inner, acc),
            Expr::IndexAccess(inner, idx) => { walk_string_exprs(inner, acc); walk_string_exprs(idx, acc); }
            _ => {}
        }
    }
    fn walk_string_stmts<'a>(stmts: &'a [Statement], acc: &mut Vec<&'a String>) {
        for stmt in stmts {
            match stmt {
                Statement::LetBinding { value, .. } => walk_string_exprs(value, acc),
                Statement::Assign { value, .. } => walk_string_exprs(value, acc),
                Statement::ExprStmt(expr) => walk_string_exprs(expr, acc),
                Statement::Return(expr) => walk_string_exprs(expr, acc),
                Statement::Each { body, .. } => walk_string_stmts(body, acc),
                Statement::While { body, .. } => walk_string_stmts(body, acc),
                Statement::IfElseBlock { then_body, else_ifs, else_body, .. } => {
                    walk_string_stmts(then_body, acc);
                    for (_, body) in else_ifs { walk_string_stmts(body, acc); }
                    if let Some(body) = else_body { walk_string_stmts(body, acc); }
                }
                Statement::IfThen(_, body) => walk_string_stmts(body, acc),
                _ => {}
            }
        }
    }

    for decl in declarations {
        let mut strings: Vec<&String> = Vec::new();
        match decl {
            Declaration::Pattern(p) => walk_string_stmts(&p.body, &mut strings),
            Declaration::Tool(t) => {
                for m in &t.methods {
                    walk_string_stmts(&m.body, &mut strings);
                }
            }
            Declaration::MlogServer(srv) => {
                for route in &srv.routes {
                    walk_string_stmts(&route.body, &mut strings);
                }
            }
            Declaration::Hook(h) => walk_string_stmts(&h.body, &mut strings),
            Declaration::Eval(_e) => {
                // Dataset strings are test data, not secrets
            }
            Declaration::Fluid(f) => {
                for v in &f.variants {
                    walk_string_exprs(&v.value, &mut strings);
                }
            }
            Declaration::Memorize(m) => walk_string_exprs(&m.value, &mut strings),
            Declaration::Forget(f) => walk_string_exprs(&f.query, &mut strings),
            Declaration::Rule(r) => {
                walk_string_exprs(&r.target, &mut strings);
                walk_string_exprs(&r.value, &mut strings);
            }
            Declaration::Relate(r) => {
                walk_string_exprs(&r.from, &mut strings);
                walk_string_exprs(&r.to, &mut strings);
            }
            Declaration::Adapt(a) => {
                walk_string_exprs(&a.input_example, &mut strings);
                walk_string_exprs(&a.output_example, &mut strings);
            }
            Declaration::Mutate(m) => {
                for (inp, out) in &m.new_examples {
                    walk_string_exprs(inp, &mut strings);
                    walk_string_exprs(out, &mut strings);
                }
            }
            Declaration::Flow(f) => walk_string_exprs(&f.source, &mut strings),
            Declaration::EntitySimple(e) => walk_string_exprs(&e.value, &mut strings),
            Declaration::EntityRecord(e) => {
                for fi in &e.fields {
                    walk_string_exprs(&fi.value, &mut strings);
                }
            }
            _ => {}
        }

        for s in strings {
            if looks_like_secret(s) {
                // Find a snippet to locate in source
                let snippet = if s.len() > 20 { &s[..20] } else { s.as_str() };
                let line = find_line(source, snippet);
                findings.push(AuditFinding {
                    severity: Severity::Warning,
                    check_id: "SECRETS",
                    line,
                    message: format!("possible hardcoded secret: string matches secret pattern (length={})", s.len()),
                });
            }
        }
    }
}

// ── Check: SQL_DYNAMIC — non-literal SQL in query() ──────────────────

/// Check that all query() calls use literal SQL strings.
fn check_sql_dynamic(declarations: &[Declaration], source: &str, findings: &mut Vec<AuditFinding>) {
    let mut all_literal = true;
    let mut literal_count = 0usize;

    fn check_stmts(stmts: &[Statement], source: &str, findings: &mut Vec<AuditFinding>, all_literal: &mut bool, literal_count: &mut usize) {
        fn walk_query(expr: &Expr, source: &str, findings: &mut Vec<AuditFinding>, all_literal: &mut bool, literal_count: &mut usize) {
            if let Expr::FnCall(name, args) = expr {
                if name == "query" {
                    if let Some(arg) = args.first() {
                        match arg {
                            Expr::StringLit(_) => {
                                *literal_count += 1;
                            }
                            _ => {
                                *all_literal = false;
                                let snippet = match arg {
                                    Expr::Ident(id) => id.clone(),
                                    Expr::BinaryOp(_, _, _) => "dynamic expression".to_string(),
                                    Expr::FnCall(n, _) => format!("{}()", n),
                                    _ => "non-literal".to_string(),
                                };
                                let line = find_line(source, &snippet);
                                findings.push(AuditFinding {
                                    severity: Severity::Error,
                                    check_id: "SQL_DYNAMIC",
                                    line,
                                    message: format!("SQL injection risk — query() with non-literal SQL ({})", snippet),
                                });
                            }
                        }
                    }
                }
                // Recurse into args to find nested query() calls
                for arg in args {
                    walk_query(arg, source, findings, all_literal, literal_count);
                }
            } else {
                // Recurse into other expression types
                match expr {
                    Expr::QualifiedCall { args, .. } => {
                        for arg in args { walk_query(arg, source, findings, all_literal, literal_count); }
                    }
                    Expr::BinaryOp(l, _, r) => {
                        walk_query(l, source, findings, all_literal, literal_count);
                        walk_query(r, source, findings, all_literal, literal_count);
                    }
                    _ => {}
                }
            }
        }
        fn walk_stmt(stmt: &Statement, source: &str, findings: &mut Vec<AuditFinding>, all_literal: &mut bool, literal_count: &mut usize) {
            match stmt {
                Statement::LetBinding { value, .. } => walk_query(value, source, findings, all_literal, literal_count),
                Statement::Assign { value, .. } => walk_query(value, source, findings, all_literal, literal_count),
                Statement::ExprStmt(expr) => walk_query(expr, source, findings, all_literal, literal_count),
                Statement::Return(expr) => walk_query(expr, source, findings, all_literal, literal_count),
                Statement::Each { body, .. } => { for s in body { walk_stmt(s, source, findings, all_literal, literal_count); } }
                Statement::While { body, .. } => { for s in body { walk_stmt(s, source, findings, all_literal, literal_count); } }
                Statement::IfElseBlock { then_body, else_ifs, else_body, .. } => {
                    for s in then_body { walk_stmt(s, source, findings, all_literal, literal_count); }
                    for (_, body) in else_ifs { for s in body { walk_stmt(s, source, findings, all_literal, literal_count); } }
                    if let Some(body) = else_body { for s in body { walk_stmt(s, source, findings, all_literal, literal_count); } }
                }
                Statement::IfThen(_, body) => { for s in body { walk_stmt(s, source, findings, all_literal, literal_count); } }
                _ => {}
            }
        }
        for stmt in stmts {
            walk_stmt(stmt, source, findings, all_literal, literal_count);
        }
    }

    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => check_stmts(&p.body, source, findings, &mut all_literal, &mut literal_count),
            Declaration::Tool(t) => {
                for m in &t.methods { check_stmts(&m.body, source, findings, &mut all_literal, &mut literal_count); }
            }
            Declaration::MlogServer(srv) => {
                for route in &srv.routes { check_stmts(&route.body, source, findings, &mut all_literal, &mut literal_count); }
            }
            Declaration::Hook(h) => check_stmts(&h.body, source, findings, &mut all_literal, &mut literal_count),
            _ => {}
        }
    }

    if literal_count > 0 && all_literal {
        findings.push(AuditFinding {
            severity: Severity::Info,
            check_id: "SQL_DYNAMIC",
            line: 1,
            message: format!("all {} query() calls use literal SQL \u{2713}", literal_count),
        });
    }
}

// ── Check: SANDBOX_COVERAGE — adapt/mutate without sandbox ──────────

fn check_sandbox_coverage(declarations: &[Declaration], source: &str, findings: &mut Vec<AuditFinding>) {
    let has_sandbox = declarations.iter().any(|d| matches!(d, Declaration::Sandbox(_)));

    if !has_sandbox {
        for decl in declarations {
            match decl {
                Declaration::Adapt(a) => {
                    let line = find_line(source, &format!("adapt {}", a.pattern_name));
                    findings.push(AuditFinding {
                        severity: Severity::Warning,
                        check_id: "SANDBOX_COVERAGE",
                        line,
                        message: format!("adapt {} without sandbox declaration", a.pattern_name),
                    });
                }
                Declaration::Mutate(m) => {
                    let line = find_line(source, &format!("mutate {}", m.pattern_name));
                    findings.push(AuditFinding {
                        severity: Severity::Warning,
                        check_id: "SANDBOX_COVERAGE",
                        line,
                        message: format!("mutate {} without sandbox declaration", m.pattern_name),
                    });
                }
                _ => {}
            }
        }
    }
}

// ── Check: RATE_LIMIT — server without rate_limit middleware ─────────

fn check_rate_limit(declarations: &[Declaration], source: &str, findings: &mut Vec<AuditFinding>) {
    for decl in declarations {
        if let Declaration::MlogServer(srv) = decl {
            let has_rate_limit = srv.middleware.iter().any(|m| m == "rate_limit");
            if has_rate_limit {
                findings.push(AuditFinding {
                    severity: Severity::Info,
                    check_id: "RATE_LIMIT",
                    line: find_line(source, "mlogserver"),
                    message: "server has rate_limit middleware \u{2713}".to_string(),
                });
            } else {
                findings.push(AuditFinding {
                    severity: Severity::Warning,
                    check_id: "RATE_LIMIT",
                    line: find_line(source, "mlogserver"),
                    message: "no rate limiting — recommend adding 'rate_limit' middleware".to_string(),
                });
            }
        }
    }
}

// ── Check: CSRF — POST routes without csrf middleware ────────────────

fn check_csrf(declarations: &[Declaration], source: &str, findings: &mut Vec<AuditFinding>) {
    for decl in declarations {
        if let Declaration::MlogServer(srv) = decl {
            let has_post = srv.routes.iter()
                .any(|r| r.method == "POST" || r.method == "PUT" || r.method == "DELETE");
            let has_csrf = srv.middleware.iter().any(|m| m == "csrf");

            if has_post && has_csrf {
                findings.push(AuditFinding {
                    severity: Severity::Info,
                    check_id: "CSRF",
                    line: find_line(source, "mlogserver"),
                    message: "server has csrf middleware \u{2713}".to_string(),
                });
            } else if has_post && !has_csrf {
                findings.push(AuditFinding {
                    severity: Severity::Warning,
                    check_id: "CSRF",
                    line: find_line(source, "mlogserver"),
                    message: "POST routes without CSRF middleware — recommend adding 'csrf'".to_string(),
                });
            }
        }
    }
}

// ── Check: HTML_INJECTION — LLM output in respond() without template ──

fn check_html_injection(declarations: &[Declaration], source: &str, findings: &mut Vec<AuditFinding>) {
    fn check_respond_for_html(expr: &Expr, tracker: &TaintTracker, source: &str, findings: &mut Vec<AuditFinding>) {
        if let Expr::FnCall(fn_name, args) = expr {
            if fn_name == "respond" || fn_name == "respond_html" {
                for arg in args {
                    if let Expr::Ident(var) = arg {
                        if tracker.get_taint(var) == Some(TaintKind::LlmOutput) {
                            let line = find_line(source, "respond");
                            findings.push(AuditFinding {
                                severity: Severity::Warning,
                                check_id: "HTML_INJECTION",
                                line,
                                message: "LLM output passed to respond() — use template/render for XSS safety".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    /// Analyze a list of statements for LLM→respond taint flow.
    fn analyze_scope(stmts: &[Statement], source: &str, findings: &mut Vec<AuditFinding>) {
        let mut tracker = TaintTracker::new();

        fn process_stmt(stmt: &Statement, tracker: &mut TaintTracker, source: &str, findings: &mut Vec<AuditFinding>) {
            match stmt {
                Statement::LetBinding { name, value, mutable: _ } => {
                    // Check if this let-binding calls respond() with tainted args
                    check_respond_for_html(value, tracker, source, findings);
                    // Track taint sources
                    if let Expr::FnCall(fn_name, _) = value {
                        if fn_name == "call_llm" || fn_name == "call_claude" {
                            tracker.taint(name, TaintKind::LlmOutput);
                        } else if fn_name == "env" {
                            tracker.taint(name, TaintKind::Secret);
                        } else if fn_name == "render" || fn_name == "escape_html" {
                            tracker.taint(name, TaintKind::Sanitized);
                        } else if fn_name == "form_data" || fn_name == "json_body" || fn_name == "query_param" {
                            tracker.taint(name, TaintKind::UserInput);
                        }
                    }
                }
                Statement::Assign { name, value } => {
                    if let Expr::FnCall(fn_name, _) = value {
                        if fn_name == "render" || fn_name == "escape_html" {
                            tracker.taint(name, TaintKind::Sanitized);
                        }
                    }
                }
                Statement::ExprStmt(expr) => {
                    check_respond_for_html(expr, tracker, source, findings);
                }
                Statement::Return(expr) => {
                    check_respond_for_html(expr, tracker, source, findings);
                }
                Statement::Each { body, .. } => {
                    for s in body { process_stmt(s, tracker, source, findings); }
                }
                Statement::While { body, .. } => {
                    for s in body { process_stmt(s, tracker, source, findings); }
                }
                Statement::IfElseBlock { then_body, else_ifs, else_body, .. } => {
                    for s in then_body { process_stmt(s, tracker, source, findings); }
                    for (_, body) in else_ifs { for s in body { process_stmt(s, tracker, source, findings); } }
                    if let Some(body) = else_body { for s in body { process_stmt(s, tracker, source, findings); } }
                }
                Statement::IfThen(_, body) => {
                    for s in body { process_stmt(s, tracker, source, findings); }
                }
                _ => {}
            }
        }

        for stmt in stmts {
            process_stmt(stmt, &mut tracker, source, findings);
        }
    }

    for decl in declarations {
        match decl {
            Declaration::MlogServer(srv) => {
                for route in &srv.routes {
                    analyze_scope(&route.body, source, findings);
                }
            }
            Declaration::Pattern(p) => analyze_scope(&p.body, source, findings),
            Declaration::Tool(t) => {
                for m in &t.methods {
                    analyze_scope(&m.body, source, findings);
                }
            }
            Declaration::Hook(h) => analyze_scope(&h.body, source, findings),
            _ => {}
        }
    }
}

// ── Check: SECRET_LEAK — env() result passed to respond/http_post/write_file

fn check_secret_leak(declarations: &[Declaration], source: &str, findings: &mut Vec<AuditFinding>) {
    /// Sink functions that must not receive secrets.
    /// Note: http_post/send_message are NOT sinks — they are intentional
    /// API call points where passing auth tokens is expected.
    const SINK_FUNCTIONS: &[&str] = &["respond", "respond_html", "write_file"];

    fn is_sink(name: &str) -> bool {
        SINK_FUNCTIONS.contains(&name)
    }

    fn analyze_scope(stmts: &[Statement], source: &str, findings: &mut Vec<AuditFinding>) {
        let mut tracker = TaintTracker::new();

        fn process_stmt(stmt: &Statement, tracker: &mut TaintTracker, source: &str, findings: &mut Vec<AuditFinding>) {
            match stmt {
                Statement::LetBinding { name, value, mutable: _ } => {
                    // Check if this let-binding calls a sink function with tainted args
                    check_expr_for_leak(value, tracker, source, findings);
                    // Track taint sources
                    if let Expr::FnCall(fn_name, _) = value {
                        if fn_name == "env" {
                            tracker.taint(name, TaintKind::Secret);
                        }
                    }
                }
                Statement::Assign { name, value } => {
                    // Clear taint if variable is reassigned to a literal
                    if let Expr::StringLit(_) = value {
                        tracker.untaint(name);
                    }
                }
                Statement::ExprStmt(expr) => {
                    check_expr_for_leak(expr, tracker, source, findings);
                }
                Statement::Return(expr) => {
                    check_expr_for_leak(expr, tracker, source, findings);
                }
                Statement::Each { body, .. } => {
                    for s in body { process_stmt(s, tracker, source, findings); }
                }
                Statement::While { body, .. } => {
                    for s in body { process_stmt(s, tracker, source, findings); }
                }
                Statement::IfElseBlock { then_body, else_ifs, else_body, .. } => {
                    for s in then_body { process_stmt(s, tracker, source, findings); }
                    for (_, body) in else_ifs { for s in body { process_stmt(s, tracker, source, findings); } }
                    if let Some(body) = else_body { for s in body { process_stmt(s, tracker, source, findings); } }
                }
                Statement::IfThen(_, body) => {
                    for s in body { process_stmt(s, tracker, source, findings); }
                }
                _ => {}
            }
        }

        fn check_expr_for_leak(expr: &Expr, tracker: &TaintTracker, source: &str, findings: &mut Vec<AuditFinding>) {
            if let Expr::FnCall(fn_name, args) = expr {
                if is_sink(fn_name) {
                    for arg in args {
                        if let Expr::Ident(var) = arg {
                            if tracker.get_taint(var) == Some(TaintKind::Secret) {
                                let line = find_line(source, fn_name);
                                findings.push(AuditFinding {
                                    severity: Severity::Error,
                                    check_id: "SECRET_LEAK",
                                    line,
                                    message: format!("secret may be leaked — env() value passed to {}()", fn_name),
                                });
                            }
                        }
                    }
                }
            }
        }

        for stmt in stmts {
            process_stmt(stmt, &mut tracker, source, findings);
        }
    }

    for decl in declarations {
        match decl {
            Declaration::MlogServer(srv) => {
                for route in &srv.routes { analyze_scope(&route.body, source, findings); }
            }
            Declaration::Pattern(p) => analyze_scope(&p.body, source, findings),
            Declaration::Tool(t) => {
                for m in &t.methods { analyze_scope(&m.body, source, findings); }
            }
            Declaration::Hook(h) => analyze_scope(&h.body, source, findings),
            _ => {}
        }
    }
}

// ── Check: OPEN_REDIRECT — respond() with user-controlled URL ────────

fn check_open_redirect(declarations: &[Declaration], source: &str, findings: &mut Vec<AuditFinding>) {
    fn check_expr_for_redirect(expr: &Expr, tracker: &TaintTracker, source: &str, findings: &mut Vec<AuditFinding>) {
        if let Expr::FnCall(fn_name, args) = expr {
            // Only flag respond_html for open redirect (HTML can set Location header)
            if fn_name == "respond_html" {
                for arg in args {
                    if let Expr::Ident(var) = arg {
                        if tracker.get_taint(var) == Some(TaintKind::UserInput) {
                            let line = find_line(source, "respond_html");
                            findings.push(AuditFinding {
                                severity: Severity::Warning,
                                check_id: "OPEN_REDIRECT",
                                line,
                                message: "possible open redirect — respond_html() with user-controlled input".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    fn analyze_scope(stmts: &[Statement], source: &str, findings: &mut Vec<AuditFinding>) {
        let mut tracker = TaintTracker::new();

        fn process_stmt(stmt: &Statement, tracker: &mut TaintTracker, source: &str, findings: &mut Vec<AuditFinding>) {
            match stmt {
                Statement::LetBinding { name, value, mutable: _ } => {
                    // Check if this let-binding calls respond() with tainted args
                    check_expr_for_redirect(value, tracker, source, findings);
                    // Track taint sources
                    if let Expr::FnCall(fn_name, _) = value {
                        if fn_name == "query_param" || fn_name == "form_data" || fn_name == "json_body" {
                            tracker.taint(name, TaintKind::UserInput);
                        }
                    }
                }
                Statement::Assign { name, value } => {
                    // Clear taint if variable is reassigned to a literal
                    if let Expr::StringLit(_) = value {
                        tracker.untaint(name);
                    }
                }
                Statement::ExprStmt(expr) => {
                    check_expr_for_redirect(expr, tracker, source, findings);
                }
                Statement::Return(expr) => {
                    check_expr_for_redirect(expr, tracker, source, findings);
                }
                Statement::Each { body, .. } => {
                    for s in body { process_stmt(s, tracker, source, findings); }
                }
                Statement::While { body, .. } => {
                    for s in body { process_stmt(s, tracker, source, findings); }
                }
                Statement::IfElseBlock { then_body, else_ifs, else_body, .. } => {
                    for s in then_body { process_stmt(s, tracker, source, findings); }
                    for (_, body) in else_ifs { for s in body { process_stmt(s, tracker, source, findings); } }
                    if let Some(body) = else_body { for s in body { process_stmt(s, tracker, source, findings); } }
                }
                Statement::IfThen(_, body) => {
                    for s in body { process_stmt(s, tracker, source, findings); }
                }
                _ => {}
            }
        }

        for stmt in stmts {
            process_stmt(stmt, &mut tracker, source, findings);
        }
    }

    for decl in declarations {
        match decl {
            Declaration::MlogServer(srv) => {
                for route in &srv.routes { analyze_scope(&route.body, source, findings); }
            }
            Declaration::Pattern(p) => analyze_scope(&p.body, source, findings),
            Declaration::Tool(t) => {
                for m in &t.methods { analyze_scope(&m.body, source, findings); }
            }
            Declaration::Hook(h) => analyze_scope(&h.body, source, findings),
            _ => {}
        }
    }
}

// ── Public API ───────────────────────────────────────────────────────

/// Perform static security analysis on a .mlog source string.
/// Returns an AuditResult with findings, or an error if parsing fails.
pub fn audit_program(source: &str) -> Result<AuditResult, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;

    let mut findings: Vec<AuditFinding> = Vec::new();

    // Run all security checks
    check_secrets(&declarations, source, &mut findings);
    check_sql_dynamic(&declarations, source, &mut findings);
    check_sandbox_coverage(&declarations, source, &mut findings);
    check_rate_limit(&declarations, source, &mut findings);
    check_csrf(&declarations, source, &mut findings);
    check_html_injection(&declarations, source, &mut findings);
    check_secret_leak(&declarations, source, &mut findings);
    check_open_redirect(&declarations, source, &mut findings);

    // Sort findings by line number for deterministic output
    findings.sort_by_key(|f| (f.line, f.check_id));

    Ok(AuditResult { findings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast;

    #[test]
    fn test_clean_program() {
        let source = r#"
            entity greeting: String = "Hello"
            pattern SayHello(text: String) -> String { return text }
            flow Main { input: String = greeting -> SayHello -> output }
        "#;
        let result = audit_program(source).unwrap();
        assert_eq!(result.error_count(), 0);
        assert_eq!(result.warning_count(), 0);
        assert_eq!(result.exit_code(), 0);
    }

    #[test]
    fn test_hardcoded_secret() {
        let source = r#"
            pattern Init() -> String {
                let key = "sk-ant-api03-very-long-string-here-abcdef1234567890"
                return key
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(result.warning_count() > 0);
        assert!(result.findings.iter().any(|f| f.check_id == "SECRETS"));
    }

    #[test]
    fn test_env_is_ok() {
        let source = r#"
            pattern Init() -> String {
                let key = env("API_KEY")
                return key
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(result.findings.iter().all(|f| f.check_id != "SECRETS"));
    }

    #[test]
    fn test_adapt_without_sandbox() {
        let source = r#"
            learnable pattern Classify(text: String) -> Category {
                prompt: "Classify"
            }
            adapt Classify add_example("input", "output")
        "#;
        let result = audit_program(source).unwrap();
        assert!(result.findings.iter().any(|f| f.check_id == "SANDBOX_COVERAGE"));
    }

    #[test]
    fn test_mutate_without_sandbox() {
        let source = r#"
            learnable pattern Classify(text: String) -> Category {
                prompt: "Classify"
            }
            mutate Classify { add_example("in", "out") }
        "#;
        let result = audit_program(source).unwrap();
        assert!(result.findings.iter().any(|f| f.check_id == "SANDBOX_COVERAGE"));
    }

    #[test]
    fn test_adapt_with_sandbox_ok() {
        let source = r#"
            learnable pattern Classify(text: String) -> Category {
                prompt: "Classify"
            }
            sandbox safe { allowed: [Classify], forbidden: [], timeout: 30 }
            adapt Classify add_example("input", "output")
        "#;
        let result = audit_program(source).unwrap();
        assert!(!result.findings.iter().any(|f| f.check_id == "SANDBOX_COVERAGE"));
    }

    #[test]
    fn test_rate_limit_check_direct() {
        // Test check_rate_limit by constructing a MlogServerDecl directly
        let srv = ast::MlogServerDecl {
            port: 8080,
            middleware: vec!["session".to_string(), "csrf".to_string(), "security_headers".to_string()],
            routes: vec![],
        };
        let source = "mlogserver { port: 8080 middleware: [session, csrf, security_headers] }";
        let mut findings: Vec<AuditFinding> = Vec::new();
        check_rate_limit(&[Declaration::MlogServer(srv)], source, &mut findings);
        assert!(findings.iter().any(|f| f.check_id == "RATE_LIMIT" && f.severity == Severity::Warning));
    }

    #[test]
    fn test_rate_limit_with_middleware_direct() {
        let srv = ast::MlogServerDecl {
            port: 8080,
            middleware: vec!["rate_limit".to_string()],
            routes: vec![],
        };
        let source = "mlogserver { middleware: [rate_limit] }";
        let mut findings: Vec<AuditFinding> = Vec::new();
        check_rate_limit(&[Declaration::MlogServer(srv)], source, &mut findings);
        assert!(findings.iter().any(|f| f.check_id == "RATE_LIMIT" && f.severity == Severity::Info));
    }

    #[test]
    fn test_csrf_post_without_csrf_direct() {
        let srv = ast::MlogServerDecl {
            port: 8080,
            middleware: vec!["session".to_string()],
            routes: vec![ast::RouteDecl {
                path: "/login".to_string(),
                method: "POST".to_string(),
                requires: vec![],
                body: vec![],
            }],
        };
        let source = "mlogserver { middleware: [session] }";
        let mut findings: Vec<AuditFinding> = Vec::new();
        check_csrf(&[Declaration::MlogServer(srv)], source, &mut findings);
        assert!(findings.iter().any(|f| f.check_id == "CSRF" && f.severity == Severity::Warning));
    }

    #[test]
    fn test_csrf_post_with_csrf_direct() {
        let srv = ast::MlogServerDecl {
            port: 8080,
            middleware: vec!["csrf".to_string()],
            routes: vec![ast::RouteDecl {
                path: "/login".to_string(),
                method: "POST".to_string(),
                requires: vec![],
                body: vec![],
            }],
        };
        let source = "mlogserver { middleware: [csrf] }";
        let mut findings: Vec<AuditFinding> = Vec::new();
        check_csrf(&[Declaration::MlogServer(srv)], source, &mut findings);
        assert!(findings.iter().any(|f| f.check_id == "CSRF" && f.severity == Severity::Info));
    }

    #[test]
    fn test_query_literal_sql_ok() {
        let source = r#"
            pattern GetUsers() -> String {
                let result = query("SELECT * FROM users")
                return result
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(result.findings.iter().any(|f| f.check_id == "SQL_DYNAMIC" && f.severity == Severity::Info));
        assert!(!result.findings.iter().any(|f| f.check_id == "SQL_DYNAMIC" && f.severity == Severity::Error));
    }

    #[test]
    fn test_query_dynamic_sql_error() {
        let source = r#"
            pattern GetUsers(table: String) -> String {
                let sql = "SELECT * FROM " + table
                let result = query(sql)
                return result
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(result.findings.iter().any(|f| f.check_id == "SQL_DYNAMIC" && f.severity == Severity::Error));
    }

    #[test]
    fn test_secret_leak_to_respond() {
        let source = r#"
            pattern LeakSecret() -> String {
                let api_key = env("API_KEY")
                let resp = respond("200 OK", api_key)
                return resp
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(result.findings.iter().any(|f| f.check_id == "SECRET_LEAK" && f.severity == Severity::Error));
    }

    #[test]
    fn test_secret_leak_to_http_post() {
        let source = r#"
            pattern LeakPost() -> String {
                let token = env("AUTH_TOKEN")
                let resp = http_post("https://api.example.com", token, "application/json")
                return resp
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(result.findings.iter().any(|f| f.check_id == "SECRET_LEAK" && f.severity == Severity::Error));
    }

    #[test]
    fn test_llm_to_respond_warning() {
        let source = r#"
            pattern LlmRespond() -> String {
                let result = call_llm("Tell me a joke")
                let resp = respond("200 OK", result)
                return resp
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(result.findings.iter().any(|f| f.check_id == "HTML_INJECTION" && f.severity == Severity::Warning));
    }

    #[test]
    fn test_llm_via_template_ok() {
        let source = r#"
            template Safe(html: String) -> Html {
                <div>{{ html }}</div>
            }
            pattern SafeLlm() -> String {
                let result = call_llm("Tell me a joke")
                let safe = render("Safe", "html", result)
                let resp = respond("200 OK", safe)
                return resp
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(!result.findings.iter().any(|f| f.check_id == "HTML_INJECTION" && f.severity == Severity::Warning));
    }

    #[test]
    fn test_format_output() {
        // Construct MlogServerDecl directly to test taint + server checks together
        let srv = ast::MlogServerDecl {
            port: 8080,
            middleware: vec!["session".to_string()],
            routes: vec![ast::RouteDecl {
                path: "/data".to_string(),
                method: "POST".to_string(),
                requires: vec![],
                body: vec![
                    Statement::LetBinding { name: "api_key".to_string(), value: Expr::FnCall("env".to_string(), vec![Expr::StringLit("KEY".to_string())]), mutable: false },
                    Statement::LetBinding { name: "result".to_string(), value: Expr::FnCall("call_llm".to_string(), vec![Expr::StringLit("summarize".to_string())]), mutable: false },
                    Statement::LetBinding { name: "resp".to_string(), value: Expr::FnCall("respond".to_string(), vec![Expr::StringLit("200 OK".to_string()), Expr::Ident("result".to_string())]), mutable: false },
                    Statement::Return(Expr::Ident("resp".to_string())),
                ],
            }],
        };
        let mut findings: Vec<AuditFinding> = Vec::new();
        check_rate_limit(&[Declaration::MlogServer(srv.clone())], "mlogserver", &mut findings);
        check_csrf(&[Declaration::MlogServer(srv.clone())], "mlogserver", &mut findings);
        check_html_injection(&[Declaration::MlogServer(srv.clone())], "mlogserver", &mut findings);
        check_secret_leak(&[Declaration::MlogServer(srv)], "mlogserver", &mut findings);

        assert!(!findings.is_empty(), "should have findings from server+route analysis");
        let result = AuditResult { findings };
        let formatted = result.format();
        assert!(formatted.contains("Summary:"));
    }

    #[test]
    fn test_exit_code_clean() {
        let source = r#"
            entity greeting: String = "Hello"
        "#;
        let result = audit_program(source).unwrap();
        assert_eq!(result.exit_code(), 0);
    }

    #[test]
    fn test_exit_code_errors() {
        let source = r#"
            pattern Leak() -> String {
                let key = env("KEY")
                let r = respond("200 OK", key)
                return r
            }
        "#;
        let result = audit_program(source).unwrap();
        assert_eq!(result.exit_code(), 1);
    }

    #[test]
    fn test_exit_code_warnings_only() {
        let source = r#"
            learnable pattern Foo(x: String) -> String { prompt: "test" }
            adapt Foo add_example("a", "b")
        "#;
        let result = audit_program(source).unwrap();
        assert_eq!(result.exit_code(), 2);
    }
}
