use super::*;

impl Interpreter {
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

        // Embed the query for semantic search
        let query_embedding = self.embedding_manager.embed(&query).unwrap_or_default();

        // Use MemoryStore trait for recall (handles both InMemory and SQLite)
        match lock_or_err(self.memory.lock())?.recall(&query, &query_embedding, min_confidence) {
            Some((entry, _score)) => {
                // Walk the knowledge graph for related memories
                let edges = lock_or_err(self.kg.lock())?.edges_for(&entry.value);
                if edges.is_empty() {
                    Ok(Value::String(entry.value.clone()))
                } else {
                    let mut result = entry.value.clone();
                    for (relation, other, _weight) in &edges {
                        result.push('\n');
                        result.push_str(&format!("[GRAPH] {} -> {}", relation, other));
                    }
                    Ok(Value::String(result))
                }
            }
            None => Ok(Value::String(String::new())),
        }
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
            other => return Err(format!(
                "recall_top_k() expected String as first arg, got {}",
                other.type_name()
            )),
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
        Ok(Value::String(serde_json::to_string(&json_results).unwrap_or_default()))
    }
}
