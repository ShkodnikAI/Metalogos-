// ── METALOGOS Bytecode Compiler — Phase 4.1 ────────────────────
// Translates AST (Vec<Declaration>) into a bytecode Program.
// Contract: the emitted Program, when executed by the VM, produces
// the same output as the tree-walking interpreter.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::ast::*;
use crate::ast::CompareOp as AstCompareOp;
use crate::bytecode::*;
use crate::interpreter::Value;

/// The compiler translates declarations into a bytecode Program.
pub struct Compiler {
    /// Global variable name -> slot index.
    global_slots: HashMap<String, usize>,
    /// Next available global slot.
    next_global: usize,
    /// Pattern name -> index in program.patterns.
    pattern_indices: HashMap<String, usize>,
    /// Learnable pattern name -> index in program.learnables.
    learnable_indices: HashMap<String, usize>,
    /// Builtin name -> index in builtin table.
    builtin_indices: HashMap<String, usize>,
    /// Struct type name -> field names (in declaration order).
    struct_fields: HashMap<String, Vec<String>>,
    /// Rule declarations to compile.
    rules: Vec<CompiledRule>,
    /// Root directory for import resolution.
    std_root: PathBuf,
    /// Already-imported modules.
    imported_modules: HashSet<String>,
    /// Collections loaded flag.
    collections_loaded: bool,
    /// Sandbox declarations (recorded).
    sandboxes: HashMap<String, SandboxDecl>,
}



/// Loop context for break/continue jump patching. (Наряд №18)
struct LoopCtx {
    /// Address of the condition check (for continue to jump back to).
    continue_addr: usize,
    /// Locations of Jump(0) placeholders emitted by Break that need patching.
    break_patches: Vec<usize>,
}

impl Compiler {
    /// Create a new compiler with default settings.
    pub fn new() -> Self {
        Self::with_std_root(
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        )
    }

    /// Create a compiler with a custom std root directory for import resolution.
    pub fn with_std_root(std_root: PathBuf) -> Self {
        let mut builtin_indices = HashMap::new();
        // Наряд №18: full builtin list — must match builtins.rs Builtins::new() order
        // for consistent indexing between compiler and VM.
        let builtins = [
            // String operations (builtins.rs order)
            "upper", "lower", "len", "str", "print", "contains", "float",
            "to_string", "get", "push",
            // Environment
            "env",
            // String operations Phase 5.3
            "index_of", "substring", "char_at", "starts_with", "ends_with", "to_float",
            // Fluid
            "confidence",
            // Internal string/math ops
            "__trim", "__replace", "__split", "__join",
            "__abs", "__min", "__max", "__clamp", "__round",
            "__first", "__last",
            // Phase 6: Web
            "respond", "respond_html", "form_data", "json_body", "query_param",
            "render", "escape_html",
            // Phase 6.3: DB
            "query", "db_execute",
            // Phase 6.4: Crypto
            "hash_password", "verify_password", "encrypt", "decrypt", "generate_key",
            // Phase 6.5: Auth/Session
            "authenticate", "session_login", "session_logout",
            // Phase 6: Messaging
            "send_message", "require",
            // HTTP
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
            "now",
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
            "dict_get", "type_of",
            // Memory (recall)
            "recall",
            // Phase 4.4 self-hosting
            "stdin", "split_tokens", "if_eq", "newline", "is_string_token",
        ];
        for (i, name) in builtins.iter().enumerate() {
            builtin_indices.insert(name.to_string(), i);
        }

        Compiler {
            global_slots: HashMap::new(),
            next_global: 0,
            pattern_indices: HashMap::new(),
            learnable_indices: HashMap::new(),
            builtin_indices,
            struct_fields: HashMap::new(),
            rules: Vec::new(),
            std_root,
            imported_modules: HashSet::new(),
            collections_loaded: false,
            sandboxes: HashMap::new(),
        }
    }

    /// Compile a list of declarations into a bytecode Program.
    pub fn compile(&mut self, declarations: Vec<Declaration>) -> Result<Program, String> {
        // Phase 1: resolve imports
        let mut all_decls = Vec::new();
        for decl in declarations {
            if let Declaration::Import(import) = &decl {
                // Fix 2: use `import.path` instead of `import.module_path`
                if !self.imported_modules.contains(&import.path) {
                    let imported = self.resolve_import(&import.path)?;
                    all_decls.extend(imported);
                }
            } else {
                all_decls.push(decl);
            }
        }

        // Phase 2: two-pass compilation
        // Pass 1: collect struct types, pattern names, learnable names, global slots
        self.pass1(&all_decls)?;

        // Pass 2: generate main_code
        let main_code = self.pass2(&all_decls)?;

        let program = Program {
            // Build globals list with actual names (indexed by slot)
            globals: (0..self.next_global)
                .map(|i| self.global_slots.iter()
                    .find(|(_, &slot)| slot == i)
                    .map(|(name, _)| name.clone())
                    .unwrap_or_default())
                .collect(),
            patterns: Vec::new(), // will be filled from pass1 data
            learnables: Vec::new(),
            rules: std::mem::take(&mut self.rules),
            main_code,
            collections_loaded: self.collections_loaded,
        };

        Ok(program)
    }

    /// Pass 1: collect type info, register patterns, assign global slots.
    fn pass1(&mut self, decls: &[Declaration]) -> Result<(), String> {
        for decl in decls {
            match decl {
                Declaration::EntityType(e) => {
                    let fields: Vec<String> = e.fields.iter().map(|f| f.name.clone()).collect();
                    self.struct_fields.insert(e.name.clone(), fields);
                }
                Declaration::EntityRecord(e) => {
                    self.ensure_global(&e.name);
                }
                Declaration::EntitySimple(e) => {
                    self.ensure_global(&e.name);
                }
                Declaration::Pattern(p) => {
                    let idx = self.pattern_indices.len();
                    self.pattern_indices.insert(p.name.clone(), idx);
                }
                Declaration::LearnablePattern(lp) => {
                    let idx = self.learnable_indices.len();
                    self.learnable_indices.insert(lp.name.clone(), idx);
                }
                Declaration::Fluid(fl) => {
                    self.ensure_global(&fl.name);
                }
                Declaration::Rule(r) => {
                    self.rules.push(self.compile_rule(r)?);
                }
                Declaration::Memorize(_) => {
                    // Handled in pass2
                }
                Declaration::Sandbox(s) => {
                    self.sandboxes.insert(s.name.clone(), s.clone());
                }
                Declaration::Adapt(_) | Declaration::Relate(_) | Declaration::Mutate(_) | Declaration::Forget(_) => {
                    // Handled in pass2
                }
                Declaration::MlogServer(_) | Declaration::Template(_) | Declaration::Db(_) | Declaration::Memory(_) | Declaration::Conversation(_) | Declaration::Tool(_) | Declaration::LlmConfig(_) => {
                    // Phase 6: handled elsewhere
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Pass 2: generate main_code instructions.
    fn pass2(&mut self, decls: &[Declaration]) -> Result<Vec<Instruction>, String> {
        let mut code = Vec::new();

        for decl in decls {
            match decl {
                Declaration::EntityType(e) => {
                    // Struct type already registered in pass1. No runtime instruction needed.
                    // (The VM will need to know about struct types for MakeStruct.)
                    // We emit a special marker or nothing — the VM handles this via metadata.
                    let _ = e;
                }
                Declaration::EntityRecord(e) => {
                    // Evaluate each field initializer, create struct, store globally
                    let field_names = self.struct_fields.get(&e.type_name)
                        .ok_or_else(|| format!("compile: unknown struct type: {}", e.type_name))?;

                    // Push field values in field order
                    for fd_name in field_names {
                        // Find initializer for this field
                        let init = e.fields.iter().find(|fi| fi.name == *fd_name);
                        match init {
                            Some(fi) => self.compile_expr(&fi.value, &mut code)?,
                            None => code.push(Instruction::Const(Value::Unit)),
                        }
                    }

                    let slot = self.global_slots[&e.name];
                    code.push(Instruction::MakeStruct(e.type_name.clone(), field_names.clone()));
                    code.push(Instruction::StoreGlobal(slot));
                }
                Declaration::EntitySimple(e) => {
                    self.compile_expr(&e.value, &mut code)?;
                    let slot = self.global_slots[&e.name];
                    code.push(Instruction::StoreGlobal(slot));
                }
                Declaration::Pattern(p) => {
                    // Compile pattern body with parameter names as locals
                    let mut locals: HashMap<String, usize> = p.params.iter()
                        .enumerate()
                        .map(|(i, param)| (param.name.clone(), i))
                        .collect();
                    let fn_code = self.compile_pattern_body_with_locals(&p.body, &mut locals)?;
                    let is_pure = Self::analyze_purity(&fn_code, &p.params);
                    let compiled = CompiledFn {
                        name: p.name.clone(),
                        param_count: p.params.len(),
                        param_types: p.params.iter().map(|param| param.type_name.clone()).collect(),
                        code: fn_code,
                        is_pure,
                    };
                    // We'll add this to the program's patterns list
                    // For now, store in a side-channel; we'll fix this below.
                    // Actually, let's store directly. The problem is that we're building
                    // the program in compile(), not here. Let's use a different approach.
                    // We'll add a pseudo-instruction to register the pattern.
                    code.push(Instruction::RegisterPattern(compiled));
                }
                Declaration::LearnablePattern(lp) => {
                    code.push(Instruction::RegisterLearnable(CompiledLearnableInfo {
                        name: lp.name.clone(),
                        param_count: lp.params.len(),
                        prompt: lp.prompt.clone(),
                        few_shot: Vec::new(),
                    }));
                }
                Declaration::Rule(_) => {
                    // Rules are already compiled. Emit ExecuteRules before any flow.
                    // We'll insert this before the flow instruction.
                }
                Declaration::Memorize(m) => {
                    self.compile_expr(&m.value, &mut code)?;
                    code.push(Instruction::Memorize(m.priority));
                }
                Declaration::Forget(f) => {
                    self.compile_expr(&f.query, &mut code)?;
                    // We need a Forget instruction — let's add it to the bytecode
                    // For now, emit as a special instruction
                    code.push(Instruction::Forget(f.days));
                }
                Declaration::Fluid(fl) => {
                    // Compile Fluid value construction: push value, confidence pairs
                    for v in &fl.variants {
                        self.compile_expr(&v.value, &mut code)?;
                        // Push confidence
                        code.push(Instruction::Const(Value::Float(v.confidence)));
                    }
                    let slot = self.global_slots[&fl.name];
                    code.push(Instruction::MakeFluid(fl.variants.len()));
                    code.push(Instruction::StoreGlobal(slot));
                }
                Declaration::Adapt(a) => {
                    // Evaluate input/output examples (try to resolve literals)
                    // For now, just emit an adapt instruction
                    self.compile_expr(&a.input_example, &mut code)?;
                    self.compile_expr(&a.output_example, &mut code)?;
                    code.push(Instruction::Adapt(a.pattern_name.clone()));
                }
                Declaration::Relate(r) => {
                    self.compile_expr(&r.from, &mut code)?;
                    self.compile_expr(&r.to, &mut code)?;
                    code.push(Instruction::Const(Value::String(r.relation.clone())));
                    code.push(Instruction::Relate);
                }
                Declaration::Sandbox(_) => {
                    // No runtime instruction needed
                }
                Declaration::Mutate(m) => {
                    // Compile new examples and rollback info
                    let mut examples = Vec::new();
                    for (inp, out) in &m.new_examples {
                        self.compile_expr(inp, &mut code)?;
                        self.compile_expr(out, &mut code)?;
                        examples.push((String::new(), String::new())); // placeholder
                    }
                    let rollback_op = m.rollback_op.map(|op| match op {
                        AstCompareOp::Gt => ConditionOp::Gt,
                        AstCompareOp::Lt => ConditionOp::Lt,
                        AstCompareOp::Ge => ConditionOp::Ge,
                        AstCompareOp::Le => ConditionOp::Le,
                        AstCompareOp::Eq => ConditionOp::Eq,
                        AstCompareOp::Ne => ConditionOp::Ne,
                        _ => ConditionOp::Eq, // fallback
                    });
                    code.push(Instruction::Mutate {
                        pattern_name: m.pattern_name.clone(),
                        example_count: m.new_examples.len(),
                        rollback_threshold: m.rollback_threshold,
                        rollback_op,
                    });
                }
                Declaration::Flow(f) => {
                    // Emit ExecuteRules before the flow (if any rules exist)
                    if !self.rules.is_empty() {
                        code.push(Instruction::ExecuteRules);
                    }
                    // Compile the flow source expression
                    let source_expr = self.compile_flow_expr(&f.source);
                    let mut branch_defs = Vec::new();
                    for (step_name, branches) in &f.branch_defs {
                        let compiled_branches: Vec<BranchDef> = branches.iter()
                            .map(|b| {
                                let op = match b.condition.op {
                                    AstCompareOp::Gt => ConditionOp::Gt,
                                    AstCompareOp::Lt => ConditionOp::Lt,
                                    AstCompareOp::Ge => ConditionOp::Ge,
                                    AstCompareOp::Le => ConditionOp::Le,
                                    AstCompareOp::Eq => ConditionOp::Eq,
                                    AstCompareOp::Ne => ConditionOp::Ne,
                                    _ => ConditionOp::Eq, // fallback
                                };
                                // Compile the threshold expression to a constant if possible
                                let threshold_val = self.eval_const_expr(&b.condition.threshold);
                                BranchDef {
                                    label: b.label.clone(),
                                    condition_field: b.condition.field.clone(),
                                    condition_op: op,
                                    condition_threshold: threshold_val,
                                    target: b.target.clone(),
                                }
                            })
                            .collect();
                        branch_defs.push((step_name.clone(), compiled_branches));
                    }
                    code.push(Instruction::FlowExec {
                        source_expr,
                        pipeline: f.pipeline.clone(),
                        branch_defs,
                    });
                }
                Declaration::Import(_) => {
                    // Already resolved in import preprocessing
                }
                Declaration::MlogServer(_) | Declaration::Template(_) | Declaration::Db(_) | Declaration::Memory(_)
                | Declaration::Hook(_) | Declaration::Eval(_) | Declaration::Conversation(_) | Declaration::Tool(_) | Declaration::LlmConfig(_) => {
                    // Phase 6+: no bytecode instruction needed
                }
            }
        }

        code.push(Instruction::Halt);
        Ok(code)
    }

    /// Compile an AST expression into stack instructions (no locals context).
    fn compile_expr(&self, expr: &Expr, code: &mut Vec<Instruction>) -> Result<(), String> {
        self.compile_expr_with_locals(expr, code, &HashMap::new())
    }

    /// Compile an AST expression into stack instructions with a locals map.
    fn compile_expr_with_locals(&self, expr: &Expr, code: &mut Vec<Instruction>, locals: &HashMap<String, usize>) -> Result<(), String> {
        match expr {
            Expr::StringLit(s) => {
                code.push(Instruction::Const(Value::String(s.clone())));
            }
            Expr::FloatLit(f) => {
                code.push(Instruction::Const(Value::Float(*f)));
            }
            Expr::Ident(name) => {
                // Check if it's a local (parameter or let binding) first
                if let Some(&slot) = locals.get(name) {
                    code.push(Instruction::LoadLocal(slot));
                } else if let Some(&slot) = self.global_slots.get(name) {
                    code.push(Instruction::LoadGlobal(slot));
                } else {
                    code.push(Instruction::LoadGlobalByName(name.clone()));
                }
            }
            Expr::FieldAccess(base, field) => {
                self.compile_expr_with_locals(base, code, locals)?;
                code.push(Instruction::GetField(field.clone()));
            }
            Expr::FnCall(name, args) => {
                for arg in args {
                    self.compile_expr_with_locals(arg, code, locals)?;
                }
                let arity = args.len();
                // Check if it's a builtin
                if let Some(&idx) = self.builtin_indices.get(name) {
                    code.push(Instruction::CallBuiltin(idx, arity));
                } else if let Some(&idx) = self.pattern_indices.get(name) {
                    code.push(Instruction::CallPattern(idx, arity));
                } else if let Some(&idx) = self.learnable_indices.get(name) {
                    code.push(Instruction::LlmCall(idx, arity));
                } else {
                    return Err(format!("compile: undefined function: {}", name));
                }
            }
            Expr::BinaryOp(left, op, right) => {
                self.compile_expr_with_locals(left, code, locals)?;
                self.compile_expr_with_locals(right, code, locals)?;
                match op {
                    BinOp::Add => code.push(Instruction::Add),
                    BinOp::Sub => code.push(Instruction::Sub),
                    BinOp::Mul => code.push(Instruction::Mul),
                    BinOp::Div => code.push(Instruction::Div),
                    // Phase 5.1: comparison operators
                    BinOp::Gt => code.push(Instruction::CmpGt),
                    BinOp::Lt => code.push(Instruction::CmpLt),
                    BinOp::Ge => code.push(Instruction::CmpGe),
                    BinOp::Le => code.push(Instruction::CmpLe),
                    BinOp::Eq => code.push(Instruction::CmpEq),
                    BinOp::Ne => code.push(Instruction::CmpNe),
                }
            }
            Expr::IfElse(cond, then_expr, else_expr) => {
                // Compile condition
                self.compile_expr_with_locals(cond, code, locals)?;
                // Jump to else branch if falsy
                let jump_to_else = code.len();
                code.push(Instruction::JumpIfNot(0)); // placeholder
                // Compile then branch
                self.compile_expr_with_locals(then_expr, code, locals)?;
                // Jump past else branch
                let jump_to_end = code.len();
                code.push(Instruction::Jump(0)); // placeholder
                // Patch: else branch starts here
                let else_start = code.len();
                if let Some(Instruction::JumpIfNot(ref mut target)) = code.get_mut(jump_to_else) {
                    *target = else_start;
                }
                // Compile else branch
                self.compile_expr_with_locals(else_expr, code, locals)?;
                // Patch: end jump target
                let end = code.len();
                if let Some(Instruction::Jump(ref mut target)) = code.get_mut(jump_to_end) {
                    *target = end;
                }
            }
            // Additional expression forms in Metalogos- AST
            Expr::BoolLit(b) => {
                code.push(Instruction::Const(Value::Float(if *b { 1.0 } else { 0.0 })));
            }
            // Наряд №18: QualifiedCall — resolve function part (ignore module prefix)
            Expr::QualifiedCall { module: _, function, args } => {
                for arg in args {
                    self.compile_expr_with_locals(arg, code, locals)?;
                }
                let arity = args.len();
                if let Some(&idx) = self.builtin_indices.get(function) {
                    code.push(Instruction::CallBuiltin(idx, arity));
                } else if let Some(&idx) = self.pattern_indices.get(function) {
                    code.push(Instruction::CallPattern(idx, arity));
                } else if let Some(&idx) = self.learnable_indices.get(function) {
                    code.push(Instruction::LlmCall(idx, arity));
                } else {
                    return Err(format!("compile: undefined function in qualified call: {}", function));
                }
            }
            // Наряд №18: proper list construction via MakeList
            Expr::List(items) => {
                for item in items {
                    self.compile_expr_with_locals(item, code, locals)?;
                }
                code.push(Instruction::MakeList(items.len()));
            }
            Expr::IndexAccess(base, index) => {
                self.compile_expr_with_locals(base, code, locals)?;
                self.compile_expr_with_locals(index, code, locals)?;
                code.push(Instruction::IndexAccess);
            }
            Expr::StructLit(fields) => {
                // Single iteration ensures field_names and compiled values
                // stay in the same order (HashMap iteration is consistent
                // within a process but we collect once to be explicit).
                let items: Vec<(String, &Expr)> = fields.iter().collect();
                let field_names: Vec<String> = items.iter().map(|(k, _)| k.clone()).collect();
                for (_, val_expr) in &items {
                    self.compile_expr_with_locals(val_expr, code, locals)?;
                }
                code.push(Instruction::MakeStruct("Struct".to_string(), field_names));
            }
            // Наряд №14 P0-3: block if/else expression — deferred to tree-walking
            Expr::BlockIfElse { .. } => {
                // Compiled as Unit placeholder; tree-walking interpreter handles it
                code.push(Instruction::Const(Value::Unit));
            }
            // Наряд №14 P1-4: try expression — deferred to tree-walking
            Expr::Try(_) => {
                code.push(Instruction::Const(Value::Unit));
            }
        }
        Ok(())
    }

    /// Compile a pattern body with parameter names as locals.
    /// Phase 5.1: also handles let bindings, assigning additional local slots.
    /// Наряд №18: full statement compilation — IfElseBlock, Each, EachWithIndex,
    /// While, Assign, Match, ExprStmt, IfThen, Break, Continue.
    fn compile_pattern_body_with_locals(&self, body: &[Statement], locals: &mut HashMap<String, usize>) -> Result<Vec<Instruction>, String> {
        let mut code = Vec::new();
        let mut next_slot = locals.len();
        self.compile_stmts(body, &mut code, locals, &mut next_slot, &mut None)?;
        Ok(code)
    }

    /// Compile a list of statements into the code buffer.
    /// Наряд №18: the core statement compilation engine.
    fn compile_stmts(
        &self,
        stmts: &[Statement],
        code: &mut Vec<Instruction>,
        locals: &mut HashMap<String, usize>,
        next_slot: &mut usize,
        loop_ctx: &mut Option<LoopCtx>,
    ) -> Result<(), String> {
        for stmt in stmts {
            self.compile_stmt(stmt, code, locals, next_slot, loop_ctx)?;
        }
        Ok(())
    }

    /// Compile a single statement into the code buffer.
    fn compile_stmt(
        &self,
        stmt: &Statement,
        code: &mut Vec<Instruction>,
        locals: &mut HashMap<String, usize>,
        next_slot: &mut usize,
        loop_ctx: &mut Option<LoopCtx>,
    ) -> Result<(), String> {
        match stmt {
            Statement::LetBinding { name, value, mutable: _ } => {
                let slot = *next_slot;
                *next_slot += 1;
                locals.insert(name.clone(), slot);
                self.compile_expr_with_locals(value, code, locals)?;
                code.push(Instruction::StoreLocal(slot));
            }
            Statement::Return(expr) => {
                self.compile_expr_with_locals(expr, code, locals)?;
                code.push(Instruction::Return);
            }
            // Наряд №18: Assign — store to local if known, else global
            Statement::Assign { name, value } => {
                self.compile_expr_with_locals(value, code, locals)?;
                if let Some(&slot) = locals.get(name) {
                    code.push(Instruction::StoreLocal(slot));
                } else if let Some(&slot) = self.global_slots.get(name) {
                    code.push(Instruction::StoreGlobal(slot));
                } else {
                    // Fallback: store by name
                    code.push(Instruction::Pop);
                }
            }
            // Наряд №18: ExprStmt — compile expression, discard result
            Statement::ExprStmt(expr) => {
                self.compile_expr_with_locals(expr, code, locals)?;
                code.push(Instruction::Pop);
            }
            // Наряд №18: IfElseBlock — full JumpIfNot/Jump chain
            Statement::IfElseBlock { condition, then_body, else_ifs, else_body } => {
                self.compile_expr_with_locals(condition, code, locals)?;
                let jump_to_else = code.len();
                code.push(Instruction::JumpIfNot(0)); // placeholder
                // Compile then_body
                self.compile_stmts(then_body, code, locals, next_slot, loop_ctx)?;
                let jump_to_end = code.len();
                code.push(Instruction::Jump(0)); // placeholder
                // Patch: else branch starts here
                let else_start = code.len();
                if let Some(Instruction::JumpIfNot(ref mut target)) = code.get_mut(jump_to_else) {
                    *target = else_start;
                }
                // Compile else-if chain — track each end-jump for patching
                let mut ei_end_jumps: Vec<usize> = Vec::new();
                for (ei_cond, ei_body) in else_ifs {
                    self.compile_expr_with_locals(ei_cond, code, locals)?;
                    let ei_jump = code.len();
                    code.push(Instruction::JumpIfNot(0)); // placeholder
                    self.compile_stmts(ei_body, code, locals, next_slot, loop_ctx)?;
                    let ei_jump_end = code.len();
                    code.push(Instruction::Jump(0)); // placeholder
                    ei_end_jumps.push(ei_jump_end);
                    // Patch else-if condition jump to next arm
                    let ei_next = code.len();
                    if let Some(Instruction::JumpIfNot(ref mut target)) = code.get_mut(ei_jump) {
                        *target = ei_next;
                    }
                }
                // Compile else body
                if let Some(eb) = else_body {
                    self.compile_stmts(eb, code, locals, next_slot, loop_ctx)?;
                }
                // Patch all end jumps to here
                let end = code.len();
                if let Some(Instruction::Jump(ref mut target)) = code.get_mut(jump_to_end) {
                    *target = end;
                }
                for &patch_addr in &ei_end_jumps {
                    if let Some(Instruction::Jump(ref mut target)) = code.get_mut(patch_addr) {
                        *target = end;
                    }
                }
            }
            // Наряд №18: IfThen — single-branch conditional
            Statement::IfThen(cond, body) => {
                self.compile_expr_with_locals(cond, code, locals)?;
                let jump_past = code.len();
                code.push(Instruction::JumpIfNot(0)); // placeholder
                self.compile_stmts(body, code, locals, next_slot, loop_ctx)?;
                let after = code.len();
                if let Some(Instruction::JumpIfNot(ref mut target)) = code.get_mut(jump_past) {
                    *target = after;
                }
            }
            // Наряд №18: Each — indexed iteration over list
            Statement::Each { variable, iterable, body } => {
                // Evaluate iterable → list on stack
                self.compile_expr_with_locals(iterable, code, locals)?;
                // Store list in hidden local
                let list_slot = *next_slot;
                *next_slot += 1;
                code.push(Instruction::StoreLocal(list_slot));
                // Initialize index = 0
                let idx_slot = *next_slot;
                *next_slot += 1;
                code.push(Instruction::Const(Value::Float(0.0)));
                code.push(Instruction::StoreLocal(idx_slot));
                // Set up loop context
                let mut inner_loop = LoopCtx {
                    continue_addr: 0,
                    break_patches: Vec::new(),
                };
                // Condition check: idx < len(list)
                let cond_addr = code.len();
                inner_loop.continue_addr = cond_addr;
                code.push(Instruction::LoadLocal(idx_slot));
                code.push(Instruction::LoadLocal(list_slot));
                code.push(Instruction::ListLen);
                code.push(Instruction::CmpLt);
                let exit_jump = code.len();
                code.push(Instruction::JumpIfNot(0)); // placeholder
                // Load item: list[idx]
                code.push(Instruction::LoadLocal(list_slot));
                code.push(Instruction::LoadLocal(idx_slot));
                code.push(Instruction::IndexAccess);
                // Bind item variable
                let item_slot = if let Some(&existing) = locals.get(variable) {
                    existing
                } else {
                    let s = *next_slot;
                    *next_slot += 1;
                    locals.insert(variable.clone(), s);
                    s
                };
                code.push(Instruction::StoreLocal(item_slot));
                // Compile body with loop context
                self.compile_stmts(body, code, locals, next_slot, &mut Some(inner_loop))?;
                // Increment index
                code.push(Instruction::LoadLocal(idx_slot));
                code.push(Instruction::Const(Value::Float(1.0)));
                code.push(Instruction::Add);
                code.push(Instruction::StoreLocal(idx_slot));
                // Jump back to condition
                code.push(Instruction::Jump(cond_addr));
                // Loop end
                let loop_end = code.len();
                // Patch exit jump
                if let Some(Instruction::JumpIfNot(ref mut target)) = code.get_mut(exit_jump) {
                    *target = loop_end;
                }
                // Patch break jumps
                for &patch_addr in &inner_loop.break_patches {
                    if let Some(Instruction::Jump(ref mut target)) = code.get_mut(patch_addr) {
                        *target = loop_end;
                    }
                }
            }
            // Наряд №18: EachWithIndex — two-variable iteration
            Statement::EachWithIndex { index_var, item_var, iterable, body } => {
                self.compile_expr_with_locals(iterable, code, locals)?;
                let list_slot = *next_slot;
                *next_slot += 1;
                code.push(Instruction::StoreLocal(list_slot));
                let idx_slot = *next_slot;
                *next_slot += 1;
                code.push(Instruction::Const(Value::Float(0.0)));
                code.push(Instruction::StoreLocal(idx_slot));
                let mut inner_loop = LoopCtx {
                    continue_addr: 0,
                    break_patches: Vec::new(),
                };
                let cond_addr = code.len();
                inner_loop.continue_addr = cond_addr;
                code.push(Instruction::LoadLocal(idx_slot));
                code.push(Instruction::LoadLocal(list_slot));
                code.push(Instruction::ListLen);
                code.push(Instruction::CmpLt);
                let exit_jump = code.len();
                code.push(Instruction::JumpIfNot(0));
                // Bind index var
                let idx_bind = if let Some(&existing) = locals.get(index_var) {
                    existing
                } else {
                    let s = *next_slot;
                    *next_slot += 1;
                    locals.insert(index_var.clone(), s);
                    s
                };
                code.push(Instruction::LoadLocal(idx_slot));
                code.push(Instruction::StoreLocal(idx_bind));
                // Load and bind item var
                code.push(Instruction::LoadLocal(list_slot));
                code.push(Instruction::LoadLocal(idx_slot));
                code.push(Instruction::IndexAccess);
                let item_slot = if let Some(&existing) = locals.get(item_var) {
                    existing
                } else {
                    let s = *next_slot;
                    *next_slot += 1;
                    locals.insert(item_var.clone(), s);
                    s
                };
                code.push(Instruction::StoreLocal(item_slot));
                // Compile body
                self.compile_stmts(body, code, locals, next_slot, &mut Some(inner_loop))?;
                // Increment index
                code.push(Instruction::LoadLocal(idx_slot));
                code.push(Instruction::Const(Value::Float(1.0)));
                code.push(Instruction::Add);
                code.push(Instruction::StoreLocal(idx_slot));
                code.push(Instruction::Jump(cond_addr));
                let loop_end = code.len();
                if let Some(Instruction::JumpIfNot(ref mut target)) = code.get_mut(exit_jump) {
                    *target = loop_end;
                }
                for &patch_addr in &inner_loop.break_patches {
                    if let Some(Instruction::Jump(ref mut target)) = code.get_mut(patch_addr) {
                        *target = loop_end;
                    }
                }
            }
            // Наряд №18: While — condition + body loop
            Statement::While { condition, body } => {
                let mut inner_loop = LoopCtx {
                    continue_addr: 0,
                    break_patches: Vec::new(),
                };
                let cond_addr = code.len();
                inner_loop.continue_addr = cond_addr;
                self.compile_expr_with_locals(condition, code, locals)?;
                let exit_jump = code.len();
                code.push(Instruction::JumpIfNot(0));
                self.compile_stmts(body, code, locals, next_slot, &mut Some(inner_loop))?;
                code.push(Instruction::Jump(cond_addr));
                let loop_end = code.len();
                if let Some(Instruction::JumpIfNot(ref mut target)) = code.get_mut(exit_jump) {
                    *target = loop_end;
                }
                for &patch_addr in &inner_loop.break_patches {
                    if let Some(Instruction::Jump(ref mut target)) = code.get_mut(patch_addr) {
                        *target = loop_end;
                    }
                }
            }
            // Наряд №18: Match — dispatch via chained comparisons
            Statement::Match { scrutinee, arms, else_body } => {
                // Store scrutinee in a hidden local for repeated access
                self.compile_expr_with_locals(scrutinee, code, locals)?;
                let scrut_slot = *next_slot;
                *next_slot += 1;
                code.push(Instruction::StoreLocal(scrut_slot));
                let mut arm_end_jumps: Vec<usize> = Vec::new();
                for arm in arms {
                    match arm {
                        MatchArm::Exact(val, arm_body) => {
                            code.push(Instruction::LoadLocal(scrut_slot));
                            // Convert scrutinee to string via str() builtin if possible,
                            // or use CmpEq directly for string comparison
                            code.push(Instruction::Const(Value::String(val.clone())));
                            code.push(Instruction::CmpEq);
                            let arm_skip = code.len();
                            code.push(Instruction::JumpIfNot(0));
                            self.compile_stmts(arm_body, code, locals, next_slot, loop_ctx)?;
                            let arm_end = code.len();
                            code.push(Instruction::Jump(0));
                            arm_end_jumps.push(arm_end);
                            let next_arm = code.len();
                            if let Some(Instruction::JumpIfNot(ref mut target)) = code.get_mut(arm_skip) {
                                *target = next_arm;
                            }
                        }
                        MatchArm::Contains(substr, arm_body) => {
                            code.push(Instruction::LoadLocal(scrut_slot));
                            code.push(Instruction::Const(Value::String(substr.clone())));
                            code.push(Instruction::Contains);
                            let arm_skip = code.len();
                            code.push(Instruction::JumpIfNot(0));
                            self.compile_stmts(arm_body, code, locals, next_slot, loop_ctx)?;
                            let arm_end = code.len();
                            code.push(Instruction::Jump(0));
                            arm_end_jumps.push(arm_end);
                            let next_arm = code.len();
                            if let Some(Instruction::JumpIfNot(ref mut target)) = code.get_mut(arm_skip) {
                                *target = next_arm;
                            }
                        }
                        MatchArm::StartsWith(prefix, arm_body) => {
                            // Наряд №21: StartsWith now compiled using the StartsWith instruction
                            code.push(Instruction::LoadLocal(scrut_slot));
                            code.push(Instruction::Const(Value::String(prefix.clone())));
                            code.push(Instruction::StartsWith);
                            let arm_skip = code.len();
                            code.push(Instruction::JumpIfNot(0));
                            self.compile_stmts(arm_body, code, locals, next_slot, loop_ctx)?;
                            let arm_end = code.len();
                            code.push(Instruction::Jump(0));
                            arm_end_jumps.push(arm_end);
                            let next_arm = code.len();
                            if let Some(Instruction::JumpIfNot(ref mut target)) = code.get_mut(arm_skip) {
                                *target = next_arm;
                            }
                        }
                        MatchArm::Compare(op, threshold, arm_body) => {
                            code.push(Instruction::LoadLocal(scrut_slot));
                            self.compile_expr_with_locals(threshold, code, locals)?;
                            let cmp_instr = match op {
                                AstCompareOp::Gt => Instruction::CmpGt,
                                AstCompareOp::Lt => Instruction::CmpLt,
                                AstCompareOp::Ge => Instruction::CmpGe,
                                AstCompareOp::Le => Instruction::CmpLe,
                                AstCompareOp::Eq => Instruction::CmpEq,
                                AstCompareOp::Ne => Instruction::CmpNe,
                            };
                            code.push(cmp_instr);
                            let arm_skip = code.len();
                            code.push(Instruction::JumpIfNot(0));
                            self.compile_stmts(arm_body, code, locals, next_slot, loop_ctx)?;
                            let arm_end = code.len();
                            code.push(Instruction::Jump(0));
                            arm_end_jumps.push(arm_end);
                            let next_arm = code.len();
                            if let Some(Instruction::JumpIfNot(ref mut target)) = code.get_mut(arm_skip) {
                                *target = next_arm;
                            }
                        }
                    }
                }
                // Else body
                if let Some(eb) = else_body {
                    self.compile_stmts(eb, code, locals, next_slot, loop_ctx)?;
                }
                // Patch all arm end jumps to here
                let match_end = code.len();
                for &patch_addr in &arm_end_jumps {
                    if let Some(Instruction::Jump(ref mut target)) = code.get_mut(patch_addr) {
                        *target = match_end;
                    }
                }
            }
            // Наряд №18: Break — Jump to loop end (placeholder, patched by loop)
            Statement::Break => {
                if let Some(lc) = loop_ctx {
                    let patch = code.len();
                    code.push(Instruction::Jump(0)); // placeholder
                    lc.break_patches.push(patch);
                }
                // Outside loop: silently ignore (matches interpreter error behavior at higher level)
            }
            // Наряд №18: Continue — Jump to loop condition check
            Statement::Continue => {
                if let Some(lc) = loop_ctx {
                    code.push(Instruction::Jump(lc.continue_addr));
                }
            }
        }
        Ok(())
    }

    /// Compile a flow source expression into a FlowExpr.
    fn compile_flow_expr(&self, expr: &Expr) -> FlowExpr {
        match expr {
            Expr::Ident(name) => {
                if let Some(&slot) = self.global_slots.get(name) {
                    FlowExpr::GlobalSlot(slot)
                } else {
                    FlowExpr::Ident(name.clone())
                }
            }
            Expr::StringLit(s) => FlowExpr::Const(Value::String(s.clone())),
            Expr::FloatLit(f) => FlowExpr::Const(Value::Float(*f)),
            _ => FlowExpr::Ident(format!("{:?}", expr)),
        }
    }

    /// Try to evaluate an expression to a constant Value.
    fn eval_const_expr(&self, expr: &Expr) -> Value {
        match expr {
            Expr::StringLit(s) => Value::String(s.clone()),
            Expr::FloatLit(f) => Value::Float(*f),
            _ => Value::Unit,
        }
    }

    /// Compile a rule into CompiledRule.
    fn compile_rule(&self, rule: &RuleDecl) -> Result<CompiledRule, String> {
        let condition = match &rule.condition {
            Condition::Contains { left, right } => {
                RuleCondition::Contains {
                    left: self.rule_value_expr(left),
                    right: self.rule_value_expr(right),
                }
            }
            Condition::Compare { left, op, right } => {
                RuleCondition::Compare {
                    left: self.rule_value_expr(left),
                    op: match op {
                        AstCompareOp::Gt => ConditionOp::Gt,
                        AstCompareOp::Lt => ConditionOp::Lt,
                        AstCompareOp::Ge => ConditionOp::Ge,
                        AstCompareOp::Le => ConditionOp::Le,
                        AstCompareOp::Eq => ConditionOp::Eq,
                        AstCompareOp::Ne => ConditionOp::Ne,
                        _ => ConditionOp::Eq, // fallback
                    },
                    right: self.rule_value_expr(right),
                }
            }
        };

        let target_name = match &rule.target {
            Expr::Ident(name) => name.clone(),
            _ => return Err("rule target must be an identifier".to_string()),
        };

        Ok(CompiledRule {
            condition,
            target_name,
            field: rule.field.clone(),
            value_expr: self.rule_value_expr(&rule.value),
            priority: rule.priority,
        })
    }

    /// Convert an AST expression to a simplified rule value expression.
    fn rule_value_expr(&self, expr: &Expr) -> RuleValueExpr {
        match expr {
            Expr::Ident(name) => RuleValueExpr::Ident(name.clone()),
            Expr::StringLit(s) => RuleValueExpr::StringLit(s.clone()),
            Expr::FloatLit(f) => RuleValueExpr::FloatLit(*f),
            Expr::FieldAccess(base, field) => {
                if let Expr::Ident(name) = base.as_ref() {
                    RuleValueExpr::FieldAccess(name.clone(), field.clone())
                } else {
                    RuleValueExpr::Ident(format!("{:?}", expr))
                }
            }
            _ => RuleValueExpr::Ident(format!("{:?}", expr)),
        }
    }

    /// Analyze a compiled pattern body for purity.
    /// A pattern is pure if it contains only:
    ///   - LoadLocal, Const (any type), Add, Sub, Mul, Div
    ///   - CmpGt/CmpLt/CmpGe/CmpLe/CmpEq (allowed in purity, but NOT JIT-compiled)
    ///   - Return
    /// And ALL parameter types are "Float".
    /// No globals, no builtins, no LLM calls, no struct operations, no memory.
    /// Note: is_pure is a broader check than JIT-eligibility. JIT additionally
    /// requires only arithmetic (no Cmp*). See JitCompiler::is_jit_eligible().
    fn analyze_purity(code: &[Instruction], params: &[crate::ast::Param]) -> bool {
        // All params must be Float
        if params.iter().any(|p| p.type_name != "Float") {
            return false;
        }
        for instr in code {
            match instr {
                Instruction::LoadLocal(_) |
                Instruction::Const(_) |
                Instruction::Add |
                Instruction::Sub |
                Instruction::Mul |
                Instruction::Div |
                Instruction::CmpGt |
                Instruction::CmpLt |
                Instruction::CmpGe |
                Instruction::CmpLe |
                Instruction::CmpEq |
                Instruction::CmpNe |
                Instruction::Pop |
                Instruction::Return => {}
                _ => return false,
            }
        }
        true
    }

    /// Ensure a global slot exists for the given name.
    fn ensure_global(&mut self, name: &str) {
        if !self.global_slots.contains_key(name) {
            let slot = self.next_global;
            self.global_slots.insert(name.to_string(), slot);
            self.next_global += 1;
        }
    }

    /// Resolve an import: find file, parse, recursively resolve sub-imports.
    fn resolve_import(&mut self, module_path: &str) -> Result<Vec<Declaration>, String> {
        if self.imported_modules.contains(module_path) {
            return Ok(Vec::new());
        }
        self.imported_modules.insert(module_path.to_string());

        if module_path == "std/collections" {
            self.collections_loaded = true;
        }

        let file_path = self.std_root.join(module_path).with_extension("mlog");
        let source = std::fs::read_to_string(&file_path)
            .map_err(|e| format!(
                "import '{}': cannot read {:?}: {}",
                module_path, file_path, e
            ))?;

        let mut declarations = crate::parser::parse(&source)
            .map_err(|e| format!("import '{}': parse error: {}", module_path, e))?;

        let mut resolved = Vec::new();
        for decl in declarations.drain(..) {
            if let Declaration::Import(sub_import) = &decl {
                // Fix 2: use `sub_import.path` instead of `sub_import.module_path`
                if !self.imported_modules.contains(&sub_import.path) {
                    let sub_decls = self.resolve_import(&sub_import.path)?;
                    resolved.extend(sub_decls);
                }
            } else {
                resolved.push(decl);
            }
        }

        Ok(resolved)
    }
}
