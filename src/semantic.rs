// ── Semantic analysis for METALOGOS Phase 1 ──────────────────────────────
// Validates the AST before execution: undefined entities, unknown patterns/steps,
// rule target validation. Returns clear error messages instead of runtime panics.

use std::collections::{HashSet, HashMap};

use crate::ast::*;

/// Built-in functions known to the runtime (not defined in .mlog source).
const KNOWN_BUILTINS: &[&str] = &[
    "upper", "lower", "len", "str", "print", "contains", "float",
    "confidence", "find", "count", "recall",
];

/// Semantic analysis context: accumulates definitions and checks references.
struct Context {
    /// User-defined struct type names (from EntityType declarations).
    types: HashSet<String>,
    /// Entity variable names (from EntityRecord, EntitySimple, Fluid declarations).
    entities: HashSet<String>,
    /// Pattern names.
    patterns: HashSet<String>,
    /// Learnable pattern names.
    learnables: HashSet<String>,
    /// Pattern param names → param types (for future type checking).
    #[allow(dead_code)]
    pattern_params: HashMap<String, Vec<(String, String)>>,
}

impl Context {
    fn new() -> Self {
        Context {
            types: HashSet::new(),
            entities: HashSet::new(),
            patterns: HashSet::new(),
            learnables: HashSet::new(),
            pattern_params: HashMap::new(),
        }
    }

    /// Pass 1: collect definitions (types, patterns, learnables first so they
    /// can be forward-referenced; entities in declaration order).
    fn collect(&mut self, decl: &Declaration) {
        match decl {
            Declaration::EntityType(e) => {
                self.types.insert(e.name.clone());
            }
            Declaration::Pattern(p) => {
                self.patterns.insert(p.name.clone());
                let params: Vec<(String, String)> = p.params.iter()
                    .map(|pm| (pm.name.clone(), pm.type_name.clone()))
                    .collect();
                self.pattern_params.insert(p.name.clone(), params);
            }
            Declaration::LearnablePattern(lp) => {
                self.learnables.insert(lp.name.clone());
            }
            // Entities are collected in order during pass 2
            _ => {}
        }
    }

    /// Collect an entity name (called during pass 2 before checking references).
    fn collect_entity(&mut self, name: &str) {
        self.entities.insert(name.to_string());
    }

    /// Check if a name is a known callable (pattern, learnable, or builtin).
    fn is_known_call(&self, name: &str) -> bool {
        self.patterns.contains(name)
            || self.learnables.contains(name)
            || KNOWN_BUILTINS.contains(&name)
    }

    /// Check an expression for undefined references.
    /// `scope` contains in-scope names (e.g. pattern parameters).
    fn check_expr(&self, expr: &Expr, scope: &HashSet<&str>) -> Result<(), String> {
        match expr {
            Expr::StringLit(_) | Expr::FloatLit(_) => Ok(()),
            Expr::Ident(name) => {
                if scope.contains(name.as_str()) {
                    return Ok(());
                }
                if self.entities.contains(name) {
                    return Ok(());
                }
                Err(format!("undefined entity '{}'", name))
            }
            Expr::FieldAccess(base, _field) => {
                self.check_expr(base, scope)
            }
            Expr::FnCall(name, args) => {
                if !self.is_known_call(name) {
                    return Err(format!("undefined pattern or builtin '{}'", name));
                }
                for arg in args {
                    self.check_expr(arg, scope)?;
                }
                Ok(())
            }
            Expr::BinaryOp(left, _op, right) => {
                self.check_expr(left, scope)?;
                self.check_expr(right, scope)
            }
        }
    }

    /// Check a declaration's references (pass 2: sequential).
    fn check(&mut self, decl: &Declaration) -> Result<(), String> {
        let empty_scope = HashSet::new();

        match decl {
            Declaration::EntityType(_) => Ok(()),
            Declaration::EntityRecord(e) => {
                self.collect_entity(&e.name);
                // Check type exists
                if !self.types.contains(&e.type_name) {
                    return Err(format!(
                        "unknown type '{}' for entity '{}'",
                        e.type_name, e.name
                    ));
                }
                // Check field init expressions
                for init in &e.fields {
                    self.check_expr(&init.value, &empty_scope)?;
                }
                Ok(())
            }
            Declaration::EntitySimple(e) => {
                self.collect_entity(&e.name);
                self.check_expr(&e.value, &empty_scope)
            }
            Declaration::Rule(r) => {
                // Check rule target is a known entity
                if let Expr::Ident(name) = &r.target {
                    if !self.entities.contains(name) {
                        return Err(format!(
                            "rule target '{}' is not a defined entity",
                            name
                        ));
                    }
                }
                // Check condition
                match &r.condition {
                    Condition::Contains { left, right } => {
                        self.check_expr(left, &empty_scope)?;
                        self.check_expr(right, &empty_scope)?;
                    }
                    Condition::Compare { left, right, .. } => {
                        self.check_expr(left, &empty_scope)?;
                        self.check_expr(right, &empty_scope)?;
                    }
                }
                // Check value expression
                self.check_expr(&r.value, &empty_scope)
            }
            Declaration::Pattern(p) => {
                // Check body with params in scope
                let scope: HashSet<&str> = p.params.iter()
                    .map(|pm| pm.name.as_str())
                    .collect();
                for stmt in &p.body {
                    let Statement::Return(expr) = stmt;
                    self.check_expr(expr, &scope)?;
                }
                Ok(())
            }
            Declaration::LearnablePattern(_) => Ok(()),
            Declaration::Flow(f) => {
                // Collect branch def step names (valid pipeline targets)
                let branch_steps: HashSet<&str> = f.branch_defs.iter()
                    .map(|(name, _)| name.as_str())
                    .collect();

                // Check source expression
                self.check_expr(&f.source, &empty_scope)?;

                // Check pipeline steps
                for step in &f.pipeline {
                    if !self.is_known_call(step)
                        && !branch_steps.contains(step.as_str())
                    {
                        return Err(format!(
                            "undefined step '{}' in flow '{}'",
                            step, f.name
                        ));
                    }
                }

                // Check branch targets
                for (_step_name, branches) in &f.branch_defs {
                    for branch in branches {
                        if !self.is_known_call(&branch.target) {
                            return Err(format!(
                                "undefined target '{}' in flow '{}' branch '{}'",
                                branch.target, f.name, branch.label
                            ));
                        }
                    }
                }

                Ok(())
            }
            Declaration::Fluid(fl) => {
                self.collect_entity(&fl.name);
                for v in &fl.variants {
                    self.check_expr(&v.value, &empty_scope)?;
                }
                Ok(())
            }
            Declaration::Memorize(m) => {
                self.check_expr(&m.value, &empty_scope)
            }
            Declaration::Forget(f) => {
                self.check_expr(&f.query, &empty_scope)
            }
            Declaration::Adapt(a) => {
                if !self.learnables.contains(&a.pattern_name) {
                    return Err(format!(
                        "adapt target '{}' is not a learnable pattern",
                        a.pattern_name
                    ));
                }
                self.check_expr(&a.input_example, &empty_scope)?;
                self.check_expr(&a.output_example, &empty_scope)
            }
        }
    }
}

/// Run semantic analysis on a list of declarations.
/// Returns Err with a clear error message if any issue is found.
pub fn analyze(decls: &[Declaration]) -> Result<(), String> {
    let mut ctx = Context::new();

    // Pass 1: collect type definitions and pattern signatures (forward-referenceable)
    for decl in decls {
        ctx.collect(decl);
    }

    // Pass 2: walk declarations in order, collecting entities + checking references
    for decl in decls {
        ctx.check(decl)?;
    }

    Ok(())
}
