// ── Semantic analysis for METALOGOS ──────────────────────────────
// Validates declarations without execution. Reports errors and warnings.
// Phase 6+: Enforces opaque type constraints (Html, Query, Secret, etc.)

use crate::ast::*;
use std::collections::HashSet;

/// Result of semantic analysis: errors prevent execution, warnings are advisory.
#[derive(Debug, Clone, Default)]
pub struct AnalysisResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl AnalysisResult {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    /// Format analysis result for display.
    pub fn format(&self) -> String {
        let mut lines = Vec::new();
        if !self.errors.is_empty() {
            let n = self.errors.len();
            if n == 1 {
                lines.push("1 error:".to_string());
            } else {
                lines.push(format!("{} errors:", n));
            }
            for (i, e) in self.errors.iter().enumerate() {
                lines.push(format!("  {}: {}", i + 1, e));
            }
        }
        if !self.warnings.is_empty() {
            let n = self.warnings.len();
            if n == 1 {
                lines.push("1 warning:".to_string());
            } else {
                lines.push(format!("{} warnings:", n));
            }
            for (i, w) in self.warnings.iter().enumerate() {
                lines.push(format!("  {}: {}", i + 1, w));
            }
        }
        if self.errors.is_empty() && self.warnings.is_empty() {
            lines.push("OK: no issues found.".to_string());
        }
        lines.join("\n")
    }
}

/// Valid middleware names for mlogserver blocks.
const VALID_MIDDLEWARE: &[&str] = &["session", "csrf", "security_headers", "rate_limit", "cors"];

/// Valid HTTP methods for route declarations.
const VALID_METHODS: &[&str] = &["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

/// Perform semantic analysis on a list of declarations (without executing them).
/// Validates:
///   - Entity types referenced in records exist
///   - Field initializers reference valid fields
///   - Patterns/learnables invoked in flows exist
///   - Flow branch targets are known patterns
///   - Duplicate entity/pattern/flow names
///   - Rule targets reference existing entities
///   - Adapt/mutate targets reference existing learnable patterns
///   - Relate/sandbox declarations are well-formed
///   - MlogServer middleware names are valid (Phase 6.1)
///   - Route methods are valid HTTP methods (Phase 6.1)
///   - Template return type is Html (Phase 6.2)
///   - Opaque types used in correct contexts (Phase 6.2–6.5)
pub fn check_program(declarations: &[Declaration]) -> AnalysisResult {
    let mut result = AnalysisResult::default();
    let mut entity_types: HashSet<String> = HashSet::new();
    let mut entity_names: HashSet<String> = HashSet::new();
    let mut pattern_names: HashSet<String> = HashSet::new();
    let mut learnable_names: HashSet<String> = HashSet::new();
    let mut flow_names: HashSet<String> = HashSet::new();
    let builtin_names = crate::builtins::builtin_name_set();
    let mut role_names: HashSet<String> = HashSet::new();
    let mut pattern_param_counts: HashSet<(String, usize)> = HashSet::new();

    // First pass: collect all declarations (names)
    for decl in declarations {
        match decl {
            Declaration::EntityType(e) => {
                if !entity_types.insert(e.name.clone()) {
                    result
                        .errors
                        .push(format!("duplicate entity type: {}", e.name));
                }
            }
            Declaration::EntityRecord(e) => {
                if !entity_names.insert(e.name.clone()) {
                    result.errors.push(format!("duplicate entity: {}", e.name));
                }
            }
            Declaration::EntitySimple(e) => {
                if !entity_names.insert(e.name.clone()) {
                    result.errors.push(format!("duplicate entity: {}", e.name));
                }
            }
            Declaration::Pattern(p) => {
                if !pattern_names.insert(p.name.clone()) {
                    result.errors.push(format!("duplicate pattern: {}", p.name));
                }
                pattern_param_counts.insert((p.name.clone(), p.params.len()));
            }
            Declaration::LearnablePattern(lp) => {
                if !learnable_names.insert(lp.name.clone()) {
                    result
                        .errors
                        .push(format!("duplicate learnable pattern: {}", lp.name));
                }
            }
            Declaration::Flow(f) => {
                if !flow_names.insert(f.name.clone()) {
                    result.errors.push(format!("duplicate flow: {}", f.name));
                }
            }
            Declaration::Template(t) => {
                // Templates are also callable as render targets
                pattern_names.insert(t.name.clone());
                if is_opaque_type(&t.return_type) && t.return_type != "Html" {
                    result.errors.push(format!(
                        "template '{}' returns opaque type '{}' — only Html is supported as template return type",
                        t.name, t.return_type
                    ));
                }
            }
            _ => {}
        }
    }

    // Second pass: cross-reference validation
    for decl in declarations {
        match decl {
            Declaration::EntityRecord(e) => {
                if !entity_types.contains(&e.type_name) {
                    result.errors.push(format!(
                        "entity '{}' references unknown type '{}'",
                        e.name, e.type_name
                    ));
                }
                if let Some(fields) = get_type_fields(declarations, &e.type_name) {
                    for init in &e.fields {
                        if !fields.contains(&init.name.as_str()) {
                            result.errors.push(format!(
                                "entity '{}' initializes unknown field '{}' on type '{}'",
                                e.name, init.name, e.type_name
                            ));
                        }
                    }
                }
            }
            Declaration::EntitySimple(e) => {
                let known_primitives = [
                    "String",
                    "Float",
                    "Bool",
                    "Html",
                    "Query",
                    "Secret",
                    "Encrypted",
                    "Hash",
                    "Session",
                ];
                if !known_primitives.contains(&e.type_name.as_str())
                    && !entity_types.contains(&e.type_name)
                {
                    result.warnings.push(format!(
                        "entity '{}' uses undeclared type '{}' (may be a forward reference)",
                        e.name, e.type_name
                    ));
                }
            }
            Declaration::Rule(r) => {
                if let Expr::Ident(name) = &r.target {
                    if !entity_names.contains(name) {
                        result.errors.push(format!(
                            "rule target '{}' references undefined entity",
                            name
                        ));
                    }
                }
            }
            Declaration::Adapt(a) => {
                if !learnable_names.contains(&a.pattern_name) {
                    result.errors.push(format!(
                        "adapt: learnable pattern '{}' not found",
                        a.pattern_name
                    ));
                }
            }
            Declaration::Mutate(m) => {
                if !learnable_names.contains(&m.pattern_name) {
                    result.errors.push(format!(
                        "mutate: learnable pattern '{}' not found",
                        m.pattern_name
                    ));
                }
            }
            Declaration::Eval(e) => {
                if !learnable_names.contains(&e.pattern_name) {
                    result.errors.push(format!(
                        "eval: learnable pattern '{}' not found",
                        e.pattern_name
                    ));
                }
                if e.dataset.is_empty() {
                    result.warnings.push(format!(
                        "eval '{}': dataset is empty — eval will trivially pass",
                        e.pattern_name
                    ));
                }
            }
            Declaration::Flow(f) => {
                for step in &f.pipeline {
                    let known = pattern_names.contains(step)
                        || learnable_names.contains(step)
                        || builtin_names.contains(step)
                        || step == "recall";
                    if !known {
                        let has_branch_def = f.branch_defs.iter().any(|(name, _)| name == step);
                        if !has_branch_def {
                            result.errors.push(format!(
                                "flow '{}': pipeline step '{}' is not a known pattern, builtin, or branch definition",
                                f.name, step
                            ));
                        }
                    }
                }
                for (_, branches) in &f.branch_defs {
                    for branch in branches {
                        if !pattern_names.contains(&branch.target)
                            && !learnable_names.contains(&branch.target)
                            && !builtin_names.contains(&branch.target)
                        {
                            result.errors.push(format!(
                                "flow '{}': branch '{}' target '{}' is not a known pattern",
                                f.name, branch.label, branch.target
                            ));
                        }
                    }
                }
            }
            Declaration::Pattern(p) => {
                // Walk expression tree: check arity and undefined functions
                for stmt in &p.body {
                    check_stmt_exprs(
                        stmt,
                        &builtin_names,
                        &pattern_param_counts,
                        &learnable_names,
                        &mut result.errors,
                    );
                }
            }
            // Phase 6.1: Validate mlogserver block
            Declaration::MlogServer(srv) => {
                // Validate middleware names
                for mw in &srv.middleware {
                    if !VALID_MIDDLEWARE.contains(&mw.as_str()) {
                        result.errors.push(format!(
                            "mlogserver: unknown middleware '{}'. Valid: {:?}",
                            mw, VALID_MIDDLEWARE
                        ));
                    }
                }
                // Validate route methods and role references
                for route in &srv.routes {
                    if !VALID_METHODS.contains(&route.method.as_str()) {
                        result.errors.push(format!(
                            "route '{}': unknown HTTP method '{}'. Valid: {:?}",
                            route.path, route.method, VALID_METHODS
                        ));
                    }
                    for role in &route.requires {
                        // Collect role names for cross-reference
                        role_names.insert(role.clone());
                    }
                }
                // Warn if no security_headers middleware
                if !srv.middleware.contains(&"security_headers".to_string()) {
                    result.warnings.push(
                        "mlogserver: no 'security_headers' middleware — recommend adding it for OWASP compliance".to_string()
                    );
                }
                // Warn if POST routes but no csrf middleware
                let has_post = srv
                    .routes
                    .iter()
                    .any(|r| r.method == "POST" || r.method == "PUT" || r.method == "DELETE");
                if has_post && !srv.middleware.contains(&"csrf".to_string()) {
                    result.warnings.push(
                        "mlogserver: has mutating routes but no 'csrf' middleware — recommend adding it".to_string()
                    );
                }
            }
            _ => {}
        }
    }

    // ── Нарjad №74: SVG/HTML security lint (ADR-0102) ──
    // AST-level analysis: detect potential injection vectors that could
    // bypass runtime escaping. See `svg_security_lint` docstring below.
    svg_security_lint(declarations, &mut result);

    result
}

// ── Наряд №74: SVG/HTML Security Lint ────────────────────────────────
//
// Walks the AST of every declaration and inspects all `Expr::FnCall`
// nodes. For each call to an SVG/HTML-emitting builtin (svg_text,
// svg_callout, svg_path, svg_canvas, svg_group, chart_*, diagram_*,
// html_response, escape_html), it inspects the string-literal arguments.
//
// Findings:
//
//   ERROR (potential bypass):
//     A string literal containing `<script`, `javascript:`, or `on\w+=`
//     is passed to a builtin that does NOT auto-escape that argument
//     (e.g. svg_path's `d` argument is structural and not escaped).
//     Also: a `<script` literal appearing in any string concatenation
//     that ends up in an HTML context.
//
//   WARNING (suspicious but auto-escaped):
//     A string literal containing `<script>`, `on\w+=`, etc. passed to
//     a builtin that DOES auto-escape (svg_text content, svg_callout
//     text). Runtime will escape it correctly, but the source intent
//     looks like an attempted injection — worth flagging for review.
//
// This is defense-in-depth: runtime escaping (escape_html_chars in
// svg_text, svg_callout) is the primary barrier. The lint catches the
// case where an attacker could bypass escaping by passing the payload
// to a non-escaping argument (svg_path d, svg_canvas viewbox, etc.).
//
// Whitelist: Google Fonts URLs ("https://fonts.googleapis.com/...")
// are explicitly permitted in href/src contexts — the only external
// resource allowed (matches the source repo's self_check.py rule).

/// Builtins whose string arguments are auto-escaped at runtime.
/// String literals with `<script>` here generate a WARNING (suspicious
/// but safe — runtime will escape).
///
/// chart_bar / chart_donut / chart_line / chart_area accept user labels
/// inside `data: List<Struct{label, value}>` at arg 0. Their labels are
/// escaped via escape_html_chars at runtime (defense-in-depth).
/// chart_scatter uses `List<Struct{x, y, label?}>` — same `label` key,
/// but optional and at a different struct position. The walker has a
/// special case (scan_chart_labels) that scans the list-of-structs
/// pattern by field NAME, so it works uniformly across all five shapes.
/// chart_boxplot uses `List<Struct{label, values}>` — same `label` key
/// in a list-of-structs, so scan_chart_labels covers it without changes.
/// chart_radar uses a DIFFERENT top-level shape (Struct{axes, series},
/// not List<Struct>), so it has its own scanner (scan_radar_labels).
/// chart_heatmap is intentionally NOT in this list — its data is purely
/// numeric (List<List<Float>>), there is no user text to scan.
const SVG_AUTO_ESCAPE_BUILTINS: &[&str] = &[
    "svg_text",
    "svg_callout",
    "chart_bar",
    "chart_donut",
    "chart_line",
    "chart_scatter",
    "chart_area",
    "chart_radar",
    "chart_boxplot",
];

/// Builtins whose string arguments are NOT auto-escaped (structural).
/// String literals with `<script>` here generate an ERROR (injection
/// vector — runtime cannot catch it).
const SVG_NO_ESCAPE_BUILTINS: &[&str] = &[
    "svg_path",           // d is path-data mini-language, not escaped
    "svg_canvas",         // viewbox is structural, not escaped
    "svg_group",          // transform is structural, not escaped
    "svg_sketchy_filter", // id is structural, validated but not escaped as text
];

/// Argument indices that are auto-escaped within SVG_AUTO_ESCAPE_BUILTINS.
/// For svg_text: arg 2 (content). For svg_callout: arg 0 (text).
fn auto_escaped_arg_index(builtin: &str) -> Option<usize> {
    match builtin {
        "svg_text" => Some(2),
        "svg_callout" => Some(0),
        _ => None,
    }
}

/// Detect potential XSS/injection payload in a string literal.
/// Returns Some(reason) if the string looks dangerous.
fn detect_xss_payload(s: &str) -> Option<&'static str> {
    let lower = s.to_lowercase();
    if lower.contains("<script") {
        return Some("contains <script> tag");
    }
    if lower.contains("javascript:") {
        return Some("contains javascript: URL");
    }
    // on<event>= attributes: onclick=, onload=, onerror=, etc.
    // Match `on` followed by 2+ letters followed by `=`
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 4 < bytes.len() {
        if bytes[i] == b'o' && bytes[i + 1] == b'n' {
            // Check that what follows is letters then '='
            let mut j = i + 2;
            let mut letter_count = 0;
            while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
                letter_count += 1;
                j += 1;
            }
            if letter_count >= 2 && j < bytes.len() && bytes[j] == b'=' {
                return Some("contains onX event handler attribute");
            }
        }
        i += 1;
    }
    None
}

/// Check if a URL is on the whitelist (Google Fonts only).
fn is_whitelisted_url(url: &str) -> bool {
    let trimmed = url.trim();
    // Allow Google Fonts CSS and font files
    if trimmed.starts_with("https://fonts.googleapis.com/") {
        return true;
    }
    if trimmed.starts_with("https://fonts.gstatic.com/") {
        return true;
    }
    // Allow data: URIs for SVG inline (common for icons)
    if trimmed.starts_with("data:image/svg+xml") {
        return true;
    }
    false
}

/// Scan chart_* `data` arg (arg 0) for XSS payloads in label string
/// literals. The data arg is a List literal of Struct literals:
///   chart_bar / chart_donut / chart_line / chart_area:
///     [{label: "...", value: 10.0}, ...]
///   chart_scatter:
///     [{x: 1.0, y: 2.0, label: "..."}, ...]   (label OPTIONAL)
///   chart_boxplot:
///     [{label: "...", values: [1.0, 2.0, ...]}, ...]
/// For each struct, we look up the `label` field BY NAME (not position).
/// This makes the scanner shape-agnostic: it catches <script> in label
/// regardless of whether label is the first, second, or third field, and
/// regardless of whether other fields (x, y, value, values) are present.
/// If the struct has no `label` field at all (legal for chart_scatter),
/// the scanner simply skips it — no label, no injection vector.
///
/// If the `label` value is a StringLit with an XSS payload, we emit a
/// WARNING (runtime escapes label text via escape_html_chars — this is
/// a defense-in-depth review hint, not a hard error).
fn scan_chart_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::List(items)) = args.first() {
        for item in items {
            if let Expr::StructLit(fields) = item {
                if let Some(Expr::StringLit(s)) = fields.get("label") {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(format!(
                            "security: {} data[].label string literal {} — runtime will escape, but review intent",
                            fn_name, reason
                        ));
                    }
                }
            }
        }
    }
}

/// Scan chart_radar `data` arg (arg 0) for XSS payloads. Radar has a
/// DIFFERENT top-level shape from other chart_* builtins — it's a
/// Struct with two List fields, not a List of Structs:
///   Struct {
///     axes:  List<String>,                  // scan each StringLit
///     series: List<Struct{name, values}>,   // scan each series.name
///   }
///
/// We look up `axes` and `series` by field name on the top-level
/// StructLit. For `axes`, the elements are StringLits directly (no
/// struct wrapping) — so we scan them in place. For `series`, each
/// element is a StructLit and we look up its `name` field by key
/// (same approach as scan_chart_labels, just one struct level deeper).
///
/// If any string literal carries an XSS payload, we emit a WARNING
/// (runtime escapes via escape_html_chars — defense-in-depth, not a
/// hard error). The warning identifies WHICH field is suspicious
/// (axes vs series[].name) so the caller can locate the input.
fn scan_radar_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit(fields)) = args.first() {
        // axes: List<String> — scan each StringLit directly
        if let Some(Expr::List(axes_items)) = fields.get("axes") {
            for axis in axes_items {
                if let Expr::StringLit(s) = axis {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(format!(
                            "security: {} data.axes string literal {} — runtime will escape, but review intent",
                            fn_name, reason
                        ));
                    }
                }
            }
        }
        // series: List<Struct{name, values}> — scan each series.name
        if let Some(Expr::List(series_items)) = fields.get("series") {
            for item in series_items {
                if let Expr::StructLit(series_fields) = item {
                    if let Some(Expr::StringLit(s)) = series_fields.get("name") {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(format!(
                                "security: {} data.series[].name string literal {} — runtime will escape, but review intent",
                                fn_name, reason
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Walk an expression and run the security check on every FnCall node.
fn walk_expr_for_svg_security(expr: &Expr, result: &mut AnalysisResult, ctx: &str) {
    match expr {
        Expr::FnCall(name, args) => {
            // Check string-literal arguments to SVG builtins
            if SVG_AUTO_ESCAPE_BUILTINS.contains(&name.as_str()) {
                if let Some(content_idx) = auto_escaped_arg_index(name) {
                    if let Some(Expr::StringLit(s)) = args.get(content_idx) {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(format!(
                                "security: {}({} arg) string literal {} — runtime will escape, but review intent",
                                name, content_idx + 1, reason
                            ));
                        }
                    }
                }
                // Special case for chart_* builtins: their `label` field
                // lives inside `data: List<Struct{...}>` at arg 0. The list
                // element shape varies:
                //   chart_bar / chart_donut / chart_line / chart_area:
                //     {label, value}  — label first
                //   chart_scatter:
                //     {x, y, label?}  — label third, optional
                //   chart_boxplot:
                //     {label, values} — label first, but values is a List
                // scan_chart_labels looks up `label` BY NAME, so it works
                // uniformly across all these list-of-struct shapes.
                //
                // chart_radar has a DIFFERENT top-level shape
                // (Struct{axes, series}, not List<Struct>), so it gets
                // its own scanner (scan_radar_labels) that walks both
                // the `axes` List<String> and the `series[].name` field.
                //
                // chart_heatmap is intentionally NOT scanned — its data
                // is `List<List<Float>>` (pure numeric, no user text).
                // Runtime escapes label text via escape_html_chars, so
                // these are WARNINGs (suspicious intent) not ERRORs.
                if name == "chart_bar"
                    || name == "chart_donut"
                    || name == "chart_line"
                    || name == "chart_area"
                    || name == "chart_scatter"
                    || name == "chart_boxplot"
                {
                    scan_chart_labels(name, args, result);
                }
                if name == "chart_radar" {
                    scan_radar_labels(name, args, result);
                }
            }
            if SVG_NO_ESCAPE_BUILTINS.contains(&name.as_str()) {
                // Check ALL string-literal args — any of them could be an injection vector
                for (i, arg) in args.iter().enumerate() {
                    if let Expr::StringLit(s) = arg {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.errors.push(format!(
                                "security: {}({} arg) string literal {} — this builtin does NOT auto-escape this argument; potential injection vector",
                                name, i + 1, reason
                            ));
                        }
                        // For svg_canvas, check viewbox arg specifically
                        if name == "svg_canvas" && i == 2 {
                            // viewbox should be 4 numbers — any other format is suspicious
                            let parts: Vec<&str> = s.split_whitespace().collect();
                            if parts.len() != 4 || parts.iter().any(|p| p.parse::<f64>().is_err()) {
                                result.warnings.push(format!(
                                    "security: svg_canvas viewbox argument should be 4 numbers, got {:?}",
                                    s
                                ));
                            }
                        }
                    } else {
                        // Recurse into non-literal expressions (concat, list, etc.)
                        // to catch <script> hidden inside e.g. "M 10 10 " + "<script>"
                        walk_expr_for_svg_security(arg, result, ctx);
                    }
                }
            }
            // Check for external URLs in non-whitelisted contexts
            // (e.g. svg_icon color, svg_text fill) — these are colors, not URLs,
            // but if someone passes "javascript:..." as a color, that's suspicious
            for (i, arg) in args.iter().enumerate() {
                if let Expr::StringLit(s) = arg {
                    if s.starts_with("javascript:") || s.starts_with("data:text/html") {
                        result.errors.push(format!(
                            "security: {}({} arg) contains potentially dangerous URL scheme: {:?}",
                            name,
                            i + 1,
                            s
                        ));
                    }
                }
            }
            // Recurse into arguments
            for arg in args {
                walk_expr_for_svg_security(arg, result, ctx);
            }
        }
        Expr::QualifiedCall { function, args, .. } => {
            for arg in args {
                walk_expr_for_svg_security(arg, result, ctx);
            }
            let _ = function;
        }
        Expr::BinaryOp(lhs, _, rhs) => {
            // String concatenation: walk BOTH sides to scan all string literals.
            // If a StringLit appears inside a concat that's an arg to an SVG
            // no-escape builtin, the surrounding FnCall walker has already
            // recursed into us (via the `else` branch). Here we additionally
            // scan the immediate StringLit children of BinaryOp for XSS payloads
            // — this catches concat expressions where the FnCall walker didn't
            // flag them because the payload is buried inside a BinaryOp.
            if let Expr::StringLit(s) = lhs.as_ref() {
                if let Some(reason) = detect_xss_payload(s) {
                    // This is a WARNING only — we don't know if this concat
                    // feeds an SVG context. The FnCall walker will escalate
                    // to ERROR if appropriate.
                    result.warnings.push(format!(
                        "security: string literal in concatenation {} — review usage",
                        reason
                    ));
                }
            }
            if let Expr::StringLit(s) = rhs.as_ref() {
                if let Some(reason) = detect_xss_payload(s) {
                    result.warnings.push(format!(
                        "security: string literal in concatenation {} — review usage",
                        reason
                    ));
                }
            }
            walk_expr_for_svg_security(lhs, result, ctx);
            walk_expr_for_svg_security(rhs, result, ctx);
        }
        Expr::IfElse(cond, then_e, else_e) => {
            walk_expr_for_svg_security(cond, result, ctx);
            walk_expr_for_svg_security(then_e, result, ctx);
            walk_expr_for_svg_security(else_e, result, ctx);
        }
        Expr::List(items) => {
            for item in items {
                walk_expr_for_svg_security(item, result, ctx);
            }
        }
        Expr::StructLit(fields) => {
            for v in fields.values() {
                walk_expr_for_svg_security(v, result, ctx);
            }
        }
        Expr::FieldAccess(inner, _) => {
            walk_expr_for_svg_security(inner, result, ctx);
        }
        Expr::IndexAccess(inner, idx) => {
            walk_expr_for_svg_security(inner, result, ctx);
            walk_expr_for_svg_security(idx, result, ctx);
        }
        Expr::Try(inner) => {
            walk_expr_for_svg_security(inner, result, ctx);
        }
        Expr::BlockIfElse {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            walk_expr_for_svg_security(condition, result, ctx);
            for s in then_body {
                walk_stmt_for_svg_security(s, result, ctx);
            }
            for (cond, body) in else_ifs {
                walk_expr_for_svg_security(cond, result, ctx);
                for s in body {
                    walk_stmt_for_svg_security(s, result, ctx);
                }
            }
            if let Some(body) = else_body {
                for s in body {
                    walk_stmt_for_svg_security(s, result, ctx);
                }
            }
        }
        Expr::StringLit(_) | Expr::FloatLit(_) | Expr::BoolLit(_) | Expr::Ident(_) => {}
    }
    let _ = ctx;
}

/// Walk a statement and run security check on every expression it contains.
fn walk_stmt_for_svg_security(stmt: &Statement, result: &mut AnalysisResult, ctx: &str) {
    match stmt {
        Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
            walk_expr_for_svg_security(value, result, ctx);
        }
        Statement::Return(e) => walk_expr_for_svg_security(e, result, ctx),
        Statement::ExprStmt(e) => walk_expr_for_svg_security(e, result, ctx),
        Statement::Each { iterable, body, .. } => {
            walk_expr_for_svg_security(iterable, result, ctx);
            for s in body {
                walk_stmt_for_svg_security(s, result, ctx);
            }
        }
        Statement::EachWithIndex { iterable, body, .. } => {
            walk_expr_for_svg_security(iterable, result, ctx);
            for s in body {
                walk_stmt_for_svg_security(s, result, ctx);
            }
        }
        Statement::While { condition, body } => {
            walk_expr_for_svg_security(condition, result, ctx);
            for s in body {
                walk_stmt_for_svg_security(s, result, ctx);
            }
        }
        Statement::IfElseBlock {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            walk_expr_for_svg_security(condition, result, ctx);
            for s in then_body {
                walk_stmt_for_svg_security(s, result, ctx);
            }
            for (cond, body) in else_ifs {
                walk_expr_for_svg_security(cond, result, ctx);
                for s in body {
                    walk_stmt_for_svg_security(s, result, ctx);
                }
            }
            if let Some(body) = else_body {
                for s in body {
                    walk_stmt_for_svg_security(s, result, ctx);
                }
            }
        }
        Statement::IfThen(cond, body) => {
            walk_expr_for_svg_security(cond, result, ctx);
            for s in body {
                walk_stmt_for_svg_security(s, result, ctx);
            }
        }
        Statement::Match {
            scrutinee,
            arms,
            else_body,
        } => {
            walk_expr_for_svg_security(scrutinee, result, ctx);
            for arm in arms {
                match arm {
                    MatchArm::Compare(_, e, body) => {
                        walk_expr_for_svg_security(e, result, ctx);
                        for s in body {
                            walk_stmt_for_svg_security(s, result, ctx);
                        }
                    }
                    MatchArm::Exact(_, body)
                    | MatchArm::StartsWith(_, body)
                    | MatchArm::Contains(_, body) => {
                        for s in body {
                            walk_stmt_for_svg_security(s, result, ctx);
                        }
                    }
                }
            }
            if let Some(body) = else_body {
                for s in body {
                    walk_stmt_for_svg_security(s, result, ctx);
                }
            }
        }
        Statement::Break | Statement::Continue => {}
    }
}

/// Walk all declarations and run the SVG/HTML security lint.
fn svg_security_lint(declarations: &[Declaration], result: &mut AnalysisResult) {
    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                let ctx = format!("pattern {}", p.name);
                for s in &p.body {
                    walk_stmt_for_svg_security(s, result, &ctx);
                }
            }
            Declaration::Flow(f) => {
                let ctx = format!("flow {}", f.name);
                walk_expr_for_svg_security(&f.source, result, &ctx);
                for (_, branches) in &f.branch_defs {
                    for branch in branches {
                        // Branch bodies can contain expressions in pipeline steps
                        let _ = branch;
                    }
                }
            }
            Declaration::EntitySimple(e) => {
                let ctx = format!("entity {}", e.name);
                walk_expr_for_svg_security(&e.value, result, &ctx);
            }
            Declaration::EntityRecord(e) => {
                let ctx = format!("entity {}", e.name);
                for f in &e.fields {
                    walk_expr_for_svg_security(&f.value, result, &ctx);
                }
            }
            Declaration::MlogServer(srv) => {
                let ctx = "mlogserver".to_string();
                for route in &srv.routes {
                    for s in &route.body {
                        walk_stmt_for_svg_security(s, result, &ctx);
                    }
                }
            }
            Declaration::Template(t) => {
                // Template body is a raw string with {{ var }} placeholders,
                // not parsed statements. We do a simple text scan for XSS payloads.
                let ctx = format!("template {}", t.name);
                if let Some(reason) = detect_xss_payload(&t.body) {
                    result.errors.push(format!(
                        "security: template '{}' body {} — templates are raw HTML, no auto-escaping",
                        t.name, reason
                    ));
                }
                let _ = ctx;
            }
            _ => {}
        }
    }
    // Suppress unused warning for is_whitelisted_url (kept for future URL-context checks)
    let _ = is_whitelisted_url("https://fonts.googleapis.com/css");
}

/// Helper: extract field names from an EntityType declaration.
fn get_type_fields<'a>(declarations: &'a [Declaration], type_name: &str) -> Option<Vec<&'a str>> {
    for decl in declarations {
        if let Declaration::EntityType(e) = decl {
            if e.name == type_name {
                return Some(e.fields.iter().map(|f| f.name.as_str()).collect());
            }
        }
    }
    None
}

/// Walk an expression tree, checking FnCall arity and detecting undefined functions.
fn check_expr_calls(
    expr: &Expr,
    builtin_names: &HashSet<String>,
    pattern_param_counts: &HashSet<(String, usize)>,
    learnable_names: &HashSet<String>,
    errors: &mut Vec<String>,
) {
    if let Expr::FnCall(name, args) = expr {
        let is_known = builtin_names.contains(name)
            || pattern_param_counts.iter().any(|(n, _)| n == name)
            || learnable_names.contains(name);

        const INTERCEPTED_FUNCTIONS: &[&str] = &["recall_top_k"];

        if !is_known && !INTERCEPTED_FUNCTIONS.contains(&name.as_str()) {
            errors.push(format!(
                "undefined: function '{}' is not a builtin, pattern, or learnable",
                name
            ));
        }

        // Check builtin arity
        if builtin_names.contains(name) {
            if let Err(e) = crate::builtins::check_builtin_arity(name, args.len()) {
                errors.push(e);
            }
        }

        // Check pattern param count
        for (pname, pcount) in pattern_param_counts {
            if *pname == *name && !builtin_names.contains(name) {
                if args.len() != *pcount {
                    errors.push(format!(
                        "function '{}' expects {} argument(s), got {}",
                        name,
                        pcount,
                        args.len()
                    ));
                }
                break;
            }
        }

        // Recurse into arguments
        for arg in args {
            check_expr_calls(
                arg,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
    } else if let Expr::BinaryOp(left, _op, right) = expr {
        check_expr_calls(
            left,
            builtin_names,
            pattern_param_counts,
            learnable_names,
            errors,
        );
        check_expr_calls(
            right,
            builtin_names,
            pattern_param_counts,
            learnable_names,
            errors,
        );
    } else if let Expr::IfElse(cond, then_br, else_br) = expr {
        check_expr_calls(
            cond,
            builtin_names,
            pattern_param_counts,
            learnable_names,
            errors,
        );
        check_expr_calls(
            then_br,
            builtin_names,
            pattern_param_counts,
            learnable_names,
            errors,
        );
        check_expr_calls(
            else_br,
            builtin_names,
            pattern_param_counts,
            learnable_names,
            errors,
        );
    } else if let Expr::List(items) = expr {
        for item in items {
            check_expr_calls(
                item,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
    } else if let Expr::IndexAccess(inner, idx) = expr {
        check_expr_calls(
            inner,
            builtin_names,
            pattern_param_counts,
            learnable_names,
            errors,
        );
        check_expr_calls(
            idx,
            builtin_names,
            pattern_param_counts,
            learnable_names,
            errors,
        );
    } else if let Expr::StructLit(fields) = expr {
        for v in fields.values() {
            check_expr_calls(
                v,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
    }
}

/// Extract expressions from a statement and walk them for arity/undefined checks.
fn check_stmt_exprs(
    stmt: &Statement,
    builtin_names: &HashSet<String>,
    pattern_param_counts: &HashSet<(String, usize)>,
    learnable_names: &HashSet<String>,
    errors: &mut Vec<String>,
) {
    match stmt {
        Statement::LetBinding { value, .. } => {
            check_expr_calls(
                value,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
        Statement::Assign { value, .. } => {
            check_expr_calls(
                value,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
        Statement::Return(expr) => {
            check_expr_calls(
                expr,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
        Statement::ExprStmt(expr) => {
            check_expr_calls(
                expr,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
        Statement::Each { iterable, body, .. } => {
            check_expr_calls(
                iterable,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
            for s in body {
                check_stmt_exprs(
                    s,
                    builtin_names,
                    pattern_param_counts,
                    learnable_names,
                    errors,
                );
            }
        }
        Statement::EachWithIndex { iterable, body, .. } => {
            check_expr_calls(
                iterable,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
            for s in body {
                check_stmt_exprs(
                    s,
                    builtin_names,
                    pattern_param_counts,
                    learnable_names,
                    errors,
                );
            }
        }
        Statement::While { condition, body } => {
            check_expr_calls(
                condition,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
            for s in body {
                check_stmt_exprs(
                    s,
                    builtin_names,
                    pattern_param_counts,
                    learnable_names,
                    errors,
                );
            }
        }
        Statement::IfElseBlock {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            check_expr_calls(
                condition,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
            for s in then_body {
                check_stmt_exprs(
                    s,
                    builtin_names,
                    pattern_param_counts,
                    learnable_names,
                    errors,
                );
            }
            for (cond, body) in else_ifs {
                check_expr_calls(
                    cond,
                    builtin_names,
                    pattern_param_counts,
                    learnable_names,
                    errors,
                );
                for s in body {
                    check_stmt_exprs(
                        s,
                        builtin_names,
                        pattern_param_counts,
                        learnable_names,
                        errors,
                    );
                }
            }
            if let Some(else_body) = else_body {
                for s in else_body {
                    check_stmt_exprs(
                        s,
                        builtin_names,
                        pattern_param_counts,
                        learnable_names,
                        errors,
                    );
                }
            }
        }
        Statement::IfThen(cond, body) => {
            check_expr_calls(
                cond,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
            for s in body {
                check_stmt_exprs(
                    s,
                    builtin_names,
                    pattern_param_counts,
                    learnable_names,
                    errors,
                );
            }
        }
        Statement::Match {
            scrutinee,
            arms,
            else_body,
        } => {
            check_expr_calls(
                scrutinee,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
            for arm in arms {
                match arm {
                    MatchArm::Exact(_, body)
                    | MatchArm::StartsWith(_, body)
                    | MatchArm::Contains(_, body) => {
                        for s in body {
                            check_stmt_exprs(
                                s,
                                builtin_names,
                                pattern_param_counts,
                                learnable_names,
                                errors,
                            );
                        }
                    }
                    MatchArm::Compare(_, expr, body) => {
                        check_expr_calls(
                            expr,
                            builtin_names,
                            pattern_param_counts,
                            learnable_names,
                            errors,
                        );
                        for s in body {
                            check_stmt_exprs(
                                s,
                                builtin_names,
                                pattern_param_counts,
                                learnable_names,
                                errors,
                            );
                        }
                    }
                }
            }
            if let Some(else_body) = else_body {
                for s in else_body {
                    check_stmt_exprs(
                        s,
                        builtin_names,
                        pattern_param_counts,
                        learnable_names,
                        errors,
                    );
                }
            }
        }
        Statement::Break | Statement::Continue => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ok_program() {
        let source = r#"
            entity greeting: String = "Hello, Metalogos!"
            pattern SayHello(text: String) -> String { return text }
            flow Main { input: String = greeting -> SayHello -> output }
        "#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(result.is_ok());
        assert_eq!(result.error_count(), 0);
    }

    #[test]
    fn test_undefined_type() {
        let source = r#"
            entity m: UnknownType = { text: "hi" }
        "#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result.errors.iter().any(|e| e.contains("unknown type")));
    }

    #[test]
    fn test_adapt_target_not_found() {
        let source = r#"
            adapt NonExistent add_example("in", "out")
        "#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result.errors.iter().any(|e| e.contains("not found")));
    }

    #[test]
    fn test_duplicate_pattern() {
        let source = r#"
            pattern Foo(x: String) -> String { return x }
            pattern Foo(y: String) -> String { return y }
        "#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("duplicate pattern")));
    }

    // ── Phase 6 semantic tests ────────────────────────────────

    #[test]
    fn test_mlogserver_valid() {
        let source = r#"
mlogserver {
  port: 8080
  middleware: [session, csrf, security_headers]
  route "/" method=GET { return "Hello" }
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(result.is_ok(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_mlogserver_unknown_middleware() {
        let source = r#"
mlogserver {
  middleware: [bogus_middleware]
  route "/" method=GET { return "Hello" }
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("unknown middleware")));
    }

    #[test]
    fn test_mlogserver_invalid_method() {
        let source = r#"
mlogserver {
  route "/" method=INVALID { return "Hello" }
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("unknown HTTP method")));
    }

    #[test]
    fn test_mlogserver_warns_no_security_headers() {
        let source = r#"
mlogserver {
  route "/" method=GET { return "Hello" }
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(result.is_ok()); // Only warning, not error
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("security_headers")));
    }

    #[test]
    fn test_mlogserver_warns_no_csrf_with_post() {
        let source = r#"
mlogserver {
  middleware: [session, security_headers]
  route "/login" method=POST { return "OK" }
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(result.is_ok());
        assert!(result.warnings.iter().any(|w| w.contains("csrf")));
    }

    #[test]
    fn test_template_valid() {
        let source = r#"
template Page(title: String) -> Html {
  <h1>{{ title }}</h1>
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(result.is_ok());
    }

    #[test]
    fn test_template_wrong_return_type() {
        let source = r#"
template Page(title: String) -> Secret {
  <h1>{{ title }}</h1>
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("only Html is supported")));
    }
}
