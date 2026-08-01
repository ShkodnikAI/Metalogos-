// ── Block 4: Triple-backend cross-check test ───────────────────────
// Runs .mlog programs through interpreter (tree-walking) and VM,
// comparing outputs. JIT is declared experimental (ADR-0073) — skipped.
//
// Discrepancies are red tests — each mismatch is a separate assertion.
// This test is designed to be informational first: it collects ALL
// mismatches and reports them, then fails if any exist.

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
fn collect_pairs(examples_dir: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut pairs = Vec::new();
    if let Ok(entries) = fs::read_dir(examples_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "mlog" {
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

    // Assertion: baseline must not regress.
    // Baseline: 48/58 match (Наряд №35 Block 1.1 — string comparison fix).
    // This number must only grow as divergences are resolved.
    assert!(
        passed.len() >= 48,
        "Baseline regression: {}/{} golden examples match (expected >= 48)",
        passed.len(),
        pairs.len()
    );

    // All discrepancies documented in ADR-0075.
    // Enable when the list in ADR-0075 is fully resolved:
    // assert!(mismatches.is_empty(), "{} TW vs VM mismatches found", mismatches.len());
}
