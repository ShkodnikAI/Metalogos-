// ── Golden tests: run examples/, compare stdout with .expected ─────────
// Each pair (examples/X.mlog, examples/X.expected) is a contract test.
// Error contracts use (examples/X.mlog, examples/X.error) — verified by
// all_error_tests_pass which checks the error message contains the expected text.
//
// Наряд №31, Блок 2: runner collects ALL failures before panicking,
// so broken examples don't mask subsequent tests.
//
// Наряд №49, Блок 2: p7_* tests require environment variables or a live HTTP
// server. They are excluded from the main golden suite and tracked separately
// in p7_contract_visibility so they remain visible without blocking CI.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Find all .mlog files in `examples/` and pair them with .expected files.
fn collect_pairs(examples_dir: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut pairs = Vec::new();
    if let Ok(entries) = fs::read_dir(examples_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "mlog" {
                    // p7_* tests require env vars or a live server (Наряд №49 БЛОК 2)
                    // p88_html_render_success requires a real Chromium binary —
                    // not available in CI; tracked in p88_browser_contract_visibility.
                    let stem = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if stem.starts_with("p7_") || stem == "p88_html_render_success" {
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
        let report = failures
            .iter()
            .map(|f| format!("  - {}", f))
            .collect::<Vec<_>>()
            .join("\n");
        panic!(
            "{} golden test(s) FAILED ({} passed):\n{}",
            failures.len(),
            pairs.len() - failures.len(),
            report
        );
    }
}

/// p7_* tests require either environment variables or a live HTTP server.
/// They are excluded from the main golden suite but tracked here for visibility.
/// Наряд №49: "Hidden red is worse than visible red."
#[test]
fn p7_contract_visibility() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");

    let mut p7_pairs: Vec<_> = fs::read_dir(&examples_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let p = e.path();
            p.extension().map(|ext| ext == "mlog").unwrap_or(false)
                && p.file_stem()
                    .map(|s| s.to_string_lossy().starts_with("p7_"))
                    .unwrap_or(false)
        })
        .filter_map(|e| {
            let expected = e.path().with_extension("expected");
            if expected.exists() {
                Some((e.path(), expected))
            } else {
                None
            }
        })
        .collect();

    p7_pairs.sort_by(|a, b| a.0.file_name().cmp(&b.0.file_name()));

    assert!(
        !p7_pairs.is_empty(),
        "p7_* .expected files must exist (Наряд №49 БЛОК 2)"
    );

    const CASE_TIMEOUT: Duration = Duration::from_secs(5);
    let mut passed = 0usize;
    let mut failed = 0usize;

    for (mlog_path, expected_path) in &p7_pairs {
        let name = mlog_path.file_name().unwrap().to_string_lossy().to_string();
        let source = fs::read_to_string(mlog_path).unwrap();
        let expected_content = fs::read_to_string(expected_path).unwrap();

        let (tx, rx) = mpsc::channel();
        let src = source.clone();
        let handle = thread::spawn(move || {
            let result = metalogos::run_program(&src);
            let _ = tx.send(result);
        });

        match rx.recv_timeout(CASE_TIMEOUT) {
            Ok(Ok(declarations)) => {
                let actual = declarations.unwrap_or_default();
                if actual.trim_end() == expected_content.trim_end() {
                    passed += 1;
                } else {
                    failed += 1;
                    eprintln!(
                        "  FAIL: {} — expected {:?}, got {:?}",
                        name,
                        expected_content.trim_end(),
                        actual.trim_end()
                    );
                }
                let _ = handle.join();
            }
            Ok(Err(err)) => {
                failed += 1;
                eprintln!("  FAIL: {} — runtime error: {}", name, err);
                let _ = handle.join();
            }
            Err(_) => {
                // Timeout — server/env not available.
                // Thread may still be blocked on I/O; let it detach.
                failed += 1;
                eprintln!("  FAIL: {} — timed out (no server/env)", name);
                // Do NOT join — thread is likely blocked on HTTP connect.
            }
        }
    }

    eprintln!(
        "p7 contract visibility: {}/{} passed, {}/{} failed (require env vars or live server)",
        passed,
        p7_pairs.len(),
        failed,
        p7_pairs.len()
    );

    // Intentionally does NOT assert all passed — these tests require
    // environment variables or a live HTTP server. Visibility is the goal.
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

/// p88_html_render_success requires METALOGOS_BROWSER_BIN and a real
/// Chromium/Chrome binary. Not available in CI — tracked here for
/// visibility, never blocking. Наряд №88: "Hidden red is worse than
/// visible red" (same principle as Наряд №49).
#[test]
#[ignore = "requires METALOGOS_BROWSER_BIN and a real Chromium binary — not available in CI"]
fn p88_browser_contract_visibility() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");

    let mlog_path = examples_dir.join("p88_html_render_success.mlog");
    let expected_path = examples_dir.join("p88_html_render_success.expected");

    assert!(
        mlog_path.exists(),
        "p88_html_render_success.mlog must exist"
    );
    assert!(
        expected_path.exists(),
        "p88_html_render_success.expected must exist"
    );

    let source = fs::read_to_string(&mlog_path).unwrap();
    let expected = fs::read_to_string(&expected_path).unwrap();

    let result = metalogos::run_program(&source)
        .map(|v| v.unwrap_or_default())
        .unwrap_or_default();

    assert_eq!(
        result.trim_end(),
        expected.trim_end(),
        "p88_html_render_success contract mismatch"
    );
}
