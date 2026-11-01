#![allow(clippy::disallowed_methods)]

// ── Naryad №865 (issue #1707, P0 — the migration honesty 0.35–0.38,
// out-of-wave on the owner's directive 2026-11-01) ──────────────────────
//
// The done-when of the naryad: a 0.34.1-shaped program (http_post +
// send_message + cron_run), migrated strictly per docs/upgrade.md,
// passes `mlog check` and executes on 0.38.0 — and the UNMIGRATED twin
// refuses with the carrier NAMED and the resolution paths verbatim.
// This file is that proof, on BOTH backends (the TW/VM parity harness
// form, №748/№829):
//
//   1. the unmigrated refusal names the carrier and the three legal
//      paths (the capability gate text, src/capability.rs::refusal) —
//      the inventory anchor the upgrade page documents;
//   2. the cron_run refusal names the approval_issue recipe
//      (src/approval.rs::approval_refusal);
//   3. `mlog check` stays static: BOTH the migrated and the unmigrated
//      program pass it — the gate is a runtime decision before the
//      action (the split docs/upgrade.md documents, pinned here so a
//      future "gate at check time" move is a conscious act);
//   4. the minimal migration (the profile permit + the mandate) lets
//      the program run — cron_run executes offline (the queued status)
//      and the delivery carriers pass BOTH gates (the completion — not
//      the capability refusal — fires next, the n845 assertion form);
//   5. the strict migration (the covering grant, the "n" quota) passes
//      the capability gate the same way.
//
// The capability/approval rule code is NOT touched by №865 — this file
// only consumes the documented surface (the upgrade page is the
// recipe; the test is the receipt).

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

/// The lane-runner type — the TW/VM parity harness form (№748/№829).
type LaneRunner = fn(&str) -> Result<Option<String>, String>;

fn runners() -> [(&'static str, LaneRunner); 2] {
    [("tw", run_tw), ("vm", run_vm)]
}

fn reset() {
    metalogos::approval::mandate_store_reset();
}

/// The 0.34.1-shaped program: the delivery pair (http_post +
/// send_message) and the autonomous trigger (cron_run), NO capability
/// surface — exactly what the №865 fact list measured on the two
/// downstream repositories (127 http_post, 37 send_message, 6 exec,
/// 2 cron_run; no allow_carrier anywhere).
const UNMIGRATED: &str = r#"
pattern T(_t: String) -> String {
  let job = cron_add("0 0 * * *", "N865Legacy")
  let _m = cron_run(job.id)
  let _h = http_post("https://legacy.example.invalid/hook", "ping")
  let _s = send_message("123", "ping")
  return "done"
}
flow Main { input: String = "t" -> T -> output }
"#;

/// The SAME program under the minimal migration (docs/upgrade.md §
/// Step 2 + Step 4): the profile permit for the delivery pair, the
/// live mandate for the autonomous trigger.
const MIGRATED_MINIMAL: &str = r#"
profile capability { allow_carrier: "http_post" allow_carrier: "send_message" }
pattern T(_t: String) -> String {
  let job = cron_add("0 0 * * *", "N865Migrated")
  let _m = approval_issue("owner", "carrier:cron_run", 600)
  let _c = cron_run(job.id)
  let _h = http_post("https://migrated.example.invalid/hook", "ping")
  let _s = send_message("123", "ping")
  return "done"
}
flow Main { input: String = "t" -> T -> output }
"#;

// ── (1) the unmigrated refusal names the carrier and the paths ────────

#[test]
#[serial_test::serial]
fn unmigrated_program_refuses_naming_the_carrier_and_the_paths() {
    for (name, runner) in runners() {
        let err = runner(UNMIGRATED).unwrap_err();
        assert!(
            err.starts_with("IRREVERSIBLE_NO_GRANT"),
            "{name}: the unmigrated program refuses BEFORE the action, got: {err}"
        );
        // The inventory anchor (docs/upgrade.md § Step 1): the refusal
        // names the CARRIER...
        assert!(
            err.contains("cron_run"),
            "{name}: the refusal names the carrier (cron_run fires first), got: {err}"
        );
        // ...and the RESOLUTION PATHS verbatim (the capability text) —
        // the upgrade page's claim, pinned.
        let cap_err = runner(
            r#"
pattern T(_t: String) -> String {
  let _h = http_post("https://legacy.example.invalid/hook", "ping")
  return "done"
}
flow Main { input: String = "t" -> T -> output }
"#,
        )
        .unwrap_err();
        assert!(cap_err.starts_with("IRREVERSIBLE_NO_GRANT"));
        for fragment in ["builtin:http_post", "allow_carrier", "http_post"] {
            assert!(
                cap_err.contains(fragment),
                "{name}: the capability refusal names the path {fragment}, got: {cap_err}"
            );
        }
    }
    reset();
}

// ── (2) the cron_run refusal names the approval_issue recipe ──────────

#[test]
#[serial_test::serial]
fn cron_run_refusal_names_the_mandate_recipe() {
    for (name, runner) in runners() {
        let err = runner(
            r#"
pattern T(_t: String) -> String {
  let job = cron_add("0 0 * * *", "N865NoMandate")
  let r = cron_run(job.id)
  return r.status
}
flow Main { input: String = "t" -> T -> output }
"#,
        )
        .unwrap_err();
        assert!(
            err.starts_with("IRREVERSIBLE_NO_GRANT"),
            "{name}: cron_run without a mandate refuses BEFORE the action, got: {err}"
        );
        for fragment in ["approval_issue", "carrier:cron_run"] {
            assert!(
                err.contains(fragment),
                "{name}: the approval refusal names the recipe fragment {fragment}, got: {err}"
            );
        }
    }
    reset();
}

// ── (3) mlog check stays static — both programs pass it ───────────────

#[test]
fn check_lane_passes_both_the_migrated_and_the_unmigrated_program() {
    let root = Path::new(MANIFEST).to_path_buf();
    for (label, source) in [("unmigrated", UNMIGRATED), ("migrated", MIGRATED_MINIMAL)] {
        let result = metalogos::check_program_with_root(source.trim(), Some(root.as_path()))
            .unwrap_or_else(|e| panic!("{label}: check_program errored: {e}"));
        assert!(
            result.is_ok(),
            "{label}: the static lane passes (the capability gate is a runtime decision — the docs/upgrade.md split), got: {}",
            result.format()
        );
    }
}

// ── (4) the minimal migration passes BOTH gates on both lanes ─────────

#[test]
#[serial_test::serial]
fn migrated_program_executes_the_autonomous_trigger_and_passes_the_delivery_gates() {
    reset();
    // (4a) the FULLY executable pass: the mandate is live, cron_run
    // queues offline — the whole program completes (the n861 form).
    for (name, runner) in runners() {
        let out = runner(
            r#"
pattern T(_t: String) -> String {
  let job = cron_add("0 0 * * *", "N865FullPass")
  let _m = approval_issue("owner", "carrier:cron_run", 600)
  let r = cron_run(job.id)
  return r.status
}
flow Main { input: String = "t" -> T -> output }
"#,
        )
        .unwrap()
        .unwrap();
        assert!(
            out.contains("queued"),
            "{name}: the migrated program executes cron_run under the live mandate, got: {out}"
        );
    }
    // (4b) the three-carrier program (the done-when shape): BOTH gates
    // pass — the completion that fires next is the carriers' own
    // delivery mechanics (the network in CI), never the capability or
    // the approval refusal (the n845 assertion form).
    for (name, runner) in runners() {
        match runner(MIGRATED_MINIMAL) {
            Ok(v) => {
                let out = v.unwrap_or_default();
                assert!(
                    !out.contains("IRREVERSIBLE_NO_GRANT"),
                    "{name}: the migrated program must pass both gates, got: {out}"
                );
            }
            Err(e) => assert!(
                !e.starts_with("IRREVERSIBLE_NO_GRANT"),
                "{name}: the migrated program must pass both gates (the delivery mechanics fire next), got: {e}"
            ),
        }
    }
    reset();
}

// ── (5) the strict migration: the covering grant meters the carrier ───

#[test]
#[serial_test::serial]
fn strict_grant_migration_passes_the_capability_gate() {
    for (name, runner) in runners() {
        match runner(
            r#"
pattern T(_t: String) -> String {
  let cap = grant_issue("builtin:http_post", 600, "n", 3)
  let _h = http_post("https://strict.example.invalid/hook", "ping")
  return type_of(cap)
}
flow Main { input: String = "t" -> T -> output }
"#,
        ) {
            Ok(v) => {
                let out = v.unwrap_or_default();
                assert!(
                    !out.contains("IRREVERSIBLE_NO_GRANT"),
                    "{name}: the covering grant passes the capability gate, got: {out}"
                );
            }
            Err(e) => assert!(
                !e.starts_with("IRREVERSIBLE_NO_GRANT"),
                "{name}: the covering grant passes the capability gate (the delivery mechanics fire next), got: {e}"
            ),
        }
    }
}
