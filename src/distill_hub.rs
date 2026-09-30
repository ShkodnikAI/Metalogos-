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
                conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS distill_samples (
                        pattern_name TEXT NOT NULL,
                        input        TEXT NOT NULL,
                        output       TEXT NOT NULL,
                        source       TEXT NOT NULL,
                        created_at   INTEGER NOT NULL
                    );",
                )
                .map_err(|e| format!("distill hub: create distill_samples: {}", e))?;
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
        });

        // Load the persisted examples (honest migration: existing saved
        // weights read through reflex_save as before; the example ledger
        // starts from what distill_samples holds — zero on the first
        // 0.27.1 boot, then it survives restarts).
        if let Some(persist) = &shared.persist {
            let conn = persist.lock().unwrap_or_else(|e| e.into_inner());
            let mut stmt = conn
                .prepare(
                    "SELECT pattern_name, input, output, source FROM distill_samples ORDER BY rowid",
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
                    return Ok(None);
                }
                Ok(Some(Value::String(best_label)))
            }
        }
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

// ── №530 (issue #839; audit 30.09 N-4): the out-of-lock training, the
// DISTILLED retraining, and the loud degradation → TEACHING revert ──
#[cfg(test)]
mod n530_tests {
    use super::*;

    fn source() -> String {
        let mock_label = crate::llm::mock_response("answer");
        format!(
            r#"
reflex Head {{
  input: embedding(4)
  layers: [dense(4, "relu"), dense(2, "softmax")]
  labels: ["{mock_label}", "other"]
  seed: 42
}}
"#
        )
    }

    struct Fixture {
        hub: std::sync::Arc<DistillHub>,
        audit_lines: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
        pattern: &'static str,
        label: String,
    }

    fn make_hub() -> Fixture {
        let decls = crate::parser::parse(&source()).expect("reflex source parses");
        let audit_lines: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = std::sync::Arc::clone(&audit_lines);
        let audit: std::sync::Arc<dyn Fn(String) + Send + Sync> =
            std::sync::Arc::new(move |line| {
                sink.lock().unwrap_or_else(|e| e.into_inner()).push(line)
            });
        let hub = DistillHub::open(audit, None, &decls).expect("hub opens");
        let label = crate::llm::mock_response("answer");
        Fixture {
            hub,
            audit_lines,
            pattern: "Ask",
            label,
        }
    }

    fn spec(distill_after: usize) -> DistillSpec {
        DistillSpec {
            reflex_name: "Head".to_string(),
            distill_after,
            fallback_if: Some((crate::ast::CompareOp::Lt, 0.7)),
            min_accuracy: 0.8,
            margin: 0.05,
        }
    }

    fn wait_for(fixture: &Fixture, marker: &str, timeout_ms: u64) -> bool {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        while std::time::Instant::now() < deadline {
            if fixture
                .audit_lines
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .iter()
                .any(|l| l.contains(marker))
            {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        false
    }

    fn fingerprint(fixture: &Fixture) -> u64 {
        let id = *fixture
            .hub
            .shared
            .names
            .get("Head")
            .expect("the reflex handle");
        let reg_arc = fixture.hub.raw_registry_for_tests();
        let reg = reg_arc.lock().unwrap_or_else(|e| e.into_inner());
        match reg.get(id) {
            Some(crate::nn::ModelKind::Dense(m)) => m.weights_fingerprint(),
            _ => panic!("a Dense model exists"),
        }
    }

    fn mode(fixture: &Fixture) -> DistillMode {
        let inner = fixture
            .hub
            .shared
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        inner
            .states
            .get(fixture.pattern)
            .map(|s| s.mode)
            .unwrap_or(DistillMode::Teaching)
    }

    #[test]
    fn n530_failed_gate_leaves_weights_untouched() {
        // The initial training passes (single-label data: the №496 honest
        // trick — the all-one-label holdout accuracy is 1.0 and the
        // single-class carve-out applies min_accuracy raw).
        let f = make_hub();
        for i in 0..25 {
            f.hub
                .record_example(f.pattern, &format!("alpha {i}"), &f.label);
        }
        let s = spec(20);
        f.hub.try_distilled_call(f.pattern, &s, "alpha 1").unwrap();
        assert!(
            wait_for(&f, "distill.training-finished", 15000),
            "the initial training finishes"
        );
        assert_eq!(mode(&f), DistillMode::Distilled);

        // The retrain: 30 NEW examples on the SAME input with the
        // OPPOSITE label — the feature vectors collapse to one point
        // (dim 4 hashed TF-IDF), the majority-baseline gate (№485)
        // refuses the degenerate majority model: the gate REJECTS.
        let other = "other";
        for i in 0..30 {
            let _ = i;
            f.hub.record_example(f.pattern, "alpha conflict", other);
        }
        let before = fingerprint(&f);
        f.hub.try_distilled_call(f.pattern, &s, "alpha 1").unwrap();
        assert!(
            wait_for(&f, "distill.degraded", 15000),
            "the loud degradation audit lands"
        );
        assert_eq!(mode(&f), DistillMode::Teaching, "the loud TEACHING revert");
        let after = fingerprint(&f);
        assert_eq!(
            before, after,
            "the live weights are untouched by the failed-gate retrain"
        );
    }

    #[test]
    fn n530_retrain_succeeds_and_stays_distilled() {
        let f = make_hub();
        for i in 0..25 {
            f.hub
                .record_example(f.pattern, &format!("alpha {i}"), &f.label);
        }
        let s = spec(20);
        f.hub.try_distilled_call(f.pattern, &s, "alpha 1").unwrap();
        assert!(wait_for(&f, "distill.training-finished", 15000));
        assert_eq!(mode(&f), DistillMode::Distilled);

        // Fresh CONSISTENT examples — the retrain passes the gate and the
        // pattern stays DISTILLED (the weights swap in atomically).
        for i in 0..25 {
            f.hub
                .record_example(f.pattern, &format!("gamma {i}"), &f.label);
        }
        f.hub.try_distilled_call(f.pattern, &s, "gamma 1").unwrap();
        assert!(
            wait_for(&f, "distill.retraining-started", 15000),
            "the retrain trigger fires on the accumulated buffer"
        );
        assert!(
            wait_for(&f, "distill.training-finished", 15000),
            "the retrain finishes"
        );
        assert!(
            !f.audit_lines
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .iter()
                .any(|l| l.contains("distill.degraded")),
            "no degradation on consistent data"
        );
        assert_eq!(mode(&f), DistillMode::Distilled);
    }

    #[test]
    fn n530_training_does_not_hold_the_registry_lock() {
        // The large buffer makes the 30-epoch run long enough (hundreds of
        // ms) that the pre-№530 behavior — holding the registry lock for
        // the WHOLE run — is distinguishable deterministically: the test
        // acquires the lock repeatedly and every acquisition BEFORE the
        // finished line must be sub-100ms (the old code blocked for the
        // whole training).
        let f = make_hub();
        for i in 0..4000 {
            f.hub
                .record_example(f.pattern, &format!("alpha token{i} word{i} {i}"), &f.label);
        }
        let s = spec(20);
        f.hub.try_distilled_call(f.pattern, &s, "alpha 1").unwrap();
        // Wait for the STARTED line — the trainer is now inside the run.
        assert!(wait_for(&f, "distill.training-started", 15000));
        std::thread::sleep(std::time::Duration::from_millis(10));
        let mut acquired_before_finish = 0usize;
        let mut worst = std::time::Duration::ZERO;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        let finished = |f: &Fixture| {
            f.audit_lines
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .iter()
                .any(|l| l.contains("distill.training-finished"))
        };
        while std::time::Instant::now() < deadline && !finished(&f) {
            let t0 = std::time::Instant::now();
            let reg_arc = f.hub.raw_registry_for_tests();
            {
                let _reg = reg_arc.lock().unwrap_or_else(|e| e.into_inner());
            }
            drop(reg_arc);
            let elapsed = t0.elapsed();
            worst = worst.max(elapsed);
            acquired_before_finish += 1;
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(finished(&f), "the training finishes within the deadline");
        assert!(
            acquired_before_finish >= 3,
            "the lock was acquirable DURING training (acquired {acquired_before_finish})"
        );
        assert!(
            worst < std::time::Duration::from_millis(100),
            "every in-training acquisition is fast (worst {worst:?}) — the lock is never held by an epoch loop"
        );
    }
}
