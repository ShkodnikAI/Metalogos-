// ── METALOGOS — library root ──────────────────────────────────────────
// Exposes: parser, interpreter, LLM client, semantic analysis, run_program for binary + tests.
//
// Наряд №29 §6.4: deny clippy::unwrap_used / clippy::expect_used in non-test code.
// All production unwrap/expect calls have been eliminated (Наряд №29 Blocks 3.1–3.4).
// Grammar invariant .expect() calls in parser.rs are allowed via module-level #![allow].

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

/// Grammar/ABI revision counter.
/// Increment when making incompatible changes to the grammar
/// (grammar.pest) or bytecode format that would cause .mlog files
/// or .mbc bytecode compiled under a different revision to behave
/// incorrectly. This is a visible diagnostic marker, not a full
/// compatibility contract.
pub const GRAMMAR_REV: u32 = 1;

pub mod ast;
use crate::audit::{audit_category_a, Severity};
pub mod audit;
pub mod audit_ops;
pub mod builtins;
pub mod secret_label;
// №544 (gh#882) step 2: the SQL lane's label semantics — the leaf module
// the audit's check_sql_dynamic consumes (the Labeled migration).
pub mod sql_label;
// №544 (gh#882) step 3: the HTML lane's label semantics — the leaf module
// the audit's check_html_injection consumes (the Labeled migration).
pub mod html_label;
// Naryad #475 (issue #723): the filesystem FACADE — every program-
// influenced file operation enters here (the №455 gate + the hard
// write-deny + the serve data-dir containment); the clippy.toml
// disallowed-methods ratchet keeps the raw std::fs methods out.
pub mod fs_gate;
// Наряд №335 (spec §7.2 v2): consent ledger — subject/scope/TTL records
// for every grant and revocation (process-local SQLite; export = egress).
pub mod consent;
// Naryad #390 (ADR-0155): Grant algebra — capability ledger + scope math.
pub mod grants;
// Naryad #387 (ADR-0149 D1/D6): LikenessToken — the opaque likeness
// consent credential (challenge/verify ritual + registry).
pub mod likeness;
// Naryad #392: DenyEvent — the typed deny-reason vocabulary + event
// contract shared by the static gate, the TW interpreter and the VM.
pub mod deny;
pub mod distill_hub;
// Naryad #393 (ADR-0167): Action Ledger v1 — signed append-only journal
// of actions (prev-hash chain + Ed25519 per-record signatures; the
// in-toto/PROV export profile is ADR-0157). The external verifier
// (`mlog ledger verify`) reads only the exported file.
pub mod ledger;
// Наряд №316 (issue #403): SSOT-классификация builtins — роль × метка ×
// обратимость. Статическая карта + тесты покрытия 100% (устав §11 Шаг 3).
pub mod backends;
pub mod backends_weights;
pub mod builtins_classification;
pub mod bytecode;
pub mod compiler;
pub mod db_ops;
pub mod doc_tests;
pub mod embeddings;
pub mod error;
pub mod interpreter;
pub mod labels;
pub mod llm;
pub mod mcp_policy;
pub mod mcp_server;
pub mod media;
pub mod media_ops;
pub mod memory_graph;
pub mod memory_ops;
pub mod memory_store;
pub mod recipe_ops;
pub mod runtime_ops;
// Наряд №350: the typed Memory<K> layer — label-typed containers over
// the process-global registry; at-rest AES-GCM under per-subject keys,
// consent-gated private opens, audited sink reads/exports.
pub mod memory_typed;
pub mod nn;
pub mod parser;
// Наряд №325 (issue #419, ADR-0161): compatibility profiles —
// `profile legacy { egress: permissive_with_audit }` switches the
// №325 SINK_CLEARANCE gate into advisory mode (audit events, not errors).
pub mod profile;
// №466 (gh#687) group 4 (reflex): the shared live module — the seven reflex
// builtins' name literals leave both backends; the bodies were already
// shared in src/builtins/reflex.rs since №179b/№180/№187/№193.
pub mod reflex_ops;
// Наряд №348 (ADR-0172): the real session model — process-global
// session registry, wake/interrupt queues, duty-profile runtime state;
// every transition is an Action-Ledger record (surfaces №393/№415).
pub mod duplex;
pub mod embodied;
pub mod session;
// №466 (gh#687) group 3 (sessions): the shared live module — the five conv
// builtins and the consent name pair leave both backends.
pub mod session_ops;
// Наряд №286: shared JSON-Schema validation (ADR-0133) — один валидатор
// для call_llm_schema и json_validate (дифференциальный контракт «ни одного
// нового правила»).
pub mod schema;
// Наряд №440 (P1, feature/forecast): the forecasting domain —
// SeriesHandle/ForecastHandle over the `timeseries` registry class, the
// degradation ladder (timesfm-2.5 -> statsforecast -> seasonal_naive),
// the LabelJoin taint transfer and the gated forecast export (the
// typed FORECAST_TAINTED refusal + the forecast.* ledger family).
pub mod forecast;
pub mod semantic;
pub mod semantic_types;
#[cfg(feature = "server")]
pub mod server;
pub mod util;
pub mod video;
pub mod vision;
pub mod vm;
pub mod vm_pool;
pub mod voice;

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

    // Наряд №98: enforce Category A security invariants before execution.
    // SQL_DYNAMIC, SECRET_LEAK, HTML_INJECTION are now compile-time errors.
    // We call audit_category_a directly (not check_program) because the
    // interpreter resolves imports at runtime — check_program would
    // false-positive on "undefined function" for imported symbols.
    let cat_a = audit_category_a(&declarations, "");
    let cat_a_errors: Vec<String> = cat_a
        .iter()
        .filter_map(|f| match f.severity {
            Severity::Error | Severity::Warning => Some(format!("[{}] {}", f.check_id, f.message)),
            Severity::Info => None,
        })
        .collect();
    if !cat_a_errors.is_empty() {
        return Err(format!(
            "Category A security invariant violated:\n{}",
            cat_a_errors.join("\n")
        ));
    }

    // Наряд №181 (ADR-0117 §2-3): enforce distill_to semantic checks at
    // run_program time too (not just `mlog check`).
    //
    // Наряд №523 (audit 30.09 N-1, release-block): EVERY semantic::check_program
    // error now blocks the run path. The №181-era substring filters
    // (`contains("distill_to")`, `contains("[DENY_")`) are gone — a
    // message-substring filter is a liar-class mechanism: with only two
    // classes blocking, `if is_admn(user)` on an undefined `is_admn`
    // evaluated to the truthy string "[ERROR: unknown function ...]" and
    // a security check became its own bypass. Exemptions classify ONLY by
    // the structured kind in `semantic::is_exempt_from_blocking` (the ONE
    // explicit place), never by substring. Imports are resolved
    // STATICALLY first (same file-lookup rule as the runtime loader), so
    // symbols merged from imported modules are known to the pass — the
    // historical false-positive class that once justified NOT blocking
    // (see the №98 note above) is gone.
    {
        let module_decls = semantic::resolve_imports_statically(&declarations, &base_dir)
            .map_err(|e| format!("Compilation error (Naryad #523): {}", e))?;
        let mut merged_decls = module_decls;
        merged_decls.extend(declarations.clone());
        let sem_result = semantic::check_program(&merged_decls);
        let blocking: Vec<&semantic::SpannedError> = sem_result
            .errors
            .iter()
            .filter(|err| !semantic::is_exempt_from_blocking(err.kind))
            .collect();
        if !blocking.is_empty() {
            // №479: the refusal carries the FIRST blocking finding's stable
            // code at position 0 — machine consumers read the class, not
            // the prose (mirrors the bytecode compiler's coded errors).
            let code = blocking.iter().find_map(|err| err.kind.stable_code());
            let stamp = code.map(|c| format!("[{}] ", c)).unwrap_or_default();
            let lines: Vec<String> = blocking
                .iter()
                .map(|err| semantic::format_blocking_line(err))
                .collect();
            return Err(format!(
                "{}Compilation error (Naryad #523): semantic findings block execution:\n{}",
                stamp,
                lines.join("\n")
            ));
        }
    }

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
                result.push('\n');
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
            // №475: the check/root-source loader — the CLI boundary (argv
            // paths, author-controlled), outside the program-runtime
            // sandbox domain.
            #[allow(clippy::disallowed_methods)]
            // №475: the check/root-source loader — the CLI boundary (argv
            // paths, author-controlled), outside the program-runtime
            // sandbox domain.
            #[allow(clippy::disallowed_methods)]
            // №475: the check/root-source loader — the CLI boundary (argv
            // paths, author-controlled), outside the program-runtime
            // sandbox domain.
            #[allow(clippy::disallowed_methods)]
            // №475: the check/root-source loader — the CLI boundary (argv
            // paths, author-controlled), outside the program-runtime
            // sandbox domain.
            #[allow(clippy::disallowed_methods)]
            // №475: the check/root-source loader — the CLI boundary (argv
            // paths, author-controlled), outside the program-runtime
            // sandbox domain.
            #[allow(clippy::disallowed_methods)]
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
                result.push('\n');
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

/// Compile a .mlog source to bytecode Program.
/// Note: imports are not resolved during compilation. Use `mlog serve` for import support.
pub fn compile_program(source: &str) -> Result<bytecode::Program, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;

    // Наряд №98: enforce Category A security invariants before compilation.
    // Same as run_program_with_dir: call audit_category_a directly,
    // not check_program — the compiler handles its own semantic errors.
    let cat_a = audit_category_a(&declarations, "");
    let cat_a_errors: Vec<String> = cat_a
        .iter()
        .filter_map(|f| match f.severity {
            Severity::Error | Severity::Warning => Some(format!("[{}] {}", f.check_id, f.message)),
            Severity::Info => None,
        })
        .collect();
    if !cat_a_errors.is_empty() {
        return Err(format!(
            "Category A security invariant violated:\n{}",
            cat_a_errors.join("\n")
        ));
    }

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

/// Parse a .mlog program, execute declarations, then run all test blocks (Наряд №120).
/// Each test block runs in isolation; assertion errors are caught per-test.
pub fn test_program(source: &str) -> Result<Vec<interpreter::TestResult>, String> {
    test_program_with_dir(source, std::path::PathBuf::from("."))
}

/// Parse with explicit base directory, execute declarations, then run test blocks.
pub fn test_program_with_dir(
    source: &str,
    base_dir: std::path::PathBuf,
) -> Result<Vec<interpreter::TestResult>, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut interp = interpreter::Interpreter::new();
    interp.set_base_dir(base_dir);
    interp.run(declarations)?;
    Ok(interp.run_test_blocks())
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
                result.push('\n');
                result.push_str(&flow_output);
                Ok(Some(result))
            }
            None => Ok(Some(mutate_log.join("\n"))),
        }
    }
}
