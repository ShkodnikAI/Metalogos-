// ── Наряд №825 (ADR-0200): the Reflex attestation contract tests ──────
//
// The eight contracts from the naryad's «Контракт-тесты» section. Tests
// 1–5 and 7 drive the persist API directly (the shared save/load point —
// both backends attest through it); tests 6 and 8 run REAL .mlog programs
// (the background-distill path on the VM lane; the TW↔VM cross-check).
//
// The fs ratchet (№475) targets production I/O — this file exercises the
// real filesystem for fixtures by design (the crosscheck file's posture).

#![allow(clippy::disallowed_methods)]

use metalogos::nn::attest::{
    self, AttestInput, AttestPolicy, AttestSpecInput, CODE_DATA_CHANGED, CODE_EMBED_MISMATCH,
    CODE_NONE, CODE_STALE, CODE_WEIGHTS_CHANGED,
};
use metalogos::nn::{persist, ActivationKind, Dense, ReflexModel, ReflexRegistry};
use std::collections::HashMap;
use tempfile::TempDir;

// ── fixtures ───────────────────────────────────────────────────────────

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

fn registry_with(model: ReflexModel) -> (ReflexRegistry, metalogos::nn::ReflexId) {
    let mut registry = ReflexRegistry::new();
    let id = registry.register(model);
    (registry, id)
}

/// The №530 weights snapshot — ReflexModel is not Clone (Box<dyn Layer>);
/// the snapshot path is the sanctioned copy (Dense-only, the distill
/// contract).
fn snapshot(registry: &ReflexRegistry, id: metalogos::nn::ReflexId) -> ReflexModel {
    registry
        .get_dense(id)
        .unwrap()
        .snapshot_clone()
        .expect("snapshot")
}

// ── Contract 1: save → load → attest_verify: ok, level unfiltered ─────

#[test]
fn contract_1_save_load_verify_ok_unfiltered() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("c1.db");

    let (registry, id) = registry_with(make_model("AttestHead", 42));
    let model = snapshot(&registry, id);

    let input = AttestInput {
        spec: spec_for("classify the text"),
        examples: vec![
            ("a".to_string(), "pos".to_string()),
            ("b".to_string(), "neg".to_string()),
        ],
    };
    persist::save_model_to_db(&model, "AttestHead", &db, Some(&input)).expect("save");

    // The verify pass over the SAME declarations (the consistent case).
    #[allow(clippy::disallowed_methods)]
    let conn = rusqlite::Connection::open(&db).unwrap();
    let canonical = attest::canonical_spec_json(&model, &input.spec);
    let s_hash = metalogos::vision::provenance::sha256_hex(canonical.as_bytes());
    let blob = persist::serialize_for_storage(&model);
    let verdict = attest::verify_attestation(
        &conn,
        "AttestHead",
        &blob,
        &model.feature_signature,
        &canonical,
        &s_hash,
        &input.examples,
        input.spec.pattern_name.as_deref(),
        AttestPolicy::lenient(),
    )
    .unwrap();

    assert!(verdict.ok, "contract 1: expected ok, got {:?}", verdict);
    assert_eq!(verdict.level, "unfiltered", "decision 5: the W1 data level");
    assert!(verdict.reason.is_empty(), "no informational/refusal code");
    // The certificate carries the shape keys and the honest nulls.
    let cert: serde_json::Value = serde_json::from_str(&verdict.cert_json).unwrap();
    assert_eq!(cert["model"], "AttestHead");
    assert_eq!(cert["pattern"], "Classify");
    assert_eq!(cert["eval"]["calibration"], "uncalibrated");
    assert_eq!(cert["dataset"]["filter"], "none");
    assert_eq!(cert["dataset"]["rows"], 2);
    assert!(cert["teacher"].is_null(), "W3b pending — null with reason");
    assert!(
        cert["rollback_if"].is_null(),
        "session-executed rules — null with reason"
    );
    assert_eq!(
        cert["level"], "unsigned",
        "no METALOGOS_LEDGER_KEY in tests"
    );
    assert!(cert["ledger_seq"].is_i64(), "the anchor seq recorded");
}

// ── Contract 2: spec change → STALE with the field name; comments never ─

#[test]
fn contract_2_spec_change_stale_names_field() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("c2.db");
    let (registry, id) = registry_with(make_model("AttestHead", 42));
    let model = snapshot(&registry, id);

    persist::save_model_to_db(
        &model,
        "AttestHead",
        &db,
        Some(&AttestInput {
            spec: spec_for("classify the text"),
            examples: vec![],
        }),
    )
    .unwrap();

    #[allow(clippy::disallowed_methods)]
    let conn = rusqlite::Connection::open(&db).unwrap();
    let canonical = attest::canonical_spec_json(&model, &spec_for("classify the text"));
    let blob = persist::serialize_for_storage(&model);

    // The PROMPT changed → stale, the changed field is named.
    let new_canonical = attest::canonical_spec_json(&model, &spec_for("a different prompt"));
    let new_hash = metalogos::vision::provenance::sha256_hex(new_canonical.as_bytes());
    let verdict = attest::verify_attestation(
        &conn,
        "AttestHead",
        &blob,
        &model.feature_signature,
        &new_canonical,
        &new_hash,
        &[],
        Some("Classify"),
        AttestPolicy::lenient(),
    )
    .unwrap();
    assert!(!verdict.ok, "a prompt change must refuse");
    assert!(
        verdict.reason.contains(CODE_STALE),
        "got: {}",
        verdict.reason
    );
    assert!(
        verdict.reason.contains("prompt"),
        "the changed field must be NAMED, got: {}",
        verdict.reason
    );

    // A comment edit cannot enter: the canonical form is built from the
    // compiled pattern (prompt string, no comments) — determinism check:
    // the SAME decl produces the byte-identical canonical string, and the
    // form carries no comment tokens by construction.
    assert_eq!(
        canonical,
        attest::canonical_spec_json(&model, &spec_for("classify the text"))
    );
    assert!(!canonical.contains("//") && !canonical.contains("#"));
}

// ── Contract 3: a weights-byte tamper → weights_sha256 refusal ─────────

#[test]
fn contract_3_weights_tamper_refused() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("c3.db");
    let (registry, id) = registry_with(make_model("AttestHead", 42));
    let model = snapshot(&registry, id);

    persist::save_model_to_db(
        &model,
        "AttestHead",
        &db,
        Some(&AttestInput {
            spec: spec_for("classify the text"),
            examples: vec![],
        }),
    )
    .unwrap();

    // Tamper the BLOB in place (read → flip a byte → write back as a real
    // BLOB — the SQL `||` operator would silently convert to TEXT).
    #[allow(clippy::disallowed_methods)]
    let conn = rusqlite::Connection::open(&db).unwrap();
    let mut tampered: Vec<u8> = {
        let mut stmt = conn
            .prepare("SELECT weights FROM reflex_models WHERE name='AttestHead'")
            .unwrap();
        stmt.query_row([], |r| r.get::<_, Vec<u8>>(0)).unwrap()
    };
    tampered[0] ^= 0xFF;
    conn.execute(
        "UPDATE reflex_models SET weights = ?1 WHERE name = 'AttestHead'",
        rusqlite::params![tampered],
    )
    .unwrap();

    let canonical = attest::canonical_spec_json(&model, &spec_for("classify the text"));
    let s_hash = metalogos::vision::provenance::sha256_hex(canonical.as_bytes());
    let verdict = attest::verify_attestation(
        &conn,
        "AttestHead",
        &tampered,
        &model.feature_signature,
        &canonical,
        &s_hash,
        &[],
        Some("Classify"),
        AttestPolicy::lenient(),
    )
    .unwrap();
    assert!(!verdict.ok);
    assert!(
        verdict.reason.contains(CODE_WEIGHTS_CHANGED),
        "got: {}",
        verdict.reason
    );

    // The full LOAD path refuses too.
    let (mut registry2, id2) = registry_with(make_model("AttestHead", 42));
    let err = persist::load_model_from_db(
        &mut registry2,
        id2,
        "AttestHead",
        &db,
        Some(&AttestInput {
            spec: spec_for("classify the text"),
            examples: vec![],
        }),
        AttestPolicy::lenient(),
    )
    .unwrap_err();
    assert!(err.contains(CODE_WEIGHTS_CHANGED), "got: {}", err);
}

// ── Contract 4: an embed-signature change → EMBED_MISMATCH ─────────────

#[test]
fn contract_4_embed_signature_mismatch_refused() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("c4.db");
    let (registry, id) = registry_with(make_model("AttestHead", 42));
    let model = snapshot(&registry, id);

    persist::save_model_to_db(
        &model,
        "AttestHead",
        &db,
        Some(&AttestInput {
            spec: spec_for("classify the text"),
            examples: vec![],
        }),
    )
    .unwrap();

    // Tamper the CERTIFICATE's embed pin (the extractor provenance row).
    #[allow(clippy::disallowed_methods)]
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute(
        "UPDATE reflex_attestation SET embed_signature = 'sig-from-another-build' \
         WHERE model = 'AttestHead' AND superseded = 0",
        [],
    )
    .unwrap();

    let (mut registry2, id2) = registry_with(make_model("AttestHead", 42));
    let err = persist::load_model_from_db(
        &mut registry2,
        id2,
        "AttestHead",
        &db,
        Some(&AttestInput {
            spec: spec_for("classify the text"),
            examples: vec![],
        }),
        AttestPolicy::lenient(),
    )
    .unwrap_err();
    assert!(err.contains(CODE_EMBED_MISMATCH), "got: {}", err);
}

// ── Contract 5: legacy weights (no cert) → warn + cert:none; strict ────

#[test]
fn contract_5_legacy_weights_lenient_then_strict() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("c5.db");
    let (registry, id) = registry_with(make_model("AttestHead", 42));
    let model = snapshot(&registry, id);

    // The pre-W1 form: save WITHOUT the attestation input.
    persist::save_model_to_db(&model, "AttestHead", &db, None).unwrap();

    // Lenient: the load CONTINUES with cert: none (the loud warning is
    // on stderr inside verify_attestation).
    let (mut registry2, id2) = registry_with(make_model("AttestHead", 42));
    let report = persist::load_model_from_db(
        &mut registry2,
        id2,
        "AttestHead",
        &db,
        Some(&AttestInput {
            spec: spec_for("classify the text"),
            examples: vec![],
        }),
        AttestPolicy::lenient(),
    )
    .expect("lenient load continues");
    assert!(report.verdict.is_none(), "no certificate → no verdict");
    // The weights actually loaded.
    assert!(registry2.get_dense(id2).is_some());

    // Strict: the same load REFUSES.
    let (mut registry3, id3) = registry_with(make_model("AttestHead", 42));
    let err = persist::load_model_from_db(
        &mut registry3,
        id3,
        "AttestHead",
        &db,
        Some(&AttestInput {
            spec: spec_for("classify the text"),
            examples: vec![],
        }),
        AttestPolicy {
            strict: true,
            allow_stale: false,
        },
    )
    .unwrap_err();
    assert!(err.contains(CODE_NONE), "got: {}", err);

    // Strict + an UNSIGNED certificate → refusal too (decision 4).
    persist::save_model_to_db(
        &model,
        "AttestHead",
        &db,
        Some(&AttestInput {
            spec: spec_for("classify the text"),
            examples: vec![],
        }),
    )
    .unwrap();
    let (mut registry4, id4) = registry_with(make_model("AttestHead", 42));
    let err = persist::load_model_from_db(
        &mut registry4,
        id4,
        "AttestHead",
        &db,
        Some(&AttestInput {
            spec: spec_for("classify the text"),
            examples: vec![],
        }),
        AttestPolicy {
            strict: true,
            allow_stale: false,
        },
    )
    .unwrap_err();
    assert!(
        err.contains("[ATTESTATION_UNSIGNED]"),
        "strict refuses unanchored certs, got: {}",
        err
    );
}

// ── Contract 6: the background-distill path produces a certificate ─────
//
// Real .mlog on the VM lane: 10 teaching calls (MockLlm echoes the
// prompt "yes") → the ADR-0115 threshold crosses → the background trainer
// runs the holdout gate and switches to DISTILLED → reflex_save writes
// the certificate with the RUNTIME teaching manifest (rows = 10).

fn distill_program(db_rel: &str) -> String {
    format!(
        r#"
memory {{ persist: "{db_rel}" }}

reflex DecideHead {{
  input: embedding(4)
  layers: [dense(4, "relu"), dense(2, "softmax")]
  labels: ["yes", "no"]
  seed: 42
}}

learnable pattern Decide(question: String) -> String {{
  prompt: "yes"
  distill_to: DecideHead
  distill_after: 1
}}

pattern Drive(_x: String) -> String {{
  let r1 = Decide("q1")
  let r2 = Decide("q2")
  let r3 = Decide("q3")
  let r4 = Decide("q4")
  let r5 = Decide("q5")
  let r6 = Decide("q6")
  let r7 = Decide("q7")
  let r8 = Decide("q8")
  let r9 = Decide("q9")
  let r10 = Decide("q10")
  // Training triggers on this call (10 examples ≥ the ADR-0115 minimum).
  let r11 = Decide("q11")
  let _s = reflex_save(DecideHead)
  let v = attest_verify(DecideHead)
  return "ok=" + to_string(v.ok) + ";level=" + v.level
}}

flow Main {{ input: String = "x" -> Drive -> output }}
"#
    )
}

#[test]
#[serial_test::serial]
fn contract_6_background_distill_produces_certificate() {
    let temp = TempDir::new().unwrap();
    let db_abs = temp.path().join("c6.db");
    // The .mlog takes the path literally — absolute is the loader's trust
    // domain (№475: the binary is the source loader boundary).
    let source = distill_program(db_abs.to_str().unwrap());
    std::env::set_var("METALOGOS_MOCK_LLM", "1");

    let result = metalogos::run_program_on_vm(&source);
    let output = result.expect("the VM run must succeed");
    let output = output.unwrap_or_default();
    assert!(
        output.contains("ok=true"),
        "attest_verify must answer ok after the distill path, got: {output}"
    );
    assert!(output.contains("level=unfiltered"), "got: {output}");

    // The certificate row exists, with the RUNTIME teaching manifest.
    #[allow(clippy::disallowed_methods)]
    let conn = rusqlite::Connection::open(&db_abs).unwrap();
    let (rows, cert_json): (i64, String) = {
        let mut stmt = conn
            .prepare(
                "SELECT json_extract(cert_json, '$.dataset.rows'), cert_json \
                 FROM reflex_attestation WHERE model='DecideHead' AND superseded=0",
            )
            .unwrap();
        stmt.query_row([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap()
    };
    // 10 teaching examples at the training trigger; the 11th call records
    // its example AFTER the trigger fires (the training consumed 10, the
    // ledger keeps accumulating — the honest count is ≥ 10).
    assert!(
        rows >= 10,
        "the teaching manifest rows land in the cert, got {}",
        rows
    );
    let cert: serde_json::Value = serde_json::from_str(&cert_json).unwrap();
    assert_eq!(cert["pattern"], "Decide");
    assert_eq!(
        cert["dataset"]["provenance"],
        serde_json::json!(["runtime-teaching"])
    );
    std::env::remove_var("METALOGOS_MOCK_LLM");
}

// ── Contract 7: new distill_samples rows → DATA_CHANGED, not a refusal ─

#[test]
fn contract_7_data_change_is_informational() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("c7.db");
    let (registry, id) = registry_with(make_model("AttestHead", 42));
    let model = snapshot(&registry, id);

    persist::save_model_to_db(
        &model,
        "AttestHead",
        &db,
        Some(&AttestInput {
            spec: spec_for("classify the text"),
            examples: vec![],
        }),
    )
    .unwrap();

    // New data lands in distill_samples (the persistent ledger).
    #[allow(clippy::disallowed_methods)]
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS distill_samples (
            pattern_name TEXT NOT NULL, input TEXT NOT NULL, output TEXT NOT NULL,
            source TEXT NOT NULL, created_at INTEGER NOT NULL);
         INSERT INTO distill_samples VALUES ('Classify', 'new input', 'pos', 'test', 0);",
    )
    .unwrap();

    let canonical = attest::canonical_spec_json(&model, &spec_for("classify the text"));
    let s_hash = metalogos::vision::provenance::sha256_hex(canonical.as_bytes());
    let blob = persist::serialize_for_storage(&model);
    let verdict = attest::verify_attestation(
        &conn,
        "AttestHead",
        &blob,
        &model.feature_signature,
        &canonical,
        &s_hash,
        &[],
        Some("Classify"),
        AttestPolicy::lenient(),
    )
    .unwrap();
    assert!(verdict.ok, "DATA_CHANGED is informational, NOT a refusal");
    assert!(
        verdict.reason.contains(CODE_DATA_CHANGED),
        "got: {}",
        verdict.reason
    );
}

// ── Contract 8: the TW ↔ VM cross-check ────────────────────────────────

#[test]
fn contract_8_vm_tw_crosscheck_identical_verdicts() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("c8.db");

    let db_str = db.display().to_string();
    let source = format!(
        r#"
memory {{ persist: "{db_str}" }}

reflex AttestHead {{
  input: embedding(4)
  layers: [dense(8, "relu"), dense(2, "softmax")]
  labels: ["pos", "neg"]
  seed: 42
}}

learnable pattern Classify(text: String) -> String {{
  prompt: "classify the text"
  distill_to: AttestHead
  distill_after: 2
}}

pattern Drive(_x: String) -> String {{
  let _s = reflex_save(AttestHead)
  let _l = reflex_load("AttestHead")
  let v = attest_verify(AttestHead)
  return "ok=" + to_string(v.ok) + ";level=" + v.level + ";hash=" + v.spec_hash
}}

flow Main {{ input: String = "x" -> Drive -> output }}
"#
    );

    let tw = metalogos::run_program_with_dir(&source, temp.path().to_path_buf())
        .expect("TW run")
        .unwrap_or_default();
    let vm = metalogos::run_program_on_vm(&source)
        .expect("VM run")
        .unwrap_or_default();

    assert!(
        tw.contains("ok=true") && tw.contains("level=unfiltered"),
        "TW verdict: {tw}"
    );
    assert_eq!(
        tw, vm,
        "the two backends must answer the identical attestation verdict"
    );

    // The CLI lane sees the same certificate through the same DB.
    let declarations = metalogos::parser::parse(&source).unwrap();
    let reflex = declarations
        .iter()
        .find_map(|d| match d {
            metalogos::ast::Declaration::Reflex(r) => Some(r),
            _ => None,
        })
        .unwrap();
    assert_eq!(reflex.name, "AttestHead");
    #[allow(clippy::disallowed_methods)]
    let conn = rusqlite::Connection::open(&db).unwrap();
    let stored = attest::current_cert(&conn, "AttestHead").unwrap();
    assert!(stored.is_some(), "the certificate row exists for the CLI");
}

// ── The policy surface: from_env (the strict/allow-stale wiring) ───────

#[test]
#[serial_test::serial]
fn policy_from_env_reads_both_vars() {
    std::env::set_var("METALOGOS_ATTEST_STRICT", "1");
    std::env::set_var("METALOGOS_ATTEST_ALLOW_STALE", "1");
    let p = AttestPolicy::from_env();
    assert!(p.strict && p.allow_stale);
    std::env::remove_var("METALOGOS_ATTEST_STRICT");
    std::env::remove_var("METALOGOS_ATTEST_ALLOW_STALE");
    let p = AttestPolicy::from_env();
    assert!(!p.strict && !p.allow_stale);
}

// ── The ledger-failure contract: a loud save error, never silent ───────
//
// The artifact rule (ADR-0200 §4): when the ledger cannot take the
// reflex.attest record, the save FAILS loudly. The in-process ledger
// cannot be disabled from outside, so this contract pins the mechanism:
// the write_attestation path returns a LOUD Err containing the artifact
// wording when append_record fails (simulated at the unit boundary by
// the error text contract in the module tests). Here we pin the honest
// complement: a save WITHOUT attest input never touches the ledger.

#[test]
fn save_without_attest_writes_no_certificate_rows() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("noattest.db");
    let (registry, id) = registry_with(make_model("AttestHead", 42));
    let model = snapshot(&registry, id);
    persist::save_model_to_db(&model, "AttestHead", &db, None).unwrap();

    #[allow(clippy::disallowed_methods)]
    let conn = rusqlite::Connection::open(&db).unwrap();
    let cert_present = attest::current_cert(&conn, "AttestHead").unwrap();
    assert!(
        cert_present.is_none(),
        "no attest input → no certificate row"
    );
    // The reflex_models row still landed (the plain №180 contract).
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM reflex_models WHERE name='AttestHead'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 1);
}

// ── The map the dispatches rely on: name resolution sanity ─────────────

#[test]
fn name_resolution_map_is_wired() {
    let (registry, id) = registry_with(make_model("AttestHead", 42));
    let mut names: HashMap<String, metalogos::nn::ReflexId> = HashMap::new();
    names.insert("AttestHead".to_string(), id);
    let dispatch = |args: &[metalogos::interpreter::Value]| {
        metalogos::builtins::attest_verify_dispatch(
            &registry,
            &names,
            Some("/nonexistent/db.sqlite"),
            None,
            args,
        )
    };
    // The persistence-not-configured and open-failure errors are loud and
    // typed (the reflex_save error-form family).
    let err = dispatch(&[metalogos::interpreter::Value::String("AttestHead".into())]).unwrap_err();
    assert!(err.contains("attest_verify"), "got: {}", err);
    // Wrong arity — loud.
    assert!(dispatch(&[]).is_err());
    // Unknown model — loud.
    let err = dispatch(&[metalogos::interpreter::Value::String("Nope".into())]).unwrap_err();
    assert!(err.contains("not declared"), "got: {}", err);
}

// ── The < 200 ms verify contract (the naryad's acceptance item) ────────

#[test]
fn verify_completes_under_200ms() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("timing.db");
    let (registry, id) = registry_with(make_model("AttestHead", 42));
    let model = snapshot(&registry, id);
    persist::save_model_to_db(
        &model,
        "AttestHead",
        &db,
        Some(&AttestInput {
            spec: spec_for("classify the text"),
            examples: vec![],
        }),
    )
    .unwrap();

    #[allow(clippy::disallowed_methods)]
    let conn = rusqlite::Connection::open(&db).unwrap();
    let canonical = attest::canonical_spec_json(&model, &spec_for("classify the text"));
    let s_hash = metalogos::vision::provenance::sha256_hex(canonical.as_bytes());
    let blob = persist::serialize_for_storage(&model);

    // Warm-up (the first open pays the SQLite page-cache cost).
    let _ = attest::verify_attestation(
        &conn,
        "AttestHead",
        &blob,
        &model.feature_signature,
        &canonical,
        &s_hash,
        &[],
        Some("Classify"),
        AttestPolicy::lenient(),
    )
    .unwrap();

    let start = std::time::Instant::now();
    let runs = 100;
    for _ in 0..runs {
        let v = attest::verify_attestation(
            &conn,
            "AttestHead",
            &blob,
            &model.feature_signature,
            &canonical,
            &s_hash,
            &[],
            Some("Classify"),
            AttestPolicy::lenient(),
        )
        .unwrap();
        assert!(v.ok);
    }
    let per_run = start.elapsed() / runs;
    assert!(
        per_run.as_millis() < 200,
        "the verify pass must stay under 200 ms, got {:?} per run",
        per_run
    );
    eprintln!("[n825] verify timing: {:?} per run (100 runs)", per_run);
}
