use super::core::*;
use super::http::*;
use super::json::*;
use super::memory::*;
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
