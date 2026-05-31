// ── Tree-walking interpreter for METALOGOS M1+M2 ─────────────────────
// M1: entities, patterns, linear flow
// M2: struct entities, rules, branching flow, comparisons

use std::collections::HashMap;

use crate::ast::*;
use crate::builtins::Builtins;

/// Runtime value.
#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Float(f64),
    Struct {
        type_name: String,
        fields: HashMap<String, Value>,
    },
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
}

/// A compiled pattern ready for invocation.
#[derive(Clone)]
struct CompiledPattern {
    params: Vec<Param>,
    body: Vec<Statement>,
}

/// A registered struct type.
#[derive(Clone)]
struct StructType {
    #[allow(dead_code)]
    name: String,
    fields: Vec<FieldDecl>,
}

/// The interpreter holds all runtime state.
pub struct Interpreter {
    /// Global variable store.
    variables: HashMap<String, Value>,
    /// Struct type registry.
    struct_types: HashMap<String, StructType>,
    /// Compiled patterns.
    patterns: HashMap<String, CompiledPattern>,
    /// Rule declarations (stored for later execution).
    rules: Vec<RuleDecl>,
    /// Built-in function registry.
    builtins: Builtins,
    /// Flow branch definitions: step_name -> [Branch]
    branch_defs: HashMap<String, Vec<Branch>>,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            variables: HashMap::new(),
            struct_types: HashMap::new(),
            patterns: HashMap::new(),
            rules: Vec::new(),
            builtins: Builtins::new(),
            branch_defs: HashMap::new(),
        }
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
                    self.variables.insert(e.name.clone(), value);
                }
                Declaration::EntitySimple(e) => {
                    let value = self.eval_expr(&e.value)?;
                    self.variables.insert(e.name.clone(), value);
                }
                Declaration::Rule(r) => {
                    self.rules.push(r);
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
        // Check builtins first
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

        // Bind parameters
        let mut local_env = HashMap::new();
        for (param, arg) in pattern.params.iter().zip(args.iter()) {
            local_env.insert(param.name.clone(), arg.clone());
        }

        self.eval_statements(&pattern.body, &local_env)
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

                let mut local_env = HashMap::new();
                for (param, arg) in pattern.params.iter().zip(eval_args.iter()) {
                    local_env.insert(param.name.clone(), arg.clone());
                }
                self.eval_statements(&pattern.body, &local_env)
            }
            Expr::BinaryOp(left, op, right) => {
                let l = self.eval_expr_with_env(left, env)?;
                let r = self.eval_expr_with_env(right, env)?;
                self.eval_binop(l, *op, r)
            }
        }
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
