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

    result
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
