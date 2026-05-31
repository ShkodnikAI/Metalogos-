// ── AST types for METALOGOS M2 ────────────────────────────────────────

use std::fmt;

/// Top-level declaration in a .mlog program.
#[derive(Debug, Clone)]
pub enum Declaration {
    /// `entity TypeName { field: Type = default, ... }`
    EntityType(EntityTypeDecl),
    /// `entity name: TypeName = { field: value, ... }`
    EntityRecord(EntityRecordDecl),
    /// `entity greeting: String = "Hello"` (M1 simple)
    EntitySimple(EntitySimpleDecl),
    /// `rule If(...) then ... with priority=N`
    Rule(RuleDecl),
    /// `memorize "fact" with priority=0.9`
    Memorize(MemorizeDecl),
    /// `forget "query" after 30.days`
    Forget(ForgetDecl),
        /// `fluid x = Float[42.0][0.9] or String["answer"][0.1]`
    Fluid(FluidDecl),
    /// `adapt PatternName add_example("input", "output")`
    Adapt(AdaptDecl),
    /// `learn PatternName with { data: "corpus", epochs: 5 }`
    Learn(LearnDecl),
    /// `relate "from" to "to" as "relation"`
    Relate(RelateDecl),
    /// `sandbox name { allowed: [...], forbidden: [...], timeout: N }`
    Sandbox(SandboxDecl),
    /// `mutate PatternName { add_example(...) rollback_if: accuracy op threshold }`
    Mutate(MutateDecl),
    /// `pattern Name(params) -> Type { body }`
    Pattern(PatternDecl),
    /// `learnable pattern Name(params) -> Type { prompt: "..." }`
    LearnablePattern(LearnablePatternDecl),
    /// `flow Name { input: Type = expr -> steps -> output }`
    Flow(FlowDecl),
}

// ── Entity types ────────────────────────────────────────────────────

/// `entity Message { text: String, urgency: Float = 0.0 }`
#[derive(Debug, Clone)]
pub struct EntityTypeDecl {
    pub name: String,
    pub fields: Vec<FieldDecl>,
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub name: String,
    pub type_name: String,
    pub default: Option<Expr>,
}

/// `entity m: Message = { text: "срочно нужна помощь", urgency: 0.0 }`
#[derive(Debug, Clone)]
pub struct EntityRecordDecl {
    pub name: String,
    pub type_name: String,
    pub fields: Vec<FieldInit>,
}

#[derive(Debug, Clone)]
pub struct FieldInit {
    pub name: String,
    pub value: Expr,
}

/// `entity greeting: String = "Hello, Metalogos!"` (M1)
#[derive(Debug, Clone)]
pub struct EntitySimpleDecl {
    pub name: String,
    pub type_name: String,
    pub value: Expr,
}

// ── Rule ──────────────────────────────────────────────────────────────

/// `rule If(m.text contains "срочно") then m.urgency = 0.9 with priority=10`
#[derive(Debug, Clone)]
pub struct RuleDecl {
    pub condition: Condition,
    pub target: Expr,
    pub field: String,
    pub value: Expr,
    pub priority: i32,
}

#[derive(Debug, Clone)]
pub enum Condition {
    Contains { left: Expr, right: Expr },
    Compare { left: Expr, op: CompareOp, right: Expr },
}

#[derive(Debug, Clone, Copy)]
pub enum CompareOp {
    Gt,
    Lt,
    Ge,
    Le,
    Eq,
}

impl fmt::Display for CompareOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompareOp::Gt => write!(f, ">"),
            CompareOp::Lt => write!(f, "<"),
            CompareOp::Ge => write!(f, ">="),
            CompareOp::Le => write!(f, "<="),
            CompareOp::Eq => write!(f, "=="),
        }
    }
}

// ── Fluid Types (Phase 1) ─────────────────────────────────────

/// `fluid x = Float[42.0][0.9] or String["answer"][0.1]`
/// A superposition of typed variants with confidence scores.
#[derive(Debug, Clone)]
pub struct FluidDecl {
    pub name: String,
    pub variants: Vec<FluidVariant>,
}

/// A single variant in a fluid declaration: type + value + confidence.
#[derive(Debug, Clone)]
pub struct FluidVariant {
    pub type_name: String,
    pub value: Expr,
    pub confidence: f64,
}

// ── Adapt (M5) ────────────────────────────────────────────────

/// `adapt PatternName add_example("input", "output")`
#[derive(Debug, Clone)]
pub struct AdaptDecl {
    pub pattern_name: String,
    pub input_example: Expr,
    pub output_example: Expr,
}

// ── Learn (Phase 2.3) ────────────────────────────────────────

/// `learn PatternName with { data: "corpus", epochs: 5 }`
/// Triggers fine-tuning of a learnable pattern via the ML backend.
#[derive(Debug, Clone)]
pub struct LearnDecl {
    pub pattern_name: String,
    /// Hyperparameters: [(param_name, value_expr)]
    pub hyperparams: Vec<(String, Expr)>,
}

// ── Relate (knowledge graph edge) ──────────────────────────────

/// `relate "from" to "to" as "relation"`
#[derive(Debug, Clone)]
pub struct RelateDecl {
    pub from: Expr,
    pub to: Expr,
    pub relation: String,
}

// ── Sandbox (P2) ────────────────────────────────────────────────

/// `sandbox name { allowed: [...], forbidden: [...], timeout: N }`
#[derive(Debug, Clone)]
pub struct SandboxDecl {
    pub name: String,
    pub allowed: Vec<String>,
    pub forbidden: Vec<String>,
    pub timeout: i64,
}

// ── Mutate (P2) ─────────────────────────────────────────────────

/// `mutate PatternName { add_example("in", "out") rollback_if: accuracy op threshold }`
#[derive(Debug, Clone)]
pub struct MutateDecl {
    pub pattern_name: String,
    pub new_examples: Vec<(Expr, Expr)>,
    pub rollback_threshold: Option<f64>,
    pub rollback_op: Option<CompareOp>,
}

// ── Memory (M4) ────────────────────────────────────────────────

/// `memorize "fact" with priority=0.9`
#[derive(Debug, Clone)]
pub struct MemorizeDecl {
    pub value: Expr,
    pub priority: f64,
}

/// `forget "query" after 30.days`
#[derive(Debug, Clone)]
pub struct ForgetDecl {
    pub query: Expr,
    pub days: i64,
}

// ── Learnable Pattern (M3) ────────────────────────────────────────────

/// `learnable pattern Classify(text: String) -> Category {
///   prompt: "Классифицируй сообщение: question | complaint | greeting"
/// }`
#[derive(Debug, Clone)]
pub struct LearnablePatternDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: String,
    pub prompt: String,
}

// ── Pattern (M1, unchanged) ─────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PatternDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: String,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Return(Expr),
}

// ── Flow (M1 + M2 branching) ────────────────────────────────────────
// Pipeline is a simple list of step names; branch definitions are
// separate named blocks that appear after the pipeline line.

#[derive(Debug, Clone)]
pub struct FlowDecl {
    pub name: String,
    pub input_type: String,
    pub source: Expr,
    /// Pipeline step names (e.g. ["Classify"] from `-> Classify` in pipeline)
    pub pipeline: Vec<String>,
    /// Named branch definitions: (step_name, [Branch])
    pub branch_defs: Vec<(String, Vec<Branch>)>,
}

#[derive(Debug, Clone)]
pub struct Branch {
    pub label: String,
    pub condition: BranchCondition,
    pub target: String,
}

/// Condition inside a flow branch: `m.urgency > 0.8`
#[derive(Debug, Clone)]
pub struct BranchCondition {
    pub target: Expr,
    pub field: String,
    pub op: CompareOp,
    pub threshold: Expr,
}

// ── Expressions ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Expr {
    StringLit(String),
    FloatLit(f64),
    Ident(String),
    FieldAccess(Box<Expr>, String),
    FnCall(String, Vec<Expr>),
    BinaryOp(Box<Expr>, BinOp, Box<Expr>),
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mul => write!(f, "*"),
            BinOp::Div => write!(f, "/"),
        }
    }
}
