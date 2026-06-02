// ── METALOGOS CLI ─────────────────────────────────────────────────────
// `mlog run <file.mlog>`    — execute a .mlog program
// `mlog repl`               — interactive session with persistent state
// `mlog check <file.mlog>`  — semantic analysis without execution
// `mlog serve <file.mlog>`  — start HTTP server from mlogserver block

use clap::Parser;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::fs;
use std::io::{self, BufRead};
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
    /// Run a .mlog program
    Run {
        /// Path to .mlog source file
        file: PathBuf,
    },
    /// Start interactive REPL session with persistent state
    Repl,
    /// Run semantic analysis without execution
    Check {
        /// Path to .mlog source file
        file: PathBuf,
    },
    /// Start HTTP server from mlogserver block (Phase 6)
    Serve {
        /// Path to .mlog source file
        file: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file } => cmd_run(file),
        Commands::Repl => cmd_repl_stdio(),
        Commands::Check { file } => cmd_check(file),
        Commands::Serve { file } => cmd_serve(file),
    }
}

/// `mlog run <file>` — parse + execute
fn cmd_run(file: PathBuf) {
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

/// `mlog check <file>` — parse + semantic analysis, no execution
fn cmd_check(file: PathBuf) {
    let source = match fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {:?}: {}", file, e);
            std::process::exit(1);
        }
    };

    match metalogos::check_program(&source) {
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
fn cmd_serve(file: PathBuf) {
    let source = match fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {:?}: {}", file, e);
            std::process::exit(1);
        }
    };

    // Use tokio runtime for async server
    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| {
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
    if std::env::var("METALOGOS_FORCE_PIPE").map(|v| v == "1" || v == "true").unwrap_or(false) {
        return true;
    }
    #[cfg(unix)]
    {
        // Use raw isatty syscall on Unix
        unsafe { libc::isatty(0) != 1 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}
