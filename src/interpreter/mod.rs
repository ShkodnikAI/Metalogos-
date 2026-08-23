// ── Tree-walking interpreter for METALOGOS M1+M2 ─────────────────────
// M1: entities, patterns, linear flow
// M2: struct entities, rules, branching flow, comparisons

pub(crate) mod conversations;
pub(crate) mod db;
pub(crate) mod events;
pub(crate) mod execution;
pub(crate) mod flow;
pub(crate) mod hooks;
pub(crate) mod learnable;
pub(crate) mod memory;
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
    #[allow(dead_code)]
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
    /// Test blocks: collected during run(), executed by test runner.
    test_blocks: Vec<TestDecl>,
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
    /// ADR-0089: Propagated confidence from Fluid collapse through pattern calls.
    /// When a Fluid value is collapsed to concrete, its confidence is stored here.
    /// After pattern execution, if < 1.0, the result is wrapped as Fluid with this confidence.
    /// ADR-0089 recommends min() as the combining heuristic.
    propagated_confidence: std::sync::Mutex<f64>,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
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
            test_blocks: Vec::new(),
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
            propagated_confidence: std::sync::Mutex::new(1.0),
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

    /// Take the audit log messages (consuming them).
    pub fn take_audit_log(&mut self) -> Vec<String> {
        self.audit_log
            .get_mut()
            .map(std::mem::take)
            .unwrap_or_default()
    }

    /// Get a variable value by name (for testing).
    pub fn get_variable(&self, name: &str) -> Option<Value> {
        self.variables.get(name).cloned()
    }

    /// Get the server configuration (Phase 6.1).
    pub fn get_server_config(&self) -> Option<&MlogServerDecl> {
        self.server_config.as_ref()
    }

    /// Get the template registry (Phase 6.2).
    pub fn get_templates(&self) -> &HashMap<String, TemplateDecl> {
        &self.templates
    }

    /// Get the builtin function by name (used by server scheduler).
    pub fn get_builtin(&self, name: &str) -> Option<&crate::builtins::BuiltinFn> {
        self.builtins.get(name)
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
}
