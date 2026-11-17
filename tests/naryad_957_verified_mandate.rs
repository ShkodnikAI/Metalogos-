#![allow(clippy::disallowed_methods)]

// ── Naryad №957 (issue #1924, ADR-0211 §3, [security][consent], P1): the
// verifiable mandate actor — a program cannot approve itself ────────────
//
// The fact (the audit 12.11 probe `4_capability/c05`, reproduced on the
// published v0.39.0): `approval_issue("owner", "carrier:cron_run", 60)`
// FROM THE PROGRAM opened `cron_run` — the actor a program passes is a
// string, not a human, and the human-oversight gate honored it. ADR-0211
// (№901) recommends variant (б)+(в): the mandate is issued OUTSIDE the
// program (the host CLI `mlog approve`), the program-issued ones are
// marked `actor_verified=false` and NEVER count.
//
// The contract this file pins:
// - the DEFAULT mode refuses a program-issued mandate (the c05 flip —
//   THE MUTATION TARGET: making the gate count an unverified mandate
//   reds `n957_the_c05_probe_refuses_a_program_issued_mandate`);
// - the transition window (`profile legacy { approval: warn }`, the
//   №949 п.5 rule / the §12.5 form) opens the carrier LOUDLY for one
//   release window;
// - the VERIFIED path — `mlog approve` / `mandate_issue_verified`, the
//   persisted host-owned store (`METALOGOS_MANDATE_STORE` or
//   `~/.metalogos/mandates.json`, on the №455 sensitive-path deny
//   vocabulary) — passes the carrier on both lanes, and the store
//   survives the restart (the CLI e2e in metalogos-server spawns the
//   real binary twice);
// - the refusal class stays the frozen `IRREVERSIBLE_NO_GRANT` (№385);
//   only the remedy text changed (it names the host CLI now).
//
// The env-var discipline: the store tests set `METALOGOS_MANDATE_STORE`
// to a tempdir and RESTORE the caller's value — serialized (the env is
// process-global, the serial harness form).

use std::path::Path;

const MANIFEST: &str = env!("CARGO_MANIFEST_DIR");

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source.trim(), Path::new(MANIFEST).to_path_buf())
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(Path::new(MANIFEST).to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

type LaneRunner = fn(&str) -> Result<Option<String>, String>;

fn runners() -> [(&'static str, LaneRunner); 2] {
    [("tw", run_tw), ("vm", run_vm)]
}

fn reset() {
    metalogos::approval::mandate_store_reset();
}

/// The store isolation helper: point the verified store at a FILE inside
/// a held tempdir (the override names the store FILE, not its dir) and
/// restore the caller's env value on drop.
struct StoreEnv {
    _dir: tempfile::TempDir,
    saved: Option<String>,
}

impl StoreEnv {
    fn fresh() -> Self {
        let saved = std::env::var("METALOGOS_MANDATE_STORE").ok();
        let dir = tempfile::tempdir().expect("tempdir");
        let store = dir.path().join("mandates.json");
        std::env::set_var("METALOGOS_MANDATE_STORE", &store);
        Self { _dir: dir, saved }
    }
}

impl Drop for StoreEnv {
    fn drop(&mut self) {
        match self.saved.take() {
            Some(v) => std::env::set_var("METALOGOS_MANDATE_STORE", v),
            None => std::env::remove_var("METALOGOS_MANDATE_STORE"),
        }
    }
}

/// THE c05 FLIP (the mutation target): the audit-12.11 probe shape — the
/// program issues its own mandate naming any human it can spell —
/// REFUSES in the default mode on BOTH lanes. On v0.39.0 this program
/// opened the carrier; the trust anchor moved outside the machine.
#[test]
#[serial_test::serial]
fn n957_the_c05_probe_refuses_a_program_issued_mandate() {
    reset();
    let _store = StoreEnv::fresh();
    for (name, runner) in runners() {
        let err = runner(
            r#"
pattern T(_t: String) -> String {
  let _m = approval_issue("owner", "carrier:cron_run", 60)
  let job = cron_add("0 0 * * *", "N957C05")
  let r = cron_run(job.id)
  return r.status
}
flow Main { input: String = "t" -> T -> output<public, untrusted> }
"#,
        )
        .unwrap_err();
        assert!(
            err.starts_with("IRREVERSIBLE_NO_GRANT"),
            "{name}: the program-issued mandate must NOT open the carrier in the \
             default mode (№957, ADR-0211 §3 — the mutation target), got: {err}"
        );
        assert!(
            err.contains("mlog approve"),
            "{name}: the refusal names the HOST remedy, got: {err}"
        );
        assert!(
            err.contains("actor_verified=false"),
            "{name}: the refusal names the marking, got: {err}"
        );
    }
    reset();
}

/// The transition window (the №949 п.5 rule, the §12.5 form): the same
/// program WITH the legacy key passes on both lanes — one release window
/// of loud warnings, then the key is removed and the program carrying it
/// is a check error.
#[test]
#[serial_test::serial]
fn n957_the_transition_window_opens_the_carrier_loudly() {
    reset();
    let _store = StoreEnv::fresh();
    for (name, runner) in runners() {
        let out = runner(
            r#"
profile legacy { approval: warn }
pattern T(_t: String) -> String {
  let _m = approval_issue("owner", "carrier:cron_run", 60)
  let job = cron_add("0 0 * * *", "N957Window")
  let r = cron_run(job.id)
  return r.status
}
flow Main { input: String = "t" -> T -> output<public, untrusted> }
"#,
        )
        .unwrap()
        .unwrap();
        assert!(
            out.contains("queued"),
            "{name}: the window opens the carrier for the migration, got: {out}"
        );
    }
    reset();
}

/// The VERIFIED path: the host-issued mandate (the Rust seam — the same
/// function `mlog approve` calls) passes the carrier WITHOUT the window
/// key and WITHOUT any program-issued mandate, on BOTH lanes. The store
/// is the persisted file — the gate read it AT CALL TIME.
#[test]
#[serial_test::serial]
fn n957_the_verified_mandate_passes_the_carrier_on_both_lanes() {
    reset();
    let _store = StoreEnv::fresh();
    metalogos::approval::mandate_issue_verified("owner", "carrier:cron_run", 600)
        .expect("the host seam issues the verified mandate");
    for (name, runner) in runners() {
        let out = runner(
            r#"
pattern T(_t: String) -> String {
  let job = cron_add("0 0 * * *", "N957Verified")
  let r = cron_run(job.id)
  return r.status
}
flow Main { input: String = "t" -> T -> output<public, untrusted> }
"#,
        )
        .unwrap()
        .unwrap();
        assert!(
            out.contains("queued"),
            "{name}: the verified mandate opens the carrier in the default mode, got: {out}"
        );
    }
    reset();
}

/// The marking is VISIBLE and honest: the program can read its own
/// mandate's `actor_verified=false` — the honesty of variant (в) — while
/// the field carries no authority (the gate reads the store).
#[test]
#[serial_test::serial]
fn n957_the_mandate_view_carries_the_marking() {
    reset();
    for (name, runner) in runners() {
        let out = runner(
            r#"
pattern T(_t: String) -> String {
  let m = approval_issue("owner", "carrier:cron_run", 60)
  return to_string(m.actor_verified)
}
flow Main { input: String = "t" -> T -> output<public, untrusted> }
"#,
        )
        .unwrap()
        .unwrap();
        assert!(
            out.trim() == "false",
            "{name}: the program-issued mandate reads actor_verified=false, got: {out}"
        );
    }
    reset();
}

/// The store isolation: `.metalogos/**` and `mandates.json` are on the
/// №455 sensitive-path deny vocabulary — the program cannot read OR
/// write the host's approval authority (a forged store would mint the
/// counted class). The LAYERS, honestly: a LITERAL sensitive path is
/// refused by the STATIC label engine (the sensitive-named path yields
/// the Secret label — the print refuses BEFORE any file bytes move); a
/// runtime-CONSTRUCTED path slips the literal rule and hits the RUNTIME
/// sandbox re-check on the resolved path (`SANDBOX_SENSITIVE_PATH`).
#[test]
#[serial_test::serial]
fn n957_the_program_cannot_touch_the_verified_store() {
    reset();
    for (name, runner) in runners() {
        // (1) The literal form: the first line refuses — WHICH line is
        // lane-order: TW labels the sensitive-named path Secret and the
        // print refuses (PII_EGRESS_OUTPUT); the VM compile runs the
        // №455 sandbox pre-check on the literal and refuses with the
        // sandbox class. Both are the honest BEFORE-the-action refusal.
        let err = runner(
            r#"
pattern T(_t: String) -> String {
  return read_file("/home/someone/.metalogos/mandates.json")
}
flow Main { input: String = "t" -> T -> output<public, untrusted> }
"#,
        )
        .unwrap_err();
        assert!(
            err.contains("PII_EGRESS_OUTPUT") || err.contains("SANDBOX_SENSITIVE_PATH"),
            "{name}: a literal sensitive path refuses BEFORE the action (the static \
             first line — the label class on TW, the sandbox class on VM), got: {err}"
        );
        // (2) The runtime form: the path CONSTRUCTED at runtime — the
        // literal rule does not fire, the sandbox re-check on the
        // resolved path does (the №455 layer 2, symlink-proof).
        let err = runner(
            r#"
pattern T(_t: String) -> String {
  let p = ".metalogos/" + "mandates.json"
  return read_file(p)
}
flow Main { input: String = "t" -> T -> output<public, untrusted> }
"#,
        )
        .unwrap_err();
        assert!(
            err.contains("SANDBOX_SENSITIVE_PATH"),
            "{name}: a runtime-constructed store path hits the sandbox deny at \
             runtime (№957), got: {err}"
        );
        // (3) The relocated write: the store NAME is denied in every
        // directory — the program cannot write a namesake past the dir
        // form.
        let err = runner(
            r#"
pattern T(_t: String) -> String {
  write_file("mandates.json", "forged")
  return "written"
}
flow Main { input: String = "t" -> T -> output<public, untrusted> }
"#,
        )
        .unwrap_err();
        assert!(
            err.contains("SANDBOX_SENSITIVE_PATH"),
            "{name}: writing a relocated store refuses (the name form of the deny \
             vocabulary, №957), got: {err}"
        );
    }
    reset();
}
