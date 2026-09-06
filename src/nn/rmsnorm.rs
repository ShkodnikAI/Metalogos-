//! RmsNorm — second `SequenceLayer` (Наряд №184, Block 1).
//!
//! Implements root-mean-square normalization (Zhang & Sennrich 2019),
//! the standard normalization in modern transformer architectures
//! (Llama, Mistral, etc.). Documented in Наряд №176's reference analysis
//! of `candle-transformers` Llama structure.
//!
//! ## Formula
//!
//! ```text
//! y_i = x_i / sqrt(mean(x^2) + eps) * w_i
//! ```
//!
//! where `mean(x^2)` is computed across the feature dimension (last axis).
//!
//! ## Relation to LayerNorm
//!
//! RmsNorm drops the mean-subtraction step of LayerNorm — only the
//! scaling by reciprocal RMS. This is what Llama uses; it's cheaper
//! than LayerNorm (no mean computation) and empirically performs
//! comparably. The reference analysis in Наряд №176 confirmed Llama
//! uses exactly RmsNorm (not LayerNorm) in both pre-norm positions.
//!
//! ## Weight init
//!
//! Default init: `weight = ones(dim)` (matches Llama's init: at start
//! of training, RmsNorm is the identity when eps is small). For tests
//! that need a non-trivial weight, the layer accepts a separate
//! `norm_seed` parameter so norm weights can differ from attention
//! weights (otherwise the same RNG stream would couple them).
//!
//! ## Determinism
//!
//! Same `xorshift64` PRNG (Наряд №177) as Attention (Наряд №183) and
//! Dense (Наряд №178) — same seed → same weights → same forward-pass
//! result. No `candle` RNG involved.

#![cfg(feature = "candle")]

use crate::interpreter::Value;
use crate::nn::sequence_layer::SequenceLayer;

use candle_core::{DType, Device, Tensor};

/// RmsNorm layer.
///
/// Operates on `[seq_len, dim]` tensors (per ADR-0119 — `SequenceLayer`
/// always operates on sequences, never single vectors).
pub struct RmsNorm {
    /// Feature dimension (input == output for norm layers).
    dim: usize,
    /// Scale weights `[dim]`.
    weight: Tensor,
    /// Epsilon for numerical stability (prevents divide-by-zero on
    /// near-zero inputs). Llama uses 1e-6.
    eps: f64,
}

impl RmsNorm {
    /// Construct with explicit weight vector.
    pub fn with_weights(dim: usize, weights: Vec<f32>, eps: f64) -> Result<Self, String> {
        if weights.len() != dim {
            return Err(format!(
                "rms_norm: weight length {} != dim {}",
                weights.len(),
                dim
            ));
        }
        let device = Device::Cpu;
        let weight = map_err(
            Tensor::from_slice(&weights, (dim,), &device).and_then(|t| t.to_dtype(DType::F32)),
            "rms_norm: weight init",
        )?;
        Ok(Self { dim, weight, eps })
    }

    /// Construct with ones-init weights (Llama default).
    /// `seed` is accepted for API uniformity with other SequenceLayers
    /// (each layer receives the model-level seed), but since the
    /// default init is `ones`, the seed has no effect here. Tests
    /// that need non-trivial weights use `with_weights` directly.
    pub fn new(dim: usize, _seed: u64, eps: f64) -> Result<Self, String> {
        let weights = vec![1.0f32; dim];
        Self::with_weights(dim, weights, eps)
    }

    fn forward_impl(&self, input: &Tensor) -> Result<Tensor, String> {
        let (_seq_len, in_dim) = map_err(input.dims2(), "rms_norm: input dims2")?;
        if in_dim != self.dim {
            return Err(format!(
                "rms_norm: input dim {} != layer dim {}",
                in_dim, self.dim
            ));
        }

        // Compute mean(x^2) along last axis: x^2 → mean → [seq, 1]
        let x_f64 = map_err(input.to_dtype(DType::F64), "rms_norm: cast to f64")?;
        let sq = map_err(x_f64.sqr(), "rms_norm: sqr")?;
        let mean_sq = map_err(sq.mean_keepdim(1), "rms_norm: mean_keepdim")?;

        // rsqrt(mean + eps) → [seq, 1]. candle 0.11 has no `rsqrt` method;
        // use `1.0 / sqrt(...)` (computed in f64 for numerical stability
        // near zero — f32 would lose precision when mean is tiny).
        let eps_t = map_err(Tensor::new(self.eps, input.device()), "rms_norm: eps")?;
        let denom = map_err(mean_sq.broadcast_add(&eps_t), "rms_norm: mean+eps")?;
        let sqrt_denom = map_err(denom.sqrt(), "rms_norm: sqrt")?;
        let one = map_err(Tensor::new(1.0f64, input.device()), "rms_norm: 1.0")?;
        let inv = map_err(one.broadcast_div(&sqrt_denom), "rms_norm: 1/sqrt")?;

        // x * inv → broadcast over dim → [seq, dim]
        let inv = map_err(inv.to_dtype(DType::F32), "rms_norm: inv to f32")?;
        let normalized = map_err(input.broadcast_mul(&inv), "rms_norm: normalize")?;

        // * weight (broadcast [dim] → [seq, dim] via unsqueeze)
        let weight_b = map_err(self.weight.unsqueeze(0), "rms_norm: weight unsqueeze")?;
        map_err(normalized.broadcast_mul(&weight_b), "rms_norm: scale")
    }
}

impl SequenceLayer for RmsNorm {
    fn forward(&self, input: &Tensor) -> Result<Tensor, String> {
        self.forward_impl(input)
    }

    fn input_dim(&self) -> usize {
        self.dim
    }

    fn output_dim(&self) -> usize {
        self.dim
    }

    fn name(&self) -> &str {
        "rms_norm"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Build function for the SEQUENCE_LAYER_REGISTRY.
///
/// Args: `(dim)` or `(dim, eps)`. The `eps` parameter is optional —
/// default `1e-6` (Llama's value) when omitted.
pub fn build_rmsnorm(args: &[Value], seed: u64) -> Result<Box<dyn SequenceLayer>, String> {
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "rms_norm: expected 1 or 2 args (dim, [eps]), got {}",
            args.len()
        ));
    }
    let dim = parse_usize_arg(&args[0], "rms_norm", "dim")?;
    let eps = if args.len() == 2 {
        parse_f64_arg(&args[1], "rms_norm", "eps")?
    } else {
        1e-6
    };
    let layer = RmsNorm::new(dim, seed, eps)?;
    Ok(Box::new(layer))
}

// ── helpers (local copies; kept local to avoid cross-module coupling) ──

fn map_err<T, E: std::fmt::Display>(r: Result<T, E>, ctx: &str) -> Result<T, String> {
    r.map_err(|e| format!("{}: {}", ctx, e))
}

fn parse_usize_arg(v: &Value, layer: &str, name: &str) -> Result<usize, String> {
    match v {
        Value::Float(n) => Ok(*n as usize),
        Value::String(s) => s.parse::<usize>().map_err(|_| {
            format!(
                "{}: {} must be a positive integer, got '{}'",
                layer, name, s
            )
        }),
        other => Err(format!(
            "{}: {} must be a number, got {}",
            layer,
            name,
            other.type_name()
        )),
    }
}

fn parse_f64_arg(v: &Value, layer: &str, name: &str) -> Result<f64, String> {
    match v {
        Value::Float(n) => Ok(*n),
        Value::String(s) => s
            .parse::<f64>()
            .map_err(|_| format!("{}: {} must be a number, got '{}'", layer, name, s)),
        other => Err(format!(
            "{}: {} must be a number, got {}",
            layer,
            name,
            other.type_name()
        )),
    }
}
