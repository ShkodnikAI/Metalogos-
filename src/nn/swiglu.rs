//! SwiGLU feedforward — third `SequenceLayer` (Наряд №184, Block 2).
//!
//! Implements the SwiGLU (SiLU-Gated Linear Unit) feedforward block
//! used in modern transformer architectures (Llama, PaLM). Documented
//! in Наряд №176's reference analysis of `candle-transformers` Llama.
//!
//! ## Formula
//!
//! ```text
//! SwiGLU(x) = (silu(x @ W_gate) * (x @ W_up)) @ W_down
//! ```
//!
//! where `silu(z) = z * sigmoid(z)` (also called Swish).
//!
//! ## Why SwiGLU (not vanilla MLP)
//!
//! The classic transformer FFN (Vaswani 2017) is `ReLU(x @ W1) @ W2`.
//! GLU variants (Dauphin 2017, Shazeer 2020) add a gating mechanism:
//! the output is the elementwise product of two parallel projections.
//! SwiGLU uses SiLU as the activation in the gated branch.
//!
//! Empirically, SwiGLU outperforms ReLU/GeLU FFNs at equal parameter
//! count (Shazeer 2020). Llama, Mistral, and PaLM all use SwiGLU.
//! The reference analysis in Наряд №176 confirmed Llama uses exactly
//! SwiGLU as the FFN in every transformer block.
//!
//! ## Parameter count vs classic FFN
//!
//! Classic FFN: `2 * dim * ff_dim` (W1, W2).
//! SwiGLU: `3 * dim * ff_dim` (W_gate, W_up, W_down) — 50% more
//! parameters for the same `ff_dim`. Llama keeps the same `ff_dim`
//! as the classic baseline (4 * dim typically), making SwiGLU 50%
//! heavier than vanilla ReLU FFN — accepted as the cost of the
//! quality improvement.
//!
//! ## Weight init
//!
//! Same `xorshift64` PRNG (Наряд №177) as Attention/RmsNorm —
//! deterministic per `seed`. Weights drawn from uniform `[-1/sqrt(dim),
//! 1/sqrt(dim)]` (matches Attention's init scale, Наряд №183).

#![cfg(feature = "candle")]

use crate::interpreter::Value;
use crate::nn::sequence_layer::SequenceLayer;
// Reuse the project's PRNG (kept in attention.rs as the canonical home for
// `generate_uniform_f32`; documented public there in Наряд №184).
use crate::nn::attention::generate_uniform_f32;

use candle_core::{DType, Device, Tensor};

/// SwiGLU feedforward block.
///
/// Operates on `[seq_len, dim]` tensors → `[seq_len, dim]` output
/// (residual stream preserved, per the transformer block contract).
pub struct SwiGlu {
    /// Input dimension (residual stream width).
    dim: usize,
    /// Hidden dimension (typically 4 * dim, but configurable per
    /// `reflex_seq` declaration). Stored for debugging and shape
    /// validation, not used in the forward pass.
    #[allow(dead_code)]
    ff_dim: usize,
    /// Gate projection `[dim, ff_dim]`.
    w_gate: Tensor,
    /// Up projection `[dim, ff_dim]`.
    w_up: Tensor,
    /// Down projection `[ff_dim, dim]`.
    w_down: Tensor,
}

impl SwiGlu {
    /// Construct a SwiGLU block with weights initialized from `seed`.
    ///
    /// Weights are drawn from a uniform distribution
    /// `[-1/sqrt(dim), 1/sqrt(dim)]` using the project's `xorshift64` PRNG
    /// (наряд №177). Same `seed` → same weights → same forward-pass result.
    ///
    /// The three weight matrices are laid out in the RNG stream as:
    ///   1. W_gate `[dim, ff_dim]`
    ///   2. W_up `[dim, ff_dim]`
    ///   3. W_down `[ff_dim, dim]`
    pub fn new(dim: usize, ff_dim: usize, seed: u64) -> Result<Self, String> {
        if dim == 0 {
            return Err("swiglu: dim must be > 0".to_string());
        }
        if ff_dim == 0 {
            return Err("swiglu: ff_dim must be > 0".to_string());
        }

        let bound = 1.0 / (dim as f64).sqrt();
        let total = 3 * dim * ff_dim;
        let weights = generate_uniform_f32(seed, total, -bound, bound);

        let slice_gate = &weights[0..dim * ff_dim];
        let slice_up = &weights[dim * ff_dim..2 * dim * ff_dim];
        let slice_down = &weights[2 * dim * ff_dim..3 * dim * ff_dim];

        let device = Device::Cpu;

        let w_gate = map_err(
            Tensor::from_slice(slice_gate, (dim, ff_dim), &device)
                .and_then(|t| t.to_dtype(DType::F32)),
            "swiglu: w_gate init",
        )?;
        let w_up = map_err(
            Tensor::from_slice(slice_up, (dim, ff_dim), &device)
                .and_then(|t| t.to_dtype(DType::F32)),
            "swiglu: w_up init",
        )?;
        let w_down = map_err(
            Tensor::from_slice(slice_down, (ff_dim, dim), &device)
                .and_then(|t| t.to_dtype(DType::F32)),
            "swiglu: w_down init",
        )?;

        Ok(Self {
            dim,
            ff_dim,
            w_gate,
            w_up,
            w_down,
        })
    }

    fn forward_impl(&self, input: &Tensor) -> Result<Tensor, String> {
        let (_seq_len, in_dim) = map_err(input.dims2(), "swiglu: input dims2")?;
        if in_dim != self.dim {
            return Err(format!(
                "swiglu: input dim {} != layer dim {}",
                in_dim, self.dim
            ));
        }

        // gate = x @ W_gate  → [seq, ff_dim]
        let gate = map_err(input.matmul(&self.w_gate), "swiglu: gate matmul")?;
        // up = x @ W_up      → [seq, ff_dim]
        let up = map_err(input.matmul(&self.w_up), "swiglu: up matmul")?;

        // silu(gate) = gate * sigmoid(gate)
        // candle_nn::ops::sigmoid returns f32 if input is f32 — we use
        // input's dtype (F32 by default on CPU).
        let sigmoid_gate = map_err(candle_nn::ops::sigmoid(&gate), "swiglu: sigmoid(gate)")?;
        let silu_gate = map_err(gate.broadcast_mul(&sigmoid_gate), "swiglu: silu(gate)")?;

        // gated = silu(gate) * up  → [seq, ff_dim]
        let gated = map_err(silu_gate.broadcast_mul(&up), "swiglu: gated mul")?;

        // out = gated @ W_down  → [seq, dim]
        map_err(gated.matmul(&self.w_down), "swiglu: down matmul")
    }
}

impl SequenceLayer for SwiGlu {
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
        "swiglu"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Build function for the SEQUENCE_LAYER_REGISTRY.
///
/// Args: `(dim, ff_dim)`. Both are required — no defaults, to keep
/// the declaration explicit (consistent with Attention's `(heads, dim)`
/// API from Наряд №183).
pub fn build_swiglu(args: &[Value], seed: u64) -> Result<Box<dyn SequenceLayer>, String> {
    if args.len() != 2 {
        return Err(format!(
            "swiglu: expected 2 args (dim, ff_dim), got {}",
            args.len()
        ));
    }
    let dim = parse_usize_arg(&args[0], "swiglu", "dim")?;
    let ff_dim = parse_usize_arg(&args[1], "swiglu", "ff_dim")?;
    let layer = SwiGlu::new(dim, ff_dim, seed)?;
    Ok(Box::new(layer))
}

// ── helpers (local; mirror rmsnorm.rs's local copies) ─────────────────

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
