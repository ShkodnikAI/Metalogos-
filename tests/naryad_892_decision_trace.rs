// ── Naryad №892 (wave 80 — W-I, 8G/C8, ADR-0210): the decision-trace ──
// contract tests. The kernel negatives live in the module
// (`src/decision_trace.rs::kernel_tests`, the №867/№886 form); this file
// exercises the PUBLIC consumption surface end-to-end: the trace built
// → journaled through the LIVE Ledger ACTION contour (ADR-0167) → the
// REPLAY_MATCH reconciliation against the append-only log — plus the
// byte-in-byte serde double run and the digest stability across the
// round trip.

use metalogos::decision_trace::{
    canonical_trace, canonical_trace_digest, record_trace, verify_replay, DecisionCarrier,
    DecisionTrace, DecisionTraceLog,
};
use metalogos::ledger;

const IN: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const OUT: &str = "2222222222222222222222222222222222222222222222222222222222222222";
const OUT2: &str = "3333333333333333333333333333333333333333333333333333333333333333";

fn traced_call() -> DecisionTrace {
    DecisionTrace::new(
        "invoke_db_execute_with_grant",
        DecisionCarrier::Local,
        &[IN.to_string()],
        "",
        "",
        "",
        &["grant-0001".to_string()],
        &[],
        &[],
        &[OUT.to_string()],
        1_700_000_001,
    )
    .expect("a valid local trace")
}

fn llm_call() -> DecisionTrace {
    DecisionTrace::new(
        "llm_decide",
        DecisionCarrier::Language,
        &[IN.to_string()],
        "local",
        "mock-1.0",
        "provider-2026.11",
        &[],
        &[],
        &["Degraded(nn)".to_string()],
        &[OUT.to_string()],
        1_700_000_002,
    )
    .expect("a valid language trace")
}

#[test]
fn trace_types_digest_stability_and_the_no_raw_payload_boundary() {
    let a = traced_call();
    let b = traced_call();
    assert_eq!(a.trace_digest, b.trace_digest, "same fields — same digest");
    assert_eq!(canonical_trace(&a), canonical_trace(&b));
    assert_eq!(
        a.trace_digest,
        canonical_trace_digest(&a),
        "the stored digest IS the canonical digest"
    );
    // The privacy boundary is the constructor's contract: a raw
    // payload refuses at the trace construction, not at the audit.
    let err = DecisionTrace::new(
        "x",
        DecisionCarrier::Local,
        &[OUT.to_string()],
        "",
        "",
        "",
        &[],
        &[],
        &[],
        &["the raw output payload".to_string()],
        1,
    )
    .expect_err("a raw output is not a digest");
    assert!(err.contains("never enter the trace"), "{err}");
}

#[test]
fn language_carrier_must_carry_the_contract_line() {
    // The №879 version line is mandatory exactly where a model was
    // consumed — the Local carrier does not need it, the Language one
    // refuses without it.
    let ok = llm_call();
    assert_eq!(ok.carrier, DecisionCarrier::Language);
    assert!(!ok.model_version.is_empty());
    let err = DecisionTrace::new(
        "llm_decide",
        DecisionCarrier::Language,
        &[IN.to_string()],
        "local",
        "mock-1.0",
        "",
        &[],
        &[],
        &[],
        &[OUT.to_string()],
        1,
    )
    .expect_err("no provider version");
    assert!(err.contains("model/provider versions"), "{err}");
}

#[test]
fn log_append_only_and_supersede_semantics() {
    let mut log = DecisionTraceLog::new();
    let t1 = traced_call();
    let idx = log.append(t1.clone()).expect("first append");
    assert_eq!(idx, 0);
    let err = log
        .append(t1.clone())
        .expect_err("the exact duplicate refuses");
    assert!(err.contains("append-only, no duplicates"), "{err}");
    // The re-traced decision (a changed output) supersedes as a NEW row.
    let t2 = DecisionTrace::new(
        "invoke_db_execute_with_grant",
        DecisionCarrier::Local,
        &[IN.to_string()],
        "",
        "",
        "",
        &["grant-0001".to_string()],
        &[],
        &[],
        &[OUT2.to_string()],
        1_700_000_003,
    )
    .expect("the re-traced decision");
    assert_ne!(
        t1.trace_digest, t2.trace_digest,
        "a changed output re-digests"
    );
    assert_eq!(log.append(t2).expect("the supersede append"), 1);
    assert_eq!(log.len(), 2, "nothing was overwritten");
    assert_eq!(log.rows_for("invoke_db_execute_with_grant").len(), 2);
}

#[test]
fn verify_replay_matches_then_refuses_the_three_negative_paths() {
    let mut log = DecisionTraceLog::new();
    let t = traced_call();
    log.append(t.clone()).expect("append");

    // The positive reconciliation: the exact recorded trace matches,
    // twice, byte-identically.
    let row = verify_replay(&t, &log).expect("match");
    assert_eq!(row.trace_digest, t.trace_digest);
    let row2 = verify_replay(&t, &log).expect("match again");
    assert_eq!(row.trace_digest, row2.trace_digest);

    // Negative 1 — a decision with NO recorded trace: the fail-closed
    // coverage refusal (the composition №893 turns it into the
    // pre-effect deny).
    let untraced = llm_call();
    let err = verify_replay(&untraced, &log).expect_err("no recorded trace");
    assert!(
        err.contains("no recorded trace covers decision 'llm_decide'"),
        "{err}"
    );
    assert!(err.contains("SINK_CLEARANCE"), "{err}");

    // Negative 2 — a claimed replay of a recorded decision with a
    // TAMPERED output: the reconciliation names the field and refuses
    // with the frozen class.
    let mut tampered = traced_call();
    tampered.outputs = vec![OUT2.to_string()];
    tampered.ts += 1;
    tampered.trace_digest = canonical_trace_digest(&tampered);
    let err = verify_replay(&tampered, &log).expect_err("the tampered output");
    assert!(
        err.contains("diverges from the recorded trace on outputs"),
        "{err}"
    );
    assert!(err.contains("UNTRUSTED_EXEC_DECISION"), "{err}");

    // Negative 3 — a RECORDED row whose stored digest was swapped for
    // a foreign value after the write: the record-side re-derivation
    // refuses before any reconciliation (the log is tampered through
    // its serde surface — the rows field is private by design).
    let json = serde_json::to_string(&log).expect("serialize the log");
    let mut value: serde_json::Value = serde_json::from_str(&json).expect("parse the log");
    value["rows"][0]["trace_digest"] = serde_json::Value::String(OUT2.to_string());
    let tampered_log: DecisionTraceLog = serde_json::from_value(value).expect("the tampered log");
    let err = verify_replay(&t, &tampered_log).expect_err("the altered record");
    assert!(err.contains("the recorded trace"), "{err}");
    assert!(err.contains("does not re-derive its digest"), "{err}");
    assert!(err.contains("UNTRUSTED_EXEC_DECISION"), "{err}");
}

#[test]
fn trace_journal_rides_the_live_ledger_action_contour() {
    // The ADR-0167 consumption: the trace append journals ONE Ledger
    // action row through the live `ledger::record` API — the existing
    // contour, never a parallel store.
    let t = llm_call();
    record_trace("wave-80-e2e", "decision-trace", &t);
    let rows = ledger::all_records().expect("the ledger reads");
    let hit = rows
        .iter()
        .find(|r| {
            r.action == "decision_trace" && r.args_hash == ledger::args_hash_of(&t.trace_digest)
        })
        .expect("the trace row is in the Ledger");
    assert_eq!(hit.kind, "action");
    assert_eq!(hit.scope, "decision-trace");
    assert_eq!(hit.actor, "wave-80-e2e");
    // The preimage never enters the journal — only the digest line.
    assert!(
        !serde_json::to_string(hit).unwrap().contains("mock-1.0"),
        "the version payload does not leak into the journal row"
    );
}

#[test]
fn serde_double_run_is_byte_in_byte_and_verify_survives_the_round_trip() {
    let t = llm_call();
    let mut log = DecisionTraceLog::new();
    log.append(t.clone()).expect("append");

    let s1 = serde_json::to_string(&log).expect("serialize 1");
    let back: DecisionTraceLog = serde_json::from_str(&s1).expect("deserialize");
    let s2 = serde_json::to_string(&back).expect("serialize 2");
    assert_eq!(s1, s2, "the serde double run is byte-in-byte");
    assert_eq!(log.digest(), back.digest(), "the log digest survives");

    // The REPLAY_MATCH verdict is identical against the in-memory log
    // and its serde round trip (the deterministic reconciliation).
    let v1 = verify_replay(&t, &log).expect("match in memory");
    let v2 = verify_replay(&t, &back).expect("match after the round trip");
    assert_eq!(v1.trace_digest, v2.trace_digest);
}
