//! №466 (gh#687) — group 6 (recipe) of the TW/VM duplicate-name
//! transfer: the shared LIVE home of the recipe surface (gate gh#680,
//! decision 4-A, step 3; threshold 30 → 28).
//!
//! The two names — `recipe_save`, `recipe_search` — keep their
//! per-backend BODIES deliberately: the search lanes are the
//! documented divergent twins (the TW resolves through the FTS5+cosine
//! hybrid `recall_top_k_tw` with the embedding lane; the VM does the
//! token-level AND over its simple in-process memory — the Bug #530
//! posture "each backend reads its own store"), and the error texts
//! differ byte-for-byte (TW `got {}` + `type_name()` vs VM `got {:?}`).
//! What IS shared — because it is byte-identical on both backends —
//! moves here:
//!
//! - the single spelling of the two names outside `BUILTIN_REGISTRY`
//!   (the №462 counter's duplicates were the dispatch literals);
//! - the `__KVKEY:` value format (the save→search contract): the
//!   memorized value string and the KV-key extraction;
//! - the recipe memorization constants (priority 0.8, mem type
//!   "recipe") that both lanes use;
//! - the KV fetch + `RecipeResult` struct build, parameterized by the
//!   score: the TW lane carries the recall score (`Some(f64)` — the
//!   4-field struct), the VM lane never had one (`None` — the 3-field
//!   struct, preserved).
//!
//! The №465 diff fuzzer runs before/after the transfer (the divergence
//! classes must not move); the per-site semantics are pinned by the
//! unit tests below.

use crate::interpreter::Value;

/// The single spelling of the recipe group outside the registry.
pub const NAME_RECIPE_SAVE: &str = "recipe_save";
pub const NAME_RECIPE_SEARCH: &str = "recipe_search";

/// Whether `name` belongs to the recipe group (the dispatch hook).
pub fn handles(name: &str) -> bool {
    matches!(name, NAME_RECIPE_SAVE | NAME_RECIPE_SEARCH)
}

/// The save→search contract: a recipe description is memorized as
/// `__KVKEY:<key>\n<description>`; the search lane parses this format
/// back. Both backends write and read EXACTLY this shape.
pub const KVKEY_PREFIX: &str = "__KVKEY:";

/// The memorized value for a saved recipe (the format contract).
pub fn mem_value(kv_key: &str, description: &str) -> String {
    format!("{}{}\n{}", KVKEY_PREFIX, kv_key, description)
}

/// The memorization constants both lanes use (№67): the recipe
/// description enters memory with priority 0.8 and type "recipe".
pub const RECIPE_PRIORITY: f64 = 0.8;
pub const RECIPE_MEM_TYPE: &str = "recipe";

/// Extract the KV key from a memorized value (the parse half of the
/// format contract): everything after `__KVKEY:` up to the first
/// newline; empty when the value does not carry the prefix.
pub fn kv_key_from_value(value: &str) -> &str {
    value
        .strip_prefix(KVKEY_PREFIX)
        .and_then(|rest| rest.lines().next())
        .unwrap_or("")
}

/// Fetch a full recipe from the shared KV store and build the
/// `RecipeResult` struct. `score` is the recall relevance: the TW lane
/// (the FTS5+cosine hybrid) carries it — the 4-field struct; the VM
/// lane (the simple-memory twin) never had a score — `None` keeps the
/// 3-field struct. The field order is byte-preserved:
/// name, description, recipe_json[, score].
pub fn recipe_from_kv(kv_key: &str, score: Option<f64>) -> Option<Value> {
    let recipe_json = crate::builtins::memory::kv_get_raw(kv_key)?;
    let parsed = serde_json::from_str::<serde_json::Value>(&recipe_json).ok()?;
    let name = parsed["name"].as_str().unwrap_or("").to_string();
    let desc = parsed["description"].as_str().unwrap_or("").to_string();
    let mut fields = vec![
        ("name", Value::String(name)),
        ("description", Value::String(desc)),
        ("recipe_json", Value::String(recipe_json)),
    ];
    if let Some(score) = score {
        fields.push(("score", Value::Float(score)));
    }
    Some(crate::builtins::core::make_struct("RecipeResult", fields))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_covers_exactly_the_two_names() {
        assert!(handles(NAME_RECIPE_SAVE));
        assert!(handles(NAME_RECIPE_SEARCH));
        assert!(!handles("recipe"));
        assert!(!handles("memorize"));
        assert!(!handles("print"));
    }

    #[test]
    fn mem_value_format_is_the_save_search_contract() {
        let v = mem_value("r-123", "deploy the thing");
        assert_eq!(v, "__KVKEY:r-123\ndeploy the thing");
        // Round trip: the search lane extracts the key back.
        assert_eq!(kv_key_from_value(&v), "r-123");
    }

    #[test]
    fn kv_key_extraction_edges() {
        assert_eq!(kv_key_from_value("__KVKEY:k1\nfirst line\nsecond"), "k1");
        assert_eq!(kv_key_from_value("__KVKEY:k2"), "k2");
        assert_eq!(kv_key_from_value("no prefix here"), "");
        assert_eq!(kv_key_from_value("__KVKEY:"), "");
        // A multi-line value: only the FIRST line is the key.
        assert_eq!(kv_key_from_value("__KVKEY:k3\ndesc with\nnewlines"), "k3");
    }

    #[test]
    fn mem_constants_are_the_no67_contract() {
        assert_eq!(RECIPE_PRIORITY, 0.8);
        assert_eq!(RECIPE_MEM_TYPE, "recipe");
    }

    fn seed_kv(key: &str, name: &str, desc: &str) {
        let payload = serde_json::json!({ "name": name, "description": desc });
        crate::builtins::memory::kv_store()
            .lock()
            .expect("kv store lock")
            .insert(
                key.to_string(),
                serde_json::to_string(&payload).expect("json"),
            );
    }

    #[test]
    fn recipe_from_kv_with_score_builds_the_tw_4_field_struct() {
        seed_kv("g6-test-tw", "Deploy", "deploy the thing");
        let out = recipe_from_kv("g6-test-tw", Some(0.75)).expect("the recipe is in KV");
        match out {
            Value::Struct { type_name, fields } => {
                assert_eq!(type_name, "RecipeResult");
                assert!(matches!(
                    fields.get("name"),
                    Some(Value::String(s)) if s == "Deploy"
                ));
                assert!(matches!(
                    fields.get("description"),
                    Some(Value::String(s)) if s == "deploy the thing"
                ));
                assert!(matches!(fields.get("recipe_json"), Some(Value::String(_))));
                assert!(matches!(fields.get("score"), Some(Value::Float(f)) if *f == 0.75));
            }
            other => panic!("expected a struct, got {:?}", other),
        }
    }

    #[test]
    fn recipe_from_kv_without_score_builds_the_vm_3_field_struct() {
        seed_kv("g6-test-vm", "Backup", "back it up");
        let out = recipe_from_kv("g6-test-vm", None).expect("the recipe is in KV");
        match out {
            Value::Struct { type_name, fields } => {
                assert_eq!(type_name, "RecipeResult");
                assert!(matches!(
                    fields.get("name"),
                    Some(Value::String(s)) if s == "Backup"
                ));
                // The VM lane never had a score — the field is ABSENT.
                assert!(!fields.contains_key("score"));
            }
            other => panic!("expected a struct, got {:?}", other),
        }
    }

    #[test]
    fn recipe_from_kv_missing_key_is_none() {
        assert!(recipe_from_kv("g6-test-absent-key", Some(1.0)).is_none());
    }
}
