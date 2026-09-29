//! Naryad №501 (issue #784, P1 security) — the audit 28.09 §3.2 (minor) + §3.7.
//! BLOCKING: the deny vocabulary is case-insensitive; the hard write-deny
//! classes (*.mlog, metalogos.toml, .git/**) stay unconditional under ANY
//! spelling, and the allowlist crane cannot unlock them.
//!
//! The audit's point: on a case-insensitive filesystem (macOS by default,
//! Windows) `read_file(".ENV")` IS `read_file(".env")`, but the №455
//! comparisons were byte-exact — the letter case the program spelled
//! chose the policy. The fix (single point, shared matchers): every
//! name/component comparison runs on the lowercased form (io.rs
//! `sensitive_path_match`/`sensitive_name_match`, fs_gate.rs
//! `hard_write_name`/`hard_write_path_hit`/`gate_write_resolved`).
//!
//! Verify: cargo test --test naryad_501_write_deny_case

#![allow(clippy::disallowed_methods)]

use serial_test::serial;

// ── (1) The hard write classes under an upper-case spelling ────────────

/// `APP.MLOG` is `app.mlog` on a case-insensitive FS — the hard deny
/// must refuse it, and the allowlist must NOT unlock it (№475 task 5
/// semantics, now case-independent).
#[test]
#[serial]
fn n501_write_app_mlog_uppercase_refused_even_with_allowlist() {
    std::env::set_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST", "APP.MLOG");
    let source = r#"
pattern W(_x: String) -> String {
  return write_file("APP.MLOG", "malicious rewrite")
}
flow Main { input: String = "x" -> W -> output }
"#;
    let result = metalogos::run_program(source);
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    let err = result.expect_err("writing APP.MLOG must be refused even when allowlisted");
    assert!(
        err.contains("SANDBOX_SENSITIVE_PATH") && err.contains("ALWAYS refused"),
        "the refusal must be the hard image-integrity deny, got: {}",
        err
    );
    assert!(
        !std::path::Path::new("APP.MLOG").exists(),
        "the refusal must happen before any file is created"
    );
}

/// `Metalogos.TOML` is the config image `metalogos.toml` on a
/// case-insensitive FS — hard refuse.
#[test]
#[serial]
fn n501_write_metalogos_toml_mixedcase_refused() {
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    let source = r#"
pattern W(_x: String) -> String {
  return write_file("Metalogos.TOML", "attacker = true")
}
flow Main { input: String = "x" -> W -> output }
"#;
    let result = metalogos::run_program(source);
    let err = result.expect_err("writing Metalogos.TOML must be refused");
    assert!(
        err.contains("SANDBOX_SENSITIVE_PATH") && err.contains("ALWAYS refused"),
        "the refusal must be the hard image-integrity deny, got: {}",
        err
    );
}

/// The `.git/**` class under a `.GIT` spelling: the component match must
/// be case-insensitive on EVERY path component (raw form).
#[test]
#[serial]
fn n501_write_dot_git_uppercase_component_refused() {
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    let source = r#"
pattern W(_x: String) -> String {
  return write_file("build/.GIT/objects/x.bin", "corruption")
}
flow Main { input: String = "x" -> W -> output }
"#;
    let result = metalogos::run_program(source);
    let err = result.expect_err("writing under a .GIT component must be refused");
    assert!(
        err.contains("SANDBOX_SENSITIVE_PATH"),
        "the refusal must carry the stable code, got: {}",
        err
    );
}

// ── (2) The read-side deny-list under upper-case spellings ────────────

/// `.ENV` reads the same secrets as `.env` on a case-insensitive FS —
/// the read must refuse with the stable code and leak nothing.
#[test]
#[serial]
fn n501_read_env_uppercase_refused() {
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    std::fs::write(".ENV", "N501_ENV_MARKER_SECRET=1").expect("write the .ENV fixture");
    let result = metalogos::run_program(
        r#"
flow Main { input: String = ".ENV" -> read_file -> output }
"#,
    );
    let _ = std::fs::remove_file(".ENV");
    let err = result.expect_err("reading .ENV must be refused like .env");
    assert!(
        err.contains("SANDBOX_SENSITIVE_PATH"),
        "the refusal must carry the stable code, got: {}",
        err
    );
    assert!(
        !err.contains("N501_ENV_MARKER_SECRET"),
        "the marker must never leak into the error: {}",
        err
    );
}

/// `App.DB-WAL` — the audit's exact example class: the `.db-wal`
/// vocabulary must match regardless of the case spelling.
#[test]
#[serial]
fn n501_read_app_db_wal_mixedcase_refused() {
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    std::fs::write("App.DB-WAL", "N501_DB_WAL_MARKER=1").expect("write the fixture");
    let result = metalogos::run_program(
        r#"
flow Main { input: String = "App.DB-WAL" -> read_file -> output }
"#,
    );
    let _ = std::fs::remove_file("App.DB-WAL");
    let err = result.expect_err("reading App.DB-WAL must be refused");
    assert!(
        err.contains("SANDBOX_SENSITIVE_PATH"),
        "the refusal must carry the stable code, got: {}",
        err
    );
    assert!(
        !err.contains("N501_DB_WAL_MARKER"),
        "the marker must never leak into the error: {}",
        err
    );
}

/// The write side of the same class: `App.DB-WAL` is not a hard class,
/// so the refusal comes from the deny-list layer — and the allowlist
/// crane does NOT cover it unless explicitly named.
#[test]
#[serial]
fn n501_write_app_db_wal_mixedcase_refused() {
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    let source = r#"
pattern W(_x: String) -> String {
  return write_file("App.DB-WAL", "torn write injection")
}
flow Main { input: String = "x" -> W -> output }
"#;
    let result = metalogos::run_program(source);
    let err = result.expect_err("writing App.DB-WAL must be refused");
    assert!(
        err.contains("SANDBOX_SENSITIVE_PATH"),
        "the refusal must carry the stable code, got: {}",
        err
    );
}

// ── (3) The boundaries: legitimate paths and the exact-name crane ─────

/// A normal, non-sensitive write keeps working (the №475 boundary).
#[test]
#[serial]
fn n501_legitimate_write_still_works() {
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    let source = r#"
pattern W(_x: String) -> String {
  return write_file("n501_notes.txt", "legitimate work output")
}
flow Main { input: String = "x" -> W -> output }
"#;
    let result = metalogos::run_program(source);
    let out = result.expect("a non-sensitive write must keep working");
    let content =
        std::fs::read_to_string("n501_notes.txt").expect("the written file must exist on disk");
    let _ = std::fs::remove_file("n501_notes.txt");
    assert!(
        content.contains("legitimate work output"),
        "the content must round-trip, got: {}",
        content
    );
    let _ = out;
}

/// The exact-name escape crane keeps working for the non-hard classes:
/// `App.DB` named EXACTLY in the allowlist reads through (the №455
/// layer 3 contract, case-insensitive deny does not touch the crane).
#[test]
#[serial]
fn n501_allowlist_crane_still_works_for_exact_db_name() {
    std::fs::write("App.DB", "N501_ALLOWLISTED_CONTENT=1").expect("write the fixture");
    std::env::set_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST", "App.DB");
    let result = metalogos::run_program(
        r#"
flow Main { input: String = "App.DB" -> read_file -> output }
"#,
    );
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    let _ = std::fs::remove_file("App.DB");
    let out = result.expect("the exact allowlist name must unblock the deny-list refusal");
    assert!(
        out.unwrap_or_default().contains("N501_ALLOWLISTED_CONTENT"),
        "the allowlisted content must read through"
    );
}
