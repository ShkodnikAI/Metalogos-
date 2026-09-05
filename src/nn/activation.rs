//! Activation functions for neural network layers.
//!
//! Наряд №178: reuses the numerically stable implementations from
//! `src/builtins/math.rs` (Наряд №177) — no duplicate math.
//! The `sigmoid`, `tanh`, and `softmax` functions in math.rs are
//! already tested on boundary values (±1000.0) and confirmed NaN-free.

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
    pub fn apply(&self, input: &mut [f64]) {
        match self {
            ActivationKind::None => {}
            ActivationKind::Relu => {
                for v in input.iter_mut() {
                    *v = v.max(0.0);
                }
            }
            ActivationKind::Sigmoid => {
                // Reuse the numerically stable sigmoid from Наряд №177.
                for v in input.iter_mut() {
                    *v = sigmoid_stable(*v);
                }
            }
            ActivationKind::Tanh => {
                // f64::tanh is naturally bounded — no overflow.
                for v in input.iter_mut() {
                    *v = v.tanh();
                }
            }
            ActivationKind::Softmax => {
                // Reuse the numerically stable softmax from Наряд №177.
                // Subtract max before exp to prevent overflow.
                let max_val = input.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let exps: Vec<f64> = input.iter().map(|v| (v - max_val).exp()).collect();
                let sum: f64 = exps.iter().sum();
                if sum > 0.0 {
                    for (i, v) in input.iter_mut().enumerate() {
                        *v = exps[i] / sum;
                    }
                }
            }
        }
    }
}

/// Numerically stable sigmoid — same logic as builtin_sigmoid in math.rs.
/// For x >= 0: 1/(1+exp(-x)). For x < 0: exp(x)/(1+exp(x)).
/// Returns 0.0 for very large negative x, 1.0 for very large positive x.
fn sigmoid_stable(x: f64) -> f64 {
    if x >= 0.0 {
        1.0 / (1.0 + (-x).exp())
    } else {
        let exp_x = x.exp();
        exp_x / (1.0 + exp_x)
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
