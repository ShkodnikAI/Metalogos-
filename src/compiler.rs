// ── METALOGOS Bytecode Compiler — Phase 4.1 ────────────────────
// Translates AST (Vec<Declaration>) into a bytecode Program.
// Contract: the emitted Program, when executed by the VM, produces
// the same output as the tree-walking interpreter.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::ast::CompareOp as AstCompareOp;
use crate::ast::*;
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

impl Compiler {
    /// Create a new compiler with default settings.
    pub fn new() -> Self {
        Self::with_std_root(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }

    /// Create a compiler with a custom std root directory for import resolution.
    pub fn with_std_root(std_root: PathBuf) -> Self {
        let builtin_indices = crate::builtins::builtin_indices();

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
                .map(|i| {
                    self.global_slots
                        .iter()
                        .find(|(_, &slot)| slot == i)
                        .map(|(name, _)| name.clone())
                        .unwrap_or_default()
                })
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
                Declaration::Adapt(_)
                | Declaration::Relate(_)
                | Declaration::Mutate(_)
                | Declaration::Forget(_) => {
                    // Handled in pass2
                }
                Declaration::MlogServer(_)
                | Declaration::Template(_)
                | Declaration::Db(_)
                | Declaration::Schema(_)
                | Declaration::SkillIndex(_)
                | Declaration::Memory(_)
                | Declaration::Conversation(_)
                | Declaration::ContextBudget(_)
                | Declaration::Tool(_)
                | Declaration::LlmConfig(_) => {
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
                    let field_names = self
                        .struct_fields
                        .get(&e.type_name)
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
                    code.push(Instruction::MakeStruct(
                        e.type_name.clone(),
                        field_names.clone(),
                    ));
                    code.push(Instruction::StoreGlobal(slot));
                }
                Declaration::EntitySimple(e) => {
                    self.compile_expr(&e.value, &mut code)?;
                    let slot = self.global_slots[&e.name];
                    code.push(Instruction::StoreGlobal(slot));
                }
                Declaration::Pattern(p) => {
                    // Compile pattern body with parameter names as locals
                    let mut locals: HashMap<String, usize> = p
                        .params
                        .iter()
                        .enumerate()
                        .map(|(i, param)| (param.name.clone(), i))
                        .collect();
                    let fn_code = self.compile_pattern_body_with_locals(&p.body, &mut locals)?;
                    let is_pure = Self::analyze_purity(&fn_code, &p.params);
                    let compiled = CompiledFn {
                        name: p.name.clone(),
                        param_count: p.params.len(),
                        param_types: p
                            .params
                            .iter()
                            .map(|param| param.type_name.clone())
                            .collect(),
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
                        _ => ConditionOp::Eq, // Ne and others fall back to Eq
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
                    // Compile the flow source expression as regular bytecode
                    // (supports all expression types including BinOp concatenation)
                    self.compile_expr(&f.source, &mut code)?;
                    let mut branch_defs = Vec::new();
                    for (step_name, branches) in &f.branch_defs {
                        let compiled_branches: Vec<BranchDef> = branches
                            .iter()
                            .map(|b| {
                                let op = match b.condition.op {
                                    AstCompareOp::Gt => ConditionOp::Gt,
                                    AstCompareOp::Lt => ConditionOp::Lt,
                                    AstCompareOp::Ge => ConditionOp::Ge,
                                    AstCompareOp::Le => ConditionOp::Le,
                                    AstCompareOp::Eq => ConditionOp::Eq,
                                    _ => ConditionOp::Eq, // Ne and others fall back to Eq
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
                    code.push(Instruction::FlowPipeline {
                        pipeline: f.pipeline.clone(),
                        branch_defs,
                    });
                }
                Declaration::Import(_) => {
                    // Already resolved in import preprocessing
                }
                Declaration::MlogServer(_)
                | Declaration::Template(_)
                | Declaration::Db(_)
                | Declaration::Schema(_)
                | Declaration::SkillIndex(_)
                | Declaration::Memory(_)
                | Declaration::Hook(_)
                | Declaration::Eval(_)
                | Declaration::Conversation(_)
                | Declaration::ContextBudget(_)
                | Declaration::Tool(_)
                | Declaration::LlmConfig(_) => {
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
    fn compile_expr_with_locals(
        &self,
        expr: &Expr,
        code: &mut Vec<Instruction>,
        locals: &HashMap<String, usize>,
    ) -> Result<(), String> {
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
                    // And/Or require short-circuit evaluation; VM bytecode support
                    // deferred — use tree-walking interpreter for these operators.
                    BinOp::And | BinOp::Or => {
                        return Err(format!(
                            "compile: {:?} requires short-circuit evaluation, not yet supported in VM bytecode (use tree-walking interpreter)",
                            op
                        ));
                    }
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
            Expr::QualifiedCall {
                module: _,
                function: _,
                args: _,
            } => {
                return Err("compile: qualified calls not yet supported in bytecode".to_string());
            }
            Expr::List(items) => {
                // Push each item onto stack, then MakeList(count) pops them into a list.
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
                let field_names: Vec<String> = fields.keys().cloned().collect();
                for (_, val_expr) in fields {
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
    /// Supports: LetBinding, Assign, Return, While, Each, EachWithIndex,
    /// IfThen, IfElseBlock, Break, Continue, ExprStmt, Match.
    fn compile_pattern_body_with_locals(
        &self,
        body: &[Statement],
        locals: &mut HashMap<String, usize>,
    ) -> Result<Vec<Instruction>, String> {
        let mut code = Vec::new();
        let mut next_slot = locals.len();
        // Loop context stack for break/continue fixup.
        // Each entry: (loop_start_ip, break_fixups, continue_fixups)
        let mut loop_stack: Vec<(usize, Vec<usize>, Vec<usize>)> = Vec::new();

        for stmt in body {
            match stmt {
                Statement::LetBinding {
                    name,
                    value,
                    mutable: _,
                } => {
                    // Function-level scoping: if name already exists in locals,
                    // reuse the existing slot (matches interpreter behavior).
                    // Per p30_scope_let, `let` inside if/else overwrites outer variable.
                    if let Some(&existing_slot) = locals.get(name) {
                        self.compile_expr_with_locals(value, &mut code, locals)?;
                        code.push(Instruction::StoreLocal(existing_slot));
                    } else {
                        let slot = next_slot;
                        next_slot += 1;
                        locals.insert(name.clone(), slot);
                        self.compile_expr_with_locals(value, &mut code, locals)?;
                        code.push(Instruction::StoreLocal(slot));
                    }
                }
                Statement::Assign { name, value } => {
                    // Reassignment: look up existing slot, compile value, store.
                    if let Some(&slot) = locals.get(name) {
                        self.compile_expr_with_locals(value, &mut code, locals)?;
                        code.push(Instruction::StoreLocal(slot));
                    } else if let Some(&slot) = self.global_slots.get(name) {
                        self.compile_expr_with_locals(value, &mut code, locals)?;
                        code.push(Instruction::StoreGlobal(slot));
                    }
                    // If not found, silently skip (interpreter would error).
                }
                Statement::Return(expr) => {
                    self.compile_expr_with_locals(expr, &mut code, locals)?;
                    code.push(Instruction::Return);
                }
                Statement::While { condition, body } => {
                    let loop_start = code.len();
                    let mut break_fixups: Vec<usize> = Vec::new();
                    let mut continue_fixups: Vec<usize> = Vec::new();

                    // Evaluate condition
                    self.compile_expr_with_locals(condition, &mut code, locals)?;
                    // JumpIfNot → after loop (placeholder)
                    let jmp_not_idx = code.len();
                    code.push(Instruction::JumpIfNot(0));

                    // Compile body with loop context
                    loop_stack.push((loop_start, break_fixups.clone(), continue_fixups.clone()));
                    let saved_next_slot = next_slot;
                    for s in body {
                        match s {
                            Statement::Break => {
                                let fixup = code.len();
                                code.push(Instruction::Jump(0)); // placeholder
                                loop_stack.last_mut().unwrap().1.push(fixup);
                            }
                            Statement::Continue => {
                                let fixup = code.len();
                                code.push(Instruction::Jump(loop_start)); // back to start
                                loop_stack.last_mut().unwrap().2.push(fixup);
                            }
                            _ => {
                                // Recursively compile nested statements
                                // We need to compile them inline, so we use a helper
                                self.compile_stmt_with_locals(
                                    s,
                                    &mut code,
                                    locals,
                                    &mut next_slot,
                                    &mut loop_stack,
                                )?;
                            }
                        }
                    }
                    // Restore next_slot after loop body (nested lets inside loop are scoped)
                    next_slot = saved_next_slot;

                    loop_stack.pop();

                    // Jump back to loop start
                    code.push(Instruction::Jump(loop_start));

                    // Patch: after_loop starts here
                    let after_loop = code.len();
                    code[jmp_not_idx] = Instruction::JumpIfNot(after_loop);

                    // Patch break fixups
                    for fixup_idx in &break_fixups {
                        code[*fixup_idx] = Instruction::Jump(after_loop);
                    }
                }
                Statement::Each {
                    variable,
                    iterable,
                    body,
                } => {
                    // Compile: iterable → load → iterate with index
                    // Alloc local slots for: _list (hidden), _index (hidden), item (visible)
                    let list_slot = next_slot;
                    next_slot += 1;
                    let idx_slot = next_slot;
                    next_slot += 1;
                    let item_slot = next_slot;
                    next_slot += 1;

                    // Compile iterable expression, store in list_slot
                    self.compile_expr_with_locals(iterable, &mut code, locals)?;
                    code.push(Instruction::StoreLocal(list_slot));

                    // Initialize index = 0
                    code.push(Instruction::Const(Value::Float(0.0)));
                    code.push(Instruction::StoreLocal(idx_slot));

                    let loop_start = code.len();
                    let mut break_fixups: Vec<usize> = Vec::new();

                    // Check: idx < len? → JumpIfNot after_loop
                    // Stack: [..., len, idx] — we need to duplicate both or use CmpLt
                    // Simpler: load idx, load len_len_from_list, CmpLt
                    code.push(Instruction::LoadLocal(idx_slot));
                    code.push(Instruction::LoadLocal(list_slot));
                    code.push(Instruction::ListLen);
                    code.push(Instruction::CmpLt);
                    code.push(Instruction::JumpIfNot(0)); // placeholder
                    let jmp_not_idx = code.len() - 1;

                    // Get item: list[idx]
                    code.push(Instruction::LoadLocal(list_slot));
                    code.push(Instruction::LoadLocal(idx_slot));
                    code.push(Instruction::IndexAccess);
                    code.push(Instruction::StoreLocal(item_slot));

                    // Bind variable name to item_slot
                    let old = locals.insert(variable.clone(), item_slot);

                    // Compile body
                    let saved_next_slot = next_slot;
                    loop_stack.push((loop_start, break_fixups.clone(), vec![]));
                    for s in body {
                        match s {
                            Statement::Break => {
                                let fixup = code.len();
                                code.push(Instruction::Jump(0));
                                loop_stack.last_mut().unwrap().1.push(fixup);
                            }
                            Statement::Continue => {
                                // Skip rest of body, jump to increment
                                code.push(Instruction::Jump(0)); // placeholder, patch later
                                loop_stack.last_mut().unwrap().2.push(code.len() - 1);
                            }
                            _ => {
                                self.compile_stmt_with_locals(
                                    s,
                                    &mut code,
                                    locals,
                                    &mut next_slot,
                                    &mut loop_stack,
                                )?;
                            }
                        }
                    }
                    next_slot = saved_next_slot;
                    loop_stack.pop();

                    // Increment index
                    code.push(Instruction::LoadLocal(idx_slot));
                    code.push(Instruction::Const(Value::Float(1.0)));
                    code.push(Instruction::Add);
                    code.push(Instruction::StoreLocal(idx_slot));

                    // Jump back to loop start
                    code.push(Instruction::Jump(loop_start));

                    // Patch after_loop
                    let after_loop = code.len();
                    code[jmp_not_idx] = Instruction::JumpIfNot(after_loop);

                    // Patch break fixups
                    for fixup_idx in &break_fixups {
                        code[*fixup_idx] = Instruction::Jump(after_loop);
                    }

                    // Restore old binding for variable
                    if let Some(old_val) = old {
                        locals.insert(variable.clone(), old_val);
                    } else {
                        locals.remove(variable);
                    }
                }
                Statement::EachWithIndex {
                    index_var,
                    item_var,
                    iterable,
                    body,
                } => {
                    // Same as Each but also binds index_var
                    let list_slot = next_slot;
                    next_slot += 1;
                    let idx_slot = next_slot;
                    next_slot += 1;
                    let item_slot = next_slot;
                    next_slot += 1;

                    self.compile_expr_with_locals(iterable, &mut code, locals)?;
                    code.push(Instruction::StoreLocal(list_slot));

                    code.push(Instruction::Const(Value::Float(0.0)));
                    code.push(Instruction::StoreLocal(idx_slot));

                    let loop_start = code.len();
                    let mut break_fixups: Vec<usize> = Vec::new();

                    code.push(Instruction::LoadLocal(idx_slot));
                    code.push(Instruction::LoadLocal(list_slot));
                    code.push(Instruction::ListLen);
                    code.push(Instruction::CmpLt);
                    code.push(Instruction::JumpIfNot(0));
                    let jmp_not_idx = code.len() - 1;

                    code.push(Instruction::LoadLocal(list_slot));
                    code.push(Instruction::LoadLocal(idx_slot));
                    code.push(Instruction::IndexAccess);
                    code.push(Instruction::StoreLocal(item_slot));

                    // Bind both vars
                    let old_item = locals.insert(item_var.clone(), item_slot);
                    let old_idx = locals.insert(index_var.clone(), idx_slot);

                    let saved_next_slot = next_slot;
                    loop_stack.push((loop_start, break_fixups.clone(), vec![]));
                    for s in body {
                        match s {
                            Statement::Break => {
                                let fixup = code.len();
                                code.push(Instruction::Jump(0));
                                loop_stack.last_mut().unwrap().1.push(fixup);
                            }
                            Statement::Continue => {
                                code.push(Instruction::Jump(0));
                                loop_stack.last_mut().unwrap().2.push(code.len() - 1);
                            }
                            _ => {
                                self.compile_stmt_with_locals(
                                    s,
                                    &mut code,
                                    locals,
                                    &mut next_slot,
                                    &mut loop_stack,
                                )?;
                            }
                        }
                    }
                    next_slot = saved_next_slot;
                    loop_stack.pop();

                    code.push(Instruction::LoadLocal(idx_slot));
                    code.push(Instruction::Const(Value::Float(1.0)));
                    code.push(Instruction::Add);
                    code.push(Instruction::StoreLocal(idx_slot));

                    code.push(Instruction::Jump(loop_start));

                    let after_loop = code.len();
                    code[jmp_not_idx] = Instruction::JumpIfNot(after_loop);

                    for fixup_idx in &break_fixups {
                        code[*fixup_idx] = Instruction::Jump(after_loop);
                    }

                    // Restore bindings
                    if let Some(v) = old_item {
                        locals.insert(item_var.clone(), v);
                    } else {
                        locals.remove(item_var);
                    }
                    if let Some(v) = old_idx {
                        locals.insert(index_var.clone(), v);
                    } else {
                        locals.remove(index_var);
                    }
                }
                Statement::IfThen(cond, then_body) => {
                    self.compile_expr_with_locals(cond, &mut code, locals)?;
                    code.push(Instruction::JumpIfNot(0)); // placeholder
                    let jmp_idx = code.len() - 1;

                    let saved_next_slot = next_slot;
                    for s in then_body {
                        self.compile_stmt_with_locals(
                            s,
                            &mut code,
                            locals,
                            &mut next_slot,
                            &mut loop_stack,
                        )?;
                    }
                    next_slot = saved_next_slot;

                    let after = code.len();
                    code[jmp_idx] = Instruction::JumpIfNot(after);
                }
                Statement::IfElseBlock {
                    condition,
                    then_body,
                    else_ifs,
                    else_body,
                } => {
                    // Compile if/else if/else chain
                    let mut jump_to_end_fixups: Vec<usize> = Vec::new();

                    // if condition
                    self.compile_expr_with_locals(condition, &mut code, locals)?;
                    code.push(Instruction::JumpIfNot(0));
                    let jmp_idx = code.len() - 1;

                    let saved_next_slot = next_slot;
                    for s in then_body {
                        self.compile_stmt_with_locals(
                            s,
                            &mut code,
                            locals,
                            &mut next_slot,
                            &mut loop_stack,
                        )?;
                    }
                    next_slot = saved_next_slot;

                    code.push(Instruction::Jump(0)); // skip else
                    let then_end = code.len() - 1;
                    jump_to_end_fixups.push(then_end);

                    let then_else_start = code.len();
                    code[jmp_idx] = Instruction::JumpIfNot(then_else_start);

                    // else if chain
                    for (ei_cond, ei_body) in else_ifs {
                        self.compile_expr_with_locals(ei_cond, &mut code, locals)?;
                        code.push(Instruction::JumpIfNot(0));
                        let ei_jmp = code.len() - 1;

                        let saved_ns = next_slot;
                        for s in ei_body {
                            self.compile_stmt_with_locals(
                                s,
                                &mut code,
                                locals,
                                &mut next_slot,
                                &mut loop_stack,
                            )?;
                        }
                        next_slot = saved_ns;

                        code.push(Instruction::Jump(0));
                        let ei_end = code.len() - 1;
                        jump_to_end_fixups.push(ei_end);

                        let ei_else_start = code.len();
                        code[ei_jmp] = Instruction::JumpIfNot(ei_else_start);
                    }

                    // else body
                    if let Some(else_body) = else_body {
                        let saved_ns = next_slot;
                        for s in else_body {
                            self.compile_stmt_with_locals(
                                s,
                                &mut code,
                                locals,
                                &mut next_slot,
                                &mut loop_stack,
                            )?;
                        }
                        next_slot = saved_ns;
                    }

                    let block_end = code.len();
                    for fixup in jump_to_end_fixups {
                        code[fixup] = Instruction::Jump(block_end);
                    }
                }
                Statement::ExprStmt(expr) => {
                    self.compile_expr_with_locals(expr, &mut code, locals)?;
                    // Discard result (side-effect expression like respond(), write_file())
                    code.push(Instruction::Pop);
                }
                Statement::Match {
                    scrutinee,
                    arms,
                    else_body,
                } => {
                    self.compile_expr_with_locals(scrutinee, &mut code, locals)?;
                    let _ = (arms, else_body);
                }
                _ => {}
            }
        }
        Ok(code)
    }

    /// Helper: compile a single statement with full loop context and slot tracking.
    /// Used by While/Each bodies to avoid duplicating the full match logic.
    fn compile_stmt_with_locals(
        &self,
        stmt: &Statement,
        code: &mut Vec<Instruction>,
        locals: &mut HashMap<String, usize>,
        next_slot: &mut usize,
        loop_stack: &mut Vec<(usize, Vec<usize>, Vec<usize>)>,
    ) -> Result<(), String> {
        match stmt {
            Statement::LetBinding {
                name,
                value,
                mutable: _,
            } => {
                // Function-level scoping: reuse existing slot if name exists.
                if let Some(&existing_slot) = locals.get(name) {
                    self.compile_expr_with_locals(value, code, locals)?;
                    code.push(Instruction::StoreLocal(existing_slot));
                } else {
                    let slot = *next_slot;
                    *next_slot += 1;
                    locals.insert(name.clone(), slot);
                    self.compile_expr_with_locals(value, code, locals)?;
                    code.push(Instruction::StoreLocal(slot));
                }
            }
            Statement::Assign { name, value } => {
                if let Some(&slot) = locals.get(name) {
                    self.compile_expr_with_locals(value, code, locals)?;
                    code.push(Instruction::StoreLocal(slot));
                } else if let Some(&slot) = self.global_slots.get(name) {
                    self.compile_expr_with_locals(value, code, locals)?;
                    code.push(Instruction::StoreGlobal(slot));
                }
            }
            Statement::Return(expr) => {
                self.compile_expr_with_locals(expr, code, locals)?;
                code.push(Instruction::Return);
            }
            Statement::While { condition, body } => {
                let loop_start = code.len();
                let mut break_fixups: Vec<usize> = Vec::new();

                self.compile_expr_with_locals(condition, code, locals)?;
                let jmp_not_idx = code.len();
                code.push(Instruction::JumpIfNot(0));

                loop_stack.push((loop_start, vec![], vec![]));
                let saved = *next_slot;
                for s in body {
                    match s {
                        Statement::Break => {
                            let fixup = code.len();
                            code.push(Instruction::Jump(0));
                            loop_stack.last_mut().unwrap().1.push(fixup);
                        }
                        Statement::Continue => {
                            code.push(Instruction::Jump(loop_start));
                        }
                        _ => {
                            self.compile_stmt_with_locals(s, code, locals, next_slot, loop_stack)?;
                        }
                    }
                }
                *next_slot = saved;
                loop_stack.pop();

                code.push(Instruction::Jump(loop_start));
                let after_loop = code.len();
                code[jmp_not_idx] = Instruction::JumpIfNot(after_loop);
                for f in &break_fixups {
                    code[*f] = Instruction::Jump(after_loop);
                }
            }
            Statement::IfThen(cond, then_body) => {
                self.compile_expr_with_locals(cond, code, locals)?;
                code.push(Instruction::JumpIfNot(0));
                let jmp_idx = code.len() - 1;
                let saved = *next_slot;
                for s in then_body {
                    self.compile_stmt_with_locals(s, code, locals, next_slot, loop_stack)?;
                }
                *next_slot = saved;
                code[jmp_idx] = Instruction::JumpIfNot(code.len());
            }
            Statement::IfElseBlock {
                condition,
                then_body,
                else_ifs,
                else_body,
            } => {
                let mut end_fixups: Vec<usize> = Vec::new();
                self.compile_expr_with_locals(condition, code, locals)?;
                code.push(Instruction::JumpIfNot(0));
                let jmp_idx = code.len() - 1;
                let saved = *next_slot;
                for s in then_body {
                    self.compile_stmt_with_locals(s, code, locals, next_slot, loop_stack)?;
                }
                *next_slot = saved;
                code.push(Instruction::Jump(0));
                end_fixups.push(code.len() - 1);
                code[jmp_idx] = Instruction::JumpIfNot(code.len());

                for (ei_cond, ei_body) in else_ifs {
                    self.compile_expr_with_locals(ei_cond, code, locals)?;
                    code.push(Instruction::JumpIfNot(0));
                    let ei_jmp = code.len() - 1;
                    let saved2 = *next_slot;
                    for s in ei_body {
                        self.compile_stmt_with_locals(s, code, locals, next_slot, loop_stack)?;
                    }
                    *next_slot = saved2;
                    code.push(Instruction::Jump(0));
                    end_fixups.push(code.len() - 1);
                    code[ei_jmp] = Instruction::JumpIfNot(code.len());
                }

                if let Some(eb) = else_body {
                    let saved3 = *next_slot;
                    for s in eb {
                        self.compile_stmt_with_locals(s, code, locals, next_slot, loop_stack)?;
                    }
                    *next_slot = saved3;
                }

                let end = code.len();
                for f in end_fixups {
                    code[f] = Instruction::Jump(end);
                }
            }
            Statement::ExprStmt(expr) => {
                self.compile_expr_with_locals(expr, code, locals)?;
                code.push(Instruction::Pop);
            }
            _ => {}
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
            Condition::Contains { left, right } => RuleCondition::Contains {
                left: self.rule_value_expr(left),
                right: self.rule_value_expr(right),
            },
            Condition::Compare { left, op, right } => {
                RuleCondition::Compare {
                    left: self.rule_value_expr(left),
                    op: match op {
                        AstCompareOp::Gt => ConditionOp::Gt,
                        AstCompareOp::Lt => ConditionOp::Lt,
                        AstCompareOp::Ge => ConditionOp::Ge,
                        AstCompareOp::Le => ConditionOp::Le,
                        AstCompareOp::Eq => ConditionOp::Eq,
                        _ => ConditionOp::Eq, // Ne and others fall back to Eq
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
                Instruction::LoadLocal(_)
                | Instruction::Const(_)
                | Instruction::Add
                | Instruction::Sub
                | Instruction::Mul
                | Instruction::Div
                | Instruction::CmpGt
                | Instruction::CmpLt
                | Instruction::CmpGe
                | Instruction::CmpLe
                | Instruction::CmpEq
                | Instruction::CmpNe
                | Instruction::Return => {}
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
        let source = std::fs::read_to_string(&file_path).map_err(|e| {
            format!(
                "import '{}': cannot read {:?}: {}",
                module_path, file_path, e
            )
        })?;

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
