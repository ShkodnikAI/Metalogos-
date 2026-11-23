//! ── Naryad №998 (issue #2054, ADR-0200 W9, the dispatch wave В104 —
//! W-AH): the LINE E2E — the W1–W7 CLI surface against the REAL binary ──
//!
//! The W9 slot's e2e: every test drives the compiled `mlog` (no mocks of
//! the surface under test; the LLM is never reached — the line's fixtures
//! train directly on labeled rows). Each refusal test is RED-able: break
//! the gate it names and the assertion fails, because the test asserts the
//! REFUSAL itself (the code, the non-zero exit, the zero-writes bytes),
//! not a success-shaped smoke.
//!
//! The §11.1 check_id coverage — 10 of 15 honestly reachable at the CLI
//! surface, each pinned by one refusal assertion here:
//!   ATTESTATION_STALE                     → spec_drift_refuses_stale_and_allow_stale_bypasses
//!   ATTESTATION_EMBED_MISMATCH            → embed_signature_move_refuses
//!   ATTESTATION_WEIGHTS_CHANGED           → weights_tamper_refuses (the W1 never-cut pin)
//!   ATTESTATION_NONE                      → no_certificate (the verify refusal) + the enablement NONE
//!   ATTESTATION_UNSIGNED                  → save_then_verify (the level surfacing + the strict flip)
//!   ATTESTATION_TEACHER_MISMATCH          → forged_teacher_word_refuses (the W3b never-cut pin)
//!   ATTESTATION_ENABLEMENT_UNFILTERED     → enablement_refuses_unfiltered_with_zero_writes
//!   ATTESTATION_ENABLEMENT_TEACHER_POLICY → enablement_refuses_the_nonpermissive_teacher (W7)
//!   DISTILL_FROZEN                        → freeze_interlocks_the_build (the W5 never-cut pin)
//!   DISTILL_SOURCE_HELD_OUT (the word)    → curriculum_build_holds_out (the W4 never-cut pin)
//!
//! The five check_ids NOT honestly reachable at the CLI are named in
//! ADR-0200 §13 with their existing SSOT/lib pins: ATTESTATION_DATA_CHANGED
//! (the informational note), ATTESTATION_FILTER_CHANGED (the load note),
//! ATTESTATION_ANCHOR_MISMATCH (the live-anchor ledger check — the CLI
//! verify runs against the process-local ledger), DISTILL_SHADOW (the
//! event line), DISTILL_TEACHER_UNLICENSED (the training-move gate; the
//! SSOT registry tests of №975 pin it).
//!
//! The tamper simulation rides rusqlite UPDATEs on the store — the honest
//! way to hand the binary a broken world; the tests never patch the gates,
//! they break the ARTIFACTS. (The one cert_json filter-word rewrite in
//! `enablement_refuses_the_nonpermissive_teacher` simulates the honest
//! state AFTER a filtered re-save — the state the CLI cannot reach without
//! an LLM teacher — so the W7 policy gate can be exercised past the
//! decision-5 level check.)

#![allow(clippy::disallowed_methods)]

use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::process::Command;

fn mlog() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_mlog"))
}

/// The demo program: the direct train (no LLM — deterministic), the save
/// (the certificate + the ledger anchor in one transaction), the inline
/// verify. The reflex model and the learnable pattern share the name
/// `Classify` — the attestation model name IS the pattern name (the same
/// shape as examples/distill/line_demo.mlog, W9).
const DEMO_MLOG: &str = r#"
memory { persist: "line.db" }

reflex Classify {
  input: embedding(4)
  layers: [dense(8, "relu"), dense(2, "softmax")]
  labels: ["pos", "neg"]
  seed: 42
}

learnable pattern Classify(text: String) -> String {
  prompt: "classify the text"
  distill_to: Classify
  distill_after: 100
}

pattern Drive(_x: String) -> String {
  let data = [
    [0.0, 0.1, 0.2, 0.0, 0.0],
    [0.2, 0.0, 0.1, 0.1, 0.0],
    [0.1, 0.2, 0.0, 0.2, 0.0],
    [5.0, 5.1, 5.2, 5.0, 1.0],
    [5.2, 5.0, 5.1, 5.1, 1.0],
    [5.1, 5.2, 5.0, 5.2, 1.0],
    [0.3, 0.0, 0.1, 0.0, 0.0],
    [0.0, 0.3, 0.2, 0.1, 0.0],
    [5.3, 5.0, 5.1, 5.0, 1.0],
    [5.0, 5.3, 5.2, 5.1, 1.0],
    [5.2, 5.1, 5.3, 5.2, 1.0],
    [5.1, 5.2, 5.0, 5.3, 1.0]
  ]
  let _t = reflex_train(Classify, data, 200.0, "accuracy", 0.85)
  let _s = reflex_save(Classify)
  let v = attest_verify(Classify)
  return "ok=" + to_string(v.ok) + ";level=" + v.level
}

flow Main { input: String = "x" -> Drive -> output }
"#;

/// A declared-but-never-saved program: the store exists, the certificate
/// does not (the lenient-warn vs strict-refuse vs verify-refuse triple).
const UNSAVED_MLOG: &str = r#"
memory { persist: "unsaved.db" }

reflex BareHead {
  input: embedding(4)
  layers: [dense(8, "relu"), dense(2, "softmax")]
  labels: ["pos", "neg"]
  seed: 42
}

pattern Drive(_x: String) -> String {
  let v = attest_verify(BareHead)
  return "ok=" + to_string(v.ok) + ";level=" + v.level
}

flow Main { input: String = "x" -> Drive -> output }
"#;

const HELD_OUT: &str = "classify: glowing five stars\tpos\n\
                        classify: broke immediately\tneg\n\
                        classify: solid battery\t pos\n";

struct Out {
    success: bool,
    text: String,
}

fn run_in(cwd: &Path, envs: &[(&str, &str)], args: &[&str]) -> Out {
    let mut cmd = Command::new(mlog());
    cmd.args(args).current_dir(cwd);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("the mlog binary must spawn");
    Out {
        success: out.status.success(),
        text: format!(
            "--- stdout ---\n{}--- stderr ---\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    }
}

/// Train + save the demo model in `dir`; the DB lands at dir/line.db.
#[allow(clippy::disallowed_methods)]
fn train_and_save(dir: &Path) -> PathBuf {
    std::fs::write(dir.join("line.mlog"), DEMO_MLOG).unwrap();
    let out = run_in(dir, &[], &["run", "line.mlog"]);
    assert!(
        out.success,
        "the demo program must train+save cleanly:\n{}",
        out.text
    );
    assert!(
        out.text.contains("ok=true;level="),
        "the inline verify must surface the level:\n{}",
        out.text
    );
    dir.join("line.db")
}

fn db_conn(db: &Path) -> Connection {
    Connection::open(db).expect("the store must open")
}

fn cert_json(conn: &Connection) -> String {
    conn.query_row(
        "SELECT cert_json FROM reflex_attestation WHERE model = 'Classify' \
         AND superseded = 0 ORDER BY seq DESC LIMIT 1",
        [],
        |r| r.get::<_, String>(0),
    )
    .expect("the certificate row exists")
}

fn write_held_out(dir: &Path) {
    std::fs::write(dir.join("held_out.tsv"), HELD_OUT).unwrap();
}

fn craft_research(dir: &Path) -> String {
    let root = dir.join("research");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("e3-confidence-calibration.json"),
        "{\"experiment\":\"e3\",\"ece\":0.42,\"bins\":[]}",
    )
    .unwrap();
    std::fs::write(
        root.join("e4-teacher-baseline-agreement.json"),
        "{\"experiment\":\"e4\",\"teacher_rows\":[{\"backend\":\"mock-deterministic\",\"agreement_with_majority_baseline\":0.0,\"reason\":\"the W6 enablement gate must not read it as one\"}]}",
    )
    .unwrap();
    root.to_string_lossy().to_string()
}

// ── W1: the positive path + the unsigned level + the strict flip ─────

#[test]
fn n998_save_then_verify_is_provenance_consistent_and_strict_flips_the_verdict() {
    let dir = tempfile::tempdir().unwrap();
    let db = train_and_save(dir.path());
    assert!(db.exists(), "the save must land the store");

    // The lenient default (the transitional window, decision 4): the
    // provenance story holds; the level is surfaced, never hidden.
    let out = run_in(
        dir.path(),
        &[],
        &["attest", "verify", "line.mlog", "--json"],
    );
    assert!(out.success, "the fresh save verifies:\n{}", out.text);
    assert!(
        out.text.contains("\"ok\":true"),
        "provenance-consistent:\n{}",
        out.text
    );
    assert!(
        out.text.contains("\"level\":\"unsigned\"") || out.text.contains("\"level\":\"signed\""),
        "the level is surfaced:\n{}",
        out.text
    );

    // The strict flip (decision 4): the SAME story under
    // METALOGOS_ATTEST_STRICT=1 — the unsigned certificate is no longer
    // waved through: the inline verify verdict flips to ok=false.
    let strict = run_in(
        dir.path(),
        &[("METALOGOS_ATTEST_STRICT", "1")],
        &["run", "line.mlog"],
    );
    assert!(
        strict.text.contains("ok=false;level=unsigned"),
        "the strict policy flips the unsigned verdict (RED-able on the \
         decision-4 gate):\n{}",
        strict.text
    );
}

// ── W1: ATTESTATION_STALE + the loud bypass ──────────────────────────

#[test]
fn n998_spec_drift_refuses_stale_and_allow_stale_bypasses_loudly() {
    let dir = tempfile::tempdir().unwrap();
    train_and_save(dir.path());

    // The spec drift: the declaration moves under the saved certificate.
    let drifted = DEMO_MLOG.replace("seed: 42", "seed: 43");
    std::fs::write(dir.path().join("drifted.mlog"), drifted).unwrap();
    let out = run_in(dir.path(), &[], &["attest", "verify", "drifted.mlog"]);
    assert!(
        !out.success,
        "the drifted spec must refuse (RED-able on the STALE gate):\n{}",
        out.text
    );
    assert!(
        out.text.contains("[ATTESTATION_STALE]"),
        "the stale code is named:\n{}",
        out.text
    );

    // The bypass is loud, not silent: --allow-stale passes AND announces.
    let bypass = run_in(
        dir.path(),
        &[],
        &[
            "attest",
            "verify",
            "drifted.mlog",
            "--allow-stale",
            "--json",
        ],
    );
    assert!(
        bypass.success,
        "the explicit bypass downgrades the verdict:\n{}",
        bypass.text
    );
    assert!(
        bypass.text.contains("allow") || bypass.text.contains("stale"),
        "the bypass stays visible in the machine verdict:\n{}",
        bypass.text
    );
}

// ── W1 never-cut: the weights bytes are covered by the certificate ───

#[test]
fn n998_weights_tamper_refuses_weights_changed() {
    let dir = tempfile::tempdir().unwrap();
    let db = train_and_save(dir.path());

    // Flip the first weights byte in the store (CAST keeps the column a
    // BLOB — SQLite's substr would coerce to TEXT otherwise). The
    // certificate must notice.
    let conn = db_conn(&db);
    assert_eq!(
        conn.execute(
            "UPDATE reflex_models SET weights = \
             CAST(X'00' || substr(weights, 2, length(weights)) AS BLOB) \
             WHERE name = 'Classify'",
            [],
        )
        .expect("the tamper update applies"),
        1,
        "the tampered row is the demo model"
    );
    drop(conn);

    let out = run_in(dir.path(), &[], &["attest", "verify", "line.mlog"]);
    assert!(
        !out.success,
        "tampered weights must refuse (RED-able on the W1 never-cut lane):\n{}",
        out.text
    );
    assert!(
        out.text.contains("[ATTESTATION_WEIGHTS_CHANGED]"),
        "the weights code is named:\n{}",
        out.text
    );
}

#[test]
fn n998_embed_signature_move_refuses_embed_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let db = train_and_save(dir.path());

    let conn = db_conn(&db);
    assert_eq!(
        conn.execute(
            "UPDATE reflex_models SET feature_signature = 'moved-signature' \
             WHERE name = 'Classify'",
            [],
        )
        .expect("the tamper update applies"),
        1
    );
    drop(conn);

    let out = run_in(dir.path(), &[], &["attest", "verify", "line.mlog"]);
    assert!(
        !out.success,
        "a moved feature-extractor signature must refuse:\n{}",
        out.text
    );
    assert!(
        out.text.contains("[ATTESTATION_EMBED_MISMATCH]"),
        "the embed code is named:\n{}",
        out.text
    );
}

// ── W3b never-cut: the forged provenance story ───────────────────────

#[test]
fn n998_forged_teacher_word_refuses_teacher_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let db = train_and_save(dir.path());

    // Rewrite the teacher's license class into a word no registry record
    // carries — the provenance story no longer parses.
    let conn = db_conn(&db);
    let cert = cert_json(&conn);
    let forged = cert.replace("restrictive", "restrictionless-unknown-class");
    assert_ne!(cert, forged, "the fixture must actually change the story");
    conn.execute(
        "UPDATE reflex_attestation SET cert_json = ?1 WHERE model = 'Classify' \
         AND superseded = 0",
        [&forged],
    )
    .expect("the forged write applies");
    drop(conn);

    let out = run_in(dir.path(), &[], &["attest", "verify", "line.mlog"]);
    assert!(
        !out.success,
        "the forged teacher story must refuse (RED-able on the W3b gate):\n{}",
        out.text
    );
    assert!(
        out.text.contains("[ATTESTATION_TEACHER_MISMATCH]"),
        "the teacher-mismatch code is named:\n{}",
        out.text
    );
}

// ── decision 4: no certificate — verify refuses, the lenient warns ────

#[test]
fn n998_no_certificate_verify_refuses_and_lenient_run_warns() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("unsaved.mlog"), UNSAVED_MLOG).unwrap();

    // The CLI verify of a certificate-less model: a loud refusal.
    let verify = run_in(dir.path(), &[], &["attest", "verify", "unsaved.mlog"]);
    assert!(
        !verify.success,
        "the verify of a certificate-less model refuses (RED-able on the \
         NONE gate):\n{}",
        verify.text
    );
    assert!(
        verify.text.contains("ATTESTATION_NONE"),
        "the NONE code is named:\n{}",
        verify.text
    );

    // The lenient run: the program runs, the missing certificate is
    // announced (the inline verify honestly reports level=none; the load
    // path warns — never silent).
    let lenient = run_in(dir.path(), &[], &["run", "unsaved.mlog"]);
    assert!(
        lenient.success,
        "the lenient default keeps the transitional window:\n{}",
        lenient.text
    );
    assert!(
        lenient.text.contains("ok=false;level=none"),
        "the inline verify honestly reports the missing level:\n{}",
        lenient.text
    );
    assert!(
        lenient.text.to_lowercase().contains("warn") || lenient.text.contains("cert"),
        "the warning word is present (never silent):\n{}",
        lenient.text
    );
}

// ── W4 never-cut: the held-out never trains ──────────────────────────

#[test]
fn n998_curriculum_build_holds_out_and_never_trains_it() {
    let dir = tempfile::tempdir().unwrap();
    let db = train_and_save(dir.path());
    write_held_out(dir.path());

    let out = run_in(
        dir.path(),
        &[],
        &[
            "curriculum",
            "build",
            "line.mlog",
            "--held-out",
            "held_out.tsv",
            "--json",
        ],
    );
    assert!(out.success, "the build succeeds:\n{}", out.text);
    assert!(
        out.text.contains("\"proposed\":0"),
        "no teacher-generated units exist in this store — proposed stays 0 \
         (RED-able: feed the held-out into the teacher pool and it becomes 3):\n{}",
        out.text
    );
    assert!(
        out.text.contains("\"held_out\":3"),
        "the held-out rows are counted as loaded:\n{}",
        out.text
    );
    assert!(
        out.text.contains("\"filter\":\""),
        "the build surfaces the filter word:\n{}",
        out.text
    );

    // The invariant, read back from the store: the held-out rows KEEP the
    // held-out source — they never enter a training set.
    let conn = db_conn(&db);
    let held: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM distill_samples WHERE source = 'held-out' \
             AND pattern_name = 'Classify'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let teacher: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM distill_samples WHERE source = 'llm-response' \
             AND pattern_name = 'Classify'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(held, 3, "the held-out rows persist as held-out: {held}");
    assert_eq!(teacher, 0, "no teacher-generated rows exist: {teacher}");
}

// ── W5 never-cut: the operator lock, the read-only status, the exit ───

#[test]
fn n998_freeze_interlocks_the_build_and_status_is_read_only() {
    let dir = tempfile::tempdir().unwrap();
    let _db = train_and_save(dir.path());
    write_held_out(dir.path());

    let freeze = run_in(
        dir.path(),
        &[],
        &[
            "distill",
            "freeze",
            "--pattern",
            "Classify",
            "--db",
            "line.db",
            "--json",
        ],
    );
    assert!(
        freeze.success && freeze.text.contains("\"phase\":\"frozen\""),
        "the explicit freeze lands:\n{}",
        freeze.text
    );

    // The interlock: a distill move refuses while frozen.
    let build = run_in(
        dir.path(),
        &[],
        &[
            "curriculum",
            "build",
            "line.mlog",
            "--held-out",
            "held_out.tsv",
            "--json",
        ],
    );
    assert!(
        !build.success,
        "the frozen lane refuses the build (RED-able on the W5 interlock):\n{}",
        build.text
    );

    // The status is READ-ONLY: no transition happens on reads.
    let st1 = run_in(
        dir.path(),
        &[],
        &[
            "distill",
            "status",
            "--pattern",
            "Classify",
            "--db",
            "line.db",
            "--json",
        ],
    );
    assert!(
        st1.success && st1.text.contains("\"phase\":\"frozen\""),
        "the status reports the frozen phase:\n{}",
        st1.text
    );
    let st2 = run_in(
        dir.path(),
        &[],
        &[
            "distill",
            "status",
            "--pattern",
            "Classify",
            "--db",
            "line.db",
            "--json",
        ],
    );
    assert_eq!(
        st1.text, st2.text,
        "two reads, zero transitions — the status never mutates:\n{}\n--- second ---\n{}",
        st1.text, st2.text
    );

    // Decision 6: the explicit external unfreeze is the only exit.
    let unfreeze = run_in(
        dir.path(),
        &[],
        &[
            "distill",
            "unfreeze",
            "--pattern",
            "Classify",
            "--db",
            "line.db",
            "--json",
        ],
    );
    assert!(
        unfreeze.success && unfreeze.text.contains("\"phase\":\"active\""),
        "the unfreeze returns the lane to active:\n{}",
        unfreeze.text
    );
}

// ── W6: the enablement gate — unfiltered refuses, zero writes ────────

#[test]
fn n998_enablement_refuses_unfiltered_with_zero_writes() {
    let dir = tempfile::tempdir().unwrap();
    let db = train_and_save(dir.path());
    let research = craft_research(dir.path());

    // Snapshot the STORE CONTENT (not the file bytes — SQLite touches the
    // file header on any open): the refusal must not write a row.
    let content_snapshot = |db: &Path| -> Vec<(String, String, String)> {
        let conn = db_conn(db);
        let mut rows = Vec::new();
        let mut stmt = conn
            .prepare(
                "SELECT model, cert_json, weights_sha256 FROM reflex_attestation \
                 ORDER BY seq",
            )
            .unwrap();
        let iter = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .unwrap();
        for r in iter {
            rows.push(r.unwrap());
        }
        rows
    };
    let before = content_snapshot(&db);

    let out = run_in(
        dir.path(),
        &[],
        &[
            "distill",
            "enable-differential",
            "--pattern",
            "Classify",
            "--db",
            "line.db",
            "--research-dir",
            &research,
            "--json",
        ],
    );
    assert!(
        !out.success,
        "the unfiltered demo pattern must refuse (RED-able on the decision-5 \
         gate):\n{}",
        out.text
    );
    assert!(
        out.text.contains("[ATTESTATION_ENABLEMENT_UNFILTERED]"),
        "the unfiltered code is named:\n{}",
        out.text
    );

    let after = content_snapshot(&db);
    assert_eq!(
        before, after,
        "zero writes on refusal — the store content is identical"
    );
}

// ── W7: the enablement boundary enforces the teacher policy ──────────

#[test]
fn n998_enablement_refuses_the_nonpermissive_teacher() {
    let dir = tempfile::tempdir().unwrap();
    let db = train_and_save(dir.path());
    let research = craft_research(dir.path());

    // Move the recorded filter word INTO the curriculum state — simulating
    // the honest certificate state AFTER a filtered re-save (the state the
    // CLI cannot reach here without an LLM teacher). The decision-5 level
    // check now passes; the W7 teacher policy must still refuse: the
    // recorded teacher stays restrictive (the sandbox's honest fact).
    let conn = db_conn(&db);
    let cert = cert_json(&conn);
    let moved = cert.replace("\"filter\":\"none\"", "\"filter\":\"curriculum:deadbeef\"");
    assert_ne!(cert, moved, "the fixture must move the filter word");
    conn.execute(
        "UPDATE reflex_attestation SET cert_json = ?1 WHERE model = 'Classify' \
         AND superseded = 0",
        [&moved],
    )
    .unwrap();
    drop(conn);

    let out = run_in(
        dir.path(),
        &[],
        &[
            "distill",
            "enable-differential",
            "--pattern",
            "Classify",
            "--db",
            "line.db",
            "--research-dir",
            &research,
            "--json",
        ],
    );
    assert!(
        !out.success,
        "the honest non-permissive teacher must refuse at the enablement \
         boundary (RED-able on the W7 policy gate):\n{}",
        out.text
    );
    assert!(
        out.text.contains("[ATTESTATION_ENABLEMENT_TEACHER_POLICY]"),
        "the teacher-policy code is named:\n{}",
        out.text
    );
}
