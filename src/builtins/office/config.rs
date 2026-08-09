// ── Config / vault builtins ─────────────────────────────────────────
// ask_approval, config_load, get_profile, vault_validate
// Also contains chrono_now_timestamp (shared helper).

use super::super::core::*;
use super::super::http::*;
use super::super::json::*;
use super::super::memory::*;
use crate::interpreter::Value;

/// Helper: current Unix timestamp.
pub(crate) fn chrono_now_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

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
