//! Наряд №455 (gh#674, P0 security) — the file-ingest gate, process-side
//! contracts and the static layer.
//!
//! Layers under test (the naryad's «слоями, по нарастанию»):
//!   1. DENY-LIST (context-independent): `.env*`, `*.db`, `*.sqlite*`,
//!      `.git/**`, `*.mlog`, `metalogos.toml`, `.mlog/**` — loud
//!      `SANDBOX_SENSITIVE_PATH`, even in `mlog run` (no serve needed),
//!      even when the file does not exist;
//!   3. ESCAPE CRANE + LABELS: `METALOGOS_SENSITIVE_PATH_ALLOWLIST` lets
//!      an explicitly named sensitive file be read, and the static label
//!      engine then marks such content SECRET — a sink refusal
//!      (SINK_CLEARANCE) proves the Private label;
//!   4. TAINT: a path from an untrusted source (query_param) is the
//!      category-A `UNTRUSTED_FILE_PATH` (the file-channel twin of
//!      UNTRUSTED_EXEC_DECISION).
//!
//! Layer 2 (the serve-root containment) is pinned live in
//! `serve_read_file_env_denied.rs` — it needs the real serve context.
//!
//! Verify: cargo test --test naryad_455_file_gate

use serial_test::serial;

// ── Layer 1: the deny-list refuses in the plain process context ────────

#[test]
#[serial]
fn n455_run_context_env_read_refused_loudly() {
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    let err = metalogos::run_program(
        r#"
flow Main { input: String = ".env" -> read_file -> output }
"#,
    )
    .expect_err("reading .env must be a loud refusal, not a soft empty string");
    assert!(
        err.contains("[SANDBOX_SENSITIVE_PATH]"),
        "the refusal must carry the stable №455 code — err: {}",
        err
    );
}

#[test]
#[serial]
fn n455_deny_list_is_loud_even_when_the_file_is_missing() {
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    // The file does not exist — a policy refusal is loud regardless
    // (an attempt to read .env is a signal, the absence is not a defense).
    let err = metalogos::run_program(
        r#"
flow Main { input: String = ".env.definitely-missing" -> read_file -> output }
"#,
    )
    .expect_err("the missing sensitive file must still refuse loudly");
    assert!(err.contains("[SANDBOX_SENSITIVE_PATH]"), "err: {}", err);
}

#[test]
#[serial]
fn n455_deny_list_covers_the_audit_vocabulary() {
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    for path in [
        "app.db",
        "store.sqlite-wal",
        ".git/config",
        ".mlog/entries",
        "metalogos.toml",
        "examples/p102_secret_patterns.mlog",
    ] {
        let src = format!(
            "flow Main {{ input: String = \"{}\" -> read_file -> output }}",
            path
        );
        let err = metalogos::run_program(&src)
            .expect_err(&format!("{} must be refused", path));
        assert!(
            err.contains("[SANDBOX_SENSITIVE_PATH]"),
            "{}: wrong refusal — err: {}",
            path,
            err
        );
    }
}

#[test]
#[serial]
fn n455_ordinary_reads_keep_working_in_process_context() {
    // A non-sensitive relative read stays exactly as before (the №254
    // contract: missing file → soft empty string, existing file → content).
    let out = metalogos::run_program(
        r#"
flow Main { input: String = "n455_definitely_missing_normal.txt" -> read_file -> output }
"#,
    )
    .expect("a normal missing file keeps the soft-failure contract");
    assert_eq!(out.unwrap_or_default(), "", "soft empty string");
}

// ── Layer 3: the escape crane allows, and the label engine vouches ─────

#[test]
#[serial]
fn n455_allowlist_lets_an_explicitly_named_file_through() {
    std::env::set_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST", ".env");
    let result = metalogos::run_program(
        r#"
flow Main { input: String = ".env" -> read_file -> output }
"#,
    );
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    // Whether the file exists or not, the READ ITSELF must not be refused
    // by the deny-list anymore (the soft contract answers for missing).
    assert!(
        result.is_ok(),
        "the allowlist must unblock the deny-list refusal — err: {:?}",
        result.err()
    );
}

#[test]
#[serial]
fn n455_allowlisted_sensitive_content_is_labeled_secret() {
    // The static label engine marks a sensitive-named literal read as
    // SECRET — the respond() sink must refuse it (SINK_CLEARANCE, the
    // category-A evidence of the Private label).
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    let src = r#"
mlogserver {
  port: 18101
  route "/leak" method=GET { return respond("200", read_file(".env")) }
}
"#;
    let declarations = metalogos::parser::parse(src).expect("parse");
    let findings = metalogos::audit::audit_category_a(&declarations, src);
    let ids: Vec<&str> = findings.iter().map(|f| f.check_id).collect();
    // The output sink keeps the legacy corpus class name for a private
    // label (the leak-suite vocabulary) — the point is that the read no
    // longer passes as Internal/UserInput: the sink REFUSES.
    assert!(
        ids.contains(&"SINK_CLEARANCE") || ids.contains(&"PII_EGRESS_OUTPUT"),
        "the Private label must surface as a sink refusal — got: {:?}",
        ids
    );
}

// ── Layer 4: untrusted paths are the category-A decision class ─────────

#[test]
#[serial]
fn n455_untrusted_path_is_category_a_untrusted_file_path() {
    // The untrusted SOURCE must be in the path position directly (a bare
    // pattern parameter starts open — ADR-0154 Appendix A — and is NOT
    // the audit's exfiltration shape; the source call is).
    let src = r#"
pattern Load() -> String {
  let c = read_file(query_param("p"))
  return "loaded"
}
flow Main { input: String = "x" -> Load -> output }
"#;
    let declarations = metalogos::parser::parse(src).expect("parse");
    // The direct category-A surface (what serve startup enforces).
    let findings = metalogos::audit::audit_category_a(&declarations, src);
    let ids: Vec<&str> = findings.iter().map(|f| f.check_id).collect();
    assert!(
        ids.contains(&"UNTRUSTED_FILE_PATH"),
        "an untrusted read_file path must be UNTRUSTED_FILE_PATH — got: {:?}",
        ids
    );
}

#[test]
#[serial]
fn n455_untrusted_path_read_file_tokens_also_gated() {
    let src = r#"
pattern Load() -> String {
  let c = read_file_tokens(query_param("p"))
  return "loaded"
}
flow Main { input: String = "x" -> Load -> output }
"#;
    let declarations = metalogos::parser::parse(src).expect("parse");
    let findings = metalogos::audit::audit_category_a(&declarations, src);
    let ids: Vec<&str> = findings.iter().map(|f| f.check_id).collect();
    assert!(
        ids.contains(&"UNTRUSTED_FILE_PATH"),
        "read_file_tokens is the same file channel — got: {:?}",
        ids
    );
}

#[test]
#[serial]
fn n455_constant_paths_are_not_flagged() {
    // The sanctioned shape: a constant path (data-directory reads) — no
    // UNTRUSTED_FILE_PATH, no false positive on the normal corpus shape.
    let src = r#"
pattern Load() -> String {
  let c = read_file("data/notes.txt")
  return "ok"
}
flow Main { input: String = "x" -> Load -> output }
"#;
    let declarations = metalogos::parser::parse(src).expect("parse");
    let findings = metalogos::audit::audit_category_a(&declarations, src);
    let ids: Vec<&str> = findings.iter().map(|f| f.check_id).collect();
    assert!(
        !ids.contains(&"UNTRUSTED_FILE_PATH"),
        "a constant path must not be flagged — got: {:?}",
        ids
    );
}

// ── The runtime gate under the deny-list: read_file_tokens parity ──────

#[test]
#[serial]
fn n455_read_file_tokens_deny_list_parity() {
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    let err = metalogos::run_program(
        r#"
flow Main { input: String = ".env" -> read_file_tokens -> output }
"#,
    )
    .expect_err("read_file_tokens is the same file channel — must refuse");
    assert!(err.contains("[SANDBOX_SENSITIVE_PATH]"), "err: {}", err);
}

// ── No stubs (§16.0-D) ─────────────────────────────────────────────────

#[test]
fn n455_no_stubs() {
    let markers = [
        concat!("todo", "!"),
        concat!("unimplemented", "!"),
        concat!("SKELE", "TON"),
    ];
    let src = std::fs::read_to_string(file!()).unwrap_or_default();
    for m in markers {
        assert!(!src.contains(m), "stub marker {} in {}", m, file!());
    }
}
