// ── METALOGOS — library root (M1+M2+M3) ──────────────────────────────
// Exposes: parser, interpreter, LLM client, run_program for binary + tests.

pub mod ast;
pub mod builtins;
pub mod interpreter;
pub mod llm;
pub mod parser;

/// Parse and execute a .mlog program. Returns the flow output (if any).
pub fn run_program(source: &str) -> Result<Option<String>, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut interp = interpreter::Interpreter::new();
    interp.run(declarations)
}
