//! RuntimeContext — shared runtime state for stateful builtins.
//!
//! Наряд №178 Block 4: prevents TW/VM duplication of stateful builtins.
//! Both backends marshal arguments, but the actual stateful operations
//! (reflex_predict, reflex_train) live here — written once, used by both.
//!
//! This is intentionally minimal now — fields are added as needed
//! in follow-up naryads. The structure itself is the foundation that
//! prevents the "stateful builtin duplicated in vm.rs + execution.rs"
//! anti-pattern from the start.

use crate::nn::ReflexRegistry;

/// Shared runtime context for stateful operations.
///
/// Created once per execution (interpreter or VM), passed to builtins
/// that need shared state beyond the call stack.
#[allow(dead_code)]
pub struct RuntimeContext {
    /// Reflex model registry — stores all `reflex` declarations.
    /// `Value::Reflex(ReflexId)` indexes into this.
    pub reflex_registry: ReflexRegistry,
}

impl Default for RuntimeContext {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeContext {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            reflex_registry: ReflexRegistry::new(),
        }
    }
}
