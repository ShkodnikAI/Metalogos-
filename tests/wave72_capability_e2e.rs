#![allow(clippy::disallowed_methods)]

// ── Naryad №847 (wave 72 — W-B, P2, [testing][security]): the
// acceptance e2e of the capability contract — issue → use → revoke →
// deny, the full typed trace in the Action Ledger (ADR-0203 §2, the
// wave-3 acceptance form №395/gh#491 continued onto the capability
// layer; dispatch gh#1666) ──────────────────────────────────────────────
//
// The contract is checked NOT by unit tests but by the end-to-end
// trace: the agent's action receives a token → consumes → the token is
// revoked → the next action refuses BEFORE the run — and the whole
// trace (issue/use/revoke/deny) lies in the Ledger, readable and
// golden-pinned. Both executors (TW + VM) run the SAME scenario with
// the SAME output and the SAME ledger trail (the №829 parity lesson).

use std::path::{Path, PathBuf};

const MANIFEST: &str = env!("CARGO_MANIFEST_DIR");

const TRACE_EXAMPLE: &str = "examples/w72_capability_trace.mlog";
const TRACE_EXPECTED: &str = "examples/w72_capability_trace.expected";
const TRACE_LEDGER_GOLDEN: &str = "examples/w72_capability_trace.ledger.golden";
const NO_GRANT_EXAMPLE: &str = "examples/w72_capability_deny_no_grant.mlog";
const NO_GRANT_EXPECTED: &str = "examples/w72_capability_deny_no_grant.expected";
const EXPIRED_EXAMPLE: &str = "examples/w72_capability_expired.mlog";
const EXPIRED_EXPECTED: &str = "examples/w72_capability_expired.expected";

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source.trim(), PathBuf::from(MANIFEST))
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(PathBuf::from(MANIFEST));
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

/// The lane-runner type — the TW/VM parity harness form (№748/№829);
/// the named alias keeps the clippy type_complexity lint honest.
type LaneRunner = fn(&str) -> Result<Option<String>, String>;

fn runners() -> [(&'static str, LaneRunner); 2] {
    [("tw", run_tw), ("vm", run_vm)]
}

fn read_repo_file(rel: &str) -> String {
    std::fs::read_to_string(Path::new(MANIFEST).join(rel))
        .unwrap_or_else(|e| panic!("cannot read {rel}: {e}"))
}

/// The tmp victims live in the crate root (the fs sandbox allows the
/// relative paths); cleaned before AND after each run.
fn cleanup() {
    for name in [
        "w72_capability_trace.tmp",
        "w72_deny_no_grant.tmp",
        "w72_expired.tmp",
    ] {
        let _ = std::fs::remove_file(Path::new(MANIFEST).join(name));
    }
}

// ── The acceptance trace: issue → use → revoke → deny ──────────────────

#[test]
fn capability_trace_example_matches_the_golden_on_both_backends() {
    cleanup();
    let src = read_repo_file(TRACE_EXAMPLE);
    let expected = read_repo_file(TRACE_EXPECTED);
    for (name, runner) in runners() {
        let out = runner(&src)
            .unwrap_or_else(|e| panic!("{name}: the trace example must run, got: {e}"))
            .unwrap_or_else(|| panic!("{name}: the trace example must produce output"));
        assert_eq!(
            out.trim(),
            expected.trim(),
            "{name}: the e2e trace must match the golden fixture"
        );
    }
    // The surviving payload IS the refusal-before-action proof: the
    // denied delete never touched the file.
    let victim = Path::new(MANIFEST).join("w72_capability_trace.tmp");
    assert_eq!(
        std::fs::read_to_string(&victim).unwrap(),
        "must-survive-the-deny",
        "the denied deletion must not touch the file"
    );
    cleanup();
}

#[test]
fn capability_trace_ledger_golden_is_the_signed_trail() {
    // The №832 w1_attest anchor form: the committed golden pins the
    // ORDERED action sequence the run must journal — issue → use →
    // revoke → deny (the honest docs: the verify compares the trail
    // with the journal).
    cleanup();
    let golden = read_repo_file(TRACE_LEDGER_GOLDEN);
    let expected_actions: Vec<&str> = golden
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    // Snapshot the ledger length BEFORE the run: the trail of THIS run
    // is the tail (the in-process ledger is shared with the other tests
    // in the binary — the relative assertion is the deterministic form).
    let before = metalogos::ledger::all_records().unwrap().len();
    let src = read_repo_file(TRACE_EXAMPLE);
    run_tw(&src).expect("TW run").expect("output");
    let records = metalogos::ledger::all_records().unwrap();
    let trail: Vec<&str> = records
        .iter()
        .skip(before)
        .map(|r| r.action.as_str())
        .collect();

    // Every golden action appears IN ORDER inside the run's trail (the
    // trail may carry additional action-path records — write/read/db —
    // but the capability lifecycle must be exactly this order).
    let mut idx = 0;
    for want in &expected_actions {
        let found = trail[idx..]
            .iter()
            .position(|a| a == want)
            .unwrap_or_else(|| {
                panic!(
                    "the ledger trail must carry '{want}' AFTER the preceding golden actions; \
                     trail so far: {trail:?}"
                )
            });
        idx += found + 1;
    }
    // The full deny vocabulary stays 14 — the refusal rides the existing
    // class (ADR-0203 §2.4).
    assert_eq!(metalogos::deny::DENY_REASONS.len(), 14);
    cleanup();
}

// ── The negatives: every refusal is explained and written ──────────────

#[test]
fn no_grant_refusal_degrades_and_the_file_survives_on_both_backends() {
    cleanup();
    let src = read_repo_file(NO_GRANT_EXAMPLE);
    let expected = read_repo_file(NO_GRANT_EXPECTED);
    let mut ledger_deny = 0;
    for (name, runner) in runners() {
        let before = metalogos::ledger::all_records().unwrap().len();
        let out = runner(&src)
            .unwrap_or_else(|e| panic!("{name}: the handled refusal must not kill the flow: {e}"))
            .expect("output");
        assert_eq!(out.trim(), expected.trim(), "{name}: the negative golden");
        let records = metalogos::ledger::all_records().unwrap();
        ledger_deny += records
            .iter()
            .skip(before)
            .filter(|r| r.action == "deny.IRREVERSIBLE_NO_GRANT")
            .count();
    }
    assert!(
        ledger_deny >= 2,
        "each lane's refusal must journal deny.IRREVERSIBLE_NO_GRANT, got {ledger_deny}"
    );
    // The file survives BOTH refusals (the denial precedes the action).
    assert_eq!(
        std::fs::read_to_string(Path::new(MANIFEST).join("w72_deny_no_grant.tmp")).unwrap(),
        "must-survive"
    );
    cleanup();
}

#[test]
fn expired_token_refuses_fail_closed_on_both_backends() {
    cleanup();
    let src = read_repo_file(EXPIRED_EXAMPLE);
    let expected = read_repo_file(EXPIRED_EXPECTED);
    for (name, runner) in runners() {
        let out = runner(&src)
            .unwrap_or_else(|e| panic!("{name}: the handled refusal must not kill the flow: {e}"))
            .expect("output");
        assert_eq!(
            out.trim(),
            expected.trim(),
            "{name}: the expired token must refuse the carrier (degraded Unit)"
        );
        // The file survives — the expired token authorizes nothing.
        assert_eq!(
            std::fs::read_to_string(Path::new(MANIFEST).join("w72_expired.tmp")).unwrap(),
            "must-survive"
        );
        cleanup();
    }
}

#[test]
fn once_token_is_linear_through_the_gate_on_both_backends() {
    // A Once capability authorizes exactly ONE carrier use: the FIRST
    // delete consumes it (the gate consumes through the single consume
    // point, ADR-0203 §2.2), the SECOND refuses (the ledger is the
    // authority — copying the token duplicates nothing).
    cleanup();
    for (name, runner) in runners() {
        let out = runner(
            r#"
on_deny(file) {
  match deny_reason() {
    "IRREVERSIBLE_NO_GRANT" then { print("deny:IRREVERSIBLE_NO_GRANT") }
    else { print("deny:OTHER") }
  }
}
pattern T(_t: String) -> String {
  let cap = grant_issue("builtin:delete_file", 600, "once")
  let _w = write_file("w72_once.tmp", "once-payload")
  let d1 = delete_file("w72_once.tmp")
  let _w2 = write_file("w72_once.tmp", "must-survive")
  let d2 = delete_file("w72_once.tmp")
  let survived = read_file("w72_once.tmp")
  return "first:" + type_of(d1) + "|second:" + type_of(d2) + "|survived:" + survived
}
flow Main { input: String = "t" -> T -> output }
"#,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            out.trim(),
            "first:String|second:Unit|survived:must-survive",
            "{name}: the Once capability is linear through the gate, got: {out}"
        );
    }
    cleanup();
}

#[test]
fn the_grant_route_consumes_one_use_per_authorized_call() {
    // N(2) quota: two authorized deletes consume the quota; the THIRD
    // refuses (GRANT_EXHAUSTED → the capability refusal — the quota is
    // metered at the gate, the ledger is the SSOT).
    cleanup();
    for (name, runner) in runners() {
        let out = runner(
            r#"
on_deny(file) {
  match deny_reason() {
    "IRREVERSIBLE_NO_GRANT" then { print("deny:IRREVERSIBLE_NO_GRANT") }
    else { print("deny:OTHER") }
  }
}
pattern T(_t: String) -> String {
  let cap = grant_issue("builtin:delete_file", 600, "n", 2)
  let d1 = delete_file("w72_quota_a.tmp")
  let d2 = delete_file("w72_quota_b.tmp")
  let d3 = delete_file("w72_quota_c.tmp")
  return "d1:" + type_of(d1) + "|d2:" + type_of(d2) + "|d3:" + type_of(d3)
}
flow Main { input: String = "t" -> T -> output }
"#,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            out.trim(),
            "d1:String|d2:String|d3:Unit",
            "{name}: the N(2) quota meters at the gate, got: {out}"
        );
    }
    cleanup();
}
