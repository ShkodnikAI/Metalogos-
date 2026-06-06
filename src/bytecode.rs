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
    /// Begin flow execution: load source, step through pipeline.
    /// Encoded as a single instruction that carries the flow definition.
    /// The VM interprets it by loading the source, then calling each step.
    /// This is a "macro instruction" — the VM expands it internally.
    FlowExec {
        source_expr: FlowExpr,
        pipeline: Vec<String>,
        branch_defs: Vec<(String, Vec<BranchDef>)>,
    },

    // ── Rule Engine ───────────────────────────────────────────
    /// Execute all registered rules (conditions + assignments).
    /// This is also a macro instruction — the VM evaluates rules internally.
    ExecuteRules,

    // ── Meta ──────────────────────────────────────────────────
    /// End of program.
    Halt,
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

/// A compiled learnable pattern: name, prompt, few-shot examples.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledLearnableInfo {
    pub name: String,
    pub param_count: usize,
    pub prompt: String,
    pub few_shot: Vec<(String, String)>,
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
    Contains { left: RuleValueExpr, right: RuleValueExpr },
    Compare { left: RuleValueExpr, op: ConditionOp, right: RuleValueExpr },
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
    /// Top-level instruction sequence (declarations + flow execution).
    pub main_code: Vec<Instruction>,
    /// Whether std/collections has been imported (enables map/filter/reduce).
    pub collections_loaded: bool,
}

impl Program {
    /// Serialize the program to a binary .mbc file.
    pub fn serialize(&self) -> Result<Vec<u8>, String> {
        bincode::serialize(self).map_err(|e| format!("serialize: {}", e))
    }

    /// Deserialize a program from binary .mbc data.
    pub fn deserialize(data: &[u8]) -> Result<Program, String> {
        bincode::deserialize(data).map_err(|e| format!("deserialize: {}", e))
    }
}

/// Memory entry for the VM's memory store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmMemoryEntry {
    pub value: String,
    pub priority: f64,
    pub timestamp: i64,
    pub decay_rate: f64,
}

/// Knowledge graph relation for the VM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmRelation {
    pub from: String,
    pub to: String,
    pub relation: String,
}

/// A call frame for function invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallFrame {
    /// Instruction pointer to return to after the call.
    pub return_ip: usize,
    /// Base pointer for local variables (parameters).
    pub base_bp: usize,
}
