// ── METALOGOS — library root ──────────────────────────────────────────
// Exposes: parser, interpreter, LLM client, semantic analysis, run_program for binary + tests.

pub mod ast;
pub mod builtins;
pub mod interpreter;
pub mod llm;
pub mod parser;
pub mod semantic;
pub mod server;

/// Parse and execute a .mlog program. Returns the flow output (if any),
/// with mutate status messages prepended if present.
pub fn run_program(source: &str) -> Result<Option<String>, String> {
    run_program_with_dir(source, std::path::PathBuf::from("."))
}

/// Parse and execute a .mlog program with an explicit base directory for module resolution.
pub fn run_program_with_dir(source: &str, base_dir: std::path::PathBuf) -> Result<Option<String>, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut interp = interpreter::Interpreter::new();
    interp.set_base_dir(base_dir);
    let output = interp.run(declarations)?;

    // Prepend mutate log messages to output
    let mutate_log = interp.take_mutate_log();
    if mutate_log.is_empty() {
        Ok(output)
    } else {
        match output {
            Some(flow_output) => {
                let mut result = mutate_log.join("\n");
                result.push_str("\n");
                result.push_str(&flow_output);
                Ok(Some(result))
            }
            None => {
                Ok(Some(mutate_log.join("\n")))
            }
        }
    }
}

/// Parse and check a .mlog program without execution.
/// Returns an AnalysisResult with errors and warnings.
pub fn check_program(source: &str) -> Result<semantic::AnalysisResult, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    Ok(semantic::check_program(&declarations))
}

/// Parse a single line and feed it to a reusable interpreter.
/// Used by REPL for incremental evaluation with persistent state.
/// Returns the output (if any) from processing the declaration.
pub fn feed_line(interp: &mut interpreter::Interpreter, line: &str) -> Result<Option<String>, String> {
    let declarations = parser::parse(line).map_err(|e| format!("parse error: {}", e))?;
    let output = interp.run(declarations)?;
    let mutate_log = interp.take_mutate_log();
    if mutate_log.is_empty() {
        Ok(output)
    } else {
        match output {
            Some(flow_output) => {
                let mut result = mutate_log.join("\n");
                result.push_str("\n");
                result.push_str(&flow_output);
                Ok(Some(result))
            }
            None => Ok(Some(mutate_log.join("\n"))),
        }
    }
}

/// Serve a .mlog program as an HTTP server.
/// Parses the source, finds the mlogserver block, and starts Axum.
#[cfg(feature = "server")]
pub async fn serve_program(source: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    server::run_server(source).await
}
