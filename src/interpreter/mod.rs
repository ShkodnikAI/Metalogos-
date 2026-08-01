// ── Tree-walking interpreter for METALOGOS M1+M2 ─────────────────────
// M1: entities, patterns, linear flow
// M2: struct entities, rules, branching flow, comparisons

pub(crate) mod conversations;
pub(crate) mod db;
pub(crate) mod events;
pub(crate) mod learnable;
pub(crate) mod modules;
pub mod types;
pub mod values;
pub(crate) use types::ControlFlow;
pub use types::{
    CompiledLearnable, ConvMessage, Conversation, ConversationConfig, EvalResult, Event,
    PatternStats,
};
pub use values::{FluidValueVariant, SecretString, Value};

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ast::*;
use crate::builtins::Builtins;
#[allow(unused_imports)]
use crate::embeddings::cosine_similarity;
use crate::embeddings::EmbeddingManager;
use crate::llm;
use crate::memory_store::{
    InMemoryKg, InMemoryStore, KgStore, MemoryEntry, MemoryStore, SqliteKg, SqliteStore,
};

/// Acquire a mutex lock, converting poison errors to a user-friendly message.
/// Used in functions that return `Result<_, String>`.
fn lock_or_err<'a, T>(
    guard: Result<
        std::sync::MutexGuard<'a, T>,
        std::sync::PoisonError<std::sync::MutexGuard<'a, T>>,
    >,
) -> Result<std::sync::MutexGuard<'a, T>, String> {
    guard.map_err(|e| format!("lock poisoned: {}", e))
}

/// A compiled pattern ready for invocation.
#[derive(Clone)]
struct CompiledPattern {
    params: Vec<Param>,
    body: Vec<Statement>,
}

/// A cached LLM response (ADR-0047).
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct LlmCacheEntry {
    /// The response string from the LLM.
    response: String,
    /// Unix timestamp when the entry was created.
    created_at: i64,
    /// Time-to-live in seconds.
    ttl: u64,
}

/// ADR-0056: Serialized checkpoint data for flow lifecycle control.
/// Stores the complete state needed to resume a flow from a checkpoint.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct CheckpointData {
    /// Name of the flow being checkpointed.
    flow_name: String,
    /// Checkpoint name (e.g., "mid", "sources").
    checkpoint_name: String,
    /// Pipeline step index at which the checkpoint was taken (0-based).
    step_index: usize,
    /// Serialized current value at this point in the pipeline.
    current_value: Value,
    /// Serialized variable scope at checkpoint time.
    variables: HashMap<String, Value>,
    /// Unix timestamp (milliseconds) when checkpoint was created.
    created_at: i64,
}

/// A registered struct type.
#[derive(Clone)]
struct StructType {
    #[allow(dead_code)]
    name: String,
    fields: Vec<FieldDecl>,
}

// MemoryEntry and KgStore types are now in memory_store module (Phase 7.6).

/// The interpreter holds all runtime state.
pub struct Interpreter {
    /// Global variable store.
    pub(crate) variables: HashMap<String, Value>,
    /// Struct type registry.
    struct_types: HashMap<String, StructType>,
    /// Compiled patterns (pure).
    patterns: HashMap<String, CompiledPattern>,
    /// Compiled learnable patterns (LLM-backed).
    learnable_patterns: HashMap<String, CompiledLearnable>,
    /// Rule declarations (stored for later execution).
    rules: Vec<RuleDecl>,
    /// Built-in function registry.
    builtins: Builtins,
    /// Flow branch definitions: step_name -> [Branch]
    branch_defs: HashMap<String, Vec<Branch>>,
    /// Memory store backend (Phase 7.6): InMemoryStore or SqliteStore.
    memory: std::sync::Mutex<Box<dyn MemoryStore>>,
    /// Knowledge graph store backend (Phase 7.6): InMemoryKg or SqliteKg.
    kg: std::sync::Mutex<Box<dyn KgStore>>,
    /// Sandbox declarations (recorded but not enforced).
    sandboxes: HashMap<String, SandboxDecl>,
    /// Active sandbox enforcement (Phase 7.5): clone of the current sandbox config.
    /// When Some, iteration limits are reduced and network/timeout checks apply.
    active_sandbox: Option<SandboxDecl>,
    /// Memory persist path (Phase 7.6).
    memory_persist_path: Option<String>,
    /// Mutate status log messages.
    mutate_log: Vec<String>,
    /// Module namespaces: alias -> path (for qualified call resolution).
    module_namespaces: HashMap<String, String>,
    /// Set of currently-loading module paths (for circular import detection).
    loading_stack: Vec<String>,
    /// Base directory for resolving relative imports (set before run).
    base_dir: std::path::PathBuf,
    /// Template registry (Phase 6.2)
    pub(crate) templates: HashMap<String, TemplateDecl>,
    /// DB config (Phase 6.3)
    db_config: Option<DbDecl>,
    /// Mock DB store (Phase 6.3)
    db_store: Vec<HashMap<String, Value>>,
    /// SQLite connection for db {} block (Наряд №7).
    /// Opened when db { url: "sqlite::memory:" } or similar is declared.
    db_conn: std::sync::Arc<std::sync::Mutex<Option<rusqlite::Connection>>>,
    /// Resolved DB URL string for re-opening connections (Наряд №8).
    /// Set by init_db_connection() so per-request interpreters can open new connections.
    db_url: Option<String>,
    /// Audit log (Phase 7.5): uses Mutex for interior mutability + Send/Sync.
    audit_log: Mutex<Vec<String>>,
    /// Server config (Phase 6.1)
    server_config: Option<MlogServerDecl>,
    /// Embedding manager for semantic recall (Phase 7.2).
    embedding_manager: EmbeddingManager,
    /// Server context: parsed JSON request body (Наряд №3).
    /// Set by execute_route_body, returned by json_body() builtin.
    server_json_body: Option<Value>,
    /// Server context: parsed query string parameters (Bug 2.1 fix).
    /// Set by execute_route_body, returned by query_param() builtin.
    server_query_params: Option<std::collections::HashMap<String, String>>,
    /// Server context: user roles for RBAC (Наряд №14 P2-6).
    /// Set by execute_route_body, checked by require() builtin.
    server_user_roles: Vec<String>,
    /// LLM response cache (ADR-0047): HashMap<hash, (response, created_at)>
    /// Key = hash(effective_prompt + input), Value = cached response + timestamp.
    /// Checked before every LLM call for learnable patterns with cache: true.
    llm_cache: std::sync::Mutex<HashMap<u64, LlmCacheEntry>>,
    /// Registered hooks (ADR-0045 + O-2): 5 lifecycle points.
    /// Pattern hooks fire around every pattern invocation.
    /// Session hooks fire once at run() entry/exit.
    /// Write hooks fire before every mutating builtin.
    hooks_before: Vec<HookDecl>,
    hooks_after: Vec<HookDecl>,
    hooks_session_start: Vec<HookDecl>,
    hooks_on_write: Vec<HookDecl>,
    hooks_session_end: Vec<HookDecl>,
    /// Eval blocks (ADR-0050): collected during run(), executed by run_eval_blocks().
    eval_blocks: Vec<EvalDecl>,
    /// Per-pattern runtime statistics (ADR-0051): calls, confidence, cache hits, adapt info.
    /// Key = pattern name. Used by the `inspect()` builtin.
    pattern_stats: std::sync::Mutex<std::collections::HashMap<String, PatternStats>>,
    /// Event stream (ADR-0052): in-memory log of all operations.
    /// Each event has id, timestamp, type, source, data, duration.
    event_log: std::sync::Mutex<Vec<Event>>,
    /// Next event ID (ADR-0052): auto-incrementing counter for event IDs.
    event_next_id: std::sync::atomic::AtomicU64,
    /// Conversations storage (ADR-0053): HashMap<id, Conversation>.
    /// Thread-safe via Mutex for interior mutability.
    conversations: std::sync::Mutex<HashMap<String, Conversation>>,
    /// Conversation configuration (ADR-0053).
    /// Set by `conversation { ttl: N max_messages: N compress_after: N }`.
    conversation_config: ConversationConfig,
    /// sqz-inspired P3: token budgets per learnable pattern.
    /// Set by `context_budget { pattern: "name", limit: 4096 }`.
    context_budgets: std::collections::HashMap<String, Option<f64>>,
    /// ADR-0056: Path to checkpoint SQLite database (for lifecycle control).
    /// Set by `memory { persist: "path" }` — checkpoints use same directory.
    /// None = in-memory only (checkpoints stored in HashMap, lost on restart).
    checkpoint_db: std::sync::Mutex<Option<rusqlite::Connection>>,
    /// In-memory checkpoint fallback when no persist path is configured.
    checkpoint_mem: std::sync::Mutex<HashMap<String, CheckpointData>>,
    /// Resume target: if Some((flow_name, checkpoint_name)), the flow should
    /// skip steps until reaching the step after this checkpoint.
    resume_target: Option<(String, String)>,
    /// Problem A: registered skill indices
    skill_indices: HashMap<String, crate::ast::SkillIndexDecl>,
    /// Наряд №4: LLM routing config (providers, circuit breaker, failover).
    /// If None → backward compatible (env vars, single provider).
    llm_config: Option<crate::ast::LlmConfigDecl>,
    /// Наряд №4: Smart LLM router instance. Created from llm_config when present.
    /// Shared via Arc<Mutex> for thread safety in server mode.
    smart_router: std::sync::Arc<std::sync::Mutex<Option<llm::SmartRouter>>>,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            variables: HashMap::new(),
            struct_types: HashMap::new(),
            patterns: HashMap::new(),
            learnable_patterns: HashMap::new(),
            rules: Vec::new(),
            builtins: Builtins::new(),
            branch_defs: HashMap::new(),
            memory: std::sync::Mutex::new(Box::new(InMemoryStore::new())),
            kg: std::sync::Mutex::new(Box::new(InMemoryKg::new())),
            sandboxes: HashMap::new(),
            active_sandbox: None,
            memory_persist_path: None,
            mutate_log: Vec::new(),
            module_namespaces: HashMap::new(),
            loading_stack: Vec::new(),
            base_dir: std::path::PathBuf::from("."),
            templates: HashMap::new(),
            db_config: None,
            db_store: Vec::new(),
            db_conn: std::sync::Arc::new(std::sync::Mutex::new(None)),
            db_url: None,
            audit_log: Mutex::new(Vec::new()),
            server_config: None,
            embedding_manager: EmbeddingManager::new(),
            server_json_body: None,
            server_query_params: None,
            server_user_roles: Vec::new(),
            llm_cache: std::sync::Mutex::new(HashMap::new()),
            hooks_before: Vec::new(),
            hooks_after: Vec::new(),
            hooks_session_start: Vec::new(),
            hooks_on_write: Vec::new(),
            hooks_session_end: Vec::new(),
            eval_blocks: Vec::new(),
            pattern_stats: std::sync::Mutex::new(std::collections::HashMap::new()),
            event_log: std::sync::Mutex::new(Vec::new()),
            event_next_id: std::sync::atomic::AtomicU64::new(1),
            conversations: std::sync::Mutex::new(HashMap::new()),
            conversation_config: ConversationConfig::default(),
            context_budgets: std::collections::HashMap::new(),
            checkpoint_db: std::sync::Mutex::new(None),
            checkpoint_mem: std::sync::Mutex::new(HashMap::new()),
            resume_target: None,
            skill_indices: HashMap::new(),
            llm_config: None,
            smart_router: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }

    /// Set the base directory for resolving relative imports.
    pub fn set_base_dir(&mut self, dir: std::path::PathBuf) {
        self.base_dir = dir;
    }

    /// Set the parsed JSON body for server context (Наряд №3).
    /// Used by execute_route_body; returned by json_body() builtin.
    pub fn set_server_json_body(&mut self, val: Value) {
        self.server_json_body = Some(val);
    }

    /// Get the parsed JSON body in server context.
    pub fn get_server_json_body(&self) -> Option<&Value> {
        self.server_json_body.as_ref()
    }

    /// Set query string parameters (Bug 2.1 fix).
    /// Called by execute_route_body so query_param() can access them.
    pub fn set_server_query_params(&mut self, params: std::collections::HashMap<String, String>) {
        self.server_query_params = Some(params);
    }

    /// Get a query string parameter by name (Bug 2.1 fix).
    pub fn get_server_query_param(&self, name: &str) -> Option<String> {
        self.server_query_params.as_ref()?.get(name).cloned()
    }

    /// Set user roles for RBAC (Наряд №14 P2-6).
    pub fn set_server_user_roles(&mut self, roles: Vec<String>) {
        self.server_user_roles = roles;
    }

    /// Activate a sandbox for enforcement (Phase 7.5).
    /// When a sandbox is active, iteration limits are reduced to 10,000,
    /// network access is blocked if "network" is in the forbidden list,
    /// and LLM calls are subject to the sandbox timeout.
    pub fn set_active_sandbox(&mut self, sandbox: SandboxDecl) {
        self.active_sandbox = Some(sandbox);
    }

    /// Deactivate the current sandbox, restoring normal limits.
    pub fn clear_active_sandbox(&mut self) {
        self.active_sandbox = None;
    }

    /// Get a reference to the active sandbox (if any).
    pub fn get_active_sandbox(&self) -> Option<&SandboxDecl> {
        self.active_sandbox.as_ref()
    }

    /// Push an audit log entry (Phase 7.5).
    /// Uses Mutex for interior mutability so it can be called from `&self` methods.
    pub fn push_audit(&self, entry: String) {
        self.audit_log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(entry);
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

    /// Run a complete .mlog program.
    pub fn run(&mut self, declarations: Vec<Declaration>) -> Result<Option<String>, String> {
        let mut output: Option<String> = None;

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
                    let value = self.instantiate_struct(&e.type_name, &e.fields)?;
                    self.variables.insert(e.name.clone(), value);
                }
                Declaration::EntitySimple(e) => {
                    let value = self.eval_expr(&e.value)?;
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
            }
        }

        // O-2: Fire on_session_end hooks
        for hook in &self.hooks_session_end {
            let mut hook_env = HashMap::new();
            let _ = self.eval_statements(&hook.body, &mut hook_env);
        }

        Ok(output)
    }

    /// Instantiate a struct from a type name and field initializers.
    pub(super) fn instantiate_struct(
        &self,
        type_name: &str,
        inits: &[FieldInit],
    ) -> Result<Value, String> {
        let struct_type = self
            .struct_types
            .get(type_name)
            .ok_or_else(|| format!("unknown struct type: {}", type_name))?
            .clone();

        let mut fields = HashMap::new();
        for fd in &struct_type.fields {
            // Look for an initializer
            let init_val = inits
                .iter()
                .find(|fi| fi.name == fd.name)
                .map(|fi| self.eval_expr(&fi.value));

            let value = match init_val {
                Some(Ok(v)) => v,
                Some(Err(e)) => return Err(e),
                None => {
                    // Use default value
                    match &fd.default {
                        Some(def_expr) => self.eval_expr(def_expr)?,
                        None => Value::Unit,
                    }
                }
            };
            fields.insert(fd.name.clone(), value);
        }

        Ok(Value::Struct {
            type_name: type_name.to_string(),
            fields,
        })
    }

    /// Execute all rules in priority order (highest first, then declaration order).
    pub(super) fn execute_rules(&mut self) -> Result<(), String> {
        // Sort by priority descending; stable sort preserves declaration order for ties
        let mut sorted_rules: Vec<&RuleDecl> = self.rules.iter().collect();
        sorted_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        for rule in sorted_rules {
            let condition_met = self.eval_condition(&rule.condition, &self.variables)?;
            if condition_met {
                // Execute assignment: target.field = value
                let _target_val = self.eval_expr(&rule.target)?;
                let value_val = self.eval_expr(&rule.value)?;

                // We need to mutate the entity in variables
                // The target is an Ident (entity name), field is the field name
                if let Expr::Ident(name) = &rule.target {
                    let entity = self
                        .variables
                        .get_mut(name)
                        .ok_or_else(|| format!("rule target '{}' not found", name))?;
                    entity.set_field(&rule.field, value_val)?;
                }
            }
        }
        Ok(())
    }

    /// Evaluate a rule condition.
    fn eval_condition(
        &self,
        cond: &Condition,
        env: &HashMap<String, Value>,
    ) -> Result<bool, String> {
        match cond {
            Condition::Contains { left, right } => {
                let lv = self.eval_expr_with_env(left, env)?;
                let rv = self.eval_expr_with_env(right, env)?;
                let ls = match &lv {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(format!(
                            "contains: left must be String, got {}",
                            other.type_name()
                        ))
                    }
                };
                let rs = match &rv {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(format!(
                            "contains: right must be String, got {}",
                            other.type_name()
                        ))
                    }
                };
                Ok(ls.contains(&rs))
            }
            Condition::Compare { left, op, right } => {
                let lv = self.eval_expr_with_env(left, env)?;
                let rv = self.eval_expr_with_env(right, env)?;
                let lf = lv.as_float()?;
                let rf = rv.as_float()?;
                Ok(match op {
                    CompareOp::Gt => lf > rf,
                    CompareOp::Lt => lf < rf,
                    CompareOp::Ge => lf >= rf,
                    CompareOp::Le => lf <= rf,
                    CompareOp::Eq => lf == rf,
                    CompareOp::Ne => lf != rf,
                })
            }
        }
    }

    /// ADR-0056: Save a checkpoint for a flow at a given pipeline step.
    fn save_checkpoint(
        &self,
        flow_name: &str,
        checkpoint_name: &str,
        step_index: usize,
        current_value: &Value,
    ) -> Result<(), String> {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let data = CheckpointData {
            flow_name: flow_name.to_string(),
            checkpoint_name: checkpoint_name.to_string(),
            step_index,
            current_value: current_value.clone(),
            variables: self.variables.clone(),
            created_at: ts,
        };

        let state_json = serde_json::to_string(&data)
            .map_err(|e| format!("checkpoint serialization error: {}", e))?;

        // Try SQLite first
        if let Some(ref conn) = *self
            .checkpoint_db
            .lock()
            .map_err(|e| format!("checkpoint lock: {}", e))?
        {
            conn.execute(
                "INSERT OR REPLACE INTO checkpoints (flow_name, checkpoint_name, step_index, state_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![flow_name, checkpoint_name, step_index as i64, state_json, ts],
            ).map_err(|e| format!("checkpoint save error: {}", e))?;
        } else {
            // Fallback: in-memory
            let key = format!("{}:{}", flow_name, checkpoint_name);
            self.checkpoint_mem
                .lock()
                .map_err(|e| format!("checkpoint lock: {}", e))?
                .insert(key, data);
        }

        Ok(())
    }

    /// ADR-0056: Load a checkpoint for a flow. Returns None if not found.
    fn load_checkpoint(
        &self,
        flow_name: &str,
        checkpoint_name: &str,
    ) -> Result<Option<CheckpointData>, String> {
        // Try SQLite first
        if let Some(ref conn) = *self
            .checkpoint_db
            .lock()
            .map_err(|e| format!("checkpoint lock: {}", e))?
        {
            let mut stmt = conn.prepare(
                "SELECT state_json FROM checkpoints WHERE flow_name = ?1 AND checkpoint_name = ?2"
            ).map_err(|e| format!("checkpoint load error: {}", e))?;

            let result: Result<String, _> = stmt
                .query_row(rusqlite::params![flow_name, checkpoint_name], |row| {
                    row.get(0)
                });

            match result {
                Ok(state_json) => {
                    let data: CheckpointData = serde_json::from_str(&state_json)
                        .map_err(|e| format!("checkpoint deserialization error: {}", e))?;
                    Ok(Some(data))
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("checkpoint load error: {}", e)),
            }
        } else {
            // Fallback: in-memory
            let key = format!("{}:{}", flow_name, checkpoint_name);
            Ok(self
                .checkpoint_mem
                .lock()
                .map_err(|e| format!("checkpoint lock: {}", e))?
                .get(&key)
                .cloned())
        }
    }

    /// ADR-0056: Set the resume target for a specific flow and checkpoint.
    /// Must be called before `run()` to take effect.
    pub fn set_resume_target(&mut self, flow_name: &str, checkpoint_name: &str) {
        self.resume_target = Some((flow_name.to_string(), checkpoint_name.to_string()));
    }

    /// ADR-0056: List all checkpoints for a flow (public for tests and CLI).
    /// Returns Vec of (checkpoint_name, step_index, created_at).
    pub fn list_checkpoints(&self, flow_name: &str) -> Result<Vec<(String, usize, i64)>, String> {
        if let Some(ref conn) = *self
            .checkpoint_db
            .lock()
            .map_err(|e| format!("checkpoint lock: {}", e))?
        {
            let mut stmt = conn.prepare(
                "SELECT checkpoint_name, step_index, created_at FROM checkpoints WHERE flow_name = ?1 ORDER BY step_index"
            ).map_err(|e| format!("checkpoint list error: {}", e))?;

            let rows = stmt
                .query_map(rusqlite::params![flow_name], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)? as usize,
                        row.get::<_, i64>(2)?,
                    ))
                })
                .map_err(|e| format!("checkpoint list error: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("checkpoint list error: {}", e))?;

            Ok(rows)
        } else {
            // In-memory fallback
            let mem = self
                .checkpoint_mem
                .lock()
                .map_err(|e| format!("checkpoint lock: {}", e))?;
            let prefix = format!("{}:", flow_name);
            let mut results: Vec<(String, usize, i64)> = mem
                .iter()
                .filter(|(k, _)| k.starts_with(&prefix))
                .map(|(_k, v)| (v.checkpoint_name.clone(), v.step_index, v.created_at))
                .collect();
            results.sort_by_key(|(_, idx, _)| *idx);
            Ok(results)
        }
    }

    /// ADR-0056: Delete a specific checkpoint (public for tests and cleanup).
    pub fn delete_checkpoint(&self, flow_name: &str, checkpoint_name: &str) -> Result<(), String> {
        if let Some(ref conn) = *self
            .checkpoint_db
            .lock()
            .map_err(|e| format!("checkpoint lock: {}", e))?
        {
            conn.execute(
                "DELETE FROM checkpoints WHERE flow_name = ?1 AND checkpoint_name = ?2",
                rusqlite::params![flow_name, checkpoint_name],
            )
            .map_err(|e| format!("checkpoint delete error: {}", e))?;
        } else {
            let key = format!("{}:{}", flow_name, checkpoint_name);
            self.checkpoint_mem
                .lock()
                .map_err(|e| format!("checkpoint lock: {}", e))?
                .remove(&key);
        }
        Ok(())
    }

    /// ADR-0056: Reset all in-memory checkpoints (for test isolation).
    pub fn reset_checkpoints(&self) {
        if let Ok(mut mem) = self.checkpoint_mem.lock() {
            mem.clear();
        }
    }

    /// Execute a flow: evaluate source, thread through pipeline steps.
    /// ADR-0056: After each step, check if a checkpoint follows. If so, save state.
    /// If resume_target is set, skip steps until we reach the checkpoint, then resume.
    pub(super) fn run_flow(&mut self, flow: &FlowDecl) -> Result<String, String> {
        // Register branch definitions for this flow
        self.branch_defs.clear();
        for (step_name, branches) in &flow.branch_defs {
            self.branch_defs.insert(step_name.clone(), branches.clone());
        }

        // ADR-0056: Determine resume start position
        let mut start_idx: usize = 0;
        let mut current: Option<Value> = None;

        if let Some((ref target_flow, ref target_cp)) = self.resume_target {
            if target_flow == &flow.name {
                // Try to load checkpoint
                if let Some(data) = self.load_checkpoint(&flow.name, target_cp)? {
                    // Restore variables from checkpoint
                    for (k, v) in data.variables {
                        self.variables.insert(k, v);
                    }
                    // Start from the step AFTER the checkpoint
                    start_idx = data.step_index + 1;
                    current = Some(data.current_value);
                } else {
                    return Err(format!(
                        "checkpoint '{}' not found for flow '{}'",
                        target_cp, flow.name
                    ));
                }
                // Clear resume target (one-shot)
                self.resume_target = None;
            }
        }

        // If no resume, evaluate the source expression
        let mut current = match current {
            Some(v) => v,
            None => self.eval_expr(&flow.source)?,
        };

        // ADR-0056: Build reverse map: step_index -> checkpoint names at that position
        let mut checkpoint_at: HashMap<usize, Vec<String>> = HashMap::new();
        for (cp_name, &step_idx) in &flow.checkpoints {
            checkpoint_at
                .entry(step_idx)
                .or_default()
                .push(cp_name.clone());
        }

        // Execute pipeline steps, starting from start_idx (0 for fresh run)
        for (i, step_name) in flow.pipeline.iter().enumerate() {
            if i < start_idx {
                continue; // Skip steps before resume point
            }
            current = self.run_flow_step(step_name, current)?;

            // Check if a checkpoint follows this step
            if let Some(cp_names) = checkpoint_at.get(&i) {
                for cp_name in cp_names {
                    self.save_checkpoint(&flow.name, cp_name, i, &current)?;
                }
            }
        }

        Ok(format!("{}", current))
    }

    /// Execute a single flow step: check branch_defs first, else invoke as pattern/builtin.
    fn run_flow_step(&mut self, step_name: &str, current: Value) -> Result<Value, String> {
        if let Some(branches) = self.branch_defs.get(step_name).cloned() {
            // Step has branch definitions — evaluate conditions against current value
            for branch in &branches {
                if self.eval_branch_condition(&branch.condition, &current)? {
                    return self.invoke(&branch.target, vec![current]);
                }
            }
            Err(format!("no branch matched in step '{}'", step_name))
        } else {
            // No branch definitions — invoke as pattern/builtin directly
            self.invoke(step_name, vec![current])
        }
    }

    /// Evaluate a branch condition: `target.field op threshold`
    fn eval_branch_condition(
        &self,
        cond: &BranchCondition,
        current: &Value,
    ) -> Result<bool, String> {
        // The target in branch_condition is the flow input value (current)
        let field_val = current
            .get_field(&cond.field)
            .map_err(|e| format!("branch condition: {}", e))?
            .clone();
        let threshold = self.eval_expr(&cond.threshold)?;
        let fv = field_val.as_float()?;
        let tv = threshold.as_float()?;
        Ok(match cond.op {
            CompareOp::Gt => fv > tv,
            CompareOp::Lt => fv < tv,
            CompareOp::Ge => fv >= tv,
            CompareOp::Le => fv <= tv,
            CompareOp::Eq => fv == tv,
            CompareOp::Ne => fv != tv,
        })
    }

    /// Invoke a pattern or built-in by name with given arguments.
    /// ADR-0045: Hooks (before_pattern / after_pattern) fire around pattern
    /// and learnable pattern invocations, but NOT around builtin calls.
    fn invoke(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        // Check recall (memory) first — it's a built-in with memory access
        if name == "recall" {
            return self.invoke_recall(args);
        }

        // ADR-0045/Phase 7.1: server-context builtins (flow step dispatch)
        if name == "query_param" {
            let param_name = args
                .get(0)
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

        // forget() — callable form (flow step context)
        if name == "forget" {
            return self.invoke_forget_fn(args);
        }

        if name == "find" {
            return self.invoke_find(args);
        }

        // Problem A: resolve_skill_index(dept) — main invoke path
        if name == "resolve_skill_index" {
            let dept = match args.get(0) {
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
            let list = match args.get(0) {
                Some(Value::List(items)) => items.clone(),
                _ => return Err("fit_to_budget() expects first argument to be a List".to_string()),
            };
            return Ok(Value::List(list));
        }

        // Check learnable patterns
        if let Some(learnable) = self.learnable_patterns.get(name).cloned() {
            let collapsed_args = self.collapse_args(&learnable.params, &args);
            let learnable_clone = learnable.clone();
            return self.invoke_pattern_with_hooks(name, &collapsed_args, || {
                self.invoke_learnable_with_env(name, &learnable_clone, &collapsed_args)
            });
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
        let mut local_env = self.bind_and_collapse(&pattern.params, &args)?;

        self.invoke_pattern_with_hooks(name, &args, || {
            self.eval_statements(&pattern.body, &mut local_env)
        })
    }

    /// ADR-0045: Execute a pattern invocation wrapped with before/after hooks.
    /// O-2: Fire on_write hooks before mutating builtins.
    /// Write builtins: mem_set, mtree_store, db_execute, write_file, append_file.
    fn fire_on_write_hooks(&self, target: &str, args: &[Value]) {
        if self.hooks_on_write.is_empty() {
            return;
        }
        let mut hook_env = HashMap::new();
        hook_env.insert("target".to_string(), Value::String(target.to_string()));
        hook_env.insert("args".to_string(), Value::List(args.to_vec()));
        for hook in &self.hooks_on_write {
            // Ignore hook errors — hooks are advisory, not blocking
            let _ = self.eval_statements(&hook.body, &mut hook_env);
        }
    }

    /// Write-builtin names that trigger on_write hooks.
    const WRITE_BUILTINS: &'static [&'static str] = &[
        "mem_set",
        "mtree_store",
        "db_execute",
        "write_file",
        "append_file",
    ];

    fn is_write_builtin(name: &str) -> bool {
        Self::WRITE_BUILTINS.contains(&name)
    }

    /// Recall from memory: find best matching entry using embeddings + decay.
    /// Phase 7.2: Uses cosine similarity on embedding vectors (semantic search).
    /// Falls back to substring match if embeddings are unavailable (empty vectors).
    /// Returns the highest-activation entry above the min_confidence threshold.
    fn invoke_recall(&self, args: Vec<Value>) -> Result<Value, String> {
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
                        result.push_str("\n");
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
    fn invoke_find(&self, args: Vec<Value>) -> Result<Value, String> {
        let type_name = match args.get(0) {
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
        for (_name, value) in &self.variables {
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
    fn invoke_memorize_fn(&self, args: Vec<Value>) -> Result<Value, String> {
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
        }) {
            Ok(id) => { /* Bug 2.3 fix: removed eprintln stdout leak in HTTP context */ }
            Err(_) => { /* silent — don't leak to stdout in HTTP context */ }
        }
        Ok(Value::Unit)
    }

    /// Callable form of forget() — usable inside patterns and route handlers.
    /// Usage: forget("query", 30) — forget entries matching "query" older than 30 days.
    fn invoke_forget_fn(&self, args: Vec<Value>) -> Result<Value, String> {
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

    /// Handle a mutate declaration: replace few-shot examples, compute mock accuracy, decide keep/rollback.
    pub(super) fn handle_mutate(&mut self, m: &MutateDecl) -> Result<String, String> {
        // Evaluate new examples first (before borrowing learnable mutably)
        let mut evaluated_examples: Vec<(String, String)> = Vec::new();
        for (input_expr, output_expr) in &m.new_examples {
            let input_str = match self.eval_expr(input_expr)? {
                Value::String(s) => s,
                other => format!("{}", other),
            };
            let output_str = match self.eval_expr(output_expr)? {
                Value::String(s) => s,
                other => format!("{}", other),
            };
            evaluated_examples.push((input_str, output_str));
        }

        let num_examples = evaluated_examples.len();

        // Now borrow the learnable pattern mutably
        let learnable = self
            .learnable_patterns
            .get_mut(&m.pattern_name)
            .ok_or_else(|| format!("mutate: learnable pattern '{}' not found", m.pattern_name))?;

        // Save original few-shot for rollback
        let original_few_shot = std::mem::take(&mut learnable.few_shot);

        // Replace with new examples
        learnable.few_shot = evaluated_examples;

        // Compute mock accuracy (always 0.95 for MockLlm)
        let accuracy: f64 = 0.95;

        // Check against threshold
        let kept = match (&m.rollback_op, &m.rollback_threshold) {
            (Some(op), Some(threshold)) => {
                match op {
                    CompareOp::Lt => accuracy >= *threshold,
                    CompareOp::Le => accuracy > *threshold,
                    CompareOp::Gt => false, // accuracy >= threshold is the "kept" condition
                    CompareOp::Ge => false,
                    CompareOp::Eq => (accuracy - threshold).abs() < 1e-9,
                    CompareOp::Ne => (accuracy - threshold).abs() >= 1e-9,
                }
            }
            _ => true, // No rollback condition → always keep
        };

        if kept {
            // Keep the new examples (already in place)
            let msg = Ok(format!(
                "[MUTATE] {}: accuracy={}, kept (>= {:.1})",
                m.pattern_name,
                accuracy,
                m.rollback_threshold.unwrap_or(0.0)
            ));
            // Phase 7.5: Audit log for mutate operations (after releasing mutable borrow)
            self.push_audit(format!(
                "[AUDIT] mutate {}: {} examples, accuracy={}",
                m.pattern_name, num_examples, accuracy
            ));
            msg
        } else {
            // Rollback: restore original few-shot
            let learnable = self
                .learnable_patterns
                .get_mut(&m.pattern_name)
                .ok_or_else(|| {
                    format!("mutate: learnable pattern '{}' not found", m.pattern_name)
                })?;
            learnable.few_shot = original_few_shot;
            let msg = Ok(format!(
                "[MUTATE] {}: accuracy={}, rolled back (below {:.1})",
                m.pattern_name,
                accuracy,
                m.rollback_threshold.unwrap_or(0.0)
            ));
            // Phase 7.5: Audit log for mutate operations (rolled back)
            self.push_audit(format!(
                "[AUDIT] mutate {}: {} examples, accuracy={} (rolled back)",
                m.pattern_name, num_examples, accuracy
            ));
            msg
        }
    }

    /// Take the mutate log messages (consuming them).
    pub fn take_mutate_log(&mut self) -> Vec<String> {
        std::mem::take(&mut self.mutate_log)
    }

    /// Take the audit log messages (consuming them).
    pub fn take_audit_log(&mut self) -> Vec<String> {
        self.audit_log
            .get_mut()
            .map(|v| v.drain(..).collect())
            .unwrap_or_default()
    }

    /// Get a variable value by name (for testing).
    pub fn get_variable(&self, name: &str) -> Option<Value> {
        self.variables.get(name).cloned()
    }

    // ── Eval Harness (ADR-0050) ──────────────────────────────────────────

    /// Run all collected eval blocks and return results.
    /// Called after `run()` has registered learnable patterns (and adapt examples).
    pub fn run_eval_blocks(&self) -> Result<Vec<EvalResult>, String> {
        let mut results = Vec::new();
        for eval_decl in &self.eval_blocks {
            let result = self.run_single_eval(eval_decl)?;
            results.push(result);
        }
        Ok(results)
    }

    /// Run a single eval block: invoke learnable pattern on each dataset example,
    /// compare with expected, compute accuracy and confusion matrix.
    fn run_single_eval(&self, eval_decl: &EvalDecl) -> Result<EvalResult, String> {
        let learnable = self
            .learnable_patterns
            .get(&eval_decl.pattern_name)
            .ok_or_else(|| {
                format!(
                    "eval: learnable pattern '{}' not found",
                    eval_decl.pattern_name
                )
            })?;

        let mut correct = 0usize;
        let mut confusion: HashMap<String, HashMap<String, usize>> = HashMap::new();
        let mut failures: Vec<(String, String, String)> = Vec::new(); // (input, expected, actual)

        for (input_str, expected_label) in &eval_decl.dataset {
            // Build args from dataset input (single String argument)
            let args = vec![Value::String(input_str.clone())];

            // Invoke the learnable pattern
            let actual_value =
                self.invoke_learnable_with_env(&eval_decl.pattern_name, learnable, &args)?;
            let actual_label = match actual_value {
                Value::String(s) => s.trim().to_string(),
                other => format!("{}", other),
            };

            // Record in confusion matrix
            let pred_entry = confusion
                .entry(expected_label.clone())
                .or_default()
                .entry(actual_label.clone())
                .or_insert(0);
            *pred_entry += 1;

            if actual_label == *expected_label {
                correct += 1;
            } else {
                failures.push((input_str.clone(), expected_label.clone(), actual_label));
            }
        }

        let total = eval_decl.dataset.len();
        let accuracy = if total > 0 {
            correct as f64 / total as f64
        } else {
            1.0 // empty dataset → perfect by convention
        };

        Ok(EvalResult {
            pattern_name: eval_decl.pattern_name.clone(),
            metric: eval_decl.metric.clone(),
            total,
            correct,
            accuracy,
            threshold: eval_decl.threshold,
            passed: accuracy >= eval_decl.threshold,
            confusion,
            failures,
        })
    }

    /// Get the server configuration (Phase 6.1).
    pub fn get_server_config(&self) -> Option<&MlogServerDecl> {
        self.server_config.as_ref()
    }

    /// Get the template registry (Phase 6.2).
    pub fn get_templates(&self) -> &HashMap<String, TemplateDecl> {
        &self.templates
    }

    pub fn get_memory_persist_path(&self) -> Option<String> {
        self.memory_persist_path.clone()
    }

    /// Get a builtin function by name (used by server scheduler).
    pub fn get_builtin(&self, name: &str) -> Option<&crate::builtins::BuiltinFn> {
        self.builtins.get(name)
    }

    pub fn set_memory_persist_path(&mut self, path: Option<String>) {
        self.memory_persist_path = path;
    }

    /// Collect all known declarations from this interpreter into a Vec.
    /// Used by `check_program_with_root` to build a merged declaration list
    /// for semantic analysis.
    pub fn collect_declarations(&self) -> Vec<crate::ast::Declaration> {
        use crate::ast::*;
        let mut decls = Vec::new();
        for (name, st) in &self.struct_types {
            decls.push(Declaration::EntityType(EntityTypeDecl {
                name: name.clone(),
                fields: st.fields.clone(),
            }));
        }
        for (name, cp) in &self.patterns {
            decls.push(Declaration::Pattern(PatternDecl {
                name: name.clone(),
                params: cp.params.clone(),
                return_type: "String".to_string(),
                body: cp.body.clone(),
            }));
        }
        for (name, lp) in &self.learnable_patterns {
            decls.push(Declaration::LearnablePattern(LearnablePatternDecl {
                name: name.clone(),
                params: lp.params.clone(),
                return_type: "String".to_string(),
                prompt: lp.prompt.clone(),
                context: None,
                context_strategy: crate::ast::ContextStrategy::None,
                max_context_tokens: 2000,
                max_tokens: None,
                cache: false,
                cache_ttl: 3600,
                model: None,
                conversation: None,
            }));
        }
        for r in &self.rules {
            decls.push(Declaration::Rule(r.clone()));
        }
        for t in self.templates.values() {
            decls.push(Declaration::Template(t.clone()));
        }
        for h in &self.hooks_before {
            decls.push(Declaration::Hook(h.clone()));
        }
        for h in &self.hooks_after {
            decls.push(Declaration::Hook(h.clone()));
        }
        for h in &self.hooks_session_start {
            decls.push(Declaration::Hook(h.clone()));
        }
        for h in &self.hooks_on_write {
            decls.push(Declaration::Hook(h.clone()));
        }
        for h in &self.hooks_session_end {
            decls.push(Declaration::Hook(h.clone()));
        }
        decls
    }

    /// Clone pattern definitions, struct types, learnable patterns, rules,
    /// sandboxes, module namespaces, templates, and variables from this interpreter
    /// into another interpreter. MERGES (does not replace) into target, so
    /// definitions accumulate correctly across multiple calls.
    /// Used to propagate program definitions into per-request interpreters in server mode.
    pub fn clone_definitions_into(&self, target: &mut Interpreter) {
        for (k, v) in &self.struct_types {
            target.struct_types.entry(k.clone()).or_insert(v.clone());
        }
        for (k, v) in &self.patterns {
            target.patterns.entry(k.clone()).or_insert(v.clone());
        }
        for (k, v) in &self.learnable_patterns {
            target
                .learnable_patterns
                .entry(k.clone())
                .or_insert(v.clone());
        }
        for r in &self.rules {
            target.rules.push(r.clone());
        }
        for (k, v) in &self.sandboxes {
            target.sandboxes.entry(k.clone()).or_insert(v.clone());
        }
        for (k, v) in &self.module_namespaces {
            target
                .module_namespaces
                .entry(k.clone())
                .or_insert(v.clone());
        }
        for (k, v) in &self.templates {
            target.templates.entry(k.clone()).or_insert(v.clone());
        }
        for (k, v) in &self.variables {
            target.variables.entry(k.clone()).or_insert(v.clone());
        }
        if let Some(ref db) = self.db_config {
            target.db_config = Some(db.clone());
        }
        // Copy resolved DB URL so per-request interpreters can open their own connections
        if let Some(ref url) = self.db_url {
            target.db_url = Some(url.clone());
        }
        // Share db_conn via Arc so in-memory DB persists between requests
        target.db_conn = self.db_conn.clone();
        // Copy embedding manager (for recall() — semantic memory search)
        // EmbeddingManager is cheap to clone; it lazily initializes backends.
        // We don't clone the internal cache/embeddings — each interpreter builds its own.

        // Наряд №12 Bug 1 + O-2: Copy all hook types so they work in route handlers
        for h in &self.hooks_before {
            target.hooks_before.push(h.clone());
        }
        for h in &self.hooks_after {
            target.hooks_after.push(h.clone());
        }
        for h in &self.hooks_session_start {
            target.hooks_session_start.push(h.clone());
        }
        for h in &self.hooks_on_write {
            target.hooks_on_write.push(h.clone());
        }
        for h in &self.hooks_session_end {
            target.hooks_session_end.push(h.clone());
        }
        // ADR-0053: copy conversation config (conversations themselves are per-session)
        target.conversation_config = self.conversation_config.clone();

        // Copy LLM cache so route handlers benefit from cached LLM responses
        if let Ok(mut target_cache) = target.llm_cache.lock() {
            if let Ok(src_cache) = self.llm_cache.lock() {
                for (k, v) in src_cache.iter() {
                    target_cache.entry(*k).or_insert(v.clone());
                }
            }
        }

        // Copy pattern stats so inspect() works in route handlers
        if let Ok(mut target_stats) = target.pattern_stats.lock() {
            if let Ok(src_stats) = self.pattern_stats.lock() {
                for (k, v) in src_stats.iter() {
                    target_stats.entry(k.clone()).or_insert(v.clone());
                }
            }
        }
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
                                match Self::compare_values(&scrutinee_val, op, &threshold_val) {
                                    Ok(b) => b,
                                    Err(_) => false,
                                }
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
                if let Some(builtin_fn) = self.builtins.get(&function) {
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
                    if Self::is_write_builtin(&function) {
                        self.fire_on_write_hooks(&function, &eval_args);
                    }
                    return builtin_fn(&eval_args);
                }
                // Look up compiled pattern.
                // ADR-0054: For tool namespaces, use qualified key "module.function".
                // For import namespaces, patterns are already merged flat under their function name.
                let namespace = self.module_namespaces.get(module).map(|s| s.as_str());
                let is_tool = namespace.map_or(false, |ns| ns.starts_with("tool:"));
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
                for arg in args {
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
                        .get(0)
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
                        .get(0)
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

                // Check forget() — callable form (Definition of Done)
                // Usage: forget("query", 30)
                if name == "forget" {
                    return self.invoke_forget_fn(eval_args);
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
                    let list = match eval_args.get(0) {
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
                            self.bind_and_collapse(&pattern.params, &[item.clone()])?;
                        let result = self.eval_statements(&pattern.body, &mut local_env)?;
                        results.push(result);
                    }
                    return Ok(Value::List(results));
                }

                // Problem C (reverse-iteration): db_insert(table, struct) — needs db_conn
                if name == "db_insert" {
                    let table =
                        match eval_args.get(0) {
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
                    let dept = match eval_args.get(0) {
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
                    let list = match eval_args.get(0) {
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
                    let collapsed_args = self.collapse_args(&learnable.params, &eval_args);
                    let learnable_clone = learnable.clone();
                    return self.invoke_pattern_with_hooks(name, &collapsed_args, || {
                        self.invoke_learnable_with_env(name, &learnable_clone, &collapsed_args)
                    });
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
                let mut local_env = self.bind_and_collapse(&pattern.params, &eval_args)?;
                self.invoke_pattern_with_hooks(name, &eval_args, || {
                    self.eval_statements(&pattern.body, &mut local_env)
                })
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

    /// Collapse a Fluid value to a concrete type if needed.
    /// If the value is Fluid, finds the variant matching `required_type` with the
    /// highest confidence. If confidence >= threshold, returns the concrete value.
    /// Otherwise, returns Unit (soft-failure per soft-failure semantics).
    /// Non-Fluid values pass through unchanged.
    fn maybe_collapse(&self, value: &Value, required_type: &str) -> Result<Value, String> {
        match value {
            Value::Fluid(variants) => {
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

    /// Check if a value is an opaque type that cannot be printed.
    fn is_nonprintable_type(v: &Value) -> bool {
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
