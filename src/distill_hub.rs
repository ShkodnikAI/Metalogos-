//! №495 (Wave 19, dispatch gh#793): the SERVER-LEVEL distillation hub.
//!
//! The audit 28.09 §3.1 (High, functional): distillation in `mlog serve`
//! never worked on EITHER backend — `fresh_program_context` builds a NEW
//! `Interpreter` per request and `clone_definitions_into` copies exactly
//! 12 definition fields, NOT the distill runtime state
//! (`distill_states`/`distill_in_flight`/the №489 training mailbox); the
//! VM pool resets `distill_states` on every check-in (`reset_for_reuse`,
//! №205). Each request started from an empty example list and destroyed
//! the single recorded example with its context — silently, no error, no
//! audit event: behavior indistinguishable from "not enough data yet".
//! The training threshold `max(distill_after, 10)` was unreachable in
//! serve by construction, and the declared reflex models did not even
//! survive the startup merge (`merge_interpreter`/`clone_definitions_into`
//! never carried the registry).
//!
//! The fix shape (the naryad): `ServerState { distill: Arc<dyn
//! DistillAccess> }` — ONE hub per process holding the examples, the
//! TEACHING/DISTILLED modes, the trained-model registry, the in-flight
//! flags and ONE background training thread (the №489 posture brought to
//! serve: the request never blocks on the 30-epoch run; the VM lane stops
//! training synchronously in-request when a hub is attached). Both
//! backends call through the trait (the `DbAccess` sample, №474/№484) —
//! the TW/VM distill mirrors (`learnable.rs` ↔ `vm.rs`) collapse into ONE
//! implementation here.
//!
//! Persistence: examples land in the SQLite table `distill_samples` (the
//! №166 plan shape — with the `source` provenance column) in the SAME
//! file the server already uses for memory persistence; weights keep
//! persisting through `reflex_save` (unchanged). Migration: existing
//! saved weights read as before; example state starts accumulating from
//! zero (honestly noted in the CHANGELOG).
//!
//! Boundaries (the naryad): the language syntax and the `distill_after`
//! semantics do NOT change; the VM pool and the lazy db open (№409) do
//! not degrade; the mock path stays network-free; without a hub (plain
//! `mlog run`) both backends keep their exact pre-№495 local behavior
//! (the existing unit tests are the pin).

use crate::ast::CompareOp;
use crate::interpreter::types::{DistillConfig, DistillMode, DistillRuntimeState};
use crate::interpreter::values::Value;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

/// The per-pattern spec the hub needs — normalized from the TW
/// `DistillConfig` and the VM `CompiledLearnableInfo` (the same five
/// fields both mirrors used; the two operator enums fold into
/// `ast::CompareOp`).
#[derive(Clone, Debug)]
pub struct DistillSpec {
    pub reflex_name: String,
    pub distill_after: usize,
    pub fallback_if: Option<(CompareOp, f64)>,
    pub min_accuracy: f64,
    pub margin: f64,
}

impl From<&DistillConfig> for DistillSpec {
    fn from(c: &DistillConfig) -> Self {
        DistillSpec {
            reflex_name: c.reflex_name.clone(),
            distill_after: c.distill_after,
            fallback_if: c.fallback_if,
            min_accuracy: c.min_accuracy,
            margin: c.margin,
        }
    }
}

impl From<&crate::bytecode::CompiledLearnableInfo> for DistillSpec {
    fn from(info: &crate::bytecode::CompiledLearnableInfo) -> Self {
        DistillSpec {
            reflex_name: info.distill_to.clone().unwrap_or_default(),
            distill_after: info.distill_after,
            fallback_if: info.fallback_if.map(|(op, t)| (CompareOp::from(op), t)),
            min_accuracy: info.distill_min_accuracy.unwrap_or(0.85),
            margin: info.distill_margin.unwrap_or(0.05),
        }
    }
}

impl From<crate::bytecode::ConditionOp> for CompareOp {
    fn from(op: crate::bytecode::ConditionOp) -> Self {
        match op {
            crate::bytecode::ConditionOp::Gt => CompareOp::Gt,
            crate::bytecode::ConditionOp::Lt => CompareOp::Lt,
            crate::bytecode::ConditionOp::Ge => CompareOp::Ge,
            crate::bytecode::ConditionOp::Le => CompareOp::Le,
            crate::bytecode::ConditionOp::Eq => CompareOp::Eq,
            crate::bytecode::ConditionOp::Ne => CompareOp::Ne,
        }
    }
}

// ── №840 (wave 71 — W-A, the R38(4) decision): the distillation
// firewall — SHADOW-first, non-blocking by construction ────────────────
//
// The lesson of №581 (R44): hardening without negative controls breaks
// legitimate paths. Before ANY blocking mode exists, the shadow contour
// must prove zero interference: the hub records every distill-path
// event (donor → recipient, the examples, the passport when one
// applies) into a bounded in-process ring and mirrors it to stderr —
// pure observation, no gate, no refusal, no behavior change. The
// blocking mode is EXPLICITLY OUT of this naryad: it switches on only
// by the owner's separate decision after the shadow data validates
// (the checkbox in the dispatch).

/// One shadow observation of the distill path. `kind` vocabulary:
/// `teach` (an example lands in the ledger), `teaching-fallback` (the
/// call falls through to the LLM), `distilled-answer` (the trained head
/// answered), `train-scheduled` / `retrain-scheduled`, `train-done` /
/// `train-failed` (the background verdict).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistillShadowEvent {
    /// Unix seconds (std only — the hub is tokio-free by contract).
    pub ts_unix: u64,
    pub kind: &'static str,
    /// The donor side of the path (the LLM teacher, or the pattern's
    /// example ledger for the train events).
    pub donor: String,
    /// The recipient side (the pattern's ledger, or the reflex head).
    pub recipient: String,
    /// The examples in play (the ledger size at the event).
    pub examples: usize,
    /// The free-form detail — the passport note when a registered model
    /// applies (№838), else the honest in-tree statement. The format
    /// makes NO claim on the ledger JSON schema (the naryad boundary).
    pub note: String,
}

/// The ring bound (bounded memory — the shadow never grows unbounded).
const SHADOW_RING: usize = 1024;

/// The passport note of a distill event: the №838 passports resolve by
/// weights id — a reflex head is in-tree, so the honest default is
/// `none`; the resolution slot exists for the day a distill path names
/// a registered model.
fn passport_note(name: &str) -> String {
    match crate::model_passport::passport_for(name) {
        Some(p) => format!(
            "passport={} ({} — {})",
            p.identity,
            p.license.as_str(),
            p.version
        ),
        None => "passport=none (the in-tree reflex weights, no external model)".to_string(),
    }
}

fn shadow_push(
    shared: &HubShared,
    kind: &'static str,
    donor: &str,
    recipient: &str,
    examples: usize,
    note: String,
) {
    let ts_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let event = DistillShadowEvent {
        ts_unix,
        kind,
        donor: donor.to_string(),
        recipient: recipient.to_string(),
        examples,
        note: note.clone(),
    };
    {
        let mut ring = shared.shadow.lock().unwrap_or_else(|e| e.into_inner());
        if ring.len() >= SHADOW_RING {
            ring.remove(0);
        }
        ring.push(event);
    }
    // The local reporting surface (the same stderr posture as the
    // [AUDIT]/[REDACT]/[BACKEND_SELECT] event lines — visible in every
    // serve console, plain text, no schema claims).
    eprintln!(
        "[DISTILL_SHADOW] kind={} donor='{}' recipient='{}' examples={} {}",
        kind, donor, recipient, examples, note
    );
}

/// №495: the trait both backends call — the `DbAccess` sample
/// (№474/№484). The implementation holds ALL distill state at the
/// server level; the trait object rides on the per-request interpreter
/// (`Interpreter::distill_hub`) and on every serving VM
/// (`Vm::distill_hub`). `None` = the pre-№495 local behavior (plain
/// `mlog run` — the existing unit tests pin it).
pub trait DistillAccess: Send + Sync {
    /// The distilled-call attempt (both modes inside). Contract identical
    /// to the mirrors it replaces:
    /// - `Ok(Some(value))` — DISTILLED mode answered with a confident
    ///   prediction; the caller returns it directly (no LLM call).
    /// - `Ok(None)` — TEACHING mode, or the confidence fell below
    ///   `fallback_if`; the caller falls through to the LLM and records
    ///   the example after.
    /// - `Err(e)` — reflex-side error (registry/model/predict); the
    ///   caller logs + falls through to the LLM (safe degradation,
    ///   ADR-0117 §3 — the error never reaches the program).
    fn try_distilled_call(
        &self,
        pattern_name: &str,
        spec: &DistillSpec,
        input: &str,
    ) -> Result<Option<Value>, String>;

    /// Record a post-LLM (input, output) example for future training
    /// (the same shape the mirrors recorded; unbounded, TEACHING-first —
    /// the outer call path records unconditionally, byte-identical).
    fn record_example(&self, pattern_name: &str, input: &str, output: &str);

    /// Test/observability snapshot: `(example_count, mode)` for one
    /// pattern. `None` = the hub never saw this pattern.
    fn snapshot(&self, pattern_name: &str) -> Option<(usize, DistillMode)>;

    /// №840: the local summary of the shadow distill paths (the plain
    /// text report — the serve console / the tool surface; the format
    /// makes NO claim on the ledger JSON schema). The default is the
    /// honest empty: only a hub that records shadow events answers.
    fn shadow_summary(&self) -> String {
        String::new()
    }
}

/// The state + registry + audit + persistence shared between the request
/// threads and the ONE background training thread.
struct HubShared {
    inner: Mutex<HubInner>,
    registry: Arc<Mutex<crate::nn::ReflexRegistry>>,
    names: HashMap<String, crate::nn::ReflexId>,
    /// The SHARED server audit log (`ServerState::audit_log`) — the
    /// background thread's verdict lands where the audit trail lives,
    /// never inside a one-shot per-request object (the audit 28.09 §3.1
    /// finding: the №489 mailbox died with the request). The hub is
    /// tokio-free (it compiles under --no-default-features where the
    /// server deps are absent), so the log is handed over as an erased
    /// push sink; the server side builds it with `blocking_write` — the
    /// legal lock form, every hub caller runs on a blocking thread (the
    /// route executors, the worker).
    audit: Arc<dyn Fn(String) + Send + Sync>,
    /// `distill_samples` persistence — `Some` only when the server has a
    /// memory-persist SQLite file (the same file, one additive table).
    persist: Option<Mutex<rusqlite::Connection>>,
    /// №840: the SHADOW ring — the distill-path observations (bounded,
    /// pure observation, never a gate).
    shadow: Mutex<Vec<DistillShadowEvent>>,
}

struct HubInner {
    states: HashMap<String, DistillRuntimeState>,
    in_flight: HashSet<String>,
}

/// The job the request thread hands to the background trainer (the
/// №489 queue choice: the examples ledger keeps accepting records while
/// a training runs; the next cadence cycle retrains on the fuller set).
struct TrainJob {
    pattern_name: String,
    spec: DistillSpec,
    examples: Vec<(String, String)>,
}

/// The hub — the process-level distillation state. Cloned as
/// `Arc<dyn DistillAccess>` into `ServerState`, every per-request
/// interpreter and every serving VM.
pub struct DistillHub {
    shared: Arc<HubShared>,
    tx: mpsc::Sender<TrainJob>,
}

impl DistillHub {
    /// Build the hub for one server process. The declared reflex models
    /// are registered by running the `reflex` declarations through the
    /// SAME declaration pass production uses (a dedicated throwaway
    /// interpreter — the startup merge never carried the registry, which
    /// was half of the audit's §3.1 finding); the hub's registry is that
    /// interpreter's Arc. `persist_path` = the memory-persist SQLite file
    /// (a hub without it works in-process and says so).
    pub fn open(
        audit: Arc<dyn Fn(String) + Send + Sync>,
        persist_path: Option<&str>,
        declarations: &[crate::ast::Declaration],
    ) -> Result<std::sync::Arc<DistillHub>, String> {
        // The declared reflex models: the same Declaration::Reflex arm the
        // top-level pass runs — no parallel model-building code.
        let mut refl = crate::interpreter::Interpreter::new();
        for decl in declarations {
            if matches!(decl, crate::ast::Declaration::Reflex(_)) {
                let _ = refl.run(vec![decl.clone()]);
            }
        }
        let registry = Arc::clone(&refl.reflex_registry);
        let names = refl.reflex_names.clone();

        let mut bootstrap_lines: Vec<String> = Vec::new();
        let persist = match persist_path {
            Some(path) => {
                // №500 named-allow equivalent: the hub opens ITS OWN
                // server-configured persistence file (the memory-persist
                // SQLite path handed in by `run_server`, not a program
                // data path) — the same service class as the vector.rs
                // post-sandbox open and the memory/KG journals.
                #[allow(clippy::disallowed_methods)]
                let conn = rusqlite::Connection::open(path)
                    .map_err(|e| format!("distill hub: open {}: {}", path, e))?;
                ensure_distill_store(&conn)?;
                Some(Mutex::new(conn))
            }
            None => None,
        };

        let shared = Arc::new(HubShared {
            inner: Mutex::new(HubInner {
                states: HashMap::new(),
                in_flight: HashSet::new(),
            }),
            registry,
            names,
            audit,
            persist,
            shadow: Mutex::new(Vec::new()),
        });

        // Load the persisted examples (honest migration: existing saved
        // weights read through reflex_save as before; the example ledger
        // starts from what distill_samples holds — zero on the first
        // 0.27.1 boot, then it survives restarts).
        if let Some(persist) = &shared.persist {
            let conn = persist.lock().unwrap_or_else(|e| e.into_inner());
            // №989 (ADR-0200 W4): the held-out invariant at the hub
            // boundary — the quarantined and the held-out rows NEVER
            // enter the training states; the exclusion is loud (the
            // bootstrap line names the count).
            let excluded: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM distill_samples WHERE quarantined = 1 OR source = 'held-out'",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            if excluded > 0 {
                bootstrap_lines.push(format!(
                    "[AUDIT] distill.held-out-excluded: {} row(s) kept out of training (quarantined or held-out — the W4 invariant)",
                    excluded
                ));
            }
            let mut stmt = conn
                .prepare(
                    "SELECT pattern_name, input, output, source FROM distill_samples \
                     WHERE quarantined IS NOT 1 AND source IS NOT 'held-out' ORDER BY rowid",
                )
                .map_err(|e| format!("distill hub: load distill_samples: {}", e))?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })
                .map_err(|e| format!("distill hub: query distill_samples: {}", e))?;
            let mut loaded = 0usize;
            for row in rows {
                let (pattern, input, output, _source) =
                    row.map_err(|e| format!("distill hub: read distill_samples row: {}", e))?;
                let mut inner = shared.inner.lock().unwrap_or_else(|e| e.into_inner());
                let state = inner
                    .states
                    .entry(pattern)
                    .or_insert_with(|| DistillRuntimeState {
                        mode: DistillMode::Teaching,
                        examples: Vec::new(),
                        last_train_attempt: 0,
                    });
                state.examples.push((input, output));
                loaded += 1;
            }
            // The loaded-count line ships to the worker thread (open()
            // may run on an async runtime thread — the audit lock is a
            // tokio RwLock and `blocking_write` would panic there; the
            // worker is a plain thread and pushes it first).
            if loaded > 0 {
                bootstrap_lines.push(
                    format!(
                        "[AUDIT] distill.samples-loaded: {} example(s) restored from distill_samples (naryad №495)",
                        loaded
                    ),
                );
            }
        }

        // ONE background training thread per process (the №489 posture,
        // now at the hub level): the request hands a snapshot over the
        // channel and never blocks on the 30-epoch run; the verdict lands
        // in the hub (mode flip + audit), not in a per-request mailbox.
        let (tx, rx) = mpsc::channel::<TrainJob>();
        let worker_shared = Arc::clone(&shared);
        std::thread::Builder::new()
            .name("distill-hub-trainer".to_string())
            .spawn(move || {
                for line in bootstrap_lines {
                    DistillHub::audit_line(&worker_shared, line);
                }
                while let Ok(job) = rx.recv() {
                    let outcome = DistillHub::train_one(&worker_shared, &job);
                    // The verdict + audit lines land in the hub regardless
                    // of which request is listening.
                    for line in &outcome.audit_lines {
                        DistillHub::audit_line(&worker_shared, line.clone());
                    }
                    let mut inner = worker_shared.inner.lock().unwrap_or_else(|e| e.into_inner());
                    match &outcome.error {
                        Some(e) => DistillHub::audit_line(
                            &worker_shared,
                            format!(
                                "[AUDIT] distill.training-finished: {} ERROR {} — staying TEACHING (naryad №495)",
                                job.pattern_name, e
                            ),
                        ),
                        None => DistillHub::audit_line(
                            &worker_shared,
                            format!(
                                "[AUDIT] distill.training-finished: {} switched={}",
                                job.pattern_name, outcome.switched
                            ),
                        ),
                    }
                    // №840: the shadow observation — the train verdict
                    // (the donor is the pattern's example ledger, the
                    // recipient is the reflex head).
                    {
                        let shared_ref: &HubShared = &worker_shared;
                        let (kind, note) = match &outcome.error {
                            Some(e) => (
                                "train-failed",
                                format!("error: {} — staying TEACHING; {}", e, passport_note(&job.spec.reflex_name)),
                            ),
                            None if outcome.switched => (
                                "train-done",
                                format!(
                                    "switched=true (the weights swapped in); {}",
                                    passport_note(&job.spec.reflex_name)
                                ),
                            ),
                            None => (
                                "train-done",
                                format!(
                                    "switched=false (the stay-put verdict); {}",
                                    passport_note(&job.spec.reflex_name)
                                ),
                            ),
                        };
                        shadow_push(
                            shared_ref,
                            kind,
                            &job.pattern_name,
                            &job.spec.reflex_name,
                            job.examples.len(),
                            note,
                        );
                    }
                    if outcome.switched {
                        if let Some(s) = inner.states.get_mut(&job.pattern_name) {
                            s.mode = DistillMode::Distilled;
                        }
                    } else if outcome.gate_rejected {
                        // №530 (issue #839): the retrain's holdout gate
                        // rejected the fresh snapshot — the data drifted
                        // under the live model. The LOUD return to
                        // TEACHING: the pattern re-accumulates LLM-backed
                        // examples and retrains later; silent degradation
                        // is impossible. A TEACHING-pattern rejection is
                        // the usual stay-put (no flip).
                        if let Some(s) = inner.states.get_mut(&job.pattern_name) {
                            if s.mode == DistillMode::Distilled {
                                s.mode = DistillMode::Teaching;
                                DistillHub::audit_line(
                                    &worker_shared,
                                    format!(
                                        "[AUDIT] distill.degraded: {} reverted to TEACHING — the retrain failed the holdout gate, the old weights no longer reflect the data (naryad №530)",
                                        job.pattern_name
                                    ),
                                );
                            }
                        }
                    }
                    inner.in_flight.remove(&job.pattern_name);
                }
            })
            .map_err(|e| format!("distill hub: spawn trainer: {}", e))?;

        Ok(Arc::new(DistillHub { shared, tx }))
        // №530: the concrete Arc keeps the raw-registry test hook reachable;
        // the server call sites coerce to Arc<dyn DistillAccess> implicitly.
    }

    fn audit_line(shared: &HubShared, line: String) {
        (shared.audit)(line);
    }

    /// The background trainer's body: the shared core
    /// (`run_distill_training`, the same holdout/margin gates) against
    /// the hub's registry — the trained weights persist in the shared
    /// registry the distill path predicts through.
    fn train_one(
        shared: &HubShared,
        job: &TrainJob,
    ) -> crate::interpreter::learnable::DistillTrainOutcome {
        // №990 (ADR-0200 W5, decision 6): the frozen gate BEFORE the
        // teacher gate — a Frozen pattern refuses distill moves even
        // with a licensed teacher; the bounded auto-unfreeze rides the
        // attempt (the lazy evaluation point). The audit closure bridges
        // the lifecycle trace into the shared journal.
        if let Some(conn) = shared.persist.as_ref() {
            let c = conn.lock().unwrap_or_else(|e| e.into_inner());
            let audit_ref = |l: String| DistillHub::audit_line(shared, l);
            if let Err(reason) =
                lifecycle_gate(&c, &job.pattern_name, distill_now_secs(), &audit_ref)
            {
                return crate::interpreter::learnable::DistillTrainOutcome {
                    pattern_name: job.pattern_name.clone(),
                    switched: false,
                    gate_rejected: false,
                    error: Some(reason),
                    audit_lines: Vec::new(),
                };
            }
        }
        // №975 (ADR-0200 W3b, decision 3): the DISTILL_TEACHER_UNLICENSED
        // rule at the hub boundary — the serve/hub lane refuses the same
        // way the TW and VM runtime boundaries do (one gate fn, three
        // sites, the №973 sink-parity form): the training run refuses
        // before it executes, naming the teacher and the class, audited
        // and ledger-recorded.
        if let Err(gate) = crate::interpreter::learnable::distill_teacher_gate() {
            DistillHub::audit_line(shared, format!("[AUDIT] distill.teacher-gate: {}", gate));
            crate::ledger::record(
                "distill.teacher-refused",
                "runtime",
                "reflex",
                &format!("pattern={};{}", job.pattern_name, gate),
            );
            // №990: the hysteresis feed — the violation enters the
            // pattern's ring; crossing the threshold inside the window
            // auto-freezes (the anti-flap band). A single violation
            // stays refusal-only.
            if let Some(conn) = shared.persist.as_ref() {
                let c = conn.lock().unwrap_or_else(|e| e.into_inner());
                let audit_ref = |l: String| DistillHub::audit_line(shared, l);
                if let Err(lifecycle_err) = record_violation(
                    &c,
                    &job.pattern_name,
                    "DISTILL_TEACHER_UNLICENSED",
                    distill_now_secs(),
                    &audit_ref,
                ) {
                    DistillHub::audit_line(
                        shared,
                        format!("[AUDIT] distill.lifecycle-error: {}", lifecycle_err),
                    );
                }
            }
            return crate::interpreter::learnable::DistillTrainOutcome {
                pattern_name: job.pattern_name.clone(),
                switched: false,
                gate_rejected: false,
                error: Some(gate),
                audit_lines: Vec::new(),
            };
        }
        let model_id = match shared.names.get(&job.spec.reflex_name).copied() {
            Some(id) => id,
            None => {
                return crate::interpreter::learnable::DistillTrainOutcome {
                    pattern_name: job.pattern_name.clone(),
                    switched: false,
                    gate_rejected: false,
                    error: Some(format!(
                        "distill: reflex '{}' not declared (no `reflex {} {{ ... }}` block)",
                        job.spec.reflex_name, job.spec.reflex_name
                    )),
                    audit_lines: Vec::new(),
                }
            }
        };
        let config = DistillConfig {
            reflex_name: job.spec.reflex_name.clone(),
            distill_after: job.spec.distill_after,
            fallback_if: job.spec.fallback_if,
            min_accuracy: job.spec.min_accuracy,
            margin: job.spec.margin,
            // The training core reads the gates, not the mode (the mode
            // lives on the runtime state it reports about).
            mode: DistillMode::Teaching,
        };
        crate::interpreter::learnable::run_distill_training(
            &shared.registry,
            model_id,
            &job.pattern_name,
            &config,
            &job.examples,
        )
    }
}

impl DistillHub {
    /// №530: test-support accessor — the raw reflex registry Arc for the
    /// out-of-lock/retrain tests (the weight-fingerprint assertions read
    /// the live model). #[doc(hidden)]: not part of the public API surface.
    #[doc(hidden)]
    pub fn raw_registry_for_tests(
        &self,
    ) -> std::sync::Arc<std::sync::Mutex<crate::nn::ReflexRegistry>> {
        std::sync::Arc::clone(&self.shared.registry)
    }

    /// №530: test-support accessor — the reflex handle resolution (the
    /// same name-to-id map the training core uses).
    #[doc(hidden)]
    pub fn reflex_id_for_tests(&self, name: &str) -> Option<crate::nn::ReflexId> {
        self.shared.names.get(name).copied()
    }

    /// №840: the shadow ring snapshot (the bounded observations — the
    /// test/tool surface).
    pub fn shadow_events(&self) -> Vec<DistillShadowEvent> {
        self.shared
            .shadow
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// №840: the plain-text summary of the distill paths (the local
    /// reporting surface — the serve console, the tools; no schema
    /// claims). Aggregated per donor → recipient pair, the kind counts
    /// beside.
    pub fn shadow_summary(&self) -> String {
        let events = self.shadow_events();
        if events.is_empty() {
            return "distill shadow: no events recorded (the ring is empty)".to_string();
        }
        let mut order: Vec<(String, String)> = Vec::new();
        let mut agg: HashMap<(String, String), HashMap<&'static str, usize>> = HashMap::new();
        for e in &events {
            let key = (e.donor.clone(), e.recipient.clone());
            if !agg.contains_key(&key) {
                order.push(key.clone());
            }
            *agg.entry(key).or_default().entry(e.kind).or_insert(0) += 1;
        }
        let mut lines = vec![format!(
            "distill shadow paths (the №840 ring, {} event(s), bounded at {}):",
            events.len(),
            SHADOW_RING
        )];
        for (donor, recipient) in &order {
            let counts = &agg[&(donor.clone(), recipient.clone())];
            let mut kinds: Vec<&'static str> = counts.keys().copied().collect();
            kinds.sort_unstable();
            let detail: Vec<String> = kinds
                .iter()
                .map(|k| format!("{}={}", k, counts[k]))
                .collect();
            lines.push(format!(
                "  '{}' -> '{}': {}",
                donor,
                recipient,
                detail.join(", ")
            ));
        }
        lines.join("\n")
    }
}

impl DistillAccess for DistillHub {
    fn try_distilled_call(
        &self,
        pattern_name: &str,
        spec: &DistillSpec,
        input: &str,
    ) -> Result<Option<Value>, String> {
        let (mode, count, last_attempt) = {
            let mut inner = self.shared.inner.lock().map_err(|e| {
                format!(
                    "distill hub: state lock poisoned while calling '{}': {}",
                    pattern_name, e
                )
            })?;
            let state = inner
                .states
                .entry(pattern_name.to_string())
                .or_insert_with(|| DistillRuntimeState {
                    mode: DistillMode::Teaching,
                    examples: Vec::new(),
                    last_train_attempt: 0,
                });
            (state.mode, state.examples.len(), state.last_train_attempt)
        };

        match mode {
            DistillMode::Teaching => {
                // The threshold contract, byte-identical to the mirrors:
                // ADR-0115 requires ≥10 examples for the holdout split, so
                // training is attempted at max(distill_after, 10), once per
                // crossing (retry after +5 more examples) — never on every
                // call.
                let training_threshold = std::cmp::max(spec.distill_after, 10);
                let should_attempt =
                    count >= training_threshold && (last_attempt == 0 || count - last_attempt >= 5);
                if should_attempt {
                    let in_flight = {
                        let mut inner =
                            self.shared.inner.lock().map_err(|e| {
                                format!("distill hub: in-flight lock poisoned: {}", e)
                            })?;
                        if inner.in_flight.contains(pattern_name) {
                            // A trainer is already running for this pattern —
                            // the ledger keeps accepting; this call stays on
                            // the LLM path (the №489 queue choice).
                            true
                        } else {
                            inner.in_flight.insert(pattern_name.to_string());
                            false
                        }
                    };
                    if !in_flight {
                        let examples = {
                            let inner = self.shared.inner.lock().unwrap_or_else(|e| e.into_inner());
                            inner
                                .states
                                .get(pattern_name)
                                .map(|s| s.examples.clone())
                                .unwrap_or_default()
                        };
                        Self::audit_line(
                            &self.shared,
                            format!(
                                "[AUDIT] distill.training-started: {} examples={} — the training runs in the hub's background thread, the call is not blocked (naryad №495)",
                                pattern_name,
                                examples.len()
                            ),
                        );
                        // №840: the shadow observation — the train
                        // scheduling on the teaching path.
                        shadow_push(
                            &self.shared,
                            "train-scheduled",
                            pattern_name,
                            &spec.reflex_name,
                            examples.len(),
                            passport_note(&spec.reflex_name),
                        );
                        // The bookkeeping order the mirrors pin: mark the
                        // attempt BEFORE handing the job over (a failed send
                        // still counts as an attempt — the retry cadence
                        // keeps the request path O(1)).
                        {
                            let mut inner =
                                self.shared.inner.lock().unwrap_or_else(|e| e.into_inner());
                            if let Some(s) = inner.states.get_mut(pattern_name) {
                                s.last_train_attempt = count;
                            }
                        }
                        let job = TrainJob {
                            pattern_name: pattern_name.to_string(),
                            spec: spec.clone(),
                            examples,
                        };
                        if self.tx.send(job).is_err() {
                            // The trainer thread is gone (process shutdown
                            // path) — loud, and the pattern stays TEACHING.
                            Self::audit_line(
                                &self.shared,
                                format!(
                                    "[AUDIT] distill.training-dropped: {} — the hub trainer thread is not accepting jobs",
                                    pattern_name
                                ),
                            );
                            let mut inner =
                                self.shared.inner.lock().unwrap_or_else(|e| e.into_inner());
                            inner.in_flight.remove(pattern_name);
                        }
                    }
                }
                // Not enough examples yet (or first call) — the outer call
                // path invokes the LLM and records the example after.
                // №840: the shadow observation — the teaching fallback
                // (the LLM carries this call).
                shadow_push(
                    &self.shared,
                    "teaching-fallback",
                    pattern_name,
                    &spec.reflex_name,
                    count,
                    passport_note(&spec.reflex_name),
                );
                Ok(None)
            }
            DistillMode::Distilled => {
                // №530 (issue #839): the RETRAIN trigger. The buffer keeps
                // growing in DISTILLED mode too — every LOW-CONFIDENCE
                // call falls through to the LLM and records the fresh
                // (input, llm_output) ground truth. When the new examples
                // since the last training cross the SAME threshold that
                // built the model (the honest default: the pattern's own
                // `distill_after`, the ADR-0115 floor of 10 applies), a
                // retrain job is scheduled on the single trainer thread.
                // The mode stays DISTILLED while it runs (the old weights
                // keep answering); the outcome lands in the trainer thread:
                // gate passed → the weights swap in; gate rejected → the
                // loud TEACHING revert (the degradation branch).
                let retrain_threshold = std::cmp::max(spec.distill_after, 10);
                let new_since_train = count.saturating_sub(last_attempt);
                if new_since_train >= retrain_threshold {
                    let in_flight = {
                        let mut inner =
                            self.shared.inner.lock().map_err(|e| {
                                format!("distill hub: in-flight lock poisoned: {}", e)
                            })?;
                        if inner.in_flight.contains(pattern_name) {
                            true
                        } else {
                            inner.in_flight.insert(pattern_name.to_string());
                            false
                        }
                    };
                    if !in_flight {
                        let examples = {
                            let inner = self.shared.inner.lock().unwrap_or_else(|e| e.into_inner());
                            inner
                                .states
                                .get(pattern_name)
                                .map(|s| s.examples.clone())
                                .unwrap_or_default()
                        };
                        Self::audit_line(
                            &self.shared,
                            format!(
                                "[AUDIT] distill.retraining-started: {} new_examples={} buffer={} — the retrain runs in the hub's background thread (naryad №530)",
                                pattern_name,
                                new_since_train,
                                examples.len()
                            ),
                        );
                        // №840: the shadow observation — the retrain
                        // scheduling on the distilled path.
                        shadow_push(
                            &self.shared,
                            "retrain-scheduled",
                            pattern_name,
                            &spec.reflex_name,
                            examples.len(),
                            passport_note(&spec.reflex_name),
                        );
                        {
                            let mut inner =
                                self.shared.inner.lock().unwrap_or_else(|e| e.into_inner());
                            if let Some(s) = inner.states.get_mut(pattern_name) {
                                s.last_train_attempt = count;
                            }
                        }
                        let job = TrainJob {
                            pattern_name: pattern_name.to_string(),
                            spec: spec.clone(),
                            examples,
                        };
                        if self.tx.send(job).is_err() {
                            Self::audit_line(
                                &self.shared,
                                format!(
                                    "[AUDIT] distill.training-dropped: {} — the hub trainer thread is not accepting jobs",
                                    pattern_name
                                ),
                            );
                            let mut inner =
                                self.shared.inner.lock().unwrap_or_else(|e| e.into_inner());
                            inner.in_flight.remove(pattern_name);
                        }
                    }
                }
                // Predict through the hub's registry (the trained weights
                // live HERE, not in a per-request copy). Safe degradation:
                // any error → None → the outer path falls through to LLM.
                let model_id = self
                    .shared
                    .names
                    .get(&spec.reflex_name)
                    .copied()
                    .ok_or_else(|| {
                        format!(
                            "distill: reflex '{}' not declared (no `reflex {} {{ ... }}` block)",
                            spec.reflex_name, spec.reflex_name
                        )
                    })?;
                let reg = self
                    .shared
                    .registry
                    .lock()
                    .map_err(|e| format!("reflex registry poisoned: {}", e))?;
                let model_kind = reg.get(model_id).ok_or_else(|| {
                    format!("distill: model handle {:?} not in registry", model_id)
                })?;
                let (probs, labels): (Vec<f64>, Vec<String>) = match model_kind {
                    crate::nn::ModelKind::Dense(model) => {
                        // №504: a model whose weights were trained on
                        // DIFFERENT features (a pre-№504 reflex_load) must
                        // never answer as a confident one — refuse loudly
                        // and stay on the LLM path until the retrain.
                        if model.feature_signature != crate::embeddings::DISTILL_FEATURE_SIGNATURE {
                            Self::audit_line(
                                    &self.shared,
                                    format!(
                                        "[AUDIT] distill.feature-stale: {} — weights trained on '{}' but this build extracts '{}' (naryad №504): FEATURES CHANGED, RETRAIN THE MODEL — staying on the LLM path",
                                        pattern_name,
                                        model.feature_signature,
                                        crate::embeddings::DISTILL_FEATURE_SIGNATURE
                                    ),
                                );
                            return Ok(None);
                        }
                        let embedding =
                            crate::embeddings::hashed_tfidf_vector(input, model.input_size);
                        (model.forward(&embedding), model.labels.clone())
                    }
                    #[cfg(feature = "candle")]
                    crate::nn::ModelKind::Sequence(_) => {
                        return Err(
                            "distill: sequence models (reflex_seq) do not yet support distill_to. \
                             distill_to currently works only with Dense models (reflex). \
                             Sequence distillation is a future-naryad concern."
                                .to_string(),
                        )
                    }
                    #[cfg(feature = "candle")]
                    crate::nn::ModelKind::Gen(_) => {
                        return Err(
                            "distill: gen models (reflex_gen) do not support distill_to. \
                             distill_to works only with Dense models (reflex)."
                                .to_string(),
                        )
                    }
                };

                // The highest-confidence label (the mirror's exact form).
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
                // model must never answer as a confident one.
                if !best_prob.is_finite() {
                    return Ok(None);
                }
                // №456: the barrier defaults to `confidence < 0.7` — a
                // pattern with `distill_to` but no explicit `fallback_if`
                // is no longer barrier-free.
                let (op, threshold) = spec.fallback_if.unwrap_or((CompareOp::Lt, 0.7));
                if op.compare(best_prob, threshold) {
                    // №840: the shadow observation — the confident answer
                    // refused by the barrier; the LLM path follows.
                    let count = {
                        let inner = self.shared.inner.lock().unwrap_or_else(|e| e.into_inner());
                        inner
                            .states
                            .get(pattern_name)
                            .map(|s| s.examples.len())
                            .unwrap_or(0)
                    };
                    shadow_push(
                        &self.shared,
                        "teaching-fallback",
                        pattern_name,
                        &spec.reflex_name,
                        count,
                        format!(
                            "confidence {:.3} failed the barrier ({:?} {:?}) — the LLM path follows; {}",
                            best_prob, op, threshold,
                            passport_note(&spec.reflex_name)
                        ),
                    );
                    return Ok(None);
                }
                // №840: the shadow observation — the trained head answered
                // (the distilled path REALLY carried the call).
                {
                    let count = {
                        let inner = self.shared.inner.lock().unwrap_or_else(|e| e.into_inner());
                        inner
                            .states
                            .get(pattern_name)
                            .map(|s| s.examples.len())
                            .unwrap_or(0)
                    };
                    shadow_push(
                        &self.shared,
                        "distilled-answer",
                        pattern_name,
                        &spec.reflex_name,
                        count,
                        format!(
                            "confidence {:.3} passed the barrier — no LLM call; {}",
                            best_prob,
                            passport_note(&spec.reflex_name)
                        ),
                    );
                }
                Ok(Some(Value::String(best_label)))
            }
        }
    }

    fn shadow_summary(&self) -> String {
        DistillHub::shadow_summary(self)
    }

    fn record_example(&self, pattern_name: &str, input: &str, output: &str) {
        let mut inner = self.shared.inner.lock().unwrap_or_else(|e| e.into_inner());
        let state = inner
            .states
            .entry(pattern_name.to_string())
            .or_insert_with(|| DistillRuntimeState {
                mode: DistillMode::Teaching,
                examples: Vec::new(),
                last_train_attempt: 0,
            });
        state.examples.push((input.to_string(), output.to_string()));
        // №840: the shadow observation — the teach event (pure
        // observation, after the ledger append; zero behavior weight).
        let examples = state.examples.len();
        shadow_push(
            &self.shared,
            "teach",
            "llm-teacher (the outer call path)",
            pattern_name,
            examples,
            passport_note(pattern_name),
        );
        // Persist the example (the №166 plan shape with the `source`
        // provenance column). A failed INSERT is loud in the audit trail
        // but never fails the request: the in-process ledger already
        // holds the example.
        if let Some(persist) = &self.shared.persist {
            let conn = persist.lock().unwrap_or_else(|e| e.into_inner());
            let res = conn.execute(
                "INSERT INTO distill_samples (pattern_name, input, output, source, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    pattern_name,
                    input,
                    output,
                    "llm-response",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0)
                ],
            );
            if let Err(e) = res {
                drop(conn);
                DistillHub::audit_line(
                    &self.shared,
                    format!(
                        "[AUDIT] distill.persist-error: {} — {} (the example stays in the in-process ledger)",
                        pattern_name, e
                    ),
                );
            }
        }
    }

    fn snapshot(&self, pattern_name: &str) -> Option<(usize, DistillMode)> {
        let inner = self.shared.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner
            .states
            .get(pattern_name)
            .map(|s| (s.examples.len(), s.mode))
    }
}

/// №504 (gh#787): the canonical distillation feature extractor — the
/// hashed TF-IDF form of the TF-IDF fallback (`EmbeddingBackend`),
/// OFFLINE, deterministic, network-free. The former byte-position
/// `simple_embedding` ("sum of bytes over i % dim") was NOT a semantic
/// representation — "Where is my package?" and "Where's my parcel?"
/// produced near-unrelated vectors, so the №485 gate would honestly
/// reject most real-text models (the audit 28.09 §3.1 companion
/// finding). The ONE canonical copy lives in
/// `crate::embeddings::hashed_tfidf_vector` (next to the
/// `EmbeddingBackend` trait it implements); BOTH backends' distill
/// paths (TW learnable.rs, VM vm.rs, this hub) call it directly —
/// byte-identical by construction, and the vm.rs mirror count (the
/// №502 metric) moves DOWN as planned by the №502 baseline.
pub(crate) fn distill_features(input: &str, dim: usize) -> Vec<f64> {
    crate::embeddings::hashed_tfidf_vector(input, dim)
}

// ── №989 (ADR-0200 W4): the curriculum build — propose-and-refute ────
//
// The W4 slot of the provable tier-lowering line. The quarantine is the
// three columns on `distill_samples` (reviewed / reason / quarantined —
// ADR-0200 §5 verbatim: NO new table); the held-out invariant — the
// rows the teacher did not generate never enter training — is enforced
// at EVERY training-set assembly point (the hub boot load above, the
// attestation manifest in nn::attest, the filter word) and the proven
// crossing is the loud refusal below.

/// The `source` vocabulary row a held-out load writes. The teacher
/// provenance column keeps `"llm-response"` (№495 unchanged); `"held-out"`
/// is the new word marking the rows the teacher did NOT generate — the
/// refutation instrument, never training data (the Never-cut invariant).
pub const DISTILL_SOURCE_HELD_OUT: &str = "held-out";

/// The stable machine id of a quarantined row (the №983 machine-id
/// form): `dq-<rowid>-<hash8>` — the rowid anchors the physical row,
/// the hash binds the (pattern, input, output) triple.
fn quarantine_machine_id(rowid: i64, pattern: &str, input: &str, output: &str) -> String {
    let h =
        crate::ledger::sha256_hex(format!("{}\u{1f}{}\u{1f}{}", pattern, input, output).as_bytes());
    format!("dq-{}-{}", rowid, &h[..8])
}

fn input_hash8(input: &str) -> String {
    let h = crate::ledger::sha256_hex(input.as_bytes());
    h[..8].to_string()
}

/// The curriculum-build verdict — the machine-readable gate facts (the
/// gate-facts-sync form: the CLI prints it as one JSON line, the audit
/// journal carries the same bytes).
#[derive(Debug, Clone)]
pub struct CurriculumVerdict {
    pub pattern: String,
    /// The teacher-generated candidate units the build examined.
    pub proposed: usize,
    /// The units that SURVIVE refutation — the curriculum.
    pub survived: usize,
    /// The units the refutation quarantined (quarantined=1, the reason,
    /// reviewed=0 — the review workflow is the W5 states lane).
    pub quarantined: usize,
    /// The held-out rows the refutation consulted (never trained on).
    pub held_out: usize,
    /// The recomputed filter word ("curriculum:<sha16>" or "none") — the
    /// same pure function the certificate's `dataset.filter` writes.
    pub filter_word: String,
    /// The quarantined rows' machine ids, in rowid order.
    pub quarantine_ids: Vec<String>,
}

impl CurriculumVerdict {
    /// The one-line machine-readable verdict.
    pub fn gate_line(&self) -> String {
        format!(
            "{{\"gate\":\"curriculum-build\",\"ok\":true,\"pattern\":\"{}\",\"proposed\":{},\"survived\":{},\"quarantined\":{},\"held_out\":{},\"filter\":\"{}\"}}",
            self.pattern,
            self.proposed,
            self.survived,
            self.quarantined,
            self.held_out,
            self.filter_word
        )
    }
}

/// The store schema/migration: the FULL W4 shape on a fresh file, the
/// three quarantine columns ALTERed onto a pre-W4 file (the honest
/// migration — an existing database keeps its rows byte-for-byte). No
/// new table: the columns land on `distill_samples` (ADR-0200 §5 W4).
pub fn ensure_distill_store(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS distill_samples (
            pattern_name TEXT NOT NULL,
            input        TEXT NOT NULL,
            output       TEXT NOT NULL,
            source       TEXT NOT NULL,
            created_at   INTEGER NOT NULL,
            reviewed     INTEGER NOT NULL DEFAULT 0,
            reason       TEXT,
            quarantined  INTEGER NOT NULL DEFAULT 0
        );",
    )
    .map_err(|e| format!("distill store: create distill_samples: {}", e))?;
    let mut have: HashSet<String> = HashSet::new();
    {
        let mut stmt = conn
            .prepare("PRAGMA table_info(distill_samples)")
            .map_err(|e| format!("distill store: pragma: {}", e))?;
        let names = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| format!("distill store: pragma: {}", e))?;
        for name in names.filter_map(|n| n.ok()) {
            have.insert(name);
        }
    }
    for (col, ddl) in [
        (
            "reviewed",
            "ALTER TABLE distill_samples ADD COLUMN reviewed INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "reason",
            "ALTER TABLE distill_samples ADD COLUMN reason TEXT",
        ),
        (
            "quarantined",
            "ALTER TABLE distill_samples ADD COLUMN quarantined INTEGER NOT NULL DEFAULT 0",
        ),
    ] {
        if !have.contains(col) {
            conn.execute_batch(ddl)
                .map_err(|e| format!("distill store: migrate {}: {}", col, e))?;
        }
    }
    // №990 (ADR-0200 W5, decision 6): the per-pattern lifecycle table —
    // Active/Frozen with the transition metadata. Additive CREATE IF NOT
    // EXISTS: a pre-W4/pre-W5 database gains the table on the next
    // ensure call and keeps every existing row byte-for-byte (the same
    // honest-migration form as the W4 columns above; the lifecycle is a
    // DIFFERENT concern than the sample quarantine — its own table, the
    // W4 "no new table" rule was scoped to the quarantine columns).
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS distill_states (
            pattern             TEXT PRIMARY KEY,
            phase               TEXT NOT NULL,
            entered_at          INTEGER NOT NULL,
            violations          TEXT NOT NULL DEFAULT '',
            last_violation_code TEXT,
            auto_unfroze_at     INTEGER,
            refroze_in_window   INTEGER NOT NULL DEFAULT 0,
            operator_lock       INTEGER NOT NULL DEFAULT 0
        );",
    )
    .map_err(|e| format!("distill store: create distill_states: {}", e))?;
    Ok(())
}

// ===========================================================================
// №990 (ADR-0200 W5, decision 6): the per-pattern lifecycle machine.
// Active / Frozen, the hysteresis (the anti-flap violation ring), the
// 24h-bounded auto-unfreeze (decision 6 verbatim: the auto-exit no more
// than once per the cooldown, every auto-exit ledger-records the reason
// and the metrics, a re-freeze within the window stays frozen until the
// explicit external command). The durable home is the `distill_states`
// table in the persist DB (the hub lane) — VMs are pooled and per-request
// state resets, so the lifecycle NEVER lives in a per-request container.
// The state is a safety interlock over the distill lane: a Frozen pattern
// refuses distill MOVES (training/teaching/curriculum build) with the
// machine reason `[DISTILL_FROZEN]`; serving continues.
// ===========================================================================

/// The lifecycle phase of a distill pattern (№990).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistillPhase {
    /// Normal operation.
    Active,
    /// Distill moves refuse with `[DISTILL_FROZEN]`; serving continues.
    Frozen,
}

impl DistillPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            DistillPhase::Active => "active",
            DistillPhase::Frozen => "frozen",
        }
    }

    fn parse(s: &str) -> DistillPhase {
        match s {
            "frozen" => DistillPhase::Frozen,
            _ => DistillPhase::Active,
        }
    }
}

/// The per-pattern lifecycle state (№990) — one `distill_states` row.
#[derive(Debug, Clone)]
pub struct DistillLifecycle {
    pub pattern: String,
    pub phase: DistillPhase,
    /// When the CURRENT phase was entered (epoch seconds).
    pub entered_at: u64,
    /// The hysteresis ring: the violation timestamps inside the freeze
    /// window (epoch seconds, ascending; trimmed to the window on write).
    pub violations: Vec<u64>,
    /// The code of the last violation (e.g. `DISTILL_TEACHER_UNLICENSED`).
    pub last_violation_code: Option<String>,
    /// The last bounded auto-unfreeze — the 24h clock anchor (decision 6).
    pub auto_unfroze_at: Option<u64>,
    /// Re-frozen after an auto-unfreeze within the cooldown window: no
    /// further auto-unfreeze until the explicit external command.
    pub refroze_in_window: bool,
    /// An OPERATOR freeze (`mlog distill freeze`) — the bounded
    /// auto-exit does not release it: the operator's authority anchor
    /// (the recovery checks are machine-true, the operator's reason is
    /// not); only `mlog distill unfreeze` lifts it. This is the W5-fixed
    /// parameter surface (decision 6's own mandate: the concrete
    /// recovery conditions and the parameter surface are fixed in W5).
    pub operator_lock: bool,
}

impl DistillLifecycle {
    fn fresh(pattern: &str) -> DistillLifecycle {
        DistillLifecycle {
            pattern: pattern.to_string(),
            phase: DistillPhase::Active,
            entered_at: 0,
            violations: Vec::new(),
            last_violation_code: None,
            auto_unfroze_at: None,
            refroze_in_window: false,
            operator_lock: false,
        }
    }
}

/// The lifecycle verdict — the machine-readable gate facts (the
/// gate-facts-sync form: the CLI prints it as one JSON line, the audit
/// journal carries the same bytes).
#[derive(Debug, Clone)]
pub struct LifecycleVerdict {
    pub pattern: String,
    /// freeze | unfreeze | auto-unfreeze | violation | status | refused
    pub op: &'static str,
    pub ok: bool,
    pub phase: DistillPhase,
    pub changed: bool,
    pub reason: String,
    /// The recovery checks (the name, the result) — the recovery
    /// conditions this naryad fixes (decision 6's W5 mandate).
    pub recovery: Vec<(&'static str, bool)>,
    /// Whether the bounded auto-unfreeze is currently allowed (the 24h
    /// clock + the re-freeze lock).
    pub auto_unfreeze_allowed: bool,
}

impl LifecycleVerdict {
    pub fn gate_line(&self) -> String {
        let recovery: Vec<String> = self
            .recovery
            .iter()
            .map(|(k, v)| format!("\"{}\":{}", k, v))
            .collect();
        serde_json::json!({
            "gate": format!("distill-{}", self.op),
            "ok": self.ok,
            "pattern": self.pattern,
            "phase": self.phase.as_str(),
            "changed": self.changed,
            "reason": self.reason,
            "recovery": recovery.join(","),
            "auto_unfreeze_allowed": self.auto_unfreeze_allowed,
        })
        .to_string()
    }
}

/// Epoch seconds (the callers pass their own clock where purity matters;
/// the hub and CLI call sites use this).
pub fn distill_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// The hysteresis parameters (№990 fixes the surface; the defaults are
/// the recorded values — env-tunable, read at call time).
fn distill_freeze_threshold() -> u32 {
    std::env::var("METALOGOS_DISTILL_FREEZE_THRESHOLD")
        .ok()
        .and_then(|v| v.trim().parse::<u32>().ok())
        .filter(|v| *v >= 1)
        .unwrap_or(2)
}

fn distill_freeze_window_secs() -> u64 {
    std::env::var("METALOGOS_DISTILL_FREEZE_WINDOW_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(3600)
}

/// Decision 6's bound: the auto-unfreeze no more than once per 24 hours
/// (the default parameter — 86400s).
fn distill_auto_unfreeze_cooldown_secs() -> u64 {
    std::env::var("METALOGOS_DISTILL_AUTO_UNFREEZE_COOLDOWN_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(86400)
}

/// `--trace-distill`: the trace switch (the CLI flag sets the env var at
/// startup — one SSOT; the runtime reads it here).
pub fn distill_trace_enabled() -> bool {
    std::env::var("METALOGOS_TRACE_DISTILL").as_deref() == Ok("1")
}

/// The `--trace-distill` line — the same bytes to stderr and the audit
/// journal (the gate-line form: one event, both sinks, identical text).
pub fn trace_distill(audit: &dyn Fn(String), line: &str) {
    if !distill_trace_enabled() {
        return;
    }
    let l = format!("[TRACE] {}", line);
    eprintln!("{}", l);
    audit(l);
}

/// Read the pattern's lifecycle row; `None` for a pattern without W5
/// state (a pre-W5 database — the byte-compat no-op form).
pub fn read_lifecycle(
    conn: &rusqlite::Connection,
    pattern: &str,
) -> Result<Option<DistillLifecycle>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT phase, entered_at, violations, last_violation_code, \
             auto_unfroze_at, refroze_in_window, operator_lock FROM distill_states \
             WHERE pattern = ?1",
        )
        .map_err(|e| format!("distill states: read {}: {}", pattern, e))?;
    let mut rows = stmt
        .query([pattern])
        .map_err(|e| format!("distill states: read {}: {}", pattern, e))?;
    let Some(row) = rows
        .next()
        .map_err(|e| format!("distill states: read {}: {}", pattern, e))?
    else {
        return Ok(None);
    };
    let phase: String = row
        .get(0)
        .map_err(|e| format!("distill states: read {}: {}", pattern, e))?;
    let entered_at: i64 = row
        .get(1)
        .map_err(|e| format!("distill states: read {}: {}", pattern, e))?;
    let violations: String = row
        .get(2)
        .map_err(|e| format!("distill states: read {}: {}", pattern, e))?;
    let code: Option<String> = row
        .get(3)
        .map_err(|e| format!("distill states: read {}: {}", pattern, e))?;
    let auto: Option<i64> = row
        .get(4)
        .map_err(|e| format!("distill states: read {}: {}", pattern, e))?;
    let refroze: i64 = row
        .get(5)
        .map_err(|e| format!("distill states: read {}: {}", pattern, e))?;
    let op_lock: i64 = row
        .get(6)
        .map_err(|e| format!("distill states: read {}: {}", pattern, e))?;
    Ok(Some(DistillLifecycle {
        pattern: pattern.to_string(),
        phase: DistillPhase::parse(&phase),
        entered_at: entered_at.max(0) as u64,
        violations: violations
            .split(',')
            .filter_map(|s| s.trim().parse::<u64>().ok())
            .collect(),
        last_violation_code: code,
        auto_unfroze_at: auto.filter(|t| *t >= 0).map(|t| t as u64),
        refroze_in_window: refroze != 0,
        operator_lock: op_lock != 0,
    }))
}

fn write_lifecycle(conn: &rusqlite::Connection, lc: &DistillLifecycle) -> Result<(), String> {
    let violations = lc
        .violations
        .iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>()
        .join(",");
    conn.execute(
        "INSERT OR REPLACE INTO distill_states \
         (pattern, phase, entered_at, violations, last_violation_code, \
          auto_unfroze_at, refroze_in_window, operator_lock) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            lc.pattern,
            lc.phase.as_str(),
            lc.entered_at as i64,
            violations,
            lc.last_violation_code,
            lc.auto_unfroze_at.map(|t| t as i64),
            lc.refroze_in_window as i64,
            lc.operator_lock as i64,
        ],
    )
    .map_err(|e| format!("distill states: write {}: {}", lc.pattern, e))?;
    Ok(())
}

/// The store counts for the pattern (the auto-unfreeze ledger record's
/// metrics — the transition trains nothing, so the counts do not move;
/// the record carries them once, with the check results).
fn lifecycle_store_counts(conn: &rusqlite::Connection, pattern: &str) -> (usize, usize, usize) {
    let q = |source: &str, excluded: i64| -> usize {
        conn.query_row(
            "SELECT COUNT(*) FROM distill_samples WHERE pattern_name = ?1 \
             AND source = ?2 AND quarantined = ?3",
            rusqlite::params![pattern, source, excluded],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0) as usize
    };
    let surviving = q("llm-response", 0);
    let quarantined = q("llm-response", 1);
    let held_out = q("held-out", 0);
    (surviving, quarantined, held_out)
}

/// The recovery conditions №990 fixes (decision 6: "the concrete
/// recovery conditions and the parameter surface are fixed in W5 and in
/// the implementation record of this ADR"):
/// (1) `teacher-gate-ok` — the teacher registry gate passes (the direct
///     cause of the `DISTILL_TEACHER_UNLICENSED` violation is cleared);
/// (2) `store-integrity-ok` — the persist store opens and the schema
///     completes (`ensure_distill_store`).
/// Both are machine-true and computable from the conn + the env alone.
pub fn evaluate_recovery(conn: &rusqlite::Connection) -> Result<Vec<(&'static str, bool)>, String> {
    let teacher_ok = crate::interpreter::learnable::distill_teacher_gate().is_ok();
    let store_ok = ensure_distill_store(conn).is_ok();
    Ok(vec![
        ("teacher-gate-ok", teacher_ok),
        ("store-integrity-ok", store_ok),
    ])
}

/// The bounded auto-unfreeze (decision 6): when Frozen, the recovery
/// checks pass, the cooldown clock allows it and no re-freeze lock sits
/// on the pattern — exit Frozen automatically, ledger-record
/// `distill.auto-unfreeze` (the reason, the metrics), trace loudly.
/// Returns `Ok(true)` when the transition happened.
fn try_auto_unfreeze(
    conn: &rusqlite::Connection,
    lc: &mut DistillLifecycle,
    now: u64,
    audit: &dyn Fn(String),
) -> Result<bool, String> {
    if lc.phase != DistillPhase::Frozen {
        return Ok(false);
    }
    let cooldown = distill_auto_unfreeze_cooldown_secs();
    if lc.operator_lock {
        trace_distill(
            audit,
            &format!(
                "distill.auto-unfreeze withheld: {} carries an operator lock — the explicit external command is required (decision 6, the W5 parameter surface)",
                lc.pattern
            ),
        );
        return Ok(false);
    }
    if lc.refroze_in_window {
        trace_distill(
            audit,
            &format!(
                "distill.auto-unfreeze withheld: {} re-froze within the cooldown window — the explicit external command is required (decision 6)",
                lc.pattern
            ),
        );
        return Ok(false);
    }
    if let Some(t) = lc.auto_unfroze_at {
        if now.saturating_sub(t) < cooldown {
            trace_distill(
                audit,
                &format!(
                    "distill.auto-unfreeze withheld: {} is inside the {}s cooldown (last auto-unfreeze {}s ago)",
                    lc.pattern,
                    cooldown,
                    now.saturating_sub(t)
                ),
            );
            return Ok(false);
        }
    }
    let recovery = evaluate_recovery(conn)?;
    if !recovery.iter().all(|(_, ok)| *ok) {
        let failed: Vec<String> = recovery
            .iter()
            .filter(|(_, ok)| !ok)
            .map(|(k, _)| k.to_string())
            .collect();
        trace_distill(
            audit,
            &format!(
                "distill.auto-unfreeze withheld: {} recovery failed: {}",
                lc.pattern,
                failed.join(",")
            ),
        );
        return Ok(false);
    }
    let counts = lifecycle_store_counts(conn, &lc.pattern);
    let reason = format!(
        "recovery passed: teacher-gate-ok=true,store-integrity-ok=true; counts surviving={},quarantined={},held_out={}",
        counts.0, counts.1, counts.2
    );
    lc.phase = DistillPhase::Active;
    lc.entered_at = now;
    lc.auto_unfroze_at = Some(now);
    lc.refroze_in_window = false;
    lc.violations.clear();
    write_lifecycle(conn, lc)?;
    crate::ledger::record(
        "distill.auto-unfreeze",
        "runtime",
        "reflex",
        &format!("pattern={};{}", lc.pattern, reason),
    );
    trace_distill(
        audit,
        &format!(
            "distill.auto-unfreeze: {} exited Frozen (bounded auto-exit, decision 6)",
            lc.pattern
        ),
    );
    Ok(true)
}

/// The violation feed (the hysteresis input): the refusal sites call
/// this with their code. A single violation is refusal-only (no state
/// change); crossing the threshold inside the window auto-freezes (the
/// anti-flap band). A freeze inside the post-auto-unfreeze cooldown
/// window sets the re-freeze lock (decision 6).
pub fn record_violation(
    conn: &rusqlite::Connection,
    pattern: &str,
    code: &str,
    now: u64,
    audit: &dyn Fn(String),
) -> Result<LifecycleVerdict, String> {
    let mut lc = read_lifecycle(conn, pattern)?.unwrap_or_else(|| DistillLifecycle::fresh(pattern));
    if lc.phase == DistillPhase::Frozen {
        return Ok(LifecycleVerdict {
            pattern: pattern.to_string(),
            op: "violation",
            ok: true,
            phase: lc.phase,
            changed: false,
            reason: format!(
                "{} already frozen — the violation {} noted, no state change",
                pattern, code
            ),
            recovery: Vec::new(),
            auto_unfreeze_allowed: false,
        });
    }
    lc.violations.push(now);
    let cutoff = now.saturating_sub(distill_freeze_window_secs());
    lc.violations.retain(|t| *t >= cutoff);
    lc.last_violation_code = Some(code.to_string());
    let threshold = distill_freeze_threshold();
    if (lc.violations.len() as u32) >= threshold {
        let cooldown = distill_auto_unfreeze_cooldown_secs();
        let in_window = lc
            .auto_unfroze_at
            .map(|t| now.saturating_sub(t) < cooldown)
            .unwrap_or(false);
        if in_window {
            lc.refroze_in_window = true;
        }
        lc.phase = DistillPhase::Frozen;
        lc.entered_at = now;
        let ring = lc.violations.len();
        lc.violations.clear();
        // The automation freeze is NOT an operator freeze — the bounded
        // auto-exit stays available for it (decision 6).
        lc.operator_lock = false;
        write_lifecycle(conn, &lc)?;
        crate::ledger::record(
            "distill.freeze",
            "runtime",
            "reflex",
            &format!(
                "pattern={};auto;code={};violations={};window={}s;refroze_in_window={}",
                pattern,
                code,
                ring,
                distill_freeze_window_secs(),
                in_window
            ),
        );
        trace_distill(
            audit,
            &format!(
                "distill.freeze: {} entered Frozen ({} violations within {}s; code {})",
                pattern,
                ring,
                distill_freeze_window_secs(),
                code
            ),
        );
        return Ok(LifecycleVerdict {
            pattern: pattern.to_string(),
            op: "violation",
            ok: true,
            phase: DistillPhase::Frozen,
            changed: true,
            reason: format!(
                "the hysteresis crossed: {} violations within the window (code {})",
                ring, code
            ),
            recovery: Vec::new(),
            auto_unfreeze_allowed: !in_window,
        });
    }
    write_lifecycle(conn, &lc)?;
    trace_distill(
        audit,
        &format!(
            "distill.violation: {} recorded {} ({} in the window; threshold {})",
            pattern,
            code,
            lc.violations.len(),
            threshold
        ),
    );
    Ok(LifecycleVerdict {
        pattern: pattern.to_string(),
        op: "violation",
        ok: true,
        phase: lc.phase,
        changed: false,
        reason: format!(
            "the violation {} noted — {} of the threshold {} inside the window",
            code,
            lc.violations.len(),
            threshold
        ),
        recovery: Vec::new(),
        auto_unfreeze_allowed: false,
    })
}

/// The explicit operator freeze (`mlog distill freeze`) — always works,
/// ledger-records `distill.freeze` (source=operator).
pub fn freeze_explicit(
    conn: &rusqlite::Connection,
    pattern: &str,
    reason: &str,
    now: u64,
    audit: &dyn Fn(String),
) -> Result<LifecycleVerdict, String> {
    let mut lc = read_lifecycle(conn, pattern)?.unwrap_or_else(|| DistillLifecycle::fresh(pattern));
    let already = lc.phase == DistillPhase::Frozen;
    if !already {
        let cooldown = distill_auto_unfreeze_cooldown_secs();
        let in_window = lc
            .auto_unfroze_at
            .map(|t| now.saturating_sub(t) < cooldown)
            .unwrap_or(false);
        if in_window {
            lc.refroze_in_window = true;
        }
        lc.phase = DistillPhase::Frozen;
        lc.entered_at = now;
        lc.violations.clear();
        // The operator's freeze is the authority anchor — the bounded
        // auto-exit never releases it (the W5-fixed surface).
        lc.operator_lock = true;
        write_lifecycle(conn, &lc)?;
    }
    crate::ledger::record(
        "distill.freeze",
        "operator",
        "reflex",
        &format!(
            "pattern={};explicit;already={};reason={}",
            pattern, already, reason
        ),
    );
    trace_distill(
        audit,
        &format!(
            "distill.freeze: {} entered Frozen (explicit operator command{})",
            pattern,
            if already { "; was already frozen" } else { "" }
        ),
    );
    Ok(LifecycleVerdict {
        pattern: pattern.to_string(),
        op: "freeze",
        ok: true,
        phase: DistillPhase::Frozen,
        changed: !already,
        reason: if already {
            format!("{} was already frozen", pattern)
        } else {
            reason.to_string()
        },
        recovery: Vec::new(),
        auto_unfreeze_allowed: false,
    })
}

/// The explicit external unfreeze (decision 6: the re-frozen pattern
/// stays frozen until an explicit external command) — always works,
/// clears the re-freeze lock, ledger-records `distill.unfreeze`.
pub fn unfreeze_explicit(
    conn: &rusqlite::Connection,
    pattern: &str,
    reason: &str,
    now: u64,
    audit: &dyn Fn(String),
) -> Result<LifecycleVerdict, String> {
    let mut lc = read_lifecycle(conn, pattern)?.unwrap_or_else(|| DistillLifecycle::fresh(pattern));
    let was = lc.phase == DistillPhase::Frozen;
    if was {
        lc.phase = DistillPhase::Active;
        lc.entered_at = now;
        lc.refroze_in_window = false;
        lc.operator_lock = false;
        lc.violations.clear();
        write_lifecycle(conn, &lc)?;
    }
    crate::ledger::record(
        "distill.unfreeze",
        "operator",
        "reflex",
        &format!(
            "pattern={};explicit;was_frozen={};reason={}",
            pattern, was, reason
        ),
    );
    trace_distill(
        audit,
        &format!(
            "distill.unfreeze: {} exited Frozen (explicit external command{})",
            pattern,
            if was { "" } else { "; was not frozen" }
        ),
    );
    Ok(LifecycleVerdict {
        pattern: pattern.to_string(),
        op: "unfreeze",
        ok: true,
        phase: lc.phase,
        changed: was,
        reason: if was {
            reason.to_string()
        } else {
            format!("{} was not frozen", pattern)
        },
        recovery: Vec::new(),
        auto_unfreeze_allowed: true,
    })
}

/// The read-only status snapshot (the recovery checks run and are
/// REPORTED; no transition happens here — the bounded auto-unfreeze
/// fires on the distill-move attempts, not on reads).
pub fn lifecycle_status(
    conn: &rusqlite::Connection,
    pattern: &str,
    now: u64,
) -> Result<LifecycleVerdict, String> {
    let lc = read_lifecycle(conn, pattern)?.unwrap_or_else(|| DistillLifecycle::fresh(pattern));
    let recovery = evaluate_recovery(conn)?;
    let cooldown = distill_auto_unfreeze_cooldown_secs();
    let clock_ok = lc
        .auto_unfroze_at
        .map(|t| now.saturating_sub(t) >= cooldown)
        .unwrap_or(true);
    let allowed = lc.phase == DistillPhase::Frozen
        && !lc.refroze_in_window
        && !lc.operator_lock
        && clock_ok
        && recovery.iter().all(|(_, ok)| *ok);
    Ok(LifecycleVerdict {
        pattern: pattern.to_string(),
        op: "status",
        ok: true,
        phase: lc.phase,
        changed: false,
        reason: format!(
            "violations in window: {}; last code: {}; cooldown: {}s {}; operator lock: {}",
            lc.violations.len(),
            lc.last_violation_code
                .clone()
                .unwrap_or_else(|| "none".into()),
            cooldown,
            if lc.refroze_in_window {
                "withheld (re-froze within the window — explicit command required)"
            } else {
                "clear"
            },
            if lc.operator_lock {
                "set (explicit command required)"
            } else {
                "none"
            },
        ),
        recovery,
        auto_unfreeze_allowed: allowed,
    })
}

/// The enforcement gate — the ONE check every distill move consults
/// (the hub worker, the CLI curriculum build). Lazy evaluation point:
/// the bounded auto-unfreeze (decision 6) rides the move attempt, then
/// a still-Frozen pattern refuses with `[DISTILL_FROZEN]` (audited +
/// ledger-recorded). A pattern without W5 state passes (the byte-compat
/// no-op form).
pub fn lifecycle_gate(
    conn: &rusqlite::Connection,
    pattern: &str,
    now: u64,
    audit: &dyn Fn(String),
) -> Result<(), String> {
    let Some(mut lc) = read_lifecycle(conn, pattern)? else {
        return Ok(());
    };
    if lc.phase == DistillPhase::Frozen {
        try_auto_unfreeze(conn, &mut lc, now, audit)?;
        let lc = read_lifecycle(conn, pattern)?.unwrap_or_else(|| DistillLifecycle::fresh(pattern));
        if lc.phase == DistillPhase::Frozen {
            let reason = format!(
                "[DISTILL_FROZEN] pattern '{}' is frozen since epoch {} (last code: {}) — distill moves refuse; the bounded auto-unfreeze is {} ; the explicit external command: mlog distill unfreeze",
                pattern,
                lc.entered_at,
                lc.last_violation_code.clone().unwrap_or_else(|| "none".into()),
                if lc.operator_lock {
                    "withheld (operator lock — the explicit command releases)"
                } else if lc.refroze_in_window {
                    "withheld (re-froze within the cooldown window)"
                } else {
                    "pending the recovery checks"
                },
            );
            crate::ledger::record(
                "distill.frozen-refused",
                "runtime",
                "reflex",
                &format!("pattern={};{}", pattern, reason),
            );
            trace_distill(audit, &format!("distill.frozen-refused: {}", pattern));
            return Err(reason);
        }
    }
    Ok(())
}

/// The curriculum build: propose-and-refute over the pattern's persisted
/// teaching store.
///
/// **Propose:** the teacher-generated units (source `"llm-response"`,
/// not quarantined) for the pattern — each persisted row is one
/// candidate unit.
///
/// **Refute:** (a) a teacher self-contradiction — the same input taught
/// two different outputs quarantines the LATER row (first-teacher-wins);
/// (b) a held-out counterexample — a held-out row (source
/// `"held-out"`) whose input matches a unit's input with a DIFFERENT
/// output quarantines the unit (the reason names the counterexample's
/// rowid). The held-out rows themselves never enter the curriculum.
///
/// **The crossing refusal:** a held-out row whose (input, output) pair
/// EXACTLY duplicates a teacher unit — the «the teacher did not
/// generate this» provenance story is machine-false for that row, so
/// the build refuses loudly instead of silently trusting either side.
///
/// The mutation: quarantined rows are UPDATEd in place (quarantined=1,
/// the reason, reviewed=0), each with its machine id, the reason in the
/// audit journal and a `distill.curriculum-quarantine` ledger record.
/// Nothing else moves; the filter word is the pure recomputation
/// (`nn::attest::curriculum_filter_word`) the certificate's
/// `dataset.filter` writes at save.
pub fn curriculum_build(
    conn: &rusqlite::Connection,
    pattern: &str,
    audit: &mut dyn FnMut(String),
) -> Result<CurriculumVerdict, String> {
    ensure_distill_store(conn)?;

    // ── the two pools ──
    let mut stmt = conn
        .prepare(
            "SELECT rowid, input, output FROM distill_samples \
             WHERE pattern_name = ?1 AND source = 'llm-response' AND quarantined IS NOT 1 \
             ORDER BY rowid",
        )
        .map_err(|e| format!("curriculum: load teacher units: {}", e))?;
    let teacher: Vec<(i64, String, String)> = stmt
        .query_map(rusqlite::params![pattern], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| format!("curriculum: read teacher units: {}", e))?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);

    let mut stmt = conn
        .prepare(
            "SELECT rowid, input, output FROM distill_samples \
             WHERE pattern_name = ?1 AND source = 'held-out' ORDER BY rowid",
        )
        .map_err(|e| format!("curriculum: load held-out rows: {}", e))?;
    let held_out: Vec<(i64, String, String)> = stmt
        .query_map(rusqlite::params![pattern], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| format!("curriculum: read held-out rows: {}", e))?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);

    // ── the crossing refusal (the held-out provenance story must be
    //    machine-true: the teacher did not generate the held-out rows) ──
    for (h_rid, h_in, h_out) in &held_out {
        if teacher
            .iter()
            .any(|(_, t_in, t_out)| t_in == h_in && t_out == h_out)
        {
            return Err(format!(
                "curriculum: crossing refusal — held-out row {} carries the pair the teacher also taught (input hash {}): the held-out provenance story is machine-false; fix the held-out set or the row's source",
                h_rid,
                input_hash8(h_in)
            ));
        }
    }

    // ── propose-and-refute ──
    let mut quarantine: std::collections::BTreeMap<i64, String> = std::collections::BTreeMap::new();
    // (a) teacher self-contradiction: the same input taught two
    // different outputs — the EARLIEST row survives, the later rows
    // quarantine (deterministic first-teacher-wins).
    for i in 0..teacher.len() {
        for j in (i + 1)..teacher.len() {
            if teacher[i].1 == teacher[j].1 && teacher[i].2 != teacher[j].2 {
                let reason = format!(
                    "teacher-self-contradiction: input hash {} was taught two outputs ({:?} by row {}, then {:?} by this row)",
                    input_hash8(&teacher[i].1),
                    teacher[i].2,
                    teacher[i].0,
                    teacher[j].2
                );
                quarantine.entry(teacher[j].0).or_insert(reason);
            }
        }
    }
    // (b) the held-out counterexamples.
    for (t_rid, t_in, t_out) in &teacher {
        for (h_rid, h_in, h_out) in &held_out {
            if t_in == h_in && t_out != h_out {
                let reason = format!(
                    "contradicted-by-held-out: held-out counterexample {} pins a different output ({:?} vs this row's {:?})",
                    h_rid, h_out, t_out
                );
                quarantine.entry(*t_rid).or_insert(reason);
            }
        }
    }

    // ── write the quarantine (in place; the machine id + the reason
    //    ride the audit journal and the ledger) ──
    let mut quarantine_ids: Vec<String> = Vec::new();
    // The quarantine map's every rowid IS a teacher rowid by
    // construction, so iterating the teacher pool (rowid order) visits
    // exactly the quarantined units — no lookup can miss.
    for (rid, t_in, t_out) in &teacher {
        let Some(reason) = quarantine.get(rid) else {
            continue;
        };
        conn.execute(
            "UPDATE distill_samples SET quarantined = 1, reason = ?2, reviewed = 0 WHERE rowid = ?1",
            rusqlite::params![rid, reason],
        )
        .map_err(|e| format!("curriculum: quarantine row {}: {}", rid, e))?;
        let id = quarantine_machine_id(*rid, pattern, t_in, t_out);
        audit(format!(
            "[AUDIT] distill.curriculum-quarantine: {} — {}",
            id, reason
        ));
        crate::ledger::record(
            "distill.curriculum-quarantine",
            "runtime",
            "reflex",
            &format!("pattern={};id={};{}", pattern, id, reason),
        );
        quarantine_ids.push(id);
    }

    let filter_word = crate::nn::attest::curriculum_filter_word(conn, Some(pattern));
    let verdict = CurriculumVerdict {
        pattern: pattern.to_string(),
        proposed: teacher.len(),
        survived: teacher.len() - quarantine.len(),
        quarantined: quarantine.len(),
        held_out: held_out.len(),
        filter_word,
        quarantine_ids,
    };
    audit(format!(
        "[AUDIT] distill.curriculum-build: {}",
        verdict.gate_line()
    ));
    Ok(verdict)
}

// ===========================================================================
// №994 (ADR-0200 W6, the dispatch wave В101 — W-AE): the differential
// enablement gate — the policy enforcer at the enablement boundary
// (decision 5's). A pattern whose effective attestation level is
// `unfiltered` (the data filter word is `none` — decision 5's dominance)
// REFUSES the differential enablement with the machine code
// `[ATTESTATION_ENABLEMENT_UNFILTERED]`; a filtered pattern proceeds and
// `eval.calibration` flips from the literal "uncalibrated" to the
// measured E3 value through the №530-atomic transaction (the certificate
// row and the ledger anchor in one commit). The gate inputs are ONLY the
// recorded E3/E4 artifacts (`docs/research/`) — read without
// recomputation; a missing number is null-with-reason, never silent.
// The refusal and the flip are visible in `--trace-distill` (the same
// bytes to stderr and the audit journal). The lifecycle interlock
// composes: a Frozen pattern refuses the enablement like any other
// distill move. The verify path is untouched (the W1 never-cut lane —
// decision 5's `ok: true` semantics stay as-is; the gate governs the
// ENABLEMENT, not the verify).
// ===========================================================================

/// The W6 enablement verdict — the gate line's source.
#[derive(Debug)]
pub struct EnablementVerdict {
    pub ok: bool,
    pub pattern: String,
    pub level: String,
    pub flipped: bool,
    pub calibration_ece: Option<f64>,
    pub calibration_reason: String,
    pub baseline_agreement: Option<f64>,
    pub baseline_reason: String,
    pub reason: String,
}

impl EnablementVerdict {
    /// The gate line — the same `{"gate":"distill-<op>",...}` form the
    /// lifecycle verdicts use; the JSON nulls carry the null-with-reason
    /// inputs (the reason lines name them).
    pub fn gate_line(&self) -> String {
        serde_json::json!({
            "gate": "distill-enable-differential",
            "ok": self.ok,
            "pattern": self.pattern,
            "level": self.level,
            "flipped": self.flipped,
            "calibration_ece": self.calibration_ece,
            "calibration_reason": self.calibration_reason,
            "baseline_agreement": self.baseline_agreement,
            "baseline_reason": self.baseline_reason,
            "reason": self.reason,
        })
        .to_string()
    }
}

/// №994 (ADR-0200 W6): the differential enablement gate. Errors carry
/// the machine code and the named field/reason; a refusal writes
/// nothing. The flip runs inside one transaction — the №530 atomicity
/// (no new windows between the gate and the write).
#[allow(clippy::disallowed_methods)]
pub fn enable_differential(
    conn: &mut rusqlite::Connection,
    pattern: &str,
    research_dir: &std::path::Path,
    now: u64,
    audit: &dyn Fn(String),
) -> Result<EnablementVerdict, String> {
    // The lifecycle interlock first (W5): a Frozen pattern refuses the
    // enablement like any other distill move.
    lifecycle_gate(conn, pattern, now, audit)?;

    // The gate inputs — the recorded E3/E4 facts only.
    let facts = crate::nn::attest::read_research_facts(research_dir);
    trace_distill(
        audit,
        &format!(
            "enable-differential pattern={} inputs ece={} agreement={}",
            pattern,
            facts
                .calibration_ece
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".into()),
            facts
                .baseline_agreement
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".into()),
        ),
    );

    // The level predicate — decision 5's boundary.
    let Some(stored) = crate::nn::attest::current_cert(conn, pattern)? else {
        let reason = format!(
            "{} pattern '{}' carries no attestation certificate (saved before W1) — the \
             differential enablement refuses: the field `level` is unknown and the \
             provenance story cannot be checked",
            crate::nn::attest::CODE_NONE,
            pattern
        );
        crate::ledger::record(
            "distill.enablement-refused",
            "runtime",
            "reflex",
            &format!("pattern={};{}", pattern, reason),
        );
        trace_distill(
            audit,
            &format!("enable-differential pattern={} REFUSED {}", pattern, reason),
        );
        return Err(reason);
    };
    let level = crate::nn::attest::effective_level_from_cert(&stored.cert_json);
    if level == "unfiltered" {
        let reason = format!(
            "{} pattern '{}' has the effective level `unfiltered` (the data filter word \
             is `none` — decision 5's dominance) — the differential enablement refuses: \
             the field `level`, the measured calibration cannot dominate unfiltered \
             data; verify stays on decision 5's terms (the marker, not the gate)",
            crate::nn::attest::CODE_ENABLEMENT_UNFILTERED,
            pattern
        );
        crate::ledger::record(
            "distill.enablement-refused",
            "runtime",
            "reflex",
            &format!("pattern={};{}", pattern, reason),
        );
        trace_distill(
            audit,
            &format!("enable-differential pattern={} REFUSED {}", pattern, reason),
        );
        return Err(reason);
    }

    // The flip — through the №530-atomic transaction; withheld with the
    // reason when E3 carries no measured number (null-with-reason, the
    // certificate stays "uncalibrated", the report is loud).
    let (flipped, reason) = match facts.calibration_ece {
        Some(ece) => {
            let tx = conn
                .transaction()
                .map_err(|e| format!("enable-differential: transaction: {}", e))?;
            let outcome = crate::nn::attest::flip_calibration_tx(&tx, pattern, ece)?;
            tx.commit()
                .map_err(|e| format!("enable-differential: commit: {}", e))?;
            trace_distill(
                audit,
                &format!(
                    "enable-differential pattern={} FLIPPED eval.calibration={} \
                     ledger_seq={} cert_sha={}",
                    pattern, outcome.ece, outcome.ledger_seq, outcome.new_cert_sha
                ),
            );
            (
                true,
                format!(
                    "eval.calibration flipped to the measured E3 value {} (the №530-atomic \
                     flip, the ledger anchor seq {})",
                    outcome.ece, outcome.ledger_seq
                ),
            )
        }
        None => (false, facts.calibration_reason.clone()),
    };
    crate::ledger::record(
        "distill.enablement",
        "runtime",
        "reflex",
        &format!(
            "pattern={};level={};flipped={};{}",
            pattern, level, flipped, reason
        ),
    );
    Ok(EnablementVerdict {
        ok: true,
        pattern: pattern.to_string(),
        level,
        flipped,
        calibration_ece: facts.calibration_ece,
        calibration_reason: facts.calibration_reason,
        baseline_agreement: facts.baseline_agreement,
        baseline_reason: facts.baseline_reason,
        reason,
    })
}
