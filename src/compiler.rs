// ── METALOGOS Bytecode Compiler — Phase 4.1 ────────────────────
// Translates AST (Vec<Declaration>) into a bytecode Program.
// Contract: the emitted Program, when executed by the VM, produces
// the same output as the tree-walking interpreter.
//
// Наряд №510 (audit 28.09 C-01): `!=` compiled as `==` through the
// `_ =>` wildcard arms below. The deny lint makes the whole class
// impossible: every enum match in this file must be explicit, so a
// new enum variant is a compile error until handled.
#![deny(clippy::wildcard_enum_match_arm)]

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
    /// Наряд №415: pattern bodies collected during pass2 (emission order ==
    /// pass1's `pattern_indices` order). Becomes `Program::patterns` — the
    /// single canonical copy of every compiled body.
    pattern_bodies: Vec<CompiledFn>,
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
    /// Skill index declarations to pass to VM.
    skill_indices: Vec<crate::bytecode::CompiledSkillIndex>,
    /// Наряд №199 (ADR-0121): compiled `reflex` declarations to pass to VM.
    /// Populated in pass1 from `Declaration::Reflex(_)`. The VM processes
    /// these in `load_program` to register models in its own ReflexRegistry.
    reflex_decls: Vec<crate::bytecode::CompiledReflexDecl>,
    /// Наряд №204 (ADR-0121 stages 3-4): compiled `reflex_seq` declarations.
    /// Candle-feature-gated — only used when the VM is built with `--features candle`.
    reflex_seq_decls: Vec<crate::bytecode::CompiledReflexSeqDecl>,
    /// Наряд №204 (ADR-0121 stage 4): compiled `reflex_gen` declarations.
    /// Candle-feature-gated.
    reflex_gen_decls: Vec<crate::bytecode::CompiledReflexGenDecl>,
    /// Наряд №240 (Vision R4.2): collected `vision` declarations for the VM
    /// and the interpreter's declaration pass.
    vision_decls: Vec<crate::bytecode::CompiledVisionDecl>,
    /// Наряд №332 (ADR-0164): collected `origin` declarations.
    origin_decls: Vec<crate::bytecode::CompiledOriginDecl>,
    /// Наряд №392: compiled on_deny handlers, in declaration order.
    deny_handlers: Vec<crate::bytecode::CompiledDenyHandler>,
    /// Наряд №392: class → handler index (read during pass2 to arm
    /// SinkChecks with the deny path).
    deny_handler_indices: HashMap<String, u32>,
    /// Наряд №204 (ADR-0121 stage 2): memory persist path from `memory { persist: ... }`.
    /// Passed to the VM so reflex_save/reflex_load work without the interpreter.
    memory_persist_path: Option<String>,
    /// №521: the declared `conversation {}` config — carried to the Program
    /// so the VM lane's conv_* builtins read the DECLARED values (before
    /// this field the VM used the defaults unconditionally).
    conversation_config: crate::interpreter::types::ConversationConfig,
    /// Database URL extracted from db declaration (for VM).
    db_url: Option<String>,
    /// №758: the NAME part of `db { url: env("NAME") }` — resolved by
    /// the VM at the first db access (runtime semantics, no credential
    /// in the bytecode).
    db_url_env: Option<String>,
    /// Schema DDL statements from schema declarations.
    schema_ddl: Vec<String>,
    /// Root directory for import resolution.
    std_root: PathBuf,
    /// Already-imported modules.
    imported_modules: HashSet<String>,
    /// Import alias → module path mapping (e.g., "str" → "std/string").
    import_aliases: HashMap<String, String>,
    /// Collections loaded flag.
    collections_loaded: bool,
    /// Sandbox declarations (recorded).
    sandboxes: HashMap<String, SandboxDecl>,
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

// ── Runtime label emission (Наряд №328, ADR-0156) ────────────────────
//
// The compiler lowers the static №323/№325 knowledge into runtime
// instructions: a `let`/assignment from a №316 Source call carries a
// LabelJoin (the runtime label env is seeded), and every sink call site
// gets a SinkCheck for each identifier/direct-source argument. The
// static gate remains the SSOT — the runtime twin agrees by
// construction and rejects any divergence loudly (ADR-0156 §2).

fn is_source_call(name: &str) -> bool {
    matches!(
        crate::builtins_classification::classify(name).map(|c| c.role),
        Some(crate::builtins_classification::Role::Source)
    )
}

fn is_sink_call(name: &str) -> bool {
    matches!(
        crate::builtins_classification::classify(name).map(|c| c.role),
        Some(crate::builtins_classification::Role::Sink)
    )
}

/// Emit a SinkCheck for the argument when it is trackable at runtime:
/// a variable (by name) or a direct source call (`@name`).
/// Наряд №392: when the program declares an on_deny handler covering the
/// sink's class, `deny_handler` carries its index and each emitted
/// SinkCheck is armed with the deny path (skip_to patched later, once
/// the refused call's continuation address is known). Returns the
/// indices of the emitted SinkCheck instructions for that patching.
fn emit_sink_checks(
    code: &mut Vec<Instruction>,
    fn_name: &str,
    args: &[crate::ast::Expr],
    line: u32,
    deny_handler: Option<u32>,
) -> Vec<usize> {
    let mut emitted = Vec::new();
    for (i, a) in args.iter().enumerate() {
        let trackable = match a {
            crate::ast::Expr::Ident { name, .. } => Some(name.clone()),
            crate::ast::Expr::FnCall { name, .. } if is_source_call(name) => {
                Some(format!("@{name}"))
            }
            // №510: explicit fall-through — a new Expr variant must be
            // consciously reviewed for sink-tracking (the deny lint makes
            // adding a variant here a compile error until it is handled).
            crate::ast::Expr::StringLit { .. }
            | crate::ast::Expr::FloatLit { .. }
            | crate::ast::Expr::BoolLit { .. }
            | crate::ast::Expr::FieldAccess { .. }
            | crate::ast::Expr::FnCall { .. }
            | crate::ast::Expr::QualifiedCall { .. }
            | crate::ast::Expr::BinaryOp { .. }
            | crate::ast::Expr::IfElse { .. }
            | crate::ast::Expr::List { .. }
            | crate::ast::Expr::IndexAccess { .. }
            | crate::ast::Expr::StructLit { .. }
            | crate::ast::Expr::BlockIfElse { .. }
            | crate::ast::Expr::MatchExpr { .. }
            | crate::ast::Expr::Try { .. }
            | crate::ast::Expr::HandleSource { .. }
            | crate::ast::Expr::ProvBind { .. } => None,
        };
        if let Some(arg) = trackable {
            emitted.push(code.len());
            code.push(Instruction::SinkCheck(Box::new(SinkCheckData {
                fn_name: fn_name.to_string(),
                arg,
                line: line.max(1),
                // №392: the argument position feeds the SAME reason
                // classification the static gate uses (the network
                // address-position rule).
                arg_index: i as u32,
                deny: deny_handler.map(|handler| SinkDenyPath {
                    handler,
                    skip_to: 0,
                }),
            })));
        }
    }
    emitted
}

impl Compiler {
    /// Наряд №392: compile the `on_deny(<class|*>) { body }` handlers —
    /// zero-arg zero-result code ending in `Const(Unit); Return`, run by
    /// the VM's deny path with the CallPattern frame discipline. The
    /// class → index map arms SinkCheck emission during pass2.
    fn compile_deny_handlers(&mut self, declarations: &[Declaration]) -> Result<(), String> {
        for (i, decl) in declarations.iter().enumerate() {
            let Declaration::OnDeny(d) = decl else {
                continue;
            };
            let mut locals: HashMap<String, usize> = HashMap::new();
            let mut mutable: HashSet<String> = HashSet::new();
            let mut next_slot = 0usize;
            let mut loop_stack: Vec<(usize, Vec<usize>, Vec<usize>)> = Vec::new();
            let mut code = Vec::new();
            for stmt in &d.body {
                self.compile_stmt_with_locals(
                    stmt,
                    &mut code,
                    &mut locals,
                    &mut next_slot,
                    &mut loop_stack,
                    &mut mutable,
                )?;
            }
            // The handler's value is discarded by the deny path; end with
            // an explicit Unit so a fall-through body still returns.
            code.push(Instruction::const_(Value::Unit));
            code.push(Instruction::Return);
            self.deny_handler_indices
                .insert(d.class.clone(), self.deny_handlers.len() as u32);
            self.deny_handlers
                .push(crate::bytecode::CompiledDenyHandler {
                    class: d.class.clone(),
                    name: format!("__on_deny_{}", i),
                    code,
                });
        }
        Ok(())
    }

    /// Наряд №392: emit the SinkChecks for a direct sink call in
    /// statement position, armed with the on_deny path when the program
    /// declares a covering handler for the sink's class. Returns the
    /// emitted SinkCheck indices (skip_to patched once the continuation
    /// is known).
    fn emit_armed_sink_checks(
        &self,
        code: &mut Vec<Instruction>,
        expr: &crate::ast::Expr,
    ) -> Vec<usize> {
        if let crate::ast::Expr::FnCall {
            name, args, span, ..
        } = expr
        {
            if is_sink_call(name) {
                let class = crate::audit::sink_kind(name);
                let handler = self.deny_handler_indices.get(class).copied();
                return emit_sink_checks(code, name, args, span.start_line, handler);
            }
        }
        Vec::new()
    }

    /// Наряд №392: patch the deny paths of the given SinkChecks to jump
    /// to `skip_to` (the instruction that consumes the degraded Unit —
    /// the Pop or Return that follows the refused call).
    fn patch_deny_skip_to(code: &mut [Instruction], indices: &[usize], skip_to: usize) {
        for &i in indices {
            if let Instruction::SinkCheck(sc) = &mut code[i] {
                if let Some(path) = sc.deny.as_mut() {
                    path.skip_to = skip_to as u32;
                }
            }
        }
    }

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
            pattern_bodies: Vec::new(),
            pattern_indices: HashMap::new(),
            learnable_indices: HashMap::new(),
            builtin_indices,
            struct_fields: HashMap::new(),
            rules: Vec::new(),
            skill_indices: Vec::new(),
            reflex_decls: Vec::new(),
            reflex_seq_decls: Vec::new(),
            reflex_gen_decls: Vec::new(),
            vision_decls: Vec::new(),
            origin_decls: Vec::new(),
            deny_handlers: Vec::new(),
            deny_handler_indices: HashMap::new(),
            memory_persist_path: None,
            conversation_config: Default::default(),
            db_url: None,
            db_url_env: None,
            schema_ddl: Vec::new(),
            std_root,
            imported_modules: HashSet::new(),
            import_aliases: HashMap::new(),
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
                // Track alias mapping (e.g., "str" → "std/string")
                if let Some(alias) = &import.alias {
                    self.import_aliases
                        .insert(alias.clone(), import.path.clone());
                }
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

        // Pass 1.5 (№392): compile the on_deny handler bodies — after the
        // global slots are assigned (a handler may store into a global)
        // and before pass2 arms SinkChecks with the handler indices.
        self.compile_deny_handlers(&all_decls)?;

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
            patterns: std::sync::Arc::new(std::mem::take(&mut self.pattern_bodies)), // №415: filled from the pass2 collector (was: always empty)
            learnables: Vec::new(),
            rules: std::mem::take(&mut self.rules),
            skill_indices: std::mem::take(&mut self.skill_indices),
            reflex_decls: std::mem::take(&mut self.reflex_decls),
            reflex_seq_decls: std::mem::take(&mut self.reflex_seq_decls),
            reflex_gen_decls: std::mem::take(&mut self.reflex_gen_decls),
            vision_decls: std::mem::take(&mut self.vision_decls),
            origin_decls: std::mem::take(&mut self.origin_decls),
            deny_handlers: std::mem::take(&mut self.deny_handlers),
            db_url: self.db_url.take(),
            db_url_env: self.db_url_env.take(),
            memory_persist_path: self.memory_persist_path.take(),
            conversation_config: self.conversation_config.clone(),
            schema_ddl: std::mem::take(&mut self.schema_ddl),
            main_code,
            collections_loaded: self.collections_loaded,
            shared_cache: crate::bytecode::ProgramSharedCache::new(),
        };

        Ok(program)
    }

    /// Pass 1: collect type info, register patterns, assign global slots.
    fn pass1(&mut self, decls: &[Declaration]) -> Result<(), String> {
        for decl in decls {
            match decl {
                // №325: the compatibility profile is a compile-time
                // declaration — nothing to emit.
                Declaration::Profile(_) => {}
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
                Declaration::SkillIndex(si) => {
                    let compiled = crate::bytecode::CompiledSkillIndex {
                        name: si.name.clone(),
                        budget: si.budget,
                        tiers: si
                            .tiers
                            .iter()
                            .map(|t| crate::bytecode::CompiledSkillTier {
                                level: t.level,
                                mode: t.mode.clone(),
                                skills: t.skills.clone(),
                                rules: t
                                    .rules
                                    .iter()
                                    .map(|r| crate::bytecode::CompiledSkillTriggerRule {
                                        skill: r.skill.clone(),
                                        triggers: r.triggers.clone(),
                                    })
                                    .collect(),
                            })
                            .collect(),
                    };
                    self.skill_indices.push(compiled);
                }
                Declaration::Memorize(_) => {
                    // Handled in pass2
                }
                Declaration::Db(db) => {
                    // №758: classify the URL source for the VM lane.
                    // - string literal → recorded as-is (unchanged);
                    // - env("NAME") → the NAME is recorded, the URL is
                    //   resolved by the VM at the first db access (the
                    //   interpreter's runtime semantics, and the resolved
                    //   value — possibly a credentialed Postgres URL —
                    //   never lands in the bytecode);
                    // - anything else → a LOUD compile error: the old
                    //   silent None produced the mystery "no database
                    //   connection" on every request (the issue's exact
                    //   complaint); failing at compile time names the fix.
                    match &db.url {
                        Some(crate::ast::Expr::StringLit { value: url, .. }) => {
                            self.db_url = Some(url.clone());
                        }
                        Some(crate::ast::Expr::FnCall { name, args, .. }) if name == "env" => {
                            match args.first() {
                                Some(crate::ast::Expr::StringLit { value: var, .. }) => {
                                    self.db_url_env = Some(var.clone());
                                }
                                _ => {
                                    return Err(
                                        "db { url: env(...) } requires a string literal name \
                                         (db { url: env(\"VAR\") }) — the VM cannot compile a \
                                         non-literal env argument"
                                            .to_string(),
                                    );
                                }
                            }
                        }
                        Some(_) => {
                            return Err("the VM compiles only db { url: \"literal\" } or \
                                 db { url: env(\"NAME\") } — other expressions are an \
                                 interpreter-backend construct (the old silent path left the VM \
                                 without a connection)"
                                .to_string());
                        }
                        None => {}
                    }
                }
                Declaration::Schema(schema) => {
                    // Generate CREATE TABLE IF NOT EXISTS DDL for each table
                    for table in &schema.tables {
                        let cols: Vec<String> = table
                            .columns
                            .iter()
                            .map(|c| {
                                let sql_type = match c.col_type.as_str() {
                                    "Int" | "Float" => "INTEGER",
                                    "String" | "Text" => "TEXT",
                                    "DateTime" => "TEXT",
                                    "Bool" => "INTEGER",
                                    _ => "TEXT",
                                };
                                let mut parts = vec![format!("{} {}", c.name, sql_type)];
                                for m in &c.modifiers {
                                    match m {
                                        crate::ast::ColumnModifier::PrimaryKey => {
                                            parts.push("PRIMARY KEY".to_string());
                                        }
                                        crate::ast::ColumnModifier::AutoIncrement => {
                                            parts.push("AUTOINCREMENT".to_string());
                                        }
                                        crate::ast::ColumnModifier::Nullable => {
                                            parts.push("NULL".to_string());
                                        }
                                        crate::ast::ColumnModifier::References(t, f) => {
                                            parts.push(format!("REFERENCES {}({})", t, f));
                                        }
                                    }
                                }
                                if let Some(def) = &c.default {
                                    if def == "now()" {
                                        parts.push("DEFAULT (datetime('now'))".to_string());
                                    } else {
                                        let val = def.trim_matches('"');
                                        parts.push(format!("DEFAULT '{}'", val));
                                    }
                                }
                                parts.join(" ")
                            })
                            .collect();
                        let ddl = format!(
                            "CREATE TABLE IF NOT EXISTS {} ({})",
                            table.name,
                            cols.join(", ")
                        );
                        self.schema_ddl.push(ddl);
                    }
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
                // №521: Conversation is NOT in this bucket anymore — the
                // declared config rides the Program now (see the dedicated
                // arm below; the bucket ignored it and the VM lane kept
                // the defaults unconditionally).
                | Declaration::ContextBudget(_)
                | Declaration::TypeAlias(_)
                | Declaration::Tool(_)
                | Declaration::LlmConfig(_) => {
                    // Phase 6+: handled elsewhere
                }
                // Наряд №204 (ADR-0121 stage 2): extract memory persist path
                // from `memory { persist: "path.db" }` declaration. The VM
                // needs this for reflex_save/reflex_load.
                Declaration::Memory(m) => {
                    if let Some(ref persist) = m.persist {
                        self.memory_persist_path = Some(persist.clone());
                    }
                }
                // №521: the declared conversation config rides the Program
                // to the VM lane (the interpreter applies its own copy in
                // `run`; the VM has no interpreter to read it from).
                Declaration::Conversation(c) => {
                    self.conversation_config =
                        crate::interpreter::types::ConversationConfig {
                            ttl: c.ttl,
                            max_messages: c.max_messages,
                            compress_after: c.compress_after,
                        };
                }
                // Наряд №204 (ADR-0121 stage 3): collect `reflex_seq`
                // declarations for the VM. Candle-feature-gated — the VM
                // only registers these when `--features candle`.
                Declaration::ReflexSeq(r) => {
                    self.reflex_seq_decls
                        .push(crate::bytecode::CompiledReflexSeqDecl {
                            name: r.name.clone(),
                            input_dim: r.input_dim,
                            seq_len: r.seq_len,
                            layers: r
                                .layers
                                .iter()
                                .map(|l| crate::bytecode::CompiledReflexLayerSpec {
                                    name: l.name.clone(),
                                    args: l.args.clone(),
                                })
                                .collect(),
                            labels: r.labels.clone(),
                            seed: r.seed,
                        });
                }
                // Наряд №204 (ADR-0121 stage 4): collect `reflex_gen`
                // declarations for the VM. Candle-feature-gated.
                Declaration::ReflexGen(r) => {
                    self.reflex_gen_decls
                        .push(crate::bytecode::CompiledReflexGenDecl {
                            name: r.name.clone(),
                            input_dim: r.input_dim,
                            vocab_size: r.vocab_size,
                            layers: r
                                .layers
                                .iter()
                                .map(|l| crate::bytecode::CompiledReflexLayerSpec {
                                    name: l.name.clone(),
                                    args: l.args.clone(),
                                })
                                .collect(),
                            seed: r.seed,
                        });
                }
                // Наряд №199 (ADR-0121): collect `reflex` declarations
                // (Dense classification only) for the VM. reflex_seq and
                // reflex_gen remain excluded until stages 3-4.
                Declaration::Reflex(r) => {
                    self.reflex_decls.push(crate::bytecode::CompiledReflexDecl {
                        name: r.name.clone(),
                        input_dim: r.input_dim,
                        layers: r
                            .layers
                            .iter()
                            .map(|l| crate::bytecode::CompiledReflexLayerSpec {
                                name: l.name.clone(),
                                args: l.args.clone(),
                            })
                            .collect(),
                        labels: r.labels.clone(),
                        seed: r.seed,
                    });
                }
                // Наряд №240 (Vision R4.2): collect `vision` declarations for
                // the dispatch (fields 1:1 with AST R4.1, single conversion
                // point `CompiledVisionDecl::from_ast`). No bytecode is
                // emitted in pass2 (лекало reflex) — registration happens in
                // `Vm::load_program` / the interpreter's declaration pass.
                Declaration::Vision(v) => {
                    self.vision_decls
                        .push(crate::bytecode::CompiledVisionDecl::from_ast(v));
                }
                // Наряд №332 (ADR-0164): collect origin declarations for
                // the media_source_capture dispatch (shape validated in
                // semantic; the conversion re-checks the required fields).
                Declaration::Origin(o) => {
                    let compiled = crate::bytecode::CompiledOriginDecl::from_ast(o)
                        .map_err(|e| format!("compile: {}", e))?;
                    self.origin_decls.push(compiled);
                }
                // №510: explicit no-op set — declarations compiled elsewhere
                // or needing no pass-1 emission; a new Declaration variant
                // must be consciously routed.
                Declaration::Import(_)
                | Declaration::Hook(_)
                | Declaration::OnDeny(_)
                | Declaration::Eval(_)
                | Declaration::Test(_)
                | Declaration::Flow(_) => {}
            }
        }
        Ok(())
    }

    /// Pass 2: generate main_code instructions.
    fn pass2(&mut self, decls: &[Declaration]) -> Result<Vec<Instruction>, String> {
        let mut code = Vec::new();

        for decl in decls {
            match decl {
                // №325: the compatibility profile is a compile-time
                // declaration — nothing to emit.
                Declaration::Profile(_) => {}
                // №392: deny handlers were compiled in pass 1.5 — nothing
                // to emit into main_code.
                Declaration::OnDeny(_) => {}
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
                            None => code.push(Instruction::const_(Value::Unit)),
                        }
                    }

                    let slot = self.global_slots[&e.name];
                    code.push(Instruction::make_struct(
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
                    // Наряд №264: params are NOT mutable (TW: mutable_vars
                    // starts empty per pattern invocation).
                    let mut mutable: HashSet<String> = HashSet::new();
                    let fn_code =
                        self.compile_pattern_body_with_locals(&p.body, &mut locals, &mut mutable)?;
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
                    // Naryad №415: the body goes into the Program::patterns TABLE
                    // (exactly once) and main_code carries a 4-byte index. The
                    // collector order == pass1's pattern_indices order (both walk
                    // all_decls top-to-bottom), so the positional CallPattern
                    // indices resolve to the same bodies as before.
                    let idx = self.pattern_bodies.len();
                    self.pattern_bodies.push(compiled);
                    code.push(Instruction::RegisterPatternRef(idx as u32));
                }
                Declaration::LearnablePattern(lp) => {
                    // Compile context mode
                    let context_mode = match &lp.context {
                        Some(ContextMode::None) => crate::bytecode::CompiledContextMode::None,
                        Some(ContextMode::Literal(s)) => {
                            crate::bytecode::CompiledContextMode::Literal(s.clone())
                        }
                        Some(ContextMode::Auto) => crate::bytecode::CompiledContextMode::Auto,
                        Some(ContextMode::Recall(expr, limit)) => {
                            // Extract param name from expression (must be Ident)
                            let param_name = match expr {
                                crate::ast::Expr::Ident { name, .. } => name.clone(),
                                crate::ast::Expr::FieldAccess {
                                    object: _obj,
                                    field,
                                    ..
                                } => {
                                    // e.g., text.some_field — just use field name
                                    field.clone()
                                }
                                // №510: explicit fall-through — only the
                                // recall parameter naming cases above are
                                // special; everything else uses "input".
                                crate::ast::Expr::StringLit { .. }
                                | crate::ast::Expr::FloatLit { .. }
                                | crate::ast::Expr::BoolLit { .. }
                                | crate::ast::Expr::FnCall { .. }
                                | crate::ast::Expr::QualifiedCall { .. }
                                | crate::ast::Expr::BinaryOp { .. }
                                | crate::ast::Expr::IfElse { .. }
                                | crate::ast::Expr::List { .. }
                                | crate::ast::Expr::IndexAccess { .. }
                                | crate::ast::Expr::StructLit { .. }
                                | crate::ast::Expr::BlockIfElse { .. }
                                | crate::ast::Expr::MatchExpr { .. }
                                | crate::ast::Expr::Try { .. }
                                | crate::ast::Expr::HandleSource { .. }
                                | crate::ast::Expr::ProvBind { .. } => "input".to_string(),
                            };
                            crate::bytecode::CompiledContextMode::Recall(
                                param_name,
                                limit.unwrap_or(5),
                            )
                        }
                        None => crate::bytecode::CompiledContextMode::None,
                    };
                    code.push(Instruction::register_learnable(CompiledLearnableInfo {
                        name: lp.name.clone(),
                        param_count: lp.params.len(),
                        prompt: lp.prompt.clone(),
                        few_shot: Vec::new(),
                        context_mode,
                        // Наряд №205 (ADR-0121 stage 6): pass distillation
                        // fields through to the VM.
                        distill_to: lp.distill_to.clone(),
                        distill_after: lp.distill_after,
                        fallback_if: lp.fallback_if.map(|(op, v)| {
                            // Convert ast::CompareOp → bytecode::ConditionOp
                            use crate::bytecode::ConditionOp;
                            match op {
                                crate::ast::CompareOp::Gt => (ConditionOp::Gt, v),
                                crate::ast::CompareOp::Lt => (ConditionOp::Lt, v),
                                crate::ast::CompareOp::Ge => (ConditionOp::Ge, v),
                                crate::ast::CompareOp::Le => (ConditionOp::Le, v),
                                crate::ast::CompareOp::Eq => (ConditionOp::Eq, v),
                                crate::ast::CompareOp::Ne => (ConditionOp::Ne, v),
                            }
                        }),
                        // №456: the holdout-accuracy gate passes through to
                        // the VM (None → the 0.85 default at the runtime site).
                        distill_min_accuracy: lp.distill_min_accuracy,
                        distill_margin: lp.distill_margin,
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
                        code.push(Instruction::const_(Value::Float(v.confidence)));
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
                    code.push(Instruction::const_(Value::String(r.relation.clone())));
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
                        // №510: `Ne` was swallowed by a wildcard arm and
                        // compiled as `Eq` — rollback fired on the wrong
                        // side of the comparison. Now explicit.
                        AstCompareOp::Gt => ConditionOp::Gt,
                        AstCompareOp::Lt => ConditionOp::Lt,
                        AstCompareOp::Ge => ConditionOp::Ge,
                        AstCompareOp::Le => ConditionOp::Le,
                        AstCompareOp::Eq => ConditionOp::Eq,
                        AstCompareOp::Ne => ConditionOp::Ne,
                    });
                    code.push(Instruction::Mutate(Box::new(MutateData {
                        pattern_name: m.pattern_name.clone(),
                        example_count: m.new_examples.len(),
                        rollback_threshold: m.rollback_threshold,
                        rollback_op,
                    })));
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
                                    // №510: explicit `Ne` — same wildcard
                                    // defect as rollback_if (C-01).
                                    AstCompareOp::Gt => ConditionOp::Gt,
                                    AstCompareOp::Lt => ConditionOp::Lt,
                                    AstCompareOp::Ge => ConditionOp::Ge,
                                    AstCompareOp::Le => ConditionOp::Le,
                                    AstCompareOp::Eq => ConditionOp::Eq,
                                    AstCompareOp::Ne => ConditionOp::Ne,
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
                    code.push(Instruction::FlowPipeline(Box::new(FlowPipelineData {
                        pipeline: f.pipeline.clone(),
                        branch_defs,
                    })));
                }
                Declaration::Import(_) => {
                    // Already resolved in import preprocessing
                }
                Declaration::Template(t) => {
                    // Наряд №250 (ADR-0122 #208): register templates at
                    // COMPILE time. The VM serve path (Vm::new + load_program
                    // + execute_route_code) has no declaration walk, so the
                    // interpreter's runtime registration (execution.rs on
                    // Declaration::Template) never runs there and
                    // `render("Name", ...)` failed with "unknown template"
                    // (repro: vm_golden p115_render_basic — the n206 ignore
                    // note "template registration not being wired in the VM
                    // path"). GLOBAL_TEMPLATES is the №115 TW/VM-parity
                    // channel in builtins::http; register_template is
                    // overwrite-idempotent (it stores the same data the
                    // interpreter registers at declaration-execution time),
                    // and `mlog serve` compiles in-process, so compile-time
                    // registration covers every VM request without touching
                    // bytecode.rs (the Program schema stays frozen; a
                    // templates field would break old-.mbc deserialize).
                    // Pre-existing gap, unchanged here: `run`/`serve` FROM a
                    // deserialized .mbc cannot register templates for EITHER
                    // backend (Program carries no template data) — revisit
                    // with a bytecode schema change if a real use case asks.
                    let param_names: Vec<String> =
                        t.params.iter().map(|p| p.name.clone()).collect();
                    crate::builtins::http::register_template(&t.name, &t.body, param_names);
                }
                Declaration::MlogServer(_)
                | Declaration::Db(_)
                | Declaration::Schema(_)
                | Declaration::SkillIndex(_)
                | Declaration::Memory(_)
                | Declaration::Hook(_)
                | Declaration::Eval(_)
                | Declaration::Test(_)
                | Declaration::Conversation(_)
                | Declaration::ContextBudget(_)
                | Declaration::TypeAlias(_)
                | Declaration::Tool(_)
                | Declaration::LlmConfig(_)
                | Declaration::Reflex(_)
                | Declaration::ReflexSeq(_)
                | Declaration::ReflexGen(_)
                // Наряд №238 (Vision R4.1), updated №240 (R4.2): vision
                // declarations carry no bytecode — dispatch goes via
                // `program.vision_decls` (populated in pass1, consumed by
                // `Vm::load_program` and the interpreter's declaration pass,
                // лекало reflex_decls). Minimal arm forced by the
                // exhaustive match.
                // Наряд №332 (ADR-0164): origin declarations carry no
                // bytecode — registration via `program.origin_decls`
                // (лекало vision_decls).
                | Declaration::Vision(_)
                | Declaration::Origin(_) => {
                    // Наряд №203 Block 1: no bytecode instruction emitted
                    // for reflex declarations in pass2. Dense classification
                    // (Declaration::Reflex) is handled via program.reflex_decls
                    // (populated in pass1, consumed by Vm::load_program).
                    // ReflexSeq/ReflexGen are VM-unsupported pending ADR-0121
                    // stages 3-4 — diagnostic trace emitted in pass1 above.
                }
            }
        }

        code.push(Instruction::Halt);
        Ok(code)
    }

    /// Compile an AST expression into stack instructions (no locals context).
    /// №370: the scratch next_slot starts ABOVE the globals snapshot — the
    /// wrapper serves top-level declaration initializers (main-code
    /// execution, bp=0), so hidden slots must not overlap global slots.
    fn compile_expr(&self, expr: &Expr, code: &mut Vec<Instruction>) -> Result<(), String> {
        let mut scratch: HashMap<String, usize> = HashMap::new();
        let mut next_slot = self.next_global.max(self.global_slots.len());
        let mut loop_stack: Vec<(usize, Vec<usize>, Vec<usize>)> = Vec::new();
        let mut mutable: HashSet<String> = HashSet::new();
        self.compile_expr_with_locals(
            expr,
            code,
            &mut scratch,
            &mut next_slot,
            &mut loop_stack,
            &mut mutable,
        )
    }

    /// Compile an AST expression into stack instructions with a locals map.
    /// №370: `locals` is mutable and `next_slot` is threaded through — the
    /// BlockIfElse value compilation allocates HIDDEN scratch slots
    /// (`#`-names, impossible in user IDENTs) for its last-value register,
    /// exactly like the №369 match-expr machinery. Expression positions can
    /// nest arbitrarily deep (primary_expr), so the slot counter must follow.
    fn compile_expr_with_locals(
        &self,
        expr: &Expr,
        code: &mut Vec<Instruction>,
        locals: &mut HashMap<String, usize>,
        next_slot: &mut usize,
        loop_stack: &mut Vec<(usize, Vec<usize>, Vec<usize>)>,
        mutable: &mut HashSet<String>,
    ) -> Result<(), String> {
        match expr {
            Expr::StringLit { value: s, .. } => {
                code.push(Instruction::const_(Value::String(s.clone())));
            }
            Expr::FloatLit { value: f, .. } => {
                code.push(Instruction::const_(Value::Float(*f)));
            }
            Expr::Ident { name, .. } => {
                // Check if it's a local (parameter or let binding) first
                if let Some(&slot) = locals.get(name) {
                    code.push(Instruction::LoadLocal(slot));
                } else if let Some(&slot) = self.global_slots.get(name) {
                    code.push(Instruction::LoadGlobal(slot));
                } else {
                    code.push(Instruction::LoadGlobalByName(name.clone()));
                }
            }
            Expr::FieldAccess {
                object: base,
                field,
                ..
            } => {
                self.compile_expr_with_locals(base, code, locals, next_slot, loop_stack, mutable)?;
                code.push(Instruction::GetField(field.clone()));
            }
            Expr::FnCall { name, args, .. } => {
                for arg in args {
                    self.compile_expr_with_locals(
                        arg, code, locals, next_slot, loop_stack, mutable,
                    )?;
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
                    // №479 (ADR-0131): stable diagnostic code at the origin —
                    // the diff-fuzzer's class signature compares codes, not prose.
                    return Err(crate::interpreter::values::coded_error(
                        crate::interpreter::values::CODE_UNDEFINED_FUNCTION,
                        format!("compile: undefined function: {}", name),
                    ));
                }
            }
            Expr::BinaryOp {
                left, op, right, ..
            } => match op {
                BinOp::And | BinOp::Or => {
                    // Short-circuit evaluation — result is always Value::Bool.
                    // Must NOT eagerly compile both operands.
                    if matches!(op, BinOp::And) {
                        // And: compile left, if falsy → false, else check right
                        self.compile_expr_with_locals(
                            left, code, locals, next_slot, loop_stack, mutable,
                        )?;
                        let jump_to_false_1 = code.len();
                        code.push(Instruction::JumpIfNot(0)); // placeholder
                        self.compile_expr_with_locals(
                            right, code, locals, next_slot, loop_stack, mutable,
                        )?;
                        let jump_to_false_2 = code.len();
                        code.push(Instruction::JumpIfNot(0)); // placeholder
                        code.push(Instruction::const_(Value::Bool(true)));
                        let jump_to_end = code.len();
                        code.push(Instruction::Jump(0)); // placeholder
                                                         // L_false:
                        let l_false = code.len();
                        if let Some(Instruction::JumpIfNot(ref mut t)) =
                            code.get_mut(jump_to_false_1)
                        {
                            *t = l_false;
                        }
                        if let Some(Instruction::JumpIfNot(ref mut t)) =
                            code.get_mut(jump_to_false_2)
                        {
                            *t = l_false;
                        }
                        code.push(Instruction::const_(Value::Bool(false)));
                        // L_end:
                        let l_end = code.len();
                        if let Some(Instruction::Jump(ref mut t)) = code.get_mut(jump_to_end) {
                            *t = l_end;
                        }
                    } else {
                        // Or: compile left, if truthy → true, else check right
                        self.compile_expr_with_locals(
                            left, code, locals, next_slot, loop_stack, mutable,
                        )?;
                        let jump_to_check_right = code.len();
                        code.push(Instruction::JumpIfNot(0)); // placeholder
                                                              // left is truthy
                        code.push(Instruction::const_(Value::Bool(true)));
                        let jump_to_end_1 = code.len();
                        code.push(Instruction::Jump(0)); // placeholder
                                                         // L_check_right:
                        let l_check_right = code.len();
                        if let Some(Instruction::JumpIfNot(ref mut t)) =
                            code.get_mut(jump_to_check_right)
                        {
                            *t = l_check_right;
                        }
                        self.compile_expr_with_locals(
                            right, code, locals, next_slot, loop_stack, mutable,
                        )?;
                        let jump_to_false = code.len();
                        code.push(Instruction::JumpIfNot(0)); // placeholder
                                                              // right is truthy
                        code.push(Instruction::const_(Value::Bool(true)));
                        let jump_to_end_2 = code.len();
                        code.push(Instruction::Jump(0)); // placeholder
                                                         // L_false:
                        let l_false = code.len();
                        if let Some(Instruction::JumpIfNot(ref mut t)) = code.get_mut(jump_to_false)
                        {
                            *t = l_false;
                        }
                        code.push(Instruction::const_(Value::Bool(false)));
                        // L_end:
                        let l_end = code.len();
                        if let Some(Instruction::Jump(ref mut t)) = code.get_mut(jump_to_end_1) {
                            *t = l_end;
                        }
                        if let Some(Instruction::Jump(ref mut t)) = code.get_mut(jump_to_end_2) {
                            *t = l_end;
                        }
                    }
                }
                // №510: explicit arithmetic/comparison fall-through — the
                // short-circuit And/Or arms are handled above; anything else
                // must be consciously classified.
                BinOp::Add
                | BinOp::Sub
                | BinOp::Mul
                | BinOp::Div
                | BinOp::Gt
                | BinOp::Lt
                | BinOp::Ge
                | BinOp::Le
                | BinOp::Eq
                | BinOp::Ne => {
                    self.compile_expr_with_locals(
                        left, code, locals, next_slot, loop_stack, mutable,
                    )?;
                    self.compile_expr_with_locals(
                        right, code, locals, next_slot, loop_stack, mutable,
                    )?;
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
                        BinOp::And | BinOp::Or => unreachable!(),
                    }
                }
            },
            Expr::IfElse {
                condition: cond,
                then_branch: then_expr,
                else_branch: else_expr,
                ..
            } => {
                // Compile condition
                self.compile_expr_with_locals(cond, code, locals, next_slot, loop_stack, mutable)?;
                // Jump to else branch if falsy
                let jump_to_else = code.len();
                code.push(Instruction::JumpIfNot(0)); // placeholder
                                                      // Compile then branch
                self.compile_expr_with_locals(
                    then_expr, code, locals, next_slot, loop_stack, mutable,
                )?;
                // Jump past else branch
                let jump_to_end = code.len();
                code.push(Instruction::Jump(0)); // placeholder
                                                 // Patch: else branch starts here
                let else_start = code.len();
                if let Some(Instruction::JumpIfNot(ref mut target)) = code.get_mut(jump_to_else) {
                    *target = else_start;
                }
                // Compile else branch
                self.compile_expr_with_locals(
                    else_expr, code, locals, next_slot, loop_stack, mutable,
                )?;
                // Patch: end jump target
                let end = code.len();
                if let Some(Instruction::Jump(ref mut target)) = code.get_mut(jump_to_end) {
                    *target = end;
                }
            }
            // Additional expression forms in Metalogos- AST
            Expr::BoolLit { value: b, .. } => {
                // Наряд №250: compile bool literals as Value::Bool. The old
                // Float(1.0/0.0) encoding lost the type: make_list(true, "a", 3)
                // produced [1.0, "a", 3.0] on the VM and sort() diverged from
                // TW ([1,3,a] vs [3,a,true] — repro p118_collection_utils).
                // is_truthy / eval_cmp / shared builtins all handle Bool; the
                // .mbc format is unchanged (Value::Bool already serializable);
                // old .mbc files keep their float encoding and behave as
                // before. TW is unaffected (it never used this path).
                code.push(Instruction::const_(Value::Bool(*b)));
            }
            Expr::QualifiedCall {
                module,
                function,
                args,
                ..
            } => {
                // Resolve the qualified call:
                // Priority 1: builtin "module.function" (e.g., "std/math.abs" if registered)
                // Priority 2: imported pattern "function" (e.g., abs from std/math)
                // Priority 3: builtin "function" (global fallback)
                let qualified_name = format!("{}.{}", module, function);
                if let Some(&idx) = self.builtin_indices.get(&qualified_name) {
                    code.push(Instruction::CallBuiltin(idx, args.len()));
                } else if let Some(&idx) = self.pattern_indices.get(function) {
                    code.push(Instruction::CallPattern(idx, args.len()));
                } else if let Some(&idx) = self.builtin_indices.get(function) {
                    code.push(Instruction::CallBuiltin(idx, args.len()));
                } else {
                    return Err(format!(
                        "compile: qualified call '{}.{}()' — not found",
                        module, function
                    ));
                }
            }
            Expr::List { items, .. } => {
                // Push each item onto stack, then MakeList(count) pops them into a list.
                for item in items {
                    self.compile_expr_with_locals(
                        item, code, locals, next_slot, loop_stack, mutable,
                    )?;
                }
                code.push(Instruction::MakeList(items.len()));
            }
            Expr::IndexAccess {
                object: base,
                index,
                ..
            } => {
                self.compile_expr_with_locals(base, code, locals, next_slot, loop_stack, mutable)?;
                self.compile_expr_with_locals(index, code, locals, next_slot, loop_stack, mutable)?;
                code.push(Instruction::IndexAccess);
            }
            Expr::StructLit { fields, .. } => {
                let field_names: Vec<String> = fields.keys().cloned().collect();
                for val_expr in fields.values() {
                    self.compile_expr_with_locals(
                        val_expr, code, locals, next_slot, loop_stack, mutable,
                    )?;
                }
                code.push(Instruction::make_struct("Struct".to_string(), field_names));
            }
            // №370: if/else as a VALUE (ADR-0141 Stage 1.2). NO new opcode —
            // the jump structure (Jump/JumpIfNot) plus the №369 last-value
            // register (StoreLastLocal into a hidden slot) express it
            // exactly; the VM dispatch is unchanged by construction (nothing
            // new to dispatch — the naryad's "dispatch ×2" is vacuously
            // satisfied, recorded here and in the report). TW parity: the
            // value is the branch's last non-Unit expression (the
            // eval_statements contract); branches run against a CLONED env
            // in TW (lets do not leak — the №14 P0-3 precedent, shared with
            // the №369 match-expr arms); nothing matched and no else → Unit.
            // A `return` inside a branch is captured as the block value
            // (the expression channel cannot carry a control signal — same
            // as BlockIfElse TW eval and MatchExpr).
            Expr::BlockIfElse {
                condition,
                ref then_body,
                ref else_ifs,
                ref else_body,
                ..
            } => {
                // The branch value lives in a VM-STATE register (Begin/End
                // pair) — safe in any expression position.
                code.push(Instruction::BeginValueExpr);
                let mut end_fixups: Vec<usize> = Vec::new();
                // then branch
                self.compile_expr_with_locals(
                    condition, code, locals, next_slot, loop_stack, mutable,
                )?;
                code.push(Instruction::JumpIfNot(0));
                let mut jmp_idx = code.len() - 1;
                let saved = *next_slot;
                self.compile_match_expr_arm_body(
                    then_body, code, locals, next_slot, loop_stack, mutable,
                )?;
                *next_slot = saved;
                code.push(Instruction::Jump(0));
                end_fixups.push(code.len() - 1);
                code[jmp_idx] = Instruction::JumpIfNot(code.len());
                // else-if chain (same lazy per-branch condition evaluation
                // as TW: an else-if condition is only evaluated when
                // reached).
                for (ei_cond, ei_body) in else_ifs {
                    self.compile_expr_with_locals(
                        ei_cond, code, locals, next_slot, loop_stack, mutable,
                    )?;
                    code.push(Instruction::JumpIfNot(0));
                    jmp_idx = code.len() - 1;
                    let saved = *next_slot;
                    self.compile_match_expr_arm_body(
                        ei_body, code, locals, next_slot, loop_stack, mutable,
                    )?;
                    *next_slot = saved;
                    code.push(Instruction::Jump(0));
                    end_fixups.push(code.len() - 1);
                    code[jmp_idx] = Instruction::JumpIfNot(code.len());
                }
                // else branch (or stay Unit)
                if let Some(eb) = else_body {
                    self.compile_match_expr_arm_body(
                        eb, code, locals, next_slot, loop_stack, mutable,
                    )?;
                }
                let end = code.len();
                for f in end_fixups {
                    code[f] = Instruction::Jump(end);
                }
                // The taken branch's value: pop the register onto the stack.
                code.push(Instruction::EndValueExpr);
            }
            // №369: the match EXPRESSION form is compiled natively — but only
            // in let-binding position (the grammar's only match_expr site),
            // handled by `compile_let_match` via the LetBinding arms of the
            // two statement compilers. If this arm is ever reached, a new
            // grammar position started producing MatchExpr — fail LOUDLY
            // instead of silently mis-compiling.
            Expr::MatchExpr { .. } => {
                return Err("compile: match expression outside let binding — \
                     unsupported position (№369 compiles let-bound match only)"
                    .into());
            }
            // Наряд №91: try expression — real compilation for VM
            // Compile inner expression into a separate instruction block,
            // wrapped in TryEval so the VM can catch errors locally.
            Expr::Try { expr: inner, .. } => {
                let mut inner_code = Vec::new();
                self.compile_expr_with_locals(
                    inner,
                    &mut inner_code,
                    locals,
                    next_slot,
                    loop_stack,
                    mutable,
                )?;
                code.push(Instruction::TryEval(inner_code));
            }
            // Наряд №332 (ADR-0164): HandleSource lowers to the
            // state-carrying `media_source_capture(origin_name)` — the
            // interpreter/VM interception resolves the origin declaration
            // and captures through the media store (kind camera is a loud
            // PARKED boundary at runtime).
            Expr::HandleSource { origin, .. } => {
                let idx = *self
                    .builtin_indices
                    .get("media_source_capture")
                    .ok_or_else(|| {
                        "compile: media_source_capture not registered (registry invariant)"
                            .to_string()
                    })?;
                code.push(Instruction::const_(Value::String(origin.clone())));
                code.push(Instruction::CallBuiltin(idx, 1));
            }
            // Наряд №332 (ADR-0164): ProvBind lowers to
            // `media_bind_origin(origin_name, handle)` — evaluates the
            // construction, then binds the store entry's origin (and
            // joins the declared origin conf into the entry label).
            Expr::ProvBind { origin, inner, .. } => {
                let idx = *self
                    .builtin_indices
                    .get("media_bind_origin")
                    .ok_or_else(|| {
                        "compile: media_bind_origin not registered (registry invariant)".to_string()
                    })?;
                code.push(Instruction::const_(Value::String(origin.clone())));
                self.compile_expr_with_locals(inner, code, locals, next_slot, loop_stack, mutable)?;
                code.push(Instruction::CallBuiltin(idx, 2));
            }
        }
        Ok(())
    }

    /// Compile a pattern body with parameter names as locals.
    /// Supports: LetBinding, Assign, Return, While, Each, EachWithIndex,
    /// IfThen, IfElseBlock, Break, Continue, ExprStmt, Match.
    ///
    /// Наряд №264: `mutable` tracks which locals were bound with `let mut`
    /// (flat, never popped — TW's exact model, execution.rs:946). An
    /// assignment to any other name is a COMPILE ERROR: the compiler knows
    /// mut-ness at compile time, so a program the semantics already
    /// rejected must not serialize into a silently-executing .mbc.
    fn compile_pattern_body_with_locals(
        &self,
        body: &[Statement],
        locals: &mut HashMap<String, usize>,
        mutable: &mut HashSet<String>,
    ) -> Result<Vec<Instruction>, String> {
        let mut code = Vec::new();
        let mut next_slot = locals.len();
        // Loop context stack for break/continue fixup.
        // Each entry: (loop_start_ip, break_fixups, continue_fixups)
        let mut loop_stack: Vec<(usize, Vec<usize>, Vec<usize>)> = Vec::new();

        // №369/№250: the FINAL statement's value is the body's fall-through
        // value (execute_code: Ok(stack.pop())) — a match statement in that
        // position must keep the matched arm's trailing value.
        let last_stmt_idx = body.len().saturating_sub(1);
        for (stmt_idx, stmt) in body.iter().enumerate() {
            let is_last = stmt_idx == last_stmt_idx;
            match stmt {
                Statement::LetBinding {
                    name,
                    value,
                    mutable: is_mut,
                    ..
                } if matches!(value, Expr::MatchExpr { .. }) => {
                    // №369: the match EXPRESSION form compiles natively
                    // (ADR-0141 Stage 1.1) — full arm structure, TW parity.
                    let Expr::MatchExpr {
                        scrutinee,
                        arms,
                        else_body,
                        ..
                    } = value
                    else {
                        unreachable!("guard guarantees MatchExpr")
                    };
                    self.compile_let_match(
                        name,
                        *is_mut,
                        scrutinee,
                        arms,
                        else_body,
                        &mut code,
                        locals,
                        &mut next_slot,
                        &mut loop_stack,
                        mutable,
                    )?;
                }
                Statement::LetBinding {
                    name,
                    value,
                    mutable: is_mut,
                    ..
                } => {
                    // Function-level scoping: if name already exists in locals,
                    // reuse the existing slot (matches interpreter behavior).
                    // Per p30_scope_let, `let` inside if/else overwrites outer variable.
                    if *is_mut {
                        mutable.insert(name.clone());
                    }
                    // №328: seed the runtime label env for source-backed lets.
                    if let crate::ast::Expr::FnCall { name: src, .. } = value {
                        if is_source_call(src) {
                            code.push(Instruction::LabelJoin(Box::new(LabelJoinData {
                                dst: name.clone(),
                                src: format!("@{src}"),
                            })));
                        }
                    }
                    if let Some(&existing_slot) = locals.get(name) {
                        self.compile_expr_with_locals(
                            value,
                            &mut code,
                            locals,
                            &mut next_slot,
                            &mut loop_stack,
                            mutable,
                        )?;
                        code.push(Instruction::StoreLocal(existing_slot));
                    } else {
                        let slot = next_slot;
                        next_slot += 1;
                        locals.insert(name.clone(), slot);
                        self.compile_expr_with_locals(
                            value,
                            &mut code,
                            locals,
                            &mut next_slot,
                            &mut loop_stack,
                            mutable,
                        )?;
                        code.push(Instruction::StoreLocal(slot));
                    }
                }
                Statement::Assign { name, value, .. } => {
                    // Наряд №264: assignment to a non-`let mut` name is a
                    // compile error (previously compiled SILENTLY into
                    // StoreLocal/StoreGlobal — the VM violated the №14
                    // immutability contract that TW enforces at runtime).
                    // Order mirrors TW: mutability is checked before the
                    // name is resolved, so globals and never-declared names
                    // get the same immutability message (TW mutable_vars is
                    // function-local — globals are never in it).
                    if !mutable.contains(name) {
                        return Err(crate::semantic::immutability_error_text(name));
                    }
                    let slot = match locals.get(name) {
                        Some(&slot) => slot,
                        None => return Err(crate::semantic::immutability_error_text(name)),
                    };
                    // Reassignment to a `let mut` local: StoreAssignLocal
                    // carries the mutability fact so the VM can backstop
                    // bytecode produced past this check (VM loud, never
                    // silent — see bytecode.rs StoreAssignLocal).
                    self.compile_expr_with_locals(
                        value,
                        &mut code,
                        locals,
                        &mut next_slot,
                        &mut loop_stack,
                        mutable,
                    )?;
                    code.push(Instruction::store_assign_local(slot, name.clone(), true));
                }
                Statement::Return { value: expr, .. } => {
                    self.compile_expr_with_locals(
                        expr,
                        &mut code,
                        locals,
                        &mut next_slot,
                        &mut loop_stack,
                        mutable,
                    )?;
                    code.push(Instruction::Return);
                }
                Statement::While {
                    condition, body, ..
                } => {
                    let loop_start = code.len();
                    let break_fixups: Vec<usize> = Vec::new();
                    let continue_fixups: Vec<usize> = Vec::new();

                    // Evaluate condition
                    self.compile_expr_with_locals(
                        condition,
                        &mut code,
                        locals,
                        &mut next_slot,
                        &mut loop_stack,
                        mutable,
                    )?;
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
                                if let Some(entry) = loop_stack.last_mut() {
                                    entry.1.push(fixup);
                                }
                            }
                            Statement::Continue => {
                                let fixup = code.len();
                                code.push(Instruction::Jump(loop_start)); // back to start
                                if let Some(entry) = loop_stack.last_mut() {
                                    entry.2.push(fixup);
                                }
                            }
                            Statement::LetBinding { .. }
                            | Statement::Assign { .. }
                            | Statement::Each { .. }
                            | Statement::EachWithIndex { .. }
                            | Statement::While { .. }
                            | Statement::IfElseBlock { .. }
                            | Statement::IfThen { .. }
                            | Statement::Return { .. }
                            | Statement::ExprStmt { .. }
                            | Statement::Match { .. }
                            | Statement::Memorize(_)
                            | Statement::Forget(_)
                            | Statement::Relate(_) => {
                                // Recursively compile nested statements
                                // We need to compile them inline, so we use a helper
                                self.compile_stmt_with_locals(
                                    s,
                                    &mut code,
                                    locals,
                                    &mut next_slot,
                                    &mut loop_stack,
                                    mutable,
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
                    ..
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
                    self.compile_expr_with_locals(
                        iterable,
                        &mut code,
                        locals,
                        &mut next_slot,
                        &mut loop_stack,
                        mutable,
                    )?;
                    code.push(Instruction::StoreLocal(list_slot));

                    // Initialize index = 0
                    code.push(Instruction::const_(Value::Float(0.0)));
                    code.push(Instruction::StoreLocal(idx_slot));

                    let loop_start = code.len();
                    let break_fixups: Vec<usize> = Vec::new();

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
                                if let Some(entry) = loop_stack.last_mut() {
                                    entry.1.push(fixup);
                                }
                            }
                            Statement::Continue => {
                                // Skip rest of body, jump to increment
                                code.push(Instruction::Jump(0)); // placeholder, patch later
                                if let Some(entry) = loop_stack.last_mut() {
                                    entry.2.push(code.len() - 1);
                                }
                            }
                            // №510: explicit fall-through — the loop compiler
                            // special-cases break/continue only; every other
                            // statement goes to the generic compiler.
                            Statement::LetBinding { .. }
                            | Statement::Assign { .. }
                            | Statement::Each { .. }
                            | Statement::EachWithIndex { .. }
                            | Statement::While { .. }
                            | Statement::IfElseBlock { .. }
                            | Statement::IfThen { .. }
                            | Statement::Return { .. }
                            | Statement::ExprStmt { .. }
                            | Statement::Match { .. }
                            | Statement::Memorize(_)
                            | Statement::Forget(_)
                            | Statement::Relate(_) => {
                                self.compile_stmt_with_locals(
                                    s,
                                    &mut code,
                                    locals,
                                    &mut next_slot,
                                    &mut loop_stack,
                                    mutable,
                                )?;
                            }
                        }
                    }
                    next_slot = saved_next_slot;
                    loop_stack.pop();

                    // Increment index
                    code.push(Instruction::LoadLocal(idx_slot));
                    code.push(Instruction::const_(Value::Float(1.0)));
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
                    ..
                } => {
                    // Same as Each but also binds index_var
                    let list_slot = next_slot;
                    next_slot += 1;
                    let idx_slot = next_slot;
                    next_slot += 1;
                    let item_slot = next_slot;
                    next_slot += 1;

                    self.compile_expr_with_locals(
                        iterable,
                        &mut code,
                        locals,
                        &mut next_slot,
                        &mut loop_stack,
                        mutable,
                    )?;
                    code.push(Instruction::StoreLocal(list_slot));

                    code.push(Instruction::const_(Value::Float(0.0)));
                    code.push(Instruction::StoreLocal(idx_slot));

                    let loop_start = code.len();
                    let break_fixups: Vec<usize> = Vec::new();

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
                                if let Some(entry) = loop_stack.last_mut() {
                                    entry.1.push(fixup);
                                }
                            }
                            Statement::Continue => {
                                code.push(Instruction::Jump(0));
                                if let Some(entry) = loop_stack.last_mut() {
                                    entry.2.push(code.len() - 1);
                                }
                            }
                            // №510: explicit fall-through — the loop compiler
                            // special-cases break/continue only; every other
                            // statement goes to the generic compiler.
                            Statement::LetBinding { .. }
                            | Statement::Assign { .. }
                            | Statement::Each { .. }
                            | Statement::EachWithIndex { .. }
                            | Statement::While { .. }
                            | Statement::IfElseBlock { .. }
                            | Statement::IfThen { .. }
                            | Statement::Return { .. }
                            | Statement::ExprStmt { .. }
                            | Statement::Match { .. }
                            | Statement::Memorize(_)
                            | Statement::Forget(_)
                            | Statement::Relate(_) => {
                                self.compile_stmt_with_locals(
                                    s,
                                    &mut code,
                                    locals,
                                    &mut next_slot,
                                    &mut loop_stack,
                                    mutable,
                                )?;
                            }
                        }
                    }
                    next_slot = saved_next_slot;
                    loop_stack.pop();

                    code.push(Instruction::LoadLocal(idx_slot));
                    code.push(Instruction::const_(Value::Float(1.0)));
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
                Statement::IfThen {
                    condition: cond,
                    body: then_body,
                    ..
                } => {
                    // №574: the if-form compilation lives in ONE place with
                    // the keep-tail rule (the two arms differ only in shape).
                    self.compile_if_stmt_with_keep(
                        cond,
                        then_body,
                        &[],
                        None,
                        false,
                        is_last,
                        &mut code,
                        locals,
                        &mut next_slot,
                        &mut loop_stack,
                        mutable,
                    )?;
                }
                Statement::IfElseBlock {
                    condition,
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    self.compile_if_stmt_with_keep(
                        condition,
                        then_body,
                        else_ifs.as_slice(),
                        else_body.as_deref(),
                        true,
                        is_last,
                        &mut code,
                        locals,
                        &mut next_slot,
                        &mut loop_stack,
                        mutable,
                    )?;
                }
                Statement::ExprStmt { expr, .. } => {
                    self.compile_expr_with_locals(
                        expr,
                        &mut code,
                        locals,
                        &mut next_slot,
                        &mut loop_stack,
                        mutable,
                    )?;
                    // Discard result (side-effect expression like respond(), write_file())
                    code.push(Instruction::Pop);
                }
                Statement::Match {
                    scrutinee,
                    arms,
                    else_body,
                    ..
                } => {
                    // №369: Match statement → bytecode (ADR-0141 Stage 1.1) —
                    // the TW-only gap is closed; see compile_match_stmt.
                    // №250 parity: as the body's final statement the matched
                    // arm's trailing value is the fall-through value.
                    self.compile_match_stmt(
                        scrutinee,
                        arms,
                        else_body,
                        &mut code,
                        locals,
                        &mut next_slot,
                        &mut loop_stack,
                        mutable,
                        is_last,
                    )?;
                }
                // Наряд №266: memory ops as statements — the VM already has the
                // opcodes (Memorize/Forget/Relate execute the same stores the
                // top-level declarations compile to in pass2); emit them with
                // locals-aware expressions so pattern params/locals resolve.
                Statement::Memorize(m) => {
                    self.compile_expr_with_locals(
                        &m.value,
                        &mut code,
                        locals,
                        &mut next_slot,
                        &mut loop_stack,
                        mutable,
                    )?;
                    code.push(Instruction::Memorize(m.priority));
                }
                Statement::Forget(f) => {
                    self.compile_expr_with_locals(
                        &f.query,
                        &mut code,
                        locals,
                        &mut next_slot,
                        &mut loop_stack,
                        mutable,
                    )?;
                    code.push(Instruction::Forget(f.days));
                }
                Statement::Relate(r) => {
                    self.compile_expr_with_locals(
                        &r.from,
                        &mut code,
                        locals,
                        &mut next_slot,
                        &mut loop_stack,
                        mutable,
                    )?;
                    self.compile_expr_with_locals(
                        &r.to,
                        &mut code,
                        locals,
                        &mut next_slot,
                        &mut loop_stack,
                        mutable,
                    )?;
                    code.push(Instruction::const_(Value::String(r.relation.clone())));
                    code.push(Instruction::Relate);
                }
                // №510: explicit no-op set — loop-control statements are
                // handled by the enclosing loop compiler.
                Statement::Break | Statement::Continue => {}
            }
        }
        // Наряд №250 (ADR-0122 #208): TW-parity for the BODY VALUE. The
        // tree-walking interpreter evaluates a statement block to the value
        // of its LAST statement — route bodies rely on this: `respond(...)`
        // as the final statement IS the route's response (server.rs
        // execute_route_body). The ExprStmt compilation discards the value
        // with a trailing Pop, so the VM's fall-through return
        // (execute_code: Ok(stack.pop())) returned an arbitrary leftover
        // LOCAL instead (repro: the realistic dispatcher route returned the
        // raw query_param value "hi" instead of the HttpResponse). Drop the
        // FINAL Pop so the last statement's value stays on the stack — the
        // fall-through return then yields exactly what TW yields. Interior
        // statements keep their Pop; explicit `return` bodies are untouched.
        //
        // №582 (the audit d63cc1d X-2): the route-epilogue INVARIANT — the
        // body exit's stack.pop() NEVER reads a local slot. The local slots
        // LIVE on the stack (StoreLocal resizes it), so a body whose final
        // statement leaves no value (a let/assign tail, a loop tail, a
        // kept if/match whose branches were patched — see the keep-tail
        // synthesis) ended its fall-through pop ON THE LAST LOCAL SLOT and
        // serialized it as the response (the data-leak class: `route {
        // let row = query_row(...) }` returned the row). The dispatch:
        //   - ExprStmt tail → the №250 keep applies (its value survives);
        //   - Return/Break/Continue tail → the body exits on that path, no
        //     epilogue value is owed;
        //   - if/match tail → the keep-tail synthesis guarantees exactly
        //     one value on every live path (see compile_if_stmt_with_keep /
        //     compile_match_stmt) — nothing to add;
        //   - everything else (let/assign/while/each/memorize/…) → the
        //     epilogue `PushUnit`: the fall-through is Unit, the same `200
        //     OK` the tree-walking lane yields.
        match body.last() {
            None => {
                // An empty body — the pop still happens at the exit.
                code.push(Instruction::PushUnit);
            }
            Some(Statement::ExprStmt { .. }) => {
                if matches!(code.last(), Some(Instruction::Pop)) {
                    code.pop();
                }
            }
            Some(Statement::Return { .. } | Statement::Break | Statement::Continue) => {}
            Some(
                Statement::IfThen { .. } | Statement::IfElseBlock { .. } | Statement::Match { .. },
            ) => {
                // The keep-tail synthesis already put exactly one value on
                // every live path (or the paths all terminate).
            }
            Some(_) => {
                code.push(Instruction::PushUnit);
            }
        }
        Ok(code)
    }

    // ── Match → bytecode (№369, ADR-0141 Stage 1.1) ─────────────────

    /// №369: allocate a fresh hidden local slot for match scratch values.
    /// Hidden names contain '#' — impossible in a source-level IDENT — so
    /// they can never collide with user variables; slots are reclaimed by
    /// the caller's `next_slot` restore exactly like the existing if/else
    /// body discipline (their live ranges stay nested inside the match
    /// extent, so reuse is safe).
    fn alloc_hidden_slot(
        locals: &mut HashMap<String, usize>,
        next_slot: &mut usize,
        tag: &str,
    ) -> usize {
        let name = format!("{}#{}", tag, next_slot);
        let slot = *next_slot;
        *next_slot += 1;
        locals.insert(name, slot);
        slot
    }

    /// №370: VALUE-MODE statement compilation inside value branches (the
    /// bodies of a match-expr arm or a BlockIfElse branch). TW's
    /// `eval_statements_cf` leaks the trailing value of block statements
    /// (if/else, match) into `last_expr_value` — in a VALUE context that
    /// leak IS the observable value, so the block-statement forms must
    /// route their branch values into the same last-value register.
    /// ExprStmt keeps its value (StoreLastLocal); IfThen/IfElseBlock/Match
    /// compile their jump structures with value-mode bodies recursively;
    /// everything else falls through to the ordinary statement compiler
    /// (lets/assigns never touch the register — TW parity).
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn compile_value_stmt(
        &self,
        stmt: &Statement,
        code: &mut Vec<Instruction>,
        locals: &mut HashMap<String, usize>,
        next_slot: &mut usize,
        loop_stack: &mut Vec<(usize, Vec<usize>, Vec<usize>)>,
        mutable: &mut HashSet<String>,
    ) -> Result<(), String> {
        match stmt {
            Statement::ExprStmt { expr, .. } => {
                self.compile_expr_with_locals(expr, code, locals, next_slot, loop_stack, mutable)?;
                code.push(Instruction::KeepLastValue);
            }
            Statement::IfThen {
                condition: cond,
                body: then_body,
                ..
            } => {
                self.compile_expr_with_locals(cond, code, locals, next_slot, loop_stack, mutable)?;
                code.push(Instruction::JumpIfNot(0));
                let jmp_idx = code.len() - 1;
                let saved = *next_slot;
                for s in then_body {
                    self.compile_value_stmt(s, code, locals, next_slot, loop_stack, mutable)?;
                }
                *next_slot = saved;
                code[jmp_idx] = Instruction::JumpIfNot(code.len());
            }
            Statement::IfElseBlock {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                let mut end_fixups: Vec<usize> = Vec::new();
                self.compile_expr_with_locals(
                    condition, code, locals, next_slot, loop_stack, mutable,
                )?;
                code.push(Instruction::JumpIfNot(0));
                let mut jmp_idx = code.len() - 1;
                let saved = *next_slot;
                for s in then_body {
                    self.compile_value_stmt(s, code, locals, next_slot, loop_stack, mutable)?;
                }
                *next_slot = saved;
                code.push(Instruction::Jump(0));
                end_fixups.push(code.len() - 1);
                code[jmp_idx] = Instruction::JumpIfNot(code.len());
                for (ei_cond, ei_body) in else_ifs {
                    self.compile_expr_with_locals(
                        ei_cond, code, locals, next_slot, loop_stack, mutable,
                    )?;
                    code.push(Instruction::JumpIfNot(0));
                    jmp_idx = code.len() - 1;
                    let saved = *next_slot;
                    for s in ei_body {
                        self.compile_value_stmt(s, code, locals, next_slot, loop_stack, mutable)?;
                    }
                    *next_slot = saved;
                    code.push(Instruction::Jump(0));
                    end_fixups.push(code.len() - 1);
                    code[jmp_idx] = Instruction::JumpIfNot(code.len());
                }
                if let Some(eb) = else_body {
                    for s in eb {
                        self.compile_value_stmt(s, code, locals, next_slot, loop_stack, mutable)?;
                    }
                }
                let end = code.len();
                for f in end_fixups {
                    code[f] = Instruction::Jump(end);
                }
            }
            Statement::Match {
                scrutinee,
                arms,
                else_body,
                ..
            } => {
                // Value-mode match statement (№369 machinery, shared
                // predicates): the scrutinee lives ON THE STACK (single
                // evaluation, Dup per test — no scratch slot, safe in any
                // expression position); the matched arm's trailing value
                // lands in the OPEN value register via KeepLastValue (TW
                // eval_block! semantics).
                self.compile_expr_with_locals(
                    scrutinee, code, locals, next_slot, loop_stack, mutable,
                )?;
                let mut end_fixups: Vec<usize> = Vec::new();
                for arm in arms {
                    code.push(Instruction::Dup);
                    let test = match arm {
                        MatchArm::Exact(s, _) => MatchTest::Exact(s.clone()),
                        MatchArm::StartsWith(s, _) => MatchTest::StartsWith(s.clone()),
                        MatchArm::Contains(s, _) => MatchTest::Contains(s.clone()),
                        MatchArm::Compare(op, threshold, _) => {
                            self.compile_expr_with_locals(
                                threshold, code, locals, next_slot, loop_stack, mutable,
                            )?;
                            MatchTest::Compare(*op)
                        }
                    };
                    code.push(Instruction::match_test(test));
                    code.push(Instruction::JumpIfNot(0));
                    let jmp_idx = code.len() - 1;
                    let saved = *next_slot;
                    for st in arm.body() {
                        self.compile_value_stmt(st, code, locals, next_slot, loop_stack, mutable)?;
                    }
                    *next_slot = saved;
                    code.push(Instruction::Jump(0));
                    end_fixups.push(code.len() - 1);
                    code[jmp_idx] = Instruction::JumpIfNot(code.len());
                }
                if let Some(eb) = else_body {
                    for st in eb {
                        self.compile_value_stmt(st, code, locals, next_slot, loop_stack, mutable)?;
                    }
                }
                let end = code.len();
                for f in end_fixups {
                    code[f] = Instruction::Jump(end);
                }
                // Drop the scrutinee — the value travels in the register.
                code.push(Instruction::Pop);
            }
            // №510: explicit fall-through — pattern-body scope handles the
            // value-carrying statements above; the rest compile generically.
            other @ Statement::LetBinding { .. }
            | other @ Statement::Assign { .. }
            | other @ Statement::Each { .. }
            | other @ Statement::EachWithIndex { .. }
            | other @ Statement::While { .. }
            | other @ Statement::Return { .. }
            | other @ Statement::Break
            | other @ Statement::Continue
            | other @ Statement::Memorize(_)
            | other @ Statement::Forget(_)
            | other @ Statement::Relate(_) => {
                self.compile_stmt_with_locals(other, code, locals, next_slot, loop_stack, mutable)?;
            }
        }
        Ok(())
    }

    /// №369 + №370: TW-parity branch-body compilation, SHARED by the
    /// match-expr arms and the BlockIfElse branches: every bare expression
    /// statement stores its value into the branch's last-value slot
    /// (conditionally on non-Unit — `StoreLastLocal`) instead of being
    /// popped, so the branch's value is the last non-Unit expression of
    /// its body — exactly the TW `eval_statements_cf` contract
    /// (`if !matches!(val, Value::Unit) { last_expr_value = val }`). A
    /// trailing Unit-valued statement does not reset the value;
    /// `let`/`assign` statements never touch it.
    #[allow(clippy::too_many_arguments)]
    fn compile_match_expr_arm_body(
        &self,
        body: &[Statement],
        code: &mut Vec<Instruction>,
        locals: &mut HashMap<String, usize>,
        next_slot: &mut usize,
        loop_stack: &mut Vec<(usize, Vec<usize>, Vec<usize>)>,
        mutable: &mut HashSet<String>,
    ) -> Result<(), String> {
        for s in body {
            match s {
                // №622 (gh#1085): a `return` inside a value-channel arm
                // body is CAPTURED as the block value — the TW contract
                // (eval_statements flattens ControlFlow::Return(v) into
                // Ok(v); the expression channel cannot carry a control
                // signal — the documented BlockIfElse / MatchExpr
                // semantics). The legacy fallthrough emitted the
                // function-level Instruction::Return: the VM terminated
                // the pattern where the TW continued (the parity gap the
                // №622 scan pinned: `let v = match x { "a" then { return
                // 5.0 } }` answered "5" on the VM while the TW answered
                // the captured "5" and ran the tail).
                Statement::Return { value, .. } => {
                    self.compile_expr_with_locals(
                        value, code, locals, next_slot, loop_stack, mutable,
                    )?;
                    code.push(Instruction::SetValueReg);
                }
                // №510 posture: no wildcard — every remaining statement
                // form is named, a new variant forces this list to grow
                // consciously.
                other @ Statement::LetBinding { .. }
                | other @ Statement::Assign { .. }
                | other @ Statement::Each { .. }
                | other @ Statement::EachWithIndex { .. }
                | other @ Statement::While { .. }
                | other @ Statement::IfElseBlock { .. }
                | other @ Statement::IfThen { .. }
                | other @ Statement::ExprStmt { .. }
                | other @ Statement::Match { .. }
                | other @ Statement::Break
                | other @ Statement::Continue
                | other @ Statement::Memorize(_)
                | other @ Statement::Forget(_)
                | other @ Statement::Relate(_) => {
                    self.compile_value_stmt(other, code, locals, next_slot, loop_stack, mutable)?;
                }
            }
        }
        Ok(())
    }

    /// №369: `let x = match y { ... }` — the match EXPRESSION form compiled
    /// natively (ADR-0141 Stage 1.1 row 1). Semantics (TW parity, REFERENCE
    /// §Match): the scrutinee is evaluated EXACTLY ONCE (hidden slot — a
    /// side-effecting scrutinee must not re-run per arm); arms are tested
    /// in source order (first match wins); the let value is the last
    /// non-Unit expression of the matched arm's body; no match and no
    /// else → Unit. `Return` inside an arm body follows the VM's existing
    /// block-expression model (the value-channel cannot carry a control
    /// signal — same as `Expr::BlockIfElse`, the №14 P0-3 precedent).
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn compile_let_match(
        &self,
        name: &str,
        is_mut: bool,
        scrutinee: &Expr,
        arms: &[MatchArm],
        else_body: &Option<Vec<Statement>>,
        code: &mut Vec<Instruction>,
        locals: &mut HashMap<String, usize>,
        next_slot: &mut usize,
        loop_stack: &mut Vec<(usize, Vec<usize>, Vec<usize>)>,
        mutable: &mut HashSet<String>,
    ) -> Result<(), String> {
        if is_mut {
            mutable.insert(name.to_string());
        }
        // Function-level scoping: reuse the existing slot for `name`
        // (matches both compilers' plain-LetBinding behavior).
        let let_slot = match locals.get(name) {
            Some(&slot) => slot,
            None => {
                let slot = *next_slot;
                *next_slot += 1;
                locals.insert(name.to_string(), slot);
                slot
            }
        };
        // №370 rework: the value travels in a VM-STATE register (Begin/End
        // pair) — no scratch slots, safe in any position; the scrutinee
        // lives ON THE STACK, evaluated exactly once, Dup'd per test.
        code.push(Instruction::BeginValueExpr);
        self.compile_expr_with_locals(scrutinee, code, locals, next_slot, loop_stack, mutable)?;
        let mut end_fixups: Vec<usize> = Vec::new();
        for arm in arms {
            // Threshold expressions are compiled LAZILY, per arm, in
            // source order — the TW loop evaluates a Compare threshold
            // only when the loop reaches that arm (side-effect parity).
            code.push(Instruction::Dup);
            let test = match arm {
                MatchArm::Exact(s, _) => MatchTest::Exact(s.clone()),
                MatchArm::StartsWith(s, _) => MatchTest::StartsWith(s.clone()),
                MatchArm::Contains(s, _) => MatchTest::Contains(s.clone()),
                MatchArm::Compare(op, threshold, _) => {
                    self.compile_expr_with_locals(
                        threshold, code, locals, next_slot, loop_stack, mutable,
                    )?;
                    MatchTest::Compare(*op)
                }
            };
            code.push(Instruction::match_test(test));
            code.push(Instruction::JumpIfNot(0));
            let jmp_idx = code.len() - 1;
            let saved = *next_slot;
            self.compile_match_expr_arm_body(
                arm.body(),
                code,
                locals,
                next_slot,
                loop_stack,
                mutable,
            )?;
            *next_slot = saved;
            code.push(Instruction::Jump(0));
            end_fixups.push(code.len() - 1);
            code[jmp_idx] = Instruction::JumpIfNot(code.len());
        }
        if let Some(eb) = else_body {
            self.compile_match_expr_arm_body(eb, code, locals, next_slot, loop_stack, mutable)?;
        }
        let end = code.len();
        for f in end_fixups {
            code[f] = Instruction::Jump(end);
        }
        // Drop the scrutinee, materialize the register as the let value.
        code.push(Instruction::Pop);
        code.push(Instruction::EndValueExpr);
        code.push(Instruction::StoreLocal(let_slot));
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn compile_match_stmt(
        &self,
        scrutinee: &Expr,
        arms: &[MatchArm],
        else_body: &Option<Vec<Statement>>,
        code: &mut Vec<Instruction>,
        locals: &mut HashMap<String, usize>,
        next_slot: &mut usize,
        loop_stack: &mut Vec<(usize, Vec<usize>, Vec<usize>)>,
        mutable: &mut HashSet<String>,
        keep_last_value: bool,
    ) -> Result<(), String> {
        let tmp_slot = Self::alloc_hidden_slot(locals, next_slot, "match_scrutinee");
        self.compile_expr_with_locals(scrutinee, code, locals, next_slot, loop_stack, mutable)?;
        code.push(Instruction::StoreLocal(tmp_slot));
        let mut end_fixups: Vec<usize> = Vec::new();
        for arm in arms {
            // (See compile_let_match: single scrutinee load per test.)
            let test = match arm {
                MatchArm::Exact(s, _) => {
                    code.push(Instruction::LoadLocal(tmp_slot));
                    MatchTest::Exact(s.clone())
                }
                MatchArm::StartsWith(s, _) => {
                    code.push(Instruction::LoadLocal(tmp_slot));
                    MatchTest::StartsWith(s.clone())
                }
                MatchArm::Contains(s, _) => {
                    code.push(Instruction::LoadLocal(tmp_slot));
                    MatchTest::Contains(s.clone())
                }
                MatchArm::Compare(op, threshold, _) => {
                    code.push(Instruction::LoadLocal(tmp_slot));
                    self.compile_expr_with_locals(
                        threshold, code, locals, next_slot, loop_stack, mutable,
                    )?;
                    MatchTest::Compare(*op)
                }
            };
            code.push(Instruction::match_test(test));
            code.push(Instruction::JumpIfNot(0));
            let jmp_idx = code.len() - 1;
            let saved = *next_slot;
            // №582: the kept arm's FINAL statement goes through the keep
            // compiler (recursively) — the arm leaves EXACTLY one value on
            // the stack: the kept trailing value (the old Pop-strip, the
            // №250/№574 behavior for value tails) or a synthesized epilogue
            // Unit for the NoValue tails (let/assign/loop/…, the
            // leftover-local class). Terminator tails owe nothing.
            let arm_body = arm.body();
            for (s_idx, s) in arm_body.iter().enumerate() {
                if keep_last_value && s_idx + 1 == arm_body.len() {
                    self.compile_stmt_keeping_value(
                        s, code, locals, next_slot, loop_stack, mutable,
                    )?;
                } else {
                    self.compile_stmt_with_locals(s, code, locals, next_slot, loop_stack, mutable)?;
                }
            }
            if keep_last_value && arm_body.is_empty() {
                // An EMPTY arm: no final statement compiled — the arm path
                // leaves no value; give it the epilogue Unit. (A non-empty
                // arm's final statement went through compile_stmt_keeping_value,
                // which already guarantees exactly one value: the kept value,
                // a synthesized Unit for the NoValue tails, or a terminator
                // that exits the body.)
                code.push(Instruction::PushUnit);
            }
            *next_slot = saved;
            code.push(Instruction::Jump(0));
            end_fixups.push(code.len() - 1);
            code[jmp_idx] = Instruction::JumpIfNot(code.len());
        }
        if let Some(eb) = else_body {
            for (s_idx, s) in eb.iter().enumerate() {
                if keep_last_value && s_idx + 1 == eb.len() {
                    self.compile_stmt_keeping_value(
                        s, code, locals, next_slot, loop_stack, mutable,
                    )?;
                } else {
                    self.compile_stmt_with_locals(s, code, locals, next_slot, loop_stack, mutable)?;
                }
            }
            if keep_last_value && eb.is_empty() {
                code.push(Instruction::PushUnit);
            }
        } else if keep_last_value {
            // №582: NO else — a no-match path would reach the end with no
            // value while the arms leave one; synthesize the missing arm.
            code.push(Instruction::PushUnit);
        }
        let end = code.len();
        for f in end_fixups {
            code[f] = Instruction::Jump(end);
        }
        Ok(())
    }

    /// №574 (gh#975): the if-form statement compilation for pattern/route
    /// bodies — the two dispatcher arms (IfThen / IfElseBlock) share this
    /// ONE place, and the keep-tail rule lives here.
    ///
    /// The keep-tail rule (`keep_tail: true` exactly when the if-form is
    /// the FINAL statement of the body): the TAKEN branch's trailing value
    /// stays on the stack, so the body's fall-through value (execute_code:
    /// `Ok(stack.pop())`) IS the branch's value — TW parity. TW serves a
    /// respond() inside a taken if-branch (execute_route_body evaluates the
    /// branch through eval_statements and takes its non-Unit trailing value
    /// as the route response); the VM route executor reads the stack top,
    /// so the branch tail must SURVIVE compilation. This is the №250
    /// route-tail mechanism (the flat ExprStmt tail) extended to branch
    /// tails: only the branch-FINAL statement sheds its Pop, recursively
    /// through nested if-forms; interior statements compile as usual.
    ///
    /// Branch shapes without a trailing value are №582 territory: a
    /// let/assign tail no longer leaves the fall-through pop reading the
    /// last local slot — the keep compiler appends the epilogue `PushUnit`
    /// (and a missing else arm is synthesized as one), so the body exit is
    /// Unit, the same `200 OK` fall-through the TW yields. The №574
    /// value-tail behavior (the branch value survives) is preserved
    /// unchanged — the №574.1 regression tests must stay green.
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn compile_if_stmt_with_keep(
        &self,
        condition: &Expr,
        then_body: &[Statement],
        else_ifs: &[(Expr, Vec<Statement>)],
        else_body: Option<&[Statement]>,
        is_else_block: bool,
        keep_tail: bool,
        code: &mut Vec<Instruction>,
        locals: &mut HashMap<String, usize>,
        next_slot: &mut usize,
        loop_stack: &mut Vec<(usize, Vec<usize>, Vec<usize>)>,
        mutable: &mut HashSet<String>,
    ) -> Result<(), String> {
        if !is_else_block {
            // IfThen: cond → JumpIfNot(else) → branch → end.
            self.compile_expr_with_locals(condition, code, locals, next_slot, loop_stack, mutable)?;
            code.push(Instruction::JumpIfNot(0)); // placeholder
            let jmp_idx = code.len() - 1;

            let saved_next_slot = *next_slot;
            self.compile_branch_stmts(
                then_body, keep_tail, code, locals, next_slot, loop_stack, mutable,
            )?;
            *next_slot = saved_next_slot;

            // №582: the epilogue invariant over the if-tail — every live
            // path past the if carries EXACTLY one stack value (the kept
            // branch value, or a synthesized Unit), so the route exit's
            // pop NEVER lands on a local slot (the leftover-local class).
            let then_final_is_terminator = then_body
                .last()
                .map(Self::stmt_is_terminator)
                .unwrap_or(false);
            if !keep_tail {
                // Nested position: the general statement compiler keeps the
                // stack balanced around the whole if — the pre-№574 shape.
                let after = code.len();
                code[jmp_idx] = Instruction::JumpIfNot(after);
            } else if then_final_is_terminator {
                // The taken path EXITS the body (return/break/continue) —
                // only the cond=false path reaches the end; give it the
                // epilogue value directly.
                let else_start = code.len();
                code[jmp_idx] = Instruction::JumpIfNot(else_start);
                code.push(Instruction::PushUnit);
            } else {
                // The taken path leaves a value — skip the synthesized
                // else arm so the two paths cannot stack up twice.
                code.push(Instruction::Jump(0)); // skip the synthesized else
                let jump_end = code.len() - 1;
                let else_start = code.len();
                code[jmp_idx] = Instruction::JumpIfNot(else_start);
                code.push(Instruction::PushUnit);
                let end = code.len();
                code[jump_end] = Instruction::Jump(end);
            }
            return Ok(());
        }
        // IfElseBlock: compile if/else if/else chain (the pre-№574 shape,
        // branch compilation keep-aware).
        let mut jump_to_end_fixups: Vec<usize> = Vec::new();

        // if condition
        self.compile_expr_with_locals(condition, code, locals, next_slot, loop_stack, mutable)?;
        code.push(Instruction::JumpIfNot(0));
        let jmp_idx = code.len() - 1;

        let saved_next_slot = *next_slot;
        self.compile_branch_stmts(
            then_body, keep_tail, code, locals, next_slot, loop_stack, mutable,
        )?;
        *next_slot = saved_next_slot;

        code.push(Instruction::Jump(0)); // skip else
        let then_end = code.len() - 1;
        jump_to_end_fixups.push(then_end);

        let then_else_start = code.len();
        code[jmp_idx] = Instruction::JumpIfNot(then_else_start);

        // else if chain
        for (ei_cond, ei_body) in else_ifs {
            self.compile_expr_with_locals(ei_cond, code, locals, next_slot, loop_stack, mutable)?;
            code.push(Instruction::JumpIfNot(0));
            let ei_jmp = code.len() - 1;

            let saved_ns = *next_slot;
            self.compile_branch_stmts(
                ei_body, keep_tail, code, locals, next_slot, loop_stack, mutable,
            )?;
            *next_slot = saved_ns;

            code.push(Instruction::Jump(0));
            let ei_end = code.len() - 1;
            jump_to_end_fixups.push(ei_end);

            let ei_else_start = code.len();
            code[ei_jmp] = Instruction::JumpIfNot(ei_else_start);
        }

        // else body
        if let Some(else_body) = else_body {
            let saved_ns = *next_slot;
            self.compile_branch_stmts(
                else_body, keep_tail, code, locals, next_slot, loop_stack, mutable,
            )?;
            *next_slot = saved_ns;
        } else if keep_tail {
            // №582: NO else — the cond=false path would reach the block end
            // with NO value on the stack while the taken paths leave one.
            // Synthesize the missing else arm: a single epilogue Unit.
            code.push(Instruction::PushUnit);
        }

        let block_end = code.len();
        for fixup in jump_to_end_fixups {
            code[fixup] = Instruction::Jump(block_end);
        }
        Ok(())
    }

    /// №582: does this statement EXIT the body when executed (a terminator)?
    /// A terminator path owes no epilogue value — Return leaves the frame,
    /// Break/Continue jump to the loop edges; a PushUnit after them would
    /// be dead code. Everything else either leaves a value or needs the
    /// synthesized one.
    fn stmt_is_terminator(stmt: &Statement) -> bool {
        matches!(
            stmt,
            Statement::Return { .. } | Statement::Break | Statement::Continue
        )
    }

    /// №574: one branch body — every statement through
    /// `compile_stmt_with_locals`, EXCEPT the branch-final statement when
    /// `keep_tail` is set: it compiles through
    /// `compile_stmt_keeping_value` (its trailing value survives).
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn compile_branch_stmts(
        &self,
        stmts: &[Statement],
        keep_tail: bool,
        code: &mut Vec<Instruction>,
        locals: &mut HashMap<String, usize>,
        next_slot: &mut usize,
        loop_stack: &mut Vec<(usize, Vec<usize>, Vec<usize>)>,
        mutable: &mut HashSet<String>,
    ) -> Result<(), String> {
        // №582: an EMPTY kept branch leaves no value — give it the epilogue
        // Unit so every live path of the branch carries exactly one value.
        if keep_tail && stmts.is_empty() {
            code.push(Instruction::PushUnit);
            return Ok(());
        }
        for (s_idx, s) in stmts.iter().enumerate() {
            if keep_tail && s_idx + 1 == stmts.len() {
                self.compile_stmt_keeping_value(s, code, locals, next_slot, loop_stack, mutable)?;
            } else {
                self.compile_stmt_with_locals(s, code, locals, next_slot, loop_stack, mutable)?;
            }
        }
        Ok(())
    }

    /// №574: keep-value compilation of a branch-FINAL statement — the
    /// statement's trailing value stays on the stack (the branch's value,
    /// the route response for respond()). ExprStmt sheds its Pop (the
    /// №328/№392 sink-check machinery stays identical — only the Pop is
    /// omitted); nested if-forms recurse with keep_tail=true; everything
    /// else compiles normally (lets/assigns terminate in StoreLocal —
    /// no value to keep).
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn compile_stmt_keeping_value(
        &self,
        stmt: &Statement,
        code: &mut Vec<Instruction>,
        locals: &mut HashMap<String, usize>,
        next_slot: &mut usize,
        loop_stack: &mut Vec<(usize, Vec<usize>, Vec<usize>)>,
        mutable: &mut HashSet<String>,
    ) -> Result<(), String> {
        match stmt {
            Statement::ExprStmt { expr, .. } => {
                // №328: the runtime twin of the №325 gate at sink sites.
                // №392: armed the same way — a handled refusal degrades
                // to Unit, which (unlike compile_stmt_with_locals) stays
                // on the stack here as the kept branch value.
                let deny_checks = self.emit_armed_sink_checks(code, expr);
                self.compile_expr_with_locals(expr, code, locals, next_slot, loop_stack, mutable)?;
                let skip_to = code.len();
                Self::patch_deny_skip_to(code, &deny_checks, skip_to);
                // №574: NO trailing Pop — the value is the branch's value.
            }
            Statement::IfThen {
                condition,
                body: then_body,
                ..
            } => {
                self.compile_if_stmt_with_keep(
                    condition,
                    then_body,
                    &[],
                    None,
                    false,
                    true,
                    code,
                    locals,
                    next_slot,
                    loop_stack,
                    mutable,
                )?;
            }
            Statement::IfElseBlock {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                self.compile_if_stmt_with_keep(
                    condition,
                    then_body,
                    else_ifs.as_slice(),
                    else_body.as_deref(),
                    true,
                    true,
                    code,
                    locals,
                    next_slot,
                    loop_stack,
                    mutable,
                )?;
            }
            other @ (Statement::Return { .. } | Statement::Break | Statement::Continue) => {
                // №582: the terminator forms — every path EXITS the body
                // here (Return returns, Break/Continue jump to the loop
                // edges), so no epilogue value is owed on this path and a
                // PushUnit would be dead code. Compile as usual.
                self.compile_stmt_with_locals(other, code, locals, next_slot, loop_stack, mutable)?;
            }
            other @ (Statement::LetBinding { .. }
            | Statement::Assign { .. }
            | Statement::Each { .. }
            | Statement::EachWithIndex { .. }
            | Statement::While { .. }
            | Statement::Memorize(_)
            | Statement::Forget(_)
            | Statement::Relate(_)) => {
                // №582: these forms leave NO value on the stack (the
                // leftover-local class — the epilogue pop would read the
                // last local slot). The keep-tail contract now guarantees
                // a value on every live path: emit the epilogue Unit.
                self.compile_stmt_with_locals(other, code, locals, next_slot, loop_stack, mutable)?;
                code.push(Instruction::PushUnit);
            }
            Statement::Match {
                scrutinee,
                arms,
                else_body,
                ..
            } => {
                // №582: a kept match tail goes through the same epilogue
                // invariant as a kept if — every arm (and the synthesized
                // missing else) ends with exactly one value on the stack.
                self.compile_match_stmt(
                    scrutinee, arms, else_body, code, locals, next_slot, loop_stack, mutable, true,
                )?;
            }
        }
        Ok(())
    }

    fn compile_stmt_with_locals(
        &self,
        stmt: &Statement,
        code: &mut Vec<Instruction>,
        locals: &mut HashMap<String, usize>,
        next_slot: &mut usize,
        loop_stack: &mut Vec<(usize, Vec<usize>, Vec<usize>)>,
        mutable: &mut HashSet<String>,
    ) -> Result<(), String> {
        match stmt {
            Statement::LetBinding {
                name,
                value,
                mutable: is_mut,
                ..
            } if matches!(value, Expr::MatchExpr { .. }) => {
                // №369: the match EXPRESSION form compiles natively
                // (ADR-0141 Stage 1.1) — full arm structure, TW parity.
                let Expr::MatchExpr {
                    scrutinee,
                    arms,
                    else_body,
                    ..
                } = value
                else {
                    unreachable!("guard guarantees MatchExpr")
                };
                self.compile_let_match(
                    name, *is_mut, scrutinee, arms, else_body, code, locals, next_slot, loop_stack,
                    mutable,
                )?;
            }
            Statement::LetBinding {
                name,
                value,
                mutable: is_mut,
                ..
            } => {
                // Function-level scoping: reuse existing slot if name exists.
                if *is_mut {
                    mutable.insert(name.clone());
                }
                let slot = if let Some(&existing_slot) = locals.get(name) {
                    self.compile_expr_with_locals(
                        value, code, locals, next_slot, loop_stack, mutable,
                    )?;
                    code.push(Instruction::StoreLocal(existing_slot));
                    existing_slot
                } else {
                    let slot = *next_slot;
                    *next_slot += 1;
                    locals.insert(name.clone(), slot);
                    self.compile_expr_with_locals(
                        value, code, locals, next_slot, loop_stack, mutable,
                    )?;
                    code.push(Instruction::StoreLocal(slot));
                    slot
                };
                // №328: seed the runtime label env for source-backed lets.
                if let crate::ast::Expr::FnCall { name: src, .. } = value {
                    if is_source_call(src) {
                        code.push(Instruction::LabelJoin(Box::new(LabelJoinData {
                            dst: name.clone(),
                            src: format!("@{src}"),
                        })));
                    }
                }
                let _ = slot;
            }
            Statement::Assign { name, value, .. } => {
                // Наряд №264: same immutability contract as the top-level
                // Assign arm — non-`let mut` targets are a compile error
                // (TW errors at runtime; the VM must not receive a silent
                // store for them), and only `let mut` locals are stored via
                // the checked StoreAssignLocal opcode.
                if !mutable.contains(name) {
                    return Err(crate::semantic::immutability_error_text(name));
                }
                let slot = match locals.get(name) {
                    Some(&slot) => slot,
                    None => return Err(crate::semantic::immutability_error_text(name)),
                };
                self.compile_expr_with_locals(value, code, locals, next_slot, loop_stack, mutable)?;
                code.push(Instruction::store_assign_local(slot, name.clone(), true));
            }
            Statement::Return { value: expr, .. } => {
                // №328: the runtime twin of the №325 gate at sink sites.
                // №392: armed with the on_deny path when a covering
                // handler exists — a handled refusal degrades to Unit,
                // which becomes the return value (skip_to lands on the
                // Return below; the refused call never executes).
                let deny_checks = self.emit_armed_sink_checks(code, expr);
                self.compile_expr_with_locals(expr, code, locals, next_slot, loop_stack, mutable)?;
                let skip_to = code.len();
                Self::patch_deny_skip_to(code, &deny_checks, skip_to);
                code.push(Instruction::Return);
            }
            Statement::While {
                condition, body, ..
            } => {
                let loop_start = code.len();
                let break_fixups: Vec<usize> = Vec::new();

                self.compile_expr_with_locals(
                    condition, code, locals, next_slot, loop_stack, mutable,
                )?;
                let jmp_not_idx = code.len();
                code.push(Instruction::JumpIfNot(0));

                loop_stack.push((loop_start, vec![], vec![]));
                let saved = *next_slot;
                for s in body {
                    match s {
                        Statement::Break => {
                            let fixup = code.len();
                            code.push(Instruction::Jump(0));
                            if let Some(entry) = loop_stack.last_mut() {
                                entry.1.push(fixup);
                            }
                        }
                        Statement::Continue => {
                            code.push(Instruction::Jump(loop_start));
                        }
                        // №510: explicit fall-through — the loop compiler
                        // special-cases break/continue only; every other
                        // statement goes to the generic compiler.
                        Statement::LetBinding { .. }
                        | Statement::Assign { .. }
                        | Statement::Each { .. }
                        | Statement::EachWithIndex { .. }
                        | Statement::While { .. }
                        | Statement::IfElseBlock { .. }
                        | Statement::IfThen { .. }
                        | Statement::Return { .. }
                        | Statement::ExprStmt { .. }
                        | Statement::Match { .. }
                        | Statement::Memorize(_)
                        | Statement::Forget(_)
                        | Statement::Relate(_) => {
                            self.compile_stmt_with_locals(
                                s, code, locals, next_slot, loop_stack, mutable,
                            )?;
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
            Statement::IfThen {
                condition: cond,
                body: then_body,
                ..
            } => {
                self.compile_expr_with_locals(cond, code, locals, next_slot, loop_stack, mutable)?;
                code.push(Instruction::JumpIfNot(0));
                let jmp_idx = code.len() - 1;
                let saved = *next_slot;
                for s in then_body {
                    self.compile_stmt_with_locals(s, code, locals, next_slot, loop_stack, mutable)?;
                }
                *next_slot = saved;
                code[jmp_idx] = Instruction::JumpIfNot(code.len());
            }
            Statement::IfElseBlock {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                let mut end_fixups: Vec<usize> = Vec::new();
                self.compile_expr_with_locals(
                    condition, code, locals, next_slot, loop_stack, mutable,
                )?;
                code.push(Instruction::JumpIfNot(0));
                let jmp_idx = code.len() - 1;
                let saved = *next_slot;
                for s in then_body {
                    self.compile_stmt_with_locals(s, code, locals, next_slot, loop_stack, mutable)?;
                }
                *next_slot = saved;
                code.push(Instruction::Jump(0));
                end_fixups.push(code.len() - 1);
                code[jmp_idx] = Instruction::JumpIfNot(code.len());

                for (ei_cond, ei_body) in else_ifs {
                    self.compile_expr_with_locals(
                        ei_cond, code, locals, next_slot, loop_stack, mutable,
                    )?;
                    code.push(Instruction::JumpIfNot(0));
                    let ei_jmp = code.len() - 1;
                    let saved2 = *next_slot;
                    for s in ei_body {
                        self.compile_stmt_with_locals(
                            s, code, locals, next_slot, loop_stack, mutable,
                        )?;
                    }
                    *next_slot = saved2;
                    code.push(Instruction::Jump(0));
                    end_fixups.push(code.len() - 1);
                    code[ei_jmp] = Instruction::JumpIfNot(code.len());
                }

                if let Some(eb) = else_body {
                    let saved3 = *next_slot;
                    for s in eb {
                        self.compile_stmt_with_locals(
                            s, code, locals, next_slot, loop_stack, mutable,
                        )?;
                    }
                    *next_slot = saved3;
                }

                let end = code.len();
                for f in end_fixups {
                    code[f] = Instruction::Jump(end);
                }
            }
            Statement::ExprStmt { expr, .. } => {
                // №328: the runtime twin of the №325 gate at sink sites.
                // №392: armed the same way — a handled refusal degrades
                // to Unit, which the Pop below discards.
                let deny_checks = self.emit_armed_sink_checks(code, expr);
                self.compile_expr_with_locals(expr, code, locals, next_slot, loop_stack, mutable)?;
                let skip_to = code.len();
                Self::patch_deny_skip_to(code, &deny_checks, skip_to);
                code.push(Instruction::Pop);
            }
            // Наряд №266: memory ops as statements (loop/if bodies route here
            // via compile_stmt_with_locals) — same opcodes as top-level pass2.
            Statement::Memorize(m) => {
                self.compile_expr_with_locals(
                    &m.value, code, locals, next_slot, loop_stack, mutable,
                )?;
                code.push(Instruction::Memorize(m.priority));
            }
            Statement::Forget(f) => {
                self.compile_expr_with_locals(
                    &f.query, code, locals, next_slot, loop_stack, mutable,
                )?;
                code.push(Instruction::Forget(f.days));
            }
            Statement::Relate(r) => {
                self.compile_expr_with_locals(
                    &r.from, code, locals, next_slot, loop_stack, mutable,
                )?;
                self.compile_expr_with_locals(&r.to, code, locals, next_slot, loop_stack, mutable)?;
                code.push(Instruction::const_(Value::String(r.relation.clone())));
                code.push(Instruction::Relate);
            }
            Statement::Match {
                scrutinee,
                arms,
                else_body,
                ..
            } => {
                // №369: Match statement → bytecode (ADR-0141 Stage 1.1).
                // Previously hit the silent `_ => {}` no-op in this shared
                // statement compiler — a nested match inside an if/while
                // body compiled to NOTHING. Loud parity now.
                // Nested position: keep the stack balanced (the fall-through
                // value convention only applies at pattern/route body level,
                // handled by compile_pattern_body_with_locals).
                self.compile_match_stmt(
                    scrutinee, arms, else_body, code, locals, next_slot, loop_stack, mutable, false,
                )?;
            }
            // №510: explicit no-op set — the shared statement compiler only
            // handles value-carrying statements; loop-control and iteration
            // belong to their own compilers.
            Statement::Each { .. }
            | Statement::EachWithIndex { .. }
            | Statement::Break
            | Statement::Continue => {}
        }
        Ok(())
    }

    /// Compile a flow source expression into a FlowExpr.
    #[allow(dead_code)]
    fn compile_flow_expr(&self, expr: &Expr) -> FlowExpr {
        match expr {
            Expr::Ident { name, .. } => {
                if let Some(&slot) = self.global_slots.get(name) {
                    FlowExpr::GlobalSlot(slot)
                } else {
                    FlowExpr::Ident(name.clone())
                }
            }
            Expr::StringLit { value: s, .. } => FlowExpr::Const(Value::String(s.clone())),
            Expr::FloatLit { value: f, .. } => FlowExpr::Const(Value::Float(*f)),
            // №510: explicit fall-through — non-constant flow expressions
            // keep their debug-render identity; a new Expr variant must be
            // consciously reviewed.
            Expr::BoolLit { .. }
            | Expr::FieldAccess { .. }
            | Expr::FnCall { .. }
            | Expr::QualifiedCall { .. }
            | Expr::BinaryOp { .. }
            | Expr::IfElse { .. }
            | Expr::List { .. }
            | Expr::IndexAccess { .. }
            | Expr::StructLit { .. }
            | Expr::BlockIfElse { .. }
            | Expr::MatchExpr { .. }
            | Expr::Try { .. }
            | Expr::HandleSource { .. }
            | Expr::ProvBind { .. } => FlowExpr::Ident(format!("{:?}", expr)),
        }
    }

    /// Try to evaluate an expression to a constant Value.
    fn eval_const_expr(&self, expr: &Expr) -> Value {
        match expr {
            Expr::StringLit { value: s, .. } => Value::String(s.clone()),
            Expr::FloatLit { value: f, .. } => Value::Float(*f),
            // №510: explicit fall-through — only literals are constant;
            // a new Expr variant must be consciously classified.
            Expr::BoolLit { .. }
            | Expr::Ident { .. }
            | Expr::FieldAccess { .. }
            | Expr::FnCall { .. }
            | Expr::QualifiedCall { .. }
            | Expr::BinaryOp { .. }
            | Expr::IfElse { .. }
            | Expr::List { .. }
            | Expr::IndexAccess { .. }
            | Expr::StructLit { .. }
            | Expr::BlockIfElse { .. }
            | Expr::MatchExpr { .. }
            | Expr::Try { .. }
            | Expr::HandleSource { .. }
            | Expr::ProvBind { .. } => Value::Unit,
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
                        // №510: explicit `Ne` — the rule-condition site of
                        // the same wildcard defect (C-01).
                        AstCompareOp::Gt => ConditionOp::Gt,
                        AstCompareOp::Lt => ConditionOp::Lt,
                        AstCompareOp::Ge => ConditionOp::Ge,
                        AstCompareOp::Le => ConditionOp::Le,
                        AstCompareOp::Eq => ConditionOp::Eq,
                        AstCompareOp::Ne => ConditionOp::Ne,
                    },
                    right: self.rule_value_expr(right),
                }
            }
        };

        let target_name = match &rule.target {
            Expr::Ident { name, .. } => name.clone(),
            // №510: explicit rejection set — a rule target must be an
            // identifier; every other shape is a loud compile error.
            Expr::StringLit { .. }
            | Expr::FloatLit { .. }
            | Expr::BoolLit { .. }
            | Expr::FieldAccess { .. }
            | Expr::FnCall { .. }
            | Expr::QualifiedCall { .. }
            | Expr::BinaryOp { .. }
            | Expr::IfElse { .. }
            | Expr::List { .. }
            | Expr::IndexAccess { .. }
            | Expr::StructLit { .. }
            | Expr::BlockIfElse { .. }
            | Expr::MatchExpr { .. }
            | Expr::Try { .. }
            | Expr::HandleSource { .. }
            | Expr::ProvBind { .. } => return Err("rule target must be an identifier".to_string()),
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
            Expr::Ident { name, .. } => RuleValueExpr::Ident(name.clone()),
            Expr::StringLit { value: s, .. } => RuleValueExpr::StringLit(s.clone()),
            Expr::FloatLit { value: f, .. } => RuleValueExpr::FloatLit(*f),
            Expr::FieldAccess {
                object: base,
                field,
                ..
            } => {
                if let Expr::Ident { name, .. } = base.as_ref() {
                    RuleValueExpr::FieldAccess(name.clone(), field.clone())
                } else {
                    RuleValueExpr::Ident(format!("{:?}", expr))
                }
            }
            // №510: explicit fall-through — simplified rule values keep
            // their debug-render identity; a new Expr variant must be
            // consciously reviewed.
            Expr::BoolLit { .. }
            | Expr::FnCall { .. }
            | Expr::QualifiedCall { .. }
            | Expr::BinaryOp { .. }
            | Expr::IfElse { .. }
            | Expr::List { .. }
            | Expr::IndexAccess { .. }
            | Expr::StructLit { .. }
            | Expr::BlockIfElse { .. }
            | Expr::MatchExpr { .. }
            | Expr::Try { .. }
            | Expr::HandleSource { .. }
            | Expr::ProvBind { .. } => RuleValueExpr::Ident(format!("{:?}", expr)),
        }
    }

    /// Analyze a compiled pattern body for purity.
    /// A pattern is pure if it contains only:
    ///   - LoadLocal, Const (any type), Add, Sub, Mul, Div
    ///   - CmpGt/CmpLt/CmpGe/CmpLe/CmpEq (allowed in purity, but NOT JIT-compiled)
    ///   - Return
    ///     And ALL parameter types are "Float".
    ///     No globals, no builtins, no LLM calls, no struct operations, no memory.
    ///     Note: is_pure is a broader check than JIT-eligibility. JIT additionally
    ///     requires only arithmetic (no Cmp*). See JitCompiler::is_jit_eligible().
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
                // №510: explicit disallow mirror — every non-pure
                // instruction is named, so adding a new Instruction variant
                // is a compile error here and forces a purity review
                // (deny wildcard_enum_match_arm).
                Instruction::SetValueReg
                | Instruction::LabelJoin(_)
                | Instruction::SinkCheck(_)
                | Instruction::LoadGlobal(_)
                | Instruction::LoadGlobalByName(_)
                | Instruction::StoreGlobal(_)
                | Instruction::StoreLocal(_)
                | Instruction::RegisterPattern(_)
                | Instruction::RegisterLearnable(_)
                | Instruction::CallBuiltin(..)
                | Instruction::CallPattern(..)
                | Instruction::Contains
                | Instruction::MakeStruct(_)
                | Instruction::GetField(_)
                | Instruction::IndexAccess
                | Instruction::MakeList(_)
                | Instruction::ListLen
                | Instruction::Pop
                | Instruction::PushUnit
                | Instruction::StartsWith
                | Instruction::MakeFluid(_)
                | Instruction::Jump(_)
                | Instruction::JumpIfNot(_)
                | Instruction::JumpIfLow(..)
                | Instruction::Collapse(_)
                | Instruction::Memorize(_)
                | Instruction::Recall
                | Instruction::Forget(_)
                | Instruction::LlmCall(..)
                | Instruction::Adapt(_)
                | Instruction::Relate
                | Instruction::Mutate(_)
                | Instruction::FlowPipeline(_)
                | Instruction::FlowExec(_)
                | Instruction::TryEval(_)
                | Instruction::ExecuteRules
                | Instruction::Halt
                | Instruction::StoreAssignLocal(_)
                | Instruction::MatchTest(_)
                | Instruction::Dup
                | Instruction::BeginValueExpr
                | Instruction::KeepLastValue
                | Instruction::EndValueExpr
                | Instruction::RegisterPatternRef(_) => return false,
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

        // №558: the ONE module-search rule (src/module_path.rs) — the
        // same SSOT the runtime loader and the semantic pass call. The
        // previous extension-replacing form was equivalent on every
        // reachable input (the grammar admits no dots in module paths);
        // the append form is the SSOT.
        let file_path = crate::module_path::resolve_module_file(&self.std_root, module_path);
        // №475: the IMPORT source loader — compile-time, AUTHOR-controlled
        // source text (the same trust domain as the file being compiled);
        // the sandbox targets PROGRAM-RUNTIME I/O, and `mlog run <abs
        // path>` must keep working.
        #[allow(clippy::disallowed_methods)]
        // №475: the IMPORT source loader — compile-time, AUTHOR-controlled
        // source text (the same trust domain as the file being compiled);
        // the sandbox targets PROGRAM-RUNTIME I/O, and `mlog run <abs
        // path>` must keep working.
        #[allow(clippy::disallowed_methods)]
        // №475: the IMPORT source loader — compile-time, AUTHOR-controlled
        // source text (the same trust domain as the file being compiled);
        // the sandbox targets PROGRAM-RUNTIME I/O, and `mlog run <abs
        // path>` must keep working.
        #[allow(clippy::disallowed_methods)]
        // №475: the IMPORT source loader — compile-time, AUTHOR-controlled
        // source text (the same trust domain as the file being compiled);
        // the sandbox targets PROGRAM-RUNTIME I/O, and `mlog run <abs
        // path>` must keep working.
        #[allow(clippy::disallowed_methods)]
        // №475: the IMPORT source loader — compile-time, AUTHOR-controlled
        // source text (the same trust domain as the file being compiled);
        // the sandbox targets PROGRAM-RUNTIME I/O, and `mlog run <abs
        // path>` must keep working.
        #[allow(clippy::disallowed_methods)]
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

    // ── №584 (the audit d63cc1d X-1 step 2): the route-body TERMINAL
    //    lowering of bare respond* statements ──────────────────────────
    //
    // THE REFERENCE (the TW serve lane is the etalon and does NOT change):
    // a bare respond* answers the route early from a form-specific surface
    // (server.rs `execute_route_body` + the HttpResponse-as-Return
    // propagation, Наряда-26 P0-2):
    //   - a DIRECT statement of the route body (any position, not just the
    //     tail) answers immediately;
    //   - IfThen / Match / While / Each bodies propagate the HttpResponse
    //     as ControlFlow::Return from ANY nesting depth (eval_block!) —
    //     cycles answer on the FIRST iteration;
    //   - a TOP-LEVEL if/else branch (Statement::IfElseBlock, the block
    //     form) is walked by a PER-STATEMENT loop in server.rs: a DIRECT
    //     bare respond answers immediately, but the result of any NESTED
    //     statement (an inner if/match/loop carrying a respond) is
    //     DISCARDED and the branch CONTINUES — depth ≥ 2 under a block-form
    //     if does NOT stop the route.
    //
    // The plain body compiler only keeps the TAIL chain (№250/№574) and
    // Pops every interior respond — the X-1 guard-bypass class (the depth-1
    // guard shape). The route-body compiler therefore rewrites the body
    // ONCE at the AST level: every bare respond* STATEMENT (per
    // `semantic::bare_respond_call` — the same SSOT predicate the semantic
    // advisory uses) becomes `return respond*(...)` — EXACTLY on the TW
    // early-answer surface above; everything else the machinery already
    // knows about `return` applies unchanged:
    //   - Statement::Return compiles to `expr; Instruction::Return` in every
    //     context (the route tail, if/match branch bodies, loop bodies) —
    //     the keep-tail synthesis (№574) treats Return as a terminator that
    //     owes no epilogue value (№582), so the route exit NEVER reads a
    //     local slot and the fall-through invariant survives;
    //   - a respond* consumed INSIDE a larger expression (a let value, a
    //     match-expression arm, an argument) is NOT rewritten — the value is
    //     consumed inline on BOTH backends (parity holds without the
    //     rewrite).
    //
    // The rewrite is LOCAL to `compile_routes`: it runs on a CLONED body, so
    // the parsed AST and the semantic advisory still see the source shape —
    // the mid-route style hints are computed on the ORIGINAL body.
    fn route_body_with_terminal_responds(body: &[Statement]) -> Vec<Statement> {
        Self::body_with_terminal_responds(
            body,
            crate::semantic::RespondPosition::OnEarlyAnswerSurface,
        )
    }

    /// №600: the recursion mode is the SSOT position predicate
    /// (`semantic::RespondPosition`) — the lowering and the semantic walk
    /// apply the SAME transitions (see the enum's transition table).
    /// `DirectOnlyBranch` — the branch-body mode of a TOP-LEVEL block-form
    /// if (Statement::IfElseBlock): only the branch's DIRECT bare respond*
    /// statements become `return`; the bodies of statements NESTED inside
    /// the branch keep the pre-№584 compilation (the TW serve lane discards
    /// nested-statement responses there — the lowering must NOT invent an
    /// early answer the interpreter does not produce). `SwallowedNested` is
    /// never ENTERED by the lowering (it does not descend into nested
    /// bodies from DirectOnlyBranch) — the semantic walk classifies those
    /// sites as the blocking RESPOND_SWALLOWED refusal instead (№600).
    fn body_with_terminal_responds(
        body: &[Statement],
        mode: crate::semantic::RespondPosition,
    ) -> Vec<Statement> {
        body.iter()
            .map(|s| Self::stmt_with_terminal_responds(s, mode))
            .collect()
    }

    fn stmt_with_terminal_responds(
        stmt: &Statement,
        mode: crate::semantic::RespondPosition,
    ) -> Statement {
        match stmt {
            Statement::ExprStmt { expr, span } => {
                if crate::semantic::bare_respond_call(expr) {
                    Statement::Return {
                        value: expr.clone(),
                        span: span.clone(),
                    }
                } else {
                    stmt.clone()
                }
            }
            Statement::IfThen {
                condition,
                body,
                span,
            } => {
                if mode != crate::semantic::RespondPosition::OnEarlyAnswerSurface {
                    // Nested under a top-level if/else branch — the serve
                    // lane discards its response; keep the pre-№584 shape.
                    stmt.clone()
                } else {
                    // The IfThen surface propagates the early answer from
                    // ANY depth — descend in full-terminal mode.
                    Statement::IfThen {
                        condition: condition.clone(),
                        body: Self::body_with_terminal_responds(
                            body,
                            crate::semantic::RespondPosition::OnEarlyAnswerSurface,
                        ),
                        span: span.clone(),
                    }
                }
            }
            Statement::IfElseBlock {
                condition,
                then_body,
                else_ifs,
                else_body,
                span,
            } => {
                if mode != crate::semantic::RespondPosition::OnEarlyAnswerSurface {
                    // Nested under a top-level if/else branch — see above.
                    stmt.clone()
                } else {
                    // TOP-LEVEL block-form if: the branch bodies run under
                    // the server's per-statement loop — their DIRECT bare
                    // respond* answers, everything nested is discarded
                    // (DirectOnlyBranch mode below).
                    Statement::IfElseBlock {
                        condition: condition.clone(),
                        then_body: Self::body_with_terminal_responds(
                            then_body,
                            crate::semantic::RespondPosition::DirectOnlyBranch,
                        ),
                        else_ifs: else_ifs
                            .iter()
                            .map(|(cond, b)| {
                                (
                                    cond.clone(),
                                    Self::body_with_terminal_responds(
                                        b,
                                        crate::semantic::RespondPosition::DirectOnlyBranch,
                                    ),
                                )
                            })
                            .collect(),
                        else_body: else_body.as_ref().map(|b| {
                            Self::body_with_terminal_responds(
                                b,
                                crate::semantic::RespondPosition::DirectOnlyBranch,
                            )
                        }),
                        span: span.clone(),
                    }
                }
            }
            Statement::Match {
                scrutinee,
                arms,
                else_body,
                span,
            } => {
                if mode != crate::semantic::RespondPosition::OnEarlyAnswerSurface {
                    stmt.clone()
                } else {
                    // The Match surface propagates the early answer from ANY
                    // depth (eval_block!) — descend in full-terminal mode.
                    Statement::Match {
                        scrutinee: scrutinee.clone(),
                        arms: arms
                            .iter()
                            .map(|a| {
                                Self::arm_with_terminal_responds(
                                    a,
                                    crate::semantic::RespondPosition::OnEarlyAnswerSurface,
                                )
                            })
                            .collect(),
                        else_body: else_body.as_ref().map(|b| {
                            Self::body_with_terminal_responds(
                                b,
                                crate::semantic::RespondPosition::OnEarlyAnswerSurface,
                            )
                        }),
                        span: span.clone(),
                    }
                }
            }
            Statement::While {
                condition,
                body,
                span,
            } => {
                if mode != crate::semantic::RespondPosition::OnEarlyAnswerSurface {
                    stmt.clone()
                } else {
                    // Cycle bodies propagate from ANY depth — the first
                    // iteration answers (TW first-iteration parity).
                    Statement::While {
                        condition: condition.clone(),
                        body: Self::body_with_terminal_responds(
                            body,
                            crate::semantic::RespondPosition::OnEarlyAnswerSurface,
                        ),
                        span: span.clone(),
                    }
                }
            }
            Statement::Each {
                variable,
                iterable,
                body,
                span,
            } => {
                if mode != crate::semantic::RespondPosition::OnEarlyAnswerSurface {
                    stmt.clone()
                } else {
                    Statement::Each {
                        variable: variable.clone(),
                        iterable: iterable.clone(),
                        body: Self::body_with_terminal_responds(
                            body,
                            crate::semantic::RespondPosition::OnEarlyAnswerSurface,
                        ),
                        span: span.clone(),
                    }
                }
            }
            Statement::EachWithIndex {
                index_var,
                item_var,
                iterable,
                body,
                span,
            } => {
                if mode != crate::semantic::RespondPosition::OnEarlyAnswerSurface {
                    stmt.clone()
                } else {
                    Statement::EachWithIndex {
                        index_var: index_var.clone(),
                        item_var: item_var.clone(),
                        iterable: iterable.clone(),
                        body: Self::body_with_terminal_responds(
                            body,
                            crate::semantic::RespondPosition::OnEarlyAnswerSurface,
                        ),
                        span: span.clone(),
                    }
                }
            }
            // LetBinding/Assign/Return/Memorize/Forget/Relate/Break/Continue
            // carry no statement bodies (Return is the sanctioned early
            // answer — never rewritten); a respond* inside their VALUE
            // expressions is consumed inline on both backends (never the
            // early-answer class) — pass through untouched. (№532 posture:
            // enumerated, no wildcard arm.)
            Statement::LetBinding { .. }
            | Statement::Assign { .. }
            | Statement::Return { .. }
            | Statement::Memorize(_)
            | Statement::Forget(_)
            | Statement::Relate(_)
            | Statement::Break
            | Statement::Continue => stmt.clone(),
        }
    }

    fn arm_with_terminal_responds(
        arm: &MatchArm,
        mode: crate::semantic::RespondPosition,
    ) -> MatchArm {
        match arm {
            MatchArm::Exact(s, body) => {
                MatchArm::Exact(s.clone(), Self::body_with_terminal_responds(body, mode))
            }
            MatchArm::StartsWith(s, body) => {
                MatchArm::StartsWith(s.clone(), Self::body_with_terminal_responds(body, mode))
            }
            MatchArm::Contains(s, body) => {
                MatchArm::Contains(s.clone(), Self::body_with_terminal_responds(body, mode))
            }
            MatchArm::Compare(op, threshold, body) => MatchArm::Compare(
                *op,
                threshold.clone(),
                Self::body_with_terminal_responds(body, mode),
            ),
        }
    }

    /// Compile route declarations into compiled bytecode route bodies.
    ///
    /// Reuses the same compilation pipeline as pattern bodies:
    /// route statements (LetBinding, Assign, Return, IfThen, IfElseBlock,
    /// ExprStmt, While, Each, etc.) are compiled to stack-based bytecode.
    ///
    /// №584: the body is compiled from the terminal-lowered clone
    /// (`route_body_with_terminal_responds`) — a bare respond* statement
    /// in ANY position compiles as `Call + Return`, the exact TW early
    /// answer.
    ///
    /// The caller (server) invokes this once at startup, then stores the
    /// resulting CompiledRoutes in ServerState for per-request execution.
    pub fn compile_routes(
        &self,
        routes: &[crate::ast::RouteDecl],
    ) -> Result<Vec<CompiledRoute>, String> {
        let mut compiled = Vec::new();
        for route in routes {
            let mut locals = HashMap::new();
            // Наряд №264: route bodies get the same immutability compile
            // check as pattern bodies (TW executes them through
            // eval_statements — the let-mut contract applies identically).
            let mut mutable: HashSet<String> = HashSet::new();
            // №584: the terminal lowering — bare respond* statements become
            // `return respond*(...)` in the compiled clone (the original AST
            // is untouched; the semantic advisory reads the source shape).
            let body = Self::route_body_with_terminal_responds(&route.body);
            let code = self.compile_pattern_body_with_locals(&body, &mut locals, &mut mutable)?;
            compiled.push(CompiledRoute {
                path: route.path.clone(),
                method: route.method.clone(),
                requires: route.requires.clone(),
                code,
            });
        }
        Ok(compiled)
    }
}
