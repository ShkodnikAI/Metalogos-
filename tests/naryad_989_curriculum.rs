//! №989 (ADR-0200 W4) — the curriculum leak-suite: the held-out
//! invariant, the filter word, the quarantine columns, the
//! informational filter-mismatch code and the signed/unsigned surfacing.
//!
//! The W4 lane's Never-cut rule (ADR-0200 §5): the held-out the teacher
//! did not generate never enters training. Every training-set assembly
//! point is pinned here: the curriculum build (the propose-and-refute
//! core + the crossing refusal), the attestation manifest
//! (`data_manifest` — the save/verify SSOT both backends share), the
//! filter word (the pure function `dataset.filter` writes) and the
//! verify verdict (the informational `[ATTESTATION_FILTER_CHANGED]`
//! code + the level surfacing).

use metalogos::distill_hub::{curriculum_build, ensure_distill_store, DISTILL_SOURCE_HELD_OUT};
use metalogos::nn::attest::{self, AttestPolicy, AttestSpecInput, CODE_FILTER_CHANGED};
use metalogos::nn::{persist, ActivationKind, Dense, ReflexModel, ReflexRegistry};
use serial_test::serial;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

// ── fixtures ─────────────────────────────────────────────────────────

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

/// Seed a W4-schema store: `rows` are (input, output, source) teacher /
/// held-out pairs for the "Classify" pattern.
#[allow(clippy::disallowed_methods)]
fn open_store(path: &std::path::Path) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open(path).unwrap();
    ensure_distill_store(&conn).unwrap();
    conn
}

#[allow(clippy::disallowed_methods)]
fn seed_rows(conn: &rusqlite::Connection, rows: &[(&str, &str, &str)]) {
    // Schema-aware insert: on a W4 store the quarantine columns ride the
    // row defaults; on a pre-W4 store (the compat fixture) only the five
    // original columns exist.
    let w4 = {
        let mut stmt = conn.prepare("PRAGMA table_info(distill_samples)").unwrap();
        let names: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(|n| n.ok())
            .collect();
        names.iter().any(|n| n == "quarantined")
    };
    for (input, output, source) in rows {
        if w4 {
            conn.execute(
                "INSERT INTO distill_samples (pattern_name, input, output, source, created_at, reviewed, reason, quarantined) \
                 VALUES ('Classify', ?1, ?2, ?3, 0, 0, NULL, 0)",
                rusqlite::params![input, output, source],
            )
            .unwrap();
        } else {
            conn.execute(
                "INSERT INTO distill_samples (pattern_name, input, output, source, created_at) \
                 VALUES ('Classify', ?1, ?2, ?3, 0)",
                rusqlite::params![input, output, source],
            )
            .unwrap();
        }
    }
}

/// The audit sink the hub tests share: the collecting closure + the
/// captured lines.
type AuditSink = (Arc<dyn Fn(String) + Send + Sync>, Arc<Mutex<Vec<String>>>);

fn audit_sink() -> AuditSink {
    let lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&lines);
    (
        Arc::new(move |line: String| {
            sink.lock().unwrap().push(line);
        }),
        lines,
    )
}

// ── the refutation: the held-out counterexample quarantines the unit ──

#[test]
fn curriculum_refutation_quarantines_with_machine_id() {
    let temp = TempDir::new().unwrap();
    #[allow(clippy::disallowed_methods)]
    let conn = open_store(&temp.path().join("s.db"));
    seed_rows(
        &conn,
        &[
            ("row A", "x1", "llm-response"),
            ("row B", "x2", "llm-response"),
            ("row C", "x3", "llm-response"),
            ("row C", "z3", DISTILL_SOURCE_HELD_OUT), // the counterexample (rowid 4)
            ("row D", "z4", DISTILL_SOURCE_HELD_OUT),
        ],
    );

    // The audit journal is captured by the same sink form the hub tests
    // use; curriculum_build takes the plain FnMut form, so the Arc sink
    // wraps into a closure.
    let (sink, lines) = audit_sink();
    let mut audit = move |line: String| sink(line);
    let v = curriculum_build(&conn, "Classify", &mut audit).expect("build");

    assert_eq!(v.proposed, 3, "three teacher units proposed");
    assert_eq!(v.survived, 2, "row C is refuted, A and B survive");
    assert_eq!(v.quarantined, 1);
    assert_eq!(v.held_out, 2);
    assert_eq!(v.quarantine_ids.len(), 1);
    assert!(
        v.quarantine_ids[0].starts_with("dq-3-"),
        "the machine id anchors the physical rowid 3: {}",
        v.quarantine_ids[0]
    );
    assert!(
        v.filter_word.starts_with("curriculum:"),
        "the quarantine moves the store into the W4 state: {}",
        v.filter_word
    );

    // The row itself: the three columns, the reason naming the
    // counterexample's rowid, the audit journal carrying both lines.
    #[allow(clippy::disallowed_methods)]
    let (reviewed, reason, quarantined): (i64, String, i64) = conn
        .query_row(
            "SELECT reviewed, reason, quarantined FROM distill_samples WHERE rowid = 3",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!((reviewed, quarantined), (0, 1));
    assert!(
        reason.contains("contradicted-by-held-out") && reason.contains("counterexample 4"),
        "the reason names the held-out counterexample: {}",
        reason
    );
    let journal = lines.lock().unwrap().join("\n");
    assert!(
        journal.contains("distill.curriculum-quarantine") && journal.contains("dq-3-"),
        "the quarantine rides the audit journal with the machine id"
    );
    assert!(
        journal.contains("\"gate\":\"curriculum-build\""),
        "the gate line rides the journal: the gate-facts-sync form"
    );
}

// ── the crossing refusal ─────────────────────────────────────────────

#[test]
fn curriculum_crossing_refusal_is_loud() {
    let temp = TempDir::new().unwrap();
    #[allow(clippy::disallowed_methods)]
    let conn = open_store(&temp.path().join("s.db"));
    seed_rows(
        &conn,
        &[
            ("row A", "x1", "llm-response"),
            ("row A", "x1", DISTILL_SOURCE_HELD_OUT), // the exact pair the teacher taught
        ],
    );
    let mut audit = |_line: String| {};
    let err = curriculum_build(&conn, "Classify", &mut audit).unwrap_err();
    assert!(
        err.contains("crossing refusal"),
        "the held-out provenance story is machine-false — the loud refusal: {}",
        err
    );
    // Nothing moved: the refusal is a no-op on the store.
    #[allow(clippy::disallowed_methods)]
    let (q, r): (i64, Option<String>) = conn
        .query_row(
            "SELECT quarantined, reason FROM distill_samples WHERE rowid = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!((q, r), (0, None), "the refusal quarantines nothing");
}

// ── the teacher self-contradiction: first-teacher-wins ───────────────

#[test]
fn curriculum_self_contradiction_keeps_first() {
    let temp = TempDir::new().unwrap();
    #[allow(clippy::disallowed_methods)]
    let conn = open_store(&temp.path().join("s.db"));
    seed_rows(
        &conn,
        &[
            ("row A", "first", "llm-response"),
            ("row A", "second", "llm-response"),
        ],
    );
    let mut audit = |_line: String| {};
    let v = curriculum_build(&conn, "Classify", &mut audit).expect("build");
    assert_eq!((v.proposed, v.survived, v.quarantined), (2, 1, 1));
    #[allow(clippy::disallowed_methods)]
    let (rid, reason): (i64, String) = conn
        .query_row(
            "SELECT rowid, reason FROM distill_samples WHERE quarantined = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(rid, 2, "the LATER row loses (first-teacher-wins)");
    assert!(reason.contains("teacher-self-contradiction"), "{}", reason);
    assert!(
        v.filter_word.starts_with("curriculum:"),
        "the quarantine alone moves the filter word (no held-out needed): {}",
        v.filter_word
    );
}

// ── the held-out never enters the training manifest ──────────────────

#[test]
fn held_out_never_enters_training_manifest() {
    let temp = TempDir::new().unwrap();
    #[allow(clippy::disallowed_methods)]
    let conn = open_store(&temp.path().join("s.db"));
    seed_rows(
        &conn,
        &[
            ("row A", "x1", "llm-response"),
            ("row B", "x2", "llm-response"),
            ("row H", "z", DISTILL_SOURCE_HELD_OUT),
        ],
    );
    let mut audit = |_line: String| {};
    curriculum_build(&conn, "Classify", &mut audit).expect("build");

    // The manifest the certificate hashes: exactly the surviving
    // teacher rows — the held-out row is invisible to training.
    let m = attest::data_manifest(&conn, Some("Classify"), &[]);
    assert_eq!(m.rows, 2, "the held-out row never enters the manifest");
    assert_eq!(m.provenance, "distill_samples");
    assert!(m.filter.starts_with("curriculum:"));

    // The pre-W4 schema stays byte-compatible: a store without the
    // quarantine columns reads ALL rows and says "none" (the honest
    // pre-W4 shape — the old certificate story is untouched).
    let temp2 = TempDir::new().unwrap();
    #[allow(clippy::disallowed_methods)]
    {
        let old = rusqlite::Connection::open(temp2.path().join("old.db")).unwrap();
        old.execute_batch(
            "CREATE TABLE distill_samples (pattern_name TEXT NOT NULL, input TEXT NOT NULL, \
             output TEXT NOT NULL, source TEXT NOT NULL, created_at INTEGER NOT NULL);",
        )
        .unwrap();
        seed_rows(&old, &[("row A", "x1", "llm-response")]);
        let m2 = attest::data_manifest(&old, Some("Classify"), &[]);
        assert_eq!(m2.rows, 1);
        assert_eq!(m2.filter, "none", "no W4 columns — the pre-W4 word");
    }
}

// ── the filter word is pure, order-independent and state-moving ──────

#[test]
fn filter_word_is_pure_and_moves_with_state() {
    let temp_a = TempDir::new().unwrap();
    #[allow(clippy::disallowed_methods)]
    let conn_a = open_store(&temp_a.path().join("a.db"));
    seed_rows(
        &conn_a,
        &[
            ("row A", "x1", "llm-response"),
            ("row B", "x2", "llm-response"),
        ],
    );
    // No curriculum state yet — the honest word stays "none".
    assert_eq!(
        attest::data_manifest(&conn_a, Some("Classify"), &[]).filter,
        "none"
    );

    // The same surviving set in a DIFFERENT rowid order — the same word
    // (the word hashes the sorted unit list, not the physical rows).
    let temp_b = TempDir::new().unwrap();
    #[allow(clippy::disallowed_methods)]
    let conn_b = open_store(&temp_b.path().join("b.db"));
    seed_rows(
        &conn_b,
        &[
            ("row B", "x2", "llm-response"),
            ("row A", "x1", "llm-response"),
        ],
    );
    let word_a = attest::curriculum_filter_word(&conn_a, Some("Classify"));
    assert_eq!(
        attest::curriculum_filter_word(&conn_b, Some("Classify")),
        word_a,
        "order-independent: the word is a pure function of the unit set"
    );
    assert_eq!(word_a, "none", "surviving rows alone are NOT W4 state");

    // Quarantine one row in A — the word goes live; the stable suffix
    // is 16 hex chars; a recompute is byte-identical.
    seed_rows(
        &conn_a,
        &[
            ("row C", "x3", "llm-response"), // the unit the held-out refutes
            ("row C", "OTHER", DISTILL_SOURCE_HELD_OUT), // the counterexample
        ],
    );
    let mut audit = |_line: String| {};
    let v = curriculum_build(&conn_a, "Classify", &mut audit).expect("build");
    assert_eq!(v.quarantined, 1, "the C-unit is refuted, A and B survive");
    let word = v.filter_word;
    assert!(word.starts_with("curriculum:"));
    assert_eq!(word.len(), "curriculum:".len() + 16);
    assert_eq!(
        attest::curriculum_filter_word(&conn_a, Some("Classify")),
        word,
        "pure: the recompute agrees with the build's word"
    );
}

// ── the verify side: the informational filter code + the level ───────

fn save_and_verify(db: &std::path::Path) -> (metalogos::nn::attest::AttestVerdict, ReflexModel) {
    let (registry, id) = {
        let mut registry = ReflexRegistry::new();
        let model = make_model("SmokeHead", 42);
        let mid = registry.register(model);
        (registry, mid)
    };
    let model = registry
        .get_dense(id)
        .unwrap()
        .snapshot_clone()
        .expect("snapshot");
    let input = metalogos::nn::attest::AttestInput {
        spec: spec_for("classify the text"),
        examples: vec![], // empty → the manifest rides distill_samples
    };
    persist::save_model_to_db(&model, "SmokeHead", db, Some(&input)).expect("save");
    #[allow(clippy::disallowed_methods)]
    let conn = rusqlite::Connection::open(db).unwrap();
    let canonical = attest::canonical_spec_json(&model, &input.spec);
    let s_hash = metalogos::vision::provenance::sha256_hex(canonical.as_bytes());
    let blob = persist::serialize_for_storage(&model);
    let verdict = attest::verify_attestation(
        &conn,
        "SmokeHead",
        &blob,
        &model.feature_signature,
        &canonical,
        &s_hash,
        &input.examples,
        input.spec.pattern_name.as_deref(),
        AttestPolicy::lenient(),
    )
    .unwrap();
    (verdict, model)
}

#[test]
fn verify_filter_mismatch_is_informational_not_a_refusal() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("s.db");
    // The store carries plain teacher rows at save time — the
    // certificate honestly says "none".
    #[allow(clippy::disallowed_methods)]
    {
        let conn = open_store(&db);
        seed_rows(
            &conn,
            &[
                ("row A", "x1", "llm-response"),
                ("row B", "x2", "llm-response"),
            ],
        );
    }
    let (v1, model) = save_and_verify(&db);
    assert!(v1.ok, "the consistent pre-refutation story: {:?}", v1);
    assert_eq!(v1.level, "unfiltered", "decision 5: the W1 data level");
    let cert1: serde_json::Value = serde_json::from_str(&v1.cert_json).unwrap();
    assert_eq!(cert1["dataset"]["filter"], "none");

    // The curriculum build quarantines one unit — the store's word moves;
    // the STORED certificate keeps its story. The verify must stay ok
    // (the data_hash form) and NAME the move with the code.
    #[allow(clippy::disallowed_methods)]
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        seed_rows(&conn, &[("row B", "DIFFERENT", DISTILL_SOURCE_HELD_OUT)]);
        let mut audit = |_line: String| {};
        curriculum_build(&conn, "Classify", &mut audit).expect("build");
    }
    #[allow(clippy::disallowed_methods)]
    let conn = rusqlite::Connection::open(&db).unwrap();
    let canonical = attest::canonical_spec_json(&model, &spec_for("classify the text"));
    let s_hash = metalogos::vision::provenance::sha256_hex(canonical.as_bytes());
    let blob = persist::serialize_for_storage(&model);
    let v2 = attest::verify_attestation(
        &conn,
        "SmokeHead",
        &blob,
        &model.feature_signature,
        &canonical,
        &s_hash,
        &[],
        Some("Classify"),
        AttestPolicy::lenient(),
    )
    .unwrap();
    assert!(
        v2.ok,
        "a filter move is informational — never a refusal: {:?}",
        v2.reason
    );
    assert!(
        v2.reason.contains(CODE_FILTER_CHANGED),
        "the code names the move: {}",
        v2.reason
    );
    assert!(
        v2.reason.contains("the certificate stores 'none'"),
        "both words ride the reason: {}",
        v2.reason
    );
    assert_eq!(
        v2.level, "unfiltered",
        "the STORED word decides the data level — the stored story is 'none'"
    );
}

#[test]
#[serial]
fn verify_surfaces_signed_unsigned_once_the_filter_is_live() {
    // Save AFTER a curriculum state exists — the certificate itself
    // carries a live filter word, and the verify output level surfaces
    // the certificate's own level (the §7.4 record: signed/unsigned
    // from W4 on). Without a configured ledger key: "unsigned".
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("s.db");
    #[allow(clippy::disallowed_methods)]
    {
        let conn = open_store(&db);
        seed_rows(
            &conn,
            &[
                ("row A", "x1", "llm-response"),
                ("row B", "x2", DISTILL_SOURCE_HELD_OUT),
            ],
        );
    }
    let (v, _model) = save_and_verify(&db);
    assert!(v.ok, "{:?}", v.reason);
    let cert: serde_json::Value = serde_json::from_str(&v.cert_json).unwrap();
    let filter_word = cert["dataset"]["filter"].as_str().unwrap().to_string();
    assert!(
        filter_word.starts_with("curriculum:"),
        "the W4 state at save time writes the live word: {}",
        filter_word
    );
    assert_eq!(
        v.level, "unsigned",
        "no ledger key configured — the honest certificate level surfaces"
    );

    // With the key: the anchor is signed and the level reads "signed".
    let temp2 = TempDir::new().unwrap();
    let db2 = temp2.path().join("k.db");
    // The W4 state must exist at SAVE time — the certificate's own
    // filter word is what surfaces as the level.
    #[allow(clippy::disallowed_methods)]
    {
        let conn2 = open_store(&db2);
        seed_rows(
            &conn2,
            &[
                ("row A", "x1", "llm-response"),
                ("row B", "x2", DISTILL_SOURCE_HELD_OUT),
            ],
        );
    }
    std::env::set_var("METALOGOS_LEDGER_KEY", "n989-leak-suite-key");
    let (v2, _m2) = save_and_verify(&db2);
    std::env::remove_var("METALOGOS_LEDGER_KEY");
    assert!(v2.ok, "{:?}", v2.reason);
    assert_eq!(
        v2.level, "signed",
        "the §7.4 surfacing: the signed certificate's own level with a live filter"
    );
}
