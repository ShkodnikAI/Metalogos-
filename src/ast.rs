// ── AST types for METALOGOS M1 ────────────────────────────────────────
// Minimal types to represent entity, pattern, flow and their expressions.

use std::fmt;

/// Top-level declaration in a .mlog program.
#[derive(Debug, Clone)]
pub enum Declaration {
    Entity(EntityDecl),
    Pattern(PatternDecl),
    Flow(FlowDecl),
}

/// `entity greeting: String = "Hello, Metalogos!"`
#[derive(Debug, Clone)]
pub struct EntityDecl {
    pub name: String,
    pub type_name: String,
    pub value: Expr,
}

/// `pattern Shout(s: String) -> String { return upper(s) + "!" }`
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

/// `flow Main { input: String = greeting -> Shout -> output }`
#[derive(Debug, Clone)]
pub struct FlowDecl {
    pub name: String,
    pub input_type: String,
    pub source: Expr,
    pub pipeline: Vec<String>, // step names in order
}

/// Expressions
#[derive(Debug, Clone)]
pub enum Expr {
    StringLit(String),
    Ident(String),
    FnCall(String, Vec<Expr>), // name, args
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
