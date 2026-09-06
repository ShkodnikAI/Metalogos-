//! Trainable attention — `Var`-based variant of `Attention` (Наряд №185).
//!
//! Per the naryad spec: "использовать candle's autograd напрямую, не
//! реализовывать backward вручную, ADR-0118". The existing `Attention`
//! from Наряд №183 uses plain `Tensor` weights (no grad tracking) — it
//! cannot participate in `Tensor::backward()`. Rather than refactor
//! `Attention` (which would risk breaking Наряд №183's numerical
//! contracts), this module provides a parallel `TrainableAttention`
//! struct that uses candle's `Var` for weights — fully autograd-tracked.
//!
//! ## Why a separate struct (not refactor Attention)
//!
//! Per the spec: "Не трогать существующую ветку" — symmetric to Наряд
//! №182/184. The existing `Attention` forward-pass contracts (tests
//! naryad_183_attention_forward.rs) verify specific numerical outputs;
//! changing the weight type from `Tensor` to `Var` is semantically the
//! same for forward, but ANY refactor risk to those tests is
//! unjustified when a parallel struct costs nothing.
//!
//! ## Algorithm
//!
//! Identical to `Attention` (Наряд №183):
//!   - Q, K, V projections: `hidden_dim → hidden_dim` (no bias, matching Llama)
//!   - RoPE applied to Q and K (NOT V)
//!   - Attention scores: `softmax(Q @ K^T / sqrt(head_dim))`
//!   - Output projection: `hidden_dim → hidden_dim` (no bias)
//!
//! ## Determinism
//!
//! Weights initialized via the project's xorshift64 PRNG (Наряд №177),
//! then converted to `Var`s via `Var::from_tensor`. The vars are
//! inserted into the VarMap manually so `backward()` finds them.
//! Determinism is preserved: same seed → same weights → same forward
//! result → same training trajectory (assuming deterministic matmul
//! on CPU, which candle provides).

#![cfg(feature = "candle")]

use crate::interpreter::Value;
use crate::nn::attention::generate_uniform_f32;
use crate::nn::sequence_layer::SequenceLayer;

use candle_core::{DType, Device, Tensor, Var};

/// Multi-head self-attention with RoPE — `Var`-based (autograd-tracked).
///
/// Same algorithm as `Attention` (Наряд №183), but weights are `Var`s
/// registered in a `VarMap` via `VarBuilder`. This enables
/// `Tensor::backward()` to populate gradients for SGD training.
pub struct TrainableAttention {
    heads: usize,
    n_kv_heads: usize,
    dim: usize,
    head_dim: usize,
    w_q: Var,
    w_k: Var,
    w_v: Var,
    w_o: Var,
    rope_theta: f64,
}

impl TrainableAttention {
    /// Construct standard MHA (backward compat, Наряд №185).
    ///
    /// Equivalent to `new_with_kv_heads(heads, heads, dim, seed, var_map, "attn")`.
    /// Uses the default prefix "attn" — fine for single-layer models.
    /// For **stacks** of attention/transformer_block layers, use the
    /// `with_prefix` variant (Наряд №190) to avoid VarMap name collisions.
    pub fn new(
        heads: usize,
        dim: usize,
        seed: u64,
        var_map: &candle_nn::VarMap,
    ) -> Result<Self, String> {
        Self::new_with_kv_heads(heads, heads, dim, seed, var_map, "attn")
    }

    /// Construct with GQA and a custom VarMap prefix (Наряд №188 + №190).
    ///
    /// The `prefix` parameter makes each layer in a stack register its
    /// weights under unique names (e.g. "block0_attn_w_q", "block1_attn_w_q").
    /// Without this, a stack of N transformer_blocks would all write to
    /// "attn_w_q" and overwrite each other — bug found by Наряд №190.
    pub fn new_with_kv_heads(
        heads: usize,
        n_kv_heads: usize,
        dim: usize,
        seed: u64,
        var_map: &candle_nn::VarMap,
        prefix: &str,
    ) -> Result<Self, String> {
        if heads == 0 {
            return Err("trainable_attention: heads must be > 0".to_string());
        }
        if n_kv_heads == 0 {
            return Err("trainable_attention: n_kv_heads must be > 0".to_string());
        }
        if n_kv_heads > heads {
            return Err(format!(
                "trainable_attention: n_kv_heads ({}) must be <= n_heads ({})",
                n_kv_heads, heads
            ));
        }
        if !dim.is_multiple_of(heads) {
            return Err(format!(
                "trainable_attention: dim ({}) must be divisible by heads ({})",
                dim, heads
            ));
        }
        if !heads.is_multiple_of(n_kv_heads) {
            return Err(format!(
                "trainable_attention: n_heads ({}) must be divisible by n_kv_heads ({})",
                heads, n_kv_heads
            ));
        }
        let head_dim = dim / heads;
        let kv_dim = n_kv_heads * head_dim;
        let bound = 1.0 / (dim as f64).sqrt();
        let device = Device::Cpu;

        // Weight layout: Q [dim,dim], K [dim,kv_dim], V [dim,kv_dim], O [dim,dim].
        // When n_kv_heads == n_heads → kv_dim == dim → total == 4*dim*dim
        // (backward compatible with Наряд №185).
        let total = 2 * dim * dim + 2 * dim * kv_dim;
        let weights = generate_uniform_f32(seed, total, -bound, bound);

        let slice_q = &weights[0..dim * dim];
        let slice_k = &weights[dim * dim..dim * dim + dim * kv_dim];
        let slice_v = &weights[dim * dim + dim * kv_dim..dim * dim + 2 * dim * kv_dim];
        let slice_o = &weights[dim * dim + 2 * dim * kv_dim..];

        // Наряд №190: include prefix in VarMap names so stacked layers don't
        // collide. Format: "{prefix}_w_q", "{prefix}_w_k", etc.
        let make_var =
            |slice: &[f32], suffix: &str, shape: (usize, usize)| -> Result<Var, String> {
                let name = format!("{}_{}", prefix, suffix);
                let tensor = Tensor::from_slice(slice, shape, &device)
                    .and_then(|t| t.to_dtype(DType::F32))
                    .map_err(|e| format!("trainable_attn {}: tensor: {}", name, e))?;
                let var = Var::from_tensor(&tensor)
                    .map_err(|e| format!("trainable_attn {}: from_tensor: {}", name, e))?;
                let mut guard = var_map
                    .data()
                    .lock()
                    .map_err(|e| format!("trainable_attn {}: lock: {}", name, e))?;
                guard.insert(name, var.clone());
                Ok(var)
            };

        let w_q = make_var(slice_q, "w_q", (dim, dim))?;
        let w_k = make_var(slice_k, "w_k", (dim, kv_dim))?;
        let w_v = make_var(slice_v, "w_v", (dim, kv_dim))?;
        let w_o = make_var(slice_o, "w_o", (dim, dim))?;

        Ok(Self {
            heads,
            n_kv_heads,
            dim,
            head_dim,
            w_q,
            w_k,
            w_v,
            w_o,
            rope_theta: 10000.0,
        })
    }

    fn forward_impl(&self, input: &Tensor) -> Result<Tensor, String> {
        let (seq_len, _in_dim) = input
            .dims2()
            .map_err(|e| format!("trainable_attn dims: {}", e))?;
        let device = input.device();

        // Q, K, V projections — Var as_tensor() returns the underlying
        // Tensor (which is in the autograd graph because it came from
        // a Var). Q: [seq, dim], K/V: [seq, kv_dim].
        let q = input
            .matmul(self.w_q.as_tensor())
            .map_err(|e| format!("trainable_attn Q matmul: {}", e))?;
        let k = input
            .matmul(self.w_k.as_tensor())
            .map_err(|e| format!("trainable_attn K matmul: {}", e))?;
        let v = input
            .matmul(self.w_v.as_tensor())
            .map_err(|e| format!("trainable_attn V matmul: {}", e))?;

        // RoPE on Q (n_heads) and K (n_kv_heads)
        let q = self.apply_rope(&q, seq_len, self.heads, device)?;
        let k = self.apply_rope(&k, seq_len, self.n_kv_heads, device)?;

        // Reshape to [heads, seq, head_dim] (Q) / [n_kv_heads, seq, head_dim] (K/V)
        let q = q
            .reshape((seq_len, self.heads, self.head_dim))
            .and_then(|t| t.transpose(0, 1))
            .map_err(|e| format!("trainable_attn Q reshape: {}", e))?;
        let k = k
            .reshape((seq_len, self.n_kv_heads, self.head_dim))
            .and_then(|t| t.transpose(0, 1))
            .map_err(|e| format!("trainable_attn K reshape: {}", e))?;
        let v = v
            .reshape((seq_len, self.n_kv_heads, self.head_dim))
            .and_then(|t| t.transpose(0, 1))
            .map_err(|e| format!("trainable_attn V reshape: {}", e))?;

        // GQA: repeat K, V to match Q's head count.
        let k = self.repeat_kv(&k)?;
        let v = self.repeat_kv(&v)?;

        // Attention scores = Q @ K^T / sqrt(head_dim)
        let k_t = k
            .transpose(1, 2)
            .map_err(|e| format!("trainable_attn K^T: {}", e))?;
        let scale = 1.0 / (self.head_dim as f64).sqrt();
        let scale_tensor = Tensor::new(scale as f32, device)
            .map_err(|e| format!("trainable_attn scale: {}", e))?;
        let scores = q
            .matmul(&k_t)
            .map_err(|e| format!("trainable_attn Q@K^T: {}", e))?;
        let scores = scores
            .broadcast_mul(&scale_tensor)
            .map_err(|e| format!("trainable_attn scale: {}", e))?;

        // softmax
        let attn = candle_nn::ops::softmax(&scores, candle_core::D::Minus1)
            .map_err(|e| format!("trainable_attn softmax: {}", e))?;

        // attn @ V → [heads, seq, head_dim]
        let out = attn
            .matmul(&v)
            .map_err(|e| format!("trainable_attn attn@V: {}", e))?;

        // Reshape back to [seq, dim]
        let out = out
            .transpose(0, 1)
            .map_err(|e| format!("trainable_attn out transpose: {}", e))?
            .reshape((seq_len, self.dim))
            .map_err(|e| format!("trainable_attn out reshape: {}", e))?;

        // Output projection
        out.matmul(self.w_o.as_tensor())
            .map_err(|e| format!("trainable_attn out proj: {}", e))
    }

    /// GQA: repeat KV heads along head axis (Наряд №188).
    /// Same as `Attention::repeat_kv` but for TrainableAttention.
    fn repeat_kv(&self, x: &Tensor) -> Result<Tensor, String> {
        let n_rep = self.heads / self.n_kv_heads;
        if n_rep == 1 {
            return Ok(x.clone());
        }
        let dims = x.dims();
        let n_kv = dims[0];
        let seq_len = dims[1];
        let head_dim = dims[2];

        let x = x
            .unsqueeze(1)
            .map_err(|e| format!("trainable_gqa unsqueeze: {}", e))?;
        let x = x
            .expand((n_kv, n_rep, seq_len, head_dim))
            .map_err(|e| format!("trainable_gqa expand: {}", e))?;
        x.reshape((n_kv * n_rep, seq_len, head_dim))
            .map_err(|e| format!("trainable_gqa reshape: {}", e))
    }

    /// Apply RoPE — same algorithm as `Attention::apply_rope` (Наряд №183).
    /// Updated in Наряд №188 to accept `n_h` parameter (K has fewer heads in GQA).
    fn apply_rope(
        &self,
        x: &Tensor,
        seq_len: usize,
        n_h: usize,
        device: &Device,
    ) -> Result<Tensor, String> {
        let dtype = x.dtype();
        let head_dim = self.head_dim;
        if !head_dim.is_multiple_of(2) {
            return Err(format!(
                "trainable_attn: head_dim ({}) must be even for RoPE",
                head_dim
            ));
        }

        let half = head_dim / 2;
        let inv_freq: Vec<f32> = (0..half)
            .map(|i| {
                let exponent = 2.0 * (i as f64) / (head_dim as f64);
                (1.0 / self.rope_theta.powf(exponent)) as f32
            })
            .collect();

        let angles: Vec<f32> = (0..seq_len)
            .flat_map(|p| {
                let pos = p as f32;
                inv_freq
                    .iter()
                    .map(move |&freq| pos * freq)
                    .collect::<Vec<_>>()
            })
            .collect();

        let angles = Tensor::from_slice(&angles, (seq_len, half), device)
            .map_err(|e| format!("trainable_rope angles: {}", e))?;
        let angles = angles
            .to_dtype(dtype)
            .map_err(|e| format!("trainable_rope angles dtype: {}", e))?;
        let cos = angles
            .cos()
            .map_err(|e| format!("trainable_rope cos: {}", e))?;
        let sin = angles
            .sin()
            .map_err(|e| format!("trainable_rope sin: {}", e))?;

        let x_reshaped = x
            .reshape((seq_len, n_h, head_dim))
            .map_err(|e| format!("trainable_rope reshape: {}", e))?;

        let x_first = x_reshaped
            .narrow(2, 0, half)
            .map_err(|e| format!("trainable_rope narrow first: {}", e))?;
        let x_second = x_reshaped
            .narrow(2, half, half)
            .map_err(|e| format!("trainable_rope narrow second: {}", e))?;

        let cos = cos
            .unsqueeze(1)
            .map_err(|e| format!("trainable_rope cos unsqueeze: {}", e))?;
        let sin = sin
            .unsqueeze(1)
            .map_err(|e| format!("trainable_rope sin unsqueeze: {}", e))?;

        let x_first_c = x_first
            .broadcast_mul(&cos)
            .map_err(|e| format!("trainable_rope x_first*cos: {}", e))?;
        let x_first_s = x_first
            .broadcast_mul(&sin)
            .map_err(|e| format!("trainable_rope x_first*sin: {}", e))?;
        let x_second_c = x_second
            .broadcast_mul(&cos)
            .map_err(|e| format!("trainable_rope x_second*cos: {}", e))?;
        let x_second_s = x_second
            .broadcast_mul(&sin)
            .map_err(|e| format!("trainable_rope x_second*sin: {}", e))?;

        let x_first_new =
            (x_first_c - &x_second_s).map_err(|e| format!("trainable_rope first_new: {}", e))?;
        let x_second_new =
            (x_first_s + &x_second_c).map_err(|e| format!("trainable_rope second_new: {}", e))?;

        let rotated = Tensor::cat(&[&x_first_new, &x_second_new], 2)
            .map_err(|e| format!("trainable_rope concat: {}", e))?;
        rotated
            .reshape((seq_len, n_h * head_dim))
            .map_err(|e| format!("trainable_rope reshape back: {}", e))
    }
}

impl SequenceLayer for TrainableAttention {
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
        "trainable_attention"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Build function — accepts same args as `build_attention` (Наряд №188:
/// heads, dim, [kv_heads]). Takes the `VarMap` (not VarBuilder) so it can
/// register the Vars manually with deterministic init values.
///
/// Наряд №190: `prefix` parameter makes each layer in a stack register
/// its weights under unique VarMap names (avoids collision).
pub fn build_trainable_attention(
    args: &[Value],
    seed: u64,
    var_map: &candle_nn::VarMap,
    prefix: &str,
) -> Result<Box<dyn SequenceLayer>, String> {
    if args.len() != 2 && args.len() != 3 {
        return Err(format!(
            "trainable_attention: expected 2 args (heads, dim) or 3 args (heads, dim, kv_heads), got {}",
            args.len()
        ));
    }
    let heads = match &args[0] {
        Value::Float(n) => *n as usize,
        Value::String(s) => s
            .parse::<usize>()
            .map_err(|_| format!("trainable_attention: heads must be integer, got '{}'", s))?,
        other => {
            return Err(format!(
                "trainable_attention: heads must be a number, got {}",
                other.type_name()
            ))
        }
    };
    let dim = match &args[1] {
        Value::Float(n) => *n as usize,
        Value::String(s) => s
            .parse::<usize>()
            .map_err(|_| format!("trainable_attention: dim must be integer, got '{}'", s))?,
        other => {
            return Err(format!(
                "trainable_attention: dim must be a number, got {}",
                other.type_name()
            ))
        }
    };
    // Наряд №188: optional 3rd arg — n_kv_heads for GQA.
    let n_kv_heads = if args.len() == 3 {
        match &args[2] {
            Value::Float(n) => *n as usize,
            Value::String(s) => s.parse::<usize>().map_err(|_| {
                format!("trainable_attention: kv_heads must be integer, got '{}'", s)
            })?,
            other => {
                return Err(format!(
                    "trainable_attention: kv_heads must be a number, got {}",
                    other.type_name()
                ))
            }
        }
    } else {
        heads
    };
    let attn =
        TrainableAttention::new_with_kv_heads(heads, n_kv_heads, dim, seed, var_map, prefix)?;
    Ok(Box::new(attn))
}
