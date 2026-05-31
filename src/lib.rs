// ── METALOGOS — library root ──────────────────────────────────────────────
// Pipeline: parse → semantic analysis → codegen (IR) → interpreter

pub mod ast;
pub mod builtins;
pub mod codegen;
pub mod embedding;
pub mod interpreter;
pub mod ir;
pub mod llm;
pub mod parser;
pub mod semantic;

/// Parse and execute a .mlog program.
/// Pipeline: source → parse → semantic analyze → codegen → interpret
/// Returns the flow output (if any) or a semantic/parse error.
/// Warnings are prepended to the output.
pub fn run_program(source: &str) -> Result<Option<String>, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let (ir_program, warnings) = codegen::compile(declarations)?;
    let mut interp = interpreter::Interpreter::new();
    let mut output = interp.run(ir_program.declarations)?;

    // Prepend warnings to output
    if !warnings.is_empty() {
        let warning_text = warnings.join("\n");
        match &mut output {
            Some(ref mut s) => *s = format!("{}\n{}", warning_text, s),
            None => output = Some(warning_text),
        }
    }

    Ok(output)
}
