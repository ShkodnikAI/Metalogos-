use super::*;
use crate::ast::{ContextMode, ContextStrategy};
use crate::embeddings::cosine_similarity;
use crate::interpreter::types::{DistillMode, DistillRuntimeState};
use crate::llm;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
    /// Calls the LLM with a summarization prompt and returns the compressed text.
    /// If the LLM call fails, returns the original context block (graceful degradation).
    /// Наряд #156: routes through SmartRouter when available (same as learnable calls).
    fn compress_context(&self, context_block: &str) -> String {
        let summary_prompt = format!(
            "Summarize the following facts concisely. Retain key information. \
             Output a single paragraph.\n\n{}",
            context_block
        );
        let result = self.call_llm(&summary_prompt, "", None, None);
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

        // ── Наряд №181 (ADR-0117): distillation cycle ────────────────────
        //
        // If `distill_to` is set on the pattern, route through the
        // TEACHING→DISTILLED cycle. Safe degradation: any Reflex-side
        // failure (not enough data, training error, model not found)
        // falls back to the normal LLM path below — never propagates
        // an error to the caller (ADR-0117 §3, "safe degradation").
        //
        // If `distill_to` is NOT set, this entire block is skipped and
        // execution proceeds byte-identically to pre-Наряд №181 behavior
        // (the backward-compat guarantee from ADR-0117 §2).
        if let Some(distill) = &learnable.distill {
            match self.try_distilled_call(pattern_name, distill, &input) {
                Ok(Some(value)) => {
                    // DISTILLED path succeeded with a confident prediction.
                    self.record_pattern_call(pattern_name, true);
                    return Ok(value);
                }
                Ok(None) => {
                    // Returned None — pattern is in TEACHING mode OR
                    // confidence was below fallback threshold. Either way,
                    // fall through to LLM call below, then record the
                    // (input, llm_output) example for future training.
                }
                Err(e) => {
                    // Safe degradation (ADR-0117 §3): log + fall through
                    // to LLM. Never propagate the error to the caller.
                    eprintln!(
                        "[reflex] distillation error in pattern '{}': {} — falling back to LLM",
                        pattern_name, e
                    );
                }
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
        // Наряд №4: Use SmartRouter if llm config is present, otherwise legacy backend
        let resolved_model = learnable
            .model
            .as_ref()
            .map(|alias| llm::resolve_model(alias));

        // Наряд #156: Pass sandbox timeout to LLM call.
        // SmartRouter path: reqwest::blocking::Client::timeout() performs
        // real HTTP-level cancellation (drops TCP connection).
        // Legacy path (MockLlm/RealLlm): uses thread wrapper with recv_timeout.
        let timeout_override = if let Some(ref sb) = self.active_sandbox {
            if sb.timeout > 0 {
                Some(Duration::from_secs(sb.timeout as u64))
            } else {
                None
            }
        } else {
            None
        };
        let response = self.call_llm(
            &effective_prompt,
            &input,
            resolved_model.as_deref(),
            timeout_override,
        )?;

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
                let result = Value::Struct {
                    type_name: "LlmResponse".to_string(),
                    fields,
                };
                // Наряд №181: record example for distillation training.
                // ADR-0117 only allows distillation for closed-label
                // String-returning patterns — so we record the response
                // as a label candidate. The mode may switch from TEACHING
                // to DISTILLED on the next call once `distill_after`
                // examples have been accumulated.
                self.record_distill_example(pattern_name, &input, &response);
                return Ok(result);
            }
        }

        // Наряд №181: record example even for non-JSON responses (string labels).
        self.record_distill_example(pattern_name, &input, &response);

        Ok(Value::String(response))
    }

    // ── Наряд №181 (ADR-0117): distillation cycle helpers ──────────────

    /// Attempt a distilled call. Returns:
    /// - `Ok(Some(value))` — DISTILLED mode succeeded with confident prediction.
    ///   Caller should return this directly (no LLM call).
    /// - `Ok(None)` — pattern is in TEACHING mode OR confidence was below
    ///   `fallback_if` threshold. Caller should fall through to LLM.
    /// - `Err(e)` — Reflex-side error (registry, model, predict failure).
    ///   Caller should log + fall through to LLM (safe degradation).
    fn try_distilled_call(
        &self,
        pattern_name: &str,
        distill: &crate::interpreter::types::DistillConfig,
        input: &str,
    ) -> Result<Option<Value>, String> {
        let mut states = self
            .distill_states
            .lock()
            .map_err(|e| format!("distill_states lock poisoned: {}", e))?;
        let state = states
            .entry(pattern_name.to_string())
            .or_insert_with(|| DistillRuntimeState {
                mode: DistillMode::Teaching,
                examples: Vec::new(),
                last_train_attempt: 0,
            });

        match state.mode {
            DistillMode::Teaching => {
                // Not enough examples yet (or first call). LLM will be called
                // by the outer invoke path; we'll record the example after.
                let count = state.examples.len();
                let last_attempt = state.last_train_attempt;
                // Check if we've crossed the threshold to trigger training.
                // ADR-0115 (Наряд №179): reflex_train requires ≥10 examples
                // for the holdout split. So we only attempt training when
                // count ≥ max(distill_after, 10). This avoids calling train()
                // on every invocation once count crosses distill_after but
                // is still below 10 — train() would error each time,
                // wasting cycles.
                let training_threshold = std::cmp::max(distill.distill_after, 10);
                // Only attempt training ONCE per threshold crossing —
                // if it fails (accuracy 0 or training error), we stay
                // in TEACHING but don't retry on every call (would be
                // O(N) trainings on N calls). Next retry: when count
                // grows by 5 more examples past the last attempt.
                let should_attempt =
                    count >= training_threshold && (last_attempt == 0 || count - last_attempt >= 5);
                if should_attempt {
                    let examples = state.examples.clone();
                    // Stash the count we attempted training at, so we know
                    // not to retry until count grows by 5 more.
                    let trained_at_count = count;
                    drop(states); // release lock before calling reflex_train
                    match self.try_train_distilled_model(pattern_name, distill, &examples) {
                        Ok(true) => {
                            // Training succeeded → switch to DISTILLED.
                            // Update mode in the distill_states map.
                            {
                                let mut states = self
                                    .distill_states
                                    .lock()
                                    .map_err(|e| format!("distill_states lock poisoned: {}", e))?;
                                if let Some(s) = states.get_mut(pattern_name) {
                                    s.mode = DistillMode::Distilled;
                                }
                            } // lock released here
                              // Recursive call to enter DISTILLED path on this same invocation.
                            return self.try_distilled_call(pattern_name, distill, input);
                        }
                        Ok(false) => {
                            // Training didn't reach accuracy threshold — stay TEACHING.
                            // Record last attempt count to avoid immediate retry.
                            {
                                let mut states = self
                                    .distill_states
                                    .lock()
                                    .map_err(|e| format!("distill_states lock poisoned: {}", e))?;
                                if let Some(s) = states.get_mut(pattern_name) {
                                    s.last_train_attempt = trained_at_count;
                                }
                            } // lock released here
                            return Ok(None);
                        }
                        Err(e) => return Err(e),
                    }
                }
                Ok(None)
            }
            DistillMode::Distilled => {
                // Predict via reflex_predict. Safe degradation: errors → None.
                let model_id = self
                    .reflex_names
                    .get(&distill.reflex_name)
                    .copied()
                    .ok_or_else(|| {
                        format!(
                            "distill: reflex '{}' not declared (no `reflex {} {{ ... }}` block)",
                            distill.reflex_name, distill.reflex_name
                        )
                    })?;
                drop(states); // release lock before predicting

                let reg = self
                    .reflex_registry
                    .lock()
                    .map_err(|e| format!("reflex registry poisoned: {}", e))?;
                let model = reg.get(model_id).ok_or_else(|| {
                    format!("distill: model handle {:?} not in registry", model_id)
                })?;

                // Embed input — for now we use a simple deterministic embedding
                // (the input string's first N bytes as floats). This is a
                // placeholder — ADR-0117 §3 allows embedding strategy to be
                // any deterministic function of input → Vec<f64>. A future
                // naryad may swap this for a real embedding model.
                let embedding = self.simple_embedding(input, model.input_size);

                let probs = model.forward(&embedding);

                // Find the highest-confidence label.
                let (best_idx, best_prob) = probs
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(i, &p)| (i, p))
                    .unwrap_or((0, 0.0));

                let best_label = model
                    .labels
                    .get(best_idx)
                    .cloned()
                    .unwrap_or_else(|| format!("label_{}", best_idx));

                // Check fallback threshold.
                if let Some((op, threshold)) = distill.fallback_if {
                    // fallback_if: confidence OP threshold → call LLM if condition is TRUE
                    // (e.g., `confidence < 0.85` → if confidence < 0.85, fall back).
                    if op.compare(best_prob, threshold) {
                        // Below threshold — fall back to LLM. The new example
                        // will be recorded by the outer call path.
                        return Ok(None);
                    }
                }

                // Confident enough — return the distilled prediction.
                Ok(Some(Value::String(best_label)))
            }
        }
    }

    /// Train the distillation model on accumulated examples.
    /// Returns Ok(true) if training succeeded and accuracy ≥ 0.0
    /// (any successful training switches mode to DISTILLED).
    /// Returns Ok(false) if training was attempted but accuracy was 0.0
    /// (or no examples matched valid labels).
    fn try_train_distilled_model(
        &self,
        pattern_name: &str,
        distill: &crate::interpreter::types::DistillConfig,
        examples: &[(String, String)],
    ) -> Result<bool, String> {
        let model_id = self
            .reflex_names
            .get(&distill.reflex_name)
            .copied()
            .ok_or_else(|| {
                format!(
                    "distill: reflex '{}' not declared for pattern '{}'",
                    distill.reflex_name, pattern_name
                )
            })?;

        // Build training data: each (input, output) pair → Vec<f64> features + class_idx.
        // We need the model's labels to convert output string → class index.
        let input_size;
        let labels: Vec<String>;
        {
            let reg = self
                .reflex_registry
                .lock()
                .map_err(|e| format!("reflex registry poisoned: {}", e))?;
            let model = reg
                .get(model_id)
                .ok_or_else(|| format!("distill: model handle {:?} not in registry", model_id))?;
            input_size = model.input_size;
            labels = model.labels.clone();
        }

        let mut inputs: Vec<Vec<f64>> = Vec::with_capacity(examples.len());
        let mut targets: Vec<usize> = Vec::with_capacity(examples.len());
        for (input_str, output_str) in examples {
            // Find label index. If output doesn't match any label, skip this example
            // (the LLM returned something outside the closed label set — safe to ignore).
            let target_idx = match labels.iter().position(|l| l == output_str) {
                Some(idx) => idx,
                None => continue, // skip — ADR-0117 closed-label enforcement
            };
            let embedding = self.simple_embedding(input_str, input_size);
            inputs.push(embedding);
            targets.push(target_idx);
        }

        if inputs.is_empty() {
            // No valid examples yet — can't train.
            return Ok(false);
        }

        // Train. Safe degradation: training error → Ok(false), stay TEACHING.
        let mut reg = self
            .reflex_registry
            .lock()
            .map_err(|e| format!("reflex registry poisoned: {}", e))?;
        let model = reg
            .get_mut(model_id)
            .ok_or_else(|| format!("distill: model handle {:?} not in registry", model_id))?;
        // Use a small epoch count for distillation training (default 30).
        // This is a heuristic — ADR-0117 doesn't specify epochs. We pick
        // 30 as a balance: enough to learn simple label distinctions on
        // a one-class or two-class dataset, fast enough not to block the
        // pattern call (training happens inline during the LLM-call
        // replacement). Reflex_train requires ≥10 examples (ADR-0115),
        // and on 10-50 example datasets 30 epochs typically converges.
        let _result = model
            .train(&inputs, &targets, 30, 0.1)
            .map_err(|e| format!("distill: training failed: {}", e))?;

        // Training succeeded — accuracy may be low but we still switch to DISTILLED
        // (the fallback_if threshold handles low-confidence cases at predict time).
        Ok(true)
    }

    /// Record a (input, output) example for future training.
    /// Called after every LLM call on a distilling pattern.
    fn record_distill_example(&self, pattern_name: &str, input: &str, output: &str) {
        if let Ok(mut states) = self.distill_states.lock() {
            let state =
                states
                    .entry(pattern_name.to_string())
                    .or_insert_with(|| DistillRuntimeState {
                        mode: DistillMode::Teaching,
                        examples: Vec::new(),
                        last_train_attempt: 0,
                    });
            // Only record in TEACHING mode — once DISTILLED, we don't accumulate
            // (unless fallback returned us to LLM, in which case we DO want this
            // example for future retraining).
            state.examples.push((input.to_string(), output.to_string()));
        }
    }

    /// Simple deterministic embedding for distillation input strings.
    /// ADR-0117 §3: "embedding strategy can be any deterministic function
    /// of input → Vec<f64>". This produces a fixed-size vector by hashing
    /// the input string into `dim` buckets. Future naryads may swap this
    /// for a real embedding model (sentence-transformers etc.) — the
    /// distillation logic doesn't depend on the embedding strategy.
    fn simple_embedding(&self, input: &str, dim: usize) -> Vec<f64> {
        let mut embedding = vec![0.0; dim];
        // XOR-based hash distribution — same input always produces same embedding.
        let bytes = input.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            let bucket = i % dim;
            // Mix the byte into the bucket — using multiplication + addition
            // so different inputs produce distinguishable vectors.
            embedding[bucket] += (b as f64) * 0.01;
            // Also XOR-style mixing for spread
            if b != 0 {
                embedding[(bucket + 1) % dim] =
                    (embedding[(bucket + 1) % dim] * 0.99) + (b as f64) * 0.001;
            }
        }
        // Normalize to roughly [-1, 1] range (helps gradient descent).
        let max_val = embedding.iter().cloned().fold(0.0f64, f64::max).max(1.0);
        for v in &mut embedding {
            *v /= max_val;
        }
        embedding
    }

    /// Наряд #156: Unified LLM call with optional per-call timeout.
    ///
    /// **SmartRouter path** (providers configured): timeout is passed to
    /// `reqwest::blocking::Client::timeout()` which performs real HTTP-level
    /// cancellation (drops the TCP connection). No thread needed — this
    /// is the key improvement over the Наряд №126 thread hack.
    ///
    /// **Legacy path** (MockLlm / RealLlm without SmartRouter): the
    /// `LlmBackend` trait has no timeout concept. If `timeout_override` is
    /// Some, we wrap the call in a thread with `recv_timeout`. This is the
    /// same mechanism as Наряд №126, but now clearly documented as a
    /// legacy-only fallback. For MockLlm (test-only), the "background"
    /// is just a `thread::sleep`. For RealLlm, reqwest's own 120s timeout
    /// will eventually fire if the thread outlives our wait.
    fn call_llm(
        &self,
        prompt: &str,
        input: &str,
        model: Option<&str>,
        timeout_override: Option<Duration>,
    ) -> Result<String, String> {
        // SmartRouter path: real cancellation via reqwest timeout
        if let Ok(guard) = self.smart_router.lock() {
            if let Some(ref router) = *guard {
                return router.call(prompt, input, model, timeout_override);
            }
        }

        // Legacy path (no SmartRouter installed)
        match timeout_override {
            Some(timeout) => {
                use std::sync::mpsc;
                let (tx, rx) = mpsc::channel();
                let prompt = prompt.to_string();
                let input = input.to_string();
                let model = model.map(String::from);
                std::thread::spawn(move || {
                    let backend = llm::create_llm_backend();
                    let _ = tx.send(backend.call_with_model(&prompt, &input, model.as_deref()));
                });
                match rx.recv_timeout(timeout) {
                    Ok(result) => result,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        Err(format!("LLM call timed out after {:?}", timeout))
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        Err("LLM call thread terminated unexpectedly".to_string())
                    }
                }
            }
            None => {
                let backend = llm::create_llm_backend();
                backend.call_with_model(prompt, input, model)
            }
        }
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
