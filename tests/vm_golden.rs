// ── VM golden tests: run examples/ through the bytecode VM ────────────
// Phase 4.2: strict dual-mode comparison for all golden examples.

use std::fs;
use std::path::{Path, PathBuf};

/// Execute a .mlog source via the bytecode VM.
fn run_vm(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    let declarations = metalogos::parser::parse(source)
        .map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

/// Execute a .mlog source via tree-walking interpreter.
fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.to_path_buf())
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

/// Helper: trim trailing whitespace from an Option<String>.
fn trim_opt(s: &Option<String>) -> String {
    s.as_deref().map(|v| v.trim_end()).unwrap_or("").to_string()
}

#[test]
fn p4_vm_hello_matches_tw() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");

    let source = fs::read_to_string(examples_dir.join("p4_vm_hello.mlog"))
        .expect("cannot read p4_vm_hello.mlog");
    let base_dir = examples_dir.parent().unwrap_or(Path::new("."));

    let vm_result = run_vm(&source, base_dir).expect("VM execution failed");
    let tw_result = run_tw(&source, base_dir).expect("TW execution failed");

    assert_eq!(vm_result, tw_result, "VM output differs from tree-walking");
}

/// Phase 4.2 strict test: all golden examples must produce identical
/// output when run via tree-walking interpreter vs bytecode VM.
#[test]
fn all_vm_examples_match_tree_walking() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");

    let pairs = collect_pairs(&examples_dir);
    assert!(!pairs.is_empty(), "no .mlog/.expected pairs found in examples/");

    let base_dir = examples_dir.parent().unwrap_or(Path::new("."));
    let mut passed = 0;

    for (mlog_path, expected_path) in &pairs {
        let source = fs::read_to_string(mlog_path)
            .unwrap_or_else(|e| panic!("cannot read {:?}: {}", mlog_path, e));
        let _expected = fs::read_to_string(expected_path)
            .unwrap_or_else(|e| panic!("cannot read {:?}: {}", expected_path, e));

        let vm_result = run_vm(&source, base_dir)
            .unwrap_or_else(|e| panic!("VM execution failed for {:?}: {}", mlog_path.file_name(), e));
        let tw_result = run_tw(&source, base_dir)
            .unwrap_or_else(|e| panic!("TW execution failed for {:?}: {}", mlog_path.file_name(), e));

        let vm_trimmed = trim_opt(&vm_result);
        let tw_trimmed = trim_opt(&tw_result);

        assert_eq!(
            vm_trimmed, tw_trimmed,
            "VM vs TW mismatch for {:?}:\n  TW: {:?}\n  VM: {:?}",
            mlog_path.file_name(), tw_trimmed, vm_trimmed
        );
        passed += 1;
    }

    eprintln!("\nPhase 4.2 strict dual-mode test: {}/{} passed", passed, pairs.len());
    assert_eq!(passed, pairs.len(), "not all examples passed dual-mode comparison");
}

/// Legacy test (kept for CI compatibility): all VM outputs match .expected files.
#[test]
fn all_vm_golden_tests_pass() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");

    let pairs = collect_pairs(&examples_dir);
    assert!(!pairs.is_empty(), "no .mlog/.expected pairs found in examples/");

    let base_dir = examples_dir.parent().unwrap_or(Path::new("."));
    let mut passed = 0;

    for (mlog_path, expected_path) in &pairs {
        let source = fs::read_to_string(mlog_path)
            .unwrap_or_else(|e| panic!("cannot read {:?}: {}", mlog_path, e));
        let expected = fs::read_to_string(expected_path)
            .unwrap_or_else(|e| panic!("cannot read {:?}: {}", expected_path, e));

        let vm_result = run_vm(&source, base_dir).expect("VM execution failed");
        let vm_trimmed = trim_opt(&vm_result);
        let expected_trimmed = expected.trim_end();

        assert_eq!(
            vm_trimmed, expected_trimmed,
            "VM vs .expected mismatch for {:?}:\n  expected: {:?}\n  VM:       {:?}",
            mlog_path.file_name(), expected_trimmed, vm_trimmed
        );
        passed += 1;
    }

    eprintln!("\nVM golden test results: {}/{} passed", passed, pairs.len());
    assert_eq!(passed, pairs.len());
}
