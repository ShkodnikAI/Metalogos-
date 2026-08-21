// ── METALOGOS CLI ─────────────────────────────────────────────────────
// `mlog run <file.mlog>`    — execute a .mlog program
// `mlog repl`               — interactive session with persistent state
// `mlog check <file.mlog>`  — semantic analysis without execution
// `mlog serve <file.mlog>`  — start HTTP server from mlogserver block

use clap::Parser;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::fs;
use std::io::{self, BufRead, IsTerminal};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "mlog",
    about = "METALOGOS — AI-native programming language with security by design",
    version,
    propagate_version = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Run a .mlog program (or .mbc bytecode file)
    Run {
        /// Path to .mlog source file or .mbc bytecode file
        file: PathBuf,
    },
    /// Start interactive REPL session with persistent state
    Repl,
    /// Run semantic analysis without execution
    Check {
        /// Path to .mlog source file
        file: PathBuf,
        /// Optional root .mlog file for resolving imports.
        /// When checking an isolated file (e.g. dept/utils.mlog),
        /// specify the main file so its declarations are available.
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Start HTTP server from mlogserver block (Phase 6)
    #[cfg(feature = "server")]
    Serve {
        /// Path to .mlog source file
        file: PathBuf,
    },
    /// Compile a .mlog source file to .mbc bytecode
    Compile {
        /// Path to .mlog source file
        file: PathBuf,
    },
    /// Run eval blocks: test learnable patterns against datasets (ADR-0050)
    Eval {
        /// Path to .mlog source file
        file: PathBuf,
    },
    /// Resume a flow from a checkpoint (ADR-0056)
    Resume {
        /// Path to .mlog source file
        file: PathBuf,
        /// Flow name to resume
        #[arg(long)]
        flow: String,
        /// Checkpoint name to resume from
        #[arg(long)]
        from: String,
    },
    /// Static security analysis without execution (ADR-0057)
    Audit {
        /// Path to .mlog source file
        file: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file } => cmd_run(file),
        Commands::Repl => cmd_repl_stdio(),
        Commands::Check { file, root } => cmd_check(file, root),
        #[cfg(feature = "server")]
        Commands::Serve { file } => cmd_serve(file),
        Commands::Compile { file } => cmd_compile(file),
        Commands::Eval { file } => cmd_eval(file),
        Commands::Resume { file, flow, from } => cmd_resume(file, &flow, &from),
        Commands::Audit { file } => cmd_audit(file),
    }
}

/// `mlog run <file>` — parse + execute (or deserialize + VM run for .mbc)
fn cmd_run(file: PathBuf) {
    // Detect .mbc extension → bytecode path
    if file.extension().map(|e| e == "mbc").unwrap_or(false) {
        cmd_run_bytecode(file);
        return;
    }

    let source = match fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {:?}: {}", file, e);
            std::process::exit(1);
        }
    };

    match metalogos::run_program(&source) {
        Ok(output) => {
            if let Some(result) = output {
                println!("{}", result);
            }
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `mlog run <file.mbc>` — deserialize bytecode and run on VM
fn cmd_run_bytecode(file: PathBuf) {
    let data = match fs::read(&file) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: cannot read {:?}: {}", file, e);
            std::process::exit(1);
        }
    };

    let program = match metalogos::bytecode::Program::deserialize(&data) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: failed to deserialize {:?}: {}", file, e);
            std::process::exit(1);
        }
    };

    match metalogos::run_bytecode(program) {
        Ok(output) => {
            if let Some(result) = output {
                println!("{}", result);
            }
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `mlog compile <file.mlog>` — parse, compile to bytecode, write .mbc
fn cmd_compile(file: PathBuf) {
    let source = match fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {:?}: {}", file, e);
            std::process::exit(1);
        }
    };

    let program = match metalogos::compile_program(&source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    let bytecode = match program.serialize() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: serialization failed: {}", e);
            std::process::exit(1);
        }
    };

    // Write to same filename with .mbc extension
    let mbc_path = file.with_extension("mbc");
    match fs::write(&mbc_path, &bytecode) {
        Ok(_) => {
            println!(
                "Compiled {} -> {} ({} bytes)",
                file.display(),
                mbc_path.display(),
                bytecode.len()
            );
        }
        Err(e) => {
            eprintln!("error: cannot write {:?}: {}", mbc_path, e);
            std::process::exit(1);
        }
    }
}

/// `mlog eval <file>` — parse + execute declarations + run eval blocks (ADR-0050)
fn cmd_eval(file: PathBuf) {
    let source = match fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {:?}: {}", file, e);
            std::process::exit(1);
        }
    };

    match metalogos::eval_program(&source) {
        Ok(results) => {
            let mut any_failed = false;
            for result in &results {
                println!("{}", result.format_report());
                println!();
                if !result.passed {
                    any_failed = true;
                }
            }
            if any_failed {
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `mlog resume <file> --flow=Name --from=checkpoint` — resume flow from checkpoint (ADR-0056)
fn cmd_resume(file: PathBuf, flow_name: &str, checkpoint_name: &str) {
    let source = match fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {:?}: {}", file, e);
            std::process::exit(1);
        }
    };

    match metalogos::resume_program(&source, flow_name, checkpoint_name) {
        Ok(output) => {
            if let Some(result) = output {
                println!("{}", result);
            }
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `mlog audit <file>` — static security analysis without execution (ADR-0057)
fn cmd_audit(file: PathBuf) {
    let source = match fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {:?}: {}", file, e);
            std::process::exit(1);
        }
    };

    match metalogos::audit_program(&source) {
        Ok(result) => {
            println!("{}", result.format());
            std::process::exit(result.exit_code());
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `mlog check <file> [--root <main.mlog>]` — parse + semantic analysis, no execution.
/// When --root is provided, imports in `file` are resolved against the
/// declarations in the root file (same as `mlog serve` would do).
fn cmd_check(file: PathBuf, root: Option<PathBuf>) {
    let source = match fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {:?}: {}", file, e);
            std::process::exit(1);
        }
    };

    match metalogos::check_program_with_root(&source, root.as_deref()) {
        Ok(result) => {
            println!("{}", result.format());
            if !result.is_ok() {
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `mlog serve <file>` — parse + start HTTP server
#[cfg(feature = "server")]
fn cmd_serve(file: PathBuf) {
    let source = match fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {:?}: {}", file, e);
            std::process::exit(1);
        }
    };

    // Use tokio runtime for async server
    // ADR-0096: block_in_place on single-core serializes requests.
    // Default to max(4, available_parallelism) workers.
    let workers = std::env::var("METALOGOS_WORKERS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or_else(|| {
            std::cmp::max(
                4,
                std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(4),
            )
        });

    if let Ok(raw) = std::env::var("METALOGOS_WORKERS") {
        if raw.parse::<usize>().is_err() {
            eprintln!(
                "warning: METALOGOS_WORKERS='{}' is not a valid number, using default ({})",
                raw, workers
            );
        }
    }

    eprintln!("[mlog serve] tokio runtime: {} worker thread(s)", workers);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .enable_all()
        .build()
        .unwrap_or_else(|e| {
            eprintln!("error: cannot create tokio runtime: {}", e);
            std::process::exit(1);
        });

    rt.block_on(async {
        match metalogos::server::run_server(&source).await {
            Ok(()) => {}
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
    });
}

/// `mlog repl` — interactive session with persistent state.
///
/// Uses rustyline for line editing with history when stdin is a tty.
/// When stdin is piped (non-tty), reads lines from stdin silently
/// (used by integration tests feeding 3 lines via pipe).
fn cmd_repl_stdio() {
    let mut interp = metalogos::interpreter::Interpreter::new();

    if stdin_is_piped() {
        // Non-interactive (piped stdin): read lines and eval each
        cmd_repl_piped(&mut interp);
    } else {
        // Interactive tty: use rustyline with history and readline
        cmd_repl_interactive(&mut interp);
    }
}

/// Piped stdin mode: read lines, evaluate, print results. Used by tests.
fn cmd_repl_piped(interp: &mut metalogos::interpreter::Interpreter) {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(input) => {
                let trimmed = input.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if trimmed == "exit" || trimmed == "quit" {
                    break;
                }
                match metalogos::feed_line(interp, trimmed) {
                    Ok(Some(output)) => println!("{}", output),
                    Ok(None) => {} // Declaration processed, no output
                    Err(e) => eprintln!("error: {}", e),
                }
            }
            Err(_) => break, // EOF
        }
    }
}

/// Interactive tty mode: rustyline with history, prompt, readline.
fn cmd_repl_interactive(interp: &mut metalogos::interpreter::Interpreter) {
    println!("METALOGOS REPL v{}", env!("CARGO_PKG_VERSION"));
    println!("Type declarations. Use 'exit' or 'quit' to leave.");
    println!();

    let mut rl = DefaultEditor::new().unwrap_or_else(|e| {
        eprintln!("error: cannot init readline: {}", e);
        std::process::exit(1);
    });

    // Load history from ~/.mlog_history
    let history_path = dirs_home().join(".mlog_history");
    if history_path.exists() {
        let _ = rl.load_history(&history_path);
    }

    loop {
        let line = match rl.readline("mlog> ") {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(e) => {
                eprintln!("error: {}", e);
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "exit" || trimmed == "quit" {
            break;
        }

        // Add to history
        let _ = rl.add_history_entry(trimmed);

        match metalogos::feed_line(interp, trimmed) {
            Ok(Some(output)) => println!("=> {}", output),
            Ok(None) => {} // Declaration processed silently
            Err(e) => eprintln!("error: {}", e),
        }
    }

    // Save history
    let _ = rl.save_history(&history_path);
    println!("Bye.");
}

/// Get the user's home directory.
fn dirs_home() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("USERPROFILE")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
        })
}

/// Check if stdin is a terminal (tty).
/// Returns true if stdin is piped/redirected (non-interactive).
fn stdin_is_piped() -> bool {
    if std::env::var("METALOGOS_FORCE_PIPE")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false)
    {
        return true;
    }
    // `IsTerminal` is stable since Rust 1.70 and works on all platforms,
    // so no `#[cfg(unix)]` branching or `unsafe libc::isatty` is needed.
    !std::io::stdin().is_terminal()
}
