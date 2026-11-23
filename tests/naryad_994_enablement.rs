//! №994 (ADR-0200 W6, the dispatch wave В101 — W-AE) — the differential
//! enablement gate leak-suite: the `unfiltered` refusal (decision 5's
//! boundary, the field/reason named), the №530-atomic calibration flip,
//! the null-with-reason input discipline (the E3/E4 artifacts only, the
//! mock's 0.0 not read as one), the lifecycle interlock (Frozen refuses
//! like any distill move) and the `--trace-distill` byte surface.
//!
//! The W6 lane's scope discipline: the verify path is untouched (the W1
//! never-cut lane — decision 5's `ok: true` semantics stay as-is); the
//! gate governs the ENABLEMENT, not the verify.

use metalogos::distill_hub::{enable_differential, ensure_distill_store};
use metalogos::nn::attest::{current_cert, AttestInput, AttestSpecInput, CODE_NONE};
use metalogos::nn::{persist, ActivationKind, Dense, ReflexModel, ReflexRegistry};
use serial_test::serial;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

// ── fixtures ─────────────────────────────────────────────────────────

type AuditSink = (Arc<dyn Fn(String) + Send + Sync>, Arc<Mutex<Vec<String>>>);

#[allow(clippy::disallowed_methods)]
fn make_sink() -> AuditSink {
    let log = Arc::new(Mutex::new(Vec::new()));
    let l2 = Arc::clone(&log);
    let sink: Arc<dyn Fn(String) + Send + Sync> =
        Arc::new(move |line: String| l2.lock().unwrap().push(line));
    (sink, log)
}

#[allow(clippy::disallowed_methods)]
fn open_store(path: &Path) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open(path).unwrap();
    ensure_distill_store(&conn).unwrap();
    conn
}

fn bracket_env(key: &str, value: &str) {
    std::env::set_var(key, value);
}

fn unbracket_env(key: &str) {
    std::env::remove_var(key);
}

#[allow(clippy::disallowed_methods)]
fn make_model(name: &str, seed: u64) -> ReflexModel {
    ReflexModel {
        name: name.to_string(),
        layers: vec![
            Box::new(Dense::new(4, 8, ActivationKind::Relu, seed)),
            Box::new(Dense::new(8, 2, ActivationKind::Softmax, seed + 1)),
        ],
        seed,
        last_metric: None,
        input_size: 4,
        labels: vec!["pos".to_string(), "neg".to_string()],
        feature_signature: metalogos::embeddings::DISTILL_FEATURE_SIGNATURE.to_string(),
    }
}

fn spec_for(prompt: &str) -> AttestSpecInput {
    AttestSpecInput {
        pattern_name: Some("Classify".to_string()),
        prompt: Some(prompt.to_string()),
        distill_after: Some(2),
        fallback_if: Some(("lt".to_string(), 0.85)),
        min_accuracy: Some(0.85),
        margin: Some(0.05),
    }
}

/// The unfiltered fixture: the model is saved with no W4 curriculum
/// state in the store — the certificate's data filter word is `none`
/// (decision 5's dominance: the effective level is `unfiltered`).
#[allow(clippy::disallowed_methods)]
fn save_unfiltered(name: &str, db: &Path) {
    let mut registry = ReflexRegistry::new();
    let id = registry.register(make_model(name, 42));
    let model = registry.get_dense(id).unwrap().snapshot_clone().unwrap();
    let input = AttestInput {
        spec: spec_for("classify the text"),
        examples: vec![
            ("a".to_string(), "pos".to_string()),
            ("b".to_string(), "neg".to_string()),
        ],
    };
    persist::save_model_to_db(&model, name, db, Some(&input)).expect("save");
}

/// The filtered fixture: the W4 curriculum state exists for the pattern
/// BEFORE the save (a reviewed, non-quarantined sample) — the stored
/// filter word is `curriculum:<sha16>`, the effective level is the
/// certificate's own.
#[allow(clippy::disallowed_methods)]
fn save_filtered(name: &str, db: &Path) {
    {
        let conn = open_store(db);
        conn.execute(
            "INSERT INTO distill_samples \
             (pattern_name, input, output, source, created_at, reviewed, quarantined) \
             VALUES ('Classify', 'a', 'pos', 'teaching', 100, 1, 0)",
            [],
        )
        .unwrap();
    }
    save_unfiltered(name, db);
}

/// The E3/E4 research artifacts fixture — the recorded facts the gate
/// reads (the same schema the committed docs/research/ artifacts carry).
#[allow(clippy::disallowed_methods)]
fn craft_research_dir(root: &Path, e3_ece: Option<f64>, e4_mock_only: bool) -> PathBuf {
    use std::fs;
    fs::create_dir_all(root).unwrap();
    match e3_ece {
        Some(ece) => fs::write(
            root.join("e3-confidence-calibration.json"),
            format!("{{\"experiment\":\"e3\",\"ece\":{},\"bins\":[]}}", ece),
        )
        .unwrap(),
        None => fs::write(
            root.join("e3-confidence-calibration.json"),
            "{\"experiment\":\"e3\",\"bins\":[]}",
        )
        .unwrap(),
    }
    let e4 = if e4_mock_only {
        "{\"experiment\":\"e4\",\"teacher_rows\":[{\"backend\":\"mock-deterministic\",\
         \"agreement_with_majority_baseline\":0.0,\"reason\":\"the W6 enablement gate \
         must not read it as one\"}]}"
    } else {
        "{\"experiment\":\"e4\",\"teacher_rows\":[\
         {\"backend\":\"mock-deterministic\",\"agreement_with_majority_baseline\":0.0,\
         \"reason\":\"the W6 enablement gate must not read it as one\"},\
         {\"backend\":\"qwen25-vl-7b\",\"agreement_with_majority_baseline\":null,\
         \"reason\":\"not measured yet\"}]}"
    };
    fs::write(root.join("e4-teacher-baseline-agreement.json"), e4).unwrap();
    root.to_path_buf()
}

use std::path::PathBuf;

// ── the suite ────────────────────────────────────────────────────────

#[test]
#[serial]
#[allow(clippy::disallowed_methods)]
fn unfiltered_refusal_names_field_and_writes_nothing() {
    bracket_env("METALOGOS_TRACE_DISTILL", "1");
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("s.db");
    save_unfiltered("Classify", &db);
    let mut conn = open_store(&db);
    let (sink, log) = make_sink();
    let research = craft_research_dir(&temp.path().join("research"), Some(0.42), false);

    let before = current_cert(&conn, "Classify").unwrap().unwrap();
    let err = enable_differential(&mut conn, "Classify", &research, 1000, &*sink).unwrap_err();
    assert!(
        err.starts_with("[ATTESTATION_ENABLEMENT_UNFILTERED]"),
        "the machine code leads the refusal: {}",
        err
    );
    assert!(
        err.contains("the field `level`") && err.contains("unfiltered"),
        "the refusal names the field and the cause: {}",
        err
    );

    // Nothing was written: the certificate row is byte-identical, no
    // superseding row appeared.
    let after = current_cert(&conn, "Classify").unwrap().unwrap();
    assert_eq!(before.cert_json, after.cert_json);
    assert_eq!(before.seq, after.seq);

    // The refusal is loud on the trace surface.
    let log = log.lock().unwrap();
    assert!(
        log.iter()
            .any(|l| l.contains("[TRACE] enable-differential pattern=Classify REFUSED")),
        "the refusal rides the trace: {:?}",
        log.iter().take(4).collect::<Vec<_>>()
    );
    unbracket_env("METALOGOS_TRACE_DISTILL");
}

#[test]
#[serial]
#[allow(clippy::disallowed_methods)]
fn filtered_flips_calibration_to_measured_ece() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("s.db");
    save_filtered("Classify", &db);
    let mut conn = open_store(&db);
    let (sink, _log) = make_sink();
    let research = craft_research_dir(&temp.path().join("research"), Some(0.42), false);

    let before = current_cert(&conn, "Classify").unwrap().unwrap();
    let v = enable_differential(&mut conn, "Classify", &research, 1000, &*sink).unwrap();
    assert!(v.ok);
    assert!(v.flipped);
    assert!(v.level != "unfiltered", "the filtered pattern proceeds");
    assert_eq!(v.calibration_ece, Some(0.42));

    // The stored certificate now carries the measured calibration and a
    // NEW anchor (the superseding row — the current seq moved past the
    // previous one, the ledger is monotone).
    let after = current_cert(&conn, "Classify").unwrap().unwrap();
    let cert: serde_json::Value = serde_json::from_str(&after.cert_json).unwrap();
    assert_eq!(cert["eval"]["calibration"], serde_json::json!(0.42));
    assert!(
        after.seq > before.seq,
        "the flip superseded the previous row"
    );
    assert!(after.ledger_seq.unwrap_or(0) > before.ledger_seq.unwrap_or(0));
}

#[test]
#[serial]
#[allow(clippy::disallowed_methods)]
fn flip_leaves_weights_and_facts_untouched() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("s.db");
    save_filtered("Classify", &db);
    let mut conn = open_store(&db);
    let (sink, _log) = make_sink();
    let research = craft_research_dir(&temp.path().join("research"), Some(0.6013), false);

    let before = current_cert(&conn, "Classify").unwrap().unwrap();
    let weights_before: Vec<u8> = conn
        .query_row(
            "SELECT weights FROM reflex_models WHERE name = 'Classify'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    enable_differential(&mut conn, "Classify", &research, 1000, &*sink).unwrap();

    let after = current_cert(&conn, "Classify").unwrap().unwrap();
    assert_eq!(before.weights_sha256, after.weights_sha256);
    assert_eq!(before.spec_hash, after.spec_hash);
    assert_eq!(before.data_hash, after.data_hash);
    assert_eq!(before.embed_signature, after.embed_signature);
    let weights_after: Vec<u8> = conn
        .query_row(
            "SELECT weights FROM reflex_models WHERE name = 'Classify'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(weights_before, weights_after, "the W1 lane is untouched");
}

#[test]
#[serial]
#[allow(clippy::disallowed_methods)]
fn missing_research_artifacts_null_with_reason_not_silent() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("s.db");
    save_filtered("Classify", &db);
    let mut conn = open_store(&db);
    let (sink, _log) = make_sink();
    let research = craft_research_dir(&temp.path().join("research"), None, false);

    let v = enable_differential(&mut conn, "Classify", &research, 1000, &*sink).unwrap();
    assert!(v.ok);
    assert!(!v.flipped, "the flip is withheld when E3 carries no number");
    assert!(v.calibration_ece.is_none());
    assert!(
        v.calibration_reason.contains("null-with-reason"),
        "the reason names the missing input: {}",
        v.calibration_reason
    );
    // The certificate stays "uncalibrated" — the loud no-op.
    let after = current_cert(&conn, "Classify").unwrap().unwrap();
    let cert: serde_json::Value = serde_json::from_str(&after.cert_json).unwrap();
    assert_eq!(
        cert["eval"]["calibration"],
        serde_json::json!("uncalibrated")
    );
}

#[test]
#[serial]
#[allow(clippy::disallowed_methods)]
fn mock_zero_agreement_not_read_as_one() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("s.db");
    save_filtered("Classify", &db);
    let mut conn = open_store(&db);
    let (sink, _log) = make_sink();
    let research = craft_research_dir(&temp.path().join("research"), Some(0.42), true);

    let v = enable_differential(&mut conn, "Classify", &research, 1000, &*sink).unwrap();
    assert!(v.baseline_agreement.is_none());
    assert!(
        v.baseline_reason.contains("not read as one"),
        "the mock's 0.0 is not read as one: {}",
        v.baseline_reason
    );
}

#[test]
#[serial]
#[allow(clippy::disallowed_methods)]
fn frozen_pattern_refuses_before_the_gate() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("s.db");
    save_filtered("Classify", &db);
    let mut conn = open_store(&db);
    conn.execute(
        "INSERT OR REPLACE INTO distill_states \
         (pattern, phase, entered_at, violations, last_violation_code) \
         VALUES ('Classify', 'frozen', 900, '', NULL)",
        [],
    )
    .unwrap();
    let (sink, _log) = make_sink();
    let research = craft_research_dir(&temp.path().join("research"), Some(0.42), false);

    let err = enable_differential(&mut conn, "Classify", &research, 1000, &*sink).unwrap_err();
    assert!(
        err.starts_with("[DISTILL_FROZEN]"),
        "the lifecycle interlock composes first: {}",
        err
    );
}

#[test]
#[serial]
#[allow(clippy::disallowed_methods)]
fn no_certificate_refuses_loud_with_code_none() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("s.db");
    let mut conn = open_store(&db);
    let (sink, _log) = make_sink();
    let research = craft_research_dir(&temp.path().join("research"), Some(0.42), false);

    let err = enable_differential(&mut conn, "Classify", &research, 1000, &*sink).unwrap_err();
    assert!(
        err.starts_with(CODE_NONE) && err.contains("no attestation certificate"),
        "{}",
        err
    );
}

#[test]
#[serial]
#[allow(clippy::disallowed_methods)]
fn trace_lines_land_in_the_audit_sink_on_the_flip_path() {
    bracket_env("METALOGOS_TRACE_DISTILL", "1");
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("s.db");
    save_filtered("Classify", &db);
    let mut conn = open_store(&db);
    let (sink, log) = make_sink();
    let research = craft_research_dir(&temp.path().join("research"), Some(0.42), false);

    enable_differential(&mut conn, "Classify", &research, 1000, &*sink).unwrap();
    let log = log.lock().unwrap();
    assert!(
        log.iter()
            .any(|l| l.contains("[TRACE] enable-differential pattern=Classify inputs")),
        "the inputs line rides the trace: {:?}",
        log.iter().take(3).collect::<Vec<_>>()
    );
    assert!(
        log.iter()
            .any(|l| l.contains("[TRACE] enable-differential pattern=Classify FLIPPED")),
        "the flip line rides the trace"
    );
    unbracket_env("METALOGOS_TRACE_DISTILL");
}

#[test]
#[serial]
#[allow(clippy::disallowed_methods)]
fn gate_line_json_shape_carries_null_with_reason_fields() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("s.db");
    save_filtered("Classify", &db);
    let mut conn = open_store(&db);
    let (sink, _log) = make_sink();
    let research = craft_research_dir(&temp.path().join("research"), Some(0.42), true);

    let v = enable_differential(&mut conn, "Classify", &research, 1000, &*sink).unwrap();
    let js: serde_json::Value = serde_json::from_str(&v.gate_line()).unwrap();
    assert_eq!(js["gate"], "distill-enable-differential");
    assert_eq!(js["ok"], true);
    assert_eq!(js["pattern"], "Classify");
    assert_eq!(js["flipped"], true);
    assert_eq!(js["calibration_ece"], serde_json::json!(0.42));
    assert_eq!(js["baseline_agreement"], serde_json::Value::Null);
    assert!(
        js["baseline_reason"]
            .as_str()
            .unwrap()
            .contains("not read as one"),
        "{}",
        js["baseline_reason"]
    );
}
