// ── METALOGOS Stack VM — Phase 4.1 ─────────────────────────────
//
// Executes a compiled bytecode Program. The VM maintains:
//   - A value stack (operands for instructions)
//   - A call stack (frames for pattern invocations)
//   - Global variables (slots)
//   - Pattern table (compiled user-defined functions)
//   - Learnable table (LLM-backed patterns)
//   - Memory store (memorize/recall)
//   - Knowledge graph relations
//   - Mutate log (messages from mutate declarations)
//
// Design: a single main loop that dispatches on the current instruction.
// Function calls push a new frame; Return pops back.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ast::CompareOp as AstCompareOp;
use crate::builtins::Builtins;
use crate::bytecode::*;
use crate::interpreter::{FluidValueVariant, Value};
use crate::llm;

/// The METALOGOS stack-based virtual machine.
pub struct Vm {
    /// Global variable slots.
    globals: Vec<Value>,
    /// Global variable names (index = slot).
    global_names: Vec<String>,
    /// Pattern table: index → CompiledFn (modified at runtime by RegisterPattern).
    patterns: Vec<CompiledFn>,
    /// Learnable pattern table: index → (info, few_shot, original_few_shot).
    learnables: Vec<(CompiledLearnableInfo, Vec<(String, String)>)>,
    /// Built-in function registry.
    builtins: Builtins,
    /// Builtin name lookup table (index → name).
    builtin_names: Vec<String>,
    /// Memory store.
    memory: Vec<VmMemoryEntry>,
    /// Knowledge graph relations.
    relations: Vec<VmRelation>,
    /// Rule table (from program.rules).
    rules: Vec<CompiledRule>,
    /// Skill index declarations (for resolve_skill_index).
    skill_indices: Vec<CompiledSkillIndex>,
    /// Database connection (opened from program.db_url if present).
    db_conn: Option<rusqlite::Connection>,
    /// Mutate log messages.
    mutate_log: Vec<String>,
    /// Audit log entries (Наряд №41 Block 2: parity with interpreter).
    audit_log: Mutex<Vec<String>>,
    /// ADR-0089: Propagated confidence from Fluid collapse through pattern calls.
    propagated_confidence: f64,
    /// Collections loaded flag (for map/filter/reduce).
    collections_loaded: bool,
    // ── Server context (per-request, set before execute_route_code) ──
    /// Parsed JSON request body (injected by server before route execution).
    server_json_body: Option<Value>,
    /// Query string parameters (injected by server before route execution).
    server_query_params: Option<std::collections::HashMap<String, String>>,
    /// User roles for RBAC (injected by server before route execution).
    server_user_roles: Vec<String>,
}

/// Collapse threshold for Fluid values (matches interpreter).
const COLLAPSE_THRESHOLD: f64 = 0.1;

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

impl Vm {
    /// Create a new VM with empty state.
    pub fn new() -> Self {
        let builtin_names = crate::builtins::builtin_names();

        Vm {
            globals: Vec::new(),
            global_names: Vec::new(),
            patterns: Vec::new(),
            learnables: Vec::new(),
            builtins: Builtins::new(),
            builtin_names,
            memory: Vec::new(),
            relations: Vec::new(),
            rules: Vec::new(),
            skill_indices: Vec::new(),
            db_conn: None,
            mutate_log: Vec::new(),
            audit_log: Mutex::new(Vec::new()),
            propagated_confidence: 1.0,
            collections_loaded: false,
            server_json_body: None,
            server_query_params: None,
            server_user_roles: Vec::new(),
        }
    }

    /// Execute a compiled program. Returns the flow output (if any),
    /// with mutate log messages prepended if present.
    pub fn run(&mut self, program: Program) -> Result<Option<String>, String> {
        self.load_program(&program)?;
        self.execute_main_code(&program)
    }

    /// Load program state without executing main_code.
    /// Initializes globals, patterns, learnables, rules, skill_indices,
    /// and database connection. Used by server backend to set up VM state
    /// per request without re-executing flows (Наряд №40).
    pub fn load_program(&mut self, program: &Program) -> Result<(), String> {
        // Initialize globals
        self.globals = vec![Value::Unit; program.globals.len()];
        self.global_names = program.globals.clone();
        self.collections_loaded = program.collections_loaded;

        // Sort rules by priority descending (matches interpreter semantics)
        let mut rules = program.rules.clone();
        rules.sort_by_key(|b| std::cmp::Reverse(b.priority));
        self.rules = rules;
        self.skill_indices = program.skill_indices.clone();

        // Open database connection if URL is specified
        self.db_conn = program.db_url.as_ref().and_then(|url| {
            let conn = if url == "sqlite::memory:" {
                rusqlite::Connection::open_in_memory()
            } else if url.starts_with("sqlite:") {
                let path = url.trim_start_matches("sqlite:");
                rusqlite::Connection::open(path)
            } else {
                return None;
            };
            match conn {
                Ok(c) => {
                    let _ = c.execute_batch("PRAGMA journal_mode=WAL;");
                    eprintln!("[vm/db] Connected: {}", url);
                    Some(c)
                }
                Err(e) => {
                    eprintln!("[vm/db] Failed to connect to '{}': {}", url, e);
                    None
                }
            }
        });

        // Execute schema DDL statements (CREATE TABLE IF NOT EXISTS)
        if let Some(conn) = self.db_conn.as_mut() {
            for ddl in &program.schema_ddl {
                if let Err(e) = conn.execute_batch(ddl) {
                    eprintln!("[vm/db] DDL error: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Execute main_code (the top-level instruction sequence).
    /// Called by `run()` after `load_program()`.
    fn execute_main_code(&mut self, program: &Program) -> Result<Option<String>, String> {
        let mut stack: Vec<Value> = Vec::new();
        let mut call_stack: Vec<CallFrame> = Vec::new();
        let mut ip = 0;
        let mut flow_output: Option<String> = None;
        let code = &program.main_code;

        while ip < code.len() {
            let instr = &code[ip];
            match instr {
                // ── Constants & Variables ─────────────────────
                Instruction::Const(v) => {
                    stack.push(v.clone());
                    ip += 1;
                }
                Instruction::LoadGlobal(slot) => {
                    let val = self.globals.get(*slot).cloned().unwrap_or(Value::Unit);
                    stack.push(val);
                    ip += 1;
                }
                Instruction::LoadGlobalByName(name) => {
                    // Search globals by name
                    let val = program
                        .globals
                        .iter()
                        .position(|n| n == name)
                        .and_then(|slot| self.globals.get(slot).cloned())
                        .unwrap_or(Value::Unit);
                    stack.push(val);
                    ip += 1;
                }
                Instruction::StoreGlobal(slot) => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    if *slot < self.globals.len() {
                        self.globals[*slot] = val;
                    }
                    ip += 1;
                }
                Instruction::LoadLocal(slot) => {
                    // Local variables are stored on the stack, below the base pointer
                    let bp = call_stack.last().map(|f| f.base_bp).unwrap_or(0);
                    let idx = bp + slot;
                    let val = stack.get(idx).cloned().unwrap_or(Value::Unit);
                    stack.push(val);
                    ip += 1;
                }
                Instruction::StoreLocal(slot) => {
                    let bp = call_stack.last().map(|f| f.base_bp).unwrap_or(0);
                    let idx = bp + slot;
                    let val = stack.pop().unwrap_or(Value::Unit);
                    if idx >= stack.len() {
                        stack.resize(idx + 1, Value::Unit);
                    }
                    stack[idx] = val;
                    ip += 1;
                }

                // ── Registration ──────────────────────────────
                Instruction::RegisterPattern(fn_def) => {
                    self.patterns.push(fn_def.clone());
                    ip += 1;
                }
                Instruction::RegisterLearnable(info) => {
                    self.learnables.push((info.clone(), Vec::new()));
                    ip += 1;
                }

                // ── Function Calls ────────────────────────────
                Instruction::CallBuiltin(idx, arity) => {
                    let name = self
                        .builtin_names
                        .get(*idx)
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string());
                    let mut args = Vec::new();
                    for _ in 0..*arity {
                        args.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }
                    // Problem B: map(list, "pattern_name") — needs pattern table access
                    if name == "map" {
                        if let Ok(result) = self.vm_map(&args, program) {
                            stack.push(result);
                            ip += 1;
                            continue;
                        }
                    }
                    let result = self.call_builtin(&name, &args)?;
                    stack.push(result);
                    ip += 1;
                }
                Instruction::CallPattern(idx, arity) => {
                    let pattern = self
                        .patterns
                        .get(*idx)
                        .ok_or_else(|| format!("VM: pattern index {} not found", idx))?
                        .clone();

                    // Check arity
                    if *arity != pattern.param_count {
                        return Err(format!(
                            "VM: pattern {} expects {} args, got {}",
                            pattern.name, pattern.param_count, arity
                        ));
                    }

                    // ── VM bytecode path ────────────────────────
                    // Pop arguments (in reverse) and bind as locals
                    let mut locals = Vec::new();
                    for _ in 0..*arity {
                        locals.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }

                    // Push call frame
                    let return_ip = ip + 1;
                    let base_bp = stack.len();
                    call_stack.push(CallFrame { return_ip, base_bp });

                    // Push locals onto stack
                    for local in locals {
                        stack.push(local);
                    }

                    // Switch to pattern code
                    let result =
                        self.execute_code(&pattern.code, &mut stack, &mut call_stack, program)?;
                    stack.push(result);
                    // IP already advanced by 1 (return_ip)
                    ip = return_ip;
                }
                Instruction::Return => {
                    let _return_val = stack.pop().unwrap_or(Value::Unit);
                    // Pop call frame and restore stack
                    if let Some(frame) = call_stack.pop() {
                        // Remove locals from stack
                        stack.truncate(frame.base_bp);
                        // We'll push the return value after returning
                        // (handled by the caller)
                    }
                    // Return the value to the execute_code caller
                    // Actually, this is tricky with the current design...
                    // Let me use a different approach — see execute_code below
                    ip += 1;
                }
                Instruction::LlmCall(idx, arity) => {
                    let mut args = Vec::new();
                    for _ in 0..*arity {
                        args.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }
                    let result = self.call_llm(*idx, &args)?;
                    stack.push(result);
                    ip += 1;
                }

                // ── Binary Operations ──────────────────────────
                Instruction::Add => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_binop(left, crate::ast::BinOp::Add, right)?);
                    ip += 1;
                }
                Instruction::Sub => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_binop(left, crate::ast::BinOp::Sub, right)?);
                    ip += 1;
                }
                Instruction::Mul => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_binop(left, crate::ast::BinOp::Mul, right)?);
                    ip += 1;
                }
                Instruction::Div => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_binop(left, crate::ast::BinOp::Div, right)?);
                    ip += 1;
                }
                Instruction::Contains => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    let result = self.eval_contains(left, right)?;
                    stack.push(result);
                    ip += 1;
                }
                Instruction::CmpGt => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_cmp(left, right, AstCompareOp::Gt));
                    ip += 1;
                }
                Instruction::CmpLt => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_cmp(left, right, AstCompareOp::Lt));
                    ip += 1;
                }
                Instruction::CmpGe => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_cmp(left, right, AstCompareOp::Ge));
                    ip += 1;
                }
                Instruction::CmpLe => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_cmp(left, right, AstCompareOp::Le));
                    ip += 1;
                }
                Instruction::CmpEq => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_cmp(left, right, AstCompareOp::Eq));
                    ip += 1;
                }
                Instruction::CmpNe => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    let eq_result = self.eval_cmp(left, right, AstCompareOp::Eq);
                    match eq_result {
                        Value::Float(f) => {
                            stack.push(Value::Float(if f == 1.0 { 0.0 } else { 1.0 }))
                        }
                        _ => stack.push(eq_result),
                    }
                    ip += 1;
                }

                // ── Struct Operations ─────────────────────────
                Instruction::MakeStruct(type_name, field_names) => {
                    let mut fields = HashMap::new();
                    // Values are on stack in field order (first pushed = bottom)
                    // Pop in reverse to get correct order
                    let mut values = Vec::new();
                    for _ in 0..field_names.len() {
                        values.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }
                    for (name, val) in field_names.iter().zip(values.iter()) {
                        fields.insert(name.clone(), val.clone());
                    }
                    stack.push(Value::Struct {
                        type_name: type_name.clone(),
                        fields,
                    });
                    ip += 1;
                }
                Instruction::GetField(field) => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    let result = val.get_field(field).cloned().unwrap_or(Value::Unit);
                    stack.push(result);
                    ip += 1;
                }
                Instruction::IndexAccess => {
                    let idx = stack.pop().unwrap_or(Value::Unit);
                    let base = stack.pop().unwrap_or(Value::Unit);
                    let result = match (&base, &idx) {
                        (Value::List(items), Value::Float(f)) => {
                            let i = *f as isize;
                            if i < 0 {
                                let abs_i = items.len().wrapping_sub((-i) as usize);
                                items.get(abs_i).cloned().unwrap_or(Value::Unit)
                            } else {
                                items.get(i as usize).cloned().unwrap_or(Value::Unit)
                            }
                        }
                        (Value::Struct { fields, .. }, Value::String(key)) => {
                            fields.get(key).cloned().unwrap_or(Value::Unit)
                        }
                        _ => Value::Unit,
                    };
                    stack.push(result);
                    ip += 1;
                }

                // ── Fluid Types ───────────────────────────────
                Instruction::MakeFluid(count) => {
                    let mut variants: Vec<FluidValueVariant> = Vec::new();
                    // Stack has pairs: value, confidence (bottom to top)
                    // Pop count pairs
                    let mut pairs: Vec<(Value, Value)> = Vec::new();
                    for _ in 0..*count {
                        let confidence = stack.pop().unwrap_or(Value::Float(0.0));
                        let value = stack.pop().unwrap_or(Value::Unit);
                        pairs.insert(0, (value, confidence));
                    }
                    for (value, confidence) in pairs {
                        let conf_f = confidence.as_float().unwrap_or(0.0);
                        // Determine type name from value
                        let type_name = match &value {
                            Value::Float(_) => "Float",
                            Value::String(_) => "String",
                            _ => "Unit",
                        }
                        .to_string();
                        variants.push(FluidValueVariant {
                            type_name,
                            value,
                            confidence: conf_f,
                        });
                    }
                    stack.push(Value::Fluid(variants));
                    ip += 1;
                }

                // ── Control Flow ───────────────────────────────
                Instruction::Jump(target) => {
                    ip = *target;
                }
                Instruction::JumpIfNot(target) => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    if !is_truthy(&val) {
                        ip = *target;
                    } else {
                        ip += 1;
                    }
                }
                Instruction::JumpIfLow(threshold, target) => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    let below = match &val {
                        Value::Float(f) => *f < *threshold,
                        Value::Fluid(variants) => {
                            // Use the maximum confidence
                            let max_conf = variants
                                .iter()
                                .map(|v| v.confidence)
                                .fold(0.0_f64, f64::max);
                            max_conf < *threshold
                        }
                        _ => false,
                    };
                    if below {
                        ip = *target;
                    } else {
                        ip += 1;
                    }
                }

                // ── METALOGOS Memory ────────────────────────────
                Instruction::Collapse(required_type) => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    let collapsed = self.maybe_collapse(&val, required_type);
                    stack.push(collapsed);
                    ip += 1;
                }
                Instruction::Memorize(priority) => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    let value_str = match val {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    self.memory.push(VmMemoryEntry {
                        value: value_str,
                        priority: *priority,
                        timestamp: now,
                        decay_rate: 0.01,
                        mem_type: String::new(), // default: untyped
                    });
                    ip += 1;
                }
                Instruction::Recall => {
                    let query = stack.pop().unwrap_or(Value::Unit);
                    let query_str = match query {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let result = self.recall(&query_str, 0.0);
                    stack.push(Value::String(result));
                    ip += 1;
                }
                Instruction::Forget(days) => {
                    let query = stack.pop().unwrap_or(Value::Unit);
                    let query_str = match query {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    let cutoff = now - (days * 86400);
                    self.memory.retain(|entry| {
                        !(entry.value.contains(&query_str) && entry.timestamp < cutoff)
                    });
                    ip += 1;
                }

                // ── Adapt / Relate / Mutate ─────────────────────
                Instruction::Adapt(pattern_name) => {
                    let output_str = match stack.pop().unwrap_or(Value::Unit) {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let input_str = match stack.pop().unwrap_or(Value::Unit) {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    // Find the learnable by name and add the example
                    let mut found = false;
                    for (info, few_shot) in &mut self.learnables {
                        if info.name == *pattern_name {
                            few_shot.push((input_str.clone(), output_str.clone()));
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return Err(format!(
                            "VM adapt: learnable pattern '{}' not found",
                            pattern_name
                        ));
                    }
                    // Наряд №41 Block 2: audit parity with interpreter
                    self.push_audit(format!(
                        "[AUDIT] adapt {}: {} -> {}",
                        pattern_name, input_str, output_str
                    ));
                    ip += 1;
                }
                Instruction::Relate => {
                    let relation = match stack.pop().unwrap_or(Value::Unit) {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let to = match stack.pop().unwrap_or(Value::Unit) {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let from = match stack.pop().unwrap_or(Value::Unit) {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    self.relations.push(VmRelation {
                        from: from.clone(),
                        to: to.clone(),
                        relation: relation.clone(),
                    });
                    // Наряд №41 Block 2: audit parity with interpreter
                    self.push_audit(format!("[AUDIT] relate {} -[{}]-> {}", from, relation, to));
                    ip += 1;
                }
                Instruction::Mutate {
                    pattern_name,
                    example_count,
                    rollback_threshold,
                    rollback_op,
                } => {
                    // Pop example_count pairs of (input, output)
                    let mut new_examples = Vec::new();
                    for _ in 0..*example_count {
                        let output_str = match stack.pop().unwrap_or(Value::Unit) {
                            Value::String(s) => s,
                            other => format!("{}", other),
                        };
                        let input_str = match stack.pop().unwrap_or(Value::Unit) {
                            Value::String(s) => s,
                            other => format!("{}", other),
                        };
                        new_examples.push((input_str, output_str));
                    }

                    // Find the learnable and mutate
                    let msg = self.handle_mutate(
                        pattern_name,
                        new_examples,
                        *rollback_threshold,
                        *rollback_op,
                    )?;
                    self.mutate_log.push(msg.clone());
                    // Наряд №41 Block 2: audit parity with interpreter
                    self.push_audit(format!("[AUDIT] mutate {}: {}", pattern_name, msg));
                    ip += 1;
                }

                // ── Pipeline ───────────────────────────────────
                // New: FlowPipeline — pop source from stack (compiled via compile_expr)
                Instruction::FlowPipeline {
                    pipeline,
                    branch_defs,
                } => {
                    let source_val = stack.pop().unwrap_or(Value::Unit);

                    // Execute pipeline steps
                    let mut current = source_val;
                    for step_name in pipeline {
                        current = self.run_flow_step(step_name, current, branch_defs)?;
                    }

                    // The pipeline result is the flow output
                    let output_str = format!("{}", current);
                    flow_output = Some(output_str);
                    ip += 1;
                }

                // Legacy: FlowExec — load source from embedded expression
                Instruction::FlowExec {
                    source_expr,
                    pipeline,
                    branch_defs,
                } => {
                    // Load the source value (legacy path)
                    let source_val = match source_expr {
                        FlowExpr::GlobalSlot(slot) => {
                            self.globals.get(*slot).cloned().unwrap_or(Value::Unit)
                        }
                        FlowExpr::Ident(name) => program
                            .globals
                            .iter()
                            .position(|n| n == name)
                            .and_then(|slot| self.globals.get(slot).cloned())
                            .unwrap_or(Value::Unit),
                        FlowExpr::Const(v) => v.clone(),
                    };

                    // Execute pipeline steps
                    let mut current = source_val;
                    for step_name in pipeline {
                        current = self.run_flow_step(step_name, current, branch_defs)?;
                    }

                    // The pipeline result is the flow output
                    let output_str = format!("{}", current);
                    flow_output = Some(output_str);
                    ip += 1;
                }

                // ── Rule Engine ─────────────────────────────────
                Instruction::ExecuteRules => {
                    self.execute_rules()?;
                    ip += 1;
                }

                // ── Meta ───────────────────────────────────────
                Instruction::Halt => {
                    break;
                }

                // ── Collection / List instructions (Наряд №18, №21) ──
                // VM bytecode support for these is deferred; the tree-walking
                // interpreter handles Problem A/B collection builtins natively.
                Instruction::MakeList(count) => {
                    let mut items = Vec::new();
                    for _ in 0..*count {
                        items.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }
                    stack.push(Value::List(items));
                    ip += 1;
                }
                Instruction::ListLen => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    let len = match val {
                        Value::List(items) => items.len() as f64,
                        _ => 0.0,
                    };
                    stack.push(Value::Float(len));
                    ip += 1;
                }
                Instruction::Pop => {
                    stack.pop();
                    ip += 1;
                }
                Instruction::StartsWith => {
                    let needle = stack.pop().unwrap_or(Value::Unit);
                    let haystack = stack.pop().unwrap_or(Value::Unit);
                    let result = match (&haystack, &needle) {
                        (Value::String(h), Value::String(n)) => {
                            Value::Float(if h.starts_with(n.as_str()) { 1.0 } else { 0.0 })
                        }
                        _ => Value::Float(0.0),
                    };
                    stack.push(result);
                    ip += 1;
                }
            }
        }

        // Build final output with mutate log
        let mutate_log = std::mem::take(&mut self.mutate_log);
        if mutate_log.is_empty() {
            Ok(flow_output)
        } else {
            match flow_output {
                Some(flow) => {
                    let mut result = mutate_log.join("\n");
                    result.push('\n');
                    result.push_str(&flow);
                    Ok(Some(result))
                }
                None => Ok(Some(mutate_log.join("\n"))),
            }
        }
    }

    /// Execute a block of code (e.g., pattern body) and return the result.
    /// This handles the call stack and Return instructions internally.
    pub fn execute_code(
        &mut self,
        code: &[Instruction],
        stack: &mut Vec<Value>,
        call_stack: &mut Vec<CallFrame>,
        program: &Program,
    ) -> Result<Value, String> {
        let mut ip = 0;
        // Safety: prevent infinite loops (max iterations per 100 instructions)
        let max_iterations = code.len().max(1) * 100_000;
        let mut iterations = 0;
        while ip < code.len() {
            iterations += 1;
            if iterations > max_iterations {
                return Err(format!(
                    "VM execute_code: possible infinite loop ({} iterations, {} instructions)",
                    iterations,
                    code.len()
                ));
            }
            let instr = &code[ip];
            match instr {
                Instruction::LoadLocal(slot) => {
                    let bp = call_stack.last().map(|f| f.base_bp).unwrap_or(0);
                    let idx = bp + slot;
                    let val = stack.get(idx).cloned().unwrap_or(Value::Unit);
                    stack.push(val);
                    ip += 1;
                }
                Instruction::StoreLocal(slot) => {
                    let bp = call_stack.last().map(|f| f.base_bp).unwrap_or(0);
                    let idx = bp + slot;
                    let val = stack.pop().unwrap_or(Value::Unit);
                    // Ensure stack has space at idx
                    if idx >= stack.len() {
                        stack.resize(idx + 1, Value::Unit);
                    }
                    stack[idx] = val;
                    ip += 1;
                }
                Instruction::Const(v) => {
                    stack.push(v.clone());
                    ip += 1;
                }
                Instruction::LoadGlobal(slot) => {
                    let val = self.globals.get(*slot).cloned().unwrap_or(Value::Unit);
                    stack.push(val);
                    ip += 1;
                }
                Instruction::CallBuiltin(idx, arity) => {
                    let name = self
                        .builtin_names
                        .get(*idx)
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string());
                    let mut args = Vec::new();
                    for _ in 0..*arity {
                        args.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }
                    // Problem B: map(list, "pattern_name") — needs pattern table access
                    if name == "map" {
                        if let Ok(result) = self.vm_map(&args, program) {
                            stack.push(result);
                            ip += 1;
                            continue;
                        }
                    }
                    let result = self.call_builtin(&name, &args)?;
                    stack.push(result);
                    ip += 1;
                }
                Instruction::CallPattern(pidx, arity) => {
                    let pattern = self
                        .patterns
                        .get(*pidx)
                        .ok_or_else(|| format!("VM: pattern index {} not found", pidx))?
                        .clone();
                    if *arity != pattern.param_count {
                        return Err(format!(
                            "VM: pattern {} expects {} args, got {}",
                            pattern.name, pattern.param_count, arity
                        ));
                    }
                    let mut locals = Vec::new();
                    for _ in 0..*arity {
                        locals.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }
                    let base_bp = stack.len();
                    call_stack.push(CallFrame {
                        return_ip: ip + 1,
                        base_bp,
                    });
                    for local in locals {
                        stack.push(local);
                    }
                    let result = self.execute_code(&pattern.code, stack, call_stack, program)?;
                    call_stack.pop();
                    stack.push(result);
                    ip += 1;
                }
                Instruction::Add => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_binop(left, crate::ast::BinOp::Add, right)?);
                    ip += 1;
                }
                Instruction::Sub => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_binop(left, crate::ast::BinOp::Sub, right)?);
                    ip += 1;
                }
                Instruction::Mul => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_binop(left, crate::ast::BinOp::Mul, right)?);
                    ip += 1;
                }
                Instruction::Div => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_binop(left, crate::ast::BinOp::Div, right)?);
                    ip += 1;
                }
                Instruction::Return => {
                    return Ok(stack.pop().unwrap_or(Value::Unit));
                }
                // Phase 5.1: control flow in pattern bodies
                Instruction::Jump(target) => {
                    ip = *target;
                }
                Instruction::JumpIfNot(target) => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    if !is_truthy(&val) {
                        ip = *target;
                    } else {
                        ip += 1;
                    }
                }
                // Phase 5.1: comparison operators
                Instruction::CmpGt => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_cmp(left, right, AstCompareOp::Gt));
                    ip += 1;
                }
                Instruction::CmpLt => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_cmp(left, right, AstCompareOp::Lt));
                    ip += 1;
                }
                Instruction::CmpGe => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_cmp(left, right, AstCompareOp::Ge));
                    ip += 1;
                }
                Instruction::CmpLe => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_cmp(left, right, AstCompareOp::Le));
                    ip += 1;
                }
                Instruction::CmpEq => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_cmp(left, right, AstCompareOp::Eq));
                    ip += 1;
                }
                Instruction::CmpNe => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    let eq_result = self.eval_cmp(left, right, AstCompareOp::Eq);
                    match eq_result {
                        Value::Float(f) => {
                            stack.push(Value::Float(if f == 1.0 { 0.0 } else { 1.0 }))
                        }
                        _ => stack.push(eq_result),
                    }
                    ip += 1;
                }
                Instruction::GetField(field) => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    let result = val.get_field(field).cloned().unwrap_or(Value::Unit);
                    stack.push(result);
                    ip += 1;
                }
                Instruction::LoadGlobalByName(name) => {
                    let val = program
                        .globals
                        .iter()
                        .position(|n| n == name)
                        .and_then(|slot| self.globals.get(slot).cloned())
                        .unwrap_or(Value::Unit);
                    stack.push(val);
                    ip += 1;
                }
                // Problem B: IndexAccess was missing from execute_code.
                Instruction::IndexAccess => {
                    let idx = stack.pop().unwrap_or(Value::Unit);
                    let base = stack.pop().unwrap_or(Value::Unit);
                    let result = match (&base, &idx) {
                        (Value::List(items), Value::Float(f)) => {
                            let i = *f as isize;
                            if i < 0 {
                                let abs_i = items.len().wrapping_sub((-i) as usize);
                                items.get(abs_i).cloned().unwrap_or(Value::Unit)
                            } else {
                                items.get(i as usize).cloned().unwrap_or(Value::Unit)
                            }
                        }
                        (Value::Struct { fields, .. }, Value::String(key)) => {
                            fields.get(key).cloned().unwrap_or(Value::Unit)
                        }
                        _ => Value::Unit,
                    };
                    stack.push(result);
                    ip += 1;
                }
                Instruction::Collapse(required_type) => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    let collapsed = self.maybe_collapse(&val, required_type);
                    stack.push(collapsed);
                    ip += 1;
                }
                Instruction::LlmCall(idx, arity) => {
                    let mut args = Vec::new();
                    for _ in 0..*arity {
                        args.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }
                    let result = self.call_llm(*idx, &args)?;
                    stack.push(result);
                    ip += 1;
                }
                Instruction::Recall => {
                    let query = stack.pop().unwrap_or(Value::Unit);
                    let query_str = match query {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let result = self.recall(&query_str, 0.0);
                    stack.push(Value::String(result));
                    ip += 1;
                }
                // ── Collection / List instructions (Наряд №34) ──
                Instruction::MakeList(count) => {
                    let mut items = Vec::new();
                    for _ in 0..*count {
                        items.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }
                    stack.push(Value::List(items));
                    ip += 1;
                }
                Instruction::ListLen => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    let len = match val {
                        Value::List(items) => items.len() as f64,
                        _ => 0.0,
                    };
                    stack.push(Value::Float(len));
                    ip += 1;
                }
                Instruction::Pop => {
                    stack.pop();
                    ip += 1;
                }
                Instruction::StartsWith => {
                    let needle = stack.pop().unwrap_or(Value::Unit);
                    let haystack = stack.pop().unwrap_or(Value::Unit);
                    let result = match (&haystack, &needle) {
                        (Value::String(h), Value::String(n)) => {
                            Value::Float(if h.starts_with(n.as_str()) { 1.0 } else { 0.0 })
                        }
                        _ => Value::Float(0.0),
                    };
                    stack.push(result);
                    ip += 1;
                }
                Instruction::MakeStruct(type_name, field_names) => {
                    let mut fields = HashMap::new();
                    let mut values = Vec::new();
                    for _ in 0..field_names.len() {
                        values.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }
                    for (name, val) in field_names.iter().zip(values.iter()) {
                        fields.insert(name.clone(), val.clone());
                    }
                    stack.push(Value::Struct {
                        type_name: type_name.clone(),
                        fields,
                    });
                    ip += 1;
                }
                Instruction::Contains => {
                    let needle = stack.pop().unwrap_or(Value::Unit);
                    let haystack = stack.pop().unwrap_or(Value::Unit);
                    let result = match (&haystack, &needle) {
                        (Value::String(h), Value::String(n)) => {
                            Value::Float(if h.contains(n.as_str()) { 1.0 } else { 0.0 })
                        }
                        (Value::List(items), _) => Value::Float(
                            if items
                                .iter()
                                .any(|v| format!("{}", v) == format!("{}", needle))
                            {
                                1.0
                            } else {
                                0.0
                            },
                        ),
                        _ => Value::Float(0.0),
                    };
                    stack.push(result);
                    ip += 1;
                }
                // For any unhandled instruction, skip
                _ => {
                    ip += 1;
                }
            }
        }
        Ok(stack.pop().unwrap_or(Value::Unit))
    }

    /// Call a built-in function by name.
    /// Problem B: map(list, "pattern_name") — applies a compiled pattern to each list element.
    /// Needed because map requires pattern table access (not available to regular builtins).
    fn vm_map(&mut self, args: &[Value], program: &Program) -> Result<Value, String> {
        if !self.collections_loaded {
            return Err("map() requires 'import std/collections'".to_string());
        }
        let list = match args.first() {
            Some(Value::List(items)) => items.clone(),
            _ => return Err("map() expects first argument to be a List".to_string()),
        };
        let pattern_name = match args.get(1) {
            Some(Value::String(s)) => s.clone(),
            _ => {
                return Err(
                    "map() expects second argument to be a pattern name (String)".to_string(),
                )
            }
        };
        let pattern = self
            .patterns
            .iter()
            .find(|p| p.name == pattern_name)
            .ok_or_else(|| format!("map(): pattern '{}' not found", pattern_name))?
            .clone();
        if pattern.param_count != 1 {
            return Err(format!(
                "map(): pattern '{}' must accept exactly 1 argument, got {}",
                pattern_name, pattern.param_count
            ));
        }
        let mut results = Vec::new();
        for item in &list {
            let mut item_stack: Vec<Value> = vec![item.clone()];
            let mut item_cs: Vec<CallFrame> = vec![CallFrame {
                return_ip: 0,
                base_bp: 0,
            }];
            let result =
                self.execute_code(&pattern.code, &mut item_stack, &mut item_cs, program)?;
            results.push(result);
        }
        Ok(Value::List(results))
    }

    fn call_builtin(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
        if name == "recall" {
            let query = match args.first() {
                Some(Value::String(s)) => s.clone(),
                other => return Err(format!("recall() expected String, got {:?}", other)),
            };
            let min_conf = if args.len() > 1 {
                args[1].as_float().unwrap_or(0.0)
            } else {
                0.0
            };
            return Ok(Value::String(self.recall(&query, min_conf)));
        }

        // find(entity_type, field, op, threshold) — entity store query
        // Searches globals for structs matching the type and field condition.
        if name == "find" {
            let type_name = match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => return Err("find() requires type name as first argument (String)".to_string()),
            };
            let field_name = match args.get(1) {
                Some(Value::String(s)) => s.clone(),
                _ => {
                    return Err("find() requires field name as second argument (String)".to_string())
                }
            };
            let op_str = match args.get(2) {
                Some(Value::String(s)) => s.clone(),
                _ => {
                    return Err(
                        "find() requires operator as third argument (String: gt/lt/ge/le/eq)"
                            .to_string(),
                    )
                }
            };
            let threshold = match args.get(3) {
                Some(Value::Float(f)) => *f,
                _ => return Err("find() requires threshold as fourth argument (Float)".to_string()),
            };
            for val in &self.globals {
                if let Value::Struct {
                    type_name: tn,
                    fields,
                } = val
                {
                    if tn == &type_name {
                        if let Some(field_val) = fields.get(&field_name) {
                            if let Ok(fv) = field_val.as_float() {
                                let matches = match op_str.as_str() {
                                    "gt" => fv > threshold,
                                    "lt" => fv < threshold,
                                    "ge" => fv >= threshold,
                                    "le" => fv <= threshold,
                                    "eq" => (fv - threshold).abs() < 1e-9,
                                    _ => {
                                        return Err(format!(
                                            "find(): unknown operator '{}'",
                                            op_str
                                        ))
                                    }
                                };
                                if matches {
                                    return Ok(val.clone());
                                }
                            }
                        }
                    }
                }
            }
            return Ok(Value::Unit);
        }

        // db_insert(table, struct) — insert a struct into a database table
        if name == "db_insert" {
            let table = match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => {
                    return Err(
                        "db_insert() expects first argument to be a table name (String)"
                            .to_string(),
                    )
                }
            };
            let fields = match args.get(1) {
                Some(Value::Struct { fields, .. }) => fields.clone(),
                _ => return Err("db_insert() expects second argument to be a Struct".to_string()),
            };
            let conn = self.db_conn.as_mut().ok_or_else(|| {
                "db_insert() error: no database connection. Declare db { url: \"sqlite::memory:\" } first.".to_string()
            })?;
            let col_names: Vec<String> = fields.keys().cloned().collect();
            let placeholders: Vec<String> = col_names.iter().map(|_| "?".to_string()).collect();
            let sql = format!(
                "INSERT INTO {} ({}) VALUES ({})",
                table,
                col_names.join(", "),
                placeholders.join(", ")
            );
            let params: Vec<Box<dyn rusqlite::types::ToSql>> = fields
                .values()
                .map(|v| match v {
                    Value::String(s) => Box::new(s.clone()) as Box<dyn rusqlite::types::ToSql>,
                    Value::Float(f) => Box::new(*f) as Box<dyn rusqlite::types::ToSql>,
                    Value::Bool(b) => Box::new(*b) as Box<dyn rusqlite::types::ToSql>,
                    Value::Unit => {
                        Box::new(Option::<String>::None) as Box<dyn rusqlite::types::ToSql>
                    }
                    other => Box::new(format!("{}", other)) as Box<dyn rusqlite::types::ToSql>,
                })
                .collect();
            let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                params.iter().map(|p| p.as_ref()).collect();
            conn.execute(&sql, param_refs.as_slice())
                .map_err(|e| format!("db_insert() SQL error: {}", e))?;
            let rowid: i64 = conn
                .query_row("SELECT last_insert_rowid()", [], |row| row.get(0))
                .unwrap_or(0);
            return Ok(Value::Float(rowid as f64));
        }

        // query_scalar(sql, params) — execute SELECT returning one scalar value
        if name == "query_scalar" {
            let sql = match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => return Err("query_scalar() expected String SQL".to_string()),
            };
            let params: Vec<String> = if args.len() > 1 {
                match &args[1] {
                    Value::List(items) => items
                        .iter()
                        .filter_map(|v| match v {
                            Value::String(s) => Some(s.clone()),
                            Value::Float(n) => Some(format!("{}", n)),
                            Value::Bool(b) => Some(format!("{}", b)),
                            _ => None,
                        })
                        .collect(),
                    _ => Vec::new(),
                }
            } else {
                Vec::new()
            };
            let conn = self
                .db_conn
                .as_ref()
                .ok_or_else(|| "query_scalar() error: no database connection.".to_string())?;
            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| format!("query_scalar() SQL error: {}", e))?;
            let mut rows = stmt
                .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                    row.get_ref(0).map(|v| match v {
                        rusqlite::types::ValueRef::Null => Value::Unit,
                        rusqlite::types::ValueRef::Integer(n) => Value::Float(n as f64),
                        rusqlite::types::ValueRef::Real(f) => Value::Float(f),
                        rusqlite::types::ValueRef::Text(s) => {
                            Value::String(String::from_utf8_lossy(s).to_string())
                        }
                        rusqlite::types::ValueRef::Blob(b) => {
                            Value::String(b.iter().map(|byte| format!("{:02x}", byte)).collect())
                        }
                    })
                })
                .map_err(|e| format!("query_scalar() execution error: {}", e))?;
            match rows.next() {
                Some(Ok(val)) => return Ok(val),
                Some(Err(e)) => return Err(format!("query_scalar() row error: {}", e)),
                None => return Ok(Value::Unit),
            }
        }

        // query(sql) / query(sql, params) — execute SELECT returning list of structs
        if name == "query" {
            let sql = match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => return Err("query() expected String SQL".to_string()),
            };
            let conn = self
                .db_conn
                .as_ref()
                .ok_or_else(|| "query() error: no database connection.".to_string())?;
            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| format!("query() SQL error: {}", e))?;
            let col_names: Vec<String> =
                stmt.column_names().iter().map(|s| s.to_string()).collect();
            let mut rows = stmt
                .query([])
                .map_err(|e| format!("query() execution error: {}", e))?;
            let mut results = Vec::new();
            while let Some(row) = rows
                .next()
                .map_err(|e| format!("query() row error: {}", e))?
            {
                let mut fields = std::collections::HashMap::new();
                for (i, col) in col_names.iter().enumerate() {
                    let val: rusqlite::types::ValueRef = row
                        .get_ref(i)
                        .map_err(|e| format!("query() column {} error: {}", col, e))?;
                    fields.insert(
                        col.clone(),
                        match val {
                            rusqlite::types::ValueRef::Null => Value::Unit,
                            rusqlite::types::ValueRef::Integer(n) => Value::Float(n as f64),
                            rusqlite::types::ValueRef::Real(f) => Value::Float(f),
                            rusqlite::types::ValueRef::Text(s) => {
                                Value::String(String::from_utf8_lossy(s).to_string())
                            }
                            rusqlite::types::ValueRef::Blob(b) => Value::String(
                                b.iter().map(|byte| format!("{:02x}", byte)).collect(),
                            ),
                        },
                    );
                }
                results.push(Value::Struct {
                    type_name: "Row".to_string(),
                    fields,
                });
            }
            return Ok(Value::List(results));
        }

        // db_execute(sql, params?) — execute SQL (INSERT/UPDATE/DELETE/DDL)
        if name == "db_execute" {
            let sql = match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => return Err("db_execute() expected String SQL".to_string()),
            };
            let params: Vec<String> = if args.len() > 1 {
                match &args[1] {
                    Value::List(items) => items
                        .iter()
                        .filter_map(|v| match v {
                            Value::String(s) => Some(s.clone()),
                            Value::Float(n) => Some(format!("{}", n)),
                            Value::Bool(b) => Some(format!("{}", b)),
                            _ => None,
                        })
                        .collect(),
                    _ => Vec::new(),
                }
            } else {
                Vec::new()
            };
            let conn = self
                .db_conn
                .as_ref()
                .ok_or_else(|| "db_execute() error: no database connection.".to_string())?;
            conn.execute(&sql, rusqlite::params_from_iter(params.iter()))
                .map_err(|e| format!("db_execute() SQL error: {}", e))?;
            return Ok(Value::Unit);
        }

        // resolve_skill_index(dept) — returns compiled skill index as Value::Struct
        if name == "resolve_skill_index" {
            let dept = match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => {
                    return Err(
                        "resolve_skill_index() expects a department name (String)".to_string()
                    )
                }
            };
            let idx = self
                .skill_indices
                .iter()
                .find(|si| si.name == dept)
                .ok_or_else(|| {
                    format!(
                        "resolve_skill_index(): no skill_index declared for '{}'",
                        dept
                    )
                })?;
            let mut fields = std::collections::HashMap::new();
            let tier1: Vec<Value> = idx
                .tiers
                .iter()
                .filter(|t| t.mode == "always")
                .flat_map(|t| t.skills.iter().map(|s| Value::String(s.clone())))
                .collect();
            fields.insert("tier1".to_string(), Value::List(tier1));
            for tier in &idx.tiers {
                if tier.mode == "when_matches" {
                    let rules: Vec<Value> = tier
                        .rules
                        .iter()
                        .map(|r| {
                            let mut f = std::collections::HashMap::new();
                            f.insert("skill".to_string(), Value::String(r.skill.clone()));
                            f.insert(
                                "triggers".to_string(),
                                Value::List(
                                    r.triggers
                                        .iter()
                                        .map(|t| Value::String(t.clone()))
                                        .collect(),
                                ),
                            );
                            Value::Struct {
                                type_name: "TriggerRule".to_string(),
                                fields: f,
                            }
                        })
                        .collect();
                    fields.insert(format!("tier{}", tier.level), Value::List(rules));
                }
            }
            if let Some(b) = idx.budget {
                fields.insert("budget".to_string(), Value::Float(b));
            }
            return Ok(Value::Struct {
                type_name: "SkillIndex".to_string(),
                fields,
            });
        }

        // ── Server-context builtins (Наряд №40: VM server backend) ──
        // These must be intercepted here because they need access to
        // per-request server context (query_params, json_body, user_roles)
        // that the generic builtin registry does not have.

        if name == "query_param" {
            let param_name = args
                .first()
                .and_then(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            if let Some(ref params) = self.server_query_params {
                if let Some(val) = params.get(&param_name) {
                    return Ok(Value::String(val.clone()));
                }
            }
            return Ok(Value::String(String::new()));
        }

        if name == "json_body" {
            if let Some(ref body) = self.server_json_body {
                return Ok(body.clone());
            }
            return Ok(Value::Struct {
                type_name: "JsonBody".to_string(),
                fields: std::collections::HashMap::new(),
            });
        }

        if name == "form_data" {
            return Ok(Value::Struct {
                type_name: "FormData".to_string(),
                fields: std::collections::HashMap::new(),
            });
        }

        if name == "require" {
            let role = args
                .first()
                .and_then(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            if self.server_user_roles.contains(&role) {
                return Ok(Value::Bool(true));
            }
            return Err(format!(
                "require('{}'): access denied — user has roles {:?}",
                role, self.server_user_roles
            ));
        }

        // ── Наряд №67: recipe_save — intercept to also memorize for recipe_search ──
        // recipe_save(name, description, skills, plan) builds a struct via the pure
        // builtin AND memorizes the description with type "recipe" so that
        // recipe_search can find it later.
        if name == "recipe_save" {
            let result = crate::builtins::office::recipes::builtin_recipe_save(args)?;
            if let Value::Struct { ref fields, .. } = result {
                let key = fields
                    .get("key")
                    .and_then(|v| match v {
                        Value::String(s) => Some(s.as_str()),
                        _ => None,
                    })
                    .unwrap_or("");
                let desc = args
                    .get(1)
                    .and_then(|v| match v {
                        Value::String(s) => Some(s.as_str()),
                        _ => None,
                    })
                    .unwrap_or("");
                if !desc.is_empty() && !key.is_empty() {
                    let mem_value = format!("__KVKEY:{}\n{}", key, desc);
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    self.memory.push(VmMemoryEntry {
                        value: mem_value,
                        priority: 0.8,
                        timestamp: now,
                        decay_rate: 0.01,
                        mem_type: "recipe".to_string(),
                    });
                }
            }
            return Ok(result);
        }

        // ── Наряд №67: recipe_search — semantic search via memory + kv_get ──
        // Searches VM memory for entries of type "recipe" whose value contains
        // all query words (token-level AND match, case-insensitive), then
        // fetches full recipe data from the shared KV store.
        if name == "recipe_search" {
            if args.is_empty() {
                return Err("recipe_search() requires at least 1 argument (query)".to_string());
            }
            let query = match args.first() {
                Some(Value::String(s)) => s.clone(),
                other => {
                    return Err(format!(
                        "recipe_search() expected String as first arg, got {:?}",
                        other
                    ))
                }
            };
            let k = if args.len() > 1 {
                args[1].as_float().unwrap_or(5.0) as usize
            } else {
                5
            };

            // Token-level AND matching: all query words must appear (case-insensitive)
            let query_lower = query.to_lowercase();
            let query_words: Vec<&str> = query_lower.split_whitespace().collect();

            let mut matches: Vec<String> = Vec::new();
            let mut seen_keys = std::collections::HashSet::new();
            for entry in &self.memory {
                if !entry.mem_type.is_empty() && entry.mem_type != "recipe" {
                    continue;
                }
                let val_lower = entry.value.to_lowercase();
                if query_words.iter().all(|w| val_lower.contains(w)) {
                    // Extract KV key from value format: "__KVKEY:<key>\n<description>"
                    let kv_key = entry
                        .value
                        .strip_prefix("__KVKEY:")
                        .and_then(|rest| rest.lines().next())
                        .unwrap_or("");
                    if kv_key.is_empty() || seen_keys.contains(kv_key) {
                        continue;
                    }
                    seen_keys.insert(kv_key.to_string());
                    matches.push(entry.value.clone());
                    if matches.len() >= k {
                        break;
                    }
                }
            }

            // Fetch full recipes from KV store
            let mut recipes: Vec<Value> = Vec::new();
            for mem_val in &matches {
                let kv_key = mem_val
                    .strip_prefix("__KVKEY:")
                    .and_then(|rest| rest.lines().next())
                    .unwrap_or("");
                if kv_key.is_empty() {
                    continue;
                }
                if let Some(recipe_json) = crate::builtins::memory::kv_get_raw(kv_key) {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&recipe_json) {
                        let name = parsed["name"].as_str().unwrap_or("").to_string();
                        let desc = parsed["description"].as_str().unwrap_or("").to_string();
                        recipes.push(crate::builtins::core::make_struct(
                            "RecipeResult",
                            vec![
                                ("name", Value::String(name)),
                                ("description", Value::String(desc)),
                                ("recipe_json", Value::String(recipe_json)),
                            ],
                        ));
                    }
                }
            }

            return Ok(Value::List(recipes));
        }

        if let Some(builtin_fn) = self.builtins.get(name) {
            return builtin_fn(args);
        }
        Err(format!("VM: undefined builtin: {}", name))
    }

    /// Call an LLM-backed learnable pattern.
    fn call_llm(&self, idx: usize, args: &[Value]) -> Result<Value, String> {
        let (info, few_shot) = self
            .learnables
            .get(idx)
            .ok_or_else(|| format!("VM: learnable index {} not found", idx))?;

        // Build input string
        let input_parts: Vec<String> = args.iter().map(|a| format!("{}", a)).collect();
        let input = input_parts.join(", ");

        // Check few-shot cache first
        for (example_input, example_output) in few_shot {
            if input == *example_input {
                return Ok(Value::String(example_output.clone()));
            }
        }

        // Build effective prompt with context prefix (matches interpreter)
        let effective_prompt = match &info.context_mode {
            CompiledContextMode::Literal(ctx) => {
                format!("{}\n{}", ctx, info.prompt)
            }
            CompiledContextMode::Auto => {
                // Use first arg value as recall query
                let query = args.first().map(|a| format!("{}", a)).unwrap_or_default();
                let facts = self.recall_top(&query, 5);
                if facts.is_empty() {
                    info.prompt.clone()
                } else {
                    let mut block = String::from("Relevant context:\n");
                    for fact in &facts {
                        block.push_str("- ");
                        block.push_str(fact);
                        block.push('\n');
                    }
                    format!("{}\n{}", block, info.prompt)
                }
            }
            CompiledContextMode::Recall(_param_name, limit) => {
                // Use first arg as recall query
                let query = args.first().map(|a| format!("{}", a)).unwrap_or_default();
                let facts = self.recall_top(&query, *limit);
                if facts.is_empty() {
                    info.prompt.clone()
                } else {
                    let mut block = String::from("Relevant context:\n");
                    for fact in &facts {
                        block.push_str("- ");
                        block.push_str(fact);
                        block.push('\n');
                    }
                    format!("{}\n{}", block, info.prompt)
                }
            }
            CompiledContextMode::None => info.prompt.clone(),
        };

        // Call LLM backend
        let backend = llm::create_llm_backend();
        let response = backend.call(&effective_prompt, &input)?;

        // Try to parse JSON response into Value::Struct
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response) {
            if let Some(obj) = json.as_object() {
                let mut fields = std::collections::HashMap::new();
                for (k, v) in obj {
                    fields.insert(k.clone(), Vm::json_to_value(v));
                }
                return Ok(Value::Struct {
                    type_name: "LlmResponse".to_string(),
                    fields,
                });
            }
        }

        Ok(Value::String(response))
    }

    /// Convert serde_json::Value to METALOGOS Value (for VM).
    fn json_to_value(json: &serde_json::Value) -> Value {
        match json {
            serde_json::Value::String(s) => Value::String(s.clone()),
            serde_json::Value::Number(n) => Value::Float(n.as_f64().unwrap_or(0.0)),
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Null => Value::Unit,
            serde_json::Value::Array(arr) => {
                Value::List(arr.iter().map(Vm::json_to_value).collect())
            }
            serde_json::Value::Object(obj) => {
                let mut fields = std::collections::HashMap::new();
                for (k, v) in obj {
                    fields.insert(k.clone(), Vm::json_to_value(v));
                }
                Value::Struct {
                    type_name: "Json".to_string(),
                    fields,
                }
            }
        }
    }

    /// Execute a flow step: check branches, then invoke pattern/builtin.
    fn run_flow_step(
        &mut self,
        step_name: &str,
        current: Value,
        branch_defs: &[(String, Vec<BranchDef>)],
    ) -> Result<Value, String> {
        // Check if step has branch definitions
        for (bd_step, branches) in branch_defs {
            if bd_step == step_name {
                for branch in branches {
                    if self.eval_branch_condition(branch, &current)? {
                        return self.invoke_step(&branch.target, vec![current]);
                    }
                }
                return Err(format!("VM: no branch matched in step '{}'", step_name));
            }
        }

        // No branch definitions — invoke as pattern/builtin
        self.invoke_step(step_name, vec![current])
    }

    /// Evaluate a branch condition against a value.
    fn eval_branch_condition(&self, branch: &BranchDef, current: &Value) -> Result<bool, String> {
        let field_val = current
            .get_field(&branch.condition_field)
            .map_err(|e| format!("branch condition: {}", e))?
            .clone();
        let fv = field_val.as_float()?;
        let tv = branch.condition_threshold.as_float()?;
        Ok(match branch.condition_op {
            ConditionOp::Gt => fv > tv,
            ConditionOp::Lt => fv < tv,
            ConditionOp::Ge => fv >= tv,
            ConditionOp::Le => fv <= tv,
            ConditionOp::Eq => fv == tv,
            ConditionOp::Ne => fv != tv,
        })
    }

    /// Invoke a pattern or builtin by name (used in flow pipeline).
    fn invoke_step(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        // Check learnables first
        for (i, (info, _)) in self.learnables.iter().enumerate() {
            if info.name == name {
                return self.call_llm(i, &args);
            }
        }

        // Check patterns
        for pattern in self.patterns.iter() {
            if pattern.name == name {
                if args.len() != pattern.param_count {
                    return Err(format!(
                        "VM: pattern {} expects {} args, got {}",
                        name,
                        pattern.param_count,
                        args.len()
                    ));
                }

                // ── VM bytecode path ───────────────────────────
                // Clone everything needed before mutable self borrow
                let param_types = pattern.param_types.clone();
                let code = pattern.code.clone();

                // ADR-0089: reset propagated confidence before collapse
                self.propagated_confidence = 1.0;
                // Collapse Fluid arguments to parameter types
                let collapsed_args: Vec<Value> = args
                    .iter()
                    .zip(param_types.iter())
                    .map(|(arg, param_type)| self.maybe_collapse(arg, param_type))
                    .collect();
                // Execute pattern body as bytecode
                let mut stack: Vec<Value> = Vec::new();
                let mut call_stack: Vec<CallFrame> = vec![CallFrame {
                    return_ip: 0,
                    base_bp: 0,
                }];
                let program = Program {
                    globals: Vec::new(),
                    patterns: Vec::new(),
                    learnables: Vec::new(),
                    rules: Vec::new(),
                    skill_indices: Vec::new(),
                    db_url: None,
                    schema_ddl: Vec::new(),
                    main_code: Vec::new(),
                    collections_loaded: false,
                };
                for arg in collapsed_args {
                    stack.push(arg);
                }
                let result = self.execute_code(&code, &mut stack, &mut call_stack, &program);
                // ADR-0089: wrap result as Fluid if confidence was propagated
                return Self::vm_wrap_with_confidence(result, self.propagated_confidence);
            }
        }

        // Check builtins
        self.call_builtin(name, &args)
    }

    /// Execute all registered rules with priority-ordered, first-wins semantics.
    /// ADR-0090: rules sorted by priority descending; stable sort preserves
    /// declaration order for ties. For each (entity, field) pair, only the
    /// first matching rule writes the field. Rules targeting different fields all fire.
    fn execute_rules(&mut self) -> Result<(), String> {
        // ADR-0090: sort by priority descending (stable sort keeps declaration order for ties)
        let mut sorted_rules: Vec<&CompiledRule> = self.rules.iter().collect();
        sorted_rules.sort_by_key(|b| std::cmp::Reverse(b.priority));

        // Track which (entity_name, field_name) pairs have already been written
        let mut written: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();

        for rule in sorted_rules {
            // Skip if this field was already written by a higher-priority rule
            if written.contains(&(rule.target_name.clone(), rule.field.clone())) {
                continue;
            }

            let condition_met = self.eval_rule_condition(&rule.condition)?;
            if condition_met {
                // Find the target entity by name in globals
                let target_slot = self
                    .global_names
                    .iter()
                    .position(|n| n == &rule.target_name);
                if let Some(slot) = target_slot {
                    let value = self.eval_rule_value(&rule.value_expr)?;
                    // Set field on the struct
                    if let Some(entity) = self.globals.get_mut(slot) {
                        let _ = entity.set_field(&rule.field, value);
                        // Mark this (entity, field) as written — first-wins
                        written.insert((rule.target_name.clone(), rule.field.clone()));
                    }
                }
            }
        }
        Ok(())
    }

    /// Evaluate a rule condition.
    fn eval_rule_condition(&self, cond: &RuleCondition) -> Result<bool, String> {
        match cond {
            RuleCondition::Contains { left, right } => {
                let ls = self.eval_rule_value(left)?;
                let rs = self.eval_rule_value(right)?;
                match (&ls, &rs) {
                    (Value::String(l), Value::String(r)) => Ok(l.contains(r)),
                    _ => Err("contains: both sides must be String".to_string()),
                }
            }
            RuleCondition::Compare { left, op, right } => {
                let lv = self.eval_rule_value(left)?;
                let rv = self.eval_rule_value(right)?;
                let lf = lv.as_float()?;
                let rf = rv.as_float()?;
                Ok(match op {
                    ConditionOp::Gt => lf > rf,
                    ConditionOp::Lt => lf < rf,
                    ConditionOp::Ge => lf >= rf,
                    ConditionOp::Le => lf <= rf,
                    ConditionOp::Eq => lf == rf,
                    ConditionOp::Ne => lf != rf,
                })
            }
        }
    }

    /// Evaluate a rule value expression to a runtime value.
    fn eval_rule_value(&self, expr: &RuleValueExpr) -> Result<Value, String> {
        match expr {
            RuleValueExpr::StringLit(s) => Ok(Value::String(s.clone())),
            RuleValueExpr::FloatLit(f) => Ok(Value::Float(*f)),
            RuleValueExpr::Ident(name) => {
                // Search globals by name
                let slot = self.global_names.iter().position(|n| n == name);
                match slot {
                    Some(s) => Ok(self.globals.get(s).cloned().unwrap_or(Value::Unit)),
                    None => Err(format!("VM rule: undefined variable '{}'", name)),
                }
            }
            RuleValueExpr::FieldAccess(entity_name, field) => {
                let slot = self.global_names.iter().position(|n| n == entity_name);
                match slot {
                    Some(s) => {
                        let entity = self.globals.get(s).cloned().unwrap_or(Value::Unit);
                        entity
                            .get_field(field)
                            .cloned()
                            .map_err(|e| format!("VM rule field access: {}", e))
                    }
                    None => Err(format!("VM rule: undefined entity '{}'", entity_name)),
                }
            }
        }
    }

    /// Handle a mutate declaration.
    fn handle_mutate(
        &mut self,
        pattern_name: &str,
        new_examples: Vec<(String, String)>,
        rollback_threshold: Option<f64>,
        rollback_op: Option<ConditionOp>,
    ) -> Result<String, String> {
        // Find the learnable
        for (info, few_shot) in self.learnables.iter_mut() {
            if info.name == pattern_name {
                let original = few_shot.clone();
                *few_shot = new_examples;

                // Mock accuracy (always 0.95 for MockLlm)
                let accuracy: f64 = 0.95;

                let kept = match (&rollback_op, &rollback_threshold) {
                    (Some(ConditionOp::Lt), Some(threshold)) => accuracy >= *threshold,
                    (Some(ConditionOp::Le), Some(threshold)) => accuracy > *threshold,
                    (Some(ConditionOp::Gt), Some(_)) | (Some(ConditionOp::Ge), Some(_)) => false,
                    (Some(ConditionOp::Eq), Some(threshold)) => (accuracy - threshold).abs() < 1e-9,
                    _ => true,
                };

                if kept {
                    return Ok(format!(
                        "[MUTATE] {}: accuracy={}, kept (>= {:.1})",
                        pattern_name,
                        accuracy,
                        rollback_threshold.unwrap_or(0.0)
                    ));
                } else {
                    *few_shot = original;
                    return Ok(format!(
                        "[MUTATE] {}: accuracy={}, rolled back (below {:.1})",
                        pattern_name,
                        accuracy,
                        rollback_threshold.unwrap_or(0.0)
                    ));
                }
            }
        }
        Err(format!(
            "VM mutate: learnable pattern '{}' not found",
            pattern_name
        ))
    }

    /// Recall from memory: find best matching entry by substring + decay.
    fn recall(&self, query: &str, min_confidence: f64) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut best_match: Option<&VmMemoryEntry> = None;
        let mut best_activation: f64 = 0.0;

        for entry in &self.memory {
            if !entry.value.contains(query) {
                continue;
            }
            let age_days = ((now - entry.timestamp).max(0) as f64) / 86400.0;
            let activation = entry.priority * (-entry.decay_rate * age_days).exp();
            if activation > best_activation && activation >= min_confidence {
                best_activation = activation;
                best_match = Some(entry);
            }
        }

        match best_match {
            Some(entry) => {
                let mut result = entry.value.clone();
                // Walk knowledge graph for related memories
                for rel in &self.relations {
                    if rel.from == entry.value {
                        result.push('\n');
                        result.push_str(&format!("[GRAPH] {} -> {}", rel.relation, rel.to));
                    } else if rel.to == entry.value {
                        result.push('\n');
                        result.push_str(&format!("[GRAPH] {} -> {}", rel.relation, rel.from));
                    }
                }
                result
            }
            None => String::new(),
        }
    }

    /// Recall up to `limit` memory entries matching query, sorted by activation.
    fn recall_top(&self, query: &str, limit: usize) -> Vec<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut scored: Vec<(String, f64)> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for entry in &self.memory {
            if !entry.value.contains(query) {
                continue;
            }
            if seen.contains(&entry.value) {
                continue;
            }
            seen.insert(entry.value.clone());
            let age_days = ((now - entry.timestamp).max(0) as f64) / 86400.0;
            let activation = entry.priority * (-entry.decay_rate * age_days).exp();
            scored.push((entry.value.clone(), activation));
        }
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(limit).map(|(v, _)| v).collect()
    }

    // ── Server context setters (per-request, for route execution) ──

    /// Set the parsed JSON request body (Наряд №40: VM server backend).
    pub fn set_server_json_body(&mut self, val: Value) {
        self.server_json_body = Some(val);
    }

    /// Set query string parameters (Наряд №40: VM server backend).
    pub fn set_server_query_params(&mut self, params: std::collections::HashMap<String, String>) {
        self.server_query_params = Some(params);
    }

    /// Set user roles for RBAC (Наряд №40: VM server backend).
    pub fn set_server_user_roles(&mut self, roles: Vec<String>) {
        self.server_user_roles = roles;
    }

    /// Clear server context (reset per-request state for isolation).
    pub fn clear_server_context(&mut self) {
        self.server_json_body = None;
        self.server_query_params = None;
        self.server_user_roles = Vec::new();
    }

    // ── Audit log (Наряд №41 Block 2: parity with interpreter) ──

    /// Push an audit log entry.
    pub fn push_audit(&self, entry: String) {
        self.audit_log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(entry);
    }

    /// Take all audit log entries (consuming them).
    pub fn take_audit_log(&self) -> Vec<String> {
        self.audit_log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .drain(..)
            .collect()
    }

    // ── Route execution (Наряд №40: VM server backend) ──

    /// Execute a compiled route body on the VM.
    ///
    /// This is the VM equivalent of `execute_route_body` in server.rs.
    /// It creates a fresh stack, executes the compiled bytecode, and returns
    /// the result value (typically `Value::HttpResponse` from `respond()`).
    ///
    /// **Isolation**: each call gets a fresh stack — no state leaks between requests.
    /// **Server context**: must be set via `set_server_*` methods before calling.
    pub fn execute_route_code(
        &mut self,
        route: &CompiledRoute,
        program: &Program,
    ) -> Result<Value, String> {
        let mut stack: Vec<Value> = Vec::new();
        let mut call_stack: Vec<CallFrame> = Vec::new();
        self.execute_code(&route.code, &mut stack, &mut call_stack, program)
    }

    /// ADR-0089: If confidence < 1.0, wrap a concrete result as Fluid
    /// with the propagated confidence (VM version).
    fn vm_wrap_with_confidence(
        result: Result<Value, String>,
        confidence: f64,
    ) -> Result<Value, String> {
        if confidence < 1.0 {
            result.map(|val| match val {
                Value::Fluid(_) => val,
                Value::Unit => val,
                concrete => Value::Fluid(vec![crate::interpreter::values::FluidValueVariant {
                    type_name: concrete.type_name().to_string(),
                    value: concrete,
                    confidence,
                }]),
            })
        } else {
            result
        }
    }

    /// Collapse a Fluid value to a concrete type.
    fn maybe_collapse(&mut self, value: &Value, required_type: &str) -> Value {
        match value {
            Value::Fluid(variants) => {
                // If the required type IS Fluid, pass through without collapsing
                if required_type == "Fluid" {
                    // ADR-0089: propagate confidence — min of max variant confidence
                    let max_conf = variants
                        .iter()
                        .map(|v| v.confidence)
                        .fold(0.0_f64, f64::max);
                    self.propagated_confidence = self.propagated_confidence.min(max_conf);
                    return value.clone();
                }
                let best = variants
                    .iter()
                    .filter(|v| v.type_name == required_type)
                    .max_by(|a, b| {
                        a.confidence
                            .partial_cmp(&b.confidence)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                match best {
                    Some(variant) if variant.confidence >= COLLAPSE_THRESHOLD => {
                        // ADR-0089: propagate confidence — track min of all collapses
                        self.propagated_confidence =
                            self.propagated_confidence.min(variant.confidence);
                        variant.value.clone()
                    }
                    _ => Value::Unit,
                }
            }
            other => other.clone(),
        }
    }

    /// Evaluate a binary operation.
    fn eval_binop(
        &self,
        left: Value,
        op: crate::ast::BinOp,
        right: Value,
    ) -> Result<Value, String> {
        match (left, right) {
            (Value::String(a), Value::String(b)) => match op {
                crate::ast::BinOp::Add => Ok(Value::String(format!("{}{}", a, b))),
                crate::ast::BinOp::Eq => Ok(Value::Float(if a == b { 1.0 } else { 0.0 })),
                crate::ast::BinOp::Ne => Ok(Value::Float(if a != b { 1.0 } else { 0.0 })),
                _ => Err(format!("cannot apply {:?} to two Strings", op)),
            },
            (Value::Float(a), Value::Float(b)) => match op {
                crate::ast::BinOp::Add => Ok(Value::Float(a + b)),
                crate::ast::BinOp::Sub => Ok(Value::Float(a - b)),
                crate::ast::BinOp::Mul => Ok(Value::Float(a * b)),
                crate::ast::BinOp::Div => {
                    if b == 0.0 {
                        Err("division by zero".to_string())
                    } else {
                        Ok(Value::Float(a / b))
                    }
                }
                crate::ast::BinOp::Gt => Ok(Value::Float(if a > b { 1.0 } else { 0.0 })),
                crate::ast::BinOp::Lt => Ok(Value::Float(if a < b { 1.0 } else { 0.0 })),
                crate::ast::BinOp::Ge => Ok(Value::Float(if a >= b { 1.0 } else { 0.0 })),
                crate::ast::BinOp::Le => Ok(Value::Float(if a <= b { 1.0 } else { 0.0 })),
                crate::ast::BinOp::Eq => Ok(Value::Float(if a == b { 1.0 } else { 0.0 })),
                crate::ast::BinOp::Ne => Ok(Value::Float(if a != b { 1.0 } else { 0.0 })),
                // And/Or are boolean logic operators, not valid for Float operands directly.
                crate::ast::BinOp::And | crate::ast::BinOp::Or => {
                    Err(format!("BinOp::{:?} not valid for Float operands", op))
                }
            },
            (l, r) => Err(format!(
                "type mismatch: {} {:?} {}",
                l.type_name(),
                op,
                r.type_name()
            )),
        }
    }

    /// Evaluate contains(left, right).
    fn eval_contains(&self, left: Value, right: Value) -> Result<Value, String> {
        let ls = match left {
            Value::String(s) => s,
            other => {
                return Err(format!(
                    "contains: left must be String, got {}",
                    other.type_name()
                ))
            }
        };
        let rs = match right {
            Value::String(s) => s,
            other => {
                return Err(format!(
                    "contains: right must be String, got {}",
                    other.type_name()
                ))
            }
        };
        Ok(Value::Float(if ls.contains(&rs) { 1.0 } else { 0.0 }))
    }

    /// Evaluate a comparison: push 1.0 (true) or 0.0 (false).
    fn eval_cmp(&self, left: Value, right: Value, op: AstCompareOp) -> Value {
        // String-string comparisons (Eq, Ne, contains-like)
        match (&left, &right) {
            (Value::String(a), Value::String(b)) => match op {
                AstCompareOp::Eq => Value::Float(if a == b { 1.0 } else { 0.0 }),
                AstCompareOp::Ne => Value::Float(if a != b { 1.0 } else { 0.0 }),
                AstCompareOp::Gt => Value::Float(if a > b { 1.0 } else { 0.0 }),
                AstCompareOp::Lt => Value::Float(if a < b { 1.0 } else { 0.0 }),
                AstCompareOp::Ge => Value::Float(if a >= b { 1.0 } else { 0.0 }),
                AstCompareOp::Le => Value::Float(if a <= b { 1.0 } else { 0.0 }),
            },
            _ => {
                // Numeric comparisons (Float/Bool via as_float)
                let result = match (left.as_float(), right.as_float()) {
                    (Ok(lf), Ok(rf)) => match op {
                        AstCompareOp::Gt => lf > rf,
                        AstCompareOp::Lt => lf < rf,
                        AstCompareOp::Ge => lf >= rf,
                        AstCompareOp::Le => lf <= rf,
                        AstCompareOp::Eq => lf == rf,
                        AstCompareOp::Ne => lf != rf,
                    },
                    _ => false,
                };
                Value::Float(if result { 1.0 } else { 0.0 })
            }
        }
    }
}

/// Truthiness check (matches interpreter).
fn is_truthy(value: &Value) -> bool {
    match value {
        Value::String(s) => !s.is_empty(),
        Value::Float(f) => *f != 0.0,
        Value::Bool(b) => *b,
        Value::List(items) => !items.is_empty(),
        _ => false,
    }
}
