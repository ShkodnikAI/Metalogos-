// ── Наряд №355 (P1, feature/embodied — ADR-0159 §2.2/§2.3, the №343
//    contract): the Proof structure — signature + validation ──────────
//
// Corpus:
//   (1) the trace carries EXACTLY the contract fields {hash(world),
//       chunk, bounds, telemetry-digest, verdict, hash(sim)} + the
//       signature over the canonical serialization;
//   (2) the stage-A seal is PENDING by construction — the monitor
//       lands with №356; no program-forged "satisfied" exists;
//   (3) verify: the fresh seal is valid with ZERO reasons;
//   (4) tamper-detection: ANY field mutation breaks the signature —
//       verify reports invalid with the named reason;
//   (5) the verdict vocabulary is closed (satisfied/violated/pending);
//   (6) the contract-field legs: bounds resolution, backend pin/class;
//   (7) the ledger: seal + verify are records (the embodied.* family).

use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn seal_a_proof() -> String {
    let dev = metalogos::embodied::device_open("t", "embodied-mock-device", None).unwrap();
    let dev_id = match &dev {
        metalogos::interpreter::values::Value::Device(m) => m.get("id").unwrap().clone(),
        _ => panic!("device"),
    };
    let dev2 = metalogos::embodied::bounds_attach("t", &dev_id, "always(|pose.velocity| <= v_max)")
        .unwrap();
    let dev2_id = match &dev2 {
        metalogos::interpreter::values::Value::Device(m) => m.get("id").unwrap().clone(),
        _ => panic!("device"),
    };
    let ws = metalogos::embodied::world_state("t", &dev2_id).unwrap();
    let ws_id = match &ws {
        metalogos::interpreter::values::Value::WorldState(m) => m.get("id").unwrap().clone(),
        _ => panic!("world"),
    };
    let pose = metalogos::embodied::pose_make("t", 0.0, 0.0, 0.0, 0.0).unwrap();
    let traj = metalogos::embodied::trajectory_make("t", std::slice::from_ref(&pose)).unwrap();
    let traj_id = match &traj {
        metalogos::interpreter::values::Value::Trajectory(m) => m.get("id").unwrap().clone(),
        _ => panic!("trajectory"),
    };
    let chunk = metalogos::embodied::chunk_make("t", &dev2_id, &traj_id, None).unwrap();
    let chunk_id = match &chunk {
        metalogos::interpreter::values::Value::ActionChunk(m) => m.get("id").unwrap().clone(),
        _ => panic!("chunk"),
    };
    let proof =
        metalogos::embodied::proof_seal("t", &ws_id, &chunk_id, Some("stage-a telemetry")).unwrap();
    match proof {
        metalogos::interpreter::values::Value::Proof(m) => m.get("id").unwrap().clone(),
        _ => panic!("proof"),
    }
}

fn report_field(
    report: &metalogos::interpreter::values::Value,
    name: &str,
) -> metalogos::interpreter::values::Value {
    match report {
        metalogos::interpreter::values::Value::Struct { fields, .. } => fields
            .get(name)
            .cloned()
            .unwrap_or_else(|| panic!("no field {}", name)),
        other => panic!("expected Struct, got {}", other.type_name()),
    }
}

fn as_bool(v: &metalogos::interpreter::values::Value) -> bool {
    match v {
        metalogos::interpreter::values::Value::Bool(b) => *b,
        other => panic!("expected Bool, got {}", other.type_name()),
    }
}

fn as_string(v: &metalogos::interpreter::values::Value) -> String {
    match v {
        metalogos::interpreter::values::Value::String(s) => s.clone(),
        other => panic!("expected String, got {}", other.type_name()),
    }
}

fn as_strings(v: &metalogos::interpreter::values::Value) -> Vec<String> {
    match v {
        metalogos::interpreter::values::Value::List(items) => items
            .iter()
            .map(|i| match i {
                metalogos::interpreter::values::Value::String(s) => s.clone(),
                other => panic!("expected String item, got {}", other.type_name()),
            })
            .collect(),
        other => panic!("expected List, got {}", other.type_name()),
    }
}

// ── (1) The contract fields ─────────────────────────────────────────────

#[test]
fn proof_record_carries_exactly_the_contract_fields() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let id = seal_a_proof();
    let rec = metalogos::embodied::proof_record(&id).expect("the record resolves");
    // hash(world): a sha256 hex digest.
    assert_eq!(rec.world_hash.len(), 64);
    assert!(rec.world_hash.chars().all(|c| c.is_ascii_hexdigit()));
    // chunk + bounds: resolvable ids.
    assert!(rec.chunk_id.starts_with("chk-"));
    assert!(rec.bounds_id.starts_with("bnd-"));
    // telemetry: a DIGEST (the verbatim payload never enters the record
    // — the №415 posture).
    assert_eq!(
        rec.telemetry_digest,
        metalogos::ledger::sha256_hex(b"stage-a telemetry"),
        "the telemetry leg is the sha256 of the payload"
    );
    // hash(sim): a sha256 hex digest.
    assert_eq!(rec.sim_hash.len(), 64);
    // The signature covers the canonical serialization (non-empty).
    assert_eq!(rec.signature.len(), 64);
}

// ── (2) The stage-A seal is Pending by construction ─────────────────────

#[test]
fn seal_verdict_is_pending_no_forged_satisfied() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let id = seal_a_proof();
    let rec = metalogos::embodied::proof_record(&id).unwrap();
    assert_eq!(rec.verdict, metalogos::embodied::Verdict::Pending);
    // The closed vocabulary: a program cannot mint "satisfied" — there
    // is no surface that seals a verdict other than Pending.
    assert_eq!(
        metalogos::embodied::Verdict::parse("satisfied"),
        Some(metalogos::embodied::Verdict::Satisfied),
        "the word exists (the monitor's vocabulary, №356)"
    );
    assert_eq!(metalogos::embodied::Verdict::parse("SATISFIED"), None);
    assert_eq!(metalogos::embodied::Verdict::parse("unknown"), None);
}

// ── (3) verify: the fresh seal is valid ─────────────────────────────────

#[test]
fn fresh_seal_verifies_with_zero_reasons() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let id = seal_a_proof();
    let report = metalogos::embodied::proof_verify("proof_verify", &id).unwrap();
    assert!(as_bool(&report_field(&report, "valid")));
    assert_eq!(as_string(&report_field(&report, "verdict")), "pending");
    assert_eq!(as_string(&report_field(&report, "proof_id")), id);
    assert_eq!(
        as_string(&report_field(&report, "backend")),
        "embodied-mock-device"
    );
    let reasons = as_strings(&report_field(&report, "reasons"));
    assert!(
        reasons.is_empty(),
        "a fresh seal carries no reasons: {:?}",
        reasons
    );
}

// ── (4) Tamper-detection: any mutation breaks the signature ─────────────

#[test]
fn tampered_world_hash_is_reported_invalid() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let id = seal_a_proof();
    let mut rec = metalogos::embodied::proof_record(&id).unwrap();
    rec.world_hash = "f".repeat(64); // the tampered leg
    let reasons = metalogos::embodied::validate_proof_record(&rec, &locked_state());
    assert!(
        reasons
            .iter()
            .any(|r| r.contains("signature does not match")),
        "the tamper reason: {:?}",
        reasons
    );
}

#[test]
fn tampered_sim_hash_is_reported_invalid() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let id = seal_a_proof();
    let mut rec = metalogos::embodied::proof_record(&id).unwrap();
    rec.sim_hash = "e".repeat(64);
    let reasons = metalogos::embodied::validate_proof_record(&rec, &locked_state());
    assert!(
        reasons
            .iter()
            .any(|r| r.contains("signature does not match")),
        "{:?}",
        reasons
    );
}

#[test]
fn foreign_bounds_is_reported() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let id = seal_a_proof();
    let mut rec = metalogos::embodied::proof_record(&id).unwrap();
    // A forged-but-consistently-signed trace over a foreign bounds id:
    // the signature is recomputed AFTER the mutation, so the integrity
    // leg passes — the RESOLUTION leg must still catch it.
    rec.bounds_id = "bnd-nonexistent000000".to_string();
    rec.sign();
    assert!(
        rec.signature_valid(),
        "the test isolates the resolution leg"
    );
    let reasons = metalogos::embodied::validate_proof_record(&rec, &locked_state());
    assert!(
        reasons
            .iter()
            .any(|r| r.contains("does not resolve to any device-profile formula")),
        "{:?}",
        reasons
    );
}

#[test]
fn foreign_chunk_is_reported() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let id = seal_a_proof();
    let mut rec = metalogos::embodied::proof_record(&id).unwrap();
    rec.chunk_id = "chk-nonexistent0000000".to_string();
    rec.sign();
    let reasons = metalogos::embodied::validate_proof_record(&rec, &locked_state());
    assert!(
        reasons.iter().any(|r| r.contains("chunk does not resolve")),
        "{:?}",
        reasons
    );
}

#[test]
fn malformed_world_hash_is_reported() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let id = seal_a_proof();
    let mut rec = metalogos::embodied::proof_record(&id).unwrap();
    rec.world_hash = "not-a-hash".to_string();
    rec.sign();
    let reasons = metalogos::embodied::validate_proof_record(&rec, &locked_state());
    assert!(
        reasons
            .iter()
            .any(|r| r.contains("not a sha256 hex digest")),
        "{:?}",
        reasons
    );
}

// ── (5) The insert-seam proof: an unknown handle refuses typed ──────────

#[test]
fn verify_unknown_proof_refuses_typed() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let err = metalogos::embodied::proof_verify("proof_verify", "prf-never-sealed").unwrap_err();
    assert!(err.starts_with("[EMBODIED_HANDLE_UNKNOWN]"), "got: {}", err);
}

// ── (6) The ledger legs ─────────────────────────────────────────────────

#[test]
fn seal_and_verify_are_ledger_records() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let id = seal_a_proof();
    let _ = metalogos::embodied::proof_verify("proof_verify", &id).unwrap();
    let records = metalogos::ledger::all_records().unwrap_or_default();
    assert!(
        records
            .iter()
            .any(|r| r.action == "embodied.proof_seal" && r.actor == id),
        "the seal leg"
    );
    assert!(
        records
            .iter()
            .any(|r| r.action == "embodied.proof_verify" && r.actor == id),
        "the verify leg"
    );
}

// ── the locked-state helper (the std Mutex is not reentrant — the
//    validation takes the ALREADY-LOCKED state) ────────────────────────

fn locked_state() -> std::sync::MutexGuard<'static, metalogos::embodied::EmbodiedState> {
    // The state accessor is private; the test path goes through the
    // public registry lock via a fresh proof_verify round-trip. To keep
    // the tamper corpus at the data level, the validation re-uses the
    // same record set — resolve the state through the public seam.
    metalogos::embodied::state_for_tests()
}
