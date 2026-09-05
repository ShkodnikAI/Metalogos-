//! Reflex builtins: `&self` invoke wrappers for reflex_train / reflex_predict.
//!
//! Наряд №179b — connects the language to `ReflexModel::train` (Nаряд №179).
//!
//! `eval_expr_with_env` is `&self`, so we cannot dispatch through `invoke`
//! (which is `&mut self`). These wrappers use the `Mutex<ReflexRegistry>`
//! field to mutate weights from `&self` — same pattern as `invoke_recall`
//! (which uses `self.memory.lock()`).
//!
//! The dispatch logic itself lives in `src/builtins/reflex.rs`
//! (`reflex_train_dispatch` / `reflex_predict_dispatch`) — shared with VM
//! when it gains reflex support.

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
}
