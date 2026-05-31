// ── METALOGOS — library root ──────────────────────────────────────────────
// Pipeline: parse → semantic analysis → codegen (IR) → interpreter

pub mod ast;
pub mod builtins;
pub mod codegen;
pub mod embedding;
pub mod interpreter;
pub mod ir;
pub mod llm;
pub mod ml;
pub mod parser;
pub mod semantic;

/// Parse and execute a .mlog program.
/// Pipeline: source → parse → semantic analyze → codegen → interpret
/// Returns the flow output (if any) or a semantic/parse error.
/// Learn/mutate status messages and warnings are prepended to the output.
pub fn run_program(source: &str) -> Result<Option<String>, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let (ir_program, warnings) = codegen::compile(declarations)?;
    let mut interp = interpreter::Interpreter::new();
    let mut output = interp.run(ir_program.declarations)?;

    // Collect learn and mutate status messages
    let learn_log = interp.take_learn_log();
    let mutate_log = interp.take_mutate_log();

    // Prepend learn log + mutate log + warnings to output
    let mut prefix_parts: Vec<String> = Vec::new();
    if !learn_log.is_empty() {
        prefix_parts.push(learn_log.join("\n"));
    }
    if !mutate_log.is_empty() {
        prefix_parts.push(mutate_log.join("\n"));
    }
    if !warnings.is_empty() {
        prefix_parts.push(warnings.join("\n"));
    }

    if !prefix_parts.is_empty() {
        let prefix_text = prefix_parts.join("\n");
        match &mut output {
            Some(ref mut s) => *s = format!("{}\n{}", prefix_text, s),
            None => output = Some(prefix_text),
        }
    }

    Ok(output)
}
