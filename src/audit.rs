// ── Static security analysis for METALOGOS (.mlog) programs ─────
// ADR-0057: `mlog audit <file>` — analyzes without executing.
// Checks: SECRETS, HTML_INJECTION, SQL_DYNAMIC, SANDBOX_COVERAGE,
//         RATE_LIMIT, CSRF, SECRET_LEAK, OPEN_REDIRECT,
//         TAINT_PERSISTENCE, TAINT_PASSTHROUGH.

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
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count()
    }
    pub fn warning_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .count()
    }
    pub fn info_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Info)
            .count()
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
        if ec == 1 {
            parts.push("1 error".to_string());
        } else if ec > 1 {
            parts.push(format!("{} errors", ec));
        }
        if wc == 1 {
            parts.push("1 warning".to_string());
        } else if wc > 1 {
            parts.push(format!("{} warnings", wc));
        }
        if pc == 1 {
            parts.push("1 passed".to_string());
        } else if pc > 1 {
            parts.push(format!("{} passed", pc));
        }
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
        Self {
            tainted: HashMap::new(),
        }
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

/// Extract taint kind from an expression, considering variable references
/// and function call arguments. Returns None for literals and unknown expressions.
///
/// Propagation rules:
/// - `Expr::Ident { name: var, span: Span::unknown() }` → returns var's taint
/// - `Expr::FnCall { name: "render"| "escape_html", span: Span::unknown() }` → Sanitized (overrides args)
/// - `Expr::FnCall { args: args, span: Span::unknown() }` → propagate first non-Sanitized arg taint
/// - `Expr::BinaryOp { op: left, right: right, span: Span::unknown() }` → propagate from either side
/// - Literals → None (clean)
fn get_expr_taint(expr: &Expr, tracker: &TaintTracker) -> Option<TaintKind> {
    match expr {
        Expr::Ident { name: var, .. } => tracker.get_taint(var),
        Expr::FnCall {
            name: fn_name,
            args,
            ..
        } => {
            // Sanitizers override argument taint
            if fn_name == "render" || fn_name == "escape_html" {
                return Some(TaintKind::Sanitized);
            }
            // env() is a secret source even with no tainted args
            if fn_name == "env" {
                return Some(TaintKind::Secret);
            }
            // Propagate taint from first tainted argument
            for arg in args {
                if let Some(taint) = get_expr_taint(arg, tracker) {
                    if taint != TaintKind::Sanitized {
                        return Some(taint);
                    }
                }
            }
            None
        }
        Expr::BinaryOp { left, right, .. } => {
            get_expr_taint(left, tracker).or_else(|| get_expr_taint(right, tracker))
        }
        Expr::FieldAccess { object: obj, .. } => get_expr_taint(obj, tracker),
        Expr::IndexAccess { index, .. } => get_expr_taint(index, tracker),
        // Literals are always clean
        Expr::StringLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::List { .. }
        | Expr::StructLit { .. } => None,
        Expr::IfElse {
            then_branch,
            else_branch,
            ..
        } => get_expr_taint(then_branch, tracker).or_else(|| get_expr_taint(else_branch, tracker)),
        // For other complex expressions, conservatively return None
        _ => None,
    }
}

/// Determine the taint kind for a binding's RHS expression.
/// Checks direct function call sources first, then falls back to
/// expression-level taint propagation.
fn binding_taint(value: &Expr, tracker: &TaintTracker) -> Option<TaintKind> {
    if let Expr::FnCall { name: fn_name, .. } = value {
        match fn_name.as_str() {
            "call_llm" | "call_claude" => return Some(TaintKind::LlmOutput),
            "env" => return Some(TaintKind::Secret),
            "render" | "escape_html" => return Some(TaintKind::Sanitized),
            "form_data" | "json_body" | "query_param" => return Some(TaintKind::UserInput),
            _ => {}
        }
    }
    get_expr_taint(value, tracker)
}

/// Check whether an expression carries LLM-output taint.
/// Handles both variable references (via tracker) and direct LLM
/// function calls (call_llm / call_claude) without an intermediate
/// variable binding. Single-level nesting only — interprocedural
/// analysis is a separate, larger task.
fn expr_is_llm_tainted(expr: &Expr, tracker: &TaintTracker) -> bool {
    match expr {
        Expr::Ident { name, .. } => tracker.get_taint(name) == Some(TaintKind::LlmOutput),
        Expr::FnCall { name, .. } => is_llm_source(name),
        _ => false,
    }
}

/// Returns true if the function name is a known LLM output source.
fn is_llm_source(name: &str) -> bool {
    matches!(name, "call_llm" | "call_claude")
}

/// Check whether an expression carries user-input taint.
/// Handles both variable references (via tracker) and direct
/// user-input function calls (query_param, form_data, json_body)
/// without an intermediate variable binding.
/// Наряд №140: mirrors expr_is_llm_tainted but for UserInput source.
fn expr_is_user_input_tainted(expr: &Expr, tracker: &TaintTracker) -> bool {
    match expr {
        Expr::Ident { name, .. } => tracker.get_taint(name) == Some(TaintKind::UserInput),
        Expr::FnCall { name, .. } => is_user_input_source(name),
        _ => false,
    }
}

/// Returns true if the function name is a known user-input source.
fn is_user_input_source(name: &str) -> bool {
    matches!(name, "query_param" | "form_data" | "json_body")
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
    // original 18 — do not modify
    "sk-",
    "sk_",
    "skant",
    "sk-ant",
    "api_key",
    "apikey",
    "API_KEY",
    "secret_key",
    "secretkey",
    "SECRET_KEY",
    "access_token",
    "accesstoken",
    "auth_token",
    "authtoken",
    "private_key",
    "privatekey",
    "token=",
    "TOKEN=",
    // real token formats — naryad #102
    "ghp_",
    "gho_",
    "ghs_",
    "github_pat_", // GitHub (classic + fine-grained)
    "xoxb-",
    "xoxp-",
    "xoxa-",      // Slack
    "glpat-",     // GitLab
    "AIza",       // Google API key
    "-----BEGIN", // PEM private key
];

/// Patterns with their own, lower length threshold — inherently distinctive
/// formats that don't need the generic 30-char guard.
const SHORT_SECRET_PATTERNS: &[(&str, usize)] = &[
    ("AKIA", 20), // AWS access key ID: AKIA + 16 alphanumeric
    ("ASIA", 20), // AWS temporary access key ID: ASIA + 16 alphanumeric
];

/// Minimum string length to be considered a possible secret (generic guard).
const SECRET_MIN_LENGTH: usize = 30;

/// Check if a string literal looks like a hardcoded secret.
fn looks_like_secret(s: &str) -> bool {
    let lower = s.to_lowercase();

    // Check short patterns first — they have their own length thresholds
    for (pattern, min_len) in SHORT_SECRET_PATTERNS {
        if s.len() >= *min_len && lower.contains(&pattern.to_lowercase()) {
            return true;
        }
    }

    if s.len() < SECRET_MIN_LENGTH {
        return false;
    }
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
            Expr::StringLit { value: s, .. } => acc.push(s),
            Expr::FnCall { name, args, .. } => {
                // Skip env() calls — they are the OK way to get secrets
                if name != "env" {
                    for arg in args {
                        walk_string_exprs(arg, acc);
                    }
                }
            }
            Expr::QualifiedCall { args, .. } => {
                for arg in args {
                    walk_string_exprs(arg, acc);
                }
            }
            Expr::BinaryOp {
                left: l, right: r, ..
            } => {
                walk_string_exprs(l, acc);
                walk_string_exprs(r, acc);
            }
            Expr::IfElse {
                condition: c,
                then_branch: t,
                else_branch: e,
                ..
            } => {
                walk_string_exprs(c, acc);
                walk_string_exprs(t, acc);
                walk_string_exprs(e, acc);
            }
            Expr::List { items, .. } => {
                for item in items {
                    walk_string_exprs(item, acc);
                }
            }
            Expr::FieldAccess { object: inner, .. } => walk_string_exprs(inner, acc),
            Expr::IndexAccess {
                object: inner,
                index: idx,
                ..
            } => {
                walk_string_exprs(inner, acc);
                walk_string_exprs(idx, acc);
            }
            _ => {}
        }
    }
    fn walk_string_stmts<'a>(stmts: &'a [Statement], acc: &mut Vec<&'a String>) {
        for stmt in stmts {
            match stmt {
                Statement::LetBinding { value, .. } => walk_string_exprs(value, acc),
                Statement::Assign { value, .. } => walk_string_exprs(value, acc),
                Statement::ExprStmt { expr, .. } => walk_string_exprs(expr, acc),
                Statement::Return { value: expr, .. } => walk_string_exprs(expr, acc),
                Statement::Each { body, .. } => walk_string_stmts(body, acc),
                Statement::While { body, .. } => walk_string_stmts(body, acc),
                Statement::IfElseBlock {
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    walk_string_stmts(then_body, acc);
                    for (_, body) in else_ifs {
                        walk_string_stmts(body, acc);
                    }
                    if let Some(body) = else_body {
                        walk_string_stmts(body, acc);
                    }
                }
                Statement::IfThen { body, .. } => walk_string_stmts(body, acc),
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
            Declaration::Test(_) => {}
            _ => {}
        }

        for s in strings {
            if looks_like_secret(s) {
                // Find a snippet to locate in source
                let snippet = crate::util::safe_byte_truncate(s, 20);
                let line = find_line(source, snippet);
                findings.push(AuditFinding {
                    severity: Severity::Warning,
                    check_id: "SECRETS",
                    line,
                    message: format!(
                        "possible hardcoded secret: string matches secret pattern (length={})",
                        s.len()
                    ),
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

    fn check_stmts(
        stmts: &[Statement],
        source: &str,
        findings: &mut Vec<AuditFinding>,
        all_literal: &mut bool,
        literal_count: &mut usize,
    ) {
        fn walk_query(
            expr: &Expr,
            source: &str,
            findings: &mut Vec<AuditFinding>,
            all_literal: &mut bool,
            literal_count: &mut usize,
        ) {
            if let Expr::FnCall { name, args, .. } = expr {
                if name == "query" {
                    if let Some(arg) = args.first() {
                        match arg {
                            Expr::StringLit { .. } => {
                                *literal_count += 1;
                            }
                            _ => {
                                *all_literal = false;
                                let snippet = match arg {
                                    Expr::Ident { name: id, .. } => id.clone(),
                                    Expr::BinaryOp { .. } => "dynamic expression".to_string(),
                                    Expr::FnCall { name: n, .. } => format!("{}()", n),
                                    _ => "non-literal".to_string(),
                                };
                                let line = find_line(source, &snippet);
                                findings.push(AuditFinding {
                                    severity: Severity::Error,
                                    check_id: "SQL_DYNAMIC",
                                    line,
                                    message: format!(
                                        "SQL injection risk — query() with non-literal SQL ({})",
                                        snippet
                                    ),
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
                        for arg in args {
                            walk_query(arg, source, findings, all_literal, literal_count);
                        }
                    }
                    Expr::BinaryOp {
                        left: l, right: r, ..
                    } => {
                        walk_query(l, source, findings, all_literal, literal_count);
                        walk_query(r, source, findings, all_literal, literal_count);
                    }
                    _ => {}
                }
            }
        }
        fn walk_stmt(
            stmt: &Statement,
            source: &str,
            findings: &mut Vec<AuditFinding>,
            all_literal: &mut bool,
            literal_count: &mut usize,
        ) {
            match stmt {
                Statement::LetBinding { value, .. } => {
                    walk_query(value, source, findings, all_literal, literal_count)
                }
                Statement::Assign { value, .. } => {
                    walk_query(value, source, findings, all_literal, literal_count)
                }
                Statement::ExprStmt { expr, .. } => {
                    walk_query(expr, source, findings, all_literal, literal_count)
                }
                Statement::Return { value: expr, .. } => {
                    walk_query(expr, source, findings, all_literal, literal_count)
                }
                Statement::Each { body, .. } => {
                    for s in body {
                        walk_stmt(s, source, findings, all_literal, literal_count);
                    }
                }
                Statement::While { body, .. } => {
                    for s in body {
                        walk_stmt(s, source, findings, all_literal, literal_count);
                    }
                }
                Statement::IfElseBlock {
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    for s in then_body {
                        walk_stmt(s, source, findings, all_literal, literal_count);
                    }
                    for (_, body) in else_ifs {
                        for s in body {
                            walk_stmt(s, source, findings, all_literal, literal_count);
                        }
                    }
                    if let Some(body) = else_body {
                        for s in body {
                            walk_stmt(s, source, findings, all_literal, literal_count);
                        }
                    }
                }
                Statement::IfThen { body, .. } => {
                    for s in body {
                        walk_stmt(s, source, findings, all_literal, literal_count);
                    }
                }
                _ => {}
            }
        }
        for stmt in stmts {
            walk_stmt(stmt, source, findings, all_literal, literal_count);
        }
    }

    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => check_stmts(
                &p.body,
                source,
                findings,
                &mut all_literal,
                &mut literal_count,
            ),
            Declaration::Tool(t) => {
                for m in &t.methods {
                    check_stmts(
                        &m.body,
                        source,
                        findings,
                        &mut all_literal,
                        &mut literal_count,
                    );
                }
            }
            Declaration::MlogServer(srv) => {
                for route in &srv.routes {
                    check_stmts(
                        &route.body,
                        source,
                        findings,
                        &mut all_literal,
                        &mut literal_count,
                    );
                }
            }
            Declaration::Hook(h) => check_stmts(
                &h.body,
                source,
                findings,
                &mut all_literal,
                &mut literal_count,
            ),
            Declaration::Test(_) => {}
            _ => {}
        }
    }

    if literal_count > 0 && all_literal {
        findings.push(AuditFinding {
            severity: Severity::Info,
            check_id: "SQL_DYNAMIC",
            line: 1,
            message: format!(
                "all {} query() calls use literal SQL \u{2713}",
                literal_count
            ),
        });
    }
}

// ── Check: SANDBOX_COVERAGE — adapt/mutate without sandbox ──────────

fn check_sandbox_coverage(
    declarations: &[Declaration],
    source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    let has_sandbox = declarations
        .iter()
        .any(|d| matches!(d, Declaration::Sandbox(_)));

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
                Declaration::Test(_) => {}
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
                    message: "no rate limiting — recommend adding 'rate_limit' middleware"
                        .to_string(),
                });
            }
        }
    }
}

// ── Check: CSRF — POST routes without csrf middleware ────────────────

fn check_csrf(declarations: &[Declaration], source: &str, findings: &mut Vec<AuditFinding>) {
    for decl in declarations {
        if let Declaration::MlogServer(srv) = decl {
            let has_post = srv
                .routes
                .iter()
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
                    message: "POST routes without CSRF middleware — recommend adding 'csrf'"
                        .to_string(),
                });
            }
        }
    }
}

// ── Check: HTML_INJECTION — LLM output in respond() without template ──

fn check_html_injection(
    declarations: &[Declaration],
    source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    fn check_respond_for_html(
        expr: &Expr,
        tracker: &TaintTracker,
        source: &str,
        findings: &mut Vec<AuditFinding>,
    ) {
        if let Expr::FnCall {
            name: fn_name,
            args,
            ..
        } = expr
        {
            if fn_name == "respond" || fn_name == "respond_html" {
                for arg in args {
                    // Наряд №123: catch both variable references
                    // (tracker) and direct nested LLM calls
                    // (e.g. respond(call_llm(...))).
                    if expr_is_llm_tainted(arg, tracker) {
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

    /// Analyze a list of statements for LLM→respond taint flow.
    fn analyze_scope(stmts: &[Statement], source: &str, findings: &mut Vec<AuditFinding>) {
        let mut tracker = TaintTracker::new();

        fn process_stmt(
            stmt: &Statement,
            tracker: &mut TaintTracker,
            source: &str,
            findings: &mut Vec<AuditFinding>,
        ) {
            match stmt {
                Statement::LetBinding { name, value, .. } => {
                    // Check if this let-binding calls respond() with tainted args
                    check_respond_for_html(value, tracker, source, findings);
                    // Propagate taint from expression (handles both direct
                    // function calls and variable references)
                    if let Some(taint) = binding_taint(value, tracker) {
                        tracker.taint(name, taint);
                    } else {
                        // Reassignment to a clean literal clears taint
                        tracker.untaint(name);
                    }
                }
                Statement::Assign { name, value, .. } => {
                    if let Some(taint) = binding_taint(value, tracker) {
                        tracker.taint(name, taint);
                    } else {
                        tracker.untaint(name);
                    }
                }
                Statement::ExprStmt { expr, .. } => {
                    check_respond_for_html(expr, tracker, source, findings);
                }
                Statement::Return { value: expr, .. } => {
                    check_respond_for_html(expr, tracker, source, findings);
                }
                Statement::Each { body, .. } => {
                    for s in body {
                        process_stmt(s, tracker, source, findings);
                    }
                }
                Statement::While { body, .. } => {
                    for s in body {
                        process_stmt(s, tracker, source, findings);
                    }
                }
                Statement::IfElseBlock {
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    for s in then_body {
                        process_stmt(s, tracker, source, findings);
                    }
                    for (_, body) in else_ifs {
                        for s in body {
                            process_stmt(s, tracker, source, findings);
                        }
                    }
                    if let Some(body) = else_body {
                        for s in body {
                            process_stmt(s, tracker, source, findings);
                        }
                    }
                }
                Statement::IfThen { body, .. } => {
                    for s in body {
                        process_stmt(s, tracker, source, findings);
                    }
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
            Declaration::Test(_) => {}
            _ => {}
        }
    }
}

// ── Check: SECRET_LEAK — env() result passed to respond/write_file sinks;
//   http_post: positional — body (arg 1) is a leak, headers (arg 3) is normal auth

fn check_secret_leak(declarations: &[Declaration], source: &str, findings: &mut Vec<AuditFinding>) {
    /// Sink functions that must not receive secrets.
    /// Note: http_post is NOT a blanket sink — passing secrets in headers
    /// (arg 3) is intentional (Bearer auth). Only body (arg 1) is flagged.
    /// send_message is also not a sink — it is an intentional API call point.
    const SINK_FUNCTIONS: &[&str] = &["respond", "respond_html", "write_file", "print"];

    fn is_sink(name: &str) -> bool {
        SINK_FUNCTIONS.contains(&name)
    }

    fn analyze_scope(stmts: &[Statement], source: &str, findings: &mut Vec<AuditFinding>) {
        let mut tracker = TaintTracker::new();

        fn process_stmt(
            stmt: &Statement,
            tracker: &mut TaintTracker,
            source: &str,
            findings: &mut Vec<AuditFinding>,
        ) {
            match stmt {
                Statement::LetBinding { name, value, .. } => {
                    // Check if this let-binding calls a sink function with tainted args
                    check_expr_for_leak(value, tracker, source, findings);
                    // Propagate taint from expression
                    if let Some(taint) = binding_taint(value, tracker) {
                        tracker.taint(name, taint);
                    } else {
                        tracker.untaint(name);
                    }
                }
                Statement::Assign { name, value, .. } => {
                    if let Some(taint) = binding_taint(value, tracker) {
                        tracker.taint(name, taint);
                    } else {
                        tracker.untaint(name);
                    }
                }
                Statement::ExprStmt { expr, .. } => {
                    check_expr_for_leak(expr, tracker, source, findings);
                }
                Statement::Return { value: expr, .. } => {
                    check_expr_for_leak(expr, tracker, source, findings);
                }
                Statement::Each { body, .. } => {
                    for s in body {
                        process_stmt(s, tracker, source, findings);
                    }
                }
                Statement::While { body, .. } => {
                    for s in body {
                        process_stmt(s, tracker, source, findings);
                    }
                }
                Statement::IfElseBlock {
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    for s in then_body {
                        process_stmt(s, tracker, source, findings);
                    }
                    for (_, body) in else_ifs {
                        for s in body {
                            process_stmt(s, tracker, source, findings);
                        }
                    }
                    if let Some(body) = else_body {
                        for s in body {
                            process_stmt(s, tracker, source, findings);
                        }
                    }
                }
                Statement::IfThen { body, .. } => {
                    for s in body {
                        process_stmt(s, tracker, source, findings);
                    }
                }
                _ => {}
            }
        }

        fn check_expr_for_leak(
            expr: &Expr,
            tracker: &TaintTracker,
            source: &str,
            findings: &mut Vec<AuditFinding>,
        ) {
            if let Expr::FnCall {
                name: fn_name,
                args,
                ..
            } = expr
            {
                if is_sink(fn_name) {
                    for arg in args {
                        // Ident + direct env()/tainted expr (e.g. print(env("X")))
                        if get_expr_taint(arg, tracker) == Some(TaintKind::Secret) {
                            let line = find_line(source, fn_name);
                            findings.push(AuditFinding {
                                severity: Severity::Error,
                                check_id: "SECRET_LEAK",
                                line,
                                message: format!(
                                    "secret may be leaked — env() value passed to {}()",
                                    fn_name
                                ),
                            });
                        }
                    }
                }
                // http_post: секрет в теле запроса (арг 1) — утечка.
                // Секрет в заголовках (арг 3) — штатная авторизация, НЕ флагируем.
                // Наряд №123: use get_expr_taint to catch both variable refs
                // and direct env() calls (e.g. http_post(u, env("K"), h)).
                if fn_name == "http_post" {
                    if let Some(arg) = args.get(1) {
                        if get_expr_taint(arg, tracker) == Some(TaintKind::Secret) {
                            let line = find_line(source, fn_name);
                            findings.push(AuditFinding {
                                severity: Severity::Error,
                                check_id: "SECRET_LEAK",
                                line,
                                message: "secret may be leaked \u{2014} env() value passed as http_post body"
                                    .to_string(),
                            });
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
            Declaration::Test(_) => {}
            _ => {}
        }
    }
}

// ── Check: OPEN_REDIRECT — respond() with user-controlled URL ────────

fn check_open_redirect(
    declarations: &[Declaration],
    source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    fn check_expr_for_redirect(
        expr: &Expr,
        tracker: &TaintTracker,
        source: &str,
        findings: &mut Vec<AuditFinding>,
    ) {
        if let Expr::FnCall {
            name: fn_name,
            args,
            ..
        } = expr
        {
            // Only flag respond_html for open redirect (HTML can set Location header)
            if fn_name == "respond_html" {
                for arg in args {
                    // Наряд №140: catch both variable references
                    // (tracker) and direct nested user-input calls
                    // (e.g. respond_html(query_param("url"))).
                    if expr_is_user_input_tainted(arg, tracker) {
                        let line = find_line(source, "respond_html");
                        findings.push(AuditFinding {
                            severity: Severity::Warning,
                            check_id: "OPEN_REDIRECT",
                            line,
                            message:
                                "possible open redirect — respond_html() with user-controlled input"
                                    .to_string(),
                        });
                    }
                }
            }
        }
    }

    fn analyze_scope(stmts: &[Statement], source: &str, findings: &mut Vec<AuditFinding>) {
        let mut tracker = TaintTracker::new();

        fn process_stmt(
            stmt: &Statement,
            tracker: &mut TaintTracker,
            source: &str,
            findings: &mut Vec<AuditFinding>,
        ) {
            match stmt {
                Statement::LetBinding { name, value, .. } => {
                    // Check if this let-binding calls respond() with tainted args
                    check_expr_for_redirect(value, tracker, source, findings);
                    // Propagate taint from expression
                    if let Some(taint) = binding_taint(value, tracker) {
                        tracker.taint(name, taint);
                    } else {
                        tracker.untaint(name);
                    }
                }
                Statement::Assign { name, value, .. } => {
                    if let Some(taint) = binding_taint(value, tracker) {
                        tracker.taint(name, taint);
                    } else {
                        tracker.untaint(name);
                    }
                }
                Statement::ExprStmt { expr, .. } => {
                    check_expr_for_redirect(expr, tracker, source, findings);
                }
                Statement::Return { value: expr, .. } => {
                    check_expr_for_redirect(expr, tracker, source, findings);
                }
                Statement::Each { body, .. } => {
                    for s in body {
                        process_stmt(s, tracker, source, findings);
                    }
                }
                Statement::While { body, .. } => {
                    for s in body {
                        process_stmt(s, tracker, source, findings);
                    }
                }
                Statement::IfElseBlock {
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    for s in then_body {
                        process_stmt(s, tracker, source, findings);
                    }
                    for (_, body) in else_ifs {
                        for s in body {
                            process_stmt(s, tracker, source, findings);
                        }
                    }
                    if let Some(body) = else_body {
                        for s in body {
                            process_stmt(s, tracker, source, findings);
                        }
                    }
                }
                Statement::IfThen { body, .. } => {
                    for s in body {
                        process_stmt(s, tracker, source, findings);
                    }
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
            Declaration::Test(_) => {}
            _ => {}
        }
    }
}

// ── Check: TAINT_PERSISTENCE — memorize(LLM-source) + recall in respond() ──
// Наряд #141: file-level heuristic — if a file both stores LLM output via
// `memorize` and passes `recall()` results to `respond`/`respond_html`,
// warn about potential taint through persistence. Does NOT track precise
// data-flow through the memory subsystem — honest warning, not a guarantee.

fn check_taint_persistence(
    declarations: &[Declaration],
    source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    /// Recursively check if an expression contains an LLM source call.
    fn expr_contains_llm_source(expr: &Expr) -> bool {
        match expr {
            Expr::FnCall { name, args, .. } => {
                if is_llm_source(name) {
                    return true;
                }
                args.iter().any(expr_contains_llm_source)
            }
            _ => false,
        }
    }

    // Step 1: Check if any `memorize` declaration stores an LLM-sourced value.
    let has_llm_memorize = declarations.iter().any(|d| {
        if let Declaration::Memorize(m) = d {
            expr_contains_llm_source(&m.value)
        } else {
            false
        }
    });

    if !has_llm_memorize {
        return;
    }

    // Step 2: Check if any scope uses both recall() and respond()/respond_html().
    // This covers both `respond(recall(...))` and `let ctx = recall(...); respond(..., ctx)`.
    fn scope_has_recall_and_respond(stmts: &[Statement]) -> bool {
        let mut has_recall = false;
        let mut has_respond = false;
        fn walk_expr(expr: &Expr, has_recall: &mut bool, has_respond: &mut bool) {
            if let Expr::FnCall { name, args, .. } = expr {
                if name == "recall" {
                    *has_recall = true;
                }
                if name == "respond" || name == "respond_html" {
                    *has_respond = true;
                }
                for arg in args {
                    walk_expr(arg, has_recall, has_respond);
                }
            }
        }
        fn walk_stmts(stmts: &[Statement], has_recall: &mut bool, has_respond: &mut bool) {
            for stmt in stmts {
                match stmt {
                    Statement::LetBinding { value, .. } => {
                        walk_expr(value, has_recall, has_respond)
                    }
                    Statement::Assign { value, .. } => walk_expr(value, has_recall, has_respond),
                    Statement::ExprStmt { expr, .. } => walk_expr(expr, has_recall, has_respond),
                    Statement::Return { value, .. } => walk_expr(value, has_recall, has_respond),
                    Statement::Each { body, .. } => walk_stmts(body, has_recall, has_respond),
                    Statement::EachWithIndex { body, .. } => {
                        walk_stmts(body, has_recall, has_respond)
                    }
                    Statement::While { body, .. } => walk_stmts(body, has_recall, has_respond),
                    Statement::IfElseBlock {
                        then_body,
                        else_ifs,
                        else_body,
                        ..
                    } => {
                        walk_stmts(then_body, has_recall, has_respond);
                        for (_, body) in else_ifs {
                            walk_stmts(body, has_recall, has_respond);
                        }
                        if let Some(body) = else_body {
                            walk_stmts(body, has_recall, has_respond);
                        }
                    }
                    Statement::IfThen { body, .. } => walk_stmts(body, has_recall, has_respond),
                    _ => {}
                }
                if *has_recall && *has_respond {
                    return; // early exit
                }
            }
        }
        walk_stmts(stmts, &mut has_recall, &mut has_respond);
        has_recall && has_respond
    }

    for decl in declarations {
        let has_both = match decl {
            Declaration::Pattern(p) => scope_has_recall_and_respond(&p.body),
            Declaration::Tool(t) => t
                .methods
                .iter()
                .any(|m| scope_has_recall_and_respond(&m.body)),
            Declaration::MlogServer(srv) => srv
                .routes
                .iter()
                .any(|r| scope_has_recall_and_respond(&r.body)),
            Declaration::Hook(h) => scope_has_recall_and_respond(&h.body),
            _ => false,
        };
        if has_both {
            let line = find_line(source, "recall");
            findings.push(AuditFinding {
                severity: Severity::Warning,
                check_id: "TAINT_PERSISTENCE",
                line,
                message: "potential taint through memorize/recall — LLM output was memorized in this file and recall() result may reach respond(); use render()/escape_html() on recalled data"
                    .to_string(),
            });
            // One warning per file is enough
            return;
        }
    }
}

// ── Check: TAINT_PASSTHROUGH_PATTERN — respond(Wrap(call_llm(...))) ────────
// Наряд #141: heuristic for trivial passthrough patterns. If a pattern
// has exactly one parameter and its only return is that parameter
// (trivial passthrough like `pattern Wrap(x) { return x }`), and it is
// called wrapping an LLM source in a respond/respond_html sink, warn.
// This is NOT full interprocedural analysis — it only catches the
// simplest case of indirection.

/// Check if a pattern is a trivial passthrough: exactly one parameter,
/// body is a single `return <param_name>`.
fn is_trivial_passthrough(p: &PatternDecl) -> bool {
    if p.params.len() != 1 {
        return false;
    }
    let param_name = &p.params[0].name;
    // Body must be exactly one statement: `return <param_name>`
    if p.body.len() != 1 {
        return false;
    }
    if let Statement::Return {
        value: Expr::Ident { name, .. },
        ..
    } = &p.body[0]
    {
        return name == param_name;
    }
    false
}

fn check_taint_passthrough_pattern(
    declarations: &[Declaration],
    source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    // Step 1: Collect names of all trivial passthrough patterns.
    let passthrough_names: std::collections::HashSet<String> = declarations
        .iter()
        .filter_map(|d| {
            if let Declaration::Pattern(p) = d {
                if is_trivial_passthrough(p) {
                    return Some(p.name.clone());
                }
            }
            None
        })
        .collect();

    if passthrough_names.is_empty() {
        return;
    }

    // Step 2: Check respond/respond_html calls whose arg is a passthrough
    // pattern call wrapping an LLM source.
    fn is_passthrough_with_llm(
        expr: &Expr,
        passthrough_names: &std::collections::HashSet<String>,
    ) -> bool {
        if let Expr::FnCall { name, args, .. } = expr {
            if passthrough_names.contains(name) {
                return args.iter().any(expr_contains_llm_source_nested);
            }
        }
        false
    }

    fn expr_contains_llm_source_nested(expr: &Expr) -> bool {
        match expr {
            Expr::FnCall { name, args, .. } => {
                if is_llm_source(name) {
                    return true;
                }
                args.iter().any(expr_contains_llm_source_nested)
            }
            _ => false,
        }
    }

    fn collect_stmt_exprs<'a>(stmts: &'a [Statement], acc: &mut Vec<&'a Expr>) {
        for stmt in stmts {
            match stmt {
                Statement::LetBinding { value, .. } => acc.push(value),
                Statement::Assign { value, .. } => acc.push(value),
                Statement::ExprStmt { expr, .. } => acc.push(expr),
                Statement::Return { value, .. } => acc.push(value),
                Statement::Each { body, .. } => collect_stmt_exprs(body, acc),
                Statement::EachWithIndex { body, .. } => collect_stmt_exprs(body, acc),
                Statement::While { body, .. } => collect_stmt_exprs(body, acc),
                Statement::IfElseBlock {
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    collect_stmt_exprs(then_body, acc);
                    for (_, body) in else_ifs {
                        collect_stmt_exprs(body, acc);
                    }
                    if let Some(body) = else_body {
                        collect_stmt_exprs(body, acc);
                    }
                }
                Statement::IfThen { body, .. } => collect_stmt_exprs(body, acc),
                _ => {}
            }
        }
    }

    for decl in declarations {
        let mut exprs: Vec<&Expr> = Vec::new();
        match decl {
            Declaration::Pattern(p) => collect_stmt_exprs(&p.body, &mut exprs),
            Declaration::Tool(t) => {
                for m in &t.methods {
                    collect_stmt_exprs(&m.body, &mut exprs);
                }
            }
            Declaration::MlogServer(srv) => {
                for route in &srv.routes {
                    collect_stmt_exprs(&route.body, &mut exprs);
                }
            }
            Declaration::Hook(h) => collect_stmt_exprs(&h.body, &mut exprs),
            _ => continue,
        }

        for expr in &exprs {
            if let Expr::FnCall { name, args, .. } = expr {
                if name == "respond" || name == "respond_html" {
                    for arg in args {
                        if is_passthrough_with_llm(arg, &passthrough_names) {
                            let line = find_line(source, name);
                            findings.push(AuditFinding {
                                severity: Severity::Warning,
                                check_id: "TAINT_PASSTHROUGH",
                                line,
                                message: format!(
                                    "LLM output passed to {}() via trivial passthrough pattern — use render()/escape_html() for XSS safety",
                                    name
                                ),
                            });
                        }
                    }
                }
            }
        }
    }
}

// ── Public API ───────────────────────────────────────────────────────

/// Run only Category A audit checks (compiler-enforced security invariants).
/// These are checks where a finding is never a legitimate false positive —
/// the code IS insecure if the check fires. Promoted from `mlog audit` to
/// `mlog check`/`mlog run`/`mlog serve`/`mlog compile` by Наряд №98.
///
/// Category A (Наряд №98):
///   - SQL_DYNAMIC: query() with non-literal SQL (SQL injection vector)
///   - SECRET_LEAK: env() result passed to sink (respond/write_file)
///   - HTML_INJECTION: LLM output to respond() without sanitization (XSS)
///
/// Category B (advisory, stays in `mlog audit` only):
///   - SECRETS: heuristic, can false-positive on error messages/doc strings
///   - SANDBOX_COVERAGE: cross-file context needed
///   - RATE_LIMIT: external infra can handle this
///   - CSRF: not needed for token-authenticated APIs
///   - OPEN_REDIRECT: custom validation not recognized
pub fn audit_category_a(declarations: &[Declaration], source: &str) -> Vec<AuditFinding> {
    let mut findings: Vec<AuditFinding> = Vec::new();
    check_sql_dynamic(declarations, source, &mut findings);
    check_secret_leak(declarations, source, &mut findings);
    check_html_injection(declarations, source, &mut findings);
    findings
}

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
    check_taint_persistence(&declarations, source, &mut findings);
    check_taint_passthrough_pattern(&declarations, source, &mut findings);

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
        assert!(result
            .findings
            .iter()
            .any(|f| f.check_id == "SANDBOX_COVERAGE"));
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
        assert!(result
            .findings
            .iter()
            .any(|f| f.check_id == "SANDBOX_COVERAGE"));
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
        assert!(!result
            .findings
            .iter()
            .any(|f| f.check_id == "SANDBOX_COVERAGE"));
    }

    #[test]
    fn test_rate_limit_check_direct() {
        // Test check_rate_limit by constructing a MlogServerDecl directly
        let srv = ast::MlogServerDecl {
            span: Span::unknown(),
            port: 8080,
            host: None,
            middleware: vec![
                "session".to_string(),
                "csrf".to_string(),
                "security_headers".to_string(),
            ],
            routes: vec![],
        };
        let source = "mlogserver { port: 8080 middleware: [session, csrf, security_headers] }";
        let mut findings: Vec<AuditFinding> = Vec::new();
        check_rate_limit(&[Declaration::MlogServer(srv)], source, &mut findings);
        assert!(findings
            .iter()
            .any(|f| f.check_id == "RATE_LIMIT" && f.severity == Severity::Warning));
    }

    #[test]
    fn test_rate_limit_with_middleware_direct() {
        let srv = ast::MlogServerDecl {
            span: Span::unknown(),
            port: 8080,
            host: None,
            middleware: vec!["rate_limit".to_string()],
            routes: vec![],
        };
        let source = "mlogserver { middleware: [rate_limit] }";
        let mut findings: Vec<AuditFinding> = Vec::new();
        check_rate_limit(&[Declaration::MlogServer(srv)], source, &mut findings);
        assert!(findings
            .iter()
            .any(|f| f.check_id == "RATE_LIMIT" && f.severity == Severity::Info));
    }

    #[test]
    fn test_csrf_post_without_csrf_direct() {
        let srv = ast::MlogServerDecl {
            span: Span::unknown(),
            port: 8080,
            host: None,
            middleware: vec!["session".to_string()],
            routes: vec![ast::RouteDecl {
                span: Span::unknown(),
                path: "/login".to_string(),
                method: "POST".to_string(),
                requires: vec![],
                body: vec![],
            }],
        };
        let source = "mlogserver { middleware: [session] }";
        let mut findings: Vec<AuditFinding> = Vec::new();
        check_csrf(&[Declaration::MlogServer(srv)], source, &mut findings);
        assert!(findings
            .iter()
            .any(|f| f.check_id == "CSRF" && f.severity == Severity::Warning));
    }

    #[test]
    fn test_csrf_post_with_csrf_direct() {
        let srv = ast::MlogServerDecl {
            span: Span::unknown(),
            port: 8080,
            host: None,
            middleware: vec!["csrf".to_string()],
            routes: vec![ast::RouteDecl {
                span: Span::unknown(),
                path: "/login".to_string(),
                method: "POST".to_string(),
                requires: vec![],
                body: vec![],
            }],
        };
        let source = "mlogserver { middleware: [csrf] }";
        let mut findings: Vec<AuditFinding> = Vec::new();
        check_csrf(&[Declaration::MlogServer(srv)], source, &mut findings);
        assert!(findings
            .iter()
            .any(|f| f.check_id == "CSRF" && f.severity == Severity::Info));
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
        assert!(result
            .findings
            .iter()
            .any(|f| f.check_id == "SQL_DYNAMIC" && f.severity == Severity::Info));
        assert!(!result
            .findings
            .iter()
            .any(|f| f.check_id == "SQL_DYNAMIC" && f.severity == Severity::Error));
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
        assert!(result
            .findings
            .iter()
            .any(|f| f.check_id == "SQL_DYNAMIC" && f.severity == Severity::Error));
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
        assert!(result
            .findings
            .iter()
            .any(|f| f.check_id == "SECRET_LEAK" && f.severity == Severity::Error));
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
        assert!(result
            .findings
            .iter()
            .any(|f| f.check_id == "SECRET_LEAK" && f.severity == Severity::Error));
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
        assert!(result
            .findings
            .iter()
            .any(|f| f.check_id == "HTML_INJECTION" && f.severity == Severity::Warning));
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
        assert!(!result
            .findings
            .iter()
            .any(|f| f.check_id == "HTML_INJECTION" && f.severity == Severity::Warning));
    }

    #[test]
    fn test_format_output() {
        // Construct MlogServerDecl directly to test taint + server checks together
        let srv = ast::MlogServerDecl {
            span: Span::unknown(),
            port: 8080,
            host: None,
            middleware: vec!["session".to_string()],
            routes: vec![ast::RouteDecl {
                span: Span::unknown(),
                path: "/data".to_string(),
                method: "POST".to_string(),
                requires: vec![],
                body: vec![
                    Statement::LetBinding {
                        name: "api_key".to_string(),
                        value: Expr::FnCall {
                            name: "env".to_string(),
                            args: vec![Expr::StringLit {
                                value: "KEY".to_string(),
                                span: Span::unknown(),
                            }],
                            span: Span::unknown(),
                        },
                        mutable: false,
                        span: Span::unknown(),
                    },
                    Statement::LetBinding {
                        name: "result".to_string(),
                        value: Expr::FnCall {
                            name: "call_llm".to_string(),
                            args: vec![Expr::StringLit {
                                value: "summarize".to_string(),
                                span: Span::unknown(),
                            }],
                            span: Span::unknown(),
                        },
                        mutable: false,
                        span: Span::unknown(),
                    },
                    Statement::LetBinding {
                        name: "resp".to_string(),
                        value: Expr::FnCall {
                            name: "respond".to_string(),
                            args: vec![
                                Expr::StringLit {
                                    value: "200 OK".to_string(),
                                    span: Span::unknown(),
                                },
                                Expr::Ident {
                                    name: "result".to_string(),
                                    span: Span::unknown(),
                                },
                            ],
                            span: Span::unknown(),
                        },
                        mutable: false,
                        span: Span::unknown(),
                    },
                    Statement::Return {
                        value: Expr::Ident {
                            name: "resp".to_string(),
                            span: Span::unknown(),
                        },
                        span: Span::unknown(),
                    },
                ],
            }],
        };
        let mut findings: Vec<AuditFinding> = Vec::new();
        check_rate_limit(
            &[Declaration::MlogServer(srv.clone())],
            "mlogserver",
            &mut findings,
        );
        check_csrf(
            &[Declaration::MlogServer(srv.clone())],
            "mlogserver",
            &mut findings,
        );
        check_html_injection(
            &[Declaration::MlogServer(srv.clone())],
            "mlogserver",
            &mut findings,
        );
        check_secret_leak(&[Declaration::MlogServer(srv)], "mlogserver", &mut findings);

        assert!(
            !findings.is_empty(),
            "should have findings from server+route analysis"
        );
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

    // ── Наряд #102: expanded SECRET_PATTERNS + SHORT_SECRET_PATTERNS ──

    #[test]
    fn test_github_pat_detected() {
        // Exact reproduction of audit #5 finding format
        let source = r#"
            pattern Init() -> String {
                let token = "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"
                return token
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            result.findings.iter().any(|f| f.check_id == "SECRETS"),
            "ghp_ token must be detected by expanded SECRET_PATTERNS"
        );
    }

    #[test]
    fn test_aws_access_key_detected_despite_short_length() {
        // AWS access key ID is exactly 20 chars (AKIA + 16), below generic threshold of 30
        let source = r#"
            pattern Init() -> String {
                let key = "AKIA1234567890ABCDEF"
                return key
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            result.findings.iter().any(|f| f.check_id == "SECRETS"),
            "AWS AKIA key (20 chars) must be detected despite being below generic 30-char threshold"
        );
    }

    #[test]
    fn test_original_patterns_still_work() {
        // Backward compatibility: original sk- pattern still triggers
        let source = r#"
            pattern Init() -> String {
                let key = "sk-ant-api03-very-long-string-here-abcdef1234567890"
                return key
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            result.findings.iter().any(|f| f.check_id == "SECRETS"),
            "Original sk- pattern must still be detected (backward compat)"
        );
    }

    #[test]
    fn test_short_random_string_not_flagged() {
        // A short string (< 20 chars) without recognizable prefix must NOT trigger
        let source = r#"
            pattern Init() -> String {
                let x = "hello_world_123"
                return x
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            result.findings.iter().all(|f| f.check_id != "SECRETS"),
            "Short random string without recognizable prefix must not be flagged"
        );
    }

    // ── Наряд №114: print is a SECRET_LEAK sink ──
    #[test]
    fn secret_leak_print_ident() {
        let source = r#"
            pattern Leak() -> String {
                let token = env("API_KEY")
                let _ = print(token)
                return "x"
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            result.findings.iter().any(|f| f.check_id == "SECRET_LEAK"),
            "print(token) where token from env must be SECRET_LEAK, got {:?}",
            result.findings
        );
    }

    #[test]
    fn secret_leak_print_direct_env() {
        let source = r#"
            pattern Leak() -> String {
                let _ = print(env("API_KEY"))
                return "x"
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            result.findings.iter().any(|f| f.check_id == "SECRET_LEAK"),
            "print(env(...)) must be SECRET_LEAK, got {:?}",
            result.findings
        );
    }

    // ── Наряд №123: nested call taint — single-level inline nesting ──

    #[test]
    fn html_injection_nested_call_llm_in_respond() {
        // respond(call_llm(...)) — direct nesting without intermediate variable.
        // Before №123 this was NOT caught (only Ident args were checked).
        let source = r#"
            pattern Direct() -> String {
                let r = respond("200 OK", call_llm("Tell me a joke"))
                return r
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.check_id == "HTML_INJECTION" && f.severity == Severity::Warning),
            "respond(call_llm(...)) must trigger HTML_INJECTION, got {:?}",
            result.findings
        );
    }

    #[test]
    fn html_injection_nested_call_claude_in_respond() {
        // respond(call_claude(...)) — same check for call_claude.
        let source = r#"
            pattern Direct() -> String {
                let r = respond("200 OK", call_claude("Tell me a joke"))
                return r
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.check_id == "HTML_INJECTION" && f.severity == Severity::Warning),
            "respond(call_claude(...)) must trigger HTML_INJECTION, got {:?}",
            result.findings
        );
    }

    #[test]
    fn html_injection_via_variable_still_caught() {
        // Regression: existing case (let x = call_llm(...); respond(x)) must still work.
        let source = r#"
            pattern Indirect() -> String {
                let result = call_llm("Tell me a joke")
                let r = respond("200 OK", result)
                return r
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.check_id == "HTML_INJECTION" && f.severity == Severity::Warning),
            "respond(x) where x from call_llm must still trigger HTML_INJECTION, got {:?}",
            result.findings
        );
    }

    #[test]
    fn secret_leak_http_post_direct_env() {
        // http_post(url, env("KEY"), headers) — direct nesting without variable.
        // Before №123 this was NOT caught (only Ident args were checked).
        let source = r#"
            pattern Leak() -> String {
                let r = http_post("https://api.example.com", env("AUTH_TOKEN"), "application/json")
                return r
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.check_id == "SECRET_LEAK" && f.severity == Severity::Error),
            "http_post(u, env(...), h) must trigger SECRET_LEAK, got {:?}",
            result.findings
        );
    }

    #[test]
    fn secret_leak_http_post_via_variable_still_caught() {
        // Regression: existing case (let t = env(...); http_post(u, t, h)) must still work.
        let source = r#"
            pattern Leak() -> String {
                let token = env("AUTH_TOKEN")
                let r = http_post("https://api.example.com", token, "application/json")
                return r
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.check_id == "SECRET_LEAK" && f.severity == Severity::Error),
            "http_post(u, token, h) where token from env must still trigger SECRET_LEAK, got {:?}",
            result.findings
        );
    }

    // ── Наряд №140: open-redirect catches nested user-input calls ──

    /// Helper: build a MlogServerDecl with a single route containing the given statements.
    fn make_server_route(stmts: Vec<Statement>) -> Declaration {
        Declaration::MlogServer(ast::MlogServerDecl {
            span: Span::unknown(),
            port: 8080,
            host: None,
            middleware: vec![],
            routes: vec![ast::RouteDecl {
                span: Span::unknown(),
                path: "/redirect".to_string(),
                method: "GET".to_string(),
                requires: vec![],
                body: stmts,
            }],
        })
    }

    #[test]
    fn open_redirect_nested_query_param() {
        // respond_html(query_param("url")) — direct nesting without variable.
        // Before №140 this was NOT caught (only Ident args were checked).
        let route_body = vec![Statement::LetBinding {
            name: "r".to_string(),
            value: Expr::FnCall {
                name: "respond_html".to_string(),
                args: vec![Expr::FnCall {
                    name: "query_param".to_string(),
                    args: vec![Expr::StringLit {
                        value: "url".to_string(),
                        span: Span::unknown(),
                    }],
                    span: Span::unknown(),
                }],
                span: Span::unknown(),
            },
            mutable: false,
            span: Span::unknown(),
        }];
        let decls = vec![make_server_route(route_body)];
        let mut findings: Vec<AuditFinding> = Vec::new();
        check_open_redirect(&decls, "respond_html", &mut findings);
        assert!(
            findings.iter().any(|f| f.check_id == "OPEN_REDIRECT"),
            "respond_html(query_param(\"url\")) must trigger OPEN_REDIRECT, got {:?}",
            findings
        );
    }

    #[test]
    fn open_redirect_nested_form_data() {
        // respond_html(form_data("target")) — another user-input source.
        let route_body = vec![Statement::LetBinding {
            name: "r".to_string(),
            value: Expr::FnCall {
                name: "respond_html".to_string(),
                args: vec![Expr::FnCall {
                    name: "form_data".to_string(),
                    args: vec![Expr::StringLit {
                        value: "target".to_string(),
                        span: Span::unknown(),
                    }],
                    span: Span::unknown(),
                }],
                span: Span::unknown(),
            },
            mutable: false,
            span: Span::unknown(),
        }];
        let decls = vec![make_server_route(route_body)];
        let mut findings: Vec<AuditFinding> = Vec::new();
        check_open_redirect(&decls, "respond_html", &mut findings);
        assert!(
            findings.iter().any(|f| f.check_id == "OPEN_REDIRECT"),
            "respond_html(form_data(\"target\")) must trigger OPEN_REDIRECT, got {:?}",
            findings
        );
    }

    #[test]
    fn open_redirect_nested_json_body() {
        // respond_html(json_body("url")) — third user-input source.
        let route_body = vec![Statement::LetBinding {
            name: "r".to_string(),
            value: Expr::FnCall {
                name: "respond_html".to_string(),
                args: vec![Expr::FnCall {
                    name: "json_body".to_string(),
                    args: vec![Expr::StringLit {
                        value: "url".to_string(),
                        span: Span::unknown(),
                    }],
                    span: Span::unknown(),
                }],
                span: Span::unknown(),
            },
            mutable: false,
            span: Span::unknown(),
        }];
        let decls = vec![make_server_route(route_body)];
        let mut findings: Vec<AuditFinding> = Vec::new();
        check_open_redirect(&decls, "respond_html", &mut findings);
        assert!(
            findings.iter().any(|f| f.check_id == "OPEN_REDIRECT"),
            "respond_html(json_body(\"url\")) must trigger OPEN_REDIRECT, got {:?}",
            findings
        );
    }

    #[test]
    fn open_redirect_via_variable_still_caught() {
        // Regression: existing case (let u = query_param(...); respond_html(u)) must still work.
        let route_body = vec![
            Statement::LetBinding {
                name: "url".to_string(),
                value: Expr::FnCall {
                    name: "query_param".to_string(),
                    args: vec![Expr::StringLit {
                        value: "target".to_string(),
                        span: Span::unknown(),
                    }],
                    span: Span::unknown(),
                },
                mutable: false,
                span: Span::unknown(),
            },
            Statement::LetBinding {
                name: "r".to_string(),
                value: Expr::FnCall {
                    name: "respond_html".to_string(),
                    args: vec![Expr::Ident {
                        name: "url".to_string(),
                        span: Span::unknown(),
                    }],
                    span: Span::unknown(),
                },
                mutable: false,
                span: Span::unknown(),
            },
        ];
        let decls = vec![make_server_route(route_body)];
        let mut findings: Vec<AuditFinding> = Vec::new();
        check_open_redirect(&decls, "respond_html", &mut findings);
        assert!(
            findings.iter().any(|f| f.check_id == "OPEN_REDIRECT"),
            "respond_html(url) where url from query_param must still trigger OPEN_REDIRECT, got {:?}",
            findings
        );
    }

    #[test]
    fn open_redirect_clean_literal_not_flagged() {
        // respond_html with a literal string must NOT trigger.
        let route_body = vec![Statement::LetBinding {
            name: "r".to_string(),
            value: Expr::FnCall {
                name: "respond_html".to_string(),
                args: vec![Expr::StringLit {
                    value: "<h1>Hello</h1>".to_string(),
                    span: Span::unknown(),
                }],
                span: Span::unknown(),
            },
            mutable: false,
            span: Span::unknown(),
        }];
        let decls = vec![make_server_route(route_body)];
        let mut findings: Vec<AuditFinding> = Vec::new();
        check_open_redirect(&decls, "respond_html", &mut findings);
        assert!(
            !findings.iter().any(|f| f.check_id == "OPEN_REDIRECT"),
            "respond_html(literal) must NOT trigger OPEN_REDIRECT, got {:?}",
            findings
        );
    }

    #[test]
    fn open_redirect_llm_not_confused_with_user_input() {
        // respond_html(call_llm(...)) must NOT trigger OPEN_REDIRECT
        // (LLM taint is not user-input taint — separate sources).
        let route_body = vec![Statement::LetBinding {
            name: "r".to_string(),
            value: Expr::FnCall {
                name: "respond_html".to_string(),
                args: vec![Expr::FnCall {
                    name: "call_llm".to_string(),
                    args: vec![Expr::StringLit {
                        value: "generate page".to_string(),
                        span: Span::unknown(),
                    }],
                    span: Span::unknown(),
                }],
                span: Span::unknown(),
            },
            mutable: false,
            span: Span::unknown(),
        }];
        let decls = vec![make_server_route(route_body)];
        let mut findings: Vec<AuditFinding> = Vec::new();
        check_open_redirect(&decls, "respond_html", &mut findings);
        assert!(
            !findings.iter().any(|f| f.check_id == "OPEN_REDIRECT"),
            "respond_html(call_llm(...)) must NOT trigger OPEN_REDIRECT (wrong taint kind), got {:?}",
            findings
        );
    }

    // ── Наряд #141: TAINT_PERSISTENCE — memorize(LLM) + respond(recall()) ──

    #[test]
    fn taint_persistence_memorize_llm_and_respond_recall() {
        let source = r#"
            memorize call_llm("store this") with priority=0.9
            pattern Handler() -> String {
                let ctx = recall("relevant")
                let r = respond("200 OK", ctx)
                return r
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.check_id == "TAINT_PERSISTENCE" && f.severity == Severity::Warning),
            "memorize(call_llm(...)) + respond(recall(...)) must trigger TAINT_PERSISTENCE, got {:?}",
            result.findings
        );
    }

    #[test]
    fn taint_persistence_no_warning_without_recall() {
        let source = r#"
            memorize call_llm("store this") with priority=0.9
            pattern Handler() -> String {
                let r = respond("200 OK", "safe")
                return r
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            !result
                .findings
                .iter()
                .any(|f| f.check_id == "TAINT_PERSISTENCE"),
            "no recall in respond -> no TAINT_PERSISTENCE, got {:?}",
            result.findings
        );
    }

    #[test]
    fn taint_persistence_no_warning_with_non_llm_memorize() {
        let source = r#"
            memorize "just a static fact" with priority=0.5
            pattern Handler() -> String {
                let ctx = recall("relevant")
                let r = respond("200 OK", ctx)
                return r
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            !result
                .findings
                .iter()
                .any(|f| f.check_id == "TAINT_PERSISTENCE"),
            "memorize(literal) is not LLM taint -> no TAINT_PERSISTENCE, got {:?}",
            result.findings
        );
    }

    // ── Наряд #141: TAINT_PASSTHROUGH — respond(Wrap(call_llm(...))) ──

    #[test]
    fn taint_passthrough_respond_wrap_call_llm() {
        let source = r#"
            pattern Wrap(x: String) -> String {
                return x
            }
            pattern Handler() -> String {
                let r = respond("200 OK", Wrap(call_llm("Tell me a joke")))
                return r
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.check_id == "TAINT_PASSTHROUGH" && f.severity == Severity::Warning),
            "respond(Wrap(call_llm(...))) must trigger TAINT_PASSTHROUGH, got {:?}",
            result.findings
        );
    }

    #[test]
    fn taint_passthrough_respond_html_wrap_call_llm() {
        let source = r#"
            pattern Wrap(x: String) -> String {
                return x
            }
            pattern Handler() -> String {
                let r = respond_html(Wrap(call_claude("Summarize")))
                return r
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.check_id == "TAINT_PASSTHROUGH" && f.severity == Severity::Warning),
            "respond_html(Wrap(call_claude(...))) must trigger TAINT_PASSTHROUGH, got {:?}",
            result.findings
        );
    }

    #[test]
    fn taint_passthrough_non_passthrough_pattern_no_warning() {
        let source = r#"
            pattern Combine(a: String, b: String) -> String {
                return a
            }
            pattern Handler() -> String {
                let r = respond("200 OK", Combine(call_llm("prompt")))
                return r
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            !result
                .findings
                .iter()
                .any(|f| f.check_id == "TAINT_PASSTHROUGH"),
            "two-param pattern is not trivial passthrough -> no TAINT_PASSTHROUGH, got {:?}",
            result.findings
        );
    }

    #[test]
    fn taint_passthrough_pattern_with_logic_no_warning() {
        let source = r#"
            pattern Safe(x: String) -> String {
                let escaped = escape_html(x)
                return escaped
            }
            pattern Handler() -> String {
                let r = respond("200 OK", Safe(call_llm("prompt")))
                return r
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            !result
                .findings
                .iter()
                .any(|f| f.check_id == "TAINT_PASSTHROUGH"),
            "pattern with escape_html is not trivial passthrough -> no TAINT_PASSTHROUGH, got {:?}",
            result.findings
        );
    }

    #[test]
    fn taint_passthrough_no_false_positive_existing_html_injection() {
        let source = r#"
            pattern Handler() -> String {
                let r = respond("200 OK", call_llm("Tell me a joke"))
                return r
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.check_id == "HTML_INJECTION" && f.severity == Severity::Warning),
            "direct respond(call_llm(...)) must still trigger HTML_INJECTION, got {:?}",
            result.findings
        );
        assert!(
            !result
                .findings
                .iter()
                .any(|f| f.check_id == "TAINT_PASSTHROUGH"),
            "no passthrough pattern -> no TAINT_PASSTHROUGH, got {:?}",
            result.findings
        );
    }
}
