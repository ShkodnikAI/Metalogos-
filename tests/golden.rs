// ── Golden tests: run examples/, compare stdout with .expected ─────────
// Error tests: .mlog/.error pairs — program must fail with expected message.

use std::fs;
use std::path::{Path, PathBuf};

/// Find all .mlog files in `examples/` that have .expected (NOT .error) counterparts.
fn collect_pairs(examples_dir: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut pairs = Vec::new();
    if let Ok(entries) = fs::read_dir(examples_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "mlog" {
                    // Skip files that have .error counterparts (they are error tests)
                    let error_path = path.with_extension("error");
                    if error_path.exists() {
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

/// Find all .mlog files that have .error counterparts.
fn collect_error_pairs(examples_dir: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut pairs = Vec::new();
    if let Ok(entries) = fs::read_dir(examples_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "mlog" {
                    let error_path = path.with_extension("error");
                    if error_path.exists() {
                        pairs.push((path, error_path));
                    }
                }
            }
        }
    }
    pairs.sort_by(|a, b| a.0.file_name().cmp(&b.0.file_name()));
    pairs
}

/// Execute a .mlog source. Returns Ok(output) on success, Err(message) on failure.
fn run_mlog(source: &str) -> Result<String, String> {
    let output = metalogos::run_program(source)?;
    Ok(output.unwrap_or_default())
}

#[test]
fn all_golden_tests_pass() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");

    let pairs = collect_pairs(&examples_dir);

    assert!(!pairs.is_empty(), "no .mlog/.expected pairs found in examples/");

    for (mlog_path, expected_path) in &pairs {
        let source = fs::read_to_string(mlog_path)
            .unwrap_or_else(|e| panic!("cannot read {:?}: {}", mlog_path, e));
        let expected = fs::read_to_string(expected_path)
            .unwrap_or_else(|e| panic!("cannot read {:?}: {}", expected_path, e));

        let actual = run_mlog(&source).expect(&format!("execution failed for {:?}", mlog_path));

        // Trim trailing whitespace for comparison
        let expected_trimmed = expected.trim_end();
        let actual_trimmed = actual.trim_end();

        assert_eq!(
            actual_trimmed, expected_trimmed,
            "golden test mismatch for {:?}:\n  expected: {:?}\n  actual:   {:?}",
            mlog_path, expected_trimmed, actual_trimmed
        );
    }
}

#[test]
fn all_error_tests_pass() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");

    let pairs = collect_error_pairs(&examples_dir);

    for (mlog_path, error_path) in &pairs {
        let source = fs::read_to_string(mlog_path)
            .unwrap_or_else(|e| panic!("cannot read {:?}: {}", mlog_path, e));
        let expected_error = fs::read_to_string(error_path)
            .unwrap_or_else(|e| panic!("cannot read {:?}: {}", error_path, e))
            .trim().to_string();

        let result = run_mlog(&source);
        match result {
            Ok(output) => {
                panic!(
                    "error test FAILED for {:?}: expected error containing {:?}, but program succeeded with: {:?}",
                    mlog_path, expected_error, output
                );
            }
            Err(actual_error) => {
                assert!(
                    actual_error.contains(&expected_error),
                    "error test FAILED for {:?}:\n  expected error containing: {:?}\n  actual error: {:?}",
                    mlog_path, expected_error, actual_error
                );
            }
        }
    }
}
