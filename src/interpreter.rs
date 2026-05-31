// ── Tree-walking interpreter for METALOGOS M1+M2 ─────────────────────
// M1: entities, patterns, linear flow
// M2: struct entities, rules, branching flow, comparisons

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ast::*;
use crate::builtins::Builtins;
use crate::embedding;
use crate::llm;
use crate::ml;

/// A single variant inside a Fluid value (runtime). Contains a concrete
/// value, its declared type name, and a confidence score (0.0..1.0).
#[derive(Debug, Clone)]
pub struct FluidValueVariant {
    pub type_name: String,
    pub value: Value,
    pub confidence: f64,
}

/// Runtime value.
#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Float(f64),
    Struct {
        type_name: String,
        fields: HashMap<String, Value>,
    },
    /// Fluid value: superposition of typed variants with confidence scores.
    /// Collapses lazily at point of use (see `maybe_collapse`).
    Fluid(Vec<FluidValueVariant>),
    Unit,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::Float(n) => write!(f, "{}", n),
            Value::Struct { type_name, fields } => {
                write!(f, "{} {{", type_name)?;
                let pairs: Vec<_> = fields.iter().collect();
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Fluid(variants) => {
                // Display as the highest-confidence variant
                let best = variants.iter().max_by(|a, b| {
                    a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal)
                });
                match best {
                    Some(v) => write!(f, "{}", v.value),
                    None => write!(f, "()"),
                }
            }
            Value::Unit => write!(f, "()"),
        }
    }
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::String(_) => "String",
            Value::Float(_) => "Float",
            Value::Struct { .. } => "Struct",
            Value::Fluid(_) => "Fluid",
            Value::Unit => "Unit",
        }
    }

    /// Get a field value from a struct. Returns Err if not a struct or field missing.
    pub fn get_field(&self, field: &str) -> Result<&Value, String> {
        match self {
            Value::Struct { fields, .. } => fields.get(field)
                .ok_or_else(|| format!("field '{}' not found on struct", field)),
            _ => Err(format!("cannot access field '{}' on non-struct value ({})", field, self.type_name())),
        }
    }

    /// Set a field value on a mutable struct.
    pub fn set_field(&mut self, field: &str, value: Value) -> Result<(), String> {
        match self {
            Value::Struct { fields, .. } => {
                if fields.contains_key(field) {
                    fields.insert(field.to_string(), value);
                    Ok(())
                } else {
                    Err(format!("field '{}' not found on struct", field))
                }
            }
            _ => Err(format!("cannot set field '{}' on non-struct value", field)),
        }
    }

    /// Convert to f64 for numeric comparisons.
    pub fn as_float(&self) -> Result<f64, String> {
        match self {
            Value::Float(f) => Ok(*f),
            Value::String(s) => s.parse::<f64>()
                .map_err(|_| format!("cannot convert '{}' to Float", s)),
            _ => Err(format!("cannot convert {} to Float", self.type_name())),
        }
    }

    /// Check if this value is a Fluid superposition.
    pub fn is_fluid(&self) -> bool {
        matches!(self, Value::Fluid(_))
    }
}

/// A compiled pattern ready for invocation.
#[derive(Clone)]
struct CompiledPattern {
    params: Vec<Param>,
    body: Vec<Statement>,
}

/// A learnable pattern that calls an LLM.
#[derive(Clone)]
struct CompiledLearnable {
    params: Vec<Param>,
    prompt: String,
    /// Few-shot examples added by `adapt` declarations.
    /// Each entry: (input_string, output_string).
    few_shot: Vec<(String, String)>,
}

/// A registered struct type.
#[derive(Clone)]
struct StructType {
    #[allow(dead_code)]
    name: String,
    fields: Vec<FieldDecl>,
}

/// An entry in the Entity Store: a lightweight index of entity identity by type.
/// The actual value lives in `self.variables[id]` — the store is just an index
/// that enables querying by predicate without knowing variable names.
#[derive(Debug, Clone)]
struct EntityRecord {
    /// The variable name (identity) of this entity.
    id: String,
    /// The struct type name (e.g. "Message").
    #[allow(dead_code)] // Used for type-based queries in future phases
    type_name: String,
}

/// A memory entry: stored fact with priority and timestamp for decay.
#[derive(Debug, Clone)]
struct MemoryEntry {
    /// The stored value (string content of the fact).
    value: String,
    /// Priority/confidence at time of memorization (0.0..1.0).
    priority: f64,
    /// Unix timestamp (seconds) when memorized.
    timestamp: i64,
    /// Decay rate per day (0.0 = no decay, 0.01 = slow, 0.1 = fast).
    decay_rate: f64,
}

/// The interpreter holds all runtime state.
pub struct Interpreter {
    /// Global variable store.
    variables: HashMap<String, Value>,
    /// Struct type registry.
    struct_types: HashMap<String, StructType>,
    /// Compiled patterns (pure).
    patterns: HashMap<String, CompiledPattern>,
    /// Compiled learnable patterns (LLM-backed).
    learnable_patterns: HashMap<String, CompiledLearnable>,
    /// Rule declarations (stored for later execution).
    rules: Vec<RuleDecl>,
    /// Built-in function registry.
    builtins: Builtins,
    /// Flow branch definitions: step_name -> [Branch]
    branch_defs: HashMap<String, Vec<Branch>>,
    /// Memory store: entries with priority, timestamp, and decay.
    memory: Vec<MemoryEntry>,
    /// Entity Store: type_name → [entity records] index.
    /// Enables CRUD-by-id and query-by-predicate. Actual values in `variables`.
    entity_store: HashMap<String, Vec<EntityRecord>>,
    /// Embedding backend for semantic similarity in recall.
    embedding: Box<dyn embedding::EmbeddingBackend>,
    /// ML backend for fine-tuning learnable patterns.
    ml_backend: Box<dyn ml::MlBackend>,
    /// Status messages from `learn` declarations (accumulated during run).
    learn_log: Vec<String>,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            variables: HashMap::new(),
            struct_types: HashMap::new(),
            patterns: HashMap::new(),
            learnable_patterns: HashMap::new(),
            rules: Vec::new(),
            builtins: Builtins::new(),
            branch_defs: HashMap::new(),
            memory: Vec::new(),
            entity_store: HashMap::new(),
            embedding: embedding::create_embedding_backend(),
            ml_backend: ml::create_ml_backend(),
            learn_log: Vec::new(),
        }
    }

    /// Return accumulated learn status messages and clear the log.
    pub fn take_learn_log(&mut self) -> Vec<String> {
        std::mem::take(&mut self.learn_log)
    }

    /// Run a complete .mlog program.
    pub fn run(&mut self, declarations: Vec<Declaration>) -> Result<Option<String>, String> {
        let mut output: Option<String> = None;

        for decl in declarations {
            match decl {
                Declaration::EntityType(e) => {
                    self.struct_types.insert(e.name.clone(), StructType {
                        name: e.name.clone(),
                        fields: e.fields,
                    });
                }
                Declaration::EntityRecord(e) => {
                    let value = self.instantiate_struct(&e.type_name, &e.fields)?;
                    // Register in Entity Store (identity index)
                    self.entity_store.entry(e.type_name.clone())
                        .or_default()
                        .push(EntityRecord {
                            id: e.name.clone(),
                            type_name: e.type_name.clone(),
                        });
                    self.variables.insert(e.name.clone(), value);
                }
                Declaration::EntitySimple(e) => {
                    let value = self.eval_expr(&e.value)?;
                    self.variables.insert(e.name.clone(), value);
                }
                Declaration::Rule(r) => {
                    self.rules.push(r);
                }
                Declaration::Memorize(m) => {
                    let value_str = match self.eval_expr(&m.value)? {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    self.memory.push(MemoryEntry {
                        value: value_str,
                        priority: m.priority,
                        timestamp: now,
                        decay_rate: 0.01, // Default: slow decay
                    });
                }
                Declaration::Forget(f) => {
                    let query_str = match self.eval_expr(&f.query)? {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    let cutoff = now - (f.days * 86400);
                    self.memory.retain(|entry| {
                        !(entry.value.contains(&query_str) && entry.timestamp < cutoff)
                    });
                }
                Declaration::Pattern(p) => {
                    self.patterns.insert(
                        p.name.clone(),
                        CompiledPattern {
                            params: p.params.clone(),
                            body: p.body.clone(),
                        },
                    );
                }
                Declaration::LearnablePattern(lp) => {
                    self.learnable_patterns.insert(
                        lp.name.clone(),
                        CompiledLearnable {
                            params: lp.params.clone(),
                            prompt: lp.prompt.clone(),
                            few_shot: Vec::new(),
                        },
                    );
                }
                Declaration::Adapt(a) => {
                    let input_str = match self.eval_expr(&a.input_example)? {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let output_str = match self.eval_expr(&a.output_example)? {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    if let Some(learnable) = self.learnable_patterns.get_mut(&a.pattern_name) {
                        learnable.few_shot.push((input_str, output_str));
                    } else {
                        return Err(format!("adapt: learnable pattern '{}' not found", a.pattern_name));
                    }
                }
                Declaration::Learn(l) => {
                    // Fine-tune the learnable pattern via the ML backend
                    let learnable = match self.learnable_patterns.get(&l.pattern_name) {
                        Some(lp) => lp.clone(),
                        None => return Err(format!("learn: learnable pattern '{}' not found", l.pattern_name)),
                    };

                    // Evaluate hyperparameters
                    let mut data_str = String::new();
                    let mut epochs: i32 = 1;
                    for (param_name, param_expr) in &l.hyperparams {
                        let val = self.eval_expr(param_expr)?;
                        match param_name.as_str() {
                            "data" => data_str = match val {
                                Value::String(s) => s,
                                other => format!("{}", other),
                            },
                            "epochs" => epochs = val.as_float()? as i32,
                            _ => {} // Ignore unknown hyperparams
                        }
                    }

                    // Call ML backend for fine-tuning
                    let result = self.ml_backend.fine_tune(
                        &l.pattern_name,
                        &learnable.prompt,
                        &data_str,
                        epochs,
                    )?;

                    // Log the learn status
                    self.learn_log.push(format!("[LEARN] {}", result.summary));
                }
                Declaration::Fluid(fl) => {
                    let mut variants = Vec::new();
                    for v in &fl.variants {
                        let val = self.eval_expr(&v.value)?;
                        variants.push(FluidValueVariant {
                            type_name: v.type_name.clone(),
                            value: val,
                            confidence: v.confidence,
                        });
                    }
                    self.variables.insert(fl.name.clone(), Value::Fluid(variants));
                }
                Declaration::Flow(f) => {
                    // Execute rules before flow (they modify entity state)
                    self.execute_rules()?;
                    output = Some(self.run_flow(&f)?);
                }
            }
        }

        Ok(output)
    }

    /// Instantiate a struct from a type name and field initializers.
    fn instantiate_struct(&self, type_name: &str, inits: &[FieldInit]) -> Result<Value, String> {
        let struct_type = self.struct_types.get(type_name)
            .ok_or_else(|| format!("unknown struct type: {}", type_name))?
            .clone();

        let mut fields = HashMap::new();
        for fd in &struct_type.fields {
            // Look for an initializer
            let init_val = inits.iter()
                .find(|fi| fi.name == fd.name)
                .map(|fi| self.eval_expr(&fi.value));

            let value = match init_val {
                Some(Ok(v)) => v,
                Some(Err(e)) => return Err(e),
                None => {
                    // Use default value
                    match &fd.default {
                        Some(def_expr) => self.eval_expr(def_expr)?,
                        None => Value::Unit,
                    }
                }
            };
            fields.insert(fd.name.clone(), value);
        }

        Ok(Value::Struct { type_name: type_name.to_string(), fields })
    }

    /// Execute all rules in priority order (highest first, then declaration order).
    fn execute_rules(&mut self) -> Result<(), String> {
        // Sort by priority descending; stable sort preserves declaration order for ties
        let mut sorted_rules: Vec<&RuleDecl> = self.rules.iter().collect();
        sorted_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        for rule in sorted_rules {
            let condition_met = self.eval_condition(&rule.condition, &self.variables)?;
            if condition_met {
                // Execute assignment: target.field = value
                let _target_val = self.eval_expr(&rule.target)?;
                let value_val = self.eval_expr(&rule.value)?;

                // We need to mutate the entity in variables
                // The target is an Ident (entity name), field is the field name
                if let Expr::Ident(name) = &rule.target {
                    let entity = self.variables.get_mut(name)
                        .ok_or_else(|| format!("rule target '{}' not found", name))?;
                    entity.set_field(&rule.field, value_val)?;
                }
            }
        }
        Ok(())
    }

    /// Evaluate a rule condition.
    fn eval_condition(&self, cond: &Condition, env: &HashMap<String, Value>) -> Result<bool, String> {
        match cond {
            Condition::Contains { left, right } => {
                let lv = self.eval_expr_with_env(left, env)?;
                let rv = self.eval_expr_with_env(right, env)?;
                let ls = match &lv {
                    Value::String(s) => s.clone(),
                    other => return Err(format!("contains: left must be String, got {}", other.type_name())),
                };
                let rs = match &rv {
                    Value::String(s) => s.clone(),
                    other => return Err(format!("contains: right must be String, got {}", other.type_name())),
                };
                Ok(ls.contains(&rs))
            }
            Condition::Compare { left, op, right } => {
                let lv = self.eval_expr_with_env(left, env)?;
                let rv = self.eval_expr_with_env(right, env)?;
                let lf = lv.as_float()?;
                let rf = rv.as_float()?;
                Ok(match op {
                    CompareOp::Gt => lf > rf,
                    CompareOp::Lt => lf < rf,
                    CompareOp::Ge => lf >= rf,
                    CompareOp::Le => lf <= rf,
                    CompareOp::Eq => lf == rf,
                })
            }
        }
    }

    /// Execute a flow: evaluate source, thread through pipeline steps.
    fn run_flow(&mut self, flow: &FlowDecl) -> Result<String, String> {
        // Register branch definitions for this flow
        self.branch_defs.clear();
        for (step_name, branches) in &flow.branch_defs {
            self.branch_defs.insert(step_name.clone(), branches.clone());
        }

        let mut current = self.eval_expr(&flow.source)?;

        for step_name in &flow.pipeline {
            current = self.run_flow_step(step_name, current)?;
        }

        Ok(format!("{}", current))
    }

    /// Execute a single flow step: check branch_defs first, else invoke as pattern/builtin.
    fn run_flow_step(&mut self, step_name: &str, current: Value) -> Result<Value, String> {
        if let Some(branches) = self.branch_defs.get(step_name).cloned() {
            // Step has branch definitions — evaluate conditions against current value
            for branch in &branches {
                if self.eval_branch_condition(&branch.condition, &current)? {
                    return self.invoke(&branch.target, vec![current]);
                }
            }
            Err(format!("no branch matched in step '{}'", step_name))
        } else {
            // No branch definitions — invoke as pattern/builtin directly
            self.invoke(step_name, vec![current])
        }
    }

    /// Evaluate a branch condition: `target.field op threshold`
    fn eval_branch_condition(&self, cond: &BranchCondition, current: &Value) -> Result<bool, String> {
        // The target in branch_condition is the flow input value (current)
        let field_val = current.get_field(&cond.field)
            .map_err(|e| format!("branch condition: {}", e))?
            .clone();
        let threshold = self.eval_expr(&cond.threshold)?;
        let fv = field_val.as_float()?;
        let tv = threshold.as_float()?;
        Ok(match cond.op {
            CompareOp::Gt => fv > tv,
            CompareOp::Lt => fv < tv,
            CompareOp::Ge => fv >= tv,
            CompareOp::Le => fv <= tv,
            CompareOp::Eq => fv == tv,
        })
    }

    /// Invoke a pattern or built-in by name with given arguments.
    fn invoke(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        // Check recall (memory) first — it's a built-in with memory access
        if name == "recall" {
            return self.invoke_recall(args);
        }

        // Check Entity Store operations
        if name == "find" {
            return self.invoke_find(&args);
        }
        if name == "count" {
            return self.invoke_count(&args);
        }

        // Check learnable patterns
        if let Some(learnable) = self.learnable_patterns.get(name).cloned() {
            let collapsed_args = self.collapse_args(&learnable.params, &args);
            return self.invoke_learnable_with_env(&learnable, &collapsed_args);
        }

        // Check builtins
        if let Some(builtin_fn) = self.builtins.get(name) {
            return builtin_fn(&args);
        }

        // Look up compiled pattern
        let pattern = match self.patterns.get(name) {
            Some(p) => p.clone(),
            None => return Err(format!("undefined pattern or builtin: {}", name)),
        };

        if args.len() != pattern.params.len() {
            return Err(format!(
                "pattern {} expects {} arguments, got {}",
                name,
                pattern.params.len(),
                args.len()
            ));
        }

        // Bind parameters with Fluid collapse
        let (local_env, input_confidence) = self.bind_and_collapse(&pattern.params, &args)?;

        let result = self.eval_statements(&pattern.body, &local_env)?;

        // Confidence propagation: if any input was Fluid (confidence < 1.0),
        // wrap the result in a Fluid value with min(input confidences).
        if input_confidence < 1.0 {
            Ok(Value::Fluid(vec![FluidValueVariant {
                type_name: result.type_name().to_string(),
                value: result,
                confidence: input_confidence,
            }]))
        } else {
            Ok(result)
        }
    }

    /// Query the Entity Store: find the first entity matching (type, field, op, threshold).
    /// Returns a clone of the entity's current value from `variables`.
    /// Arguments: [type_name: String, field: String, op: String, threshold: Float]
    fn invoke_find(&self, args: &[Value]) -> Result<Value, String> {
        if args.len() < 4 {
            return Err("find() requires 4 arguments: find(type, field, op, threshold)".to_string());
        }
        let type_name = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Err("find() first argument must be String (type name)".to_string()),
        };
        let field = match &args[1] {
            Value::String(s) => s.clone(),
            _ => return Err("find() second argument must be String (field name)".to_string()),
        };
        let op = match &args[2] {
            Value::String(s) => s.clone(),
            _ => return Err("find() third argument must be String (operator: gt, lt, ge, le, eq)".to_string()),
        };
        let threshold = args[3].as_float()
            .map_err(|_| "find() fourth argument must be Float (threshold)".to_string())?;

        let records = match self.entity_store.get(&type_name) {
            Some(r) => r,
            None => return Ok(Value::Unit), // No entities of this type
        };

        for record in records {
            let value = match self.variables.get(&record.id) {
                Some(v) => v,
                None => continue,
            };
            let field_val = match value.get_field(&field) {
                Ok(v) => v,
                Err(_) => continue, // Field not on this entity
            };
            let fv = match field_val.as_float() {
                Ok(f) => f,
                Err(_) => continue, // Field not numeric
            };
            let matches = match op.as_str() {
                "gt" => fv > threshold,
                "lt" => fv < threshold,
                "ge" => fv >= threshold,
                "le" => fv <= threshold,
                "eq" => fv == threshold,
                _ => return Err(format!("find() unknown operator: {}", op)),
            };
            if matches {
                return Ok(value.clone());
            }
        }

        Ok(Value::Unit) // No match — soft failure
    }

    /// Count entities in the Store by type name.
    /// Arguments: [type_name: String]
    fn invoke_count(&self, args: &[Value]) -> Result<Value, String> {
        if args.is_empty() {
            return Err("count() requires 1 argument (type name)".to_string());
        }
        let type_name = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Err("count() argument must be String (type name)".to_string()),
        };
        let count = self.entity_store.get(&type_name).map(|r| r.len()).unwrap_or(0);
        Ok(Value::Float(count as f64))
    }

    /// Invoke a learnable pattern using pre-collapsed arguments.
    fn invoke_learnable_with_env(&self, learnable: &CompiledLearnable, args: &[Value]) -> Result<Value, String> {
        // Build input string from arguments
        let input_parts: Vec<String> = args.iter().map(|a| format!("{}", a)).collect();
        let input = input_parts.join(", ");

        // Check few-shot examples first (exact match → cache hit)
        for (example_input, example_output) in &learnable.few_shot {
            if input == *example_input {
                return Ok(Value::String(example_output.clone()));
            }
        }

        // No few-shot match — call LLM backend
        let backend = llm::create_llm_backend();
        let response = backend.call(&learnable.prompt, &input)?;

        Ok(Value::String(response))
    }

    /// Recall from memory: find best matching entry by semantic similarity + decay.
    /// Uses the embedding backend for cosine similarity (Phase 2.2: concept-group vectors,
    /// Phase 2.3: real embeddings via PyO3).
    /// Returns the highest-activation entry above min_confidence.
    fn invoke_recall(&self, args: Vec<Value>) -> Result<Value, String> {
        if args.is_empty() {
            return Err("recall() requires at least 1 argument (query string)".to_string());
        }

        let query = match &args[0] {
            Value::String(s) => s.clone(),
            other => return Err(format!("recall() expected String argument, got {}", other.type_name())),
        };

        let min_confidence = if args.len() > 1 {
            args[1].as_float().unwrap_or(0.0)
        } else {
            0.0
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        // Find best matching entry: semantic similarity * activation after decay
        let mut best_match: Option<&MemoryEntry> = None;
        let mut best_score: f64 = 0.0;

        for entry in &self.memory {
            // Semantic similarity via embedding backend
            let similarity = self.embedding.similarity(&query, &entry.value);
            if similarity <= 0.0 {
                continue;
            }

            // Apply decay: activation = priority * exp(-decay_rate * age_in_days)
            let age_days = ((now - entry.timestamp).max(0) as f64) / 86400.0;
            let activation = entry.priority * (-entry.decay_rate * age_days).exp();

            // Combined score: geometric mean of similarity and activation
            let score = similarity * activation;

            if score > best_score && score >= min_confidence {
                best_score = score;
                best_match = Some(entry);
            }
        }

        match best_match {
            Some(entry) => Ok(Value::String(entry.value.clone())),
            None => Ok(Value::String(String::new())), // No match found — return empty (soft-failure)
        }
    }

    fn eval_statements(
        &self,
        stmts: &[Statement],
        env: &HashMap<String, Value>,
    ) -> Result<Value, String> {
        for stmt in stmts {
            match stmt {
                Statement::Return(expr) => return self.eval_expr_with_env(expr, env),
            }
        }
        Ok(Value::Unit)
    }

    /// Evaluate an expression in the global scope.
    fn eval_expr(&self, expr: &Expr) -> Result<Value, String> {
        self.eval_expr_with_env(expr, &self.variables)
    }

    /// Evaluate an expression with a given environment.
    fn eval_expr_with_env(
        &self,
        expr: &Expr,
        env: &HashMap<String, Value>,
    ) -> Result<Value, String> {
        match expr {
            Expr::StringLit(s) => Ok(Value::String(s.clone())),
            Expr::FloatLit(f) => Ok(Value::Float(*f)),
            Expr::Ident(name) => env
                .get(name)
                .cloned()
                .ok_or_else(|| format!("undefined variable: {}", name)),
            Expr::FieldAccess(base, field) => {
                let base_val = self.eval_expr_with_env(base, env)?;
                base_val.get_field(field).cloned()
            }
            Expr::FnCall(name, args) => {
                let mut eval_args = Vec::new();
                for arg in args {
                    eval_args.push(self.eval_expr_with_env(arg, env)?);
                }

                // Check recall (memory) first
                if name == "recall" {
                    return self.invoke_recall(eval_args);
                }

                // Check Entity Store operations
                if name == "find" {
                    return self.invoke_find(&eval_args);
                }
                if name == "count" {
                    return self.invoke_count(&eval_args);
                }

                // Check learnable patterns first
                if let Some(learnable) = self.learnable_patterns.get(name).cloned() {
                    let collapsed_args = self.collapse_args(&learnable.params, &eval_args);
                    return self.invoke_learnable_with_env(&learnable, &collapsed_args);
                }

                // Check builtins
                if let Some(builtin_fn) = self.builtins.get(name) {
                    return builtin_fn(&eval_args);
                }

                // Look up compiled pattern
                let pattern = match self.patterns.get(name) {
                    Some(p) => p.clone(),
                    None => return Err(format!("undefined pattern or builtin: {}", name)),
                };

                if eval_args.len() != pattern.params.len() {
                    return Err(format!(
                        "pattern {} expects {} arguments, got {}",
                        name,
                        pattern.params.len(),
                        eval_args.len()
                    ));
                }

                // Bind parameters with Fluid collapse
                let (local_env, input_confidence) = self.bind_and_collapse(&pattern.params, &eval_args)?;
                let result = self.eval_statements(&pattern.body, &local_env)?;

                // Confidence propagation: wrap result if any input was Fluid
                if input_confidence < 1.0 {
                    Ok(Value::Fluid(vec![FluidValueVariant {
                        type_name: result.type_name().to_string(),
                        value: result,
                        confidence: input_confidence,
                    }]))
                } else {
                    Ok(result)
                }
            }
            Expr::BinaryOp(left, op, right) => {
                let l = self.eval_expr_with_env(left, env)?;
                let r = self.eval_expr_with_env(right, env)?;
                self.eval_binop(l, *op, r)
            }
        }
    }

    /// Collapse threshold for Fluid values (0.0..1.0).
    /// When a Fluid value is used in a typed context, the matching variant's
    /// confidence must be >= this threshold to collapse successfully.
    /// Below threshold → soft-failure (returns Unit).
    const COLLAPSE_THRESHOLD: f64 = 0.1;

    /// Collapse a Fluid value to a concrete type if needed.
    /// If the value is Fluid, finds the variant matching `required_type` with the
    /// highest confidence. If confidence >= threshold, returns the concrete value.
    /// Otherwise, returns Unit (soft-failure per soft-failure semantics).
    /// Non-Fluid values pass through unchanged.
    fn maybe_collapse(&self, value: &Value, required_type: &str) -> Result<Value, String> {
        let (collapsed, _conf) = self.collapse_with_confidence(value, required_type);
        Ok(collapsed)
    }

    /// Collapse a Fluid value and also return its confidence score.
    /// - Fluid with required_type == "Fluid": pass through unchanged, confidence = best variant
    /// - Fluid with matching variant: collapse to that variant, return its confidence
    /// - Fluid with no match or below threshold: Unit, confidence = variant's or 0.0
    /// - Non-Fluid: pass through, confidence = 1.0 (full certainty)
    fn collapse_with_confidence(&self, value: &Value, required_type: &str) -> (Value, f64) {
        match value {
            Value::Fluid(variants) if required_type == "Fluid" => {
                // Pass through Fluid values unchanged; confidence = best variant's
                let best_conf = variants.iter()
                    .map(|v| v.confidence)
                    .fold(0.0_f64, |a, b| a.max(b));
                (value.clone(), best_conf)
            }
            Value::Fluid(variants) => {
                // Find the best matching variant for the required type
                let best = variants.iter()
                    .filter(|v| v.type_name == required_type)
                    .max_by(|a, b| {
                        a.confidence.partial_cmp(&b.confidence)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });

                match best {
                    Some(variant) if variant.confidence >= Self::COLLAPSE_THRESHOLD => {
                        (variant.value.clone(), variant.confidence)
                    }
                    Some(variant) => {
                        // Confidence below threshold — soft failure
                        (Value::Unit, variant.confidence)
                    }
                    None => {
                        // No matching variant at all — soft failure
                        (Value::Unit, 0.0)
                    }
                }
            }
            other => (other.clone(), 1.0),
        }
    }

    /// Bind arguments to pattern parameters, collapsing Fluid values as needed.
    /// For each (param, arg) pair, if the arg is a Fluid and the param has a
    /// declared type, collapse the Fluid to that type.
    /// Also returns the minimum confidence across all bound arguments.
    fn bind_and_collapse(
        &self,
        params: &[Param],
        args: &[Value],
    ) -> Result<(HashMap<String, Value>, f64), String> {
        let mut local_env = HashMap::new();
        let mut min_confidence = 1.0_f64;
        for (param, arg) in params.iter().zip(args.iter()) {
            let (collapsed, conf) = self.collapse_with_confidence(arg, &param.type_name);
            if conf < min_confidence {
                min_confidence = conf;
            }
            local_env.insert(param.name.clone(), collapsed);
        }
        Ok((local_env, min_confidence))
    }

    /// Collapse a list of arguments using parameter type annotations.
    /// Returns a new Vec of arguments where Fluid values have been collapsed
    /// to the type required by the corresponding parameter.
    fn collapse_args(&self, params: &[Param], args: &[Value]) -> Vec<Value> {
        params.iter().zip(args.iter())
            .map(|(p, a)| {
                self.maybe_collapse(a, &p.type_name).unwrap_or_else(|_| a.clone())
            })
            .collect()
    }

    fn eval_binop(&self, left: Value, op: BinOp, right: Value) -> Result<Value, String> {
        match (left, right) {
            (Value::String(a), Value::String(b)) => match op {
                BinOp::Add => Ok(Value::String(format!("{}{}", a, b))),
                _ => Err(format!("cannot apply {:?} to two Strings", op)),
            },
            (Value::Float(a), Value::Float(b)) => match op {
                BinOp::Add => Ok(Value::Float(a + b)),
                BinOp::Sub => Ok(Value::Float(a - b)),
                BinOp::Mul => Ok(Value::Float(a * b)),
                BinOp::Div => {
                    if b == 0.0 {
                        Err("division by zero".to_string())
                    } else {
                        Ok(Value::Float(a / b))
                    }
                }
            },
            (l, r) => Err(format!(
                "type mismatch in binary operation: {} {:?} {}",
                l.type_name(),
                op,
                r.type_name()
            )),
        }
    }
}
