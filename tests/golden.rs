// ── Golden tests: run examples/, compare stdout with .expected ─────────
// Each pair (examples/X.mlog, examples/X.expected) is a contract test.
// Error contracts use (examples/X.mlog, examples/X.error) — verified by
// all_error_tests_pass which checks the error message contains the expected text.
//
// Наряд №31, Блок 2: runner collects ALL failures before panicking,
// so broken examples don't mask subsequent tests.

use std::fs;
use std::path::{Path, PathBuf};

/// Find all .mlog files in `examples/` and pair them with .expected files.
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

/// Execute a .mlog source and return stdout as a string.
fn run_mlog(source: &str) -> Result<String, String> {
    let declarations = metalogos::run_program(source)?;
    Ok(declarations.unwrap_or_default())
}

#[test]
fn all_golden_tests_pass() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");

    let pairs = collect_pairs(&examples_dir);

    assert!(
        !pairs.is_empty(),
        "no .mlog/.expected pairs found in examples/"
    );

    let mut failures: Vec<String> = Vec::new();

    for (mlog_path, expected_path) in &pairs {
        let source = fs::read_to_string(mlog_path)
            .unwrap_or_else(|e| panic!("cannot read {:?}: {}", mlog_path, e));
        let expected = fs::read_to_string(expected_path)
            .unwrap_or_else(|e| panic!("cannot read {:?}: {}", expected_path, e));

        match run_mlog(&source) {
            Err(e) => {
                failures.push(format!("EXECUTION ERROR {:?}: {}", mlog_path, e));
            }
            Ok(actual) => {
                let expected_trimmed = expected.trim_end();
                let actual_trimmed = actual.trim_end();

                if expected_trimmed != actual_trimmed {
                    failures.push(format!(
                        "MISMATCH {:?}:\n  expected: {:?}\n  actual:   {:?}",
                        mlog_path, expected_trimmed, actual_trimmed
                    ));
                }
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "{} golden test(s) FAILED ({} passed):\n{}",
            failures.len(),
            pairs.len() - failures.len(),
            failures.iter().map(|f| format!("  - {}", f)).collect::<Vec<_>>().join("\n")
        );
    }
}

/// Find .mlog files in `examples/` paired with .error files.
/// Covers p30_* and p31_* prefixes — verified semantic contracts.
/// Legacy .error files (err_*, p2_*) are reference-only, not automated.
fn collect_error_pairs(examples_dir: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut pairs = Vec::new();
    if let Ok(entries) = fs::read_dir(examples_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                continue;
            }
            if let Some(ext) = path.extension() {
                if ext == "mlog" {
                    let file_name = path
                        .file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if !file_name.starts_with("p30_") && !file_name.starts_with("p31_") {
                        continue;
                    }
                    let error_file = path.with_extension("error");
                    if error_file.exists() {
                        pairs.push((path, error_file));
                    }
                }
            }
        }
    }
    pairs
}

#[test]
fn all_error_tests_pass() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");

    let pairs = collect_error_pairs(&examples_dir);

    for (mlog_path, error_path) in &pairs {
        let source = fs::read_to_string(mlog_path)
            .unwrap_or_else(|e| panic!("cannot read {:?}: {}", mlog_path, e));
        let expected_err = fs::read_to_string(error_path)
            .unwrap_or_else(|e| panic!("cannot read {:?}: {}", error_path, e))
            .trim_end()
            .to_string();

        let result = metalogos::run_program(&source);
        match result {
            Ok(_) => panic!(
                "error test {:?} was expected to fail but succeeded",
                mlog_path
            ),
            Err(actual_err) => {
                assert!(
                    actual_err.contains(&expected_err),
                    "error test mismatch for {:?}:\n  expected substring: {:?}\n  actual error:       {:?}",
                    mlog_path, expected_err, actual_err
                );
            }
        }
    }
}
