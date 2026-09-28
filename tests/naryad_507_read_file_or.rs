// ── Naryad #507 (P2, hardening): read_file_or — the explicit-silence twin of read_file ──
//
// Contract (issue #790, the audit 28.09 §3.7 — the `env` / `read_file`
// asymmetry the external audit tripped over):
// 1. THE SYMMETRY: `env` got its `env_or` in №481; `read_file` gets its
//    `read_file_or(path, default)` here. The `_or` suffix carries the
//    silent-default semantics IN THE NAME (the №481 naming rule); the
//    fallback firing is announced on the audit stderr (the №326 posture
//    — the PATH is named, the default VALUE never).
// 2. THE LOUD BRANCHES STAY LOUD: sandbox violations (absolute paths,
//    `..`, symlink escapes) and the №455 sensitive-name deny-list refuse
//    LOUDLY in `read_file_or` too — the explicit silence never bypasses
//    them (they are programmer errors, not environmental failures).
// 3. THE BASE CONTRACT IS UNCHANGED: `read_file` of a missing file keeps
//    the №254 empty string, pinned here side by side with the `_or`
//    twin.

// Naryad #475's fs_gate ratchet targets PRODUCTION I/O paths; this test
// exercises the REAL filesystem for its fixtures by design — the scoped
// allow mirrors the naryad_481/465 test posture.
#![allow(clippy::disallowed_methods)]

fn program(body: &str) -> String {
    format!(
        "pattern Read(_x: String) -> String {{\n  {}\n}}\nflow Main {{\n  input: String = \"a\" -> Read -> output\n}}\n",
        body
    )
}

/// A temp sandbox as the process cwd (the same posture the io.rs unit
/// tests use) — restored even on panic.
struct SandboxDir(&'static str);
impl SandboxDir {
    fn enter(name: &'static str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "mlog_n507_{}_{}",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();
        PREV.with(|p| *p.borrow_mut() = Some(prev));
        SandboxDir(name)
    }
}
impl Drop for SandboxDir {
    fn drop(&mut self) {
        PREV.with(|p| {
            if let Some(prev) = p.borrow_mut().take() {
                std::env::set_current_dir(prev).unwrap();
            }
        });
        let dir = std::env::temp_dir().join(format!(
            "mlog_n507_{}_{}",
            self.0,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
thread_local! {
    static PREV: std::cell::RefCell<Option<std::path::PathBuf>> = const { std::cell::RefCell::new(None) };
}

// ── 1. The explicit default on a MISSING file (both lanes agree) ──

#[test]
#[serial_test::serial]
fn n507_read_or_missing_file_yields_the_explicit_default() {
    let _sb = SandboxDir::enter("missing");
    let src = program(r#"  return read_file_or("нет_такого_507.txt", "fallback-507")"#);
    let out = metalogos::run_program(&src).expect("read_file_or falls back, not refuses");
    assert_eq!(
        out.as_deref(),
        Some("fallback-507"),
        "the explicit default must reach the flow output"
    );
}

#[test]
#[serial_test::serial]
fn n507_read_file_missing_contract_is_unchanged() {
    let _sb = SandboxDir::enter("basecontract");
    let src = program(r#"  return read_file("нет_такого_507.txt")"#);
    let out = metalogos::run_program(&src).expect("the №254 soft contract holds");
    assert_eq!(
        out.as_deref(),
        Some(""),
        "read_file of a missing file stays the №254 empty string"
    );
}

// ── 2. An EXISTING file: the content, not the default ──

#[test]
#[serial_test::serial]
fn n507_read_or_existing_file_yields_the_content() {
    let _sb = SandboxDir::enter("existing");
    std::fs::write("config_507.txt", "the real content").unwrap();
    let src = program(r#"  return read_file_or("config_507.txt", "fallback-507")"#);
    let out = metalogos::run_program(&src).expect("read_file_or reads the file");
    assert_eq!(out.as_deref(), Some("the real content"));
}

// ── 3. The loud branches stay loud in the `_or` twin ──

#[test]
#[serial_test::serial]
fn n507_read_or_traversal_stays_loud() {
    let _sb = SandboxDir::enter("traversal");
    let src = program(r#"  return read_file_or("../escape_507.txt", "fallback-507")"#);
    let err = metalogos::run_program(&src)
        .expect_err("the `..` traversal refuses even in the explicit-silence twin");
    assert!(
        err.contains("[SANDBOX_VIOLATION]"),
        "the stable code is required, got: {}",
        err
    );
}

#[test]
#[serial_test::serial]
fn n507_read_or_deny_list_stays_loud() {
    let _sb = SandboxDir::enter("denylist");
    // The deny-list fires on the RAW name BEFORE existence matters —
    // the .env attempt is a signal by itself (№455).
    let src = program(r#"  return read_file_or(".env", "fallback-507")"#);
    let err = metalogos::run_program(&src)
        .expect_err("the sensitive-name deny-list refuses even in the `_or` twin");
    assert!(
        err.contains("sensitive-path deny-list"),
        "the refusal must name the deny-list, got: {}",
        err
    );
}
