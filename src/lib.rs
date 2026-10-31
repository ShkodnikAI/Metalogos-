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

pub mod approval; // №860 (ADR-0205, wave 75 — W-D): the typed approval mandate — the pure carrier of the human consent/oversight decision over the grant contour
pub mod ast;
pub mod capability; // №844 (ADR-0203, wave 72 — W-B): the typed capability carrier — the read-only view over the grant ledger (the single consumption contract)
use crate::audit::{audit_category_a, Severity};
pub mod audit;
pub mod audit_ops;
pub mod builtins;
pub mod limits;
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
// №854 (ADR-0204, wave 74 — W-C): the Portable Refusal Algebra — the
// refusal as a VALUE of a finite lattice (join/meet over the frozen
// №392 vocabulary); dependency-free by contract (ADR-0204 §3) so the
// №855 WASM facade compiles this very source for wasm32.
pub mod distill_hub;
pub mod refusal_algebra;
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
pub mod mcp_session; // №793: the MCP stdio session layer (the gateway upstream pool form)
pub mod model_passport; // №838 (ADR-0202, wave 71 — W-A): the model passport — the License-Typed Model Layer (the 8D-1 subphase)
/// №685 (gh#1252, В44): the shared template tokenizer + the raw-insert
/// scan — the UNGATED home (render() is ungated, the raw-insert gates
/// must hold on every configuration; the gated template_render engine
/// imports the same source — no second parser, no drift).
pub(crate) mod template_scan;
// №567 (gh#931; the crate-split stage 2): the transport modules
// (`mcp_policy`, `mcp_server`, `server`) MOVED to the metalogos-server
// crate (metalogos-server/src/lib.rs) together with the `mlog` bin and
// the 38 consumer test files. The move is a Cargo-graph necessity — the
// package cycle metalogos -> metalogos-server -> metalogos is
// forbidden — and it is NOT a silent API removal (the №524 rule): the
// paths re-bind mechanically (`metalogos::server::*` →
// `metalogos_server::server::*`), the CLI surface is unchanged, and
// the direction is one-way (this crate cannot and does not depend on
// the transport).
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
// №558 (issue #919; the audit 02.10 M-8): the ONE module-search rule —
// the runtime loader, the compiler's resolve_import and the semantic
// static resolution all call resolve_module_file (the SSOT).
pub mod module_path;
pub mod semantic;
pub mod semantic_types;
pub mod util;
pub mod video;
pub mod vision;
pub mod vm;
pub mod vm_pool;
pub mod voice;

/// Parse and execute a .mlog program. Returns the flow output (if any),
/// with mutate status messages prepended if present.
/// [TW-ORACLE] №808: the TW entry — the REFERENCE oracle runner (ADR-0197):
/// the user executor is `run_program_on_vm`; this lane rides `--reference`
/// and the CI comparisons only.
pub fn run_program(source: &str) -> Result<Option<String>, String> {
    run_program_with_dir(source, std::path::PathBuf::from("."))
}

/// Parse and execute a .mlog program with an explicit base directory for module resolution.
pub fn run_program_with_dir(
    source: &str,
    base_dir: std::path::PathBuf,
) -> Result<Option<String>, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;

    enforce_run_gates(&declarations, &base_dir, source)?;

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

/// №800 (gh#1547, V3): the SHARED run gates — the exact refusal set the
/// TW run path applies before execution (Category A + the №523 semantic
/// gate over the statically-import-merged declarations). The VM run lane
/// (`run_program_on_vm`) feeds the SAME body: the user path has ONE gate
/// set (ADR-0197) — switching the executor underneath cannot change what
/// the gates refuse (the №535 n535_run_missing_import_blocks pin caught
/// the VM lane refusing a missing import with a raw io error and no №523
/// stamp — a gate divergence, not a wording nit).
fn enforce_run_gates(
    declarations: &[ast::Declaration],
    base_dir: &std::path::Path,
    source: &str,
) -> Result<(), String> {
    // Наряд №98: enforce Category A security invariants before execution.
    // SQL_DYNAMIC, SECRET_LEAK, HTML_INJECTION are now compile-time errors.
    // We call audit_category_a directly (not check_program) because the
    // interpreter resolves imports at runtime — check_program would
    // false-positive on "undefined function" for imported symbols.
    let cat_a = audit_category_a(declarations, "");
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
        let module_decls = semantic::resolve_imports_statically(declarations, base_dir)
            .map_err(|e| format!("Compilation error (Naryad #523): {}", e))?;
        let mut merged_decls = module_decls;
        merged_decls.extend(declarations.to_vec());
        let sem_result = semantic::check_program(&merged_decls);
        // №802: the allow-comment suppressions are honored on the run
        // path too (the checked surface and the run gate must agree —
        // a suppressed-at-check program must not refuse at run).
        let mut sem_errors = sem_result.errors;
        sem_errors = semantic::filter_suppressed_unchecked(sem_errors, source);
        let blocking: Vec<&semantic::SpannedError> = sem_errors
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

    Ok(())
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
    let mut result = semantic::check_program(&declarations);
    // №802: the explicit allow-comment suppressions are honored where
    // the span lines map to THIS source.
    result.errors = semantic::filter_suppressed_unchecked(result.errors, source);
    Ok(result)
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

    let mut result = semantic::check_program(&all_decls);
    // №802: the allow-comment suppressions map to the CHECKED file's
    // lines (the merged root decls keep their own spans).
    result.errors = semantic::filter_suppressed_unchecked(result.errors, source);
    Ok(result)
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

/// Compile a .mlog source to bytecode Program.
/// №800: the stale "imports are not resolved" caveat is gone — the
/// compiler's Phase 1 resolves imports (resolve_import); the run lane's
/// import gate is the №523 static resolution in `enforce_run_gates` (the
/// compile path keeps its narrower №673/№685/№695 gate set — the recorded
/// №673-era scope).
pub fn compile_program(source: &str) -> Result<bytecode::Program, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;

    enforce_compile_gates(&declarations)?;

    let mut compiler = compiler::Compiler::new();
    compiler.compile(declarations)
}

/// №800 (gh#1547, V3): the shared compile gates — the SAME refusal set
/// `compile_program` applies, factored out so the REPL session chunks
/// (compiled against a session base) run through the IDENTICAL Category A
/// audit and the VM compile gates (№673 break/continue, №685 raw-insert,
/// №695 safe-pipe). A gate added here fires on both the program path and
/// the session path — one gate body, both entry points.
fn enforce_compile_gates(declarations: &[ast::Declaration]) -> Result<(), String> {
    // Наряд №98: enforce Category A security invariants before compilation.
    // №839 (ADR-0202 §2.3, wave 71 — W-A): LICENSE_CLASS_NOT_ALLOWED —
    // the license-class × profile permit rule. The PRECISE selection-site
    // refusal leads the compile error (the specific-over-general
    // precedence: when this rule fires, its dedicated code names the
    // class and the permit; the audit lane keeps reporting weights-id
    // literal mentions on every path it always did). A statically-visible
    // backend selection (a literal `backend_select` ladder rung, a
    // `vision` model field) whose license class the program's profile
    // does not permit refuses compilation BEFORE run, on BOTH backends
    // (the shared walker — the same rules `check_program` applies; the
    // permit predicate is the ONE decision point shared with the №333
    // audit gate). The audit lane still reports weights-id literal
    // mentions — the lanes are independent postures, not duplicates.
    {
        let violations = semantic::license_class_violations(declarations);
        if !violations.is_empty() {
            let lines: Vec<String> = violations.iter().map(|v| v.message.clone()).collect();
            return Err(format!(
                "[{}] Compilation error (Naryad #839): license-class findings block compilation:\n{}",
                semantic::CODE_LICENSE_CLASS_NOT_ALLOWED,
                lines.join("\n")
            ));
        }
    }

    // Same as run_program_with_dir: call audit_category_a directly,
    // not check_program — the compiler handles its own semantic errors.
    let cat_a = audit_category_a(declarations, "");
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

    // №673 (gh#1200, В41): BREAK_OUTSIDE_LOOP — the VM compile gate. The
    // run/serve path blocks on the finding through check_program (№523);
    // the compile→run .mbc path had NO semantic gate, so the «молча
    // неверные» break/continue edges (№658-a value-channel crossing,
    // №658-b outside any loop) compiled and silently misbehaved on the
    // VM. The SAME shared walker the TW gate uses refuses here, BEFORE
    // the bytecode exists — the refusal is backend-independent by
    // construction. Only THIS class is wired (not the whole check_program):
    // widening the compile gate to every semantic finding is a separate
    // conversation; the break/continue class is the audit's P1.
    {
        let violations = semantic::break_outside_loop_violations(declarations);
        if !violations.is_empty() {
            let lines: Vec<String> = violations
                .iter()
                .map(semantic::format_blocking_line)
                .collect();
            return Err(format!(
                "[{}] Compilation error (Naryad #673): break/continue findings block compilation:\n{}",
                semantic::CODE_BREAK_OUTSIDE_LOOP,
                lines.join("\n")
            ));
        }
    }

    // №685 (gh#1252, В44): RAW_INSERT_NOT_HTML — the VM compile gate, the
    // №673 shape: the S-1 fail-closed gate must refuse BEFORE execution on
    // BOTH backends, and the compile→run .mbc path had no semantic gate
    // beyond the №673 class. The SAME shared walker the TW gate uses
    // refuses here, BEFORE the bytecode exists — a raw-insert parameter
    // that is not Html cannot reach the VM.
    {
        let violations = semantic::raw_insert_not_html_violations(declarations);
        if !violations.is_empty() {
            let lines: Vec<String> = violations
                .iter()
                .map(semantic::format_blocking_line)
                .collect();
            return Err(format!(
                "[{}] Compilation error (Naryad #685): raw-insert findings block compilation:\n{}",
                semantic::CODE_RAW_INSERT_NOT_HTML,
                lines.join("\n")
            ));
        }
    }

    // №695 (gh#1274, В46): SAFE_PIPE_NOT_HTML + UNSAFE_PIPE_FORM — the VM
    // compile gate, the №685 shape: the safe stage `{{ param | safe }}`
    // inserts RAW, so its parameter must be declared Html, and every
    // unrecognized pipe form refuses LOUDLY (the N9 refusal shifted
    // left). The SAME shared scan the TW gate uses refuses here, BEFORE
    // the bytecode exists. The prefix is the FIRST finding's stable code
    // (the №523 shape) — the scan carries two kinds, and the prefix names
    // the class that actually fired first.
    {
        let violations = semantic::safe_pipe_violations(declarations);
        if !violations.is_empty() {
            let prefix = violations
                .first()
                .and_then(|v| v.kind.stable_code())
                .unwrap_or("SEMANTIC");
            let lines: Vec<String> = violations
                .iter()
                .map(semantic::format_blocking_line)
                .collect();
            return Err(format!(
                "[{}] Compilation error (Naryad #695): safe-pipe findings block compilation:\n{}",
                prefix,
                lines.join("\n")
            ));
        }
    }

    Ok(())
}

/// №800 (gh#1547, V3): compile a REPL session CHUNK — the same compile
/// gates, then the session compiler (the chunk's code resolves the base
/// names positionally against the live VM's tables). Returns the chunk
/// Program AND the updated session symbol base (the next chunk's seed).
fn compile_session_declarations(
    declarations: Vec<ast::Declaration>,
    base: compiler::SessionBase,
) -> Result<(bytecode::Program, compiler::SessionBase), String> {
    enforce_compile_gates(&declarations)?;
    let mut compiler = compiler::Compiler::with_session(base);
    let program = compiler.compile(declarations)?;
    let snapshot = compiler.session_base_snapshot();
    Ok((program, snapshot))
}

/// №800 (gh#1547, V3): the `mlog run` executor form — parse, the SHARED
/// run gates (the same №523 set the TW run path applies), execute on the
/// VM (the single user-command executor, ADR-0197; TW stays the reference
/// oracle in CI and behind `--reference`).
pub fn run_program_on_vm(source: &str) -> Result<Option<String>, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    enforce_run_gates(&declarations, std::path::Path::new("."), source)?;
    install_vm_llm_config(&declarations);
    let program = compile_program(source)?;
    run_bytecode(program)
}

/// №800 (gh#1547, V3): the `mlog eval` executor form — compile + run the
/// declarations on the VM (the adapt/few-shot state builds ON the VM),
/// then score the eval blocks through the VM harness (`Vm::run_eval_blocks`
/// — the same `call_llm` path a user call takes).
pub fn eval_program_on_vm(source: &str) -> Result<Vec<interpreter::types::EvalResult>, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    // №800: the run-path gate set — the default eval lane is a USER path
    // (the same posture `run` keeps); the `--reference` oracle lane keeps
    // the TW eval's historical ungated form.
    enforce_run_gates(&declarations, std::path::Path::new("."), source)?;
    install_vm_llm_config(&declarations);
    let eval_blocks: Vec<ast::EvalDecl> = declarations
        .iter()
        .filter_map(|d| match d {
            ast::Declaration::Eval(e) => Some(e.clone()),
            _ => None,
        })
        .collect();
    let program = compile_program(source)?;
    let mut vm = vm::Vm::new();
    vm.run(program)?;
    vm.run_eval_blocks(&eval_blocks)
}

/// №800 (gh#1547, V3): the VM REPL session — a persistent Vm plus the
/// session symbol base. Each `feed_line` compiles the line's declarations
/// as a session CHUNK (the same compile gates, the base names resolved
/// against the live tables) and executes it ON the same VM
/// (`Vm::run_chunk`): globals keep their slots, patterns/learnables/rules
/// join the live tables, memory persists — the incremental state between
/// inputs the TW REPL gets from its long-lived Interpreter.
pub struct VmRepl {
    vm: vm::Vm,
    base: compiler::SessionBase,
}

impl Default for VmRepl {
    fn default() -> Self {
        Self::new()
    }
}

impl VmRepl {
    pub fn new() -> Self {
        VmRepl {
            vm: vm::Vm::new(),
            base: compiler::SessionBase::default(),
        }
    }

    /// Parse one REPL line, compile it as a session chunk, execute it on
    /// the persistent VM. The output contract matches `feed_line` (the
    /// mutate log prepends the flow output) — the REPL surface is
    /// backend-agnostic, the executor underneath is the VM.
    pub fn feed_line(&mut self, line: &str) -> Result<Option<String>, String> {
        let declarations = parser::parse(line).map_err(|e| format!("parse error: {}", e))?;
        if declarations.is_empty() {
            return Ok(None);
        }
        // №819: the VM REPL honors a chunk's `llm {}` the same way the
        // run/eval lanes do (see install_vm_llm_config).
        install_vm_llm_config(&declarations);
        let chunk_base = vm::ChunkBase {
            global_count: self.base.global_names.len(),
            pattern_count: self.vm.pattern_table_len(),
            learnable_count: self.vm.learnable_table_len(),
        };
        let (program, snapshot) = compile_session_declarations(declarations, self.base.clone())?;
        let out = self.vm.run_chunk(&program, &chunk_base)?;
        self.base = snapshot;
        Ok(out)
    }
}

/// №819 (#1302): the VM user lanes honor the declared `llm {}` — install
/// the SmartRouter + the block limits exactly the way the TW declaration
/// pass does (src/interpreter/execution.rs): last-wins scope, no clear
/// after the run (the same persistence semantics both REPL lanes rely
/// on). Before this, the VM lane silently ignored `llm {}` and every
/// `call_llm` fell back to the legacy env-var backend — the declared
/// config was dead on the VM (the №521 conversation_config class; the
/// pre-existing gap the №819 RED test caught, now closed together with
/// the per-provider model resolution).
fn install_vm_llm_config(declarations: &[ast::Declaration]) {
    for d in declarations {
        if let ast::Declaration::LlmConfig(config) = d {
            llm::set_global_smart_router(llm::SmartRouter::from_config(config));
            // №757: the block-level limits ride the same declaration —
            // the legacy fallback sites honor them too.
            llm::set_block_llm_limits(config.max_tokens, config.temperature);
        }
    }
}

/// Run a pre-compiled bytecode Program on the VM.
pub fn run_bytecode(program: bytecode::Program) -> Result<Option<String>, String> {
    let mut vm = vm::Vm::new();
    vm.run(program)
}

/// Parse a .mlog program, execute declarations, then run all eval blocks (ADR-0050).
/// Returns a list of EvalResult structs with accuracy, confusion matrix, and failure details.
/// [TW-ORACLE] №808: the TW entry — the REFERENCE oracle runner (ADR-0197):
/// the user executor is `eval_program_on_vm`; this lane rides `--reference`
/// and the CI comparisons only.
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

#[cfg(test)]
mod n673_break_outside_loop_gate {
    use super::{compile_program, run_program_with_dir};

    // №673 (gh#1200, В41): the BREAK_OUTSIDE_LOOP semantic gate — the
    // «молча неверные» №658-a/№658-b shapes refuse with the stable code on
    // BOTH production paths (the run/serve gate via check_program, the
    // VM compile gate via compile_program), and the legal №658 surface
    // (break nested through if/match INSIDE a loop) keeps working.

    const OUTSIDE_LOOP: &str = r#"
pattern import_batch(x: String) -> String {
  if x == "bad" { break }
  return "published"
}
flow Main { input: String = "bad" -> import_batch -> output }
"#;

    const VALUE_ARM_BREAK: &str = r#"
pattern p(x: String) -> String {
  let mut i = 0.0
  while i < 3.0 {
    let kind = match x {
      "go" then {
        break
        "matched"
      }
      else { "other" }
    }
    i = i + 1.0
  }
  return "done"
}
flow Main { input: String = "go" -> p -> output }
"#;

    const LEGAL_NESTED_BREAK: &str = r#"
pattern p(x: String) -> String {
  let mut acc = ""
  let mut i = 0.0
  while i < 5.0 {
    if x == "stop" { break }
    acc = acc + "x;"
    i = i + 1.0
  }
  return acc
}
flow Main { input: String = "go" -> p -> output }
"#;

    #[test]
    fn compile_program_refuses_break_outside_loop_with_the_stable_code() {
        let err =
            compile_program(OUTSIDE_LOOP).expect_err("the VM compile gate must refuse №658-b");
        assert!(
            err.starts_with("[BREAK_OUTSIDE_LOOP]"),
            "the stable code must be stamped first, got: {}",
            err
        );
    }

    #[test]
    fn compile_program_refuses_value_channel_break_with_the_stable_code() {
        let err =
            compile_program(VALUE_ARM_BREAK).expect_err("the VM compile gate must refuse №658-a");
        assert!(
            err.starts_with("[BREAK_OUTSIDE_LOOP]"),
            "the stable code must be stamped first, got: {}",
            err
        );
    }

    #[test]
    fn compile_program_keeps_the_legal_nested_break_surface() {
        compile_program(LEGAL_NESTED_BREAK)
            .expect("the №658 legal surface (break nested through if inside a loop) must compile");
    }

    #[test]
    fn run_path_refuses_break_outside_loop_with_the_stable_code() {
        // [TW-ORACLE] №808: the in-lib test — the oracle lane under test.
        // The TW run/serve path: the №523 gate stamps the FIRST blocking
        // finding's stable code — the №673 check runs early in
        // check_program, so the stamp is ours.
        let err = run_program_with_dir(OUTSIDE_LOOP, std::path::PathBuf::from("."))
            .expect_err("the run gate must refuse №658-b");
        assert!(
            err.starts_with("[BREAK_OUTSIDE_LOOP]"),
            "the stable code must be stamped first, got: {}",
            err
        );
    }
}
