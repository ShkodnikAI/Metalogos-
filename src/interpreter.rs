// ── Tree-walking interpreter for METALOGOS M1+M2 ─────────────────────
// M1: entities, patterns, linear flow
// M2: struct entities, rules, branching flow, comparisons

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroizing;

use crate::ast::*;
use crate::builtins::Builtins;
use crate::embeddings::EmbeddingManager;
#[allow(unused_imports)]
use crate::embeddings::cosine_similarity;
use crate::memory_store::{MemoryEntry, MemoryStore, KgStore, InMemoryStore, InMemoryKg, SqliteStore, SqliteKg};
use crate::llm;

/// A single variant inside a Fluid value (runtime). Contains a concrete
/// value, its declared type name, and a confidence score (0.0..1.0).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FluidValueVariant {
    pub type_name: String,
    pub value: Value,
    pub confidence: f64,
}

/// Opaque secret string with automatic memory zeroing on drop (Phase 7.3).
/// Implements serde by serializing as "[SECRET]" marker — actual value is NEVER persisted.
#[derive(Clone, Debug)]
pub struct SecretString(pub Zeroizing<String>);

impl serde::Serialize for SecretString {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // Never serialize the actual secret — emit a safe marker
        s.serialize_str("[SECRET]")
    }
}

impl<'de> serde::Deserialize<'de> for SecretString {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let inner = String::deserialize(d)?;
        Ok(SecretString(Zeroizing::new(inner)))
    }
}

impl std::ops::Deref for SecretString {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl SecretString {
    pub fn new(s: String) -> Self {
        SecretString(Zeroizing::new(s))
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Runtime value.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Value {
    String(String),
    Float(f64),
    Bool(bool),
    Struct {
        type_name: String,
        fields: HashMap<String, Value>,
    },
    /// List value: ordered collection of items.
    List(Vec<Value>),
    /// Fluid value: superposition of typed variants with confidence scores.
    /// Collapses lazily at point of use (see `maybe_collapse`).
    Fluid(Vec<FluidValueVariant>),
    Unit,
    /// Opaque HTML content (Phase 6.2) — cannot be concatenated, printed, or converted to String
    Html(String),
    /// Opaque SQL query (Phase 6.3) — only created via query() builtin
    Query(String),
    /// Opaque secret value (Phase 6.4) — cannot be printed or converted to String.
    /// Phase 7.3: Internally uses SecretString (Zeroizing<String>) — memory is zeroed on drop.
    Secret(SecretString),
    /// Opaque encrypted data (Phase 6.4)
    Encrypted(Vec<u8>),
    /// Opaque password hash (Phase 6.4)
    Hash(String),
    /// Opaque session data (Phase 6.5)
    Session(std::collections::HashMap<String, String>),
    /// HTTP response value (Phase 6.1)
    HttpResponse { status: u16, body: String },
    /// Graph subgraph — opaque first-class graph value (V3).
    /// Contains a serializable GraphSnapshot that can be passed between functions.
    Subgraph(crate::memory_graph::GraphSnapshot),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::Float(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Struct { type_name, fields } => {
                write!(f, "{} {{", type_name)?;
                let pairs: Vec<_> = fields.iter().collect();
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Fluid(variants) => {
                // Display as the highest-confidence variant
                let best = variants.iter().max_by(|a, b| {
                    a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal)
                });
                match best {
                    Some(v) => write!(f, "{}", v.value),
                    None => write!(f, "()"),
                }
            }
            Value::Unit => write!(f, "()"),
            Value::Html(_) => write!(f, "[Html]"),
            Value::Query(_) => write!(f, "[Query]"),
            Value::Secret(_) => write!(f, "[Secret]"),
            Value::Encrypted(_) => write!(f, "[Encrypted]"),
            Value::Hash(_) => write!(f, "[Hash]"),
            Value::Session(_) => write!(f, "[Session]"),
            Value::HttpResponse { status, .. } => write!(f, "[HttpResponse {}]", status),
            Value::Subgraph(snap) => write!(f, "[Subgraph {} nodes, {} edges]", snap.nodes.len(), snap.edges.len()),
        }
    }
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::String(_) => "String",
            Value::Float(_) => "Float",
            Value::Bool(_) => "Bool",
            Value::List(_) => "List",
            Value::Struct { .. } => "Struct",
            Value::Fluid(_) => "Fluid",
            Value::Unit => "Unit",
            Value::Html(_) => "Html",
            Value::Query(_) => "Query",
            Value::Secret(_) => "Secret",
            Value::Encrypted(_) => "Encrypted",
            Value::Hash(_) => "Hash",
            Value::Session(_) => "Session",
            Value::HttpResponse { .. } => "HttpResponse",
            Value::Subgraph(_) => "Subgraph",
        }
    }

    /// Get a field value from a struct. Returns Err if not a struct or field missing.
    pub fn get_field(&self, field: &str) -> Result<&Value, String> {
        match self {
            Value::Struct { fields, .. } => fields.get(field)
                .ok_or_else(|| format!("field '{}' not found on struct", field)),
            _ => Err(format!("cannot access field '{}' on non-struct value ({})", field, self.type_name())),
        }
    }

    /// Set a field value on a mutable struct.
    pub fn set_field(&mut self, field: &str, value: Value) -> Result<(), String> {
        match self {
            Value::Struct { fields, .. } => {
                if fields.contains_key(field) {
                    fields.insert(field.to_string(), value);
                    Ok(())
                } else {
                    Err(format!("field '{}' not found on struct", field))
                }
            }
            _ => Err(format!("cannot set field '{}' on non-struct value", field)),
        }
    }

    /// Convert to f64 for numeric comparisons.
    pub fn as_float(&self) -> Result<f64, String> {
        match self {
            Value::Float(f) => Ok(*f),
            Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            Value::String(s) => s.parse::<f64>()
                .map_err(|_| format!("cannot convert '{}' to Float", s)),
            _ => Err(format!("cannot convert {} to Float", self.type_name())),
        }
    }

    /// Convert to bool for condition checking.
    pub fn as_bool(&self) -> Result<bool, String> {
        match self {
            Value::Bool(b) => Ok(*b),
            Value::Float(f) => Ok(*f != 0.0),
            Value::String(s) => Ok(!s.is_empty()),
            Value::Unit => Ok(false),
            _ => Err(format!("cannot convert {} to Bool", self.type_name())),
        }
    }

    /// Check if this value is a Fluid superposition.
    pub fn is_fluid(&self) -> bool {
        matches!(self, Value::Fluid(_))
    }
}

/// A compiled pattern ready for invocation.
#[derive(Clone)]
struct CompiledPattern {
    params: Vec<Param>,
    body: Vec<Statement>,
}

/// A learnable pattern that calls an LLM.
#[derive(Clone)]
pub struct CompiledLearnable {
    pub params: Vec<Param>,
    pub prompt: String,
    /// Few-shot examples added by `adapt` declarations.
    /// Each entry: (input_string, output_string).
    pub few_shot: Vec<(String, String)>,
    /// Optional context auto-loading mode (ADR-0046).
    /// - None: no context (default, backward compatible)
    /// - Auto: recall(first_param, limit=5)
    /// - Recall(query_expr, limit): explicit recall
    /// - Literal(string): static text prepended to prompt
    pub context: Option<ContextMode>,
    /// Optional context compression strategy (ADR-0055).
    /// - None: no compression (default)
    /// - Auto: inject as-is
    /// - Compress: compress via LLM if exceeds max_context_tokens
    pub context_strategy: ContextStrategy,
    /// Max estimated tokens for context before compression (ADR-0055).
    /// Default: 2000.
    pub max_context_tokens: usize,
    /// Optional max_tokens for LLM backend.
    max_tokens: Option<u32>,
    /// Enable LLM response caching (ADR-0047).
    cache: bool,
    /// Cache time-to-live in seconds. Default 3600 (1 hour).
    cache_ttl: u64,
    /// Optional per-pattern model override (ADR-0048).
    /// When set, passed to the LLM backend instead of the global model.
    model: Option<String>,
    /// Optional conversation binding (ADR-0053).
    /// When set (e.g., "current"), the learnable pattern injects conversation history.
    conversation: Option<String>,
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

/// Result of running an eval block (ADR-0050).
/// Contains accuracy, confusion matrix, and failure details.
#[derive(Debug, Clone)]
pub struct EvalResult {
    /// Name of the evaluated learnable pattern.
    pub pattern_name: String,
    /// Metric used (currently only "accuracy").
    pub metric: String,
    /// Total number of test examples.
    pub total: usize,
    /// Number of correctly predicted examples.
    pub correct: usize,
    /// Fraction of correct predictions (correct / total).
    pub accuracy: f64,
    /// Minimum acceptable accuracy threshold.
    pub threshold: f64,
    /// Whether accuracy >= threshold (eval passes).
    pub passed: bool,
    /// Confusion matrix: expected_label -> predicted_label -> count.
    pub confusion: std::collections::HashMap<String, std::collections::HashMap<String, usize>>,
    /// Failing examples: (input, expected, actual).
    pub failures: Vec<(String, String, String)>,
}

/// Per-pattern runtime statistics (ADR-0051).
/// Tracked automatically during pattern invocation and adapt operations.
/// Returned by the `inspect()` builtin.
#[derive(Debug, Clone)]
pub struct PatternStats {
    /// Total number of invocations of this learnable pattern.
    pub calls: u64,
    /// Sum of confidence values from each invocation (for computing average).
    pub confidence_sum: f64,
    /// Number of cache hits (responses served from few-shot or LLM cache).
    pub cache_hits: u64,
    /// Timestamp of the last adapt operation (Unix seconds), or 0 if never adapted.
    pub last_adapt: i64,
    /// Timestamp of the last invocation (Unix seconds), or 0 if never called.
    pub last_call: i64,
    /// Current count of few-shot examples added via adapt.
    pub examples_count: u64,
}

impl PatternStats {
    pub fn new() -> Self {
        PatternStats {
            calls: 0,
            confidence_sum: 0.0,
            cache_hits: 0,
            last_adapt: 0,
            last_call: 0,
            examples_count: 0,
        }
    }

    /// Average confidence across all invocations.
    pub fn avg_confidence(&self) -> f64 {
        if self.calls == 0 {
            0.0
        } else {
            self.confidence_sum / self.calls as f64
        }
    }
}

/// A single message within a conversation (ADR-0053).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConvMessage {
    /// Message role: "user", "assistant", or "system".
    pub role: String,
    /// Message text content.
    pub text: String,
    /// Unix timestamp when the message was added.
    pub timestamp: i64,
}

/// A conversation with its messages and metadata (ADR-0053).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Conversation {
    /// Conversation identifier.
    pub id: String,
    /// Ordered list of messages in this conversation.
    pub messages: Vec<ConvMessage>,
    /// Unix timestamp when the conversation was created.
    pub created_at: i64,
    /// Unix timestamp of last activity (message added/removed).
    pub last_active: i64,
    /// Additional metadata (key-value pairs).
    pub metadata: HashMap<String, String>,
}

/// Conversation configuration (ADR-0053).
/// Set by `conversation { ttl: N max_messages: N compress_after: N }`.
#[derive(Debug, Clone)]
pub struct ConversationConfig {
    /// Time-to-live in seconds. Default: 1800 (30 minutes).
    pub ttl: u64,
    /// Maximum messages per conversation. Default: 50.
    pub max_messages: usize,
    /// Compress older messages via LLM summarization after this count. Default: 20.
    pub compress_after: usize,
}

impl Default for ConversationConfig {
    fn default() -> Self {
        ConversationConfig {
            ttl: 1800,
            max_messages: 50,
            compress_after: 20,
        }
    }
}

/// A single event in the event stream (ADR-0052).
/// Represents a discrete operation that occurred during interpretation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Event {
    /// Auto-incrementing event ID.
    pub id: u64,
    /// Unix timestamp in milliseconds.
    pub timestamp: u64,
    /// Event type: "pattern_call", "llm_call", "memory_store", "memory_recall",
    /// "rule_fire", "adapt", "error", etc.
    pub event_type: String,
    /// Source: pattern name, "system", or "builtin".
    pub source: String,
    /// Arbitrary key-value data attached to the event.
    pub data: HashMap<String, String>,
    /// Duration of the operation in milliseconds, if measurable.
    pub duration_ms: Option<u64>,
}

impl EvalResult {
    /// Format the eval result as a human-readable report.
    pub fn format_report(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("Eval: {}", self.pattern_name));
        lines.push(format!("  Dataset: {} examples", self.total));
        lines.push(format!("  Accuracy: {:.1}% ({}/{})", self.accuracy * 100.0, self.correct, self.total));
        lines.push(format!("  Threshold: {}", self.threshold));
        lines.push(format!("  Result: {}", if self.passed { "PASS" } else { "FAIL (below threshold)" }));

        // Confusion matrix
        if !self.confusion.is_empty() {
            // Collect all unique labels
            let mut all_labels: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
            for (expected, predictions) in &self.confusion {
                all_labels.insert(expected.clone());
                for pred in predictions.keys() {
                    all_labels.insert(pred.clone());
                }
            }

            let labels: Vec<&String> = all_labels.iter().collect();

            // Header row
            let header = format!("  {:12} {}", "", labels.iter().map(|l| format!("{:>12}", l)).collect::<Vec<_>>().join(" "));
            lines.push(header);

            // Data rows
            for expected in &labels {
                let row = format!("  {:12} {}", expected,
                    labels.iter().map(|pred| {
                        let count = self.confusion.get(*expected)
                            .and_then(|m| m.get(*pred))
                            .copied()
                            .unwrap_or(0);
                        format!("{:>12}", count)
                    }).collect::<Vec<_>>().join(" ")
                );
                lines.push(row);
            }
        }

        // Failing examples with adapt suggestions
        if !self.failures.is_empty() {
            lines.push(String::new());
            lines.push("  Failing examples (suggest adapt):".to_string());
            for (input, expected, actual) in &self.failures {
                lines.push(format!("    - {:?} -> expected {:?}, got {:?}", input, expected, actual));
            }
            // Generate adapt suggestions
            lines.push(String::new());
            lines.push("  Suggested adapt commands:".to_string());
            for (input, expected, _actual) in &self.failures {
                lines.push(format!("    adapt {} add_example({:?}, {:?})", self.pattern_name, input, expected));
            }
        }

        lines.join("\n")
    }
}

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

/// Control flow signal for loop constructs (Наряд №17).
/// Break/Continue propagate through eval_statements without being confused
/// with Return values. The interpreter uses Result<ControlFlow, String>
/// internally for loop bodies, then converts back to Result<Value, String>
/// at the public eval_statements boundary.
#[derive(Debug, Clone)]
pub(crate) enum ControlFlow {
    /// Normal execution, optionally carrying a value (like implicit return).
    ContinueNormal(Value),
    /// `break` — exit the innermost loop.
    Break,
    /// `continue` — skip to next iteration of the innermost loop.
    ContinueLoop,
    /// `return expr` — early return from a pattern/function.
    Return(Value),
}

impl ControlFlow {
    fn is_break(&self) -> bool { matches!(self, ControlFlow::Break) }
    fn is_continue(&self) -> bool { matches!(self, ControlFlow::ContinueLoop) }
    fn is_return(&self) -> bool { matches!(self, ControlFlow::Return(_)) }
    /// Extract the inner value if this is ContinueNormal or Return.
    fn into_value(self) -> Value {
        match self {
            ControlFlow::ContinueNormal(v) | ControlFlow::Return(v) => v,
            ControlFlow::Break | ControlFlow::ContinueLoop => Value::Unit,
        }
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
        self.audit_log.lock().unwrap().push(entry);
    }

    /// ADR-0052: Emit an event to the event stream.
    /// Thread-safe: appends to event_log behind Mutex, auto-increments ID.
    fn emit_event(&self, event_type: &str, source: &str, data: HashMap<String, String>, duration_ms: Option<u64>) {
        let id = self.event_next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let event = Event {
            id,
            timestamp,
            event_type: event_type.to_string(),
            source: source.to_string(),
            data,
            duration_ms,
        };
        if let Ok(mut log) = self.event_log.lock() {
            log.push(event);
        }
    }

    /// ADR-0052: Get total number of events, optionally filtered by type.
    pub fn event_count(&self, event_type: Option<&str>) -> usize {
        if let Ok(log) = self.event_log.lock() {
            match event_type {
                Some(t) => log.iter().filter(|e| e.event_type == t).count(),
                None => log.len(),
            }
        } else {
            0
        }
    }

    /// ADR-0052: Get events since a given Unix timestamp (seconds).
    /// Returns events with timestamp >= since_ms (milliseconds).
    pub fn events_since_ms(&self, since_ms: u64) -> Vec<Event> {
        if let Ok(log) = self.event_log.lock() {
            log.iter().filter(|e| e.timestamp >= since_ms).cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// ADR-0052: Get a reference to the full event log (for test access).
    pub fn get_events(&self) -> Vec<Event> {
        self.event_log.lock().map(|log| log.clone()).unwrap_or_default()
    }

    /// ADR-0052: Sum a numeric field across events of a given type.
    /// Parses field values as f64 and sums them.
    pub fn event_sum(&self, event_type: &str, field: &str) -> f64 {
        if let Ok(log) = self.event_log.lock() {
            log.iter()
                .filter(|e| e.event_type == event_type)
                .filter_map(|e| e.data.get(field))
                .filter_map(|v| v.parse::<f64>().ok())
                .sum()
        } else {
            0.0
        }
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
                    let existing = self.memory.lock().unwrap().all_entries();
                    let mut new_store: Box<dyn MemoryStore> = Box::new(sqlite_store);
                    for entry in existing {
                        let _ = new_store.memorize(entry);
                    }
                    self.memory = std::sync::Mutex::new(new_store);

                    // Migrate KG edges to SQLite (sharing the same DB file)
                    let existing_edges: Vec<(String, String, String, f64)> =
                        self.kg.lock().unwrap().all_edges();
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
                            )"
                        );
                        *self.checkpoint_db.lock().unwrap() = Some(conn);
                    }
                }
                Err(e) => {
                    eprintln!("[memory] Failed to open persistent store '{}': {}. Using in-memory.", path, e);
                }
            }
        }
        // If persist is None, keep the default InMemoryStore (already set in new())
    }

    /// Map Metalogos type names to SQLite column types (Problem C).
    fn mlog_type_to_sql(t: &str) -> &'static str {
        match t {
            "Int" => "INTEGER",
            "Float" => "REAL",
            "String" | "Text" => "TEXT",
            "Bool" => "INTEGER",
            "DateTime" => "TEXT",
            _ => "TEXT",
        }
    }

    /// Problem C: Apply schema declaration — CREATE TABLE IF NOT EXISTS for each table.
    fn apply_schema(&self, schema: &SchemaDecl) -> Result<(), String> {
        let guard = self.db_conn.lock().map_err(|e| format!("db lock error: {}", e))?;
        let conn = guard.as_ref().ok_or_else(|| {
            "schema declaration requires a db connection. Declare db { url: \"...\" } first.".to_string()
        })?;

        for table in &schema.tables {
            let mut col_defs = Vec::new();
            for col in &table.columns {
                let mut def = format!("{} {}", col.name, Self::mlog_type_to_sql(&col.col_type));
                for modi in &col.modifiers {
                    match modi {
                        ColumnModifier::PrimaryKey => def.push_str(" PRIMARY KEY"),
                        ColumnModifier::AutoIncrement => def.push_str(" AUTOINCREMENT"),
                        ColumnModifier::Nullable => def.push_str(" NULL"),
                        ColumnModifier::References(ref_table, ref_field) => {
                            def.push_str(&format!(" REFERENCES {}({})", ref_table, ref_field));
                        }
                    }
                }
                if let Some(ref default_val) = col.default {
                    if default_val == "now()" {
                        def.push_str(" DEFAULT (datetime('now'))");
                    } else {
                        // Strip quotes if present
                        let val = default_val.trim_matches('"');
                        def.push_str(&format!(" DEFAULT '{}'", val));
                    }
                }
                col_defs.push(def);
            }
            let sql = format!("CREATE TABLE IF NOT EXISTS {} ({})", table.name, col_defs.join(", "));
            conn.execute(&sql, []).map_err(|e| format!("schema migration error for table '{}': {}", table.name, e))?;
        }

        Ok(())
    }

    /// Initialize SQLite connection for db { url: "..." } block (Наряд №7).
    /// Supports "sqlite::memory:" for in-memory databases and file paths.
    fn init_db_connection(&mut self, db: &DbDecl) {
        let url_expr = match &db.url {
            Some(expr) => expr,
            None => {
                eprintln!("[db] No url specified in db {{}} block — query() will be unavailable");
                return;
            }
        };
        // Evaluate the url expression (must be a string literal or variable)
        let url = match self.eval_expr(url_expr) {
            Ok(Value::String(s)) => s,
            Ok(other) => {
                eprintln!("[db] url must be a String, got {}", other.type_name());
                return;
            }
            Err(e) => {
                eprintln!("[db] Failed to evaluate url: {}", e);
                return;
            }
        };
        // Parse the URL: "sqlite::memory:" → in-memory, "sqlite:path.db" → file
        let conn = if url == "sqlite::memory:" {
            rusqlite::Connection::open_in_memory()
        } else if url.starts_with("sqlite:") {
            let path = url.trim_start_matches("sqlite:");
            rusqlite::Connection::open(path)
        } else {
            eprintln!("[db] Unsupported URL scheme: '{}'. Use 'sqlite::memory:' or 'sqlite:path.db'", url);
            return;
        };
        match conn {
            Ok(c) => {
                // Enable WAL mode for better concurrent read performance
                let _ = c.execute_batch("PRAGMA journal_mode=WAL;");
                let mut guard = self.db_conn.lock().unwrap();
                *guard = Some(c);
                // Store resolved URL for per-request interpreter reconnection
                self.db_url = Some(url.clone());
                eprintln!("[db] Connected: {}", url);
            }
            Err(e) => {
                eprintln!("[db] Failed to connect to '{}': {}", url, e);
            }
        }
    }

    /// Execute a SQL query and return readable results (Наряд №7).
    /// - SELECT → List of Struct (each row = struct with column names as fields)
    /// - INSERT/UPDATE/DELETE → String with affected row count
    /// - PRAGMA/CREATE/etc. → String "ok"
    fn invoke_query(&self, args: &[Value]) -> Result<Value, String> {
        let sql = match args.first() {
            Some(Value::String(s)) => s.clone(),
            Some(other) => return Err(format!("query() expected String SQL, got {}", other.type_name())),
            None => return Err("query() requires at least 1 argument (SQL string)".to_string()),
        };
        let params: Vec<String> = if args.len() > 1 {
            match &args[1] {
                Value::List(items) => items.iter().filter_map(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    Value::Float(n) => Some(format!("{}", n)),
                    Value::Bool(b) => Some(format!("{}", b)),
                    _ => None,
                }).collect(),
                _ => Vec::new(),
            }
        } else {
            Vec::new()
        };

        let guard = self.db_conn.lock().map_err(|e| format!("db lock error: {}", e))?;
        let conn = guard.as_ref().ok_or_else(|| {
            "query() error: no database connection. Declare db { url: \"sqlite::memory:\" } first.".to_string()
        })?;

        let sql_upper = sql.trim().to_uppercase();
        if sql_upper.starts_with("SELECT") || sql_upper.starts_with("PRAGMA") {
            // SELECT/PRAGMA → List of Struct
            let mut stmt = conn.prepare(&sql)
                .map_err(|e| format!("query() SQL error: {}", e))?;
            let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
            let rows: Vec<Value> = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
                let mut fields = std::collections::HashMap::new();
                for (i, col) in col_names.iter().enumerate() {
                    let val: Value = match row.get_ref(i) {
                        Ok(rusqlite::types::ValueRef::Null) => Value::Unit,
                        Ok(rusqlite::types::ValueRef::Integer(n)) => {
                            // Heuristic: if the column name suggests it's an ID or count, keep as Float
                            Value::Float(n as f64)
                        }
                        Ok(rusqlite::types::ValueRef::Real(f)) => Value::Float(f),
                        Ok(rusqlite::types::ValueRef::Text(s)) => {
                            Value::String(String::from_utf8_lossy(s).to_string())
                        }
                        Ok(rusqlite::types::ValueRef::Blob(b)) => {
                            // Encode blobs as hex strings
                            Value::String(b.iter().map(|byte| format!("{:02x}", byte)).collect())
                        }
                        Err(_) => Value::Unit,
                    };
                    fields.insert(col.clone(), val);
                }
                Ok(Value::Struct { type_name: "Row".to_string(), fields })
            }).map_err(|e| format!("query() execution error: {}", e))?
            .filter_map(|r| r.ok())
            .collect();
            Ok(Value::List(rows))
        } else {
            // INSERT/UPDATE/DELETE/CREATE/ALTER/etc. → affected row count as String
            let affected = conn.execute(&sql, rusqlite::params_from_iter(params.iter()))
                .map_err(|e| format!("query() SQL error: {}", e))?;
            Ok(Value::String(affected.to_string()))
        }
    }

    /// Execute a SQL statement via db_execute() — returns affected row count (Наряд №7).
    fn invoke_db_execute(&self, args: &[Value]) -> Result<Value, String> {
        let sql = match args.first() {
            Some(Value::String(s)) => s.clone(),
            Some(other) => return Err(format!("db_execute() expected String SQL, got {}", other.type_name())),
            None => return Err("db_execute() requires at least 1 argument (SQL string)".to_string()),
        };
        let guard = self.db_conn.lock().map_err(|e| format!("db lock error: {}", e))?;
        let conn = guard.as_ref().ok_or_else(|| {
            "db_execute() error: no database connection. Declare db { url: \"sqlite::memory:\" } first.".to_string()
        })?;
        let affected = conn.execute(&sql, [])
            .map_err(|e| format!("db_execute() SQL error: {}", e))?;
        Ok(Value::String(affected.to_string()))
    }

    /// Наряда-26 P1-7: query_scalar(sql, params) -> Value
    /// Executes a SELECT that returns exactly one row with one column.
    /// Returns the scalar value directly (String, Float, or Unit for NULL).
    fn invoke_query_scalar(&self, args: &[Value]) -> Result<Value, String> {
        let sql = match args.first() {
            Some(Value::String(s)) => s.clone(),
            Some(other) => return Err(format!("query_scalar() expected String SQL, got {}", other.type_name())),
            None => return Err("query_scalar() requires at least 1 argument (SQL string)".to_string()),
        };
        let params: Vec<String> = if args.len() > 1 {
            match &args[1] {
                Value::List(items) => items.iter().filter_map(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    Value::Float(n) => Some(format!("{}", n)),
                    Value::Bool(b) => Some(format!("{}", b)),
                    _ => None,
                }).collect(),
                _ => Vec::new(),
            }
        } else { Vec::new() };

        let guard = self.db_conn.lock().map_err(|e| format!("db lock error: {}", e))?;
        let conn = guard.as_ref().ok_or_else(|| {
            "query_scalar() error: no database connection.".to_string()
        })?;

        let mut stmt = conn.prepare(&sql)
            .map_err(|e| format!("query_scalar() SQL error: {}", e))?;
        let mut rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
            row.get_ref(0).map(|v| match v {
                rusqlite::types::ValueRef::Null => Value::Unit,
                rusqlite::types::ValueRef::Integer(n) => Value::Float(n as f64),
                rusqlite::types::ValueRef::Real(f) => Value::Float(f),
                rusqlite::types::ValueRef::Text(s) => Value::String(String::from_utf8_lossy(s).to_string()),
                rusqlite::types::ValueRef::Blob(b) => Value::String(b.iter().map(|byte| format!("{:02x}", byte)).collect()),
            })
        }).map_err(|e| format!("query_scalar() execution error: {}", e))?;

        match rows.next() {
            Some(Ok(val)) => Ok(val),
            Some(Err(e)) => Err(format!("query_scalar() row error: {}", e)),
            None => Ok(Value::Unit),
        }
    }

    /// Наряда-26 P1-7: query_row(sql, params) -> List
    /// Executes a SELECT that returns exactly one row.
    /// Returns a List of column values (preserving column order).
    fn invoke_query_row(&self, args: &[Value]) -> Result<Value, String> {
        let sql = match args.first() {
            Some(Value::String(s)) => s.clone(),
            Some(other) => return Err(format!("query_row() expected String SQL, got {}", other.type_name())),
            None => return Err("query_row() requires at least 1 argument (SQL string)".to_string()),
        };
        let params: Vec<String> = if args.len() > 1 {
            match &args[1] {
                Value::List(items) => items.iter().filter_map(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    Value::Float(n) => Some(format!("{}", n)),
                    Value::Bool(b) => Some(format!("{}", b)),
                    _ => None,
                }).collect(),
                _ => Vec::new(),
            }
        } else { Vec::new() };

        let guard = self.db_conn.lock().map_err(|e| format!("db lock error: {}", e))?;
        let conn = guard.as_ref().ok_or_else(|| {
            "query_row() error: no database connection.".to_string()
        })?;

        let mut stmt = conn.prepare(&sql)
            .map_err(|e| format!("query_row() SQL error: {}", e))?;
        let col_count = stmt.column_count();
        let mut rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
            let mut vals = Vec::with_capacity(col_count);
            for i in 0..col_count {
                let val = match row.get_ref(i) {
                    Ok(rusqlite::types::ValueRef::Null) => Value::Unit,
                    Ok(rusqlite::types::ValueRef::Integer(n)) => Value::Float(n as f64),
                    Ok(rusqlite::types::ValueRef::Real(f)) => Value::Float(f),
                    Ok(rusqlite::types::ValueRef::Text(s)) => Value::String(String::from_utf8_lossy(s).to_string()),
                    Ok(rusqlite::types::ValueRef::Blob(b)) => Value::String(b.iter().map(|byte| format!("{:02x}", byte)).collect()),
                    Err(_) => Value::Unit,
                };
                vals.push(val);
            }
            Ok(vals)
        }).map_err(|e| format!("query_row() execution error: {}", e))?;

        match rows.next() {
            Some(Ok(vals)) => Ok(Value::List(vals)),
            Some(Err(e)) => Err(format!("query_row() row error: {}", e)),
            None => Ok(Value::List(vec![])),
        }
    }

    /// Open a new DB connection using stored db_url (Наряд №8).
    /// Called by per-request interpreters to get their own SQLite connection.
    /// For in-memory DBs, the Arc-shared connection is already set via clone_definitions_into.
    /// For file-based DBs, opens a new connection (safe for concurrent access via WAL).
    pub fn reconnect_db(&mut self) {
        if let Some(ref url) = self.db_url {
            if url == "sqlite::memory:" {
                // In-memory DB: Arc-shared connection from main interpreter
                // No need to reconnect — clone_definitions_into already shared it
                return;
            } else if url.starts_with("sqlite:") {
                // File DB: open a new connection for this request (WAL handles concurrency)
                let path = url.trim_start_matches("sqlite:");
                match rusqlite::Connection::open(path) {
                    Ok(c) => {
                        let _ = c.execute_batch("PRAGMA journal_mode=WAL;");
                        // For file DBs, each request gets its own connection
                        // (don't overwrite the shared Arc for in-memory)
                        let mut guard = self.db_conn.lock().unwrap();
                        // Only set if no connection yet (in-memory may have set it)
                        if guard.is_none() {
                            *guard = Some(c);
                        }
                    }
                    Err(e) => {
                        eprintln!("[db] Per-request reconnect failed: {}", e);
                    }
                }
            }
        }
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
                Declaration::Hook(h) => {
                    match h.phase {
                        HookPhase::OnSessionStart => self.hooks_session_start.push(h.clone()),
                        HookPhase::OnWrite => self.hooks_on_write.push(h.clone()),
                        HookPhase::OnSessionEnd => self.hooks_session_end.push(h.clone()),
                        HookPhase::BeforePattern => self.hooks_before.push(h.clone()),
                        HookPhase::AfterPattern => self.hooks_after.push(h.clone()),
                    }
                }
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
                    self.struct_types.insert(e.name.clone(), StructType {
                        name: e.name.clone(),
                        fields: e.fields,
                    });
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
                    let _ = self.memory.lock().unwrap().memorize(MemoryEntry {
                        id: None,
                        value: value_str.clone(),
                        priority: m.priority,
                        timestamp: now,
                        decay_rate: 0.01,
                        confidence: m.priority,
                        embedding,
                    });
                    // ADR-0052: emit memory_store event
                    let preview = if value_str.len() > 30 { &value_str[..30] } else { &value_str };
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
                    self.memory.lock().unwrap().forget(&query_str, cutoff);
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
                        learnable.few_shot.push((input_str.clone(), output_str.clone()));
                    } else {
                        return Err(format!("adapt: learnable pattern '{}' not found", a.pattern_name));
                    }
                    // ADR-0051: update pattern stats for inspect()
                    {
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0);
                        let mut stats = self.pattern_stats.lock().unwrap();
                        let entry = stats.entry(a.pattern_name.clone()).or_insert_with(PatternStats::new);
                        entry.last_adapt = now;
                        entry.examples_count += 1;
                    }
                    // Phase 7.5: Audit log for adapt operations
                    self.push_audit(format!(
                        "[AUDIT] adapt {}: {} -> {}",
                        a.pattern_name, input_str, output_str
                    ));
                    // ADR-0052: emit adapt event
                    let examples_count = self.learnable_patterns.get(&a.pattern_name)
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
                    self.variables.insert(fl.name.clone(), Value::Fluid(variants));
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
                    let _ = self.kg.lock().unwrap().relate(&from_str, &to_str, &r.relation, 1.0);
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
                    self.module_namespaces.insert(t.name.clone(), format!("tool:{}", t.name));
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
    fn instantiate_struct(&self, type_name: &str, inits: &[FieldInit]) -> Result<Value, String> {
        let struct_type = self.struct_types.get(type_name)
            .ok_or_else(|| format!("unknown struct type: {}", type_name))?
            .clone();

        let mut fields = HashMap::new();
        for fd in &struct_type.fields {
            // Look for an initializer
            let init_val = inits.iter()
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

        Ok(Value::Struct { type_name: type_name.to_string(), fields })
    }

    /// Handle an import declaration: load module, register namespace or merge globally.
    fn handle_import(&mut self, import: &ImportDecl) -> Result<(), String> {
        let module_path = &import.path;

        // Register namespace mapping (alias or path itself for global merge)
        let alias = import.alias.as_ref().cloned().unwrap_or_else(|| module_path.clone());
        self.module_namespaces.insert(alias.clone(), module_path.clone());

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
        let file_path = if module_path.starts_with("./") {
            self.base_dir.join(format!("{}.mlog", module_path))
        } else if module_path.starts_with("std/") {
            self.base_dir.join(format!("{}.mlog", module_path))
        } else {
            self.base_dir.join(format!("{}.mlog", module_path))
        };

        let source = std::fs::read_to_string(&file_path).map_err(|e| {
            format!("cannot import module '{}': {} (tried {:?})", module_path, e, file_path)
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
                    self.struct_types.insert(e.name.clone(), StructType {
                        name: e.name.clone(),
                        fields: e.fields,
                    });
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
                    self.patterns.insert(p.name.clone(), CompiledPattern {
                        params: p.params.clone(),
                        body: p.body.clone(),
                    });
                }
                Declaration::LearnablePattern(lp) => {
                    self.learnable_patterns.insert(lp.name.clone(), CompiledLearnable {
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
                    });
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
                    let _ = self.memory.lock().unwrap().memorize(MemoryEntry {
                        id: None,
                        value: value_str.clone(),
                        priority: m.priority,
                        timestamp: now,
                        decay_rate: 0.01,
                        confidence: m.priority,
                        embedding,
                    });
                    // ADR-0052: emit memory_store event
                    let preview = if value_str.len() > 30 { &value_str[..30] } else { &value_str };
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
                    self.memory.lock().unwrap().forget(&query_str, cutoff);
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
                    self.variables.insert(fl.name.clone(), Value::Fluid(variants));
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
                    let _ = self.kg.lock().unwrap().relate(&from_str, &to_str, &r.relation, 1.0);
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
                        let mut stats = self.pattern_stats.lock().unwrap();
                        let entry = stats.entry(a.pattern_name.clone()).or_insert_with(PatternStats::new);
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
                Declaration::Hook(h) => {
                    match h.phase {
                        HookPhase::BeforePattern => self.hooks_before.push(h),
                        HookPhase::AfterPattern => self.hooks_after.push(h),
                        HookPhase::OnSessionStart => self.hooks_session_start.push(h),
                        HookPhase::OnWrite => self.hooks_on_write.push(h),
                        HookPhase::OnSessionEnd => self.hooks_session_end.push(h),
                    }
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
                    self.module_namespaces.insert(t.name.clone(), format!("tool:{}", t.name));
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

        Ok(())
    }

    /// Execute all rules in priority order (highest first, then declaration order).
    fn execute_rules(&mut self) -> Result<(), String> {
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
                    let entity = self.variables.get_mut(name)
                        .ok_or_else(|| format!("rule target '{}' not found", name))?;
                    entity.set_field(&rule.field, value_val)?;
                }
            }
        }
        Ok(())
    }

    /// Evaluate a rule condition.
    fn eval_condition(&self, cond: &Condition, env: &HashMap<String, Value>) -> Result<bool, String> {
        match cond {
            Condition::Contains { left, right } => {
                let lv = self.eval_expr_with_env(left, env)?;
                let rv = self.eval_expr_with_env(right, env)?;
                let ls = match &lv {
                    Value::String(s) => s.clone(),
                    other => return Err(format!("contains: left must be String, got {}", other.type_name())),
                };
                let rs = match &rv {
                    Value::String(s) => s.clone(),
                    other => return Err(format!("contains: right must be String, got {}", other.type_name())),
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
        if let Some(ref conn) = *self.checkpoint_db.lock().map_err(|e| format!("checkpoint lock: {}", e))? {
            conn.execute(
                "INSERT OR REPLACE INTO checkpoints (flow_name, checkpoint_name, step_index, state_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![flow_name, checkpoint_name, step_index as i64, state_json, ts],
            ).map_err(|e| format!("checkpoint save error: {}", e))?;
        } else {
            // Fallback: in-memory
            let key = format!("{}:{}", flow_name, checkpoint_name);
            self.checkpoint_mem.lock().map_err(|e| format!("checkpoint lock: {}", e))?.insert(key, data);
        }

        Ok(())
    }

    /// ADR-0056: Load a checkpoint for a flow. Returns None if not found.
    fn load_checkpoint(&self, flow_name: &str, checkpoint_name: &str) -> Result<Option<CheckpointData>, String> {
        // Try SQLite first
        if let Some(ref conn) = *self.checkpoint_db.lock().map_err(|e| format!("checkpoint lock: {}", e))? {
            let mut stmt = conn.prepare(
                "SELECT state_json FROM checkpoints WHERE flow_name = ?1 AND checkpoint_name = ?2"
            ).map_err(|e| format!("checkpoint load error: {}", e))?;

            let result: Result<String, _> = stmt.query_row(
                rusqlite::params![flow_name, checkpoint_name],
                |row| row.get(0),
            );

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
            Ok(self.checkpoint_mem.lock().map_err(|e| format!("checkpoint lock: {}", e))?.get(&key).cloned())
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
        if let Some(ref conn) = *self.checkpoint_db.lock().map_err(|e| format!("checkpoint lock: {}", e))? {
            let mut stmt = conn.prepare(
                "SELECT checkpoint_name, step_index, created_at FROM checkpoints WHERE flow_name = ?1 ORDER BY step_index"
            ).map_err(|e| format!("checkpoint list error: {}", e))?;

            let rows = stmt.query_map(rusqlite::params![flow_name], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize, row.get::<_, i64>(2)?))
            }).map_err(|e| format!("checkpoint list error: {}", e))?
              .collect::<Result<Vec<_>, _>>()
              .map_err(|e| format!("checkpoint list error: {}", e))?;

            Ok(rows)
        } else {
            // In-memory fallback
            let mem = self.checkpoint_mem.lock().map_err(|e| format!("checkpoint lock: {}", e))?;
            let prefix = format!("{}:", flow_name);
            let mut results: Vec<(String, usize, i64)> = mem.iter()
                .filter(|(k, _)| k.starts_with(&prefix))
                .map(|(_k, v)| (v.checkpoint_name.clone(), v.step_index, v.created_at))
                .collect();
            results.sort_by_key(|(_, idx, _)| *idx);
            Ok(results)
        }
    }

    /// ADR-0056: Delete a specific checkpoint (public for tests and cleanup).
    pub fn delete_checkpoint(&self, flow_name: &str, checkpoint_name: &str) -> Result<(), String> {
        if let Some(ref conn) = *self.checkpoint_db.lock().map_err(|e| format!("checkpoint lock: {}", e))? {
            conn.execute(
                "DELETE FROM checkpoints WHERE flow_name = ?1 AND checkpoint_name = ?2",
                rusqlite::params![flow_name, checkpoint_name],
            ).map_err(|e| format!("checkpoint delete error: {}", e))?;
        } else {
            let key = format!("{}:{}", flow_name, checkpoint_name);
            self.checkpoint_mem.lock().map_err(|e| format!("checkpoint lock: {}", e))?.remove(&key);
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
    fn run_flow(&mut self, flow: &FlowDecl) -> Result<String, String> {
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
                    return Err(format!("checkpoint '{}' not found for flow '{}'", target_cp, flow.name));
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
            checkpoint_at.entry(step_idx).or_default().push(cp_name.clone());
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
    fn eval_branch_condition(&self, cond: &BranchCondition, current: &Value) -> Result<bool, String> {
        // The target in branch_condition is the flow input value (current)
        let field_val = current.get_field(&cond.field)
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
            let param_name = args.get(0)
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
                _ => return Err("resolve_skill_index() expects a department name (String)".to_string()),
            };
            let idx = self.skill_indices.get(&dept)
                .ok_or_else(|| format!("resolve_skill_index(): no skill_index declared for '{}'", dept))?;
            let mut fields = HashMap::new();
            let tier1: Vec<Value> = idx.tiers.iter()
                .filter(|t| t.mode == "always")
                .flat_map(|t| t.skills.iter().map(|s| Value::String(s.clone())))
                .collect();
            fields.insert("tier1".to_string(), Value::List(tier1));
            for tier in &idx.tiers {
                if tier.mode == "when_matches" {
                    let rules: Vec<Value> = tier.rules.iter().map(|r| {
                        let mut f = HashMap::new();
                        f.insert("skill".to_string(), Value::String(r.skill.clone()));
                        f.insert("triggers".to_string(), Value::List(
                            r.triggers.iter().map(|t| Value::String(t.clone())).collect()
                        ));
                        Value::Struct { type_name: "TriggerRule".to_string(), fields: f }
                    }).collect();
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
                fields.insert("truncation".to_string(), Value::String(mode_str.to_string()));
            }
            return Ok(Value::Struct { type_name: format!("SkillIndex_{}", dept), fields });
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
                    if matches!(name,
                        "read_file" | "write_file" | "append_file"
                        | "delete_file" | "file_exists" | "list_dir"
                    ) {
                        return Err(format!(
                            "filesystem access forbidden in sandbox '{}'",
                            sb.name
                        ));
                    }
                    // Наряд №17 Г.2: also enforce exec() in sandbox
                    if name == "exec" {
                        return Err(format!(
                            "exec() forbidden in sandbox '{}'",
                            sb.name
                        ));
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
            None => return Ok(Value::String(format!("[ERROR: unknown function '{}']", name))),
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
        "mem_set", "mtree_store", "db_execute", "write_file", "append_file",
    ];

    fn is_write_builtin(name: &str) -> bool {
        Self::WRITE_BUILTINS.contains(&name)
    }

    /// Injects hook variables: pattern_name (String), args (List),
    /// result (after only), confidence (after only).
    /// Builtins are NOT wrapped — only user-defined patterns and learnable patterns.
    fn invoke_pattern_with_hooks<F>(
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
                        Value::Fluid(variants) => {
                            variants.iter().map(|v| v.confidence).fold(0.0_f64, f64::max)
                        }
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
                        let query = match self.eval_expr_with_env(query_expr, &mut env) {
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
                            let score = if entry.embedding.is_empty() || query_embedding.is_empty() {
                                if entry.value.to_lowercase().contains(&query.to_lowercase()) {
                                    0.5
                                } else {
                                    0.0
                                }
                            } else {
                                cosine_similarity(&query_embedding, &entry.embedding) as f32
                                    * entry.confidence as f32
                                    * entry.priority as f32
                            };
                            if score >= min_conf {
                                seen.insert(entry.value.clone());
                                scored.push((entry.value, score));
                            }
                        }
                        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
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
    fn compress_context(&self, context_block: &str) -> String {
        let backend = llm::create_llm_backend();
        let summary_prompt = format!(
            "Summarize the following facts concisely. Retain key information. \
             Output a single paragraph.\n\n{}",
            context_block
        );
        match backend.call(&summary_prompt, "") {
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
    fn invoke_learnable_with_env(&self, pattern_name: &str, learnable: &CompiledLearnable, args: &[Value]) -> Result<Value, String> {
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
                return Err(format!(
                    "network access forbidden in sandbox '{}'",
                    sb.name
                ));
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
        let resolved_model = learnable.model.as_ref().map(|alias| llm::resolve_model(alias));
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
                return Err(format!(
                    "operation timed out in sandbox '{}'",
                    sb.name
                ));
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
            let mut cache = self.llm_cache.lock().map_err(|e| format!("llm_cache lock error: {}", e))?;
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
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
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
        let effective_ttl = if entry.ttl > 0 { entry.ttl as i64 } else { ttl as i64 };
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
                Value::Struct { type_name: "Json".to_string(), fields }
            }
        }
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
            other => return Err(format!("recall() expected String argument, got {}", other.type_name())),
        };

        let min_confidence = if args.len() > 1 {
            args[1].as_float().unwrap_or(0.3) as f32
        } else {
            0.3
        };

        // Embed the query for semantic search
        let query_embedding = self.embedding_manager.embed(&query).unwrap_or_default();

        // Use MemoryStore trait for recall (handles both InMemory and SQLite)
        match self.memory.lock().unwrap().recall(&query, &query_embedding, min_confidence) {
            Some((entry, _score)) => {
                // Walk the knowledge graph for related memories
                let edges = self.kg.lock().unwrap().edges_for(&entry.value);
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
            _ => return Err("find() requires operator as third argument (String: gt/lt/ge/le/eq)".to_string()),
        };
        let threshold = match args.get(3) {
            Some(Value::Float(f)) => *f,
            _ => return Err("find() requires threshold as fourth argument (Float)".to_string()),
        };

        // Search all variables for entities of the matching type
        for (_name, value) in &self.variables {
            if let Value::Struct { type_name: tn, fields } = value {
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
            other => return Err(format!("memorize() expected String as first arg, got {}", other.type_name())),
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
        match self.memory.lock().unwrap().memorize(MemoryEntry {
            id: None,
            value: value_str.clone(),
            priority,
            timestamp: now,
            decay_rate: 0.01,
            confidence: priority,
            embedding,
        }) {
            Ok(id) => { /* Bug 2.3 fix: removed eprintln stdout leak in HTTP context */ },
            Err(_) => { /* silent — don't leak to stdout in HTTP context */ },
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
            other => return Err(format!("forget() expected String as first arg, got {}", other.type_name())),
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
        self.memory.lock().unwrap().forget(&query_str, cutoff);
        Ok(Value::Unit)
    }

    /// Handle a mutate declaration: replace few-shot examples, compute mock accuracy, decide keep/rollback.
    fn handle_mutate(&mut self, m: &MutateDecl) -> Result<String, String> {
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
        let learnable = self.learnable_patterns.get_mut(&m.pattern_name)
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
                    CompareOp::Gt => false,  // accuracy >= threshold is the "kept" condition
                    CompareOp::Ge => false,
                    CompareOp::Eq => (accuracy - threshold).abs() < 1e-9,
                    CompareOp::Ne => (accuracy - threshold).abs() >= 1e-9,
                }
            }
            _ => true, // No rollback condition → always keep
        };

        if kept {
            // Keep the new examples (already in place)
            let msg = Ok(format!("[MUTATE] {}: accuracy={}, kept (>= {:.1})",
                m.pattern_name, accuracy,
                m.rollback_threshold.unwrap_or(0.0)));
            // Phase 7.5: Audit log for mutate operations (after releasing mutable borrow)
            self.push_audit(format!(
                "[AUDIT] mutate {}: {} examples, accuracy={}",
                m.pattern_name, num_examples, accuracy
            ));
            msg
        } else {
            // Rollback: restore original few-shot
            let learnable = self.learnable_patterns.get_mut(&m.pattern_name)
                .ok_or_else(|| format!("mutate: learnable pattern '{}' not found", m.pattern_name))?;
            learnable.few_shot = original_few_shot;
            let msg = Ok(format!("[MUTATE] {}: accuracy={}, rolled back (below {:.1})",
                m.pattern_name, accuracy,
                m.rollback_threshold.unwrap_or(0.0)));
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
        self.audit_log.get_mut().unwrap().drain(..).collect()
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
        let learnable = self.learnable_patterns.get(&eval_decl.pattern_name)
            .ok_or_else(|| format!(
                "eval: learnable pattern '{}' not found",
                eval_decl.pattern_name
            ))?;

        let mut correct = 0usize;
        let mut confusion: HashMap<String, HashMap<String, usize>> = HashMap::new();
        let mut failures: Vec<(String, String, String)> = Vec::new(); // (input, expected, actual)

        for (input_str, expected_label) in &eval_decl.dataset {
            // Build args from dataset input (single String argument)
            let args = vec![Value::String(input_str.clone())];

            // Invoke the learnable pattern
            let actual_value = self.invoke_learnable_with_env(&eval_decl.pattern_name, learnable, &args)?;
            let actual_label = match actual_value {
                Value::String(s) => s.trim().to_string(),
                other => format!("{}", other),
            };

            // Record in confusion matrix
            let pred_entry = confusion.entry(expected_label.clone())
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

    /// Get a reference to the registered learnable patterns (for eval).
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
        let cyrillic_count = text.chars()
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
            let entry = stats.entry(name.to_string()).or_insert_with(PatternStats::new);
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
        data.insert("cache_hit".to_string(), if cache_hit { "true".to_string() } else { "false".to_string() });
        self.emit_event("pattern_call", name, data, None);
    }

    /// Invoke the `inspect()` builtin (ADR-0051).
    /// Returns a Struct with pattern metadata: calls, avg_confidence, cache_hits,
    /// cache_misses, last_adapt, last_call, examples_count, is_learnable.
    fn invoke_inspect(&self, args: &[Value]) -> Result<Value, String> {
        let pattern_name = match args.first() {
            Some(Value::String(s)) => s.clone(),
            Some(other) => return Err(format!("inspect() expected String pattern name, got {}", other.type_name())),
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
            Ok(stats) => stats.get(&pattern_name).cloned().unwrap_or_else(PatternStats::new),
            Err(_) => PatternStats::new(),
        };

        // If the pattern exists in learnable_patterns, get its current few_shot count
        // (which may differ from examples_count if few-shot was added outside adapt)
        let actual_examples = self.learnable_patterns.get(&pattern_name)
            .map(|lp| lp.few_shot.len() as u64)
            .unwrap_or(stats.examples_count);

        let cache_misses = stats.calls.saturating_sub(stats.cache_hits);

        let mut fields = std::collections::HashMap::new();
        fields.insert("calls".to_string(), Value::Float(stats.calls as f64));
        fields.insert("avg_confidence".to_string(), Value::Float(stats.avg_confidence()));
        fields.insert("cache_hits".to_string(), Value::Float(stats.cache_hits as f64));
        fields.insert("cache_misses".to_string(), Value::Float(cache_misses as f64));
        fields.insert("last_adapt".to_string(), Value::Float(stats.last_adapt as f64));
        fields.insert("last_call".to_string(), Value::Float(stats.last_call as f64));
        fields.insert("examples_count".to_string(), Value::Float(actual_examples as f64));
        fields.insert("is_learnable".to_string(), Value::Float(if is_learnable { 1.0 } else { 0.0 }));

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

    // ── ADR-0053: Conversation builtins ──────────────────────────────────

    /// `conv_start(id)` — create or open a conversation. Returns the conversation id.
    fn invoke_conv_start(&self, args: &[Value]) -> Result<Value, String> {
        let id = match args.get(0) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("conv_start() requires 1 argument (id: String)".to_string()),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let mut convs = self.conversations.lock()
            .map_err(|e| format!("conv_start() lock error: {}", e))?;
        convs.entry(id.clone()).or_insert_with(|| Conversation {
            id: id.clone(),
            messages: Vec::new(),
            created_at: now,
            last_active: now,
            metadata: HashMap::new(),
        });
        Ok(Value::String(id))
    }

    /// `conv_add(id, role, text)` — add a message to a conversation.
    fn invoke_conv_add(&self, args: &[Value]) -> Result<Value, String> {
        let id = match args.get(0) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("conv_add() requires 3 arguments (id, role, text)".to_string()),
        };
        let role = match args.get(1) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("conv_add() requires 3 arguments (id, role, text)".to_string()),
        };
        let text = match args.get(2) {
            Some(Value::String(s)) => s.clone(),
            Some(other) => format!("{}", other),
            None => return Err("conv_add() requires 3 arguments (id, role, text)".to_string()),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut convs = self.conversations.lock()
            .map_err(|e| format!("conv_add() lock error: {}", e))?;
        let conv = convs.get_mut(&id)
            .ok_or_else(|| format!("conv_add() conversation '{}' not found", id))?;

        // Enforce max_messages: if at limit, remove oldest message
        if conv.messages.len() >= self.conversation_config.max_messages {
            conv.messages.remove(0);
        }

        conv.messages.push(ConvMessage {
            role,
            text: text.clone(),
            timestamp: now,
        });
        conv.last_active = now;

        // ADR-0053: auto-compress when message count exceeds compress_after
        if conv.messages.len() > self.conversation_config.compress_after {
            self.compress_conversation(conv);
        }

        Ok(Value::String(text))
    }

    /// `conv_history(id)` — return the full message history as a List of Structs.
    fn invoke_conv_history(&self, args: &[Value]) -> Result<Value, String> {
        let id = match args.get(0) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("conv_history() requires 1 argument (id: String)".to_string()),
        };
        let convs = self.conversations.lock()
            .map_err(|e| format!("conv_history() lock error: {}", e))?;
        let conv = convs.get(&id)
            .ok_or_else(|| format!("conv_history() conversation '{}' not found", id))?;

        let mut list = Vec::new();
        for msg in &conv.messages {
            let mut fields = HashMap::new();
            fields.insert("role".to_string(), Value::String(msg.role.clone()));
            fields.insert("text".to_string(), Value::String(msg.text.clone()));
            fields.insert("timestamp".to_string(), Value::Float(msg.timestamp as f64));
            list.push(Value::Struct { type_name: "Message".to_string(), fields });
        }
        Ok(Value::List(list))
    }

    /// `conv_context(id)` — return a formatted string of conversation history for LLM injection.
    fn invoke_conv_context(&self, args: &[Value]) -> Result<Value, String> {
        let id = match args.get(0) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("conv_context() requires 1 argument (id: String)".to_string()),
        };
        let convs = self.conversations.lock()
            .map_err(|e| format!("conv_context() lock error: {}", e))?;
        let conv = convs.get(&id)
            .ok_or_else(|| format!("conv_context() conversation '{}' not found", id))?;

        let mut parts = Vec::new();
        for msg in &conv.messages {
            parts.push(format!("{}: {}", msg.role, msg.text));
        }
        Ok(Value::String(parts.join("\n")))
    }

    /// `conv_end(id)` — terminate a conversation. Returns "ok".
    fn invoke_conv_end(&self, args: &[Value]) -> Result<Value, String> {
        let id = match args.get(0) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("conv_end() requires 1 argument (id: String)".to_string()),
        };
        let mut convs = self.conversations.lock()
            .map_err(|e| format!("conv_end() lock error: {}", e))?;
        convs.remove(&id);
        Ok(Value::String("ok".to_string()))
    }

    /// Get a reference to the conversations store (for testing).
    pub fn get_conversations(&self) -> &std::sync::Mutex<HashMap<String, Conversation>> {
        &self.conversations
    }

    /// Get conversation config (for testing).
    pub fn get_conversation_config(&self) -> &ConversationConfig {
        &self.conversation_config
    }

    /// Compress older messages in a conversation by summarizing them via LLM.
    /// Replaces messages beyond compress_after with a single system summary message.
    fn compress_conversation(&self, conv: &mut Conversation) {
        if conv.messages.len() <= self.conversation_config.compress_after {
            return;
        }
        let old_count = conv.messages.len() - self.conversation_config.compress_after;
        let old_messages: Vec<ConvMessage> = conv.messages.drain(..old_count).collect();

        // Build text from old messages for summarization
        let old_text: Vec<String> = old_messages.iter()
            .map(|m| format!("{}: {}", m.role, m.text))
            .collect();
        let text_to_summarize = old_text.join("\n");

        // Attempt LLM summarization. On failure, keep a simple prefix summary.
        let summary = match self.summarize_conversation(&text_to_summarize) {
            Ok(s) => s,
            Err(_) => format!("[Previous conversation summary: {} messages omitted]", old_count),
        };

        // Prepend summary as a system message
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        conv.messages.insert(0, ConvMessage {
            role: "system".to_string(),
            text: summary,
            timestamp: now,
        });
    }

    /// Summarize conversation text via LLM call.
    fn summarize_conversation(&self, text: &str) -> Result<String, String> {
        let backend = llm::create_llm_backend();
        let prompt = "Summarize this conversation concisely, preserving key facts and decisions.";
        backend.call(prompt, text)
    }

    /// Get conversation history as a formatted string for LLM multi-turn injection.
    /// Returns None if conversation not found or empty.
    pub fn get_conversation_for_llm(&self, conv_id: &str) -> Option<String> {
        let convs = self.conversations.lock().ok()?;
        let conv = convs.get(conv_id)?;
        if conv.messages.is_empty() {
            return None;
        }
        let mut parts = Vec::new();
        for msg in &conv.messages {
            parts.push(format!("{}: {}", msg.role, msg.text));
        }
        Some(parts.join("\n"))
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
                name, pattern.params.len(), args.len()
            ));
        }
        let mut local_env = self.bind_and_collapse(&pattern.params, args)?;
        self.invoke_pattern_with_hooks(name, args, || {
            self.eval_statements(&pattern.body, &mut local_env)
        })
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
            target.learnable_patterns.entry(k.clone()).or_insert(v.clone());
        }
        for r in &self.rules {
            target.rules.push(r.clone());
        }
        for (k, v) in &self.sandboxes {
            target.sandboxes.entry(k.clone()).or_insert(v.clone());
        }
        for (k, v) in &self.module_namespaces {
            target.module_namespaces.entry(k.clone()).or_insert(v.clone());
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
                Statement::LetBinding { name, value, mutable } => {
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
                Statement::Each { variable, iterable, body } => {
                    let iter_val = self.eval_expr_with_env(iterable, env)?;
                    let items = match iter_val {
                        Value::List(items) => items,
                        other => return Err(format!("each: expected List, got {}", other.type_name())),
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
                            ControlFlow::Break => return Ok(ControlFlow::Break),
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
                Statement::EachWithIndex { index_var, item_var, iterable, body } => {
                    let iter_val = self.eval_expr_with_env(iterable, env)?;
                    let items = match iter_val {
                        Value::List(items) => items,
                        other => return Err(format!("each: expected List, got {}", other.type_name())),
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
                            ControlFlow::Break => return Ok(ControlFlow::Break),
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
                            ControlFlow::ContinueLoop => { iterations += 1; continue; }
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
                Statement::IfElseBlock { condition, then_body, else_ifs, else_body } => {
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
                Statement::Match { scrutinee, ref arms, ref else_body } => {
                    let scrutinee_val = self.eval_expr_with_env(scrutinee, env)?;
                    let scrutinee_str = format!("{}", scrutinee_val);
                    let mut matched = false;
                    for arm in arms {
                        let arm_matches = match arm {
                            MatchArm::Exact(val, _) => scrutinee_str == *val,
                            MatchArm::StartsWith(prefix, _) => scrutinee_str.starts_with(prefix.as_str()),
                            MatchArm::Contains(substr, _) => scrutinee_str.contains(substr.as_str()),
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
                Ok(Value::Struct { type_name: "Struct".to_string(), fields: resolved })
            }
            // Наряд №14 P0-3: block if/else as expression
            Expr::BlockIfElse { condition, ref then_body, ref else_ifs, ref else_body } => {
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
            Expr::Try(inner) => {
                match self.eval_expr_with_env(inner, env) {
                    Ok(val) => Ok(val),
                    Err(e) => {
                        eprintln!("[try] caught error: {}", e);
                        Ok(Value::Unit)
                    }
                }
            }
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
                            items.get(idx).cloned()
                                .ok_or_else(|| format!("list index out of bounds: {}", idx))
                        } else {
                            items.get(idx as usize).cloned()
                                .ok_or_else(|| format!("list index out of bounds: {}", idx))
                        }
                    }
                    (Value::Struct { fields, .. }, Value::String(key)) => {
                        fields.get(key).cloned()
                            .ok_or_else(|| format!("struct has no field '{}'", key))
                    }
                    (Value::String(s), Value::Float(f)) => {
                        let idx = *f as isize;
                        if idx < 0 {
                            // Unicode-aware: use chars().count() for character length, not s.len() (bytes)
                            let char_len = s.chars().count();
                            let abs_idx = char_len.wrapping_sub((-idx) as usize);
                            Ok(Value::String(s.chars().nth(abs_idx).unwrap_or('\0').to_string()))
                        } else {
                            Ok(Value::String(s.chars().nth(idx as usize).unwrap_or('\0').to_string()))
                        }
                    }
                    _ => Err(format!(
                        "index access: expected List[Int] or Struct[String], got {}[{}]",
                        base_val.type_name(), idx_val.type_name()
                    )),
                }
            }
            Expr::QualifiedCall { module, function, args } => {
                let mut eval_args = Vec::new();
                for arg in args {
                    eval_args.push(self.eval_expr_with_env(arg, env)?);
                }
                // Verify the module namespace was imported
                if !self.module_namespaces.contains_key(module) {
                    return Err(format!("undefined module: '{}' (did you import it?)", module));
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
                            if matches!(function.as_str(),
                                "read_file" | "write_file" | "append_file"
                                | "delete_file" | "file_exists" | "list_dir"
                            ) {
                                return Err(format!(
                                    "filesystem access forbidden in sandbox '{}'",
                                    sb.name
                                ));
                            }
                            // Наряд №17 Г.2: also enforce exec() in sandbox
                            if function == "exec" {
                                return Err(format!(
                                    "exec() forbidden in sandbox '{}'",
                                    sb.name
                                ));
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
                    self.patterns.get(&qualified_key)
                        .cloned()
                        .ok_or_else(|| format!("undefined method '{}' in tool '{}'", function, module))?
                } else {
                    match self.patterns.get(function) {
                        Some(p) => p.clone(),
                        None => return Err(format!("undefined pattern '{}' in module '{}'", function, module)),
                    }
                };
                if eval_args.len() != pattern.params.len() {
                    return Err(format!(
                        "pattern {} expects {} arguments, got {}",
                        function, pattern.params.len(), eval_args.len()
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
                        Some(other) => return Err(format!("events_since() expected Float, got {}", other.type_name())),
                        None => return Err("events_since() requires 1 argument (seconds)".to_string()),
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
                        fields.insert("data_json".to_string(), Value::String(format!("{:?}", ev.data)));
                        if let Some(dur) = ev.duration_ms {
                            fields.insert("duration_ms".to_string(), Value::Float(dur as f64));
                        }
                        list.push(Value::Struct { type_name: "Event".to_string(), fields });
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
                    let param_name = eval_args.get(0)
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
                    let role = eval_args.get(0)
                        .and_then(|v| match v {
                            Value::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or_default();
                    if self.server_user_roles.contains(&role) {
                        return Ok(Value::Bool(true));
                    }
                    return Err(format!("require('{}'): access denied — user has roles {:?}", role, self.server_user_roles));
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
                        _ => return Err("map() expects second argument to be a pattern name (String)".to_string()),
                    };
                    let pattern = match self.patterns.get(&pattern_name) {
                        Some(p) => p.clone(),
                        None => return Err(format!("map(): pattern '{}' not found", pattern_name)),
                    };
                    if pattern.params.len() != 1 {
                        return Err(format!("map(): pattern '{}' must accept exactly 1 argument, got {}", pattern_name, pattern.params.len()));
                    }
                    let mut results = Vec::new();
                    for item in &list {
                        let mut local_env = self.bind_and_collapse(&pattern.params, &[item.clone()])?;
                        let result = self.eval_statements(&pattern.body, &mut local_env)?;
                        results.push(result);
                    }
                    return Ok(Value::List(results));
                }

                // Problem C (reverse-iteration): db_insert(table, struct) — needs db_conn
                if name == "db_insert" {
                    let table = match eval_args.get(0) {
                        Some(Value::String(s)) => s.clone(),
                        _ => return Err("db_insert() expects first argument to be a table name (String)".to_string()),
                    };
                    let fields = match eval_args.get(1) {
                        Some(Value::Struct { fields, .. }) => fields.clone(),
                        _ => return Err("db_insert() expects second argument to be a Struct { field: value, ... }".to_string()),
                    };
                    let guard = self.db_conn.lock().map_err(|e| format!("db lock error: {}", e))?;
                    let conn = guard.as_ref().ok_or_else(|| {
                        "db_insert() error: no database connection. Declare db { url: \"sqlite::memory:\" } first.".to_string()
                    })?;
                    let col_names: Vec<String> = fields.keys().cloned().collect();
                    let placeholders: Vec<String> = col_names.iter().map(|_| "?".to_string()).collect();
                    let sql = format!("INSERT INTO {} ({}) VALUES ({})",
                        table,
                        col_names.join(", "),
                        placeholders.join(", "));
                    let params: Vec<Box<dyn rusqlite::types::ToSql>> = fields.values().map(|v| {
                        match v {
                            Value::String(s) => Box::new(s.clone()) as Box<dyn rusqlite::types::ToSql>,
                            Value::Float(f) => Box::new(*f) as Box<dyn rusqlite::types::ToSql>,
                            Value::Bool(b) => Box::new(*b) as Box<dyn rusqlite::types::ToSql>,
                            Value::Unit => Box::new(Option::<String>::None) as Box<dyn rusqlite::types::ToSql>,
                            other => Box::new(format!("{}", other)) as Box<dyn rusqlite::types::ToSql>,
                        }
                    }).collect();
                    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
                    conn.execute(&sql, param_refs.as_slice())
                        .map_err(|e| format!("db_insert() SQL error: {}", e))?;
                    // Return last inserted rowid
                    let rowid: i64 = conn.query_row("SELECT last_insert_rowid()", [], |row| row.get(0))
                        .unwrap_or(0);
                    return Ok(Value::Float(rowid as f64));
                }

                // Problem A: resolve_skill_index(dept) — returns registered index as Value::Struct
                if name == "resolve_skill_index" {
                    let dept = match eval_args.get(0) {
                        Some(Value::String(s)) => s.clone(),
                        _ => return Err("resolve_skill_index() expects a department name (String)".to_string()),
                    };
                    let idx = self.skill_indices.get(&dept)
                        .ok_or_else(|| format!("resolve_skill_index(): no skill_index declared for '{}'", dept))?;
                    // Convert to Value::Struct for field access
                    let mut fields = HashMap::new();
                    // tier1: list of always-skill names
                    let tier1: Vec<Value> = idx.tiers.iter()
                        .filter(|t| t.mode == "always")
                        .flat_map(|t| t.skills.iter().map(|s| Value::String(s.clone())))
                        .collect();
                    fields.insert("tier1".to_string(), Value::List(tier1));
                    // tier2+: list of trigger-rule structs
                    for tier in &idx.tiers {
                        if tier.mode == "when_matches" {
                            let rules: Vec<Value> = tier.rules.iter().map(|r| {
                                let mut f = HashMap::new();
                                f.insert("skill".to_string(), Value::String(r.skill.clone()));
                                f.insert("triggers".to_string(), Value::List(
                                    r.triggers.iter().map(|t| Value::String(t.clone())).collect()
                                ));
                                Value::Struct { type_name: "TriggerRule".to_string(), fields: f }
                            }).collect();
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
                        fields.insert("truncation".to_string(), Value::String(mode_str.to_string()));
                    }
                    return Ok(Value::Struct { type_name: format!("SkillIndex_{}", dept), fields });
                }

                // Problem A: fit_to_budget(list, budget, mode) — MVP: return list as-is
                if name == "fit_to_budget" {
                    let list = match eval_args.get(0) {
                        Some(Value::List(items)) => items.clone(),
                        _ => return Err("fit_to_budget() expects first argument to be a List".to_string()),
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
                            let template_name = eval_args.first()
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
                    None => return Ok(Value::String(format!("[ERROR: unknown function '{}']", name))),
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
                    if !Self::is_truthy(&l) { return Ok(Value::Bool(false)); }
                    let r = self.eval_expr_with_env(right, env)?;
                    return Ok(Value::Bool(Self::is_truthy(&r)));
                }
                if matches!(op, BinOp::Or) {
                    let l = self.eval_expr_with_env(left, env)?;
                    if Self::is_truthy(&l) { return Ok(Value::Bool(true)); }
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
                let best = variants.iter()
                    .filter(|v| v.type_name == required_type)
                    .max_by(|a, b| {
                        a.confidence.partial_cmp(&b.confidence)
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
    fn bind_and_collapse(
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
    fn collapse_args(&self, params: &[Param], args: &[Value]) -> Vec<Value> {
        params.iter().zip(args.iter())
            .map(|(p, a)| {
                self.maybe_collapse(a, &p.type_name).unwrap_or_else(|_| a.clone())
            })
            .collect()
    }

    /// Check if a value is an opaque type that cannot be concatenated.
    fn is_opaque_type(v: &Value) -> bool {
        matches!(v,
            Value::Html(_) | Value::Query(_) | Value::Secret(_) |
            Value::Encrypted(_) | Value::Hash(_) | Value::Subgraph(_)
        )
    }

    /// Check if a value is an opaque type that cannot be printed.
    fn is_nonprintable_type(v: &Value) -> bool {
        matches!(v,
            Value::Html(_) | Value::Query(_) | Value::Secret(_) |
            Value::Encrypted(_) | Value::Hash(_) | Value::Subgraph(_)
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
                return Err(format!("cannot concatenate opaque type {}", left.type_name()));
            }
            if Self::is_opaque_type(&right) {
                return Err(format!("cannot concatenate opaque type {}", right.type_name()));
            }
        }
        match (op, left, right) {
            // Arithmetic on Floats
            (BinOp::Add, Value::String(a), Value::String(b)) => {
                let result = format!("{}{}", a, b);
                if result.len() > Self::MAX_STRING_LENGTH {
                    return Err(format!(
                        "string length {} exceeds maximum allowed {}",
                        result.len(), Self::MAX_STRING_LENGTH
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
                eprintln!("[WARNING] implicit Unit→String conversion in '+' operation: {} + {}", l.type_name(), r.type_name());
                Err(format!(
                    "type mismatch in string concatenation: {} + {} (use to_string() explicitly)",
                    l.type_name(), r.type_name()
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
