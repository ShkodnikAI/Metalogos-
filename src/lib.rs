// ── METALOGOS — library root ──────────────────────────────────────────
// Exposes: parser, interpreter, LLM client, semantic analysis, run_program for binary + tests.
//
// Наряд №29 §6.4: deny clippy::unwrap_used / clippy::expect_used in non-test code.
// All production unwrap/expect calls have been eliminated (Наряд №29 Blocks 3.1–3.4).
// Grammar invariant .expect() calls in parser.rs are allowed via module-level #![allow].

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod ast;
pub mod audit;
pub mod builtins;
pub mod bytecode;
pub mod compiler;
pub mod embeddings;
pub mod error;
pub mod interpreter;
pub mod llm;
pub mod memory_graph;
pub mod memory_store;
pub mod parser;
pub mod semantic;
pub mod server;
pub mod vm;

/// Parse and execute a .mlog program. Returns the flow output (if any),
/// with mutate status messages prepended if present.
pub fn run_program(source: &str) -> Result<Option<String>, String> {
    run_program_with_dir(source, std::path::PathBuf::from("."))
}

/// Parse and execute a .mlog program with an explicit base directory for module resolution.
pub fn run_program_with_dir(
    source: &str,
    base_dir: std::path::PathBuf,
) -> Result<Option<String>, String> {
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
            None => Ok(Some(mutate_log.join("\n"))),
        }
    }
}

/// Static security audit on a .mlog program (ADR-0057).
/// Analyzes without executing; returns AuditResult with findings.
pub fn audit_program(source: &str) -> Result<audit::AuditResult, String> {
    audit::audit_program(source)
}

/// Parse and check a .mlog program without execution.
/// Returns an AnalysisResult with errors and warnings.
pub fn check_program(source: &str) -> Result<semantic::AnalysisResult, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    Ok(semantic::check_program(&declarations))
}

/// Parse and check a .mlog program, resolving imports against an optional
/// root file.  When `root` is Some(path), the root file is parsed and
/// executed (without running flows), then the target file's declarations
/// are checked against the merged symbol table.  This allows `mlog check
/// dept/utils.mlog --root app.mlog` to see patterns imported from std/.
pub fn check_program_with_root(
    source: &str,
    root: Option<&std::path::Path>,
) -> Result<semantic::AnalysisResult, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;

    // If no root file, just check the file in isolation
    let root_decls = match root {
        Some(root_path) => {
            let root_source = std::fs::read_to_string(root_path)
                .map_err(|e| format!("cannot read root file {:?}: {}", root_path, e))?;
            let root_dir = root_path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .to_path_buf();
            let root_decls = parser::parse(&root_source)
                .map_err(|e| format!("parse error in root file: {}", e))?;
            // Execute root declarations into an interpreter to resolve imports
            let mut interp = interpreter::Interpreter::new();
            interp.set_base_dir(root_dir);
            interp.run(root_decls)?;
            // Collect all declarations known to the interpreter (merged from imports)
            interp.collect_declarations()
        }
        None => vec![],
    };

    // Merge: root declarations first, then the file being checked
    let mut all_decls = root_decls;
    all_decls.extend(declarations);

    Ok(semantic::check_program(&all_decls))
}

/// Parse a single line and feed it to a reusable interpreter.
/// Used by REPL for incremental evaluation with persistent state.
/// Returns the output (if any) from processing the declaration.
pub fn feed_line(
    interp: &mut interpreter::Interpreter,
    line: &str,
) -> Result<Option<String>, String> {
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
pub async fn serve_program(source: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    server::run_server(source).await
}

/// Compile a .mlog source to bytecode Program.
/// Note: imports are not resolved during compilation. Use `mlog serve` for import support.
pub fn compile_program(source: &str) -> Result<bytecode::Program, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    // Warn if imports are present — bytecode compiler cannot resolve them
    for decl in &declarations {
        if let ast::Declaration::Import(import) = decl {
            eprintln!("warning: import '{}' cannot be resolved in compile mode (use `mlog serve` instead)", import.path);
        }
    }
    let mut compiler = compiler::Compiler::new();
    compiler.compile(declarations)
}

/// Run a pre-compiled bytecode Program on the VM.
pub fn run_bytecode(program: bytecode::Program) -> Result<Option<String>, String> {
    let mut vm = vm::Vm::new();
    vm.run(program)
}

/// Parse a .mlog program, execute declarations, then run all eval blocks (ADR-0050).
/// Returns a list of EvalResult structs with accuracy, confusion matrix, and failure details.
pub fn eval_program(source: &str) -> Result<Vec<interpreter::EvalResult>, String> {
    eval_program_with_dir(source, std::path::PathBuf::from("."))
}

/// Parse a .mlog program with an explicit base directory, execute declarations, then run eval blocks.
pub fn eval_program_with_dir(
    source: &str,
    base_dir: std::path::PathBuf,
) -> Result<Vec<interpreter::EvalResult>, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut interp = interpreter::Interpreter::new();
    interp.set_base_dir(base_dir);
    interp.run(declarations)?;
    interp.run_eval_blocks()
}

/// ADR-0056: Resume a flow from a checkpoint.
/// Parses the source, sets resume target, then runs — the flow skips to the checkpoint.
pub fn resume_program(
    source: &str,
    flow_name: &str,
    checkpoint_name: &str,
) -> Result<Option<String>, String> {
    resume_program_with_dir(
        source,
        flow_name,
        checkpoint_name,
        std::path::PathBuf::from("."),
    )
}

/// ADR-0056: Resume a flow from a checkpoint with explicit base directory.
pub fn resume_program_with_dir(
    source: &str,
    flow_name: &str,
    checkpoint_name: &str,
    base_dir: std::path::PathBuf,
) -> Result<Option<String>, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut interp = interpreter::Interpreter::new();
    interp.set_base_dir(base_dir);
    interp.set_resume_target(flow_name, checkpoint_name);
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
