// ── Semantic analysis for METALOGOS Phase 2 ─────────────────────────────
// Validates the AST before execution:
//   Phase 1: undefined entities, unknown patterns/steps, rule target validation.
//   Phase 2: type inference through flow pipeline, error recovery (all errors
//            collected per pass), unreachable/overlapping branch detection.
// Returns clear error messages instead of runtime panics.

use std::collections::{HashSet, HashMap};

use crate::ast::*;

/// Built-in functions known to the runtime (not defined in .mlog source).
const KNOWN_BUILTINS: &[&str] = &[
    "upper", "lower", "len", "str", "print", "contains", "float",
    "confidence", "find", "count", "recall",
];

/// Builtin return types for type inference.
fn builtin_return_type(name: &str) -> &'static str {
    match name {
        "upper" | "lower" | "str" | "print" => "String",
        "len" | "float" | "confidence" | "contains" | "count" => "Float",
        "find" => "Struct",
        "recall" => "String",
        _ => "Unknown",
    }
}

/// Result of semantic analysis: errors prevent execution, warnings do not.
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl AnalysisResult {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Format errors into a single string for error output.
    pub fn format_errors(&self) -> String {
        if self.errors.len() == 1 {
            self.errors[0].clone()
        } else {
            let mut msg = format!("{} errors:\n", self.errors.len());
            for (i, e) in self.errors.iter().enumerate() {
                msg.push_str(&format!("{}: {}\n", i + 1, e));
            }
            msg
        }
    }
}

/// Semantic analysis context: accumulates definitions, checks references,
/// infers types, detects overlapping branches.
struct Context {
    /// User-defined struct type names (from EntityType declarations).
    types: HashSet<String>,
    /// Struct type field definitions: type_name → [(field_name, field_type)]
    struct_fields: HashMap<String, Vec<(String, String)>>,
    /// Entity variable names (from EntityRecord, EntitySimple, Fluid declarations).
    entities: HashSet<String>,
    /// Entity variable → declared type name (for type inference).
    entity_types: HashMap<String, String>,
    /// Pattern names.
    patterns: HashSet<String>,
    /// Learnable pattern names.
    learnables: HashSet<String>,
    /// Pattern param names → param types.
    pattern_params: HashMap<String, Vec<(String, String)>>,
    /// Pattern name → return type.
    pattern_return_types: HashMap<String, String>,
    /// Learnable pattern name → return type.
    learnable_return_types: HashMap<String, String>,
    /// Accumulated errors (collected, not returned on first).
    errors: Vec<String>,
    /// Accumulated warnings.
    warnings: Vec<String>,
}

impl Context {
    fn new() -> Self {
        Context {
            types: HashSet::new(),
            struct_fields: HashMap::new(),
            entities: HashSet::new(),
            entity_types: HashMap::new(),
            patterns: HashSet::new(),
            learnables: HashSet::new(),
            pattern_params: HashMap::new(),
            pattern_return_types: HashMap::new(),
            learnable_return_types: HashMap::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Pass 1: collect definitions (types, patterns, learnables first so they
    /// can be forward-referenced; entities collected in pass 2).
    fn collect(&mut self, decl: &Declaration) {
        match decl {
            Declaration::EntityType(e) => {
                self.types.insert(e.name.clone());
                let fields: Vec<(String, String)> = e.fields.iter()
                    .map(|f| (f.name.clone(), f.type_name.clone()))
                    .collect();
                self.struct_fields.insert(e.name.clone(), fields);
            }
            Declaration::Pattern(p) => {
                self.patterns.insert(p.name.clone());
                let params: Vec<(String, String)> = p.params.iter()
                    .map(|pm| (pm.name.clone(), pm.type_name.clone()))
                    .collect();
                self.pattern_params.insert(p.name.clone(), params);
                self.pattern_return_types.insert(p.name.clone(), p.return_type.clone());
            }
            Declaration::LearnablePattern(lp) => {
                self.learnables.insert(lp.name.clone());
                self.learnable_return_types.insert(lp.name.clone(), lp.return_type.clone());
            }
            _ => {}
        }
    }

    /// Collect an entity name + its type (called during pass 2).
    fn collect_entity(&mut self, name: &str, type_name: &str) {
        self.entities.insert(name.to_string());
        self.entity_types.insert(name.to_string(), type_name.to_string());
    }

    /// Check if a name is a known callable (pattern, learnable, or builtin).
    fn is_known_call(&self, name: &str) -> bool {
        self.patterns.contains(name)
            || self.learnables.contains(name)
            || KNOWN_BUILTINS.contains(&name)
    }

    /// Check if two types are compatible for flow-to-pattern binding.
    /// Rules:
    ///   - Identical types: compatible
    ///   - Fluid: compatible with everything (lazy collapse at runtime)
    ///   - "Struct" (generic, e.g. from find()): compatible with any struct type
    ///   - Two different user-defined struct types: incompatible
    ///   - Two different primitive types (e.g. String vs Float): incompatible
    fn types_compatible(flow_type: &str, param_type: &str, user_types: &HashSet<String>) -> bool {
        if flow_type == param_type {
            return true;
        }
        // Fluid is compatible with everything (lazy collapse at runtime)
        if flow_type == "Fluid" || param_type == "Fluid" {
            return true;
        }
        // Generic "Struct" return (e.g. from find()) compatible with any user-defined struct
        if flow_type == "Struct" && user_types.contains(param_type) {
            return true;
        }
        if param_type == "Struct" && user_types.contains(flow_type) {
            return true;
        }
        // Two different user-defined struct types: incompatible
        if user_types.contains(flow_type) && user_types.contains(param_type) {
            return false;
        }
        // Both are known primitives but different: incompatible
        let known_primitives = ["String", "Float", "Int", "Bool", "Unit"];
        let flow_is_prim = known_primitives.contains(&flow_type);
        let param_is_prim = known_primitives.contains(&param_type);
        if flow_is_prim && param_is_prim {
            return false;
        }
        // Otherwise: conservative, allow (can't determine statically)
        true
    }

    /// Infer the type of an expression. Returns None if type cannot be determined.
    fn infer_type(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::StringLit(_) => Some("String".to_string()),
            Expr::FloatLit(_) => Some("Float".to_string()),
            Expr::Ident(name) => self.entity_types.get(name).cloned(),
            Expr::FieldAccess(base, field) => {
                let base_type = self.infer_type(base)?;
                // Look up field type in struct definitions
                if let Some(fields) = self.struct_fields.get(&base_type) {
                    for (fname, ftype) in fields {
                        if fname == field {
                            return Some(ftype.clone());
                        }
                    }
                }
                None
            }
            Expr::FnCall(name, _args) => {
                // Check pattern return type
                if let Some(rt) = self.pattern_return_types.get(name) {
                    return Some(rt.clone());
                }
                // Check learnable return type
                if let Some(rt) = self.learnable_return_types.get(name) {
                    return Some(rt.clone());
                }
                // Check builtin return type
                if KNOWN_BUILTINS.contains(&name.as_str()) {
                    return Some(builtin_return_type(name).to_string());
                }
                None
            }
            Expr::BinaryOp(left, op, right) => {
                match op {
                    BinOp::Add => {
                        let lt = self.infer_type(left)?;
                        let rt = self.infer_type(right)?;
                        // String + String → String, otherwise Float
                        if lt == "String" || rt == "String" {
                            Some("String".to_string())
                        } else {
                            Some("Float".to_string())
                        }
                    }
                    BinOp::Sub | BinOp::Mul | BinOp::Div => {
                        Some("Float".to_string())
                    }
                }
            }
        }
    }

    /// Check an expression for undefined references.
    /// `scope` contains in-scope names (e.g. pattern parameters).
    fn check_expr(&mut self, expr: &Expr, scope: &HashSet<&str>) {
        match expr {
            Expr::StringLit(_) | Expr::FloatLit(_) => {}
            Expr::Ident(name) => {
                if scope.contains(name.as_str()) {
                    return;
                }
                if self.entities.contains(name) {
                    return;
                }
                self.errors.push(format!("undefined entity '{}'", name));
            }
            Expr::FieldAccess(base, _field) => {
                self.check_expr(base, scope);
            }
            Expr::FnCall(name, args) => {
                if !self.is_known_call(name) {
                    self.errors.push(format!("undefined pattern or builtin '{}'", name));
                }
                for arg in args {
                    self.check_expr(arg, scope);
                }
            }
            Expr::BinaryOp(left, _op, right) => {
                self.check_expr(left, scope);
                self.check_expr(right, scope);
            }
        }
    }

    /// Check a declaration's references (pass 2: sequential).
    /// Collects all errors instead of returning early.
    fn check(&mut self, decl: &Declaration) {
        let empty_scope = HashSet::new();

        match decl {
            Declaration::EntityType(_) => {}
            Declaration::EntityRecord(e) => {
                // Always collect entity (even with bad type) to avoid cascading errors
                self.collect_entity(&e.name, &e.type_name);
                // Check type exists (user-defined struct type)
                if !self.types.contains(&e.type_name) {
                    self.errors.push(format!(
                        "unknown type '{}' for entity '{}'",
                        e.type_name, e.name
                    ));
                }
                // Check field init expressions
                for init in &e.fields {
                    self.check_expr(&init.value, &empty_scope);
                }
            }
            Declaration::EntitySimple(e) => {
                self.collect_entity(&e.name, &e.type_name);
                self.check_expr(&e.value, &empty_scope);
            }
            Declaration::Rule(r) => {
                // Check rule target is a known entity
                if let Expr::Ident(name) = &r.target {
                    if !self.entities.contains(name) {
                        self.errors.push(format!(
                            "rule target '{}' is not a defined entity",
                            name
                        ));
                    }
                }
                // Check condition
                match &r.condition {
                    Condition::Contains { left, right } => {
                        self.check_expr(left, &empty_scope);
                        self.check_expr(right, &empty_scope);
                    }
                    Condition::Compare { left, right, .. } => {
                        self.check_expr(left, &empty_scope);
                        self.check_expr(right, &empty_scope);
                    }
                }
                // Check value expression
                self.check_expr(&r.value, &empty_scope);
            }
            Declaration::Pattern(p) => {
                // Check body with params in scope
                let scope: HashSet<&str> = p.params.iter()
                    .map(|pm| pm.name.as_str())
                    .collect();
                for stmt in &p.body {
                    let Statement::Return(expr) = stmt;
                    self.check_expr(expr, &scope);
                }
            }
            Declaration::LearnablePattern(_) => {}
            Declaration::Flow(f) => {
                // Collect branch def step names (valid pipeline targets)
                let branch_steps: HashSet<&str> = f.branch_defs.iter()
                    .map(|(name, _)| name.as_str())
                    .collect();

                // Check source expression
                self.check_expr(&f.source, &empty_scope);

                // Infer source type for flow type checking
                let mut current_type = self.infer_type(&f.source);

                // Check pipeline steps + type inference
                for step in &f.pipeline {
                    if !self.is_known_call(step)
                        && !branch_steps.contains(step.as_str())
                    {
                        self.errors.push(format!(
                            "undefined step '{}' in flow '{}'",
                            step, f.name
                        ));
                        current_type = None; // Can't infer further
                        continue;
                    }

                    // Type check: if step is a pattern, check param compatibility
                    if let Some(params) = self.pattern_params.get(step) {
                        if let Some(ref flow_type) = current_type {
                            if let Some((_, param_type)) = params.first() {
                                if !Self::types_compatible(
                                    flow_type, param_type, &self.types
                                ) {
                                    self.errors.push(format!(
                                        "type mismatch: pattern '{}' expects {}, but flow provides {}",
                                        step, param_type, flow_type
                                    ));
                                }
                            }
                        }
                    }

                    // Update current_type: pattern/learnable return type
                    if self.patterns.contains(step) {
                        current_type = self.pattern_return_types.get(step).cloned();
                    } else if self.learnables.contains(step) {
                        current_type = self.learnable_return_types.get(step).cloned();
                    }
                    // branch_def steps: type passes through (dispatched at runtime)
                }

                // Check branch targets
                for (_step_name, branches) in &f.branch_defs {
                    for branch in branches {
                        if !self.is_known_call(&branch.target) {
                            self.errors.push(format!(
                                "undefined target '{}' in flow '{}' branch '{}'",
                                branch.target, f.name, branch.label
                            ));
                        }
                    }
                }

                // Check for overlapping branch conditions
                self.check_branch_overlap(f);
            }
            Declaration::Fluid(fl) => {
                self.collect_entity(&fl.name, "Fluid");
                for v in &fl.variants {
                    self.check_expr(&v.value, &empty_scope);
                }
            }
            Declaration::Memorize(m) => {
                self.check_expr(&m.value, &empty_scope);
            }
            Declaration::Forget(f) => {
                self.check_expr(&f.query, &empty_scope);
            }
            Declaration::Adapt(a) => {
                if !self.learnables.contains(&a.pattern_name) {
                    self.errors.push(format!(
                        "adapt target '{}' is not a learnable pattern",
                        a.pattern_name
                    ));
                }
                self.check_expr(&a.input_example, &empty_scope);
                self.check_expr(&a.output_example, &empty_scope);
            }
        }
    }

    /// Detect overlapping branch conditions within the same branch_def block.
    /// Two branches on the same field overlap if their numeric ranges intersect.
    fn check_branch_overlap(&mut self, f: &FlowDecl) {
        for (step_name, branches) in &f.branch_defs {
            for i in 0..branches.len() {
                for j in (i + 1)..branches.len() {
                    let b1 = &branches[i];
                    let b2 = &branches[j];

                    // Only check if conditions are on the same field
                    if b1.condition.field != b2.condition.field {
                        continue;
                    }

                    // Try to extract float thresholds
                    let t1 = Self::extract_float_threshold(&b1.condition.threshold);
                    let t2 = Self::extract_float_threshold(&b2.condition.threshold);

                    if let (Some(v1), Some(v2)) = (t1, t2) {
                        if Self::ranges_overlap(b1.condition.op, v1, b2.condition.op, v2) {
                            self.warnings.push(format!(
                                "warning: overlapping branches '{}' and '{}' in flow '{}' step '{}'",
                                b1.label, b2.label, f.name, step_name
                            ));
                        }
                    }
                }
            }
        }
    }

    /// Extract a float value from a threshold expression (only FloatLit supported).
    fn extract_float_threshold(expr: &Expr) -> Option<f64> {
        match expr {
            Expr::FloatLit(f) => Some(*f),
            _ => None,
        }
    }

    /// Check if two half-intervals on the same axis overlap.
    /// Each condition defines a half-interval:
    ///   > X  → (X, +∞) exclusive
    ///   >= X → [X, +∞) inclusive
    ///   < Y  → (-∞, Y) exclusive
    ///   <= Y → (-∞, Y] inclusive
    ///   == V → [V, V] inclusive
    fn ranges_overlap(op1: CompareOp, t1: f64, op2: CompareOp, t2: f64) -> bool {
        let (lo1, lo1_inc, hi1, hi1_inc) = Self::half_interval(op1, t1);
        let (lo2, lo2_inc, hi2, hi2_inc) = Self::half_interval(op2, t2);

        let lo = lo1.max(lo2);
        let hi = hi1.min(hi2);

        if lo > hi {
            return false;
        }
        if lo < hi {
            return true;
        }
        // lo == hi: the single value v = lo = hi is in both intervals
        // only if both intervals include it at their respective boundary
        let v_in_1 = (lo1 < lo) || lo1_inc;
        let v_in_1 = v_in_1 && ((hi1 > hi) || hi1_inc);
        let v_in_2 = (lo2 < lo) || lo2_inc;
        let v_in_2 = v_in_2 && ((hi2 > hi) || hi2_inc);

        v_in_1 && v_in_2
    }

    /// Convert a comparison operator + threshold to a half-interval:
    /// (lower_bound, lower_inclusive, upper_bound, upper_inclusive)
    fn half_interval(op: CompareOp, t: f64) -> (f64, bool, f64, bool) {
        match op {
            CompareOp::Gt => (t, false, f64::INFINITY, false),
            CompareOp::Ge => (t, true, f64::INFINITY, false),
            CompareOp::Lt => (f64::NEG_INFINITY, false, t, false),
            CompareOp::Le => (f64::NEG_INFINITY, false, t, true),
            CompareOp::Eq => (t, true, t, true),
        }
    }
}

/// Run semantic analysis on a list of declarations.
/// Returns an AnalysisResult with errors and warnings.
pub fn analyze(decls: &[Declaration]) -> AnalysisResult {
    let mut ctx = Context::new();

    // Pass 1: collect type definitions and pattern signatures (forward-referenceable)
    for decl in decls {
        ctx.collect(decl);
    }

    // Pass 2: walk declarations in order, collecting entities + checking references
    for decl in decls {
        ctx.check(decl);
    }

    AnalysisResult {
        errors: ctx.errors,
        warnings: ctx.warnings,
    }
}
