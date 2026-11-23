//! №995 (ADR-0200 W7, the dispatch wave В102 — W-AF) — the audit-lane
//! leak-suite: the final check_id registry (the two-way machine census:
//! every code literal in src lives in the §11 registry, every machine-
//! visible registry row lives in src), the threat-model classes and the
//! limitations mirrors (IN SYNC by id), and the three W7 executions:
//! the mutate-session rollback capture ((a) — beside the certificate,
//! the bytes stable), the STALE drift signal feeding the W5 hysteresis
//! ((b) — the verify verdict untouched), and the teacher-policy
//! enforcement at the enablement boundary ((c) — decision 3).
//!
//! The never-cut discipline is pinned here: the certificate §4 bytes do
//! not move when the capture rides; the verify verdict does not change
//! when the feed fires.

use metalogos::distill_hub::{
    enable_differential, ensure_distill_store, read_lifecycle, DistillPhase,
};
use metalogos::nn::attest::{
    current_cert, AttestInput, AttestSpecInput, CODE_ENABLEMENT_TEACHER_POLICY,
    CODE_ENABLEMENT_UNFILTERED, CODE_STALE, CODE_TEACHER_MISMATCH,
};
use metalogos::nn::{persist, ActivationKind, Dense, ReflexModel, ReflexRegistry};
use serial_test::serial;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

// ── fixtures (the №994 idioms) ───────────────────────────────────────

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

#[allow(clippy::disallowed_methods)]
fn save_with_rule(name: &str, db: &Path, rule: Option<&str>) -> String {
    let mut registry = ReflexRegistry::new();
    let id = registry.register(make_model(name, 42));
    let model = registry.get_dense(id).unwrap().snapshot_clone().unwrap();
    let input = AttestInput {
        spec: spec_for("classify the text"),
        examples: vec![
            ("a".to_string(), "pos".to_string()),
            ("b".to_string(), "neg".to_string()),
        ],
        rollback_rule: rule.map(|r| r.to_string()),
    };
    persist::save_model_to_db(&model, name, db, Some(&input)).expect("save");
    let conn = rusqlite::Connection::open(db).unwrap();
    current_cert(&conn, name)
        .unwrap()
        .expect("cert exists")
        .cert_json
}

/// The filtered fixture (the №994 form): the W4 curriculum state exists
/// BEFORE the save — the effective level is the certificate's own.
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
    save_with_rule(name, db, None);
}

#[allow(clippy::disallowed_methods)]
fn craft_research_dir(root: &Path, e3_ece: Option<f64>) -> PathBuf {
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
    fs::write(
        root.join("e4-teacher-baseline-agreement.json"),
        "{\"experiment\":\"e4\",\"teacher_rows\":[\
         {\"backend\":\"mock\",\"agreement_with_majority_baseline\":0.0,\
         \"reason\":\"the mock row is not read as one\"}]}",
    )
    .unwrap();
    root.to_path_buf()
}

/// Rewrite the stored certificate's teacher block (the (c)/(d) fixtures).
/// The gate reads the STORED cert only — no re-anchor at the gate — so
/// the swap is the honest test instrument, not a product behavior.
#[allow(clippy::disallowed_methods)]
fn swap_teacher_block(db: &Path, model: &str, teacher: serde_json::Value) {
    let conn = rusqlite::Connection::open(db).unwrap();
    let cert = current_cert(&conn, model).unwrap().expect("cert").cert_json;
    let mut v: serde_json::Value = serde_json::from_str(&cert).unwrap();
    let teacher_is_null = teacher.is_null();
    v["teacher"] = teacher;
    if teacher_is_null {
        v["teacher_reason"] =
            serde_json::json!("direct reflex_train — no distill lineage, no teacher");
    }
    conn.execute(
        "UPDATE reflex_attestation SET cert_json = ?1 WHERE model = ?2 AND superseded = 0",
        rusqlite::params![serde_json::to_string(&v).unwrap(), model],
    )
    .unwrap();
}

fn teacher_entry(backend: &str, weights_id: &str, class: &str, allowed: bool) -> serde_json::Value {
    serde_json::json!({
        "backend": backend,
        "weights_id": weights_id,
        "license_class": class,
        "training_on_output_allowed": allowed,
    })
}

// ── §11: the registry / the mirrors / the threat classes (machines) ──

const ADR: &str = "docs/adr/0200-provable-lowering.md";

/// The ADR §11 table rows whose first cell is a name — parsed per section.
#[allow(clippy::disallowed_methods)]
fn registry_section(marker: &str) -> Vec<String> {
    let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(ADR)).unwrap();
    let mut out = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        if line.contains(marker) {
            inside = true;
            continue;
        }
        if inside && line.starts_with("### ") && !line.contains(marker) {
            break;
        }
        if inside && line.starts_with("| ") && !line.contains("---") {
            let name = line
                .trim_start_matches("| ")
                .split('|')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if !name.is_empty() && name != "check_id" && name != "record" && name != "variable" {
                out.push(name);
            }
        }
    }
    out
}

#[allow(clippy::disallowed_methods)]
fn repo_rs_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut out = Vec::new();
    for dir in ["src", "metalogos-server/src"] {
        let mut stack = vec![root.join(dir)];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
                    out.push(p);
                }
            }
        }
    }
    out
}

const LINE_FILES: &[&str] = &[
    "src/nn/attest.rs",
    "src/nn/persist.rs",
    "src/distill_hub.rs",
    "src/interpreter/learnable.rs",
    "src/interpreter/hooks.rs",
    "src/interpreter/reflex_builtin.rs",
    "src/backends.rs",
    "src/llm.rs",
    "src/builtins/attest.rs",
    "src/builtins/reflex.rs",
    "src/vm.rs",
    "metalogos-server/src/bin/mlog.rs",
];

/// №995 §11.1: every bracketed check_id literal in the tree lives in the
/// registry, and every machine-visible registry row (the Scan column ✓)
/// lives in the tree — the two-way census, the №559/№959 form.
#[test]
#[allow(clippy::disallowed_methods)]
fn registry_census_is_two_way_in_sync() {
    let registry: Vec<String> = registry_section("The check_id registry (§11.1)");
    assert!(
        registry.len() >= 14,
        "the §11.1 registry carries the line's check_ids (got {})",
        registry.len()
    );
    let re_bracketed = regex_lite();
    // 1. scan → registry (the line's code families only — the [AUDIT]/
    //    [TRACE] audit-line prefixes are another subsystem's vocabulary)
    for f in repo_rs_files() {
        let text = std::fs::read_to_string(&f).unwrap();
        for cap in re_bracketed.captures_iter(&text) {
            let code = cap.get(1).unwrap().as_str().to_string();
            assert!(
                registry.contains(&code),
                "{}: the bracketed code [{}] is NOT in the §11.1 registry (the new \
                 machine word lands IN THE SAME PR, the №971 rule)",
                f.display(),
                code
            );
        }
    }
    // 2. registry (the Scan ✓ rows) → tree
    let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(ADR)).unwrap();
    let mut inside = false;
    for line in text.lines() {
        if line.contains("The check_id registry (§11.1)") {
            inside = true;
            continue;
        }
        if inside && line.starts_with("### ") {
            break;
        }
        if inside && line.starts_with("| ") && !line.contains("---") {
            let cells: Vec<&str> = line.split('|').collect();
            let code = cells.get(1).unwrap_or(&"").trim();
            let scan = cells.get(5).unwrap_or(&"").trim().to_string();
            if code.is_empty() || code == "check_id" {
                continue;
            }
            if scan == "✓" {
                let bracketed = format!("[{}]", code);
                let mut found = false;
                for f in repo_rs_files() {
                    let body = std::fs::read_to_string(&f).unwrap();
                    // The emission form is either the bracketed literal
                    // ([CODE]) or the bare word (the W3b house form —
                    // the const's value IS the bare word); both count.
                    if body.contains(&bracketed) || body.contains(&format!("\"{}\"", code)) {
                        found = true;
                        break;
                    }
                }
                assert!(
                    found,
                    "the §11.1 registry row {} is marked machine-visible (✓) but its \
                     literal is NOT in the tree — the registry states more than the \
                     tree supports (the №959 class)",
                    code
                );
            }
        }
    }
}

/// The lightweight regex stand-in (std-only): find the
/// `\[(ATTESTATION|DISTILL)_[A-Z_]+\]` family tokens.
fn regex_lite() -> &'static regex::Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"\[(ATTESTATION_[A-Z_]+|DISTILL_[A-Z_]+)\]").unwrap())
}

/// №995 §11.2/§11.3: the line's ledger records and env variables are
/// covered two-way over the line's files (the scan scope named in §11).
#[test]
#[allow(clippy::disallowed_methods)]
fn ledger_and_env_rows_are_two_way_in_sync() {
    let records: Vec<String> = registry_section("The line's ledger records (§11.2)");
    let envs: Vec<String> = registry_section("The line's environment variables (§11.3)");
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut bodies = Vec::new();
    for rel in LINE_FILES {
        bodies.push(std::fs::read_to_string(root.join(rel)).unwrap());
    }
    let joined = bodies.join("\n");
    // env → registry
    for m in regex::Regex::new(r"METALOGOS_(ATTEST|DISTILL)_[A-Z_]+|METALOGOS_TRACE_DISTILL")
        .unwrap()
        .find_iter(&joined)
    {
        let name = m.as_str().to_string();
        assert!(
            envs.contains(&name),
            "the env literal {} is used by the line's files but NOT in §11.3",
            name
        );
    }
    // ledger verbs → registry
    for m in regex::Regex::new(r#""(distill|reflex)\.[a-z][a-z-]+""#)
        .unwrap()
        .find_iter(&joined)
    {
        let verb = m.as_str().trim_matches('"').to_string();
        assert!(
            records.contains(&verb),
            "the ledger record {} is emitted by the line's files but NOT in §11.2",
            verb
        );
    }
    // registry → tree (both sections)
    for (rows, section) in [(&records, "§11.2"), (&envs, "§11.3")] {
        for name in rows {
            assert!(
                joined.contains(&format!("\"{}\"", name)) || joined.contains(name),
                "the {} registry row {} has no literal in the line's files — the \
                 registry states more than the tree supports (the №959 class)",
                section,
                name
            );
        }
    }
}

/// №995 §11.4: the limitations mirrors — every LIM-AT id in the ADR W7
/// record has the ACTIVE mirror row in limitations.md (the №674 form,
/// IN SYNC by id).
#[test]
#[allow(clippy::disallowed_methods)]
fn limitations_mirrors_are_in_sync() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let adr = std::fs::read_to_string(root.join(ADR)).unwrap();
    let lim = std::fs::read_to_string(root.join("docs/limitations.md")).unwrap();
    let re = regex::Regex::new(r"LIM-AT-\d+").unwrap();
    let adr_ids: Vec<String> = re
        .find_iter(&adr)
        .map(|m| m.as_str().to_string())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    assert!(
        adr_ids.len() >= 5,
        "the W7 record carries the line's active limitations by id (got {:?})",
        adr_ids
    );
    for id in adr_ids {
        assert!(
            lim.contains(&id),
            "the limitation {} is named in ADR-0200 §11 but has NO mirror row in \
             docs/limitations.md — the mirror is IN SYNC by id or the drift is loud",
            id
        );
    }
}

/// №995 §11.5: the threat-model classes — every TM-AT id in the dated
/// section of threat-model.md is mirrored in the ADR W7 record (the
/// №985 form: each row names its date).
#[test]
#[allow(clippy::disallowed_methods)]
fn threat_model_classes_are_dated_and_mirrored() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tm = std::fs::read_to_string(root.join("docs/threat-model.md")).unwrap();
    let adr = std::fs::read_to_string(root.join(ADR)).unwrap();
    let section = tm
        .split("The attestation line")
        .nth(1)
        .expect("the threat-model carries the attestation-line section (W7)");
    let re = regex::Regex::new(r"TM-AT-\d+").unwrap();
    let ids: Vec<String> = re
        .find_iter(section)
        .map(|m| m.as_str().to_string())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    assert!(ids.len() >= 4, "the dated threat classes are present");
    assert!(
        section.contains("2026-11-23"),
        "the class rows carry their date (the №985 form)"
    );
    for id in ids {
        assert!(
            adr.contains(&id),
            "the threat class {} has no ADR-0200 §11 mirror",
            id
        );
    }
}

// ── (a): the rollback capture ────────────────────────────────────────

/// №995 (a): the capture rides BESIDE the certificate — the §4 content is
/// stable (the W1 never-cut lane), the column carries the canonical word,
/// the ledger anchors its sha (the tamper-evident mirror).
#[test]
#[serial]
#[allow(clippy::disallowed_methods)]
fn rollback_capture_beside_certificate_bytes_stable() {
    let temp = TempDir::new().unwrap();
    let without = save_with_rule("Classify", temp.path().join("a.db").as_path(), None);
    let with = save_with_rule(
        "Classify",
        temp.path().join("b.db").as_path(),
        Some("accuracy lt 0.85"),
    );
    // The §4 CONTENT parity: the two saves happen in ONE process, so the
    // runtime ledger grows between them and `ledger_seq` differs by
    // construction (the anchor lane, not the capture's subject). Strip
    // the anchor field and the remaining bytes MUST be identical — the
    // capture rides BESIDE the certificate (the W1 never-cut lane).
    let strip = |v: &str| -> String {
        let mut val: serde_json::Value = serde_json::from_str(v).unwrap();
        val.as_object_mut().unwrap().remove("ledger_seq");
        serde_json::to_string(&val).unwrap()
    };
    assert_eq!(
        strip(&without),
        strip(&with),
        "the certificate §4 content MUST NOT move when the capture rides (the W1 \
         never-cut lane; the ledger_seq difference is the anchor lane)"
    );
    let conn = rusqlite::Connection::open(temp.path().join("b.db")).unwrap();
    let cert = current_cert(&conn, "Classify").unwrap().expect("cert");
    assert_eq!(
        cert.rollback_json, "accuracy lt 0.85",
        "the canonical session word is the column's content"
    );
    let conn0 = rusqlite::Connection::open(temp.path().join("a.db")).unwrap();
    let cert0 = current_cert(&conn0, "Classify").unwrap().expect("cert");
    assert_eq!(
        cert0.rollback_json, "",
        "a clause-less session (and a pre-W7 row) reads the empty form"
    );
    // The ledger anchor: the capture record exists and carries the rule's
    // sha256 (the mirror cross-checks against the chain).
    let rule_sha = metalogos::ledger::sha256_hex(b"accuracy lt 0.85");
    let records = metalogos::ledger::all_records().unwrap();
    let found = records
        .iter()
        .find(|r| r.action == "reflex.rollback-rules")
        .expect("the capture ledger record exists");
    assert_eq!(
        found.args_hash, rule_sha,
        "the ledger record anchors the rule's sha256 (the tamper-evident mirror)"
    );
}

// ── (b): the STALE drift signal feeds the hysteresis ─────────────────

/// №995 (b): two stale loads cross the threshold (2) inside the window —
/// the pattern auto-freezes for the distill moves; the verify verdicts
/// themselves are UNTOUCHED (both loads refuse exactly as W1 built them:
/// the reason carries [ATTESTATION_STALE]).
#[test]
#[serial]
#[allow(clippy::disallowed_methods)]
fn stale_signal_feeds_hysteresis_and_freezes() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("s.db");
    save_with_rule("Classify", &db, None);
    // The drift: the live declarations moved since the save. The load
    // goes through the PUBLIC lane (reflex_load_dispatch — the builtin
    // that carries the W7 feed; the W1 verdict shape is asserted on the
    // dispatch's error, which is the load error verbatim).
    let name_map =
        std::collections::HashMap::from([("Classify".to_string(), metalogos::nn::ReflexId(0))]);
    let load = || {
        let mut registry = ReflexRegistry::new();
        registry.register(make_model("Classify", 42));
        let input = AttestInput {
            spec: spec_for("classify the text — the declarations moved"),
            examples: vec![],
            rollback_rule: None,
        };
        metalogos::builtins::reflex::reflex_load_dispatch(
            &mut registry,
            &name_map,
            Some(db.to_str().unwrap()),
            Some(&input),
            &[metalogos::interpreter::Value::String(
                "Classify".to_string(),
            )],
        )
    };
    for attempt in 0..2 {
        let err = load().expect_err("the stale spec refuses the load (the W1 verdict untouched)");
        assert!(
            err.contains(CODE_STALE),
            "the load refusal is the W1 [ATTESTATION_STALE] verdict (attempt {})",
            attempt
        );
    }
    // The lifecycle crossed: the pattern is Frozen.
    let conn = open_store(&db);
    let lc = read_lifecycle(&conn, "Classify")
        .unwrap()
        .expect("the violation ring created the lifecycle row");
    assert_eq!(
        lc.phase,
        DistillPhase::Frozen,
        "the threshold (2) crossed inside the window — the auto-freeze"
    );
    // The distill moves refuse on the interlock; the load verdict shape
    // is unchanged (the third load refuses with [ATTESTATION_STALE] —
    // the SAME reason, not a lifecycle error).
    let err = load().expect_err("the stale verdict is the load's verdict — the feed moved nothing");
    assert!(err.contains(CODE_STALE));
}

// ── (c): the teacher-policy enforcement at the enablement boundary ───

/// №995 (c): the honest non-permissive teacher (the consistent story —
/// the block agrees with the registry) refuses the enablement with the
/// W7 policy code; zero writes (the certificate row is byte-identical).
#[test]
#[serial]
#[allow(clippy::disallowed_methods)]
fn teacher_policy_refuses_non_permissive_enablement() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("s.db");
    save_filtered("Classify", &db);
    swap_teacher_block(
        &db,
        "Classify",
        teacher_entry(
            "claude-sonnet-4-20250514",
            "claude-sonnet-4-20250514",
            "restrictive",
            false,
        ),
    );
    let cert_before = current_cert(&open_store(&db), "Classify")
        .unwrap()
        .unwrap()
        .cert_json;
    let mut conn = open_store(&db);
    let research = craft_research_dir(&temp.path().join("research"), Some(0.42));
    let err = enable_differential(
        &mut conn,
        "Classify",
        &research,
        1_000_000,
        &|_l: String| {},
    )
    .expect_err("decision 3 refuses the enablement of a non-permissive-teacher model");
    assert!(
        err.contains(CODE_ENABLEMENT_TEACHER_POLICY),
        "the refusal carries the W7 policy code: {}",
        err
    );
    assert!(
        err.contains("claude-sonnet-4-20250514") && err.contains("restrictive"),
        "the refusal names the teacher and the class: {}",
        err
    );
    let cert_after = current_cert(&open_store(&db), "Classify")
        .unwrap()
        .unwrap()
        .cert_json;
    assert_eq!(
        cert_before, cert_after,
        "zero writes on refusal — the certificate row is byte-identical"
    );
}

/// №995 (c): the forged teacher story refuses the enablement with the
/// SAME mismatch code the verify path carries (one machine truth, the
/// shared check fn).
#[test]
#[serial]
#[allow(clippy::disallowed_methods)]
fn teacher_policy_refuses_forged_story() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("s.db");
    save_filtered("Classify", &db);
    swap_teacher_block(
        &db,
        "Classify",
        teacher_entry("forged-teacher", "forged-weights", "proprietary", false),
    );
    let mut conn = open_store(&db);
    let research = craft_research_dir(&temp.path().join("research"), Some(0.42));
    let err = enable_differential(
        &mut conn,
        "Classify",
        &research,
        1_000_000,
        &|_l: String| {},
    )
    .expect_err("the forged story refuses");
    assert!(
        err.contains(CODE_TEACHER_MISMATCH),
        "the forged story is the mismatch refusal: {}",
        err
    );
}

/// №995 (c): decision 3 is vacuous without a teacher — the §7.3 null +
/// the machine-true reason passes the policy gate, and the permissive
/// teacher keeps the W6 flip working (the regression pin).
#[test]
#[serial]
#[allow(clippy::disallowed_methods)]
fn teacher_policy_passes_no_teacher_and_permissive() {
    let temp = TempDir::new().unwrap();
    // The no-teacher form: the §7.3 null + the reason.
    let db1 = temp.path().join("a.db");
    save_filtered("Classify", &db1);
    swap_teacher_block(&db1, "Classify", serde_json::Value::Null);
    let mut conn = open_store(&db1);
    let research = craft_research_dir(&temp.path().join("r1"), Some(0.42));
    let verdict = enable_differential(
        &mut conn,
        "Classify",
        &research,
        1_000_000,
        &|_l: String| {},
    )
    .expect("decision 3 is vacuous without a teacher — the gate proceeds");
    assert!(
        verdict.flipped,
        "the E3-measured flip proceeds (the W6 path)"
    );
    assert!(
        !verdict.reason.contains(CODE_ENABLEMENT_UNFILTERED),
        "the W6 verdict shape is unchanged"
    );
    // The permissive teacher: the W6 flip keeps working (the registry's
    // permissive mock record — the policy gate passes silently).
    let db2 = temp.path().join("b.db");
    save_filtered("Classify", &db2);
    swap_teacher_block(
        &db2,
        "Classify",
        teacher_entry("mock", "in-tree-mock-teacher", "osi", true),
    );
    let mut conn = open_store(&db2);
    let research = craft_research_dir(&temp.path().join("r2"), Some(0.42));
    let verdict = enable_differential(
        &mut conn,
        "Classify",
        &research,
        1_000_000,
        &|_l: String| {},
    )
    .expect("the permissive teacher keeps the W6 path");
    assert!(verdict.flipped);
}

/// №995 (c): the refusal rides the trace surface — the same bytes to
/// stderr and the audit journal (the W5/W6 trace contract; the sink here
/// is the audit journal half).
#[test]
#[serial]
#[allow(clippy::disallowed_methods)]
fn teacher_policy_refusal_rides_trace() {
    bracket_env("METALOGOS_TRACE_DISTILL", "1");
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("s.db");
    save_filtered("Classify", &db);
    swap_teacher_block(
        &db,
        "Classify",
        teacher_entry(
            "claude-sonnet-4-20250514",
            "claude-sonnet-4-20250514",
            "restrictive",
            false,
        ),
    );
    let mut conn = open_store(&db);
    let research = craft_research_dir(&temp.path().join("research"), Some(0.42));
    let (sink, log) = make_sink();
    let _ = enable_differential(&mut conn, "Classify", &research, 1_000_000, &*sink)
        .expect_err("the refusal");
    unbracket_env("METALOGOS_TRACE_DISTILL");
    let trace = log.lock().unwrap().join("\n");
    assert!(
        trace.contains("REFUSED") && trace.contains(CODE_ENABLEMENT_TEACHER_POLICY),
        "the trace carries the refusal line with the policy code:\n{}",
        trace
    );
}
