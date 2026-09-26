use super::*;

impl Interpreter {
    // ── Наряд №67: recipe_save / recipe_search via memory infrastructure ──

    /// `recipe_save(name, description, skills, plan)` — builds recipe struct AND memorizes
    /// the description for later semantic search via recipe_search.
    /// The caller still does kv_set(saved.key, json_encode(saved.recipe)) for full data.
    pub(super) fn invoke_recipe_save_fn(&self, args: Vec<Value>) -> Result<Value, String> {
        // 1. Call the existing pure builtin to build the struct
        let result = crate::builtins::office::recipes::builtin_recipe_save(&args)?;

        // 2. Extract key and description for memorization
        let (kv_key, description) = match &result {
            Value::Struct { fields, .. } => {
                let key = fields
                    .iter()
                    .find(|(k, _)| *k == "key")
                    .and_then(|(_, v)| match v {
                        Value::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();
                let desc = args
                    .get(1)
                    .and_then(|v| match v {
                        Value::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();
                (key, desc)
            }
            _ => return Ok(result),
        };

        // 3. Memorize with type "recipe" for semantic search.
        // Value format: "__KVKEY:<key>\n<description>"
        // recipe_search will parse this to extract the KV key.
        if !description.is_empty() {
            let mem_value = format!("__KVKEY:{}\n{}", kv_key, description);
            // №466: the body moved to the shared live module
            // (src/memory_ops.rs) — direct call, same semantics.
            let _ = crate::memory_ops::memorize_tw(
                &self.memory,
                &self.embedding_manager,
                &[
                    Value::String(mem_value),
                    Value::Float(0.8),
                    Value::String("recipe".to_string()),
                ],
            );
        }

        Ok(result)
    }

    /// `recipe_search(query, k?)` — semantic search for recipes via recall_top_k.
    /// Searches memory entries with type "recipe", extracts KV keys from values,
    /// retrieves full recipe data from KV, returns list of recipe structs.
    pub(super) fn invoke_recipe_search_fn(&self, args: Vec<Value>) -> Result<Value, String> {
        if args.is_empty() {
            return Err("recipe_search() requires at least 1 argument (query)".to_string());
        }
        let query = match &args[0] {
            Value::String(s) => s.clone(),
            other => {
                return Err(format!(
                    "recipe_search() expected String as first arg, got {}",
                    other.type_name()
                ))
            }
        };
        let k = if args.len() > 1 {
            args[1].as_float().unwrap_or(5.0) as usize
        } else {
            5
        };

        // 1. Recall top-k memory entries of type "recipe"
        // №466: the body moved to the shared live module
        // (src/memory_ops.rs) — direct call, same semantics.
        let recall_results = crate::memory_ops::recall_top_k_tw(
            &self.memory,
            &self.embedding_manager,
            &[
                Value::String(query),
                Value::Float(k as f64),
                Value::String("recipe".to_string()),
            ],
        )?;

        // 2. Parse recall results (JSON string), extract KV keys, fetch full recipes
        let recall_json: Vec<serde_json::Value> = serde_json::from_str(match &recall_results {
            Value::String(s) => s,
            _ => return Ok(Value::List(vec![])),
        })
        .unwrap_or_default();

        let mut recipes: Vec<Value> = Vec::new();
        for entry in &recall_json {
            let value = entry["value"].as_str().unwrap_or("");
            // Value format: "__KVKEY:<key>\n<description>"
            let kv_key = if let Some(rest) = value.strip_prefix("__KVKEY:") {
                rest.lines().next().unwrap_or("")
            } else {
                ""
            };

            if kv_key.is_empty() {
                continue;
            }

            // Fetch full recipe from KV
            if let Some(recipe_json) = crate::builtins::memory::kv_get_raw(kv_key) {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&recipe_json) {
                    let name = parsed["name"].as_str().unwrap_or("").to_string();
                    let desc = parsed["description"].as_str().unwrap_or("").to_string();
                    let score = entry["score"].as_f64().unwrap_or(0.0);
                    recipes.push(crate::builtins::core::make_struct(
                        "RecipeResult",
                        vec![
                            ("name", Value::String(name)),
                            ("description", Value::String(desc)),
                            ("recipe_json", Value::String(recipe_json)),
                            ("score", Value::Float(score)),
                        ],
                    ));
                }
            }
        }

        Ok(Value::List(recipes))
    }

    /// Configure memory persistence (Phase 7.6).
    /// If persist path is provided, switches to SQLite-backed stores.
    /// The in-memory data is migrated to SQLite during the switch.
    pub fn configure_memory(&mut self, config: &MemoryDecl) {
        if let Some(ref path) = config.persist {
            // Switch to SQLite backend
            let db_path = std::path::PathBuf::from(path);
            match SqliteStore::open(&db_path) {
                Ok(sqlite_store) => {
                    // Migrate existing in-memory data to SQLite
                    let existing = self
                        .memory
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .all_entries();
                    let mut new_store: Box<dyn MemoryStore> = Box::new(sqlite_store);
                    for entry in existing {
                        let _ = new_store.memorize(entry);
                    }
                    self.memory = std::sync::Mutex::new(new_store);

                    // Migrate KG edges to SQLite (sharing the same DB file)
                    let existing_edges: Vec<(String, String, String, f64)> = self
                        .kg
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .all_edges();
                    if let Ok(sqlite_kg) = SqliteKg::open(&db_path) {
                        let mut new_kg: Box<dyn KgStore> = Box::new(sqlite_kg);
                        for (from, to, relation, weight) in existing_edges {
                            let _ = new_kg.relate(&from, &to, &relation, weight);
                        }
                        self.kg = std::sync::Mutex::new(new_kg);
                    } else {
                        eprintln!("[memory] KG migration to SQLite failed; keeping in-memory KG");
                    }
                    eprintln!("[memory] Persistence enabled: {}", path);
                    self.memory_persist_path = Some(path.clone());

                    // Наряд №6 — also enable KV store SQLite persistence
                    if let Err(e) = crate::builtins::init_kv_persist(path) {
                        eprintln!("[kv_store] Failed to enable KV persistence: {}. KV will be in-memory only.", e);
                    }

                    // ADR-0056: initialize checkpoint SQLite (same DB directory)
                    let cp_path = std::path::PathBuf::from(path).with_file_name("checkpoints.db");
                    if let Ok(conn) = rusqlite::Connection::open(&cp_path) {
                        let _ = conn.execute_batch(
                            "CREATE TABLE IF NOT EXISTS checkpoints (
                                flow_name TEXT NOT NULL,
                                checkpoint_name TEXT NOT NULL,
                                step_index INTEGER NOT NULL,
                                state_json TEXT NOT NULL,
                                created_at INTEGER NOT NULL,
                                PRIMARY KEY (flow_name, checkpoint_name)
                            )",
                        );
                        *self.checkpoint_db.lock().unwrap_or_else(|e| e.into_inner()) = Some(conn);
                    }
                }
                Err(e) => {
                    eprintln!(
                        "[memory] Failed to open persistent store '{}': {}. Using in-memory.",
                        path, e
                    );
                }
            }
        }
        // If persist is None, keep the default InMemoryStore (already set in new())
    }

    /// Entity store query: find("TypeName", "field", "op", threshold)
    /// Searches all entities of the given type and returns the first one matching the condition.
    /// Soft-failure: returns Unit if no match found.
    pub(super) fn invoke_find(&self, args: Vec<Value>) -> Result<Value, String> {
        let type_name = match args.first() {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("find() requires type name as first argument (String)".to_string()),
        };
        let field_name = match args.get(1) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("find() requires field name as second argument (String)".to_string()),
        };
        let op_str = match args.get(2) {
            Some(Value::String(s)) => s.clone(),
            _ => {
                return Err(
                    "find() requires operator as third argument (String: gt/lt/ge/le/eq)"
                        .to_string(),
                )
            }
        };
        let threshold = match args.get(3) {
            Some(Value::Float(f)) => *f,
            _ => return Err("find() requires threshold as fourth argument (Float)".to_string()),
        };

        // Search all variables for entities of the matching type
        for value in self.variables.values() {
            if let Value::Struct {
                type_name: tn,
                fields,
            } = value
            {
                if tn == &type_name {
                    if let Some(field_val) = fields.get(&field_name) {
                        if let Ok(fv) = field_val.as_float() {
                            let matches = match op_str.as_str() {
                                "gt" => fv > threshold,
                                "lt" => fv < threshold,
                                "ge" => fv >= threshold,
                                "le" => fv <= threshold,
                                "eq" => (fv - threshold).abs() < 1e-9,
                                _ => return Err(format!("find(): unknown operator '{}'", op_str)),
                            };
                            if matches {
                                return Ok(value.clone());
                            }
                        }
                    }
                }
            }
        }

        // No match found — soft-failure
        Ok(Value::Unit)
    }

    pub fn get_memory_persist_path(&self) -> Option<String> {
        self.memory_persist_path.clone()
    }

    pub fn set_memory_persist_path(&mut self, path: Option<String>) {
        self.memory_persist_path = path;
    }
}
