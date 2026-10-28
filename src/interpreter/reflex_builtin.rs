//! Reflex builtins: `&self` invoke wrappers.
//!
//! Наряд №179b — connects the language to `ReflexModel::train` (Nаряд №179).
//! Наряд №180 — adds persistence (save/load to SQLite, ADR-0116).
//!
//! `eval_expr_with_env` is `&self`, so we cannot dispatch through `invoke`
//! (which is `&mut self`). These wrappers use the `Mutex<ReflexRegistry>`
//! field to mutate weights from `&self` — same pattern as `invoke_recall`
//! (which uses `self.memory.lock()`).
//!
//! The dispatch logic itself lives in `src/builtins/reflex.rs` — shared
//! with VM when it gains reflex support.

use super::*;

impl Interpreter {
    /// `reflex_train(model, data, epochs, metric, threshold) -> Struct`
    ///
    /// `&self` wrapper around `crate::builtins::reflex::reflex_train_dispatch`.
    /// Acquires the Mutex on `self.reflex_registry` to mutate model weights.
    pub(super) fn invoke_reflex_train(&self, args: Vec<Value>) -> Result<Value, String> {
        let mut reg = self
            .reflex_registry
            .lock()
            .map_err(|e| format!("reflex_train: registry lock poisoned: {}", e))?;
        crate::builtins::reflex_train_dispatch(&mut reg, &args)
    }

    /// `reflex_predict(model, input) -> Fluid`
    ///
    /// `&self` wrapper around `crate::builtins::reflex::reflex_predict_dispatch`.
    /// Acquires the Mutex on `self.reflex_registry` (read-only — but Mutex has
    /// no separate read mode, so we use the same lock).
    pub(super) fn invoke_reflex_predict(&self, args: Vec<Value>) -> Result<Value, String> {
        let reg = self
            .reflex_registry
            .lock()
            .map_err(|e| format!("reflex_predict: registry lock poisoned: {}", e))?;
        crate::builtins::reflex_predict_dispatch(&reg, &args)
    }

    /// `reflex_save(model) -> Unit` (Наряд №180, ADR-0116; №825 — attestation)
    ///
    /// `&self` wrapper around `crate::builtins::reflex::reflex_save_dispatch`.
    /// Acquires the Mutex on `self.reflex_registry` (read-only — save does
    /// not mutate weights) and passes the SQLite persist path (from
    /// `memory { persist: "..." }`) to the dispatch function.
    /// №825: the attestation input (the pattern spec + the live teaching
    /// examples) is built FIRST (the borrow ends before the lock).
    pub(super) fn invoke_reflex_save(&self, args: Vec<Value>) -> Result<Value, String> {
        let attest_input = self.attest_input_for_args(&args);
        let reg = self
            .reflex_registry
            .lock()
            .map_err(|e| format!("reflex_save: registry lock poisoned: {}", e))?;
        let persist = self.get_memory_persist_path();
        crate::builtins::reflex_save_dispatch(
            &reg,
            &self.reflex_names,
            persist.as_deref(),
            attest_input.as_ref(),
            &args,
        )
    }

    /// `reflex_load(name) -> Value::Reflex` (Наряд №180, ADR-0116; №825 — attestation)
    ///
    /// `&self` wrapper around `crate::builtins::reflex::reflex_load_dispatch`.
    /// Acquires the Mutex on `self.reflex_registry` (mutating — load overwrites
    /// model weights) and passes the SQLite persist path.
    /// №825: the attestation input is built FIRST (the borrow ends before
    /// the lock); the policy comes from the environment at the dispatch.
    pub(super) fn invoke_reflex_load(&self, args: Vec<Value>) -> Result<Value, String> {
        let attest_input = self.attest_input_for_args(&args);
        let mut reg = self
            .reflex_registry
            .lock()
            .map_err(|e| format!("reflex_load: registry lock poisoned: {}", e))?;
        let persist = self.get_memory_persist_path();
        crate::builtins::reflex_load_dispatch(
            &mut reg,
            &self.reflex_names,
            persist.as_deref(),
            attest_input.as_ref(),
            &args,
        )
    }

    /// №825 (ADR-0200): the attestation input for the model named by the
    /// first argument (handle or String). Finds the learnable pattern that
    /// distills into this model and packages its spec facts + live teaching
    /// examples. `None` — no distilling pattern (a direct-reflex_train
    /// model attests with the declaration-shape-only spec, the honest null
    /// pattern/prompt form).
    pub(super) fn attest_input_for_args(
        &self,
        args: &[Value],
    ) -> Option<crate::nn::attest::AttestInput> {
        let model_name: String = match args.first()? {
            Value::Reflex(id) => self
                .reflex_names
                .iter()
                .find(|(_, rid)| **rid == *id)
                .map(|(name, _)| name.clone())?,
            Value::String(name) => name.clone(),
            _ => return None,
        };
        self.attest_input_for_model(&model_name)
    }

    /// №825: the canonical TW extraction — the SAME field mapping the VM
    /// uses over `CompiledLearnableInfo` (the canonical form lives in
    /// `nn::attest`, both lanes attest identically).
    pub(super) fn attest_input_for_model(
        &self,
        model_name: &str,
    ) -> Option<crate::nn::attest::AttestInput> {
        let (pattern_name, lp) = self.learnable_patterns.iter().find(|(_, lp)| {
            lp.distill
                .as_ref()
                .map(|d| d.reflex_name == model_name)
                .unwrap_or(false)
        })?;
        let d = lp.distill.as_ref()?;
        let examples = self
            .distill_states
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(pattern_name)
            .map(|s| s.examples.clone())
            .unwrap_or_default();
        Some(crate::builtins::attest::attest_input_from_learnable(
            pattern_name,
            &lp.prompt,
            &d.reflex_name,
            d.distill_after,
            d.fallback_if
                .as_ref()
                .map(|(op, v)| (crate::builtins::attest::compare_op_word(op), *v)),
            d.min_accuracy,
            d.margin,
            examples,
        ))
    }

    /// `attest_verify(model) -> Struct { ok, level, spec_hash, reason }`
    /// (Наряд №825, ADR-0200) — read-only provenance verification; tier
    /// `experimental` (поправка (г)).
    pub(super) fn invoke_attest_verify(&self, args: Vec<Value>) -> Result<Value, String> {
        let attest_input = self.attest_input_for_args(&args);
        let reg = self
            .reflex_registry
            .lock()
            .map_err(|e| format!("attest_verify: registry lock poisoned: {}", e))?;
        let persist = self.get_memory_persist_path();
        crate::builtins::attest_verify_dispatch(
            &reg,
            &self.reflex_names,
            persist.as_deref(),
            attest_input.as_ref(),
            &args,
        )
    }

    /// `reflex_metrics(model) -> Struct` (Наряд №187, ADR-0114)
    ///
    /// `&self` wrapper around `crate::builtins::reflex::reflex_metrics_dispatch`.
    /// Read-only — acquires the Mutex on `self.reflex_registry` (no mutation,
    /// but Mutex has no separate read mode).
    pub(super) fn invoke_reflex_metrics(&self, args: Vec<Value>) -> Result<Value, String> {
        let reg = self
            .reflex_registry
            .lock()
            .map_err(|e| format!("reflex_metrics: registry lock poisoned: {}", e))?;
        crate::builtins::reflex_metrics_dispatch(&reg, &args)
    }

    /// `reflex_list() -> List<String>` (Наряд №187, ADR-0114)
    ///
    /// `&self` wrapper around `crate::builtins::reflex::reflex_list_dispatch`.
    /// Read-only — uses the `reflex_names` HashMap to list declared model names
    /// in registration order.
    pub(super) fn invoke_reflex_list(&self, args: Vec<Value>) -> Result<Value, String> {
        let reg = self
            .reflex_registry
            .lock()
            .map_err(|e| format!("reflex_list: registry lock poisoned: {}", e))?;
        crate::builtins::reflex_list_dispatch(&reg, &self.reflex_names, &args)
    }

    /// `reflex_generate(model, prompt, max_tokens, temperature) -> List<Float>`
    /// (Наряд №193, ADR-0120)
    ///
    /// `&self` wrapper around `crate::builtins::reflex::reflex_generate_dispatch`.
    /// Read-only (doesn't mutate weights — generation, not training).
    pub(super) fn invoke_reflex_generate(&self, args: Vec<Value>) -> Result<Value, String> {
        let reg = self
            .reflex_registry
            .lock()
            .map_err(|e| format!("reflex_generate: registry lock poisoned: {}", e))?;
        crate::builtins::reflex_generate_dispatch(&reg, &args)
    }
}
