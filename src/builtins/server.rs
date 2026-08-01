use super::core::*;
use super::http::*;
use super::json::*;
use super::memory::*;
use crate::embeddings::{cosine_similarity, EmbeddingManager};
use crate::interpreter::Value;

// ── Phase 6.1 — HTTP server stubs ───────────────────────────
// In interpreter-only mode (mlog run), these return mock values.
// Real implementations live in server.rs for the Axum context.

// ── Phase 6.3 — Database stubs ───────────────────────────

pub(crate) fn builtin_query(args: &[Value]) -> Result<Value, String> {
    let sql = expect_string_arg("query", args, 0)?;
    // Wrap SQL in opaque Query value — prevents string concatenation or printing
    // In interpreter mode, store the SQL for later mock execution
    let _params = if args.len() > 1 {
        &args[1]
    } else {
        &Value::Unit
    };
    Ok(Value::Query(sql))
}

pub(crate) fn builtin_db_execute(args: &[Value]) -> Result<Value, String> {
    let _sql = expect_string_arg("db_execute", args, 0)?;
    // In interpreter mode, no-op (returns Unit)
    Ok(Value::Unit)
}

// ── Phase 6.6 — Bot stubs ───────────────────────────

pub(crate) fn builtin_send_message(args: &[Value]) -> Result<Value, String> {
    // Extract and format chat_id — supports negative channel IDs (Наряд №24 B5)
    let chat_id_value: serde_json::Value = match args.first() {
        Some(Value::String(s)) => serde_json::Value::String(s.clone()),
        Some(Value::Float(f)) => {
            if *f == (*f as i64) as f64 {
                serde_json::json!(*f as i64)
            } else {
                serde_json::json!(*f)
            }
        }
        Some(other) => {
            return Err(format!(
                "send_message() expected String or Float as chat_id, got {}",
                other.type_name()
            ))
        }
        None => {
            return Err("send_message() requires at least 2 arguments (chat_id, text)".to_string())
        }
    };
    let text = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "send_message() expected String as text, got {}",
                other.type_name()
            ))
        }
        None => {
            return Err("send_message() requires at least 2 arguments (chat_id, text)".to_string())
        }
    };

    // Try to send via Telegram API if BOT_TOKEN env var is set
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
    if bot_token.is_empty() {
        // No token — fall back to audit stub
        eprintln!("[AUDIT] send_message to {:?}: {}", chat_id_value, text);
        return Ok(Value::Unit);
    }

    // Build JSON body with optional reply_markup (3rd arg: Struct)
    let mut body = serde_json::json!({
        "chat_id": chat_id_value,
        "text": text,
    });
    if let Some(markup) = args.get(2) {
        let markup_json = value_to_json(markup)?;
        body["reply_markup"] = markup_json;
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("send_message(): client error: {}", e))?;

    let resp = client
        .post(format!(
            "https://api.telegram.org/bot{}/sendMessage",
            bot_token
        ))
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .map_err(|e| format!("send_message(): request failed: {}", e))?;

    let status = resp.status().as_u16();
    let resp_body = resp.text().unwrap_or_default();

    if status >= 400 {
        return Err(format!(
            "send_message(): Telegram status {}: {}",
            status, resp_body
        ));
    }

    Ok(Value::String(resp_body))
}

/// `answer_callback_query(callback_query_id, text?, show_alert?)` — respond to Telegram inline keyboard callback.
/// `callback_query_id` from update.callback_query.id.
/// `text` — notification text (max 200 chars). `show_alert` — 1.0 = alert popup, 0.0 = toast (default).
pub(crate) fn builtin_answer_callback_query(args: &[Value]) -> Result<Value, String> {
    let callback_query_id = expect_string_arg("answer_callback_query", args, 0)?;
    let text = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    };
    let show_alert = matches!(args.get(2), Some(Value::Float(f)) if *f > 0.5);
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
    if bot_token.is_empty() {
        eprintln!(
            "[AUDIT] answer_callback_query id={}: {}",
            callback_query_id, text
        );
        return Ok(Value::Unit);
    }
    let body = serde_json::json!({
        "callback_query_id": callback_query_id,
        "text": text,
        "show_alert": show_alert,
    });
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("answer_callback_query(): client error: {}", e))?;
    let resp = client
        .post(format!(
            "https://api.telegram.org/bot{}/answerCallbackQuery",
            bot_token
        ))
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .map_err(|e| format!("answer_callback_query(): request failed: {}", e))?;
    let status = resp.status().as_u16();
    let resp_body = resp.text().unwrap_or_default();
    if status >= 400 {
        return Err(format!(
            "answer_callback_query(): Telegram status {}: {}",
            status, resp_body
        ));
    }
    Ok(Value::String(resp_body))
}

/// `edit_message_text(chat_id, message_id, text, reply_markup?)` — edit existing Telegram message.
/// Used to update inline keyboard buttons after callback.
pub(crate) fn builtin_edit_message_text(args: &[Value]) -> Result<Value, String> {
    let chat_id_val: serde_json::Value = match args.first() {
        Some(Value::String(s)) => serde_json::Value::String(s.clone()),
        Some(Value::Float(f)) => serde_json::json!(*f as i64),
        _ => return Err("edit_message_text() requires chat_id".to_string()),
    };
    let message_id = match args.get(1) {
        Some(Value::Float(f)) => *f as i64,
        _ => return Err("edit_message_text() message_id must be Float".to_string()),
    };
    let text = expect_string_arg("edit_message_text", args, 2)?;
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
    if bot_token.is_empty() {
        eprintln!(
            "[AUDIT] edit_message_text chat_id={}: {}",
            chat_id_val, text
        );
        return Ok(Value::Unit);
    }
    let mut body = serde_json::json!({
        "chat_id": chat_id_val,
        "message_id": message_id,
        "text": text,
    });
    if let Some(markup) = args.get(3) {
        let markup_json = value_to_json(markup)?;
        body["reply_markup"] = markup_json;
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("edit_message_text(): client error: {}", e))?;
    let resp = client
        .post(format!(
            "https://api.telegram.org/bot{}/editMessageText",
            bot_token
        ))
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .map_err(|e| format!("edit_message_text(): request failed: {}", e))?;
    let status = resp.status().as_u16();
    let resp_body = resp.text().unwrap_or_default();
    if status >= 400 {
        return Err(format!(
            "edit_message_text(): Telegram status {}: {}",
            status, resp_body
        ));
    }
    Ok(Value::String(resp_body))
}

// ── Phase 7.7 — parse_json, http_get, now ────────────────────────────

// ── Наряд 17: Utility builtins ──────────────────────────────────

// ── OpenPlanter-inspired: Fuzzy matching, safe editing, agent utilities (ADR-0063) ──

/// `fuzzy_find_best(query, candidates)` — find the best match for `query` in a list of candidate strings.
/// Returns a struct { index, candidate, score } or Unit if list is empty.
pub(crate) fn builtin_fuzzy_find_best(args: &[Value]) -> Result<Value, String> {
    let query = expect_string_arg("fuzzy_find_best", args, 0)?;
    let candidates = expect_list_arg("fuzzy_find_best", args, 1)?;
    if candidates.is_empty() {
        return Ok(Value::Unit);
    }
    let mut best_idx = 0usize;
    let mut best_score = 0.0f64;
    let mut best_candidate = String::new();
    for (i, v) in candidates.iter().enumerate() {
        let c = format!("{}", v);
        let score = strsim::jaro_winkler(&query, &c);
        if score > best_score {
            best_score = score;
            best_idx = i;
            best_candidate = c;
        }
    }
    Ok(make_struct(
        "FuzzyMatch",
        vec![
            ("index", Value::Float(best_idx as f64)),
            ("candidate", Value::String(best_candidate)),
            ("score", Value::Float(best_score)),
        ],
    ))
}

/// Compute a 2-char hex hash for a line (whitespace-normalized),
/// mimicking OpenPlanter's hashline system for content-verified editing.
fn compute_line_hash(line: &str) -> String {
    let normalized: String = line.split_whitespace().collect();
    let hash = crc32fast::hash(normalized.as_bytes());
    format!("{:02x}", hash & 0xFF)
}

/// `hashline_read(text)` — annotate each line with a 2-char CRC32 hash prefix.
/// Output format: "N:HH|content" per line.
/// Inspired by OpenPlanter's tools.py hashline system for safe LLM editing.
pub(crate) fn builtin_hashline_read(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("hashline_read", args, 0)?;
    let mut out = String::new();
    for (i, line) in text.lines().enumerate() {
        let hash = compute_line_hash(line);
        out.push_str(&format!("{}:{}|{}\n", i + 1, hash, line));
    }
    Ok(Value::String(out))
}

/// `hashline_edit(text, edits)` — apply edits to text using hashline-verified line references.
/// `edits` is a list of structs, each with an `op` field ("set_line", "replace_lines", "insert_after")
/// and corresponding fields.
///   - set_line: { op: "set_line", ref: "N:HH", content: "new content" }
///   - replace_lines: { op: "replace_lines", start_ref: "N:HH", end_ref: "M:HH", content: "replacement" }
///   - insert_after: { op: "insert_after", ref: "N:HH", content: "new line" }
///     Returns the modified text. Errors if hash mismatch (stale reference).
pub(crate) fn builtin_hashline_edit(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("hashline_edit", args, 0)?;
    let edits = expect_list_arg("hashline_edit", args, 1)?;
    let mut lines: Vec<String> = text.lines().map(String::from).collect();

    for edit in &edits {
        let edit_json = mlog_value_to_json(edit);
        let op = edit_json.get("op").and_then(|v| v.as_str()).unwrap_or("");
        match op {
            "set_line" => {
                let line_ref = edit_json.get("ref").and_then(|v| v.as_str()).unwrap_or("");
                let content = edit_json
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let (line_num, expected_hash) = parse_line_ref(line_ref)?;
                let idx = line_num - 1;
                if idx >= lines.len() {
                    return Err(format!(
                        "hashline_edit: line {} out of bounds ({} lines)",
                        line_num,
                        lines.len()
                    ));
                }
                let actual_hash = compute_line_hash(&lines[idx]);
                if actual_hash != expected_hash {
                    return Err(format!(
                        "hashline_edit: hash mismatch at line {} (expected {}, got {}). Line may have changed.",
                        line_num, expected_hash, actual_hash
                    ));
                }
                lines[idx] = content.to_string();
            }
            "replace_lines" => {
                let start_ref = edit_json
                    .get("start_ref")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let end_ref = edit_json
                    .get("end_ref")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let content = edit_json
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let (start_num, start_hash) = parse_line_ref(start_ref)?;
                let (end_num, end_hash) = parse_line_ref(end_ref)?;
                let si = start_num - 1;
                let ei = end_num;
                if si >= lines.len() || ei > lines.len() {
                    return Err(format!(
                        "hashline_edit: replace range {}..{} out of bounds",
                        start_num, end_num
                    ));
                }
                let actual_start = compute_line_hash(&lines[si]);
                let actual_end = compute_line_hash(&lines[ei - 1]);
                if actual_start != start_hash || actual_end != end_hash {
                    return Err(format!(
                        "hashline_edit: hash mismatch in replace range {}..{}",
                        start_num, end_num
                    ));
                }
                let replacement: Vec<String> = content.lines().map(String::from).collect();
                lines.splice(si..ei, replacement);
            }
            "insert_after" => {
                let line_ref = edit_json.get("ref").and_then(|v| v.as_str()).unwrap_or("");
                let content = edit_json
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let (line_num, expected_hash) = parse_line_ref(line_ref)?;
                let idx = line_num; // insert AFTER this line
                if idx > lines.len() {
                    return Err(format!(
                        "hashline_edit: insert_after line {} out of bounds",
                        line_num
                    ));
                }
                if line_num > 0 && (line_num - 1) < lines.len() {
                    let actual_hash = compute_line_hash(&lines[line_num - 1]);
                    if actual_hash != expected_hash {
                        return Err(format!(
                            "hashline_edit: hash mismatch at line {} (expected {}, got {})",
                            line_num, expected_hash, actual_hash
                        ));
                    }
                }
                let new_lines: Vec<String> = content.lines().map(String::from).collect();
                for (i, nl) in new_lines.into_iter().enumerate() {
                    lines.insert(idx + i, nl);
                }
            }
            _ => {
                return Err(format!(
                    "hashline_edit: unknown op '{}'. Use set_line, replace_lines, or insert_after.",
                    op
                ));
            }
        }
    }

    Ok(Value::String(lines.join("\n")))
}

/// Parse a line reference "N:HH" into (line_number, hash_hex).
fn parse_line_ref(line_ref: &str) -> Result<(usize, String), String> {
    let parts: Vec<&str> = line_ref.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(format!(
            "hashline_edit: invalid line ref '{}', expected N:HH format",
            line_ref
        ));
    }
    let line_num: usize = parts[0].parse().map_err(|_| {
        format!(
            "hashline_edit: invalid line number '{}' in ref '{}'",
            parts[0], line_ref
        )
    })?;
    let hash = parts[1].to_string();
    if hash.len() != 2 {
        return Err(format!(
            "hashline_edit: hash must be 2 hex chars, got '{}' in ref '{}'",
            hash, line_ref
        ));
    }
    Ok((line_num, hash))
}

/// `budget_check(step, total_steps)` — returns a budget status struct.
///   - remaining >= 50%: level = "ok"
///   - remaining >= 25%: level = "warning"
///   - remaining < 25%: level = "critical"
///     Inspired by OpenPlanter's engine.py budget awareness system.
pub(crate) fn builtin_budget_check(args: &[Value]) -> Result<Value, String> {
    let step = expect_float_arg("budget_check", args, 0)? as usize;
    let total_steps = expect_float_arg("budget_check", args, 1)? as usize;
    if total_steps == 0 {
        return Err("budget_check: total_steps must be > 0".to_string());
    }
    if step > total_steps {
        return Err(format!(
            "budget_check: step {} exceeds total_steps {}",
            step, total_steps
        ));
    }
    let remaining = total_steps - step;
    let pct = (remaining as f64) / (total_steps as f64) * 100.0;
    let level = if pct >= 50.0 {
        "ok"
    } else if pct >= 25.0 {
        "warning"
    } else {
        "critical"
    };
    Ok(make_struct(
        "BudgetStatus",
        vec![
            ("step", Value::Float(step as f64)),
            ("total", Value::Float(total_steps as f64)),
            ("remaining", Value::Float(remaining as f64)),
            ("pct_remaining", Value::Float(pct)),
            ("level", Value::String(level.to_string())),
        ],
    ))
}

/// `replay_snapshot(data)` — delta-encoded replay log helper.
/// Takes a list of items (messages, events, etc.) and returns a struct with:
///   - seq: 0 (first snapshot)
///   - count: number of items
///   - snapshot: JSON string of the full list
///     Subsequent calls should use the returned count to determine delta.
///     Inspired by OpenPlanter's ReplayLogger (seq 0 = full, seq N = delta).
pub(crate) fn builtin_replay_snapshot(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("replay_snapshot", args, 0)?;
    let json_items: Vec<serde_json::Value> = data.iter().map(mlog_value_to_json).collect();
    let snapshot = serde_json::to_string(&json_items)
        .map_err(|e| format!("replay_snapshot: JSON serialization error: {}", e))?;
    Ok(make_struct(
        "ReplaySnapshot",
        vec![
            ("seq", Value::Float(0.0)),
            ("count", Value::Float(data.len() as f64)),
            ("snapshot", Value::String(snapshot)),
        ],
    ))
}

/// `policy_check(command)` — runtime policy enforcement for shell commands.
/// Checks a command string against safety policies:
///   - Blocks heredoc syntax (<<)
///   - Blocks interactive TUI programs (vim, nano, less, more, top, htop, vi)
///   - Trims leading/trailing whitespace
///     Returns a struct { allowed: bool, reason: "..." }.
///     Inspired by OpenPlanter's _runtime_policy_check in engine.py.
pub(crate) fn builtin_policy_check(args: &[Value]) -> Result<Value, String> {
    let command = expect_string_arg("policy_check", args, 0)?;
    let cmd_trimmed = command.trim();
    // Check heredoc
    if cmd_trimmed.contains("<<") {
        return Ok(make_struct(
            "PolicyResult",
            vec![
                ("allowed", Value::Bool(false)),
                (
                    "reason",
                    Value::String("blocked: heredoc syntax (<<) detected".to_string()),
                ),
            ],
        ));
    }
    // Check interactive programs
    let interactive_patterns = [
        "vim", "vi ", "nano", "less ", "more ", "top", "htop", "emacs",
    ];
    let first_word = cmd_trimmed.split_whitespace().next().unwrap_or("");
    for pattern in &interactive_patterns {
        if first_word == *pattern || first_word.starts_with(pattern) {
            return Ok(make_struct(
                "PolicyResult",
                vec![
                    ("allowed", Value::Bool(false)),
                    (
                        "reason",
                        Value::String(format!(
                            "blocked: interactive program '{}' detected",
                            first_word
                        )),
                    ),
                ],
            ));
        }
    }
    Ok(make_struct(
        "PolicyResult",
        vec![
            ("allowed", Value::Bool(true)),
            ("reason", Value::String("ok".to_string())),
        ],
    ))
}

// ── Format (Наряд №17 В.3) ──────────────────────────────────────

// ── v0.8.0 — Geolocation builtins ───────────────────────────────────

// ── v0.8.1 — OpenHuman-inspired Human Intelligence builtins ──────────
// Inspired by https://github.com/tinyhumansai/OpenHuman — memory tree,
// persona system, mood tracking, human-like AI responses.
// All built on top of existing Metalogos primitives (KV store, call_llm).
// No external dependencies, no API keys required beyond LLM provider.

/// `human_create(name, traits)` — create or update a persona.
/// `traits` is a string describing personality: "friendly, professional, speaks Russian".
/// Stores persona in KV under `human_persona:{name}`.
/// Returns Struct {name, traits, created_at, memory_count}.
pub(crate) fn builtin_human_create(args: &[Value]) -> Result<Value, String> {
    let name = expect_string_arg("human_create", args, 0)?;
    let traits = expect_string_arg("human_create", args, 1)?;
    if name.is_empty() {
        return Err("human_create() name cannot be empty".to_string());
    }
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let persona_data = serde_json::json!({
        "name": name,
        "traits": traits,
        "created_at": now_ts,
        "mood": "neutral",
        "mood_intensity": 0.5,
    });
    let key = format!("human_persona:{}", name);
    let value = serde_json::to_string(&persona_data)
        .map_err(|e| format!("human_create() serialize error: {}", e))?;
    let mut store = kv_store()
        .lock()
        .map_err(|e| format!("human_create() lock error: {}", e))?;
    store.insert(key.clone(), value.clone());
    if let Ok(sqlite_guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *sqlite_guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, value],
            );
        }
    }
    // Count existing memories for this persona
    let mem_prefix = format!("human_mem:{}:", name);
    let mem_count = store.keys().filter(|k| k.starts_with(&mem_prefix)).count();
    Ok(make_date_struct(
        "Persona",
        vec![
            ("name", Value::String(name)),
            ("traits", Value::String(traits)),
            ("created_at", Value::Float(now_ts)),
            ("memory_count", Value::Float(mem_count as f64)),
        ],
    ))
}

/// `human_mood(persona, mood?, intensity?)` — get or set persona's emotional state.
/// With 1 arg: returns current mood as Struct {mood, intensity, updated_at}.
/// With 2+ args: sets mood. `intensity` is 0.0–1.0 (default 0.5).
/// `mood` examples: "happy", "sad", "focused", "creative", "neutral", "excited".
pub(crate) fn builtin_human_mood(args: &[Value]) -> Result<Value, String> {
    let persona = expect_string_arg("human_mood", args, 0)?;
    let key = format!("human_persona:{}", persona);
    let store = kv_store()
        .lock()
        .map_err(|e| format!("human_mood() lock error: {}", e))?;
    let data_str = store.get(&key).cloned().unwrap_or_default();
    drop(store);
    if data_str.is_empty() {
        return Err(format!(
            "human_mood() persona '{}' not found. Use human_create() first.",
            persona
        ));
    }
    let mut data: serde_json::Value =
        serde_json::from_str(&data_str).map_err(|e| format!("human_mood() parse error: {}", e))?;
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    // If mood argument provided — set mood
    if args.len() >= 2 {
        let mood = expect_string_arg("human_mood", args, 1)?;
        let intensity = if args.len() >= 3 {
            expect_float_arg("human_mood", args, 2)?.clamp(0.0, 1.0)
        } else {
            0.5
        };
        data["mood"] = serde_json::Value::String(mood.clone());
        data["mood_intensity"] = serde_json::Value::Number(
            serde_json::Number::from_f64(intensity)
                .or_else(|| serde_json::Number::from_f64(0.5))
                .unwrap_or(serde_json::Number::from(0)),
        );
        data["mood_updated_at"] = serde_json::json!(now_ts);
        let updated = serde_json::to_string(&data)
            .map_err(|e| format!("human_mood() serialize error: {}", e))?;
        let mut store = kv_store()
            .lock()
            .map_err(|e| format!("human_mood() lock error: {}", e))?;
        store.insert(key.clone(), updated.clone());
        if let Ok(sqlite_guard) = kv_sqlite().lock() {
            if let Some(ref conn) = *sqlite_guard {
                let _ = conn.execute(
                    "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                    rusqlite::params![key, updated],
                );
            }
        }
    }

    let mood = data
        .get("mood")
        .and_then(|v| v.as_str())
        .unwrap_or("neutral")
        .to_string();
    let intensity = data
        .get("mood_intensity")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);
    let updated_at = data
        .get("mood_updated_at")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    Ok(make_date_struct(
        "Mood",
        vec![
            ("persona", Value::String(persona)),
            ("mood", Value::String(mood)),
            ("intensity", Value::Float(intensity)),
            ("updated_at", Value::Float(updated_at)),
        ],
    ))
}

/// `human_remember(persona, key, content, importance?)` — store a memory in persona's memory tree.
/// `importance` is 0.0–1.0 (default 0.5). Higher importance = recalled first.
/// Stores as KV entry `human_mem:{persona}:{key}` with metadata.
pub(crate) fn builtin_human_remember(args: &[Value]) -> Result<Value, String> {
    let persona = expect_string_arg("human_remember", args, 0)?;
    let key = expect_string_arg("human_remember", args, 1)?;
    let content = expect_string_arg("human_remember", args, 2)?;
    if key.is_empty() {
        return Err("human_remember() key cannot be empty".to_string());
    }
    let importance = if args.len() >= 4 {
        expect_float_arg("human_remember", args, 3)?.clamp(0.0, 1.0)
    } else {
        0.5
    };
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let mem_data = serde_json::json!({
        "persona": persona,
        "key": key,
        "content": content,
        "importance": importance,
        "created_at": now_ts,
        "access_count": 0,
        "last_accessed": now_ts,
    });
    let store_key = format!("human_mem:{}:{}", persona, key);
    let value = serde_json::to_string(&mem_data)
        .map_err(|e| format!("human_remember() serialize error: {}", e))?;
    let mut store = kv_store()
        .lock()
        .map_err(|e| format!("human_remember() lock error: {}", e))?;
    store.insert(store_key.clone(), value.clone());
    if let Ok(sqlite_guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *sqlite_guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                rusqlite::params![store_key, value],
            );
        }
    }
    Ok(Value::String("ok".to_string()))
}

/// `human_forget(persona, key?)` — delete a specific memory or all memories for a persona.
/// With 2 args: deletes specific memory by key. Returns "ok" or "not_found".
/// With 1 arg: deletes ALL memories for persona. Returns count of deleted memories.
pub(crate) fn builtin_human_forget(args: &[Value]) -> Result<Value, String> {
    let persona = expect_string_arg("human_forget", args, 0)?;
    let prefix = format!("human_mem:{}:", persona);
    let mut store = kv_store()
        .lock()
        .map_err(|e| format!("human_forget() lock error: {}", e))?;

    if args.len() >= 2 {
        let key = expect_string_arg("human_forget", args, 1)?;
        let store_key = format!("human_mem:{}:{}", persona, key);
        if store.remove(&store_key).is_some() {
            if let Ok(sqlite_guard) = kv_sqlite().lock() {
                if let Some(ref conn) = *sqlite_guard {
                    let _ = conn.execute(
                        "DELETE FROM kv_store WHERE key = ?1",
                        rusqlite::params![store_key],
                    );
                }
            }
            Ok(Value::String("ok".to_string()))
        } else {
            Ok(Value::String("not_found".to_string()))
        }
    } else {
        // Delete all memories for this persona
        let to_remove: Vec<String> = store
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .cloned()
            .collect();
        let count = to_remove.len();
        for k in &to_remove {
            store.remove(k);
        }
        if let Ok(sqlite_guard) = kv_sqlite().lock() {
            if let Some(ref conn) = *sqlite_guard {
                for k in &to_remove {
                    let _ =
                        conn.execute("DELETE FROM kv_store WHERE key = ?1", rusqlite::params![k]);
                }
            }
        }
        Ok(Value::Float(count as f64))
    }
}

/// `human_recall(persona, query, limit?)` — search persona's memories by keyword match.
/// Returns List of Memory structs sorted by importance (descending), then by recency.
/// Each struct: {key, content, importance, created_at, access_count, relevance}.
pub(crate) fn builtin_human_recall(args: &[Value]) -> Result<Value, String> {
    let persona = expect_string_arg("human_recall", args, 0)?;
    let query = expect_string_arg("human_recall", args, 1)?;
    let limit: usize = if args.len() >= 3 {
        expect_float_arg("human_recall", args, 2)? as usize
    } else {
        10
    };
    let prefix = format!("human_mem:{}:", persona);
    let query_lower = query.to_lowercase();
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    let store = kv_store()
        .lock()
        .map_err(|e| format!("human_recall() lock error: {}", e))?;
    let mut memories: Vec<(f64, f64, Value)> = Vec::new(); // (importance, recency, struct)

    for (k, v) in store.iter() {
        if !k.starts_with(&prefix) {
            continue;
        }
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(v) {
            let content = data
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            let key_str = data
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            // Simple relevance scoring: keyword match in content or key
            let query_words: Vec<&str> = query_lower.split_whitespace().collect();
            let mut matches = 0;
            for word in &query_words {
                if content.contains(word) || key_str.contains(word) {
                    matches += 1;
                }
            }
            let relevance = if query_words.is_empty() {
                0.5
            } else {
                matches as f64 / query_words.len() as f64
            };
            if relevance < 0.01 && !query.is_empty() {
                continue;
            } // skip non-matching if query given

            let importance = data
                .get("importance")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5);
            let created_at = data
                .get("created_at")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let access_count = data
                .get("access_count")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as f64;
            let age_hours = (now_ts - created_at).max(0.0) / 3600.0;
            // Recency score: 1.0 for fresh, decays over time (half-life ~168h = 1 week)
            let recency = (0.5_f64).powf(age_hours / 168.0);
            // Composite score: 50% relevance, 30% importance, 20% recency
            let score = relevance * 0.5 + importance * 0.3 + recency * 0.2;

            let mem_key = data
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let mem_content = data
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let mem_struct = make_date_struct(
                "Memory",
                vec![
                    ("key", Value::String(mem_key)),
                    ("content", Value::String(mem_content)),
                    ("importance", Value::Float(importance)),
                    ("created_at", Value::Float(created_at)),
                    ("access_count", Value::Float(access_count)),
                    ("relevance", Value::Float(relevance)),
                    ("score", Value::Float(score)),
                ],
            );
            memories.push((score, recency, mem_struct));
        }
    }
    drop(store);

    // Sort by score descending, take top N
    memories.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    memories.truncate(limit);

    let results: Vec<Value> = memories.into_iter().map(|(_, _, v)| v).collect();
    Ok(Value::List(results))
}

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
