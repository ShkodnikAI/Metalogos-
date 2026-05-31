// ── Golden tests: run examples/, compare stdout with .expected ─────────
// Each pair (examples/X.mlog, examples/X.expected) is a contract test.

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
