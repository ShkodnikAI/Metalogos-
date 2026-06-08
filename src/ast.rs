// ── AST types for METALOGOS ────────────────────────────────────────

use std::fmt;

/// Top-level declaration in a .mlog program.
#[derive(Debug, Clone)]
pub enum Declaration {
    /// `mlogserver { port: 8080, middleware: [...], route ... }` (Phase 6.1)
    MlogServer(MlogServerDecl),
    /// `template Name(params) -> Html { <html>...</html> }` (Phase 6.2)
    Template(TemplateDecl),
    /// `db { url: ..., pool_size: 10 }` (Phase 6.3)
    Db(DbDecl),
    /// `memory { persist: "./data/memory.db" }` (Phase 7.6)
    Memory(MemoryDecl),
    /// `import std/string as str` or `import ./my_utils`
    Import(ImportDecl),
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
    /// `relate "from" to "to" as "relation"`
    Relate(RelateDecl),
    /// `sandbox name { allowed: [...], forbidden: [...], timeout: N }`
    Sandbox(SandboxDecl),
    /// `hook before_pattern { <statements> }` or `hook after_pattern { <statements> }` (ADR-0045)
    Hook(HookDecl),
    /// `mutate PatternName { add_example(...) rollback_if: accuracy op threshold }`
    Mutate(MutateDecl),
    /// `pattern Name(params) -> Type { body }`
    Pattern(PatternDecl),
    /// `learnable pattern Name(params) -> Type { prompt: "..." }`
    LearnablePattern(LearnablePatternDecl),
    /// `flow Name { input: Type = expr -> steps -> output }`
    Flow(FlowDecl),
}

// ── MlogServer (Phase 6.1) ──────────────────────────────────────

/// `mlogserver { port: 8080, middleware: [...], route ... { body } }`
#[derive(Debug, Clone)]
pub struct MlogServerDecl {
    pub port: u16,
    pub middleware: Vec<String>,
    pub routes: Vec<RouteDecl>,
}

/// `route "/path" method=GET requires=[admin] { body }`
#[derive(Debug, Clone)]
pub struct RouteDecl {
    pub path: String,
    pub method: String,       // "GET", "POST", "PUT", "DELETE"
    pub requires: Vec<String>, // role names
    pub body: Vec<Statement>,
}

// ── Template (Phase 6.2) ──────────────────────────────────────

/// `template Page(title: String, body: String) -> Html { <html>...</html> }`
#[derive(Debug, Clone)]
pub struct TemplateDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: String,
    pub body: String, // Raw template body with {{ var }} placeholders
}

// ── DB (Phase 6.3) ──────────────────────────────────────

/// `db { url: expr, pool_size: 10, migrate: "./migrations" }`
#[derive(Debug, Clone)]
pub struct DbDecl {
    pub url: Option<Expr>,
    pub pool_size: Option<u32>,
    pub migrate: Option<String>,
}

// ── Memory Config (Phase 7.6) ──────────────────────────────

/// `memory { persist: "./data/memory.db" }`
/// Without persist → in-memory stores (backward compatible).
/// With persist → SQLite, auto-creates file and directories.
#[derive(Debug, Clone)]
pub struct MemoryDecl {
    /// Path to SQLite database file. If None, uses in-memory stores.
    pub persist: Option<String>,
}

// ── Import (Phase 5.4) ─────────────────────────────────────

/// `import std/string as str` or `import ./my_utils`
#[derive(Debug, Clone)]
pub struct ImportDecl {
    /// Module path: "std/string", "./my_utils", "pkg/utils"
    pub path: String,
    /// Optional alias: `as str` → Some("str"). Without `as` → None (global merge).
    pub alias: Option<String>,
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
    Ne,
}

impl fmt::Display for CompareOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompareOp::Gt => write!(f, ">"),
            CompareOp::Lt => write!(f, "<"),
            CompareOp::Ge => write!(f, ">="),
            CompareOp::Le => write!(f, "<="),
            CompareOp::Eq => write!(f, "=="),
            CompareOp::Ne => write!(f, "!="),
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

// ── Hook (ADR-0045): before_pattern / after_pattern ──────────────────────

/// Hook phase: before or after pattern invocation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HookPhase {
    /// Execute statements BEFORE the pattern is invoked.
    BeforePattern,
    /// Execute statements AFTER the pattern returns.
    AfterPattern,
}

/// `hook before_pattern { <statements> }` or `hook after_pattern { <statements> }`
/// Variables available in hook body: pattern_name (String), args (List),
/// result (after only), confidence (after only).
#[derive(Debug, Clone)]
pub struct HookDecl {
    pub phase: HookPhase,
    pub body: Vec<Statement>,
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
///   context: recall(text, limit=5)   // optional
///   max_tokens: 4000                 // optional
///   cache: true                      // optional — enable LLM response caching
///   cache_ttl: 60.minutes            // optional — time-to-live for cached responses
/// }`
#[derive(Debug, Clone)]
pub struct LearnablePatternDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: String,
    pub prompt: String,
    /// Optional context auto-loading: `recall(query_expr, limit=N)`.
    pub context_query: Option<Expr>,
    pub context_limit: Option<usize>,
    /// Optional max_tokens for LLM backend.
    pub max_tokens: Option<u32>,
    /// Enable LLM response caching. When true, identical (prompt + args) calls
    /// return cached response without hitting the LLM backend.
    pub cache: bool,
    /// Cache time-to-live in seconds. Default 3600 (1 hour) when cache is enabled.
    pub cache_ttl: u64,
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
    LetBinding { name: String, value: Expr },
    Assign { name: String, value: Expr },
    Each { variable: String, iterable: Expr, body: Vec<Statement> },
    While { condition: Expr, body: Vec<Statement> },
    /// Block-style if: `if expr { stmts } else if expr { stmts } else { stmts }` (v0.5.0)
    IfElseBlock {
        condition: Expr,
        then_body: Vec<Statement>,
        else_ifs: Vec<(Expr, Vec<Statement>)>,
        else_body: Option<Vec<Statement>>,
    },
    /// Single-branch if-then (no else): `if expr then { stmts }` (Phase 7.7)
    IfThen(Box<Expr>, Vec<Statement>),
    Return(Expr),
    /// Bare expression statement: `respond("ok")`, `http_post(...)` etc.
    /// The expression is evaluated for side effects; result is discarded unless in route context.
    ExprStmt(Expr),
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
    BoolLit(bool),
    Ident(String),
    FieldAccess(Box<Expr>, String),
    FnCall(String, Vec<Expr>),
    /// Qualified call: `module.function(args)` — resolved through namespace imports.
    QualifiedCall { module: String, function: String, args: Vec<Expr> },
    BinaryOp(Box<Expr>, BinOp, Box<Expr>),
    IfElse(Box<Expr>, Box<Expr>, Box<Expr>),
    List(Vec<Expr>),
    /// Index access: `list[index]` or `struct["field"]` (v0.5.0)
    IndexAccess(Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Gt,
    Lt,
    Ge,
    Le,
    Eq,
    Ne,
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mul => write!(f, "*"),
            BinOp::Div => write!(f, "/"),
            BinOp::Gt => write!(f, ">"),
            BinOp::Lt => write!(f, "<"),
            BinOp::Ge => write!(f, ">="),
            BinOp::Le => write!(f, "<="),
            BinOp::Eq => write!(f, "=="),
            BinOp::Ne => write!(f, "!="),
        }
    }
}

// ── Opaque type markers (used by semantic analysis) ──────────────────

/// Types that are opaque: cannot be printed, concatenated, or converted to String.
/// Used by semantic analysis to enforce security constraints.
pub const OPAQUE_TYPES: &[&str] = &[
    "Html",      // Phase 6.2: type-safe HTML, XSS prevention
    "Query",     // Phase 6.3: parameterized SQL, injection prevention
    "Secret",    // Phase 6.4: opaque secret (env vars, passwords)
    "Encrypted", // Phase 6.4: encrypted data
    "Hash",      // Phase 6.4: password hash (argon2)
    "Session",   // Phase 6.5: server session data
];

/// Check if a type name is an opaque type.
pub fn is_opaque_type(type_name: &str) -> bool {
    OPAQUE_TYPES.contains(&type_name)
}
