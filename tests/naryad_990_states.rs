//! №990 (ADR-0200 W5) — the lifecycle leak-suite: the Active/Frozen
//! machine, the hysteresis (the anti-flap ring), the 24h-bounded
//! auto-unfreeze (decision 6 verbatim), the re-freeze lock, the
//! `--trace-distill` byte form and the gate-line JSON.
//!
//! The W5 lane's honest boundary (ADR-0200 §5 + decision 6): the state
//! is a safety interlock over the distill lane — a Frozen pattern
//! refuses distill MOVES with `[DISTILL_FROZEN]` while serving
//! continues; the auto-exit fires no more than once per the 24h
//! cooldown, every auto-exit ledger-records, and a pattern re-frozen
//! within the window stays frozen until the explicit external command.

use metalogos::distill_hub::{
    ensure_distill_store, freeze_explicit, lifecycle_gate, lifecycle_status, read_lifecycle,
    record_violation, unfreeze_explicit, DistillPhase,
};
use serial_test::serial;
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
fn open_store(path: &std::path::Path) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open(path).unwrap();
    ensure_distill_store(&conn).unwrap();
    conn
}

#[allow(clippy::disallowed_methods)]
fn craft_frozen_row(
    conn: &rusqlite::Connection,
    pattern: &str,
    entered_at: u64,
    auto_unfroze_at: Option<u64>,
    refroze_in_window: bool,
) {
    conn.execute(
        "INSERT OR REPLACE INTO distill_states \
         (pattern, phase, entered_at, violations, last_violation_code, \
          auto_unfroze_at, refroze_in_window) \
         VALUES (?1, 'frozen', ?2, '', NULL, ?3, ?4)",
        rusqlite::params![
            pattern,
            entered_at as i64,
            auto_unfroze_at.map(|t| t as i64),
            refroze_in_window as i64,
        ],
    )
    .unwrap();
}

fn bracket_env(key: &str, value: &str) {
    std::env::set_var(key, value);
}

fn unbracket_env(key: &str) {
    std::env::remove_var(key);
}

// ── the suite ────────────────────────────────────────────────────────

#[test]
#[allow(clippy::disallowed_methods)]
fn pre_w5_store_is_a_byte_compat_noop() {
    // A store with samples but no lifecycle rows: the gate passes, the
    // read answers None — the pre-W5 database needs no migration beyond
    // the additive CREATE (the W4 byte-compat form).
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("s.db");
    let conn = open_store(&db);
    conn.execute(
        "INSERT INTO distill_samples \
         (pattern_name, input, output, source, created_at, reviewed, reason, quarantined) \
         VALUES ('Classify', 'in', 'out', 'llm-response', 1, 0, NULL, 0)",
        [],
    )
    .unwrap();
    assert!(read_lifecycle(&conn, "Classify").unwrap().is_none());
    let (sink, _log) = make_sink();
    assert!(lifecycle_gate(&conn, "Classify", 1000, &*sink).is_ok());
    // The ensure call created the table — the additive migration ran.
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='distill_states'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 1, "the distill_states table exists");
}

#[test]
#[allow(clippy::disallowed_methods)]
fn explicit_freeze_refuses_moves_then_explicit_unfreeze_releases() {
    let temp = TempDir::new().unwrap();
    let conn = open_store(&temp.path().join("s.db"));
    let (sink, _log) = make_sink();

    // Fresh pattern: Active, no rows.
    assert!(read_lifecycle(&conn, "Classify").unwrap().is_none());
    assert!(lifecycle_gate(&conn, "Classify", 1000, &*sink).is_ok());

    // The operator freeze — the move refuses with the machine reason.
    let v = freeze_explicit(&conn, "Classify", "the audit found a leak", 1100, &*sink).unwrap();
    assert_eq!(v.phase, DistillPhase::Frozen);
    assert!(v.changed);
    let err = lifecycle_gate(&conn, "Classify", 1200, &*sink).unwrap_err();
    assert!(
        err.contains("[DISTILL_FROZEN]"),
        "the machine reason rides the refusal: {}",
        err
    );
    assert!(
        err.contains("mlog distill unfreeze"),
        "the exit command is named: {}",
        err
    );
    let lc = read_lifecycle(&conn, "Classify").unwrap().unwrap();
    assert_eq!(lc.phase, DistillPhase::Frozen);
    assert_eq!(lc.entered_at, 1100);

    // The explicit external command releases; the row stays (history).
    let v2 = unfreeze_explicit(&conn, "Classify", "the operator cleared it", 1300, &*sink).unwrap();
    assert_eq!(v2.phase, DistillPhase::Active);
    assert!(v2.changed);
    assert!(lifecycle_gate(&conn, "Classify", 1400, &*sink).is_ok());
}

#[test]
#[allow(clippy::disallowed_methods)]
fn hysteresis_crosses_threshold_inside_window() {
    // The anti-flap band: ONE violation is refusal-only; the SECOND
    // inside the window auto-freezes.
    let temp = TempDir::new().unwrap();
    let conn = open_store(&temp.path().join("s.db"));
    let (sink, _log) = make_sink();

    let v1 = record_violation(
        &conn,
        "Classify",
        "DISTILL_TEACHER_UNLICENSED",
        1000,
        &*sink,
    )
    .unwrap();
    assert!(!v1.changed, "a single violation does not freeze");
    assert_eq!(v1.phase, DistillPhase::Active);

    let v2 = record_violation(
        &conn,
        "Classify",
        "DISTILL_TEACHER_UNLICENSED",
        1500,
        &*sink,
    )
    .unwrap();
    assert!(v2.changed, "the second violation inside the window freezes");
    assert_eq!(v2.phase, DistillPhase::Frozen);
    let lc = read_lifecycle(&conn, "Classify").unwrap().unwrap();
    assert_eq!(
        lc.last_violation_code.as_deref(),
        Some("DISTILL_TEACHER_UNLICENSED")
    );
    assert!(!lc.refroze_in_window, "no prior auto-unfreeze — no lock");
}

#[test]
#[allow(clippy::disallowed_methods)]
fn violations_outside_the_window_do_not_freeze() {
    // The ring trims to the window (default 3600s): a second violation
    // far outside it does NOT cross the threshold.
    let temp = TempDir::new().unwrap();
    let conn = open_store(&temp.path().join("s.db"));
    let (sink, _log) = make_sink();

    record_violation(
        &conn,
        "Classify",
        "DISTILL_TEACHER_UNLICENSED",
        1000,
        &*sink,
    )
    .unwrap();
    let v2 = record_violation(
        &conn,
        "Classify",
        "DISTILL_TEACHER_UNLICENSED",
        1000 + 7200,
        &*sink,
    )
    .unwrap();
    assert!(!v2.changed, "outside the window the ring forgot the first");
    assert_eq!(v2.phase, DistillPhase::Active);
    let lc = read_lifecycle(&conn, "Classify").unwrap().unwrap();
    assert_eq!(lc.violations.len(), 1, "only the fresh violation rides");
}

#[test]
#[serial]
#[allow(clippy::disallowed_methods)]
fn bounded_auto_unfreeze_first_exit_fires() {
    // Decision 6: a pattern AUTO-frozen (the hysteresis) with passing
    // recovery checks exits automatically on the move attempt — the
    // FIRST auto-exit (no clock anchor yet, no re-freeze lock, no
    // operator lock).
    bracket_env("METALOGOS_MOCK_LLM", "1");
    bracket_env("METALOGOS_TRACE_DISTILL", "1");
    let temp = TempDir::new().unwrap();
    let conn = open_store(&temp.path().join("s.db"));
    let (sink, log) = make_sink();

    record_violation(&conn, "Classify", "DISTILL_TEACHER_UNLICENSED", 900, &*sink).unwrap();
    record_violation(
        &conn,
        "Classify",
        "DISTILL_TEACHER_UNLICENSED",
        1000,
        &*sink,
    )
    .unwrap();
    assert_eq!(
        read_lifecycle(&conn, "Classify").unwrap().unwrap().phase,
        DistillPhase::Frozen,
        "the hysteresis froze the pattern"
    );
    // The first distill-move attempt: the lazy evaluation fires the
    // bounded auto-unfreeze (recovery passes under the mock teacher).
    assert!(
        lifecycle_gate(&conn, "Classify", 1100, &*sink).is_ok(),
        "the bounded auto-exit released the pattern"
    );
    let lc = read_lifecycle(&conn, "Classify").unwrap().unwrap();
    assert_eq!(lc.phase, DistillPhase::Active);
    assert_eq!(lc.auto_unfroze_at, Some(1100), "the 24h clock is anchored");
    assert!(
        !lc.operator_lock,
        "an automation freeze carries no operator lock"
    );
    let lines = log.lock().unwrap().clone();
    assert!(
        lines
            .iter()
            .any(|l| l.contains("[TRACE] distill.auto-unfreeze")),
        "the auto-exit traces loudly: {:?}",
        lines
    );
    unbracket_env("METALOGOS_MOCK_LLM");
    unbracket_env("METALOGOS_TRACE_DISTILL");
}

#[test]
#[serial]
#[allow(clippy::disallowed_methods)]
fn refreeze_within_window_requires_the_explicit_command() {
    // Decision 6 verbatim: «If the model re-freezes within the window,
    // it stays frozen until an explicit external command».
    bracket_env("METALOGOS_MOCK_LLM", "1");
    bracket_env("METALOGOS_TRACE_DISTILL", "1");
    let temp = TempDir::new().unwrap();
    let conn = open_store(&temp.path().join("s.db"));
    let (sink, log) = make_sink();

    // The first AUTO freeze + the auto-exit at t=1100 (the clock anchor).
    record_violation(&conn, "Classify", "DISTILL_TEACHER_UNLICENSED", 900, &*sink).unwrap();
    record_violation(
        &conn,
        "Classify",
        "DISTILL_TEACHER_UNLICENSED",
        1000,
        &*sink,
    )
    .unwrap();
    assert!(lifecycle_gate(&conn, "Classify", 1100, &*sink).is_ok());
    // The re-freeze INSIDE the 24h window (the hysteresis again) — the
    // re-freeze lock sets.
    record_violation(
        &conn,
        "Classify",
        "DISTILL_TEACHER_UNLICENSED",
        3500,
        &*sink,
    )
    .unwrap();
    let v = record_violation(
        &conn,
        "Classify",
        "DISTILL_TEACHER_UNLICENSED",
        3600,
        &*sink,
    )
    .unwrap();
    assert_eq!(v.phase, DistillPhase::Frozen);
    let lc = read_lifecycle(&conn, "Classify").unwrap().unwrap();
    assert!(lc.refroze_in_window, "the re-freeze lock set");
    // The move attempt withholds the auto-exit (the lock), refuses.
    let err = lifecycle_gate(&conn, "Classify", 3700, &*sink).unwrap_err();
    assert!(
        err.contains("[DISTILL_FROZEN]"),
        "the lock keeps the pattern frozen: {}",
        err
    );
    assert!(
        log.lock()
            .unwrap()
            .iter()
            .any(|l| l.contains("re-froze within the cooldown window")),
        "the withholding traces the decision-6 reason"
    );
    // The explicit external command — the ONLY exit.
    unfreeze_explicit(&conn, "Classify", "the operator cleared it", 3800, &*sink).unwrap();
    assert!(lifecycle_gate(&conn, "Classify", 3900, &*sink).is_ok());
    let lc = read_lifecycle(&conn, "Classify").unwrap().unwrap();
    assert!(
        !lc.refroze_in_window,
        "the explicit command clears the lock"
    );
    unbracket_env("METALOGOS_MOCK_LLM");
    unbracket_env("METALOGOS_TRACE_DISTILL");
}

#[test]
#[serial]
#[allow(clippy::disallowed_methods)]
fn cooldown_clock_withholds_inside_24h_then_allows() {
    // The 24h bound itself (the default 86400s): no lock, recovery
    // passes — the auto-exit still waits for the clock.
    bracket_env("METALOGOS_MOCK_LLM", "1");
    let temp = TempDir::new().unwrap();
    let conn = open_store(&temp.path().join("s.db"));
    let (sink, _log) = make_sink();

    // A frozen row with a fresh auto-unfreeze anchor (t=1000), no lock.
    craft_frozen_row(&conn, "Classify", 2000, Some(1000), false);
    let err = lifecycle_gate(&conn, "Classify", 1000 + 3600, &*sink).unwrap_err();
    assert!(
        err.contains("[DISTILL_FROZEN]"),
        "inside the cooldown the auto-exit withholds: {}",
        err
    );
    // Beyond the cooldown the auto-exit fires.
    assert!(
        lifecycle_gate(&conn, "Classify", 1000 + 86401, &*sink).is_ok(),
        "the bounded auto-exit fires after 24h"
    );
    let lc = read_lifecycle(&conn, "Classify").unwrap().unwrap();
    assert_eq!(lc.auto_unfroze_at, Some(1000 + 86401));
    unbracket_env("METALOGOS_MOCK_LLM");
}

#[test]
#[serial]
#[allow(clippy::disallowed_methods)]
fn trace_lines_and_gate_line_json_shapes() {
    // --trace-distill: the [TRACE] lines land in the audit sink (the
    // same bytes go to stderr — the helper emits both). The gate lines
    // parse as JSON with the stable keys.
    bracket_env("METALOGOS_TRACE_DISTILL", "1");
    let temp = TempDir::new().unwrap();
    let conn = open_store(&temp.path().join("s.db"));
    let (sink, log) = make_sink();

    let v = freeze_explicit(&conn, "Classify", "the audit found a leak", 1000, &*sink).unwrap();
    let line = v.gate_line();
    let js: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(js["gate"], "distill-freeze");
    assert_eq!(js["ok"], true);
    assert_eq!(js["phase"], "frozen");
    assert_eq!(js["changed"], true);

    let v2 = lifecycle_status(&conn, "Classify", 1100).unwrap();
    let js2: serde_json::Value = serde_json::from_str(&v2.gate_line()).unwrap();
    assert_eq!(js2["gate"], "distill-status");
    assert_eq!(js2["phase"], "frozen");
    assert!(
        js2["recovery"]
            .as_str()
            .unwrap()
            .contains("teacher-gate-ok"),
        "the recovery checks ride the status gate line: {}",
        js2["recovery"]
    );

    let v3 = unfreeze_explicit(&conn, "Classify", "cleared", 1200, &*sink).unwrap();
    let js3: serde_json::Value = serde_json::from_str(&v3.gate_line()).unwrap();
    assert_eq!(js3["gate"], "distill-unfreeze");
    assert_eq!(js3["phase"], "active");

    let lines = log.lock().unwrap().clone();
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("[TRACE] distill.freeze")),
        "the freeze traced: {:?}",
        lines
    );
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("[TRACE] distill.unfreeze")),
        "the unfreeze traced: {:?}",
        lines
    );
    unbracket_env("METALOGOS_TRACE_DISTILL");
}
