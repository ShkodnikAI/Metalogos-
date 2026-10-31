#![allow(clippy::disallowed_methods)]

// ── Naryad №861 (wave 75 — W-D, P1, [consent][core]): the coverage rule —
// the fail-closed approval gate on the human-oversight carriers
// (ADR-0205 §2.6, dispatch gh#1699) ─────────────────────────────────────
//
// The rule: a human-oversight carrier without a LIVE mandate (verify =
// Deny) refuses BEFORE the action; the refusal is the W-C algebra class
// IRREVERSIBLE_NO_GRANT (the vocabulary does not grow — №385), the same
// deny-event path as the capability gate of №845. The e2e trace:
// request → approve (the mandate issued) → execute (passes) →
// revoke/expire → deny — on BOTH backends (TW/VM parity — the №829
// lesson), the Ledger records in the existing form (the deny event rides
// the refusal path; no new record kinds).
//
// Negative controls: (а) execution without a mandate → the refusal
// BEFORE the action + the Ledger trace; (б) after `approval_revoke` the
// previously live mandate stops covering the carrier. The foreign-digest
// control rides the approval.rs unit tests (a mandate over another
// target never covers the carrier).
//
// The registry transaction (the class composition) is explicit:
// `approval::HUMAN_OVERSIGHT` opens with `cron_run` — pinned by the
// totality test in src/approval.rs (an empty class or a
// non-ProfilePermit name reddens there).

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

// ── The e2e trace: request → approve → execute → revoke → deny ────────

#[test]
#[serial_test::serial]
fn the_full_consent_arc_passes_then_denies_on_both_lanes() {
    reset();
    // (1)-(3): the request (cron_add — the scheduled-run request that
    // WILL need the human oversight), the approve (approval_issue — the
    // mandate issued), the execute (cron_run passes under the LIVE
    // mandate). The ask_approval REQUEST surface keeps its own semantics
    // untouched (ADR-0205 §6) — its registry arity contract is not this
    // naryad's boundary (the finding rides the PR description).
    for (name, runner) in runners() {
        let out = runner(
            r#"
pattern T(_t: String) -> String {
  let job = cron_add("0 0 * * *", "N861Archive")
  let m = approval_issue("owner", "carrier:cron_run", 600)
  let r = cron_run(job.id)
  return type_of(job) + ":" + type_of(m) + ":" + r.status
}
flow Main { input: String = "t" -> T -> output }
"#,
        )
        .unwrap()
        .unwrap();
        assert!(
            out.contains("Struct:Struct:queued"),
            "{name}: request → approve → execute passes under the live mandate, got: {out}"
        );
    }

    // (4)-(5): the revoke (approval_revoke — the human withdraws), then
    // the deny — the previously live mandate stops covering the carrier.
    for (name, runner) in runners() {
        let err = runner(
            r#"
pattern T(_t: String) -> String {
  let _m = approval_issue("owner", "carrier:cron_run", 600)
  let _r1 = approval_revoke("carrier:cron_run")
  let _r2 = approval_revoke("carrier:cron_run")
  let job = cron_add("0 0 * * *", "N861Revoked")
  let r = cron_run(job.id)
  return r.status
}
flow Main { input: String = "t" -> T -> output }
"#,
        )
        .unwrap_err();
        assert!(
            err.starts_with("IRREVERSIBLE_NO_GRANT"),
            "{name}: after the revoke the carrier refuses BEFORE the action, got: {err}"
        );
        assert!(
            err.contains("approval gate"),
            "{name}: the refusal names the approval gate (the remedy surface), got: {err}"
        );
    }
    reset();
}

// ── Negative control (а): no mandate → the refusal BEFORE the action ──

#[test]
#[serial_test::serial]
fn execution_without_a_mandate_refuses_before_the_action_on_both_lanes() {
    reset();
    for (name, runner) in runners() {
        // (1) the loud typed refusal (no handler): the class word, no
        // execution side effect (the job is never queued).
        let err = runner(
            r#"
pattern T(_t: String) -> String {
  let job = cron_add("0 0 * * *", "N861NeverRuns")
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
        // (2) the deny event degrades the call (the on_deny contract of
        // №845 — the same handler surface, the same reason class).
        let out = runner(
            r#"
on_deny(network) {
  match deny_reason() {
    "IRREVERSIBLE_NO_GRANT" then { print("deny:IRREVERSIBLE_NO_GRANT") }
    else { print("deny:OTHER") }
  }
}
pattern T(_t: String) -> String {
  let job = cron_add("0 0 * * *", "N861Degrades")
  let r = cron_run(job.id)
  return type_of(r)
}
flow Main { input: String = "t" -> T -> output }
"#,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            out.trim(),
            "Unit",
            "{name}: the approval refusal degrades to Unit, got: {out}"
        );
    }
    // (3) the Ledger carries the deny record (the ACTION-path binding,
    // the existing form — no new record kinds).
    let records = metalogos::ledger::all_records().unwrap();
    assert!(
        records
            .iter()
            .any(|r| r.action == "deny.IRREVERSIBLE_NO_GRANT"),
        "the approval refusal must journal itself (deny.IRREVERSIBLE_NO_GRANT)"
    );
    reset();
}

// ── Negative control (б): revoke stops a previously live mandate ──────

#[test]
#[serial_test::serial]
fn a_revoked_mandate_stops_covering_the_carrier_on_both_lanes() {
    reset();
    for (name, runner) in runners() {
        // The Rust-side arc (the store seam) then the in-language arc:
        // issue → pass → revoke → refuse.
        metalogos::approval::mandate_issue("owner", "carrier:cron_run", 600).unwrap();
        assert!(metalogos::approval::authorize_overseen_call("cron_run").is_ok());
        assert_eq!(
            metalogos::approval::mandate_revoke("carrier:cron_run").unwrap(),
            1
        );
        assert!(metalogos::approval::authorize_overseen_call("cron_run").is_err());

        // The in-language revoke is idempotent (the second call is a
        // no-op over the already-revoked record) and the carrier still
        // refuses after both.
        let err = runner(
            r#"
pattern T(_t: String) -> String {
  let _m = approval_issue("owner", "carrier:cron_run", 600)
  let _r1 = approval_revoke("carrier:cron_run")
  let _r2 = approval_revoke("carrier:cron_run")
  let job = cron_add("0 0 * * *", "N861Idempotent")
  let r = cron_run(job.id)
  return "unreachable" + r.status
}
flow Main { input: String = "t" -> T -> output }
"#,
        )
        .unwrap_err();
        assert!(
            err.starts_with("IRREVERSIBLE_NO_GRANT"),
            "{name}: the second revoke is a no-op and the carrier still refuses, got: {err}"
        );
    }
    reset();
}

// ── The window: an expired mandate stops covering the carrier ─────────

#[test]
#[serial_test::serial]
fn an_expired_mandate_stops_covering_the_carrier_on_both_lanes() {
    reset();
    for (name, runner) in runners() {
        // ttl 1 second; the sleep pushes `now` past the window (the
        // phase23 harness form — the 1.1s sleep is the house pattern).
        let err = runner(
            r#"
pattern T(_t: String) -> String {
  let _m = approval_issue("owner", "carrier:cron_run", 1)
  let job = cron_add("0 0 * * *", "N861Expired")
  sleep(1.1)
  let r = cron_run(job.id)
  return r.status
}
flow Main { input: String = "t" -> T -> output }
"#,
        )
        .unwrap_err();
        assert!(
            err.starts_with("IRREVERSIBLE_NO_GRANT"),
            "{name}: an expired mandate refuses BEFORE the action, got: {err}"
        );
    }
    reset();
}

// ── The foreign-target control at the gate: another target never covers

#[test]
#[serial_test::serial]
fn a_mandate_over_a_foreign_target_never_covers_the_carrier() {
    reset();
    for (name, runner) in runners() {
        let err = runner(
            r#"
pattern T(_t: String) -> String {
  let _m = approval_issue("owner", "carrier:other_carrier", 600)
  let job = cron_add("0 0 * * *", "N861Foreign")
  let r = cron_run(job.id)
  return r.status
}
flow Main { input: String = "t" -> T -> output }
"#,
        )
        .unwrap_err();
        assert!(
            err.starts_with("IRREVERSIBLE_NO_GRANT"),
            "{name}: the foreign-target mandate must not cover cron_run, got: {err}"
        );
    }
    reset();
}

// ── The composed surface: one decision, both gates, both backends ─────

#[test]
#[serial_test::serial]
fn the_composed_authorize_call_carries_both_layers() {
    reset();
    // The capability layer alone is NOT enough for the oversight class:
    // the standard baseline passes cron_run, the approval gate refuses —
    // and a live mandate satisfies the approval layer without any
    // capability artifact (the baseline keeps its named posture).
    assert!(
        metalogos::capability::authorize_call(
            "cron_run",
            &metalogos::capability::CapabilityPermits::default()
        )
        .is_err(),
        "the baseline posture alone must not satisfy the oversight class"
    );
    metalogos::approval::mandate_issue("owner", "carrier:cron_run", 600).unwrap();
    assert!(
        metalogos::capability::authorize_call(
            "cron_run",
            &metalogos::capability::CapabilityPermits::default()
        )
        .is_ok(),
        "the live mandate satisfies the approval layer on the baseline posture"
    );
    // A carrier outside the class: the approval layer resolves trivially.
    assert!(
        metalogos::capability::authorize_call(
            "print",
            &metalogos::capability::CapabilityPermits::default()
        )
        .is_ok(),
        "print (baseline, outside the class) keeps the standard posture"
    );
    // The roster is the explicit transaction — the machine shape.
    assert_eq!(metalogos::approval::HUMAN_OVERSIGHT, &["cron_run"]);
    reset();
}
