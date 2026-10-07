// Naryad #475 (issue #723): the fs_gate ratchet (clippy disallowed-methods)
// targets PRODUCTION I/O paths. This test file exercises the REAL filesystem
// for fixtures and assertions by design — the allow is scoped to this file.
#![allow(clippy::disallowed_methods)]

// ── Self-hosted lexer integration test: Phase 4.4 ────────────────
// Tests that the Metalogos lexer (written in Metalogos) correctly tokenizes
// a .mlog source file by piping it through stdin.

use std::fs;
use std::process::{Command, Stdio};

/// Find the mlog binary relative to the test workspace.
fn mlog_bin() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let target_dir = PathBuf::from(&manifest_dir).join("target").join("debug");
    target_dir.join("mlog").to_string_lossy().into_owned()
}

#[test]
// №634 (gh#1102): the ignore is LIFTED — the diagnosis found the FIXTURE
// drifted (not the language): (1) `let` without `mut` against the current
// mutability discipline, (2) no entry-point flow (the stdin piping was
// never a mechanism — `mlog run` prints the FLOW output), (3) the newline
// fell into the quote-class (spurious STRING tokens on empty lines),
// (4) the keyword list missed `input`/`output`. The fixture now follows
// the №197 parser.mlog pattern (env target + read_file) and reproduces
// examples/p4_self_host_lexer.expected byte-for-byte.
fn self_host_lexer_tokenizes_m1_hello() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let project_dir = PathBuf::from(&manifest_dir);

    let lexer_path = project_dir.join("self-host/lexer.mlog");
    let input_path = project_dir.join("examples/m1_hello.mlog");
    let expected_path = project_dir.join("examples/p4_self_host_lexer.expected");

    // Verify files exist
    assert!(
        lexer_path.exists(),
        "lexer.mlog not found at {:?}",
        lexer_path
    );
    assert!(
        input_path.exists(),
        "input file not found at {:?}",
        input_path
    );
    assert!(
        expected_path.exists(),
        "expected file not found at {:?}",
        expected_path
    );

    let expected = fs::read_to_string(&expected_path)
        .unwrap_or_else(|e| panic!("cannot read {:?}: {}", expected_path, e));

    // №634: the №197 parser.mlog invocation pattern — the target file
    // rides the env var (stdin was never wired into `mlog run`); the
    // №455 sensitive-path deny-list needs the explicit allowlist crane
    // for the .mlog read.
    let output_result = Command::new(mlog_bin())
        .arg("run")
        .arg(&lexer_path)
        // №634: the RELATIVE target path — the file-I/O sandbox refuses
        // absolute paths (the same shape the №197 test uses).
        .env("MLOG_LEXER_TARGET", "examples/m1_hello.mlog")
        .env("METALOGOS_SENSITIVE_PATH_ALLOWLIST", "m1_hello.mlog")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to spawn mlog process");

    let stdout = String::from_utf8_lossy(&output_result.stdout);
    let stderr = String::from_utf8_lossy(&output_result.stderr);

    // Process should succeed
    assert!(
        output_result.status.success(),
        "mlog process failed:\nstderr: {}",
        stderr
    );

    // Trim trailing whitespace for comparison
    let actual_trimmed = stdout.trim_end();
    let expected_trimmed = expected.trim_end();

    assert_eq!(
        actual_trimmed, expected_trimmed,
        "self-host lexer output mismatch:\n  expected:\n{}\n  actual:\n{}",
        expected_trimmed, actual_trimmed
    );
}

use std::path::PathBuf;
