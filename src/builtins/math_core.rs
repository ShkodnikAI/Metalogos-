//! Core math primitives shared between `builtin_*` handlers (math.rs)
//! and `nn/activation.rs` — Наряд №182 dedup.
//!
//! Prior to Наряд №182, both `src/builtins/math.rs` (builtin_sigmoid,
//! builtin_softmax — `&[Value]` interface) and `src/nn/activation.rs`
//! (sigmoid_stable, inline softmax — `&[f64]` / `&mut [f64]` interface)
//! carried their own copy of the same numerically stable algorithm.
//!
//! The duplication wasn't a trivial "forgot to refactor" — the two sides
//! operate on different value types (`Value` vs raw `f64`), and calling
//! the `Value`-wrapping builtin on every neuron of every layer would
//! mean packing/unpacking `Value` per-element — a real performance and
//! ergonomics regression, not a hypothetical one.
//!
//! This module extracts the pure `f64` math (no `Value`, no `Vec<Value}`,
//! no `Result`) into free functions that both sides call. The numerical
//! algorithm is byte-identical to the pre-Наряд №182 implementations —
//! the existing tests in `tests/naryad_177_reflex_math.rs` (math.rs)
//! and `tests/naryad_178_dense_forward.rs` (activation.rs) must pass
//! unchanged (Наряд №182 contract: refactor structure, not behavior).
//!
//! Numerical stability (unchanged from the original):
//!   - `sigmoid_raw`: for x >= 0 uses 1/(1+exp(-x)) (exp arg in (-∞, 0]);
//!     for x < 0 uses exp(x)/(1+exp(x)) (exp arg in (0, 1) — no overflow).
//!     Returns 1.0 for very large positive x, 0.0 for very large negative x.
//!   - `softmax_raw`: subtracts max before exp (prevents overflow on large
//!     inputs); output sums to 1.0 within f64 epsilon. Edge case: all
//!     inputs at -inf or similar (sum=0) → returns uniform distribution.

/// Numerically stable logistic sigmoid: 1 / (1 + e^{-x}).
///
/// Returns:
///   - 1.0 for very large positive x (exp(-x) underflows to 0)
///   - 0.0 for very large negative x (exp(x) underflows to 0)
///   - NaN if x is NaN (propagated, as in the original implementation)
pub(crate) fn sigmoid_raw(x: f64) -> f64 {
    if x >= 0.0 {
        // For x >= 0: exp(-x) is in (0, 1], no overflow risk.
        // 1.0 / (1.0 + exp(-x)) — standard formula, safe here.
        1.0 / (1.0 + (-x).exp())
    } else {
        // For x < 0: exp(-x) would overflow for large |x|.
        // Use the equivalent form: exp(x) / (1 + exp(x))
        // where exp(x) is in (0, 1) for x < 0 — no overflow.
        let exp_x = x.exp();
        exp_x / (1.0 + exp_x)
    }
}

/// Numerically stable softmax over a slice.
///
/// Subtracts max before exp to prevent overflow. Returns a Vec<f64>
/// of the same length as the input, summing to 1.0 (within f64 epsilon).
///
/// Edge case: if all inputs are -inf or the sum underflows to 0,
/// returns a uniform distribution (1/n for each element) — matches
/// the pre-Наряд №182 behavior of `builtin_softmax` exactly.
pub(crate) fn softmax_raw(xs: &[f64]) -> Vec<f64> {
    if xs.is_empty() {
        return Vec::new();
    }
    // Numerical stability: subtract max before exp
    let max_val = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = xs.iter().map(|v| (v - max_val).exp()).collect();
    let sum: f64 = exps.iter().sum();
    if sum == 0.0 {
        // All inputs were -inf or similar; return uniform distribution.
        let n = xs.len() as f64;
        xs.iter().map(|_| 1.0 / n).collect()
    } else {
        exps.iter().map(|e| e / sum).collect()
    }
}
