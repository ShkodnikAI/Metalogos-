//! Activation functions for neural network layers.
//!
//! Наряд №178: reuses the numerically stable implementations from
//! `src/builtins/math.rs` (Наряд №177) — no duplicate math.
//!
//! Наряд №182: the duplication of `sigmoid_stable` / inline softmax
//! between this file and `src/builtins/math.rs` is now eliminated —
//! both sides call `crate::builtins::math_core::{sigmoid_raw, softmax_raw}`.
//! The numerical algorithm is byte-identical to the pre-Наряд №182
//! implementations; existing tests in `tests/naryad_177_reflex_math.rs`
//! and `tests/naryad_178_dense_forward.rs` must pass unchanged
//! (Наряд №182 contract: refactor structure, not behavior).

/// Activation function kind — parsed from layer config string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationKind {
    None,
    Relu,
    Sigmoid,
    Tanh,
    Softmax,
}

impl ActivationKind {
    /// Parse from a string (case-insensitive).
    pub fn parse_kind(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "none" | "linear" | "identity" => Ok(ActivationKind::None),
            "relu" => Ok(ActivationKind::Relu),
            "sigmoid" => Ok(ActivationKind::Sigmoid),
            "tanh" => Ok(ActivationKind::Tanh),
            "softmax" => Ok(ActivationKind::Softmax),
            other => Err(format!(
                "unknown activation '{}'. Valid: none, relu, sigmoid, tanh, softmax",
                other
            )),
        }
    }

    /// Apply the activation to a vector of values (in-place for elementwise,
    /// returns new vec for softmax which needs the full vector).
    ///
    /// Наряд №182: sigmoid and softmax delegate to
    /// `crate::builtins::math_core::sigmoid_raw` / `softmax_raw` —
    /// the same numerically stable algorithm shared with `math.rs`.
    pub fn apply(&self, input: &mut [f64]) {
        match self {
            ActivationKind::None => {}
            ActivationKind::Relu => {
                for v in input.iter_mut() {
                    *v = v.max(0.0);
                }
            }
            ActivationKind::Sigmoid => {
                // Numerically stable sigmoid — shared with builtin_sigmoid.
                for v in input.iter_mut() {
                    *v = crate::builtins::math_core::sigmoid_raw(*v);
                }
            }
            ActivationKind::Tanh => {
                // f64::tanh is naturally bounded — no overflow.
                for v in input.iter_mut() {
                    *v = v.tanh();
                }
            }
            ActivationKind::Softmax => {
                // Numerically stable softmax — shared with builtin_softmax.
                // Returns a new Vec; copy back into the in-place buffer.
                // (Same edge case handling: sum=0 → uniform distribution.)
                let softmaxed = crate::builtins::math_core::softmax_raw(input);
                for (i, v) in input.iter_mut().enumerate() {
                    *v = softmaxed[i];
                }
            }
        }
    }
}

/// Marker trait — ActivationKind is the only activation enum.
/// This allows future extensibility (custom activations) without
/// changing the Layer trait.
pub trait Activation {
    fn kind(&self) -> ActivationKind;
}

impl Activation for ActivationKind {
    fn kind(&self) -> ActivationKind {
        *self
    }
}
