// ── Self-hosted lexer integration test: Phase 4.4 ────────────────
// Tests that the Metalogos lexer (written in Metalogos) correctly tokenizes
// a .mlog source file by piping it through stdin.

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

/// Find the mlog binary relative to the test workspace.
fn mlog_bin() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let target_dir = PathBuf::from(&manifest_dir).join("target").join("debug");
    target_dir.join("mlog").to_string_lossy().into_owned()
}

#[test]
#[ignore = "TODO: self-hosted lexer produces no output — needs investigation"]
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

    let input_content = fs::read_to_string(&input_path)
        .unwrap_or_else(|e| panic!("cannot read {:?}: {}", input_path, e));
    let expected = fs::read_to_string(&expected_path)
        .unwrap_or_else(|e| panic!("cannot read {:?}: {}", expected_path, e));

    // Run: mlog run self-host/lexer.mlog < input.mlog
    let mut output = Command::new(mlog_bin())
        .arg("run")
        .arg(&lexer_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn mlog process");

    // Write input to stdin
    if let Some(stdin) = output.stdin.as_mut() {
        stdin
            .write_all(input_content.as_bytes())
            .expect("failed to write to stdin");
    }

    let output_result = output.wait_with_output().expect("failed to wait for mlog");

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
