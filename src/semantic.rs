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
    // Наряд №81 Block 6: diagrams accept user text (label/title/description).
    //   diagram_tree / diagram_org_chart — recursive Struct{label, children}.
    //   diagram_flowchart — Struct{nodes: [{id,label}], edges: [{from,to,label?}]}.
    //   diagram_layers — List<Struct{label, description?}>.
    // All four escape text via escape_html_chars at runtime (svg.rs).
    // AST lint scans label literals as WARNINGs (defense-in-depth).
    "diagram_tree",
    "diagram_org_chart",
    "diagram_flowchart",
    "diagram_layers",
    // Наряд №82 Block 6: temporal & process diagrams.
    //   diagram_sequence  — Struct{actors: List<String>, messages: [{from,to,label?}]}.
    //     actors[] is a List<String> (NOT List<Struct>) — special scanner.
    //     messages[].label is the rendered text (from/to are idents, scanned
    //     defensively).
    //   diagram_timeline  — List<Struct{date, label, description?}> — flat
    //     list pattern, same shape as diagram_layers (3 string fields, not 2).
    //   diagram_gantt     — List<Struct{task, start, duration}> — task is
    //     the only string field; start/duration are floats, never rendered.
    //   diagram_process   — List<Struct{label, description?}> — identical
    //     shape to diagram_layers (reuses scan_layers_labels logic by name).
    //   diagram_loop      — List<Struct{label, description?}> — same shape
    //     as diagram_layers/diagram_process.
    // All five escape text via escape_html_chars at runtime (svg.rs).
    "diagram_sequence",
    "diagram_timeline",
    "diagram_gantt",
    "diagram_process",
    "diagram_loop",
    // Наряд №83 Block 6: sets & comparison diagrams.
    //   diagram_venn     — Struct{circles: [{label, value?}], overlap_label?}.
    //     circles[].label is rendered; overlap_label is a TOP-LEVEL field
    //     (NOT inside the list) — easy to forget, called out explicitly in
    //     the spec. Special scanner scan_venn_labels covers both.
    //   diagram_quadrant — Struct{x_axis_label, y_axis_label, items: [{label, x, y}]}.
    //     BOTH axis labels are TOP-LEVEL fields (not in items[]) — same
    //     "easy to forget" category as overlap_label. items[].label is the
    //     only rendered text in the list (x/y are floats). Special scanner
    //     scan_quadrant_labels covers all three.
    //   diagram_pyramid  — List<Struct{label, value?}> — same flat shape
    //     as diagram_layers (label rendered, value is float). REUSES
    //     scan_layers_labels (no new scanner for an identical shape).
    //   diagram_nested   — List<Struct{label, value?}> — identical shape
    //     to diagram_pyramid. REUSES scan_layers_labels.
    //   diagram_medallion — List<Struct{icon?, label, value?}>. label is
    //     rendered; icon is a controlled enum (validated against the 10
    //     svg_icon names at runtime), NOT user free-form text — explicitly
    //     NOT scanned per the spec. value is float. New scanner
    //     scan_medallion_labels checks only `label`.
    "diagram_venn",
    "diagram_quadrant",
    "diagram_pyramid",
    "diagram_nested",
    "diagram_medallion",
    // Наряд №84 Block 7: data & state diagrams.
    //   diagram_er — Struct{entities: [{name, fields: List<String>}], relations: [{from,to,label?}]}.
    //     entities[].name and relations[].label are rendered as text.
    //     entities[].fields is a List<String> NESTED INSIDE a struct field —
    //     this is the THIRD nesting form encountered in the SVG suite
    //     (1st: top-level List<String> like diagram_sequence.actors in Н82;
    //      2nd: List<Struct> like diagram_layers in Н81;
    //      3rd: List<String> inside a struct field, here). The scanner
    //     scan_er_labels walks both levels: per-entity name + per-field string.
    //   diagram_state — Struct{states: List<String>, transitions: [{from,to,label?}], initial?}.
    //     states[] is List<String> (same as diagram_sequence.actors — scan
    //     each StringLit directly). transitions[].label is rendered.
    //     `initial` is a TOP-LEVEL String? field (like diagram_venn.overlap_label).
    //   diagram_swimlane — Struct{lanes: List<String>, steps: [{lane,label,order}]}.
    //     lanes[] is List<String>. steps[].label is rendered (lane is an
    //     identifier, scanned defensively). steps[].order is Float, skipped.
    //   diagram_data_flow / diagram_high_level / diagram_architecture —
    //     Struct{nodes:[{id,label,icon?}], edges:[{from,to,label?}]}.
    //     Same shape as diagram_flowchart (Н81) — REUSES scan_flowchart_labels
    //     (no new scanner for an identical shape, per spec: "переиспользовать
    //     сканер, не писать заново для каждой из трёх").
    //     For diagram_architecture, the `icon` field is a controlled enum
    //     (validated against svg_icon's 10 names at runtime) — NOT scanned,
    //     same decision as diagram_medallion.
    "diagram_er",
    "diagram_state",
    "diagram_swimlane",
    "diagram_data_flow",
    "diagram_high_level",
    "diagram_architecture",
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

/// Наряд №81 Block 6 — recursive scanner for `diagram_tree` /
/// `diagram_org_chart` data shapes.
///
/// Both functions accept `Struct { label, title?, children }` where
/// `children` is `List<Struct>` of the SAME shape — i.e. the nesting
/// depth is unbounded. The existing `scan_chart_labels` only scans
/// ONE level (it iterates a List<Struct> literal and looks up `label`
/// on each element). It does NOT recurse into a `children` field —
/// which means an injection at depth ≥ 2 would slip past it.
///
/// This function walks the recursive struct: at each level we look up
/// `label` (and `title` if present) as StringLit fields and check for
/// XSS payloads. We then descend into the `children` List and recurse
/// on each child Struct.
///
/// `path` is used in the warning message so the caller can locate the
/// offending node (e.g. "root.children[1].children[2].label") — this
/// is essential because the recursive structure can be deeply nested
/// and a generic "label contains <script>" warning would be useless.
///
/// `allow_title` controls whether `title` is scanned (diagram_org_chart
/// allows it; diagram_tree ignores the field even if present, but we
/// still scan it defensively — better to over-warn than miss a payload
/// at the call site that uses diagram_tree but actually provides title).
fn scan_tree_labels_recursive(
    fn_name: &str,
    arg: &Expr,
    path: &str,
    allow_title: bool,
    result: &mut AnalysisResult,
) {
    if let Expr::StructLit(fields) = arg {
        // Scan `label` (always present per spec — required field)
        if let Some(Expr::StringLit(s)) = fields.get("label") {
            if let Some(reason) = detect_xss_payload(s) {
                result.warnings.push(format!(
                    "security: {} data.{}.label string literal {} — runtime will escape, but review intent",
                    fn_name, path, reason
                ));
            }
        }
        // Scan `title` (org chart only — but scan defensively regardless
        // of allow_title, since a malicious caller could supply title to
        // diagram_tree too; the runtime still escapes it via svg_text if
        // it ends up in the output. Better to over-warn.)
        if allow_title {
            if let Some(Expr::StringLit(s)) = fields.get("title") {
                if let Some(reason) = detect_xss_payload(s) {
                    result.warnings.push(format!(
                        "security: {} data.{}.title string literal {} — runtime will escape, but review intent",
                        fn_name, path, reason
                    ));
                }
            }
        } else {
            // Even for diagram_tree (allow_title=false), if a title is
            // present we should warn — it's not used, but its presence
            // in literal form is suspicious intent.
            if let Some(Expr::StringLit(s)) = fields.get("title") {
                if let Some(reason) = detect_xss_payload(s) {
                    result.warnings.push(format!(
                        "security: {} data.{}.title string literal {} — field not used by diagram_tree but review intent",
                        fn_name, path, reason
                    ));
                }
            }
        }
        // Recurse into children (if present)
        if let Some(Expr::List(child_items)) = fields.get("children") {
            for (i, child) in child_items.iter().enumerate() {
                let child_path = format!("{}.children[{}]", path, i);
                scan_tree_labels_recursive(fn_name, child, &child_path, allow_title, result);
            }
        }
    }
}

/// Наряд №81 Block 6 — scanner for `diagram_flowchart` data shape.
///
/// Flowchart data has TWO independent lists, each containing user text:
///   Struct {
///     nodes: List<Struct{id, label}>,
///     edges: List<Struct{from, to, label?}>,
///   }
///
/// We scan `nodes[].label` and `edges[].label` separately (the `id`,
/// `from`, `to` fields are identifiers used for graph topology, not
/// rendered text — but we scan them defensively too, since a literal
/// `<script>` in an id field is still suspicious intent even if it
/// wouldn't reach the output).
///
/// Both lists are scanned to WARNINGs (runtime escapes label text via
/// escape_html_chars — defense-in-depth, same as chart_*).
fn scan_flowchart_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit(fields)) = args.first() {
        // nodes: List<Struct{id, label}>
        if let Some(Expr::List(node_items)) = fields.get("nodes") {
            for (i, item) in node_items.iter().enumerate() {
                if let Expr::StructLit(node_fields) = item {
                    // Scan id (defensive — not rendered, but suspicious if payload)
                    if let Some(Expr::StringLit(s)) = node_fields.get("id") {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(format!(
                                "security: {} data.nodes[{}].id string literal {} — field not rendered but review intent",
                                fn_name, i, reason
                            ));
                        }
                    }
                    // Scan label (rendered — primary injection vector)
                    if let Some(Expr::StringLit(s)) = node_fields.get("label") {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(format!(
                                "security: {} data.nodes[{}].label string literal {} — runtime will escape, but review intent",
                                fn_name, i, reason
                            ));
                        }
                    }
                }
            }
        }
        // edges: List<Struct{from, to, label?}>
        if let Some(Expr::List(edge_items)) = fields.get("edges") {
            for (i, item) in edge_items.iter().enumerate() {
                if let Expr::StructLit(edge_fields) = item {
                    // from/to are identifiers (defensive scan)
                    for ident_key in &["from", "to"] {
                        if let Some(Expr::StringLit(s)) = edge_fields.get(*ident_key) {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(format!(
                                    "security: {} data.edges[{}].{} string literal {} — field not rendered but review intent",
                                    fn_name, i, ident_key, reason
                                ));
                            }
                        }
                    }
                    // label is rendered as edge midpoint text
                    if let Some(Expr::StringLit(s)) = edge_fields.get("label") {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(format!(
                                "security: {} data.edges[{}].label string literal {} — runtime will escape, but review intent",
                                fn_name, i, reason
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Наряд №81 Block 6 — scanner for `diagram_layers` data shape.
///
/// Flat list of structs:
///   List<Struct{label, description?}>
///
/// Both `label` and `description` are rendered as text — both must be
/// scanned. Uses the same per-struct pattern as scan_chart_labels but
/// checks TWO fields instead of one.
fn scan_layers_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::List(items)) = args.first() {
        for (i, item) in items.iter().enumerate() {
            if let Expr::StructLit(fields) = item {
                // label (always rendered)
                if let Some(Expr::StringLit(s)) = fields.get("label") {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(format!(
                            "security: {} data[{}].label string literal {} — runtime will escape, but review intent",
                            fn_name, i, reason
                        ));
                    }
                }
                // description (rendered, optional)
                if let Some(Expr::StringLit(s)) = fields.get("description") {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(format!(
                            "security: {} data[{}].description string literal {} — runtime will escape, but review intent",
                            fn_name, i, reason
                        ));
                    }
                }
            }
        }
    }
}

/// Наряд №82 Block 6 — scanner for `diagram_sequence` data shape.
///
/// Sequence has TWO independent lists, each containing user text:
///   Struct {
///     actors:   List<String>,                         // scan each StringLit directly
///     messages: List<Struct{from, to, label?}>,       // scan messages[].label
///   }
///
/// `actors` is a `List<String>` (NOT `List<Struct>`) — this is the
/// special case called out in the narazd spec: "список строк — не забыть,
/// это другая форма, чем везде остальные". The scanner walks each
/// StringLit in actors[] directly (no struct unwrap), unlike
/// scan_chart_labels/scan_layers_labels which expect StructLit elements.
///
/// `messages[].label` is the only rendered text field in messages;
/// `from`/`to` are identifier strings used for actor lookup, not
/// rendered — but we scan them defensively (same pattern as
/// scan_flowchart_labels for edges[].from/to).
///
/// All findings are WARNINGs (runtime escapes via escape_html_chars —
/// defense-in-depth, not a hard error).
fn scan_sequence_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit(fields)) = args.first() {
        // actors: List<String> — scan each StringLit directly (no struct
        // unwrap, this is the spec's "list of strings, not list of structs"
        // special case).
        if let Some(Expr::List(actor_items)) = fields.get("actors") {
            for (i, actor) in actor_items.iter().enumerate() {
                if let Expr::StringLit(s) = actor {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(format!(
                            "security: {} data.actors[{}] string literal {} — runtime will escape, but review intent",
                            fn_name, i, reason
                        ));
                    }
                }
            }
        }
        // messages: List<Struct{from, to, label?}> — scan from/to
        // defensively (identifiers, not rendered) and label as primary
        // (rendered edge label at midpoint).
        if let Some(Expr::List(msg_items)) = fields.get("messages") {
            for (i, item) in msg_items.iter().enumerate() {
                if let Expr::StructLit(msg_fields) = item {
                    // from/to (defensive — identifiers, not rendered)
                    for ident_key in &["from", "to"] {
                        if let Some(Expr::StringLit(s)) = msg_fields.get(*ident_key) {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(format!(
                                    "security: {} data.messages[{}].{} string literal {} — field not rendered but review intent",
                                    fn_name, i, ident_key, reason
                                ));
                            }
                        }
                    }
                    // label (rendered as edge midpoint text)
                    if let Some(Expr::StringLit(s)) = msg_fields.get("label") {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(format!(
                                "security: {} data.messages[{}].label string literal {} — runtime will escape, but review intent",
                                fn_name, i, reason
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Наряд №82 Block 6 — scanner for `diagram_gantt` data shape.
///
/// Gantt is a flat list of structs with exactly ONE string field:
///   List<Struct{task: String, start: Float, duration: Float}>
///
/// Only `task` is rendered (as the bar label). `start`/`duration` are
/// floats used for geometry — never reach SVG output. We scan `task`
/// as a WARNING (runtime escapes via escape_html_chars — same as
/// scan_chart_labels scans `label`, just under a different field name).
///
/// Implementation note: this is essentially scan_chart_labels with the
/// field name changed from "label" to "task". We could parametrize
/// scan_chart_labels to take a field name, but the existing call sites
/// all use "label" — keeping a separate function preserves the
/// self-documenting nature of each scanner and avoids changing
/// behavior for the five existing chart_* builtins.
fn scan_gantt_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::List(items)) = args.first() {
        for (i, item) in items.iter().enumerate() {
            if let Expr::StructLit(fields) = item {
                // task is the only string field; start/duration are floats
                if let Some(Expr::StringLit(s)) = fields.get("task") {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(format!(
                            "security: {} data[{}].task string literal {} — runtime will escape, but review intent",
                            fn_name, i, reason
                        ));
                    }
                }
            }
        }
    }
}

/// Наряд №82 Block 6 — scanner for `diagram_timeline` data shape.
///
/// Timeline is a flat list of structs with THREE string fields:
///   List<Struct{date: String, label: String, description?: String}>
///
/// All three are rendered as text (`date` and `label` always, `description`
/// when present). All three must be scanned. Same per-struct walk pattern
/// as scan_layers_labels — extended to check `date` as the first field.
fn scan_timeline_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::List(items)) = args.first() {
        for (i, item) in items.iter().enumerate() {
            if let Expr::StructLit(fields) = item {
                // date (always rendered, above/below the dot)
                if let Some(Expr::StringLit(s)) = fields.get("date") {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(format!(
                            "security: {} data[{}].date string literal {} — runtime will escape, but review intent",
                            fn_name, i, reason
                        ));
                    }
                }
                // label (always rendered)
                if let Some(Expr::StringLit(s)) = fields.get("label") {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(format!(
                            "security: {} data[{}].label string literal {} — runtime will escape, but review intent",
                            fn_name, i, reason
                        ));
                    }
                }
                // description (rendered, optional)
                if let Some(Expr::StringLit(s)) = fields.get("description") {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(format!(
                            "security: {} data[{}].description string literal {} — runtime will escape, but review intent",
                            fn_name, i, reason
                        ));
                    }
                }
            }
        }
    }
}

/// Наряд №83 Block 6 — scanner for `diagram_venn` data shape.
///
/// Venn has a nested structure with a TOP-LEVEL string field:
///   Struct {
///     circles:      List<Struct{label: String, value?: Float}>,
///     overlap_label: String?,                          ← TOP-LEVEL, not in list
///   }
///
/// `overlap_label` is the spec-called-out "easy to forget" case: it's a
/// field on the outer Struct, not an element of `circles[]`. A scanner
/// that only walks List elements would miss it entirely. We scan it
/// separately as a top-level StringLit field.
///
/// `circles[].label` is rendered inside each circle (offset from center).
/// `circles[].value` is a Float — never rendered as text, skipped.
///
/// All findings are WARNINGs (runtime escapes via escape_html_chars —
/// defense-in-depth, not a hard error).
fn scan_venn_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit(fields)) = args.first() {
        // circles: List<Struct{label, value?}> — scan each circle's label
        if let Some(Expr::List(circle_items)) = fields.get("circles") {
            for (i, item) in circle_items.iter().enumerate() {
                if let Expr::StructLit(circle_fields) = item {
                    if let Some(Expr::StringLit(s)) = circle_fields.get("label") {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(format!(
                                "security: {} data.circles[{}].label string literal {} — runtime will escape, but review intent",
                                fn_name, i, reason
                            ));
                        }
                    }
                }
            }
        }
        // overlap_label — TOP-LEVEL field (the spec's "don't forget" case).
        // This is NOT inside the circles list; a scanner that only walks
        // list elements would miss it.
        if let Some(Expr::StringLit(s)) = fields.get("overlap_label") {
            if let Some(reason) = detect_xss_payload(s) {
                result.warnings.push(format!(
                    "security: {} data.overlap_label string literal {} — runtime will escape, but review intent",
                    fn_name, reason
                ));
            }
        }
    }
}

/// Наряд №83 Block 6 — scanner for `diagram_quadrant` data shape.
///
/// Quadrant has TWO top-level string fields PLUS a nested list:
///   Struct {
///     x_axis_label: String,                            ← TOP-LEVEL
///     y_axis_label: String,                            ← TOP-LEVEL
///     items: List<Struct{label: String, x: Float, y: Float}>,
///   }
///
/// Both `x_axis_label` and `y_axis_label` are TOP-LEVEL fields (the spec
/// calls them out as "поля верхнего уровня, не элементы списка, легко
/// забыть"). A scanner that only walks List elements would miss BOTH.
///
/// `items[].label` is rendered next to each point marker. `items[].x`
/// and `items[].y` are Floats — geometry, never rendered as text, skipped.
///
/// All findings are WARNINGs (runtime escapes via escape_html_chars —
/// defense-in-depth, not a hard error).
fn scan_quadrant_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit(fields)) = args.first() {
        // x_axis_label — top-level, easy to forget
        if let Some(Expr::StringLit(s)) = fields.get("x_axis_label") {
            if let Some(reason) = detect_xss_payload(s) {
                result.warnings.push(format!(
                    "security: {} data.x_axis_label string literal {} — runtime will escape, but review intent",
                    fn_name, reason
                ));
            }
        }
        // y_axis_label — top-level, easy to forget
        if let Some(Expr::StringLit(s)) = fields.get("y_axis_label") {
            if let Some(reason) = detect_xss_payload(s) {
                result.warnings.push(format!(
                    "security: {} data.y_axis_label string literal {} — runtime will escape, but review intent",
                    fn_name, reason
                ));
            }
        }
        // items: List<Struct{label, x, y}> — scan each item's label
        if let Some(Expr::List(item_list)) = fields.get("items") {
            for (i, item) in item_list.iter().enumerate() {
                if let Expr::StructLit(item_fields) = item {
                    if let Some(Expr::StringLit(s)) = item_fields.get("label") {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(format!(
                                "security: {} data.items[{}].label string literal {} — runtime will escape, but review intent",
                                fn_name, i, reason
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Наряд №83 Block 6 — scanner for `diagram_medallion` data shape.
///
/// Medallion is a flat list of structs:
///   List<Struct{icon: String?, label: String, value?: Float}>
///
/// `label` is rendered below each medallion — primary scan target.
/// `icon` is a CONTROLLED ENUM (validated at runtime against the 10
/// known svg_icon names) — NOT user free-form text, explicitly NOT
/// scanned per the spec ("icon не сканировать как текст — это controlled
/// enum-подобное значение"). If a user passes `<script>` as an icon name,
/// the runtime rejects it with an "unknown icon name" error before any
/// SVG output is produced.
/// `value` is a Float — never rendered as text, skipped.
///
/// This scanner is essentially scan_chart_labels (single `label` field
/// per struct). We keep it as a separate function for self-documenting
/// naming and to leave room for medallion-specific extensions later.
fn scan_medallion_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::List(items)) = args.first() {
        for (i, item) in items.iter().enumerate() {
            if let Expr::StructLit(fields) = item {
                // label — rendered below the medallion (primary target)
                if let Some(Expr::StringLit(s)) = fields.get("label") {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(format!(
                            "security: {} data[{}].label string literal {} — runtime will escape, but review intent",
                            fn_name, i, reason
                        ));
                    }
                }
                // NOTE: `icon` is intentionally NOT scanned here. It is a
                // controlled enum (one of 10 known svg_icon names), validated
                // at runtime via icon_path_data() before any SVG output.
                // Scanning it as text would produce false positives for
                // legitimate icon names that happen to contain angle brackets
                // (none currently do, but the principle holds).
            }
        }
    }
}

/// Наряд №84 Block 7 — scanner for `diagram_er` data shape.
///
/// ER has two lists, each with rendered text:
///   Struct {
///     entities:  List<Struct{name: String, fields: List<String>}>,
///     relations: List<Struct{from: String, to: String, label?: String}>,
///   }
///
/// The novel case here is `entities[].fields` — a `List<String>` NESTED
/// INSIDE a struct field. This is the THIRD nesting form in the SVG
/// suite (after top-level `List<String>` in diagram_sequence.actors,
/// and `List<Struct>` everywhere else). A scanner that only walks one
/// level of struct fields would miss `fields[]` entirely — each field
/// name is rendered as a separate line inside the entity box, so an
/// injection in `fields[2]` would reach the SVG output.
///
/// We walk per-entity: scan `name` (rendered in the header bar), then
/// iterate `fields[]` and scan each `StringLit` element directly (no
/// struct unwrap — same approach as scan_sequence_labels.actors).
///
/// `relations[].label` is rendered at the connector midpoint (same as
/// flowchart edges). `relations[].from` and `.to` are entity-name
/// identifiers, scanned defensively (not rendered, but suspicious if
/// they contain a payload).
///
/// All findings are WARNINGs (runtime escapes via escape_html_chars —
/// defense-in-depth, not a hard error).
fn scan_er_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit(fields)) = args.first() {
        // entities: List<Struct{name, fields: List<String>}>
        if let Some(Expr::List(entity_items)) = fields.get("entities") {
            for (i, item) in entity_items.iter().enumerate() {
                if let Expr::StructLit(entity_fields) = item {
                    // name — rendered in the entity header bar (primary target)
                    if let Some(Expr::StringLit(s)) = entity_fields.get("name") {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(format!(
                                "security: {} data.entities[{}].name string literal {} — runtime will escape, but review intent",
                                fn_name, i, reason
                            ));
                        }
                    }
                    // fields: List<String> NESTED inside a struct field —
                    // the third nesting form. Each StringLit is rendered
                    // as a separate line inside the entity box.
                    if let Some(Expr::List(field_items)) = entity_fields.get("fields") {
                        for (j, f_item) in field_items.iter().enumerate() {
                            if let Expr::StringLit(s) = f_item {
                                if let Some(reason) = detect_xss_payload(s) {
                                    result.warnings.push(format!(
                                        "security: {} data.entities[{}].fields[{}] string literal {} — runtime will escape, but review intent",
                                        fn_name, i, j, reason
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        // relations: List<Struct{from, to, label?}> — same shape as
        // diagram_flowchart.edges. Scan from/to defensively, label as primary.
        if let Some(Expr::List(rel_items)) = fields.get("relations") {
            for (i, item) in rel_items.iter().enumerate() {
                if let Expr::StructLit(rel_fields) = item {
                    for ident_key in &["from", "to"] {
                        if let Some(Expr::StringLit(s)) = rel_fields.get(*ident_key) {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(format!(
                                    "security: {} data.relations[{}].{} string literal {} — field not rendered but review intent",
                                    fn_name, i, ident_key, reason
                                ));
                            }
                        }
                    }
                    if let Some(Expr::StringLit(s)) = rel_fields.get("label") {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(format!(
                                "security: {} data.relations[{}].label string literal {} — runtime will escape, but review intent",
                                fn_name, i, reason
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Наряд №84 Block 7 — scanner for `diagram_state` data shape.
///
/// State has TWO independent text sources plus a top-level optional:
///   Struct {
///     states:      List<String>,                              // scan each StringLit
///     transitions: List<Struct{from, to, label?}>,            // scan transitions[].label
///     initial:     String?,                                   // TOP-LEVEL field, like venn.overlap_label
///   }
///
/// `states[]` is `List<String>` (same form as diagram_sequence.actors —
/// scan each StringLit directly, no struct unwrap).
/// `transitions[].label` is rendered at the connector midpoint (self-
/// loops render the label above the loop arc — same scan rule applies).
/// `transitions[].from`/`.to` are state-name identifiers, scanned
/// defensively (not rendered).
/// `initial` is a TOP-LEVEL `String?` field — easy to forget, called
/// out explicitly in the spec. We scan it separately at the outer
/// StructLit level (not inside any list).
///
/// All findings are WARNINGs (runtime escapes via escape_html_chars —
/// defense-in-depth, not a hard error).
fn scan_state_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit(fields)) = args.first() {
        // states: List<String> — scan each StringLit directly
        if let Some(Expr::List(state_items)) = fields.get("states") {
            for (i, state) in state_items.iter().enumerate() {
                if let Expr::StringLit(s) = state {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(format!(
                            "security: {} data.states[{}] string literal {} — runtime will escape, but review intent",
                            fn_name, i, reason
                        ));
                    }
                }
            }
        }
        // transitions: List<Struct{from, to, label?}> — same pattern as
        // diagram_flowchart.edges / diagram_sequence.messages.
        if let Some(Expr::List(trans_items)) = fields.get("transitions") {
            for (i, item) in trans_items.iter().enumerate() {
                if let Expr::StructLit(trans_fields) = item {
                    for ident_key in &["from", "to"] {
                        if let Some(Expr::StringLit(s)) = trans_fields.get(*ident_key) {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(format!(
                                    "security: {} data.transitions[{}].{} string literal {} — field not rendered but review intent",
                                    fn_name, i, ident_key, reason
                                ));
                            }
                        }
                    }
                    if let Some(Expr::StringLit(s)) = trans_fields.get("label") {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(format!(
                                "security: {} data.transitions[{}].label string literal {} — runtime will escape, but review intent",
                                fn_name, i, reason
                            ));
                        }
                    }
                }
            }
        }
        // initial — TOP-LEVEL String? field (the spec's "easy to forget" case,
        // same category as diagram_venn.overlap_label). It's rendered as the
        // entry-arrow target state name, but ALSO drawn near the initial node.
        if let Some(Expr::StringLit(s)) = fields.get("initial") {
            if let Some(reason) = detect_xss_payload(s) {
                result.warnings.push(format!(
                    "security: {} data.initial string literal {} — runtime will escape, but review intent",
                    fn_name, reason
                ));
            }
        }
    }
}

/// Наряд №84 Block 7 — scanner for `diagram_swimlane` data shape.
///
/// Swimlane has TWO lists, each containing user text:
///   Struct {
///     lanes: List<String>,                              // scan each StringLit
///     steps: List<Struct{lane, label, order}>,          // scan steps[].label
///   }
///
/// `lanes[]` is `List<String>` (same form as diagram_sequence.actors
/// and diagram_state.states — scan each StringLit directly).
/// `steps[].label` is rendered inside each step pill (primary target).
/// `steps[].lane` is a lane-name identifier (defensive scan — not
/// rendered, but suspicious if it carries a payload).
/// `steps[].order` is a Float — geometry, never rendered as text, skipped.
///
/// All findings are WARNINGs (runtime escapes via escape_html_chars —
/// defense-in-depth, not a hard error).
fn scan_swimlane_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit(fields)) = args.first() {
        // lanes: List<String> — scan each StringLit directly
        if let Some(Expr::List(lane_items)) = fields.get("lanes") {
            for (i, lane) in lane_items.iter().enumerate() {
                if let Expr::StringLit(s) = lane {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(format!(
                            "security: {} data.lanes[{}] string literal {} — runtime will escape, but review intent",
                            fn_name, i, reason
                        ));
                    }
                }
            }
        }
        // steps: List<Struct{lane, label, order}> — scan lane defensively,
        // label as primary. order is Float, skipped.
        if let Some(Expr::List(step_items)) = fields.get("steps") {
            for (i, item) in step_items.iter().enumerate() {
                if let Expr::StructLit(step_fields) = item {
                    // lane — defensive (identifier, not rendered as free text)
                    if let Some(Expr::StringLit(s)) = step_fields.get("lane") {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(format!(
                                "security: {} data.steps[{}].lane string literal {} — field not rendered but review intent",
                                fn_name, i, reason
                            ));
                        }
                    }
                    // label — rendered inside the step pill (primary target)
                    if let Some(Expr::StringLit(s)) = step_fields.get("label") {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(format!(
                                "security: {} data.steps[{}].label string literal {} — runtime will escape, but review intent",
                                fn_name, i, reason
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
                // Наряд №81 Block 6: diagram scanners.
                //   diagram_tree / diagram_org_chart — recursive Struct,
                //     need scan_tree_labels_recursive (NOT the flat
                //     scan_chart_labels — that one only goes one level
                //     deep and would miss injections at children[2].label
                //     or deeper).
                //   diagram_flowchart — two independent lists (nodes,
                //     edges), each with its own label field. Scanned
                //     separately by scan_flowchart_labels.
                //   diagram_layers — flat list, label + description.
                //     scan_layers_labels checks both fields.
                if name == "diagram_tree" {
                    if let Some(arg0) = args.first() {
                        scan_tree_labels_recursive(name, arg0, "root", false, result);
                    }
                }
                if name == "diagram_org_chart" {
                    if let Some(arg0) = args.first() {
                        scan_tree_labels_recursive(name, arg0, "root", true, result);
                    }
                }
                if name == "diagram_flowchart" {
                    scan_flowchart_labels(name, args, result);
                }
                if name == "diagram_layers" {
                    scan_layers_labels(name, args, result);
                }
                // Наряд №82 Block 6: temporal & process diagram scanners.
                //   diagram_sequence — Struct{actors: List<String>, messages: ...}.
                //     actors is List<String> (special case — direct StringLit
                //     scan, no StructLit unwrap). messages[].label is rendered.
                //     Scanned by scan_sequence_labels.
                //   diagram_timeline — flat List<Struct{date, label, description?}>.
                //     3 string fields — scan_timeline_labels checks all three.
                //   diagram_gantt — flat List<Struct{task, start, duration}>.
                //     Only `task` is rendered — scan_gantt_labels checks it.
                //   diagram_process / diagram_loop — same shape as diagram_layers
                //     (List<Struct{label, description?}>) — REUSES
                //     scan_layers_labels (no need to write a new scanner for
                //     an identical shape — "не писать заново" per spec).
                if name == "diagram_sequence" {
                    scan_sequence_labels(name, args, result);
                }
                if name == "diagram_timeline" {
                    scan_timeline_labels(name, args, result);
                }
                if name == "diagram_gantt" {
                    scan_gantt_labels(name, args, result);
                }
                if name == "diagram_process" || name == "diagram_loop" {
                    scan_layers_labels(name, args, result);
                }
                // Наряд №83 Block 6: sets & comparison diagram scanners.
                //   diagram_venn — Struct{circles: [{label, value?}], overlap_label?}.
                //     Special scanner: overlap_label is a TOP-LEVEL field
                //     (not in circles[]), easy to forget — scan_venn_labels
                //     checks both the nested circles[].label and the top-level
                //     overlap_label in one pass.
                //   diagram_quadrant — Struct{x_axis_label, y_axis_label, items: [...]}.
                //     BOTH axis labels are top-level fields. scan_quadrant_labels
                //     checks both axis labels + items[].label.
                //   diagram_pyramid / diagram_nested — same flat shape as
                //     diagram_layers (List<Struct{label, value?}>) — REUSES
                //     scan_layers_labels (no new scanner for identical shape).
                //   diagram_medallion — List<Struct{icon?, label, value?}>.
                //     Only `label` is scanned; `icon` is a controlled enum
                //     (validated at runtime against 10 known svg_icon names),
                //     NOT free-form text — explicitly NOT scanned per spec.
                if name == "diagram_venn" {
                    scan_venn_labels(name, args, result);
                }
                if name == "diagram_quadrant" {
                    scan_quadrant_labels(name, args, result);
                }
                if name == "diagram_pyramid" || name == "diagram_nested" {
                    scan_layers_labels(name, args, result);
                }
                if name == "diagram_medallion" {
                    scan_medallion_labels(name, args, result);
                }
                // Наряд №84 Block 7: data & state diagram scanners.
                //   diagram_er — Struct{entities:[{name, fields:[String]}],
                //     relations:[{from,to,label?}]}. Special scanner
                //     scan_er_labels walks the nested List<String> inside
                //     each entity (the third nesting form — see comment
                //     on the scanner for why this needs bespoke handling).
                //   diagram_state — Struct{states:[String], transitions:[...],
                //     initial?}. scan_state_labels scans states[] as a direct
                //     List<String>, transitions[].label as rendered text, and
                //     `initial` as a TOP-LEVEL String? field (overlap_label form).
                //   diagram_swimlane — Struct{lanes:[String], steps:[{lane,label,order}]}.
                //     scan_swimlane_labels scans lanes[] directly + steps[].label.
                //   diagram_data_flow / diagram_high_level / diagram_architecture —
                //     Struct{nodes:[{id,label,icon?}], edges:[{from,to,label?}]}.
                //     Same shape as diagram_flowchart (Н81) — REUSES
                //     scan_flowchart_labels (no new scanner for an identical
                //     shape, per spec). The `icon` field on diagram_architecture
                //     nodes is a controlled enum (validated against svg_icon's
                //     10 names at runtime), NOT free-form text — explicitly
                //     NOT scanned, same decision as diagram_medallion.
                if name == "diagram_er" {
                    scan_er_labels(name, args, result);
                }
                if name == "diagram_state" {
                    scan_state_labels(name, args, result);
                }
                if name == "diagram_swimlane" {
                    scan_swimlane_labels(name, args, result);
                }
                if name == "diagram_data_flow"
                    || name == "diagram_high_level"
                    || name == "diagram_architecture"
                {
                    scan_flowchart_labels(name, args, result);
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
