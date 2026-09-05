// ── Block 4: TW vs VM cross-check test ─────────────────────────────
// Runs .mlog programs through interpreter (tree-walking) and VM,
// comparing outputs. JIT is experimental (ADR-0073) — skipped.
// VM is experimental for full-language coverage (ADR-0105): programs
// that need `match` / block if-else are excluded (e.g. p_match_switch).
//
// Discrepancies are red tests — each mismatch is a separate assertion.
// Collects ALL mismatches, then fails if any exist.

use std::fs;
use std::path::{Path, PathBuf};

/// Execute via tree-walking interpreter.
fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.to_path_buf())
}

/// Execute via bytecode VM.
fn run_vm(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

/// Trim trailing whitespace from an Option<String>.
fn trim_opt(s: &Option<String>) -> String {
    s.as_deref()
        .map(|v| v.trim_end().to_string())
        .unwrap_or_default()
}

/// Find all .mlog files with .expected files.
/// Skips negative-test contracts (e.g. p50_unknown_fn) that are designed
/// to produce errors — crosscheck only tests valid programs.
fn collect_pairs(examples_dir: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut pairs = Vec::new();
    if let Ok(entries) = fs::read_dir(examples_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "mlog" {
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    // Skip negative-test contracts (designed to produce errors)
                    if name.contains("unknown_fn") || name.contains("wrong_") {
                        continue;
                    }
                    // Наряд №109 / ADR-0105: p_match_switch exercises `match`,
                    // which the VM cannot compile. TW has a golden .expected;
                    // full TW↔VM parity for this file is out of scope until
                    // a deliberate decision to implement Match in the VM.
                    if name == "p_match_switch.mlog" {
                        continue;
                    }
                    // Наряд №118 / ADR-0105: p118_collection_utils exercises
                    // unique/chunk/sort builtins whose results flow through
                    // string concatenation (+). The VM's eval_binop rejects
                    // heterogeneous operand types (e.g. List + String),
                    // whereas the TW interpreter auto-coerces. VM parity
                    // for these builtins requires a deliberate VM eval_binop
                    // relaxation — tracked separately.
                    if name == "p118_collection_utils.mlog" {
                        continue;
                    }
                    // Наряд №177: reflex_math uses random_seed/random (TW-only —
                    // VM has no PRNG state). Also uses Bool→String formatting
                    // (to_string(true) → "true" in TW, "1" in VM).
                    if name == "reflex_math.mlog" {
                        continue;
                    }
                    // Наряд №179b: reflex_train_predict uses reflex_train/
                    // reflex_predict builtins whose handlers need access to
                    // the ReflexRegistry (lives on the Interpreter struct,
                    // not the VM). VM Reflex support is tracked in a future
                    // naryad (ADR-0114). The TW-only path is exercised by
                    // the golden test suite.
                    if name == "reflex_train_predict.mlog" {
                        continue;
                    }
                    // Наряд №180: reflex_persist.mlog uses reflex_save/
                    // reflex_load (same VM limitation as reflex_train/predict).
                    // Also writes to a real SQLite file under
                    // target/test_artifacts/ which the VM cannot reach.
                    if name == "reflex_persist.mlog" {
                        continue;
                    }

                    let expected = path.with_extension("expected");
                    if expected.exists() {
                        pairs.push((path, expected));
                    }
                }
            }
        }
    }
    pairs.sort_by(|a, b| a.0.file_name().cmp(&b.0.file_name()));
    pairs
}

/// Cross-check: TW vs VM for all golden examples.
/// Collects mismatches, then reports them all at once.
#[test]
fn crosscheck_tw_vs_vm_all_golden() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");
    let base_dir = examples_dir.parent().unwrap_or(Path::new("."));

    let pairs = collect_pairs(&examples_dir);
    assert!(
        !pairs.is_empty(),
        "no .mlog/.expected pairs found in examples/"
    );

    let mut passed: Vec<String> = Vec::new();
    let mut mismatches: Vec<(String, String, String)> = Vec::new(); // (name, tw, vm)
    let mut tw_errors: Vec<(String, String)> = Vec::new(); // (name, error)
    let mut vm_errors: Vec<(String, String)> = Vec::new(); // (name, error)

    for (mlog_path, _expected_path) in &pairs {
        let name = mlog_path
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or("?")
            .to_string();
        let source = fs::read_to_string(mlog_path)
            .unwrap_or_else(|e| panic!("cannot read {:?}: {}", mlog_path, e));

        let tw_result = run_tw(&source, base_dir);
        let vm_result = run_vm(&source, base_dir);

        match (&tw_result, &vm_result) {
            (Ok(tw), Ok(vm)) => {
                let tw_trimmed = trim_opt(tw);
                let vm_trimmed = trim_opt(vm);
                if tw_trimmed == vm_trimmed {
                    passed.push(name);
                } else {
                    mismatches.push((name, tw_trimmed, vm_trimmed));
                }
            }
            (Ok(_), Err(e)) => {
                vm_errors.push((name, e.clone()));
            }
            (Err(e), Ok(_)) => {
                tw_errors.push((name, e.clone()));
            }
            (Err(e1), Err(e2)) => {
                // Both error — check if same error
                if e1 != e2 {
                    mismatches.push((name, format!("TW err: {}", e1), format!("VM err: {}", e2)));
                } else {
                    // Same error in both — acceptable
                    passed.push(name);
                }
            }
        }
    }

    // Report results
    eprintln!(
        "\n═══ Cross-check TW vs VM: {}/{} passed ═══",
        passed.len(),
        pairs.len()
    );

    if !mismatches.is_empty() {
        eprintln!("\n── Mismatches ({}): ──", mismatches.len());
        for (name, tw, vm) in &mismatches {
            eprintln!("  ✗ {}: TW={:?} VM={:?}", name, tw, vm);
        }
    }

    if !vm_errors.is_empty() {
        eprintln!("\n── VM errors ({}): ──", vm_errors.len());
        for (name, err) in &vm_errors {
            eprintln!("  ✗ {}: {}", name, err);
        }
    }

    if !tw_errors.is_empty() {
        eprintln!("\n── TW errors ({}): ──", tw_errors.len());
        for (name, err) in &tw_errors {
            eprintln!("  ✗ {}: {}", name, err);
        }
    }

    // Assertion: all 58 golden examples must match between TW and VM.
    // All discrepancies documented in ADR-0075 have been resolved (Наряд №36).
    assert!(
        mismatches.is_empty(),
        "{} TW vs VM mismatches found",
        mismatches.len()
    );
    assert!(vm_errors.is_empty(), "{} VM errors found", vm_errors.len());
    assert!(tw_errors.is_empty(), "{} TW errors found", tw_errors.len());
}
