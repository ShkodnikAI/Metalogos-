// ── Naryad #418 (P0, vm/cron, issue #571): the cron reliability corpus ──
//
// Red/green corpus (D1–D6):
//   (1)  D1 dedup — one matched window fires AT MOST once: inside the
//        window every tick re-decision says already-fired; a new window
//        fires again. (The pre-№418 bug: refire every 5s — up to 12×/min.)
//   (2)  D1 restart idempotency — the stamp survives in the spec; the same
//        window is not re-fired after a reload.
//   (3)  D2 catch-up run_once — windows missed while down coalesce into
//        ONE fire (fire=true, reason=catch-up, the window = the latest).
//   (4)  D2 catch-up skip — missed windows are consumed without firing
//        (advance=true), the next regular window fires normally.
//   (5)  D3 TZ identity — "0 7 * * *" in Europe/Moscow fires at the
//        Moscow 07:00 wall clock, NOT at 07:00 UTC; the same instant with
//        the UTC job does not fire. The env default (MLOG_CRON_TZ) is
//        honored; an unknown TZ is a LOUD error (never silent drift).
//   (6)  D4 payload — the dispatch args carry the fixed payload as a
//        single String; the zero-arg shape is unchanged without payload.
//   (7)  D5 reminder delivery — the delivery args shape; a failing
//        handler's error is stamped CRON_JOB_FAILED (№413 stamps on every
//        fail path); a missing handler is not an error.
//   (8)  Store migration 0.20.x → 0.21.0 — an old-shape job JSON (without
//        tz/catch_up/payload/last_window) parses with the defaults and the
//        job behaves; cron_list surfaces the new fields (D6).
//   (9)  cron_add validation — unknown TZ / bad catch_up refuse loudly
//        (stamped CRON_JOB_FAILED at the registry boundary).
//
// NOTE on isolation: the cron job store is process-global (the KV store),
// so every store-touching assertion lives in ONE serial test at the bottom
// of this file; the rest of the corpus is pure (no global state).

use chrono::Timelike;
use metalogos::builtins::cron::{
    cron_dispatch_args, cron_fire_decision, deliver_due_reminders, job_spec_from_json,
    last_window_before, next_window_after, resolve_job_tz, CatchUpPolicy, CronJobSpec,
};

fn spec(
    expr: &str,
    tz: &str,
    catch_up: CatchUpPolicy,
    last_window: Option<i64>,
    created_at: i64,
) -> CronJobSpec {
    CronJobSpec {
        id: "n418".to_string(),
        cron_expr: expr.to_string(),
        prompt: "n418_target".to_string(),
        tz: tz.to_string(),
        catch_up,
        payload: None,
        last_window,
        force_run: false,
        created_at,
    }
}

/// Epoch seconds of the UTC wall clock Y-M-D H:M (computed, never magic).
fn utc_epoch(y: i32, m: u32, d: u32, h: u32, min: u32) -> i64 {
    chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, y, m, d, h, min, 0)
        .single()
        .expect("valid utc wall clock")
        .timestamp()
}

/// Epoch seconds of the wall clock Y-M-D H:M as seen in `tz` (computed).
fn tz_epoch(tz_name: &str, y: i32, m: u32, d: u32, h: u32, min: u32) -> i64 {
    let tz: chrono_tz::Tz = tz_name.parse().unwrap();
    chrono::TimeZone::with_ymd_and_hms(&tz, y, m, d, h, min, 0)
        .single()
        .expect("valid tz wall clock")
        .timestamp()
}

// ── (1) D1: one window, one fire ────────────────────────────────────────

#[test]
fn n418_dedup_one_fire_per_window() {
    // "* * * * *" matches every minute.
    let now = utc_epoch(2026, 10, 9, 14, 23) + 20; // inside the 14:23 window
    let w = (now / 60) * 60;

    // Never fired, inside the window → fire (the scheduled due path).
    let d = cron_fire_decision(
        &spec("* * * * *", "UTC", CatchUpPolicy::RunOnce, None, 0),
        now,
    )
    .expect("decision");
    assert!(
        d.fire && d.window == Some(w) && d.reason == "due",
        "{:?}",
        d
    );

    // The SAME window already fired → no fire (the pre-№418 bug re-fired
    // here every 5 seconds — up to 12×/minute).
    let d = cron_fire_decision(
        &spec("* * * * *", "UTC", CatchUpPolicy::RunOnce, Some(w), 0),
        now + 5,
    )
    .expect("decision");
    assert!(!d.fire && d.reason == "already-fired", "{:?}", d);

    // ... and 55 seconds after the window opened, still no re-fire.
    let d = cron_fire_decision(
        &spec("* * * * *", "UTC", CatchUpPolicy::RunOnce, Some(w), 0),
        w + 55,
    )
    .expect("decision");
    assert!(!d.fire, "{:?}", d);

    // The NEXT window → fire again (exactly once per window).
    let d = cron_fire_decision(
        &spec("* * * * *", "UTC", CatchUpPolicy::RunOnce, Some(w), 0),
        w + 65,
    )
    .expect("decision");
    assert!(d.fire && d.window == Some(w + 60), "{:?}", d);
}

// ── (2) D1: restart idempotency — the stamp is in the persisted spec ────

#[test]
fn n418_dedup_survives_reload() {
    let now = utc_epoch(2026, 10, 9, 14, 30) + 7;
    let w = (now / 60) * 60;
    // The job fired this window, the process restarted, the store came
    // back with last_window set (job_spec_from_json reads the stamp).
    let stored = serde_json::json!({
        "id": "cron_1", "cron_expr": "* * * * *", "prompt": "tick",
        "enabled": true, "created_at": 0, "run_count": 3,
        "last_window": w,
    });
    let s = job_spec_from_json(&stored);
    let d = cron_fire_decision(&s, now).expect("decision");
    assert!(!d.fire && d.reason == "already-fired", "{:?}", d);
}

// ── (3) D2: catch-up run_once — missed windows coalesce into ONE fire ───

#[test]
fn n418_catch_up_run_once() {
    // "0 9 * * *" — daily 09:00 UTC. The job fired on the 7th, slept
    // through the 8th and 9th, and wakes at 09:07 on the 10th.
    let fired_last = utc_epoch(2026, 10, 7, 9, 0);
    let wake = utc_epoch(2026, 10, 10, 9, 7);
    let s = spec(
        "0 9 * * *",
        "UTC",
        CatchUpPolicy::RunOnce,
        Some(fired_last),
        0,
    );
    let d = cron_fire_decision(&s, wake).expect("decision");
    assert!(d.fire, "run_once fires after the sleep: {:?}", d);
    assert_eq!(d.reason, "catch-up");
    // The window is the LATEST matching one (coalesced — NOT one fire per
    // missed day).
    let expected = last_window_before("0 9 * * *", "UTC".parse().unwrap(), wake, None);
    assert_eq!(d.window, expected, "coalesced to the latest 09:00");
    assert_eq!(d.window, Some(utc_epoch(2026, 10, 10, 9, 0)));
}

// ── (4) D2: catch-up skip — missed windows consumed, next regular fires ─

#[test]
fn n418_catch_up_skip() {
    let fired_last = utc_epoch(2026, 10, 7, 9, 0);
    let wake = utc_epoch(2026, 10, 10, 9, 7);
    let s = spec("0 9 * * *", "UTC", CatchUpPolicy::Skip, Some(fired_last), 0);
    let d = cron_fire_decision(&s, wake).expect("decision");
    assert!(!d.fire, "skip does not fire a missed window: {:?}", d);
    assert!(d.advance, "the window is consumed without firing");
    assert_eq!(d.reason, "skipped");
    assert_eq!(d.window, Some(utc_epoch(2026, 10, 10, 9, 0)));

    // The next REGULAR window (inside its minute) fires normally — the
    // consumed stamp is the OLD window, not the new one.
    let consumed = d.window.unwrap();
    let next_window = utc_epoch(2026, 10, 11, 9, 0);
    let s2 = spec("0 9 * * *", "UTC", CatchUpPolicy::Skip, Some(consumed), 0);
    let d2 = cron_fire_decision(&s2, next_window + 30).expect("decision");
    assert!(
        d2.fire && d2.reason == "due" && d2.window == Some(next_window),
        "{:?}",
        d2
    );
}

// ── (5) D3: the timezone identity of a window ───────────────────────────

#[test]
fn n418_tz_identity_moscow_vs_utc() {
    // The Moscow 07:00 wall clock on 2026-10-09 == 04:00 UTC.
    let at_moscow_0700 = tz_epoch("Europe/Moscow", 2026, 10, 9, 7, 0);
    let moscow: chrono_tz::Tz = "Europe/Moscow".parse().unwrap();
    assert_eq!(
        last_window_before("0 7 * * *", moscow, at_moscow_0700, None),
        Some(at_moscow_0700),
        "the Moscow 07:00 window starts at this epoch"
    );
    // Sanity: the same instant is 04:00 in UTC (Moscow = UTC+3).
    let utc_h = chrono::DateTime::from_timestamp(at_moscow_0700, 0)
        .unwrap()
        .with_timezone(&chrono::Utc)
        .hour();
    assert_eq!(utc_h, 4);

    // A Moscow job created just before its window fires; a UTC job created
    // at the same moment does not (its 07:00 is three hours away — and the
    // pre-creation guard keeps the earlier UTC window from catch-up).
    let created = at_moscow_0700 - 120;
    let s_msk = spec(
        "0 7 * * *",
        "Europe/Moscow",
        CatchUpPolicy::RunOnce,
        None,
        created,
    );
    let d_msk = cron_fire_decision(&s_msk, at_moscow_0700 + 20).expect("decision");
    assert!(d_msk.fire && d_msk.reason == "due", "{:?}", d_msk);
    assert_eq!(d_msk.window, Some(at_moscow_0700));

    let s_utc = spec("0 7 * * *", "UTC", CatchUpPolicy::RunOnce, None, created);
    let d_utc = cron_fire_decision(&s_utc, at_moscow_0700 + 20).expect("decision");
    assert!(!d_utc.fire, "07:00 UTC is not 07:00 Moscow: {:?}", d_utc);
    // The window identities differ per job TZ — a Moscow fire never
    // consumes the UTC job's window.
    assert_ne!(d_msk.window, d_utc.window);
}

#[test]
fn n418_tz_env_default_and_loud_unknown() {
    // The env default: an empty/absent per-job tz resolves via MLOG_CRON_TZ.
    std::env::set_var("MLOG_CRON_TZ", "Europe/Moscow");
    let tz = resolve_job_tz(Some("")).expect("env default resolves");
    assert_eq!(tz.to_string(), "Europe/Moscow");
    std::env::remove_var("MLOG_CRON_TZ");
    let tz = resolve_job_tz(Some("")).expect("no env → UTC");
    assert_eq!(tz.to_string(), "UTC");

    // An unknown IANA name is a LOUD error — a cron job must never drift
    // silently to another zone.
    assert!(resolve_job_tz(Some("Europe/Mosccow")).is_err());
    // The decision surfaces the same loudness.
    let s = spec(
        "0 7 * * *",
        "Europe/Mosccow",
        CatchUpPolicy::RunOnce,
        None,
        0,
    );
    assert!(cron_fire_decision(&s, 1_760_001_600).is_err());
}

// ── (6) D4: the payload dispatch args ───────────────────────────────────

#[test]
fn n418_payload_dispatch_args() {
    // No payload → the classic zero-arg shape (existing jobs unaffected).
    assert!(cron_dispatch_args(None).is_empty());
    assert!(cron_dispatch_args(Some("")).is_empty());
    // A payload → exactly one String argument (DATA, never code).
    let args = cron_dispatch_args(Some("{\"order_id\":\"f-1\"}"));
    assert_eq!(args.len(), 1);
    match &args[0] {
        metalogos::interpreter::Value::String(s) => {
            assert_eq!(s, "{\"order_id\":\"f-1\"}");
        }
        other => panic!("payload must travel as String, got {:?}", other),
    }
}

// ── (7) D5: the reminder delivery contract ──────────────────────────────

#[test]
fn n418_reminder_delivery_contract() {
    let due = vec![
        ("standup".to_string(), "d1".to_string(), "once".to_string()),
        (
            "deploy".to_string(),
            "d2".to_string(),
            "recurring".to_string(),
        ),
    ];

    // No handler → journal-only, no failures (the 0.20.x behavior).
    let f = deliver_due_reminders(&due, None, |_, _| panic!("must not dispatch"));
    assert!(f.is_empty());

    // A handler present → every reminder dispatched with (message, data,
    // type); successes produce no failures.
    let mut calls: Vec<(String, usize)> = Vec::new();
    let f = deliver_due_reminders(&due, Some("ReminderCheck"), |name, args| {
        calls.push((name.to_string(), args.len()));
        Ok(metalogos::interpreter::Value::Unit)
    });
    assert!(f.is_empty());
    assert_eq!(calls.len(), 2);
    assert!(calls.iter().all(|(n, a)| n == "ReminderCheck" && *a == 3));

    // A FAILING handler → the failure is stamped CRON_JOB_FAILED
    // (the №413 stamps hold on every fail path).
    let f = deliver_due_reminders(&due, Some("ReminderCheck"), |_, _| {
        Err("handler exploded".to_string())
    });
    assert_eq!(f.len(), 2);
    assert!(
        f.iter().all(|e| e.contains("CRON_JOB_FAILED")),
        "stamped: {:?}",
        f
    );
}

// ── (8)(9) Store-level behavior — ONE serial test (the global KV store) ─

const MIGRATION_SRC: &str = r#"
pattern Seed(_x: String) -> String {
  // A 0.20.x-era job: NO tz / catch_up / payload / last_window fields.
  kv_set("cron_jobs", "[{\"id\":\"legacy1\",\"cron_expr\":\"0 9 * * 1-5\",\"prompt\":\"OfficeMorning\",\"enabled\":true,\"created_at\":1000,\"last_run\":null,\"run_count\":2}]")
  return "seeded"
}
flow Main { input: String = "x" -> Seed -> output }
"#;

const LIST_SRC: &str = r#"
pattern P(_x: String) -> String {
  let j = cron_add("30 21 * * *", "OfficeEvening", "Europe/Moscow", "skip")
  let jobs = cron_list()
  let added = jobs[-1]
  return added.id + "|" + added.tz + "|" + added.catch_up + "|" + type_of(added.next_run) + "|" + type_of(added.last_run) + "|" + type_of(added.payload)
}
flow Main { input: String = "x" -> P -> output }
"#;

#[test]
fn n418_store_lifecycle_migration_list_and_validation() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // (8) A 0.20.x-era job seeded through the language surface survives —
    // parsed with the documented defaults.
    let out = metalogos::run_program_with_dir(MIGRATION_SRC.trim(), manifest.clone())
        .expect("kv seed runs");
    assert_eq!(out.as_deref(), Some("seeded"));
    let jobs = metalogos::builtins::cron::enabled_job_specs();
    let legacy = jobs
        .iter()
        .find(|j| j.id == "legacy1")
        .expect("legacy job alive");
    assert_eq!(legacy.cron_expr, "0 9 * * 1-5");
    assert_eq!(legacy.prompt, "OfficeMorning");
    assert_eq!(legacy.catch_up, CatchUpPolicy::RunOnce, "default run_once");
    assert!(legacy.payload.is_none(), "default no payload");
    assert!(legacy.last_window.is_none(), "never fired");

    // (9) cron_add validation: unknown TZ and bad catch_up refuse loudly —
    // the try contract carries r.ok=false + the stamped r.error.code.
    let bad = r#"
pattern P(_x: String) -> String {
  let r1 = try cron_add("0 9 * * *", "X", "Europe/Mosccow")
  let r2 = try cron_add("0 9 * * *", "X", "UTC", "sometimes")
  return to_string(r1.ok) + "|" + r1.error.code + "|" + to_string(r2.ok) + "|" + r2.error.code
}
flow Main { input: String = "x" -> P -> output }
"#;
    let out = metalogos::run_program_with_dir(bad.trim(), manifest.clone()).expect("try catches");
    assert_eq!(
        out.as_deref(),
        Some("false|CRON_JOB_FAILED|false|CRON_JOB_FAILED"),
        "both refusals carry the stamped code"
    );

    // (8/9) The additive arity: 5-arg cron_add + cron_list's new fields.
    let out = metalogos::run_program_with_dir(LIST_SRC.trim(), manifest).expect("add+list run");
    let out = out.as_deref().unwrap_or_default();
    assert!(
        out.contains("|Europe/Moscow|skip|") && out.contains("|Float|") && out.ends_with("String"),
        "tz/catch_up/payload/next_run/last_run surface via cron_list: {:?}",
        out
    );
    // D6: next_run IS computable and surfaced (Float), last_run is
    // surfaced (Unit while never fired — the field exists, the drift is
    // closed).
    assert!(
        out.matches("|Float|").count() >= 1,
        "next_run is a Float: {:?}",
        out
    );
    let jobs = metalogos::builtins::cron::enabled_job_specs();
    let j = jobs
        .iter()
        .find(|j| j.prompt == "OfficeEvening")
        .expect("the added job");
    let tz = resolve_job_tz(Some(&j.tz)).expect("moscow tz");
    let now = utc_epoch(2026, 10, 9, 14, 40);
    let next = next_window_after("30 21 * * *", tz, now).expect("an evening window exists");
    let local = chrono::DateTime::from_timestamp(next, 0)
        .unwrap()
        .with_timezone(&"Europe/Moscow".parse::<chrono_tz::Tz>().unwrap());
    assert_eq!(local.hour(), 21);
    assert_eq!(local.minute(), 30, "next_run is in the JOB's wall clock");
}
