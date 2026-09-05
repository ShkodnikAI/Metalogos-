//! Multi-head self-attention with RoPE — first `SequenceLayer` (Наряд №183).
//!
//! Implements the reference structure from наряд №176 (real `candle-transformers`
//! Llama): RoPE positional encoding + multi-head self-attention.
//!
//! ## What this is NOT
//!
//! Per the naryad spec:
//!   - NOT a full transformer block (no RmsNorm/SwiGLU/residual) — that's
//!     a separate, later naryad.
//!   - NOT GQA (grouped-query attention) — standard multi-head is
//!     sufficient for the first block; GQA is a separate, later naryad.
//!   - NOT integrated with `reflex_train`/`reflex_predict` — first
//!     confirm forward-pass correctness in isolation; training is the
//!     next naryad.
//!
//! ## Determinism
//!
//! `candle 0.11` CPU backend does NOT support `Device::set_seed` (the
//! method exists but bails with "cannot seed the CPU rng"). Наряд №176's
//! determinism claim was specifically about CUDA/Metal backends — CPU
//! uses a non-seedable thread-local RNG.
//!
//! To honor `reflex_seq { seed: N }` determinism (Наряд №183 Contract 5),
//! this module initializes Q/K/V/O weights **manually** via the project's
//! own `xorshift64` PRNG (наряд №177, already verified deterministic). The
//! weights are constructed as `Vec<f32>` and converted to `Tensor` via
//! `Tensor::from_vec` — fully deterministic, no `candle` RNG involved.
//!
//! This is the same discipline the rest of the `Reflex` pillar uses:
//! `Dense` (наряд №178) also initializes weights via `xorshift64`, not via
//! `candle`'s RNG. The two RNG sources are kept separate by design —
//! `candle`'s RNG is for `candle`-internal ops (dropout, etc., which
//! this naryad doesn't use); project-seeded init is for the layer's
//! declared `seed`.

#![cfg(feature = "candle")]

use crate::interpreter::Value;
use crate::nn::sequence_layer::SequenceLayer;

use candle_core::{DType, Device, Tensor, D};

/// Multi-head self-attention with RoPE positional encoding.
///
/// Structure (per Llama reference, наряд №176):
///   - Q, K, V projections: `hidden_dim → hidden_dim` (no bias, matching Llama).
///   - RoPE applied to Q and K (NOT V) — standard rotary position embedding.
///   - Attention scores: `softmax(Q @ K^T / sqrt(head_dim))`.
///   - Output projection: `hidden_dim → hidden_dim` (no bias).
///
/// `heads` must divide `hidden_dim` evenly. `head_dim = hidden_dim / heads`.
pub struct Attention {
    /// Number of attention heads.
    heads: usize,
    /// Hidden dimension (input == output for attention blocks).
    dim: usize,
    /// Per-head dimension: `dim / heads`.
    head_dim: usize,
    /// Q projection: `[dim, dim]`.
    w_q: Tensor,
    /// K projection: `[dim, dim]`.
    w_k: Tensor,
    /// V projection: `[dim, dim]`.
    w_v: Tensor,
    /// Output projection: `[dim, dim]`.
    w_o: Tensor,
    /// RoPE theta (base frequency). Llama uses 10000.
    rope_theta: f64,
}

/// Helper macro for mapping `candle_core::Error` to `String`.
///
/// `candle`'s tensor operations return `Result<_, candle_core::Error>` —
/// the project convention is `Result<_, String>`. Without this helper
/// every `?` would need an explicit `.map_err(|e| format!("...: {}", e))`.
macro_rules! ctry {
    ($expr:expr, $context:expr) => {
        $expr.map_err(|e| format!("{}: {}", $context, e))
    };
}

impl Attention {
    /// Construct an attention block with weights initialized from `seed`.
    ///
    /// Weights are drawn from a uniform distribution `[-1/sqrt(dim), 1/sqrt(dim)]`
    /// using the project's `xorshift64` PRNG (наряд №177). Same `seed` always
    /// produces the same weights → same forward-pass result (Наряд №183
    /// Contract 5: determinism).
    pub fn new(heads: usize, dim: usize, seed: u64) -> Result<Self, String> {
        if heads == 0 {
            return Err("attention: heads must be > 0".to_string());
        }
        if !dim.is_multiple_of(heads) {
            return Err(format!(
                "attention: dim ({}) must be divisible by heads ({})",
                dim, heads
            ));
        }
        let head_dim = dim / heads;

        let bound = 1.0 / (dim as f64).sqrt();

        // Generate all 4 weight matrices from the project's PRNG.
        // Same sequence consumed in order: Q first, then K, then V, then O —
        // so the same seed produces a fully reproducible weight set.
        let total_floats = 4 * dim * dim;
        let weights = generate_uniform_f32(seed, total_floats, -bound, bound);

        let device = Device::Cpu;

        // Each weight matrix is [dim, dim] in row-major order.
        let slice_q = &weights[0..dim * dim];
        let slice_k = &weights[dim * dim..2 * dim * dim];
        let slice_v = &weights[2 * dim * dim..3 * dim * dim];
        let slice_o = &weights[3 * dim * dim..4 * dim * dim];

        let w_q = ctry!(
            Tensor::from_slice(slice_q, (dim, dim), &device).and_then(|t| t.to_dtype(DType::F32)),
            "attention: w_q init"
        )?;
        let w_k = ctry!(
            Tensor::from_slice(slice_k, (dim, dim), &device).and_then(|t| t.to_dtype(DType::F32)),
            "attention: w_k init"
        )?;
        let w_v = ctry!(
            Tensor::from_slice(slice_v, (dim, dim), &device).and_then(|t| t.to_dtype(DType::F32)),
            "attention: w_v init"
        )?;
        let w_o = ctry!(
            Tensor::from_slice(slice_o, (dim, dim), &device).and_then(|t| t.to_dtype(DType::F32)),
            "attention: w_o init"
        )?;

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

    /// Apply RoPE (rotary position embedding) to a tensor of shape
    /// `[seq_len, dim]`.
    ///
    /// RoPE encodes position by rotating pairs of dimensions. For pair (i, j)
    /// at position `pos`, the rotation angle is `pos / theta^(2 * pair_idx / dim)`.
    ///
    /// This is a minimal, correct implementation matching the standard
    /// formulation (Llama, GPT-NeoX). Complex-number rotation is
    /// expressed via real-valued 2x2 rotation matrices applied to
    /// consecutive pairs of channels (within each head).
    fn apply_rope(&self, x: &Tensor, seq_len: usize) -> Result<Tensor, String> {
        let device = x.device();
        let dtype = x.dtype();
        let head_dim = self.head_dim;
        if !head_dim.is_multiple_of(2) {
            return Err(format!(
                "attention: head_dim ({}) must be even for RoPE",
                head_dim
            ));
        }

        // Compute the inverse frequencies: theta_i = 1.0 / (theta ^ (2i / head_dim))
        // for i in 0..(head_dim/2).
        let half = head_dim / 2;
        let inv_freq: Vec<f32> = (0..half)
            .map(|i| {
                let exponent = 2.0 * (i as f64) / (head_dim as f64);
                (1.0 / self.rope_theta.powf(exponent)) as f32
            })
            .collect();

        // Outer product: positions × inv_freq → angles[seq_len, half]
        let angles: Vec<f32> = (0..seq_len)
            .flat_map(|p| {
                let pos = p as f32;
                inv_freq
                    .iter()
                    .map(move |&freq| pos * freq)
                    .collect::<Vec<_>>()
            })
            .collect();

        let angles = ctry!(
            Tensor::from_slice(&angles, (seq_len, half), device),
            "rope angles"
        )?;
        let angles = ctry!(angles.to_dtype(dtype), "rope angles dtype")?;
        let cos = ctry!(angles.cos(), "rope cos")?;
        let sin = ctry!(angles.sin(), "rope sin")?;

        // x shape: [seq_len, dim]. Reshape to [seq_len, heads, head_dim]
        // so we can apply RoPE per-head.
        let x_reshaped = ctry!(
            x.reshape((seq_len, self.heads, head_dim)),
            "rope reshape to heads"
        )?;

        // Split head_dim into two halves: [d_0, d_1, ..., d_{half-1}] and
        // [d_half, ..., d_{head_dim-1}]. RoPE rotates (d_i, d_{i+half}) pairs.
        let x_first = ctry!(x_reshaped.narrow(2, 0, half), "rope narrow first")?;
        let x_second = ctry!(x_reshaped.narrow(2, half, half), "rope narrow second")?;

        // Broadcast cos/sin from [seq, half] to [seq, 1, half] for per-head apply.
        let cos = ctry!(cos.unsqueeze(1), "rope cos unsqueeze")?;
        let sin = ctry!(sin.unsqueeze(1), "rope sin unsqueeze")?;

        // rotation: x_first' = x_first * cos - x_second * sin
        //           x_second' = x_first * sin + x_second * cos
        let x_first_c = ctry!(x_first.broadcast_mul(&cos), "rope x_first*cos")?;
        let x_first_s = ctry!(x_first.broadcast_mul(&sin), "rope x_first*sin")?;
        let x_second_c = ctry!(x_second.broadcast_mul(&cos), "rope x_second*cos")?;
        let x_second_s = ctry!(x_second.broadcast_mul(&sin), "rope x_second*sin")?;

        let x_first_new = ctry!(x_first_c - &x_second_s, "rope first_new")?;
        let x_second_new = ctry!(x_first_s + &x_second_c, "rope second_new")?;

        // Concatenate along the head_dim axis and reshape back to [seq, dim].
        let rotated = ctry!(
            Tensor::cat(&[&x_first_new, &x_second_new], 2),
            "rope concat"
        )?;
        ctry!(
            rotated.reshape((seq_len, self.dim)),
            "rope reshape back to [seq, dim]"
        )
    }

    /// Multi-head attention forward pass.
    ///
    /// Input: `[seq_len, dim]` (a single sequence; batch dim assumed 1).
    /// Output: `[seq_len, dim]`.
    ///
    /// Steps:
    ///   1. Project input to Q, K, V (each `[seq_len, dim]`).
    ///   2. Apply RoPE to Q and K.
    ///   3. Reshape to `[seq, heads, head_dim]` and transpose to `[heads, seq, head_dim]`.
    ///   4. Attention scores = Q @ K^T / sqrt(head_dim) → `[heads, seq, seq]`.
    ///   5. softmax(scores) → `[heads, seq, seq]`.
    ///   6. Output = scores @ V → `[heads, seq, head_dim]`.
    ///   7. Transpose back and reshape to `[seq, dim]`.
    ///   8. Output projection w_o: `[seq, dim]`.
    fn forward_impl(&self, input: &Tensor) -> Result<Tensor, String> {
        let (seq_len, _in_dim) = ctry!(input.dims2(), "attention: input dims2")?;
        let device = input.device();

        // Step 1: Q, K, V projections — input @ W.
        // input: [seq, dim], W: [dim, dim] → output: [seq, dim].
        let q = ctry!(input.matmul(&self.w_q), "attention: Q matmul")?;
        let k = ctry!(input.matmul(&self.w_k), "attention: K matmul")?;
        let v = ctry!(input.matmul(&self.w_v), "attention: V matmul")?;

        // Step 2: apply RoPE to Q and K.
        let q = self.apply_rope(&q, seq_len)?;
        let k = self.apply_rope(&k, seq_len)?;

        // Step 3: reshape to [heads, seq, head_dim] for parallel head computation.
        let q = ctry!(
            q.reshape((seq_len, self.heads, self.head_dim))
                .and_then(|t| t.transpose(0, 1)),
            "attention: Q reshape+transpose"
        )?;
        let k = ctry!(
            k.reshape((seq_len, self.heads, self.head_dim))
                .and_then(|t| t.transpose(0, 1)),
            "attention: K reshape+transpose"
        )?;
        let v = ctry!(
            v.reshape((seq_len, self.heads, self.head_dim))
                .and_then(|t| t.transpose(0, 1)),
            "attention: V reshape+transpose"
        )?;

        // Step 4: attention scores = Q @ K^T / sqrt(head_dim)
        let k_t = ctry!(k.transpose(1, 2), "attention: K^T")?;
        let scale = 1.0 / (self.head_dim as f64).sqrt();
        // Use F32 to match the score tensor's dtype (candle won't auto-broadcast
        // across dtype boundaries — F32 * F64 raises "dtype mismatch").
        let scale_tensor = ctry!(Tensor::new(scale as f32, device), "attention: scale tensor")?;
        let scores = ctry!(q.matmul(&k_t), "attention: Q@K^T")?;
        let scores = ctry!(
            scores.broadcast_mul(&scale_tensor),
            "attention: scale scores"
        )?;

        // Step 5: softmax along last dim (the keys).
        let attn = ctry!(
            candle_nn::ops::softmax(&scores, D::Minus1),
            "attention: softmax"
        )?;

        // Step 6: weighted sum — attn @ V → [heads, seq, head_dim]
        let out = ctry!(attn.matmul(&v), "attention: attn@V")?;

        // Step 7: transpose back and reshape to [seq, dim].
        let out = ctry!(out.transpose(0, 1), "attention: out transpose")?;
        let out = ctry!(out.reshape((seq_len, self.dim)), "attention: out reshape")?;

        // Step 8: output projection.
        ctry!(out.matmul(&self.w_o), "attention: out proj")
    }
}

impl SequenceLayer for Attention {
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
        "attention"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Build function for the SEQUENCE_LAYER_REGISTRY.
///
/// `args` is the parsed `layer_arg` list from the grammar
/// (наряд №178's `layer_spec = { IDENT ~ "(" ~ layer_arg_list? ~ ")" }`).
/// For `attention`, the expected args are: `(heads, dim)`.
///
/// `seed` is the model-level seed (from `reflex_seq { seed: N }`) —
/// used for deterministic weight initialization.
pub fn build_attention(args: &[Value], seed: u64) -> Result<Box<dyn SequenceLayer>, String> {
    if args.len() != 2 {
        return Err(format!(
            "attention: expected 2 args (heads, dim), got {}",
            args.len()
        ));
    }
    let heads = match &args[0] {
        Value::Float(n) => *n as usize,
        Value::String(s) => s
            .parse::<usize>()
            .map_err(|_| format!("attention: heads must be a positive integer, got '{}'", s))?,
        other => {
            return Err(format!(
                "attention: heads must be a number, got {}",
                other.type_name()
            ))
        }
    };
    let dim = match &args[1] {
        Value::Float(n) => *n as usize,
        Value::String(s) => s
            .parse::<usize>()
            .map_err(|_| format!("attention: dim must be a positive integer, got '{}'", s))?,
        other => {
            return Err(format!(
                "attention: dim must be a number, got {}",
                other.type_name()
            ))
        }
    };
    let attn = Attention::new(heads, dim, seed)?;
    Ok(Box::new(attn))
}

// ── Deterministic PRNG (xorshift64, наряд №177) ──────────────────────
//
// Re-implementation of the project's xorshift64 for `f32` weight
// generation. Same algorithm as `src/builtins/math.rs`'s `random()`
// builtin and `src/nn/dense.rs`'s Xavier init — kept local rather than
// imported to avoid pulling the builtin module into a low-level nn
// module (the nn module should not depend on the builtins layer).
//
// The algorithm is byte-identical to its other appearances: state XORs
// with shifted self, then top 53 bits → [0, 1) f64. We downcast to f32
// because that's what `candle`'s default dtype is on CPU.

/// Generate `n` uniform f32 values in `[lo, up]` from a seeded xorshift64.
///
/// Same seed → same sequence → same weights → same forward-pass result
/// (Наряд №183 Contract 5: determinism).
fn generate_uniform_f32(seed: u64, n: usize, lo: f64, up: f64) -> Vec<f32> {
    let mut state = seed_to_state(seed);
    let range = up - lo;
    (0..n)
        .map(|_| {
            state = xorshift64(state);
            let u01 = u64_to_float_01(state);
            (lo + range * u01) as f32
        })
        .collect()
}

/// Seed → xorshift64 state (handles the degenerate seed=0 case the same
/// way `src/builtins/math.rs` does — XOR with a fixed constant).
fn seed_to_state(seed: u64) -> u64 {
    let state = seed ^ 0x9E3779B97F4A7C15;
    if state == 0 {
        0x9E3779B97F4A7C15
    } else {
        state
    }
}

fn xorshift64(mut state: u64) -> u64 {
    state ^= state << 13;
    state ^= state >> 7;
    state ^= state << 17;
    state
}

/// Convert a u64 to a float in [0.0, 1.0).
/// Uses the top 53 bits (mantissa width of f64) for maximum precision.
fn u64_to_float_01(bits: u64) -> f64 {
    let mantissa = bits >> 11; // top 53 bits
    (mantissa as f64) / ((1u64 << 53) as f64)
}
