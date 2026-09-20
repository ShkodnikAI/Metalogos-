// ── METALOGOS Bytecode Instruction Set — Phase 4.1 ─────────────────
//
// Stack-based bytecode for the METALOGOS VM.
// Design: one-opcode-per-action, operands inline in the enum.
// The Program is a flat sequence of top-level instructions plus
// compiled function bodies for patterns.

use serde::{Deserialize, Serialize};

use crate::interpreter::Value;

/// A single bytecode instruction. Operands are embedded in the enum variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    // ── Constants & Variables ───────────────────────────────────
    /// Push a constant value onto the stack.
    Const(Value),
    /// Наряд №328 (ADR-0156): join the runtime label of `src` into `dst`
    /// (componentwise, ADR-0154 §2.4). A `src` starting with `@` names a
    /// №316 Source builtin — the runtime seed label of that source.
    LabelJoin {
        dst: String,
        src: String,
    },
    /// Наряд №328 (ADR-0156): the runtime twin of the №325 gate — check
    /// the runtime label of `arg` against the sink's clearance; a
    /// violation is a loud runtime error + an audit event.
    /// Наряд №392: `arg_index` feeds the same reason-classification the
    /// static gate uses (the network address-position rule), and `deny`
    /// arms the check with the on_deny path: when the program declares a
    /// covering handler, the runtime runs it, pushes the degraded Unit
    /// and jumps PAST the refused call (which never executes). `None`
    /// keeps the loud default error.
    SinkCheck {
        fn_name: String,
        arg: String,
        line: u32,
        /// Argument position of this check within the sink call (0-based).
        arg_index: u32,
        /// The on_deny path (Naryad #392): handler index into
        /// `Program::deny_handlers` + the continuation after the refused
        /// sink call (patched by the compiler).
        deny: Option<SinkDenyPath>,
    },
    /// Push the value of a global variable (by slot index).
    LoadGlobal(usize),
    /// Push a global variable by name (for unresolved references).
    LoadGlobalByName(String),
    /// Pop the top value and store it into a global variable slot.
    StoreGlobal(usize),
    /// Push the value of a local variable (parameter by slot index).
    LoadLocal(usize),
    /// Pop the top value and store it into a local variable slot (Phase 5.1 let bindings).
    StoreLocal(usize),

    // ── Function Registration ──────────────────────────────────
    /// Register a compiled pattern function. Stores it in the VM's pattern table.
    RegisterPattern(CompiledFn),
    /// Register a compiled learnable pattern. Stores it in the VM's learnable table.
    RegisterLearnable(CompiledLearnableInfo),

    // ── Function Calls ──────────────────────────────────────────
    /// Call a built-in function. arity = number of args already on stack.
    CallBuiltin(usize, usize),
    /// Call a compiled pattern (user-defined). arity = number of args on stack.
    /// Pushes a new call frame; the pattern's body is executed.
    CallPattern(usize, usize),
    /// Return from the current call frame. The top value becomes the return value.
    Return,

    // ── Binary Operations ─────────────────────────────────────
    /// Pop two values, apply operation, push result. Left is deeper, right is TOS.
    Add,
    Sub,
    Mul,
    Div,
    /// Pop two strings; push 1.0 if left contains right, else 0.0.
    Contains,
    /// Comparison operations: pop right then left, push 1.0 (true) or 0.0 (false).
    CmpGt,
    CmpLt,
    CmpGe,
    CmpLe,
    CmpEq,
    CmpNe,

    // ── Struct Operations ─────────────────────────────────────
    /// Pop N field values (in reverse order), create a Struct with the given type name
    /// and field names. N = len(field_names). Values on stack are in field order.
    MakeStruct(String, Vec<String>),
    /// Get a field from the struct on TOS. Pushes the field value.
    GetField(String),
    /// Pop index and base; push base[index] (list/struct/string index access).
    IndexAccess,
    /// Pop N values (in reverse stack order), create a Value::List. (Наряд №18)
    MakeList(usize),
    /// Pop a list/string, push its length as Float. (Наряд №18)
    ListLen,
    /// Discard top-of-stack value. Used after ExprStmt to keep stack clean. (Наряд №18)
    Pop,
    /// Pop two strings; push 1.0 if left starts with right, else 0.0. (Наряд №21)
    StartsWith,

    // ── Fluid Types ──────────────────────────────────────────
    /// Pop 2*N values (N value-confidence pairs), create a Fluid value.
    MakeFluid(usize),

    // ── Control Flow ────────────────────────────────────────────
    /// Unconditional jump to instruction offset.
    Jump(usize),
    /// Pop top value; jump if it is falsy (empty string, zero float, empty list).
    JumpIfNot(usize),
    /// Pop top value; jump if its confidence (for Fluid) or itself (as Float)
    /// is strictly less than the threshold. Used for confidence-based branching.
    JumpIfLow(f64, usize),

    // ── METALOGOS Memory ──────────────────────────────────────
    /// Collapse a Fluid value on TOS to a concrete type. Pops Fluid, pushes
    /// the highest-confidence variant matching the type (or Unit on failure).
    Collapse(String),
    /// Pop a value, memorize it with the given priority (0.0..1.0).
    Memorize(f64),
    /// Pop a query string; push the best matching recalled value (or empty string).
    Recall,
    /// Pop a query string and days; remove matching memories older than cutoff.
    Forget(i64),

    // ── LLM Calls ──────────────────────────────────────────────
    /// Call an LLM-backed learnable pattern by index. arity args on stack.
    LlmCall(usize, usize),

    // ── Adapt / Relate / Mutate ────────────────────────────────
    /// Pop input, output strings; add as few-shot example to learnable.
    Adapt(String),
    /// Pop from, to, relation strings; add relation to knowledge graph.
    Relate,
    /// Execute mutate: pop example_count*2 values, apply to learnable pattern.
    Mutate {
        pattern_name: String,
        example_count: usize,
        rollback_threshold: Option<f64>,
        rollback_op: Option<ConditionOp>,
    },

    // ── Pipeline ──────────────────────────────────────────────
    /// Begin flow execution: pop source from stack, step through pipeline.
    /// The source value is already compiled as regular bytecode expressions
    /// (LoadGlobal, Const, Add, etc.) that push the result onto the stack.
    /// This instruction pops TOS as the source, then executes each step.
    FlowPipeline {
        pipeline: Vec<String>,
        branch_defs: Vec<(String, Vec<BranchDef>)>,
    },

    // ── Pipeline (legacy) ──────────────────────────────────────
    /// Legacy flow instruction with embedded source expression.
    /// Kept for backward compatibility with serialized programs.
    FlowExec {
        source_expr: FlowExpr,
        pipeline: Vec<String>,
        branch_defs: Vec<(String, Vec<BranchDef>)>,
    },

    // ── Error Handling ───────────────────────────────────────
    /// Evaluate inner bytecode in a try-catch: on Ok push result, on Err push Unit.
    /// Наряд №91: real `try` semantics for the VM (previously always Unit).
    TryEval(Vec<Instruction>),

    // ── Rule Engine ───────────────────────────────────────────
    /// Execute all registered rules (conditions + assignments).
    /// This is also a macro instruction — the VM evaluates rules internally.
    ExecuteRules,

    // ── Meta ──────────────────────────────────────────────────
    /// End of program.
    Halt,

    // ── Immutability (Наряд №264) ─────────────────────────────
    /// Assignment statement (NOT a `let` binding — plain StoreLocal covers
    /// bindings): pop the top value and store it into the local variable
    /// slot. Carries the compiler's immutability fact as instruction
    /// metadata:
    ///   - `slot`  — local slot (base_bp + slot at runtime);
    ///   - `name`  — source-level variable name (for the TW-parity error
    ///     text; locals are anonymous slots otherwise);
    ///   - `mutable` — whether the target was bound with `let mut`. The
    ///     compiler only ever emits `mutable: true` (it rejects non-mut
    ///     assignments at compile time since №264), so `false` on the wire
    ///     means bytecode produced past the check — the VM must fail
    ///     LOUDLY, never silently overwrite (the TW interpreter rejects
    ///     the same program at runtime).
    ///
    /// Backward compatibility: appended at the END of the enum, so
    /// bincode's positional variant indices of all existing instructions
    /// are unchanged — old .mbc files deserialize and run identically. An
    /// OLD binary reading NEW bytecode fails loudly at deserialize time
    /// (unknown variant index), never silently. The `Program` struct is
    /// untouched (schema frozen per the №250 precedent).
    StoreAssignLocal {
        slot: usize,
        name: String,
        mutable: bool,
    },

    // ── Match (№369, ADR-0141 Stage 1.1) ─────────────────
    /// Test ONE match arm against the scrutinee VALUE on the stack.
    /// Pops the scrutinee (Compare also pops the threshold first — the
    /// compiler emits threshold evaluation before the test for Compare
    /// arms), pushes Float(1.0/0.0) — the VM's boolean form.
    /// The matching predicate lives in `MatchTest::matches` — the SAME
    /// code the TW interpreter runs, so the backends cannot drift.
    /// Jump structure (first match wins, TW order) is emitted by the
    /// compiler with MatchTest + JumpIfNot + Jump; nothing here decides it.
    /// Appended at the END of the enum (bincode positional-index
    /// compatibility — see StoreAssignLocal's note).
    MatchTest(MatchTest),

    /// №370: duplicate the top stack value (the match scrutinee lives on
    /// the stack while arms test copies of it — single evaluation, no
    /// scratch slot). Appended at the END of the enum.
    Dup,

    /// №370: open a VALUE EXPRESSION register (pushed onto the VM's
    /// register stack, NOT the value stack) — the last-value register of a
    /// block-if-else / match expression. Starts as Unit. Registers live in
    /// VM state (not stack cells), so a value form is safe in ANY
    /// expression position: intermediate temporaries of enclosing
    /// expressions can never be clobbered by a register write (the
    /// slot-based register of the first №370 draft was unsound exactly
    /// there: `"[" + (if c {..} else {..}) + "]"` overwrote the "["
    /// temporary). Appended at the END of the enum.
    BeginValueExpr,

    /// №370: pop the top value; if it is NOT Unit, store it into the
    /// topmost value register. Mirrors the TW `eval_statements_cf`
    /// contract (`if !matches!(val, Value::Unit) { last_expr_value = val }`)
    /// — the branch's value is the last NON-Unit expression of its body,
    /// and a trailing Unit-valued statement does not reset it. Appended at
    /// the END of the enum.
    KeepLastValue,

    /// №370: pop the topmost value register and push its value onto the
    /// value stack — the value of the whole block-if-else / match
    /// expression. Pairs with BeginValueExpr (balanced, nestable — nested
    /// value forms open their own register). Appended at the END of the
    /// enum.
    EndValueExpr,
}

/// №369: the pattern side of one match arm — the payload of
/// `Instruction::MatchTest`. Appended AFTER the Instruction enum in its own
/// type (the Instruction enum itself is untouched variant-index-wise:
/// MatchTest is appended at its END — see StoreAssignLocal's note).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MatchTest {
    /// `"literal" then {...}` — scrutinee's Display form equals the literal.
    Exact(String),
    /// `starts_with "prefix" then {...}`
    StartsWith(String),
    /// `contains "substr" then {...}`
    Contains(String),
    /// `> expr then {...}` — numeric-first, string-fallback comparison
    /// (`ast::MatchArm::compare_values`); threshold is the value below
    /// the scrutinee on the stack (compiler emits threshold first).
    Compare(crate::ast::CompareOp),
}

impl MatchTest {
    /// The shared predicate — for non-Compare arms `threshold` is ignored
    /// and `Value::Unit` is passed by the VM. Compare arms go through
    /// `ast::MatchArm::compare_values` (the same function the TW
    /// interpreter calls — single source of truth, №369).
    pub fn matches(&self, scrutinee: &Value, threshold: &Value) -> bool {
        let s = format!("{}", scrutinee);
        match self {
            MatchTest::Exact(v) => s == *v,
            MatchTest::StartsWith(p) => s.starts_with(p.as_str()),
            MatchTest::Contains(sub) => s.contains(sub.as_str()),
            MatchTest::Compare(op) => {
                crate::ast::MatchArm::compare_values(scrutinee, op, threshold)
            }
        }
    }
}

/// A flow expression that can be compiled inline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlowExpr {
    /// Reference a global variable by slot index.
    GlobalSlot(usize),
    /// Reference by name (unresolved).
    Ident(String),
    /// A constant value.
    Const(Value),
}

/// Branch definition for flow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchDef {
    pub label: String,
    pub condition_field: String,
    pub condition_op: ConditionOp,
    pub condition_threshold: Value,
    pub target: String,
}

/// Comparison operator for branch conditions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ConditionOp {
    Gt,
    Lt,
    Ge,
    Le,
    Eq,
    /// Наряд №21: added Ne — previously fell back to Eq (bug)
    Ne,
}

impl ConditionOp {
    /// Наряд №205: evaluate this operator on two f64 values.
    /// Used by VM distillation's fallback_if check.
    pub fn compare(self, lhs: f64, rhs: f64) -> bool {
        match self {
            ConditionOp::Gt => lhs > rhs,
            ConditionOp::Lt => lhs < rhs,
            ConditionOp::Ge => lhs >= rhs,
            ConditionOp::Le => lhs <= rhs,
            ConditionOp::Eq => lhs == rhs,
            ConditionOp::Ne => lhs != rhs,
        }
    }
}

/// A compiled pattern function: name, parameter count, types, and instruction body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledFn {
    pub name: String,
    pub param_count: usize,
    /// Parameter type names (for Fluid collapse).
    pub param_types: Vec<String>,
    pub code: Vec<Instruction>,
    /// Whether this pattern is pure (no LLM, no side effects, no globals).
    /// Set by the compiler's purity analysis. Used by the JIT to determine
    /// which patterns can be compiled to native code.
    pub is_pure: bool,
}

/// Наряд №392: the on_deny path compiled into a deny-armed SinkCheck —
/// the handler to run (index into `Program::deny_handlers`) and the
/// continuation ip AFTER the refused sink call (the call is skipped, a
/// degraded Unit becomes its result).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SinkDenyPath {
    pub handler: u32,
    pub skip_to: u32,
}

/// Наряд №392: a compiled `on_deny(<class|*>) { body }` handler. Zero-arg,
/// zero-result code executed by the VM when a runtime gate refuses an
/// action covered by `class` (`*` = every class).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledDenyHandler {
    /// The sink class covered: "*" or one of the sink-class words.
    pub class: String,
    /// Mangled internal name (`__on_deny_N`) — diagnostics only.
    pub name: String,
    pub code: Vec<Instruction>,
}

/// A compiled learnable pattern: name, prompt, few-shot examples.
/// Compiled context mode for learnable patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompiledContextMode {
    /// No context.
    None,
    /// Auto: recall using first param value as query.
    Auto,
    /// Recall with query param name and limit.
    Recall(String, usize),
    /// Static literal context.
    Literal(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledLearnableInfo {
    pub name: String,
    pub param_count: usize,
    pub prompt: String,
    pub few_shot: Vec<(String, String)>,
    /// Context mode for the learnable pattern.
    pub context_mode: CompiledContextMode,
    /// Наряд №205 (ADR-0121 stage 6): distillation target.
    /// When Some, this learnable pattern distills its LLM traffic into
    /// the named reflex model after `distill_after` examples accumulate.
    #[serde(default)]
    pub distill_to: Option<String>,
    /// Наряд №205: minimum examples before training triggers.
    #[serde(default)]
    pub distill_after: usize,
    /// Наряд №205: confidence threshold for fallback to LLM.
    /// Form: (operator, threshold_value). E.g. `confidence < 0.85` → (Lt, 0.85).
    #[serde(default)]
    pub fallback_if: Option<(ConditionOp, f64)>,
}

/// A compiled skill_index for tiered skill matching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledSkillIndex {
    pub name: String,
    pub tiers: Vec<CompiledSkillTier>,
    pub budget: Option<f64>,
}

/// A compiled tier within a skill_index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledSkillTier {
    pub level: u32,
    pub mode: String,
    pub skills: Vec<String>,
    pub rules: Vec<CompiledSkillTriggerRule>,
}

/// A trigger rule within a "when_matches" tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledSkillTriggerRule {
    pub skill: String,
    pub triggers: Vec<String>,
}

/// A compiled rule for the rule engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledRule {
    pub condition: RuleCondition,
    pub target_name: String,
    pub field: String,
    pub value_expr: RuleValueExpr,
    /// Priority (higher = evaluated first). Matches interpreter semantics.
    pub priority: i32,
}

/// Condition expression for a rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleCondition {
    Contains {
        left: RuleValueExpr,
        right: RuleValueExpr,
    },
    Compare {
        left: RuleValueExpr,
        op: ConditionOp,
        right: RuleValueExpr,
    },
}

/// Value expression inside a rule (simplified — only ident/lit/field-access).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleValueExpr {
    Ident(String),
    FieldAccess(String, String),
    StringLit(String),
    FloatLit(f64),
}

/// The complete compiled program: globals, patterns, learnables, rules, main code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    /// Global variable names (index = slot).
    pub globals: Vec<String>,
    /// Compiled user-defined patterns.
    pub patterns: Vec<CompiledFn>,
    /// Compiled learnable patterns (LLM-backed).
    pub learnables: Vec<CompiledLearnableInfo>,
    /// Compiled rules.
    pub rules: Vec<CompiledRule>,
    /// Compiled skill indices (for resolve_skill_index).
    pub skill_indices: Vec<CompiledSkillIndex>,
    /// Наряд №199 (ADR-0121): compiled `reflex` declarations (Dense only).
    /// Processed by `Vm::load_program` to register models in the VM's own
    /// `reflex_registry`. Empty vec when no reflex declarations are present.
    #[serde(default)]
    pub reflex_decls: Vec<CompiledReflexDecl>,
    /// Наряд №204 (ADR-0121 stages 3-4): compiled `reflex_seq` declarations.
    /// Candle-feature-gated — only populated when `--features candle`.
    #[serde(default)]
    pub reflex_seq_decls: Vec<CompiledReflexSeqDecl>,
    /// Наряд №204 (ADR-0121 stage 4): compiled `reflex_gen` declarations.
    /// Candle-feature-gated — only populated when `--features candle`.
    #[serde(default)]
    pub reflex_gen_decls: Vec<CompiledReflexGenDecl>,
    /// Наряд №240 (Vision R4.2): compiled `vision` declarations.
    /// Processed by `Vm::load_program` and the interpreter's declaration
    /// pass to register name → parameters for `vision_generate` dispatch.
    /// Empty vec when no vision declarations are present.
    #[serde(default)]
    pub vision_decls: Vec<CompiledVisionDecl>,
    /// Наряд №332 (ADR-0164): compiled `origin` declarations.
    /// Processed by `Vm::load_program` and the interpreter's declaration
    /// pass for `media_source_capture` dispatch. Empty vec when no origin
    /// declarations are present.
    #[serde(default)]
    pub origin_decls: Vec<CompiledOriginDecl>,
    /// Наряд №392: compiled `on_deny(<class|*>) { body }` handlers.
    /// Indexed by the deny-armed SinkCheck's `deny.handler`. Empty vec
    /// when the program declares no deny handlers.
    #[serde(default)]
    pub deny_handlers: Vec<CompiledDenyHandler>,
    /// Database URL (if declared). Enables db_insert, query_scalar, etc.
    pub db_url: Option<String>,
    /// Наряд №204 (ADR-0121 stage 2): memory persist path from
    /// `memory { persist: "path.db" }` declaration. Enables reflex_save/
    /// reflex_load on the VM (same field the interpreter has at
    /// `interpreter.memory_persist_path`).
    #[serde(default)]
    pub memory_persist_path: Option<String>,
    /// Schema DDL statements to execute on DB init (CREATE TABLE IF NOT EXISTS).
    pub schema_ddl: Vec<String>,
    /// Top-level instruction sequence (declarations + flow execution).
    pub main_code: Vec<Instruction>,
    /// Whether std/collections has been imported (enables map/filter/reduce).
    pub collections_loaded: bool,
    /// Naryad #402 (step A): lazily-built IMMUTABLE shared snapshots of the
    /// collections the per-request VM installs on every `load_program`
    /// (rules pre-sorted, the RegisterPattern scan of main_code, deny
    /// handlers, skill indices, global names). Built ONCE per program on
    /// the first load, then every per-request `load_program` pays one Arc
    /// increment instead of a deep clone — the honest content of the
    /// "Arc<Program> step A" (the ServerState itself has shared the
    /// program as `Arc<Program>` since naryad #40; the aggregate deep copy
    /// actually lived here, inside load_program). NOT serialized — the
    /// snapshots are pure derivations of the wire fields and are rebuilt
    /// on deserialization.
    #[serde(skip, default = "ProgramSharedCache::new")]
    pub shared_cache: ProgramSharedCache,
}

/// Naryad #402 (step A): the lazily-built shared snapshots behind
/// `Program::shared_cache`. `OnceLock` per snapshot: the FIRST
/// `load_program` pays the build (the same deep copy the old code paid
/// on EVERY request); every later request clones an `Arc` (one atomic
/// increment). Mutation paths (the run()-only `RegisterPattern` push)
/// go through `Arc::make_mut` — copy-on-write, so the mutable run path
/// keeps today's semantics at today's cost, and the serve path (which
/// never executes main_code) never pays it.
#[derive(Debug, Default, Clone)]
pub struct ProgramSharedCache {
    rules_sorted: std::sync::OnceLock<std::sync::Arc<Vec<CompiledRule>>>,
    pre_registered_patterns: std::sync::OnceLock<std::sync::Arc<Vec<CompiledFn>>>,
    deny_handlers: std::sync::OnceLock<std::sync::Arc<Vec<CompiledDenyHandler>>>,
    skill_indices: std::sync::OnceLock<std::sync::Arc<Vec<CompiledSkillIndex>>>,
    global_names: std::sync::OnceLock<std::sync::Arc<Vec<String>>>,
    /// №409: the schema-DDL snapshot — applied once per LAZY connection
    /// open (see `Vm::ensure_db_open`), never deep-copied per request.
    schema_ddl: std::sync::OnceLock<std::sync::Arc<Vec<String>>>,
}

impl ProgramSharedCache {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Maximum accepted size for .mbc byte input. Real programs compile to
/// well under 1 MB; this bounds worst-case memory use on untrusted input.
/// Наряд №146.
const DESERIALIZE_BYTE_LIMIT: usize = 50 * 1024 * 1024;

impl Program {
    /// Serialize the program to a binary .mbc file.
    pub fn serialize(&self) -> Result<Vec<u8>, String> {
        bincode::serde::encode_to_vec(self, bincode::config::legacy())
            .map_err(|e| format!("serialize: {}", e))
    }

    // ── Naryad #402 (step A): the shared-snapshot accessors ────────────
    // Each builds ONCE (the exact same content the per-request path used
    // to deep-copy on every load), then hands out Arc clones.

    /// The rules table, pre-sorted by priority descending — the order
    /// `Vm::load_program` used to establish per request. Read-only at
    /// runtime: no Vm path mutates the rule table after load.
    pub fn rules_sorted(&self) -> std::sync::Arc<Vec<CompiledRule>> {
        self.shared_cache
            .rules_sorted
            .get_or_init(|| {
                let mut rules = self.rules.clone();
                rules.sort_by_key(|b| std::cmp::Reverse(b.priority));
                std::sync::Arc::new(rules)
            })
            .clone()
    }

    /// The patterns pre-registered from main_code's `RegisterPattern`
    /// instructions — the scan `Vm::load_program` used to run per request
    /// (№250 ADR-0122 #208: the serve path never executes main_code, so
    /// the scan+clone ran on every request). Compiler index order is
    /// preserved 1:1 (pass1 assigns idx by declaration order, pass2 emits
    /// RegisterPattern in the same order).
    pub fn pre_registered_patterns(&self) -> std::sync::Arc<Vec<CompiledFn>> {
        self.shared_cache
            .pre_registered_patterns
            .get_or_init(|| {
                std::sync::Arc::new(
                    self.main_code
                        .iter()
                        .filter_map(|instr| match instr {
                            Instruction::RegisterPattern(fn_def) => Some(fn_def.clone()),
                            _ => None,
                        })
                        .collect(),
                )
            })
            .clone()
    }

    /// The deny handler table (№392) — read-only after load
    /// (`select_handler` + handler lookup).
    pub fn deny_handlers_shared(&self) -> std::sync::Arc<Vec<CompiledDenyHandler>> {
        self.shared_cache
            .deny_handlers
            .get_or_init(|| std::sync::Arc::new(self.deny_handlers.clone()))
            .clone()
    }

    /// The skill index declarations — read-only after load
    /// (`resolve_skill_index`).
    pub fn skill_indices_shared(&self) -> std::sync::Arc<Vec<CompiledSkillIndex>> {
        self.shared_cache
            .skill_indices
            .get_or_init(|| std::sync::Arc::new(self.skill_indices.clone()))
            .clone()
    }

    /// The global variable NAMES (index = slot) — read-only after load
    /// (slot resolution for `LoadGlobalByName`).
    pub fn global_names_shared(&self) -> std::sync::Arc<Vec<String>> {
        self.shared_cache
            .global_names
            .get_or_init(|| std::sync::Arc::new(self.globals.clone()))
            .clone()
    }

    /// №409: the schema-DDL statements (CREATE TABLE IF NOT EXISTS ...) —
    /// read-only program data the per-request VM used to receive only
    /// indirectly (the eager open ran it from `&program.schema_ddl` at
    /// every `load_program`). With the LAZY db open the VM stores this
    /// shared snapshot at load and applies it once per connection open —
    /// one Arc increment per request instead of retaining a private copy.
    pub fn schema_ddl_shared(&self) -> std::sync::Arc<Vec<String>> {
        self.shared_cache
            .schema_ddl
            .get_or_init(|| std::sync::Arc::new(self.schema_ddl.clone()))
            .clone()
    }

    /// Deserialize a program from binary .mbc data.
    pub fn deserialize(data: &[u8]) -> Result<Program, String> {
        if data.len() > DESERIALIZE_BYTE_LIMIT {
            return Err(format!(
                "deserialize: input exceeds {} bytes limit",
                DESERIALIZE_BYTE_LIMIT
            ));
        }
        let config = bincode::config::legacy().with_limit::<DESERIALIZE_BYTE_LIMIT>();
        let (program, _bytes_read) = bincode::serde::decode_from_slice(data, config)
            .map_err(|e| format!("deserialize: {}", e))?;
        Ok(program)
    }
}

/// Memory entry for the VM's memory store.
///
/// `mem_type` classifies the entry for type-aware recall and differentiated decay.
/// Types follow a taxonomy inspired by TencentDB-Agent-Memory:
///   ""             — legacy/untyped (backward compatible)
///   "persona"       — user profile, stable preferences, long-term traits
///   "episodic"      — specific events, conversations, one-time occurrences
///   "instruction"   — rules, directives, standing orders
///   "fact"          — factual knowledge, objective information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmMemoryEntry {
    pub value: String,
    pub priority: f64,
    pub timestamp: i64,
    pub decay_rate: f64,
    #[serde(default)]
    pub mem_type: String,
}

/// Knowledge graph relation for the VM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmRelation {
    pub from: String,
    pub to: String,
    pub relation: String,
}

/// A compiled route body — bytecode for a single HTTP route handler.
/// Produced by Compiler::compile_route_body, consumed by Vm::execute_route_code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledRoute {
    /// Route path (e.g., "/webhook/telegram").
    pub path: String,
    /// HTTP method (e.g., "GET", "POST").
    pub method: String,
    /// Required roles (from RouteDecl::requires).
    pub requires: Vec<String>,
    /// Compiled bytecode for the route body.
    pub code: Vec<Instruction>,
}

/// Наряд №199 (ADR-0121): a compiled `reflex Name { ... }` declaration.
///
/// Stored in `Program::reflex_decls` and processed by `Vm::load_program` to
/// register each model into the VM's own `reflex_registry` — mirroring the
/// interpreter's `Declaration::Reflex(r)` handling at
/// `src/interpreter/execution.rs`. The VM must own its own ReflexRegistry
/// (not borrow the interpreter's) because the VM is a separate execution
/// backend that may run without the interpreter ever being instantiated.
///
/// Only the Dense classification path (`reflex Name { ... }`) is included
/// in stage 1 of ADR-0121. `reflex_seq` and `reflex_gen` remain excluded
/// from the VM until stages 3-4.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledReflexDecl {
    pub name: String,
    pub input_dim: usize,
    pub layers: Vec<CompiledReflexLayerSpec>,
    pub labels: Vec<String>,
    pub seed: u64,
}

/// A single layer specification in a compiled reflex declaration.
/// e.g. `dense(8, relu)` → { name: "dense", args: ["8", "relu"] }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledReflexLayerSpec {
    pub name: String,
    pub args: Vec<String>,
}

/// Наряд №204 (ADR-0121 stage 3): compiled `reflex_seq Name { ... }` declaration.
/// Candle-feature-gated — the VM registers these only when `--features candle`.
/// Mirrors `ast::ReflexSeqDecl` minus the span (not needed at runtime).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledReflexSeqDecl {
    pub name: String,
    pub input_dim: usize,
    pub seq_len: usize,
    pub layers: Vec<CompiledReflexLayerSpec>,
    pub labels: Vec<String>,
    pub seed: u64,
}

/// Наряд №204 (ADR-0121 stage 4): compiled `reflex_gen Name { ... }` declaration.
/// Candle-feature-gated — the VM registers these only when `--features candle`.
/// Mirrors `ast::ReflexGenDecl` minus the span (not needed at runtime).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledReflexGenDecl {
    pub name: String,
    pub input_dim: usize,
    pub vocab_size: usize,
    pub layers: Vec<CompiledReflexLayerSpec>,
    pub seed: u64,
}

/// Наряд №240 (Vision R4.2): usage policy of a `vision { }` declaration.
/// Mirrors `ast::VisionPolicy` (R4.1: only `Safe`) minus the span, in a
/// serde-serializable form for `Program` round-trips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledVisionPolicy {
    Safe,
}

/// Наряд №240 (Vision R4.2): VRAM/compute profile of a `vision { }` declaration.
/// Mirrors `ast::VisionProfile` (ADR-0124: `fp16 | fp8 | gguf-q4`) minus the
/// span. The profile is recorded on the declaration (R5 manifest territory);
/// the R4.2 inference path is fp32 — no silent reinterpretation of the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledVisionProfile {
    Fp16,
    Fp8,
    GgufQ4,
}

/// Наряд №240 (Vision R4.2): compiled `vision "Name" { ... }` declaration.
/// Mirrors `ast::VisionDecl` (R4.1) minus the span — fields 1:1:
/// name, model, steps, width, height, seed, policy, profile.
/// Processed by `Vm::load_program` / the interpreter's declaration pass to
/// register name → parameters for the `vision_generate` dispatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledVisionDecl {
    pub name: String,
    /// Model id — runtime re-checked against `KNOWN_VISION_MODELS`
    /// (defense-in-depth; semantic checks at compile time).
    pub model: String,
    /// Number of Euler updates (sampler sigmas = steps + 1; decl steps=8
    /// is the ADR-0124 recommended distilled-NFE setting).
    pub steps: u32,
    pub width: u32,
    pub height: u32,
    pub seed: u64,
    /// Наряд №241 (Block 3.1, ADR-0125 SSOT): `None` = the declaration
    /// omitted `policy:` (audit Warning VISION_POLICY_MISSING).
    pub policy: Option<CompiledVisionPolicy>,
    pub profile: CompiledVisionProfile,
}

/// Наряд №332 (ADR-0164): a compiled perception origin — the declared
/// SOURCE of handles (kind camera|file|generation, media kind, label
/// conf, optional file path). Validation lives in semantic (loud);
/// the runtime re-checks the shape (defense-in-depth, №240 lecalo).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CompiledOriginDecl {
    pub name: String,
    /// camera | file | generation.
    pub kind: String,
    /// image | audio | video_frame | video_segment.
    pub media: String,
    /// public | consented | private.
    pub conf: String,
    /// Required for `kind: file` — the sandboxed capture path.
    pub path: Option<String>,
}

impl CompiledOriginDecl {
    /// Single conversion point from the AST (№240 lecalo): field
    /// extraction happens HERE only.
    pub fn from_ast(v: &crate::ast::OriginDecl) -> Result<Self, String> {
        let get = |k: &str| -> Option<String> {
            v.fields
                .iter()
                .find(|(fk, _)| fk == k)
                .map(|(_, fv)| fv.clone())
        };
        let kind = get("kind")
            .ok_or_else(|| format!("origin '{}': missing required field 'kind'", v.name))?;
        let media = get("media")
            .ok_or_else(|| format!("origin '{}': missing required field 'media'", v.name))?;
        let conf = get("label")
            .ok_or_else(|| format!("origin '{}': missing required field 'label'", v.name))?;
        let path = get("path");
        // №387: a `kind: file` origin REQUIRES the sandboxed capture
        // path. A `kind: likeness` origin takes a path OPTIONALLY — the
        // file-backed capture uses it when present; the ProvBind
        // construction (`from <origin> media_store_image(…)`) needs no
        // file at all (the ritual gates the EGRESS side, not capture).
        if kind == "file" && path.is_none() {
            return Err(format!(
                "origin '{}': kind 'file' requires the 'path' field (the sandboxed capture source)",
                v.name
            ));
        }
        Ok(Self {
            name: v.name.clone(),
            kind,
            media,
            conf,
            path,
        })
    }
}

impl CompiledVisionDecl {
    /// Single conversion point from the AST (used by the compiler's pass1
    /// and the interpreter's declaration pass — no field-by-field
    /// duplication between the two backends).
    pub fn from_ast(v: &crate::ast::VisionDecl) -> Self {
        Self {
            name: v.name.clone(),
            model: v.model.clone(),
            steps: v.steps,
            width: v.width,
            height: v.height,
            seed: v.seed,
            policy: v.policy.map(|p| match p {
                crate::ast::VisionPolicy::Safe => CompiledVisionPolicy::Safe,
            }),
            profile: match v.profile {
                crate::ast::VisionProfile::Fp16 => CompiledVisionProfile::Fp16,
                crate::ast::VisionProfile::Fp8 => CompiledVisionProfile::Fp8,
                crate::ast::VisionProfile::GgufQ4 => CompiledVisionProfile::GgufQ4,
            },
        }
    }
}

/// A call frame for function invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallFrame {
    /// Instruction pointer to return to after the call.
    pub return_ip: usize,
    /// Base pointer for local variables (parameters).
    pub base_bp: usize,
}

// ── JIT eligibility (Наряд №328, ADR-0156) ───────────────────────────
//
// The "dispatch gap = explicit error" rule: label instructions are NOT
// in the JIT-eligible class (arithmetic-only). `is_jit_eligible` is the
// SSOT predicate for the (future) JIT dispatcher: a function containing
// LabelJoin/SinkCheck must never be silently skipped by a JIT pass —
// the dispatcher is required to reject such functions with a distinct
// error naming ADR-0156.

pub fn is_jit_eligible(instrs: &[Instruction]) -> bool {
    instrs.iter().all(|i| {
        !matches!(
            i,
            Instruction::LabelJoin { .. } | Instruction::SinkCheck { .. }
        )
    })
}
