// ── Cron Scheduler + Reminders builtins ──────────────────────────────────

use std::sync::Mutex as StdMutex;

use crate::interpreter::Value;

use super::chrono_now_timestamp;
use super::core::*;
use super::http::make_date_struct;
use super::memory::*;

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
pub(crate) fn builtin_cron_add(args: &[Value]) -> Result<Value, String> {
    let cron_expr = expect_string_arg("cron_add", args, 0)?;
    let prompt = expect_string_arg("cron_add", args, 1)?;
    // Validate cron expression has 5 fields
    let parts: Vec<&str> = cron_expr.split_whitespace().collect();
    if parts.len() != 5 {
        return Err(
            "cron_add() expects a 5-field cron expression (min hour dom month dow)".to_string(),
        );
    }
    let id = format!("cron_{}", chrono_now_timestamp());
    let mut jobs = get_cron_jobs();
    let job = serde_json::json!({
        "id": id,
        "cron_expr": cron_expr,
        "prompt": prompt,
        "enabled": true,
        "created_at": chrono_now_timestamp(),
        "last_run": serde_json::Value::Null,
        "run_count": 0
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
/// Returns List of Struct { id, cron_expr, prompt, enabled, created_at, run_count }.
pub(crate) fn builtin_cron_list(args: &[Value]) -> Result<Value, String> {
    let _ = args; // variadic
    let jobs = get_cron_jobs();
    let mut result = Vec::new();
    for job in &jobs {
        let force_run = job["force_run"].as_bool().unwrap_or(false);
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
    let mut jobs = get_cron_jobs();
    let mut found = false;
    for job in &mut jobs {
        if job["id"].as_str() == Some(&id) {
            job["force_run"] = serde_json::Value::Bool(false);
            let count = job["run_count"].as_u64().unwrap_or(0) + 1;
            job["run_count"] = serde_json::Value::Number(count.into());
            job["last_run"] = serde_json::Value::Number(chrono_now_timestamp().into());
            found = true;
            break;
        }
    }
    if found {
        save_cron_jobs(&jobs);
        Ok(make_date_struct(
            "CronMarkResult",
            vec![
                ("id", Value::String(id)),
                ("status", Value::String("fired".to_string())),
            ],
        ))
    } else {
        Ok(make_date_struct(
            "CronMarkResult",
            vec![
                ("id", Value::String(id)),
                ("status", Value::String("not_found".to_string())),
            ],
        ))
    }
}
