// ── Cron Scheduler + Reminders builtins ──────────────────────────────────

use std::sync::Mutex as StdMutex;

use crate::interpreter::values::{coded_error, split_origin_stamp, CODE_CRON_JOB_FAILED};
use crate::interpreter::Value;

use super::chrono_now_timestamp;
use super::core::*;
use super::http::make_date_struct;
use super::memory::*;

/// №413 (issue #558, ADR-0169 §3.1 extension): stamp every cron/reminder
/// mechanics failure with the subsystem code at the place that KNOWS what
/// failed. Never double-stamps: an inner origin stamp (e.g. `SQL_ERROR`
/// from the persistence layer) stays authoritative at position 0.
/// №418: pub(crate) — the server's dispatch fail-paths stamp through it.
pub(crate) fn cron_stamped(e: String) -> String {
    if split_origin_stamp(&e).is_some() {
        e
    } else {
        coded_error(CODE_CRON_JOB_FAILED, e)
    }
}

/// The registry-facing wrappers: every cron/reminder builtin's failures
/// (arg/type refusals, the 5-field cron contract, lock errors) travel the
/// error channel stamped `CRON_JOB_FAILED` — the `try` classifier can then
/// branch the office's reschedule/alert policies on the subsystem code.
macro_rules! cron_wrapped {
    ($vis:vis fn $name:ident = $inner:ident;) => {
        $vis fn $name(args: &[Value]) -> Result<Value, String> {
            $inner(args).map_err(cron_stamped)
        }
    };
}

cron_wrapped!(pub(crate) fn builtin_remind_stamped = builtin_remind;);
cron_wrapped!(pub(crate) fn builtin_remind_recurring_stamped = builtin_remind_recurring;);
cron_wrapped!(pub(crate) fn builtin_cancel_remind_stamped = builtin_cancel_remind;);
cron_wrapped!(pub(crate) fn builtin_list_reminders_stamped = builtin_list_reminders;);
cron_wrapped!(pub(crate) fn builtin_check_reminders_stamped = builtin_check_reminders;);
cron_wrapped!(pub(crate) fn builtin_cron_add_stamped = builtin_cron_add;);
cron_wrapped!(pub(crate) fn builtin_cron_list_stamped = builtin_cron_list;);
cron_wrapped!(pub(crate) fn builtin_cron_remove_stamped = builtin_cron_remove;);
cron_wrapped!(pub(crate) fn builtin_cron_run_stamped = builtin_cron_run;);
cron_wrapped!(pub(crate) fn builtin_cron_mark_fired_stamped = builtin_cron_mark_fired;);

// ── v0.8.0 — Reminders builtins ─────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct ReminderEntry {
    id: String,
    message: String,
    fire_at: f64,
    interval: f64,
    next_fire: f64,
    data: String,
    active: bool,
    created_at: f64,
}

static REMINDERS: std::sync::OnceLock<StdMutex<Vec<ReminderEntry>>> = std::sync::OnceLock::new();

fn reminders_store() -> &'static StdMutex<Vec<ReminderEntry>> {
    REMINDERS.get_or_init(|| StdMutex::new(Vec::new()))
}

/// Global SQLite persistence for reminders (same pattern as KV_SQLITE).
static REMINDERS_SQLITE: std::sync::OnceLock<StdMutex<Option<rusqlite::Connection>>> =
    std::sync::OnceLock::new();

fn reminders_sqlite() -> &'static StdMutex<Option<rusqlite::Connection>> {
    REMINDERS_SQLITE.get_or_init(|| StdMutex::new(None))
}

/// Initialize SQLite persistence for reminders. Called from server.rs on startup.
pub fn init_reminder_persist(db_path: &str) -> Result<(), String> {
    let conn = rusqlite::Connection::open(db_path)
        .map_err(|e| format!("[reminders] Failed to open database '{}': {}", db_path, e))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS reminders (
            id TEXT PRIMARY KEY,
            message TEXT NOT NULL,
            fire_at REAL NOT NULL,
            interval REAL NOT NULL DEFAULT 0,
            next_fire REAL NOT NULL,
            data TEXT NOT NULL DEFAULT '',
            active INTEGER NOT NULL DEFAULT 1,
            created_at REAL NOT NULL
        );",
    )
    .map_err(|e| format!("[reminders] Failed to create table: {}", e))?;
    // Load existing reminders into memory
    let mut stmt = conn
        .prepare(
            "SELECT id, message, fire_at, interval, next_fire, data, active, created_at FROM reminders",
        )
        .map_err(|e| format!("[reminders] Failed to query: {}", e))?;
    let rows: Vec<ReminderEntry> = stmt
        .query_map([], |row| {
            Ok(ReminderEntry {
                id: row.get::<_, String>(0)?,
                message: row.get::<_, String>(1)?,
                fire_at: row.get::<_, f64>(2)?,
                interval: row.get::<_, f64>(3)?,
                next_fire: row.get::<_, f64>(4)?,
                data: row.get::<_, String>(5)?,
                active: row.get::<_, i32>(6)? != 0,
                created_at: row.get::<_, f64>(7)?,
            })
        })
        .map_err(|e| format!("[reminders] Failed to iterate: {}", e))?
        .filter_map(|r| r.ok())
        .collect();
    if let Ok(mut store) = reminders_store().lock() {
        store.extend(rows);
    }
    drop(stmt);
    let mut guard = reminders_sqlite()
        .lock()
        .map_err(|e| format!("[reminders] lock error: {}", e))?;
    *guard = Some(conn);
    eprintln!("[reminders] SQLite persistence enabled: {}", db_path);
    Ok(())
}

/// Write a single reminder to SQLite (write-through, called after mutations).
fn reminder_sqlite_upsert(entry: &ReminderEntry) {
    if let Ok(guard) = reminders_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO reminders (id, message, fire_at, interval, next_fire, data, active, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                rusqlite::params![entry.id, entry.message, entry.fire_at, entry.interval, entry.next_fire, entry.data, entry.active as i32, entry.created_at],
            );
        }
    }
}

#[allow(dead_code)]
fn reminder_sqlite_delete(id: &str) {
    if let Ok(guard) = reminders_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute("DELETE FROM reminders WHERE id = ?1", rusqlite::params![id]);
        }
    }
}

#[allow(dead_code)]
fn reminder_sqlite_delete_all_for_persona() {
    if let Ok(guard) = reminders_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute("DELETE FROM reminders", []);
        }
    }
}

/// `remind(message, timestamp, data?)` — one-time reminder. Returns ID.
pub(crate) fn builtin_remind(args: &[Value]) -> Result<Value, String> {
    let message = expect_string_arg("remind", args, 0)?;
    let fire_at = expect_float_arg("remind", args, 1)?;
    let data = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        Some(o) => format!("{}", o),
        None => String::new(),
    };
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let mut store = reminders_store()
        .lock()
        .map_err(|e| format!("remind() lock error: {}", e))?;
    let entry = ReminderEntry {
        id: id.clone(),
        message,
        fire_at,
        interval: 0.0,
        next_fire: fire_at,
        data,
        active: true,
        created_at: now_ts,
    };
    reminder_sqlite_upsert(&entry);
    store.push(entry);
    Ok(Value::String(id))
}

/// `remind_recurring(message, interval_seconds, data?)` — recurring reminder. Returns ID.
pub(crate) fn builtin_remind_recurring(args: &[Value]) -> Result<Value, String> {
    let message = expect_string_arg("remind_recurring", args, 0)?;
    let interval = expect_float_arg("remind_recurring", args, 1)?;
    if interval <= 0.0 {
        return Err("remind_recurring() interval must be positive".to_string());
    }
    let data = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        Some(o) => format!("{}", o),
        None => String::new(),
    };
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let mut store = reminders_store()
        .lock()
        .map_err(|e| format!("remind_recurring() lock error: {}", e))?;
    let entry = ReminderEntry {
        id: id.clone(),
        message,
        fire_at: now_ts,
        interval,
        next_fire: now_ts + interval,
        data,
        active: true,
        created_at: now_ts,
    };
    reminder_sqlite_upsert(&entry);
    store.push(entry);
    Ok(Value::String(id))
}

/// `cancel_remind(id)` — cancel reminder. Returns "ok" or "not_found".
pub(crate) fn builtin_cancel_remind(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("cancel_remind", args, 0)?;
    let mut store = reminders_store()
        .lock()
        .map_err(|e| format!("cancel_remind() lock error: {}", e))?;
    for entry in store.iter_mut() {
        if entry.id == id && entry.active {
            entry.active = false;
            reminder_sqlite_upsert(entry);
            return Ok(Value::String("ok".to_string()));
        }
    }
    Ok(Value::String("not_found".to_string()))
}

/// `list_reminders()` — list all active reminders.
pub(crate) fn builtin_list_reminders(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let store = reminders_store()
        .lock()
        .map_err(|e| format!("list_reminders() lock error: {}", e))?;
    let mut result = Vec::new();
    for entry in store.iter().filter(|r| r.active) {
        let rtype = if entry.interval > 0.0 {
            "recurring"
        } else {
            "once"
        };
        let ec = entry.clone();
        result.push(make_date_struct(
            "Reminder",
            vec![
                ("id", Value::String(ec.id)),
                ("message", Value::String(ec.message)),
                ("fire_at", Value::Float(ec.fire_at)),
                ("interval", Value::Float(ec.interval)),
                ("next_fire", Value::Float(ec.next_fire)),
                ("data", Value::String(ec.data)),
                ("created_at", Value::Float(ec.created_at)),
                ("type", Value::String(rtype.to_string())),
            ],
        ));
    }
    Ok(Value::List(result))
}

/// `check_reminders()` — get due reminders. One-shot deactivated; recurring advanced.
pub(crate) fn builtin_check_reminders(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let mut store = reminders_store()
        .lock()
        .map_err(|e| format!("check_reminders() lock error: {}", e))?;
    let mut due = Vec::new();
    for entry in store.iter_mut() {
        if !entry.active {
            continue;
        }
        if now_ts >= entry.next_fire {
            let rtype = if entry.interval > 0.0 {
                "recurring"
            } else {
                "once"
            };
            due.push(make_date_struct(
                "DueReminder",
                vec![
                    ("id", Value::String(entry.id.clone())),
                    ("message", Value::String(entry.message.clone())),
                    ("data", Value::String(entry.data.clone())),
                    ("type", Value::String(rtype.to_string())),
                    ("next_fire", Value::Float(entry.next_fire)),
                    ("overdue_seconds", Value::Float(now_ts - entry.next_fire)),
                ],
            ));
            if entry.interval > 0.0 {
                entry.next_fire += entry.interval;
            } else {
                entry.active = false;
            }
            reminder_sqlite_upsert(entry);
        }
    }
    Ok(Value::List(due))
}

// ── Cron Scheduler (inspired by OpenHuman cron_add/cron_list/cron_remove/cron_run) ──
// Stores cron jobs in KV store under "cron_jobs" key as JSON array.
// The server.rs scheduler loop (5s tick) checks these jobs and fires due ones.

fn get_cron_jobs() -> Vec<serde_json::Value> {
    let store = kv_store().lock().ok();
    let sqlite = kv_sqlite().lock().ok();
    let raw = match (store, sqlite) {
        (Some(s), _) => s.get("cron_jobs").cloned(),
        (_, Some(guard)) => guard.as_ref().and_then(|conn| {
            conn.query_row(
                "SELECT value FROM kv_store WHERE key = 'cron_jobs'",
                [],
                |row| row.get(0),
            )
            .ok()
        }),
        _ => None,
    };
    raw.and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default()
}

fn save_cron_jobs(jobs: &[serde_json::Value]) {
    let json = serde_json::to_string(jobs).unwrap_or_else(|_| "[]".to_string());
    if let Ok(mut store) = kv_store().lock() {
        store.insert("cron_jobs".to_string(), json.clone());
    }
    if let Ok(guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES ('cron_jobs', ?1)",
                rusqlite::params![json],
            );
        }
    }
}

/// `cron_add(cron_expr, prompt)` — register a recurring cron job.
/// Returns Struct { id, cron_expr, prompt, enabled, next_run, status }.
/// cron_expr: "0 9 * * 1-5" (standard 5-field cron: min hour dom month dow)
/// The server scheduler tick loop fires due jobs by calling the prompt as a pattern.
///
/// №418 (issue #571): optional positional extensions —
/// `cron_add(expr, prompt, tz?, catch_up?, payload?)`:
///   - `tz` — an IANA timezone name (e.g. "Europe/Moscow"); the job's
///     windows are matched IN this TZ. Default: env `MLOG_CRON_TZ`, else UTC.
///   - `catch_up` — "run_once" (default) or "skip": what to do with the
///     windows missed while the process was down/asleep. run_once coalesces
///     every missed window into ONE fire at the next tick; skip drops them
///     (the next REGULAR window fires normally).
///   - `payload` — a fixed JSON (any) string handed to the target
///     builtin/pattern at dispatch. It is DATA, never interpreted as code;
///     the dispatch surface does not expand (only the registered target is
///     called). Zero-arg patterns must not set a payload — a pattern that
///     wants the payload declares one String parameter.
pub(crate) fn builtin_cron_add(args: &[Value]) -> Result<Value, String> {
    const FN: &str = "cron_add";
    if args.len() > 5 {
        return Err(format!(
            "{}: expects 2..5 arguments (cron_expr, prompt, tz?, catch_up?, payload?), got {}",
            FN,
            args.len()
        ));
    }
    let cron_expr = expect_string_arg(FN, args, 0)?;
    let prompt = expect_string_arg(FN, args, 1)?;
    // Validate cron expression has 5 fields
    let parts: Vec<&str> = cron_expr.split_whitespace().collect();
    if parts.len() != 5 {
        return Err(
            "cron_add() expects a 5-field cron expression (min hour dom month dow)".to_string(),
        );
    }
    // №418 D3: optional per-job IANA timezone (validated loudly here).
    let tz = match args.get(2) {
        None => String::new(), // default resolution at decision time (env/UTC)
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                String::new()
            } else {
                s.trim()
                    .parse::<chrono_tz::Tz>()
                    .map_err(|_| format!("{}: unknown IANA timezone '{}'", FN, s.trim()))?;
                s.trim().to_string()
            }
        }
        Some(o) => return Err(format!("{}: tz must be String, got {}", FN, o.type_name())),
    };
    // №418 D2: optional per-job catch-up policy.
    let catch_up = match args.get(3) {
        None => String::new(),
        Some(Value::String(s)) => {
            let s = s.trim();
            if s.is_empty() {
                String::new()
            } else if s == "run_once" || s == "skip" {
                s.to_string()
            } else {
                return Err(format!(
                    "{}: catch_up must be \"run_once\" or \"skip\", got '{}'",
                    FN, s
                ));
            }
        }
        Some(o) => {
            return Err(format!(
                "{}: catch_up must be String, got {}",
                FN,
                o.type_name()
            ))
        }
    };
    // №418 D4: optional fixed payload (DATA — never interpreted as code).
    let payload = match args.get(4) {
        None => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(o) => {
            return Err(format!(
                "{}: payload must be String, got {}",
                FN,
                o.type_name()
            ))
        }
    };
    let id = format!("cron_{}", chrono_now_timestamp());
    let mut jobs = get_cron_jobs();
    let job = serde_json::json!({
        "id": id,
        "cron_expr": cron_expr,
        "prompt": prompt,
        "enabled": true,
        "created_at": chrono_now_timestamp(),
        "last_run": serde_json::Value::Null,
        "run_count": 0,
        "tz": tz,
        "catch_up": catch_up,
        "payload": payload,
        "last_window": serde_json::Value::Null,
    });
    jobs.push(job);
    save_cron_jobs(&jobs);
    Ok(make_date_struct(
        "CronJob",
        vec![
            ("id", Value::String(id)),
            ("cron_expr", Value::String(cron_expr)),
            ("prompt", Value::String(prompt)),
            ("enabled", Value::Float(1.0)),
            ("status", Value::String("created".to_string())),
        ],
    ))
}

/// `cron_list()` — list all registered cron jobs.
/// Returns List of Struct { id, cron_expr, prompt, enabled, created_at,
/// run_count, last_run, last_window, tz, catch_up, payload,
/// next_run, next_run_tz, last_run_tz }.
/// №418 (D6): `last_run` IS surfaced now (the REFERENCE drift is closed);
/// `next_run`/`last_run` are given both as epoch seconds and as ISO strings
/// in the JOB's timezone (`*_tz`) — the office (№419) reads Europe/Moscow
/// schedules in its own wall clock.
pub(crate) fn builtin_cron_list(args: &[Value]) -> Result<Value, String> {
    let _ = args; // variadic
    let jobs = get_cron_jobs();
    let mut result = Vec::new();
    for job in &jobs {
        let force_run = job["force_run"].as_bool().unwrap_or(false);
        // №418 D3: resolve the job TZ for the *_tz read-outs.
        let tz_res = resolve_job_tz(job["tz"].as_str());
        let tz = tz_res.clone().ok();
        let last_run = job["last_run"].as_f64();
        let next_run = tz_res.ok().and_then(|tz| {
            next_window_after(
                job["cron_expr"].as_str().unwrap_or(""),
                tz,
                chrono_now_timestamp(),
            )
        });
        let iso_in = |epoch: Option<f64>, tz: Option<chrono_tz::Tz>| -> String {
            match (epoch, tz) {
                (Some(e), Some(tz)) => chrono::DateTime::from_timestamp(e as i64, 0)
                    .map(|d| d.with_timezone(&tz).to_rfc3339())
                    .unwrap_or_default(),
                _ => String::new(),
            }
        };
        result.push(make_date_struct(
            "CronJob",
            vec![
                (
                    "id",
                    Value::String(job["id"].as_str().unwrap_or("").to_string()),
                ),
                (
                    "cron_expr",
                    Value::String(job["cron_expr"].as_str().unwrap_or("").to_string()),
                ),
                (
                    "prompt",
                    Value::String(job["prompt"].as_str().unwrap_or("").to_string()),
                ),
                (
                    "enabled",
                    Value::Float(if job["enabled"].as_bool().unwrap_or(false) {
                        1.0
                    } else {
                        0.0
                    }),
                ),
                (
                    "run_count",
                    Value::Float(job["run_count"].as_u64().unwrap_or(0) as f64),
                ),
                ("force_run", Value::Float(if force_run { 1.0 } else { 0.0 })),
                (
                    "created_at",
                    Value::Float(job["created_at"].as_f64().unwrap_or(0.0)),
                ),
                // D6: the last_run drift — surfaced in both readings.
                (
                    "last_run",
                    match last_run {
                        Some(e) => Value::Float(e),
                        None => Value::Unit,
                    },
                ),
                ("last_run_tz", Value::String(iso_in(last_run, tz))),
                (
                    "last_window",
                    match job["last_window"].as_f64() {
                        Some(w) => Value::Float(w),
                        None => Value::Unit,
                    },
                ),
                (
                    "tz",
                    Value::String(job["tz"].as_str().unwrap_or("").to_string()),
                ),
                (
                    "catch_up",
                    Value::String(job["catch_up"].as_str().unwrap_or("").to_string()),
                ),
                (
                    "payload",
                    Value::String(job["payload"].as_str().unwrap_or("").to_string()),
                ),
                (
                    "next_run",
                    match next_run {
                        Some(e) => Value::Float(e as f64),
                        None => Value::Unit,
                    },
                ),
                (
                    "next_run_tz",
                    Value::String(iso_in(next_run.map(|e| e as f64), tz)),
                ),
            ],
        ));
    }
    Ok(Value::List(result))
}

/// `cron_remove(id)` — remove a cron job by id.
/// Returns Struct { removed: Float, status: String }.
pub(crate) fn builtin_cron_remove(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("cron_remove", args, 0)?;
    let jobs = get_cron_jobs();
    let before = jobs.len();
    let filtered: Vec<serde_json::Value> = jobs
        .into_iter()
        .filter(|j| j["id"].as_str() != Some(&id))
        .collect();
    let removed = (before - filtered.len()) as f64;
    save_cron_jobs(&filtered);
    let status = if removed > 0.0 {
        "removed"
    } else {
        "not_found"
    };
    Ok(make_date_struct(
        "CronRemoveResult",
        vec![
            ("removed", Value::Float(removed)),
            ("status", Value::String(status.to_string())),
        ],
    ))
}

/// `cron_run(id)` — immediately execute a cron job (bypass schedule).
/// Returns Struct { id, executed: Float, status: String }.
/// Note: actual execution dispatch is handled by the server scheduler.
/// This builtin marks the job for immediate execution on next tick.
pub(crate) fn builtin_cron_run(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("cron_run", args, 0)?;
    let mut jobs = get_cron_jobs();
    let mut found = false;
    for job in &mut jobs {
        if job["id"].as_str() == Some(&id) {
            job["force_run"] = serde_json::Value::Bool(true);
            found = true;
            break;
        }
    }
    if found {
        save_cron_jobs(&jobs);
        Ok(make_date_struct(
            "CronRunResult",
            vec![
                ("id", Value::String(id)),
                ("executed", Value::Float(1.0)),
                ("status", Value::String("queued".to_string())),
            ],
        ))
    } else {
        Ok(make_date_struct(
            "CronRunResult",
            vec![
                ("id", Value::String(id)),
                ("executed", Value::Float(0.0)),
                ("status", Value::String("not_found".to_string())),
            ],
        ))
    }
}

/// `cron_mark_fired(id)` — internal: reset force_run, increment run_count, set last_run.
/// Called by the server scheduler after dispatching a cron job.
/// Returns Struct { id, status }.
pub(crate) fn builtin_cron_mark_fired(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("cron_mark_fired", args, 0)?;
    // №418 D1: without an explicit window, the window identity is the
    // job's current matching window (computed here so a legacy caller
    // keeps the dedup contract).
    let jobs = get_cron_jobs();
    let window = jobs
        .iter()
        .find(|j| j["id"].as_str() == Some(&id))
        .and_then(|j| {
            let tz = resolve_job_tz(j["tz"].as_str()).ok()?;
            last_window_before(
                j["cron_expr"].as_str().unwrap_or(""),
                tz,
                chrono_now_timestamp(),
                None,
            )
        });
    mark_fired_with_window(&id, window)
}

/// №418 D1: the scheduler calls this with the DETERMINISTIC window it
/// decided on (`FireDecision.window`) — the dedup stamp and the fire are
/// the same transaction from the loop's point of view.
pub fn mark_fired_with_window(id: &str, window: Option<i64>) -> Result<Value, String> {
    let mut jobs = get_cron_jobs();
    let mut found = false;
    for job in &mut jobs {
        if job["id"].as_str() == Some(id) {
            job["force_run"] = serde_json::Value::Bool(false);
            let count = job["run_count"].as_u64().unwrap_or(0) + 1;
            job["run_count"] = serde_json::Value::Number(count.into());
            job["last_run"] = serde_json::Value::Number(chrono_now_timestamp().into());
            if let Some(w) = window {
                job["last_window"] = serde_json::Value::Number(w.into());
            }
            found = true;
            break;
        }
    }
    if found {
        save_cron_jobs(&jobs);
        Ok(make_date_struct(
            "CronMarkResult",
            vec![
                ("id", Value::String(id.to_string())),
                ("status", Value::String("fired".to_string())),
            ],
        ))
    } else {
        Ok(make_date_struct(
            "CronMarkResult",
            vec![
                ("id", Value::String(id.to_string())),
                ("status", Value::String("not_found".to_string())),
            ],
        ))
    }
}

/// №418 D2: consume a window WITHOUT firing (the `skip` catch-up policy
/// and the pre-creation guard) — advances the dedup stamp so the same
/// window is not re-evaluated every tick.
pub fn advance_window(id: &str, window: i64) -> Result<Value, String> {
    let mut jobs = get_cron_jobs();
    let mut found = false;
    for job in &mut jobs {
        if job["id"].as_str() == Some(id) {
            if job["last_window"].as_f64().unwrap_or(f64::NEG_INFINITY) < window as f64 {
                job["last_window"] = serde_json::Value::Number(window.into());
            }
            found = true;
            break;
        }
    }
    if found {
        save_cron_jobs(&jobs);
    }
    Ok(Value::Unit)
}

// ── Naryad #418 (issue #571): the reliability core (D1–D5) ──────────────
//
// The fire decision is a PURE function over a parsed job spec + the wall
// clock: `cron_fire_decision` is the single place that decides whether a
// tick fires a job, so the dedup window (D1), the catch-up policy (D2) and
// the timezone identity (D3) are all pinned by tests against ONE
// implementation. The server's 5s tick loop (Phase 3) only executes the
// decision: dispatch → mark_fired_with_window / advance_window.

use chrono::Datelike as _;
use chrono::Timelike as _;

/// Check if a cron field (min/hour/dom/month/dow) matches a value.
/// Supports: `*`, `*/N`, `N`, `N-M`, `N,M,O`, `N-M/S`.
/// (Moved here from server.rs by №418 — the cron subsystem owns its
/// matching; server.rs imports it.)
pub(crate) fn cron_field_matches(field: &str, value: u32) -> bool {
    for part in field.split(',') {
        let part = part.trim();
        if part == "*" {
            return true;
        }
        if let Some(step_str) = part.strip_prefix("*/") {
            if let Ok(step) = step_str.parse::<u32>() {
                if step == 0 {
                    continue;
                }
                if value.is_multiple_of(step) {
                    return true;
                }
            }
            continue;
        }
        // Handle range with optional step: N-M or N-M/S
        if part.contains('-') {
            let segments: Vec<&str> = part.split('/').collect();
            let range_str = segments[0];
            let step: u32 = if segments.len() > 1 {
                segments[1].parse().unwrap_or(1)
            } else {
                1
            };
            if step == 0 {
                continue;
            }
            let bounds: Vec<&str> = range_str.split('-').collect();
            if bounds.len() == 2 {
                if let (Ok(lo), Ok(hi)) = (bounds[0].parse::<u32>(), bounds[1].parse::<u32>()) {
                    if value >= lo && value <= hi && (value - lo).is_multiple_of(step) {
                        return true;
                    }
                }
            }
            continue;
        }
        // Plain number
        if let Ok(n) = part.parse::<u32>() {
            if n == value {
                return true;
            }
        }
    }
    false
}

/// The per-job catch-up policy (D2). Default `RunOnce`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatchUpPolicy {
    /// All windows missed while down/asleep coalesce into ONE fire.
    RunOnce,
    /// Missed windows are dropped; the next regular window fires.
    Skip,
}

/// The parsed, scheduler-facing view of one stored job (additive fields
/// over the 0.20.x store shape — a job JSON without them migrates by
/// defaulting: tz → env `MLOG_CRON_TZ` → UTC; catch_up → RunOnce;
/// payload → none; last_window → never fired).
#[derive(Debug, Clone)]
pub struct CronJobSpec {
    pub id: String,
    pub cron_expr: String,
    /// The registered dispatch target: a builtin or pattern NAME (never
    /// code — the surface does not expand beyond the registry).
    pub prompt: String,
    pub tz: String,
    pub catch_up: CatchUpPolicy,
    pub payload: Option<String>,
    /// Epoch seconds of the last fired window's start (D1 dedup stamp).
    pub last_window: Option<i64>,
    pub force_run: bool,
    pub created_at: i64,
}

/// The pure fire decision for one tick of one job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FireDecision {
    pub fire: bool,
    /// The window identity this decision is about (epoch seconds of the
    /// window's start minute) — the dedup stamp on fire/advance.
    pub window: Option<i64>,
    /// Advance the stamp WITHOUT firing (the skip policy consumed a
    /// missed window; a pre-creation window was passed over).
    pub advance: bool,
    pub reason: &'static str,
}

/// D3: the effective timezone of a job — the explicit per-job IANA name,
/// else the env default `MLOG_CRON_TZ`, else UTC. Unknown names are LOUD
/// (the caller stamps `CRON_JOB_FAILED`); a cron job must never silently
/// drift to another zone.
pub fn resolve_job_tz(explicit: Option<&str>) -> Result<chrono_tz::Tz, String> {
    let name = match explicit {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => std::env::var("MLOG_CRON_TZ").unwrap_or_else(|_| "UTC".to_string()),
    };
    name.parse::<chrono_tz::Tz>()
        .map_err(|_| format!("unknown IANA timezone '{}'", name))
}

/// Does the 5-field expression match the wall clock of `at_epoch` as seen
/// in `tz`? (Fields: min hour dom month dow; dow 0=Sunday.)
fn expr_matches_at(expr: &str, tz: chrono_tz::Tz, at_epoch: i64) -> bool {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() != 5 {
        return false;
    }
    let Some(local) = chrono::DateTime::from_timestamp(at_epoch, 0) else {
        return false;
    };
    let local = local.with_timezone(&tz);
    let min = local.minute();
    let hour = local.hour();
    let dom = local.day(); // 1-31
    let month = local.month(); // 1-12
    let dow = local.weekday().num_days_from_sunday(); // 0=Sun
    cron_field_matches(parts[0], min)
        && cron_field_matches(parts[1], hour)
        && cron_field_matches(parts[2], dom)
        && cron_field_matches(parts[3], month)
        && cron_field_matches(parts[4], dow)
}

/// The latest matching window START (epoch seconds, the wall-minute's
/// beginning) at or before `at_epoch`, in the job's timezone. The scan is
/// bounded by the job's own horizon (its `last_window` / `created_at`):
/// the dedup stamp IS a matching window, so reaching it answers the query.
pub fn last_window_before(
    expr: &str,
    tz: chrono_tz::Tz,
    at_epoch: i64,
    horizon: Option<i64>,
) -> Option<i64> {
    let start = (at_epoch / 60) * 60;
    for cand in (0..=start).rev().step_by(60) {
        if expr_matches_at(expr, tz, cand) {
            return Some(cand);
        }
        if let Some(h) = horizon {
            if cand <= h {
                return None; // the horizon itself was the latest candidate
            }
        }
    }
    None
}

/// The next matching window strictly after `after_epoch` (cron_list's
/// `next_run`), capped at 366 days of forward scan.
pub fn next_window_after(expr: &str, tz: chrono_tz::Tz, after_epoch: i64) -> Option<i64> {
    let start = ((after_epoch / 60) + 1) * 60;
    let cap = start + 60 * 60 * 24 * 366;
    let mut cand = start;
    while cand < cap {
        if expr_matches_at(expr, tz, cand) {
            return Some(cand);
        }
        cand += 60;
    }
    None
}

/// D1+D2+D3: the pure tick decision. One matched window fires AT MOST
/// once (the dedup stamp `last_window`), manual `force_run` fires
/// immediately and stamps the window it fired in, missed windows follow
/// the per-job catch-up policy, and every decision names its reason so
/// the scheduler's log is auditable.
pub fn cron_fire_decision(spec: &CronJobSpec, now_epoch: i64) -> Result<FireDecision, String> {
    let tz = resolve_job_tz(Some(&spec.tz))?;
    // Manual run: an explicit override — fire now, stamp the CURRENT
    // window (a scheduled tick in the same window will not re-fire).
    if spec.force_run {
        let window = last_window_before(&spec.cron_expr, tz, now_epoch, None)
            .or_else(|| Some((now_epoch / 60) * 60));
        return Ok(FireDecision {
            fire: true,
            window,
            advance: false,
            reason: "force_run",
        });
    }
    // The horizon bounds the backward scan: the latest window that could
    // still need a decision is the one after the stamp (or after creation).
    let horizon = spec.last_window.or(Some(spec.created_at));
    let Some(current) = last_window_before(&spec.cron_expr, tz, now_epoch, horizon) else {
        return Ok(FireDecision {
            fire: false,
            window: None,
            advance: false,
            reason: "no-window",
        });
    };
    // D1: this exact window already fired (or was consumed) — never twice.
    if spec.last_window == Some(current) {
        return Ok(FireDecision {
            fire: false,
            window: Some(current),
            advance: false,
            reason: "already-fired",
        });
    }
    // A window that predates the job's creation is never catch-up.
    if current < spec.created_at && spec.last_window.is_none() {
        return Ok(FireDecision {
            fire: false,
            window: Some(current),
            advance: true,
            reason: "pre-creation",
        });
    }
    let inside = now_epoch < current + 60;
    if inside {
        return Ok(FireDecision {
            fire: true,
            window: Some(current),
            advance: false,
            reason: "due",
        });
    }
    // We are PAST the window (the process slept / deployed through it):
    // D2 — the per-job policy decides.
    match spec.catch_up {
        CatchUpPolicy::RunOnce => Ok(FireDecision {
            fire: true,
            window: Some(current),
            advance: false,
            reason: "catch-up",
        }),
        CatchUpPolicy::Skip => Ok(FireDecision {
            fire: false,
            window: Some(current),
            advance: true,
            reason: "skipped",
        }),
    }
}

/// D4: the dispatch arguments for a fire — the fixed payload (DATA, never
/// code) as the single String argument, or no arguments for the classic
/// zero-arg surface.
pub fn cron_dispatch_args(payload: Option<&str>) -> Vec<Value> {
    match payload {
        Some(p) if !p.is_empty() => vec![Value::String(p.to_string())],
        _ => Vec::new(),
    }
}

/// D5: deliver due reminders to the SAME dispatch surface the cron jobs
/// use (a pattern by name — the office convention is `ReminderCheck`).
/// The eprintln journal in the tick loop stays; a failing EXISTING handler
/// is stamped `CRON_JOB_FAILED` (the №413 stamps hold on every fail path).
/// A missing handler is not an error — the journal line is the delivery.
pub fn deliver_due_reminders<D>(
    due: &[(String, String, String)],
    handler: Option<&str>,
    mut dispatch: D,
) -> Vec<String>
where
    D: FnMut(&str, &[Value]) -> Result<Value, String>,
{
    let mut failures = Vec::new();
    let Some(handler) = handler else {
        return failures; // no handler registered: journal-only (the status quo)
    };
    for (message, data, rtype) in due {
        let args = vec![
            Value::String(message.clone()),
            Value::String(data.clone()),
            Value::String(rtype.clone()),
        ];
        if let Err(e) = dispatch(handler, &args) {
            failures.push(cron_stamped(format!(
                "reminder delivery to '{}': {}",
                handler, e
            )));
        }
    }
    failures
}

/// Parse one stored job JSON into the scheduler spec (additive migration:
/// a 0.20.x job without the new fields defaults — tz env/UTC, RunOnce, no
/// payload, never fired).
pub fn job_spec_from_json(job: &serde_json::Value) -> CronJobSpec {
    CronJobSpec {
        id: job["id"].as_str().unwrap_or("").to_string(),
        cron_expr: job["cron_expr"].as_str().unwrap_or("").to_string(),
        prompt: job["prompt"].as_str().unwrap_or("").to_string(),
        tz: job["tz"].as_str().unwrap_or("").to_string(),
        catch_up: match job["catch_up"].as_str() {
            Some("skip") => CatchUpPolicy::Skip,
            _ => CatchUpPolicy::RunOnce,
        },
        payload: job["payload"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
        last_window: job["last_window"].as_f64().map(|w| w as i64),
        force_run: job["force_run"].as_bool().unwrap_or(false),
        created_at: job["created_at"].as_f64().unwrap_or(0.0) as i64,
    }
}

/// The scheduler's Phase-1 read: every ENABLED job as a parsed spec
/// (the tick loop calls this instead of re-parsing Value structs).
pub fn enabled_job_specs() -> Vec<CronJobSpec> {
    get_cron_jobs()
        .iter()
        .filter(|j| j["enabled"].as_bool().unwrap_or(false))
        .map(job_spec_from_json)
        .collect()
}
