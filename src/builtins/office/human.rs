// ── Human persona builtins ─────────────────────────────────────────
// human_respond, human_personas, human_delete, learn_preference

use super::super::core::*;
use super::super::http::*;
use super::super::memory::*;
use super::super::server::builtin_human_recall;
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

    // Call LLM — Наряд #156: route through GLOBAL_SMART_ROUTER when available
    // (same as call_llm builtin), falling back to legacy backend.
    // No sandbox timeout for human_respond — pass None.
    let response = if let Some(result) =
        crate::llm::call_via_smart_router(&full_prompt, "", None, None)
    {
        result.map_err(|e| format!("human_respond() LLM call failed: {}", e))?
    } else {
        // No SmartRouter — check mock mode, then legacy backend
        let mock_mode = std::env::var("METALOGOS_LLM_MOCK")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(true);

        if mock_mode {
            format!("[{} (mood: {}): {}]", persona, mood, message)
        } else {
            let backend = crate::llm::create_llm_backend();
            backend
                .call(&full_prompt, "")
                .map_err(|e| format!("human_respond() LLM call failed: {}", e))?
        }
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
        "last_observed": super::super::office::config::chrono_now_timestamp(),
        "state": "candidate"
    });
    let json = serde_json::to_string(&entry).unwrap_or_default();
    if let Ok(mut store) = kv_store().lock() {
        // If already exists, increment evidence count
        if let Some(existing) = store.get(&pref_key) {
            if let Ok(mut prev) = serde_json::from_str::<serde_json::Value>(existing) {
                let count = prev["evidence_count"].as_u64().unwrap_or(0) + 1;
                prev["evidence_count"] = serde_json::Value::Number(count.into());
                prev["last_observed"] = serde_json::Value::Number(
                    super::super::office::config::chrono_now_timestamp().into(),
                );
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
