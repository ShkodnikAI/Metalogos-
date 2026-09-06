//! Multi-head self-attention with RoPE — first `SequenceLayer` (Наряд №183).
//!
//! Implements the reference structure from наряд №176 (real `candle-transformers`
//! Llama): RoPE positional encoding + multi-head self-attention.
//!
//! Наряд №188 extended this to support GQA (Grouped-Query Attention):
//!   - Q has `n_heads` heads, K/V have `n_kv_heads` heads (≤ n_heads).
//!   - When `n_kv_heads == n_heads` → behaviour is identical to Наряд №183
//!     (backward compatibility, regression-tested).
//!   - When `n_kv_heads < n_heads` → K and V are repeated (tiled) along
//!     the head axis before the attention computation. This matches
//!     Llama 2/3's actual architecture (verified in Наряд №176 reference).
//!
//! ## What this is NOT
//!
//! Per the naryad spec:
//!   - NOT a full transformer block (no RmsNorm/SwiGLU/residual) — that's
//!     a separate, later naryad.
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
    /// Number of attention heads (Q heads).
    heads: usize,
    /// Number of KV heads (n_kv_heads ≤ n_heads, n_heads % n_kv_heads == 0).
    /// When == n_heads → standard multi-head (Наряд №183 backward compat).
    /// When < n_heads → GQA (Наряд №188): K and V are repeated by
    /// n_heads / n_kv_heads before the attention computation.
    n_kv_heads: usize,
    /// Hidden dimension (input == output for attention blocks).
    dim: usize,
    /// Per-head dimension: `dim / heads`.
    head_dim: usize,
    /// Q projection: `[dim, dim]`.
    w_q: Tensor,
    /// K projection: `[dim, kv_dim]` where kv_dim = n_kv_heads * head_dim.
    w_k: Tensor,
    /// V projection: `[dim, kv_dim]`.
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
    /// Construct a standard multi-head attention block (backward compat).
    ///
    /// Equivalent to `new_with_kv_heads(heads, heads, dim, seed)` —
    /// when `n_kv_heads == n_heads`, K and V have the same number of
    /// heads as Q, no repetition is needed, and the forward pass is
    /// identical to Наряд №183's implementation.
    ///
    /// This is the constructor used by:
    ///   - Наряд №183 tests (regression contract — must produce the
    ///     same numerical output).
    ///   - TransformerBlock (Наряд №184) — calls `Attention::new(heads, dim, seed)`,
    ///     which defaults to standard MHA. GQA is opt-in only when the
    ///     user explicitly writes `attention(heads, dim, kv_heads)`.
    pub fn new(heads: usize, dim: usize, seed: u64) -> Result<Self, String> {
        Self::new_with_kv_heads(heads, heads, dim, seed)
    }

    /// Construct an attention block with GQA (Grouped-Query Attention).
    ///
    /// `n_kv_heads` is the number of KV heads. When `n_kv_heads < n_heads`,
    /// K and V are repeated by factor `n_heads / n_kv_heads` before the
    /// attention computation (Llama 2/3 architecture).
    ///
    /// Constraints:
    ///   - `n_kv_heads > 0`
    ///   - `n_kv_heads <= n_heads`
    ///   - `n_heads % n_kv_heads == 0` (even grouping)
    ///
    /// Weight layout (same RNG stream order, backward compatible when
    /// `n_kv_heads == n_heads` because `kv_dim == dim`):
    ///   1. W_q `[dim, dim]`
    ///   2. W_k `[dim, kv_dim]` where `kv_dim = n_kv_heads * head_dim`
    ///   3. W_v `[dim, kv_dim]`
    ///   4. W_o `[dim, dim]`
    pub fn new_with_kv_heads(
        heads: usize,
        n_kv_heads: usize,
        dim: usize,
        seed: u64,
    ) -> Result<Self, String> {
        if heads == 0 {
            return Err("attention: heads must be > 0".to_string());
        }
        if n_kv_heads == 0 {
            return Err("attention: n_kv_heads must be > 0".to_string());
        }
        if n_kv_heads > heads {
            return Err(format!(
                "attention: n_kv_heads ({}) must be <= n_heads ({})",
                n_kv_heads, heads
            ));
        }
        if !dim.is_multiple_of(heads) {
            return Err(format!(
                "attention: dim ({}) must be divisible by heads ({})",
                dim, heads
            ));
        }
        if !heads.is_multiple_of(n_kv_heads) {
            return Err(format!(
                "attention: n_heads ({}) must be divisible by n_kv_heads ({})",
                heads, n_kv_heads
            ));
        }
        let head_dim = dim / heads;
        let kv_dim = n_kv_heads * head_dim;

        let bound = 1.0 / (dim as f64).sqrt();

        // Generate all 4 weight matrices from the project's PRNG.
        // When n_kv_heads == n_heads → kv_dim == dim → total == 4*dim*dim,
        // and slices match Наряд №183's layout exactly.
        let total = 2 * dim * dim + 2 * dim * kv_dim;
        let weights = generate_uniform_f32(seed, total, -bound, bound);

        let device = Device::Cpu;

        let slice_q = &weights[0..dim * dim];
        let slice_k = &weights[dim * dim..dim * dim + dim * kv_dim];
        let slice_v = &weights[dim * dim + dim * kv_dim..dim * dim + 2 * dim * kv_dim];
        let slice_o = &weights[dim * dim + 2 * dim * kv_dim..];

        let w_q = ctry!(
            Tensor::from_slice(slice_q, (dim, dim), &device).and_then(|t| t.to_dtype(DType::F32)),
            "attention: w_q init"
        )?;
        let w_k = ctry!(
            Tensor::from_slice(slice_k, (dim, kv_dim), &device)
                .and_then(|t| t.to_dtype(DType::F32)),
            "attention: w_k init"
        )?;
        let w_v = ctry!(
            Tensor::from_slice(slice_v, (dim, kv_dim), &device)
                .and_then(|t| t.to_dtype(DType::F32)),
            "attention: w_v init"
        )?;
        let w_o = ctry!(
            Tensor::from_slice(slice_o, (dim, dim), &device).and_then(|t| t.to_dtype(DType::F32)),
            "attention: w_o init"
        )?;

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

    /// Apply RoPE (rotary position embedding) to a tensor of shape
    /// `[seq_len, n_h * head_dim]`.
    ///
    /// RoPE encodes position by rotating pairs of dimensions. For pair (i, j)
    /// at position `pos`, the rotation angle is `pos / theta^(2 * pair_idx / head_dim)`.
    ///
    /// `n_h` is the number of heads in the tensor — for Q it's `self.heads`,
    /// for K (in GQA) it's `self.n_kv_heads`. head_dim is the same for both.
    fn apply_rope(&self, x: &Tensor, seq_len: usize, n_h: usize) -> Result<Tensor, String> {
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

        // x shape: [seq_len, n_h * head_dim]. Reshape to [seq_len, n_h, head_dim]
        // so we can apply RoPE per-head.
        let x_reshaped = ctry!(x.reshape((seq_len, n_h, head_dim)), "rope reshape to heads")?;

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

        // Concatenate along the head_dim axis and reshape back.
        let rotated = ctry!(
            Tensor::cat(&[&x_first_new, &x_second_new], 2),
            "rope concat"
        )?;
        ctry!(
            rotated.reshape((seq_len, n_h * head_dim)),
            "rope reshape back"
        )
    }

    /// Repeat KV heads along the head axis (GQA).
    ///
    /// Input: `[n_kv_heads, seq, head_dim]`
    /// Output: `[n_kv_heads * n_rep, seq, head_dim]` where `n_rep = n_heads / n_kv_heads`.
    ///
    /// Uses the unsqueeze + expand + reshape pattern (same as
    /// candle-transformers' `repeat_kv`). When `n_rep == 1` (standard
    /// MHA), this is a no-op.
    fn repeat_kv(&self, x: &Tensor) -> Result<Tensor, String> {
        let n_rep = self.heads / self.n_kv_heads;
        if n_rep == 1 {
            return Ok(x.clone());
        }
        // x: [n_kv_heads, seq, head_dim]
        let dims = x.dims();
        let n_kv = dims[0];
        let seq_len = dims[1];
        let head_dim = dims[2];

        // unsqueeze(1): [n_kv_heads, 1, seq, head_dim]
        let x = ctry!(x.unsqueeze(1), "gqa: unsqueeze")?;
        // expand: [n_kv_heads, n_rep, seq, head_dim]
        let x = ctry!(x.expand((n_kv, n_rep, seq_len, head_dim)), "gqa: expand")?;
        // reshape: [n_kv_heads * n_rep, seq, head_dim] = [n_heads, seq, head_dim]
        ctry!(x.reshape((n_kv * n_rep, seq_len, head_dim)), "gqa: reshape")
    }

    /// Multi-head attention forward pass (Наряд №188: GQA-aware).
    ///
    /// Input: `[seq_len, dim]` (a single sequence; batch dim assumed 1).
    /// Output: `[seq_len, dim]`.
    ///
    /// Steps:
    ///   1. Project input to Q `[seq, dim]`, K `[seq, kv_dim]`, V `[seq, kv_dim]`.
    ///   2. Apply RoPE to Q (with n_heads) and K (with n_kv_heads).
    ///   3. Reshape Q to `[n_heads, seq, head_dim]`, K/V to `[n_kv_heads, seq, head_dim]`.
    ///   4. GQA: repeat K, V along head axis to `[n_heads, seq, head_dim]`
    ///      (no-op when n_kv_heads == n_heads).
    ///   5. Attention scores = Q @ K^T / sqrt(head_dim) → `[heads, seq, seq]`.
    ///   6. softmax(scores) → `[heads, seq, seq]`.
    ///   7. Output = scores @ V → `[heads, seq, head_dim]`.
    ///   8. Transpose back and reshape to `[seq, dim]`.
    ///   9. Output projection w_o: `[seq, dim]`.
    fn forward_impl(&self, input: &Tensor) -> Result<Tensor, String> {
        let (seq_len, _in_dim) = ctry!(input.dims2(), "attention: input dims2")?;
        let device = input.device();

        // Step 1: Q, K, V projections.
        // Q: [seq, dim], K/V: [seq, kv_dim] (kv_dim < dim when GQA).
        let q = ctry!(input.matmul(&self.w_q), "attention: Q matmul")?;
        let k = ctry!(input.matmul(&self.w_k), "attention: K matmul")?;
        let v = ctry!(input.matmul(&self.w_v), "attention: V matmul")?;

        // Step 2: apply RoPE to Q and K. Q has n_heads, K has n_kv_heads.
        let q = self.apply_rope(&q, seq_len, self.heads)?;
        let k = self.apply_rope(&k, seq_len, self.n_kv_heads)?;

        // Step 3: reshape to [heads, seq, head_dim].
        let q = ctry!(
            q.reshape((seq_len, self.heads, self.head_dim))
                .and_then(|t| t.transpose(0, 1)),
            "attention: Q reshape+transpose"
        )?;
        let k = ctry!(
            k.reshape((seq_len, self.n_kv_heads, self.head_dim))
                .and_then(|t| t.transpose(0, 1)),
            "attention: K reshape+transpose"
        )?;
        let v = ctry!(
            v.reshape((seq_len, self.n_kv_heads, self.head_dim))
                .and_then(|t| t.transpose(0, 1)),
            "attention: V reshape+transpose"
        )?;

        // Step 4: GQA — repeat K, V to match Q's head count.
        let k = self.repeat_kv(&k)?;
        let v = self.repeat_kv(&v)?;

        // Step 5: attention scores = Q @ K^T / sqrt(head_dim)
        let k_t = ctry!(k.transpose(1, 2), "attention: K^T")?;
        let scale = 1.0 / (self.head_dim as f64).sqrt();
        let scale_tensor = ctry!(Tensor::new(scale as f32, device), "attention: scale tensor")?;
        let scores = ctry!(q.matmul(&k_t), "attention: Q@K^T")?;
        let scores = ctry!(
            scores.broadcast_mul(&scale_tensor),
            "attention: scale scores"
        )?;

        // Step 6: softmax along last dim (the keys).
        let attn = ctry!(
            candle_nn::ops::softmax(&scores, D::Minus1),
            "attention: softmax"
        )?;

        // Step 7: weighted sum — attn @ V → [heads, seq, head_dim]
        let out = ctry!(attn.matmul(&v), "attention: attn@V")?;

        // Step 8: transpose back and reshape to [seq, dim].
        let out = ctry!(out.transpose(0, 1), "attention: out transpose")?;
        let out = ctry!(out.reshape((seq_len, self.dim)), "attention: out reshape")?;

        // Step 9: output projection.
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
///
/// Args:
///   - `attention(heads, dim)` → standard MHA (Наряд №183 backward compat)
///   - `attention(heads, dim, kv_heads)` → GQA (Наряд №188)
///
/// The 3rd arg is optional — when omitted, `n_kv_heads = n_heads`.
pub fn build_attention(args: &[Value], seed: u64) -> Result<Box<dyn SequenceLayer>, String> {
    if args.len() != 2 && args.len() != 3 {
        return Err(format!(
            "attention: expected 2 args (heads, dim) or 3 args (heads, dim, kv_heads), got {}",
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
    // Наряд №188: optional 3rd arg — n_kv_heads for GQA.
    let n_kv_heads = if args.len() == 3 {
        match &args[2] {
            Value::Float(n) => *n as usize,
            Value::String(s) => s.parse::<usize>().map_err(|_| {
                format!(
                    "attention: kv_heads must be a positive integer, got '{}'",
                    s
                )
            })?,
            other => {
                return Err(format!(
                    "attention: kv_heads must be a number, got {}",
                    other.type_name()
                ))
            }
        }
    } else {
        heads // default: standard MHA
    };

    let attn = Attention::new_with_kv_heads(heads, n_kv_heads, dim, seed)?;
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
///
/// Public so other SequenceLayer modules (SwiGLU, TransformerBlock in
/// Наряд №184; TrainableAttention in Наряд №185) can reuse the exact
/// same PRNG — keeping a single source of truth for the project's
/// weight-init algorithm within the `nn` module. Still local to `nn`
/// (not exported to `builtins`) per the separation principle
/// documented in Наряд №183.
pub fn generate_uniform_f32(seed: u64, n: usize, lo: f64, up: f64) -> Vec<f32> {
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
