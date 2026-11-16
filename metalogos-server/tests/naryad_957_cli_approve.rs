//! ── Naryad №957 (issue #1924, ADR-0211 §3, [security][consent], P1) —
//! the CLI e2e: the HOST issuance of the verified mandate ────────────────
//!
//! The «сделано, когда» form of the naryad, run against the REAL binary:
//! the c05 program refuses; `mlog approve carrier:cron_run --actor owner
//! --ttl 60` issues the verified mandate into the host-owned persisted
//! store; the SAME program then passes — on a fresh process (the store
//! survives the restart, because the mandate never lived in the
//! process). The store path is a tempdir per test (the
//! `METALOGOS_MANDATE_STORE` override — the host contour's form).
//!
//! The mutation contract: counting a program-issued mandate (the gate
//! accepting actor_verified=false) reds
//! `tests/naryad_957_verified_mandate.rs::n957_the_c05_probe_refuses_a_program_issued_mandate`
//! — the mechanical form is `scripts/mutation_verify_957.sh`.

#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;
use std::process::Command;

fn mlog() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_mlog"))
}

/// The c05 program (the audit 12.11 probe `4_capability/c05`): the
/// program issues its own mandate naming any human it can spell.
const C05_WITHOUT_KEY: &str = r#"pattern T(_t: String) -> String {
  let _m = approval_issue("owner", "carrier:cron_run", 60)
  let job = cron_add("0 0 * * *", "N957CliC05")
  let r = cron_run(job.id)
  return r.status
}
flow Main { input: String = "t" -> T -> output }
"#;

fn run_mlog(args: &[&str], store: &std::path::Path, cwd: &std::path::Path) -> (bool, String) {
    let out = Command::new(mlog())
        .args(args)
        .env("METALOGOS_MANDATE_STORE", store.join("mandates.json"))
        .current_dir(cwd)
        .output()
        .expect("the mlog binary must spawn");
    let text = format!(
        "--- stdout ---\n{}--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

#[test]
fn n957_cli_approve_is_the_host_ritual_the_program_cannot_perform() {
    let workdir = tempfile::tempdir().expect("tempdir");
    let store = tempfile::tempdir().expect("tempdir");
    let program = workdir.path().join("c05.mlog");
    std::fs::write(&program, C05_WITHOUT_KEY).expect("write the c05 program");

    // 1. WITHOUT the host approval: the carrier refuses BEFORE the
    //    action (the default mode — the program-issued mandate inside
    //    the program does not count).
    let (ok, text) = run_mlog(
        &["run", program.to_str().unwrap()],
        store.path(),
        workdir.path(),
    );
    assert!(
        !ok,
        "the c05 program must refuse in the default mode (№957); output: {text}"
    );
    assert!(
        text.contains("IRREVERSIBLE_NO_GRANT") && text.contains("mlog approve"),
        "the refusal names the frozen class and the HOST remedy: {text}"
    );

    // 2. The host ritual: the verified mandate issued OUTSIDE the
    //    program (the counted class is born outside the program's reach).
    let (ok, text) = run_mlog(
        &[
            "approve",
            "carrier:cron_run",
            "--actor",
            "owner",
            "--ttl",
            "600",
        ],
        store.path(),
        workdir.path(),
    );
    assert!(
        ok,
        "the host CLI issues the verified mandate; output: {text}"
    );
    assert!(
        text.contains("actor_verified=true") && text.contains("store:"),
        "the approval prints the honest record: {text}"
    );

    // 3. The SAME program on a FRESH process now passes — the store
    //    persisted (the restart half of the naryad: the mandate never
    //    lived in the process, so there is nothing to lose).
    let (ok, text) = run_mlog(
        &["run", program.to_str().unwrap()],
        store.path(),
        workdir.path(),
    );
    assert!(
        ok,
        "after the host approval the c05 program passes on a fresh process (the \
         persisted verified store); output: {text}"
    );
    assert!(
        text.contains("queued"),
        "the carrier executed under the verified mandate: {text}"
    );
}
