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
    dim: usize,
    head_dim: usize,
    w_q: Var,
    w_k: Var,
    w_v: Var,
    w_o: Var,
    rope_theta: f64,
}

impl TrainableAttention {
    /// Construct with weights drawn from xorshift64 PRNG (deterministic),
    /// then inserted into the provided VarMap via `set_one`.
    ///
    /// The weights are NOT registered through `VarBuilder::get` (which
    /// would use candle's non-seedable RNG on CPU). Instead, we create
    /// them manually as Tensors, convert to Var, and store in the VarMap
    /// under well-known names ("attn_w_q", "attn_w_k", etc.).
    pub fn new(
        heads: usize,
        dim: usize,
        seed: u64,
        var_map: &candle_nn::VarMap,
    ) -> Result<Self, String> {
        if heads == 0 {
            return Err("trainable_attention: heads must be > 0".to_string());
        }
        if !dim.is_multiple_of(heads) {
            return Err(format!(
                "trainable_attention: dim ({}) must be divisible by heads ({})",
                dim, heads
            ));
        }
        let head_dim = dim / heads;
        let bound = 1.0 / (dim as f64).sqrt();
        let device = Device::Cpu;

        let total = 4 * dim * dim;
        let weights = generate_uniform_f32(seed, total, -bound, bound);

        let slice_q = &weights[0..dim * dim];
        let slice_k = &weights[dim * dim..2 * dim * dim];
        let slice_v = &weights[2 * dim * dim..3 * dim * dim];
        let slice_o = &weights[3 * dim * dim..4 * dim * dim];

        let make_var = |slice: &[f32], name: &str| -> Result<Var, String> {
            let tensor = Tensor::from_slice(slice, (dim, dim), &device)
                .and_then(|t| t.to_dtype(DType::F32))
                .map_err(|e| format!("trainable_attn {}: tensor: {}", name, e))?;
            let var = Var::from_tensor(&tensor)
                .map_err(|e| format!("trainable_attn {}: from_tensor: {}", name, e))?;
            // VarMap::set_one takes &mut self, which conflicts with the
            // closure's borrow pattern. Insert directly into the data map
            // — same effect (VarMap::data() returns &Mutex<HashMap>).
            // Use map_err to satisfy clippy (no unwrap on Result).
            let mut guard = var_map
                .data()
                .lock()
                .map_err(|e| format!("trainable_attn {}: lock: {}", name, e))?;
            guard.insert(name.to_string(), var.clone());
            Ok(var)
        };

        let w_q = make_var(slice_q, "attn_w_q")?;
        let w_k = make_var(slice_k, "attn_w_k")?;
        let w_v = make_var(slice_v, "attn_w_v")?;
        let w_o = make_var(slice_o, "attn_w_o")?;

        Ok(Self {
            heads,
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
        // a Var).
        let q = input
            .matmul(self.w_q.as_tensor())
            .map_err(|e| format!("trainable_attn Q matmul: {}", e))?;
        let k = input
            .matmul(self.w_k.as_tensor())
            .map_err(|e| format!("trainable_attn K matmul: {}", e))?;
        let v = input
            .matmul(self.w_v.as_tensor())
            .map_err(|e| format!("trainable_attn V matmul: {}", e))?;

        // RoPE on Q and K
        let q = self.apply_rope(&q, seq_len, device)?;
        let k = self.apply_rope(&k, seq_len, device)?;

        // Reshape to [heads, seq, head_dim]
        let q = q
            .reshape((seq_len, self.heads, self.head_dim))
            .and_then(|t| t.transpose(0, 1))
            .map_err(|e| format!("trainable_attn Q reshape: {}", e))?;
        let k = k
            .reshape((seq_len, self.heads, self.head_dim))
            .and_then(|t| t.transpose(0, 1))
            .map_err(|e| format!("trainable_attn K reshape: {}", e))?;
        let v = v
            .reshape((seq_len, self.heads, self.head_dim))
            .and_then(|t| t.transpose(0, 1))
            .map_err(|e| format!("trainable_attn V reshape: {}", e))?;

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

    /// Apply RoPE — same algorithm as `Attention::apply_rope` (Наряд №183).
    fn apply_rope(&self, x: &Tensor, seq_len: usize, device: &Device) -> Result<Tensor, String> {
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
            .reshape((seq_len, self.heads, head_dim))
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
            .reshape((seq_len, self.dim))
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

/// Build function — accepts same args as `build_attention` (heads, dim).
/// Takes the `VarMap` (not VarBuilder) so it can register the Vars
/// manually with deterministic init values.
pub fn build_trainable_attention(
    args: &[Value],
    seed: u64,
    var_map: &candle_nn::VarMap,
) -> Result<Box<dyn SequenceLayer>, String> {
    if args.len() != 2 {
        return Err(format!(
            "trainable_attention: expected 2 args (heads, dim), got {}",
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
    let attn = TrainableAttention::new(heads, dim, seed, var_map)?;
    Ok(Box::new(attn))
}
