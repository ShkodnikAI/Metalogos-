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
            let _ = self.invoke_memorize_fn(vec![
                Value::String(mem_value),
                Value::Float(0.8),
                Value::String("recipe".to_string()),
            ]);
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
        let recall_results = self.invoke_recall_top_k_fn(vec![
            Value::String(query),
            Value::Float(k as f64),
            Value::String("recipe".to_string()),
        ])?;

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

    /// Recall from memory: find best matching entry using embeddings + decay.
    /// Phase 7.2: Uses cosine similarity on embedding vectors (semantic search).
    /// Falls back to substring match if embeddings are unavailable (empty vectors).
    /// Returns the highest-activation entry above the min_confidence threshold.
    ///
    /// №442: recall is the front door of memory. The store lane now runs
    /// through the HYBRID engines (FTS5 BM25 + cosine RRF on the SQLite
    /// store; the all-entries scan on the in-memory store) with the old
    /// full-scan `recall()` kept as the safety net, and the TYPED lane
    /// joins as a recall source with the №413 fail-closed consent
    /// contract: a query that names gated private memory refuses with
    /// the typed MEMORY_RECALL_CONSENT_REQUIRED stamp (the refusal is a
    /// `memory.recall.denied` record), typed hits carry the `[MEM]`
    /// provenance suffix, and every call leaves a `memory.recall`
    /// ledger record. The store lane's external contract is unchanged:
    /// the best entry's value (plus its `[GRAPH]` edges) as a String.
    pub(super) fn invoke_recall(&self, args: Vec<Value>) -> Result<Value, String> {
        if args.is_empty() {
            return Err("recall() requires at least 1 argument (query string)".to_string());
        }

        let query = match &args[0] {
            Value::String(s) => s.clone(),
            other => {
                return Err(format!(
                    "recall() expected String argument, got {}",
                    other.type_name()
                ))
            }
        };

        let min_confidence = if args.len() > 1 {
            args[1].as_float().unwrap_or(0.3) as f32
        } else {
            0.3
        };

        // ── №442: the typed lane — the fail-closed consent gate comes
        // FIRST (a refusal discloses nothing, not even partial results).
        let lane = crate::memory_typed::recall_lane(&query);
        if let Some((container_id, subject)) = lane.gated_key_matches.first() {
            crate::memory_typed::ledger_recall_denied(&query, container_id);
            return Err(crate::memory_typed::recall_consent_refusal(
                &query,
                container_id,
                subject,
            ));
        }

        // Embed the query for semantic search
        let query_embedding = self.embedding_manager.embed(&query).unwrap_or_default();

        // ── The store lane, through the hybrid engines (№442). RRF
        // scores are rank-based and not comparable with the 0..1
        // confidence threshold, so the threshold keeps the store's
        // activation semantics (sim × priority × decay — the exact
        // scoring `recall()` has always applied) and is re-checked per
        // candidate; the old full-scan recall() stays as the safety net
        // for candidates the BM25 candidate set missed.
        let store_hit = {
            let memory = lock_or_err(self.memory.lock())?;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let hybrid = memory.recall_top_k(&query, &query_embedding, 0.0, 5, "");
            let from_hybrid = hybrid
                .into_iter()
                .map(|(entry, _rrf)| {
                    let sim = if !query_embedding.is_empty() && !entry.embedding.is_empty() {
                        crate::embeddings::cosine_similarity(&query_embedding, &entry.embedding)
                    } else if entry.value.contains(&query) {
                        1.0
                    } else {
                        0.0
                    };
                    let age_days = ((now - entry.timestamp).max(0) as f64) / 86400.0;
                    let decay = (-entry.decay_rate * age_days).exp() as f32;
                    let signal = sim * (entry.priority as f32) * decay;
                    (entry, signal)
                })
                .find(|(_, signal)| *signal >= min_confidence);
            match from_hybrid {
                Some((entry, _)) => Some(entry),
                None => memory
                    .recall(&query, &query_embedding, min_confidence)
                    .map(|(entry, _)| entry),
            }
        };

        // ── Merge: the store lane is recall's primary lane (its
        // external contract is regression-pinned); the typed lane is the
        // fallback source whose hits carry the [MEM] provenance suffix.
        let had_store_hit = store_hit.is_some();
        let result = match store_hit {
            Some(entry) => {
                // Walk the knowledge graph for related memories
                let edges = lock_or_err(self.kg.lock())?.edges_for(&entry.value);
                if edges.is_empty() {
                    entry.value.clone()
                } else {
                    let mut result = entry.value.clone();
                    for (relation, other, _weight) in &edges {
                        result.push('\n');
                        result.push_str(&format!("[GRAPH] {} -> {}", relation, other));
                    }
                    result
                }
            }
            None => match lane.hits.first() {
                Some(hit) => {
                    let mut result = hit.text.clone();
                    result.push_str(&crate::memory_typed::recall_hit_provenance(hit));
                    result
                }
                None => String::new(),
            },
        };

        // ── №442: the ledger record — every recall call is audited
        // {query hash, containers, hits, consent fact} (№393/ADR-0167).
        let disclosed = if had_store_hit { 1 } else { lane.hits.len() };
        crate::memory_typed::ledger_recall(&query, &lane, disclosed);

        Ok(Value::String(result))
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

    /// Callable form of memorize() — usable inside patterns and route handlers.
    /// Usage: memorize("user likes spicy food", 0.5) or memorize("fact")
    /// Differs from declaration `memorize "text" with priority=0.5` (top-level only).
    pub(super) fn invoke_memorize_fn(&self, args: Vec<Value>) -> Result<Value, String> {
        if args.is_empty() {
            return Err("memorize() requires at least 1 argument (text)".to_string());
        }
        let value_str = match &args[0] {
            Value::String(s) => s.clone(),
            other => {
                return Err(format!(
                    "memorize() expected String as first arg, got {}",
                    other.type_name()
                ))
            }
        };
        let priority = if args.len() > 1 {
            args[1].as_float().unwrap_or(1.0)
        } else {
            1.0
        };
        let mem_type = if args.len() > 2 {
            match &args[2] {
                Value::String(s) => s.clone(),
                other => format!("{}", other),
            }
        } else {
            String::new()
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let embedding = self.embedding_manager.embed(&value_str).unwrap_or_default();
        match lock_or_err(self.memory.lock())?.memorize(MemoryEntry {
            id: None,
            value: value_str.clone(),
            priority,
            timestamp: now,
            decay_rate: 0.01,
            confidence: priority,
            embedding,
            mem_type,
        }) {
            Ok(_id) => { /* Bug 2.3 fix: removed eprintln stdout leak in HTTP context */ }
            Err(_) => { /* silent — don't leak to stdout in HTTP context */ }
        }
        Ok(Value::Unit)
    }

    /// Callable form of forget() — usable inside patterns and route handlers.
    /// Usage: forget("query", 30) — forget entries matching "query" older than 30 days.
    pub(super) fn invoke_forget_fn(&self, args: Vec<Value>) -> Result<Value, String> {
        if args.is_empty() {
            return Err("forget() requires at least 1 argument (query)".to_string());
        }
        let query_str = match &args[0] {
            Value::String(s) => s.clone(),
            other => {
                return Err(format!(
                    "forget() expected String as first arg, got {}",
                    other.type_name()
                ))
            }
        };
        let days = if args.len() > 1 {
            args[1].as_float().unwrap_or(30.0) as i64
        } else {
            30
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let cutoff = now - (days * 86400);
        lock_or_err(self.memory.lock())?.forget(&query_str, cutoff);
        Ok(Value::Unit)
    }

    pub fn get_memory_persist_path(&self) -> Option<String> {
        self.memory_persist_path.clone()
    }

    pub fn set_memory_persist_path(&mut self, path: Option<String>) {
        self.memory_persist_path = path;
    }

    /// Callable form of recall_top_k() — hybrid FTS5 BM25 + cosine RRF search.
    /// Usage: recall_top_k("query", 5, "persona") — top-5 memories of type "persona".
    /// Third arg (type filter) is optional; empty string = search all types.
    pub(super) fn invoke_recall_top_k_fn(&self, args: Vec<Value>) -> Result<Value, String> {
        if args.is_empty() {
            return Err("recall_top_k() requires at least 1 argument (query)".to_string());
        }
        let query = match &args[0] {
            Value::String(s) => s.clone(),
            other => {
                return Err(format!(
                    "recall_top_k() expected String as first arg, got {}",
                    other.type_name()
                ))
            }
        };
        let k = if args.len() > 1 {
            args[1].as_float().unwrap_or(5.0) as usize
        } else {
            5
        };
        let type_filter = if args.len() > 2 {
            match &args[2] {
                Value::String(s) => s.clone(),
                Value::Unit => String::new(),
                other => format!("{}", other),
            }
        } else {
            String::new()
        };
        let query_embedding = self.embedding_manager.embed(&query).unwrap_or_default();
        let memory = lock_or_err(self.memory.lock())?;
        let results = memory.recall_top_k(&query, &query_embedding, 0.0, k, &type_filter);
        let json_results: Vec<serde_json::Value> = results
            .into_iter()
            .map(|(entry, score)| {
                serde_json::json!({
                    "value": entry.value,
                    "score": score,
                    "type": entry.mem_type,
                    "priority": entry.priority,
                })
            })
            .collect();
        Ok(Value::String(
            serde_json::to_string(&json_results).unwrap_or_default(),
        ))
    }
}
