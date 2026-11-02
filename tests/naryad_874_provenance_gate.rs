#![allow(clippy::disallowed_methods)]

// ── Naryad №874 (wave 77 — W-F, P1, [provenance][core]): the coverage
// rule — the fail-closed I/O gate on the provenance carriers + the e2e
// trace (ADR-0207 §2.5, dispatch gh#1729) ───────────────────────────────
//
// The rule: an EGRESS carrier (write_file / http_post / respond /
// print) without a live boundary record refuses BEFORE the side
// effect; an INGRESS carrier's value (read_file / http_get) without a
// record refuses ON USE — «молча пропустить» is forbidden (the
// №584/№601 posture). The refusal rides the FROZEN SINK_CLEARANCE
// class (№385) with the typed «provenance missing» detail, and
// journals through the EXISTING Ledger ACTION path (the fire_on_deny
// form of ADR-0167 §3.4 — no new record kind).
//
// The e2e trace: action → ingress (recorded) → processing → egress
// (recorded) → manifest — on BOTH backends (TW/VM parity — the №829
// lesson). The negative controls: (а) the egress without a record →
// the refusal + the Ledger ribbon carries «provenance missing»; (б)
// the legit path with the full provenance passes WITHOUT degradation
// — the bytes land byte-for-byte and the manifest serialization is
// byte-identical across two runs (the G5 form of №828).

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

// ── The e2e trace: action → ingress → processing → egress → manifest ───

#[test]
#[serial_test::serial]
fn the_full_trace_records_ingress_and_egress_on_both_lanes() {
    metalogos::provenance::store_reset();

    let dir = std::path::Path::new(MANIFEST)
        .join("target")
        .join("n874_e2e");
    std::fs::create_dir_all(&dir).unwrap();
    let in_path = dir.join("in.txt");
    let out_path = dir.join("out.txt");
    let in_content = "the wave-77 gate trace: вход → обработка → выход → манифест\n";
    std::fs::write(&in_path, in_content).unwrap();
    let _ = std::fs::remove_file(&out_path);

    let program = r#"
pattern T(_t: String) -> String {
  let c = read_file("target/n874_e2e/in.txt")
  let n = c + "!"
  let w = write_file("target/n874_e2e/out.txt", n)
  print("n874 gate marker")
  let r = respond("200", "n874 respond body")
  return w
}
flow Main { input: String = "t" -> T -> output }
"#;

    for (lane, runner) in runners() {
        // (б) the legit path passes WITHOUT degradation: the write
        // lands, the print runs, the respond value exists.
        let out = runner(program).unwrap().unwrap();
        assert!(
            out.contains("ok"),
            "{lane}: the legit path passes under the fail-closed gate, got: {out}"
        );
        // the written bytes match the derived content byte-for-byte
        let expected = format!("{}!", in_content);
        let landed = std::fs::read(&out_path).unwrap();
        assert_eq!(
            landed,
            expected.as_bytes(),
            "{lane}: the egress bytes are byte-for-byte (no degradation)"
        );
    }

    // the manifest: the ingress row pins the exact input bytes; the
    // egress rows pin the exact output bytes (the same digests both
    // lanes produced)
    let snap = metalogos::provenance::snapshot();
    let in_digest = metalogos::provenance::origin_digest(in_content.as_bytes());
    let out_digest = metalogos::provenance::origin_digest(format!("{}!", in_content).as_bytes());
    assert!(
        snap.rows().iter().any(|r| r.carrier == "read_file"
            && r.origin_digest == in_digest
            && r.direction == metalogos::provenance::Direction::Ingress),
        "the ingress crossing is recorded"
    );
    assert!(
        snap.rows().iter().any(|r| r.carrier == "write_file"
            && r.origin_digest == out_digest
            && r.direction == metalogos::provenance::Direction::Egress),
        "the egress crossing is recorded"
    );
    let marker_digest = metalogos::provenance::origin_digest(b"n874 gate marker");
    assert!(
        snap.rows()
            .iter()
            .any(|r| r.carrier == "print" && r.origin_digest == marker_digest),
        "the print egress is recorded"
    );
    let resp_digest = metalogos::provenance::origin_digest(b"n874 respond body");
    assert!(
        snap.rows()
            .iter()
            .any(|r| r.carrier == "respond" && r.origin_digest == resp_digest),
        "the respond egress is recorded"
    );

    // the use-site rule (the ingress side of the coverage rule): the
    // recorded origin passes, the foreign digest refuses ON USE
    assert!(metalogos::provenance::verify_origin(&in_digest, in_content.as_bytes()).is_ok());
    let foreign = metalogos::provenance::origin_digest(b"never-crossed-bytes");
    let err = metalogos::provenance::require(&foreign).unwrap_err();
    assert!(
        err.starts_with("SINK_CLEARANCE") && err.contains("provenance missing"),
        "the use of an unrecorded origin refuses ON USE (fail-closed), got: {err}"
    );
    metalogos::provenance::store_reset();
}

// ── Negative control (а): no record → the refusal + the Ledger ribbon ──

#[test]
#[serial_test::serial]
fn the_egress_without_a_record_refuses_and_journals() {
    metalogos::provenance::store_reset();

    // The gate seam: a boundary record that cannot land (the forged
    // carrier — the registry is the closed vocabulary) refuses the
    // side effect LOUDLY. The refusal carries the frozen class word
    // and the typed «provenance missing» detail.
    let err = metalogos::provenance::gate_emit("forged_carrier", b"bytes").unwrap_err();
    assert!(
        err.starts_with("SINK_CLEARANCE"),
        "the refusal rides the frozen SINK_CLEARANCE class, got: {err}"
    );
    assert!(
        err.contains("provenance missing"),
        "the refusal names «provenance missing», got: {err}"
    );

    // the Ledger ribbon: the refusal journaled through the EXISTING
    // ACTION path — action `deny.SINK_CLEARANCE`, scope `provenance`
    // (no new record kind, the №393 form)
    let records = metalogos::ledger::all_records().unwrap_or_default();
    let row = records
        .iter()
        .find(|r| r.action == "deny.SINK_CLEARANCE" && r.scope == "provenance")
        .expect("the refusal must be visible in the Ledger ribbon");
    assert_eq!(row.kind, "action");
    assert!(
        !row.args_hash.is_empty(),
        "the typed detail rides the hashed args"
    );

    // the use-site rule refuses a never-crossed digest (the ingress
    // side of the rule — the same frozen class, «provenance missing»)
    let foreign = metalogos::provenance::origin_digest(b"no-ingress-here");
    let err = metalogos::provenance::require(&foreign).unwrap_err();
    assert!(err.starts_with("SINK_CLEARANCE") && err.contains("provenance missing"));

    // the substitution control (the G5 form of №828): the bytes
    // changed between the crossing and the use — the verify refuses
    // even with the row present
    let bytes = b"the pinned payload";
    let m = metalogos::provenance::ingest("read_file", bytes).unwrap();
    assert!(m.verify_bytes(bytes).is_ok());
    let err = m.verify_bytes(b"the tampered payload").unwrap_err();
    assert!(err.contains("digest mismatch"), "got: {err}");
    metalogos::provenance::store_reset();
}

// ── Negative control (б): the legit path + the determinism contract ────

#[test]
#[serial_test::serial]
fn the_legit_path_records_without_degradation_and_is_deterministic() {
    metalogos::provenance::store_reset();

    // run one: the record lands, the manifest pins the bytes
    let bytes = b"the determinism payload";
    let m1 = metalogos::provenance::gate_emit("write_file", bytes).unwrap();
    assert_eq!(m1.carrier, "write_file");
    assert_eq!(m1.direction, metalogos::provenance::Direction::Egress);
    assert!(metalogos::provenance::verify_origin(&m1.origin_digest, bytes).is_ok());

    // run two (after the reset — the same starting state): the SAME
    // serialization — the double-run byte-identity (the G5 form of
    // №828; the dispatch acceptance line)
    metalogos::provenance::store_reset();
    let m2 = metalogos::provenance::gate_emit("write_file", bytes).unwrap();
    assert_eq!(
        m1.record_digest, m2.record_digest,
        "the double run is byte-identical"
    );
    assert_eq!(
        m1.revision, m2.revision,
        "the reset restores the revision sequence"
    );

    // the Ledger stayed silent on the happy path (the
    // record-on-refusal-only posture — the golden traces are
    // untouched). The ledger is process-global and keeps the earlier
    // negative-control rows, so the honest gate is the DELTA: no NEW
    // provenance-flavored rows during this test.
    let provenance_rows_before = metalogos::ledger::all_records()
        .unwrap_or_default()
        .iter()
        .filter(|r| r.action.contains("provenance") || r.scope.contains("provenance"))
        .count() as u64;
    let m3 = metalogos::provenance::gate_emit("write_file", bytes).unwrap();
    assert_ne!(m2.revision, m3.revision);
    assert_ne!(m2.record_digest, m3.record_digest);
    let snap = metalogos::provenance::snapshot();
    assert_eq!(snap.for_origin(&m3.origin_digest).len(), 2);
    let records = metalogos::ledger::all_records().unwrap_or_default();
    let provenance_rows_after = records
        .iter()
        .filter(|r| r.action.contains("provenance") || r.scope.contains("provenance"))
        .count() as u64;
    assert_eq!(
        provenance_rows_before, provenance_rows_after,
        "the legit path must not journal NEW provenance rows (the record-on-refusal-only posture)"
    );
    // the journaled rows (from the negative-control test in this
    // process) are exactly the deny class — never a happy-path kind
    assert!(
        records
            .iter()
            .filter(|r| r.scope.contains("provenance"))
            .all(|r| r.action == "deny.SINK_CLEARANCE"),
        "the only journaled provenance rows are the fail-closed refusals"
    );
    metalogos::provenance::store_reset();
}
