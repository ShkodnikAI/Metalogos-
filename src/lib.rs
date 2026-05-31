// ── METALOGOS — library root ──────────────────────────────────────────────
// Pipeline: parse → semantic analysis → codegen (IR) → interpreter

pub mod ast;
pub mod builtins;
pub mod codegen;
pub mod interpreter;
pub mod ir;
pub mod llm;
pub mod parser;
pub mod semantic;

/// Parse and execute a .mlog program.
/// Pipeline: source → parse → semantic analyze → codegen → interpret
/// Returns the flow output (if any) or a semantic/parse error.
pub fn run_program(source: &str) -> Result<Option<String>, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let ir_program = codegen::compile(declarations)?; // semantic analysis + IR generation
    let mut interp = interpreter::Interpreter::new();
    interp.run(ir_program.declarations)
}
