#![allow(clippy::disallowed_methods)]

// ── Naryad №893 (wave 80 — W-I, P1, [replay][core]): the coverage rule
// — the fail-closed decision-trace gate + the e2e trace (ADR-0210 §4,
// dispatch gh#1779) ──────────────────────────────────────────────────────
//
// The rule (the fire_on_deny form of №874, the №845 rule): an
// irreversible decision without a live, verify-passing DecisionTrace
// refuses BEFORE the effect and journals the Ledger deny row through
// the EXISTING action path (action `deny.SINK_CLEARANCE`, scope
// `decision-trace` — no new record kind, the frozen №385 vocabulary
// does not grow). «Молча пропустить» is impossible (the №845/№887
// form): the only Ok exit of the gate IS the verified recorded row.
//
// The e2e trace on BOTH execution roads (the №847/№880 form — the
// carrier passes through the language): the same .mlog program runs on
// TW and VM — the two roads are byte-identical; the run of each road
// is traced (the source digest in, the output digest out), the trace
// is journaled through the live Ledger ACTION contour, the gate
// reconciles the claim against the log; the double run is
// byte-in-byte (the №880 acceptance form).
//
// The negative controls: (а) the decision without a trace refuses and
// journals; (б) the valid path passes without degradation — the
// outputs are byte-identical across the roads and the runs; (в) the
// verify catches the swapped model version and the swapped output;
// (г) the journaled refusal rows carry the EXISTING classes only.

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

fn sha(s: &str) -> String {
    metalogos::ledger::args_hash_of(s)
}

const PROGRAM: &str = r#"
pattern T(t: String) -> String {
  let n = t + "-w893"
  return n
}
flow Main { input: String = "trace" -> T -> output }
"#;

use metalogos::decision_trace::{replay_gate, DecisionCarrier, DecisionTrace, DecisionTraceLog};

fn trace_of_run(decision: &str, source_digest: &str, output: &str) -> DecisionTrace {
    DecisionTrace::new(
        decision,
        DecisionCarrier::Local,
        &[source_digest.to_string()],
        "",
        "",
        "",
        &[],
        &[],
        &[],
        &[sha(output)],
        1_700_000_010,
    )
    .expect("the run trace is constructible")
}

/// The decision literal per lane — the QUOTED literals are the machine
/// evidence of the gate_041 wiring (№894: the count is CHECKED, not
/// declared — the registry row counts on the quoted decision name in
/// this source).
fn lane_decision(lane: &str) -> &'static str {
    match lane {
        "tw" => "run_program@tw",
        "vm" => "run_program@vm",
        other => panic!("unknown lane {other}"),
    }
}

// ── e2e: action → record → verify → verdict on BOTH roads ──────────────

#[test]
#[serial_test::serial]
fn the_traced_run_passes_the_gate_on_both_lanes_byte_in_byte() {
    let source_digest = sha(PROGRAM);

    // run one on both roads: the outputs are byte-identical across the
    // lanes (the №880 parity), each road's run is traced, the gate
    // reconciles the claim against the recorded log.
    let mut log = DecisionTraceLog::new();
    let mut first_pass = Vec::new();
    for (lane, runner) in runners() {
        let out = runner(PROGRAM).unwrap().expect("the run output");
        let t = trace_of_run(lane_decision(lane), &source_digest, &out);
        log.append(t.clone()).expect("the run trace appends");
        metalogos::decision_trace::record_trace("wave-80-e2e", "decision-trace", &t);
        let row = replay_gate(lane_decision(lane), Some(&t), &log)
            .expect("the traced run passes the gate");
        first_pass.push((lane.to_string(), out, row.trace_digest.clone()));
    }

    // the two roads agree byte-in-byte (the №880 form)
    assert_eq!(first_pass[0].1, first_pass[1].1, "TW/VM byte-in-byte");

    // run two (a fresh log — the same starting state): the SAME
    // digests — the double run is byte-in-byte, the gate verdict is
    // deterministic.
    let mut log2 = DecisionTraceLog::new();
    for (lane, runner) in runners() {
        let out = runner(PROGRAM).unwrap().expect("the run output");
        let t = trace_of_run(lane_decision(lane), &source_digest, &out);
        log2.append(t.clone()).expect("append");
        let row =
            replay_gate(lane_decision(lane), Some(&t), &log2).expect("the second pass passes");
        let first = first_pass
            .iter()
            .find(|(l, _, _)| l == lane)
            .expect("the lane row");
        assert_eq!(
            first.2, row.trace_digest,
            "{lane}: the double run is byte-in-byte"
        );
        assert_eq!(first.1, out, "{lane}: the outputs are deterministic");
    }

    // the Ledger ribbon carries the journaled trace rows (the ACTION
    // contour) — and NO deny row was needed on the happy path
    let rows = metalogos::ledger::all_records().unwrap_or_default();
    assert!(
        rows.iter()
            .any(|r| r.action == "decision_trace" && r.scope == "decision-trace"),
        "the trace row is in the Ledger"
    );
}

// ── Negative control (а): no trace → the refusal + the Ledger ribbon ────

#[test]
#[serial_test::serial]
fn the_uncovered_decision_refuses_before_the_effect_and_journals() {
    let log = DecisionTraceLog::new();

    // the gate refuses the trace-less irreversible decision BEFORE the
    // effect — the frozen generic class, the decision named
    let err = replay_gate("run_program@tw", None, &log).unwrap_err();
    assert!(err.contains("refuses BEFORE the effect"), "got: {err}");
    assert!(err.contains("SINK_CLEARANCE"), "got: {err}");
    assert!(err.contains("run_program@tw"), "got: {err}");

    // the Ledger ribbon: the refusal journaled through the EXISTING
    // action path — action `deny.SINK_CLEARANCE`, scope
    // `decision-trace` (no new record kind, the №393/№874 form)
    let records = metalogos::ledger::all_records().unwrap_or_default();
    let row = records
        .iter()
        .find(|r| {
            r.action == "deny.SINK_CLEARANCE"
                && r.scope == "decision-trace"
                && r.args_hash == metalogos::ledger::args_hash_of(&err)
        })
        .expect(
            "THIS refusal must be visible in the Ledger ribbon (the args hash pins the exact row)",
        );
    assert_eq!(row.kind, "action");
    assert_eq!(row.args_hash, metalogos::ledger::args_hash_of(&err));

    // a traced-but-diverging decision refuses too (the gate consumes
    // the №892 reconciliation verbatim)
    let out = run_tw(PROGRAM).unwrap().unwrap();
    let mut log2 = DecisionTraceLog::new();
    let t = trace_of_run("run_program@tw", &sha(PROGRAM), &out);
    log2.append(t).expect("append");
    let mut foreign = trace_of_run("run_program@tw", &sha(PROGRAM), &out);
    foreign.model_version = "other-model-9".to_string();
    foreign.carrier = DecisionCarrier::Language;
    foreign.contract_name = "other".to_string();
    foreign.provider_version = "other-1".to_string();
    foreign.trace_digest = metalogos::decision_trace::canonical_trace_digest(&foreign);
    let err = replay_gate("run_program@tw", Some(&foreign), &log2).unwrap_err();
    assert!(
        err.contains("diverges from the recorded trace on"),
        "the divergence names the field, got: {err}"
    );
}

// ── Negative controls (б)(в)(г): the valid path is clean, the tamper
// is caught, the classes stay frozen ─────────────────────────────────────

#[test]
#[serial_test::serial]
fn the_gate_catches_the_swaps_and_the_classes_stay_frozen() {
    let source_digest = sha(PROGRAM);
    let out = run_vm(PROGRAM).unwrap().unwrap();

    // the deny-row count before this control (the ledger is
    // process-global — the honest gate is the delta)
    let deny_rows = || {
        metalogos::ledger::all_records()
            .unwrap_or_default()
            .iter()
            .filter(|r| r.scope == "decision-trace")
            .count()
    };
    let before = deny_rows();

    let mut log = DecisionTraceLog::new();
    let t = trace_of_run("run_program@vm", &source_digest, &out);
    log.append(t.clone()).expect("append");

    // (в) the swapped MODEL VERSION on the language carrier: the honest
    // claim re-digests, the reconciliation names contract-line
    let version_swap = DecisionTrace::new(
        "run_program@vm",
        DecisionCarrier::Language,
        &[source_digest],
        "local",
        "mock-1.0",
        "provider-2026.11",
        &[],
        &[],
        &[],
        &[sha(&out)],
        1_700_000_011,
    )
    .expect("the language claim is constructible");
    log.append(version_swap.clone())
        .expect("append the language row");
    let mut claim = version_swap.clone();
    claim.model_version = "mock-9.9".to_string();
    claim.trace_digest = metalogos::decision_trace::canonical_trace_digest(&claim);
    let err = replay_gate("run_program@vm", Some(&claim), &log).unwrap_err();
    assert!(
        err.contains("contract-line"),
        "the version swap is named, got: {err}"
    );
    assert!(err.contains("UNTRUSTED_EXEC_DECISION"), "got: {err}");

    // (в) the swapped OUTPUT (the claim against the LATEST row — the
    // language one): the reconciliation names outputs
    let mut output_swap = version_swap.clone();
    output_swap.outputs = vec![sha("never-produced-bytes")];
    output_swap.trace_digest = metalogos::decision_trace::canonical_trace_digest(&output_swap);
    let err = replay_gate("run_program@vm", Some(&output_swap), &log).unwrap_err();
    assert!(
        err.contains("on outputs"),
        "the output swap is named, got: {err}"
    );

    // (б) the valid path still passes without degradation — the same
    // claim that matches the record reconciles byte-in-byte
    let row = replay_gate("run_program@vm", Some(&t), &log).expect("the valid claim passes");
    assert_eq!(row.trace_digest, t.trace_digest);

    // (г) the journaled rows of this contour carry the EXISTING
    // classes only — deny.SINK_CLEARANCE for the refusals, nothing new
    let rows = metalogos::ledger::all_records().unwrap_or_default();
    assert!(
        rows.iter()
            .filter(|r| r.action.starts_with("deny.") && r.scope == "decision-trace")
            .all(|r| r.action == "deny.SINK_CLEARANCE"),
        "the only deny rows are the frozen generic class"
    );
    assert!(
        deny_rows() > before,
        "every refused gate above journaled its deny row"
    );
}
