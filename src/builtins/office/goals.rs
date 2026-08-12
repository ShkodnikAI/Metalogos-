// ── Goals & todos builtins ──────────────────────────────────────────
// goal_set, goal_get, goal_complete, goals_add, goals_list, goals_reflect,
// todo_add, todo_update, todo_list

use super::super::core::*;
use super::super::http::*;
use super::super::memory::*;
use crate::interpreter::Value;

/// `goal_set(objective, budget?)` — set the current thread goal.
/// Returns Struct { objective, status, budget, spent }.
pub(crate) fn builtin_goal_set(args: &[Value]) -> Result<Value, String> {
    let objective = expect_string_arg("goal_set", args, 0)?;
    let budget = match args.get(1) {
        Some(Value::Float(f)) => Some(*f),
        _ => None,
    };
    let goal = serde_json::json!({
        "objective": objective,
        "status": "active",
        "budget": budget,
        "spent": 0.0,
        "set_at": super::config::chrono_now_timestamp()
    });
    let json = serde_json::to_string(&goal).unwrap_or_default();
    if let Ok(mut store) = kv_store().lock() {
        store.insert("thread_goal".to_string(), json.clone());
    }
    if let Ok(guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES ('thread_goal', ?1)",
                rusqlite::params![json],
            );
        }
    }
    Ok(make_date_struct(
        "ThreadGoal",
        vec![
            ("objective", Value::String(objective)),
            ("status", Value::String("active".to_string())),
            ("budget", Value::Float(budget.unwrap_or(0.0))),
            ("spent", Value::Float(0.0)),
        ],
    ))
}

/// `goal_get()` — get the current thread goal.
/// Returns Struct or empty struct if no goal is set.
pub(crate) fn builtin_goal_get(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let raw = kv_get_raw("thread_goal");
    if let Some(json_str) = raw {
        if let Ok(goal) = serde_json::from_str::<serde_json::Value>(&json_str) {
            return Ok(make_date_struct(
                "ThreadGoal",
                vec![
                    (
                        "objective",
                        Value::String(goal["objective"].as_str().unwrap_or("").to_string()),
                    ),
                    (
                        "status",
                        Value::String(goal["status"].as_str().unwrap_or("none").to_string()),
                    ),
                    (
                        "budget",
                        Value::Float(goal["budget"].as_f64().unwrap_or(0.0)),
                    ),
                    ("spent", Value::Float(goal["spent"].as_f64().unwrap_or(0.0))),
                ],
            ));
        }
    }
    Ok(make_date_struct(
        "ThreadGoal",
        vec![
            ("objective", Value::String("".to_string())),
            ("status", Value::String("none".to_string())),
            ("budget", Value::Float(0.0)),
            ("spent", Value::Float(0.0)),
        ],
    ))
}

/// `goal_complete()` — mark the current thread goal as complete.
/// Returns Struct { status, objective }.
pub(crate) fn builtin_goal_complete(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let raw = kv_get_raw("thread_goal");
    let objective = match raw {
        Some(ref json_str) => serde_json::from_str::<serde_json::Value>(json_str)
            .ok()
            .and_then(|g| g["objective"].as_str().map(|s| s.to_string()))
            .unwrap_or_default(),
        None => String::new(),
    };
    let goal = serde_json::json!({
        "objective": objective,
        "status": "complete",
        "completed_at": super::config::chrono_now_timestamp()
    });
    let json = serde_json::to_string(&goal).unwrap_or_default();
    if let Ok(mut store) = kv_store().lock() {
        store.insert("thread_goal".to_string(), json.clone());
    }
    if let Ok(guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES ('thread_goal', ?1)",
                rusqlite::params![json],
            );
        }
    }
    Ok(make_date_struct(
        "GoalComplete",
        vec![
            ("status", Value::String("complete".to_string())),
            ("objective", Value::String(objective)),
        ],
    ))
}

/// `goals_list()` — list all long-term goals.
/// Returns List of Struct { id, text, status }.
pub(crate) fn builtin_goals_list(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let raw = kv_get_raw("goals_list");
    let goals: Vec<serde_json::Value> = raw
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut result = Vec::new();
    for (i, g) in goals.iter().enumerate() {
        result.push(make_date_struct(
            "Goal",
            vec![
                ("id", Value::String(format!("g{}", i))),
                (
                    "text",
                    Value::String(g["text"].as_str().unwrap_or("").to_string()),
                ),
                (
                    "status",
                    Value::String(g["status"].as_str().unwrap_or("active").to_string()),
                ),
            ],
        ));
    }
    Ok(Value::List(result))
}

/// `goals_add(text)` — add a long-term goal (max 8).
/// Returns Struct { id, text, status }.
pub(crate) fn builtin_goals_add(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("goals_add", args, 0)?;
    let raw = kv_get_raw("goals_list");
    let mut goals: Vec<serde_json::Value> = raw
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if goals.len() >= 8 {
        return Err("goals_add() maximum 8 long-term goals".to_string());
    }
    let goal = serde_json::json!({
        "text": text,
        "status": "active",
        "added_at": super::config::chrono_now_timestamp()
    });
    let id = format!("g{}", goals.len());
    goals.push(goal);
    let json = serde_json::to_string(&goals).unwrap_or_default();
    if let Ok(mut store) = kv_store().lock() {
        store.insert("goals_list".to_string(), json.clone());
    }
    if let Ok(guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES ('goals_list', ?1)",
                rusqlite::params![json],
            );
        }
    }
    Ok(make_date_struct(
        "Goal",
        vec![
            ("id", Value::String(id)),
            ("text", Value::String(text)),
            ("status", Value::String("active".to_string())),
        ],
    ))
}

/// `goals_reflect()` — returns a summary of goals for reflection.
/// This is a stub: real implementation would call LLM to evaluate goals.
/// Returns Struct { goal_count, active, status }.
pub(crate) fn builtin_goals_reflect(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let raw = kv_get_raw("goals_list");
    let goals: Vec<serde_json::Value> = raw
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let active = goals
        .iter()
        .filter(|g| g["status"].as_str() == Some("active"))
        .count() as f64;
    Ok(make_date_struct(
        "GoalsReflection",
        vec![
            ("goal_count", Value::Float(goals.len() as f64)),
            ("active", Value::Float(active)),
            ("status", Value::String("ready_for_reflection".to_string())),
        ],
    ))
}

// ── Todos / Kanban (inspired by OpenHuman task board) ──
// Stored in KV store under "todos" key as JSON array.

/// `todo_add(title, status?)` — add a todo card. Default status: "todo".
/// Returns Struct { id, title, status, created_at }.
pub(crate) fn builtin_todo_add(args: &[Value]) -> Result<Value, String> {
    let title = expect_string_arg("todo_add", args, 0)?;
    let status = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => "todo".to_string(),
    };
    let valid = [
        "todo",
        "in_progress",
        "awaiting_approval",
        "ready",
        "blocked",
        "done",
        "rejected",
    ];
    if !valid.contains(&status.as_str()) {
        return Err(format!("todo_add() invalid status '{}'. Valid: todo, in_progress, awaiting_approval, ready, blocked, done, rejected", status));
    }
    let raw = kv_get_raw("todos");
    let mut todos: Vec<serde_json::Value> = raw
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let id = format!("todo_{}", super::config::chrono_now_timestamp());
    let todo = serde_json::json!({
        "id": id,
        "title": title,
        "status": status,
        "created_at": super::config::chrono_now_timestamp()
    });
    todos.push(todo);
    let json = serde_json::to_string(&todos).unwrap_or_default();
    if let Ok(mut store) = kv_store().lock() {
        store.insert("todos".to_string(), json.clone());
    }
    if let Ok(guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES ('todos', ?1)",
                rusqlite::params![json],
            );
        }
    }
    Ok(make_date_struct(
        "Todo",
        vec![
            ("id", Value::String(id)),
            ("title", Value::String(title)),
            ("status", Value::String(status)),
        ],
    ))
}

/// `todo_update(id, new_status)` — update a todo's status.
/// Returns Struct { id, old_status, new_status, updated }.
pub(crate) fn builtin_todo_update(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("todo_update", args, 0)?;
    let new_status = expect_string_arg("todo_update", args, 1)?;
    let raw = kv_get_raw("todos");
    let mut todos: Vec<serde_json::Value> = raw
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut updated = false;
    let mut old_status = "not_found".to_string();
    for todo in &mut todos {
        if todo["id"].as_str() == Some(&id) {
            old_status = todo["status"].as_str().unwrap_or("").to_string();
            todo["status"] = serde_json::Value::String(new_status.clone());
            todo["updated_at"] =
                serde_json::Value::Number(super::config::chrono_now_timestamp().into());
            updated = true;
            break;
        }
    }
    if updated {
        let json = serde_json::to_string(&todos).unwrap_or_default();
        if let Ok(mut store) = kv_store().lock() {
            store.insert("todos".to_string(), json.clone());
        }
        if let Ok(guard) = kv_sqlite().lock() {
            if let Some(ref conn) = *guard {
                let _ = conn.execute(
                    "INSERT OR REPLACE INTO kv_store (key, value) VALUES ('todos', ?1)",
                    rusqlite::params![json],
                );
            }
        }
    }
    Ok(make_date_struct(
        "TodoUpdate",
        vec![
            ("id", Value::String(id)),
            ("old_status", Value::String(old_status)),
            ("new_status", Value::String(new_status)),
            ("updated", Value::Float(if updated { 1.0 } else { 0.0 })),
        ],
    ))
}

/// `todo_list()` — list all todos.
/// Returns List of Struct { id, title, status, created_at }.
pub(crate) fn builtin_todo_list(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let raw = kv_get_raw("todos");
    let todos: Vec<serde_json::Value> = raw
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut result = Vec::new();
    for t in &todos {
        result.push(make_date_struct(
            "Todo",
            vec![
                (
                    "id",
                    Value::String(t["id"].as_str().unwrap_or("").to_string()),
                ),
                (
                    "title",
                    Value::String(t["title"].as_str().unwrap_or("").to_string()),
                ),
                (
                    "status",
                    Value::String(t["status"].as_str().unwrap_or("").to_string()),
                ),
            ],
        ));
    }
    Ok(Value::List(result))
}
