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
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ast::CompareOp as AstCompareOp;
use crate::bytecode::*;
use crate::builtins::Builtins;
use crate::interpreter::{
    FluidValueVariant, Value,
};
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
    /// Mutate log messages.
    mutate_log: Vec<String>,
    /// Collections loaded flag (for map/filter/reduce).
    collections_loaded: bool,
}

/// Collapse threshold for Fluid values (matches interpreter).
const COLLAPSE_THRESHOLD: f64 = 0.1;

impl Vm {
    /// Create a new VM with empty state.
    pub fn new() -> Self {
        // Наряд №18: full builtin name list — must match compiler builtin_indices order
        let builtin_names = vec![
            // String operations
            "upper".to_string(),        // 0
            "lower".to_string(),        // 1
            "len".to_string(),          // 2
            "str".to_string(),          // 3
            "print".to_string(),        // 4
            "contains".to_string(),     // 5
            "float".to_string(),        // 6
            "to_string".to_string(),    // 7
            "get".to_string(),          // 8
            "push".to_string(),         // 9
            // Environment
            "env".to_string(),          // 10
            // String operations Phase 5.3
            "index_of".to_string(),     // 11
            "substring".to_string(),    // 12
            "char_at".to_string(),      // 13
            "starts_with".to_string(),  // 14
            "ends_with".to_string(),    // 15
            "to_float".to_string(),     // 16
            // Fluid
            "confidence".to_string(),   // 17
            // Internal string/math ops
            "__trim".to_string(),       // 18
            "__replace".to_string(),    // 19
            "__split".to_string(),      // 20
            "__join".to_string(),       // 21
            "__abs".to_string(),        // 22
            "__min".to_string(),        // 23
            "__max".to_string(),        // 24
            "__clamp".to_string(),      // 25
            "__round".to_string(),      // 26
            "__first".to_string(),      // 27
            "__last".to_string(),       // 28
            // Phase 6: Web
            "respond".to_string(),      // 29
            "respond_html".to_string(), // 30
            "form_data".to_string(),    // 31
            "json_body".to_string(),    // 32
            "query_param".to_string(),  // 33
            "render".to_string(),       // 34
            "escape_html".to_string(),  // 35
            // Phase 6.3: DB
            "query".to_string(),        // 36
            "db_execute".to_string(),   // 37
            // Phase 6.4: Crypto
            "hash_password".to_string(),  // 38
            "verify_password".to_string(), // 39
            "encrypt".to_string(),      // 40
            "decrypt".to_string(),      // 41
            "generate_key".to_string(), // 42
            // Phase 6.5: Auth/Session
            "authenticate".to_string(),   // 43
            "session_login".to_string(),  // 44
            "session_logout".to_string(), // 45
            // Phase 6: Messaging
            "send_message".to_string(),  // 46
            "require".to_string(),       // 47
            // HTTP
            "http_post".to_string(),    // 48
            "http_get".to_string(),     // 49
            // Public string/math ops
            "trim".to_string(),         // 50
            "replace".to_string(),      // 51
            "split".to_string(),        // 52
            "join".to_string(),         // 53
            "length".to_string(),       // 54
            "to_int".to_string(),       // 55
            "reverse".to_string(),      // 56
            // LLM
            "call_llm".to_string(),     // 57
            // KV store
            "kv_set".to_string(),       // 58
            "kv_get".to_string(),       // 59
            "kv_delete".to_string(),    // 60
            "kv_exists".to_string(),    // 61
            "kv_list".to_string(),      // 62
            // Memory
            "mem_set".to_string(),      // 63
            "mem_get".to_string(),      // 64
            "mem_delete".to_string(),   // 65
            // File I/O
            "read_file".to_string(),    // 66
            "write_file".to_string(),   // 67
            "append_file".to_string(),  // 68
            "delete_file".to_string(),  // 69
            "file_exists".to_string(),  // 70
            "list_dir".to_string(),     // 71
            // AI providers
            "call_claude".to_string(),  // 72
            // LLM usage
            "llm_usage".to_string(),    // 73
            // JSON
            "escape_json".to_string(),  // 74
            "parse_json".to_string(),   // 75
            "json_encode".to_string(),  // 76
            "json_get".to_string(),     // 77
            "has_field".to_string(),    // 78
            // Time
            "now".to_string(),          // 79
            // Session
            "session_set".to_string(),  // 80
            "session_get".to_string(),  // 81
            "session_clear".to_string(), // 82
            // HTTP extras
            "http_post_multipart".to_string(), // 83
            // Media
            "whisper_transcribe".to_string(), // 84
            "tts_send".to_string(),     // 85
            // Encoding
            "base64_encode".to_string(), // 86
            "base64_decode".to_string(), // 87
            // System
            "exec".to_string(),         // 88
            "escape_js".to_string(),    // 89
            // Misc
            "dict_get".to_string(),     // 90
            "type_of".to_string(),      // 91
            // Memory (recall)
            "recall".to_string(),       // 92
            // Phase 4.4 self-hosting
            "stdin".to_string(),        // 93
            "split_tokens".to_string(), // 94
            "if_eq".to_string(),        // 95
            "newline".to_string(),      // 96
            "is_string_token".to_string(), // 97
        ];

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
            mutate_log: Vec::new(),
            collections_loaded: false,
        }
    }

    /// Execute a compiled program. Returns the flow output (if any),
    /// with mutate log messages prepended if present.
    pub fn run(&mut self, program: Program) -> Result<Option<String>, String> {
        // Initialize globals
        self.globals = vec![Value::Unit; program.globals.len()];
        self.global_names = program.globals.clone();
        self.collections_loaded = program.collections_loaded;

        // Sort rules by priority descending (matches interpreter semantics)
        let mut rules = program.rules.clone();
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        self.rules = rules;

        // Execute main_code
        let mut stack: Vec<Value> = Vec::new();
        let mut call_stack: Vec<CallFrame> = Vec::new();
        let mut ip = 0;
        let code = &program.main_code;
        let mut flow_output: Option<String> = None;

        while ip < code.len() {
            let instr = &code[ip];
            match instr {
                // ── Constants & Variables ─────────────────────
                Instruction::Const(v) => {
                    stack.push(v.clone());
                    ip += 1;
                }
                Instruction::LoadGlobal(slot) => {
                    let val = self.globals.get(*slot)
                        .cloned()
                        .unwrap_or(Value::Unit);
                    stack.push(val);
                    ip += 1;
                }
                Instruction::LoadGlobalByName(name) => {
                    // Search globals by name
                    let val = program.globals.iter().position(|n| n == name)
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
                    let name = self.builtin_names.get(*idx)
                        .map(|s| s.as_str())
                        .unwrap_or("unknown");
                    let mut args = Vec::new();
                    for _ in 0..*arity {
                        args.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }
                    let result = self.call_builtin(name, &args)?;
                    stack.push(result);
                    ip += 1;
                }
                Instruction::CallPattern(idx, arity) => {
                    let pattern = self.patterns.get(*idx)
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
                    let result = self.execute_code(&pattern.code, &mut stack, &mut call_stack, &program)?;
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
                        Value::Float(f) => stack.push(Value::Float(if f == 1.0 { 0.0 } else { 1.0 })),
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
                    let result = val.get_field(field)
                        .cloned()
                        .unwrap_or(Value::Unit);
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

                // ── List Operations (Наряд №18) ───────────────
                Instruction::MakeList(n) => {
                    let mut items: Vec<Value> = Vec::with_capacity(*n);
                    for _ in 0..*n {
                        items.push(stack.pop().unwrap_or(Value::Unit));
                    }
                    items.reverse(); // Restore original order
                    stack.push(Value::List(items));
                    ip += 1;
                }
                Instruction::ListLen => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    let len = match &val {
                        Value::List(items) => items.len() as f64,
                        Value::String(s) => s.len() as f64,
                        _ => 0.0,
                    };
                    stack.push(Value::Float(len));
                    ip += 1;
                }
                Instruction::Pop => {
                    stack.pop();
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
                        }.to_string();
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
                            let max_conf = variants.iter()
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
                            few_shot.push((input_str, output_str));
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return Err(format!("VM adapt: learnable pattern '{}' not found", pattern_name));
                    }
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
                    self.relations.push(VmRelation { from, to, relation });
                    ip += 1;
                }
                Instruction::Mutate { pattern_name, example_count, rollback_threshold, rollback_op } => {
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
                    let msg = self.handle_mutate(pattern_name, new_examples, *rollback_threshold, rollback_op.clone())?;
                    self.mutate_log.push(msg);
                    ip += 1;
                }

                // ── Pipeline ───────────────────────────────────
                Instruction::FlowExec { source_expr, pipeline, branch_defs } => {
                    // Load the source value
                    let source_val = match source_expr {
                        FlowExpr::GlobalSlot(slot) => {
                            self.globals.get(*slot).cloned().unwrap_or(Value::Unit)
                        }
                        FlowExpr::Ident(name) => {
                            program.globals.iter().position(|n| n == name)
                                .and_then(|slot| self.globals.get(slot).cloned())
                                .unwrap_or(Value::Unit)
                        }
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
                    result.push_str("\n");
                    result.push_str(&flow);
                    Ok(Some(result))
                }
                None => Ok(Some(mutate_log.join("\n"))),
            }
        }
    }

    /// Execute a block of code (e.g., pattern body) and return the result.
    /// This handles the call stack and Return instructions internally.
    fn execute_code(
        &self,
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
                return Err(format!("VM execute_code: possible infinite loop ({} iterations, {} instructions)", iterations, code.len()));
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
                    let name = self.builtin_names.get(*idx)
                        .map(|s| s.as_str())
                        .unwrap_or("unknown");
                    let mut args = Vec::new();
                    for _ in 0..*arity {
                        args.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }
                    let result = self.call_builtin(name, &args)?;
                    stack.push(result);
                    ip += 1;
                }
                Instruction::CallPattern(pidx, arity) => {
                    let pattern = self.patterns.get(*pidx)
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
                    call_stack.push(CallFrame { return_ip: ip + 1, base_bp });
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
                        Value::Float(f) => stack.push(Value::Float(if f == 1.0 { 0.0 } else { 1.0 })),
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
                    let val = program.globals.iter().position(|n| n == name)
                        .and_then(|slot| self.globals.get(slot).cloned())
                        .unwrap_or(Value::Unit);
                    stack.push(val);
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
                // For any unhandled instruction, skip
                _ => { ip += 1; }
            }
        }
        Ok(stack.pop().unwrap_or(Value::Unit))
    }

    /// Call a built-in function by name.
    fn call_builtin(&self, name: &str, args: &[Value]) -> Result<Value, String> {
        if name == "recall" {
            let query = match args.get(0) {
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

        // Collections operations
        if self.collections_loaded {
            match name {
                "map" | "filter" | "reduce" => {
                    return Err(format!("VM: collection op '{}' not yet implemented in VM path", name));
                }
                _ => {}
            }
        }

        if let Some(builtin_fn) = self.builtins.get(name) {
            return builtin_fn(args);
        }
        Err(format!("VM: undefined builtin: {}", name))
    }

    /// Call an LLM-backed learnable pattern.
    fn call_llm(&self, idx: usize, args: &[Value]) -> Result<Value, String> {
        let (info, few_shot) = self.learnables.get(idx)
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

        // Call LLM backend
        let backend = llm::create_llm_backend();
        let response = backend.call(&info.prompt, &input)?;

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
                Value::List(arr.iter().map(|v| Vm::json_to_value(v)).collect())
            }
            serde_json::Value::Object(obj) => {
                let mut fields = std::collections::HashMap::new();
                for (k, v) in obj {
                    fields.insert(k.clone(), Vm::json_to_value(v));
                }
                Value::Struct { type_name: "Json".to_string(), fields }
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
        let field_val = current.get_field(&branch.condition_field)
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
                        name, pattern.param_count, args.len()
                    ));
                }

                // ── VM bytecode path ───────────────────────────
                // Collapse Fluid arguments to parameter types
                let collapsed_args: Vec<Value> = args.iter()
                    .zip(pattern.param_types.iter())
                    .map(|(arg, param_type)| self.maybe_collapse(arg, param_type))
                    .collect();
                // Execute pattern body as bytecode
                let mut stack: Vec<Value> = Vec::new();
                let mut call_stack: Vec<CallFrame> = Vec::new();
                let program = Program {
                    globals: Vec::new(),
                    patterns: Vec::new(),
                    learnables: Vec::new(),
                    rules: Vec::new(),
                    main_code: Vec::new(),
                    collections_loaded: false,
                };
                for arg in collapsed_args {
                    stack.push(arg);
                }
                return self.execute_code(&pattern.code, &mut stack, &mut call_stack, &program);
            }
        }

        // Check builtins
        self.call_builtin(name, &args)
    }

    /// Execute all registered rules (already sorted by priority descending).
    fn execute_rules(&mut self) -> Result<(), String> {
        for rule in &self.rules {
            let condition_met = self.eval_rule_condition(&rule.condition)?;
            if condition_met {
                // Find the target entity by name in globals
                let target_slot = self.global_names.iter().position(|n| n == &rule.target_name);
                if let Some(slot) = target_slot {
                    let value = self.eval_rule_value(&rule.value_expr)?;
                    // Set field on the struct
                    if let Some(entity) = self.globals.get_mut(slot) {
                        let _ = entity.set_field(&rule.field, value);
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
                        entity.get_field(field).cloned()
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
        for (_, (info, few_shot)) in self.learnables.iter_mut().enumerate() {
            if info.name == pattern_name {
                let original = few_shot.clone();
                *few_shot = new_examples;

                // Mock accuracy (always 0.95 for MockLlm)
                let accuracy: f64 = 0.95;

                let kept = match (&rollback_op, &rollback_threshold) {
                    (Some(ConditionOp::Lt), Some(threshold)) => accuracy >= *threshold,
                    (Some(ConditionOp::Le), Some(threshold)) => accuracy > *threshold,
                    (Some(ConditionOp::Gt), Some(_)) |
                    (Some(ConditionOp::Ge), Some(_)) => false,
                    (Some(ConditionOp::Eq), Some(threshold)) => (accuracy - threshold).abs() < 1e-9,
                    _ => true,
                };

                if kept {
                    return Ok(format!("[MUTATE] {}: accuracy={}, kept (>= {:.1})",
                        pattern_name, accuracy,
                        rollback_threshold.unwrap_or(0.0)));
                } else {
                    *few_shot = original;
                    return Ok(format!("[MUTATE] {}: accuracy={}, rolled back (below {:.1})",
                        pattern_name, accuracy,
                        rollback_threshold.unwrap_or(0.0)));
                }
            }
        }
        Err(format!("VM mutate: learnable pattern '{}' not found", pattern_name))
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
                        result.push_str("\n");
                        result.push_str(&format!("[GRAPH] {} -> {}", rel.relation, rel.to));
                    } else if rel.to == entry.value {
                        result.push_str("\n");
                        result.push_str(&format!("[GRAPH] {} -> {}", rel.relation, rel.from));
                    }
                }
                result
            }
            None => String::new(),
        }
    }

    /// Collapse a Fluid value to a concrete type.
    fn maybe_collapse(&self, value: &Value, required_type: &str) -> Value {
        match value {
            Value::Fluid(variants) => {
                let best = variants.iter()
                    .filter(|v| v.type_name == required_type)
                    .max_by(|a, b| {
                        a.confidence.partial_cmp(&b.confidence)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                match best {
                    Some(variant) if variant.confidence >= COLLAPSE_THRESHOLD => {
                        variant.value.clone()
                    }
                    _ => Value::Unit,
                }
            }
            other => other.clone(),
        }
    }

    /// Evaluate a binary operation.
    fn eval_binop(&self, left: Value, op: crate::ast::BinOp, right: Value) -> Result<Value, String> {
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
            },
            (l, r) => Err(format!(
                "type mismatch: {} {:?} {}",
                l.type_name(), op, r.type_name()
            )),
        }
    }

    /// Evaluate contains(left, right).
    fn eval_contains(&self, left: Value, right: Value) -> Result<Value, String> {
        let ls = match left {
            Value::String(s) => s,
            other => return Err(format!("contains: left must be String, got {}", other.type_name())),
        };
        let rs = match right {
            Value::String(s) => s,
            other => return Err(format!("contains: right must be String, got {}", other.type_name())),
        };
        Ok(Value::Float(if ls.contains(&rs) { 1.0 } else { 0.0 }))
    }

    /// Evaluate a comparison: push 1.0 (true) or 0.0 (false).
    fn eval_cmp(&self, left: Value, right: Value, op: AstCompareOp) -> Value {
        let result = match (left.as_float(), right.as_float()) {
            (Ok(lf), Ok(rf)) => match op {
                AstCompareOp::Gt => lf > rf,
                AstCompareOp::Lt => lf < rf,
                AstCompareOp::Ge => lf >= rf,
                AstCompareOp::Le => lf <= rf,
                AstCompareOp::Eq => lf == rf,
                _ => false,
            },
            _ => false,
        };
        Value::Float(if result { 1.0 } else { 0.0 })
    }
}

/// Truthiness check (matches interpreter).
fn is_truthy(value: &Value) -> bool {
    match value {
        Value::String(s) => !s.is_empty(),
        Value::Float(f) => *f != 0.0,
        Value::List(items) => !items.is_empty(),
        _ => false,
    }
}
