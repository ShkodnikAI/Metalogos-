// ── METALOGOS Stack VM — Phase 4.1 ─────────────────────────────
//
// Executes a compiled bytecode Program. The VM maintains:
//   - A value stack (operands for instructions)
//   - A call stack (frames for pattern invocations)
//   - Global variables (slots)
//   - Pattern table (compiled user-defined functions)
//   - Learnable table (LLM-backed patterns)
//   - Memory store (memorize/recall)
//   - Knowledge graph relations
//   - Mutate log (messages from mutate declarations)
//
// Design: a single main loop that dispatches on the current instruction.
// Function calls push a new frame; Return pops back.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ast::CompareOp as AstCompareOp;
use crate::builtins::Builtins;
use crate::bytecode::*;
use crate::interpreter::{
    ConvMessage, Conversation, ConversationConfig, Event, FluidValueVariant, PatternStats, Value,
};
use crate::llm;

/// The METALOGOS stack-based virtual machine.
pub struct Vm {
    /// Runtime label environment (Наряд №328): variable → ADR-0154
    /// label. Seeded by LabelJoin, consulted by SinkCheck.
    label_env: std::collections::BTreeMap<String, crate::labels::Label>,
    /// №370: stack of VALUE-EXPRESSION registers (BeginValueExpr/
    /// KeepLastValue/EndValueExpr) — the last-value registers of block
    /// value forms live here, NOT in stack cells, so arbitrary expression
    /// positions are safe (no temporaries can be clobbered). Nested value
    /// forms nest registers; execute_code saves/restores the stack per
    /// invocation so an early Return inside a branch cannot leak a
    /// register into the caller's execution.
    value_registers: Vec<Value>,
    /// Global variable slots.
    globals: Vec<Value>,
    /// Global variable names (index = slot). Naryad #402 step A: SHARED
    /// with the Program (Arc snapshot) — read-only after load.
    global_names: std::sync::Arc<Vec<String>>,
    /// Pattern table: index → CompiledFn. Naryad #402 step A: SHARED with
    /// the Program (Arc snapshot of the main_code RegisterPattern scan);
    /// the run()-only RegisterPattern mutation goes through
    /// `Arc::make_mut` (copy-on-write — the serve path never pays it).
    patterns: std::sync::Arc<Vec<CompiledFn>>,
    /// Learnable pattern table: index → (info, few_shot, original_few_shot).
    learnables: Vec<(CompiledLearnableInfo, Vec<(String, String)>)>,
    /// Built-in function registry.
    /// №409 (step B, candidate C1): SHARED process-wide (`shared_builtins_registry`) —
    /// the registry is a pure derivation of `BUILTIN_REGISTRY` and is read-only on
    /// every VM path (handler override is an Interpreter-only affordance, №287),
    /// so each `Vm::new()` pays one Arc increment instead of rebuilding the
    /// ~460-entry map. The interpreter (TW) keeps its own owned instance — the
    /// benchmark baseline is untouched.
    builtins: std::sync::Arc<Builtins>,
    /// Builtin name lookup table (index → name).
    /// №409 (C1): shared process-wide for the same reason — one allocation
    /// for the whole process instead of ~460 `String`s per `Vm::new()`.
    builtin_names: std::sync::Arc<Vec<String>>,
    /// Memory store.
    memory: Vec<VmMemoryEntry>,
    /// Knowledge graph relations.
    relations: Vec<VmRelation>,
    /// Rule table (from program.rules, PRE-SORTED by priority — naryad
    /// #402 step A: SHARED with the Program, read-only after load).
    rules: std::sync::Arc<Vec<CompiledRule>>,
    /// Skill index declarations (for resolve_skill_index). Naryad #402
    /// step A: SHARED with the Program, read-only after load.
    skill_indices: std::sync::Arc<Vec<CompiledSkillIndex>>,
    /// Database connection (opened from program.db_url if present).
    /// №409 (step B, candidate C2): opens LAZILY on the first db access
    /// (`ensure_db_open`) — `load_program` only records the declared URL,
    /// so requests that never touch the db never pay the in-memory sqlite
    /// open + schema DDL (the per-request class measured in the ADR-0141
    /// Addendum 4 decomposition). Pooled idle VMs rest connection-free.
    db_conn: Option<rusqlite::Connection>,
    /// №409 (C2): the declared db URL, recorded at `load_program`. The
    /// connection itself opens on demand; `None` = no db declared (the
    /// access sites produce the same legacy error as before).
    db_url: Option<String>,
    /// №409 (C2): shared schema-DDL snapshot (`Program::schema_ddl_shared`)
    /// applied once per connection open — program-immutable data, shared
    /// not copied.
    db_schema_ddl: std::sync::Arc<Vec<String>>,
    /// №409 (C2): a FAILED connection attempt is remembered for this VM
    /// generation (load → reset cycle). The eager open produced exactly one
    /// attempt + one error line per request and left every access failing
    /// with the legacy "no database connection" message; the lazy open
    /// reproduces those semantics (fail fast, no silent retry storm).
    db_open_failed: bool,
    /// Mutate log messages.
    mutate_log: Vec<String>,
    /// Audit log entries (Наряд №41 Block 2: parity with interpreter).
    audit_log: Mutex<Vec<String>>,
    /// ADR-0089: Propagated confidence from Fluid collapse through pattern calls.
    propagated_confidence: f64,
    /// Collections loaded flag (for map/filter/reduce).
    collections_loaded: bool,
    // ── Server context (per-request, set before execute_route_code) ──
    /// Parsed JSON request body (injected by server before route execution).
    server_json_body: Option<Value>,
    /// Query string parameters (injected by server before route execution).
    server_query_params: Option<std::collections::HashMap<String, String>>,
    /// Path parameters extracted from a templated route (Наряд №283).
    /// Parity with server_query_params — `server_path_param(name)` builtin.
    server_path_params: Option<std::collections::HashMap<String, String>>,
    /// User roles for RBAC (injected by server before route execution).
    server_user_roles: Vec<String>,
    /// Наряд №72: Conversations storage (ADR-0053 parity with interpreter).
    conversations: std::sync::Mutex<HashMap<String, Conversation>>,
    /// Наряд №72: Conversation configuration (ADR-0053 parity with interpreter).
    conversation_config: ConversationConfig,
    /// Наряд №72: Event stream (ADR-0052 parity with interpreter).
    event_log: std::sync::Mutex<Vec<Event>>,
    /// Наряд №72: Next event ID (ADR-0052 parity with interpreter).
    #[allow(dead_code)] // Used when VM event emission is added
    event_next_id: std::sync::atomic::AtomicU64,
    /// Наряд №72: Per-pattern runtime statistics (ADR-0051 parity with interpreter).
    pattern_stats: std::sync::Mutex<HashMap<String, PatternStats>>,
    /// Наряд №199 (ADR-0121): VM-owned ReflexRegistry — mirrors the
    /// interpreter's `reflex_registry` field. The VM owns its own registry
    /// (not a borrow) because it's a separate execution backend that may
    /// run without the interpreter ever being instantiated. No Mutex — the
    /// VM is single-threaded per request (unlike `conversations`/`event_log`
    /// which are Mutex'd because they're touched from `&self` server-context
    /// entrypoints; `call_builtin` is `&mut self`).
    reflex_registry: crate::nn::ReflexRegistry,
    /// Наряд №199: maps model name → ReflexId. Populated by `load_program`
    /// when processing `program.reflex_decls`. Used by the `LoadGlobalByName`
    /// handler to resolve bare-Ident model references like
    /// `reflex_train(TestClassifier, ...)` → `Value::Reflex(id)`.
    reflex_names: HashMap<String, crate::nn::ReflexId>,
    /// Наряд №392: the DenyEvent currently being handled (Some exactly
    /// while an on_deny body runs). deny_event()/deny_reason() read it
    /// inside `call_builtin`; outside a handler both are loud runtime
    /// errors — the event cannot be forged or stale-read.
    current_deny_event: Option<Value>,
    /// №392: deny handlers lifted from `program.deny_handlers` at
    /// load_program — consulted by the deny path regardless of WHICH
    /// program reference the executing code sees (flow steps execute
    /// pattern bodies against a synthetic empty Program, so the handler
    /// table must live on the VM like the pattern table does).
    deny_handlers: std::sync::Arc<Vec<CompiledDenyHandler>>,
    /// Наряд №240 (Vision R4.2): vision artifact registry — stores generated
    /// PNG buffers. `Value::Vision(VisionId)` indexes into this. No Mutex —
    /// same single-threaded-per-request rationale as `reflex_registry` above.
    vision_registry: crate::vision::VisionRegistry,
    /// Наряд №331 (ADR-0162): unified media store — the VM's own byte state
    /// behind `Value::Media(MediaHandle)` opaque handles. No Mutex (same
    /// single-threaded rationale); the shared dispatches in
    /// src/builtins/media.rs keep both backends identical.
    media_store: crate::media::MediaStore,
    /// Наряд №240 (Vision R4.2): maps declaration name → compiled parameters.
    /// Populated by `load_program` when processing `program.vision_decls`.
    /// Used by the `vision_generate` intercept to resolve the declaration.
    vision_decls: HashMap<String, crate::bytecode::CompiledVisionDecl>,
    /// Наряд №332 (ADR-0164): registered `origin` declarations for the
    /// media_source_capture / media_bind_origin intercepts.
    origin_decls: HashMap<String, crate::bytecode::CompiledOriginDecl>,
    /// Наряд №204 (ADR-0121 stage 2): memory persist path from
    /// `memory { persist: "path.db" }` declaration. Enables reflex_save/
    /// reflex_load on the VM (same field the interpreter has at
    /// `interpreter.memory_persist_path`).
    memory_persist_path: Option<String>,
    /// Наряд №205 (ADR-0121 stage 6): per-pattern distillation runtime state.
    /// Mirrors `Interpreter::distill_states` but without Mutex (VM is
    /// single-threaded per request — `call_llm` is `&mut self`).
    distill_states: HashMap<String, crate::interpreter::types::DistillRuntimeState>,
}

/// Collapse threshold for Fluid values (matches interpreter).
const COLLAPSE_THRESHOLD: f64 = 0.1;

/// №409 (step B, candidate C1): the process-wide builtin registry.
///
/// `Builtins` is a pure derivation of `BUILTIN_REGISTRY` (№170 SSOT) and is
/// READ-ONLY on every VM path — the only mutation affordance
/// (`override_handler`, №287) is an Interpreter-only mechanism and the
/// interpreter keeps its own owned instance. Sharing one `Arc<Builtins>`
/// across all VMs therefore preserves behavior exactly while removing the
/// per-`Vm::new()` registry rebuild (~460-entry map) — the largest single
/// per-request allocation class measured in the ADR-0141 Addendum 4
/// decomposition (~70 KB per instance).
fn shared_builtins_registry() -> std::sync::Arc<Builtins> {
    static SHARED: std::sync::OnceLock<std::sync::Arc<Builtins>> = std::sync::OnceLock::new();
    SHARED
        .get_or_init(|| std::sync::Arc::new(Builtins::new()))
        .clone()
}

/// №409 (C1): the process-wide builtin NAME table (index → name, parallel
/// to the compiler's index ordering). One allocation per process instead
/// of ~460 `String`s per `Vm::new()`.
fn shared_builtin_names() -> std::sync::Arc<Vec<String>> {
    static SHARED: std::sync::OnceLock<std::sync::Arc<Vec<String>>> = std::sync::OnceLock::new();
    SHARED
        .get_or_init(|| std::sync::Arc::new(crate::builtins::builtin_names()))
        .clone()
}

/// №409 (step B, candidate C2): open a database connection and apply the
/// schema DDL — the exact steps the eager `load_program` path performed
/// per request, extracted verbatim so the lazy open reproduces the same
/// pragmas, the same DDL error tolerance (log + continue) and the same
/// "Connected" log line.
fn open_db_connection(
    url: &str,
    schema_ddl: &[String],
) -> Result<rusqlite::Connection, rusqlite::Error> {
    let conn = if url == "sqlite::memory:" {
        rusqlite::Connection::open_in_memory()?
    } else {
        let path = url.trim_start_matches("sqlite:");
        rusqlite::Connection::open(path)?
    };
    let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
    eprintln!("[vm/db] Connected: {}", url);
    for ddl in schema_ddl {
        if let Err(e) = conn.execute_batch(ddl) {
            eprintln!("[vm/db] DDL error: {}", e);
        }
    }
    Ok(conn)
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

impl Vm {
    /// Create a new VM with empty state.
    pub fn new() -> Self {
        Vm {
            label_env: std::collections::BTreeMap::new(),
            value_registers: Vec::new(),
            globals: Vec::new(),
            global_names: std::sync::Arc::new(Vec::new()),
            patterns: std::sync::Arc::new(Vec::new()),
            learnables: Vec::new(),
            builtins: shared_builtins_registry(),
            builtin_names: shared_builtin_names(),
            memory: Vec::new(),
            relations: Vec::new(),
            rules: std::sync::Arc::new(Vec::new()),
            skill_indices: std::sync::Arc::new(Vec::new()),
            db_conn: None,
            db_url: None,
            db_schema_ddl: std::sync::Arc::new(Vec::new()),
            db_open_failed: false,
            mutate_log: Vec::new(),
            audit_log: Mutex::new(Vec::new()),
            propagated_confidence: 1.0,
            collections_loaded: false,
            server_json_body: None,
            server_query_params: None,
            server_path_params: None,
            server_user_roles: Vec::new(),
            conversations: std::sync::Mutex::new(HashMap::new()),
            conversation_config: ConversationConfig::default(),
            event_log: std::sync::Mutex::new(Vec::new()),
            event_next_id: std::sync::atomic::AtomicU64::new(0),
            pattern_stats: std::sync::Mutex::new(HashMap::new()),
            reflex_registry: crate::nn::ReflexRegistry::new(),
            reflex_names: HashMap::new(),
            current_deny_event: None,
            deny_handlers: std::sync::Arc::new(Vec::new()),
            vision_registry: crate::vision::VisionRegistry::new(),
            media_store: crate::media::MediaStore::new(),
            vision_decls: HashMap::new(),
            origin_decls: HashMap::new(),
            memory_persist_path: None,
            distill_states: HashMap::new(),
        }
    }

    /// Execute a compiled program. Returns the flow output (if any),
    /// with mutate log messages prepended if present.
    pub fn run(&mut self, program: Program) -> Result<Option<String>, String> {
        // Наряд №276: LLM traces emitted from builtins during this run carry
        // backend="vm"; the previous tag is restored on exit (thread pools
        // reuse threads — a leaked tag would lie about the next program).
        let prev_tag = crate::llm::set_llm_backend_tag("vm");
        let load_result = self.load_program(&program);
        let out = match load_result {
            Ok(()) => self.execute_main_code(&program),
            Err(e) => Err(e),
        };
        crate::llm::set_llm_backend_tag(prev_tag);
        out
    }

    /// Load program state without executing main_code.
    /// Initializes globals, patterns, learnables, rules, skill_indices,
    /// and database connection. Used by server backend to set up VM state
    /// per request without re-executing flows (Наряд №40).
    pub fn load_program(&mut self, program: &Program) -> Result<(), String> {
        // Initialize globals (the Value slots stay PER-REQUEST: globals are
        // mutable execution state — never shared).
        self.globals = vec![Value::Unit; program.globals.len()];
        // Naryad #402 (step A): the immutable collections below are SHARED
        // with the Program through lazily-built Arc snapshots
        // (Program::shared_cache) — the first load builds, every later
        // load pays one Arc increment instead of a deep clone. Read-only
        // after load on every Vm path (the run()-only RegisterPattern
        // mutation is copy-on-write); per-request server context
        // (body/query/path/roles) is injected AFTER load_program and stays
        // per-request — the isolation boundary is unchanged.
        self.global_names = program.global_names_shared();
        self.collections_loaded = program.collections_loaded;
        // №392: lift the deny handler table onto the VM (flow steps run
        // pattern bodies against a synthetic empty Program — see
        // invoke_step — so the deny path reads the VM's own table).
        self.deny_handlers = program.deny_handlers_shared();

        // Наряд №250 (ADR-0122 #208): pre-register ALL patterns declared in
        // main_code so route bodies can dispatch user calls. Root (repro:
        // naryad_161 `block3_vm_serves_imported_pattern`, verbatim
        // "status 500 != 200"): the server VM path builds a fresh Vm per
        // request and runs load_program + execute_route_code WITHOUT
        // executing main_code — but self.patterns was only ever filled by
        // RegisterPattern instructions DURING execute_main_code, so every
        // CallPattern(idx) from a route body hit "VM: pattern index N not
        // found" → HTTP 500 (the #206 debt ADR-0122 #208 "reserved").
        // Scanning main_code's RegisterPattern instructions preserves the
        // compiler's index order 1:1 (pass1 assigns idx by declaration order,
        // pass2 emits RegisterPattern in the same order), so the positional
        // CallPattern indices resolve to the right patterns. This also
        // honors the documented load_program contract ("Initializes globals,
        // patterns, learnables, rules, skill_indices") and works identically
        // for deserialized .mbc programs (their main_code carries the same
        // instructions). Idempotence across repeated load_program calls:
        // the shared snapshot REPLACES the table (assignment, not append) —
        // same effect as the old clear()+scan; the run() path re-registers
        // via the RegisterPattern handler (copy-on-write), which is
        // index-stable (replace-in-place) — no duplicates.
        self.patterns = program.pre_registered_patterns();

        // Rules arrive PRE-SORTED by priority descending (matches
        // interpreter semantics; the sort is part of the shared snapshot).
        self.rules = program.rules_sorted();
        self.skill_indices = program.skill_indices_shared();

        // Наряд №199 (ADR-0121): register reflex models from compiled
        // declarations. Mirrors the interpreter's `Declaration::Reflex(r)`
        // handling in `src/interpreter/execution.rs`. The same shared
        // `build_reflex_model` function is used (moved to
        // `src/builtins/reflex.rs`) — the neural-network logic is NOT
        // reimplemented, only the VM-side plumbing that routes to it.
        for decl in &program.reflex_decls {
            let model = crate::builtins::build_reflex_model(decl)?;
            let id = self.reflex_registry.register(model);
            self.reflex_names.insert(decl.name.clone(), id);
        }

        // Наряд №204 (ADR-0121 stages 3-4): register reflex_seq and
        // reflex_gen models. Candle-feature-gated — these require the
        // candle ML framework for autograd. The interpreter registers
        // them at runtime via `construct_reflex_seq_model` /
        // `construct_reflex_gen_model`. The VM calls the SAME shared
        // construction functions — the neural-network logic is NOT
        // reimplemented.
        #[cfg(feature = "candle")]
        {
            for decl in &program.reflex_seq_decls {
                let model = crate::builtins::build_reflex_seq_model(decl)?;
                let id = self.reflex_registry.register_seq(model);
                self.reflex_names.insert(decl.name.clone(), id);
            }
            for decl in &program.reflex_gen_decls {
                let model = crate::builtins::build_reflex_gen_model(decl)?;
                let id = self.reflex_registry.register_gen(model);
                self.reflex_names.insert(decl.name.clone(), id);
            }
        }

        // Наряд №204 (ADR-0121 stage 2): memory persist path for
        // reflex_save/reflex_load.
        self.memory_persist_path = program.memory_persist_path.clone();

        // Наряд №240 (Vision R4.2): register vision declarations
        // (name → parameters). Generation state lives in the VM's own
        // `vision_registry`; the actual inference is routed through the
        // shared dispatch functions in `src/builtins/vision.rs` (лекало
        // reflex: neural-network logic is NOT reimplemented on the VM side).
        for decl in &program.vision_decls {
            self.vision_decls.insert(decl.name.clone(), decl.clone());
        }
        // Наряд №332 (ADR-0164): register origin declarations.
        for decl in &program.origin_decls {
            self.origin_decls.insert(decl.name.clone(), decl.clone());
        }

        // ── №409 (candidate C2): the db connection is LAZY ────────
        // The eager path opened the connection (in-memory sqlite or file)
        // and ran the schema DDL here, on EVERY load_program — i.e. on
        // every serve request and every pooled checkout, even when the
        // request never touched the db. The measured cost of that class
        // is ~86 KB peak per instance (ADR-0141 Addendum 4 decomposition,
        // load_db minus load_nodb). load_program now only RECORDS the
        // declared URL and takes the SHARED schema-DDL snapshot (one Arc
        // increment — program-immutable data, previously deep-copied
        // implicitly per request); the connection itself opens on the
        // first db access (ensure_db_open) with semantics identical to
        // the eager open (same pragmas, same DDL tolerance, same log
        // lines, same legacy access error). The №381 isolation class is
        // UNCHANGED: the connection is still per-VM — never shared across
        // requests; reset_for_reuse still drops it FIRST.
        self.db_url = program.db_url.clone();
        self.db_schema_ddl = program.schema_ddl_shared();

        Ok(())
    }

    /// №409 (candidate C2): open the database connection on the FIRST db
    /// access — the lazy twin of the eager open this file used to perform
    /// inside `load_program` on every request.
    ///
    /// Semantics contract (pinned by `mod n409_tests`):
    ///   * no db declared → no-op; access sites produce the same legacy
    ///     "no database connection" error as before;
    ///   * unsupported scheme → no-op (the eager open was equally silent);
    ///   * connect failure → ONE "[vm/db] Failed to connect" line, then
    ///     `db_open_failed` makes every later access fail fast with the
    ///     same legacy message the eager path produced (no retry storm) —
    ///     the flag resets on `reset_for_reuse`, matching the per-request
    ///     retry semantics of the eager path (a fresh VM = a fresh attempt);
    ///   * success → same WAL pragma, same DDL application (log +
    ///     continue on error), same "[vm/db] Connected" line.
    fn ensure_db_open(&mut self) {
        if self.db_conn.is_some() || self.db_open_failed {
            return;
        }
        let url = match self.db_url.as_ref() {
            Some(u) => u.clone(),
            None => return,
        };
        if !(url == "sqlite::memory:" || url.starts_with("sqlite:")) {
            return;
        }
        match open_db_connection(&url, &self.db_schema_ddl) {
            Ok(c) => self.db_conn = Some(c),
            Err(e) => {
                eprintln!("[vm/db] Failed to connect to '{}': {}", url, e);
                self.db_open_failed = true;
            }
        }
    }

    /// Наряд №403: fail-closed reset between POOLED serve requests.
    ///
    /// The warm VM pool (src/vm_pool.rs) recycles `Vm` objects across
    /// requests. Reuse is ONLY sound when every piece of execution state
    /// a request could have touched is provably gone before the next
    /// request checks the VM out. This method IS that proof, enforced
    /// three ways:
    ///
    ///   1. Every mutable field NOT wholesale-reassigned by
    ///      `load_program` is explicitly reset here, one comment per
    ///      state class (the field-level contract tests in
    ///      `mod n403_reset_tests` pin the enumeration — adding a
    ///      mutable Vm field without resetting it here must fail those
    ///      tests, that is their purpose);
    ///   2. The program-scoped fields ARE wholesale-reassigned by
    ///      `load_program` (globals, patterns, rules, skill_indices,
    ///      deny_handlers, global_names, collections_loaded,
    ///      memory_persist_path, db_url, db_schema_ddl) — the reset
    ///      re-runs it, so a half-updated future `load_program` cannot
    ///      silently skip a class;
    ///   3. Anything the reset cannot guarantee is not reset but
    ///      DISCARDED by the caller: the pool never checks a VM back in
    ///      after a failed route execution, a failed reset, or a panic
    ///      (fail-closed, not best-effort reuse — the №381 shared-DB
    ///      bug is the cautionary precedent).
    ///
    /// The db connection is dropped FIRST: an open connection (its
    /// transactions, temp tables, in-memory content) must never survive
    /// into the next request even if `load_program` below fails midway.
    /// №409: with the LAZY db open the reload no longer re-opens the
    /// connection — a checked-in VM rests CONNECTION-FREE (the measured
    /// idle-VM residency drops by the live-sqlite share) and the next
    /// request's first db access re-opens it exactly like a fresh
    /// per-request VM would. `db_open_failed` resets with the same
    /// per-generation semantics (a fresh VM = a fresh attempt).
    pub fn reset_for_reuse(&mut self, program: &Program) -> Result<(), String> {
        // ── 0. the №381 class: database connection goes FIRST ──
        self.db_conn = None;
        // №409: a failed lazy open must not poison the next generation.
        self.db_open_failed = false;

        // ── 1. execution scratch ──
        // Value-expression registers (№370): block-value temporaries.
        self.value_registers = Vec::new();
        // Runtime label environment (№328/ADR-0154): a label attached to
        // a variable name in request A must not clear sink checks in
        // request B.
        self.label_env = std::collections::BTreeMap::new();
        // Mutate log: per-execution messages, never carried over.
        self.mutate_log = Vec::new();
        // Propagated confidence (ADR-0089): resets to the neutral 1.0.
        self.propagated_confidence = 1.0;

        // ── 2. in-memory stores (byte/knowledge state behind handles) ──
        // Memory store entries (`self.memory`): request A's memories must
        // be invisible to request B (the in-memory-db content class).
        self.memory = Vec::new();
        // Knowledge-graph relations: same class as memory.
        self.relations = Vec::new();
        // Media store (№331/ADR-0162): bytes behind Value::Media handles —
        // request A's bytes must not be reachable in request B.
        self.media_store = crate::media::MediaStore::new();
        // Vision artifact registry (№240 R4.2): generated PNG buffers.
        self.vision_registry = crate::vision::VisionRegistry::new();
        // Distillation runtime state (№205): per-pattern training state.
        self.distill_states = HashMap::new();

        // ── 3. security-sensitive single-value state ──
        // №392: the deny event being handled is Some exactly while an
        // on_deny body runs. A stale event surviving into request B would
        // let deny_reason()/deny_event() succeed OUTSIDE a handler —
        // a forgeable, stale security read. None is the only legal
        // resting state.
        self.current_deny_event = None;

        // ── 4. program-scoped tables that load_program INSERTS into
        //      (not wholesale) — cleared BEFORE the reload so nothing
        //      accumulates across pooled generations ──
        self.reflex_registry = crate::nn::ReflexRegistry::new();
        self.reflex_names = HashMap::new();
        self.vision_decls = HashMap::new();
        self.origin_decls = HashMap::new();
        // Learnables are populated by RegisterLearnable during main_code
        // execution (and mutated by distillation) — load_program never
        // assigns this field; on the serve path it must rest empty.
        self.learnables = Vec::new();

        // ── 5. logs/stats/event streams exposed through &self ──
        // audit_log: take_audit_log() already drained it post-execution;
        // drained again here — belt and braces (a double-reported audit
        // trail is a lying trail).
        self.audit_log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        // №72 conversation state: request A's dialogue must not continue
        // in request B.
        self.conversations
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        // №72 event stream + id counter.
        self.event_log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        self.event_next_id
            .store(0, std::sync::atomic::Ordering::SeqCst);
        // №72 per-pattern runtime statistics.
        self.pattern_stats
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();

        // ── 6. per-request server context ──
        // The isolation boundary (pinned by tests/naryad_402_step_a.rs);
        // cleared again here so a checked-in VM rests context-free even
        // before the next checkout injects fresh context.
        self.clear_server_context();

        // ── 7. program-scoped reload ──
        // Wholesale reassignment of globals/global_names/patterns/rules/
        // skill_indices/deny_handlers/collections_loaded/
        // memory_persist_path + db re-open + schema DDL + re-registration
        // of reflex/vision/origin declarations on the (now empty) tables.
        self.load_program(program)
    }

    /// Execute main_code (the top-level instruction sequence).
    /// Called by `run()` after `load_program()`.
    fn execute_main_code(&mut self, program: &Program) -> Result<Option<String>, String> {
        let mut stack: Vec<Value> = Vec::new();
        let mut call_stack: Vec<CallFrame> = Vec::new();
        let mut ip = 0;
        let mut flow_output: Option<String> = None;
        let code = &program.main_code;

        while ip < code.len() {
            let instr = &code[ip];
            match instr {
                // ── Constants & Variables ─────────────────────
                Instruction::Const(v) => {
                    stack.push((**v).clone());
                    ip += 1;
                }
                // ── Runtime labels (Наряд №328, ADR-0156) ─────
                Instruction::LabelJoin(lj) => {
                    let LabelJoinData { dst, src } = &**lj;
                    let incoming = if let Some(source) = src.strip_prefix('@') {
                        runtime_source_label(source)
                    } else {
                        self.label_env.get(src).cloned().unwrap_or_default()
                    };
                    let merged = self
                        .label_env
                        .get(dst)
                        .cloned()
                        .unwrap_or_default()
                        .join(&incoming);
                    self.label_env.insert(dst.clone(), merged);
                    ip += 1;
                }
                Instruction::SinkCheck(sc) => {
                    let SinkCheckData {
                        fn_name,
                        arg,
                        line,
                        arg_index,
                        deny,
                    } = &**sc;
                    let label = if let Some(source) = arg.strip_prefix('@') {
                        runtime_source_label(source)
                    } else {
                        self.label_env.get(arg).cloned().unwrap_or_default()
                    };
                    // Quarantine clears nothing; everything else must be
                    // public at a sink (the №325 contract, runtime twin).
                    // EXEC additionally refuses untrusted (№325/№327).
                    let exec_untrusted = fn_name == "exec" || fn_name == "exec_argv";
                    if label.conf != crate::labels::Conf::Public
                        || (exec_untrusted
                            && label.integrity == crate::labels::Integrity::Untrusted)
                    {
                        // №392: the reason class is the SAME sink_check_id
                        // the static audit uses — the event's reason and
                        // the diagnostic class agree verbatim.
                        let reason =
                            crate::audit::sink_check_id(fn_name, *arg_index as usize, &label);
                        // №385: the stamp goes through the shared `coded_error`
                        // so the try classifier (ADR-0169) reads the same
                        // marker the code constant pins — the message text
                        // after the stamp is unchanged.
                        let message = crate::interpreter::values::coded_error(
                            crate::interpreter::values::CODE_SINK_CLEARANCE_RUNTIME,
                            format!(
                                "sink clearance violated at runtime: {} argument '{}' carries label '{}' (line {}) — the static gate and the runtime agree on the verdict; deny reason class: {}",
                                fn_name,
                                arg,
                                label,
                                line,
                                reason
                            ),
                        );
                        let class = crate::audit::sink_kind(fn_name);
                        if deny.is_some() {
                            // №392: a covering on_deny handler handles the
                            // refusal — the refused call is skipped and a
                            // degraded Unit becomes its result. The verdict
                            // itself is final: the handler cannot re-allow.
                            let handled = self.vm_fire_on_deny(
                                program,
                                fn_name,
                                arg,
                                class,
                                reason,
                                &format!("{}", label),
                                *line as f64,
                                &message,
                                &mut stack,
                                &mut call_stack,
                                ip + 1,
                            )?;
                            if handled {
                                if let Some(path) = deny {
                                    ip = path.skip_to as usize;
                                    continue;
                                }
                            }
                        }
                        eprintln!("[SINK_CLEARANCE][audit-event] {}", message);
                        return Err(message);
                    }
                    ip += 1;
                }
                Instruction::LoadGlobal(slot) => {
                    let val = self.globals.get(*slot).cloned().unwrap_or(Value::Unit);
                    stack.push(val);
                    ip += 1;
                }
                Instruction::LoadGlobalByName(name) => {
                    // Search globals by name
                    let val = program
                        .globals
                        .iter()
                        .position(|n| n == name)
                        .and_then(|slot| self.globals.get(slot).cloned())
                        // Наряд №199 (ADR-0121): if not a global, check if
                        // it's a registered reflex model name. Bare-Ident
                        // references like `reflex_train(TestClassifier, ...)`
                        // compile to `LoadGlobalByName("TestClassifier")` because
                        // the compiler doesn't know about reflex models (they're
                        // registered at runtime). The VM resolves the name to
                        // `Value::Reflex(id)` here, mirroring the interpreter's
                        // `eval_expr_with_env` special-case for reflex_train/
                        // reflex_predict first-arg (execution.rs:1311-1370).
                        .or_else(|| self.reflex_names.get(name).map(|id| Value::Reflex(*id)))
                        .unwrap_or(Value::Unit);
                    stack.push(val);
                    ip += 1;
                }
                Instruction::StoreGlobal(slot) => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    if *slot < self.globals.len() {
                        self.globals[*slot] = val;
                    }
                    ip += 1;
                }
                Instruction::LoadLocal(slot) => {
                    // Local variables are stored on the stack, below the base pointer
                    let bp = call_stack.last().map(|f| f.base_bp).unwrap_or(0);
                    let idx = bp + slot;
                    let val = stack.get(idx).cloned().unwrap_or(Value::Unit);
                    stack.push(val);
                    ip += 1;
                }
                Instruction::StoreLocal(slot) => {
                    let bp = call_stack.last().map(|f| f.base_bp).unwrap_or(0);
                    let idx = bp + slot;
                    let val = stack.pop().unwrap_or(Value::Unit);
                    if idx >= stack.len() {
                        stack.resize(idx + 1, Value::Unit);
                    }
                    stack[idx] = val;
                    ip += 1;
                }
                Instruction::StoreAssignLocal(sal) => {
                    let StoreAssignLocalData {
                        slot,
                        name,
                        mutable,
                    } = &**sal;
                    // Наряд №264: VM backstop — an assignment encoded by the
                    // compiler carries its immutability fact; `mutable: false`
                    // means bytecode produced past the compile-time check.
                    // Such a store must fail LOUDLY — the same source is
                    // rejected by the TW interpreter at runtime, so the VM
                    // silently overwriting the slot (the pre-№264 behavior)
                    // broke backend parity. Plain `let` bindings keep using
                    // StoreLocal — they are definitions, not assignments.
                    if !*mutable {
                        return Err(crate::semantic::immutability_error_text(name));
                    }
                    let bp = call_stack.last().map(|f| f.base_bp).unwrap_or(0);
                    let idx = bp + slot;
                    let val = stack.pop().unwrap_or(Value::Unit);
                    if idx >= stack.len() {
                        stack.resize(idx + 1, Value::Unit);
                    }
                    stack[idx] = val;
                    ip += 1;
                }

                // ── Registration ──────────────────────────────
                Instruction::RegisterPattern(fn_def) => {
                    // Наряд №250: index-stable (re)registration. load_program
                    // pre-registers patterns (table or legacy main_code scan —
                    // №415); when execute_main_code then re-runs the SAME
                    // RegisterPattern instructions (run path — legacy .mbc
                    // only after №415), replacing the existing entry in place
                    // keeps every positional CallPattern(idx) index valid —
                    // no duplicates, and the final layout is identical to a
                    // fresh single registration (same instruction sequence
                    // over the pre-registered table; rposition makes the
                    // k-th occurrence replace the k-th slot).
                    // Naryad #402 step A: the table is a SHARED Arc snapshot —
                    // Arc::make_mut copies it once (COW) on the first
                    // run()-path mutation; the serve path (which never
                    // executes main_code) never pays the copy.
                    let patterns = std::sync::Arc::make_mut(&mut self.patterns);
                    match patterns.iter().rposition(|p| p.name == fn_def.name) {
                        Some(i) => patterns[i] = (**fn_def).clone(),
                        None => patterns.push((**fn_def).clone()),
                    }
                    ip += 1;
                }
                Instruction::RegisterPatternRef(idx) => {
                    // Naryad №415: the body already lives in the table at
                    // `idx` (the compiler fills Program::patterns 1:1 with
                    // the RegisterPatternRef emission order, and load_program
                    // installed the table). Validation-only no-op on the run
                    // path — re-executing main_code cannot drift from the
                    // table. Out-of-range = bytecode produced past the
                    // compiler check → fail LOUDLY (the №264 backstop
                    // contract), never silently skip.
                    if (*idx as usize) >= self.patterns.len() {
                        return Err(format!(
                            "VM: RegisterPatternRef index {} out of range (pattern table has {} entries) — bytecode produced past the compiler check",
                            idx,
                            self.patterns.len()
                        ));
                    }
                    ip += 1;
                }
                Instruction::RegisterLearnable(info) => {
                    self.learnables.push(((**info).clone(), Vec::new()));
                    ip += 1;
                }

                // ── Function Calls ────────────────────────────
                Instruction::CallBuiltin(idx, arity) => {
                    let name = self
                        .builtin_names
                        .get(*idx)
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string());
                    let mut args = Vec::new();
                    for _ in 0..*arity {
                        args.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }
                    // Problem B: map(list, "pattern_name") — needs pattern table access
                    if name == "map" {
                        if let Ok(result) = self.vm_map(&args, program) {
                            stack.push(result);
                            ip += 1;
                            continue;
                        }
                    }
                    // Наряд №392: a grant refusal (GRANT_*) on an
                    // irreversible action is a runtime deny event — the
                    // on_deny handler for the db class handles it (degraded
                    // Unit pushed by the helper); without a handler the
                    // loud typed error is unchanged.
                    let result = match self.call_builtin(&name, &args) {
                        Ok(r) => r,
                        Err(e) if e.starts_with("GRANT_") => {
                            let handled = self.vm_fire_on_deny(
                                program,
                                &name,
                                "sql",
                                "db",
                                "IRREVERSIBLE_NO_GRANT",
                                "bottom",
                                0.0,
                                &e,
                                &mut stack,
                                &mut call_stack,
                                ip + 1,
                            )?;
                            if handled {
                                ip += 1;
                                continue;
                            }
                            return Err(e);
                        }
                        Err(e) => return Err(e),
                    };
                    stack.push(result);
                    ip += 1;
                }
                Instruction::CallPattern(idx, arity) => {
                    let pattern = self
                        .patterns
                        .get(*idx)
                        .ok_or_else(|| format!("VM: pattern index {} not found", idx))?
                        .clone();

                    // Check arity
                    if *arity != pattern.param_count {
                        return Err(format!(
                            "VM: pattern {} expects {} args, got {}",
                            pattern.name, pattern.param_count, arity
                        ));
                    }

                    // ── VM bytecode path ────────────────────────
                    // Pop arguments (in reverse) and bind as locals
                    let mut locals = Vec::new();
                    for _ in 0..*arity {
                        locals.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }

                    // Push call frame
                    let return_ip = ip + 1;
                    let base_bp = stack.len();
                    call_stack.push(CallFrame { return_ip, base_bp });

                    // Push locals onto stack
                    for local in locals {
                        stack.push(local);
                    }

                    // Switch to pattern code
                    let result =
                        self.execute_code(&pattern.code, &mut stack, &mut call_stack, program)?;
                    // Clean up locals left on the stack by the called pattern
                    // (StoreLocal writes to base_bp+slot which may resize the Vec;
                    //  execute_code's Return only pops the return value, leaving
                    //  locals behind). Truncate back to pre-call size, then push result.
                    stack.truncate(base_bp);
                    stack.push(result);
                    // IP already advanced by 1 (return_ip)
                    ip = return_ip;
                }
                Instruction::Return => {
                    // This Return handler in execute_main_code is only reachable
                    // if the main (flow-level) code itself contains a Return
                    // instruction — not from nested pattern calls (those go
                    // through execute_code which has its own Return handler).
                    // For correctness, truncate the stack and stop execution.
                    let return_val = stack.pop().unwrap_or(Value::Unit);
                    if let Some(frame) = call_stack.pop() {
                        stack.truncate(frame.base_bp);
                    }
                    flow_output = match return_val {
                        Value::String(s) => Some(s),
                        _ => Some(return_val.to_string()),
                    };
                    break; // Exit the main execution loop
                }
                Instruction::LlmCall(idx, arity) => {
                    let mut args = Vec::new();
                    for _ in 0..*arity {
                        args.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }
                    let result = self.call_llm(*idx, &args)?;
                    stack.push(result);
                    ip += 1;
                }

                // ── Binary Operations ──────────────────────────
                Instruction::Add => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_binop(left, crate::ast::BinOp::Add, right)?);
                    ip += 1;
                }
                Instruction::Sub => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_binop(left, crate::ast::BinOp::Sub, right)?);
                    ip += 1;
                }
                Instruction::Mul => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_binop(left, crate::ast::BinOp::Mul, right)?);
                    ip += 1;
                }
                Instruction::Div => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_binop(left, crate::ast::BinOp::Div, right)?);
                    ip += 1;
                }
                Instruction::Contains => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    let result = self.eval_contains(left, right)?;
                    stack.push(result);
                    ip += 1;
                }
                Instruction::CmpGt => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_cmp(left, right, AstCompareOp::Gt));
                    ip += 1;
                }
                Instruction::CmpLt => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_cmp(left, right, AstCompareOp::Lt));
                    ip += 1;
                }
                Instruction::CmpGe => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_cmp(left, right, AstCompareOp::Ge));
                    ip += 1;
                }
                Instruction::CmpLe => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_cmp(left, right, AstCompareOp::Le));
                    ip += 1;
                }
                Instruction::CmpEq => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_cmp(left, right, AstCompareOp::Eq));
                    ip += 1;
                }
                Instruction::CmpNe => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    let eq_result = self.eval_cmp(left, right, AstCompareOp::Eq);
                    match eq_result {
                        // №372: Bool encoding (TW parity) — invert the Bool.
                        Value::Bool(b) => stack.push(Value::Bool(!b)),
                        // Legacy .mbc safety: old bytecode may still surface
                        // Float-encoded booleans through custom paths.
                        Value::Float(f) => {
                            stack.push(Value::Float(if f == 1.0 { 0.0 } else { 1.0 }))
                        }
                        _ => stack.push(eq_result),
                    }
                    ip += 1;
                }

                // ── Struct Operations ─────────────────────────
                Instruction::MakeStruct(ms) => {
                    let MakeStructData {
                        type_name,
                        field_names,
                    } = &**ms;
                    let mut fields = HashMap::new();
                    // Values are on stack in field order (first pushed = bottom)
                    // Pop in reverse to get correct order
                    let mut values = Vec::new();
                    for _ in 0..field_names.len() {
                        values.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }
                    for (name, val) in field_names.iter().zip(values.iter()) {
                        fields.insert(name.clone(), val.clone());
                    }
                    stack.push(Value::Struct {
                        type_name: type_name.clone(),
                        fields,
                    });
                    ip += 1;
                }
                Instruction::GetField(field) => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    let result = val.get_field(field).cloned().unwrap_or(Value::Unit);
                    stack.push(result);
                    ip += 1;
                }
                Instruction::IndexAccess => {
                    let idx = stack.pop().unwrap_or(Value::Unit);
                    let base = stack.pop().unwrap_or(Value::Unit);
                    let result = match (&base, &idx) {
                        (Value::List(items), Value::Float(f)) => {
                            let i = *f as isize;
                            if i < 0 {
                                let abs_i = items.len().wrapping_sub((-i) as usize);
                                items.get(abs_i).cloned().unwrap_or(Value::Unit)
                            } else {
                                items.get(i as usize).cloned().unwrap_or(Value::Unit)
                            }
                        }
                        (Value::Struct { fields, .. }, Value::String(key)) => {
                            fields.get(key).cloned().unwrap_or(Value::Unit)
                        }
                        _ => Value::Unit,
                    };
                    stack.push(result);
                    ip += 1;
                }

                // ── Fluid Types ───────────────────────────────
                Instruction::MakeFluid(count) => {
                    let mut variants: Vec<FluidValueVariant> = Vec::new();
                    // Stack has pairs: value, confidence (bottom to top)
                    // Pop count pairs
                    let mut pairs: Vec<(Value, Value)> = Vec::new();
                    for _ in 0..*count {
                        let confidence = stack.pop().unwrap_or(Value::Float(0.0));
                        let value = stack.pop().unwrap_or(Value::Unit);
                        pairs.insert(0, (value, confidence));
                    }
                    for (value, confidence) in pairs {
                        let conf_f = confidence.as_float().unwrap_or(0.0);
                        // Determine type name from value
                        let type_name = match &value {
                            Value::Float(_) => "Float",
                            Value::String(_) => "String",
                            _ => "Unit",
                        }
                        .to_string();
                        variants.push(FluidValueVariant {
                            type_name,
                            value,
                            confidence: conf_f,
                        });
                    }
                    stack.push(Value::Fluid(variants));
                    ip += 1;
                }

                // ── Control Flow ───────────────────────────────
                Instruction::Jump(target) => {
                    ip = *target;
                }
                Instruction::JumpIfNot(target) => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    if !is_truthy(&val) {
                        ip = *target;
                    } else {
                        ip += 1;
                    }
                }
                Instruction::JumpIfLow(threshold, target) => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    let below = match &val {
                        Value::Float(f) => *f < *threshold,
                        Value::Fluid(variants) => {
                            // Use the maximum confidence
                            let max_conf = variants
                                .iter()
                                .map(|v| v.confidence)
                                .fold(0.0_f64, f64::max);
                            max_conf < *threshold
                        }
                        _ => false,
                    };
                    if below {
                        ip = *target;
                    } else {
                        ip += 1;
                    }
                }

                // ── METALOGOS Memory ────────────────────────────
                Instruction::Collapse(required_type) => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    let collapsed = self.maybe_collapse(&val, required_type);
                    stack.push(collapsed);
                    ip += 1;
                }
                Instruction::Memorize(priority) => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    let value_str = match val {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    self.memory.push(VmMemoryEntry {
                        value: value_str,
                        priority: *priority,
                        timestamp: now,
                        decay_rate: 0.01,
                        mem_type: String::new(), // default: untyped
                    });
                    ip += 1;
                }
                Instruction::Recall => {
                    let query = stack.pop().unwrap_or(Value::Unit);
                    let query_str = match query {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let result = self.recall(&query_str, 0.0);
                    stack.push(Value::String(result));
                    ip += 1;
                }
                Instruction::Forget(days) => {
                    let query = stack.pop().unwrap_or(Value::Unit);
                    let query_str = match query {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    let cutoff = now - (days * 86400);
                    self.memory.retain(|entry| {
                        !(entry.value.contains(&query_str) && entry.timestamp < cutoff)
                    });
                    ip += 1;
                }

                // ── Adapt / Relate / Mutate ─────────────────────
                Instruction::Adapt(pattern_name) => {
                    let output_str = match stack.pop().unwrap_or(Value::Unit) {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let input_str = match stack.pop().unwrap_or(Value::Unit) {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    // Find the learnable by name and add the example
                    let mut found = false;
                    for (info, few_shot) in &mut self.learnables {
                        if info.name == *pattern_name {
                            few_shot.push((input_str.clone(), output_str.clone()));
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return Err(format!(
                            "VM adapt: learnable pattern '{}' not found",
                            pattern_name
                        ));
                    }
                    // Наряд №41 Block 2: audit parity with interpreter
                    self.push_audit(format!(
                        "[AUDIT] adapt {}: {} -> {}",
                        pattern_name, input_str, output_str
                    ));
                    ip += 1;
                }
                Instruction::Relate => {
                    let relation = match stack.pop().unwrap_or(Value::Unit) {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let to = match stack.pop().unwrap_or(Value::Unit) {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let from = match stack.pop().unwrap_or(Value::Unit) {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    self.relations.push(VmRelation {
                        from: from.clone(),
                        to: to.clone(),
                        relation: relation.clone(),
                    });
                    // Наряд №41 Block 2: audit parity with interpreter
                    self.push_audit(format!("[AUDIT] relate {} -[{}]-> {}", from, relation, to));
                    ip += 1;
                }
                Instruction::Mutate(md) => {
                    let MutateData {
                        pattern_name,
                        example_count,
                        rollback_threshold,
                        rollback_op,
                    } = &**md;
                    // Pop example_count pairs of (input, output)
                    let mut new_examples = Vec::new();
                    for _ in 0..*example_count {
                        let output_str = match stack.pop().unwrap_or(Value::Unit) {
                            Value::String(s) => s,
                            other => format!("{}", other),
                        };
                        let input_str = match stack.pop().unwrap_or(Value::Unit) {
                            Value::String(s) => s,
                            other => format!("{}", other),
                        };
                        new_examples.push((input_str, output_str));
                    }

                    // Find the learnable and mutate
                    let msg = self.handle_mutate(
                        pattern_name,
                        new_examples,
                        *rollback_threshold,
                        *rollback_op,
                    )?;
                    self.mutate_log.push(msg.clone());
                    // Наряд №41 Block 2: audit parity with interpreter
                    self.push_audit(format!("[AUDIT] mutate {}: {}", pattern_name, msg));
                    ip += 1;
                }

                // ── Pipeline ───────────────────────────────────
                // New: FlowPipeline — pop source from stack (compiled via compile_expr)
                Instruction::FlowPipeline(fp) => {
                    let FlowPipelineData {
                        pipeline,
                        branch_defs,
                    } = &**fp;
                    let source_val = stack.pop().unwrap_or(Value::Unit);

                    // Execute pipeline steps
                    let mut current = source_val;
                    for step_name in pipeline {
                        current = self.run_flow_step(step_name, current, branch_defs)?;
                    }

                    // The pipeline result is the flow output
                    let output_str = format!("{}", current);
                    flow_output = Some(output_str);
                    ip += 1;
                }

                // Legacy: FlowExec — load source from embedded expression
                Instruction::FlowExec(fe) => {
                    let FlowExecData {
                        source_expr,
                        pipeline,
                        branch_defs,
                    } = &**fe;
                    // Load the source value (legacy path)
                    let source_val = match source_expr {
                        FlowExpr::GlobalSlot(slot) => {
                            self.globals.get(*slot).cloned().unwrap_or(Value::Unit)
                        }
                        FlowExpr::Ident(name) => program
                            .globals
                            .iter()
                            .position(|n| n == name)
                            .and_then(|slot| self.globals.get(slot).cloned())
                            .unwrap_or(Value::Unit),
                        FlowExpr::Const(v) => v.clone(),
                    };

                    // Execute pipeline steps
                    let mut current = source_val;
                    for step_name in pipeline {
                        current = self.run_flow_step(step_name, current, branch_defs)?;
                    }

                    // The pipeline result is the flow output
                    let output_str = format!("{}", current);
                    flow_output = Some(output_str);
                    ip += 1;
                }

                // ── Error Handling ──────────────────────────────
                // Наряд №91: real `try` for the VM — catch errors locally
                Instruction::TryEval(inner_code) => {
                    // №374 (ADR-0142): structured result — the shared
                    // `try_result_struct` builds the SAME shape as the TW
                    // (error information is no longer discarded as Unit).
                    let inner: Vec<Instruction> = inner_code.clone();
                    match self.execute_code(&inner, &mut stack, &mut call_stack, program) {
                        Ok(val) => stack.push(crate::interpreter::values::try_result_struct(
                            true, val, None,
                        )),
                        Err(e) => {
                            // №385 (ADR-0169): the SAME shared classifier as the
                            // TW — one error string, one code, both backends.
                            eprintln!("[try] caught error: {}", e);
                            stack.push(crate::interpreter::values::try_result_struct(
                                false,
                                Value::Unit,
                                Some((
                                    crate::interpreter::values::stable_try_error_code(&e)
                                        .to_string(),
                                    e,
                                )),
                            ));
                        }
                    }
                    ip += 1;
                }

                // ── Rule Engine ─────────────────────────────────
                Instruction::ExecuteRules => {
                    self.execute_rules()?;
                    ip += 1;
                }

                // ── Meta ───────────────────────────────────────
                Instruction::Halt => {
                    break;
                }

                // ── Collection / List instructions (Наряд №18, №21) ──
                // VM bytecode support for these is deferred; the tree-walking
                // interpreter handles Problem A/B collection builtins natively.
                Instruction::MakeList(count) => {
                    let mut items = Vec::new();
                    for _ in 0..*count {
                        items.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }
                    stack.push(Value::List(items));
                    ip += 1;
                }
                Instruction::ListLen => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    let len = match val {
                        Value::List(items) => items.len() as f64,
                        _ => 0.0,
                    };
                    stack.push(Value::Float(len));
                    ip += 1;
                }
                Instruction::Pop => {
                    stack.pop();
                    ip += 1;
                }
                Instruction::StartsWith => {
                    let needle = stack.pop().unwrap_or(Value::Unit);
                    let haystack = stack.pop().unwrap_or(Value::Unit);
                    let result = match (&haystack, &needle) {
                        (Value::String(h), Value::String(n)) => {
                            Value::Bool(h.starts_with(n.as_str()))
                        }
                        _ => Value::Bool(false),
                    };
                    stack.push(result);
                    ip += 1;
                }
                // ── Match (№369, ADR-0141 Stage 1.1) ──────────
                Instruction::MatchTest(test) => {
                    // Compare arms pop the threshold FIRST (the compiler
                    // emits threshold evaluation right before the test),
                    // then the scrutinee. Other arms pop just the
                    // scrutinee. The predicate is the SHARED
                    // MatchTest::matches — same code TW runs.
                    let ok = match &**test {
                        MatchTest::Compare(op) => {
                            let threshold = stack.pop().unwrap_or(Value::Unit);
                            let scrutinee = stack.pop().unwrap_or(Value::Unit);
                            crate::ast::MatchArm::compare_values(&scrutinee, op, &threshold)
                        }
                        other => {
                            let scrutinee = stack.pop().unwrap_or(Value::Unit);
                            other.matches(&scrutinee, &Value::Unit)
                        }
                    };
                    stack.push(Value::Bool(ok));
                    ip += 1;
                }
                // ── Value expressions (№370, ADR-0141 Stage 1.2) ──
                Instruction::Dup => {
                    let top = stack.last().cloned().unwrap_or(Value::Unit);
                    stack.push(top);
                    ip += 1;
                }
                Instruction::BeginValueExpr => {
                    self.value_registers.push(Value::Unit);
                    ip += 1;
                }
                Instruction::KeepLastValue => {
                    // TW eval_statements_cf contract: only a NON-Unit value
                    // updates the register; a trailing Unit-valued
                    // statement does not reset it.
                    let val = stack.pop().unwrap_or(Value::Unit);
                    if !matches!(val, Value::Unit) {
                        if let Some(reg) = self.value_registers.last_mut() {
                            *reg = val;
                        }
                    }
                    ip += 1;
                }
                Instruction::EndValueExpr => {
                    let reg = self.value_registers.pop().unwrap_or(Value::Unit);
                    stack.push(reg);
                    ip += 1;
                }
            }
        }

        // Build final output with mutate log
        let mutate_log = std::mem::take(&mut self.mutate_log);
        if mutate_log.is_empty() {
            Ok(flow_output)
        } else {
            match flow_output {
                Some(flow) => {
                    let mut result = mutate_log.join("\n");
                    result.push('\n');
                    result.push_str(&flow);
                    Ok(Some(result))
                }
                None => Ok(Some(mutate_log.join("\n"))),
            }
        }
    }

    /// Execute a block of code (e.g., pattern body) and return the result.
    /// This handles the call stack and Return instructions internally.
    pub fn execute_code(
        &mut self,
        code: &[Instruction],
        stack: &mut Vec<Value>,
        call_stack: &mut Vec<CallFrame>,
        program: &Program,
    ) -> Result<Value, String> {
        // №370: register-stack isolation — a pattern/route executed via a
        // CallPattern from inside another function's value expression must
        // not see (or leak through an early Return into) the caller's open
        // registers. Save/restore around the inner loop.
        let saved_registers = std::mem::take(&mut self.value_registers);
        let out = self.execute_code_inner(code, stack, call_stack, program);
        self.value_registers = saved_registers;
        out
    }

    fn execute_code_inner(
        &mut self,
        code: &[Instruction],
        stack: &mut Vec<Value>,
        call_stack: &mut Vec<CallFrame>,
        program: &Program,
    ) -> Result<Value, String> {
        let mut ip = 0;
        // Safety: prevent infinite loops (max iterations per 100 instructions)
        let max_iterations = code.len().max(1) * 100_000;
        let mut iterations = 0;
        while ip < code.len() {
            iterations += 1;
            if iterations > max_iterations {
                return Err(format!(
                    "VM execute_code: possible infinite loop ({} iterations, {} instructions)",
                    iterations,
                    code.len()
                ));
            }
            let instr = &code[ip];
            match instr {
                Instruction::LoadLocal(slot) => {
                    let bp = call_stack.last().map(|f| f.base_bp).unwrap_or(0);
                    let idx = bp + slot;
                    let val = stack.get(idx).cloned().unwrap_or(Value::Unit);
                    stack.push(val);
                    ip += 1;
                }
                Instruction::StoreLocal(slot) => {
                    let bp = call_stack.last().map(|f| f.base_bp).unwrap_or(0);
                    let idx = bp + slot;
                    let val = stack.pop().unwrap_or(Value::Unit);
                    // Ensure stack has space at idx
                    if idx >= stack.len() {
                        stack.resize(idx + 1, Value::Unit);
                    }
                    stack[idx] = val;
                    ip += 1;
                }
                Instruction::StoreAssignLocal(sal) => {
                    let StoreAssignLocalData {
                        slot,
                        name,
                        mutable,
                    } = &**sal;
                    // Наряд №264: VM backstop (mirrors execute_main_code) —
                    // `mutable: false` on the wire must fail loudly with the
                    // TW-parity text, never silently overwrite the slot.
                    if !*mutable {
                        return Err(crate::semantic::immutability_error_text(name));
                    }
                    let bp = call_stack.last().map(|f| f.base_bp).unwrap_or(0);
                    let idx = bp + slot;
                    let val = stack.pop().unwrap_or(Value::Unit);
                    // Ensure stack has space at idx
                    if idx >= stack.len() {
                        stack.resize(idx + 1, Value::Unit);
                    }
                    stack[idx] = val;
                    ip += 1;
                }
                Instruction::Const(v) => {
                    stack.push((**v).clone());
                    ip += 1;
                }
                Instruction::LoadGlobal(slot) => {
                    let val = self.globals.get(*slot).cloned().unwrap_or(Value::Unit);
                    stack.push(val);
                    ip += 1;
                }
                Instruction::CallBuiltin(idx, arity) => {
                    let name = self
                        .builtin_names
                        .get(*idx)
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string());
                    let mut args = Vec::new();
                    for _ in 0..*arity {
                        args.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }
                    // Problem B: map(list, "pattern_name") — needs pattern table access
                    if name == "map" {
                        if let Ok(result) = self.vm_map(&args, program) {
                            stack.push(result);
                            ip += 1;
                            continue;
                        }
                    }
                    // Наряд №392: grant refusal → on_deny (db class),
                    // same contract as the main-code dispatch site.
                    let result = match self.call_builtin(&name, &args) {
                        Ok(r) => r,
                        Err(e) if e.starts_with("GRANT_") => {
                            let handled = self.vm_fire_on_deny(
                                program,
                                &name,
                                "sql",
                                "db",
                                "IRREVERSIBLE_NO_GRANT",
                                "bottom",
                                0.0,
                                &e,
                                stack,
                                call_stack,
                                ip + 1,
                            )?;
                            if handled {
                                ip += 1;
                                continue;
                            }
                            return Err(e);
                        }
                        Err(e) => return Err(e),
                    };
                    stack.push(result);
                    ip += 1;
                }
                Instruction::CallPattern(pidx, arity) => {
                    let pattern = self
                        .patterns
                        .get(*pidx)
                        .ok_or_else(|| format!("VM: pattern index {} not found", pidx))?
                        .clone();
                    if *arity != pattern.param_count {
                        return Err(format!(
                            "VM: pattern {} expects {} args, got {}",
                            pattern.name, pattern.param_count, arity
                        ));
                    }
                    let mut locals = Vec::new();
                    for _ in 0..*arity {
                        locals.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }
                    let base_bp = stack.len();
                    call_stack.push(CallFrame {
                        return_ip: ip + 1,
                        base_bp,
                    });
                    for local in locals {
                        stack.push(local);
                    }
                    let result = self.execute_code(&pattern.code, stack, call_stack, program)?;
                    // Clean up: the called pattern's locals (written via
                    // StoreLocal to base_bp+slot) remain on the stack after
                    // execute_code's Return pops only the return value.
                    // Truncate back to pre-call boundary before pushing result.
                    stack.truncate(base_bp);
                    call_stack.pop();
                    stack.push(result);
                    ip += 1;
                }
                Instruction::Add => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_binop(left, crate::ast::BinOp::Add, right)?);
                    ip += 1;
                }
                Instruction::Sub => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_binop(left, crate::ast::BinOp::Sub, right)?);
                    ip += 1;
                }
                Instruction::Mul => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_binop(left, crate::ast::BinOp::Mul, right)?);
                    ip += 1;
                }
                Instruction::Div => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_binop(left, crate::ast::BinOp::Div, right)?);
                    ip += 1;
                }
                Instruction::Return => {
                    return Ok(stack.pop().unwrap_or(Value::Unit));
                }
                // Phase 5.1: control flow in pattern bodies
                Instruction::Jump(target) => {
                    ip = *target;
                }
                Instruction::JumpIfNot(target) => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    if !is_truthy(&val) {
                        ip = *target;
                    } else {
                        ip += 1;
                    }
                }
                // Phase 5.1: comparison operators
                Instruction::CmpGt => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_cmp(left, right, AstCompareOp::Gt));
                    ip += 1;
                }
                Instruction::CmpLt => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_cmp(left, right, AstCompareOp::Lt));
                    ip += 1;
                }
                Instruction::CmpGe => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_cmp(left, right, AstCompareOp::Ge));
                    ip += 1;
                }
                Instruction::CmpLe => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_cmp(left, right, AstCompareOp::Le));
                    ip += 1;
                }
                Instruction::CmpEq => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    stack.push(self.eval_cmp(left, right, AstCompareOp::Eq));
                    ip += 1;
                }
                Instruction::CmpNe => {
                    let right = stack.pop().unwrap_or(Value::Unit);
                    let left = stack.pop().unwrap_or(Value::Unit);
                    let eq_result = self.eval_cmp(left, right, AstCompareOp::Eq);
                    match eq_result {
                        // №372: Bool encoding (TW parity) — invert the Bool.
                        Value::Bool(b) => stack.push(Value::Bool(!b)),
                        // Legacy .mbc safety: old bytecode may still surface
                        // Float-encoded booleans through custom paths.
                        Value::Float(f) => {
                            stack.push(Value::Float(if f == 1.0 { 0.0 } else { 1.0 }))
                        }
                        _ => stack.push(eq_result),
                    }
                    ip += 1;
                }
                Instruction::GetField(field) => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    let result = val.get_field(field).cloned().unwrap_or(Value::Unit);
                    stack.push(result);
                    ip += 1;
                }
                Instruction::LoadGlobalByName(name) => {
                    let val = program
                        .globals
                        .iter()
                        .position(|n| n == name)
                        .and_then(|slot| self.globals.get(slot).cloned())
                        // Наряд №199: same reflex_names resolution as
                        // execute_main_code's LoadGlobalByName handler.
                        .or_else(|| self.reflex_names.get(name).map(|id| Value::Reflex(*id)))
                        .unwrap_or(Value::Unit);
                    stack.push(val);
                    ip += 1;
                }
                // Problem B: IndexAccess was missing from execute_code.
                Instruction::IndexAccess => {
                    let idx = stack.pop().unwrap_or(Value::Unit);
                    let base = stack.pop().unwrap_or(Value::Unit);
                    let result = match (&base, &idx) {
                        (Value::List(items), Value::Float(f)) => {
                            let i = *f as isize;
                            if i < 0 {
                                let abs_i = items.len().wrapping_sub((-i) as usize);
                                items.get(abs_i).cloned().unwrap_or(Value::Unit)
                            } else {
                                items.get(i as usize).cloned().unwrap_or(Value::Unit)
                            }
                        }
                        (Value::Struct { fields, .. }, Value::String(key)) => {
                            fields.get(key).cloned().unwrap_or(Value::Unit)
                        }
                        _ => Value::Unit,
                    };
                    stack.push(result);
                    ip += 1;
                }
                Instruction::Collapse(required_type) => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    let collapsed = self.maybe_collapse(&val, required_type);
                    stack.push(collapsed);
                    ip += 1;
                }
                Instruction::LlmCall(idx, arity) => {
                    let mut args = Vec::new();
                    for _ in 0..*arity {
                        args.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }
                    let result = self.call_llm(*idx, &args)?;
                    stack.push(result);
                    ip += 1;
                }
                Instruction::Recall => {
                    let query = stack.pop().unwrap_or(Value::Unit);
                    let query_str = match query {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let result = self.recall(&query_str, 0.0);
                    stack.push(Value::String(result));
                    ip += 1;
                }
                // ── Наряд №266: memory ops in pattern/route bodies ──────────
                // The statement form compiles to the SAME opcodes the top-level
                // declarations use (Memorize/Forget/Relate), but pattern bodies
                // execute through execute_code — which previously SILENTLY
                // SKIPPED these opcodes (`_ => ip += 1`), the exact class of
                // dishonest silence this наряд excludes. Handlers mirror
                // execute_main_code verbatim (same stores, same audit parity).
                Instruction::Memorize(priority) => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    let value_str = match val {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    self.memory.push(VmMemoryEntry {
                        value: value_str,
                        priority: *priority,
                        timestamp: now,
                        decay_rate: 0.01,
                        mem_type: String::new(), // default: untyped
                    });
                    ip += 1;
                }
                Instruction::Forget(days) => {
                    let query = stack.pop().unwrap_or(Value::Unit);
                    let query_str = match query {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    let cutoff = now - (days * 86400);
                    self.memory.retain(|entry| {
                        !(entry.value.contains(&query_str) && entry.timestamp < cutoff)
                    });
                    ip += 1;
                }
                Instruction::Relate => {
                    let relation = match stack.pop().unwrap_or(Value::Unit) {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let to = match stack.pop().unwrap_or(Value::Unit) {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    let from = match stack.pop().unwrap_or(Value::Unit) {
                        Value::String(s) => s,
                        other => format!("{}", other),
                    };
                    self.relations.push(VmRelation {
                        from: from.clone(),
                        to: to.clone(),
                        relation: relation.clone(),
                    });
                    // Наряд №41 Block 2: audit parity with interpreter
                    self.push_audit(format!("[AUDIT] relate {} -[{}]-> {}", from, relation, to));
                    ip += 1;
                }
                // ── Collection / List instructions (Наряд №34) ──
                Instruction::MakeList(count) => {
                    let mut items = Vec::new();
                    for _ in 0..*count {
                        items.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }
                    stack.push(Value::List(items));
                    ip += 1;
                }
                Instruction::ListLen => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    let len = match val {
                        Value::List(items) => items.len() as f64,
                        _ => 0.0,
                    };
                    stack.push(Value::Float(len));
                    ip += 1;
                }
                Instruction::Pop => {
                    stack.pop();
                    ip += 1;
                }
                Instruction::StartsWith => {
                    let needle = stack.pop().unwrap_or(Value::Unit);
                    let haystack = stack.pop().unwrap_or(Value::Unit);
                    let result = match (&haystack, &needle) {
                        (Value::String(h), Value::String(n)) => {
                            Value::Bool(h.starts_with(n.as_str()))
                        }
                        _ => Value::Bool(false),
                    };
                    stack.push(result);
                    ip += 1;
                }
                // ── Match (№369, ADR-0141 Stage 1.1) ──────────
                // Same shared dispatch as execute_main_code — pattern
                // bodies and route handlers run through execute_code,
                // so match inside pattern/route bodies lands HERE.
                Instruction::MatchTest(test) => {
                    let ok = match &**test {
                        MatchTest::Compare(op) => {
                            let threshold = stack.pop().unwrap_or(Value::Unit);
                            let scrutinee = stack.pop().unwrap_or(Value::Unit);
                            crate::ast::MatchArm::compare_values(&scrutinee, op, &threshold)
                        }
                        other => {
                            let scrutinee = stack.pop().unwrap_or(Value::Unit);
                            other.matches(&scrutinee, &Value::Unit)
                        }
                    };
                    stack.push(Value::Bool(ok));
                    ip += 1;
                }
                // ── Value expressions (№370) — see execute_main_code ──
                Instruction::Dup => {
                    let top = stack.last().cloned().unwrap_or(Value::Unit);
                    stack.push(top);
                    ip += 1;
                }
                Instruction::BeginValueExpr => {
                    self.value_registers.push(Value::Unit);
                    ip += 1;
                }
                Instruction::KeepLastValue => {
                    let val = stack.pop().unwrap_or(Value::Unit);
                    if !matches!(val, Value::Unit) {
                        if let Some(reg) = self.value_registers.last_mut() {
                            *reg = val;
                        }
                    }
                    ip += 1;
                }
                Instruction::EndValueExpr => {
                    let reg = self.value_registers.pop().unwrap_or(Value::Unit);
                    stack.push(reg);
                    ip += 1;
                }
                Instruction::MakeStruct(ms) => {
                    let MakeStructData {
                        type_name,
                        field_names,
                    } = &**ms;
                    let mut fields = HashMap::new();
                    let mut values = Vec::new();
                    for _ in 0..field_names.len() {
                        values.insert(0, stack.pop().unwrap_or(Value::Unit));
                    }
                    for (name, val) in field_names.iter().zip(values.iter()) {
                        fields.insert(name.clone(), val.clone());
                    }
                    stack.push(Value::Struct {
                        type_name: type_name.clone(),
                        fields,
                    });
                    ip += 1;
                }
                Instruction::Contains => {
                    let needle = stack.pop().unwrap_or(Value::Unit);
                    let haystack = stack.pop().unwrap_or(Value::Unit);
                    let result = match (&haystack, &needle) {
                        (Value::String(h), Value::String(n)) => Value::Bool(h.contains(n.as_str())),
                        (Value::List(items), _) => Value::Bool(
                            items
                                .iter()
                                .any(|v| format!("{}", v) == format!("{}", needle)),
                        ),
                        _ => Value::Bool(false),
                    };
                    stack.push(result);
                    ip += 1;
                }
                // ── Error Handling ──────────────────────────────
                // Наряд №91: real `try` for the VM — catch errors locally
                Instruction::TryEval(inner_code) => {
                    // №374 (ADR-0142): structured result — the shared
                    // `try_result_struct` builds the SAME shape as the TW
                    // (error information is no longer discarded as Unit).
                    let inner: Vec<Instruction> = inner_code.clone();
                    match self.execute_code(&inner, stack, call_stack, program) {
                        Ok(val) => stack.push(crate::interpreter::values::try_result_struct(
                            true, val, None,
                        )),
                        Err(e) => {
                            // №385 (ADR-0169): the SAME shared classifier as the
                            // TW — one error string, one code, both backends.
                            eprintln!("[try] caught error: {}", e);
                            stack.push(crate::interpreter::values::try_result_struct(
                                false,
                                Value::Unit,
                                Some((
                                    crate::interpreter::values::stable_try_error_code(&e)
                                        .to_string(),
                                    e,
                                )),
                            ));
                        }
                    }
                    ip += 1;
                }
                // For any unhandled instruction, skip
                _ => {
                    ip += 1;
                }
            }
        }
        Ok(stack.pop().unwrap_or(Value::Unit))
    }

    /// Call a built-in function by name.
    /// Problem B: map(list, "pattern_name") — applies a compiled pattern to each list element.
    /// Needed because map requires pattern table access (not available to regular builtins).
    fn vm_map(&mut self, args: &[Value], program: &Program) -> Result<Value, String> {
        if !self.collections_loaded {
            return Err("map() requires 'import std/collections'".to_string());
        }
        let list = match args.first() {
            Some(Value::List(items)) => items.clone(),
            _ => return Err("map() expects first argument to be a List".to_string()),
        };
        let pattern_name = match args.get(1) {
            Some(Value::String(s)) => s.clone(),
            _ => {
                return Err(
                    "map() expects second argument to be a pattern name (String)".to_string(),
                )
            }
        };
        let pattern = self
            .patterns
            .iter()
            .find(|p| p.name == pattern_name)
            .ok_or_else(|| format!("map(): pattern '{}' not found", pattern_name))?
            .clone();
        if pattern.param_count != 1 {
            return Err(format!(
                "map(): pattern '{}' must accept exactly 1 argument, got {}",
                pattern_name, pattern.param_count
            ));
        }
        let mut results = Vec::new();
        for item in &list {
            let mut item_stack: Vec<Value> = vec![item.clone()];
            let mut item_cs: Vec<CallFrame> = vec![CallFrame {
                return_ip: 0,
                base_bp: 0,
            }];
            let result =
                self.execute_code(&pattern.code, &mut item_stack, &mut item_cs, program)?;
            results.push(result);
        }
        Ok(Value::List(results))
    }

    /// Наряд №392: fire the on_deny handler for a refused action (VM side).
    ///
    /// Selection: exact sink-class match wins over `*` (crate::deny is the
    /// shared selector). No covering handler → `Ok(false)` and the caller
    /// keeps the loud default error. While the handler runs,
    /// `current_deny_event` holds the typed event. The handler's return
    /// value is discarded; on success the degraded `Unit` is pushed as the
    /// refused call's result and the caller continues. The verdict is
    /// final — the handler can only handle a refusal, never re-allow it.
    #[allow(clippy::too_many_arguments)]
    fn vm_fire_on_deny(
        &mut self,
        program: &Program,
        sink: &str,
        argument: &str,
        class: &str,
        reason: &str,
        label: &str,
        line: f64,
        human: &str,
        stack: &mut Vec<Value>,
        call_stack: &mut Vec<CallFrame>,
        return_ip: usize,
    ) -> Result<bool, String> {
        // ── Naryad #393 (ADR-0167 §3.4): the deny HAPPENED regardless of
        // whether a handler covers it — the ledger record is written
        // BEFORE handler selection, as a side effect of the refusal path
        // itself (runtime-twin parity with the TW hook in
        // src/interpreter/hooks.rs). Best-effort: loud stderr on failure,
        // outcome unchanged.
        crate::ledger::record(
            &format!("deny.{}", reason),
            "runtime",
            class,
            &format!("{}|{}|{}|{}|{}", sink, argument, label, line, human),
        );
        let (handler_class, code) = {
            let classes: Vec<(String, ())> = self
                .deny_handlers
                .iter()
                .map(|h| (h.class.clone(), ()))
                .collect();
            let Some(idx) = crate::deny::select_handler(&classes, class) else {
                return Ok(false);
            };
            (
                self.deny_handlers[idx].class.clone(),
                self.deny_handlers[idx].code.clone(),
            )
        };
        eprintln!(
            "[DENY_EVENT][audit-event] {} refused {} (class {}, reason {}, line {}) — handled by on_deny({})",
            sink, argument, class, reason, line, handler_class
        );
        let event = crate::deny::make_event(reason, sink, class, argument, label, line, human);
        self.current_deny_event = Some(event);
        let result = self.run_deny_handler(&code, stack, call_stack, program, return_ip);
        self.current_deny_event = None;
        // A failing handler is loud — a broken degradation path must not
        // masquerade as a handled refusal.
        result?;
        stack.push(Value::Unit);
        Ok(true)
    }

    /// Наряд №392: execute an on_deny handler body (zero-arg code) with
    /// the CallPattern frame discipline; the handler's value is discarded.
    fn run_deny_handler(
        &mut self,
        handler_code: &[Instruction],
        stack: &mut Vec<Value>,
        call_stack: &mut Vec<CallFrame>,
        program: &Program,
        return_ip: usize,
    ) -> Result<(), String> {
        let base_bp = stack.len();
        call_stack.push(CallFrame { return_ip, base_bp });
        let result = self.execute_code(handler_code, stack, call_stack, program);
        // Pop the handler's frame AND truncate its locals — the same
        // cleanup the CallPattern arm performs (a leftover frame would
        // shadow the caller's base_bp and corrupt every later StoreLocal).
        stack.truncate(base_bp);
        call_stack.pop();
        result.map(|_| ())
    }

    fn call_builtin(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
        // ── Наряд №392: the DenyEvent surface ──────────────────────
        // Handler-scoped, runtime-constructed. The analyzer blocks usage
        // outside an on_deny handler at compile time; this runtime gate
        // (event live exactly while the handler body runs) is the second
        // half of the double protection.
        if name == "deny_event" || name == "deny_reason" {
            let event = self.current_deny_event.clone().ok_or_else(|| {
                "deny_event() is only available inside an on_deny handler".to_string()
            })?;
            let reason = match &event {
                Value::Struct { fields, .. } => {
                    fields.get("reason").cloned().unwrap_or(Value::Unit)
                }
                other => other.clone(),
            };
            return Ok(if name == "deny_event" { event } else { reason });
        }

        // find(entity_type, field, op, threshold) — entity store query
        // Searches globals for structs matching the type and field condition.
        // №466: the memory group dispatches through the shared live
        // module (src/memory_ops.rs) — the simple-memory twin engines
        // moved there; the VM keeps only argument marshaling through the
        // VmMemoryAccess contract. The 3..4-argument forget falls
        // through to the §10.3 registry front door (parity with the TW).
        if crate::memory_ops::handles(name) {
            if let Some(result) = crate::memory_ops::dispatch_vm(name, self, args) {
                return result;
            }
        }
        if name == "find" {
            let type_name = match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => return Err("find() requires type name as first argument (String)".to_string()),
            };
            let field_name = match args.get(1) {
                Some(Value::String(s)) => s.clone(),
                _ => {
                    return Err("find() requires field name as second argument (String)".to_string())
                }
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
            for val in &self.globals {
                if let Value::Struct {
                    type_name: tn,
                    fields,
                } = val
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
                                    _ => {
                                        return Err(format!(
                                            "find(): unknown operator '{}'",
                                            op_str
                                        ))
                                    }
                                };
                                if matches {
                                    return Ok(val.clone());
                                }
                            }
                        }
                    }
                }
            }
            return Ok(Value::Unit);
        }

        // db_insert(table, struct) — insert a struct into a database table
        // №466: the body lives in the shared live module (src/db_ops.rs);
        // the VM keeps only the marshaling hook through the VmDbAccess
        // contract (the lazy open fires inside, exactly as before).
        if name == crate::db_ops::NAME_DB_INSERT {
            return crate::db_ops::db_insert_vm(self, args);
        }

        // query_scalar(sql, params) — execute SELECT returning one scalar value
        // №466: the body lives in the shared live module (src/db_ops.rs).
        if name == crate::db_ops::NAME_QUERY_SCALAR {
            return crate::db_ops::query_scalar_vm(self, args);
        }

        // query(sql) / query(sql, params) — execute SELECT returning list of structs
        // №466: the body lives in the shared live module (src/db_ops.rs).
        if name == crate::db_ops::NAME_QUERY {
            return crate::db_ops::query_vm(self, args);
        }

        // db_execute(sql, params?) — execute SQL (INSERT/UPDATE/DELETE/DDL)
        // №466: the body lives in the shared live module (src/db_ops.rs).
        if name == crate::db_ops::NAME_DB_EXECUTE {
            return crate::db_ops::db_execute_vm(self, args);
        }

        // db_execute_with_grant(g, sql, params?) — Naryad #390 (ADR-0155):
        // the granted destructive-SQL action. №466: the body lives in the
        // shared live module (src/db_ops.rs) — the gates (ledger state/
        // TTL/scope via src/grants.rs) and the post-success consumption
        // stay byte-for-byte the contract they were.
        if name == crate::db_ops::NAME_DB_EXECUTE_WITH_GRANT {
            return crate::db_ops::db_execute_with_grant_vm(self, args);
        }

        // resolve_skill_index(dept) — returns compiled skill index as Value::Struct
        if name == "resolve_skill_index" {
            let dept = match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => {
                    return Err(
                        "resolve_skill_index() expects a department name (String)".to_string()
                    )
                }
            };
            let idx = self
                .skill_indices
                .iter()
                .find(|si| si.name == dept)
                .ok_or_else(|| {
                    format!(
                        "resolve_skill_index(): no skill_index declared for '{}'",
                        dept
                    )
                })?;
            let mut fields = std::collections::HashMap::new();
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
                            let mut f = std::collections::HashMap::new();
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
            return Ok(Value::Struct {
                type_name: "SkillIndex".to_string(),
                fields,
            });
        }

        // ── Server-context builtins (Наряд №40: VM server backend) ──
        // These must be intercepted here because they need access to
        // per-request server context (query_params, json_body, user_roles)
        // that the generic builtin registry does not have.

        // №466: the shared parse/return shape lives in src/db_ops.rs; the
        // VM map accessor is injected.
        if name == crate::db_ops::NAME_QUERY_PARAM {
            return crate::db_ops::query_param_vm(self, args);
        }

        // Наряд №283: server_path_param(name) — path parameter from a
        // templated route (`/demo/{name}` matched against `/demo/test`).
        // Parity with query_param: empty string when no match / no context.
        if name == "server_path_param" {
            let param_name = args
                .first()
                .and_then(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            if let Some(ref params) = self.server_path_params {
                if let Some(val) = params.get(&param_name) {
                    return Ok(Value::String(val.clone()));
                }
            }
            return Ok(Value::String(String::new()));
        }

        if name == "json_body" {
            if let Some(ref body) = self.server_json_body {
                return Ok(body.clone());
            }
            return Ok(Value::Struct {
                type_name: "JsonBody".to_string(),
                fields: std::collections::HashMap::new(),
            });
        }

        if name == "form_data" {
            return Ok(Value::Struct {
                type_name: "FormData".to_string(),
                fields: std::collections::HashMap::new(),
            });
        }

        if name == "require" {
            let role = args
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

        // ── Наряд №67: recipe_save — intercept to also memorize for recipe_search ──
        // recipe_save(name, description, skills, plan) builds a struct via the pure
        // builtin AND memorizes the description with type "recipe" so that
        // recipe_search can find it later.
        if name == "recipe_save" {
            let result = crate::builtins::office::recipes::builtin_recipe_save(args)?;
            if let Value::Struct { ref fields, .. } = result {
                let key = fields
                    .get("key")
                    .and_then(|v| match v {
                        Value::String(s) => Some(s.as_str()),
                        _ => None,
                    })
                    .unwrap_or("");
                let desc = args
                    .get(1)
                    .and_then(|v| match v {
                        Value::String(s) => Some(s.as_str()),
                        _ => None,
                    })
                    .unwrap_or("");
                if !desc.is_empty() && !key.is_empty() {
                    let mem_value = format!("__KVKEY:{}\n{}", key, desc);
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    self.memory.push(VmMemoryEntry {
                        value: mem_value,
                        priority: 0.8,
                        timestamp: now,
                        decay_rate: 0.01,
                        mem_type: "recipe".to_string(),
                    });
                }
            }
            return Ok(result);
        }

        // ── Наряд №67: recipe_search — semantic search via memory + kv_get ──
        // Searches VM memory for entries of type "recipe" whose value contains
        // all query words (token-level AND match, case-insensitive), then
        // fetches full recipe data from the shared KV store.
        if name == "recipe_search" {
            if args.is_empty() {
                return Err("recipe_search() requires at least 1 argument (query)".to_string());
            }
            let query = match args.first() {
                Some(Value::String(s)) => s.clone(),
                other => {
                    return Err(format!(
                        "recipe_search() expected String as first arg, got {:?}",
                        other
                    ))
                }
            };
            let k = if args.len() > 1 {
                args[1].as_float().unwrap_or(5.0) as usize
            } else {
                5
            };

            // Token-level AND matching: all query words must appear (case-insensitive)
            let query_lower = query.to_lowercase();
            let query_words: Vec<&str> = query_lower.split_whitespace().collect();

            let mut matches: Vec<String> = Vec::new();
            let mut seen_keys = std::collections::HashSet::new();
            for entry in &self.memory {
                if !entry.mem_type.is_empty() && entry.mem_type != "recipe" {
                    continue;
                }
                let val_lower = entry.value.to_lowercase();
                if query_words.iter().all(|w| val_lower.contains(w)) {
                    // Extract KV key from value format: "__KVKEY:<key>\n<description>"
                    let kv_key = entry
                        .value
                        .strip_prefix("__KVKEY:")
                        .and_then(|rest| rest.lines().next())
                        .unwrap_or("");
                    if kv_key.is_empty() || seen_keys.contains(kv_key) {
                        continue;
                    }
                    seen_keys.insert(kv_key.to_string());
                    matches.push(entry.value.clone());
                    if matches.len() >= k {
                        break;
                    }
                }
            }

            // Fetch full recipes from KV store
            let mut recipes: Vec<Value> = Vec::new();
            for mem_val in &matches {
                let kv_key = mem_val
                    .strip_prefix("__KVKEY:")
                    .and_then(|rest| rest.lines().next())
                    .unwrap_or("");
                if kv_key.is_empty() {
                    continue;
                }
                if let Some(recipe_json) = crate::builtins::memory::kv_get_raw(kv_key) {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&recipe_json) {
                        let name = parsed["name"].as_str().unwrap_or("").to_string();
                        let desc = parsed["description"].as_str().unwrap_or("").to_string();
                        recipes.push(crate::builtins::core::make_struct(
                            "RecipeResult",
                            vec![
                                ("name", Value::String(name)),
                                ("description", Value::String(desc)),
                                ("recipe_json", Value::String(recipe_json)),
                            ],
                        ));
                    }
                }
            }

            return Ok(Value::List(recipes));
        }

        // ── Наряд №72: memorize — parity with interpreter::invoke_memorize_fn ──

        // ── Наряд №72: forget — parity with interpreter::invoke_forget_fn ──
        // №445: the legacy 1..2-argument surface only; 3..4 arguments
        // are the canon §10.3 typed front door and fall through to the
        // registry handler (parity by construction — the shared engine
        // lives in src/memory_typed.rs).

        // ── Bug #530 (FO-050 / office #182): recall_top_k — parity with the
        // interpreter's invoke_recall_top_k_fn ──
        // The TW resolves the name through its interception table; the VM
        // compiler consults the builtin registry — with neither a registry
        // entry nor a VM dispatch, every program calling recall_top_k failed
        // to COMPILE on the VM while running on the TW. The VM's memory is
        // the simple in-process Vec (memorize/forget parity above): the
        // search is token-level AND over the value with a matched-words
        // score weighted by priority — the honest simple-memory twin of the
        // TW's FTS5+cosine hybrid (each backend reads its own store, the
        // same posture as memorize/forget). Returns the same JSON shape:
        // [{value, score, type, priority}] as a String.

        // ── Наряд №72: query_row — parity with the TW lane ──
        // №466: the body lives in the shared live module (src/db_ops.rs);
        // the stringify-bind lane stays the VM's own (the №465 pin).
        if name == crate::db_ops::NAME_QUERY_ROW {
            return crate::db_ops::query_row_vm(self, args);
        }

        // ── Наряд №72: inspect — parity with interpreter::invoke_inspect ──
        if name == "inspect" {
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

            // Check if pattern exists in either learnables or patterns
            let is_learnable = self
                .learnables
                .iter()
                .any(|(info, _)| info.name == pattern_name);
            let is_regular = self.patterns.iter().any(|p| p.name == pattern_name);
            if !is_learnable && !is_regular {
                return Ok(Value::Unit);
            }

            let stats = self
                .pattern_stats
                .lock()
                .map(|s| s.get(&pattern_name).cloned().unwrap_or_default())
                .unwrap_or_default();

            let actual_examples = self
                .learnables
                .iter()
                .find(|(info, _)| info.name == pattern_name)
                .map(|(_, few_shot)| few_shot.len() as u64)
                .unwrap_or(stats.examples_count);

            let cache_misses = stats.calls.saturating_sub(stats.cache_hits);

            let mut fields = HashMap::new();
            fields.insert("calls".to_string(), Value::Float(stats.calls as f64));
            fields.insert(
                "avg_confidence".to_string(),
                Value::Float(if stats.calls > 0 {
                    stats.confidence_sum / stats.calls as f64
                } else {
                    0.0
                }),
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

            return Ok(Value::Struct {
                type_name: "PatternStats".to_string(),
                fields,
            });
        }

        // ── Наряд №72: conv_start — parity with interpreter::invoke_conv_start ──
        if name == "conv_start" {
            let id = match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => return Err("conv_start() requires 1 argument (id: String)".to_string()),
            };
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let mut convs = self
                .conversations
                .lock()
                .map_err(|e| format!("conv_start() lock error: {}", e))?;
            convs.entry(id.clone()).or_insert_with(|| Conversation {
                id: id.clone(),
                messages: Vec::new(),
                created_at: now,
                last_active: now,
                metadata: HashMap::new(),
            });
            return Ok(Value::String(id));
        }

        // ── Наряд №72: conv_add — parity with interpreter::invoke_conv_add ──
        if name == "conv_add" {
            let id = match args.first() {
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

            let mut convs = self
                .conversations
                .lock()
                .map_err(|e| format!("conv_add() lock error: {}", e))?;
            let conv = convs
                .get_mut(&id)
                .ok_or_else(|| format!("conv_add() conversation '{}' not found", id))?;

            if conv.messages.len() >= self.conversation_config.max_messages {
                conv.messages.remove(0);
            }

            conv.messages.push(ConvMessage {
                role,
                text: text.clone(),
                timestamp: now,
            });
            conv.last_active = now;

            return Ok(Value::String(text));
        }

        // ── Наряд №72: conv_history — parity with interpreter::invoke_conv_history ──
        if name == "conv_history" {
            let id = match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => return Err("conv_history() requires 1 argument (id: String)".to_string()),
            };
            let convs = self
                .conversations
                .lock()
                .map_err(|e| format!("conv_history() lock error: {}", e))?;
            let conv = convs
                .get(&id)
                .ok_or_else(|| format!("conv_history() conversation '{}' not found", id))?;

            let mut list = Vec::new();
            for msg in &conv.messages {
                let mut fields = HashMap::new();
                fields.insert("role".to_string(), Value::String(msg.role.clone()));
                fields.insert("text".to_string(), Value::String(msg.text.clone()));
                fields.insert("timestamp".to_string(), Value::Float(msg.timestamp as f64));
                list.push(Value::Struct {
                    type_name: "Message".to_string(),
                    fields,
                });
            }
            return Ok(Value::List(list));
        }

        // ── Наряд №72: conv_context — parity with interpreter::invoke_conv_context ──
        if name == "conv_context" {
            let id = match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => return Err("conv_context() requires 1 argument (id: String)".to_string()),
            };
            let convs = self
                .conversations
                .lock()
                .map_err(|e| format!("conv_context() lock error: {}", e))?;
            let conv = convs
                .get(&id)
                .ok_or_else(|| format!("conv_context() conversation '{}' not found", id))?;

            let mut parts = Vec::new();
            for msg in &conv.messages {
                parts.push(format!("{}: {}", msg.role, msg.text));
            }
            return Ok(Value::String(parts.join("\n")));
        }

        // ── Наряд №72: conv_end — parity with interpreter::invoke_conv_end ──
        if name == "conv_end" {
            let id = match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => return Err("conv_end() requires 1 argument (id: String)".to_string()),
            };
            let mut convs = self
                .conversations
                .lock()
                .map_err(|e| format!("conv_end() lock error: {}", e))?;
            convs.remove(&id);
            return Ok(Value::String("ok".to_string()));
        }

        // ── Наряд №72: event_count — parity with interpreter::event_count ──
        if name == "event_count" {
            let etype = args.first().map(|a| format!("{}", a));
            let count = if let Ok(log) = self.event_log.lock() {
                match etype.as_deref() {
                    Some(t) => log.iter().filter(|e| e.event_type == t).count(),
                    None => log.len(),
                }
            } else {
                0
            };
            return Ok(Value::Float(count as f64));
        }

        // ── Наряд №72: event_sum — parity with interpreter::event_sum ──
        if name == "event_sum" {
            if args.len() < 2 {
                return Err("event_sum() requires 2 arguments (type, field)".to_string());
            }
            let etype = format!("{}", args[0]);
            let field = format!("{}", args[1]);
            let sum = if let Ok(log) = self.event_log.lock() {
                log.iter()
                    .filter(|e| e.event_type == etype)
                    .filter_map(|e| e.data.get(&field))
                    .filter_map(|v| v.parse::<f64>().ok())
                    .sum()
            } else {
                0.0
            };
            return Ok(Value::Float(sum));
        }

        // ── Наряд №72: events_since — parity with interpreter::events_since (inline) ──
        if name == "events_since" {
            let seconds = match args.first() {
                Some(Value::Float(s)) => *s,
                Some(other) => {
                    return Err(format!(
                        "events_since() expected Float, got {}",
                        other.type_name()
                    ))
                }
                None => return Err("events_since() requires 1 argument (seconds)".to_string()),
            };
            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            let since_ms = now_ms.saturating_sub((seconds * 1000.0) as u64);
            let events = if let Ok(log) = self.event_log.lock() {
                log.iter()
                    .filter(|e| e.timestamp >= since_ms)
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
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

        // ── Наряд №72: fit_to_budget — parity with interpreter (identity stub) ──
        if name == "fit_to_budget" {
            let list = match args.first() {
                Some(Value::List(items)) => items.clone(),
                _ => return Err("fit_to_budget() expects first argument to be a List".to_string()),
            };
            return Ok(Value::List(list));
        }

        // Наряд №199 (ADR-0121): intercept reflex_train/reflex_predict
        // BEFORE the generic builtin fallback (which would call the stub).
        // Routes to the VM's own reflex_registry via the shared dispatch
        // functions in src/builtins/reflex.rs.
        if let Some(result) = self.call_reflex_builtin(name, args) {
            return result;
        }

        // Наряд №240 (Vision R4.2): intercept vision_generate/vision_list/
        // vision_export before the generic fallback — routes to the VM's own
        // vision_registry/vision_decls via the shared dispatch functions in
        // src/builtins/vision.rs (лекало call_reflex_builtin).
        if let Some(result) = self.call_vision_builtin(name, args) {
            return result;
        }

        // Наряд №331 (ADR-0162): intercept the unified media family before
        // the generic fallback — routes to the VM's own media_store via the
        // shared dispatch functions in src/builtins/media.rs (лекало
        // call_vision_builtin). Byte egress stays №325-gated on the VM too.
        if let Some(result) = self.call_media_builtin(name, args) {
            return result;
        }

        if let Some(builtin_fn) = self.builtins.get(name) {
            return builtin_fn(args);
        }
        Err(format!("VM: undefined builtin: {}", name))
    }

    /// Наряд №199 + №204 (ADR-0121): intercept all reflex_* builtins before
    /// the generic builtin fallback (which calls the stub that produces
    /// "VM backend does not yet support Reflex"). The intercept routes to
    /// the same shared dispatch functions the interpreter uses (in
    /// `src/builtins/reflex.rs`), passing the VM's own `reflex_registry` and
    /// `reflex_names`. The neural-network logic is NOT reimplemented — only
    /// the argument marshalling and registry access differ from the
    /// interpreter path.
    fn call_reflex_builtin(&mut self, name: &str, args: &[Value]) -> Option<Result<Value, String>> {
        if name == "reflex_train" {
            return Some(crate::builtins::reflex_train_dispatch(
                &mut self.reflex_registry,
                args,
            ));
        }
        if name == "reflex_predict" {
            return Some(crate::builtins::reflex_predict_dispatch(
                &self.reflex_registry,
                args,
            ));
        }
        if name == "reflex_save" {
            return Some(crate::builtins::reflex_save_dispatch(
                &self.reflex_registry,
                &self.reflex_names,
                self.memory_persist_path.as_deref(),
                args,
            ));
        }
        if name == "reflex_load" {
            return Some(crate::builtins::reflex_load_dispatch(
                &mut self.reflex_registry,
                &self.reflex_names,
                self.memory_persist_path.as_deref(),
                args,
            ));
        }
        if name == "reflex_metrics" {
            return Some(crate::builtins::reflex_metrics_dispatch(
                &self.reflex_registry,
                args,
            ));
        }
        if name == "reflex_list" {
            return Some(crate::builtins::reflex_list_dispatch(
                &self.reflex_registry,
                &self.reflex_names,
                args,
            ));
        }
        if name == "reflex_generate" {
            return Some(crate::builtins::reflex_generate_dispatch(
                &self.reflex_registry,
                args,
            ));
        }
        None
    }

    /// Наряд №240 (Vision R4.2): intercept vision_generate/vision_list/
    /// vision_export before the generic builtin fallback. Routes to the
    /// shared dispatch functions in `src/builtins/vision.rs`, passing the
    /// VM's own `vision_decls` and `vision_registry` (лекало
    /// `call_reflex_builtin`: inference logic is NOT reimplemented — only
    /// the argument marshalling and registry access differ from the
    /// interpreter path). Наряд №242 (R6.1): `vision_save`/`vision_load`
    /// are intercepted too — the dispatch additionally receives the VM's
    /// SQLite connection (`db_conn`, opened from `program.db_url`).
    /// Наряд №243 (R6.2): `vision_edit` is intercepted as well
    /// (state-carrying like save/load — the registry, no db involved);
    /// the source must be signed (Block 2.2), the output signs ALWAYS
    /// (Block 2.3).
    fn call_vision_builtin(&mut self, name: &str, args: &[Value]) -> Option<Result<Value, String>> {
        if name == "vision_generate" {
            return Some(crate::builtins::vision_generate_dispatch(
                &self.vision_decls,
                &mut self.vision_registry,
                args,
            ));
        }
        if name == "vision_list" {
            return Some(crate::builtins::vision_list_dispatch(
                &self.vision_registry,
                args,
            ));
        }
        if name == "vision_export" {
            return Some(crate::builtins::vision_export_dispatch(
                &self.vision_registry,
                args,
            ));
        }
        // Наряд №241 (R5, Block 2.1): raw opt-out — same interception
        // pattern, VM-side (ADR-0125 explicit form).
        if name == "vision_export_raw" {
            return Some(crate::builtins::vision_export_raw_dispatch(
                &self.vision_registry,
                args,
            ));
        }
        // Наряд №242 (R6.1): SQLite persistence — the dispatch receives
        // the registry and the VM's database connection (no-db → loud Err
        // naming the `db { url: ... }` declaration; load returns a fresh
        // monotonic session handle, the persisted key is the name).
        // №409: lazy db open on first use (the connection materializes
        // here for save/load/LoRA; vision_edit keeps its no-db contract).
        if name == "vision_save" {
            self.ensure_db_open();
            return Some(crate::builtins::vision_save_dispatch(
                &self.vision_registry,
                self.db_conn.as_ref(),
                args,
            ));
        }
        if name == "vision_load" {
            return Some(crate::builtins::vision_load_dispatch(
                &mut self.vision_registry,
                self.db_conn.as_ref(),
                args,
            ));
        }
        // Наряд №243 (R6.2): in-context editing — state-carrying like
        // save/load (the VM's own registry; no db in the edit contract).
        if name == "vision_edit" {
            return Some(crate::builtins::vision_edit_dispatch(
                &mut self.vision_registry,
                args,
            ));
        }
        // Наряд №244 (R6.3): LoRA adapters — state-carrying like save/load:
        // the load dispatch receives the VM's db connection (the adapter's
        // only home is SQLite — ADR-0124 §6); the generate dispatch
        // additionally owns the VM's vision declarations + registry.
        if name == "vision_lora_load" {
            return Some(crate::builtins::vision_lora_load_dispatch(
                self.db_conn.as_ref(),
                args,
            ));
        }
        if name == "vision_lora_generate" {
            return Some(crate::builtins::vision_lora_generate_dispatch(
                &self.vision_decls,
                &mut self.vision_registry,
                self.db_conn.as_ref(),
                args,
            ));
        }
        None
    }

    /// Наряд №331 (ADR-0162): intercept the unified media family before the
    /// generic builtin fallback. Routes to the VM's own `media_store` via
    /// the SAME shared dispatch functions the interpreter uses (in
    /// `src/builtins/media.rs`) — the store/sealing/refcount logic is NOT
    /// reimplemented per backend.
    fn call_media_builtin(&mut self, name: &str, args: &[Value]) -> Option<Result<Value, String>> {
        use crate::media::MediaKind;
        match name {
            "media_store_image" => Some(crate::builtins::media_store_dispatch(
                &mut self.media_store,
                MediaKind::Image,
                args,
            )),
            "media_store_audio" => Some(crate::builtins::media_store_dispatch(
                &mut self.media_store,
                MediaKind::Audio,
                args,
            )),
            "media_store_video_frame" => Some(crate::builtins::media_store_dispatch(
                &mut self.media_store,
                MediaKind::VideoFrame,
                args,
            )),
            "media_store_video_segment" => Some(crate::builtins::media_store_dispatch(
                &mut self.media_store,
                MediaKind::VideoSegment,
                args,
            )),
            "media_save" => Some(crate::builtins::media_save_dispatch(
                &self.media_store,
                args,
            )),
            // №397 (kitchen-camera e2e): consent as a RUNTIME credential —
            // the same shared dispatches the interpreter uses (the scope
            // lands on the VM's own store entry; the runtime twin of the
            // static consented-egress rule).
            "consent_grant" => Some(crate::builtins::consent::consent_grant_dispatch(
                &mut self.media_store,
                args,
            )),
            "consent_revoke" => Some(crate::builtins::consent::consent_revoke_dispatch(
                &mut self.media_store,
                args,
            )),
            "media_retain" => Some(crate::builtins::media_retain_dispatch(
                &mut self.media_store,
                args,
            )),
            "media_release" => Some(crate::builtins::media_release_dispatch(
                &mut self.media_store,
                args,
            )),
            "media_meta" => Some(crate::builtins::media_meta_dispatch(
                &self.media_store,
                args,
            )),
            // №337 (ADR-0166 §2.4): the in-program provenance read —
            // entry-level manifest facts, no byte movement.
            "media_manifest" => Some(crate::builtins::media_manifest_dispatch(
                &self.media_store,
                args,
            )),
            "media_source_capture" => Some(crate::builtins::media_source_capture_dispatch(
                &mut self.media_store,
                &self.origin_decls,
                args,
            )),
            "media_bind_origin" => Some(crate::builtins::media_bind_origin_dispatch(
                &mut self.media_store,
                &self.origin_decls,
                args,
            )),
            _ => None,
        }
    }

    // ── Наряд №205 (ADR-0121 stage 6): distillation state machine ──────
    //
    // Ported from `src/interpreter/learnable.rs` lines 406-730.
    // The VM is single-threaded per request (&mut self), so no Mutex locks
    // are needed — direct field access. The `simple_embedding` function
    // is ported verbatim for byte-for-byte determinism (ADR-0121 requires
    // same seed → same output across both backends).

    fn try_distilled_call(
        &mut self,
        pattern_name: &str,
        info: &CompiledLearnableInfo,
        input: &str,
    ) -> Result<Option<Value>, String> {
        use crate::interpreter::types::{DistillMode, DistillRuntimeState};

        let distill_to = info
            .distill_to
            .as_ref()
            .ok_or_else(|| "distill: distill_to not configured".to_string())?;
        let distill_after = info.distill_after;
        let fallback_if = info.fallback_if;
        // №456: the holdout-accuracy gate — explicit `distill_min_accuracy`
        // or the 0.85 default.
        let min_accuracy = info.distill_min_accuracy.unwrap_or(0.85);

        let state = self
            .distill_states
            .entry(pattern_name.to_string())
            .or_insert_with(|| DistillRuntimeState {
                mode: DistillMode::Teaching,
                examples: Vec::new(),
                last_train_attempt: 0,
            });

        match state.mode {
            DistillMode::Teaching => {
                let count = state.examples.len();
                let last_attempt = state.last_train_attempt;
                let training_threshold = std::cmp::max(distill_after, 10);
                let should_attempt =
                    count >= training_threshold && (last_attempt == 0 || count - last_attempt >= 5);

                if should_attempt {
                    let examples = state.examples.clone();
                    let trained_at_count = count;
                    match self.try_train_distilled_model(
                        pattern_name,
                        distill_to,
                        min_accuracy,
                        &examples,
                    ) {
                        Ok(true) => {
                            if let Some(s) = self.distill_states.get_mut(pattern_name) {
                                s.mode = DistillMode::Distilled;
                            }
                            // Recursive call to enter DISTILLED path.
                            return self.try_distilled_call(pattern_name, info, input);
                        }
                        Ok(false) => {
                            if let Some(s) = self.distill_states.get_mut(pattern_name) {
                                s.last_train_attempt = trained_at_count;
                            }
                            return Ok(None);
                        }
                        Err(e) => return Err(e),
                    }
                }
                Ok(None)
            }
            DistillMode::Distilled => {
                let model_id = self.reflex_names.get(distill_to).copied().ok_or_else(|| {
                    format!(
                        "distill: reflex '{}' not declared for pattern '{}'",
                        distill_to, pattern_name
                    )
                })?;

                let model_kind = self.reflex_registry.get(model_id).ok_or_else(|| {
                    format!("distill: model handle {:?} not in registry", model_id)
                })?;

                let (input_size, probs, labels): (usize, Vec<f64>, Vec<String>) = match model_kind {
                    crate::nn::ModelKind::Dense(model) => {
                        let embedding = self.simple_embedding(input, model.input_size);
                        let probs = model.forward(&embedding);
                        (model.input_size, probs, model.labels.clone())
                    }
                    #[cfg(feature = "candle")]
                    crate::nn::ModelKind::Sequence(_) => {
                        return Err(
                            "distill: sequence models do not yet support distill_to".to_string()
                        );
                    }
                    #[cfg(feature = "candle")]
                    crate::nn::ModelKind::Gen(_) => {
                        return Err("distill: gen models do not support distill_to".to_string());
                    }
                };
                let _ = input_size;

                let (best_idx, best_prob) = probs
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(i, &p)| (i, p))
                    .unwrap_or((0, 0.0));

                let best_label = labels
                    .get(best_idx)
                    .cloned()
                    .unwrap_or_else(|| format!("label_{}", best_idx));

                // №456: fail-closed on a non-finite confidence — a broken
                // model must never answer as a confident one. Fall back to
                // the LLM (same contract as the TW path).
                if !best_prob.is_finite() {
                    return Ok(None);
                }

                // Check fallback threshold. №456: the barrier defaults to
                // `confidence < 0.7` — no barrier-free distilled mode.
                let (op, threshold) = fallback_if.unwrap_or((ConditionOp::Lt, 0.7));
                if op.compare(best_prob, threshold) {
                    return Ok(None);
                }

                Ok(Some(Value::String(best_label)))
            }
        }
    }

    fn try_train_distilled_model(
        &mut self,
        pattern_name: &str,
        reflex_name: &str,
        min_accuracy: f64,
        examples: &[(String, String)],
    ) -> Result<bool, String> {
        let model_id = self
            .reflex_names
            .get(reflex_name)
            .copied()
            .ok_or_else(|| format!("distill: reflex '{}' not declared", reflex_name))?;
        let (input_size, labels) = {
            let model_kind = self
                .reflex_registry
                .get(model_id)
                .ok_or_else(|| format!("distill: model handle {:?} not in registry", model_id))?;
            match model_kind {
                crate::nn::ModelKind::Dense(m) => (m.input_size, m.labels.clone()),
                #[cfg(feature = "candle")]
                crate::nn::ModelKind::Sequence(_) => {
                    return Err("distill: sequence models do not support distill training".into())
                }
                #[cfg(feature = "candle")]
                crate::nn::ModelKind::Gen(_) => {
                    return Err("distill: gen models do not support distill training".into())
                }
            }
        };

        let mut inputs: Vec<Vec<f64>> = Vec::with_capacity(examples.len());
        let mut targets: Vec<usize> = Vec::with_capacity(examples.len());
        for (input_str, output_str) in examples {
            let target_idx = match labels.iter().position(|l| l == output_str) {
                Some(idx) => idx,
                None => continue,
            };
            let embedding = self.simple_embedding(input_str, input_size);
            inputs.push(embedding);
            targets.push(target_idx);
        }

        if inputs.is_empty() {
            return Ok(false);
        }

        // №456: the holdout gate — mirror of the TW path (parity). A
        // holdout smaller than MIN_HOLDOUT cannot support a meaningful
        // accuracy read: refuse the switch, stay TEACHING.
        let valid_n = inputs.len();
        let holdout_n = valid_n - (valid_n * 4) / 5;
        if holdout_n < crate::interpreter::learnable::MIN_HOLDOUT {
            self.push_audit(format!(
                "[AUDIT] distill.rejected: {} holdout too small ({} < {}) — staying TEACHING",
                pattern_name,
                holdout_n,
                crate::interpreter::learnable::MIN_HOLDOUT
            ));
            return Ok(false);
        }

        let model_kind = self
            .reflex_registry
            .get_mut(model_id)
            .ok_or_else(|| format!("distill: model handle {:?} not in registry", model_id))?;
        match model_kind {
            crate::nn::ModelKind::Dense(model) => {
                // №456: the holdout accuracy is the SWITCH GATE — mirror of
                // the TW path. Loud on rejection.
                let (loss, holdout_acc) = model.train(&inputs, &targets, 30, 0.1)?;
                if !loss.is_finite() {
                    return Ok(false);
                }
                if holdout_acc < min_accuracy {
                    self.push_audit(format!(
                        "[AUDIT] distill.rejected: {} holdout_accuracy={:.3} < min_accuracy={:.2} — staying TEACHING",
                        pattern_name, holdout_acc, min_accuracy
                    ));
                    return Ok(false);
                }
                Ok(true)
            }
            #[cfg(feature = "candle")]
            crate::nn::ModelKind::Sequence(_) => {
                Err("distill: sequence models do not support distill training".into())
            }
            #[cfg(feature = "candle")]
            crate::nn::ModelKind::Gen(_) => {
                Err("distill: gen models do not support distill training".into())
            }
        }
    }

    fn record_distill_example(&mut self, pattern_name: &str, input: &str, output: &str) {
        use crate::interpreter::types::{DistillMode, DistillRuntimeState};
        let state = self
            .distill_states
            .entry(pattern_name.to_string())
            .or_insert_with(|| DistillRuntimeState {
                mode: DistillMode::Teaching,
                examples: Vec::new(),
                last_train_attempt: 0,
            });
        state.examples.push((input.to_string(), output.to_string()));
    }

    /// Simple deterministic embedding for distillation input strings.
    /// Ported verbatim from `src/interpreter/learnable.rs::simple_embedding`
    /// for byte-for-byte determinism (ADR-0121).
    fn simple_embedding(&self, input: &str, dim: usize) -> Vec<f64> {
        let mut embedding = vec![0.0; dim];
        let bytes = input.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            let bucket = i % dim;
            embedding[bucket] += (b as f64) * 0.01;
            if b != 0 {
                embedding[(bucket + 1) % dim] =
                    (embedding[(bucket + 1) % dim] * 0.99) + (b as f64) * 0.001;
            }
        }
        let max_val = embedding.iter().cloned().fold(0.0f64, f64::max).max(1.0);
        for v in &mut embedding {
            *v /= max_val;
        }
        embedding
    }

    /// Call an LLM-backed learnable pattern.
    fn call_llm(&mut self, idx: usize, args: &[Value]) -> Result<Value, String> {
        let (info_clone, few_shot_clone) = {
            let (info, few_shot) = self
                .learnables
                .get(idx)
                .ok_or_else(|| format!("VM: learnable index {} not found", idx))?;
            (info.clone(), few_shot.clone())
        };
        let info = &info_clone;
        let few_shot = &few_shot_clone;

        // Build input string
        let input_parts: Vec<String> = args.iter().map(|a| format!("{}", a)).collect();
        let input = input_parts.join(", ");

        // Наряд №205 (ADR-0121 stage 6): distillation routing.
        // If this learnable pattern has a distill_to configured, check
        // whether we can serve from the distilled model (DISTILLED mode)
        // or need to accumulate more examples (TEACHING mode).
        if info.distill_to.is_some() {
            match self.try_distilled_call(&info.name, info, &input) {
                Ok(Some(value)) => {
                    // DISTILLED path succeeded with a confident prediction.
                    return Ok(value);
                }
                Ok(None) => {
                    // TEACHING mode OR fallback_if triggered — fall through
                    // to LLM call, then record the example.
                }
                Err(e) => {
                    // Safe degradation (ADR-0117 §3): log + fall through.
                    eprintln!(
                        "[vm/distill] error in pattern '{}': {} — falling back to LLM",
                        info.name, e
                    );
                }
            }
        }

        // Check few-shot cache first
        for (example_input, example_output) in few_shot {
            if input == *example_input {
                return Ok(Value::String(example_output.clone()));
            }
        }

        // Build effective prompt with context prefix (matches interpreter)
        let effective_prompt = match &info.context_mode {
            CompiledContextMode::Literal(ctx) => {
                format!("{}\n{}", ctx, info.prompt)
            }
            CompiledContextMode::Auto => {
                // Use first arg value as recall query
                let query = args.first().map(|a| format!("{}", a)).unwrap_or_default();
                let facts = self.recall_top(&query, 5);
                if facts.is_empty() {
                    info.prompt.clone()
                } else {
                    let mut block = String::from("Relevant context:\n");
                    for fact in &facts {
                        block.push_str("- ");
                        block.push_str(fact);
                        block.push('\n');
                    }
                    format!("{}\n{}", block, info.prompt)
                }
            }
            CompiledContextMode::Recall(_param_name, limit) => {
                // Use first arg as recall query
                let query = args.first().map(|a| format!("{}", a)).unwrap_or_default();
                let facts = self.recall_top(&query, *limit);
                if facts.is_empty() {
                    info.prompt.clone()
                } else {
                    let mut block = String::from("Relevant context:\n");
                    for fact in &facts {
                        block.push_str("- ");
                        block.push_str(fact);
                        block.push('\n');
                    }
                    format!("{}\n{}", block, info.prompt)
                }
            }
            CompiledContextMode::None => info.prompt.clone(),
        };

        // Call LLM backend
        // Наряд №276: legacy VM learnable calls are traced HERE (the
        // SmartRouter path, when a router is installed, traces inside
        // SmartRouter::call — one line per actual call, never both).
        let t0 = std::time::Instant::now();
        let backend = llm::create_llm_backend();
        let llm_result = backend.call(&effective_prompt, &input);
        let mock_mode = crate::llm::mock_llm_requested();
        crate::llm::trace_llm_call(&crate::llm::LlmTraceEvent {
            provider_name: Some(if mock_mode {
                "mock"
            } else {
                llm::provider_env_name()
            }),
            model: None,
            input_tokens: None,
            output_tokens: None,
            latency_ms: t0.elapsed().as_millis() as u64,
            status: if llm_result.is_ok() { "ok" } else { "error" },
            cache: "miss",
            provider_alias: None,
        });
        let response = llm_result?;

        // Наряд №205: record (input, output) example for distillation training.
        if info.distill_to.is_some() {
            self.record_distill_example(&info.name, &input, &response);
        }

        // Try to parse JSON response into Value::Struct
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response) {
            if let Some(obj) = json.as_object() {
                let mut fields = std::collections::HashMap::new();
                for (k, v) in obj {
                    fields.insert(k.clone(), Vm::json_to_value(v));
                }
                return Ok(Value::Struct {
                    type_name: "LlmResponse".to_string(),
                    fields,
                });
            }
        }

        Ok(Value::String(response))
    }

    /// Convert serde_json::Value to METALOGOS Value (for VM).
    fn json_to_value(json: &serde_json::Value) -> Value {
        match json {
            serde_json::Value::String(s) => Value::String(s.clone()),
            serde_json::Value::Number(n) => Value::Float(n.as_f64().unwrap_or(0.0)),
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Null => Value::Unit,
            serde_json::Value::Array(arr) => {
                Value::List(arr.iter().map(Vm::json_to_value).collect())
            }
            serde_json::Value::Object(obj) => {
                let mut fields = std::collections::HashMap::new();
                for (k, v) in obj {
                    fields.insert(k.clone(), Vm::json_to_value(v));
                }
                Value::Struct {
                    type_name: "Json".to_string(),
                    fields,
                }
            }
        }
    }

    /// Execute a flow step: check branches, then invoke pattern/builtin.
    fn run_flow_step(
        &mut self,
        step_name: &str,
        current: Value,
        branch_defs: &[(String, Vec<BranchDef>)],
    ) -> Result<Value, String> {
        // Check if step has branch definitions
        for (bd_step, branches) in branch_defs {
            if bd_step == step_name {
                for branch in branches {
                    if self.eval_branch_condition(branch, &current)? {
                        return self.invoke_step(&branch.target, vec![current]);
                    }
                }
                return Err(format!("VM: no branch matched in step '{}'", step_name));
            }
        }

        // No branch definitions — invoke as pattern/builtin
        self.invoke_step(step_name, vec![current])
    }

    /// Evaluate a branch condition against a value.
    fn eval_branch_condition(&self, branch: &BranchDef, current: &Value) -> Result<bool, String> {
        let field_val = current
            .get_field(&branch.condition_field)
            .map_err(|e| format!("branch condition: {}", e))?
            .clone();
        let fv = field_val.as_float()?;
        let tv = branch.condition_threshold.as_float()?;
        Ok(match branch.condition_op {
            ConditionOp::Gt => fv > tv,
            ConditionOp::Lt => fv < tv,
            ConditionOp::Ge => fv >= tv,
            ConditionOp::Le => fv <= tv,
            ConditionOp::Eq => fv == tv,
            ConditionOp::Ne => fv != tv,
        })
    }

    /// Invoke a pattern or builtin by name (used in flow pipeline).
    fn invoke_step(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        // Check learnables first
        for (i, (info, _)) in self.learnables.iter().enumerate() {
            if info.name == name {
                return self.call_llm(i, &args);
            }
        }

        // Check patterns
        for pattern in self.patterns.iter() {
            if pattern.name == name {
                if args.len() != pattern.param_count {
                    return Err(format!(
                        "VM: pattern {} expects {} args, got {}",
                        name,
                        pattern.param_count,
                        args.len()
                    ));
                }

                // ── VM bytecode path ───────────────────────────
                // Clone everything needed before mutable self borrow
                let param_types = pattern.param_types.clone();
                let code = pattern.code.clone();

                // ADR-0089: reset propagated confidence before collapse
                self.propagated_confidence = 1.0;
                // Collapse Fluid arguments to parameter types
                let collapsed_args: Vec<Value> = args
                    .iter()
                    .zip(param_types.iter())
                    .map(|(arg, param_type)| self.maybe_collapse(arg, param_type))
                    .collect();
                // Execute pattern body as bytecode
                let mut stack: Vec<Value> = Vec::new();
                let mut call_stack: Vec<CallFrame> = vec![CallFrame {
                    return_ip: 0,
                    base_bp: 0,
                }];
                let program = Program {
                    globals: Vec::new(),
                    patterns: std::sync::Arc::new(Vec::new()),
                    learnables: Vec::new(),
                    rules: Vec::new(),
                    skill_indices: Vec::new(),
                    reflex_decls: Vec::new(),
                    origin_decls: Vec::new(),
                    reflex_seq_decls: Vec::new(),
                    reflex_gen_decls: Vec::new(),
                    vision_decls: Vec::new(),
                    deny_handlers: Vec::new(),
                    memory_persist_path: None,
                    db_url: None,
                    schema_ddl: Vec::new(),
                    main_code: Vec::new(),
                    collections_loaded: false,
                    shared_cache: crate::bytecode::ProgramSharedCache::new(),
                };
                for arg in collapsed_args {
                    stack.push(arg);
                }
                let result = self.execute_code(&code, &mut stack, &mut call_stack, &program);
                // ADR-0089: wrap result as Fluid if confidence was propagated
                return Self::vm_wrap_with_confidence(result, self.propagated_confidence);
            }
        }

        // Check builtins
        self.call_builtin(name, &args)
    }

    /// Execute all registered rules with priority-ordered, first-wins semantics.
    /// ADR-0090: rules sorted by priority descending; stable sort preserves
    /// declaration order for ties. For each (entity, field) pair, only the
    /// first matching rule writes the field. Rules targeting different fields all fire.
    fn execute_rules(&mut self) -> Result<(), String> {
        // ADR-0090: sort by priority descending (stable sort keeps declaration order for ties)
        let mut sorted_rules: Vec<&CompiledRule> = self.rules.iter().collect();
        sorted_rules.sort_by_key(|b| std::cmp::Reverse(b.priority));

        // Track which (entity_name, field_name) pairs have already been written
        let mut written: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();

        for rule in sorted_rules {
            // Skip if this field was already written by a higher-priority rule
            if written.contains(&(rule.target_name.clone(), rule.field.clone())) {
                continue;
            }

            let condition_met = self.eval_rule_condition(&rule.condition)?;
            if condition_met {
                // Find the target entity by name in globals
                let target_slot = self
                    .global_names
                    .iter()
                    .position(|n| n == &rule.target_name);
                if let Some(slot) = target_slot {
                    let value = self.eval_rule_value(&rule.value_expr)?;
                    // Set field on the struct
                    if let Some(entity) = self.globals.get_mut(slot) {
                        let _ = entity.set_field(&rule.field, value);
                        // Mark this (entity, field) as written — first-wins
                        written.insert((rule.target_name.clone(), rule.field.clone()));
                    }
                }
            }
        }
        Ok(())
    }

    /// Evaluate a rule condition.
    fn eval_rule_condition(&self, cond: &RuleCondition) -> Result<bool, String> {
        match cond {
            RuleCondition::Contains { left, right } => {
                let ls = self.eval_rule_value(left)?;
                let rs = self.eval_rule_value(right)?;
                match (&ls, &rs) {
                    (Value::String(l), Value::String(r)) => Ok(l.contains(r)),
                    _ => Err("contains: both sides must be String".to_string()),
                }
            }
            RuleCondition::Compare { left, op, right } => {
                let lv = self.eval_rule_value(left)?;
                let rv = self.eval_rule_value(right)?;
                let lf = lv.as_float()?;
                let rf = rv.as_float()?;
                Ok(match op {
                    ConditionOp::Gt => lf > rf,
                    ConditionOp::Lt => lf < rf,
                    ConditionOp::Ge => lf >= rf,
                    ConditionOp::Le => lf <= rf,
                    ConditionOp::Eq => lf == rf,
                    ConditionOp::Ne => lf != rf,
                })
            }
        }
    }

    /// Evaluate a rule value expression to a runtime value.
    fn eval_rule_value(&self, expr: &RuleValueExpr) -> Result<Value, String> {
        match expr {
            RuleValueExpr::StringLit(s) => Ok(Value::String(s.clone())),
            RuleValueExpr::FloatLit(f) => Ok(Value::Float(*f)),
            RuleValueExpr::Ident(name) => {
                // Search globals by name
                let slot = self.global_names.iter().position(|n| n == name);
                match slot {
                    Some(s) => Ok(self.globals.get(s).cloned().unwrap_or(Value::Unit)),
                    None => Err(format!("VM rule: undefined variable '{}'", name)),
                }
            }
            RuleValueExpr::FieldAccess(entity_name, field) => {
                let slot = self.global_names.iter().position(|n| n == entity_name);
                match slot {
                    Some(s) => {
                        let entity = self.globals.get(s).cloned().unwrap_or(Value::Unit);
                        entity
                            .get_field(field)
                            .cloned()
                            .map_err(|e| format!("VM rule field access: {}", e))
                    }
                    None => Err(format!("VM rule: undefined entity '{}'", entity_name)),
                }
            }
        }
    }

    /// Handle a mutate declaration.
    fn handle_mutate(
        &mut self,
        pattern_name: &str,
        new_examples: Vec<(String, String)>,
        rollback_threshold: Option<f64>,
        rollback_op: Option<ConditionOp>,
    ) -> Result<String, String> {
        // Find the learnable
        let idx = self
            .learnables
            .iter()
            .position(|(info, _)| info.name == pattern_name)
            .ok_or_else(|| format!("VM mutate: learnable pattern '{}' not found", pattern_name))?;

        let original = self.learnables[idx].1.clone();
        let base_prompt = self.learnables[idx].0.prompt.clone();
        let build_inputs: std::collections::HashSet<String> =
            new_examples.iter().map(|(i, _)| i.clone()).collect();
        self.learnables[idx].1 = new_examples;

        // ── Accuracy: REAL golden-task battery (№375, ADR-0112 addendum) ──
        // Mock mode (METALOGOS_MOCK_LLM — explicit-only since №454): the 0.95
        // stub stays — loudly documented in ADR-0112. Real mode: the battery is the
        // pre-mutation few-shot (the VM's Program carries no eval blocks —
        // the TW path additionally merges ADR-0050 eval datasets; the
        // difference is documented in the наряд report). The answer path is
        // the pattern's real LLM call; errors count as incorrect.
        let mock_mode = crate::llm::mock_llm_requested();
        let (accuracy, battery_note) = if mock_mode {
            // Mock accuracy (always 0.95 for MockLlm) — test mode only.
            (0.95, String::new())
        } else {
            let report = crate::interpreter::learnable::measure_battery_accuracy(
                &original,
                &build_inputs,
                |input| {
                    let backend = llm::create_llm_backend();
                    backend.call(&base_prompt, input)
                },
            );
            let note = format!(
                " (battery: {} tasks, held-out {}, correct {}{})",
                report.battery_size,
                report.held_out,
                report.correct,
                if report.below_minimum {
                    ", BELOW MINIMUM 20"
                } else {
                    ""
                }
            );
            if report.held_out == 0 {
                eprintln!(
                    "[MUTATE] WARNING: no held-out battery tasks for '{}' — accuracy counts as 0.0 (no evidence, no keep)",
                    pattern_name
                );
            }
            (report.accuracy, note)
        };

        let kept = match (&rollback_op, &rollback_threshold) {
            (Some(ConditionOp::Lt), Some(threshold)) => accuracy >= *threshold,
            (Some(ConditionOp::Le), Some(threshold)) => accuracy > *threshold,
            (Some(ConditionOp::Gt), Some(_)) | (Some(ConditionOp::Ge), Some(_)) => false,
            (Some(ConditionOp::Eq), Some(threshold)) => (accuracy - threshold).abs() < 1e-9,
            _ => true,
        };

        if kept {
            Ok(format!(
                "[MUTATE] {}: accuracy={}, kept (>= {:.1}){}",
                pattern_name,
                accuracy,
                rollback_threshold.unwrap_or(0.0),
                battery_note
            ))
        } else {
            self.learnables[idx].1 = original;
            Ok(format!(
                "[MUTATE] {}: accuracy={}, rolled back (below {:.1}){}",
                pattern_name,
                accuracy,
                rollback_threshold.unwrap_or(0.0),
                battery_note
            ))
        }
    }

    /// Recall from memory: find best matching entry by substring + decay.
    fn recall(&self, query: &str, min_confidence: f64) -> String {
        // №466: the store-lane body moved to the shared live module
        // (src/memory_ops.rs) — delegation, same semantics.
        crate::memory_ops::store_recall_vm(&self.memory, &self.relations, query, min_confidence)
    }

    /// Recall up to `limit` memory entries matching query, sorted by activation.
    fn recall_top(&self, query: &str, limit: usize) -> Vec<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut scored: Vec<(String, f64)> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for entry in &self.memory {
            if !entry.value.contains(query) {
                continue;
            }
            if seen.contains(&entry.value) {
                continue;
            }
            seen.insert(entry.value.clone());
            let age_days = ((now - entry.timestamp).max(0) as f64) / 86400.0;
            let activation = entry.priority * (-entry.decay_rate * age_days).exp();
            scored.push((entry.value.clone(), activation));
        }
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(limit).map(|(v, _)| v).collect()
    }

    // ── Server context setters (per-request, for route execution) ──

    /// Set the parsed JSON request body (Наряд №40: VM server backend).
    pub fn set_server_json_body(&mut self, val: Value) {
        self.server_json_body = Some(val);
    }

    /// Set query string parameters (Наряд №40: VM server backend).
    pub fn set_server_query_params(&mut self, params: std::collections::HashMap<String, String>) {
        self.server_query_params = Some(params);
    }

    /// Set path parameters extracted from a templated route (Наряд №283).
    /// Parity with `set_server_query_params` — the `server_path_param(name)`
    /// builtin reads from this map.
    pub fn set_server_path_params(&mut self, params: std::collections::HashMap<String, String>) {
        self.server_path_params = Some(params);
    }

    /// Set user roles for RBAC (Наряд №40: VM server backend).
    pub fn set_server_user_roles(&mut self, roles: Vec<String>) {
        self.server_user_roles = roles;
    }

    /// Clear server context (reset per-request state for isolation).
    pub fn clear_server_context(&mut self) {
        self.server_json_body = None;
        self.server_query_params = None;
        self.server_path_params = None;
        self.server_user_roles = Vec::new();
    }

    // ── Audit log (Наряд №41 Block 2: parity with interpreter) ──

    /// Push an audit log entry.
    pub fn push_audit(&self, entry: String) {
        self.audit_log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(entry);
    }

    /// Take all audit log entries (consuming them).
    pub fn take_audit_log(&self) -> Vec<String> {
        self.audit_log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .drain(..)
            .collect()
    }

    // ── Route execution (Наряд №40: VM server backend) ──

    /// Execute a compiled route body on the VM.
    ///
    /// This is the VM equivalent of `execute_route_body` in server.rs.
    /// It creates a fresh stack, executes the compiled bytecode, and returns
    /// the result value (typically `Value::HttpResponse` from `respond()`).
    ///
    /// **Isolation**: each call gets a fresh stack — no state leaks between requests.
    /// **Server context**: must be set via `set_server_*` methods before calling.
    pub fn execute_route_code(
        &mut self,
        route: &CompiledRoute,
        program: &Program,
    ) -> Result<Value, String> {
        // Наряд №276: same backend tag contract as run() — server VM routes
        // trace with backend="vm" (each request runs on its own thread).
        let prev_tag = crate::llm::set_llm_backend_tag("vm");
        let mut stack: Vec<Value> = Vec::new();
        let mut call_stack: Vec<CallFrame> = Vec::new();
        let out = self.execute_code(&route.code, &mut stack, &mut call_stack, program);
        crate::llm::set_llm_backend_tag(prev_tag);
        out
    }

    /// ADR-0089: If confidence < 1.0, wrap a concrete result as Fluid
    /// with the propagated confidence (VM version).
    fn vm_wrap_with_confidence(
        result: Result<Value, String>,
        confidence: f64,
    ) -> Result<Value, String> {
        if confidence < 1.0 {
            result.map(|val| match val {
                Value::Fluid(_) => val,
                Value::Unit => val,
                concrete => Value::Fluid(vec![crate::interpreter::values::FluidValueVariant {
                    type_name: concrete.type_name().to_string(),
                    value: concrete,
                    confidence,
                }]),
            })
        } else {
            result
        }
    }

    /// Collapse a Fluid value to a concrete type.
    fn maybe_collapse(&mut self, value: &Value, required_type: &str) -> Value {
        match value {
            Value::Fluid(variants) => {
                // If the required type IS Fluid, pass through without collapsing
                if required_type == "Fluid" {
                    // ADR-0089: propagate confidence — min of max variant confidence
                    let max_conf = variants
                        .iter()
                        .map(|v| v.confidence)
                        .fold(0.0_f64, f64::max);
                    self.propagated_confidence = self.propagated_confidence.min(max_conf);
                    return value.clone();
                }
                let best = variants
                    .iter()
                    .filter(|v| v.type_name == required_type)
                    .max_by(|a, b| {
                        a.confidence
                            .partial_cmp(&b.confidence)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                match best {
                    Some(variant) if variant.confidence >= COLLAPSE_THRESHOLD => {
                        // ADR-0089: propagate confidence — track min of all collapses
                        self.propagated_confidence =
                            self.propagated_confidence.min(variant.confidence);
                        variant.value.clone()
                    }
                    _ => Value::Unit,
                }
            }
            other => other.clone(),
        }
    }

    /// Evaluate a binary operation.
    /// Наряд №371 (ADR-0141 Stage 1.3): binop semantics aligned with the TW
    /// interpreter (`src/interpreter/execution.rs` `eval_binop`). The VM is no
    /// longer stricter than TW on heterogeneous operands: both reject `+` for
    /// non-(String|Float) pairs with the SAME loud message, both enforce the
    /// same opaque-type restriction on concatenation and the same
    /// MAX_STRING_LENGTH (1 MB) limit. The old VM messages ("type mismatch:
    /// List Add String", "cannot apply Div to two Strings") diverged from TW
    /// wording and broke TW↔VM error parity.
    fn eval_binop(
        &self,
        left: Value,
        op: crate::ast::BinOp,
        right: Value,
    ) -> Result<Value, String> {
        // Same limit as TW (`Interpreter::MAX_STRING_LENGTH`).
        const MAX_STRING_LENGTH: usize = 1_000_000; // 1 MB

        // Enforce opaque type restrictions for Add (concatenation) — TW parity.
        if matches!(op, crate::ast::BinOp::Add) {
            if Self::is_opaque_value(&left) {
                return Err(format!(
                    "cannot concatenate opaque type {}",
                    left.type_name()
                ));
            }
            if Self::is_opaque_value(&right) {
                return Err(format!(
                    "cannot concatenate opaque type {}",
                    right.type_name()
                ));
            }
        }
        match (op, left, right) {
            // String concatenation — with the same length limit as TW.
            (crate::ast::BinOp::Add, Value::String(a), Value::String(b)) => {
                let result = format!("{}{}", a, b);
                if result.len() > MAX_STRING_LENGTH {
                    Err(format!(
                        "string length {} exceeds maximum allowed {}",
                        result.len(),
                        MAX_STRING_LENGTH
                    ))
                } else {
                    Ok(Value::String(result))
                }
            }
            // Arithmetic on Floats.
            (crate::ast::BinOp::Add, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (crate::ast::BinOp::Sub, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (crate::ast::BinOp::Mul, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (crate::ast::BinOp::Div, Value::Float(a), Value::Float(b)) => {
                if b == 0.0 {
                    Err("division by zero".to_string())
                } else {
                    Ok(Value::Float(a / b))
                }
            }
            // Heterogeneous Add — the same loud error as TW.
            (crate::ast::BinOp::Add, l, r) => Err(format!(
                "type mismatch in string concatenation: {} + {} (use to_string() explicitly)",
                l.type_name(),
                r.type_name()
            )),
            // Everything else — the same message as TW.
            (_, l, r) => Err(format!(
                "type mismatch in binary operation: {} {:?} {}",
                l.type_name(),
                op,
                r.type_name()
            )),
        }
    }

    /// Opaque types cannot be concatenated (№371 — same set as the TW's
    /// `Interpreter::is_opaque_type`).
    fn is_opaque_value(v: &Value) -> bool {
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

    /// Evaluate contains(left, right).
    /// №372: returns `Value::Bool` — TW parity (the shared `builtin_contains`
    /// returns Bool; the old Float 1.0/0.0 encoding printed "1"/"0" instead of
    /// "true"/"false" through `to_string`).
    fn eval_contains(&self, left: Value, right: Value) -> Result<Value, String> {
        let ls = match left {
            Value::String(s) => s,
            other => {
                return Err(format!(
                    "contains: left must be String, got {}",
                    other.type_name()
                ))
            }
        };
        let rs = match right {
            Value::String(s) => s,
            other => {
                return Err(format!(
                    "contains: right must be String, got {}",
                    other.type_name()
                ))
            }
        };
        Ok(Value::Bool(ls.contains(&rs)))
    }

    /// Evaluate a comparison: push Bool (true/false).
    /// №372 (ADR-0141 Stage 1.4): the result type is `Value::Bool` — TW parity
    /// (`eval_binop` in the interpreter returns Bool for all comparisons; the
    /// old Float 1.0/0.0 encoding made `to_string(a == b)` print "1" on the VM
    /// where TW prints "true"). Truthiness (JumpIfNot) is unchanged — Bool and
    /// Float 0.0/1.0 are truthy-equivalent.
    fn eval_cmp(&self, left: Value, right: Value, op: AstCompareOp) -> Value {
        // String-string comparisons (Eq, Ne, contains-like)
        match (&left, &right) {
            (Value::String(a), Value::String(b)) => match op {
                AstCompareOp::Eq => Value::Bool(a == b),
                AstCompareOp::Ne => Value::Bool(a != b),
                AstCompareOp::Gt => Value::Bool(a > b),
                AstCompareOp::Lt => Value::Bool(a < b),
                AstCompareOp::Ge => Value::Bool(a >= b),
                AstCompareOp::Le => Value::Bool(a <= b),
            },
            _ => {
                // Numeric comparisons (Float/Bool via as_float)
                let result = match (left.as_float(), right.as_float()) {
                    (Ok(lf), Ok(rf)) => match op {
                        AstCompareOp::Gt => lf > rf,
                        AstCompareOp::Lt => lf < rf,
                        AstCompareOp::Ge => lf >= rf,
                        AstCompareOp::Le => lf <= rf,
                        AstCompareOp::Eq => lf == rf,
                        AstCompareOp::Ne => lf != rf,
                    },
                    _ => false,
                };
                Value::Bool(result)
            }
        }
    }
}

/// Truthiness check (matches interpreter).
fn is_truthy(value: &Value) -> bool {
    match value {
        Value::String(s) => !s.is_empty(),
        Value::Float(f) => *f != 0.0,
        Value::Bool(b) => *b,
        Value::List(items) => !items.is_empty(),
        _ => false,
    }
}

// ── Runtime label seeds (Наряд №328, ADR-0156) ───────────────────────

/// The runtime seed label of a №316 Source builtin — the runtime twin of
/// the static №323 mapping: Secret sources are `(private, trusted)`,
/// every other source is untrusted ingress `(public, untrusted)`.
fn runtime_source_label(name: &str) -> crate::labels::Label {
    use crate::labels::{Conf, Integrity, Label};
    match crate::builtins_classification::classify(name) {
        Some(class) if class.role == crate::builtins_classification::Role::Source => {
            if class.default_label == crate::builtins_classification::Label::Secret {
                Label {
                    conf: Conf::Private,
                    integrity: Integrity::Trusted,
                    consent: Default::default(),
                }
            } else {
                Label {
                    conf: Conf::Public,
                    integrity: Integrity::Untrusted,
                    consent: Default::default(),
                }
            }
        }
        _ => Label::bottom(),
    }
}

// ── Наряд №403: fail-closed reset contract (pooled serve requests) ──
//
// Two pins live here:
//   1. `vm_state_enumeration` — a destructure of EVERY `Vm` field with
//      no `..` rest. Adding a mutable field to `Vm` breaks this
//      function's compile, forcing the author to either reset it in
//      `reset_for_reuse` or extend the assertions below. This is the
//      compiler-enforced "canary on EVERY state class" the naryad
//      demands.
//   2. `reset_restores_every_state_class_to_fresh_load` — dirty every
//      cheaply-constructible state class, reset, and assert the VM is
//      observationally equivalent to a fresh `new()+load_program` VM.
//      The heavy registries (media/vision/reflex) are pinned
//      structurally by reset_for_reuse (`= X::new()`) — their element
//      types are not cheaply constructible — and behaviorally by the
//      HTTP-level tests in tests/naryad_403_vm_pool.rs.
#[cfg(test)]
mod n403_reset_tests {
    use super::*;
    use crate::interpreter::types::{
        ConvMessage, Conversation, DistillMode, DistillRuntimeState, Event, PatternStats,
    };
    use std::collections::HashMap;

    /// The exhaustive field enumeration (pin 1). Compile-only.
    #[allow(clippy::type_complexity)] // the complexity IS the pin: one tuple per Vm field
    fn vm_state_enumeration(
        vm: &Vm,
    ) -> (
        &std::collections::BTreeMap<String, crate::labels::Label>,
        &Vec<Value>,
        &Vec<Value>,
        &std::sync::Arc<Vec<String>>,
        &std::sync::Arc<Vec<CompiledFn>>,
        &Vec<(CompiledLearnableInfo, Vec<(String, String)>)>,
        &std::sync::Arc<Builtins>,
        &std::sync::Arc<Vec<String>>,
        &Vec<VmMemoryEntry>,
        &Vec<VmRelation>,
        &std::sync::Arc<Vec<CompiledRule>>,
        &std::sync::Arc<Vec<CompiledSkillIndex>>,
        &Option<rusqlite::Connection>,
        &Option<String>,
        &std::sync::Arc<Vec<String>>,
        &bool,
        &Vec<String>,
        &Mutex<Vec<String>>,
        &f64,
        &bool,
        &Option<Value>,
        &Option<std::collections::HashMap<String, String>>,
        &Option<std::collections::HashMap<String, String>>,
        &Vec<String>,
        &Mutex<HashMap<String, Conversation>>,
        &ConversationConfig,
        &Mutex<Vec<Event>>,
        &std::sync::atomic::AtomicU64,
        &Mutex<HashMap<String, PatternStats>>,
        &crate::nn::ReflexRegistry,
        &HashMap<String, crate::nn::ReflexId>,
        &Option<Value>,
        &std::sync::Arc<Vec<CompiledDenyHandler>>,
        &crate::vision::VisionRegistry,
        &crate::media::MediaStore,
        &HashMap<String, crate::bytecode::CompiledVisionDecl>,
        &HashMap<String, crate::bytecode::CompiledOriginDecl>,
        &Option<String>,
        &HashMap<String, crate::interpreter::types::DistillRuntimeState>,
    ) {
        let Vm {
            label_env,
            value_registers,
            globals,
            global_names,
            patterns,
            learnables,
            builtins,
            builtin_names,
            memory,
            relations,
            rules,
            skill_indices,
            db_conn,
            db_url,
            db_schema_ddl,
            db_open_failed,
            mutate_log,
            audit_log,
            propagated_confidence,
            collections_loaded,
            server_json_body,
            server_query_params,
            server_path_params,
            server_user_roles,
            conversations,
            conversation_config,
            event_log,
            event_next_id,
            pattern_stats,
            reflex_registry,
            reflex_names,
            current_deny_event,
            deny_handlers,
            vision_registry,
            media_store,
            vision_decls,
            origin_decls,
            memory_persist_path,
            distill_states,
        } = vm;
        // Note: `label_env` sits in a private type alias position; the
        // tuple returns references so nothing here runs — this function
        // exists to break the build when a field is added unhandled.
        let _ = (
            label_env,
            value_registers,
            globals,
            global_names,
            patterns,
            learnables,
            builtins,
            builtin_names,
            memory,
            relations,
            rules,
            skill_indices,
            db_conn,
            db_url,
            db_schema_ddl,
            db_open_failed,
            mutate_log,
            audit_log,
            propagated_confidence,
            collections_loaded,
            server_json_body,
            server_query_params,
            server_path_params,
            server_user_roles,
            conversations,
            conversation_config,
            event_log,
            event_next_id,
            pattern_stats,
            reflex_registry,
            reflex_names,
            current_deny_event,
            deny_handlers,
            vision_registry,
            media_store,
            vision_decls,
            origin_decls,
            memory_persist_path,
            distill_states,
        );
        (
            label_env,
            value_registers,
            globals,
            global_names,
            patterns,
            learnables,
            builtins,
            builtin_names,
            memory,
            relations,
            rules,
            skill_indices,
            db_conn,
            db_url,
            db_schema_ddl,
            db_open_failed,
            mutate_log,
            audit_log,
            propagated_confidence,
            collections_loaded,
            server_json_body,
            server_query_params,
            server_path_params,
            server_user_roles,
            conversations,
            conversation_config,
            event_log,
            event_next_id,
            pattern_stats,
            reflex_registry,
            reflex_names,
            current_deny_event,
            deny_handlers,
            vision_registry,
            media_store,
            vision_decls,
            origin_decls,
            memory_persist_path,
            distill_states,
        )
    }

    fn compiled(source: &str) -> Program {
        let decls = crate::parser::parse(source).expect("parse");
        crate::compiler::Compiler::new()
            .compile(decls)
            .expect("compile")
    }

    fn loaded_vm(source: &str) -> (Vm, Program) {
        let program = compiled(source);
        let mut vm = Vm::new();
        vm.load_program(&program).expect("load");
        (vm, program)
    }

    const N403_SOURCE: &str = r#"
db { url: "sqlite::memory:" }
pattern Ping(n: String) -> String { return n }
entity base: String = "7"
"#;

    #[test]
    fn field_enumeration_is_callable() {
        // The enumeration must stay live and callable: adding a field to
        // `Vm` without updating the destructure breaks the crate's
        // compilation — that break IS the canary mechanism.
        let (mut vm, _) = loaded_vm(N403_SOURCE);
        {
            let fields = vm_state_enumeration(&vm);
            assert_eq!(fields.2.len(), vm.globals.len(), "globals slot count");
            // №409: the db open is DEFERRED — load_program records the URL and
            // the shared DDL snapshot but must NOT open a connection.
            assert!(fields.12.is_none(), "№409: load must not open the db");
            assert_eq!(
                fields.13.as_deref(),
                Some("sqlite::memory:"),
                "declared URL recorded"
            );
            assert!(!fields.15, "no failed attempt recorded on a clean load");
            assert!(fields.0.is_empty(), "label env starts empty");
        }
        // The first access opens the connection (the lazy twin of the old
        // eager open) and applies the schema DDL.
        vm.ensure_db_open();
        assert!(vm.db_conn.is_some(), "first access must open the db");
    }

    #[test]
    fn reset_restores_every_state_class_to_fresh_load() {
        let (mut vm, program) = loaded_vm(N403_SOURCE);
        let (fresh, _) = loaded_vm(N403_SOURCE);
        let gslots = vm.globals.len();
        assert!(gslots > 0, "the fixture program must have globals");

        // ── dirty every cheaply-constructible state class ──
        vm.value_registers.push(Value::Float(9.0));
        vm.label_env
            .insert("leakvar".into(), crate::labels::Label::default());
        vm.globals[0] = Value::Float(12345.0);
        vm.memory.push(VmMemoryEntry {
            value: "leak-secret".into(),
            priority: 1.0,
            timestamp: 1,
            decay_rate: 0.0,
            mem_type: "episodic".into(),
        });
        vm.relations.push(VmRelation {
            from: "a".into(),
            to: "b".into(),
            relation: "leak".into(),
        });
        vm.mutate_log.push("[AUDIT] fake-mutate".into());
        *vm.audit_log.lock().unwrap() = vec!["fake-audit".to_string()];
        vm.propagated_confidence = 0.42;
        // №381 class: a live connection with CONTENT of its own.
        {
            let conn = rusqlite::Connection::open_in_memory().unwrap();
            conn.execute_batch("CREATE TABLE leak(t TEXT); INSERT INTO leak VALUES('x');")
                .unwrap();
            vm.db_conn = Some(conn);
        }
        vm.conversations.lock().unwrap().insert(
            "leak-conv".into(),
            Conversation {
                id: "leak-conv".into(),
                messages: vec![ConvMessage {
                    role: "user".into(),
                    text: "stale".into(),
                    timestamp: 0,
                }],
                created_at: 0,
                last_active: 0,
                metadata: HashMap::new(),
            },
        );
        vm.event_log.lock().unwrap().push(Event {
            id: 7,
            timestamp: 0,
            event_type: "leak".into(),
            source: "test".into(),
            data: HashMap::new(),
            duration_ms: None,
        });
        vm.event_next_id
            .store(99, std::sync::atomic::Ordering::SeqCst);
        vm.pattern_stats.lock().unwrap().insert(
            "leak".into(),
            PatternStats {
                calls: 5,
                confidence_sum: 5.0,
                cache_hits: 1,
                last_adapt: 0,
                last_call: 0,
                examples_count: 0,
            },
        );
        // №392 class: a stale deny event outside a handler is FORGED
        // state — the reset must remove it.
        vm.current_deny_event = Some(Value::String("stale-reason".into()));
        vm.distill_states.insert(
            "leak-distill".into(),
            DistillRuntimeState {
                mode: DistillMode::Teaching,
                examples: vec![("a".into(), "b".into())],
                last_train_attempt: 3,
            },
        );
        vm.reflex_names
            .insert("leak".into(), crate::nn::ReflexId(0));
        // server context (pub setters — the 402 isolation boundary)
        vm.set_server_json_body(Value::String("stale-body".into()));
        let mut q = std::collections::HashMap::new();
        q.insert("q".into(), "stale".into());
        vm.set_server_query_params(q);
        let mut p = std::collections::HashMap::new();
        p.insert("p".into(), "stale".into());
        vm.set_server_path_params(p);
        vm.set_server_user_roles(vec!["stale-role".into()]);

        // ── reset ──
        vm.reset_for_reuse(&program).expect("reset must succeed");

        // ── assert observational equivalence with a fresh load ──
        assert_eq!(
            format!("{:?}", vm.globals),
            format!("{:?}", fresh.globals),
            "globals slots must be rebuilt"
        );
        assert_eq!(vm.globals.len(), gslots);
        assert!(vm.label_env.is_empty(), "label_env must not survive");
        assert!(
            vm.value_registers.is_empty(),
            "value registers must not survive"
        );
        assert!(vm.memory.is_empty(), "memory store must not survive");
        assert!(vm.relations.is_empty(), "relations must not survive");
        assert!(vm.mutate_log.is_empty(), "mutate log must not survive");
        assert!(
            vm.audit_log.lock().unwrap().is_empty(),
            "audit log must not survive"
        );
        assert_eq!(
            vm.propagated_confidence, 1.0,
            "confidence must reset to 1.0"
        );
        // №381 canary: the reset leaves the VM CONNECTION-FREE (№409 lazy
        // contract) and the FIRST ACCESS afterwards opens a FRESH
        // connection — the leaked table from the dirty connection must be
        // gone from it.
        assert!(vm.db_conn.is_none(), "№409: reset must not leave a live db");
        assert!(!vm.db_open_failed, "№409: reset must clear the failed flag");
        vm.ensure_db_open();
        let leaked = {
            let conn = vm
                .db_conn
                .as_ref()
                .expect("first access after reset must re-open");
            conn.query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='leak'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(-1)
        };
        assert_eq!(
            leaked, 0,
            "reset must NEVER carry the previous connection's content"
        );
        assert!(
            vm.conversations.lock().unwrap().is_empty(),
            "conversations must not survive"
        );
        assert!(
            vm.event_log.lock().unwrap().is_empty(),
            "event log must not survive"
        );
        assert_eq!(
            vm.event_next_id.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "event id counter must reset"
        );
        assert!(
            vm.pattern_stats.lock().unwrap().is_empty(),
            "pattern stats must not survive"
        );
        assert!(
            vm.current_deny_event.is_none(),
            "a stale deny event outside a handler is forged state"
        );
        assert!(
            vm.distill_states.is_empty(),
            "distill states must not survive"
        );
        assert!(
            vm.reflex_names.is_empty(),
            "reflex name map must not accumulate"
        );
        assert!(
            vm.server_json_body.is_none(),
            "body context must not survive"
        );
        assert!(
            vm.server_query_params.is_none(),
            "query context must not survive"
        );
        assert!(
            vm.server_path_params.is_none(),
            "path context must not survive"
        );
        assert!(
            vm.server_user_roles.is_empty(),
            "roles context must not survive"
        );
        // Program-scoped reload parity with a fresh load.
        assert_eq!(vm.global_names, fresh.global_names);
        assert_eq!(
            vm.patterns.len(),
            fresh.patterns.len(),
            "pattern table must reload identically"
        );
        assert_eq!(vm.rules.len(), fresh.rules.len());
        assert_eq!(vm.skill_indices.len(), fresh.skill_indices.len());
        assert_eq!(vm.collections_loaded, fresh.collections_loaded);
        assert!(
            vm.learnables.is_empty(),
            "serve-path learnables must rest empty"
        );
    }

    #[test]
    fn reset_is_repeatable() {
        // Reset twice in a row (pool reuse across several generations):
        // the second reset must also succeed and leave fresh state.
        let (mut vm, program) = loaded_vm(N403_SOURCE);
        vm.reset_for_reuse(&program).expect("first reset");
        vm.globals[0] = Value::Float(777.0);
        vm.reset_for_reuse(&program).expect("second reset");
        let (fresh, _) = loaded_vm(N403_SOURCE);
        assert_eq!(format!("{:?}", vm.globals), format!("{:?}", fresh.globals));
    }
}

/// ── Naryad №409 (issue #554): the new VM-serve footprint invariants ──
///
/// Red→green + mutation verification (№382 protocol): each test below
/// names the invariant it pins; kicking the corresponding candidate out
/// (re-eagering the db open, un-sharing the builtin registry, keeping a
/// live connection across a pooled reset) MUST make the named test fall.
#[cfg(test)]
mod n409_tests {
    use super::*;

    fn compiled_with_routes(source: &str) -> (Program, Vec<crate::bytecode::CompiledRoute>) {
        let decls = crate::parser::parse(source).expect("parse");
        let server_cfg = decls
            .iter()
            .find_map(|d| match d {
                crate::ast::Declaration::MlogServer(s) => Some(s.clone()),
                _ => None,
            })
            .expect("mlogserver block");
        let mut comp = crate::compiler::Compiler::new();
        let program = comp.compile(decls).expect("compile");
        let routes = comp.compile_routes(&server_cfg.routes).expect("routes");
        (program, routes)
    }

    /// Program WITH a db declaration AND a schema (→ schema_ddl)
    /// AND one db-free route AND one db route.
    const N409_SOURCE: &str = r#"
db { url: "sqlite::memory:" }
schema n409_dept {
  table n409_probe {
    a: String
  }
}
pattern Echo409(n: String) -> String { return n }
mlogserver {
  port: 0
  route "/echo409" method=GET {
    respond("200", Echo409("pong"))
  }
  route "/put409" method=POST {
    db_insert("n409_probe", {a: "x"})
    respond("200", "stored")
  }
}
"#;

    #[test]
    fn n409_load_defers_db_open_until_first_access() {
        // INVARIANT (candidate C2): load_program records the declared URL
        // and the shared DDL snapshot but opens NO connection.
        let (program, _) = compiled_with_routes(N409_SOURCE);
        assert!(
            !program.schema_ddl.is_empty(),
            "fixture must carry schema DDL (the entity table)"
        );
        let mut vm = Vm::new();
        vm.load_program(&program).expect("load");
        assert!(vm.db_conn.is_none(), "load must NOT open the db");
        // First db access opens AND applies the schema DDL.
        vm.ensure_db_open();
        let conn = vm.db_conn.as_ref().expect("first access must open");
        let have = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='n409_probe'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(-1);
        assert_eq!(have, 1, "schema DDL must be applied on the lazy open");
    }

    #[test]
    fn n409_db_free_route_never_opens_the_db() {
        // INVARIANT (candidate C2): a request whose route body never
        // touches the db pays NO sqlite open + DDL — the connection rests
        // None through load + route execution. This is the measured
        // ~86 KB/request class the lazy open removes (ADR-0141 Add. 4).
        let (program, routes) = compiled_with_routes(N409_SOURCE);
        let mut vm = Vm::new();
        vm.load_program(&program).expect("load");
        let route = routes
            .iter()
            .find(|r| r.path == "/echo409")
            .expect("db-free route")
            .clone();
        let out = vm.execute_route_code(&route, &program);
        assert!(out.is_ok(), "db-free route must succeed: {:?}", out);
        assert!(
            vm.db_conn.is_none(),
            "a db-free request must not open a connection"
        );
        // Mutation pin: re-eagering the open inside load_program makes the
        // assertion above fail — that IS the red phase of this test.
    }

    #[test]
    fn n409_db_route_opens_and_serves_on_first_access() {
        // INVARIANT (candidate C2): a request that DOES touch the db gets
        // the same behavior as the eager path — open + DDL + write succeed.
        let (program, routes) = compiled_with_routes(N409_SOURCE);
        let mut vm = Vm::new();
        vm.load_program(&program).expect("load");
        let route = routes
            .iter()
            .find(|r| r.path == "/put409")
            .expect("db route")
            .clone();
        let out = vm.execute_route_code(&route, &program);
        assert!(out.is_ok(), "db route must succeed: {:?}", out);
        assert!(vm.db_conn.is_some(), "the db route must have opened");
    }

    #[test]
    fn n409_db_open_failure_fails_fast_with_legacy_message() {
        // INVARIANT (candidate C2): a failed connect is remembered for the
        // VM generation (fail fast, no silent retry storm) and the access
        // sites surface the SAME legacy message the eager path produced.
        let source = r#"
db { url: "sqlite:/nonexistent-dir-n409/probe.db" }
pattern Echo409(n: String) -> String { return n }
mlogserver {
  port: 0
  route "/read409" method=GET {
    query("SELECT 1", [])
    respond("200", "unreachable")
  }
}
"#;
        let (program, routes) = compiled_with_routes(source);
        let mut vm = Vm::new();
        vm.load_program(&program).expect("load");
        assert!(vm.db_conn.is_none());
        vm.ensure_db_open();
        assert!(vm.db_conn.is_none(), "a bad path must not open");
        assert!(vm.db_open_failed, "the failed attempt must be remembered");
        // No retry on the second access (fail fast).
        vm.ensure_db_open();
        assert!(vm.db_conn.is_none() && vm.db_open_failed);
        // The access site's error is the legacy message (via a real route
        // body — the query() builtin arm).
        let route = routes
            .iter()
            .find(|r| r.path == "/read409")
            .expect("read route")
            .clone();
        let err = vm
            .execute_route_code(&route, &program)
            .expect_err("access must fail with the legacy error");
        assert!(
            err.contains("no database connection"),
            "legacy message expected, got: {}",
            err
        );
    }

    #[test]
    fn n409_shared_registry_is_process_wide() {
        // INVARIANT (candidate C1): every Vm::new() shares ONE builtin
        // registry and ONE name table (Arc identity) — the per-request
        // ~70 KB rebuild is gone. Mutation pin: reverting Vm::new to a
        // fresh Builtins::new() per VM makes the ptr_eq assertions fail.
        let a = Vm::new();
        let b = Vm::new();
        assert!(
            std::sync::Arc::ptr_eq(&a.builtins, &b.builtins),
            "the builtin registry must be shared process-wide"
        );
        assert!(
            std::sync::Arc::ptr_eq(&a.builtin_names, &b.builtin_names),
            "the builtin name table must be shared process-wide"
        );
    }

    #[test]
    fn n409_pool_reset_restores_lazy_state_and_kills_bytes() {
        // INVARIANT (the new discard invariant, pool path): a pooled VM
        // whose request OPENED the db and wrote private content resets to
        // the CONNECTION-FREE lazy resting state — the next generation's
        // first access opens a FRESH db with the schema DDL and none of
        // the previous request's content (the №381 canary, lazy edition).
        let (program, _) = compiled_with_routes(N409_SOURCE);
        let program = std::sync::Arc::new(program);
        let mut vm = Vm::new();
        vm.load_program(&program).expect("load");
        vm.ensure_db_open();
        {
            let conn = vm.db_conn.as_ref().expect("opened for the request");
            conn.execute_batch(
                "CREATE TABLE req_private(x TEXT); INSERT INTO req_private VALUES('leak');",
            )
            .unwrap();
        }
        // The pool's checkin path: fail-closed reset.
        vm.reset_for_reuse(&program).expect("reset");
        assert!(vm.db_conn.is_none(), "reset must rest connection-free");
        assert!(!vm.db_open_failed, "reset must clear the failed flag");
        // Next generation: first access re-opens fresh — no request A bytes.
        vm.ensure_db_open();
        let conn = vm.db_conn.as_ref().expect("re-opened");
        let leaked = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='req_private'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(-1);
        assert_eq!(leaked, 0, "request A's content must never survive");
        let schema = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='n409_probe'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(-1);
        assert_eq!(schema, 1, "the shared DDL snapshot must re-apply");
    }
}

// ── №456 (gh#675): VM mirror of the distill holdout gate ───────────────
// Parity by construction: the same holdout-size and holdout-accuracy
// gates, the same audit wording, the same MIN_HOLDOUT constant as the
// TW path (interpreter/learnable.rs).

#[cfg(test)]
mod n456_vm_distill_holdout_tests {
    use super::*;
    use crate::nn::dense::Dense;

    fn make_vm_with_head() -> Vm {
        let mut vm = Vm::new();
        let dense = Dense::new(4, 2, crate::nn::ActivationKind::Softmax, 42);
        let model = crate::nn::ReflexModel {
            name: "TestHead".to_string(),
            layers: vec![Box::new(dense)],
            seed: 42,
            last_metric: None,
            input_size: 4,
            labels: vec!["yes".to_string(), "no".to_string()],
        };
        let id = vm.reflex_registry.register(model);
        vm.reflex_names.insert("TestHead".to_string(), id);
        vm
    }

    #[test]
    fn n456_vm_holdout_too_small_is_rejected() {
        let mut vm = make_vm_with_head();
        let examples: Vec<(String, String)> = (0..10)
            .map(|i| (format!("k{}", i), "yes".to_string()))
            .collect();
        let result = vm
            .try_train_distilled_model("P", "TestHead", 0.85, &examples)
            .expect("train must not error");
        assert!(
            !result,
            "holdout < MIN_HOLDOUT must NOT switch to DISTILLED"
        );
        let audit = vm.take_audit_log().join("\n");
        assert!(
            audit.contains("distill.rejected") && audit.contains("holdout too small"),
            "the holdout rejection must be loud: {}",
            audit
        );
    }

    #[test]
    fn n456_vm_noisy_labels_stay_teaching() {
        let mut vm = make_vm_with_head();
        let examples: Vec<(String, String)> = (0..24)
            .map(|i| {
                let label = if (i * 7 + 3) % 2 == 0 { "yes" } else { "no" };
                (format!("k{}", i), label.to_string())
            })
            .collect();
        let result = vm
            .try_train_distilled_model("P", "TestHead", 0.85, &examples)
            .expect("train must not error");
        assert!(!result, "noisy labels must NOT switch to DISTILLED");
        let audit = vm.take_audit_log().join("\n");
        assert!(
            audit.contains("distill.rejected") && audit.contains("holdout_accuracy"),
            "the accuracy rejection must be loud: {}",
            audit
        );
    }

    #[test]
    fn n456_vm_consistent_labels_pass_the_gate() {
        let mut vm = make_vm_with_head();
        let examples: Vec<(String, String)> = (0..24)
            .map(|i| (format!("k{}", i), "yes".to_string()))
            .collect();
        let result = vm
            .try_train_distilled_model("P", "TestHead", 0.85, &examples)
            .expect("train must not error");
        assert!(result, "consistent labels must pass the holdout gate");
    }
}

// №466: the live contract the shared memory module (src/memory_ops.rs)
// uses to reach the VM's simple-memory store — the replacing live
// contract for the RuntimeContext stub deleted by №465.
impl crate::memory_ops::VmMemoryAccess for Vm {
    fn mem(&self) -> &[VmMemoryEntry] {
        &self.memory
    }
    fn mem_mut(&mut self) -> &mut Vec<VmMemoryEntry> {
        &mut self.memory
    }
    fn relations(&self) -> &[VmRelation] {
        &self.relations
    }
}

// №466 group 2 (db): the live contract the shared db module
// (src/db_ops.rs) uses to reach the VM's lazily-opened connection and
// the per-request server query params — the same live-contract posture
// as VmMemoryAccess above (the RuntimeContext stub stays deleted).
impl crate::db_ops::VmDbAccess for Vm {
    fn ensure_db_open(&mut self) {
        Vm::ensure_db_open(self)
    }
    fn vm_db_conn(&mut self) -> &mut Option<rusqlite::Connection> {
        &mut self.db_conn
    }
    fn vm_server_query_params(&self) -> Option<&std::collections::HashMap<String, String>> {
        self.server_query_params.as_ref()
    }
}
