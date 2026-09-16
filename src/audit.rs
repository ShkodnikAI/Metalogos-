// ── Static security analysis for METALOGOS (.mlog) programs ─────
// ADR-0057: `mlog audit <file>` — analyzes without executing.
// Checks: SECRETS, HTML_INJECTION, SQL_DYNAMIC, SANDBOX_COVERAGE,
//         RATE_LIMIT, CSRF, SECRET_LEAK, OPEN_REDIRECT,
//         TAINT_PERSISTENCE, TAINT_PASSTHROUGH, CANARY_LEAK (№284).

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
    /// Наряд №284: value is the response of a channel where a canary leak
    /// was CONFIRMED (canary_check → leaked=true) — «компрометированный
    /// канал». Поставляется ТОЛЬКО внутри then-ветки `if (r.leaked)`
    /// (path-sensitive approximation, check_canary_leak ниже); sinks
    /// получают advisory-warning CANARY_LEAK. Детектор, не гейт:
    /// render/escape_html снимают (Sanitized), redact НЕ снимает
    /// (маскирование ≠ санитизация канала, лекало ADR-0136 D2).
    CanaryLeak,
}

/// Per-scope taint tracker. Maps variable name to its taint kind.
/// Clone — для path-sensitive форка в check_canary_leak (№284).
#[derive(Clone)]
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
            // Наряд №274 (ADR-0136): redact — taint-санитайзер для Secret.
            // Inline-вызовы вида respond(redact(env("K"), "secrets")) должны
            // проходить аудитом так же, как цепочка через let-связывание.
            if fn_name == "redact" {
                return redact_result_taint(args, tracker);
            }
            // env() and secret() are secret sources even with no tainted args.
            // Наряд №172: secret() has the same taint as env() — both produce
            // TaintKind::Secret. binding_taint above also handles this, but
            // get_expr_taint is called for inline calls like respond(secret("K"))
            // where there's no intermediate let-binding to taint.
            if fn_name == "env" || fn_name == "secret" {
                return Some(TaintKind::Secret);
            }
            // Наряд №201: reflex_generate is an LLM-output-equivalent source
            // (model output is untrusted per ADR-0117). Same treatment as
            // call_llm — produces LlmOutput taint even with no tainted args.
            if fn_name == "reflex_generate" {
                return Some(TaintKind::LlmOutput);
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
        Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::StringLit { .. }
        | Expr::StructLit { .. } => None,
        // Наряд №201: list literals propagate taint from their elements.
        // E.g. [[env("K"), 0.0]] — the list carries Secret taint because
        // it contains a tainted element. This is needed for reflex_train
        // data/labels taint checking where data is a List<List<Float>>.
        Expr::List { items, .. } => {
            for item in items {
                if let Some(taint) = get_expr_taint(item, tracker) {
                    if taint != TaintKind::Sanitized {
                        return Some(taint);
                    }
                }
            }
            None
        }
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
    if let Expr::FnCall {
        name: fn_name,
        args,
        ..
    } = value
    {
        match fn_name.as_str() {
            // Наряд №201: вывод модели наследует недоверенность LLM-учителя (ADR-0117);
            // learnable pattern — тот же класс: вывод call_llm за пределами pattern body.
            "call_llm" | "call_claude" | "call_llm_schema" | "reflex_generate" => {
                return Some(TaintKind::LlmOutput)
            }
            "env" | "secret" => return Some(TaintKind::Secret),
            "render" | "escape_html" => return Some(TaintKind::Sanitized),
            // Наряд №274 (ADR-0136): redact — taint-санитайзер для Secret
            // («mask before sink»). Семантика снятия — redact_result_taint.
            "redact" => return redact_result_taint(args, tracker),
            // Наряд №268 (ADR-0132 D3, решение владельца 2026-09-12): вывод
            // MCP-инструмента — недоверенные данные. Reuse `UserInput`
            // (ToolOutput — Future, заводится только с первой политикой,
            // различающей роды). mcp_list_tools НЕ tainted: метаданные, не вывод.
            "form_data" | "json_body" | "query_param" | "mcp_call" => {
                return Some(TaintKind::UserInput)
            }
            _ => {}
        }
    }
    get_expr_taint(value, tracker)
}

/// Наряд №309 (ADR-0151 D6): an I2V reference frame is untrusted when it
/// carries UserInput taint (form_data/json_body/query_param/mcp_call) or is
/// a direct untrusted-source call of the http/file class (`http_get`,
/// `read_file`) — the http/form/file classes of ADR-0149 D5. Inline calls
/// are matched by name because these builtins are not global expression-
/// level taint sources (get_expr_taint only propagates from tainted args;
/// binding_taint applies to let-bindings), so a direct
/// `video_render(m, p, form_data("f"))` would otherwise escape.
fn is_untrusted_frame_expr(expr: &Expr, tracker: &TaintTracker) -> bool {
    if let Expr::FnCall { name, .. } = expr {
        if matches!(
            name.as_str(),
            "http_get" | "read_file" | "form_data" | "json_body" | "query_param" | "mcp_call"
        ) {
            return true;
        }
    }
    get_expr_taint(expr, tracker) == Some(TaintKind::UserInput)
}

/// Maximum nesting depth for `expr_is_llm_tainted` recursion.
/// Баунделенная константа — prevent stack overflow on deeply nested
/// expressions. Громкое примечание при превышении — анализ отказывается
/// идти глубже, но это не crash, и documented в README "Known boundaries".
/// Наряд №295 (issue #359): was single-level (depth=1), now 3.
const TAINT_NESTING_MAX_DEPTH: usize = 3;

/// Check whether an expression carries LLM-output taint.
/// Handles both variable references (via tracker) and direct LLM
/// function calls (call_llm / call_claude / reflex_generate) without
/// an intermediate variable binding.
///
/// Наряд №295 (issue #359): was single-level nesting only (`FnCall { name: "call_llm", .. }`
/// matched directly; `upper(call_llm(...))` did NOT match because the outer
/// FnCall name was "upper"). Now bounded-recursive up to
/// `TAINT_NESTING_MAX_DEPTH = 3` — catches `respond(upper(upper(call_llm(...))))`
/// and equivalent chains. Sanitizers (`render`/`escape_html`) at any depth
/// return false (taint lifted) — zero false positives on legitimate code.
///
/// Interprocedural analysis (across pattern-call boundaries) is a separate
/// check (`check_taint_interp_pattern`, Наряд №292).
fn expr_is_llm_tainted(expr: &Expr, tracker: &TaintTracker) -> bool {
    expr_is_llm_tainted_bounded(expr, tracker, 0)
}

fn expr_is_llm_tainted_bounded(expr: &Expr, tracker: &TaintTracker, depth: usize) -> bool {
    if depth > TAINT_NESTING_MAX_DEPTH {
        // Громкое примечание не выдается здесь (return false) — README
        // "Known boundaries" документирует границу. Interprocedural
        // taint (TAINT_INTERP, Наряд №292) ловит через summary-based analysis.
        return false;
    }
    match expr {
        Expr::Ident { name, .. } => tracker.get_taint(name) == Some(TaintKind::LlmOutput),
        Expr::FnCall { name, args, .. } => {
            // Direct LLM source — return true regardless of depth.
            if is_llm_source(name) {
                return true;
            }
            // Sanitizers lift the taint — render()/escape_html() at any depth.
            if name == "render" || name == "escape_html" {
                return false;
            }
            // Recurse into args — bounded nesting. Any arg that's
            // LLM-tainted (directly or through a bounded sub-chain) → true.
            args.iter()
                .any(|arg| expr_is_llm_tainted_bounded(arg, tracker, depth + 1))
        }
        // BinaryOp / IfElse / List / FieldAccess / IndexAccess — recurse
        // into sub-expressions (mirrors `get_expr_taint` propagation).
        Expr::BinaryOp { left, right, .. } => {
            expr_is_llm_tainted_bounded(left, tracker, depth + 1)
                || expr_is_llm_tainted_bounded(right, tracker, depth + 1)
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            expr_is_llm_tainted_bounded(condition, tracker, depth + 1)
                || expr_is_llm_tainted_bounded(then_branch, tracker, depth + 1)
                || expr_is_llm_tainted_bounded(else_branch, tracker, depth + 1)
        }
        Expr::List { items, .. } => items
            .iter()
            .any(|item| expr_is_llm_tainted_bounded(item, tracker, depth + 1)),
        Expr::FieldAccess { object, .. } => expr_is_llm_tainted_bounded(object, tracker, depth + 1),
        Expr::IndexAccess { object, index, .. } => {
            expr_is_llm_tainted_bounded(object, tracker, depth + 1)
                || expr_is_llm_tainted_bounded(index, tracker, depth + 1)
        }
        // Literals, struct literals, etc. — never LLM-tainted directly.
        _ => false,
    }
}

/// Returns true if the function name is a known LLM output source.
/// Наряд №201: reflex_generate produces untrusted output (model trained
/// on data that may include LLM-tainted content per ADR-0117), so its
/// output is treated as LlmOutput for HTML_INJECTION purposes.
fn is_llm_source(name: &str) -> bool {
    matches!(name, "call_llm" | "call_claude" | "reflex_generate")
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

/// Наряд №274 (ADR-0136, стоп-гейт СГ-2 решён владельцем 2026-09-12):
/// taint-семантика `redact(text, mode)`.
///
/// mode читается СТАТИЧЕСКИ из строкового литерала:
///   - `"secrets" | "all"` снимают ТОЛЬКО `Secret`-taint → результат
///     `Sanitized` (легальный путь «mask before sink»);
///   - `"pii"` `Secret` НЕ снимает — тест-инвариант
///     `secret → redact("pii") → http_post` ОТКЛОНЯЕТСЯ;
///   - `LlmOutput` не снимается redact'ом вообще (для вывода модели
///     санитайзер один — render; маскирование ≠ HTML-escape);
///   - `UserInput` не снимается (маскирование не меняет происхождение);
///   - динамический/нелитеральный/неизвестный mode — fail-closed:
///     taint входа наследуется без снятия.
fn redact_result_taint(args: &[Expr], tracker: &TaintTracker) -> Option<TaintKind> {
    let input = args.first().and_then(|a| get_expr_taint(a, tracker));
    let mode = args.get(1).and_then(|m| match m {
        Expr::StringLit { value, .. } => Some(value.as_str()),
        _ => None,
    });
    // №326: the policy is a value — one-way policies (registry
    // target_conf == "public") destroy the data, so any taint lifts to
    // Sanitized; conservative policies pass the taint through.
    let target_public = mode
        .and_then(crate::builtins::string::redact_policy)
        .is_some_and(|p| p.target_conf == "public");
    match mode {
        // One-way policies destroy the SECRET DATA (ADR-0136 D2 stays
        // authoritative): Secret lifts to Sanitized. Channel-level kinds
        // (LlmOutput) and the quarantine path (CanaryLeak) are NOT
        // curable by redact — masking is not channel sanitization (№284).
        Some(_) if target_public => match input {
            Some(TaintKind::Secret) => Some(TaintKind::Sanitized),
            other => other,
        },
        _ => input,
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
                // Наряд №266: statement-form memory ops inside bodies — string
                // scanning parity with the top-level Declaration::Memorize/
                // Forget/Relate handling (same walker).
                Statement::Memorize(m) => walk_string_exprs(&m.value, acc),
                Statement::Forget(f) => walk_string_exprs(&f.query, acc),
                Statement::Relate(r) => {
                    walk_string_exprs(&r.from, acc);
                    walk_string_exprs(&r.to, acc);
                }
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

// ── Check: SQL_DYNAMIC — non-literal SQL in query()/db_execute() ──────────

/// Check that all query() and db_execute() calls use literal SQL strings.
/// db_execute has no safe non-literal path (same as query: parameterized
/// queries require a literal SQL template with ?/$N placeholders).
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
                if name == "query" || name == "db_execute" {
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
                                        "SQL injection risk — {}() with non-literal SQL ({})",
                                        name, snippet
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
                "all {} query()/db_execute() calls use literal SQL \u{2713}",
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
    // Наряд №201: collect learnable pattern names so calls to them are
    // treated as LlmOutput sources (their body contains call_llm, so
    // their output is untrusted text per ADR-0117). Mechanism choice:
    // mark any declared learnable pattern as a taint source — this is
    // conservative (a learnable pattern that doesn't actually call
    // call_llm is still flagged), but it matches the existing
    // intraprocedural audit level and avoids the complexity of walking
    // pattern bodies. If a learnable pattern is provably safe (no
    // call_llm in body), the user can refactor it as a regular pattern.
    let learnable_names: std::collections::HashSet<String> = declarations
        .iter()
        .filter_map(|d| match d {
            Declaration::LearnablePattern(lp) => Some(lp.name.clone()),
            _ => None,
        })
        .collect();

    fn check_respond_for_html(
        expr: &Expr,
        tracker: &TaintTracker,
        learnable_names: &std::collections::HashSet<String>,
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
                    // Наряд №201: also catch calls to learnable patterns
                    // (their output is untrusted per ADR-0117).
                    if expr_is_llm_tainted(arg, tracker)
                        || expr_is_learnable_tainted(arg, learnable_names)
                    {
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

    /// Check if an expression is a direct call to a learnable pattern.
    fn expr_is_learnable_tainted(
        expr: &Expr,
        learnable_names: &std::collections::HashSet<String>,
    ) -> bool {
        match expr {
            Expr::FnCall { name, .. } => learnable_names.contains(name),
            _ => false,
        }
    }

    /// Analyze a list of statements for LLM→respond taint flow.
    fn analyze_scope(
        stmts: &[Statement],
        learnable_names: &std::collections::HashSet<String>,
        source: &str,
        findings: &mut Vec<AuditFinding>,
    ) {
        let mut tracker = TaintTracker::new();

        fn process_stmt(
            stmt: &Statement,
            tracker: &mut TaintTracker,
            learnable_names: &std::collections::HashSet<String>,
            source: &str,
            findings: &mut Vec<AuditFinding>,
        ) {
            match stmt {
                Statement::LetBinding { name, value, .. } => {
                    // Check if this let-binding calls respond() with tainted args
                    check_respond_for_html(value, tracker, learnable_names, source, findings);
                    // Наряд №201: calls to learnable patterns produce LlmOutput taint.
                    if let Expr::FnCall { name: fn_name, .. } = value {
                        if learnable_names.contains(fn_name) {
                            tracker.taint(name, TaintKind::LlmOutput);
                            return;
                        }
                    }
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
                    if let Expr::FnCall { name: fn_name, .. } = value {
                        if learnable_names.contains(fn_name) {
                            tracker.taint(name, TaintKind::LlmOutput);
                            return;
                        }
                    }
                    if let Some(taint) = binding_taint(value, tracker) {
                        tracker.taint(name, taint);
                    } else {
                        tracker.untaint(name);
                    }
                }
                Statement::ExprStmt { expr, .. } => {
                    check_respond_for_html(expr, tracker, learnable_names, source, findings);
                }
                Statement::Return { value: expr, .. } => {
                    check_respond_for_html(expr, tracker, learnable_names, source, findings);
                }
                Statement::Each { body, .. } => {
                    for s in body {
                        process_stmt(s, tracker, learnable_names, source, findings);
                    }
                }
                Statement::While { body, .. } => {
                    for s in body {
                        process_stmt(s, tracker, learnable_names, source, findings);
                    }
                }
                Statement::IfElseBlock {
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    for s in then_body {
                        process_stmt(s, tracker, learnable_names, source, findings);
                    }
                    for (_, body) in else_ifs {
                        for s in body {
                            process_stmt(s, tracker, learnable_names, source, findings);
                        }
                    }
                    if let Some(body) = else_body {
                        for s in body {
                            process_stmt(s, tracker, learnable_names, source, findings);
                        }
                    }
                }
                Statement::IfThen { body, .. } => {
                    for s in body {
                        process_stmt(s, tracker, learnable_names, source, findings);
                    }
                }
                _ => {}
            }
        }

        for stmt in stmts {
            process_stmt(stmt, &mut tracker, learnable_names, source, findings);
        }
    }

    for decl in declarations {
        match decl {
            Declaration::MlogServer(srv) => {
                for route in &srv.routes {
                    analyze_scope(&route.body, &learnable_names, source, findings);
                }
            }
            Declaration::Pattern(p) => analyze_scope(&p.body, &learnable_names, source, findings),
            Declaration::Tool(t) => {
                for m in &t.methods {
                    analyze_scope(&m.body, &learnable_names, source, findings);
                }
            }
            Declaration::Hook(h) => analyze_scope(&h.body, &learnable_names, source, findings),
            Declaration::Test(_) => {}
            _ => {}
        }
    }
}

// ── Check: SECRET_LEAK — env() result passed to respond/write_file sinks;
//   http_post: positional — url (arg 0) is a leak, body (arg 1) is a leak,
//   headers (arg 3) is normal auth (Bearer tokens etc.)

fn check_secret_leak(declarations: &[Declaration], source: &str, findings: &mut Vec<AuditFinding>) {
    /// Sink functions where ANY argument carrying a secret is a leak.
    /// http_post is handled separately with positional logic below:
    ///   arg 0 (url) — leak;  arg 1 (body) — leak;  arg 3 (headers) — safe.
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
                // http_post positional logic (Наряд #157):
                //   arg 0 (url) — secret in URL is a leak (logged, visible).
                //   arg 1 (body) — secret in request body is a leak.
                //   arg 3 (headers) — legitimate auth (Bearer tokens), NOT flagged.
                // Наряд №123: use get_expr_taint to catch both variable refs
                // and direct env() calls (e.g. http_post(u, env("K"), h)).
                if fn_name == "http_post" {
                    // arg 0: URL — secret leak
                    if let Some(arg) = args.first() {
                        if get_expr_taint(arg, tracker) == Some(TaintKind::Secret) {
                            let line = find_line(source, fn_name);
                            findings.push(AuditFinding {
                                severity: Severity::Error,
                                check_id: "SECRET_LEAK",
                                line,
                                message: "secret may be leaked \u{2014} env() value passed as http_post URL"
                                    .to_string(),
                            });
                        }
                    }
                    // arg 1: body — secret leak
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
                    // arg 3 (headers) intentionally NOT checked — Bearer auth is legitimate.
                }

                // Наряд №201: reflex_train positional logic.
                //   arg 1 (data) / arg 2 (labels) — то, что попадает в веса.
                //   arg 0 (name) — имя модели, не данные: не флагается.
                // Веса уходят на диск через reflex_save (ADR-0116) — в обход
                // файловых стоков. Taint физически не может сидеть на
                // Value::Reflex (opaque handle) — поэтому перехват цепочки
                // env→train→save делается именно здесь, на args reflex_train.
                if fn_name == "reflex_train" {
                    for pos in [1usize, 2usize] {
                        if let Some(arg) = args.get(pos) {
                            let t = get_expr_taint(arg, tracker);
                            if t == Some(TaintKind::Secret) {
                                let line = find_line(source, fn_name);
                                findings.push(AuditFinding {
                                    severity: Severity::Error,
                                    check_id: "SECRET_LEAK",
                                    line,
                                    message: "secret may be leaked \u{2014} env() value used as reflex_train data/labels (weights persist via reflex_save)"
                                        .to_string(),
                                });
                            } else if t == Some(TaintKind::UserInput) {
                                let line = find_line(source, fn_name);
                                findings.push(AuditFinding {
                                    severity: Severity::Error,
                                    check_id: "UNTRUSTED_TRAINING_DATA",
                                    line,
                                    message: "untrusted user input used as reflex_train data/labels (model poisoning / PII baked into weights)"
                                        .to_string(),
                                });
                            }
                        }
                    }
                }

                // Наряд №240 (Vision R4.2): vision_generate positional logic
                // (plan §4, лекало n201 reflex_train). Arg 0 is the
                // declaration name (not data); arg 1 is the prompt — if it
                // carries UserInput taint (form_data/json_body/query_param),
                // emit an audit-WARNING, NOT a Category-A error: submitting a
                // user-typed image prompt is a legitimate use case; the
                // prompt will be recorded in the generation manifest (R5).
                // No taint is placed on Value::Vision (opaque handle —
                // plan §4); the print-guard on the handle already stands.
                if fn_name == "vision_generate" {
                    if let Some(arg) = args.get(1) {
                        if get_expr_taint(arg, tracker) == Some(TaintKind::UserInput) {
                            let line = find_line(source, fn_name);
                            findings.push(AuditFinding {
                                severity: Severity::Warning,
                                check_id: "VISION_PROMPT_USER_INPUT",
                                line,
                                message: "user input used as vision_generate prompt — prompt will be recorded in the generation manifest (R5); unsigned export gates are R5"
                                    .to_string(),
                            });
                        }
                    }
                }

                // Наряд №243 (Vision R6.2): the SAME positional check
                // extends to `vision_edit` calls — arg 0 is the Vision
                // handle (not data; лекало №240 Block 3: arg 0 is not
                // flagged), arg 1 is the edit prompt. Same check-id and
                // category as vision_generate (no new check-id in №243):
                // the prompt is recorded in the edited artifact's
                // provenance manifest (prompt_sha256), so UserInput taint
                // must stay loud (advisory Warning — a user-typed edit
                // instruction is a legitimate use case).
                if fn_name == "vision_edit" {
                    if let Some(arg) = args.get(1) {
                        if get_expr_taint(arg, tracker) == Some(TaintKind::UserInput) {
                            let line = find_line(source, fn_name);
                            findings.push(AuditFinding {
                                severity: Severity::Warning,
                                check_id: "VISION_PROMPT_USER_INPUT",
                                line,
                                message: "user input used as vision_edit prompt — prompt will be recorded in the edited artifact's provenance manifest (R6.2); the source must be a signed artifact"
                                    .to_string(),
                            });
                        }
                    }
                }

                // Наряд №244 (Vision R6.3): the SAME positional check
                // extends to `vision_lora_generate` calls — arg 0 is the
                // declaration name and arg 2 the adapter name (neither is
                // data; лекало №240/№243: positional args that are not the
                // prompt are not flagged), arg 1 is the prompt. Same
                // check-id and category as vision_generate/vision_edit (no
                // new check-id in №244): the prompt is recorded in the
                // generated artifact's provenance manifest (prompt_sha256),
                // so UserInput taint must stay loud (advisory Warning).
                if fn_name == "vision_lora_generate" {
                    if let Some(arg) = args.get(1) {
                        if get_expr_taint(arg, tracker) == Some(TaintKind::UserInput) {
                            let line = find_line(source, fn_name);
                            findings.push(AuditFinding {
                                severity: Severity::Warning,
                                check_id: "VISION_PROMPT_USER_INPUT",
                                line,
                                message: "user input used as vision_lora_generate prompt — prompt will be recorded in the generated artifact's provenance manifest (R6.3); the LoRA adapter is resolved from the program database"
                                    .to_string(),
                            });
                        }
                    }
                }

                // Наряд №309 (ADR-0151 D6, ADR-0149 D5): UNTRUSTED_FRAME —
                // advisory taint по лекалу UNTRUSTED_AUDIO (ADR-0145 D6) и
                // VISION_PROMPT_USER_INPUT (№240). I2V-референс video_render
                // (позиции 2 и 3 — first/last anchor) из недоверенного
                // источника — UserInput taint (form_data/json_body/
                // query_param/mcp_call) или прямой недоверенный вызов класса
                // http/file (http_get/read_file) — помечается Warning'ом.
                // Требование: screen+consent-путь перед I2V (frame_screen /
                // LikenessToken — полная механика в V6, ADR-0149 D1/D6; до
                // тех пор детектор — громкий путь). На compile-пути
                // (audit_category_a → semantic №98-промоция) это громкая
                // ошибка компиляции; в `mlog audit` — advisory Warning.
                // Честная граница (та же, что у всех MVP-детекторов taint):
                // let-связанные переменные с ранее полученными недоверенными
                // кадрами вне перечисленных источников не отслеживаются —
                // глобальные taint-источники других столпов не менялись.
                if fn_name == "video_render" && args.len() >= 3 {
                    for pos in [2usize, 3usize] {
                        if let Some(arg) = args.get(pos) {
                            if is_untrusted_frame_expr(arg, tracker) {
                                let line = find_line(source, fn_name);
                                findings.push(AuditFinding {
                                    severity: Severity::Warning,
                                    check_id: "UNTRUSTED_FRAME",
                                    line,
                                    message: "untrusted frame (user input / http / file) passed as I2V reference to video_render — screen+consent path required before I2V (ADR-0149 D5, ADR-0151 D6)"
                                        .to_string(),
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

// ── Check: VISION_UNSIGNED_EXPORT / VISION_UNSIGNED_EXPORT_RAW ───────
//
// Наряд №241 (R5, Block 2.2/2.3 — ADR-0125 Category-A gates).
//
// VISION_UNSIGNED_EXPORT (Severity::Error, Category A): a `vision_export`
//   call site in a file with NO `vision { }` declaration — the provenance
//   manifest source is impossible there, so the artifact cannot be signed
//   by construction. Лекало SECRET_LEAK: structural invariant, never a
//   legitimate false positive. Runtime backstop with the same check-id:
//   exporting a manifest-less artifact via vision_export_dispatch is a
//   loud Err (src/builtins/vision.rs).
// VISION_UNSIGNED_EXPORT_RAW (Severity::Warning, advisory — `mlog audit`):
//   every `vision_export_raw` call site — the explicit opt-out is chosen,
//   the audit makes it loud. ADR-0125 does not name the raw-warning; the
//   check-id is fixed here and in the dispatch docs (gромко в PR №241).
//
// Scope set mirrors check_secret_leak (MlogServer routes, Pattern, Tool,
// Hook bodies) — the same statically walkable scopes.

fn check_vision_export_gates(
    declarations: &[Declaration],
    source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    check_vision_export_gates_impl(declarations, source, findings, false)
}

/// Compile-path entry (audit_category_a): ONLY the Category-A Error.
/// The raw-export Warning must stay advisory — semantic.rs promotes every
/// audit_category_a Warning to a compile error (№98 promotion), which
/// would contradict ADR-0125's explicit "warning" for the opt-out.
fn check_vision_export_gates_errors_only(
    declarations: &[Declaration],
    source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    check_vision_export_gates_impl(declarations, source, findings, true)
}

fn check_vision_export_gates_impl(
    declarations: &[Declaration],
    source: &str,
    findings: &mut Vec<AuditFinding>,
    errors_only: bool,
) {
    let has_vision_decl = declarations
        .iter()
        .any(|d| matches!(d, Declaration::Vision(_)));

    fn check_expr(
        expr: &Expr,
        has_vision_decl: bool,
        errors_only: bool,
        source: &str,
        findings: &mut Vec<AuditFinding>,
    ) {
        if let Expr::FnCall { name: fn_name, .. } = expr {
            if fn_name == "vision_export" && !has_vision_decl {
                let line = find_line(source, "vision_export");
                findings.push(AuditFinding {
                    severity: Severity::Error,
                    check_id: "VISION_UNSIGNED_EXPORT",
                    line,
                    message: "vision_export call site in a file with no `vision { }` declaration \
                              — the provenance manifest source is impossible here, the artifact \
                              cannot be signed by construction (ADR-0125). Declare `vision { }` \
                              in this file, or use vision_export_raw explicitly"
                        .to_string(),
                });
            }
            if fn_name == "vision_export_raw" && !errors_only {
                let line = find_line(source, "vision_export_raw");
                findings.push(AuditFinding {
                    severity: Severity::Warning,
                    check_id: "VISION_UNSIGNED_EXPORT_RAW",
                    line,
                    message: "raw (unsigned) vision export — no watermark, no manifest sidecar \
                              (ADR-0125 explicit opt-out; chosen in source)"
                        .to_string(),
                });
            }
        }
    }

    fn walk_stmt(
        stmt: &Statement,
        has_vision_decl: bool,
        errors_only: bool,
        source: &str,
        findings: &mut Vec<AuditFinding>,
    ) {
        match stmt {
            Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
                walk_expr_deep(value, has_vision_decl, errors_only, source, findings);
            }
            Statement::ExprStmt { expr, .. } | Statement::Return { value: expr, .. } => {
                walk_expr_deep(expr, has_vision_decl, errors_only, source, findings);
            }
            Statement::Each { body, .. }
            | Statement::While { body, .. }
            | Statement::IfThen { body, .. } => {
                for s in body {
                    walk_stmt(s, has_vision_decl, errors_only, source, findings);
                }
            }
            Statement::IfElseBlock {
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                for s in then_body {
                    walk_stmt(s, has_vision_decl, errors_only, source, findings);
                }
                for (_, body) in else_ifs {
                    for s in body {
                        walk_stmt(s, has_vision_decl, errors_only, source, findings);
                    }
                }
                if let Some(body) = else_body {
                    for s in body {
                        walk_stmt(s, has_vision_decl, errors_only, source, findings);
                    }
                }
            }
            _ => {}
        }
    }

    fn walk_expr_deep(
        expr: &Expr,
        has_vision_decl: bool,
        errors_only: bool,
        source: &str,
        findings: &mut Vec<AuditFinding>,
    ) {
        check_expr(expr, has_vision_decl, errors_only, source, findings);
        if let Expr::FnCall { args, .. } = expr {
            for arg in args {
                walk_expr_deep(arg, has_vision_decl, errors_only, source, findings);
            }
        }
    }

    fn analyze_scope(
        stmts: &[Statement],
        has_vision_decl: bool,
        errors_only: bool,
        source: &str,
        findings: &mut Vec<AuditFinding>,
    ) {
        for s in stmts {
            walk_stmt(s, has_vision_decl, errors_only, source, findings);
        }
    }

    for decl in declarations {
        match decl {
            Declaration::MlogServer(srv) => {
                for route in &srv.routes {
                    analyze_scope(&route.body, has_vision_decl, errors_only, source, findings);
                }
            }
            Declaration::Pattern(p) => {
                analyze_scope(&p.body, has_vision_decl, errors_only, source, findings)
            }
            Declaration::Tool(t) => {
                for m in &t.methods {
                    analyze_scope(&m.body, has_vision_decl, errors_only, source, findings);
                }
            }
            Declaration::Hook(h) => {
                analyze_scope(&h.body, has_vision_decl, errors_only, source, findings)
            }
            _ => {}
        }
    }
}

// ── Check: MODEL_WEIGHTS_UNSAFE + VISION_POLICY_MISSING ──────────────
//
// Наряд №241 (R5, Block 3 — ADR-0125 Category-A gates).
//
// MODEL_WEIGHTS_UNSAFE (Severity::Error, Category A) — statically visible
// violations at `vision_fetch_weights(url, ...)` call sites:
//   1. literal URL whose host is in the SSRF-blocked class (loopback /
//      private / link-local / metadata IPs, `localhost`) — such a target
//      is unreachable through the SSRF guard no matter what the runtime
//      allowlist says;
//   2. literal URL pointing at a bare `.safetensors` file — no manifest,
//      no pinned-SHA source ("конструкция загрузки без pin");
//   3. literal URL with a pickle-RCE-class extension (.pkl/.pt/.pth/...)
//      — non-safetensors class refused by construction.
// The RUNTIME layers (allowlist default-deny env MLOG_VISION_WEIGHTS_
// ALLOWLIST + SSRF resolve-pinning + per-entry SHA pinning) live in
// `vision_fetch_weights` — both layers, exact names, per ADR-0125.
// The gate is written reusably in audit.rs (SSOT for the Voice pillar's
// equivalent AUDIO gates per ADR-0125).
//
// VISION_POLICY_MISSING (Severity::Warning, advisory — `mlog audit`):
// a `vision { }` block whose `policy:` field is omitted. ADR-0125 table
// says "warning; policy validated statically" — the audit IS the static
// validation. NOT wired into audit_category_a: semantic.rs №98 promotes
// every Warning there to a compile error, which would contradict the
// ADR's "warning" severity and the prescribed parser relax (Block 3.1).

/// Pickle-RCE-class extensions — same refusal list as the runtime
/// `vision_fetch_weights` gate (defense-in-depth, one vocabulary).
const VISION_PICKLE_CLASS_EXTS: &[&str] = &[
    ".pkl", ".pickle", ".pt", ".pth", ".ckpt", ".bin", ".py", ".so", ".dll", ".exe", ".zip",
    ".tar", ".gz", ".7z",
];

/// SSOT list of weights-fetch function names (Наряд №300, issue #368).
/// Adding a new pillar's weights-fetch builtin (e.g., `voice_fetch_weights`)
/// is a one-line append — no changes to `walk_expr_deep` logic needed.
/// Additionally, any function name ending in `_fetch_weights` is automatically
/// covered by the suffix convention — no list update required at all.
const WEIGHTS_FETCH_FNS: &[&str] = &["vision_fetch_weights"];

/// Check if a function name is a weights-fetch function. Returns true if:
/// (a) the name is in `WEIGHTS_FETCH_FNS` (explicit SSOT list), OR
/// (b) the name ends with `_fetch_weights` (suffix convention).
/// This ensures new pillar weights-fetch builtins are covered automatically.
fn is_weights_fetch_fn(name: &str) -> bool {
    WEIGHTS_FETCH_FNS.contains(&name) || name.ends_with("_fetch_weights")
}

fn check_model_weights_unsafe(
    declarations: &[Declaration],
    source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    fn literal_url_violation(url: &str) -> Option<String> {
        let parsed = reqwest::Url::parse(url).ok()?;
        let host = parsed.host_str()?.to_lowercase();
        // 1. SSRF-blocked host class.
        let blocked = if host == "localhost" || host.ends_with(".localhost") {
            true
        } else if let Ok(ip) = host.parse::<std::net::IpAddr>() {
            crate::builtins::http::is_blocked_address(&ip)
        } else {
            false
        };
        if blocked {
            return Some(format!(
                // Наряд №261: классы блокировки расширены SSOT is_blocked_address
                // (mapped-IPv6, unspecified, CGNAT 100.64/10, benchmark 198.18/15).
                "URL host '{}' is in the SSRF-blocked class (loopback/private/link-local/\
                 reserved/mapped) \
                 — unreachable through the SSRF guard regardless of the allowlist",
                host
            ));
        }
        let path = parsed.path().to_lowercase();
        // 2. Bare weights file — no manifest, no pin.
        if path.ends_with(".safetensors") {
            return Some(
                "URL points at a bare .safetensors file — no manifest, no pinned SHA-256 source; \
                 fetch the manifest.json of the package instead"
                    .to_string(),
            );
        }
        // 3. Pickle-RCE class.
        if VISION_PICKLE_CLASS_EXTS
            .iter()
            .any(|ext| path.ends_with(ext))
        {
            return Some(
                "URL is not a manifest.json-class path — pickle-RCE-class weight formats \
                 are refused (non-safetensors)"
                    .to_string(),
            );
        }
        None
    }

    fn walk_stmt(stmt: &Statement, source: &str, findings: &mut Vec<AuditFinding>) {
        match stmt {
            Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
                walk_expr_deep(value, source, findings);
            }
            Statement::ExprStmt { expr, .. } | Statement::Return { value: expr, .. } => {
                walk_expr_deep(expr, source, findings);
            }
            Statement::Each { body, .. }
            | Statement::While { body, .. }
            | Statement::IfThen { body, .. } => {
                for s in body {
                    walk_stmt(s, source, findings);
                }
            }
            Statement::IfElseBlock {
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                for s in then_body {
                    walk_stmt(s, source, findings);
                }
                for (_, body) in else_ifs {
                    for s in body {
                        walk_stmt(s, source, findings);
                    }
                }
                if let Some(body) = else_body {
                    for s in body {
                        walk_stmt(s, source, findings);
                    }
                }
            }
            _ => {}
        }
    }

    fn walk_expr_deep(expr: &Expr, source: &str, findings: &mut Vec<AuditFinding>) {
        if let Expr::FnCall {
            name: fn_name,
            args,
            ..
        } = expr
        {
            if is_weights_fetch_fn(fn_name) {
                if let Some(Expr::StringLit { value, .. }) = args.first() {
                    if let Some(violation) = literal_url_violation(value) {
                        let line = find_line(source, fn_name.as_str());
                        findings.push(AuditFinding {
                            severity: Severity::Error,
                            check_id: "MODEL_WEIGHTS_UNSAFE",
                            line,
                            message: format!(
                                "{}: {} (ADR-0125; runtime layers: allowlist \
                                 default-deny MLOG_VISION_WEIGHTS_ALLOWLIST + SSRF guard + SHA pinning)",
                                fn_name, violation
                            ),
                        });
                    }
                }
            }
            for arg in args {
                walk_expr_deep(arg, source, findings);
            }
        }
    }

    for decl in declarations {
        match decl {
            Declaration::MlogServer(srv) => {
                for route in &srv.routes {
                    for s in &route.body {
                        walk_stmt(s, source, findings);
                    }
                }
            }
            Declaration::Pattern(p) => {
                for s in &p.body {
                    walk_stmt(s, source, findings);
                }
            }
            Declaration::Tool(t) => {
                for m in &t.methods {
                    for s in &m.body {
                        walk_stmt(s, source, findings);
                    }
                }
            }
            Declaration::Hook(h) => {
                for s in &h.body {
                    walk_stmt(s, source, findings);
                }
            }
            _ => {}
        }
    }
}

// ── Check: MEDIA_SYNTHETIC_UNMARKED (Наряд №320, ADR-0152) ───────────
//
// EU AI Act Art. 50 — marking of synthetic content (window closes
// 2026-12-02). Every locally generated vision artifact is synthetic by
// construction (generation is the only artifact writer), so a
// `vision_export_raw` call site IS the statically visible attempt to
// egress synthetic content without its manifest. Category-A Error by the
// MODEL_WEIGHTS_UNSAFE / VISION_UNSIGNED_EXPORT template (1607–1757).
// The №241 VISION_UNSIGNED_EXPORT_RAW advisory Warning is unchanged — it
// records the opt-out intent; THIS gate enforces the marking. Runtime
// backstop lives in vision_export_raw_dispatch (ADR-0152 D3).
fn check_media_synthetic_unmarked(
    declarations: &[Declaration],
    source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    fn walk_stmt(stmt: &Statement, source: &str, findings: &mut Vec<AuditFinding>) {
        match stmt {
            Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
                walk_expr_deep(value, source, findings);
            }
            Statement::ExprStmt { expr, .. } | Statement::Return { value: expr, .. } => {
                walk_expr_deep(expr, source, findings);
            }
            Statement::Each { body, .. }
            | Statement::While { body, .. }
            | Statement::IfThen { body, .. } => {
                for s in body {
                    walk_stmt(s, source, findings);
                }
            }
            Statement::IfElseBlock {
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                for s in then_body {
                    walk_stmt(s, source, findings);
                }
                for (_, body) in else_ifs {
                    for s in body {
                        walk_stmt(s, source, findings);
                    }
                }
                if let Some(body) = else_body {
                    for s in body {
                        walk_stmt(s, source, findings);
                    }
                }
            }
            _ => {}
        }
    }

    fn walk_expr_deep(expr: &Expr, source: &str, findings: &mut Vec<AuditFinding>) {
        if let Expr::FnCall { name: fn_name, .. } = expr {
            if fn_name == "vision_export_raw" {
                let line = find_line(source, fn_name);
                findings.push(AuditFinding {
                    severity: Severity::Error,
                    check_id: "MEDIA_SYNTHETIC_UNMARKED",
                    line,
                    message: "vision_export_raw call site — raw egress ships no provenance \
                              manifest; locally generated artifacts are synthetic by \
                              construction, so this is an unmarked synthetic-media egress \
                              (EU AI Act Art. 50, ADR-0152 D2; runtime backstop refuses \
                              synthetic/manifest-less artifacts). Use vision_export (signed \
                              sidecar egress)"
                        .to_string(),
                });
            }
        }
        if let Expr::FnCall { name: _, args, .. } = expr {
            for arg in args {
                walk_expr_deep(arg, source, findings);
            }
        }
    }

    for decl in declarations {
        match decl {
            Declaration::MlogServer(srv) => {
                for route in &srv.routes {
                    for s in &route.body {
                        walk_stmt(s, source, findings);
                    }
                }
            }
            Declaration::Pattern(p) => {
                for s in &p.body {
                    walk_stmt(s, source, findings);
                }
            }
            Declaration::Tool(t) => {
                for m in &t.methods {
                    for s in &m.body {
                        walk_stmt(s, source, findings);
                    }
                }
            }
            Declaration::Hook(h) => {
                for s in &h.body {
                    walk_stmt(s, source, findings);
                }
            }
            _ => {}
        }
    }
}

fn check_vision_policy_missing(
    declarations: &[Declaration],
    _source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    for decl in declarations {
        if let Declaration::Vision(v) = decl {
            if v.policy.is_none() {
                findings.push(AuditFinding {
                    severity: Severity::Warning,
                    check_id: "VISION_POLICY_MISSING",
                    line: v.span.start_line.max(1) as usize,
                    message: format!(
                        "vision '{}' has no `policy:` field — honest use is not explicit \
                         (ADR-0125: policy makes it explicit; manifest records \"policy\": \"unspecified\")",
                        v.name
                    ),
                });
            }
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
// this is a taint-through-persistence violation. Does NOT track precise
// data-flow through the memory subsystem — but the pattern IS a real
// security issue (FOSVED: HandleX → AuditLog → db_execute).
// Promoted to Error / Category A by Наряд #157.

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
                severity: Severity::Error,
                check_id: "TAINT_PERSISTENCE",
                line,
                message: "potential taint through memorize/recall — LLM output was memorized in this file and recall() result may reach respond(); use render()/escape_html() on recalled data"
                    .to_string(),
            });
            // One finding per file is enough
            return;
        }
    }
}

// ── Check: TAINT_PASSTHROUGH_PATTERN — respond(Wrap(call_llm(...))) ────────
// Наряд #141: heuristic for trivial passthrough patterns. If a pattern
// has exactly one parameter and its only return is that parameter
// (trivial passthrough like `pattern Wrap(x) { return x }`), and it is
// called wrapping an LLM source in a respond/respond_html sink, this is
// an XSS vector — the LLM output reaches the user without sanitization.
// Promoted to Error / Category A by Наряд #157.

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
                                severity: Severity::Error,
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

// ── Наряд №292 (P0, security): TAINT_INTERP — interprocedural taint MVP ──
//
// Summary-based interprocedural taint. The existing TAINT_PASSTHROUGH
// (Наряд #141/#157) catches only the *trivial* passthrough: a pattern
// with exactly one parameter whose body is `return <param>`. Real
// Fosved-class dept-handlers wrap LLM output through non-trivial helpers
// (e.g. `pattern Wrap(x) { return upper(x) }` + `respond(Wrap(call_llm(...)))`),
// which the trivial check misses.
//
// Approach: compute a summary for each `pattern` declaration — which
// parameters (by position) flow into the return value, possibly through
// calls to OTHER user-patterns. Propagate taint through 1–2 levels of
// calls (bounded, no fixpoint). Recursion/loops in the call graph →
// loud warning INTERP_DEPTH_LIMIT (not Error — analysis terminates with
// the boundary documented).
//
// **Zero false positives on legitimate code**: render/escape_html are
// sanitizers — `respond(render(...))` and `respond(escape_html(...))`
// are NOT flagged (test contract (в) in issue #355).

/// Maximum call-graph depth explored by `check_taint_interp_pattern`.
/// Bounded — no fixpoint analysis. Patterns deeper than this in the
/// call graph are flagged with `INTERP_DEPTH_LIMIT` warning (analysis
/// terminates cleanly, the boundary is documented in README + threat-model).
/// №376: the interprocedural taint depth limit is CONFIGURABLE via the
/// `METALOGOS_TAINT_DEPTH` env var (integer, 1..=16; unset or invalid → the
/// measured default below). The `INTERP_DEPTH_LIMIT` warning and the
/// `bounded_recursion` flag are PRESERVED — the limit moved, the loudness
/// stayed.
///
/// Default chosen by the №376 overhead measurement (see the наряд report):
/// auditing the 222-file examples corpus, depth 2 → 4 cost +14.2% cold
/// analysis time (59.7 ms → 68.1 ms; depth 8 → +23.8%) — within the +50%
/// dispatch threshold, so the default is 4 (closes the depth-3/4 coverage
/// hole for office dept/chain patterns).
const DEFAULT_TAINT_INTERP_MAX_DEPTH: usize = 4;

/// Read the configured depth (once per audit run). Values outside 1..=16 or
/// non-numeric fall back to the default.
fn taint_interp_max_depth() -> usize {
    match std::env::var("METALOGOS_TAINT_DEPTH") {
        Ok(v) => match v.trim().parse::<usize>() {
            Ok(d) if (1..=16).contains(&d) => d,
            _ => DEFAULT_TAINT_INTERP_MAX_DEPTH,
        },
        Err(_) => DEFAULT_TAINT_INTERP_MAX_DEPTH,
    }
}

/// №376: cross-module summaries cache. Key = FNV-1a hash of the module
/// source; value = the computed summaries. A workspace audit run visits
/// many modules — an UNCHANGED module's summaries are computed once and
/// reused on the next run/visit (recalculation only when the module
/// content changes, per the наряд contract). Counters are exposed for
/// tests via [`summaries_cache_stats`].
type SummariesByModule =
    std::collections::HashMap<(u64, usize), std::collections::HashMap<String, PatternSummary>>;

static SUMMARIES_CACHE: std::sync::LazyLock<std::sync::Mutex<SummariesByModule>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));
static SUMMARIES_CACHE_INSERTS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static SUMMARIES_CACHE_HITS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn fnv1a_source(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// #[doc(hidden)] test/observability hook: (cache inserts, cache hits).
#[doc(hidden)]
pub fn summaries_cache_stats() -> (u64, u64) {
    (
        SUMMARIES_CACHE_INSERTS.load(std::sync::atomic::Ordering::SeqCst),
        SUMMARIES_CACHE_HITS.load(std::sync::atomic::Ordering::SeqCst),
    )
}

/// Clear the summaries cache (test isolation / forced recompute).
#[doc(hidden)]
pub fn summaries_cache_clear() {
    SUMMARIES_CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
    SUMMARIES_CACHE_INSERTS.store(0, std::sync::atomic::Ordering::SeqCst);
    SUMMARIES_CACHE_HITS.store(0, std::sync::atomic::Ordering::SeqCst);
}

/// Summary of a `pattern` declaration for interprocedural taint analysis.
///
/// `params_tainting_return` holds the indices (0-based, into `params`)
/// of parameters that flow — directly or through user-pattern calls —
/// into the pattern's `return` expression. If `return` is not present
/// (control-flow falls through), the set is empty.
///
/// `bounded_recursion` is true when the pattern appears (directly or
/// transitively through other patterns) in its own call chain. The
/// analysis still computes the summary (best-effort), but emits a
/// loud `INTERP_DEPTH_LIMIT` warning so the boundary is visible.
#[derive(Debug, Default, Clone)]
struct PatternSummary {
    params_tainting_return: std::collections::HashSet<usize>,
    bounded_recursion: bool,
}

/// Compute summaries for all `pattern` declarations. Returns a map
/// keyed by pattern name. Recursion / call cycles are detected via a
/// visited-set during traversal; `bounded_recursion` is set on every
/// pattern that participates in a cycle.
fn compute_pattern_summaries_with_depth(
    declarations: &[Declaration],
    max_depth: usize,
) -> std::collections::HashMap<String, PatternSummary> {
    use std::collections::{HashMap, HashSet};

    // First pass: index patterns by name + collect the raw (pre-propagation)
    // summary — which params directly flow into `return`.
    let mut raw_summaries: HashMap<String, PatternSummary> = HashMap::new();
    let mut pattern_bodies: HashMap<String, (&[crate::ast::Param], &[crate::ast::Statement])> =
        HashMap::new();
    for decl in declarations {
        if let Declaration::Pattern(p) = decl {
            let mut summary = PatternSummary::default();
            // Walk the body, collect `return <expr>` statements — for each,
            // find which params contribute.
            collect_params_into_return(&p.body, &p.params, &mut summary.params_tainting_return);
            raw_summaries.insert(p.name.clone(), summary);
            pattern_bodies.insert(p.name.clone(), (&p.params, &p.body));
        }
    }

    // Second pass: propagate taint through user-pattern calls. Bounded
    // depth = the configured `taint_interp_max_depth()`. We track a visited set per starting
    // pattern so cycles are detected and `bounded_recursion` is set.
    let pattern_names: HashSet<String> = raw_summaries.keys().cloned().collect();
    let mut propagated: HashMap<String, PatternSummary> = raw_summaries.clone();

    for start_name in pattern_names.iter() {
        let mut visited: HashSet<String> = HashSet::new();
        visited.insert(start_name.clone());
        propagate_params(
            start_name,
            &pattern_bodies,
            &pattern_names,
            &mut propagated,
            &mut visited,
            0,
            max_depth,
        );
    }

    propagated
}

/// Helper for `compute_pattern_summaries` — traverse `body`, for each
/// `return <expr>` statement mark which params (by index) directly
/// appear in `expr` (Ident references to param names).
fn collect_params_into_return(
    body: &[crate::ast::Statement],
    params: &[crate::ast::Param],
    out: &mut std::collections::HashSet<usize>,
) {
    for stmt in body {
        match stmt {
            crate::ast::Statement::Return { value, .. } => {
                collect_params_in_expr(value, params, out);
            }
            crate::ast::Statement::Each { body, .. }
            | crate::ast::Statement::EachWithIndex { body, .. }
            | crate::ast::Statement::While { body, .. }
            | crate::ast::Statement::IfThen { body, .. } => {
                collect_params_into_return(body, params, out);
            }
            crate::ast::Statement::IfElseBlock {
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                collect_params_into_return(then_body, params, out);
                for (_, b) in else_ifs {
                    collect_params_into_return(b, params, out);
                }
                if let Some(b) = else_body {
                    collect_params_into_return(b, params, out);
                }
            }
            _ => {}
        }
    }
}

/// Walk `expr` and mark every param (by index) whose name appears as
/// an `Expr::Ident` directly. Does NOT traverse into user-pattern call
/// args — that's the propagation pass's job.
fn collect_params_in_expr(
    expr: &crate::ast::Expr,
    params: &[crate::ast::Param],
    out: &mut std::collections::HashSet<usize>,
) {
    match expr {
        crate::ast::Expr::Ident { name, .. } => {
            for (i, p) in params.iter().enumerate() {
                if &p.name == name {
                    out.insert(i);
                }
            }
        }
        crate::ast::Expr::FnCall { args, .. } => {
            for a in args {
                collect_params_in_expr(a, params, out);
            }
        }
        crate::ast::Expr::BinaryOp { left, right, .. } => {
            collect_params_in_expr(left, params, out);
            collect_params_in_expr(right, params, out);
        }

        crate::ast::Expr::FieldAccess { object, .. } => {
            collect_params_in_expr(object, params, out);
        }
        crate::ast::Expr::IndexAccess { object, index, .. } => {
            collect_params_in_expr(object, params, out);
            collect_params_in_expr(index, params, out);
        }
        crate::ast::Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            collect_params_in_expr(condition, params, out);
            collect_params_in_expr(then_branch, params, out);
            collect_params_in_expr(else_branch, params, out);
        }
        _ => {}
    }
}

/// Recursive propagation: for the starting `pattern_name`, traverse
/// the body looking for `return <expr>` statements; for any user-pattern
/// call inside the return, mark the called pattern's params-tainting-return
/// as contributing to the starting pattern's return (if not already).
///
/// Bounded by `depth < max_depth` (the configured limit). Cycles → `bounded_recursion`
/// flag is set on the calling pattern.
fn propagate_params(
    pattern_name: &str,
    pattern_bodies: &std::collections::HashMap<
        String,
        (&[crate::ast::Param], &[crate::ast::Statement]),
    >,
    pattern_names: &std::collections::HashSet<String>,
    propagated: &mut std::collections::HashMap<String, PatternSummary>,
    visited: &mut std::collections::HashSet<String>,
    depth: usize,
    max_depth: usize,
) {
    if depth >= max_depth {
        return;
    }
    // Get the (params, body) for this pattern; if missing, nothing to do.
    let Some((params, body)) = pattern_bodies.get(pattern_name) else {
        return;
    };
    let params: &[crate::ast::Param] = params;
    let body: &[crate::ast::Statement] = body;

    // Find user-pattern calls inside return expressions of this body.
    let mut calls_to_propagate: Vec<(String, Vec<Option<usize>>)> = Vec::new();
    for stmt in body {
        if let crate::ast::Statement::Return { value, .. } = stmt {
            // Find user-pattern calls in this return expression; for each,
            // record which params (by index) of THIS pattern are passed in
            // which arg-position of the called pattern.
            find_user_pattern_calls(value, params, pattern_names, &mut calls_to_propagate);
        }
    }

    for (called_name, caller_param_indices) in calls_to_propagate {
        // Cycle detection: if `called_name` is already in `visited`, mark
        // `bounded_recursion` on the current pattern + skip recursion.
        if visited.contains(&called_name) {
            if let Some(s) = propagated.get_mut(pattern_name) {
                s.bounded_recursion = true;
            }
            if let Some(s) = propagated.get_mut(&called_name) {
                s.bounded_recursion = true;
            }
            continue;
        }
        // Get the called pattern's summary; its `params_tainting_return`
        // are the param-indices of `called_name` whose values flow into
        // `called_name`'s return. Map those back to caller pattern's
        // param indices. Clone the set to release the immutable borrow
        // before the mutable borrow below.
        let Some(called_summary) = propagated.get(&called_name) else {
            continue;
        };
        let called_tainting: std::collections::HashSet<usize> =
            called_summary.params_tainting_return.clone();
        // `propagated` was built from the same `pattern_bodies` keys as
        // `pattern_name` (which came from iterating the same map), so this
        // is guaranteed to be Some. The borrow-checker-pleasing form
        // `match ... { Some(s) => s, None => return }` avoids `expect()`
        // (clippy::expect_used is denied for non-test code in lib.rs).
        let Some(caller_summary) = propagated.get_mut(pattern_name) else {
            continue;
        };
        for &called_param_idx in &called_tainting {
            // `caller_param_indices[called_param_idx]` (if Some) is the
            // caller-pattern param index that flows through `called_name`'s
            // param `called_param_idx` into `called_name`'s return, which
            // in turn flows into the caller's return.
            if let Some(Some(caller_idx)) = caller_param_indices.get(called_param_idx) {
                caller_summary.params_tainting_return.insert(*caller_idx);
            }
        }
        // Recurse: visited now includes `called_name`, depth+1.
        visited.insert(called_name.clone());
        propagate_params(
            &called_name,
            pattern_bodies,
            pattern_names,
            propagated,
            visited,
            depth + 1,
            max_depth,
        );
        visited.remove(&called_name);
    }
}

/// Walk `expr` and find user-pattern calls. For each call, record a
/// `(called_name, Vec<caller_param_idx>)` mapping — `Vec[calling_idx]`
/// is the index (into `params`) of the caller-pattern param passed as
/// the `calling_idx`-th argument of the called pattern. If the arg is
/// not a direct param reference, the slot is None — but the call still
/// propagates other args.
///
/// Note: we do NOT traverse into the called pattern's body here — that's
/// the propagation pass's job. We just identify the call sites and which
/// caller-params flow into which called-param positions.
fn find_user_pattern_calls(
    expr: &crate::ast::Expr,
    params: &[crate::ast::Param],
    pattern_names: &std::collections::HashSet<String>,
    out: &mut Vec<(String, Vec<Option<usize>>)>,
) {
    match expr {
        crate::ast::Expr::FnCall { name, args, .. } => {
            if pattern_names.contains(name.as_str()) {
                // Build the caller-param-index mapping for this call.
                let mapping: Vec<Option<usize>> = args
                    .iter()
                    .map(|arg| {
                        if let crate::ast::Expr::Ident { name: arg_name, .. } = arg {
                            params.iter().position(|p| &p.name == arg_name)
                        } else {
                            None
                        }
                    })
                    .collect();
                out.push((name.clone(), mapping));
            }
            // Recurse into args regardless — nested user-pattern calls matter.
            for a in args {
                find_user_pattern_calls(a, params, pattern_names, out);
            }
        }
        crate::ast::Expr::BinaryOp { left, right, .. } => {
            find_user_pattern_calls(left, params, pattern_names, out);
            find_user_pattern_calls(right, params, pattern_names, out);
        }

        crate::ast::Expr::FieldAccess { object, .. } => {
            find_user_pattern_calls(object, params, pattern_names, out);
        }
        crate::ast::Expr::IndexAccess { object, index, .. } => {
            find_user_pattern_calls(object, params, pattern_names, out);
            find_user_pattern_calls(index, params, pattern_names, out);
        }
        crate::ast::Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            find_user_pattern_calls(condition, params, pattern_names, out);
            find_user_pattern_calls(then_branch, params, pattern_names, out);
            find_user_pattern_calls(else_branch, params, pattern_names, out);
        }
        _ => {}
    }
}

/// Check: TAINT_INTERP — respond/respond_html/write_file/print sink
/// receiving the result of a user-pattern call wrapping an LLM source.
///
/// Catches the case `check_taint_passthrough_pattern` misses: non-trivial
/// patterns where `return <param>` is wrapped in another expression
/// (e.g. `return upper(x)`), or chains through 2 user-pattern calls
/// (`respond(Wrap2(Wrap1(call_llm(...))))`).
///
/// **Sanitizers take precedence** (test contract (в)): if the LLM source
/// is wrapped in `render(...)` or `escape_html(...)` BEFORE reaching
/// the user-pattern call, the taint is lifted — no finding is emitted.
/// This mirrors the intra-procedural `binding_taint` behavior.
///
/// **Depth limit**: bounded to the configured `taint_interp_max_depth()` (№376, default 4). If a
/// pattern is detected as part of a call cycle (`bounded_recursion` flag),
/// emit `INTERP_DEPTH_LIMIT` warning — analysis terminated cleanly.
fn check_taint_interp_pattern(
    declarations: &[Declaration],
    source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    // №376: the configured depth (one read per audit run) + the cross-module
    // summaries cache — recompute only when the module (source) changed.
    let max_depth = taint_interp_max_depth();
    // The cache key includes the depth: the same module measured at a
    // different `METALOGOS_TAINT_DEPTH` must recompute (summaries differ).
    let source_key = (fnv1a_source(source.as_bytes()), max_depth);
    let summaries = {
        let mut cache = SUMMARIES_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(cached) = cache.get(&source_key) {
            SUMMARIES_CACHE_HITS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            cached.clone()
        } else {
            let computed = compute_pattern_summaries_with_depth(declarations, max_depth);
            cache.insert(source_key, computed.clone());
            SUMMARIES_CACHE_INSERTS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            computed
        }
    };

    // If there are no patterns at all, nothing to check.
    if summaries.is_empty() {
        return;
    }

    // Emit INTERP_DEPTH_LIMIT warnings for patterns in cycles —
    // the boundary is documented loudly (issue #355 contract (г)).
    for (name, summary) in &summaries {
        if summary.bounded_recursion {
            let line = find_line(source, name);
            findings.push(AuditFinding {
                severity: Severity::Warning,
                check_id: "INTERP_DEPTH_LIMIT",
                line,
                message: format!(
                    "pattern `{}` participates in a call cycle — interprocedural taint analysis bounded at depth {}, the cycle is not fully explored",
                    name, max_depth
                ),
            });
        }
    }

    // For each sink call (respond/respond_html/write_file/print), check
    // if any arg is a user-pattern call whose summary says some param
    // taints the return, and that param's corresponding arg-expression
    // contains an LLM source.
    let sink_names = ["respond", "respond_html", "write_file", "print"];

    fn is_sanitized_expr(expr: &crate::ast::Expr) -> bool {
        // render / escape_html wrapping anything is sanitized — taint lifted.
        if let crate::ast::Expr::FnCall { name, .. } = expr {
            if name == "render" || name == "escape_html" {
                return true;
            }
        }
        false
    }

    /// Walk `expr` and find sink calls. For each sink call's args, check
    /// for user-pattern calls wrapping LLM sources (with sanitization
    /// override).
    fn check_sink_calls(
        expr: &crate::ast::Expr,
        summaries: &std::collections::HashMap<String, PatternSummary>,
        sink_names: &[&str; 4],
        source: &str,
        findings: &mut Vec<AuditFinding>,
    ) {
        match expr {
            crate::ast::Expr::FnCall { name, args, .. } => {
                if sink_names.contains(&name.as_str()) {
                    for arg in args {
                        // Sanitizer wraps the arg → safe, skip.
                        if is_sanitized_expr(arg) {
                            continue;
                        }
                        if let Some(finding_line) =
                            check_user_pattern_call_for_taint(arg, summaries, source)
                        {
                            findings.push(AuditFinding {
                                severity: Severity::Error,
                                check_id: "TAINT_INTERP",
                                line: finding_line,
                                message: format!(
                                    "LLM output reaches {}() via interprocedural pattern call — use render()/escape_html() for XSS safety",
                                    name
                                ),
                            });
                        }
                    }
                }
                // Recurse into nested calls — sinks can be nested.
                for a in args {
                    check_sink_calls(a, summaries, sink_names, source, findings);
                }
            }
            crate::ast::Expr::BinaryOp { left, right, .. } => {
                check_sink_calls(left, summaries, sink_names, source, findings);
                check_sink_calls(right, summaries, sink_names, source, findings);
            }

            crate::ast::Expr::IfElse {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                check_sink_calls(condition, summaries, sink_names, source, findings);
                check_sink_calls(then_branch, summaries, sink_names, source, findings);
                check_sink_calls(else_branch, summaries, sink_names, source, findings);
            }
            _ => {}
        }
    }

    /// Check if `expr` is a user-pattern call wrapping an LLM source (directly
    /// or through 1-2 levels of pattern calls). Returns `Some(line)` if a
    /// finding should be emitted, `None` otherwise.
    fn check_user_pattern_call_for_taint(
        expr: &crate::ast::Expr,
        summaries: &std::collections::HashMap<String, PatternSummary>,
        source: &str,
    ) -> Option<usize> {
        let crate::ast::Expr::FnCall { name, args, .. } = expr else {
            return None;
        };
        let summary = summaries.get(name)?;
        // For each param-index that taints the return of this pattern,
        // check if the corresponding arg-expression contains an LLM source
        // OR is itself a user-pattern call wrapping LLM source (recursively,
        // bounded by summary depth).
        for (i, arg) in args.iter().enumerate() {
            if !summary.params_tainting_return.contains(&i) {
                continue;
            }
            // Does this arg contain an LLM source?
            if expr_contains_llm_source_direct(arg) {
                return Some(find_line(source, name));
            }
            // Is this arg itself a user-pattern call wrapping LLM source?
            if let Some(line) = check_user_pattern_call_for_taint(arg, summaries, source) {
                return Some(line);
            }
        }
        None
    }

    /// Walk `expr` and return true if any sub-expression is a direct LLM
    /// source (call_llm, call_claude, reflex_generate). Sanitizers
    /// (render/escape_html) wrapping the LLM source lift the taint.
    fn expr_contains_llm_source_direct(expr: &crate::ast::Expr) -> bool {
        match expr {
            crate::ast::Expr::FnCall { name, args, .. } => {
                if is_llm_source(name) {
                    return true;
                }
                // Sanitizer wraps the call → safe.
                if name == "render" || name == "escape_html" {
                    return false;
                }
                // Recurse into args.
                args.iter().any(expr_contains_llm_source_direct)
            }
            crate::ast::Expr::BinaryOp { left, right, .. } => {
                expr_contains_llm_source_direct(left) || expr_contains_llm_source_direct(right)
            }
            _ => false,
        }
    }

    // Walk all declarations looking for sink calls with interprocedural
    // LLM-tainted args.
    for decl in declarations {
        let mut exprs: Vec<&crate::ast::Expr> = Vec::new();
        match decl {
            Declaration::Pattern(p) => collect_stmt_exprs_interp(&p.body, &mut exprs),
            Declaration::Tool(t) => {
                for m in &t.methods {
                    collect_stmt_exprs_interp(&m.body, &mut exprs);
                }
            }
            Declaration::MlogServer(srv) => {
                for route in &srv.routes {
                    collect_stmt_exprs_interp(&route.body, &mut exprs);
                }
            }
            Declaration::Hook(h) => collect_stmt_exprs_interp(&h.body, &mut exprs),
            _ => continue,
        }

        for expr in &exprs {
            check_sink_calls(expr, &summaries, &sink_names, source, findings);
        }
    }
}

/// Category-A-only variant of `check_taint_interp_pattern`. Promotes the
/// Errors (TAINT_INTERP) but drops the Warnings (INTERP_DEPTH_LIMIT) —
/// the boundary is informational and stays in `audit_program` (advisory).
/// Mirrors `check_vision_export_gates_errors_only`'s discipline (Наряд №241).
fn check_taint_interp_pattern_errors_only(
    declarations: &[Declaration],
    source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    let mut tmp: Vec<AuditFinding> = Vec::new();
    check_taint_interp_pattern(declarations, source, &mut tmp);
    for f in tmp {
        if f.severity == Severity::Error {
            findings.push(f);
        }
    }
}

/// Helper — collect all expressions inside a statement body (mirrors
/// `check_taint_passthrough_pattern`'s `collect_stmt_exprs` but kept
/// local to avoid borrow issues).
fn collect_stmt_exprs_interp<'a>(stmts: &'a [Statement], acc: &mut Vec<&'a Expr>) {
    for stmt in stmts {
        match stmt {
            Statement::LetBinding { value, .. } => acc.push(value),
            Statement::Assign { value, .. } => acc.push(value),
            Statement::ExprStmt { expr, .. } => acc.push(expr),
            Statement::Return { value, .. } => acc.push(value),
            Statement::Each { body, .. } => collect_stmt_exprs_interp(body, acc),
            Statement::EachWithIndex { body, .. } => collect_stmt_exprs_interp(body, acc),
            Statement::While { body, .. } => collect_stmt_exprs_interp(body, acc),
            Statement::IfElseBlock {
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                collect_stmt_exprs_interp(then_body, acc);
                for (_, body) in else_ifs {
                    collect_stmt_exprs_interp(body, acc);
                }
                if let Some(body) = else_body {
                    collect_stmt_exprs_interp(body, acc);
                }
            }
            Statement::IfThen { body, .. } => collect_stmt_exprs_interp(body, acc),
            _ => {}
        }
    }
}

// ── Наряд №284 (P1, M1): CANARY_LEAK — «компрометированный канал» ──────

/// Static half of the canary contour (src/builtins/canary.rs — runtime
/// half: stderr warning + llm_usage().canary_leaks). Advisory-only:
/// included in audit_program, NOT in audit_category_a (a Warning there
/// would be promoted to a compile error, contradicting «детектор,
/// не гейт»).
///
/// Path-sensitive approximation:
///   - `let r = canary_check(resp, id)` (или inline-условие
///     `canary_check(resp, id).leaked`) регистрирует пару r → resp;
///   - в then-ветке `if (r.leaked) { ... }` resp помечается
///     TaintKind::CanaryLeak (форк трекера; else/после ветки — без метки);
///   - использование CanaryLeak-значения в sink (respond, http_post,
///     call_llm, call_claude, reflex_generate, mcp_call) → Warning
///     CANARY_LEAK. Решение об остановке — за автором.
///
/// Honest boundaries: ветки с инвертированным/сравнительным условием
/// (`not r.leaked`, `r.leaked == false`) не помечаются; выражения в
/// Memorize/Forget/Relate/Match-arms не обходятся; flows — pipeline-шаги,
/// не statements (their bodies are patterns) — границы зафиксированы.
fn check_canary_leak(declarations: &[Declaration], source: &str, findings: &mut Vec<AuditFinding>) {
    /// Sink-каналы вывода наружу / повторного входа недоверенного
    /// контента (эксфильтрация / re-injection loop).
    const CANARY_SINKS: &[&str] = &[
        "respond",
        "http_post",
        "call_llm",
        "call_claude",
        "reflex_generate",
        "mcp_call",
    ];

    fn is_canary_sink(name: &str) -> bool {
        CANARY_SINKS.contains(&name)
    }

    /// `canary_check(X, ...)` → имя проверяемой переменной X.
    fn canary_source(args: &[Expr]) -> Option<String> {
        match args.first() {
            Some(Expr::Ident { name, .. }) => Some(name.clone()),
            _ => None,
        }
    }

    /// Условие ветки утечки: `E.leaked`, где E — inline-вызов
    /// `canary_check(Ident(X), ...)` или `Ident(r)` при связи r → X.
    fn leak_branch_var(cond: &Expr, checks: &HashMap<String, String>) -> Option<String> {
        if let Expr::FieldAccess { object, field, .. } = cond {
            if field == "leaked" {
                match object.as_ref() {
                    Expr::FnCall { name, args, .. } if name == "canary_check" => {
                        return canary_source(args);
                    }
                    Expr::Ident { name: r, .. } => {
                        if let Some(src) = checks.get(r) {
                            return Some(src.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
        None
    }

    fn check_expr_for_canary(
        expr: &Expr,
        tracker: &TaintTracker,
        source: &str,
        findings: &mut Vec<AuditFinding>,
    ) {
        if let Expr::FnCall { name, args, .. } = expr {
            if is_canary_sink(name) {
                for arg in args {
                    if get_expr_taint(arg, tracker) == Some(TaintKind::CanaryLeak) {
                        findings.push(AuditFinding {
                            severity: Severity::Warning,
                            check_id: "CANARY_LEAK",
                            line: find_line(source, name),
                            message: format!(
                                "compromised channel: canary leak confirmed (canary_check \u{2192} leaked), the leaked response reaches {}() \u{2014} treat as attacker-controlled; detector, not gate (No. 284)",
                                name
                            ),
                        });
                        break;
                    }
                }
            }
            for arg in args {
                check_expr_for_canary(arg, tracker, source, findings);
            }
        }
        match expr {
            Expr::FieldAccess { object, .. } => {
                check_expr_for_canary(object, tracker, source, findings);
            }
            Expr::BinaryOp { left, right, .. } => {
                check_expr_for_canary(left, tracker, source, findings);
                check_expr_for_canary(right, tracker, source, findings);
            }
            Expr::IfElse {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                check_expr_for_canary(condition, tracker, source, findings);
                check_expr_for_canary(then_branch, tracker, source, findings);
                check_expr_for_canary(else_branch, tracker, source, findings);
            }
            Expr::List { items, .. } => {
                for item in items {
                    check_expr_for_canary(item, tracker, source, findings);
                }
            }
            _ => {}
        }
    }

    fn process_stmt(
        stmt: &Statement,
        tracker: &mut TaintTracker,
        checks: &mut HashMap<String, String>,
        source: &str,
        findings: &mut Vec<AuditFinding>,
    ) {
        match stmt {
            Statement::LetBinding { name, value, .. } => {
                check_expr_for_canary(value, tracker, source, findings);
                if let Expr::FnCall {
                    name: fn_name,
                    args,
                    ..
                } = value
                {
                    if fn_name == "canary_check" {
                        if let Some(src) = canary_source(args) {
                            checks.insert(name.clone(), src);
                        }
                    } else {
                        checks.remove(name);
                    }
                } else {
                    checks.remove(name);
                }
                if let Some(taint) = binding_taint(value, tracker) {
                    tracker.taint(name, taint);
                } else {
                    tracker.untaint(name);
                }
            }
            Statement::Assign { name, value, .. } => {
                check_expr_for_canary(value, tracker, source, findings);
                if let Expr::FnCall {
                    name: fn_name,
                    args,
                    ..
                } = value
                {
                    if fn_name == "canary_check" {
                        if let Some(src) = canary_source(args) {
                            checks.insert(name.clone(), src);
                        }
                    } else {
                        checks.remove(name);
                    }
                } else {
                    checks.remove(name);
                }
                if let Some(taint) = binding_taint(value, tracker) {
                    tracker.taint(name, taint);
                } else {
                    tracker.untaint(name);
                }
            }
            Statement::ExprStmt { expr, .. } => {
                check_expr_for_canary(expr, tracker, source, findings);
            }
            Statement::Return { value: expr, .. } => {
                check_expr_for_canary(expr, tracker, source, findings);
            }
            Statement::IfElseBlock {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                check_expr_for_canary(condition, tracker, source, findings);
                let leaked_src = leak_branch_var(condition, checks);
                match leaked_src {
                    Some(src) => {
                        // Path-sensitive fork: метка живёт ТОЛЬКО в then-ветке.
                        let mut fork = tracker.clone();
                        fork.taint(&src, TaintKind::CanaryLeak);
                        for s in then_body {
                            process_stmt(s, &mut fork, checks, source, findings);
                        }
                        for (_, body) in else_ifs {
                            for s in body {
                                process_stmt(s, tracker, checks, source, findings);
                            }
                        }
                        if let Some(body) = else_body {
                            for s in body {
                                process_stmt(s, tracker, checks, source, findings);
                            }
                        }
                    }
                    None => {
                        for s in then_body {
                            process_stmt(s, tracker, checks, source, findings);
                        }
                        for (_, body) in else_ifs {
                            for s in body {
                                process_stmt(s, tracker, checks, source, findings);
                            }
                        }
                        if let Some(body) = else_body {
                            for s in body {
                                process_stmt(s, tracker, checks, source, findings);
                            }
                        }
                    }
                }
            }
            Statement::IfThen {
                condition, body, ..
            } => {
                check_expr_for_canary(condition, tracker, source, findings);
                match leak_branch_var(condition, checks) {
                    Some(src) => {
                        let mut fork = tracker.clone();
                        fork.taint(&src, TaintKind::CanaryLeak);
                        for s in body {
                            process_stmt(s, &mut fork, checks, source, findings);
                        }
                    }
                    None => {
                        for s in body {
                            process_stmt(s, tracker, checks, source, findings);
                        }
                    }
                }
            }
            Statement::Each { iterable, body, .. }
            | Statement::EachWithIndex { iterable, body, .. } => {
                check_expr_for_canary(iterable, tracker, source, findings);
                for s in body {
                    process_stmt(s, tracker, checks, source, findings);
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                check_expr_for_canary(condition, tracker, source, findings);
                for s in body {
                    process_stmt(s, tracker, checks, source, findings);
                }
            }
            _ => {}
        }
    }

    fn analyze_body(stmts: &[Statement], source: &str, findings: &mut Vec<AuditFinding>) {
        let mut tracker = TaintTracker::new();
        let mut checks: HashMap<String, String> = HashMap::new();
        for s in stmts {
            process_stmt(s, &mut tracker, &mut checks, source, findings);
        }
    }

    for decl in declarations {
        match decl {
            Declaration::MlogServer(srv) => {
                for route in &srv.routes {
                    analyze_body(&route.body, source, findings);
                }
            }
            Declaration::Pattern(p) => analyze_body(&p.body, source, findings),
            Declaration::Tool(t) => {
                for m in &t.methods {
                    analyze_body(&m.body, source, findings);
                }
            }
            Declaration::Hook(h) => analyze_body(&h.body, source, findings),
            _ => {}
        }
    }
}

// ── Public API ───────────────────────────────────────────────────────

/// Run only Category A audit checks (compiler-enforced security invariants).
/// These are checks where a finding is never a legitimate false positive —
/// the code IS insecure if the check fires. Promoted from `mlog audit` to
/// `mlog check`/`mlog run`/`mlog serve`/`mlog compile` by Наряд №98.
///
/// Category A (Наряд №98, expanded Наряд #157, #201):
///   - SQL_DYNAMIC: query()/db_execute() with non-literal SQL (SQL injection vector)
///   - SECRET_LEAK: env() result passed to sink (respond/write_file/http_post/reflex_train)
///   - HTML_INJECTION: LLM output to respond() without sanitization (XSS)
///   - UNTRUSTED_TRAINING_DATA: json_body()/query_param() to reflex_train data/labels (Наряд #201)
///   - TAINT_PERSISTENCE: memorize(LLM) + respond(recall()) (Наряд #157)
///   - TAINT_PASSTHROUGH: respond(Wrap(call_llm(...))) trivial passthrough (Наряд #157)
///
/// Category B (advisory, stays in `mlog audit` only):
///   - SECRETS: heuristic, can false-positive on error messages/doc strings
///   - SANDBOX_COVERAGE: cross-file context needed
///   - RATE_LIMIT: external infra can handle this
///   - CSRF: not needed for token-authenticated APIs
///   - OPEN_REDIRECT: custom validation not recognized
///   - CANARY_LEAK: advisory detector (№284) — WARNING stays in
///     audit_program, never promoted to a compile error
// ── Check: SINK_CLEARANCE (Наряд №325, ADR-0161) ─────────────────────
//
// Category-A gate: at every sink-builtin call site (the sink list comes
// from the №316 SSOT classification — Role::Sink, never a hand-written
// list), an argument whose inferred label (№322/№323 machinery, via
// semantic::sink_clearance_violations) does not clear the sink is a
// Severity::Error with a specialized check_id; `poisoned` clears no
// sink (ADR-0154 §2.1).
//
// Specialized classes (the leak-suite vocabulary):
//   VOICE_EGRESS_UNCONSENTED  — voice egress without a consent scope
//                               (consent sources are Phase 2, №335;
//                               until then voice egress is unconsented
//                               by default — loud by design);
//   IRREVERSIBLE_NO_GRANT     — destructive SQL literal in db_execute
//                               (DROP/DELETE/TRUNCATE/ALTER; grant
//                               algebra is Phase 3, №339);
//   UNTRUSTED_EXEC_DECISION   — untrusted data drives exec/exec_argv;
//   SECRET_TO_EXEC            — a private label enters exec/exec_argv;
//   SECRET_EGRESS_VCS         — a private label enters git_push;
//   SECRET_EGRESS_NETWORK     — a private-URL marker in the address
//                               position of a network sink;
//   PII_EGRESS_NETWORK        — personal-data label in a network sink
//                               body;
//   PII_EGRESS_OUTPUT         — personal-data label in a public output;
//   UNTRUSTED_EGRESS_NETWORK  — untrusted label in a network sink body;
//   SINK_CLEARANCE            — every other confidentiality excess.
//
// Compatibility profile (ADR-0161): `profile legacy { egress:
// permissive_with_audit }` switches the gate to ADVISORY — each
// violation becomes Severity::Info (an audit event in the report and a
// stderr event on the compile/run path) instead of an Error.
fn sink_check_id(fn_name: &str, arg_index: usize, label: &crate::labels::Label) -> &'static str {
    use crate::labels::Conf;
    use crate::labels::Integrity;
    // Quarantine clears no sink (ADR-0154 §2.1) — the generic class.
    if label.conf == Conf::Poisoned {
        return "SINK_CLEARANCE";
    }
    let kind = match fn_name {
        "exec" | "exec_argv" => "exec",
        "git_push" => "vcs",
        "tts_send" => "voice",
        "db_execute" => "db",
        "print" | "respond" | "respond_html" | "html_response" => "output",
        "write_file" | "append_file" | "delete_file" => "file",
        // №331 (ADR-0162): the sanctioned materialization sink is file
        // egress — same class mapping as write_file (private conf →
        // SECRET_LEAK, the corpus vocabulary).
        "media_save" => "file",
        "memorize" | "mem_set" | "mtree_store" | "kv_set" => "memory",
        _ => "network",
    };
    match kind {
        "voice" => "VOICE_EGRESS_UNCONSENTED",
        "exec" => {
            if label.integrity == Integrity::Untrusted {
                "UNTRUSTED_EXEC_DECISION"
            } else if label.conf == Conf::Private {
                "SECRET_TO_EXEC"
            } else {
                "SINK_CLEARANCE"
            }
        }
        "vcs" => {
            if label.conf == Conf::Private {
                "SECRET_EGRESS_VCS"
            } else {
                "UNTRUSTED_EGRESS_NETWORK"
            }
        }
        "network" => {
            // Address position (arg 0) of a network sink carrying a
            // private-infrastructure marker: the destination is the leak.
            if arg_index == 0 && matches!(fn_name, "http_post" | "send_message") {
                return "SECRET_EGRESS_NETWORK";
            }
            if label.integrity == Integrity::Untrusted {
                "UNTRUSTED_EGRESS_NETWORK"
            } else {
                "PII_EGRESS_NETWORK"
            }
        }
        "output" => {
            if label.integrity == Integrity::Untrusted {
                // Untrusted data into a public output — the HTML-injection
                // class (the leak-suite corpus vocabulary; the lattice
                // generalizes the legacy LLM-only check).
                "HTML_INJECTION"
            } else {
                "PII_EGRESS_OUTPUT"
            }
        }
        "memory" => "TAINT_PERSISTENCE",
        "file" => {
            if label.conf == Conf::Private {
                // A private label into a file sink — the SECRET_LEAK
                // class (the corpus vocabulary keeps the legacy name).
                "SECRET_LEAK"
            } else {
                "SINK_CLEARANCE"
            }
        }
        // db_execute: the bottom label marks the CONTENT gate (a
        // destructive SQL literal needs no tainted data) — the grant
        // vocabulary of Phase 3 (№339) starts here as
        // IRREVERSIBLE_NO_GRANT; a NON-bottom label is a plain
        // confidentiality excess.
        "db" if *label == crate::labels::Label::bottom() => "IRREVERSIBLE_NO_GRANT",
        _ => "SINK_CLEARANCE",
    }
}

fn check_sink_clearance(
    declarations: &[Declaration],
    source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    let advisory = crate::profile::resolve(declarations).permissive();
    let violations = crate::semantic::sink_clearance_violations(declarations);
    for v in violations {
        // Attribution: the semantic layer hands us the arg label and
        // container; recover the arg expression for the URL-position
        // rule by re-walking — done inside the semantic layer's
        // sink_arg_label via the label itself; the address-position rule
        // needs the TEXT, so the semantic layer flags it through the
        // arg_index==0 + private-label contract (see sink_check_id).
        let check_id = sink_check_id(&v.fn_name, v.arg_index, &v.label);
        let severity = if advisory {
            Severity::Info
        } else {
            Severity::Error
        };
        findings.push(AuditFinding {
            severity,
            check_id,
            line: v.span.start_line as usize,
            message: format!(
                "sink clearance violated: argument {} of {} in {} carries label '{}'; \
                 sinks require public{}",
                v.arg_index,
                v.fn_name,
                v.container,
                v.label,
                if advisory {
                    " (audit event: profile legacy / egress permissive_with_audit)"
                } else {
                    ""
                }
            ),
        });
    }
    let _ = source;
}

// ── Check: REDACT_APPLIED audit events (Наряд №326, ADR-0154 §10) ────
//
// Every redact() application is an AUDIT EVENT — what was processed
// (container + argument), which policy ran, which target conf it
// declares. Events are UNCONDITIONAL (Severity::Info on the report +
// an [REDACT][audit-event] stderr line): they are the paper trail of
// the only sanctioned downward move on the conf axis, and they cannot
// be switched off (no profile, no env toggles — ADR-0154 §10).

/// ── Check: MEDIA_HANDLE_OPAQUE (Наряд №331, ADR-0162 §2.5) ───────────
/// Field access on a media handle (Image/Audio/VideoFrame/VideoSegment)
/// is a TYPE-level invariant, not a policy choice: bytes never live in
/// Value (ADR-0114), so the site can never compile. Always Error —
/// `profile legacy` cannot downgrade a type contradiction.
fn check_media_handle_opacity(
    declarations: &[Declaration],
    _source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    for v in crate::semantic::media_opacity_violations(declarations) {
        findings.push(AuditFinding {
            severity: Severity::Error,
            check_id: "MEDIA_HANDLE_OPAQUE",
            line: v.span.start_line as usize,
            message: v.message(),
        });
    }
}

/// ── Check: ORIGIN_REQUIRED (Наряд №332, ADR-0164) ────────────────────
/// The origin-chain rule (§7.4): a media handle without origin is not
/// constructed. Type-of-construction invariant — always Error, no
/// profile downgrades it (the №331 opacity posture). Runs on BOTH
/// compile paths (audit_category_a + audit_program), so
/// `compile_program` refuses unbound constructions too.
fn check_origin_required(
    declarations: &[Declaration],
    _source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    for v in crate::semantic::media_origin_violations(declarations) {
        findings.push(AuditFinding {
            severity: Severity::Error,
            check_id: "ORIGIN_REQUIRED",
            line: v.span.start_line as usize,
            message: v.message,
        });
    }
}

/// ── Check: ORIGIN_DECL_INVALID (Наряд №332, ADR-0164) ─────────────
/// Declared-origin shape/vocabulary validation on EVERY compile path
/// (the same rules `check_program` applies via validate_origin_decls):
/// unknown kind/media/label words, unknown fields, `kind: file` without
/// `path`. Always Error — a mis-declared provenance source is a
/// provenance lie, and no profile downgrades a lie into silence.
fn check_origin_decls_valid(
    declarations: &[Declaration],
    _source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    for e in crate::semantic::origin_decl_errors(declarations) {
        findings.push(AuditFinding {
            severity: Severity::Error,
            check_id: "ORIGIN_DECL_INVALID",
            line: e.span.start_line as usize,
            message: e.message,
        });
    }
}

/// ── Check: COMPAT_PROFILE_INVALID (Наряд №336, ADR-0165 §2.4) ──
/// A compat-profile mistake must never be a silent no-op (№325 rule):
/// the shape validation `check_program` applies is mirrored onto EVERY
/// compile path (the №332 posture). Unknown profile names / option keys
/// / mode words fail compilation, not just `mlog check`.
fn check_profile_shape(
    declarations: &[Declaration],
    _source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    for decl in declarations {
        if let Declaration::Profile(p) = decl {
            if let Err(e) = crate::profile::validate(p) {
                findings.push(AuditFinding {
                    severity: Severity::Error,
                    check_id: "COMPAT_PROFILE_INVALID",
                    line: p.span.start_line as usize,
                    message: e,
                });
            }
        }
    }
}

/// ── Check: BACKEND_SELECT_INVALID + BACKEND_LADDER_UNVERIFIABLE
/// (Наряд №336, ADR-0165 §2.4) ──
/// The BackendSelect ladder companion check on EVERY compile path (the
/// №332 origin-chain posture): a statically-visible ladder is verified
/// against the №333 registry SSOT — unknown rung, class mismatch,
/// unknown class word, duplicate/empty ladders (BACKEND_SELECT_INVALID);
/// under `profile device { mode: production }` a PendingNo334 rung is
/// UNVERIFIABLE for the profile and fails compilation
/// (BACKEND_LADDER_UNVERIFIABLE). Always Error — a broken ladder must
/// never surface as a runtime surprise; no profile downgrades it.
fn check_backend_ladder(
    declarations: &[Declaration],
    _source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    for v in crate::semantic::backend_select_ladder_violations(declarations) {
        let check_id = match v.kind {
            crate::semantic::LadderViolationKind::Invalid => "BACKEND_SELECT_INVALID",
            crate::semantic::LadderViolationKind::UnverifiableForProduction => {
                "BACKEND_LADDER_UNVERIFIABLE"
            }
        };
        findings.push(AuditFinding {
            severity: Severity::Error,
            check_id,
            line: v.span.start_line as usize,
            message: v.message,
        });
    }
}

/// ── Check: BACKEND_LICENSE_DISTRIBUTION (Наряд №333, ADR-0163 §2.2) ──
/// A program that NAMES non-osi/restrictive weights (string literals at
/// any position + the `vision { model: … }` field) is a distribution
/// violation under the default profile: compile-blocking Error naming
/// the license class and the registry record. Under
/// `profile licensing { backends: permissive_with_audit }` (the loud
/// bridge, №325 precedent) the same sites become Info audit events —
/// usage is allowed but never silent. Restrictive entries are
/// default-deny (unverified license) and unlock together with non-osi
/// under the bridge.
/// ── Check: QUARANTINE_EGRESS + CONSENT_LEDGER_EXPORT (Наряд №335) ────
/// The consent surface's audit events (№326 posture: unconditional,
/// Severity::Info — never blocking):
///   - every `quarantine_write` call site — the legal egress of a
///     poisoned value (the №325 clearance exempts THIS sink only);
///   - every `consent_ledger_export` call site — the ledger leaves the
///     process as file egress.
fn check_consent_events(
    declarations: &[Declaration],
    source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    fn walk(expr: &Expr, container: &str, source: &str, findings: &mut Vec<AuditFinding>) {
        if let Expr::FnCall { name, args, .. } = expr {
            if name == "quarantine_write" || name == "consent_ledger_export" {
                let (check_id, message) = if name == "quarantine_write" {
                    (
                        "QUARANTINE_EGRESS",
                        format!(
                            "poisoned value reaches the quarantine sink in {} — legal egress with audit event (№335)",
                            container
                        ),
                    )
                } else {
                    (
                        "CONSENT_LEDGER_EXPORT",
                        format!(
                            "consent ledger exported in {} — grant/TTL/revoke records leave the process with an audit event (№335)",
                            container
                        ),
                    )
                };
                eprintln!("[CONSENT][audit-event] {}", message);
                findings.push(AuditFinding {
                    severity: Severity::Info,
                    check_id,
                    line: find_line(source, name),
                    message,
                });
            }
            for a in args {
                walk(a, container, source, findings);
            }
        }
    }
    fn walk_stmts(
        stmts: &[Statement],
        container: &str,
        source: &str,
        findings: &mut Vec<AuditFinding>,
    ) {
        for st in stmts {
            match st {
                Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
                    walk(value, container, source, findings)
                }
                Statement::ExprStmt { expr: value, .. } | Statement::Return { value, .. } => {
                    walk(value, container, source, findings)
                }
                Statement::Each { iterable, body, .. }
                | Statement::EachWithIndex { iterable, body, .. } => {
                    walk(iterable, container, source, findings);
                    walk_stmts(body, container, source, findings);
                }
                Statement::While {
                    condition, body, ..
                } => {
                    walk(condition, container, source, findings);
                    walk_stmts(body, container, source, findings);
                }
                Statement::IfElseBlock {
                    condition,
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    walk(condition, container, source, findings);
                    walk_stmts(then_body, container, source, findings);
                    for (_, b) in else_ifs {
                        walk_stmts(b, container, source, findings);
                    }
                    if let Some(eb) = else_body {
                        walk_stmts(eb, container, source, findings);
                    }
                }
                Statement::IfThen {
                    condition, body, ..
                } => {
                    walk(condition, container, source, findings);
                    walk_stmts(body, container, source, findings);
                }
                Statement::Match {
                    scrutinee,
                    arms,
                    else_body,
                    ..
                } => {
                    walk(scrutinee, container, source, findings);
                    for arm in arms {
                        let b = match arm {
                            crate::ast::MatchArm::Exact(_, b)
                            | crate::ast::MatchArm::StartsWith(_, b)
                            | crate::ast::MatchArm::Contains(_, b)
                            | crate::ast::MatchArm::Compare(_, _, b) => b,
                        };
                        walk_stmts(b, container, source, findings);
                    }
                    if let Some(eb) = else_body {
                        walk_stmts(eb, container, source, findings);
                    }
                }
                Statement::Memorize(m) => walk(&m.value, container, source, findings),
                Statement::Forget(f) => walk(&f.query, container, source, findings),
                Statement::Relate(r) => {
                    walk(&r.from, container, source, findings);
                    walk(&r.to, container, source, findings);
                }
                _ => {}
            }
        }
    }
    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                walk_stmts(&p.body, &format!("pattern '{}'", p.name), source, findings);
            }
            Declaration::Tool(t) => {
                for m in &t.methods {
                    walk_stmts(
                        &m.body,
                        &format!("tool method '{}.{}'", t.name, m.name),
                        source,
                        findings,
                    );
                }
            }
            Declaration::MlogServer(srv) => {
                for r in &srv.routes {
                    walk_stmts(
                        &r.body,
                        &format!("route {} {}", r.method, r.path),
                        source,
                        findings,
                    );
                }
            }
            _ => {}
        }
    }
}

fn check_backend_license(
    declarations: &[Declaration],
    source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    fn walk_string_exprs<'a>(expr: &'a Expr, acc: &mut Vec<&'a String>) {
        match expr {
            Expr::StringLit { value, .. } => acc.push(value),
            Expr::FnCall { args, .. } | Expr::QualifiedCall { args, .. } => {
                for a in args {
                    walk_string_exprs(a, acc);
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                walk_string_exprs(left, acc);
                walk_string_exprs(right, acc);
            }
            Expr::IfElse {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                walk_string_exprs(condition, acc);
                walk_string_exprs(then_branch, acc);
                walk_string_exprs(else_branch, acc);
            }
            Expr::List { items, .. } => {
                for i in items {
                    walk_string_exprs(i, acc);
                }
            }
            Expr::FieldAccess { object, .. } => walk_string_exprs(object, acc),
            Expr::IndexAccess { object, index, .. } => {
                walk_string_exprs(object, acc);
                walk_string_exprs(index, acc);
            }
            Expr::BlockIfElse {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                walk_string_exprs(condition, acc);
                walk_string_stmts(then_body, acc);
                for (c, body) in else_ifs {
                    walk_string_exprs(c, acc);
                    walk_string_stmts(body, acc);
                }
                if let Some(eb) = else_body {
                    walk_string_stmts(eb, acc);
                }
            }
            Expr::MatchExpr {
                scrutinee,
                arms,
                else_body,
                ..
            } => {
                walk_string_exprs(scrutinee, acc);
                for arm in arms {
                    walk_string_stmts(arm.body(), acc);
                }
                if let Some(eb) = else_body {
                    walk_string_stmts(eb, acc);
                }
            }
            Expr::Try { expr, .. } => walk_string_exprs(expr, acc),
            _ => {}
        }
    }

    fn walk_string_stmts<'a>(stmts: &'a [Statement], acc: &mut Vec<&'a String>) {
        for stmt in stmts {
            match stmt {
                Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
                    walk_string_exprs(value, acc)
                }
                Statement::ExprStmt { expr, .. } | Statement::Return { value: expr, .. } => {
                    walk_string_exprs(expr, acc)
                }
                Statement::Each { iterable, body, .. } => {
                    walk_string_exprs(iterable, acc);
                    walk_string_stmts(body, acc);
                }
                Statement::EachWithIndex { iterable, body, .. } => {
                    walk_string_exprs(iterable, acc);
                    walk_string_stmts(body, acc);
                }
                Statement::While {
                    condition, body, ..
                } => {
                    walk_string_exprs(condition, acc);
                    walk_string_stmts(body, acc);
                }
                Statement::IfElseBlock {
                    condition,
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    walk_string_exprs(condition, acc);
                    walk_string_stmts(then_body, acc);
                    for (c, body) in else_ifs {
                        walk_string_exprs(c, acc);
                        walk_string_stmts(body, acc);
                    }
                    if let Some(eb) = else_body {
                        walk_string_stmts(eb, acc);
                    }
                }
                Statement::IfThen {
                    condition, body, ..
                } => {
                    walk_string_exprs(condition, acc);
                    walk_string_stmts(body, acc);
                }
                Statement::Match {
                    scrutinee,
                    arms,
                    else_body,
                    ..
                } => {
                    walk_string_exprs(scrutinee, acc);
                    for arm in arms {
                        walk_string_stmts(arm.body(), acc);
                    }
                    if let Some(eb) = else_body {
                        walk_string_stmts(eb, acc);
                    }
                }
                Statement::Memorize(m) => walk_string_exprs(&m.value, acc),
                Statement::Forget(f) => walk_string_exprs(&f.query, acc),
                Statement::Relate(r) => {
                    walk_string_exprs(&r.from, acc);
                    walk_string_exprs(&r.to, acc);
                }
                Statement::Break | Statement::Continue => {}
            }
        }
    }

    let permissive = crate::profile::resolve(declarations).backend_license_permissive();

    for decl in declarations {
        let mut strings: Vec<&String> = Vec::new();
        match decl {
            Declaration::Pattern(p) => walk_string_stmts(&p.body, &mut strings),
            Declaration::LearnablePattern(lp) => {
                // The prompt is a literal carrier too — a learnable
                // pattern can name the weights it wants (№334 surface).
                strings.push(&lp.prompt);
            }
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
            Declaration::Vision(v) => {
                // The declaration model field is the canonical reference
                // position (`vision { model: "z-image-turbo", … }`).
                strings.push(&v.model);
            }
            Declaration::Memorize(m) => walk_string_exprs(&m.value, &mut strings),
            Declaration::Forget(f) => walk_string_exprs(&f.query, &mut strings),
            Declaration::Relate(r) => {
                walk_string_exprs(&r.from, &mut strings);
                walk_string_exprs(&r.to, &mut strings);
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
            // A literal NAMES the weights when it equals (case-insensitive)
            // a registered weights id. Substring matches would false-positive
            // on documentation prose — exact (ci) only.
            let entry = match crate::backends::find_by_weights_id_ci(s) {
                Some(e) if e.license != crate::backends::LicenseClass::Osi => e,
                _ => continue,
            };
            let snippet = crate::util::safe_byte_truncate(s, 24);
            let line = find_line(source, snippet);
            if permissive {
                findings.push(AuditFinding {
                    severity: Severity::Info,
                    check_id: "BACKEND_LICENSE",
                    line,
                    message: format!(
                        "[audit-event] non-osi/restrictive backend weights '{}' ({}),                          license: {} — allowed by profile licensing (bridge, not residence;                          ADR-0163)",
                        entry.weights_id,
                        entry.class.as_str(),
                        entry.license_note
                    ),
                });
            } else {
                findings.push(AuditFinding {
                    severity: Severity::Error,
                    check_id: "BACKEND_LICENSE_DISTRIBUTION",
                    line,
                    message: format!(
                        "backend weights '{}' (class {}) are {} and FORBIDDEN in the \
                         distribution profile — license: {}. Unlock explicitly with \
                         'profile licensing {{ backends: permissive_with_audit }}' \
                         (audited bridge, ADR-0163)",
                        entry.weights_id,
                        entry.class.as_str(),
                        entry.license.as_str(),
                        entry.license_note
                    ),
                });
            }
        }
    }
}

fn check_redact_events(
    declarations: &[Declaration],
    source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    fn walk(expr: &Expr, container: &str, source: &str, findings: &mut Vec<AuditFinding>) {
        if let Expr::FnCall { name, args, .. } = expr {
            if name == "redact" {
                let (policy, target) = match args.get(1) {
                    Some(Expr::StringLit { value, .. }) => {
                        match crate::builtins::string::redact_policy(value) {
                            Some(p) => (p.name.to_string(), p.target_conf.to_string()),
                            None => (value.clone(), "unknown".to_string()),
                        }
                    }
                    _ => ("<dynamic>".to_string(), "source".to_string()),
                };
                let message = format!(
                    "redact applied in {} — policy '{}', target conf '{}'",
                    container, policy, target
                );
                eprintln!("[REDACT][audit-event] {}", message);
                findings.push(AuditFinding {
                    severity: Severity::Info,
                    check_id: "REDACT_APPLIED",
                    line: find_line(source, "redact"),
                    message,
                });
            }
            for a in args {
                walk(a, container, source, findings);
            }
        }
    }
    fn walk_stmts(
        stmts: &[Statement],
        container: &str,
        source: &str,
        findings: &mut Vec<AuditFinding>,
    ) {
        for st in stmts {
            match st {
                Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
                    walk(value, container, source, findings)
                }
                Statement::ExprStmt { expr: value, .. } | Statement::Return { value, .. } => {
                    walk(value, container, source, findings)
                }
                Statement::Each { iterable, body, .. }
                | Statement::EachWithIndex { iterable, body, .. } => {
                    walk(iterable, container, source, findings);
                    walk_stmts(body, container, source, findings);
                }
                Statement::While {
                    condition, body, ..
                } => {
                    walk(condition, container, source, findings);
                    walk_stmts(body, container, source, findings);
                }
                Statement::IfElseBlock {
                    condition,
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    walk(condition, container, source, findings);
                    walk_stmts(then_body, container, source, findings);
                    for (_, b) in else_ifs {
                        walk_stmts(b, container, source, findings);
                    }
                    if let Some(eb) = else_body {
                        walk_stmts(eb, container, source, findings);
                    }
                }
                Statement::IfThen {
                    condition, body, ..
                } => {
                    walk(condition, container, source, findings);
                    walk_stmts(body, container, source, findings);
                }
                Statement::Match {
                    scrutinee,
                    arms,
                    else_body,
                    ..
                } => {
                    walk(scrutinee, container, source, findings);
                    for arm in arms {
                        let b = match arm {
                            crate::ast::MatchArm::Exact(_, b)
                            | crate::ast::MatchArm::StartsWith(_, b)
                            | crate::ast::MatchArm::Contains(_, b)
                            | crate::ast::MatchArm::Compare(_, _, b) => b,
                        };
                        walk_stmts(b, container, source, findings);
                    }
                    if let Some(eb) = else_body {
                        walk_stmts(eb, container, source, findings);
                    }
                }
                Statement::Memorize(m) => walk(&m.value, container, source, findings),
                Statement::Forget(f) => walk(&f.query, container, source, findings),
                Statement::Relate(r) => {
                    walk(&r.from, container, source, findings);
                    walk(&r.to, container, source, findings);
                }
                _ => {}
            }
        }
    }
    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                walk_stmts(&p.body, &format!("pattern {}", p.name), source, findings)
            }
            Declaration::Tool(t) => {
                for m in &t.methods {
                    walk_stmts(
                        &m.body,
                        &format!("tool {}.{}", t.name, m.name),
                        source,
                        findings,
                    );
                }
            }
            Declaration::MlogServer(srv) => {
                for r in &srv.routes {
                    walk_stmts(
                        &r.body,
                        &format!("route {} {}", r.method, r.path),
                        source,
                        findings,
                    );
                }
            }
            Declaration::Hook(h) => {
                walk_stmts(&h.body, &format!("hook {:?}", h.phase), source, findings)
            }
            Declaration::Test(t) => {
                walk_stmts(&t.body, &format!("test \"{}\"", t.name), source, findings)
            }
            _ => {}
        }
    }
}

// ── Check: UNTRUSTED_DECISION (Наряд №327) ───────────────────────────
//
// Category-A anti-injection gate: data that DECIDES control flow —
// `if`/`else if` conditions, `while` conditions, `match` scrutinees —
// must be `trusted`. Untrusted data as DATA is legal (the integrity
// axis is about decisions, not about existence). The message names the
// untrusted source (a direct №316 Source call) and the decision point.

fn check_integrity_decisions(
    declarations: &[Declaration],
    source: &str,
    findings: &mut Vec<AuditFinding>,
) {
    for v in crate::semantic::integrity_decision_violations(declarations) {
        findings.push(AuditFinding {
            severity: Severity::Error,
            check_id: "UNTRUSTED_DECISION",
            line: v.span.start_line as usize,
            message: format!(
                "anti-injection: untrusted data (label '{}' from source '{}') decides a '{}' in {} — validate/one-way-redact it before deciding",
                v.label, v.source_name, v.kind, v.container
            ),
        });
    }
    let _ = source;
}

pub fn audit_category_a(declarations: &[Declaration], source: &str) -> Vec<AuditFinding> {
    let mut findings: Vec<AuditFinding> = Vec::new();
    check_sql_dynamic(declarations, source, &mut findings);
    check_secret_leak(declarations, source, &mut findings);
    check_html_injection(declarations, source, &mut findings);
    check_taint_persistence(declarations, source, &mut findings);
    check_taint_passthrough_pattern(declarations, source, &mut findings);
    // Наряд №292 (P0, security): interprocedural taint MVP — summary-based,
    // bounded depth 2, catches non-trivial passthrough chains that the
    // trivial TAINT_PASSTHROUGH misses. Sanitizers (render/escape_html)
    // lift the taint — zero false positives on legitimate code.
    // INTERP_DEPTH_LIMIT Warning stays advisory (NOT in audit_category_a
    // promoted to compile error) — the boundary is informational, not a
    // security violation.
    check_taint_interp_pattern_errors_only(declarations, source, &mut findings);
    // Наряд №241 (R5, ADR-0125): vision export gate — ONLY the Error
    // (VISION_UNSIGNED_EXPORT) on the compile path. The raw-export
    // Warning stays advisory (audit_program below): this caller promotes
    // every Warning to a compile error (semantic.rs №98 promotion), which
    // would contradict ADR-0125's "warning" for the opt-out.
    check_vision_export_gates_errors_only(declarations, source, &mut findings);
    // Наряд №241 (R5, Block 3.2, ADR-0125): MODEL_WEIGHTS_UNSAFE —
    // Category-A Error, shared SSOT gate for the Voice pillar. Runtime
    // layers (allowlist default-deny, SSRF guard, SHA pinning) live in
    // vision_fetch_weights — both layers, same check-id.
    check_model_weights_unsafe(declarations, source, &mut findings);
    // Наряд №320 (ADR-0152 D2): Art. 50 marking gate — vision_export_raw
    // call sites are statically visible unmarked synthetic egress.
    check_media_synthetic_unmarked(declarations, source, &mut findings);
    // Наряд №325 (ADR-0161): sink clearance on classified sinks — LAST so
    // the pre-existing specialized checks keep their classes on shared
    // sites (e.g. env→print is SECRET_LEAK first).
    check_sink_clearance(declarations, source, &mut findings);
    // Наряд №331 (ADR-0162 §2.5): opaque media handles — type-level
    // gate, always Error (see the check doc above).
    check_media_handle_opacity(declarations, source, &mut findings);
    // Наряд №333 (ADR-0163): backend license gate — distribution
    // refusal for non-osi/restrictive weights references, audited
    // bridge under 'profile licensing'.
    check_backend_license(declarations, source, &mut findings);
    // Наряд №332 (ADR-0164): the origin chain — unbound handle
    // constructions are refused (type-of-construction invariant).
    check_origin_required(declarations, source, &mut findings);
    // Наряд №332 (ADR-0164): declared origins are validated loudly —
    // a mis-declared provenance source is a provenance lie.
    check_origin_decls_valid(declarations, source, &mut findings);
    // Наряд №326 (ADR-0154 §10): every redact application is an
    // unconditional audit event (Severity::Info — never blocking).
    check_redact_events(declarations, source, &mut findings);
    // Наряд №335: consent surface audit events — quarantine egress and
    // ledger export are legal but never silent.
    check_consent_events(declarations, source, &mut findings);
    // Наряд №327: the integrity axis — untrusted data must not decide
    // control flow (Category-A Error).
    check_integrity_decisions(declarations, source, &mut findings);
    // Наряд №336 (ADR-0165 §2.4): compat-profile shapes are loud on
    // every compile path, and statically visible BackendSelect ladders
    // are verified against the registry SSOT; the production device
    // profile refuses unverifiable (pending-pin) rungs.
    check_profile_shape(declarations, source, &mut findings);
    check_backend_ladder(declarations, source, &mut findings);
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
    // Наряд №292 (P0, security): interprocedural taint MVP — full version
    // (with INTERP_DEPTH_LIMIT advisory Warnings for call cycles).
    check_taint_interp_pattern(&declarations, source, &mut findings);
    // Наряд №241 (R5, ADR-0125): full vision gates — Error
    // VISION_UNSIGNED_EXPORT + Warning VISION_UNSIGNED_EXPORT_RAW
    // (advisory layer).
    check_vision_export_gates(&declarations, source, &mut findings);
    // Наряд №241 (R5, Block 3): MODEL_WEIGHTS_UNSAFE Error +
    // VISION_POLICY_MISSING Warning (advisory — ADR-0125 "warning;
    // policy validated statically").
    check_model_weights_unsafe(&declarations, source, &mut findings);
    check_vision_policy_missing(&declarations, source, &mut findings);
    // Наряд №284 (P1, M1): canary — compromised-channel detector.
    // Advisory Warning (audit_program), НЕ Category-A: промоция Warning
    // до compile-error противоречила бы «детектор, не гейт».
    check_canary_leak(&declarations, source, &mut findings);
    // Наряд №320 (ADR-0152 D2): Art. 50 marking gate (Category-A Error).
    check_media_synthetic_unmarked(&declarations, source, &mut findings);
    // Наряд №325 (ADR-0161): sink clearance — strict (Error) or, under
    // `profile legacy`, advisory audit events.
    check_sink_clearance(&declarations, source, &mut findings);
    // Наряд №331 (ADR-0162 §2.5): opaque media handles — type-level
    // gate, always Error.
    check_media_handle_opacity(&declarations, source, &mut findings);
    // Наряд №333 (ADR-0163): backend license gate.
    check_backend_license(&declarations, source, &mut findings);
    // Наряд №332 (ADR-0164): the origin chain.
    check_origin_required(&declarations, source, &mut findings);
    // Наряд №332 (ADR-0164): declared-origin validation.
    check_origin_decls_valid(&declarations, source, &mut findings);
    // Наряд №326 (ADR-0154 §10): every redact application is an
    // unconditional audit event (Severity::Info — never blocking).
    check_redact_events(&declarations, source, &mut findings);
    // Наряд №335: consent surface audit events.
    check_consent_events(&declarations, source, &mut findings);
    // Наряд №327: the integrity axis — decision gate.
    check_integrity_decisions(&declarations, source, &mut findings);

    // Sort findings by line number for deterministic output
    findings.sort_by_key(|f| (f.line, f.check_id));

    Ok(AuditResult { findings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast;

    // Наряд №322 (ADR-0154 §5): every TaintKind variant must keep an
    // entry in the label-lattice projection table
    // (`labels::legacy_taint_label`, keyed by variant name). The table
    // is the bridge the sink-gate (№325) will read; this test pins that
    // the enum and the table cannot silently drift apart — adding a
    // TaintKind variant without a projection fails HERE, at CI, not at
    // the gate. (The projection itself lives in labels.rs; no dead
    // accessor is kept on the private enum in this naryad.)
    #[test]
    fn n322_taint_kind_label_projection_covers_all_variants() {
        let all = [
            TaintKind::LlmOutput,
            TaintKind::Secret,
            TaintKind::UserInput,
            TaintKind::Sanitized,
            TaintKind::CanaryLeak,
        ];
        for kind in all {
            let name = format!("{:?}", kind);
            let label = crate::labels::legacy_taint_label(&name)
                .unwrap_or_else(|| panic!("kind {} lost its ADR-0154 §5 projection", name));
            assert!(!label.to_string().is_empty());
        }
        // Spot-check the quarantine projection: a confirmed-compromised
        // channel is `poisoned` — no legal sinks (sink-gate №325).
        assert_eq!(
            crate::labels::legacy_taint_label("CanaryLeak")
                .unwrap()
                .conf,
            crate::labels::Conf::Poisoned
        );
        // Secrets are the confidentiality concern.
        assert_eq!(
            crate::labels::legacy_taint_label("Secret").unwrap().conf,
            crate::labels::Conf::Private
        );
        // LLM output / user input are the integrity concern.
        assert_eq!(
            crate::labels::legacy_taint_label("LlmOutput")
                .unwrap()
                .integrity,
            crate::labels::Integrity::Untrusted
        );
        assert_eq!(
            crate::labels::legacy_taint_label("UserInput")
                .unwrap()
                .integrity,
            crate::labels::Integrity::Untrusted
        );
        // Sanitization restores trust.
        assert_eq!(
            crate::labels::legacy_taint_label("Sanitized")
                .unwrap()
                .integrity,
            crate::labels::Integrity::Trusted
        );
    }

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
            rate_limit: None,
            redact_mode: None, // №263: fixtures do not configure a limit
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
            rate_limit: None,
            redact_mode: None, // №263: fixtures do not configure a limit
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
            rate_limit: None,
            redact_mode: None, // №263: fixtures do not configure a limit
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
            rate_limit: None,
            redact_mode: None, // №263: fixtures do not configure a limit
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
            rate_limit: None,
            redact_mode: None, // №263: fixtures do not configure a limit
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

    // ── Наряд #157: http_post URL leak + headers safety contracts ──

    #[test]
    fn secret_leak_http_post_url_with_secret() {
        // http_post(url + env("TOKEN"), body, headers) — secret in URL is a leak.
        let source = r#"
            pattern LeakUrl() -> String {
                let token = env("AUTH_TOKEN")
                let url = "https://api.example.com/" + token
                let r = http_post(url, "{}", "application/json")
                return r
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            result.findings.iter().any(|f| f.check_id == "SECRET_LEAK"
                && f.severity == Severity::Error
                && f.message.contains("http_post URL")),
            "http_post(url+env(...), b, h) must trigger SECRET_LEAK for URL, got {:?}",
            result.findings
        );
    }

    #[test]
    fn secret_leak_http_post_headers_safe() {
        // http_post(url, body, env("TOKEN")) — secret in headers is legitimate auth.
        let source = r#"
            pattern AuthPost() -> String {
                let token = env("AUTH_TOKEN")
                let r = http_post("https://api.example.com", "{\"data\": 1}", token)
                return r
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            !result.findings.iter().any(|f| f.check_id == "SECRET_LEAK"),
            "http_post(u, b, env(...)) — headers auth must NOT trigger SECRET_LEAK, got {:?}",
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
            rate_limit: None,
            redact_mode: None, // №263: fixtures do not configure a limit
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
                .any(|f| f.check_id == "TAINT_PERSISTENCE" && f.severity == Severity::Error),
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
                .any(|f| f.check_id == "TAINT_PASSTHROUGH" && f.severity == Severity::Error),
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
                .any(|f| f.check_id == "TAINT_PASSTHROUGH" && f.severity == Severity::Error),
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

    // ── Наряд №309 (ADR-0151 D6): UNTRUSTED_FRAME taint tests ─────────

    #[test]
    fn n309_untrusted_frame_form_data_ref_flagged() {
        let source = r#"
            pattern Scene() -> String {
                let frame_ref = form_data("frame")
                let v = video_render("wan-2.2-ti2v-5b", "scene", frame_ref)
                return v
            }
        "#;
        let result = audit_program(source).unwrap();
        let findings: Vec<_> = result
            .findings
            .iter()
            .filter(|f| f.check_id == "UNTRUSTED_FRAME")
            .collect();
        assert_eq!(
            findings.len(),
            1,
            "UserInput-tainted ref must be flagged exactly once, got {:?}",
            result.findings
        );
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn n309_untrusted_frame_inline_http_get_flagged() {
        let source = r#"
            pattern Scene() -> String {
                let v = video_render("wan-2.2-ti2v-5b", "scene", http_get("https://cdn.example/frame.png"))
                return v
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.check_id == "UNTRUSTED_FRAME"),
            "inline http_get ref (http class per ADR-0149 D5) must be flagged, got {:?}",
            result.findings
        );
    }

    #[test]
    fn n309_untrusted_frame_inline_read_file_flagged() {
        let source = r#"
            pattern Scene() -> String {
                let v = video_render("wan-2.2-ti2v-5b", "scene", read_file("frame.raw"))
                return v
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.check_id == "UNTRUSTED_FRAME"),
            "inline read_file ref (file class per ADR-0149 D5) must be flagged, got {:?}",
            result.findings
        );
    }

    #[test]
    fn n309_untrusted_frame_two_anchor_last_ref_flagged() {
        let source = r#"
            pattern Scene() -> String {
                let first = "hero_first_frame.png"
                let v = video_render("wan-2.2-ti2v-5b", "scene", first, query_param("last"))
                return v
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.check_id == "UNTRUSTED_FRAME"),
            "tainted ref_last (position 3) must be flagged, got {:?}",
            result.findings
        );
    }

    #[test]
    fn n309_untrusted_frame_clean_literal_not_flagged() {
        // T2V (no ref) and a clean literal ref must NOT be flagged.
        let source = r#"
            pattern Scene() -> String {
                let t2v = video_render("wan-2.2-ti2v-5b", "clean scene")
                let i2v = video_render("wan-2.2-ti2v-5b", "clean i2v", "hero_frame.png")
                return i2v
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            !result
                .findings
                .iter()
                .any(|f| f.check_id == "UNTRUSTED_FRAME"),
            "clean refs must not be flagged, got {:?}",
            result.findings
        );
    }

    // ── Наряд №320 (ADR-0152): MEDIA_SYNTHETIC_UNMARKED gate tests ────

    #[test]
    fn n320_media_synthetic_unmarked_vision_export_raw_flagged() {
        let source = r#"
            pattern Ship() -> String {
                let v = vision_export_raw(handle, "out.png")
                return v
            }
        "#;
        let result = audit_program(source).unwrap();
        let findings: Vec<_> = result
            .findings
            .iter()
            .filter(|f| f.check_id == "MEDIA_SYNTHETIC_UNMARKED")
            .collect();
        assert_eq!(findings.len(), 1, "got {:?}", result.findings);
        assert_eq!(findings[0].severity, Severity::Error);
        // №98 promotion: the Error lands on the compile path too.
        let cat_a = audit_category_a(&crate::parser::parse(source).unwrap(), source);
        assert!(cat_a
            .iter()
            .any(|f| f.check_id == "MEDIA_SYNTHETIC_UNMARKED"));
    }

    #[test]
    fn n320_marked_export_not_flagged() {
        // Signed egress (vision_export) is the marked path — no finding.
        let source = r#"
            pattern Ship() -> String {
                let v = vision_export(handle, "out.png")
                return v
            }
        "#;
        let result = audit_program(source).unwrap();
        assert!(
            !result
                .findings
                .iter()
                .any(|f| f.check_id == "MEDIA_SYNTHETIC_UNMARKED"),
            "marked egress must not be flagged, got {:?}",
            result.findings
        );
    }
}
