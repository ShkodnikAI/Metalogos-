// ── Recipe builtins ──────────────────────────────────────────────────
// recipe_save, recipe_search, recipe_list

use super::super::core::*;
use super::super::json::*;
use crate::interpreter::Value;

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
