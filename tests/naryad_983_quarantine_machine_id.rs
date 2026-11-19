// ── №983 (issue #1997): the quarantine receipt is a MACHINE ID ────────
//
// The product fix the №980 echo hand named as its own removal condition:
// `quarantine_write` returns `qrt-<unix>-<seq>` — neither the PAYLOAD
// (args[0]) nor the REASON (args[1]) rides the return; the reason goes
// to the audit journal on stderr (the №326 unconditional event), so the
// №981/№980 label hand could be retired (`RECEIPT_ECHO_ARGS` is empty)
// and the receipt tables are honest by the new body (the §7 method).
//
// THE MUTATION TARGET (№983's own, `scripts/mutation_verify_983.sh`):
// restore the reason in the format string of `builtin_quarantine_write`
// → THIS pin file reddens in three places at once: the TW format pin,
// the VM format pin and the marker pin (the return stops matching
// `qrt-…` and starts carrying the reason). The stderr journal test
// holds under the mutation too (the reason was ALWAYS journal-bound in
// the mutant's stderr line) — the return format is the discriminating
// assertion.
#![allow(clippy::disallowed_methods)]

use std::path::Path;
use std::sync::OnceLock;

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program(source)
}

fn mlog_bin() -> &'static str {
    static MLOG_BIN: OnceLock<String> = OnceLock::new();
    MLOG_BIN.get_or_init(|| {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        std::path::Path::new(&manifest_dir)
            .join("target")
            .join("debug")
            .join("mlog")
            .to_string_lossy()
            .into_owned()
    })
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp =
        metalogos::compiler::Compiler::with_std_root(Path::new("examples").to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

const MACHINE_ID_RETURN: &str = r#"
pattern P(d: String) -> String {
  let r = quarantine_write("q", "ATTACKER_REASON_MARKER_983")
  return r
}
flow Main { input: String = "x" -> P -> output }
"#;

fn assert_machine_id(backend: &str, value: &str) {
    assert!(
        value.starts_with("qrt-"),
        "{backend}: the quarantine receipt must be a machine id \\
         `qrt-<unix>-<seq>` (№983) — got: {value:?}; a non-qrt return \\
         means the audit text came back (the echo re-entered the return)"
    );
    assert!(
        !value.contains("ATTACKER_REASON_MARKER_983"),
        "{backend}: the receipt must NOT echo its reason (№983 — the \\
         reason belongs to the audit journal) — got: {value:?}"
    );
    assert!(
        !value.contains("[QUARANTINE_EGRESS]"),
        "{backend}: the receipt must not be the audit text (the old body \\
         re-entered) — got: {value:?}"
    );
}

/// TW: the walk lane's return is the machine id, with neither the
/// payload nor the reason. THE mutation discriminator (see the header).
#[test]
fn n983_tw_the_receipt_is_a_machine_id_not_the_reason() {
    match run_tw(MACHINE_ID_RETURN) {
        Ok(Some(v)) => assert_machine_id("TW", &v),
        other => panic!("TW/machine-id: the probe did not yield the receipt — got {other:?}"),
    }
}

/// VM: the executor lane — the same discriminator.
#[test]
fn n983_vm_the_receipt_is_a_machine_id_not_the_reason() {
    match run_vm(MACHINE_ID_RETURN) {
        Ok(Some(v)) => assert_machine_id("VM", &v),
        other => panic!("VM/machine-id: the probe did not yield the receipt — got {other:?}"),
    }
}

/// THE AUDIT JOURNAL (the №983 task 4): the reason still lands in the
/// journal — the unconditional №326 event on stderr, with the
/// audit-event convention — or the fix would LOSE information, not move
/// it. Verified through the CLI subprocess (stderr capture, the lib API
/// cannot capture eprintln in-process).
#[test]
fn n983_the_reason_lands_in_the_audit_journal() {
    let src = r#"
pattern P(d: String) -> String {
  let _ = quarantine_write("q", "AUDIT_REASON_MARKER_983")
  return "ok"
}
flow Main { input: String = "x" -> P -> output }
"#;
    let probe_dir = std::env::temp_dir().join("n983_journal_probe");
    let _ = std::fs::remove_dir_all(&probe_dir);
    std::fs::create_dir_all(&probe_dir).expect("the probe dir");
    let path = probe_dir.join("journal_probe.mlog");
    std::fs::write(&path, src).expect("write the probe");
    let out = std::process::Command::new(mlog_bin())
        .arg("run")
        .arg(&path)
        .current_dir(&probe_dir)
        .output()
        .expect("the CLI subprocess");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let _ = std::fs::remove_dir_all(&probe_dir);
    assert!(
        stderr.contains("[QUARANTINE_EGRESS][audit-event]"),
        "the quarantine write must emit the audit-event line (№326, the \\
         unconditional event) on stderr — got stderr: {stderr:?}"
    );
    assert!(
        stderr.contains("AUDIT_REASON_MARKER_983"),
        "the reason must travel to the audit journal (№983 task 4 — the \\
         fix must MOVE information, not lose it) — got stderr: {stderr:?}"
    );
    assert!(
        stderr.contains("qrt-"),
        "the journal line must carry the record id it refers to — got \\
         stderr: {stderr:?}"
    );
}

/// THE SECRET-REASON JOURNAL ROUTE (the №980 task 4's new home): a
/// SECRET passed AS THE REASON lands ONLY in the journal (the
/// sanctioned №326 channel) — the RETURN stays the clean machine id,
/// so the program passes `check` and `run` (the quarantine sink is the
/// legal egress, №325/№964). If the echo ever returns, the secret rides
/// the receipt again and the static pins (n980_*) redden.
#[test]
fn n983_secret_reason_goes_to_the_journal_only() {
    let src = r#"
pattern P(d: String) -> String {
  let r = quarantine_write("q", env("SP"))
  return r
}
flow Main { input: String = "x" -> P -> output }
"#;
    let probe_dir = std::env::temp_dir().join("n983_secret_probe");
    let _ = std::fs::remove_dir_all(&probe_dir);
    std::fs::create_dir_all(&probe_dir).expect("the probe dir");
    let path = probe_dir.join("secret_probe.mlog");
    std::fs::write(&path, src).expect("write the probe");
    let out = std::process::Command::new(mlog_bin())
        .arg("run")
        .arg(&path)
        .current_dir(&probe_dir)
        .env("SP", "TOPSECRET123")
        .output()
        .expect("the CLI subprocess");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let _ = std::fs::remove_dir_all(&probe_dir);
    assert!(
        out.status.success(),
        "the quarantine sink is the LEGAL egress (№325) — the run must \\
         succeed with a secret reason — got stderr: {stderr:?}"
    );
    assert!(
        stderr.contains("TOPSECRET123"),
        "the secret reason must reach the audit journal (the №326 \\
         unconditional event) — got stderr: {stderr:?}"
    );
    assert!(
        !stdout.contains("TOPSECRET123"),
        "the secret must NOT surface on stdout (the receipt is the \\
         machine id) — got stdout: {stdout:?}"
    );
    assert!(
        stdout.contains("qrt-"),
        "the returned receipt is the machine id even for a secret reason \\
         — got stdout: {stdout:?}"
    );
}
