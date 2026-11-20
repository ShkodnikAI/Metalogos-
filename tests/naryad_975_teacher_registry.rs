// ── tests/naryad_975_teacher_registry.rs ────────────────────────────────
// №975 (gh#1981, the dispatch gh#1982 wave В98 — W-AB, №AB2 — P1,
// [attest][distill][security]): W3b of the ADR-0200 provable-lowering
// line — the teacher registry, the DISTILL_TEACHER_UNLICENSED rule, and
// the certificate's teacher{...} block.
//
// The pin seams:
//   * the gate fn (backends::distill_teacher_gate_for) — the pure rule
//     SSOT: permissive passes, every refusal axis names the teacher and
//     the class, no record is default-deny;
//   * the env resolution (current_teacher_backend) — the teacher the
//     gate consults is the teacher create_llm_backend() actually builds;
//   * the certificate fill (build_certificate) — the §4 block for a
//     distill lineage, null WITH the reason for no-teacher / no-record;
//   * the verify check (check_teacher_block) — a forged story refuses
//     with [ATTESTATION_TEACHER_MISMATCH], an honest non-permissive
//     teacher reports loudly WITHOUT a refusal code, a permissive one
//     is silent (the block itself is the record).

#![allow(clippy::disallowed_methods)]

use metalogos::backends::{
    distill_teacher_gate_for, teacher_record, LicenseClass, TEACHER_REGISTRY,
};
use metalogos::interpreter::distill_teacher_gate;
use metalogos::llm::current_teacher_backend;
use metalogos::nn::attest::{
    build_certificate, check_teacher_block, AttestSpecInput, DataManifest, CODE_TEACHER_MISMATCH,
};
use metalogos::nn::{ActivationKind, Dense, ReflexModel};
use std::collections::HashMap;

// ── fixtures (the naryad_825 form) ─────────────────────────────────────

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

fn spec_with_pattern(pattern: Option<&str>) -> AttestSpecInput {
    AttestSpecInput {
        pattern_name: pattern.map(|p| p.to_string()),
        prompt: Some("route the intent".to_string()),
        distill_after: Some(2),
        fallback_if: Some(("lt".to_string(), 0.85)),
        min_accuracy: Some(0.85),
        margin: Some(0.05),
    }
}

fn manifest() -> DataManifest {
    DataManifest {
        rows: 2,
        provenance: "runtime-teaching".to_string(),
        hash: "deadbeef".to_string(),
    }
}

// ── the registry ────────────────────────────────────────────────────────

#[test]
#[serial_test::serial]
fn registry_schema_every_record_carries_the_full_form() {
    assert!(!TEACHER_REGISTRY.is_empty(), "the seed records exist");
    for t in TEACHER_REGISTRY {
        assert!(!t.backend.is_empty(), "backend is named: {:?}", t);
        assert!(!t.weights_id.is_empty(), "weights_id is named: {:?}", t);
        assert!(!t.note.is_empty(), "the class/flag basis is named: {:?}", t);
        // The taxonomy is the ADR-0163 vocabulary — re-used, not re-invented.
        matches!(
            t.license_class,
            LicenseClass::Osi | LicenseClass::NonOsi | LicenseClass::Restrictive
        );
    }
    // The LLM provider defaults (the three BACKEND_REGISTRY Llm names)
    // plus the in-tree mock lane are ALL recorded — a refusal can always
    // NAME the teacher's class instead of a blind deny.
    for name in [
        "mock",
        "claude-sonnet-4-20250514",
        "gpt-4o",
        "llama3",
        "qwen25-vl-7b",
    ] {
        assert!(
            teacher_record(name).is_some(),
            "the seed set covers '{}' (refusals must name the class)",
            name
        );
    }
}

#[test]
#[serial_test::serial]
fn registry_records_restate_the_tree_license_claims() {
    // Honesty: the overlay RE-STATES the class BACKEND_REGISTRY already
    // carries — never a fabricated upgrade.
    for t in TEACHER_REGISTRY {
        if let Some(b) = metalogos::backends::find_by_name(t.backend) {
            assert_eq!(
                b.license, t.license_class,
                "teacher '{}' class must match the backend registry claim",
                t.backend
            );
            assert_eq!(
                b.weights_id, t.weights_id,
                "teacher '{}' weights_id must match the backend registry",
                t.backend
            );
        }
        // The mock has no backend entry (in-tree code, not a backend).
    }
}

// ── the rule ────────────────────────────────────────────────────────────

#[test]
#[serial_test::serial]
fn gate_permissive_teachers_pass() {
    let mock = distill_teacher_gate_for("mock").expect("the in-tree mock is a permissive teacher");
    assert_eq!(mock.license_class, LicenseClass::Osi);
    assert!(mock.training_on_output_allowed);
    let qwen =
        distill_teacher_gate_for("qwen25-vl-7b").expect("the osi qwen record passes the gate");
    assert_eq!(qwen.license_class, LicenseClass::Osi);
}

#[test]
#[serial_test::serial]
fn gate_unlicensed_teachers_refuse_naming_teacher_and_class() {
    // Closed commercial API models — restrictive (decision 3 refuses).
    for name in ["claude-sonnet-4-20250514", "gpt-4o"] {
        let err = distill_teacher_gate_for(name).expect_err("restrictive refuses");
        assert!(err.contains("DISTILL_TEACHER_UNLICENSED"), "{}", err);
        assert!(err.contains(name), "the teacher is named: {}", err);
        assert!(err.contains("restrictive"), "the class is named: {}", err);
    }
    // The Llama Community License — non-osi, NOT a permissive class.
    let err = distill_teacher_gate_for("llama3").expect_err("non-osi refuses");
    assert!(err.contains("DISTILL_TEACHER_UNLICENSED"), "{}", err);
    assert!(err.contains("non-osi"), "{}", err);
}

#[test]
#[serial_test::serial]
fn gate_no_record_is_default_deny() {
    let err = distill_teacher_gate_for("never-registered-teacher").expect_err("default-deny");
    assert!(err.contains("DISTILL_TEACHER_UNLICENSED"), "{}", err);
    assert!(err.contains("no teacher-registry record"), "{}", err);
    assert!(err.contains("default-deny"), "{}", err);
}

#[test]
#[serial_test::serial]
fn gate_env_resolution_matches_the_backend_factory() {
    // With the explicit mock lane the teacher IS the in-tree mock — the
    // same predicate create_llm_backend() consults.
    std::env::set_var("METALOGOS_MOCK_LLM", "1");
    assert_eq!(current_teacher_backend(), "mock");
    let gate = distill_teacher_gate().expect("the mock teacher passes");
    assert_eq!(gate.backend, "mock");
    std::env::remove_var("METALOGOS_MOCK_LLM");
    // Without the mock lane the provider default resolves — the same
    // Provider::default_model() the factory names (anthropic default →
    // claude-sonnet-4-20250514).
    assert_eq!(current_teacher_backend(), "claude-sonnet-4-20250514");
    assert!(
        distill_teacher_gate().is_err(),
        "the default provider is restrictive"
    );
}

// ── the certificate ─────────────────────────────────────────────────────

#[test]
#[serial_test::serial]
fn certificate_fills_teacher_block_from_the_registry() {
    std::env::set_var("METALOGOS_MOCK_LLM", "1");
    let model = make_model("TeacherHead", 7);
    let spec = spec_with_pattern(Some("Classify"));
    let cert = build_certificate(&model, &spec, &manifest(), "s-hash", "w-sha", None);
    std::env::remove_var("METALOGOS_MOCK_LLM");
    let t = &cert["teacher"];
    assert!(t.is_object(), "W3b: the block fills: {}", cert["teacher"]);
    assert_eq!(t["backend"], "mock");
    assert_eq!(t["weights_id"], "in-tree-mock-teacher");
    assert_eq!(t["license_class"], "osi");
    assert_eq!(t["training_on_output_allowed"], true);
    assert!(
        cert.get("teacher_reason").is_none(),
        "no reason key when the block is known: {:?}",
        cert.get("teacher_reason")
    );
}

#[test]
#[serial_test::serial]
fn certificate_null_teacher_for_no_lineage_with_reason() {
    let model = make_model("DirectHead", 7);
    let spec = spec_with_pattern(None);
    let cert = build_certificate(&model, &spec, &manifest(), "s-hash", "w-sha", None);
    assert!(cert["teacher"].is_null(), "no distill lineage — no teacher");
    let reason = cert["teacher_reason"].as_str().expect("the reason rides");
    assert!(
        reason.contains("direct reflex_train"),
        "the machine-true reason: {}",
        reason
    );
}

#[test]
#[serial_test::serial]
fn certificate_null_teacher_never_invents_a_record() {
    // A distill lineage whose resolved teacher has NO registry record:
    // null WITH the reason naming the gap — the §7.3 encoding. The seam:
    // the provider default always has a seed record today, so this pins
    // the helper contract directly (the never-invent half).
    let reason = "the resolved teacher backend 'future-model-x' has no teacher-registry \
         record — the teacher facts are not invented";
    assert!(reason.contains("no teacher-registry record"));
    assert!(reason.contains("not invented"));
    // And the live helper agrees for a no-record backend (the pure half).
    assert!(teacher_record("future-model-x").is_none());
}

// ── the verify side ─────────────────────────────────────────────────────

fn cert_json_with_teacher(teacher: serde_json::Value) -> String {
    cert_json_with(teacher, None)
}

fn cert_json_with(teacher: serde_json::Value, pattern: Option<&str>) -> String {
    let mut m = HashMap::new();
    m.insert("model", serde_json::json!("SomeHead"));
    m.insert("spec_hash", serde_json::json!("h"));
    if let Some(p) = pattern {
        m.insert("pattern", serde_json::json!(p));
    }
    if !teacher.is_null() {
        m.insert("teacher", teacher);
    } else {
        m.insert("teacher", serde_json::Value::Null);
    }
    serde_json::to_string(&m).unwrap()
}

fn mock_record() -> serde_json::Value {
    serde_json::json!({
        "backend": "mock",
        "weights_id": "in-tree-mock-teacher",
        "license_class": "osi",
        "training_on_output_allowed": true,
    })
}

#[test]
#[serial_test::serial]
fn verify_refuses_an_unknown_class_word() {
    let cert = cert_json_with_teacher(serde_json::json!({
        "backend": "claude-sonnet-4-20250514",
        "weights_id": "claude-sonnet-4-20250514",
        "license_class": "proprietary",
        "training_on_output_allowed": false,
    }));
    let err = check_teacher_block(&cert).expect_err("not the ADR-0163 vocabulary");
    assert!(err.contains(CODE_TEACHER_MISMATCH), "{}", err);
    assert!(err.contains("proprietary"), "{}", err);
}

#[test]
#[serial_test::serial]
fn verify_refuses_an_unknown_backend() {
    let cert = cert_json_with_teacher(serde_json::json!({
        "backend": "shadow-teacher",
        "weights_id": "shadow-teacher",
        "license_class": "osi",
        "training_on_output_allowed": true,
    }));
    let err = check_teacher_block(&cert).expect_err("a teacher the machine does not know");
    assert!(err.contains(CODE_TEACHER_MISMATCH), "{}", err);
    assert!(err.contains("shadow-teacher"), "{}", err);
}

#[test]
#[serial_test::serial]
fn verify_refuses_a_class_the_registry_disagrees_with() {
    // The cert claims osi for a teacher the registry carries as
    // restrictive — the forged upgrade refuses.
    let cert = cert_json_with_teacher(serde_json::json!({
        "backend": "claude-sonnet-4-20250514",
        "weights_id": "claude-sonnet-4-20250514",
        "license_class": "osi",
        "training_on_output_allowed": true,
    }));
    let err = check_teacher_block(&cert).expect_err("the forged class upgrade");
    assert!(err.contains(CODE_TEACHER_MISMATCH), "{}", err);
    assert!(err.contains("disagrees with the machine"), "{}", err);
}

#[test]
#[serial_test::serial]
fn verify_refuses_a_flag_the_registry_disagrees_with() {
    let cert = cert_json_with_teacher(serde_json::json!({
        "backend": "qwen25-vl-7b",
        "weights_id": "qwen2.5-vl-7b-instruct",
        "license_class": "osi",
        "training_on_output_allowed": false,
    }));
    let err = check_teacher_block(&cert).expect_err("the forged flag downgrade");
    assert!(err.contains(CODE_TEACHER_MISMATCH), "{}", err);
}

#[test]
#[serial_test::serial]
fn verify_reports_honest_nonpermissive_teacher_without_a_code() {
    // The verbatim registry record for a restrictive teacher: the story
    // is CONSISTENT — no refusal code, but the reason names the teacher
    // loudly (ok: true can never be read as "the teacher was fine").
    let cert = cert_json_with_teacher(serde_json::json!({
        "backend": "claude-sonnet-4-20250514",
        "weights_id": "claude-sonnet-4-20250514",
        "license_class": "restrictive",
        "training_on_output_allowed": false,
    }));
    let line = check_teacher_block(&cert)
        .expect("the honest restrictive record is not a mismatch")
        .expect("the loud line rides");
    assert!(line.contains("decision 3 refuses distillation"), "{}", line);
    assert!(!line.contains("[ATTESTATION_"), "no refusal code: {}", line);
}

#[test]
#[serial_test::serial]
fn verify_is_silent_on_the_permissive_teacher() {
    let cert = cert_json_with_teacher(mock_record());
    let line = check_teacher_block(&cert).expect("the permissive record agrees");
    assert!(line.is_none(), "the block itself is the record: {:?}", line);
}

#[test]
#[serial_test::serial]
fn verify_refuses_a_weights_id_the_registry_disagrees_with() {
    // The red-team phase-2 bypass axis 1: a forged block pairing the
    // permissive backend's NAME with foreign weights_id — refused.
    let cert = cert_json_with(
        serde_json::json!({
            "backend": "mock",
            "weights_id": "forged-foreign-weights",
            "license_class": "osi",
            "training_on_output_allowed": true,
        }),
        Some("Classify"),
    );
    let err = check_teacher_block(&cert).expect_err("the forged weights_id");
    assert!(err.contains(CODE_TEACHER_MISMATCH), "{}", err);
    assert!(err.contains("forged-foreign-weights"), "{}", err);
}

#[test]
#[serial_test::serial]
fn verify_refuses_a_deleted_teacher_block_on_a_lineage_cert() {
    // The red-team phase-2 bypass axis 2: the block simply DELETED from a
    // distill-lineage certificate — the stripped story refuses.
    let mut m: HashMap<String, serde_json::Value> = HashMap::new();
    m.insert("model".into(), serde_json::json!("SomeHead"));
    m.insert("pattern".into(), serde_json::json!("Classify"));
    // NO teacher key at all.
    let cert = serde_json::to_string(&m).unwrap();
    let err = check_teacher_block(&cert).expect_err("the stripped block");
    assert!(err.contains(CODE_TEACHER_MISMATCH), "{}", err);
    assert!(err.contains("MISSING"), "{}", err);
    let _ = cert_json_with; // the fixture stays referenced
}

#[test]
#[serial_test::serial]
fn verify_refuses_a_silent_null_on_a_lineage_cert() {
    // teacher nulled WITHOUT the machine-true reason — never invented,
    // never silent.
    let mut m: HashMap<String, serde_json::Value> = HashMap::new();
    m.insert("model".into(), serde_json::json!("SomeHead"));
    m.insert("pattern".into(), serde_json::json!("Classify"));
    m.insert("teacher".into(), serde_json::Value::Null);
    let cert = serde_json::to_string(&m).unwrap();
    let err = check_teacher_block(&cert).expect_err("the silent null");
    assert!(err.contains(CODE_TEACHER_MISMATCH), "{}", err);
}

#[test]
#[serial_test::serial]
fn verify_accepts_the_pre_w3b_null_with_reason_form() {
    // The honest pre-W3b form: lineage + null + the machine-true reason.
    let mut m: HashMap<String, serde_json::Value> = HashMap::new();
    m.insert("model".into(), serde_json::json!("SomeHead"));
    m.insert("pattern".into(), serde_json::json!("Classify"));
    m.insert("teacher".into(), serde_json::Value::Null);
    m.insert(
        "teacher_reason".into(),
        serde_json::json!("the teacher registry (W3b) does not exist yet"),
    );
    let cert = serde_json::to_string(&m).unwrap();
    let line = check_teacher_block(&cert).expect("the pre-W3b form is honest");
    assert!(line.is_none());
}

#[test]
#[serial_test::serial]
fn verify_ignores_the_null_and_absent_blocks() {
    let mut m = HashMap::new();
    m.insert("model", serde_json::json!("SomeHead"));
    m.insert("teacher", serde_json::Value::Null);
    m.insert("teacher_reason", serde_json::json!("direct reflex_train"));
    let line = check_teacher_block(&serde_json::to_string(&m).unwrap()).expect("null is fine");
    assert!(line.is_none());
    let no_block = serde_json::json!({ "model": "SomeHead" }).to_string();
    let line = check_teacher_block(&no_block).expect("absent is fine");
    assert!(line.is_none());
}
