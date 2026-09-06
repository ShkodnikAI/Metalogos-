//! Full transformer block — composite `SequenceLayer` (Наряд №184, Block 3).
//!
//! Implements the standard pre-norm transformer block structure used in
//! modern LLMs (Llama, Mistral, etc.), as documented in Наряд №176's
//! reference analysis of `candle-transformers` Llama:
//!
//! ```text
//! x → + (attention(norm1(x)))  // pre-norm + residual
//!   → + (ffn(norm2(x)))        // pre-norm + residual
//! ```
//!
//! ## Why pre-norm (not post-norm)
//!
//! Per the naryad spec: "точный порядок (pre-norm, не post-norm) — по
//! референсу наряда №176, не по интуиции."
//!
//! Pre-norm applies normalization BEFORE the sublayer (attention/FFN),
//! then adds the residual. This is more stable for training deep models
//! than post-norm (Vaswani 2017's original), where the residual is
//! added before normalization. Llama, GPT-3, and essentially all modern
//! transformers use pre-norm.
//!
//! ## Composite layer pattern
//!
//! TransformerBlock is itself a `SequenceLayer` — it composes four
//! inner `SequenceLayer`s (Attention + 2x RmsNorm + SwiGLU) into a
//! single layer that can be placed in `reflex_seq`'s `layers: [...]`
//! list. This is the registry pattern's composition use-case: a layer
//! that contains other layers, transparent to the layer list.
//!
//! ## Weight init
//!
//! Each sub-layer receives the model-level `seed` PLUS a per-position
//! offset so their weights are independent (otherwise attention's RNG
//! stream would be consumed by RmsNorm's "ones" init, then SwiGLU
//! would start from the same point as attention). Offsets are derived
//! from the block's position in the model, NOT from a separate user-
//! visible parameter — the user writes `transformer_block(heads, dim, ff_dim)`
//! and the layer handles internal seeding.

#![cfg(feature = "candle")]

use crate::interpreter::Value;
use crate::nn::sequence_layer::SequenceLayer;

use candle_core::Tensor;

/// Full pre-norm transformer block: Attention → Residual → RmsNorm →
/// SwiGLU → Residual.
///
/// Composes four inner SequenceLayers:
///   - `attention`: multi-head self-attention with RoPE (Наряд №183)
///   - `norm1`: RmsNorm before attention
///   - `swiglu`: gated feedforward (Наряд №184, Block 2)
///   - `norm2`: RmsNorm before FFN
pub struct TransformerBlock {
    attention: crate::nn::attention::Attention,
    norm1: crate::nn::rmsnorm::RmsNorm,
    ffn: crate::nn::swiglu::SwiGlu,
    norm2: crate::nn::rmsnorm::RmsNorm,
}

impl TransformerBlock {
    /// Construct a transformer block.
    ///
    /// `seed` is the model-level seed. Each sub-layer receives a
    /// derived seed so their weights are independent:
    ///   - attention: `seed`           (matches Наряд №183 if used standalone)
    ///   - norm1:     `seed ^ 0x4E44`  (arbitrary offset, just needs to differ)
    ///   - swiglu:    `seed ^ 0x576F`  (different offset)
    ///   - norm2:     `seed ^ 0x4E45`  (different from norm1)
    ///
    /// `norm1` and `norm2` use `RmsNorm::new` with ones-init — the
    /// seed has no effect on them (only the eps matters), but the API
    /// requires it for uniformity. The offsets are documented here so
    /// tests can reproduce the exact weight stream.
    pub fn new(heads: usize, dim: usize, ff_dim: usize, seed: u64) -> Result<Self, String> {
        if heads == 0 {
            return Err("transformer_block: heads must be > 0".to_string());
        }
        if !dim.is_multiple_of(heads) {
            return Err(format!(
                "transformer_block: dim ({}) must be divisible by heads ({})",
                dim, heads
            ));
        }
        if ff_dim == 0 {
            return Err("transformer_block: ff_dim must be > 0".to_string());
        }

        let attention = crate::nn::attention::Attention::new(heads, dim, seed)?;
        // RmsNorm with ones-init: seed is accepted but unused (init is deterministic)
        let norm1 = crate::nn::rmsnorm::RmsNorm::new(dim, seed ^ 0x4E44, 1e-6)?;
        let ffn = crate::nn::swiglu::SwiGlu::new(dim, ff_dim, seed ^ 0x576F)?;
        let norm2 = crate::nn::rmsnorm::RmsNorm::new(dim, seed ^ 0x4E45, 1e-6)?;

        Ok(Self {
            attention,
            norm1,
            ffn,
            norm2,
        })
    }

    fn forward_impl(&self, x: &Tensor) -> Result<Tensor, String> {
        // Pre-norm + residual: y = x + attn(norm1(x))
        let h1 = self.norm1.forward(x)?;
        let attn_out = self.attention.forward(&h1)?;
        let x_after_attn = map_err(x.broadcast_add(&attn_out), "transformer_block: residual1")?;

        // Pre-norm + residual: y = x + ffn(norm2(x))
        let h2 = self.norm2.forward(&x_after_attn)?;
        let ffn_out = self.ffn.forward(&h2)?;
        map_err(
            x_after_attn.broadcast_add(&ffn_out),
            "transformer_block: residual2",
        )
    }
}

// Local map_err (same as in rmsnorm.rs / swiglu.rs).
fn map_err<T, E: std::fmt::Display>(r: Result<T, E>, ctx: &str) -> Result<T, String> {
    r.map_err(|e| format!("{}: {}", ctx, e))
}

impl SequenceLayer for TransformerBlock {
    fn forward(&self, input: &Tensor) -> Result<Tensor, String> {
        self.forward_impl(input)
    }

    fn input_dim(&self) -> usize {
        self.attention.input_dim()
    }

    fn output_dim(&self) -> usize {
        self.attention.output_dim()
    }

    fn name(&self) -> &str {
        "transformer_block"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Build function for the SEQUENCE_LAYER_REGISTRY.
///
/// Args: `(heads, dim, ff_dim)`. All three are required — no defaults.
///
/// Matches the naryad spec's example:
/// ```text
/// reflex_seq TinyTransformer {
///   input: embedding(64)
///   seq_len: 16
///   layers: [transformer_block(4, 64, 256)]
///   seed: 42
/// }
/// ```
pub fn build_transformer_block(
    args: &[Value],
    seed: u64,
) -> Result<Box<dyn SequenceLayer>, String> {
    if args.len() != 3 {
        return Err(format!(
            "transformer_block: expected 3 args (heads, dim, ff_dim), got {}",
            args.len()
        ));
    }
    let heads = parse_usize_arg(&args[0], "transformer_block", "heads")?;
    let dim = parse_usize_arg(&args[1], "transformer_block", "dim")?;
    let ff_dim = parse_usize_arg(&args[2], "transformer_block", "ff_dim")?;
    let layer = TransformerBlock::new(heads, dim, ff_dim, seed)?;
    Ok(Box::new(layer))
}

// ── helpers (local; mirror rmsnorm.rs / swiglu.rs) ─────────────────────

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
