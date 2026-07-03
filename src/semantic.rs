// ── Semantic analysis for METALOGOS ──────────────────────────────
// Validates declarations without execution. Reports errors and warnings.
// Phase 6+: Enforces opaque type constraints (Html, Query, Secret, etc.)
// Наряд №19: opaque type enforcement (print/to_string/concat restrictions)
// Наряд №20: variable scope tracking + arity checking

use crate::ast::*;
use std::collections::{HashMap, HashSet};

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
                lines.push(format!("1 error:"));
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
                lines.push(format!("1 warning:"));
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
const VALID_MIDDLEWARE: &[&str] = &[
    "session",
    "csrf",
    "security_headers",
    "rate_limit",
    "cors",
];

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
    let mut builtin_names: HashSet<String> = HashSet::new();
    let mut role_names: HashSet<String> = HashSet::new();

    // Known builtins (Наряд №18: full list synced with builtins.rs)
    for b in &[
        // String operations
        "upper", "lower", "len", "str", "print", "contains", "float",
        "to_string", "get", "push",
        // Environment
        "env",
        // String operations Phase 5.3
        "index_of", "substring", "char_at", "starts_with", "ends_with", "to_float",
        // Fluid
        "confidence",
        // Phase 6 builtins (web, crypto, auth, etc.)
        "respond", "respond_html", "form_data", "json_body", "query_param",
        "render", "escape_html",
        "query", "db_execute",
        "hash_password", "verify_password", "encrypt", "decrypt", "generate_key",
        "authenticate", "session_login", "session_logout",
        "send_message", "answer_callback_query", "edit_message_text", "require",
        "http_post", "http_get",
        // Public string/math ops
        "trim", "replace", "split", "join", "length", "to_int", "reverse",
        // LLM
        "call_llm",
        // KV store
        "kv_set", "kv_get", "kv_delete", "kv_exists", "kv_list",
        // Memory
        "mem_set", "mem_get", "mem_delete",
        // File I/O
        "read_file", "write_file", "append_file", "delete_file", "file_exists", "list_dir",
        // AI providers
        "call_claude",
        // LLM usage
        "llm_usage",
        // JSON
        "escape_json", "parse_json", "json_encode", "json_get", "has_field",
        // Time
        "now", "format_date",
        // Session
        "session_set", "session_get", "session_clear",
        // HTTP extras
        "http_post_multipart",
        // Media
        "whisper_transcribe", "tts_send",
        // Encoding
        "base64_encode", "base64_decode",
        // System
        "exec", "escape_js",
        // Misc
        "dict_get", "dict_set", "dict_keys", "dict_values", "dict_has", "type_of",
        // Memory (recall)
        "recall",
        // Phase 4.4 self-hosting
        "stdin", "split_tokens", "if_eq", "newline", "is_string_token",
        // Format & list utilities
        "format", "first", "last", "make_list",
        // Time alias
        "time",
        // Наряд №24: integrations
        "git_push", "web_search",
        // Server request helpers
        "request_body",
        // Problem B (reverse-iteration): list aggregation + helpers
        "zip", "sort_by", "filter", "reduce", "map",
        "extract_param", "estimate_tokens", "db_insert",
        "matches_any", "read_file_tokens",
        // OpenHuman-inspired (v0.8.3): Scheduling
        "cron_add", "cron_list", "cron_remove", "cron_run",
        // OpenHuman-inspired (v0.8.3): Approval Gate
        "ask_approval",
        // OpenHuman-inspired (v0.8.3): Goals & Todos
        "goal_set", "goal_get", "goal_complete", "goals_list", "goals_add", "goals_reflect",
        "todo_add", "todo_update", "todo_list",
        // OpenHuman-inspired (v0.8.3): Entity Extraction
        "extract_entities",
        // OpenHuman-inspired (v0.8.3): Memory Scoring
        "memory_score",
        // OpenHuman-inspired (v0.8.3): Token Compression
        "compress_html",
        // OpenHuman-inspired (v0.8.3): Personalization
        "learn_preference", "get_profile",
    ] {
        builtin_names.insert(b.to_string());
    }

    // First pass: collect all declarations (names)
    for decl in declarations {
        match decl {
            Declaration::EntityType(e) => {
                if !entity_types.insert(e.name.clone()) {
                    result.errors.push(format!("duplicate entity type: {}", e.name));
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
            }
            Declaration::LearnablePattern(lp) => {
                if !learnable_names.insert(lp.name.clone()) {
                    result.errors.push(format!("duplicate learnable pattern: {}", lp.name));
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
                let known_primitives = ["String", "Float", "Bool", "Html", "Query", "Secret", "Encrypted", "Hash", "Session"];
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
                let has_post = srv.routes.iter().any(|r| r.method == "POST" || r.method == "PUT" || r.method == "DELETE");
                if has_post && !srv.middleware.contains(&"csrf".to_string()) {
                    result.warnings.push(
                        "mlogserver: has mutating routes but no 'csrf' middleware — recommend adding it".to_string()
                    );
                }
            }
            _ => {}
        }
    }

    // ── Наряд №19: Opaque type constraint checking ────────────────────
    // Build a type map: entity name -> declared type, pattern name -> param types + return type
    let mut entity_type_map: HashMap<String, String> = HashMap::new();
    let mut pattern_arity: HashMap<String, usize> = HashMap::new();
    for decl in declarations {
        match decl {
            Declaration::EntitySimple(e) => {
                entity_type_map.insert(e.name.clone(), e.type_name.clone());
            }
            Declaration::EntityRecord(e) => {
                entity_type_map.insert(e.name.clone(), e.type_name.clone());
            }
            Declaration::Pattern(p) => {
                pattern_arity.insert(p.name.clone(), p.params.len());
            }
            Declaration::LearnablePattern(lp) => {
                pattern_arity.insert(lp.name.clone(), lp.params.len());
            }
            Declaration::Template(t) => {
                pattern_arity.insert(t.name.clone(), t.params.len());
            }
            _ => {}
        }
    }
    // Known builtins with their arity (for Наряд №20 checking)
    let builtin_arity: HashMap<&str, usize> = [
        ("upper", 1), ("lower", 1), ("len", 1), ("str", 1), ("print", 1),
        ("contains", 2), ("float", 1), ("to_string", 1), ("get", 2), ("push", 2),
        ("env", 1), ("index_of", 2), ("substring", 3), ("char_at", 2),
        ("starts_with", 2), ("ends_with", 2), ("to_float", 1), ("confidence", 1),
        ("trim", 1), ("replace", 3), ("split", 2), ("join", 2), ("length", 1),
        ("to_int", 1), ("reverse", 1), ("call_llm", 1), ("call_claude", 1),
        ("kv_set", 2), ("kv_get", 1), ("kv_delete", 1), ("kv_exists", 1), ("kv_list", 0),
        ("mem_set", 2), ("mem_get", 1), ("mem_delete", 1), ("recall", 1),
        ("read_file", 1), ("write_file", 2), ("append_file", 2), ("delete_file", 1),
        ("file_exists", 1), ("list_dir", 1), ("llm_usage", 0),
        ("escape_json", 1), ("parse_json", 1), ("json_encode", 1),
        ("json_get", 2), ("has_field", 2), ("now", 0), ("format_date", 0),
        ("session_set", 2), ("session_get", 1), ("session_clear", 0),
        ("hash_password", 1), ("verify_password", 2), ("encrypt", 2),
        ("decrypt", 2), ("generate_key", 0), ("authenticate", 2),
        ("session_login", 1), ("session_logout", 0),
        ("send_message", 0), ("answer_callback_query", 0), ("edit_message_text", 0), ("require", 1),
        ("http_post", 2), ("http_get", 1), ("http_post_multipart", 3),
        ("whisper_transcribe", 0), ("tts_send", 0),
        ("base64_encode", 1), ("base64_decode", 1),
        ("exec", 1), ("escape_js", 1), ("dict_get", 2), ("dict_set", 3), ("dict_keys", 1), ("dict_values", 1), ("dict_has", 2), ("type_of", 1),
        ("respond", 2), ("respond_html", 1), ("form_data", 0),
        ("json_body", 0), ("query_param", 1), ("render", 2), ("escape_html", 1),
        ("query", 2), ("db_execute", 1),
        ("stdin", 0), ("split_tokens", 1), ("if_eq", 3), ("newline", 0),
        ("is_string_token", 1),
        // List & format utilities (variadic: 0 = skip strict arity check)
        ("first", 1), ("last", 1), ("make_list", 0), ("format", 0),
        // Time alias
        ("time", 0),
        // Наряд №24: integrations
        ("git_push", 0), ("web_search", 1),
        // Server request helpers
        ("request_body", 0),
        // Problem B (reverse-iteration): list aggregation + helpers
        ("zip", 0), ("sort_by", 0), ("filter", 0), ("reduce", 0),
        ("map", 0),
        ("extract_param", 0), ("estimate_tokens", 0),
        ("db_insert", 0),
        ("matches_any", 0), ("read_file_tokens", 0),
        // OpenHuman-inspired (v0.8.4): Scheduling
        ("cron_add", 2), ("cron_list", 0), ("cron_remove", 1), ("cron_run", 1),
        // OpenHuman-inspired (v0.8.4): Approval Gate
        ("ask_approval", 2),
        // OpenHuman-inspired (v0.8.4): Goals & Todos
        ("goal_set", 0), ("goal_get", 0), ("goal_complete", 0), ("goals_list", 0),
        ("goals_add", 1), ("goals_reflect", 0),
        ("todo_add", 0), ("todo_update", 2), ("todo_list", 0),
        // OpenHuman-inspired (v0.8.4): Entity Extraction
        ("extract_entities", 1),
        // OpenHuman-inspired (v0.8.4): Memory Scoring
        ("memory_score", 1),
        // OpenHuman-inspired (v0.8.4): Token Compression
        ("compress_html", 1),
        // OpenHuman-inspired (v0.8.4): Personalization
        ("learn_preference", 3), ("get_profile", 0),
    ].iter().cloned().collect();

    // Functions that must NOT receive opaque types as arguments.
    // Map: function_name -> set of param indices that reject opaque types.
    // "print" rejects opaque on param 0; "to_string" rejects opaque on param 0.
    const OPAQUE_RESTRICTED: &[(&str, usize)] = &[
        ("print", 0),
        ("to_string", 0),
        ("lower", 0),
        ("upper", 0),
        ("trim", 0),
        ("replace", 0),
        ("split", 0),
        ("contains", 0),
        ("contains", 1),
        ("starts_with", 0),
        ("ends_with", 0),
        ("index_of", 0),
        ("substring", 0),
    ];

    // Walk all statement bodies in patterns, tools, hooks, routes, etc.
    for decl in declarations {
        let stmts: Option<&[Statement]> = match decl {
            Declaration::Pattern(p) => Some(&p.body),
            Declaration::Tool(_) => {
                // Tool methods are checked individually below (line 470-483)
                // Returning None here; the Tool branch hits `continue` before using stmts.
                None
            }
            Declaration::Hook(h) => Some(&h.body),
            Declaration::MlogServer(srv) => {
                let mut all = Vec::new();
                for route in &srv.routes {
                    all.extend(route.body.iter().cloned());
                }
                if all.is_empty() { None } else {
                    // We need a slice; use a trick with LEAK detection avoided
                    // by just checking in a loop below
                    None
                }
            }
            _ => None,
        };

        if let Declaration::MlogServer(srv) = decl {
            for route in &srv.routes {
                let mut local_scope: HashSet<String> = HashSet::new();
                // Route body has no params, but let bindings create locals
                collect_let_bindings(&route.body, &mut local_scope);
                check_opaque_in_stmts(&route.body, &entity_type_map, &builtin_arity,
                    &local_scope, &entity_names, &pattern_arity, OPAQUE_RESTRICTED, &mut result);
                // Наряд №20: variable + arity checks in route bodies
                check_variables_in_stmts(&route.body, &local_scope, &entity_names,
                    &builtin_names, &builtin_arity, &pattern_arity, &mut result);
            }
        }

        if let Some(body) = stmts {
            let mut local_scope: HashSet<String> = HashSet::new();

            // Populate scope with pattern/tool method parameters
            match decl {
                Declaration::Pattern(p) => {
                    for param in &p.params {
                        local_scope.insert(param.name.clone());
                    }
                }
                Declaration::Tool(t) => {
                    // Each method has its own scope — check each separately
                    for method in &t.methods {
                        let mut method_scope: HashSet<String> = HashSet::new();
                        for param in &method.params {
                            method_scope.insert(param.name.clone());
                        }
                        collect_let_bindings(&method.body, &mut method_scope);
                        check_opaque_in_stmts(&method.body, &entity_type_map, &builtin_arity,
                            &method_scope, &entity_names, &pattern_arity, OPAQUE_RESTRICTED, &mut result);
                        check_variables_in_stmts(&method.body, &method_scope, &entity_names,
                            &builtin_names, &builtin_arity, &pattern_arity, &mut result);
                    }
                    continue;
                }
                Declaration::Hook(h) => {
                    // Hook has implicit vars: pattern_name, args, result (after), confidence (after)
                    local_scope.insert("pattern_name".to_string());
                    local_scope.insert("args".to_string());
                }
                _ => {}
            }

            collect_let_bindings(body, &mut local_scope);
            check_opaque_in_stmts(body, &entity_type_map, &builtin_arity,
                &local_scope, &entity_names, &pattern_arity, OPAQUE_RESTRICTED, &mut result);
            // Наряд №20: variable + arity checks
            check_variables_in_stmts(body, &local_scope, &entity_names,
                &builtin_names, &builtin_arity, &pattern_arity, &mut result);
        }
    }

    result
}

/// Наряд №20: collect let-binding names from a statement list into the scope.
fn collect_let_bindings(stmts: &[Statement], scope: &mut HashSet<String>) {
    for stmt in stmts {
        match stmt {
            Statement::LetBinding { name, .. } => {
                scope.insert(name.clone());
            }
            Statement::Each { variable, body, .. } => {
                scope.insert(variable.clone());
                collect_let_bindings(body, scope);
            }
            Statement::EachWithIndex { index_var, item_var, body, .. } => {
                scope.insert(index_var.clone());
                scope.insert(item_var.clone());
                collect_let_bindings(body, scope);
            }
            Statement::While { body, .. } => {
                collect_let_bindings(body, scope);
            }
            Statement::IfElseBlock { then_body, else_ifs, else_body, .. } => {
                collect_let_bindings(then_body, scope);
                for (_, ei_body) in else_ifs {
                    collect_let_bindings(ei_body, scope);
                }
                if let Some(eb) = else_body {
                    collect_let_bindings(eb, scope);
                }
            }
            Statement::IfThen(_, body) => {
                collect_let_bindings(body, scope);
            }
            Statement::Match { arms, else_body, .. } => {
                for arm in arms {
                    match arm {
                        MatchArm::Exact(_, body) => collect_let_bindings(body, scope),
                        MatchArm::StartsWith(_, body) => collect_let_bindings(body, scope),
                        MatchArm::Contains(_, body) => collect_let_bindings(body, scope),
                        MatchArm::Compare(_, _, body) => collect_let_bindings(body, scope),
                    }
                }
                if let Some(eb) = else_body {
                    collect_let_bindings(eb, scope);
                }
            }
            _ => {}
        }
    }
}

/// Наряд №19: Check opaque type constraint violations in statements.
/// Walks expressions and flags:
///   - print(secret_entity), to_string(secret_entity) — leaking opaque types
///   - BinaryOp::Add where one side is an opaque-typed entity — constructing
///     Html/Query/Secret from string concatenation (XSS/injection vector)
fn check_opaque_in_stmts(
    stmts: &[Statement],
    entity_type_map: &HashMap<String, String>,
    _builtin_arity: &HashMap<&str, usize>,
    scope: &HashSet<String>,
    entity_names: &HashSet<String>,
    _pattern_arity: &HashMap<String, usize>,
    opaque_restricted: &[(&str, usize)],
    result: &mut AnalysisResult,
) {
    for stmt in stmts {
        check_opaque_in_stmt(stmt, entity_type_map, scope, entity_names, opaque_restricted, result);
    }
}

fn check_opaque_in_stmt(
    stmt: &Statement,
    entity_type_map: &HashMap<String, String>,
    scope: &HashSet<String>,
    entity_names: &HashSet<String>,
    opaque_restricted: &[(&str, usize)],
    result: &mut AnalysisResult,
) {
    match stmt {
        Statement::LetBinding { value, .. } => {
            check_opaque_in_expr(value, entity_type_map, scope, entity_names, opaque_restricted, result);
        }
        Statement::Assign { value, .. } => {
            check_opaque_in_expr(value, entity_type_map, scope, entity_names, opaque_restricted, result);
        }
        Statement::Return(expr) => {
            check_opaque_in_expr(expr, entity_type_map, scope, entity_names, opaque_restricted, result);
        }
        Statement::ExprStmt(expr) => {
            check_opaque_in_expr(expr, entity_type_map, scope, entity_names, opaque_restricted, result);
        }
        Statement::Each { iterable, body, .. } => {
            check_opaque_in_expr(iterable, entity_type_map, scope, entity_names, opaque_restricted, result);
            check_opaque_in_stmts(body, entity_type_map, _builtin_arity_placeholder(), scope, entity_names, _pattern_arity_placeholder(), opaque_restricted, result);
        }
        Statement::EachWithIndex { iterable, body, .. } => {
            check_opaque_in_expr(iterable, entity_type_map, scope, entity_names, opaque_restricted, result);
            check_opaque_in_stmts(body, entity_type_map, _builtin_arity_placeholder(), scope, entity_names, _pattern_arity_placeholder(), opaque_restricted, result);
        }
        Statement::While { condition, body } => {
            check_opaque_in_expr(condition, entity_type_map, scope, entity_names, opaque_restricted, result);
            check_opaque_in_stmts(body, entity_type_map, _builtin_arity_placeholder(), scope, entity_names, _pattern_arity_placeholder(), opaque_restricted, result);
        }
        Statement::IfElseBlock { condition, then_body, else_ifs, else_body } => {
            check_opaque_in_expr(condition, entity_type_map, scope, entity_names, opaque_restricted, result);
            check_opaque_in_stmts(then_body, entity_type_map, _builtin_arity_placeholder(), scope, entity_names, _pattern_arity_placeholder(), opaque_restricted, result);
            for (ei_cond, ei_body) in else_ifs {
                check_opaque_in_expr(ei_cond, entity_type_map, scope, entity_names, opaque_restricted, result);
                check_opaque_in_stmts(ei_body, entity_type_map, _builtin_arity_placeholder(), scope, entity_names, _pattern_arity_placeholder(), opaque_restricted, result);
            }
            if let Some(eb) = else_body {
                check_opaque_in_stmts(eb, entity_type_map, _builtin_arity_placeholder(), scope, entity_names, _pattern_arity_placeholder(), opaque_restricted, result);
            }
        }
        Statement::IfThen(cond, body) => {
            check_opaque_in_expr(cond, entity_type_map, scope, entity_names, opaque_restricted, result);
            check_opaque_in_stmts(body, entity_type_map, _builtin_arity_placeholder(), scope, entity_names, _pattern_arity_placeholder(), opaque_restricted, result);
        }
        Statement::Match { scrutinee, arms, else_body } => {
            check_opaque_in_expr(scrutinee, entity_type_map, scope, entity_names, opaque_restricted, result);
            for arm in arms {
                match arm {
                    MatchArm::Exact(_, body) |
                    MatchArm::StartsWith(_, body) |
                    MatchArm::Contains(_, body) => {
                        check_opaque_in_stmts(body, entity_type_map, _builtin_arity_placeholder(), scope, entity_names, _pattern_arity_placeholder(), opaque_restricted, result);
                    }
                    MatchArm::Compare(_, threshold, body) => {
                        check_opaque_in_expr(threshold, entity_type_map, scope, entity_names, opaque_restricted, result);
                        check_opaque_in_stmts(body, entity_type_map, _builtin_arity_placeholder(), scope, entity_names, _pattern_arity_placeholder(), opaque_restricted, result);
                    }
                }
            }
            if let Some(eb) = else_body {
                check_opaque_in_stmts(eb, entity_type_map, _builtin_arity_placeholder(), scope, entity_names, _pattern_arity_placeholder(), opaque_restricted, result);
            }
        }
        Statement::Break | Statement::Continue => {}
    }
}

/// Placeholder to satisfy the function signature without threading extra params.
fn _builtin_arity_placeholder() -> &'static HashMap<&'static str, usize> {
    static MAP: std::sync::OnceLock<HashMap<&'static str, usize>> = std::sync::OnceLock::new();
    MAP.get_or_init(HashMap::new)
}
fn _pattern_arity_placeholder() -> &'static HashMap<String, usize> {
    static MAP: std::sync::OnceLock<HashMap<String, usize>> = std::sync::OnceLock::new();
    MAP.get_or_init(HashMap::new)
}

/// Check opaque type violations in an expression.
fn check_opaque_in_expr(
    expr: &Expr,
    entity_type_map: &HashMap<String, String>,
    scope: &HashSet<String>,
    entity_names: &HashSet<String>,
    opaque_restricted: &[(&str, usize)],
    result: &mut AnalysisResult,
) {
    match expr {
        Expr::StringLit(_) | Expr::FloatLit(_) | Expr::BoolLit(_) => {}
        Expr::Ident(name) => {
            // Check if this ident refers to an opaque-typed entity
            if let Some(typ) = entity_type_map.get(name) {
                if is_opaque_type(typ) {
                    // We don't error on mere reference — only on specific operations.
                    // The violation is caught at the call site (print, to_string, +).
                }
            }
        }
        Expr::FieldAccess(base, _field) => {
            check_opaque_in_expr(base, entity_type_map, scope, entity_names, opaque_restricted, result);
        }
        Expr::FnCall(name, args) => {
            // Наряд №19: Check if this function rejects opaque types on specific params
            for &(fn_name, param_idx) in opaque_restricted {
                if name == fn_name {
                    if let Some(arg) = args.get(param_idx) {
                        if expr_refers_to_opaque(arg, entity_type_map, scope, entity_names) {
                            result.errors.push(format!(
                                "opaque type constraint: '{}'() does not accept opaque-typed argument (Secret, Hash, Encrypted, etc. are not printable)",
                                name
                            ));
                        }
                    }
                }
            }
            for arg in args {
                check_opaque_in_expr(arg, entity_type_map, scope, entity_names, opaque_restricted, result);
            }
        }
        Expr::QualifiedCall { function, args, .. } => {
            // Same opaque restriction check for the function part
            for &(fn_name, param_idx) in opaque_restricted {
                if function == fn_name {
                    if let Some(arg) = args.get(param_idx) {
                        if expr_refers_to_opaque(arg, entity_type_map, scope, entity_names) {
                            result.errors.push(format!(
                                "opaque type constraint: '{}.{}'() does not accept opaque-typed argument",
                                "module", function
                            ));
                        }
                    }
                }
            }
            for arg in args {
                check_opaque_in_expr(arg, entity_type_map, scope, entity_names, opaque_restricted, result);
            }
        }
        Expr::BinaryOp(left, op, right) => {
            // Наряд №19: String concatenation (+) with opaque types is forbidden
            if matches!(op, BinOp::Add) {
                if expr_refers_to_opaque(left, entity_type_map, scope, entity_names) {
                    result.errors.push(
                        "opaque type constraint: cannot concatenate (+) opaque-typed value with another value".to_string()
                    );
                }
                if expr_refers_to_opaque(right, entity_type_map, scope, entity_names) {
                    result.errors.push(
                        "opaque type constraint: cannot concatenate (+) value with an opaque-typed value".to_string()
                    );
                }
            }
            check_opaque_in_expr(left, entity_type_map, scope, entity_names, opaque_restricted, result);
            check_opaque_in_expr(right, entity_type_map, scope, entity_names, opaque_restricted, result);
        }
        Expr::IfElse(cond, then_expr, else_expr) => {
            check_opaque_in_expr(cond, entity_type_map, scope, entity_names, opaque_restricted, result);
            check_opaque_in_expr(then_expr, entity_type_map, scope, entity_names, opaque_restricted, result);
            check_opaque_in_expr(else_expr, entity_type_map, scope, entity_names, opaque_restricted, result);
        }
        Expr::List(items) => {
            for item in items {
                check_opaque_in_expr(item, entity_type_map, scope, entity_names, opaque_restricted, result);
            }
        }
        Expr::IndexAccess(base, index) => {
            check_opaque_in_expr(base, entity_type_map, scope, entity_names, opaque_restricted, result);
            check_opaque_in_expr(index, entity_type_map, scope, entity_names, opaque_restricted, result);
        }
        Expr::StructLit(fields) => {
            for (_, val) in fields {
                check_opaque_in_expr(val, entity_type_map, scope, entity_names, opaque_restricted, result);
            }
        }
        Expr::BlockIfElse { condition, then_body, else_ifs, else_body } => {
            check_opaque_in_expr(condition, entity_type_map, scope, entity_names, opaque_restricted, result);
            for stmt in then_body {
                check_opaque_in_stmt(stmt, entity_type_map, scope, entity_names, opaque_restricted, result);
            }
            for (ei_cond, ei_body) in else_ifs {
                check_opaque_in_expr(ei_cond, entity_type_map, scope, entity_names, opaque_restricted, result);
                for stmt in ei_body {
                    check_opaque_in_stmt(stmt, entity_type_map, scope, entity_names, opaque_restricted, result);
                }
            }
            if let Some(eb) = else_body {
                for stmt in eb {
                    check_opaque_in_stmt(stmt, entity_type_map, scope, entity_names, opaque_restricted, result);
                }
            }
        }
        Expr::Try(inner) => {
            check_opaque_in_expr(inner, entity_type_map, scope, entity_names, opaque_restricted, result);
        }
    }
}

/// Check whether an expression refers to an opaque-typed entity/variable.
fn expr_refers_to_opaque(
    expr: &Expr,
    entity_type_map: &HashMap<String, String>,
    scope: &HashSet<String>,
    entity_names: &HashSet<String>,
) -> bool {
    match expr {
        Expr::Ident(name) => {
            // Check if it's a known entity with opaque type
            if entity_names.contains(name) {
                if let Some(typ) = entity_type_map.get(name) {
                    return is_opaque_type(typ);
                }
            }
            // Check if it's a let-bound variable with opaque type
            // (We can't fully track let-binding types yet, so we rely on entity types)
            false
        }
        Expr::FieldAccess(base, _field) => {
            // If base is an opaque-typed entity, field access is OK (e.g., session.role)
            false
        }
        _ => false,
    }
}

/// Наряд №20: Check variable references and call arity in statements.
fn check_variables_in_stmts(
    stmts: &[Statement],
    scope: &HashSet<String>,
    entity_names: &HashSet<String>,
    builtin_names: &HashSet<String>,
    builtin_arity: &HashMap<&str, usize>,
    pattern_arity: &HashMap<String, usize>,
    result: &mut AnalysisResult,
) {
    for stmt in stmts {
        check_variables_in_stmt(stmt, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
    }
}

fn check_variables_in_stmt(
    stmt: &Statement,
    scope: &HashSet<String>,
    entity_names: &HashSet<String>,
    builtin_names: &HashSet<String>,
    builtin_arity: &HashMap<&str, usize>,
    pattern_arity: &HashMap<String, usize>,
    result: &mut AnalysisResult,
) {
    match stmt {
        Statement::LetBinding { value, .. } => {
            check_variables_in_expr(value, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
        }
        Statement::Assign { name, value } => {
            // Check that the assigned-to variable exists in scope or is a global
            if !scope.contains(name) && !entity_names.contains(name) {
                result.errors.push(format!(
                    "assignment to undefined variable '{}'",
                    name
                ));
            }
            check_variables_in_expr(value, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
        }
        Statement::Return(expr) => {
            check_variables_in_expr(expr, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
        }
        Statement::ExprStmt(expr) => {
            check_variables_in_expr(expr, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
        }
        Statement::Each { iterable, body, variable } => {
            check_variables_in_expr(iterable, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
            let mut inner_scope = scope.clone();
            inner_scope.insert(variable.clone());
            collect_let_bindings(body, &mut inner_scope);
            check_variables_in_stmts(body, &inner_scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
        }
        Statement::EachWithIndex { iterable, body, index_var, item_var } => {
            check_variables_in_expr(iterable, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
            let mut inner_scope = scope.clone();
            inner_scope.insert(index_var.clone());
            inner_scope.insert(item_var.clone());
            collect_let_bindings(body, &mut inner_scope);
            check_variables_in_stmts(body, &inner_scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
        }
        Statement::While { condition, body } => {
            check_variables_in_expr(condition, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
            let mut inner_scope = scope.clone();
            collect_let_bindings(body, &mut inner_scope);
            check_variables_in_stmts(body, &inner_scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
        }
        Statement::IfElseBlock { condition, then_body, else_ifs, else_body } => {
            check_variables_in_expr(condition, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
            let mut inner_scope = scope.clone();
            collect_let_bindings(then_body, &mut inner_scope);
            check_variables_in_stmts(then_body, &inner_scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
            for (ei_cond, ei_body) in else_ifs {
                check_variables_in_expr(ei_cond, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
                let mut ei_scope = scope.clone();
                collect_let_bindings(ei_body, &mut ei_scope);
                check_variables_in_stmts(ei_body, &ei_scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
            }
            if let Some(eb) = else_body {
                let mut eb_scope = scope.clone();
                collect_let_bindings(eb, &mut eb_scope);
                check_variables_in_stmts(eb, &eb_scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
            }
        }
        Statement::IfThen(cond, body) => {
            check_variables_in_expr(cond, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
            let mut inner_scope = scope.clone();
            collect_let_bindings(body, &mut inner_scope);
            check_variables_in_stmts(body, &inner_scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
        }
        Statement::Match { scrutinee, arms, else_body } => {
            check_variables_in_expr(scrutinee, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
            for arm in arms {
                match arm {
                    MatchArm::Exact(_, body) |
                    MatchArm::StartsWith(_, body) |
                    MatchArm::Contains(_, body) => {
                        let mut arm_scope = scope.clone();
                        collect_let_bindings(body, &mut arm_scope);
                        check_variables_in_stmts(body, &arm_scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
                    }
                    MatchArm::Compare(_, threshold, body) => {
                        check_variables_in_expr(threshold, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
                        let mut arm_scope = scope.clone();
                        collect_let_bindings(body, &mut arm_scope);
                        check_variables_in_stmts(body, &arm_scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
                    }
                }
            }
            if let Some(eb) = else_body {
                let mut eb_scope = scope.clone();
                collect_let_bindings(eb, &mut eb_scope);
                check_variables_in_stmts(eb, &eb_scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
            }
        }
        Statement::Break | Statement::Continue => {}
    }
}

/// Наряд №20: Check variable references and call arity in an expression.
fn check_variables_in_expr(
    expr: &Expr,
    scope: &HashSet<String>,
    entity_names: &HashSet<String>,
    builtin_names: &HashSet<String>,
    builtin_arity: &HashMap<&str, usize>,
    pattern_arity: &HashMap<String, usize>,
    result: &mut AnalysisResult,
) {
    match expr {
        Expr::StringLit(_) | Expr::FloatLit(_) | Expr::BoolLit(_) => {}
        Expr::Ident(name) => {
            // Check if the variable is defined
            if !scope.contains(name) && !entity_names.contains(name) && !builtin_names.contains(name) {
                result.errors.push(format!(
                    "undefined variable '{}'",
                    name
                ));
            }
        }
        Expr::FieldAccess(base, _field) => {
            check_variables_in_expr(base, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
        }
        Expr::FnCall(name, args) => {
            // Check all argument expressions
            for arg in args {
                check_variables_in_expr(arg, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
            }
            // Наряд №20: arity check for builtins (0 = variadic, skip check)
            if let Some(&expected) = builtin_arity.get(name.as_str()) {
                if expected > 0 && args.len() != expected {
                    result.errors.push(format!(
                        "builtin '{}' expects {} argument(s), got {}",
                        name, expected, args.len()
                    ));
                }
            }
            // Наряд №20: arity check for user-defined patterns
            if let Some(&expected) = pattern_arity.get(name) {
                if args.len() != expected {
                    result.errors.push(format!(
                        "pattern '{}' expects {} argument(s), got {}",
                        name, expected, args.len()
                    ));
                }
            }
            // If not a builtin and not a known pattern, that's an error
            if !builtin_names.contains(name) && !pattern_arity.contains_key(name) {
                result.errors.push(format!(
                    "undefined function '{}'",
                    name
                ));
            }
        }
        Expr::QualifiedCall { function, args, .. } => {
            for arg in args {
                check_variables_in_expr(arg, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
            }
            // Qualified calls: check function part
            if !builtin_names.contains(function) && !pattern_arity.contains_key(function) {
                result.errors.push(format!(
                    "undefined function '{}' in qualified call",
                    function
                ));
            }
            if let Some(&expected) = builtin_arity.get(function.as_str()) {
                if args.len() != expected {
                    result.errors.push(format!(
                        "builtin '{}' expects {} argument(s), got {}",
                        function, expected, args.len()
                    ));
                }
            }
        }
        Expr::BinaryOp(left, _, right) => {
            check_variables_in_expr(left, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
            check_variables_in_expr(right, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
        }
        Expr::IfElse(cond, then_expr, else_expr) => {
            check_variables_in_expr(cond, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
            check_variables_in_expr(then_expr, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
            check_variables_in_expr(else_expr, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
        }
        Expr::List(items) => {
            for item in items {
                check_variables_in_expr(item, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
            }
        }
        Expr::IndexAccess(base, index) => {
            check_variables_in_expr(base, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
            check_variables_in_expr(index, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
        }
        Expr::StructLit(fields) => {
            for (_, val) in fields {
                check_variables_in_expr(val, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
            }
        }
        Expr::BlockIfElse { condition, then_body, else_ifs, else_body } => {
            check_variables_in_expr(condition, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
            for stmt in then_body {
                check_variables_in_stmt(stmt, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
            }
            for (ei_cond, ei_body) in else_ifs {
                check_variables_in_expr(ei_cond, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
                for stmt in ei_body {
                    check_variables_in_stmt(stmt, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
                }
            }
            if let Some(eb) = else_body {
                for stmt in eb {
                    check_variables_in_stmt(stmt, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
                }
            }
        }
        Expr::Try(inner) => {
            check_variables_in_expr(inner, scope, entity_names, builtin_names, builtin_arity, pattern_arity, result);
        }
    }
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
        assert!(result.errors.iter().any(|e| e.contains("duplicate pattern")));
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
        assert!(result.errors.iter().any(|e| e.contains("unknown middleware")));
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
        assert!(result.errors.iter().any(|e| e.contains("unknown HTTP method")));
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
        assert!(result.warnings.iter().any(|w| w.contains("security_headers")));
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
        assert!(result.errors.iter().any(|e| e.contains("only Html is supported")));
    }

    // ── Наряд №19: Opaque type constraint tests ─────────────────────

    #[test]
    fn test_opaque_print_secret() {
        let source = r#"
entity api_key: Secret = env("API_KEY")
pattern leak() -> String {
    print(api_key)
    return "done"
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result.errors.iter().any(|e| e.contains("opaque type constraint") && e.contains("print")));
    }

    #[test]
    fn test_opaque_to_string_secret() {
        let source = r#"
entity api_key: Secret = env("API_KEY")
pattern leak2() -> String {
    return to_string(api_key)
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result.errors.iter().any(|e| e.contains("opaque type constraint") && e.contains("to_string")));
    }

    #[test]
    fn test_opaque_concat_forbidden() {
        let source = r#"
entity html_content: Html = escape_html("<b>bold</b>")
pattern xss_vector() -> String {
    let payload = "<script>alert(1)</script>" + html_content
    return payload
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result.errors.iter().any(|e| e.contains("concatenate") && e.contains("opaque")));
    }

    #[test]
    fn test_opaque_field_access_allowed() {
        let source = r#"
entity sess: Session = session_login("user1")
pattern check_session() -> String {
    return sess.role
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        // Field access on opaque is OK — only print/concat/toString are restricted
        // (We expect other errors like undefined variable 'sess.role' but NOT opaque errors)
        let has_opaque_error = result.errors.iter().any(|e| e.contains("opaque type constraint"));
        assert!(!has_opaque_error, "should not have opaque errors for field access, got: {:?}", result.errors);
    }

    // ── Наряд №20: Variable + arity checking tests ────────────────

    #[test]
    fn test_undefined_variable_detected() {
        let source = r#"
pattern use_undef() -> String {
    return undefined_var
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result.errors.iter().any(|e| e.contains("undefined variable") && e.contains("undefined_var")));
    }

    #[test]
    fn test_let_binding_in_scope() {
        let source = r#"
pattern scoped_var(x: String) -> String {
    let y = upper(x)
    return y
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(result.is_ok(), "let binding should be in scope, errors: {:?}", result.errors);
    }

    #[test]
    fn test_arity_mismatch_builtin() {
        let source = r#"
pattern wrong_arity(x: String) -> String {
    return upper(x, "extra")
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result.errors.iter().any(|e| e.contains("expects 1 argument") && e.contains("got 2")));
    }

    #[test]
    fn test_arity_mismatch_pattern() {
        let source = r#"
pattern double(x: String) -> String {
    return x + x
}
pattern call_wrong() -> String {
    return double("a", "b")
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result.errors.iter().any(|e| e.contains("expects 1 argument") && e.contains("double")));
    }

    #[test]
    fn test_undefined_function_detected() {
        let source = r#"
pattern call_nonexistent() -> String {
    return nonexistent_fn("hi")
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result.errors.iter().any(|e| e.contains("undefined function") && e.contains("nonexistent_fn")));
    }

    #[test]
    fn test_assign_undefined_var() {
        let source = r#"
pattern bad_assign() -> String {
    nonexistent = "oops"
    return "done"
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result.errors.iter().any(|e| e.contains("assignment to undefined variable") && e.contains("nonexistent")));
    }

    #[test]
    fn test_each_var_in_scope() {
        let source = r#"
pattern sum_items(items: List) -> String {
    let mut total = ""
    each item in items {
        total = total + item
    }
    return total
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        // 'item' should be in scope inside the each block
        let has_undef = result.errors.iter().any(|e| e.contains("undefined variable") && e.contains("item"));
        assert!(!has_undef, "'item' should be in scope in each block, errors: {:?}", result.errors);
    }

    #[test]
    fn test_builtin_arity_ok() {
        let source = r#"
pattern correct_calls(x: String) -> String {
    let up = upper(x)
    let idx = index_of(x, "a")
    let l = len(x)
    return up
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        let arity_errors: Vec<_> = result.errors.iter().filter(|e| e.contains("expects")).collect();
        assert!(arity_errors.is_empty(), "unexpected arity errors: {:?}", arity_errors);
    }
}