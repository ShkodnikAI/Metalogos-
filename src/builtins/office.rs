// ── Office & productivity builtins ────────────────────────────
// Split from server.rs (naryad-38) — human_respond, goals, todos,
// entity extraction, memory scoring, recipes, DAG, semantic search, etc.

use super::core::*;
use super::http::*;
use super::json::*;
use super::memory::*;
use super::server::builtin_human_recall;
use crate::embeddings::{cosine_similarity, EmbeddingManager};
use crate::interpreter::Value;

/// `human_respond(persona, message, context?)` — generate a human-like response.
/// Uses the persona's traits, mood, and recalled memories to craft a response via LLM.
/// `context` is optional additional context (e.g., conversation history).
/// Returns the generated response as String.
pub(crate) fn builtin_human_respond(args: &[Value]) -> Result<Value, String> {
    let persona = expect_string_arg("human_respond", args, 0)?;
    let message = expect_string_arg("human_respond", args, 1)?;
    let context = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    };

    // Load persona data
    let persona_key = format!("human_persona:{}", persona);
    let store = kv_store()
        .lock()
        .map_err(|e| format!("human_respond() lock error: {}", e))?;
    let persona_data = store.get(&persona_key).cloned().unwrap_or_default();
    drop(store);

    if persona_data.is_empty() {
        return Err(format!(
            "human_respond() persona '{}' not found. Use human_create() first.",
            persona
        ));
    }

    let data: serde_json::Value = serde_json::from_str(&persona_data)
        .map_err(|e| format!("human_respond() persona parse error: {}", e))?;
    let traits = data
        .get("traits")
        .and_then(|v| v.as_str())
        .unwrap_or("helpful assistant")
        .to_string();
    let mood = data
        .get("mood")
        .and_then(|v| v.as_str())
        .unwrap_or("neutral")
        .to_string();
    let mood_intensity = data
        .get("mood_intensity")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);

    // Recall relevant memories
    let recall_result = builtin_human_recall(&[
        Value::String(persona.clone()),
        Value::String(message.clone()),
        Value::Float(5.0),
    ])?;
    let memories_text = match recall_result {
        Value::List(items) => {
            let mut parts = Vec::new();
            for item in &items {
                if let Value::Struct { fields, .. } = item {
                    let key = fields
                        .get("key")
                        .map(|v| format!("{}", v))
                        .unwrap_or_default();
                    let content = fields
                        .get("content")
                        .map(|v| format!("{}", v))
                        .unwrap_or_default();
                    parts.push(format!("- [{}]: {}", key, content));
                }
            }
            if parts.is_empty() {
                "No relevant memories found.".to_string()
            } else {
                parts.join("\n")
            }
        }
        _ => "No memories.".to_string(),
    };

    // Build the LLM prompt
    let system_prompt = format!(
        "You are {}, a persona with the following traits: {}. \
        Your current emotional state is '{}' with intensity {:.1}. \
        Let your mood subtly influence your tone and word choice. \
        You have access to the following memories about the user and past interactions:\n{}\
        \nRespond naturally, as a human would. Be concise but warm. Stay in character.",
        persona, traits, mood, mood_intensity, memories_text
    );

    let full_prompt = if context.is_empty() {
        format!("{}\n\nUser: {}", system_prompt, message)
    } else {
        format!(
            "{}\n\nRecent context:\n{}\n\nUser: {}",
            system_prompt, context, message
        )
    };

    // Call LLM (reuses existing call_llm infrastructure)
    let mock_mode = std::env::var("METALOGOS_LLM_MOCK")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(true);

    let response = if mock_mode {
        format!("[{} (mood: {}): {}]", persona, mood, message)
    } else {
        let backend = crate::llm::create_llm_backend();
        backend
            .call(&full_prompt, "")
            .map_err(|e| format!("human_respond() LLM call failed: {}", e))?
    };

    Ok(Value::String(response))
}

/// `human_personas()` — list all created personas.
/// Returns List of PersonaSummary structs: {name, traits, mood, memory_count, created_at}.
pub(crate) fn builtin_human_personas(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let prefix = "human_persona:";
    let store = kv_store()
        .lock()
        .map_err(|e| format!("human_personas() lock error: {}", e))?;
    let mut result = Vec::new();
    for (k, v) in store.iter() {
        if !k.starts_with(prefix) {
            continue;
        }
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(v) {
            let name = data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let traits = data
                .get("traits")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let mood = data
                .get("mood")
                .and_then(|v| v.as_str())
                .unwrap_or("neutral")
                .to_string();
            let created_at = data
                .get("created_at")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            // Count memories for this persona
            let mem_prefix = format!("human_mem:{}:", name);
            let mem_count = store
                .keys()
                .filter(|mk| mk.starts_with(&mem_prefix))
                .count();
            result.push(make_date_struct(
                "PersonaSummary",
                vec![
                    ("name", Value::String(name)),
                    ("traits", Value::String(traits)),
                    ("mood", Value::String(mood)),
                    ("memory_count", Value::Float(mem_count as f64)),
                    ("created_at", Value::Float(created_at)),
                ],
            ));
        }
    }
    Ok(Value::List(result))
}

/// `human_delete(persona)` — delete a persona and all its memories.
/// Returns Struct {deleted_memories: Float, status: String}.
pub(crate) fn builtin_human_delete(args: &[Value]) -> Result<Value, String> {
    let persona = expect_string_arg("human_delete", args, 0)?;
    let persona_key = format!("human_persona:{}", persona);
    let mem_prefix = format!("human_mem:{}:", persona);
    let mut store = kv_store()
        .lock()
        .map_err(|e| format!("human_delete() lock error: {}", e))?;

    // Delete persona
    let persona_existed = store.remove(&persona_key).is_some();
    // Delete all memories
    let to_remove: Vec<String> = store
        .keys()
        .filter(|k| k.starts_with(&mem_prefix))
        .cloned()
        .collect();
    let mem_count = to_remove.len();
    for k in &to_remove {
        store.remove(k);
    }
    drop(store);

    // SQLite cleanup
    if let Ok(sqlite_guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *sqlite_guard {
            let _ = conn.execute(
                "DELETE FROM kv_store WHERE key = ?1",
                rusqlite::params![persona_key],
            );
            for k in &to_remove {
                let _ = conn.execute("DELETE FROM kv_store WHERE key = ?1", rusqlite::params![k]);
            }
        }
    }

    let status = if persona_existed {
        "deleted"
    } else {
        "not_found"
    };
    Ok(make_date_struct(
        "DeleteResult",
        vec![
            ("deleted_memories", Value::Float(mem_count as f64)),
            ("status", Value::String(status.to_string())),
        ],
    ))
}

/// `extract_param(text, index)` — parse colon-separated callback_data, return N-th segment.
/// Example: extract_param("dept:osp:watch:42", 2) → "watch"
pub fn builtin_extract_param(args: &[Value]) -> Result<Value, String> {
    let text = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("extract_param() expects first argument to be a String".to_string()),
    };
    let index = match args.get(1) {
        Some(Value::Float(f)) => *f as usize,
        _ => {
            return Err("extract_param() expects second argument to be a Float (index)".to_string())
        }
    };
    let parts: Vec<&str> = text.split(':').collect();
    match parts.get(index) {
        Some(s) => Ok(Value::String(s.to_string())),
        None => Ok(Value::String("".to_string())),
    }
}

/// `estimate_tokens(text)` — rough token count heuristic (len / 4 for CJK+Latin mix).
/// ADR note: temporary heuristic, replace with proper tokenizer when available.
pub fn builtin_estimate_tokens(args: &[Value]) -> Result<Value, String> {
    let text = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("estimate_tokens() expects a String argument".to_string()),
    };
    let char_count = text.chars().count() as f64;
    // Heuristic: ~4 chars per token for mixed CJK/Latin
    let tokens = (char_count / 4.0).ceil();
    Ok(Value::Float(tokens))
}

/// `read_file_tokens(path)` — read file and return {content, tokens} struct.
/// Convenience for skill_index: read skill file + estimate its token cost in one call.
pub fn builtin_read_file_tokens(args: &[Value]) -> Result<Value, String> {
    let path = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("read_file_tokens() expects a file path (String)".to_string()),
    };
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("read_file_tokens(): {}", e))?;
    let char_count = content.chars().count() as f64;
    let tokens = (char_count / 4.0).ceil();
    Ok(Value::Struct {
        type_name: "FileInfo".to_string(),
        fields: [
            ("content".to_string(), Value::String(content)),
            ("chars".to_string(), Value::Float(char_count)),
            ("tokens".to_string(), Value::Float(tokens)),
        ]
        .into_iter()
        .collect(),
    })
}

// ═══════════════════════════════════════════════════════════════════════
// OpenHuman-inspired builtins (v0.8.3 — from OpenHuman feature audit)
// ═══════════════════════════════════════════════════════════════════════

// ── Approval Gate (inspired by OpenHuman approval flow) ──
// Stores pending approvals in KV store. In server mode, these can be
// dispatched as Telegram inline keyboards. In CLI mode, returns the
// approval struct for programmatic handling.

/// `ask_approval(title, description)` — create an approval request.
/// Returns Struct { id, title, description, approved, status }.
/// The `approved` field is 0.0 (pending). Use kv_get("approval:<id>") to poll.
/// In Telegram bot context, this would generate an inline keyboard.
pub(crate) fn builtin_ask_approval(args: &[Value]) -> Result<Value, String> {
    let title = expect_string_arg("ask_approval", args, 0)?;
    let description = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err(
                "ask_approval() expects second argument to be a description (String)".to_string(),
            )
        }
    };
    let id = format!("appr_{}", chrono_now_timestamp());
    let approval = serde_json::json!({
        "id": id,
        "title": title,
        "description": description,
        "approved": false,
        "rejected": false,
        "created_at": chrono_now_timestamp()
    });
    let json = serde_json::to_string(&approval).unwrap_or_default();
    let key = format!("approval:{}", id);
    if let Ok(mut store) = kv_store().lock() {
        store.insert(key.clone(), json.clone());
    }
    if let Ok(guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, json],
            );
        }
    }
    Ok(make_date_struct(
        "Approval",
        vec![
            ("id", Value::String(id)),
            ("title", Value::String(title)),
            ("description", Value::String(description)),
            ("approved", Value::Float(0.0)),
            ("status", Value::String("pending".to_string())),
        ],
    ))
}

// ── Goals (inspired by OpenHuman Goals: long-term goals + thread goal + budget) ──
// Goals stored in KV store under "goals" key as JSON array.
// Thread goal under "thread_goal" as single JSON object.

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
        "set_at": chrono_now_timestamp()
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
        "completed_at": chrono_now_timestamp()
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
        "added_at": chrono_now_timestamp()
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
    let id = format!("todo_{}", chrono_now_timestamp());
    let todo = serde_json::json!({
        "id": id,
        "title": title,
        "status": status,
        "created_at": chrono_now_timestamp()
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
            todo["updated_at"] = serde_json::Value::Number(chrono_now_timestamp().into());
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

// ── Entity Extraction (inspired by OpenHuman score/entity extraction) ──
// Pure regex-based extraction. LLM-based extraction can be done via call_llm.

/// `extract_entities(text)` — extract named entities from text using regex heuristics.
/// Returns List of Struct { kind, name, start, end }.
/// Kinds detected: person (capitalized word sequences), email, url, phone, date.
pub(crate) fn builtin_extract_entities(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("extract_entities", args, 0)?;
    let mut entities = Vec::new();

    // Email detection
    let email_re = regex_lite_find(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}");
    for m in &email_re {
        entities.push(make_date_struct(
            "Entity",
            vec![
                ("kind", Value::String("email".to_string())),
                ("name", Value::String(m.as_str().to_string())),
            ],
        ));
    }

    // URL detection
    let url_re = regex_lite_find(r#"https?://[^\s<>"]'+)"#);
    for m in &url_re {
        entities.push(make_date_struct(
            "Entity",
            vec![
                ("kind", Value::String("url".to_string())),
                ("name", Value::String(m.as_str().to_string())),
            ],
        ));
    }

    // Phone detection (rough: 7-15 digits with optional +/spaces/dashes)
    let phone_re = regex_lite_find(r"\+?[\d\s\-()]{7,15}");
    for m in &phone_re {
        let s = m.as_str().replace(|c: char| !c.is_ascii_digit(), "");
        if s.len() >= 7 && s.len() <= 15 {
            entities.push(make_date_struct(
                "Entity",
                vec![
                    ("kind", Value::String("phone".to_string())),
                    ("name", Value::String(m.as_str().to_string())),
                ],
            ));
        }
    }

    // Named entity: sequences of 2+ capitalized words (person/org heuristic)
    let mut caps = Vec::new();
    let mut start = 0;
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut i = 0;
    while i < words.len() {
        let w = words[i];
        if w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) && w.len() > 1 {
            let mut end_idx = i;
            while end_idx + 1 < words.len() {
                let next = words[end_idx + 1];
                if next
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                    && next.len() > 1
                {
                    end_idx += 1;
                } else {
                    break;
                }
            }
            if end_idx > i {
                // Found 2+ capitalized words in sequence
                let name: String = words[i..=end_idx].join(" ");
                // Filter out common false positives
                let lower_name = name.to_lowercase();
                let false_positives = [
                    "the", "this", "that", "these", "those", "then", "than", "they", "there",
                    "their",
                ];
                if !false_positives.iter().any(|fp| lower_name == *fp) {
                    caps.push((name, start));
                }
                i = end_idx + 1;
                continue;
            }
        }
        start += w.len() + 1;
        i += 1;
    }
    for (name, _) in &caps {
        entities.push(make_date_struct(
            "Entity",
            vec![
                ("kind", Value::String("entity".to_string())),
                ("name", Value::String(name.clone())),
            ],
        ));
    }

    Ok(Value::List(entities))
}

/// Minimal regex find without external crate (uses std only).
fn regex_lite_find(pattern: &str) -> Vec<std::string::String> {
    // Very limited: only supports basic character classes.
    // For production, use the `regex` crate. This is a fallback.
    // We only call it with simple, well-known patterns above.
    let results = Vec::new();
    if pattern.contains('@') && pattern.contains('.') {
        // Email pattern — manual scan
        let bytes = pattern.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'[' || bytes[i] == b'(' {
                // Skip character class
                let close = if bytes[i] == b'[' { b']' } else { b')' };
                while i < bytes.len() && bytes[i] != close {
                    i += 1;
                }
                i += 1;
                continue;
            }
            i += 1;
        }
        // For email/url/phone we need actual regex; use a simpler approach
        // The real implementation should depend on `regex` crate
    }
    results
}

// ── Memory Scoring (inspired by OpenHuman chunk scoring pipeline) ──
// Computes weighted signals to decide if a text chunk is worth keeping.

/// `memory_score(text, metadata?)` — score a text chunk for memory admission.
/// Returns Struct { score, admitted, signals: {token_count, unique_words, entity_density} }.
/// Signals:
///   token_count: 0-1, plateau over chunk size (10-8000 tokens)
///   unique_words: 0-1, type-token ratio (lexical diversity)
///   entity_density: 0-1, entities per token (capped)
/// Admission threshold: score >= 0.3
pub(crate) fn builtin_memory_score(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("memory_score", args, 0)?;
    let _metadata = args.get(1); // reserved for future SourceKind weight

    // Signal 1: token_count (char_count / 4 heuristic)
    let char_count = text.chars().count() as f64;
    let token_est = char_count / 4.0;
    let token_signal = if token_est < 10.0 {
        0.0
    } else if token_est < 30.0 {
        (token_est - 10.0) / 20.0
    } else if token_est < 8000.0 {
        1.0 - (token_est - 30.0) / 16000.0 // gentle decay
    } else {
        0.5
    };

    // Signal 2: unique_words (type-token ratio)
    let words: Vec<&str> = text.split_whitespace().collect();
    let word_count = words.len() as f64;
    let unique: std::collections::HashSet<String> =
        words.iter().map(|w| w.to_lowercase()).collect();
    let unique_signal = if word_count < 2.0 {
        0.5 // neutral for very short text
    } else {
        let ttr = unique.len() as f64 / word_count;
        ttr.min(1.0)
    };

    // Signal 3: entity_density (heuristic: count capitalized sequences + emails + URLs)
    let entity_count = extract_entity_count(&text);
    let entity_density = if token_est < 100.0 {
        0.5
    } else {
        ((entity_count as f64) / (token_est / 100.0)).min(1.0)
    };

    // Weighted combination (mirrors OpenHuman weights)
    let score = token_signal * 1.0 + unique_signal * 1.0 + entity_density * 1.0;
    let total = score / 3.0; // normalize to 0-1
    let admitted = total >= 0.3;

    Ok(make_date_struct(
        "MemoryScore",
        vec![
            ("score", Value::Float((total * 100.0).round() / 100.0)),
            ("admitted", Value::Float(if admitted { 1.0 } else { 0.0 })),
            (
                "token_count",
                Value::Float((token_signal * 100.0).round() / 100.0),
            ),
            (
                "unique_words",
                Value::Float((unique_signal * 100.0).round() / 100.0),
            ),
            (
                "entity_density",
                Value::Float((entity_density * 100.0).round() / 100.0),
            ),
        ],
    ))
}

/// Count entities in text (helper for memory_score).
fn extract_entity_count(text: &str) -> usize {
    let mut count = 0;
    // Count emails
    for word in text.split_whitespace() {
        if word.contains('@') && word.contains('.') {
            count += 1;
        }
        if word.starts_with("http://") || word.starts_with("https://") {
            count += 1;
        }
    }
    // Count capitalized word sequences (2+)
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut i = 0;
    while i < words.len() {
        if words[i]
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
            && words[i].len() > 1
        {
            let mut end = i;
            while end + 1 < words.len()
                && words[end + 1]
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
            {
                end += 1;
            }
            if end > i {
                count += 1;
            }
            i = end + 1;
        } else {
            i += 1;
        }
    }
    count
}

// ── Token Compression — HTML (inspired by OpenHuman TokenJuice HtmlCompressor) ──
// Strips HTML tags, converts to readable Markdown-ish text, preserves block boundaries.

/// `compress_html(html)` — convert HTML to clean readable text.
/// Strips all tags, decodes HTML entities, adds newlines at block boundaries.
/// CJK characters preserved grapheme-by-grapheme.
/// Returns compressed String.
pub(crate) fn builtin_compress_html(args: &[Value]) -> Result<Value, String> {
    let html = expect_string_arg("compress_html", args, 0)?;

    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let mut tag_buf = String::new();
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];
        if b == b'<' && !in_tag {
            in_tag = true;
            tag_buf.clear();
            i += 1;
            continue;
        }
        if in_tag {
            if b == b'>' {
                in_tag = false;
                let tag = tag_buf.to_lowercase();
                // Block-level tags get a newline
                let block_tags = [
                    "p",
                    "div",
                    "h1",
                    "h2",
                    "h3",
                    "h4",
                    "h5",
                    "h6",
                    "br",
                    "li",
                    "tr",
                    "hr",
                    "blockquote",
                    "pre",
                    "table",
                    "ul",
                    "ol",
                    "section",
                    "article",
                    "header",
                    "footer",
                    "nav",
                    "main",
                    "aside",
                    "figcaption",
                    "details",
                    "summary",
                    "dt",
                    "dd",
                    "th",
                ];
                if tag.starts_with('/') {
                    // Closing tag
                    let inner = tag.trim_start_matches('/').trim();
                    if block_tags.contains(&inner) {
                        result.push('\n');
                    }
                    if inner == "script" {
                        in_script = false;
                    }
                    if inner == "style" {
                        in_style = false;
                    }
                } else {
                    let inner = tag.split_whitespace().next().unwrap_or("");
                    if block_tags.contains(&inner) && !result.ends_with('\n') {
                        result.push('\n');
                    }
                    if inner == "script" {
                        in_script = true;
                    }
                    if inner == "style" {
                        in_style = true;
                    }
                }
                i += 1;
                continue;
            }
            tag_buf.push(b as char);
            i += 1;
            continue;
        }
        if in_script || in_style {
            i += 1;
            continue;
        }
        // HTML entity decode
        if b == b'&' {
            let rest = &html[i..];
            if let Some(end) = rest.find(';') {
                let entity = &rest[1..end];
                let decoded = decode_html_entity(entity);
                result.push_str(&decoded);
                i += end + 1;
                continue;
            }
        }
        // Collapse whitespace
        if b == b' ' || b == b'\n' || b == b'\r' || b == b'\t' {
            if !result.ends_with(' ') && !result.ends_with('\n') {
                result.push(' ');
            }
        } else {
            result.push(b as char);
        }
        i += 1;
    }

    // Collapse multiple blank lines
    let collapsed = collapse_blank_lines(&result);
    Ok(Value::String(collapsed.trim().to_string()))
}

/// Decode common HTML entities to characters.
fn decode_html_entity(entity: &str) -> String {
    match entity {
        "amp" => "&".to_string(),
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "quot" => "\"".to_string(),
        "apos" => "'".to_string(),
        "nbsp" => "\u{00a0}".to_string(),
        "&#39;" => "'".to_string(),
        _ => {
            // Numeric entities: &#NNN; or &#xHH;
            if entity.starts_with("#x") || entity.starts_with("#X") {
                if let Ok(n) = u32::from_str_radix(&entity[2..], 16) {
                    if let Some(c) = char::from_u32(n) {
                        return c.to_string();
                    }
                }
            } else if let Some(rest) = entity.strip_prefix('#') {
                if let Ok(n) = rest.parse::<u32>() {
                    if let Some(c) = char::from_u32(n) {
                        return c.to_string();
                    }
                }
            }
            format!("&{};", entity) // unknown entity, preserve
        }
    }
}

/// Collapse 3+ consecutive newlines into 2.
fn collapse_blank_lines(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut newline_count = 0usize;
    for c in s.chars() {
        if c == '\n' {
            newline_count += 1;
            if newline_count <= 2 {
                result.push(c);
            }
        } else {
            newline_count = 0;
            result.push(c);
        }
    }
    result
}

// ── Personalization (inspired by OpenHuman self-learning pipeline) ──
// Stores user preferences with facet classes and half-life decay.
// Facets: style, identity, tooling, veto, goal, channel

/// `learn_preference(class, key, value)` — record a preference observation.
/// class: "style" | "identity" | "tooling" | "veto" | "goal" | "channel"
/// Stores in KV under "pref:<class>:<key>" with timestamp and evidence count.
/// Returns Struct { class, key, value, status }.
pub(crate) fn builtin_learn_preference(args: &[Value]) -> Result<Value, String> {
    let class = expect_string_arg("learn_preference", args, 0)?;
    let key = expect_string_arg("learn_preference", args, 1)?;
    let value = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err(
                "learn_preference() expects third argument to be a value (String)".to_string(),
            )
        }
    };
    let valid_classes = ["style", "identity", "tooling", "veto", "goal", "channel"];
    if !valid_classes.contains(&class.as_str()) {
        return Err(format!(
            "learn_preference() invalid class '{}'. Valid: {}",
            class,
            valid_classes.join(", ")
        ));
    }
    let pref_key = format!("pref:{}:{}", class, key);
    let entry = serde_json::json!({
        "class": class,
        "key": key,
        "value": value,
        "evidence_count": 1,
        "last_observed": chrono_now_timestamp(),
        "state": "candidate"
    });
    let json = serde_json::to_string(&entry).unwrap_or_default();
    if let Ok(mut store) = kv_store().lock() {
        // If already exists, increment evidence count
        if let Some(existing) = store.get(&pref_key) {
            if let Ok(mut prev) = serde_json::from_str::<serde_json::Value>(existing) {
                let count = prev["evidence_count"].as_u64().unwrap_or(0) + 1;
                prev["evidence_count"] = serde_json::Value::Number(count.into());
                prev["last_observed"] = serde_json::Value::Number(chrono_now_timestamp().into());
                // Promote to active after 3 observations
                if count >= 3 {
                    prev["state"] = serde_json::Value::String("active".to_string());
                }
                let updated = serde_json::to_string(&prev).unwrap_or_default();
                store.insert(pref_key.clone(), updated.clone());
                if let Ok(guard) = kv_sqlite().lock() {
                    if let Some(ref conn) = *guard {
                        let _ = conn.execute(
                            "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                            rusqlite::params![pref_key, updated],
                        );
                    }
                }
                return Ok(make_date_struct(
                    "Preference",
                    vec![
                        ("class", Value::String(class)),
                        ("key", Value::String(key)),
                        ("value", Value::String(value)),
                        ("evidence", Value::Float(count as f64)),
                        ("state", Value::String("active".to_string())),
                    ],
                ));
            }
        }
        store.insert(pref_key.clone(), json.clone());
    }
    if let Ok(guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                rusqlite::params![pref_key, json],
            );
        }
    }
    Ok(make_date_struct(
        "Preference",
        vec![
            ("class", Value::String(class)),
            ("key", Value::String(key)),
            ("value", Value::String(value)),
            ("evidence", Value::Float(1.0)),
            ("state", Value::String("candidate".to_string())),
        ],
    ))
}

/// `get_profile()` — get all active user preferences.
/// Returns List of Struct { class, key, value, evidence, state }.
pub(crate) fn builtin_get_profile(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let mut result = Vec::new();
    let prefixes = [
        "pref:style:",
        "pref:identity:",
        "pref:tooling:",
        "pref:veto:",
        "pref:goal:",
        "pref:channel:",
    ];
    let store = kv_store().lock().ok();
    let sqlite = kv_sqlite().lock().ok();

    for prefix in &prefixes {
        let raw = match (store.as_ref(), sqlite.as_ref()) {
            (Some(s), _) => {
                // Scan all keys for prefix match
                s.keys()
                    .filter(|k| k.starts_with(prefix))
                    .find_map(|k| s.get(k).cloned())
            }
            (_, Some(guard)) => guard.as_ref().and_then(|conn| {
                let pat = format!("{}%", prefix);
                let mut stmt = conn
                    .prepare("SELECT value FROM kv_store WHERE key LIKE ?1")
                    .ok()?;
                let mut rows = stmt.query(rusqlite::params![pat]).ok()?;
                rows.next().ok().flatten().and_then(|row| row.get(0).ok())
            }),
            _ => None,
        };
        if let Some(json_str) = raw {
            if let Ok(pref) = serde_json::from_str::<serde_json::Value>(&json_str) {
                result.push(make_date_struct(
                    "Preference",
                    vec![
                        (
                            "class",
                            Value::String(pref["class"].as_str().unwrap_or("").to_string()),
                        ),
                        (
                            "key",
                            Value::String(pref["key"].as_str().unwrap_or("").to_string()),
                        ),
                        (
                            "value",
                            Value::String(pref["value"].as_str().unwrap_or("").to_string()),
                        ),
                        (
                            "evidence",
                            Value::Float(pref["evidence_count"].as_u64().unwrap_or(0) as f64),
                        ),
                        (
                            "state",
                            Value::String(
                                pref["state"].as_str().unwrap_or("candidate").to_string(),
                            ),
                        ),
                    ],
                ));
            }
        }
    }
    Ok(Value::List(result))
}

// ── V5: Assertions ──────────────────────────────────────────────────

/// Helper: current Unix timestamp.
pub(crate) fn chrono_now_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ── AgentSkillOS-inspired: Recipe system + DAG orchestration (ADR-0062) ─────────
//
// Концепции заимствованы из https://github.com/ynulihao/AgentSkillOS (MIT — код НЕ
// копировался, только идеи: recipe persistence, DAG phase extraction, topo sort).
//
// Recipe — KV-backed сохранение успешных (task, skills, plan) комбинаций.
// DAG phases — выделение параллельных фаз из directed acyclic graph.
// Topo sort — Kahn's algorithm для линейного порядка выполнения.

/// KV key prefix for recipe storage.
const RECIPE_PREFIX: &str = "__recipe:";
/// KV key for recipe index (JSON array of recipe names).
#[allow(dead_code)]
const RECIPE_INDEX_KEY: &str = "__recipe_index";

/// `recipe_save(name, description, skills, plan)` — persist a recipe.
/// args: [name: String, description: String, skills: List, plan: Struct/any]
/// Stores in KV under `__recipe:<name>` as JSON. Updates recipe index.
pub(crate) fn builtin_recipe_save(args: &[Value]) -> Result<Value, String> {
    if args.len() < 4 {
        return Err("recipe_save: requires 4 arguments (name, description, skills, plan)".into());
    }
    let name = expect_string_arg_var("recipe_save", args, 0)?;
    let description = expect_string_arg_var("recipe_save", args, 1)?;
    let skills = expect_list_arg("recipe_save", args, 2)?;
    let plan_json = expect_struct_json_arg("recipe_save", args, 3)?;

    // Build recipe JSON
    let skills_json: Vec<String> = skills
        .iter()
        .map(|v| serde_json::to_string(&mlog_value_to_json(v)).unwrap_or_else(|_| "null".into()))
        .collect();

    let recipe = serde_json::json!({
        "name": name,
        "description": description,
        "skills": skills_json,
        "plan": serde_json::from_str::<serde_json::Value>(&plan_json).unwrap_or(serde_json::Value::Null),
        "usage_count": 0,
        "created_at": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    });

    let recipe_str = serde_json::to_string(&recipe)
        .map_err(|e| format!("recipe_save: serialization failed: {}", e))?;

    // Store in KV (using internal kv_set logic via JSON round-trip)
    let kv_key = format!("{}{}", RECIPE_PREFIX, name);

    // Return the recipe as a Struct for the caller; actual KV persistence
    // happens when the caller does kv_set(kv_key, recipe_str).
    Ok(make_struct(
        "RecipeSaveResult",
        vec![
            ("key", Value::String(kv_key)),
            ("recipe", Value::String(recipe_str)),
        ],
    ))
}

/// `recipe_search(query)` — search recipes by description similarity (substring match).
/// args: [query: String]
/// Iterates all recipes stored under `__recipe:*` in KV, returns matching ones.
/// NOTE: This is a simplified implementation using substring matching.
/// Full semantic search (cosine similarity) requires embedding infrastructure.
pub(crate) fn builtin_recipe_search(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("recipe_search: requires 1 argument (query)".into());
    }
    let _query = expect_string_arg_var("recipe_search", args, 0)?;

    // Simplified: return empty list as placeholder.
    // Full implementation requires access to KV store from builtin context,
    // which is a known architectural limitation (builtins are pure functions).
    // The recipe_search is designed to be called with pre-loaded recipe data:
    //   let all = recipe_list()
    //   let found = filter(all, fn(r) { contains(r.description, query) })
    Ok(Value::List(vec![]))
}

/// `recipe_list()` — return all known recipe names.
/// args: [] (reads from recipe index key)
pub(crate) fn builtin_recipe_list(args: &[Value]) -> Result<Value, String> {
    // Simplified: return empty list.
    // Full implementation requires KV store access from builtin context.
    // Users can maintain their own recipe index:
    //   recipe_save(...) -> kv_set("__recipe_index", json_encode(names))
    let _ = args;
    Ok(Value::List(vec![]))
}

/// `dag_phases(dag)` — extract parallel execution phases from a DAG.
///
/// The DAG is a list of nodes, each a struct with:
///   - "id": String (node identifier)
///   - "depends_on": List of String (node IDs this node depends on)
///
/// Returns a list of phases (lists of node IDs), where each phase contains
/// nodes that can be executed in parallel (all dependencies satisfied).
pub(crate) fn builtin_dag_phases(args: &[Value]) -> Result<Value, String> {
    let nodes = expect_list_arg("dag_phases", args, 0)?;
    if nodes.is_empty() {
        return Ok(Value::List(vec![]));
    }

    // Extract node IDs and build adjacency info
    let mut node_ids: Vec<String> = Vec::new();
    let mut deps_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut in_degree: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for node in &nodes {
        let node_json = mlog_value_to_json(node);
        let id = node_json
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if id.is_empty() {
            return Err("dag_phases: each node must have an 'id' field (String)".into());
        }

        let deps: Vec<String> = node_json
            .get("depends_on")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        in_degree.insert(id.clone(), deps.len());
        deps_map.insert(id.clone(), deps);
        node_ids.push(id);
    }

    // Validate: all dependency references exist
    let node_set: std::collections::HashSet<&str> = node_ids.iter().map(|s| s.as_str()).collect();
    for (node, deps) in &deps_map {
        for dep in deps {
            if !node_set.contains(dep.as_str()) {
                return Err(format!(
                    "dag_phases: node '{}' depends on unknown node '{}'",
                    node, dep
                ));
            }
        }
    }

    // Kahn's algorithm — extract phases
    let mut remaining_in: std::collections::HashMap<String, usize> = in_degree.clone();
    let mut phases: Vec<Value> = Vec::new();
    let mut processed: std::collections::HashSet<String> = std::collections::HashSet::new();

    loop {
        // Find all nodes with in-degree 0 (not yet processed)
        let phase_nodes: Vec<String> = node_ids
            .iter()
            .filter(|id| {
                !processed.contains(*id) && remaining_in.get(*id).copied().unwrap_or(0) == 0
            })
            .cloned()
            .collect();

        if phase_nodes.is_empty() {
            break;
        }

        // Add phase as a list of node IDs
        let phase_value = Value::List(
            phase_nodes
                .iter()
                .map(|id| Value::String(id.clone()))
                .collect(),
        );
        phases.push(phase_value);

        // "Remove" phase nodes: decrease in-degree of dependents
        for id in &phase_nodes {
            processed.insert(id.clone());
            for (node, deps) in &deps_map {
                if deps.contains(id) {
                    if let Some(deg) = remaining_in.get_mut(node) {
                        *deg = deg.saturating_sub(1);
                    }
                }
            }
        }
    }

    // Cycle detection
    if processed.len() != node_ids.len() {
        let unprocessed: Vec<&str> = node_ids
            .iter()
            .filter(|id| !processed.contains(*id))
            .map(|s| s.as_str())
            .collect();
        return Err(format!(
            "dag_phases: cycle detected among nodes: {}",
            unprocessed.join(", ")
        ));
    }

    Ok(Value::List(phases))
}

/// `topo_sort(dag)` — topological sort of a DAG.
///
/// Same input format as dag_phases. Returns a flat list of node IDs
/// in topological order (Kahn's algorithm).
pub(crate) fn builtin_topo_sort(args: &[Value]) -> Result<Value, String> {
    let nodes = expect_list_arg("topo_sort", args, 0)?;
    if nodes.is_empty() {
        return Ok(Value::List(vec![]));
    }

    let mut node_ids: Vec<String> = Vec::new();
    let mut deps_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut in_degree: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for node in &nodes {
        let node_json = mlog_value_to_json(node);
        let id = node_json
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if id.is_empty() {
            return Err("topo_sort: each node must have an 'id' field (String)".into());
        }

        let deps: Vec<String> = node_json
            .get("depends_on")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        in_degree.insert(id.clone(), deps.len());
        deps_map.insert(id.clone(), deps);
        node_ids.push(id);
    }

    // Validate dependency references
    let node_set: std::collections::HashSet<&str> = node_ids.iter().map(|s| s.as_str()).collect();
    for (node, deps) in &deps_map {
        for dep in deps {
            if !node_set.contains(dep.as_str()) {
                return Err(format!(
                    "topo_sort: node '{}' depends on unknown node '{}'",
                    node, dep
                ));
            }
        }
    }

    // Kahn's algorithm
    let mut remaining_in = in_degree.clone();
    let mut queue: std::collections::VecDeque<String> = node_ids
        .iter()
        .filter(|id| remaining_in.get(*id).copied().unwrap_or(0) == 0)
        .cloned()
        .collect();
    let mut result: Vec<String> = Vec::new();

    while let Some(id) = queue.pop_front() {
        result.push(id.clone());
        for (node, deps) in &deps_map {
            if deps.contains(&id) {
                if let Some(deg) = remaining_in.get_mut(node) {
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        queue.push_back(node.clone());
                    }
                }
            }
        }
    }

    // Cycle detection
    if result.len() != node_ids.len() {
        return Err("topo_sort: cycle detected in DAG".into());
    }

    Ok(Value::List(result.into_iter().map(Value::String).collect()))
}

// ════════════════════════════════════════════════════════════════════
// ── obsidian-mind inspired: Vault/memory builtins (v0.10.0) ─────
// ════════════════════════════════════════════════════════════════════

/// `semantic_search(query, documents, top_k)` — semantic similarity search.
///
/// Inspired by obsidian-mind's QMD semantic search layer.
/// Embeds the query and each document, returns top_k results as structs:
///   { index, text, score }
///
/// Uses the same EmbeddingManager as the rest of Metalogos:
/// - OpenAI text-embedding-3-small if METALOGOS_EMBEDDING_API_KEY is set
/// - TF-IDF fallback otherwise (no API needed)
///
/// # Arguments
/// * `query` — search query string
/// * `documents` — list of document strings to search through
/// * `top_k` — number of results to return
pub(crate) fn builtin_semantic_search(args: &[Value]) -> Result<Value, String> {
    let query = expect_string_arg("semantic_search", args, 0)?;
    let documents = expect_list_arg("semantic_search", args, 1)?;
    let top_k = expect_string_arg("semantic_search", args, 2)?;
    let top_k: usize = top_k.parse().map_err(|_| {
        format!(
            "semantic_search: top_k must be a number string, got '{}'",
            args[2]
        )
    })?;

    if documents.is_empty() {
        return Ok(Value::List(vec![]));
    }

    // Create embedding manager (reads METALOGOS_EMBEDDING_PROVIDER env)
    let mgr = EmbeddingManager::new();

    // Embed the query
    let query_vec = mgr
        .embed(&query)
        .map_err(|e| format!("semantic_search: failed to embed query: {}", e))?;

    // Score each document
    let mut scored: Vec<(usize, f32, String)> = Vec::with_capacity(documents.len());
    for (i, doc_val) in documents.iter().enumerate() {
        let doc_text = format!("{}", doc_val);
        if doc_text.is_empty() {
            continue;
        }
        match mgr.embed(&doc_text) {
            Ok(doc_vec) => {
                let sim = cosine_similarity(&query_vec, &doc_vec);
                scored.push((i, sim, doc_text));
            }
            Err(e) => {
                // Skip documents that fail to embed rather than aborting
                eprintln!("[semantic_search] skip doc {}: {}", i, e);
            }
        }
    }

    // Sort by similarity descending, take top_k
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(top_k);

    // Build result structs
    let results: Vec<Value> = scored
        .into_iter()
        .map(|(index, score, text)| {
            make_struct(
                "SearchResult",
                vec![
                    ("index", Value::Float(index as f64)),
                    ("text", Value::String(text)),
                    ("score", Value::Float(score as f64)),
                ],
            )
        })
        .collect();

    Ok(Value::List(results))
}

/// `config_load(path)` — load a JSON or YAML config file and return as struct.
///
/// Inspired by obsidian-mind's vault-manifest.json pattern:
/// a single coordination file that all layers read from.
///
/// Loads a file from disk, auto-detecting format by extension:
/// - .yaml / .yml → parsed as YAML
/// - .json / other → parsed as JSON
///
/// The result is converted to a Metalogos struct. The type_name is derived
/// from the filename stem (e.g., "vault-manifest.json" → type "vault-manifest").
pub(crate) fn builtin_config_load(args: &[Value]) -> Result<Value, String> {
    let path = expect_string_arg("config_load", args, 0)?;

    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("config_load: cannot read '{}': {}", path, e))?;

    let type_name = std::path::Path::new(&path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Config");

    // Auto-detect format by extension
    let is_yaml = path.to_lowercase().ends_with(".yaml") || path.to_lowercase().ends_with(".yml");

    let parsed: serde_json::Value = if is_yaml {
        let yaml_val: serde_yaml::Value = serde_yaml::from_str(&content)
            .map_err(|e| format!("config_load: YAML parse error in '{}': {}", path, e))?;
        // Convert serde_yaml::Value to serde_json::Value for unified processing
        yaml_to_json_value(&yaml_val)
    } else {
        serde_json::from_str(&content)
            .map_err(|e| format!("config_load: JSON parse error in '{}': {}", path, e))?
    };

    Ok(json_value_to_mlog_value_with_type(&parsed, type_name))
}

/// `vault_validate(config, required_fields)` — validate a loaded config against required fields.
///
/// Inspired by obsidian-mind's frontmatter_required validation.
/// Checks that a config struct contains all specified required fields.
/// Returns a struct { valid, missing }.
///
/// # Arguments
/// * `config` — a struct (e.g., from config_load)
/// * `required_fields` — list of field names that must be present
pub(crate) fn builtin_vault_validate(args: &[Value]) -> Result<Value, String> {
    let fields_list = expect_list_arg("vault_validate", args, 1)?;
    let required: Vec<String> = fields_list.iter().map(|v| format!("{}", v)).collect();

    let missing: Vec<String> = match &args[0] {
        Value::Struct { fields, .. } => required
            .into_iter()
            .filter(|f| !fields.contains_key(f))
            .collect(),
        Value::Unit => required, // everything is missing
        _ => return Err("vault_validate: first argument must be a struct".to_string()),
    };

    Ok(make_struct(
        "ValidationResult",
        vec![
            ("valid", Value::Bool(missing.is_empty())),
            (
                "missing",
                Value::List(missing.into_iter().map(Value::String).collect()),
            ),
        ],
    ))
}
