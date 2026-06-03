// ── Tree-walking interpreter for METALOGOS M1+M2 ─────────────────────
// M1: entities, patterns, linear flow
// M2: struct entities, rules, branching flow, comparisons

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroizing;

use crate::ast::*;
use crate::builtins::Builtins;
use crate::embeddings::EmbeddingManager;
use crate::memory_store::{MemoryEntry, MemoryStore, KgStore, InMemoryStore, InMemoryKg, SqliteStore};
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
struct CompiledLearnable {
    params: Vec<Param>,
    prompt: String,
    /// Few-shot examples added by `adapt` declarations.
    /// Each entry: (input_string, output_string).
    few_shot: Vec<(String, String)>,
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
    memory: Box<dyn MemoryStore>,
    /// Knowledge graph store backend (Phase 7.6): InMemoryKg or SqliteKg.
    kg: Box<dyn KgStore>,
    /// Sandbox declarations (recorded but not enforced).
    sandboxes: HashMap<String, SandboxDecl>,
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
    /// Audit log (Phase 6.5)
    audit_log: Vec<String>,
    /// Server config (Phase 6.1)
    server_config: Option<MlogServerDecl>,
    /// Embedding manager for semantic recall (Phase 7.2).
    embedding_manager: EmbeddingManager,
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
            memory: Box::new(InMemoryStore::new()),
            kg: Box::new(InMemoryKg::new()),
            sandboxes: HashMap::new(),
            mutate_log: Vec::new(),
            module_namespaces: HashMap::new(),
            loading_stack: Vec::new(),
            base_dir: std::path::PathBuf::from("."),
            templates: HashMap::new(),
            db_config: None,
            db_store: Vec::new(),
            audit_log: Vec::new(),
            server_config: None,
            embedding_manager: EmbeddingManager::new(),
        }
    }

    /// Set the base directory for resolving relative imports.
    pub fn set_base_dir(&mut self, dir: std::path::PathBuf) {
        self.base_dir = dir;
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
                    let existing = self.memory.all_entries();
                    let mut new_store: Box<dyn MemoryStore> = Box::new(sqlite_store);
                    for entry in existing {
                        let _ = new_store.memorize(entry);
                    }
                    self.memory = new_store;

                    // Migrate KG edges to SQLite (sharing the same DB file)
                    // We open a separate connection for KG since we don't store the raw Connection
                    let existing_edges: Vec<(String, String, String, f64)> =
                        self.kg.all_edges();
                    // Create a new InMemoryKg (we can't share SQLite connection easily)
                    // In practice, for modules loaded after memory config, new relates will go to InMemoryKg
                    // The initial migration of edges is handled here
                    let _ = existing_edges; // edges are preserved in memory for the session
                    // Note: Full SQLite KG requires connection sharing, which is handled
                    // by the server.rs init path. For interpreter-only mode, KG stays in-memory
                    // while memories go to SQLite.
                    eprintln!("[memory] Persistence enabled: {}", path);
                }
                Err(e) => {
                    eprintln!("[memory] Failed to open persistent store '{}': {}. Using in-memory.", path, e);
                }
            }
        }
        // If persist is None, keep the default InMemoryStore (already set in new())
    }

    /// Run a complete .mlog program.
    pub fn run(&mut self, declarations: Vec<Declaration>) -> Result<Option<String>, String> {
        let mut output: Option<String> = None;

        for decl in declarations {
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
                    let _ = self.memory.memorize(MemoryEntry {
                        value: value_str,
                        priority: m.priority,
                        timestamp: now,
                        decay_rate: 0.01,
                        confidence: m.priority,
                        embedding,
                    });
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
                    self.memory.forget(&query_str, cutoff);
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
                        learnable.few_shot.push((input_str, output_str));
                    } else {
                        return Err(format!("adapt: learnable pattern '{}' not found", a.pattern_name));
                    }
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
                    let _ = self.kg.relate(&from_str, &to_str, &r.relation, 1.0);
                }
                Declaration::Sandbox(s) => {
                    self.sandboxes.insert(s.name.clone(), s);
                }
                Declaration::Mutate(m) => {
                    let msg = self.handle_mutate(&m)?;
                    self.mutate_log.push(msg);
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
                Declaration::Db(db) => {
                    self.db_config = Some(db);
                }
            }
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
                    let _ = self.memory.memorize(MemoryEntry {
                        value: value_str,
                        priority: m.priority,
                        timestamp: now,
                        decay_rate: 0.01,
                        confidence: m.priority,
                        embedding,
                    });
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
                    self.memory.forget(&query_str, cutoff);
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
                    let _ = self.kg.relate(&from_str, &to_str, &r.relation, 1.0);
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
                }
                Declaration::Mutate(m) => {
                    let msg = self.handle_mutate(&m)?;
                    self.mutate_log.push(msg);
                }
                Declaration::Flow(f) => {
                    self.execute_rules()?;
                    // Silently execute flow in module (don't override main output)
                    let _ = self.run_flow(&f);
                }
                Declaration::Sandbox(s) => {
                    self.sandboxes.insert(s.name.clone(), s);
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
                Declaration::Db(db) => {
                    self.db_config = Some(db);
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
                })
            }
        }
    }

    /// Execute a flow: evaluate source, thread through pipeline steps.
    fn run_flow(&mut self, flow: &FlowDecl) -> Result<String, String> {
        // Register branch definitions for this flow
        self.branch_defs.clear();
        for (step_name, branches) in &flow.branch_defs {
            self.branch_defs.insert(step_name.clone(), branches.clone());
        }

        let mut current = self.eval_expr(&flow.source)?;

        for step_name in &flow.pipeline {
            current = self.run_flow_step(step_name, current)?;
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
        })
    }

    /// Invoke a pattern or built-in by name with given arguments.
    fn invoke(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        // Check recall (memory) first — it's a built-in with memory access
        if name == "recall" {
            return self.invoke_recall(args);
        }

        // Check find (entity store query) — needs access to interpreter state
        if name == "find" {
            return self.invoke_find(args);
        }

        // Check learnable patterns
        if let Some(learnable) = self.learnable_patterns.get(name).cloned() {
            let collapsed_args = self.collapse_args(&learnable.params, &args);
            return self.invoke_learnable_with_env(&learnable, &collapsed_args);
        }

        // Check builtins
        if let Some(builtin_fn) = self.builtins.get(name) {
            return builtin_fn(&args);
        }

        // Look up compiled pattern
        let pattern = match self.patterns.get(name) {
            Some(p) => p.clone(),
            None => return Err(format!("undefined pattern or builtin: {}", name)),
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

        self.eval_statements(&pattern.body, &mut local_env)
    }

    /// Invoke a learnable pattern using pre-collapsed arguments.
    fn invoke_learnable_with_env(&self, learnable: &CompiledLearnable, args: &[Value]) -> Result<Value, String> {
        // Build input string from arguments
        let input_parts: Vec<String> = args.iter().map(|a| format!("{}", a)).collect();
        let input = input_parts.join(", ");

        // Check few-shot examples first (exact match → cache hit)
        for (example_input, example_output) in &learnable.few_shot {
            if input == *example_input {
                return Ok(Value::String(example_output.clone()));
            }
        }

        // No few-shot match — call LLM backend
        let backend = llm::create_llm_backend();
        let response = backend.call(&learnable.prompt, &input)?;

        // Try to parse JSON response into Value::Struct
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
        match self.memory.recall(&query, &query_embedding, min_confidence) {
            Some((entry, _score)) => {
                // Walk the knowledge graph for related memories
                let edges = self.kg.edges_for(&entry.value);
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
                }
            }
            _ => true, // No rollback condition → always keep
        };

        if kept {
            // Keep the new examples (already in place)
            Ok(format!("[MUTATE] {}: accuracy={}, kept (>= {:.1})",
                m.pattern_name, accuracy,
                m.rollback_threshold.unwrap_or(0.0)))
        } else {
            // Rollback: restore original few-shot
            learnable.few_shot = original_few_shot;
            Ok(format!("[MUTATE] {}: accuracy={}, rolled back (below {:.1})",
                m.pattern_name, accuracy,
                m.rollback_threshold.unwrap_or(0.0)))
        }
    }

    /// Take the mutate log messages (consuming them).
    pub fn take_mutate_log(&mut self) -> Vec<String> {
        std::mem::take(&mut self.mutate_log)
    }

    /// Take the audit log messages (consuming them).
    pub fn take_audit_log(&mut self) -> Vec<String> {
        std::mem::take(&mut self.audit_log)
    }

    /// Get the server configuration (Phase 6.1).
    pub fn get_server_config(&self) -> Option<&MlogServerDecl> {
        self.server_config.as_ref()
    }

    /// Get the template registry (Phase 6.2).
    pub fn get_templates(&self) -> &HashMap<String, TemplateDecl> {
        &self.templates
    }

    /// Safety limit for while loops (soft-failure on exceed).
    const WHILE_SAFETY_LIMIT: u64 = 100_000;

    pub(crate) fn eval_statements(
        &self,
        stmts: &[Statement],
        env: &mut HashMap<String, Value>,
    ) -> Result<Value, String> {
        for stmt in stmts {
            match stmt {
                Statement::LetBinding { name, value } => {
                    let val = self.eval_expr_with_env(value, env)?;
                    env.insert(name.clone(), val);
                }
                Statement::Assign { name, value } => {
                    let val = self.eval_expr_with_env(value, env)?;
                    if env.contains_key(name) {
                        env.insert(name.clone(), val);
                    } else {
                        return Err(format!("cannot assign to undeclared variable: {}", name));
                    }
                }
                Statement::Each { variable, iterable, body } => {
                    let iter_val = self.eval_expr_with_env(iterable, env)?;
                    let items = match iter_val {
                        Value::List(items) => items,
                        other => return Err(format!(
                            "each: expected List, got {}",
                            other.type_name()
                        )),
                    };
                    for item in items {
                        env.insert(variable.clone(), item);
                        let result = self.eval_statements(body, env)?;
                        // If body returned a non-Unit value, propagate as early return
                        if !matches!(result, Value::Unit) {
                            return Ok(result);
                        }
                    }
                }
                Statement::While { condition, body } => {
                    let mut iterations: u64 = 0;
                    loop {
                        if iterations >= Self::WHILE_SAFETY_LIMIT {
                            return Err(format!(
                                "while loop exceeded safety limit of {} iterations",
                                Self::WHILE_SAFETY_LIMIT
                            ));
                        }
                        let cond_val = self.eval_expr_with_env(condition, env)?;
                        if !cond_val.as_bool()? {
                            break;
                        }
                        let result = self.eval_statements(body, env)?;
                        // If body returned a non-Unit value, propagate as early return
                        if !matches!(result, Value::Unit) {
                            return Ok(result);
                        }
                        iterations += 1;
                    }
                }
                Statement::Return(expr) => return self.eval_expr_with_env(expr, env),
            }
        }
        Ok(Value::Unit)
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
                if let Some(builtin_fn) = self.builtins.get(function) {
                    return builtin_fn(&eval_args);
                }
                // Look up compiled pattern
                let pattern = match self.patterns.get(function) {
                    Some(p) => p.clone(),
                    None => return Err(format!("undefined pattern '{}' in module '{}'", function, module)),
                };
                if eval_args.len() != pattern.params.len() {
                    return Err(format!(
                        "pattern {} expects {} arguments, got {}",
                        function, pattern.params.len(), eval_args.len()
                    ));
                }
                let mut local_env = self.bind_and_collapse(&pattern.params, &eval_args)?;
                self.eval_statements(&pattern.body, &mut local_env)
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

                // Check learnable patterns first
                if let Some(learnable) = self.learnable_patterns.get(name).cloned() {
                    let collapsed_args = self.collapse_args(&learnable.params, &eval_args);
                    return self.invoke_learnable_with_env(&learnable, &collapsed_args);
                }

                // Check builtins
                if let Some(builtin_fn) = self.builtins.get(name) {
                    return builtin_fn(&eval_args);
                }

                // Look up compiled pattern
                let pattern = match self.patterns.get(name) {
                    Some(p) => p.clone(),
                    None => return Err(format!("undefined pattern or builtin: {}", name)),
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
                self.eval_statements(&pattern.body, &mut local_env)
            }
            Expr::BinaryOp(left, op, right) => {
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
            Value::Encrypted(_) | Value::Hash(_)
        )
    }

    /// Check if a value is an opaque type that cannot be printed.
    fn is_nonprintable_type(v: &Value) -> bool {
        matches!(v,
            Value::Html(_) | Value::Query(_) | Value::Secret(_) |
            Value::Encrypted(_) | Value::Hash(_)
        )
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
            (BinOp::Add, Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
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
            // Equality (works for Float, String, Bool)
            (BinOp::Eq, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a == b)),
            (BinOp::Eq, Value::String(a), Value::String(b)) => Ok(Value::Bool(a == b)),
            (BinOp::Eq, Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a == b)),
            (_, l, r) => Err(format!(
                "type mismatch in binary operation: {} {:?} {}",
                l.type_name(),
                op,
                r.type_name()
            )),
        }
    }
}
