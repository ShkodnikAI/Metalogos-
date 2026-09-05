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

    /// `reflex_save(model) -> Unit` (Наряд №180, ADR-0116)
    ///
    /// `&self` wrapper around `crate::builtins::reflex::reflex_save_dispatch`.
    /// Acquires the Mutex on `self.reflex_registry` (read-only — save does
    /// not mutate weights) and passes the SQLite persist path (from
    /// `memory { persist: "..." }`) to the dispatch function.
    pub(super) fn invoke_reflex_save(&self, args: Vec<Value>) -> Result<Value, String> {
        let reg = self
            .reflex_registry
            .lock()
            .map_err(|e| format!("reflex_save: registry lock poisoned: {}", e))?;
        let persist = self.get_memory_persist_path();
        crate::builtins::reflex_save_dispatch(&reg, &self.reflex_names, persist.as_deref(), &args)
    }

    /// `reflex_load(name) -> Value::Reflex` (Наряд №180, ADR-0116)
    ///
    /// `&self` wrapper around `crate::builtins::reflex::reflex_load_dispatch`.
    /// Acquires the Mutex on `self.reflex_registry` (mutating — load overwrites
    /// model weights) and passes the SQLite persist path.
    pub(super) fn invoke_reflex_load(&self, args: Vec<Value>) -> Result<Value, String> {
        let mut reg = self
            .reflex_registry
            .lock()
            .map_err(|e| format!("reflex_load: registry lock poisoned: {}", e))?;
        let persist = self.get_memory_persist_path();
        crate::builtins::reflex_load_dispatch(
            &mut reg,
            &self.reflex_names,
            persist.as_deref(),
            &args,
        )
    }
}
