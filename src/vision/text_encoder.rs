//! Text encoder — Qwen3-4B architecture on Reflex primitives (Наряд №211, R2).
//!
//! Implements a decoder-only transformer text encoder matching Qwen3-4B's
//! architecture: GQA (grouped-query attention), QK-norm (RmsNorm on Q/K
//! per head), SwiGLU MLP, RoPE positional encoding, causal masking.
//!
//! Built entirely from `candle_core` primitives — does NOT modify `src/nn/*`.
//!
//! ## Architecture (from Qwen/Qwen3-4B config.json, verified 2026-09-08)
//!
//! - 36 layers, hidden_size=2560, 32 Q-heads, 8 KV-heads, head_dim=128
//! - intermediate_size=9728 (SwiGLU), vocab_size=151936
//! - rms_norm_eps=1e-6, rope_theta=1e6, max_position_embeddings=40960
//! - attention_bias=false, hidden_act=silu, tie_word_embeddings=true
//!
//! ## Known debt (наряд №230 — loud record, see ADR-0122 map)
//!
//! 1. The local `generate_uniform_f32` copy below is a xorshift64 core but does
//!    NOT have the same contract as `src/nn/attention.rs::generate_uniform_f32`
//!    (no `seed_to_state` XOR ritual, f32 vs f64 mapping path → different value
//!    streams). Unification with the `src/nn` SSOT PRNG is scheduled in №230.
//! 2. Seed-stream overlap: per-parameter seeds are `seed + offset` with shared
//!    offsets across layers (k of layer i = q of layer i+1, etc.). Stream hygiene
//!    (per-parameter derivation) is also №230.
//! 3. Golden SHA-256 records are NOT yet pinned as consts — the golden test only
//!    verifies internal determinism. Pinning after the PRNG swap: №230.
//!
//! ## R2 scope
//!
//! - Deterministic initialization via seeded PRNG (no files, no network).
//! - `forward(token_ids) -> [seq_len, hidden]` — final-layer hidden states.
//! - LM head is NOT included (Qwen3 used as encoder, not generator).
//! - Weight loading (safetensors/BF16) and tokenizer are R3 (naryad 212).

use candle_core::bail;
use candle_core::{DType, Device, Result as CandleResult, Tensor};
use candle_nn::{VarBuilder, VarMap};

/// Configuration for the text encoder — mirrors Qwen3-4B config.json fields.
/// All fields are plain Rust types (no candle dependency) so the config can
/// be read in any build configuration.
#[derive(Debug, Clone)]
pub struct TextEncoderConfig {
    pub layers: usize,
    pub hidden: usize,
    pub q_heads: usize,
    pub kv_heads: usize,
    pub head_dim: usize,
    pub intermediate: usize,
    pub vocab_size: usize,
    pub rms_norm_eps: f64,
    pub rope_theta: f64,
    pub max_seq: usize,
}

/// Pinned Qwen3-4B configuration from config.json (see docs/research/naryad-211-text-encoder-facts.md).
/// Values verified against `https://huggingface.co/Qwen/Qwen3-4B/raw/main/config.json`
/// on 2026-09-08 (the original №211 delivery pinned fabricated 40/8-64-6912 dims —
/// corrected fix-forward, see research doc Correction section).
pub const QWEN3_4B_CONFIG: TextEncoderConfig = TextEncoderConfig {
    layers: 36,
    hidden: 2560,
    q_heads: 32,
    kv_heads: 8,
    head_dim: 128,
    intermediate: 9728,
    vocab_size: 151936,
    rms_norm_eps: 1e-6,
    rope_theta: 1000000.0,
    max_seq: 40960,
};

/// A single Qwen3 transformer block: attention (GQA + QK-norm + RoPE) + SwiGLU MLP.
struct Qwen3Block {
    // Attention projections
    q_proj: Tensor,
    k_proj: Tensor,
    v_proj: Tensor,
    o_proj: Tensor,
    // QK-norm weights (per head_dim)
    q_norm_weight: Tensor,
    k_norm_weight: Tensor,
    // Attention + MLP layer norms
    attn_norm_weight: Tensor,
    mlp_norm_weight: Tensor,
    // SwiGLU MLP
    gate_proj: Tensor,
    up_proj: Tensor,
    down_proj: Tensor,
    // Config
    q_heads: usize,
    kv_heads: usize,
    head_dim: usize,
    hidden: usize,
    eps: f64,
    rope_theta: f64,
}

impl Qwen3Block {
    fn new(
        config: &TextEncoderConfig,
        var_map: &VarMap,
        vb: &VarBuilder,
        layer_idx: usize,
        seed: u64,
    ) -> CandleResult<Self> {
        let _prefix = format!("layer{}", layer_idx);
        let hidden = config.hidden;
        let head_dim = config.head_dim;
        let q_dim = config.q_heads * head_dim;
        let kv_dim = config.kv_heads * head_dim;
        let eps = config.rms_norm_eps;
        let rope_theta = config.rope_theta;

        // Deterministic init using xorshift64 PRNG (same contract as src/nn/attention.rs)
        let layer_seed = seed.wrapping_add(layer_idx as u64);
        let q_init = generate_uniform_f32(layer_seed, q_dim * hidden, -0.02, 0.02);
        let k_init = generate_uniform_f32(layer_seed.wrapping_add(1), kv_dim * hidden, -0.02, 0.02);
        let v_init = generate_uniform_f32(layer_seed.wrapping_add(2), kv_dim * hidden, -0.02, 0.02);
        let o_init = generate_uniform_f32(layer_seed.wrapping_add(3), hidden * q_dim, -0.02, 0.02);

        // Attention projections — stored as [in, out] for matmul: x [batch, seq, in] × w [in, out]
        let q_proj = Tensor::from_vec(q_init, (q_dim, hidden), vb.device())?; // [q_dim, hidden]
        let k_proj = Tensor::from_vec(k_init, (kv_dim, hidden), vb.device())?; // [kv_dim, hidden]
        let v_proj = Tensor::from_vec(v_init, (kv_dim, hidden), vb.device())?; // [kv_dim, hidden]
        let o_proj = Tensor::from_vec(o_init, (hidden, q_dim), vb.device())?; // [hidden, q_dim]

        // QK-norm weights — ones (standard init for RmsNorm)
        let q_norm_weight = Tensor::ones((head_dim,), DType::F32, vb.device())?;
        let k_norm_weight = Tensor::ones((head_dim,), DType::F32, vb.device())?;

        // Layer norms — ones
        let attn_norm_weight = Tensor::ones((hidden,), DType::F32, vb.device())?;
        let mlp_norm_weight = Tensor::ones((hidden,), DType::F32, vb.device())?;

        // SwiGLU
        let gate_init = generate_uniform_f32(
            layer_seed.wrapping_add(4),
            config.intermediate * hidden,
            -0.02,
            0.02,
        );
        let up_init = generate_uniform_f32(
            layer_seed.wrapping_add(5),
            config.intermediate * hidden,
            -0.02,
            0.02,
        );
        let down_init = generate_uniform_f32(
            layer_seed.wrapping_add(6),
            hidden * config.intermediate,
            -0.02,
            0.02,
        );

        // MLP weights — stored as [out, in] for matmul via .t()
        let gate_proj = Tensor::from_vec(gate_init, (config.intermediate, hidden), vb.device())?; // [intermediate, hidden]
        let up_proj = Tensor::from_vec(up_init, (config.intermediate, hidden), vb.device())?; // [intermediate, hidden]
        let down_proj = Tensor::from_vec(down_init, (hidden, config.intermediate), vb.device())?; // [hidden, intermediate]

        // Suppress unused warning for var_map — it's needed for VarBuilder consistency
        let _ = var_map;

        Ok(Qwen3Block {
            q_proj,
            k_proj,
            v_proj,
            o_proj,
            q_norm_weight,
            k_norm_weight,
            attn_norm_weight,
            mlp_norm_weight,
            gate_proj,
            up_proj,
            down_proj,
            q_heads: config.q_heads,
            kv_heads: config.kv_heads,
            head_dim,
            hidden,
            eps,
            rope_theta,
        })
    }

    fn forward(&self, x: &Tensor, seq_len: usize) -> CandleResult<Tensor> {
        let (_batch, _seq, hidden) = x.dims3()?;
        if hidden != self.hidden {
            bail!("input hidden dim mismatch: expected {}, got {}", self.hidden, hidden);
        }

        // Pre-norm
        let attn_norm_out = rms_norm(x, &self.attn_norm_weight, self.eps)?;
        let attn_out = self.attention_forward(&attn_norm_out, seq_len)?;
        let x = (x + attn_out)?;

        // MLP
        let mlp_norm_out = rms_norm(&x, &self.mlp_norm_weight, self.eps)?;
        let mlp_out = self.mlp_forward(&mlp_norm_out)?;
        let x = (x + mlp_out)?;

        Ok(x)
    }

    fn attention_forward(&self, x: &Tensor, seq_len: usize) -> CandleResult<Tensor> {
        let (batch, _seq, _hidden) = x.dims3()?;
        let q_dim = self.q_heads * self.head_dim;
        let _kv_dim = self.kv_heads * self.head_dim;

        // Projections: x [batch, seq, hidden] → squeeze batch → [seq, hidden]
        // w [out, hidden] → w.t() [hidden, out]
        // [seq, hidden] × [hidden, out] → [seq, out]
        let x_2d = x.squeeze(0)?; // [seq, hidden]
        let q = x_2d.matmul(&self.q_proj.t()?)?; // [seq, q_dim]
        let k = x_2d.matmul(&self.k_proj.t()?)?; // [seq, kv_dim]
        let v = x_2d.matmul(&self.v_proj.t()?)?; // [seq, kv_dim]

        // Reshape to [batch, seq, heads, head_dim]
        let q = q
            .unsqueeze(0)?
            .reshape((batch, seq_len, self.q_heads, self.head_dim))?;
        let k = k
            .unsqueeze(0)?
            .reshape((batch, seq_len, self.kv_heads, self.head_dim))?;
        let v = v
            .unsqueeze(0)?
            .reshape((batch, seq_len, self.kv_heads, self.head_dim))?;

        // QK-norm: RmsNorm per head on head_dim
        let q = rms_norm_last_dim(&q, &self.q_norm_weight, self.eps)?;
        let k = rms_norm_last_dim(&k, &self.k_norm_weight, self.eps)?;

        // RoPE
        let q = apply_rope(&q, seq_len, self.head_dim, self.rope_theta, x.device())?;
        let k = apply_rope(&k, seq_len, self.head_dim, self.rope_theta, x.device())?;

        // GQA: repeat KV heads to match Q heads
        let q = q.transpose(1, 2)?; // [batch, q_heads, seq, head_dim]
        let k = k.transpose(1, 2)?; // [batch, kv_heads, seq, head_dim]
        let v = v.transpose(1, 2)?; // [batch, kv_heads, seq, head_dim]

        let rep = self.q_heads / self.kv_heads;
        let k = repeat_kv(&k, rep)?;
        let v = repeat_kv(&v, rep)?;

        // Attention scores
        let scale = 1.0 / (self.head_dim as f64).sqrt();
        let scores = q.matmul(&k.transpose(2, 3)?)?; // [batch, q_heads, seq, seq]
        let scores = (scores * scale)?;

        // Causal mask
        let mask = causal_mask(seq_len, x.device())?;
        let scores = scores.broadcast_add(&mask)?;

        // Softmax
        let attn = candle_nn::ops::softmax_last_dim(&scores)?;

        // Attention output
        let out = attn.matmul(&v)?; // [batch, q_heads, seq, head_dim]
        let out = out.transpose(1, 2)?; // [batch, seq, q_heads, head_dim]
        let out = out.reshape((batch, seq_len, q_dim))?;
        // Squeeze batch for 2D matmul: [seq, q_dim]
        let out_2d = out.squeeze(0)?;

        // Output projection: out_2d [seq, q_dim] × w_t [q_dim, hidden] → [seq, hidden]
        let out = out_2d.matmul(&self.o_proj.t()?)?;
        let out = out.unsqueeze(0)?; // [1, seq, hidden]
        Ok(out)
    }

    fn mlp_forward(&self, x: &Tensor) -> CandleResult<Tensor> {
        let (batch, seq, _hidden) = x.dims3()?;
        let x_2d = x.squeeze(0)?; // [seq, hidden]
                                  // gate: [seq, hidden] × [hidden, intermediate] → [seq, intermediate]
        let gate = x_2d.matmul(&self.gate_proj.t()?)?; // gate_proj [intermediate, hidden] → .t() [hidden, intermediate]
        let up = x_2d.matmul(&self.up_proj.t()?)?;
        let gate = candle_nn::ops::silu(&gate)?;
        let hidden = (gate * up)?; // [seq, intermediate]
                                   // down: [seq, intermediate] × [intermediate, hidden] → [seq, hidden]
        let out = hidden.matmul(&self.down_proj.t()?)?; // down_proj [hidden, intermediate] → .t() [intermediate, hidden]
        let out = out.unsqueeze(0)?; // [1, seq, hidden]
        let _ = (batch, seq);
        Ok(out)
    }
}

/// Text encoder — Qwen3 architecture.
pub struct TextEncoder {
    token_embedding: Tensor,
    blocks: Vec<Qwen3Block>,
    final_norm_weight: Tensor,
    config: TextEncoderConfig,
    device: Device,
}

impl TextEncoder {
    /// Create a text encoder with deterministic seeded initialization.
    ///
    /// Uses a local xorshift64 copy (NOT the `src/nn` SSOT PRNG — contract
    /// diverges, see the module-level known-debt note; unification in №230).
    pub fn new(config: &TextEncoderConfig, seed: u64) -> Result<Self, String> {
        let device = Device::Cpu;
        let var_map = VarMap::new();
        let vb = VarBuilder::from_varmap(&var_map, DType::F32, &device);

        // Token embedding
        let emb_init = generate_uniform_f32(seed, config.vocab_size * config.hidden, -0.02, 0.02);
        let token_embedding =
            Tensor::from_vec(emb_init, (config.vocab_size, config.hidden), &device)
                .map_err(|e| format!("TextEncoder::new: token embedding init failed: {}", e))?;

        // Layers
        let mut blocks = Vec::with_capacity(config.layers);
        for i in 0..config.layers {
            let block = Qwen3Block::new(config, &var_map, &vb, i, seed)
                .map_err(|e| format!("TextEncoder::new: layer {} init failed: {}", i, e))?;
            blocks.push(block);
        }

        // Final norm
        let final_norm_weight = Tensor::ones((config.hidden,), DType::F32, &device)
            .map_err(|e| format!("TextEncoder::new: final norm init failed: {}", e))?;

        Ok(TextEncoder {
            token_embedding,
            blocks,
            final_norm_weight,
            config: config.clone(),
            device,
        })
    }

    /// Forward pass: token IDs → hidden states [seq_len, hidden].
    /// Returns the final-layer hidden states (LM head is NOT included —
    /// Qwen3 is used as an encoder, not a generator).
    pub fn forward(&self, token_ids: &[u32]) -> Result<Tensor, String> {
        let seq_len = token_ids.len();
        if seq_len == 0 {
            return Err("TextEncoder::forward: empty token_ids".to_string());
        }
        if seq_len > self.config.max_seq {
            return Err(format!(
                "TextEncoder::forward: seq_len {} exceeds max_seq {}",
                seq_len, self.config.max_seq
            ));
        }

        // Embedding lookup
        let ids = Tensor::from_vec(token_ids.to_vec(), (seq_len,), &self.device)
            .map_err(|e| format!("TextEncoder::forward: token_ids to tensor failed: {}", e))?;

        let mut x = self
            .token_embedding
            .embedding(&ids)
            .map_err(|e| format!("TextEncoder::forward: embedding lookup failed: {}", e))?;

        // x is [seq_len, hidden] — add batch dim for attention
        x = x
            .unsqueeze(0)
            .map_err(|e| format!("TextEncoder::forward: unsqueeze failed: {}", e))?;

        // Transformer blocks
        for block in &self.blocks {
            x = block
                .forward(&x, seq_len)
                .map_err(|e| format!("TextEncoder::forward: block forward failed: {}", e))?;
        }

        // Final norm
        x = rms_norm(&x, &self.final_norm_weight, self.config.rms_norm_eps)
            .map_err(|e| format!("TextEncoder::forward: final norm failed: {}", e))?;

        // Remove batch dim → [seq_len, hidden]
        let x = x
            .squeeze(0)
            .map_err(|e| format!("TextEncoder::forward: squeeze failed: {}", e))?;

        Ok(x)
    }

    /// Get the config used to create this encoder.
    pub fn config(&self) -> &TextEncoderConfig {
        &self.config
    }
}

// ── Helper functions (candle primitives, NOT from src/nn/) ───────────

/// RmsNorm: x / sqrt(mean(x^2) + eps) * weight
/// Operates on the last dimension.
fn rms_norm(x: &Tensor, weight: &Tensor, eps: f64) -> CandleResult<Tensor> {
    let x_f32 = x.to_dtype(DType::F32)?;
    let sq = (&x_f32 * &x_f32)?;
    let mean = sq.mean_keepdim(x_f32.dims().len() - 1)?;
    let eps_t = Tensor::full(eps as f32, mean.dims(), x.device())?;
    let norm = ((mean + eps_t)?).sqrt()?;
    // Divide element-wise: normed = x / norm (broadcast norm across last dim)
    let normed = x_f32.broadcast_div(&norm)?;
    // weight is [hidden] or [head_dim] — reshape and broadcast
    let ndims = normed.dims().len();
    let mut w_shape: Vec<usize> = vec![1; ndims];
    w_shape[ndims - 1] = weight.dims()[0];
    let w = weight.reshape(w_shape.as_slice())?;
    let w = w.broadcast_as(normed.dims())?;
    let result = (&normed * &w)?;
    Ok(result)
}

/// RmsNorm on the last dimension of a 4D tensor [batch, heads, seq, head_dim].
fn rms_norm_last_dim(x: &Tensor, weight: &Tensor, eps: f64) -> CandleResult<Tensor> {
    rms_norm(x, weight, eps)
}

/// RoPE (Rotary Position Embedding) — applies rotary encoding to Q or K.
/// Input: [batch, seq, heads, head_dim]
/// Output: same shape, with rotary encoding applied.
fn apply_rope(
    x: &Tensor,
    seq_len: usize,
    head_dim: usize,
    theta: f64,
    device: &Device,
) -> CandleResult<Tensor> {
    let half = head_dim / 2;
    let dims = x.dims();
    let batch = dims[0];
    let heads = dims[2];

    // Compute frequencies
    let mut freqs = Vec::with_capacity(half);
    for i in 0..half {
        freqs.push(1.0 / theta.powf((2.0 * i as f64) / head_dim as f64));
    }

    // Compute positions × frequencies
    let mut cos_vals = Vec::with_capacity(seq_len * half);
    let mut sin_vals = Vec::with_capacity(seq_len * half);
    for pos in 0..seq_len {
        for f in freqs.iter().take(half) {
            let angle = pos as f64 * f;
            cos_vals.push(angle.cos() as f32);
            sin_vals.push(angle.sin() as f32);
        }
    }

    let cos = Tensor::from_vec(cos_vals, (seq_len, half), device)?;
    let sin = Tensor::from_vec(sin_vals, (seq_len, half), device)?;

    // Expand cos/sin to match x shape [batch, seq, heads, half]
    let cos = cos.unsqueeze(0)?.unsqueeze(2)?; // [1, seq, 1, half]
    let cos = cos.broadcast_as((batch, seq_len, heads, half))?;
    let sin = sin.unsqueeze(0)?.unsqueeze(2)?;
    let sin = sin.broadcast_as((batch, seq_len, heads, half))?;

    // Split along last dim into two halves using narrow on dim 3 (head_dim)
    let x1 = x.narrow(3, 0, half)?; // [batch, seq, heads, half]
    let x2 = x.narrow(3, half, half)?;

    // Apply rotation: out1 = x1 * cos - x2 * sin, out2 = x1 * sin + x2 * cos
    let out1 = ((&x1 * &cos)? - (&x2 * &sin)?)?;
    let out2 = ((&x1 * &sin)? + (&x2 * &cos)?)?;

    // Concatenate back along last dim (dim 3)
    let out = Tensor::cat(&[&out1, &out2], 3)?;
    Ok(out)
}

/// Causal mask: upper-triangular -inf, lower-triangular 0.
/// Shape: [1, 1, seq, seq] for broadcasting.
fn causal_mask(seq_len: usize, device: &Device) -> CandleResult<Tensor> {
    let mut mask = vec![0.0f32; seq_len * seq_len];
    for i in 0..seq_len {
        for j in (i + 1)..seq_len {
            mask[i * seq_len + j] = f32::NEG_INFINITY;
        }
    }
    let mask = Tensor::from_vec(mask, (seq_len, seq_len), device)?;
    let mask = mask.unsqueeze(0)?; // [1, seq, seq]
    mask.unsqueeze(0) // [1, 1, seq, seq]
}

/// Repeat KV heads to match Q heads (GQA).
/// Input: [batch, kv_heads, seq, head_dim]
/// Output: [batch, q_heads, seq, head_dim]
fn repeat_kv(x: &Tensor, rep: usize) -> CandleResult<Tensor> {
    if rep == 1 {
        return Ok(x.clone());
    }
    let dims = x.dims();
    let batch = dims[0];
    let kv_heads = dims[1];
    let seq = dims[2];
    let head_dim = dims[3];
    let q_heads = kv_heads * rep;

    // Reshape [batch, kv_heads, 1, seq, head_dim] → repeat → reshape
    let x = x.unsqueeze(2)?;
    let x = x.broadcast_as((batch, kv_heads, rep, seq, head_dim))?;
    x.reshape((batch, q_heads, seq, head_dim))
}

/// Deterministic PRNG — xorshift64 core.
///
/// KNOWN DIVERGENCE from `src/nn/attention.rs::generate_uniform_f32` (the SSOT):
/// no `seed_to_state` XOR ritual and an f32 (not f64) mapping path — the value
/// streams differ from the `src/nn` PRNG given the same seed. Local copy is
/// scheduled for removal in №230 (PRNG SSOT unification); until then this is
/// the only value source for the text encoder and it is deterministic.
/// Returns n values in [lo, up) with deterministic ordering.
fn generate_uniform_f32(seed: u64, n: usize, lo: f32, up: f32) -> Vec<f32> {
    let mut state = seed;
    let range = up - lo;
    let mut result = Vec::with_capacity(n);
    for _ in 0..n {
        // xorshift64
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        // Map to [0, 1)
        let val = (state >> 11) as f32 / (1u64 << 53) as f32;
        result.push(lo + val * range);
    }
    result
}
