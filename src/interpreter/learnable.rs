use super::*;
use crate::ast::{ContextMode, ContextStrategy};
use crate::embeddings::cosine_similarity;
use crate::llm;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

impl Interpreter {
    /// Injects hook variables: pattern_name (String), args (List),
    /// result (after only), confidence (after only).
    /// Builtins are NOT wrapped — only user-defined patterns and learnable patterns.
    pub(super) fn invoke_pattern_with_hooks<F>(
        &self,
        name: &str,
        args: &[Value],
        f: F,
    ) -> Result<Value, String>
    where
        F: FnOnce() -> Result<Value, String>,
    {
        // ADR-0051: Track stats for regular (non-learnable) patterns.
        // Learnable patterns track themselves in invoke_learnable_with_env.
        if !self.learnable_patterns.contains_key(name) {
            self.record_pattern_call(name, false);
        }

        // Execute all before_pattern hooks
        if !self.hooks_before.is_empty() {
            let mut hook_env = HashMap::new();
            hook_env.insert("pattern_name".to_string(), Value::String(name.to_string()));
            hook_env.insert("args".to_string(), Value::List(args.to_vec()));
            for hook in &self.hooks_before {
                // Ignore hook errors — hooks are advisory, not blocking
                let _ = self.eval_statements(&hook.body, &mut hook_env);
            }
        }

        // Execute the actual pattern
        let result = f();

        // Execute all after_pattern hooks
        if !self.hooks_after.is_empty() {
            let mut hook_env = HashMap::new();
            hook_env.insert("pattern_name".to_string(), Value::String(name.to_string()));
            hook_env.insert("args".to_string(), Value::List(args.to_vec()));
            match &result {
                Ok(val) => {
                    hook_env.insert("result".to_string(), val.clone());
                    // Extract confidence for Fluid results, default 1.0
                    let conf = match val {
                        Value::Fluid(variants) => variants
                            .iter()
                            .map(|v| v.confidence)
                            .fold(0.0_f64, f64::max),
                        _ => 1.0,
                    };
                    hook_env.insert("confidence".to_string(), Value::Float(conf));
                }
                Err(e) => {
                    hook_env.insert("result".to_string(), Value::String(e.clone()));
                    hook_env.insert("confidence".to_string(), Value::Float(0.0));
                }
            }
            for hook in &self.hooks_after {
                // Ignore hook errors — hooks are advisory, not blocking
                let _ = self.eval_statements(&hook.body, &mut hook_env);
            }
        }

        result
    }

    /// ADR-0046: Build the effective system prompt for a learnable pattern.
    /// Handles all ContextMode variants:
    /// - None → return base prompt unchanged
    /// - Auto → recall(first_param_value, limit=5) + prepend context
    /// - Recall(query_expr, limit) → evaluate query, recall, prepend context
    /// - Literal(string) → prepend static text as context
    ///
    /// ADR-0055: Context compression.
    /// When context_strategy is Compress and the recalled context exceeds
    /// max_context_tokens estimated tokens, the facts are compressed via LLM
    /// summarization before being prepended to the prompt.
    fn build_effective_prompt(&self, learnable: &CompiledLearnable, args: &[Value]) -> String {
        let context_mode = match &learnable.context {
            None => return learnable.prompt.clone(),
            Some(ContextMode::None) => return learnable.prompt.clone(),
            Some(mode) => mode.clone(),
        };

        match context_mode {
            ContextMode::None | ContextMode::Auto | ContextMode::Recall(_, _) => {
                // Determine the query string and limit for recall
                let (query, limit) = match &learnable.context {
                    Some(ContextMode::Auto) => {
                        // Use first parameter's runtime value as query
                        let query_str = if !args.is_empty() {
                            match &args[0] {
                                Value::String(s) => s.clone(),
                                other => format!("{}", other),
                            }
                        } else {
                            return learnable.prompt.clone();
                        };
                        (query_str, 5)
                    }
                    Some(ContextMode::Recall(query_expr, limit_opt)) => {
                        // Evaluate the context query expression with param names bound to args
                        let mut env: HashMap<String, Value> = HashMap::new();
                        for (i, param) in learnable.params.iter().enumerate() {
                            if i < args.len() {
                                env.insert(param.name.clone(), args[i].clone());
                            }
                        }
                        let query = match self.eval_expr_with_env(query_expr, &env) {
                            Ok(Value::String(s)) => s,
                            Ok(other) => format!("{}", other),
                            Err(_) => return learnable.prompt.clone(), // context eval failed → skip
                        };
                        (query, limit_opt.unwrap_or(5))
                    }
                    _ => unreachable!(),
                };

                // Recall memories: collect up to `limit` unique results
                let mut facts = Vec::new();
                let query_embedding = self.embedding_manager.embed(&query).unwrap_or_default();
                let mut seen = std::collections::HashSet::new();
                let min_conf = 0.1_f32;

                match self.memory.lock() {
                    Ok(store) => {
                        let all = store.all_entries();
                        let mut scored: Vec<(String, f32)> = Vec::new();
                        for entry in all {
                            if seen.contains(&entry.value) {
                                continue;
                            }
                            let score = if entry.embedding.is_empty() || query_embedding.is_empty()
                            {
                                if entry.value.to_lowercase().contains(&query.to_lowercase()) {
                                    0.5
                                } else {
                                    0.0
                                }
                            } else {
                                cosine_similarity(&query_embedding, &entry.embedding)
                                    * entry.confidence as f32
                                    * entry.priority as f32
                            };
                            if score >= min_conf {
                                seen.insert(entry.value.clone());
                                scored.push((entry.value, score));
                            }
                        }
                        scored.sort_by(|a, b| {
                            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
                        });
                        for (fact, _score) in scored.iter().take(limit) {
                            facts.push(fact.clone());
                        }
                    }
                    Err(_) => return learnable.prompt.clone(),
                }

                if facts.is_empty() {
                    return learnable.prompt.clone();
                }

                // Format context block
                let context_block = self.format_context_block(&facts);

                // ADR-0055: Apply compression strategy
                if learnable.context_strategy == ContextStrategy::Compress {
                    let estimated_tokens = Self::estimate_tokens(&context_block);
                    if estimated_tokens > learnable.max_context_tokens {
                        // Compress: call LLM to summarize the context
                        let compressed = self.compress_context(&context_block);
                        return format!("{}\n{}", compressed, learnable.prompt);
                    }
                }

                // No compression needed — inject as-is
                format!("{}\n{}", context_block, learnable.prompt)
            }
            ContextMode::Literal(literal_text) => {
                // Prepend static literal text as context
                format!("{}\n{}", literal_text, learnable.prompt)
            }
        }
    }

    /// ADR-0055: Format recalled facts into a context block string.
    /// Format: "Relevant context:\n- fact1\n- fact2\n..."
    fn format_context_block(&self, facts: &[String]) -> String {
        let mut block = String::from("Relevant context:\n");
        for fact in facts {
            block.push_str("- ");
            block.push_str(fact);
            block.push('\n');
        }
        block
    }

    /// ADR-0055: Estimate token count for a string.
    /// Uses a rough approximation:
    /// - English text: ~4 chars per token
    /// - Cyrillic text: ~2 chars per token
    /// - Mixed: detect Cyrillic ratio and blend
    fn estimate_tokens(text: &str) -> usize {
        Self::estimate_tokens_static(text)
    }

    /// ADR-0055: Compress a context block via LLM summarization.
    /// Uses SmartRouter if available, falls back to legacy backend.
    /// If the LLM call fails, returns the original context block (graceful degradation).
    fn compress_context(&self, context_block: &str) -> String {
        let summary_prompt = format!(
            "Summarize the following facts concisely. Retain key information. \
             Output a single paragraph.\n\n{}",
            context_block
        );
        let result = llm::call_via_global_router(&summary_prompt, "", None)
            .unwrap_or_else(|| {
                let backend = llm::create_llm_backend();
                backend.call(&summary_prompt, "")
            });
        match result {
            Ok(summary) => {
                let trimmed = summary.trim().to_string();
                if trimmed.is_empty() {
                    context_block.to_string()
                } else {
                    format!("Compressed context:\n{}", trimmed)
                }
            }
            Err(_) => {
                // Graceful degradation: use original context if compression fails
                context_block.to_string()
            }
        }
    }

    /// Invoke a learnable pattern using pre-collapsed arguments.
    pub(super) fn invoke_learnable_with_env(
        &self,
        pattern_name: &str,
        learnable: &CompiledLearnable,
        args: &[Value],
    ) -> Result<Value, String> {
        // Build input string from arguments
        let input_parts: Vec<String> = args.iter().map(|a| format!("{}", a)).collect();
        let input = input_parts.join(", ");

        // Check few-shot examples first (exact match → effectively a cache hit)
        for (example_input, example_output) in &learnable.few_shot {
            if input == *example_input {
                // ADR-0051: record stats — few-shot match counts as cache hit
                self.record_pattern_call(pattern_name, true);
                return Ok(Value::String(example_output.clone()));
            }
        }

        // Phase 7.5: Sandbox enforcement — network isolation
        if let Some(ref sb) = self.active_sandbox {
            if sb.forbidden.iter().any(|f| f == "network") {
                return Err(format!("network access forbidden in sandbox '{}'", sb.name));
            }
        }

        // Build the effective system prompt (base prompt + optional context)
        let effective_prompt = self.build_effective_prompt(learnable, args);

        // ADR-0047: Check LLM response cache
        if learnable.cache {
            let cache_key = self.compute_cache_key(&effective_prompt, &input);
            if let Some(cached) = self.llm_cache_get(&cache_key, learnable.cache_ttl) {
                // Cache hit — return cached response without calling LLM
                // ADR-0051: record stats — cache hit
                self.record_pattern_call(pattern_name, true);
                return cached;
            }
        }

        // No few-shot match — call LLM backend
        let start = SystemTime::now();

        // Наряд №4: Use SmartRouter if llm config is present, otherwise legacy backend
        let resolved_model = learnable
            .model
            .as_ref()
            .map(|alias| llm::resolve_model(alias));
        let response = match self.smart_router.lock() {
            Ok(guard) => {
                if let Some(ref router) = *guard {
                    router.call(&effective_prompt, &input, resolved_model.as_deref())
                } else {
                    let backend = llm::create_llm_backend();
                    backend.call_with_model(&effective_prompt, &input, resolved_model.as_deref())
                }
            }
            Err(_) => {
                let backend = llm::create_llm_backend();
                backend.call_with_model(&effective_prompt, &input, resolved_model.as_deref())
            }
        }?;

        // Phase 7.5: Sandbox enforcement — timeout check
        if let Some(ref sb) = self.active_sandbox {
            let elapsed = start.elapsed().unwrap_or_default();
            if sb.timeout > 0 && elapsed.as_secs() >= sb.timeout as u64 {
                return Err(format!("operation timed out in sandbox '{}'", sb.name));
            }
        }

        // ADR-0047: Store response in cache
        if learnable.cache {
            let cache_key = self.compute_cache_key(&effective_prompt, &input);
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let entry = LlmCacheEntry {
                response: response.clone(),
                created_at: now,
                ttl: learnable.cache_ttl,
            };
            let mut cache = self
                .llm_cache
                .lock()
                .map_err(|e| format!("llm_cache lock error: {}", e))?;
            // Persist before inserting (entry is moved into cache.insert)
            self.llm_cache_persist(&cache_key, &entry);
            cache.insert(cache_key, entry);
        }

        // Try to parse JSON response into Value::Struct
        // ADR-0051: record stats — normal LLM call (not a cache hit)
        self.record_pattern_call(pattern_name, false);

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response) {
            if let Some(obj) = json.as_object() {
                let mut fields = std::collections::HashMap::new();
                for (k, v) in obj {
                    fields.insert(k.clone(), self.json_value_to_value(v));
                }
                return Ok(Value::Struct {
                    type_name: "LlmResponse".to_string(),
                    fields,
                });
            }
        }

        Ok(Value::String(response))
    }

    /// ADR-0047: Compute a cache key from prompt + input using simple SipHash.
    fn compute_cache_key(&self, prompt: &str, input: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        prompt.hash(&mut hasher);
        input.hash(&mut hasher);
        hasher.finish()
    }

    /// ADR-0047: Look up a cached response. Checks TTL expiry.
    /// Returns None on miss or expired entry (also removes expired entry).
    fn llm_cache_get(&self, key: &u64, ttl: u64) -> Option<Result<Value, String>> {
        let mut cache = self.llm_cache.lock().ok()?;
        let entry = cache.get(key)?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        // Check TTL — use the entry's own TTL if set, otherwise use the provided default
        let effective_ttl = if entry.ttl > 0 {
            entry.ttl as i64
        } else {
            ttl as i64
        };
        if now - entry.created_at > effective_ttl {
            cache.remove(key); // expired — evict
            return None;
        }

        // Try to parse cached JSON response
        let response = &entry.response;
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(response) {
            if let Some(obj) = json.as_object() {
                let mut fields = std::collections::HashMap::new();
                for (k, v) in obj {
                    fields.insert(k.clone(), self.json_value_to_value(v));
                }
                return Some(Ok(Value::Struct {
                    type_name: "LlmResponse".to_string(),
                    fields,
                }));
            }
        }

        Some(Ok(Value::String(response.clone())))
    }

    /// ADR-0047: Persist a cache entry to SQLite if persist is enabled.
    fn llm_cache_persist(&self, key: &u64, entry: &LlmCacheEntry) {
        if self.memory_persist_path.is_none() {
            return;
        }
        if let Some(ref path) = self.memory_persist_path {
            let db_path = std::path::PathBuf::from(path);
            if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                let _ = conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS llm_cache (hash INTEGER PRIMARY KEY, response TEXT NOT NULL, created_at INTEGER NOT NULL, ttl INTEGER NOT NULL);"
                );
                let _ = conn.execute(
                    "INSERT OR REPLACE INTO llm_cache (hash, response, created_at, ttl) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![(*key) as i64, &entry.response, entry.created_at, entry.ttl as i64],
                );
            }
        }
    }

    /// Convert serde_json::Value to METALOGOS Value.
    fn json_value_to_value(&self, json: &serde_json::Value) -> Value {
        match json {
            serde_json::Value::String(s) => Value::String(s.clone()),
            serde_json::Value::Number(n) => Value::Float(n.as_f64().unwrap_or(0.0)),
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Null => Value::Unit,
            serde_json::Value::Array(arr) => {
                Value::List(arr.iter().map(|v| self.json_value_to_value(v)).collect())
            }
            serde_json::Value::Object(obj) => {
                let mut fields = std::collections::HashMap::new();
                for (k, v) in obj {
                    fields.insert(k.clone(), self.json_value_to_value(v));
                }
                Value::Struct {
                    type_name: "Json".to_string(),
                    fields,
                }
            }
        }
    }

    pub fn get_learnable_patterns(&self) -> &HashMap<String, CompiledLearnable> {
        &self.learnable_patterns
    }

    /// ADR-0055: Public token estimation (static, no interpreter state needed).
    /// Exposed for contract tests.
    pub fn estimate_tokens_static(text: &str) -> usize {
        let total_chars = text.chars().count();
        if total_chars == 0 {
            return 0;
        }
        let cyrillic_count = text
            .chars()
            .filter(|c| *c >= '\u{0400}' && *c <= '\u{04FF}')
            .count();
        let cyrillic_ratio = cyrillic_count as f64 / total_chars as f64;
        let chars_per_token = 4.0 * (1.0 - cyrillic_ratio) + 2.0 * cyrillic_ratio;
        (total_chars as f64 / chars_per_token).ceil() as usize
    }

    // ── inspect() builtin support (ADR-0051) ─────────────────────────────

    /// Record a learnable pattern invocation for stats tracking.
    /// Called by invoke_learnable_with_env() on every invocation.
    fn record_pattern_call(&self, name: &str, cache_hit: bool) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        if let Ok(mut stats) = self.pattern_stats.lock() {
            let entry = stats
                .entry(name.to_string())
                .or_insert_with(PatternStats::new);
            entry.calls += 1;
            entry.confidence_sum += 1.0; // Default confidence for non-Fluid results
            if cache_hit {
                entry.cache_hits += 1;
            }
            entry.last_call = now;
        }
        // ADR-0052: emit pattern_call event
        let mut data = HashMap::new();
        data.insert("name".to_string(), name.to_string());
        data.insert(
            "cache_hit".to_string(),
            if cache_hit {
                "true".to_string()
            } else {
                "false".to_string()
            },
        );
        self.emit_event("pattern_call", name, data, None);
    }

    /// Invoke the `inspect()` builtin (ADR-0051).
    /// Returns a Struct with pattern metadata: calls, avg_confidence, cache_hits,
    /// cache_misses, last_adapt, last_call, examples_count, is_learnable.
    pub(super) fn invoke_inspect(&self, args: &[Value]) -> Result<Value, String> {
        let pattern_name = match args.first() {
            Some(Value::String(s)) => s.clone(),
            Some(other) => {
                return Err(format!(
                    "inspect() expected String pattern name, got {}",
                    other.type_name()
                ))
            }
            None => return Err("inspect() requires 1 argument (pattern name)".to_string()),
        };

        // Soft-failure: nonexistent pattern → Value::Unit
        let is_learnable = self.learnable_patterns.contains_key(&pattern_name);
        let is_regular = self.patterns.contains_key(&pattern_name);
        if !is_learnable && !is_regular {
            return Ok(Value::Unit);
        }

        // Look up stats
        let stats = match self.pattern_stats.lock() {
            Ok(stats) => stats
                .get(&pattern_name)
                .cloned()
                .unwrap_or_else(PatternStats::new),
            Err(_) => PatternStats::new(),
        };

        // If the pattern exists in learnable_patterns, get its current few_shot count
        // (which may differ from examples_count if few-shot was added outside adapt)
        let actual_examples = self
            .learnable_patterns
            .get(&pattern_name)
            .map(|lp| lp.few_shot.len() as u64)
            .unwrap_or(stats.examples_count);

        let cache_misses = stats.calls.saturating_sub(stats.cache_hits);

        let mut fields = std::collections::HashMap::new();
        fields.insert("calls".to_string(), Value::Float(stats.calls as f64));
        fields.insert(
            "avg_confidence".to_string(),
            Value::Float(stats.avg_confidence()),
        );
        fields.insert(
            "cache_hits".to_string(),
            Value::Float(stats.cache_hits as f64),
        );
        fields.insert(
            "cache_misses".to_string(),
            Value::Float(cache_misses as f64),
        );
        fields.insert(
            "last_adapt".to_string(),
            Value::Float(stats.last_adapt as f64),
        );
        fields.insert(
            "last_call".to_string(),
            Value::Float(stats.last_call as f64),
        );
        fields.insert(
            "examples_count".to_string(),
            Value::Float(actual_examples as f64),
        );
        fields.insert(
            "is_learnable".to_string(),
            Value::Float(if is_learnable { 1.0 } else { 0.0 }),
        );

        Ok(Value::Struct {
            type_name: "PatternStats".to_string(),
            fields,
        })
    }

    /// Public helper: inspect a pattern by name, returning its stats as a Value.
    /// Returns Value::Unit for nonexistent patterns (soft-failure).
    /// Used by contract tests and may be exposed as a library API in the future.
    pub fn inspect_pattern(&self, name: &str) -> Value {
        self.invoke_inspect(&[Value::String(name.to_string())])
            .unwrap_or(Value::Unit)
    }

    /// Call a user-defined pattern by name with given arguments.
    /// Used by the server scheduler (cron dispatch) and future webhook handlers.
    /// Returns Err if pattern not found or arity mismatch.
    pub fn call_pattern(&self, name: &str, args: &[Value]) -> Result<Value, String> {
        // Check learnable patterns first
        if let Some(learnable) = self.learnable_patterns.get(name).cloned() {
            let collapsed_args = self.collapse_args(&learnable.params, args);
            let learnable_clone = learnable.clone();
            return self.invoke_pattern_with_hooks(name, &collapsed_args, || {
                self.invoke_learnable_with_env(name, &learnable_clone, &collapsed_args)
            });
        }
        // Regular pattern
        let pattern = match self.patterns.get(name) {
            Some(p) => p.clone(),
            None => return Err(format!("call_pattern: unknown pattern '{}'", name)),
        };
        if args.len() != pattern.params.len() {
            return Err(format!(
                "call_pattern: '{}' expects {} args, got {}",
                name,
                pattern.params.len(),
                args.len()
            ));
        }
        let mut local_env = self.bind_and_collapse(&pattern.params, args)?;
        self.invoke_pattern_with_hooks(name, args, || {
            self.eval_statements(&pattern.body, &mut local_env)
        })
    }
}
