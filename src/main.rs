// ── METALOGOS CLI ─────────────────────────────────────────────────────
// `mlog run <file.mlog>`    — execute a .mlog program
// `mlog repl`               — interactive session with persistent state
// `mlog check <file.mlog>`  — semantic analysis without execution
// `mlog serve <file.mlog>`  — start HTTP server from mlogserver block

use clap::{CommandFactory, FromArgMatches, Parser};
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
    /// Run test blocks: execute `test "..." { }` declarations (Наряд №120);
    /// with --docs: doc-tests on markdown files instead (Наряд №287)
    Test {
        /// Path to .mlog source file (required without --docs)
        file: Option<PathBuf>,
        /// Only run tests whose name contains this substring
        #[arg(long)]
        filter: Option<String>,
        /// Doc-tests mode (Наряд №287): execute ```mlog blocks from docs
        #[arg(long)]
        docs: bool,
        /// Doc-files/globs to scan (default: REFERENCE.md, README.md, docs/**/*.md)
        #[arg(long, value_name = "GLOB", requires = "docs")]
        docs_glob: Vec<String>,
        /// Execution backend for doc blocks (vm-compile skips are not failures)
        #[arg(long, default_value = "tw")]
        backend: String,
    },
    /// Static security analysis without execution (ADR-0057)
    Audit {
        /// Path to .mlog source file
        file: PathBuf,
    },
    /// Start MCP server (Наряд №297, ADR-0132) — expose .mlog tool constructs as MCP tools via stdio
    McpServe {
        /// Path to .mlog source file
        file: PathBuf,
        /// Tool names to expose (fail-closed: required, no default exposure)
        #[arg(long, value_delimiter = ',')]
        allowlist: Vec<String>,
    },
    /// Action Ledger v1 (Naryad #393, ADR-0167) — external verification
    /// and archival of an exported ledger file. Pure file reading: no
    /// Metalogos runtime, no interpreter, no database.
    Ledger {
        #[command(subcommand)]
        cmd: LedgerCmd,
    },
}

#[derive(clap::Subcommand, Debug)]
enum LedgerCmd {
    /// Verify an exported ledger JSONL file: seq continuity, prev-hash
    /// chain, record hashes, key ids, Ed25519 signatures, signer
    /// continuity across rotations, snapshot anchoring. Exit 0 = valid,
    /// 1 = INVALID (the loud reason is printed), 2 = usage error.
    Verify {
        /// Path to the exported ledger file (JSONL)
        file: PathBuf,
        /// External head anchor: fail unless the last record's hash equals this
        #[arg(long)]
        expect_head: Option<String>,
        /// External signer anchor: fail unless the chain's key equals this
        #[arg(long)]
        expect_key: Option<String>,
    },
    /// Archive: truncate the chain at a snapshot record (inclusive); the
    /// output starts at the anchor and is verified before it is written.
    Archive {
        /// Path to the exported ledger file (JSONL)
        file: PathBuf,
        /// Path of the archived output file
        out: PathBuf,
        /// Snapshot record seq to anchor at (the snapshot's hash is printed
        /// by `ledger_snapshot()`)
        #[arg(long)]
        at: u64,
    },
}

fn main() {
    let version_long: &'static str = Box::leak(
        format!(
            "mlog {}\ngrammar rev: {}",
            env!("CARGO_PKG_VERSION"),
            metalogos::GRAMMAR_REV
        )
        .into_boxed_str(),
    );
    let cli = Cli::from_arg_matches(&Cli::command().long_version(version_long).get_matches())
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        });

    match cli.command {
        Commands::Run { file } => cmd_run(file),
        Commands::Repl => cmd_repl_stdio(),
        Commands::Check { file, root } => cmd_check(file, root),
        #[cfg(feature = "server")]
        Commands::Serve { file } => cmd_serve(file),
        Commands::Compile { file } => cmd_compile(file),
        Commands::Eval { file } => cmd_eval(file),
        Commands::Test {
            file,
            filter,
            docs,
            docs_glob,
            backend,
        } => {
            if docs {
                cmd_doc_tests(docs_glob, backend);
            } else {
                match file {
                    Some(f) => cmd_test(f, filter),
                    None => {
                        eprintln!("error: mlog test requires <file> (or --docs for doc-tests)");
                        std::process::exit(1);
                    }
                }
            }
        }
        Commands::Resume { file, flow, from } => cmd_resume(file, &flow, &from),
        Commands::Audit { file } => cmd_audit(file),
        Commands::McpServe { file, allowlist } => cmd_mcp_serve(file, &allowlist),
        Commands::Ledger { cmd } => cmd_ledger(cmd),
    }
}

/// `mlog ledger verify|archive` — the external Action Ledger verifier
/// (Naryad #393, ADR-0167 §3.6). No runtime: reads the file, checks the
/// chain, prints a loud verdict.
fn cmd_ledger(cmd: LedgerCmd) {
    match cmd {
        LedgerCmd::Verify {
            file,
            expect_head,
            expect_key,
        } => match metalogos::ledger::verify_file(
            &file,
            expect_head.as_deref(),
            expect_key.as_deref(),
        ) {
            Ok(report) => {
                println!(
                    "VALID: {} records, head {} ({} distinct key(s), anchored start: {}) — schema v{}",
                    report.records,
                    report.head_hash,
                    report.distinct_keys,
                    report.anchored_start,
                    report.schema_version,
                );
            }
            Err(e) => {
                eprintln!("INVALID: {}", e);
                std::process::exit(1);
            }
        },
        LedgerCmd::Archive { file, out, at } => {
            match metalogos::ledger::archive_file(&file, &out, at) {
                Ok(report) => {
                    println!(
                        "ARCHIVED: {} records anchored at snapshot seq {} (head {}) → {}",
                        report.records,
                        at,
                        report.head_hash,
                        out.display(),
                    );
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            }
        }
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

    // Наряд №197: run the interpreter in a thread with a large stack so that
    // deeply-recursive Metalogos patterns (e.g. self-host/parser.mlog parsing
    // itself) do not overflow the default 8MB main-thread stack. The interpreter's
    // pattern-call mechanism is recursive on the Rust side: each Metalogos
    // pattern invocation pushes frames for eval_statements → invoke_pattern_with_hooks
    // → eval_statements → ... For parser.mlog's nested if-else expressions,
    // this can recurse 100+ levels deep. 256MB is a safe upper bound that
    // still fits within typical container memory limits.
    let source_for_thread = source.clone();
    let handle = std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(move || metalogos::run_program(&source_for_thread))
        .expect("failed to spawn interpreter thread");

    match handle.join() {
        Ok(Ok(output)) => {
            if let Some(result) = output {
                println!("{}", result);
            }
        }
        Ok(Err(e)) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!("error: interpreter thread panicked");
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

/// `mlog test --docs [GLOB...]` — doc-tests on markdown docs (Наряд №287).
fn cmd_doc_tests(globs: Vec<String>, backend: String) {
    let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let files = metalogos::doc_tests::resolve_doc_files(&root, &globs);
    if files.is_empty() {
        eprintln!("doc-tests: no markdown files matched");
        std::process::exit(1);
    }
    let backend = match backend.as_str() {
        "tw" => metalogos::doc_tests::DocBackend::Tw,
        "vm" => metalogos::doc_tests::DocBackend::Vm,
        other => {
            eprintln!("error: unknown --backend '{other}' (allowed: tw, vm)");
            std::process::exit(1);
        }
    };
    match metalogos::doc_tests::run_doc_tests(&files, backend, &root) {
        Ok(report) => {
            for f in &report.failures {
                eprintln!("FAIL {f}");
            }
            eprintln!("{}", report.summary());
            if !report.failures.is_empty() {
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

/// `mlog test <file> [--filter=name]` — run test blocks (Наряд №120)
fn cmd_test(file: PathBuf, filter: Option<String>) {
    let source = match fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {:?}: {}", file, e);
            std::process::exit(1);
        }
    };

    let base_dir = file
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf();
    match metalogos::test_program_with_dir(&source, base_dir) {
        Ok(results) => {
            let filtered: Vec<_> = match &filter {
                Some(f) => results.into_iter().filter(|r| r.name.contains(f)).collect(),
                None => results,
            };
            let mut passed = 0usize;
            let mut failed = 0usize;
            for result in &filtered {
                println!("{}", result.format_line());
                if result.passed {
                    passed += 1;
                } else {
                    failed += 1;
                }
            }
            eprintln!("\n{}/{} tests passed", passed, passed + failed);
            if failed > 0 {
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

/// `mlog mcp-serve <file> --allowlist tool1,tool2` — start MCP server over stdio.
/// Fail-closed: --allowlist is required (no tools exposed by default).
fn cmd_mcp_serve(file: PathBuf, allowlist: &[String]) {
    let source = match fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {:?}: {}", file, e);
            std::process::exit(1);
        }
    };
    let declarations = match metalogos::parser::parse(&source) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: parse error: {}", e);
            std::process::exit(1);
        }
    };
    if let Err(e) = metalogos::mcp_server::run_mcp_server(&declarations, allowlist) {
        eprintln!("error: {}", e);
        std::process::exit(1);
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

    // --- Loud warnings for dangerous opt-out env vars ---
    let mut danger_flags: Vec<&str> = Vec::new();
    if std::env::var("METALOGOS_ALLOW_EXEC").unwrap_or_default() == "1" {
        danger_flags.push("METALOGOS_ALLOW_EXEC=1");
    }
    if std::env::var("METALOGOS_SERVE_ALLOW_EXEC").unwrap_or_default() == "1" {
        danger_flags.push("METALOGOS_SERVE_ALLOW_EXEC=1");
    }
    if std::env::var("METALOGOS_HTTP_ALLOW_PRIVATE").unwrap_or_default() == "1" {
        danger_flags.push("METALOGOS_HTTP_ALLOW_PRIVATE=1");
    }
    // Наряд №259: env-эскейп-хэтчи serve-контекста ослабляют дефолтный
    // deny — тоже громкие danger-флаги.
    if std::env::var("METALOGOS_SERVE_ALLOW_ENV").unwrap_or_default() == "1" {
        danger_flags.push("METALOGOS_SERVE_ALLOW_ENV=1");
    }
    if !std::env::var("METALOGOS_ENV_ALLOWLIST")
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        danger_flags.push("METALOGOS_ENV_ALLOWLIST (route env allowlist set)");
    }
    if !danger_flags.is_empty() {
        eprintln!();
        eprintln!("  WARNING: security protections are DISABLED by:");
        for flag in &danger_flags {
            eprintln!("    - {}", flag);
        }
        eprintln!("  Use only in trusted development environments.");
        eprintln!();
    }

    // Наряд №253 (Вариант А): состояние exec-гейта тел роутов — громко, при старте serve.
    // Тела роутов смотрят ТОЛЬКО на METALOGOS_SERVE_ALLOW_EXEC (замена, не AND).
    let route_exec_enabled = std::env::var("METALOGOS_SERVE_ALLOW_EXEC").unwrap_or_default() == "1";
    eprintln!(
        "[serve] route exec: {}",
        if route_exec_enabled {
            "ENABLED (METALOGOS_SERVE_ALLOW_EXEC=1)"
        } else {
            "denied (set METALOGOS_SERVE_ALLOW_EXEC=1 to allow exec() in route bodies)"
        }
    );

    // Наряд №259: состояние env-гейта тел роутов — громко, при старте serve
    // (лекало route exec выше). Тела роутов смотрят на METALOGOS_SERVE_ALLOW_ENV=1
    // (разрешить всё) ИЛИ на METALOGOS_ENV_ALLOWLIST (точечные имена);
    // без них env() в роутах отказан с ENV_NOT_PERMITTED.
    let route_env_all = std::env::var("METALOGOS_SERVE_ALLOW_ENV").unwrap_or_default() == "1";
    let route_env_allowlist = std::env::var("METALOGOS_ENV_ALLOWLIST").unwrap_or_default();
    let route_env_state = if route_env_all {
        "ENABLED — all variables (METALOGOS_SERVE_ALLOW_ENV=1)".to_string()
    } else if route_env_allowlist.trim().is_empty() {
        "denied (set METALOGOS_SERVE_ALLOW_ENV=1 to allow all env() reads in route bodies, or METALOGOS_ENV_ALLOWLIST=\"NAME1,NAME2\" for specific variables)"
            .to_string()
    } else {
        format!("allowlist: {}", route_env_allowlist)
    };
    eprintln!("[serve] route env: {}", route_env_state);

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
