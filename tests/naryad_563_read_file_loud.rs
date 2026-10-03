// ── tests/naryad_563_read_file_loud.rs ───────────────────────────────
// №563 (Wave 25 P2; the audit 02.10 "the rest" line; dispatch gh#925):
// the read_file soft-missing debt is 5 → 0 — the №254 empty-string
// contract ENDED at v0.28.0 and a MISSING file refuses LOUDLY with
// `[IO_ERROR]` on BOTH backends (the №514 rule, gh#798: softness lives
// in the `_or` name). The explicit-silence surface is and stays
// `read_file_or(path, default)` (unchanged); existing-file reads are
// byte-identical.
//
// The debt counter (`scripts/ci/debt_counters.py`,
// `read_file_soft_missing`, the only-down lock in
// scripts/ci/debt_baseline.txt) reads 0 — this file keeps the marker
// string out of the tree on purpose (the counter counts .rs lines; the
// flip is proven by the CODE, not by the old marker).
#![allow(clippy::disallowed_methods)]

// The SandboxDir helper below moves the PROCESS cwd (a chdir is not
// thread-scoped): the six tests touch relative paths and must not run
// concurrently with each other or the process-global cwd is yanked out
// from under a mid-test read (the CI race: an existing-file read saw a
// foreign sandbox as its cwd and refused with the loud missing-file
// error — the №563 flip turned the formerly-silent race into a failure,
// which is exactly what a loud contract is FOR). serial_test serializes
// the suite within the binary; cargo test runs the binaries sequentially.
use serial_test::serial;
use std::cell::RefCell;

thread_local! {
    static PREV: RefCell<Option<std::path::PathBuf>> = const { RefCell::new(None) };
}

/// A temp sandbox as the process cwd (the same posture the №507 suite
/// uses) — restored even on panic.
struct SandboxDir(&'static str);
impl SandboxDir {
    fn enter(name: &'static str) -> Self {
        let dir = std::env::temp_dir().join(format!("mlog_n563_{}_{}", name, std::process::id()));
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
        let dir = std::env::temp_dir().join(format!("mlog_n563_{}_{}", self.0, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let program = metalogos::compiler::Compiler::new().compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

const MISSING_FLOW: &str = r#"
flow Main { input: String = "n563_definitely_missing.txt" -> read_file -> output }
"#;

const MISSING_OR_FLOW: &str = "pattern ReadOr563(_x: String) -> String {\n  return read_file_or(\"n563_definitely_missing.txt\", \"fallback-563\")\n}\nflow Main {\n  input: String = \"a\" -> ReadOr563 -> output\n}\n";

// ── the loud flip on BOTH backends ───────────────────────────────────

#[test]
#[serial]
fn n563_tw_read_file_missing_refuses_loudly() {
    let _sb = SandboxDir::enter("tw_missing");
    let err = metalogos::run_program(MISSING_FLOW)
        .expect_err("the soft empty-string contract ended at v0.28.0 (№563)");
    assert!(
        err.contains("[IO_ERROR]"),
        "the missing file refuses loudly with the stable code, got: {}",
        err
    );
    assert!(
        err.contains("read_file_or"),
        "the refusal names the explicit-fallback surface, got: {}",
        err
    );
}

#[test]
#[serial]
fn n563_vm_read_file_missing_refuses_loudly() {
    let _sb = SandboxDir::enter("vm_missing");
    let err =
        run_vm(MISSING_FLOW).expect_err("the VM lane must refuse the missing file identically");
    assert!(
        err.contains("[IO_ERROR]"),
        "the VM refusal carries the same stable code, got: {}",
        err
    );
}

// ── the `_or` twin: the ONLY remaining soft surface (unchanged) ──────

#[test]
#[serial]
fn n563_read_file_or_still_yields_the_default_tw() {
    let _sb = SandboxDir::enter("tw_or");
    let out = metalogos::run_program(MISSING_OR_FLOW)
        .expect("read_file_or keeps the explicit-silence contract");
    assert_eq!(
        out.as_deref(),
        Some("fallback-563"),
        "the explicit default reaches the flow output (unchanged by №563)"
    );
}

#[test]
#[serial]
fn n563_read_file_or_still_yields_the_default_vm() {
    let _sb = SandboxDir::enter("vm_or");
    let out = run_vm(MISSING_OR_FLOW)
        .expect("read_file_or keeps the explicit-silence contract on the VM");
    assert_eq!(
        out.map(|s| s.trim().to_string()),
        Some("fallback-563".to_string()),
        "the explicit default reaches the VM flow output (unchanged by №563)"
    );
}

// ── existing-file reads: byte-identical ──────────────────────────────

#[test]
#[serial]
fn n563_existing_file_reads_exactly_as_before() {
    let _sb = SandboxDir::enter("existing");
    std::fs::write("n563_present.txt", "the real content").unwrap();
    let src = program_of(r#"  return read_file("n563_present.txt")"#);
    let out = metalogos::run_program(&src).expect("an existing file reads");
    assert_eq!(out.as_deref(), Some("the real content"));
}

#[test]
#[serial]
fn n563_debt_counter_reads_zero() {
    // The only-down lock: the committed baseline records 0 and the live
    // count agrees (the marker string is gone from the tree).
    let out = Command::new("python3")
        .args([
            "scripts/ci/debt_counters.py",
            "--gate",
            "scripts/ci/debt_baseline.txt",
        ])
        .output()
        .expect("python3 must exist (the CI image runs the gate scripts)");
    assert!(
        out.status.success(),
        "the debt gate must pass with read_file_soft_missing at 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn program_of(body: &str) -> String {
    format!(
        "pattern Read563(_x: String) -> String {{\n{}\n}}\nflow Main {{\n  input: String = \"a\" -> Read563 -> output\n}}\n",
        body
    )
}

use std::process::Command;
