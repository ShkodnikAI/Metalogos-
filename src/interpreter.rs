// ── Tree-walking interpreter for METALOGOS M1 ─────────────────────────
// Evaluates AST declarations: entities are stored, patterns compiled,
// flows executed as linear pipelines.

use std::collections::HashMap;

use crate::ast::*;
use crate::builtins::Builtins;

/// Runtime value.
#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Float(f64),
    Unit,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::Float(n) => write!(f, "{}", n),
            Value::Unit => write!(f, "()"),
        }
    }
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::String(_) => "String",
            Value::Float(_) => "Float",
            Value::Unit => "Unit",
        }
    }
}

/// A compiled pattern ready for invocation.
#[derive(Clone)]
struct CompiledPattern {
    params: Vec<Param>,
    body: Vec<Statement>,
}

/// The interpreter holds all runtime state.
pub struct Interpreter {
    /// Global variable store (entity values).
    variables: HashMap<String, Value>,
    /// Compiled patterns.
    patterns: HashMap<String, CompiledPattern>,
    /// Built-in function registry.
    builtins: Builtins,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            variables: HashMap::new(),
            patterns: HashMap::new(),
            builtins: Builtins::new(),
        }
    }

    /// Run a complete .mlog program. Returns the final output value (if any flow was executed).
    pub fn run(&mut self, declarations: Vec<Declaration>) -> Result<Option<String>, String> {
        let mut output: Option<String> = None;

        for decl in declarations {
            match decl {
                Declaration::Entity(e) => {
                    let value = self.eval_expr(&e.value)?;
                    self.variables.insert(e.name.clone(), value);
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
                    output = Some(self.run_flow(&f)?);
                }
            }
        }

        Ok(output)
    }

    /// Execute a flow: evaluate source, thread through pipeline steps, return output.
    fn run_flow(&mut self, flow: &FlowDecl) -> Result<String, String> {
        let mut current = self.eval_expr(&flow.source)?;

        for step_name in &flow.pipeline {
            current = self.invoke(step_name, vec![current.clone()])?;
        }

        // The final value is the output
        Ok(format!("{}", current))
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

        // Execute body (M1: return statement)
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

    /// Evaluate an expression with a given environment (for pattern bodies).
    fn eval_expr_with_env(
        &self,
        expr: &Expr,
        env: &HashMap<String, Value>,
    ) -> Result<Value, String> {
        match expr {
            Expr::StringLit(s) => Ok(Value::String(s.clone())),
            Expr::Ident(name) => env
                .get(name)
                .cloned()
                .ok_or_else(|| format!("undefined variable: {}", name)),
            Expr::FnCall(name, args) => {
                let mut eval_args = Vec::new();
                for arg in args {
                    eval_args.push(self.eval_expr_with_env(arg, env)?);
                }

                // Check builtins
                if let Some(builtin_fn) = self.builtins.get(name) {
                    return builtin_fn(&eval_args);
                }

                // Look up compiled pattern (read-only)
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
