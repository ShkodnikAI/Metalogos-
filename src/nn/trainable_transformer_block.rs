//! Trainable transformer block — Var-based variant of TransformerBlock.
//!
//! Наряд №185 follow-up: after merging Наряд №184 (TransformerBlock)
//! with Наряд №185 (training), `construct_reflex_seq_model` needs to
//! support `transformer_block` layers, not just `attention`. This
//! module provides a `TrainableTransformerBlock` that mirrors
//! `TransformerBlock` (Наряд №184) but uses `Var`-based weights so
//! `backward()` populates gradients.
//!
//! ## Why a separate struct (not refactor TransformerBlock)
//!
//! Same discipline as Наряд №185's `TrainableAttention` vs `Attention`:
//! the existing `TransformerBlock` from Наряд №184 has numerical
//! contracts (naryad_184_transformer_block_forward.rs) that verify
//! specific outputs. Refactoring it to use `Var` would risk breaking
//! those contracts. A parallel struct costs nothing.
//!
//! ## Composition
//!
//! Like `TransformerBlock` (Наряд №184), this composes:
//!   - TrainableAttention (Наряд №185) — Var-based
//!   - RmsNorm (Наряд №184) — uses Tensor, NOT Var (no autograd for
//!     norm weights; acceptable because RmsNorm's weight is typically
//!     not the dominant learning signal — most gradient flows through
//!     attention and FFN)
//!   - SwiGLU (Наряд №184) — uses Tensor, NOT Var (same reason)
//!   - RmsNorm (Наряд №184)
//!
//! The pre-norm + residual structure is identical to TransformerBlock.
//!
//! ## What this means for training
//!
//! Only the attention weights (Q/K/V/O) and the classifier weights are
//! updated by SGD. RmsNorm and SwiGLU weights stay at their initial
//! values. This is a partial training — sufficient for the contract
//! (model can be trained, accuracy improves), but not full end-to-end
//! training of all transformer parameters. Full trainable RmsNorm/SwiGLU
//! is a future naryad.

#![cfg(feature = "candle")]

use crate::interpreter::Value;
use crate::nn::rmsnorm::RmsNorm;
use crate::nn::sequence_layer::SequenceLayer;
use crate::nn::swiglu::SwiGlu;
use crate::nn::trainable_attention::TrainableAttention;

use candle_core::Tensor;

/// Trainable transformer block — pre-norm + residual, Var-based attention.
///
/// See module docs for the rationale and the partial-training caveat.
pub struct TrainableTransformerBlock {
    attention: TrainableAttention,
    norm1: RmsNorm,
    ffn: SwiGlu,
    norm2: RmsNorm,
}

impl TrainableTransformerBlock {
    pub fn new(
        heads: usize,
        dim: usize,
        ff_dim: usize,
        seed: u64,
        var_map: &candle_nn::VarMap,
        prefix: &str,
    ) -> Result<Self, String> {
        // Наряд №190: pass prefix to TrainableAttention so each block in a
        // stack registers under unique VarMap names ("block0_attn_w_q", etc.).
        // Without this, stacked blocks overwrite each other — bug fixed by this naryad.
        let attention = TrainableAttention::new_with_kv_heads(
            heads,
            heads, // standard MHA inside transformer_block (GQA opt-in only standalone)
            dim,
            seed,
            var_map,
            &format!("{}_attn", prefix),
        )?;
        // RmsNorm with ones-init (Наряд №184 default; seed has no effect).
        let norm1 = RmsNorm::new(dim, seed ^ 0x4E44, 1e-6)?;
        // SwiGLU with seed XOR offset (matches Наряд №184's offset).
        let ffn = SwiGlu::new(dim, ff_dim, seed ^ 0x576F)?;
        let norm2 = RmsNorm::new(dim, seed ^ 0x4E45, 1e-6)?;

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
        let x_after_attn = x
            .broadcast_add(&attn_out)
            .map_err(|e| format!("trainable_tb residual1: {}", e))?;

        // Pre-norm + residual: y = x + ffn(norm2(x))
        let h2 = self.norm2.forward(&x_after_attn)?;
        let ffn_out = self.ffn.forward(&h2)?;
        x_after_attn
            .broadcast_add(&ffn_out)
            .map_err(|e| format!("trainable_tb residual2: {}", e))
    }

    /// KV-cache forward for a SINGLE position (Наряд №193).
    ///
    /// Delegates to `TrainableAttention::forward_step` for the attention
    /// layer, and uses regular `forward` for norms/FFN (which are
    /// position-independent — they process each position independently).
    pub fn forward_step(
        &self,
        x: &Tensor,
        position: usize,
        k_cache: &mut Option<Tensor>,
        v_cache: &mut Option<Tensor>,
    ) -> Result<Tensor, String> {
        // Pre-norm + residual: y = x + attn(norm1(x))
        let h1 = self.norm1.forward(x)?;
        let attn_out = self
            .attention
            .forward_step(&h1, position, k_cache, v_cache)?;
        let x_after_attn = x
            .broadcast_add(&attn_out)
            .map_err(|e| format!("tb_step residual1: {}", e))?;

        // Pre-norm + residual: y = x + ffn(norm2(x))
        let h2 = self.norm2.forward(&x_after_attn)?;
        let ffn_out = self.ffn.forward(&h2)?;
        x_after_attn
            .broadcast_add(&ffn_out)
            .map_err(|e| format!("tb_step residual2: {}", e))
    }
}

impl SequenceLayer for TrainableTransformerBlock {
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
        "trainable_transformer_block"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Build function — accepts same args as `build_transformer_block`
/// (heads, dim, ff_dim) from Наряд №184.
///
/// Наряд №190: `prefix` parameter makes each block in a stack register
/// its weights under unique VarMap names.
pub fn build_trainable_transformer_block(
    args: &[Value],
    seed: u64,
    var_map: &candle_nn::VarMap,
    prefix: &str,
) -> Result<Box<dyn SequenceLayer>, String> {
    if args.len() != 3 {
        return Err(format!(
            "trainable_transformer_block: expected 3 args (heads, dim, ff_dim), got {}",
            args.len()
        ));
    }
    let heads = match &args[0] {
        Value::Float(n) => *n as usize,
        Value::String(s) => s
            .parse::<usize>()
            .map_err(|_| format!("trainable_tb: heads must be integer, got '{}'", s))?,
        other => {
            return Err(format!(
                "trainable_tb: heads must be a number, got {}",
                other.type_name()
            ))
        }
    };
    let dim = match &args[1] {
        Value::Float(n) => *n as usize,
        Value::String(s) => s
            .parse::<usize>()
            .map_err(|_| format!("trainable_tb: dim must be integer, got '{}'", s))?,
        other => {
            return Err(format!(
                "trainable_tb: dim must be a number, got {}",
                other.type_name()
            ))
        }
    };
    let ff_dim = match &args[2] {
        Value::Float(n) => *n as usize,
        Value::String(s) => s
            .parse::<usize>()
            .map_err(|_| format!("trainable_tb: ff_dim must be integer, got '{}'", s))?,
        other => {
            return Err(format!(
                "trainable_tb: ff_dim must be a number, got {}",
                other.type_name()
            ))
        }
    };
    let block = TrainableTransformerBlock::new(heads, dim, ff_dim, seed, var_map, prefix)?;
    Ok(Box::new(block))
}
