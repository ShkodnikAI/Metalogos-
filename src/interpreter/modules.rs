use super::*;
use crate::ast::*;
use crate::llm;
use crate::memory_store::MemoryEntry;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

impl Interpreter {
    /// Handle an import declaration: load module, register namespace or merge globally.
    pub(super) fn handle_import(&mut self, import: &ImportDecl) -> Result<(), String> {
        let module_path = &import.path;

        // Register namespace mapping (alias or path itself for global merge)
        let alias = import
            .alias
            .as_ref()
            .cloned()
            .unwrap_or_else(|| module_path.clone());
        self.module_namespaces
            .insert(alias.clone(), module_path.clone());

        // Load the module file and execute its declarations into this interpreter
        self.load_module(module_path)?;

        Ok(())
    }

    /// Load a module by path, parsing and executing its declarations.
    /// Detects circular imports via loading_stack.
    fn load_module(&mut self, module_path: &str) -> Result<(), String> {
        // Circular import detection
        if self.loading_stack.contains(&module_path.to_string()) {
            return Err(format!(
                "circular import detected: {} -> {}",
                self.loading_stack.join(" -> "),
                module_path
            ));
        }

        // Don't reload if already loaded (check if patterns from this module exist)
        // We track this via the loading_stack during the current load pass
        self.loading_stack.push(module_path.to_string());

        let result = self.load_module_inner(module_path);

        self.loading_stack.pop();
        result
    }

    fn load_module_inner(&mut self, module_path: &str) -> Result<(), String> {
        // Resolve file path: std/string -> std/string.mlog, ./my_utils -> ./my_utils.mlog
        let file_path = self.base_dir.join(format!("{}.mlog", module_path));

        let source = std::fs::read_to_string(&file_path).map_err(|e| {
            format!(
                "cannot import module '{}': {} (tried {:?})",
                module_path, e, file_path
            )
        })?;

        // Parse the module source
        let declarations = crate::parser::parse(&source)
            .map_err(|e| format!("parse error in module '{}': {}", module_path, e))?;

        // Execute all declarations from the module into the current interpreter
        // This merges patterns, entities, etc. into the global scope
        for decl in declarations {
            match decl {
                Declaration::Import(sub_import) => {
                    self.handle_import(&sub_import)?;
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
                    let value = self.instantiate_struct(&e.type_name, &e.fields)?;
                    self.variables.insert(e.name.clone(), value);
                }
                Declaration::EntitySimple(e) => {
                    let value = self.eval_expr(&e.value)?;
                    self.variables.insert(e.name.clone(), value);
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
                Declaration::Rule(r) => self.rules.push(r),
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
                        learnable.few_shot.push((input_str, output_str));
                    }
                    // ADR-0051: track stats
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
                }
                Declaration::Mutate(m) => {
                    let msg = self.handle_mutate(&m)?;
                    self.mutate_log.push(msg);
                }
                Declaration::Eval(e) => {
                    self.eval_blocks.push(e);
                }
                Declaration::Flow(f) => {
                    self.execute_rules()?;
                    // Silently execute flow in module (don't override main output)
                    let _ = self.run_flow(&f);
                }
                Declaration::Sandbox(s) => {
                    self.sandboxes.insert(s.name.clone(), s);
                }
                Declaration::Hook(h) => match h.phase {
                    HookPhase::BeforePattern => self.hooks_before.push(h),
                    HookPhase::AfterPattern => self.hooks_after.push(h),
                    HookPhase::OnSessionStart => self.hooks_session_start.push(h),
                    HookPhase::OnWrite => self.hooks_on_write.push(h),
                    HookPhase::OnSessionEnd => self.hooks_session_end.push(h),
                },
                Declaration::MlogServer(srv) => {
                    self.server_config = Some(srv);
                }
                Declaration::Template(t) => {
                    self.templates.insert(t.name.clone(), t);
                }
                Declaration::Memory(m) => {
                    self.configure_memory(&m);
                }
                Declaration::Conversation(c) => {
                    // ADR-0053: store conversation configuration (merge)
                    self.conversation_config = ConversationConfig {
                        ttl: c.ttl,
                        max_messages: c.max_messages,
                        compress_after: c.compress_after,
                    };
                }
                Declaration::ContextBudget(b) => {
                    self.context_budgets.insert(b.pattern_name.clone(), b.limit);
                }
                Declaration::LlmConfig(config) => {
                    // Наряд №4: store LLM config and create smart router
                    let router = llm::SmartRouter::from_config(&config);
                    if let Ok(mut sr) = self.smart_router.lock() {
                        *sr = Some(router);
                    }
                    self.llm_config = Some(config);
                }
                Declaration::Tool(t) => {
                    // ADR-0054: register tool as namespace + compile methods as qualified patterns
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
                // Наряд №119: type aliases — no runtime action for modules
                Declaration::TypeAlias(_) => {}
                Declaration::Test(t) => {
                    self.test_blocks.push(t);
                }
            }
        }

        Ok(())
    }
}
