// ── Execution engine: run, invoke, eval ─────────────────────────

use super::*;

impl Interpreter {
    /// Run a complete .mlog program.
    pub fn run(&mut self, declarations: Vec<Declaration>) -> Result<Option<String>, String> {
        let mut output: Option<String> = None;

        // Наряд №119: build type alias map for runtime resolution
        let (type_alias_map, alias_errors) = crate::ast::build_type_alias_map(&declarations);
        if !alias_errors.is_empty() {
            return Err(alias_errors.join("\n"));
        }

        // O-2: Fire on_session_start hooks (first pass: register them, then fire)
        // We do a two-phase approach: first collect all declarations to register hooks,
        // then fire session_start, then process remaining declarations.
        // Simpler approach: fire session_start hooks AFTER all declarations are registered.
        // We split: first pass registers all hooks, second pass fires session_start then processes the rest.
        let mut remaining_decls = Vec::new();
        for decl in &declarations {
            match decl {
                Declaration::Hook(h) => match h.phase {
                    HookPhase::OnSessionStart => self.hooks_session_start.push(h.clone()),
                    HookPhase::OnWrite => self.hooks_on_write.push(h.clone()),
                    HookPhase::OnSessionEnd => self.hooks_session_end.push(h.clone()),
                    HookPhase::BeforePattern => self.hooks_before.push(h.clone()),
                    HookPhase::AfterPattern => self.hooks_after.push(h.clone()),
                },
                _ => remaining_decls.push(decl.clone()),
            }
        }

        // Fire on_session_start hooks
        for hook in &self.hooks_session_start {
            let mut hook_env = HashMap::new();
            let _ = self.eval_statements(&hook.body, &mut hook_env);
        }

        for decl in remaining_decls {
            match decl {
                Declaration::Import(import) => {
                    self.handle_import(&import)?;
                }
                Declaration::EntityType(e) => {
                    self.struct_types.insert(
                        e.name.clone(),
                        StructType {
                            name: e.name.clone(),
                            fields: e.fields,
                        },
                    );
                }
                Declaration::EntityRecord(e) => {
                    let resolved_type = type_alias_map
                        .get(&e.type_name)
                        .map(|s| s.as_str())
                        .unwrap_or(&e.type_name);
                    let value = self.instantiate_struct(resolved_type, &e.fields)?;
                    self.variables.insert(e.name.clone(), value);
                }
                Declaration::EntitySimple(e) => {
                    let value = self.eval_expr(&e.value)?;
                    // Наряд №114: apply declared opaque types (esp. Secret)
                    // Наряд №119: resolve type alias before coercion
                    let resolved_type = type_alias_map
                        .get(&e.type_name)
                        .map(|s| s.as_str())
                        .unwrap_or(&e.type_name);
                    let value = Self::coerce_to_declared_type(value, resolved_type)?;
                    self.variables.insert(e.name.clone(), value);
                }
                Declaration::Rule(r) => {
                    self.rules.push(r);
                }
                Declaration::Memorize(m) => {
                    let value_str = match self.eval_expr(&m.value)? {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    let embedding = self.embedding_manager.embed(&value_str).unwrap_or_default();
                    let _ = lock_or_err(self.memory.lock())?.memorize(MemoryEntry {
                        id: None,
                        value: value_str.clone(),
                        priority: m.priority,
                        timestamp: now,
                        decay_rate: 0.01,
                        confidence: m.priority,
                        embedding,
                        mem_type: String::new(),
                    });
                    // ADR-0052: emit memory_store event
                    let preview = if value_str.len() > 30 {
                        &value_str[..30]
                    } else {
                        &value_str
                    };
                    let mut data = HashMap::new();
                    data.insert("key_preview".to_string(), preview.to_string());
                    data.insert("priority".to_string(), m.priority.to_string());
                    self.emit_event("memory_store", "system", data, None);
                }
                Declaration::Forget(f) => {
                    let query_str = match self.eval_expr(&f.query)? {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    let cutoff = now - (f.days * 86400);
                    lock_or_err(self.memory.lock())?.forget(&query_str, cutoff);
                }
                Declaration::Pattern(p) => {
                    self.patterns.insert(
                        p.name.clone(),
                        CompiledPattern {
                            params: p.params.clone(),
                            body: p.body.clone(),
                        },
                    );
                }
                Declaration::LearnablePattern(lp) => {
                    self.learnable_patterns.insert(
                        lp.name.clone(),
                        CompiledLearnable {
                            params: lp.params.clone(),
                            prompt: lp.prompt.clone(),
                            few_shot: Vec::new(),
                            context: lp.context.clone(),
                            context_strategy: lp.context_strategy.clone(),
                            max_context_tokens: lp.max_context_tokens,
                            max_tokens: lp.max_tokens,
                            cache: lp.cache,
                            cache_ttl: lp.cache_ttl,
                            model: lp.model.clone(),
                            conversation: lp.conversation.clone(),
                        },
                    );
                }
                Declaration::Adapt(a) => {
                    let input_str = match self.eval_expr(&a.input_example)? {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let output_str = match self.eval_expr(&a.output_example)? {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    if let Some(learnable) = self.learnable_patterns.get_mut(&a.pattern_name) {
                        learnable
                            .few_shot
                            .push((input_str.clone(), output_str.clone()));
                    } else {
                        return Err(format!(
                            "adapt: learnable pattern '{}' not found",
                            a.pattern_name
                        ));
                    }
                    // ADR-0051: update pattern stats for inspect()
                    {
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0);
                        let mut stats = lock_or_err(self.pattern_stats.lock())?;
                        let entry = stats
                            .entry(a.pattern_name.clone())
                            .or_insert_with(PatternStats::new);
                        entry.last_adapt = now;
                        entry.examples_count += 1;
                    }
                    // Phase 7.5: Audit log for adapt operations
                    self.push_audit(format!(
                        "[AUDIT] adapt {}: {} -> {}",
                        a.pattern_name, input_str, output_str
                    ));
                    // ADR-0052: emit adapt event
                    let examples_count = self
                        .learnable_patterns
                        .get(&a.pattern_name)
                        .map(|lp| lp.few_shot.len())
                        .unwrap_or(0);
                    let mut data = HashMap::new();
                    data.insert("pattern".to_string(), a.pattern_name.clone());
                    data.insert("action".to_string(), "add_example".to_string());
                    data.insert("examples_count".to_string(), examples_count.to_string());
                    self.emit_event("adapt", &a.pattern_name, data, None);
                }
                Declaration::Fluid(fl) => {
                    let mut variants = Vec::new();
                    for v in &fl.variants {
                        let val = self.eval_expr(&v.value)?;
                        variants.push(FluidValueVariant {
                            type_name: v.type_name.clone(),
                            value: val,
                            confidence: v.confidence,
                        });
                    }
                    self.variables
                        .insert(fl.name.clone(), Value::Fluid(variants));
                }
                Declaration::Relate(r) => {
                    let from_str = match self.eval_expr(&r.from)? {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let to_str = match self.eval_expr(&r.to)? {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let _ =
                        lock_or_err(self.kg.lock())?.relate(&from_str, &to_str, &r.relation, 1.0);
                }
                Declaration::Sandbox(s) => {
                    self.sandboxes.insert(s.name.clone(), s);
                }
                Declaration::Hook(h) => {
                    // ADR-0045 + O-2: register hooks in declaration order by phase
                    match h.phase {
                        HookPhase::BeforePattern => self.hooks_before.push(h),
                        HookPhase::AfterPattern => self.hooks_after.push(h),
                        HookPhase::OnSessionStart => self.hooks_session_start.push(h),
                        HookPhase::OnWrite => self.hooks_on_write.push(h),
                        HookPhase::OnSessionEnd => self.hooks_session_end.push(h),
                    }
                }
                Declaration::Mutate(m) => {
                    let msg = self.handle_mutate(&m)?;
                    self.mutate_log.push(msg);
                }
                Declaration::Eval(e) => {
                    // ADR-0050: store eval block for later execution by run_eval_blocks()
                    self.eval_blocks.push(e);
                }
                Declaration::Flow(f) => {
                    // Execute rules before flow (they modify entity state)
                    self.execute_rules()?;
                    output = Some(self.run_flow(&f)?);
                }
                Declaration::MlogServer(srv) => {
                    self.server_config = Some(srv);
                }
                Declaration::Template(t) => {
                    // Наряд №115: also register in global map for builtin_render (TW/VM parity)
                    let param_names: Vec<String> =
                        t.params.iter().map(|p| p.name.clone()).collect();
                    crate::builtins::http::register_template(&t.name, &t.body, param_names);
                    self.templates.insert(t.name.clone(), t);
                }
                Declaration::Memory(m) => {
                    self.configure_memory(&m);
                }
                Declaration::Conversation(c) => {
                    // ADR-0053: store conversation configuration
                    self.conversation_config = ConversationConfig {
                        ttl: c.ttl,
                        max_messages: c.max_messages,
                        compress_after: c.compress_after,
                    };
                }
                Declaration::ContextBudget(b) => {
                    // sqz-inspired P3: store token budget for a learnable pattern
                    self.context_budgets.insert(b.pattern_name.clone(), b.limit);
                }
                Declaration::LlmConfig(config) => {
                    // Наряд №4: store LLM config and create smart router
                    let router = llm::SmartRouter::from_config(&config);
                    // Наряд №4: install into global bridge for builtin_call_llm()
                    llm::set_global_smart_router(llm::SmartRouter::from_config(&config));
                    if let Ok(mut sr) = self.smart_router.lock() {
                        *sr = Some(router);
                    }
                    self.llm_config = Some(config);
                }
                Declaration::Tool(t) => {
                    // ADR-0054: register tool as a module namespace and
                    // compile each method as a qualified pattern.
                    // Tool methods are stored as "toolname.methodname" to
                    // avoid namespace collisions between tools with same method names.
                    // tool.method(args) resolves via QualifiedCall.
                    self.module_namespaces
                        .insert(t.name.clone(), format!("tool:{}", t.name));
                    for method in &t.methods {
                        let qualified_name = format!("{}.{}", t.name, method.name);
                        self.patterns.insert(
                            qualified_name,
                            CompiledPattern {
                                params: method.params.clone(),
                                body: method.body.clone(),
                            },
                        );
                    }
                }
                Declaration::Db(db) => {
                    self.db_config = Some(db.clone());
                    self.init_db_connection(&db);
                }
                Declaration::Schema(schema) => {
                    self.apply_schema(&schema)?;
                }
                Declaration::SkillIndex(idx) => {
                    self.skill_indices.insert(idx.name.clone(), idx);
                }
                // Наряд №119: type aliases handled via type_alias_map (built above)
                Declaration::TypeAlias(_) => {}
            }
        }

        // O-2: Fire on_session_end hooks
        for hook in &self.hooks_session_end {
            let mut hook_env = HashMap::new();
            let _ = self.eval_statements(&hook.body, &mut hook_env);
        }

        Ok(output)
    }

    /// Invoke a pattern or built-in by name with given arguments.
    /// ADR-0045: Hooks (before_pattern / after_pattern) fire around pattern
    /// and learnable pattern invocations, but NOT around builtin calls.
    pub(super) fn invoke(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        // Check recall (memory) first — it's a built-in with memory access
        if name == "recall" {
            return self.invoke_recall(args);
        }

        // ADR-0045/Phase 7.1: server-context builtins (flow step dispatch)
        if name == "query_param" {
            let param_name = args
                .first()
                .and_then(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            if let Some(val) = self.get_server_query_param(&param_name) {
                return Ok(Value::String(val));
            }
            return Ok(Value::String(String::new()));
        }

        // memorize() — callable form (flow step context)
        if name == "memorize" {
            return self.invoke_memorize_fn(args);
        }

        // recall_top_k() — hybrid FTS5 BM25 + cosine RRF search (flow step context)
        if name == "recall_top_k" {
            return self.invoke_recall_top_k_fn(args);
        }

        // forget() — callable form (flow step context)
        if name == "forget" {
            return self.invoke_forget_fn(args);
        }

        // Наряд №67: recipe_save — intercept to also memorize for recipe_search
        if name == "recipe_save" {
            return self.invoke_recipe_save_fn(args);
        }

        // Наряд №67: recipe_search — semantic search via recall_top_k + kv_get
        if name == "recipe_search" {
            return self.invoke_recipe_search_fn(args);
        }

        if name == "find" {
            return self.invoke_find(args);
        }

        // Problem A: resolve_skill_index(dept) — main invoke path
        if name == "resolve_skill_index" {
            let dept = match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => {
                    return Err(
                        "resolve_skill_index() expects a department name (String)".to_string()
                    )
                }
            };
            let idx = self.skill_indices.get(&dept).ok_or_else(|| {
                format!(
                    "resolve_skill_index(): no skill_index declared for '{}'",
                    dept
                )
            })?;
            let mut fields = HashMap::new();
            let tier1: Vec<Value> = idx
                .tiers
                .iter()
                .filter(|t| t.mode == "always")
                .flat_map(|t| t.skills.iter().map(|s| Value::String(s.clone())))
                .collect();
            fields.insert("tier1".to_string(), Value::List(tier1));
            for tier in &idx.tiers {
                if tier.mode == "when_matches" {
                    let rules: Vec<Value> = tier
                        .rules
                        .iter()
                        .map(|r| {
                            let mut f = HashMap::new();
                            f.insert("skill".to_string(), Value::String(r.skill.clone()));
                            f.insert(
                                "triggers".to_string(),
                                Value::List(
                                    r.triggers
                                        .iter()
                                        .map(|t| Value::String(t.clone()))
                                        .collect(),
                                ),
                            );
                            Value::Struct {
                                type_name: "TriggerRule".to_string(),
                                fields: f,
                            }
                        })
                        .collect();
                    fields.insert(format!("tier{}", tier.level), Value::List(rules));
                }
            }
            if let Some(b) = idx.budget {
                fields.insert("budget".to_string(), Value::Float(b));
            }
            if let Some(ref t) = idx.truncation {
                let mode_str = match t {
                    TruncationMode::WholeSkillOnly => "whole_skill_only",
                    TruncationMode::TruncateAtBoundary => "truncate_at_boundary",
                };
                fields.insert(
                    "truncation".to_string(),
                    Value::String(mode_str.to_string()),
                );
            }
            return Ok(Value::Struct {
                type_name: format!("SkillIndex_{}", dept),
                fields,
            });
        }

        // Problem A: fit_to_budget(list) — MVP: return list as-is
        if name == "fit_to_budget" {
            let list = match args.first() {
                Some(Value::List(items)) => items.clone(),
                _ => return Err("fit_to_budget() expects first argument to be a List".to_string()),
            };
            return Ok(Value::List(list));
        }

        // Check learnable patterns
        if let Some(learnable) = self.learnable_patterns.get(name).cloned() {
            // ADR-0089: reset propagated confidence before collapse
            *self
                .propagated_confidence
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = 1.0;
            let collapsed_args = self.collapse_args(&learnable.params, &args);
            let learnable_clone = learnable.clone();
            let result = self.invoke_pattern_with_hooks(name, &collapsed_args, || {
                self.invoke_learnable_with_env(name, &learnable_clone, &collapsed_args)
            });
            // ADR-0089: wrap result as Fluid if confidence was propagated
            let conf = *self
                .propagated_confidence
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            return Self::maybe_wrap_with_confidence(result, conf);
        }

        // Check builtins
        if let Some(builtin_fn) = self.builtins.get(name) {
            // Phase 7.5: Sandbox enforcement — filesystem isolation
            if let Some(ref sb) = self.active_sandbox {
                if sb.forbidden.iter().any(|f| f == "filesystem") {
                    if matches!(
                        name,
                        "read_file"
                            | "write_file"
                            | "append_file"
                            | "delete_file"
                            | "file_exists"
                            | "list_dir"
                    ) {
                        return Err(format!(
                            "filesystem access forbidden in sandbox '{}'",
                            sb.name
                        ));
                    }
                    // Наряд №17 Г.2: also enforce exec() in sandbox
                    if name == "exec" {
                        return Err(format!("exec() forbidden in sandbox '{}'", sb.name));
                    }
                }
            }
            // O-2: Fire on_write hooks before mutating builtins
            if Self::is_write_builtin(name) {
                self.fire_on_write_hooks(name, &args);
            }
            return builtin_fn(&args);
        }

        // Look up compiled pattern
        let pattern = match self.patterns.get(name) {
            Some(p) => p.clone(),
            None => {
                return Ok(Value::String(format!(
                    "[ERROR: unknown function '{}']",
                    name
                )))
            }
        };

        if args.len() != pattern.params.len() {
            return Err(format!(
                "pattern {} expects {} arguments, got {}",
                name,
                pattern.params.len(),
                args.len()
            ));
        }

        // Bind parameters with Fluid collapse
        // ADR-0089: reset propagated confidence before collapse
        *self
            .propagated_confidence
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = 1.0;
        let mut local_env = self.bind_and_collapse(&pattern.params, &args)?;

        let result = self.invoke_pattern_with_hooks(name, &args, || {
            self.eval_statements(&pattern.body, &mut local_env)
        });
        // ADR-0089: wrap result as Fluid if confidence was propagated
        let conf = *self
            .propagated_confidence
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        Self::maybe_wrap_with_confidence(result, conf)
    }

    /// Safety limit for while loops (soft-failure on exceed).
    const WHILE_SAFETY_LIMIT: u64 = 100_000;
    /// Phase 7.5: Sandbox iteration limit (10,000 for both while and each).
    const SANDBOX_ITER_LIMIT: u64 = 10_000;
    /// Наряд №17 В.4: Maximum string length to prevent memory exhaustion.
    /// Applied to builtin string operations (concat, replace, split, etc.).
    const MAX_STRING_LENGTH: usize = 1_000_000; // 1 MB

    /// Наряд №14: Compare two Values using a CompareOp.
    /// Used by match statement's Compare arm.
    fn compare_values(left: &Value, op: &CompareOp, right: &Value) -> Result<bool, String> {
        // Try numeric comparison first
        let left_f = left.as_float().ok();
        let right_f = right.as_float().ok();
        if let (Some(lf), Some(rf)) = (left_f, right_f) {
            return Ok(match op {
                CompareOp::Gt => lf > rf,
                CompareOp::Lt => lf < rf,
                CompareOp::Ge => lf >= rf,
                CompareOp::Le => lf <= rf,
                CompareOp::Eq => lf == rf,
                CompareOp::Ne => lf != rf,
            });
        }
        // Fall back to string comparison
        let ls = format!("{}", left);
        let rs = format!("{}", right);
        Ok(match op {
            CompareOp::Eq => ls == rs,
            CompareOp::Ne => ls != rs,
            CompareOp::Gt => ls > rs,
            CompareOp::Lt => ls < rs,
            CompareOp::Ge => ls >= rs,
            CompareOp::Le => ls <= rs,
        })
    }

    pub(crate) fn eval_statements(
        &self,
        stmts: &[Statement],
        env: &mut HashMap<String, Value>,
    ) -> Result<Value, String> {
        let mut mutable_vars: std::collections::HashSet<String> = std::collections::HashSet::new();
        match self.eval_statements_cf(stmts, env, &mut mutable_vars)? {
            ControlFlow::ContinueNormal(v) => Ok(v),
            ControlFlow::Return(v) => Ok(v),
            ControlFlow::Break | ControlFlow::ContinueLoop => {
                // break/continue at top level (not inside a loop) is an error
                Err("break/continue used outside of a loop".to_string())
            }
        }
    }

    /// Internal statement evaluator that returns ControlFlow signals.
    /// This allows break/continue to propagate through nested if/match blocks
    /// up to the nearest each/while loop without being swallowed.
    fn eval_statements_cf(
        &self,
        stmts: &[Statement],
        env: &mut HashMap<String, Value>,
        mutable_vars: &mut std::collections::HashSet<String>,
    ) -> Result<ControlFlow, String> {
        // Phase 7.5: Determine iteration limit based on active sandbox
        let iter_limit = if self.active_sandbox.is_some() {
            Self::SANDBOX_ITER_LIMIT
        } else {
            Self::WHILE_SAFETY_LIMIT
        };

        // Наряд №13: Track the last non-Unit expression value for implicit return.
        let mut last_expr_value = Value::Unit;

        /// Helper: evaluate a sub-block and propagate ControlFlow signals.
        /// Returns Ok(Some(cf)) if the block produced a signal (break/continue/return).
        /// Returns Ok(None) if the block completed normally.
        macro_rules! eval_block {
            ($stmts:expr, $env:expr) => {{
                let cf = self.eval_statements_cf($stmts, $env, mutable_vars)?;
                if cf.is_break() || cf.is_continue() || cf.is_return() {
                    return Ok(cf);
                }
                // Extract the implicit return value
                let v = cf.into_value();
                if !matches!(v, Value::Unit) {
                    last_expr_value = v;
                }
            }};
        }

        for stmt in stmts {
            match stmt {
                Statement::LetBinding {
                    name,
                    value,
                    mutable,
                } => {
                    let val = self.eval_expr_with_env(value, env)?;
                    if *mutable {
                        mutable_vars.insert(name.clone());
                    }
                    env.insert(name.clone(), val);
                }
                Statement::Assign { name, value } => {
                    if !mutable_vars.contains(name) {
                        return Err(format!("cannot assign to immutable variable: {} (use 'let mut {}' to make it mutable)", name, name));
                    }
                    let val = self.eval_expr_with_env(value, env)?;
                    env.insert(name.clone(), val);
                }
                Statement::Each {
                    variable,
                    iterable,
                    body,
                } => {
                    let iter_val = self.eval_expr_with_env(iterable, env)?;
                    let items = match iter_val {
                        Value::List(items) => items,
                        other => {
                            return Err(format!("each: expected List, got {}", other.type_name()))
                        }
                    };
                    if self.active_sandbox.is_some() && (items.len() as u64) > iter_limit {
                        return Err(format!(
                            "iteration limit exceeded in sandbox: each loop has {} items (limit {})",
                            items.len(), iter_limit
                        ));
                    }
                    for (idx, item) in items.into_iter().enumerate() {
                        if self.active_sandbox.is_some() && (idx as u64) >= iter_limit {
                            return Err(format!(
                                "iteration limit exceeded in sandbox: each loop exceeded {} iterations",
                                iter_limit
                            ));
                        }
                        env.insert(variable.clone(), item);
                        let cf = self.eval_statements_cf(body, env, mutable_vars)?;
                        match cf {
                            ControlFlow::Break => break, // absorb Break — loop exits normally
                            ControlFlow::ContinueLoop => continue, // skip to next iteration
                            ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                            ControlFlow::ContinueNormal(v) => {
                                if !matches!(v, Value::Unit) {
                                    return Ok(ControlFlow::Return(v));
                                }
                            }
                        }
                    }
                }
                // Наряд №17.3: each i, item in list { ... }
                Statement::EachWithIndex {
                    index_var,
                    item_var,
                    iterable,
                    body,
                } => {
                    let iter_val = self.eval_expr_with_env(iterable, env)?;
                    let items = match iter_val {
                        Value::List(items) => items,
                        other => {
                            return Err(format!("each: expected List, got {}", other.type_name()))
                        }
                    };
                    if self.active_sandbox.is_some() && (items.len() as u64) > iter_limit {
                        return Err(format!(
                            "iteration limit exceeded in sandbox: each loop has {} items (limit {})",
                            items.len(), iter_limit
                        ));
                    }
                    for (idx, item) in items.into_iter().enumerate() {
                        if self.active_sandbox.is_some() && (idx as u64) >= iter_limit {
                            return Err(format!(
                                "iteration limit exceeded in sandbox: each loop exceeded {} iterations",
                                iter_limit
                            ));
                        }
                        env.insert(index_var.clone(), Value::Float(idx as f64));
                        env.insert(item_var.clone(), item);
                        let cf = self.eval_statements_cf(body, env, mutable_vars)?;
                        match cf {
                            ControlFlow::Break => break, // absorb Break — loop exits normally
                            ControlFlow::ContinueLoop => continue,
                            ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                            ControlFlow::ContinueNormal(v) => {
                                if !matches!(v, Value::Unit) {
                                    return Ok(ControlFlow::Return(v));
                                }
                            }
                        }
                    }
                }
                Statement::While { condition, body } => {
                    let mut iterations: u64 = 0;
                    loop {
                        if iterations >= iter_limit {
                            if self.active_sandbox.is_some() {
                                return Err(format!(
                                    "iteration limit exceeded in sandbox: while loop exceeded {} iterations",
                                    iter_limit
                                ));
                            } else {
                                return Err(format!(
                                    "while loop exceeded safety limit of {} iterations",
                                    Self::WHILE_SAFETY_LIMIT
                                ));
                            }
                        }
                        let cond_val = self.eval_expr_with_env(condition, env)?;
                        if !cond_val.as_bool()? {
                            break;
                        }
                        let cf = self.eval_statements_cf(body, env, mutable_vars)?;
                        match cf {
                            ControlFlow::Break => break,
                            ControlFlow::ContinueLoop => {
                                iterations += 1;
                                continue;
                            }
                            ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                            ControlFlow::ContinueNormal(v) => {
                                if !matches!(v, Value::Unit) {
                                    return Ok(ControlFlow::Return(v));
                                }
                            }
                        }
                        iterations += 1;
                    }
                }
                Statement::IfElseBlock {
                    condition,
                    then_body,
                    else_ifs,
                    else_body,
                } => {
                    let cond_val = self.eval_expr_with_env(condition, env)?;
                    if cond_val.as_bool()? {
                        eval_block!(then_body, env);
                    } else {
                        let mut matched = false;
                        for (ei_cond, ei_body) in else_ifs {
                            let ei_val = self.eval_expr_with_env(ei_cond, env)?;
                            if ei_val.as_bool()? {
                                eval_block!(ei_body, env);
                                matched = true;
                                break;
                            }
                        }
                        if !matched {
                            if let Some(eb) = else_body {
                                eval_block!(eb, env);
                            }
                        }
                    }
                }
                Statement::Return(expr) => {
                    let val = self.eval_expr_with_env(expr, env)?;
                    return Ok(ControlFlow::Return(val));
                }
                Statement::IfThen(cond, body) => {
                    let cond_val = self.eval_expr_with_env(cond, env)?;
                    if cond_val.as_bool()? {
                        eval_block!(body, env);
                    }
                }
                Statement::Match {
                    scrutinee,
                    ref arms,
                    ref else_body,
                } => {
                    let scrutinee_val = self.eval_expr_with_env(scrutinee, env)?;
                    let scrutinee_str = format!("{}", scrutinee_val);
                    let mut matched = false;
                    for arm in arms {
                        let arm_matches = match arm {
                            MatchArm::Exact(val, _) => scrutinee_str == *val,
                            MatchArm::StartsWith(prefix, _) => {
                                scrutinee_str.starts_with(prefix.as_str())
                            }
                            MatchArm::Contains(substr, _) => {
                                scrutinee_str.contains(substr.as_str())
                            }
                            MatchArm::Compare(op, threshold, _) => {
                                let threshold_val = self.eval_expr_with_env(threshold, env)?;
                                Self::compare_values(&scrutinee_val, op, &threshold_val)
                                    .unwrap_or_default()
                            }
                        };
                        if arm_matches {
                            matched = true;
                            let body = match arm {
                                MatchArm::Exact(_, b) => b,
                                MatchArm::StartsWith(_, b) => b,
                                MatchArm::Contains(_, b) => b,
                                MatchArm::Compare(_, _, b) => b,
                            };
                            eval_block!(body, env);
                            break;
                        }
                    }
                    if !matched {
                        if let Some(eb) = else_body {
                            eval_block!(eb, env);
                        }
                    }
                }
                // Наряд №17: break statement
                Statement::Break => return Ok(ControlFlow::Break),
                // Наряд №17: continue statement
                Statement::Continue => return Ok(ControlFlow::ContinueLoop),
                Statement::ExprStmt(expr) => {
                    let val = self.eval_expr_with_env(expr, env)?;
                    // Наряда-26 P0-2: respond() as early return.
                    // When an ExprStmt produces HttpResponse inside any block (if/while/match/route),
                    // propagate it as Return so the server can catch it and stop processing.
                    if let Value::HttpResponse { .. } = &val {
                        return Ok(ControlFlow::Return(val));
                    }
                    if !matches!(val, Value::Unit) {
                        last_expr_value = val;
                    }
                }
            }
        }
        Ok(ControlFlow::ContinueNormal(last_expr_value))
    }

    pub fn eval_expr(&self, expr: &Expr) -> Result<Value, String> {
        self.eval_expr_with_env(expr, &self.variables)
    }

    pub(crate) fn eval_expr_with_env(
        &self,
        expr: &Expr,
        env: &HashMap<String, Value>,
    ) -> Result<Value, String> {
        match expr {
            Expr::StringLit(s) => Ok(Value::String(s.clone())),
            Expr::FloatLit(f) => Ok(Value::Float(*f)),
            Expr::BoolLit(b) => Ok(Value::Bool(*b)),
            Expr::List(exprs) => {
                let mut items = Vec::new();
                for expr in exprs {
                    items.push(self.eval_expr_with_env(expr, env)?);
                }
                Ok(Value::List(items))
            }
            Expr::StructLit(fields) => {
                let mut resolved = std::collections::HashMap::new();
                for (k, v) in fields {
                    resolved.insert(k.clone(), self.eval_expr_with_env(v, env)?);
                }
                Ok(Value::Struct {
                    type_name: "Struct".to_string(),
                    fields: resolved,
                })
            }
            // Наряд №14 P0-3: block if/else as expression
            Expr::BlockIfElse {
                condition,
                ref then_body,
                ref else_ifs,
                ref else_body,
            } => {
                let cond_val = self.eval_expr_with_env(condition, env)?;
                if cond_val.as_bool()? {
                    let mut local_env = env.clone();
                    return self.eval_statements(then_body, &mut local_env);
                }
                for (ei_cond, ei_body) in else_ifs {
                    let ei_val = self.eval_expr_with_env(ei_cond, env)?;
                    if ei_val.as_bool()? {
                        let mut local_env = env.clone();
                        return self.eval_statements(ei_body, &mut local_env);
                    }
                }
                if let Some(eb) = else_body {
                    let mut local_env = env.clone();
                    return self.eval_statements(eb, &mut local_env);
                }
                Ok(Value::Unit)
            }
            // Наряд №14 P1-4: try expression — catch errors, return Unit
            Expr::Try(inner) => match self.eval_expr_with_env(inner, env) {
                Ok(val) => Ok(val),
                Err(e) => {
                    eprintln!("[try] caught error: {}", e);
                    Ok(Value::Unit)
                }
            },
            Expr::IfElse(cond, then_br, else_br) => {
                let cond_val = self.eval_expr_with_env(cond, env)?;
                if cond_val.as_bool()? {
                    self.eval_expr_with_env(then_br, env)
                } else {
                    self.eval_expr_with_env(else_br, env)
                }
            }
            Expr::Ident(name) => env
                .get(name)
                .cloned()
                .ok_or_else(|| format!("undefined variable: {}", name)),
            Expr::FieldAccess(base, field) => {
                let base_val = self.eval_expr_with_env(base, env)?;
                base_val.get_field(field).cloned()
            }
            Expr::IndexAccess(base, index) => {
                let base_val = self.eval_expr_with_env(base, env)?;
                let idx_val = self.eval_expr_with_env(index, env)?;
                match (&base_val, &idx_val) {
                    (Value::List(items), Value::Float(f)) => {
                        let idx = *f as isize;
                        if idx < 0 {
                            let idx = items.len().wrapping_sub((-idx) as usize);
                            items
                                .get(idx)
                                .cloned()
                                .ok_or_else(|| format!("list index out of bounds: {}", idx))
                        } else {
                            items
                                .get(idx as usize)
                                .cloned()
                                .ok_or_else(|| format!("list index out of bounds: {}", idx))
                        }
                    }
                    (Value::Struct { fields, .. }, Value::String(key)) => fields
                        .get(key)
                        .cloned()
                        .ok_or_else(|| format!("struct has no field '{}'", key)),
                    (Value::String(s), Value::Float(f)) => {
                        let idx = *f as isize;
                        if idx < 0 {
                            // Unicode-aware: use chars().count() for character length, not s.len() (bytes)
                            let char_len = s.chars().count();
                            let abs_idx = char_len.wrapping_sub((-idx) as usize);
                            Ok(Value::String(
                                s.chars().nth(abs_idx).unwrap_or('\0').to_string(),
                            ))
                        } else {
                            Ok(Value::String(
                                s.chars().nth(idx as usize).unwrap_or('\0').to_string(),
                            ))
                        }
                    }
                    _ => Err(format!(
                        "index access: expected List[Int] or Struct[String], got {}[{}]",
                        base_val.type_name(),
                        idx_val.type_name()
                    )),
                }
            }
            Expr::QualifiedCall {
                module,
                function,
                args,
            } => {
                let mut eval_args = Vec::new();
                for arg in args {
                    eval_args.push(self.eval_expr_with_env(arg, env)?);
                }
                // Verify the module namespace was imported
                if !self.module_namespaces.contains_key(module) {
                    return Err(format!(
                        "undefined module: '{}' (did you import it?)",
                        module
                    ));
                }
                // Patterns from imported modules are merged into self.patterns.
                // Resolve as if it were a regular FnCall with the function name.
                // Check builtins first
                // Наряд №7 — query/db_execute need db_conn (intercept before generic builtin)
                if function == "query" {
                    return self.invoke_query(&eval_args);
                }
                if function == "db_execute" {
                    return self.invoke_db_execute(&eval_args);
                }
                // ADR-0051: inspect() needs interpreter state
                if function == "inspect" {
                    return self.invoke_inspect(&eval_args);
                }
                if let Some(builtin_fn) = self.builtins.get(function) {
                    // Phase 7.5: Sandbox enforcement — filesystem isolation
                    if let Some(ref sb) = self.active_sandbox {
                        if sb.forbidden.iter().any(|f| f == "filesystem") {
                            if matches!(
                                function.as_str(),
                                "read_file"
                                    | "write_file"
                                    | "append_file"
                                    | "delete_file"
                                    | "file_exists"
                                    | "list_dir"
                            ) {
                                return Err(format!(
                                    "filesystem access forbidden in sandbox '{}'",
                                    sb.name
                                ));
                            }
                            // Наряд №17 Г.2: also enforce exec() in sandbox
                            if function == "exec" {
                                return Err(format!("exec() forbidden in sandbox '{}'", sb.name));
                            }
                        }
                    }
                    // O-2: Fire on_write hooks before mutating builtins
                    if Self::is_write_builtin(function) {
                        self.fire_on_write_hooks(function, &eval_args);
                    }
                    return builtin_fn(&eval_args);
                }
                // Look up compiled pattern.
                // ADR-0054: For tool namespaces, use qualified key "module.function".
                // For import namespaces, patterns are already merged flat under their function name.
                let namespace = self.module_namespaces.get(module).map(|s| s.as_str());
                let is_tool = namespace.is_some_and(|ns| ns.starts_with("tool:"));
                let qualified_key = format!("{}.{}", module, function);
                let pattern = if is_tool {
                    self.patterns.get(&qualified_key).cloned().ok_or_else(|| {
                        format!("undefined method '{}' in tool '{}'", function, module)
                    })?
                } else {
                    match self.patterns.get(function) {
                        Some(p) => p.clone(),
                        None => {
                            return Err(format!(
                                "undefined pattern '{}' in module '{}'",
                                function, module
                            ))
                        }
                    }
                };
                if eval_args.len() != pattern.params.len() {
                    return Err(format!(
                        "pattern {} expects {} arguments, got {}",
                        function,
                        pattern.params.len(),
                        eval_args.len()
                    ));
                }
                let mut local_env = self.bind_and_collapse(&pattern.params, &eval_args)?;
                self.invoke_pattern_with_hooks(function, &eval_args, || {
                    self.eval_statements(&pattern.body, &mut local_env)
                })
            }
            Expr::FnCall(name, args) => {
                let mut eval_args = Vec::new();
                for (i, arg) in args.iter().enumerate() {
                    // Наряд №115: render(TemplateName, ...) — bare Ident is the template name
                    if name == "render" && i == 0 {
                        if let Expr::Ident(n) = arg {
                            eval_args.push(Value::String(n.clone()));
                            continue;
                        }
                    }
                    eval_args.push(self.eval_expr_with_env(arg, env)?);
                }

                // Check recall (memory) first
                if name == "recall" {
                    return self.invoke_recall(eval_args);
                }

                // Check find (entity store query)
                if name == "find" {
                    return self.invoke_find(eval_args);
                }

                // ADR-0051: inspect() — needs interpreter state (pattern_stats)
                if name == "inspect" {
                    return self.invoke_inspect(&eval_args);
                }

                // ADR-0052: event_count() — read event stream
                if name == "event_count" {
                    let etype = eval_args.first().map(|a| format!("{}", a));
                    let count = self.event_count(etype.as_deref());
                    return Ok(Value::Float(count as f64));
                }

                // ADR-0052: events_since(seconds) — get events since N seconds ago
                if name == "events_since" {
                    let seconds = match eval_args.first() {
                        Some(Value::Float(s)) => *s,
                        Some(other) => {
                            return Err(format!(
                                "events_since() expected Float, got {}",
                                other.type_name()
                            ))
                        }
                        None => {
                            return Err("events_since() requires 1 argument (seconds)".to_string())
                        }
                    };
                    let now_ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    let since_ms = now_ms.saturating_sub((seconds * 1000.0) as u64);
                    let events = self.events_since_ms(since_ms);
                    let mut list = Vec::new();
                    for ev in events {
                        let mut fields = HashMap::new();
                        fields.insert("id".to_string(), Value::Float(ev.id as f64));
                        fields.insert("timestamp".to_string(), Value::Float(ev.timestamp as f64));
                        fields.insert("event_type".to_string(), Value::String(ev.event_type));
                        fields.insert("source".to_string(), Value::String(ev.source));
                        fields.insert(
                            "data_json".to_string(),
                            Value::String(format!("{:?}", ev.data)),
                        );
                        if let Some(dur) = ev.duration_ms {
                            fields.insert("duration_ms".to_string(), Value::Float(dur as f64));
                        }
                        list.push(Value::Struct {
                            type_name: "Event".to_string(),
                            fields,
                        });
                    }
                    return Ok(Value::List(list));
                }

                // ADR-0052: event_sum(type, field) — sum numeric field across events
                if name == "event_sum" {
                    if eval_args.len() < 2 {
                        return Err("event_sum() requires 2 arguments (type, field)".to_string());
                    }
                    let etype = format!("{}", eval_args[0]);
                    let field = format!("{}", eval_args[1]);
                    return Ok(Value::Float(self.event_sum(&etype, &field)));
                }

                // ADR-0053: conversation builtins
                if name == "conv_start" {
                    return self.invoke_conv_start(&eval_args);
                }
                if name == "conv_add" {
                    return self.invoke_conv_add(&eval_args);
                }
                if name == "conv_history" {
                    return self.invoke_conv_history(&eval_args);
                }
                if name == "conv_context" {
                    return self.invoke_conv_context(&eval_args);
                }
                if name == "conv_end" {
                    return self.invoke_conv_end(&eval_args);
                }

                // Check json_body() — server context builtin (Наряд №3)
                // Returns the parsed JSON request body set by execute_route_body.
                if name == "json_body" {
                    if let Some(body) = self.server_json_body.clone() {
                        return Ok(body);
                    }
                    // Fallback: empty struct (non-server context)
                    return Ok(Value::Struct {
                        type_name: "JsonBody".to_string(),
                        fields: std::collections::HashMap::new(),
                    });
                }

                // Bug 2.1 fix: query_param() — intercept to access server_query_params
                if name == "query_param" {
                    let param_name = eval_args
                        .first()
                        .and_then(|v| match v {
                            Value::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or_default();
                    if let Some(val) = self.get_server_query_param(&param_name) {
                        return Ok(Value::String(val));
                    }
                    // Fallback: empty string (non-server context, param not found)
                    return Ok(Value::String(String::new()));
                }

                // Наряд №14 P2-6: require() — RBAC check
                if name == "require" {
                    let role = eval_args
                        .first()
                        .and_then(|v| match v {
                            Value::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or_default();
                    if self.server_user_roles.contains(&role) {
                        return Ok(Value::Bool(true));
                    }
                    return Err(format!(
                        "require('{}'): access denied — user has roles {:?}",
                        role, self.server_user_roles
                    ));
                }

                // Check memorize() — callable form (Definition of Done)
                // Usage: let _ = memorize("text", 0.5) or memorize("text")
                // Differs from declaration: memorize "text" with priority=0.5
                if name == "memorize" {
                    return self.invoke_memorize_fn(eval_args);
                }

                // recall_top_k() — hybrid FTS5 BM25 + cosine RRF search (expression context)
                if name == "recall_top_k" {
                    return self.invoke_recall_top_k_fn(eval_args);
                }

                // Check forget() — callable form (Definition of Done)
                // Usage: forget("query", 30)
                if name == "forget" {
                    return self.invoke_forget_fn(eval_args);
                }

                // Наряд №67: recipe_save — intercept to also memorize for recipe_search
                if name == "recipe_save" {
                    return self.invoke_recipe_save_fn(eval_args);
                }

                // Наряд №67: recipe_search — semantic search via recall_top_k + kv_get
                if name == "recipe_search" {
                    return self.invoke_recipe_search_fn(eval_args);
                }

                // Наряд №7 — query() / db_execute() need access to db_conn
                // Intercept before generic builtin dispatch
                if name == "query" {
                    return self.invoke_query(&eval_args);
                }
                if name == "db_execute" {
                    return self.invoke_db_execute(&eval_args);
                }
                // Наряда-26 P1-7: query_scalar / query_row
                if name == "query_scalar" {
                    return self.invoke_query_scalar(&eval_args);
                }
                if name == "query_row" {
                    return self.invoke_query_row(&eval_args);
                }

                // Problem B (reverse-iteration): map(list, "pattern_name") — needs pattern access
                if name == "map" {
                    let list = match eval_args.first() {
                        Some(Value::List(items)) => items.clone(),
                        _ => return Err("map() expects first argument to be a List".to_string()),
                    };
                    let pattern_name = match eval_args.get(1) {
                        Some(Value::String(s)) => s.clone(),
                        _ => {
                            return Err(
                                "map() expects second argument to be a pattern name (String)"
                                    .to_string(),
                            )
                        }
                    };
                    let pattern = match self.patterns.get(&pattern_name) {
                        Some(p) => p.clone(),
                        None => return Err(format!("map(): pattern '{}' not found", pattern_name)),
                    };
                    if pattern.params.len() != 1 {
                        return Err(format!(
                            "map(): pattern '{}' must accept exactly 1 argument, got {}",
                            pattern_name,
                            pattern.params.len()
                        ));
                    }
                    let mut results = Vec::new();
                    for item in &list {
                        let mut local_env =
                            self.bind_and_collapse(&pattern.params, std::slice::from_ref(item))?;
                        let result = self.eval_statements(&pattern.body, &mut local_env)?;
                        results.push(result);
                    }
                    return Ok(Value::List(results));
                }

                // Problem C (reverse-iteration): db_insert(table, struct) — needs db_conn
                if name == "db_insert" {
                    let table =
                        match eval_args.first() {
                            Some(Value::String(s)) => s.clone(),
                            _ => return Err(
                                "db_insert() expects first argument to be a table name (String)"
                                    .to_string(),
                            ),
                        };
                    let fields = match eval_args.get(1) {
                        Some(Value::Struct { fields, .. }) => fields.clone(),
                        _ => return Err("db_insert() expects second argument to be a Struct { field: value, ... }".to_string()),
                    };
                    let guard = self
                        .db_conn
                        .lock()
                        .map_err(|e| format!("db lock error: {}", e))?;
                    let conn = guard.as_ref().ok_or_else(|| {
                        "db_insert() error: no database connection. Declare db { url: \"sqlite::memory:\" } first.".to_string()
                    })?;
                    let col_names: Vec<String> = fields.keys().cloned().collect();
                    let placeholders: Vec<String> =
                        col_names.iter().map(|_| "?".to_string()).collect();
                    let sql = format!(
                        "INSERT INTO {} ({}) VALUES ({})",
                        table,
                        col_names.join(", "),
                        placeholders.join(", ")
                    );
                    let params: Vec<Box<dyn rusqlite::types::ToSql>> =
                        fields
                            .values()
                            .map(|v| match v {
                                Value::String(s) => {
                                    Box::new(s.clone()) as Box<dyn rusqlite::types::ToSql>
                                }
                                Value::Float(f) => Box::new(*f) as Box<dyn rusqlite::types::ToSql>,
                                Value::Bool(b) => Box::new(*b) as Box<dyn rusqlite::types::ToSql>,
                                Value::Unit => Box::new(Option::<String>::None)
                                    as Box<dyn rusqlite::types::ToSql>,
                                other => Box::new(format!("{}", other))
                                    as Box<dyn rusqlite::types::ToSql>,
                            })
                            .collect();
                    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                        params.iter().map(|p| p.as_ref()).collect();
                    conn.execute(&sql, param_refs.as_slice())
                        .map_err(|e| format!("db_insert() SQL error: {}", e))?;
                    // Return last inserted rowid
                    let rowid: i64 = conn
                        .query_row("SELECT last_insert_rowid()", [], |row| row.get(0))
                        .unwrap_or(0);
                    return Ok(Value::Float(rowid as f64));
                }

                // Problem A: resolve_skill_index(dept) — returns registered index as Value::Struct
                if name == "resolve_skill_index" {
                    let dept = match eval_args.first() {
                        Some(Value::String(s)) => s.clone(),
                        _ => {
                            return Err("resolve_skill_index() expects a department name (String)"
                                .to_string())
                        }
                    };
                    let idx = self.skill_indices.get(&dept).ok_or_else(|| {
                        format!(
                            "resolve_skill_index(): no skill_index declared for '{}'",
                            dept
                        )
                    })?;
                    // Convert to Value::Struct for field access
                    let mut fields = HashMap::new();
                    // tier1: list of always-skill names
                    let tier1: Vec<Value> = idx
                        .tiers
                        .iter()
                        .filter(|t| t.mode == "always")
                        .flat_map(|t| t.skills.iter().map(|s| Value::String(s.clone())))
                        .collect();
                    fields.insert("tier1".to_string(), Value::List(tier1));
                    // tier2+: list of trigger-rule structs
                    for tier in &idx.tiers {
                        if tier.mode == "when_matches" {
                            let rules: Vec<Value> = tier
                                .rules
                                .iter()
                                .map(|r| {
                                    let mut f = HashMap::new();
                                    f.insert("skill".to_string(), Value::String(r.skill.clone()));
                                    f.insert(
                                        "triggers".to_string(),
                                        Value::List(
                                            r.triggers
                                                .iter()
                                                .map(|t| Value::String(t.clone()))
                                                .collect(),
                                        ),
                                    );
                                    Value::Struct {
                                        type_name: "TriggerRule".to_string(),
                                        fields: f,
                                    }
                                })
                                .collect();
                            fields.insert(format!("tier{}", tier.level), Value::List(rules));
                        }
                    }
                    // budget
                    if let Some(b) = idx.budget {
                        fields.insert("budget".to_string(), Value::Float(b));
                    }
                    // truncation
                    if let Some(ref t) = idx.truncation {
                        let mode_str = match t {
                            TruncationMode::WholeSkillOnly => "whole_skill_only",
                            TruncationMode::TruncateAtBoundary => "truncate_at_boundary",
                        };
                        fields.insert(
                            "truncation".to_string(),
                            Value::String(mode_str.to_string()),
                        );
                    }
                    return Ok(Value::Struct {
                        type_name: format!("SkillIndex_{}", dept),
                        fields,
                    });
                }

                // Problem A: fit_to_budget(list, budget, mode) — MVP: return list as-is
                if name == "fit_to_budget" {
                    let list = match eval_args.first() {
                        Some(Value::List(items)) => items.clone(),
                        _ => {
                            return Err(
                                "fit_to_budget() expects first argument to be a List".to_string()
                            )
                        }
                    };
                    return Ok(Value::List(list));
                }

                // Check learnable patterns first
                if let Some(learnable) = self.learnable_patterns.get(name).cloned() {
                    // ADR-0089: reset propagated confidence before collapse
                    *self
                        .propagated_confidence
                        .lock()
                        .unwrap_or_else(|e| e.into_inner()) = 1.0;
                    let collapsed_args = self.collapse_args(&learnable.params, &eval_args);
                    let learnable_clone = learnable.clone();
                    let result = self.invoke_pattern_with_hooks(name, &collapsed_args, || {
                        self.invoke_learnable_with_env(name, &learnable_clone, &collapsed_args)
                    });
                    // ADR-0089: wrap result as Fluid if confidence was propagated
                    let conf = *self
                        .propagated_confidence
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    return Self::maybe_wrap_with_confidence(result, conf);
                }

                // Check builtins
                if let Some(builtin_fn) = self.builtins.get(name) {
                    // O-2: Fire on_write hooks before mutating builtins
                    if Self::is_write_builtin(name) {
                        self.fire_on_write_hooks(name, &eval_args);
                    }
                    let result = builtin_fn(&eval_args);
                    // Phase 7.5: Audit log for unsafe_html rendering
                    if name == "render" {
                        if let Ok(Value::Html(_)) = &result {
                            let template_name = eval_args
                                .first()
                                .map(|a| format!("{}", a))
                                .unwrap_or_else(|| "unknown".to_string());
                            self.push_audit(format!(
                                "[AUDIT] unsafe_html: rendered template '{}'",
                                template_name
                            ));
                        }
                    }
                    return result;
                }

                // Look up compiled pattern
                let pattern = match self.patterns.get(name) {
                    Some(p) => p.clone(),
                    None => {
                        return Ok(Value::String(format!(
                            "[ERROR: unknown function '{}']",
                            name
                        )))
                    }
                };

                if eval_args.len() != pattern.params.len() {
                    return Err(format!(
                        "pattern {} expects {} arguments, got {}",
                        name,
                        pattern.params.len(),
                        eval_args.len()
                    ));
                }

                // Bind parameters with Fluid collapse
                // ADR-0089: reset propagated confidence before collapse
                *self
                    .propagated_confidence
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = 1.0;
                let mut local_env = self.bind_and_collapse(&pattern.params, &eval_args)?;
                let result = self.invoke_pattern_with_hooks(name, &eval_args, || {
                    self.eval_statements(&pattern.body, &mut local_env)
                });
                // ADR-0089: wrap result as Fluid if confidence was propagated
                let conf = *self
                    .propagated_confidence
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                Self::maybe_wrap_with_confidence(result, conf)
            }
            Expr::BinaryOp(left, op, right) => {
                // Short-circuit for logical operators: and/or
                if matches!(op, BinOp::And) {
                    let l = self.eval_expr_with_env(left, env)?;
                    if !Self::is_truthy(&l) {
                        return Ok(Value::Bool(false));
                    }
                    let r = self.eval_expr_with_env(right, env)?;
                    return Ok(Value::Bool(Self::is_truthy(&r)));
                }
                if matches!(op, BinOp::Or) {
                    let l = self.eval_expr_with_env(left, env)?;
                    if Self::is_truthy(&l) {
                        return Ok(Value::Bool(true));
                    }
                    let r = self.eval_expr_with_env(right, env)?;
                    return Ok(Value::Bool(Self::is_truthy(&r)));
                }
                let l = self.eval_expr_with_env(left, env)?;
                let r = self.eval_expr_with_env(right, env)?;
                self.eval_binop(l, *op, r)
            }
        }
    }

    /// Collapse threshold for Fluid values (0.0..1.0).
    /// When a Fluid value is used in a typed context, the matching variant's
    /// confidence must be >= this threshold to collapse successfully.
    /// Below threshold → soft-failure (returns Unit).
    const COLLAPSE_THRESHOLD: f64 = 0.1;

    /// ADR-0089: If confidence < 1.0, wrap a concrete result as Fluid
    /// with the propagated confidence, so downstream consumers can query it
    /// via the `confidence()` builtin.
    fn maybe_wrap_with_confidence(
        result: Result<Value, String>,
        confidence: f64,
    ) -> Result<Value, String> {
        // Only wrap successful concrete results with propagated confidence < 1.0
        if confidence < 1.0 {
            result.map(|val| match val {
                // Don't double-wrap Fluid values
                Value::Fluid(_) => val,
                // Don't wrap Unit
                Value::Unit => val,
                // Wrap concrete values as Fluid with propagated confidence
                concrete => Value::Fluid(vec![FluidValueVariant {
                    type_name: concrete.type_name().to_string(),
                    value: concrete,
                    confidence,
                }]),
            })
        } else {
            result
        }
    }

    /// Collapse a Fluid value to a concrete type if needed.
    /// If the value is Fluid, finds the variant matching `required_type` with the
    /// highest confidence. If confidence >= threshold, returns the concrete value.
    /// Otherwise, returns Unit (soft-failure per soft-failure semantics).
    /// Non-Fluid values pass through unchanged.
    fn maybe_collapse(&self, value: &Value, required_type: &str) -> Result<Value, String> {
        match value {
            Value::Fluid(variants) => {
                // If the required type IS Fluid, pass through without collapsing
                if required_type == "Fluid" {
                    // ADR-0089: propagate confidence — min of all variant confidences
                    let max_conf = variants
                        .iter()
                        .map(|v| v.confidence)
                        .fold(0.0_f64, f64::max);
                    let current = *self
                        .propagated_confidence
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    *self
                        .propagated_confidence
                        .lock()
                        .unwrap_or_else(|e| e.into_inner()) = current.min(max_conf);
                    return Ok(value.clone());
                }
                // Find the best matching variant for the required type
                let best = variants
                    .iter()
                    .filter(|v| v.type_name == required_type)
                    .max_by(|a, b| {
                        a.confidence
                            .partial_cmp(&b.confidence)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });

                match best {
                    Some(variant) if variant.confidence >= Self::COLLAPSE_THRESHOLD => {
                        // ADR-0089: propagate confidence — track min of all collapses
                        let mut conf = self
                            .propagated_confidence
                            .lock()
                            .unwrap_or_else(|e| e.into_inner());
                        *conf = (*conf).min(variant.confidence);
                        Ok(variant.value.clone())
                    }
                    Some(_variant) => {
                        // Confidence below threshold — soft failure
                        Ok(Value::Unit)
                    }
                    None => {
                        // No matching variant at all — soft failure
                        Ok(Value::Unit)
                    }
                }
            }
            other => Ok(other.clone()),
        }
    }

    /// Bind arguments to pattern parameters, collapsing Fluid values as needed.
    /// For each (param, arg) pair, if the arg is a Fluid and the param has a
    /// declared type, collapse the Fluid to that type.
    pub(super) fn bind_and_collapse(
        &self,
        params: &[Param],
        args: &[Value],
    ) -> Result<HashMap<String, Value>, String> {
        let mut local_env = HashMap::new();
        for (param, arg) in params.iter().zip(args.iter()) {
            let collapsed = self.maybe_collapse(arg, &param.type_name)?;
            local_env.insert(param.name.clone(), collapsed);
        }
        Ok(local_env)
    }

    /// Collapse a list of arguments using parameter type annotations.
    /// Returns a new Vec of arguments where Fluid values have been collapsed
    /// to the type required by the corresponding parameter.
    pub(super) fn collapse_args(&self, params: &[Param], args: &[Value]) -> Vec<Value> {
        params
            .iter()
            .zip(args.iter())
            .map(|(p, a)| {
                self.maybe_collapse(a, &p.type_name)
                    .unwrap_or_else(|_| a.clone())
            })
            .collect()
    }

    /// Check if a value is an opaque type that cannot be concatenated.
    fn is_opaque_type(v: &Value) -> bool {
        matches!(
            v,
            Value::Html(_)
                | Value::Query(_)
                | Value::Secret(_)
                | Value::Encrypted(_)
                | Value::Hash(_)
                | Value::Subgraph(_)
        )
    }

    /// Apply entity type annotation for security-critical opaque types.
    ///
    /// - `Secret` + String → wrap as Value::Secret
    /// - Html / Query / Encrypted / Hash: refuse constructing from plain String
    ///   (must use their constructors: query(), encrypt(), hash_password(), …)
    /// - All other declared types: leave value unchanged
    fn coerce_to_declared_type(value: Value, type_name: &str) -> Result<Value, String> {
        match type_name {
            "Secret" => match value {
                Value::Secret(_) => Ok(value),
                Value::String(s) => Ok(Value::Secret(SecretString::new(s))),
                other => Err(format!(
                    "cannot coerce {} to Secret (expected String from env() or similar)",
                    other.type_name()
                )),
            },
            "Html" | "Query" | "Encrypted" | "Hash" => match &value {
                Value::Html(_) | Value::Query(_) | Value::Encrypted(_) | Value::Hash(_) => Ok(value),
                Value::String(_) => Err(format!(
                    "cannot construct opaque type {} from String — use the dedicated builtin                      (query(), encrypt(), hash_password(), render(), …)",
                    type_name
                )),
                other => Err(format!(
                    "cannot coerce {} to opaque type {}",
                    other.type_name(),
                    type_name
                )),
            },
            _ => Ok(value),
        }
    }

    fn is_truthy(value: &Value) -> bool {
        match value {
            Value::String(s) => !s.is_empty(),
            Value::Float(f) => *f != 0.0,
            Value::Bool(b) => *b,
            Value::List(items) => !items.is_empty(),
            _ => false,
        }
    }

    fn eval_binop(&self, left: Value, op: BinOp, right: Value) -> Result<Value, String> {
        // Enforce opaque type restrictions for Add (concatenation)
        if matches!(op, BinOp::Add) {
            if Self::is_opaque_type(&left) {
                return Err(format!(
                    "cannot concatenate opaque type {}",
                    left.type_name()
                ));
            }
            if Self::is_opaque_type(&right) {
                return Err(format!(
                    "cannot concatenate opaque type {}",
                    right.type_name()
                ));
            }
        }
        match (op, left, right) {
            // Arithmetic on Floats
            (BinOp::Add, Value::String(a), Value::String(b)) => {
                let result = format!("{}{}", a, b);
                if result.len() > Self::MAX_STRING_LENGTH {
                    return Err(format!(
                        "string length {} exceeds maximum allowed {}",
                        result.len(),
                        Self::MAX_STRING_LENGTH
                    ));
                }
                Ok(Value::String(result))
            }
            (BinOp::Add, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (BinOp::Sub, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (BinOp::Mul, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (BinOp::Div, Value::Float(a), Value::Float(b)) => {
                if b == 0.0 {
                    Err("division by zero".to_string())
                } else {
                    Ok(Value::Float(a / b))
                }
            }
            // Comparison ops (return Bool)
            (BinOp::Gt, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a > b)),
            (BinOp::Lt, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a < b)),
            (BinOp::Ge, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a >= b)),
            (BinOp::Le, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a <= b)),
            // Equality (works for Float, String, Bool, Unit)
            (BinOp::Eq, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a == b)),
            (BinOp::Eq, Value::String(a), Value::String(b)) => Ok(Value::Bool(a == b)),
            (BinOp::Eq, Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a == b)),
            (BinOp::Eq, Value::Unit, Value::Unit) => Ok(Value::Bool(true)),
            (BinOp::Eq, Value::Unit, _) | (BinOp::Eq, _, Value::Unit) => Ok(Value::Bool(false)),
            (BinOp::Ne, Value::String(a), Value::String(b)) => Ok(Value::Bool(a != b)),
            (BinOp::Ne, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a != b)),
            (BinOp::Ne, Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a != b)),
            (BinOp::Ne, Value::Unit, Value::Unit) => Ok(Value::Bool(false)),
            (BinOp::Ne, Value::Unit, _) | (BinOp::Ne, _, Value::Unit) => Ok(Value::Bool(true)),
            (BinOp::Add, l, r) => {
                eprintln!(
                    "[WARNING] implicit Unit→String conversion in '+' operation: {} + {}",
                    l.type_name(),
                    r.type_name()
                );
                Err(format!(
                    "type mismatch in string concatenation: {} + {} (use to_string() explicitly)",
                    l.type_name(),
                    r.type_name()
                ))
            }
            (_, l, r) => Err(format!(
                "type mismatch in binary operation: {} {:?} {}",
                l.type_name(),
                op,
                r.type_name()
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::values::{FluidValueVariant, Value};

    /// Helper: create an Interpreter instance for testing.
    fn test_interp() -> Interpreter {
        Interpreter::new()
    }

    #[test]
    fn collapse_by_type_picks_matching_variant() {
        let interp = test_interp();
        let fluid = Value::Fluid(vec![
            FluidValueVariant {
                type_name: "Float".into(),
                value: Value::Float(42.0),
                confidence: 0.9,
            },
            FluidValueVariant {
                type_name: "String".into(),
                value: Value::String("answer".into()),
                confidence: 0.1,
            },
        ]);
        let result = interp.maybe_collapse(&fluid, "Float").unwrap();
        assert!(matches!(result, Value::Float(f) if f == 42.0));
    }

    #[test]
    fn collapse_by_type_picks_string_variant() {
        let interp = test_interp();
        let fluid = Value::Fluid(vec![
            FluidValueVariant {
                type_name: "Float".into(),
                value: Value::Float(42.0),
                confidence: 0.9,
            },
            FluidValueVariant {
                type_name: "String".into(),
                value: Value::String("answer".into()),
                confidence: 0.1,
            },
        ]);
        let result = interp.maybe_collapse(&fluid, "String").unwrap();
        assert!(matches!(result, Value::String(ref s) if s == "answer"));
    }

    #[test]
    fn collapse_max_confidence_wins_for_same_type() {
        let interp = test_interp();
        let fluid = Value::Fluid(vec![
            FluidValueVariant {
                type_name: "Float".into(),
                value: Value::Float(10.0),
                confidence: 0.3,
            },
            FluidValueVariant {
                type_name: "Float".into(),
                value: Value::Float(99.0),
                confidence: 0.9,
            },
        ]);
        let result = interp.maybe_collapse(&fluid, "Float").unwrap();
        assert!(matches!(result, Value::Float(f) if f == 99.0));
    }

    #[test]
    fn collapse_below_threshold_returns_unit() {
        let interp = test_interp();
        let fluid = Value::Fluid(vec![FluidValueVariant {
            type_name: "Float".into(),
            value: Value::Float(42.0),
            confidence: 0.05,
        }]);
        let result = interp.maybe_collapse(&fluid, "Float").unwrap();
        assert!(matches!(result, Value::Unit));
    }

    #[test]
    fn collapse_at_threshold_boundary() {
        let interp = test_interp();
        // Exactly at threshold (0.1) should succeed
        let fluid_at = Value::Fluid(vec![FluidValueVariant {
            type_name: "Float".into(),
            value: Value::Float(42.0),
            confidence: 0.1,
        }]);
        let result_at = interp.maybe_collapse(&fluid_at, "Float").unwrap();
        assert!(
            matches!(result_at, Value::Float(f) if f == 42.0),
            "at threshold should return value"
        );

        // Just below threshold should return Unit
        let fluid_below = Value::Fluid(vec![FluidValueVariant {
            type_name: "Float".into(),
            value: Value::Float(42.0),
            confidence: 0.099999,
        }]);
        let result_below = interp.maybe_collapse(&fluid_below, "Float").unwrap();
        assert!(
            matches!(result_below, Value::Unit),
            "below threshold should return Unit"
        );
    }

    #[test]
    fn collapse_no_matching_variant_returns_unit() {
        let interp = test_interp();
        let fluid = Value::Fluid(vec![FluidValueVariant {
            type_name: "Float".into(),
            value: Value::Float(42.0),
            confidence: 0.9,
        }]);
        let result = interp.maybe_collapse(&fluid, "String").unwrap();
        assert!(matches!(result, Value::Unit));
    }

    #[test]
    fn collapse_non_fluid_passes_through() {
        let interp = test_interp();
        let concrete = Value::Float(42.0);
        let result = interp.maybe_collapse(&concrete, "Float").unwrap();
        assert!(matches!(result, Value::Float(f) if f == 42.0));

        let s = Value::String("hello".into());
        let result_s = interp.maybe_collapse(&s, "String").unwrap();
        assert!(matches!(result_s, Value::String(ref v) if v == "hello"));
    }

    #[test]
    fn collapse_empty_fluid_returns_unit() {
        let interp = test_interp();
        let fluid = Value::Fluid(vec![]);
        let result = interp.maybe_collapse(&fluid, "Float").unwrap();
        assert!(matches!(result, Value::Unit));
    }
}
