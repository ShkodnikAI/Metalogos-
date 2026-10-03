// ── Naryad #481 (P1, security/hardening): loud env/read_file + the *_or semantics ──
//
// Contract (issue #729, audit 25.09 §3.9 — "open, io.rs:183 unchanged"):
// 1. THE HOLE: `env()` returned an EMPTY STRING for a missing variable —
//    a configuration error (a misnamed/undeployed secret) was masked
//    silently. `env()` now refuses LOUDLY with the stable code
//    ENV_NOT_FOUND, naming the variable.
// 2. THE EXPLICIT-SILENCE TWIN: `env_or(name, default)` — the `_or`
//    suffix carries the silent-default semantics IN THE NAME (the audit
//    naming rule); the fallback firing is announced on the audit stderr
//    (the №326 op-log posture — the variable NAME, never the value). The
//    SAME №259 gate applies — explicit silence does not bypass the
//    serve-route env policy.
// 3. read_file: a file that passed the sandbox and is NOT missing but
//    cannot be opened/read is a config/environment error — refused
//    LOUDLY (IO_ERROR, the OS reason) instead of a silent "". The
//    missing-file soft contract (№254) was preserved and pinned here —
//    and ENDED at v0.28.0 (№563): the №531 transition window closed
//    with the release, so a MISSING file refuses LOUDLY too; the
//    explicit-silence surface is `read_file_or(path, default)` (the
//    №514 naming rule). This pin was updated with the flip (the pin
//    follows the contract, the contract does not follow the pin).
// 4. Back-compat: programs needing the old silence migrate to `env_or`
//    (the name already says "default"); nothing else changes.

// Naryad #475's fs_gate ratchet targets PRODUCTION I/O paths; this test
// exercises the REAL filesystem for its fixtures by design — the scoped
// allow mirrors the naryad_465 fuzzer posture.
#![allow(clippy::disallowed_methods)]

use metalogos::builtins::{set_process_mode, ProcessMode};
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

struct EnvVar(&'static str);
impl EnvVar {
    fn set(name: &'static str, value: &str) -> Self {
        std::env::set_var(name, value);
        EnvVar(name)
    }
}
impl Drop for EnvVar {
    fn drop(&mut self) {
        std::env::remove_var(self.0);
    }
}

/// Drop-guard: the serve-mode tests must restore the Process default even
/// on panic (the №457 test-posture leksalo).
struct RestoreProcessMode;
impl Drop for RestoreProcessMode {
    fn drop(&mut self) {
        set_process_mode(ProcessMode::Process);
    }
}

const PROBE: &str = "METALOGOS_N481_PROBE";

fn program(body: &str) -> String {
    format!(
        "pattern Read(_x: String) -> String {{\n  {}\n}}\nflow Main {{\n  input: String = \"a\" -> Read -> output\n}}\n",
        body
    )
}

#[test]
fn n481_env_missing_is_loud_and_names_the_variable() {
    let _env = lock_env();
    std::env::remove_var(PROBE);

    let src = program(&format!("  return env(\"{}\")", PROBE));

    // TW: loud refusal with the stable code and the variable name.
    let err =
        metalogos::run_program(&src).expect_err("env() of a missing variable must refuse loudly");
    assert!(
        err.contains("[ENV_NOT_FOUND]"),
        "the refusal must carry the stable code: {}",
        err
    );
    assert!(
        err.contains(PROBE),
        "the refusal must name the variable: {}",
        err
    );
    assert!(
        err.contains("env_or"),
        "the refusal must point at the explicit-silence twin: {}",
        err
    );

    // VM parity: the SAME builtin → the SAME loud refusal.
    let decls = metalogos::parser::parse(&src).expect("parses");
    let mut comp = metalogos::compiler::Compiler::with_std_root(std::path::PathBuf::from("."));
    let program = comp.compile(decls).expect("compiles");
    let mut vm = metalogos::vm::Vm::new();
    let vm_err = vm
        .run(program)
        .expect_err("the VM must refuse identically (the builtin is shared)");
    assert!(
        vm_err.contains("[ENV_NOT_FOUND]") && vm_err.contains(PROBE),
        "VM parity of the loud env refusal: {}",
        vm_err
    );
}

#[test]
fn n481_env_present_returns_the_value() {
    let _env = lock_env();
    let _probe = EnvVar::set(PROBE, "n481-value");

    let src = program(&format!("  return env(\"{}\")", PROBE));
    let out = metalogos::run_program(&src).expect("a set variable reads fine");
    assert_eq!(out.unwrap_or_default().trim_end(), "n481-value");
}

#[test]
fn n481_env_or_carries_the_explicit_silence() {
    let _env = lock_env();
    std::env::remove_var(PROBE);

    // Missing → the default, NO refusal (the silence the name declares).
    let src = program(&format!("  return env_or(\"{}\", \"n481-default\")", PROBE));
    let out = metalogos::run_program(&src).expect("env_or falls back silently");
    assert_eq!(out.unwrap_or_default().trim_end(), "n481-default");

    // Present → the value, not the default.
    let _probe = EnvVar::set(PROBE, "n481-real");
    let out = metalogos::run_program(&src).expect("env_or reads a set variable");
    assert_eq!(out.unwrap_or_default().trim_end(), "n481-real");

    // VM parity: the SAME builtin → the SAME silent fallback and the SAME
    // read (the №476 lesson — env is a blocked domain, parity is pinned).
    std::env::remove_var(PROBE);
    let decls = metalogos::parser::parse(&src).expect("parses");
    let mut comp = metalogos::compiler::Compiler::with_std_root(std::path::PathBuf::from("."));
    let compiled = comp.compile(decls).expect("compiles");
    let mut vm = metalogos::vm::Vm::new();
    let vm_out = vm.run(compiled).expect("the VM falls back identically");
    assert_eq!(
        vm_out.unwrap_or_default().trim_end(),
        "n481-default",
        "VM parity of the env_or silent fallback"
    );
}

/// The №326 op-log posture: the fallback firing is ANNOUNCED on the audit
/// stderr (the variable NAME only) — and the VALUE never reaches the log.
/// Pinned through the REAL binary (the same-process stderr is not
/// capturable in-process).
#[test]
fn n481_env_or_fallback_is_announced_without_the_value() {
    let probe = "METALOGOS_N481_OR_PROBE";
    std::env::remove_var(probe);

    let src = program(&format!(
        "  return env_or(\"{}\", \"n481-secret-default\")",
        probe
    ));
    let file =
        std::path::Path::new("target").join(format!("n481_or_pin_{}.mlog", std::process::id()));
    // №567: the test's CWD is the package dir now — its local target/ may
    // not exist yet (the workspace target lives at the repo root).
    std::fs::create_dir_all("target").expect("create the local target dir");
    std::fs::write(&file, &src).expect("write the probe program");

    let out = std::process::Command::new(env!("CARGO_BIN_EXE_mlog"))
        .arg("run")
        .arg(&file)
        .output()
        .expect("the mlog binary runs");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);

    let _ = std::fs::remove_file(&file);

    assert!(
        out.status.success(),
        "env_or must NOT refuse: {} | {}",
        stdout.trim(),
        stderr.trim()
    );
    assert!(
        stderr.contains("[ENV_OR]") && stderr.contains(probe),
        "the fallback firing must be announced with the variable name: {}",
        stderr.trim()
    );
    assert!(
        !stderr.contains("n481-secret-default"),
        "the VALUE must never reach the log (the №326 posture): {}",
        stderr.trim()
    );
    assert!(
        stdout.contains("n481-secret-default"),
        "the value still flows to the PROGRAM result (silence is about the log, not the data): {}",
        stdout.trim()
    );
}

#[test]
fn n481_env_or_does_not_bypass_the_serve_gate() {
    let _env = lock_env();
    let _restore = RestoreProcessMode;
    set_process_mode(ProcessMode::Serve);
    std::env::remove_var(PROBE);

    // The explicit silence does NOT bypass the №259 serve-route env policy:
    // an unmarked thread inside a serve process is refused BEFORE the
    // silence can fire.
    let src = program(&format!("  return env_or(\"{}\", \"n481-default\")", PROBE));
    let err = metalogos::run_program(&src)
        .expect_err("env_or on an unmarked serve-route thread must hit the №259 gate");
    assert!(
        err.contains("ENV_NOT_PERMITTED"),
        "the №259 gate must fire BEFORE the silence: {}",
        err
    );
}

#[cfg(unix)]
#[test]
fn n481_read_file_unreadable_and_missing_refuse_loudly() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let _env = lock_env();
    // The test CWD is the repo root = the sandbox base; a relative path
    // stays inside the sandbox and reaches the IO layer.
    let rel = "target/n481_unreadable.txt";
    let path = std::path::Path::new(rel);
    let _ = fs::create_dir_all(path.parent().unwrap());
    fs::write(path, "n481").expect("write probe file");

    // (a) An EXISTING but unreadable file → the loud IO_ERROR (the config
    //     error the old silent "" masked).
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(path, perms.clone()).expect("chmod");

    let src = program(&format!("  return read_file(\"{}\")", rel));
    let err = metalogos::run_program(&src)
        .expect_err("an existing-but-unreadable file must refuse loudly");
    assert!(
        err.contains("[IO_ERROR]") && err.contains(rel),
        "the refusal must carry the stable code and the path: {}",
        err
    );

    // (b) A MISSING file → the LOUD [IO_ERROR] refusal since v0.28.0
    //     (№563: the №254 soft contract ended with the transition
    //     release; migrate to read_file_or(path, default) — the message
    //     names the migration path, the same posture the loud env
    //     refusal has).
    fs::remove_file(path).ok();
    let src = program("  return read_file(\"target/n481_really_missing.txt\")");
    let err = metalogos::run_program(&src)
        .expect_err("the missing-file soft contract ended at v0.28.0 (№563)");
    assert!(
        err.contains("[IO_ERROR]") && err.contains("target/n481_really_missing.txt"),
        "the missing-file refusal must carry the stable code and the path: {}",
        err
    );
    assert!(
        err.contains("read_file_or"),
        "the refusal must name the migration path (the loud contract teaches): {}",
        err
    );

    perms.set_mode(0o644);
}
