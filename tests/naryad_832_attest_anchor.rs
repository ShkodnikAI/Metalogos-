// ── tests/naryad_832_attest_anchor.rs ───────────────────────────────
// №832 (wave 70, P1, [correctness][provenance]) — the H-3 finding of the
// merged audit `24fde8a`: verify never reads the ledger; `signed` means
// «the key was configured at save», not «the signature was checked»; a
// DB-only attacker (weights BLOB + recomputed cert_json in the
// reflex_attestation table) passes verify WITHOUT the ledger key.
//
// The fix: verify_attestation and verify_from_db_only locate the
// `reflex.attest` anchor record by `ledger_seq` and compare its
// `args_hash` against the sha256 of the certificate's PROVISIONAL view
// (the same bytes write_attestation hashed at save). Three outcomes:
// the anchor agrees / `anchor not found` (level unsigned) /
// [ATTESTATION_ANCHOR_MISMATCH] (loud refusal).
//
// RED-before evidence: the tamper test below RAN GREEN-against-the-hole
// on main `24fde8a` — the tampered DB passed verify as ok/unfiltered
// (the run recorded in the PR); the fix flips it to the loud refusal.

#![allow(clippy::disallowed_methods)]

use metalogos::nn::attest::{self, AttestInput, AttestPolicy, AttestSpecInput};
use metalogos::nn::{persist, ActivationKind, Dense, ReflexModel, ReflexRegistry};
use metalogos::vision::provenance::sha256_hex;
use tempfile::TempDir;

// ── fixtures (the №825 shapes) ──────────────────────────────────────

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

fn snapshot(registry: &ReflexRegistry, id: metalogos::nn::ReflexId) -> ReflexModel {
    registry
        .get_dense(id)
        .unwrap()
        .snapshot_clone()
        .expect("snapshot")
}

fn save(name: &str, db: &std::path::Path) -> (ReflexModel, AttestInput) {
    let registry = {
        let mut r = ReflexRegistry::new();
        let id = r.register(make_model(name, 42));
        (r, id)
    };
    let model = snapshot(&registry.0, registry.1);
    let input = AttestInput {
        spec: spec_for("classify the text"),
        examples: vec![
            ("a".to_string(), "pos".to_string()),
            ("b".to_string(), "neg".to_string()),
        ],
    };
    persist::save_model_to_db(&model, name, db, Some(&input)).expect("save");
    (model, input)
}

fn verify_conn(db: &std::path::Path) -> rusqlite::Connection {
    rusqlite::Connection::open(db).unwrap()
}

// ── the happy path: the anchor exists and agrees ────────────────────

#[test]
fn anchor_present_and_agrees_on_the_honest_path() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("anchor_ok.db");
    let (model, input) = save("AnchorOk", &db);

    // The anchor record exists in the process-local ledger.
    let conn = verify_conn(&db);
    let cert_json: String = conn
        .query_row(
            "SELECT cert_json FROM reflex_attestation WHERE model = 'AnchorOk' AND superseded = 0",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let cert: serde_json::Value = serde_json::from_str(&cert_json).unwrap();
    let seq = cert["ledger_seq"].as_u64().expect("ledger_seq present");
    let record = metalogos::ledger::all_records()
        .expect("ledger read")
        .into_iter()
        .find(|r| r.seq == seq)
        .expect("the anchor record exists in the process-local ledger");
    assert_eq!(record.action, "reflex.attest");
    assert_eq!(record.scope, "AnchorOk");

    // Verify: ok, no anchor noise on the honest path.
    let canonical = attest::canonical_spec_json(&model, &input.spec);
    let s_hash = sha256_hex(canonical.as_bytes());
    let blob = persist::serialize_for_storage(&model);
    let verdict = attest::verify_attestation(
        &conn,
        "AnchorOk",
        &blob,
        &model.feature_signature,
        &canonical,
        &s_hash,
        &input.examples,
        input.spec.pattern_name.as_deref(),
        AttestPolicy::lenient(),
    )
    .unwrap();
    assert!(verdict.ok, "the honest path verifies: {:?}", verdict);
    assert!(
        !verdict.reason.contains("anchor"),
        "no anchor noise on the honest path: {}",
        verdict.reason
    );
}

// ── the tamper scenario (H-3 verbatim): DB-only attacker ────────────

/// Swap the weights BLOB and rewrite `weights_sha256` + `cert_json` in
/// the reflex_attestation row (level=signed, the anchor seq kept) — the
/// exact attack the audit named: no ledger key needed, DB access only.
fn tamper_db_attestation(db: &std::path::Path, model_name: &str, attacker_weights: &[u8]) {
    let conn = rusqlite::Connection::open(db).unwrap();
    let cert_json: String = conn
        .query_row(
            "SELECT cert_json FROM reflex_attestation WHERE model = ?1 AND superseded = 0",
            rusqlite::params![model_name],
            |row| row.get(0),
        )
        .unwrap();
    let mut cert: serde_json::Value = serde_json::from_str(&cert_json).unwrap();
    cert["weights_sha256"] = serde_json::json!(sha256_hex(attacker_weights));
    cert["level"] = serde_json::json!("signed");
    let fake_cert = serde_json::to_string(&cert).unwrap();
    conn.execute(
        "UPDATE reflex_models SET weights = ?1 WHERE name = ?2",
        rusqlite::params![attacker_weights, model_name],
    )
    .unwrap();
    conn.execute(
        "UPDATE reflex_attestation SET weights_sha256 = ?1, cert_json = ?2 \
         WHERE model = ?3 AND superseded = 0",
        rusqlite::params![sha256_hex(attacker_weights), fake_cert, model_name],
    )
    .unwrap();
}

#[test]
fn tampered_db_attestation_refuses_with_anchor_mismatch() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("tamper.db");
    let (model, input) = save("TamperMe", &db);

    let attacker_weights = b"attacker-owned weights bytes 0xDEADBEEF";
    tamper_db_attestation(&db, "TamperMe", attacker_weights);

    // Everything the certificate checks is now self-consistent with the
    // attacker's bytes — the ONLY evidence is the ledger anchor.
    let conn = verify_conn(&db);
    let canonical = attest::canonical_spec_json(&model, &input.spec);
    let s_hash = sha256_hex(canonical.as_bytes());
    let verdict = attest::verify_attestation(
        &conn,
        "TamperMe",
        attacker_weights,
        &model.feature_signature,
        &canonical,
        &s_hash,
        &input.examples,
        input.spec.pattern_name.as_deref(),
        AttestPolicy::lenient(),
    )
    .unwrap();

    assert!(
        !verdict.ok,
        "the DB-only tamper must NOT pass verify (RED-before: it did)"
    );
    assert!(
        verdict.reason.contains("ATTESTATION_ANCHOR_MISMATCH"),
        "the refusal names the anchor mismatch: {}",
        verdict.reason
    );
}

/// The strict policy refuses the tamper too (the same evidence lane).
#[test]
fn tampered_db_attestation_refuses_in_strict_mode() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("tamper_strict.db");
    let (model, input) = save("TamperStrict", &db);
    let attacker_weights = b"strict-mode attacker bytes";
    tamper_db_attestation(&db, "TamperStrict", attacker_weights);

    let conn = verify_conn(&db);
    let canonical = attest::canonical_spec_json(&model, &input.spec);
    let s_hash = sha256_hex(canonical.as_bytes());
    let verdict = attest::verify_attestation(
        &conn,
        "TamperStrict",
        attacker_weights,
        &model.feature_signature,
        &canonical,
        &s_hash,
        &input.examples,
        input.spec.pattern_name.as_deref(),
        AttestPolicy {
            strict: true,
            allow_stale: false,
        },
    )
    .unwrap();
    assert!(!verdict.ok);
    assert!(verdict.reason.contains("ATTESTATION_ANCHOR_MISMATCH"));
}

// ── the anchor-not-found lane (the honest demotion) ─────────────────

/// A cert whose ledger_seq points nowhere (the cross-process shape —
/// the process-local ledger does not carry the record) demotes to
/// `unsigned` with the anchor-not-found reason, loudly; strict refuses.
#[test]
fn anchor_not_found_demotes_to_unsigned() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("orphan.db");
    let (model, input) = save("OrphanAnchor", &db);

    // Point the cert's anchor at a seq the ledger does not carry.
    let conn = verify_conn(&db);
    conn.execute(
        "UPDATE reflex_attestation SET cert_json = json_set(cert_json, '$.ledger_seq', 424242) \
         WHERE model = 'OrphanAnchor' AND superseded = 0",
        [],
    )
    .unwrap();

    let canonical = attest::canonical_spec_json(&model, &input.spec);
    let s_hash = sha256_hex(canonical.as_bytes());
    let blob = persist::serialize_for_storage(&model);
    let verdict = attest::verify_attestation(
        &conn,
        "OrphanAnchor",
        &blob,
        &model.feature_signature,
        &canonical,
        &s_hash,
        &input.examples,
        input.spec.pattern_name.as_deref(),
        AttestPolicy::lenient(),
    )
    .unwrap();
    assert!(
        verdict.ok && !verdict.level.contains("signed"),
        "anchor not found: the level claim demotes (never signed), got ok={} level={}",
        verdict.ok,
        verdict.level
    );
    assert!(
        verdict.reason.contains("anchor not found"),
        "the reason names the missing anchor: {}",
        verdict.reason
    );

    // Strict: the same state is a refusal.
    let strict = attest::verify_attestation(
        &conn,
        "OrphanAnchor",
        &blob,
        &model.feature_signature,
        &canonical,
        &s_hash,
        &input.examples,
        input.spec.pattern_name.as_deref(),
        AttestPolicy {
            strict: true,
            allow_stale: false,
        },
    )
    .unwrap();
    assert!(!strict.ok, "strict refuses the unanchored certificate");
}

// ── the CLI route (verify_from_db_only) carries the same anchor ─────

#[test]
fn cli_verify_carries_the_anchor_check() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("cli.db");
    let (model, input) = save("CliAnchor", &db);
    let attacker_weights = b"cli attacker bytes";
    tamper_db_attestation(&db, "CliAnchor", attacker_weights);

    let conn = verify_conn(&db);
    let canonical = attest::canonical_spec_json(&model, &input.spec);
    let s_hash = sha256_hex(canonical.as_bytes());
    let report = attest::verify_from_db_only(
        &conn,
        "CliAnchor",
        &canonical,
        &s_hash,
        AttestPolicy::lenient(),
    )
    .unwrap();
    assert!(
        !report.ok,
        "the CLI verify refuses the tampered attestation (RED-before: it passed)"
    );
    assert!(report.reason.contains("ATTESTATION_ANCHOR_MISMATCH"));
}
