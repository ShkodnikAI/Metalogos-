//! Z-Image Transformer (DiT) — naryad №212 Block 4.1, rebuilt to reference in №232.
//!
//! Faithful implementation of the Z-Image-Turbo transformer following
//! `diffusers/models/transformers/transformer_z_image.py` (fetch 2026-09-08).
//!
//! ## Architecture (verified against diffusers source + safetensors headers)
//!
//! - ADALN_EMBED_DIM = 256 (constant, L32)
//! - t_embedder: TimestepEmbedder(min(dim, 256), mid_size=1024)
//!   → sinusoidal(256, max_period=10000) → Linear(256→1024) → SiLU → Linear(1024→out)
//! - Block adaLN: Linear(min(dim,256) → 4*dim), 4 chunks: scale_msa, gate_msa,
//!   scale_mlp, gate_mlp. gate=tanh(gate), scale=1+scale. NO shift.
//! - Block norms: 4 RMSNorms (attention_norm1, attention_norm2, ffn_norm1, ffn_norm2).
//!   Forward: attn_out = attention(attn_norm1(x) * scale_msa, freqs_cis);
//!   x = x + gate_msa * attn_norm2(attn_out);
//!   x = x + gate_mlp * ffn_norm2(ffn(ffn_norm1(x) * scale_mlp))
//! - FinalLayer: LayerNorm(dim, affine=False, eps=1e-6) → ×(1+scale) → Linear.
//!   No gate, no shift, no residual.
//! - cap_embedder: Sequential(RMSNorm(cap_feat_dim, eps), Linear(cap_feat_dim→dim, bias=True)).
//!   No SiLU.
//! - Refiners BEFORE main: noise_refiner (with adaLN, on x-tokens),
//!   context_refiner (without adaLN, on cap-tokens), THEN main layers.
//! - Unified sequence: [x, cap] (basic mode, x first).
//! - FeedForward: SwiGLU, hidden_dim = int(dim/3*8), bias-free.
//! - Axial RoPE: 3-axes (axes_dims), complex rotation applied AFTER qk-norm.
//! - Attention: MHA, non-causal, freqs_cis lookup by position IDs.

#![allow(non_snake_case)]
#![allow(clippy::all)]
#![allow(clippy::expect_used)]
#![allow(clippy::needless_borrow)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use std::collections::HashMap;

use candle_core::bail;
use candle_core::{DType, Device, Result as CandleResult, Tensor};

use crate::nn::attention::generate_uniform_f32;
use crate::vision::text_encoder::param_seed;

/// Helper: convert CandleError to String for `?` in String-returning functions.
fn s<T>(r: candle_core::Result<T>) -> Result<T, String> {
    r.map_err(|e| e.to_string())
}

/// ADALN_EMBED_DIM — constant from transformer_z_image.py L32.
const ADALN_EMBED_DIM: usize = 256;

/// TimestepEmbedder mid_size — constant from transformer_z_image.py L438.
const TIMESTEP_MID_SIZE: usize = 1024;

/// Sinusoidal frequency_embedding_size — constant from TimestepEmbedder default (L38).
const FREQ_EMBED_SIZE: usize = 256;

/// Sinusoidal max_period — constant from timestep_embedding default (L51).
const MAX_PERIOD: f64 = 10000.0;

// ── Seed derivation constants ──
const PARAM_DIT_PATCH_EMBED: u64 = 200;
const PARAM_DIT_CAP_EMBED: u64 = 201;
const PARAM_DIT_T_EMBED: u64 = 202;
const PARAM_DIT_LAYER: u64 = 210;
const PARAM_DIT_FINAL: u64 = 220;
const PARAM_DIT_REFINER: u64 = 230;
const PARAM_DIT_PAD_TOKEN: u64 = 240;

// ── Config ──

#[derive(Debug, Clone)]
pub struct ZImageConfig {
    pub dim: usize,
    pub n_layers: usize,
    pub n_heads: usize,
    pub n_kv_heads: usize,
    pub head_dim: usize,
    pub axes_dims: Vec<usize>,
    pub axes_lens: Vec<usize>,
    pub rope_theta: f64,
    pub in_channels: usize,
    pub patch_size: usize,
    pub cap_feat_dim: usize,
    pub qk_norm: bool,
    pub norm_eps: f64,
    pub t_scale: f64,
    pub n_refiner_layers: usize,
}

impl ZImageConfig {
    /// FeedForward hidden_dim = int(dim/3*8) — source: transformer_z_image.py L213.
    pub fn intermediate(&self) -> usize {
        (self.dim as f64 / 3.0 * 8.0) as usize
    }

    /// adaln_input dim = min(dim, ADALN_EMBED_DIM) — source: L224, L291, L438.
    pub fn adaln_dim(&self) -> usize {
        self.dim.min(ADALN_EMBED_DIM)
    }
}

pub fn zimage_turbo_config() -> ZImageConfig {
    ZImageConfig {
        dim: 3840,
        n_layers: 30,
        n_heads: 30,
        n_kv_heads: 30,
        head_dim: 128,
        axes_dims: vec![32, 48, 48],
        axes_lens: vec![1536, 512, 512],
        rope_theta: 256.0,
        in_channels: 16,
        patch_size: 2,
        cap_feat_dim: 2560,
        qk_norm: true,
        norm_eps: 1e-5,
        t_scale: 1000.0,
        n_refiner_layers: 2,
    }
}

pub fn tiny_dit_config() -> ZImageConfig {
    ZImageConfig {
        dim: 64,
        n_layers: 2,
        n_heads: 2,
        n_kv_heads: 2,
        head_dim: 32,
        axes_dims: vec![8, 12, 12],
        axes_lens: vec![8, 4, 4],
        rope_theta: 256.0,
        in_channels: 4,
        patch_size: 2,
        cap_feat_dim: 32,
        qk_norm: true,
        norm_eps: 1e-5,
        t_scale: 1000.0,
        n_refiner_layers: 1,
    }
}

// ── Linear helpers ──

fn linear_seeded(
    in_ch: usize,
    out_ch: usize,
    seed: u64,
    device: &Device,
    use_bias: bool,
) -> Result<(Tensor, Option<Tensor>), String> {
    let weight_init = generate_uniform_f32(seed, out_ch * in_ch, -0.02, 0.02);
    let weight = Tensor::from_vec(weight_init, (out_ch, in_ch), device)
        .map_err(|e| format!("linear_seeded: weight: {}", e))?;
    let bias = if use_bias {
        let bias_init = generate_uniform_f32(seed.wrapping_add(1), out_ch, -0.02, 0.02);
        let b = Tensor::from_vec(bias_init, (out_ch,), device)
            .map_err(|e| format!("linear_seeded: bias: {}", e))?;
        Some(b)
    } else {
        None
    };
    Ok((weight, bias))
}

fn linear_forward(x: &Tensor, weight: &Tensor, bias: Option<&Tensor>) -> CandleResult<Tensor> {
    // Reshape 3D → 2D for matmul (candle 0.11 doesn't broadcast 3D @ 2D).
    // Also handle 1D → 2D (unsqueeze) and reshape back.
    let x_ndims = x.dims().len();
    let (x_2d, batch_shape) = if x_ndims > 2 {
        let dims = x.dims();
        let last = dims[dims.len() - 1];
        let rest: usize = dims[..dims.len() - 1].iter().product();
        (x.reshape((rest, last))?, Some(dims.to_vec()))
    } else if x_ndims == 1 {
        // 1D [dim] → [1, dim]
        let dims = x.dims();
        (x.reshape((1, dims[0]))?, Some(vec![dims[0]]))
    } else {
        (x.clone(), None)
    };
    let out = x_2d.matmul(&weight.t()?)?;
    let out = if let Some(ref orig_dims) = batch_shape {
        if orig_dims.len() == 1 {
            // 1D input → squeeze back to 1D
            out.squeeze(0)?
        } else {
            // 3D+ input → reshape back to original shape
            let out_dims = out.dims();
            let mut new_dims = orig_dims[..orig_dims.len() - 1].to_vec();
            new_dims.push(out_dims[out_dims.len() - 1]);
            out.reshape(new_dims.as_slice())?
        }
    } else {
        out
    };
    if let Some(b) = bias {
        let b_dim = b.dim(0)?;
        let ndims = out.dims().len();
        let mut b_shape: Vec<usize> = vec![1; ndims];
        b_shape[ndims - 1] = b_dim;
        let b_broadcast = b.reshape(b_shape.as_slice())?.broadcast_as(out.dims())?;
        out + b_broadcast
    } else {
        Ok(out)
    }
}

// ── RMSNorm helper ──

fn rms_norm_last_dim(x: &Tensor, weight: &Tensor, eps: f64) -> CandleResult<Tensor> {
    let x_f32 = x.to_dtype(DType::F32)?;
    let ndims = x_f32.dims().len();
    let sq = (&x_f32 * &x_f32)?;
    let mean = sq.mean_keepdim(ndims - 1)?;
    let eps_t = Tensor::full(eps as f32, mean.dims(), x.device())?;
    let denom = (&mean + eps_t)?.sqrt()?;
    let normed = x_f32.broadcast_div(&denom)?;
    let mut w_shape: Vec<usize> = vec![1; ndims];
    w_shape[ndims - 1] = weight.dims()[0];
    let w = weight
        .reshape(w_shape.as_slice())?
        .broadcast_as(normed.dims())?;
    Ok((&normed * &w)?)
}

// ── LayerNorm (affine=False) — source: transformer_z_image.py L286 ──

fn layernorm_no_affine(x: &Tensor, eps: f64) -> CandleResult<Tensor> {
    let x_f32 = x.to_dtype(DType::F32)?;
    let ndims = x_f32.dims().len();
    let mean = x_f32.mean_keepdim(ndims - 1)?;
    let mean_b = mean.broadcast_as(x_f32.dims())?;
    let centered = (&x_f32 - &mean_b)?;
    let sq = (&centered * &centered)?;
    let var = sq.mean_keepdim(ndims - 1)?;
    let var_b = var.broadcast_as(x_f32.dims())?;
    let eps_t = Tensor::full(eps as f32, var_b.dims(), x.device())?;
    let denom = (&var_b + eps_t)?.sqrt()?;
    centered.broadcast_div(&denom)
}

// ── SiLU ──

fn silu(x: &Tensor) -> CandleResult<Tensor> {
    candle_nn::ops::silu(x)
}

fn tanh_tensor(x: &Tensor) -> CandleResult<Tensor> {
    // tanh(x) = 2*sigmoid(2x) - 1
    let two = Tensor::full(2.0f32, x.dims(), x.device())?;
    let two_x = (x * &two)?;
    let sig = candle_nn::ops::sigmoid(&two_x)?;
    let two_sig = (&sig * &two)?;
    let one = Tensor::full(1.0f32, two_sig.dims(), x.device())?;
    &two_sig - &one
}

// ── Axial RoPE ──

struct AxialRoPE {
    /// Precomputed cos/sin per axis. Each entry: [axes_lens[i], axes_dims[i]/2].
    cos: Vec<Tensor>,
    sin: Vec<Tensor>,
    axes_dims: Vec<usize>,
    axes_lens: Vec<usize>,
    theta: f64,
}

impl AxialRoPE {
    fn new(config: &ZImageConfig, device: &Device) -> Result<Self, String> {
        let mut cos_vec = Vec::new();
        let mut sin_vec = Vec::new();
        for i in 0..config.axes_dims.len() {
            let d = config.axes_dims[i];
            let end = config.axes_lens[i];
            let half = d / 2;
            // freqs = 1 / theta^(arange(0, d, 2)/d) — source L331
            let freqs: Vec<f32> = (0..half)
                .map(|j| {
                    let exponent = (2.0 * j as f64) / d as f64;
                    (1.0 / config.rope_theta.powf(exponent)) as f32
                })
                .collect();
            // angles = outer(arange(0, end), freqs) — source L333
            let mut cos_vals = Vec::with_capacity(end * half);
            let mut sin_vals = Vec::with_capacity(end * half);
            for pos in 0..end {
                for f in &freqs {
                    let angle = pos as f64 * *f as f64;
                    cos_vals.push(angle.cos() as f32);
                    sin_vals.push(angle.sin() as f32);
                }
            }
            let cos_t = Tensor::from_vec(cos_vals, (end, half), device)
                .map_err(|e| format!("RoPE cos[{}]: {}", i, e))?;
            let sin_t = Tensor::from_vec(sin_vals, (end, half), device)
                .map_err(|e| format!("RoPE sin[{}]: {}", i, e))?;
            cos_vec.push(cos_t);
            sin_vec.push(sin_t);
        }
        Ok(AxialRoPE {
            cos: cos_vec,
            sin: sin_vec,
            axes_dims: config.axes_dims.clone(),
            axes_lens: config.axes_lens.clone(),
            theta: config.rope_theta,
        })
    }

    /// Apply rotary embedding to Q or K.
    /// x: [batch, heads, seq, head_dim], pos_ids: [seq, 3] (t, h, w per token).
    fn apply(&self, x: &Tensor, pos_ids: &[(usize, usize, usize)]) -> CandleResult<Tensor> {
        // x: [batch, heads, seq, head_dim]
        let dims = x.dims();
        let batch = dims[0];
        let heads = dims[1];
        let seq = dims[2];
        let head_dim = dims[3];

        // For each token in the sequence, look up cos/sin for all 3 axes
        // and concatenate → [seq, head_dim/2]
        let total_half = head_dim / 2;
        let mut cos_all = Vec::with_capacity(seq * total_half);
        let mut sin_all = Vec::with_capacity(seq * total_half);
        for &(t, h, w) in pos_ids {
            for (axis, &pos) in [t, h, w].iter().enumerate() {
                let axis_half = self.axes_dims[axis] / 2;
                // n233: loud Err on out-of-range pos — reference does not clamp
                // (axes_lens must cover all positions). Clamp would silently
                // produce wrong RoPE for long sequences.
                if pos >= self.axes_lens[axis] {
                    bail!(
                        "AxialRoPE::apply: pos {} on axis {} exceeds axes_lens[{}] = {} \
                         — reference does not clamp; check pos_ids builder",
                        pos,
                        axis,
                        axis,
                        self.axes_lens[axis]
                    );
                }
                let cos_row = self.cos[axis].get(pos)?;
                let sin_row = self.sin[axis].get(pos)?;
                let cos_vals = cos_row.to_vec1::<f32>()?;
                let sin_vals = sin_row.to_vec1::<f32>()?;
                cos_all.extend_from_slice(&cos_vals);
                sin_all.extend_from_slice(&sin_vals);
            }
        }
        // Verify total_half matches sum of axes_dims/2
        let expected_half: usize = self.axes_dims.iter().map(|d| d / 2).sum();
        if expected_half != total_half {
            bail!(
                "RoPE: axes_dims/2 sum {} != head_dim/2 {}",
                expected_half,
                total_half
            );
        }

        let device = x.device();
        let cos_t = Tensor::from_vec(cos_all, (seq, total_half), device)?;
        let sin_t = Tensor::from_vec(sin_all, (seq, total_half), device)?;

        // Broadcast cos/sin to [batch, heads, seq, half]
        let cos_b = cos_t
            .unsqueeze(0)?
            .unsqueeze(0)?
            .broadcast_as((batch, heads, seq, total_half))?;
        let sin_b = sin_t
            .unsqueeze(0)?
            .unsqueeze(0)?
            .broadcast_as((batch, heads, seq, total_half))?;

        // Split head_dim into pairs: x[..., 0::2] and x[..., 1::2]
        // (interleaved: even indices are real, odd are imag)
        // Source: view_as_complex(x.reshape(..., -1, 2)) — pairs are consecutive
        let x_pairs = x.reshape((batch, heads, seq, total_half, 2))?;
        let x_real = x_pairs.narrow(4, 0, 1)?.squeeze(4)?;
        let x_imag = x_pairs.narrow(4, 1, 1)?.squeeze(4)?;

        // Complex multiply: (x_real + i*x_imag) * (cos + i*sin)
        // = (x_real*cos - x_imag*sin) + i*(x_real*sin + x_imag*cos)
        let out_real = (&(&x_real * &cos_b)? - &(&x_imag * &sin_b)?)?;
        let out_imag = (&(&x_real * &sin_b)? + &(&x_imag * &cos_b)?)?;

        // Interleave back: [batch, heads, seq, total_half, 2] → [batch, heads, seq, head_dim]
        let out = Tensor::stack(&[&out_real, &out_imag], 4)?;
        out.reshape((batch, heads, seq, head_dim))
    }
}

// ── FeedForward (SwiGLU, bias-free) ──

struct FeedForward {
    w1: Tensor, // [hidden, dim]
    w2: Tensor, // [dim, hidden]
    w3: Tensor, // [hidden, dim]
}

impl FeedForward {
    fn forward(&self, x: &Tensor) -> CandleResult<Tensor> {
        // w2(silu(w1(x)) * w3(x)) — source: L179-180
        let gate = linear_forward(x, &self.w1, None)?;
        let up = linear_forward(x, &self.w3, None)?;
        let gate = silu(&gate)?;
        let hidden = (&gate * &up)?;
        linear_forward(&hidden, &self.w2, None)
    }
}

// ── DiT Block ──

struct DiTBlock {
    modulation: bool,
    adaLN: Option<(Tensor, Option<Tensor>)>, // Linear(adaln_dim → 4*dim)
    attention_norm1: Tensor,
    attention_norm2: Tensor,
    ffn_norm1: Tensor,
    ffn_norm2: Tensor,
    to_q: Tensor,
    to_k: Tensor,
    to_v: Tensor,
    to_out: Tensor,
    norm_q: Tensor,
    norm_k: Tensor,
    feed_forward: FeedForward,
    n_heads: usize,
    n_kv_heads: usize,
    head_dim: usize,
    dim: usize,
    eps: f64,
}

impl DiTBlock {
    fn forward(
        &self,
        x: &Tensor,
        pos_ids: &[(usize, usize, usize)],
        adaln_input: Option<&Tensor>,
        rope: &AxialRoPE,
    ) -> CandleResult<Tensor> {
        if self.modulation {
            let adaln = adaln_input.expect("adaLN required when modulation=True");
            // mod = adaLN(adaln_input) → [batch, 4*dim] — source: L259-260
            let (ref ada_w, ref ada_b) = self.adaLN.as_ref().expect("adaLN weights");
            let mod_ = linear_forward(adaln, ada_w, ada_b.as_ref())?;
            // chunk(4, dim=-1) → scale_msa, gate_msa, scale_mlp, gate_mlp — source: L260
            let dims = mod_.dims();
            let dim = self.dim;
            let chunks = mod_.chunk(4, dims.len() - 1)?;
            let scale_msa = &chunks[0];
            let gate_msa = &chunks[1];
            let scale_mlp = &chunks[2];
            let gate_mlp = &chunks[3];

            // gate = tanh(gate), scale = 1 + scale — source: L261-262
            let gate_msa = tanh_tensor(gate_msa)?;
            let gate_mlp = tanh_tensor(gate_mlp)?;
            let one_t = Tensor::full(1.0f32, scale_msa.dims(), scale_msa.device())?;
            let scale_msa = (scale_msa + &one_t)?;
            let one_t2 = Tensor::full(1.0f32, scale_mlp.dims(), scale_mlp.device())?;
            let scale_mlp = (scale_mlp + &one_t2)?;

            // Broadcast scale/gate to match x dims: [batch, dim] → [batch, 1, dim]
            let x_ndims = x.dims().len();
            let scale_msa_b = scale_msa
                .reshape({
                    let mut s = vec![1; x_ndims];
                    s[x_ndims - 1] = dim;
                    s
                })?
                .broadcast_as(x.dims())?;
            let gate_msa_b = gate_msa
                .reshape({
                    let mut s = vec![1; x_ndims];
                    s[x_ndims - 1] = dim;
                    s
                })?
                .broadcast_as(x.dims())?;
            let scale_mlp_b = scale_mlp
                .reshape({
                    let mut s = vec![1; x_ndims];
                    s[x_ndims - 1] = dim;
                    s
                })?
                .broadcast_as(x.dims())?;
            let gate_mlp_b = gate_mlp
                .reshape({
                    let mut s = vec![1; x_ndims];
                    s[x_ndims - 1] = dim;
                    s
                })?
                .broadcast_as(x.dims())?;

            // Attention block:
            // attn_out = attention(attention_norm1(x) * scale_msa, freqs_cis) — source: L265-266
            let normed = rms_norm_last_dim(x, &self.attention_norm1, self.eps)?;
            let normed = (&normed * &scale_msa_b)?;
            let attn_out = self.attention_forward(&normed, pos_ids, rope)?;
            // x = x + gate_msa * attention_norm2(attn_out) — source: L268
            let attn_normed = rms_norm_last_dim(&attn_out, &self.attention_norm2, self.eps)?;
            let x = (x + (&gate_msa_b * &attn_normed)?)?;

            // FFN block:
            // x = x + gate_mlp * ffn_norm2(ffn(ffn_norm1(x) * scale_mlp)) — source: L271
            let normed = rms_norm_last_dim(&x, &self.ffn_norm1, self.eps)?;
            let normed = (&normed * &scale_mlp_b)?;
            let ffn_out = self.feed_forward.forward(&normed)?;
            let ffn_normed = rms_norm_last_dim(&ffn_out, &self.ffn_norm2, self.eps)?;
            let x = (x + (&gate_mlp_b * &ffn_normed)?)?;

            Ok(x)
        } else {
            // No modulation (context_refiner) — source: L273-278
            let attn_out = self.attention_forward(
                &rms_norm_last_dim(x, &self.attention_norm1, self.eps)?,
                pos_ids,
                rope,
            )?;
            let x = (x + rms_norm_last_dim(&attn_out, &self.attention_norm2, self.eps)?)?;
            let ffn_out =
                self.feed_forward
                    .forward(&rms_norm_last_dim(&x, &self.ffn_norm1, self.eps)?)?;
            let x = (x + rms_norm_last_dim(&ffn_out, &self.ffn_norm2, self.eps)?)?;
            Ok(x)
        }
    }

    fn attention_forward(
        &self,
        x: &Tensor,
        pos_ids: &[(usize, usize, usize)],
        rope: &AxialRoPE,
    ) -> CandleResult<Tensor> {
        // x: [batch, seq, dim]
        let batch = x.dim(0)?;
        let seq = x.dim(1)?;
        let q_dim = self.n_heads * self.head_dim;
        let kv_dim = self.n_kv_heads * self.head_dim;

        let q = linear_forward(x, &self.to_q, None)?; // [batch, seq, q_dim]
        let k = linear_forward(x, &self.to_k, None)?;
        let v = linear_forward(x, &self.to_v, None)?;

        // Reshape to [batch, seq, heads, head_dim] → [batch, heads, seq, head_dim]
        let q = q
            .reshape((batch, seq, self.n_heads, self.head_dim))?
            .transpose(1, 2)?;
        let k = k
            .reshape((batch, seq, self.n_kv_heads, self.head_dim))?
            .transpose(1, 2)?;
        let v = v
            .reshape((batch, seq, self.n_kv_heads, self.head_dim))?
            .transpose(1, 2)?;

        // QK-norm (RMSNorm per head_dim, eps=1e-5) — BEFORE RoPE — source: L107-110
        let q = rms_norm_last_dim(&q, &self.norm_q, 1e-5)?;
        let k = rms_norm_last_dim(&k, &self.norm_k, 1e-5)?;

        // Apply axial RoPE to Q and K AFTER qk-norm — source: L120-122
        let q = rope.apply(&q, pos_ids)?;
        let k = rope.apply(&k, pos_ids)?;

        // GQA: repeat KV if needed (n_kv_heads == n_heads for Z-Image, so no repeat)
        let k = if self.n_kv_heads == self.n_heads {
            k
        } else {
            let rep = self.n_heads / self.n_kv_heads;
            let dims = k.dims();
            let b = dims[0];
            let kv = dims[1];
            let s = dims[2];
            let hd = dims[3];
            let k = k.unsqueeze(2)?.broadcast_as((b, kv, rep, s, hd))?;
            k.reshape((b, kv * rep, s, hd))?
        };
        let v = if self.n_kv_heads == self.n_heads {
            v
        } else {
            let rep = self.n_heads / self.n_kv_heads;
            let dims = v.dims();
            let b = dims[0];
            let kv = dims[1];
            let s = dims[2];
            let hd = dims[3];
            let v = v.unsqueeze(2)?.broadcast_as((b, kv, rep, s, hd))?;
            v.reshape((b, kv * rep, s, hd))?
        };

        // Attention scores: Q @ K^T / sqrt(head_dim) — non-causal — source: L133-142
        let scale = 1.0 / (self.head_dim as f64).sqrt();
        let scores = q.matmul(&k.transpose(2, 3)?)?;
        let scale_t = Tensor::full(scale as f32, scores.dims(), x.device())?;
        let scores = (scores * scale_t)?;
        let attn = candle_nn::ops::softmax_last_dim(&scores)?;
        let out = attn.matmul(&v)?; // [batch, heads, seq, head_dim]

        // Reshape back → [batch, seq, q_dim] → to_out
        let out = out.transpose(1, 2)?.reshape((batch, seq, q_dim))?;
        linear_forward(&out, &self.to_out, None)
    }
}

// ── ZImageTransformer ──

pub struct ZImageTransformer {
    x_embedder: Tensor, // [dim, pp] where pp = patch^2 * in_channels
    x_embedder_bias: Option<Tensor>,
    x_pad_token: Tensor,         // [1, dim]
    cap_embedder_norm: Tensor,   // [cap_feat_dim] — RMSNorm gain
    cap_embedder_linear: Tensor, // [dim, cap_feat_dim]
    cap_embedder_bias: Option<Tensor>,
    cap_pad_token: Tensor,   // [1, dim]
    t_embedder_mlp0: Tensor, // [1024, 256]
    t_embedder_mlp0_bias: Option<Tensor>,
    t_embedder_mlp2: Tensor, // [out_size, 1024]
    t_embedder_mlp2_bias: Option<Tensor>,
    noise_refiner: Vec<DiTBlock>,   // modulation=True
    context_refiner: Vec<DiTBlock>, // modulation=False
    layers: Vec<DiTBlock>,          // modulation=True
    final_norm: (),                 // LayerNorm(affine=False) — no weights
    final_adaLN: Tensor,            // [dim, adaln_dim]
    final_adaLN_bias: Option<Tensor>,
    final_linear: Tensor, // [pp, dim]
    final_linear_bias: Option<Tensor>,
    rope: AxialRoPE,
    config: ZImageConfig,
}

impl ZImageTransformer {
    /// Seeded tiny-init for CI goldens.
    pub fn new_tiny(config: &ZImageConfig, seed: u64) -> Result<Self, String> {
        let device = Device::Cpu;
        let dim = config.dim;
        let pp = config.patch_size * config.patch_size * config.in_channels;
        let adaln_dim = config.adaln_dim();
        let inter = config.intermediate();

        // x_embedder: Linear(pp → dim, bias=True) — source: L402
        let (x_embedder, x_embedder_bias) = linear_seeded(
            pp,
            dim,
            param_seed(seed, 0, PARAM_DIT_PATCH_EMBED),
            &device,
            true,
        )?;
        // x_pad_token: [1, dim] — source: L466
        let x_pad_token =
            Tensor::zeros((1, dim), DType::F32, &device).map_err(|e| e.to_string())?;
        // cap_embedder: RMSNorm(cap_feat_dim) + Linear(cap_feat_dim→dim, bias=True) — source: L439
        let cap_embedder_norm =
            Tensor::ones((config.cap_feat_dim,), DType::F32, &device).map_err(|e| e.to_string())?;
        let (cap_embedder_linear, cap_embedder_bias) = linear_seeded(
            config.cap_feat_dim,
            dim,
            param_seed(seed, 0, PARAM_DIT_CAP_EMBED),
            &device,
            true,
        )?;
        // cap_pad_token: [1, dim] — source: L467
        let cap_pad_token =
            Tensor::zeros((1, dim), DType::F32, &device).map_err(|e| e.to_string())?;

        // t_embedder: Linear(256→1024) + SiLU + Linear(1024→adaln_dim) — source: L42-45, L438
        let (t_mlp0, t_mlp0_bias) = linear_seeded(
            FREQ_EMBED_SIZE,
            TIMESTEP_MID_SIZE,
            param_seed(seed, 0, PARAM_DIT_T_EMBED),
            &device,
            true,
        )?;
        let (t_mlp2, t_mlp2_bias) = linear_seeded(
            TIMESTEP_MID_SIZE,
            adaln_dim,
            param_seed(seed, 2, PARAM_DIT_T_EMBED),
            &device,
            true,
        )?;

        // noise_refiner (modulation=True) — source: L410-423
        let mut noise_refiner = Vec::new();
        for i in 0..config.n_refiner_layers {
            noise_refiner.push(build_block_seeded(
                config,
                param_seed(seed, i as u64, PARAM_DIT_REFINER),
                &device,
                true,
            )?);
        }
        // context_refiner (modulation=False) — source: L424-437
        let mut context_refiner = Vec::new();
        for i in 0..config.n_refiner_layers {
            context_refiner.push(build_block_seeded(
                config,
                param_seed(seed, (i + 100) as u64, PARAM_DIT_REFINER),
                &device,
                false,
            )?);
        }
        // main layers — source: L469-474
        let mut layers = Vec::new();
        for i in 0..config.n_layers {
            layers.push(build_block_seeded(
                config,
                param_seed(seed, i as u64, PARAM_DIT_LAYER),
                &device,
                true,
            )?);
        }

        // Final layer: adaLN = Linear(adaln_dim → dim, bias=True) — source: L289-292
        let (final_adaLN, final_adaLN_bias) = linear_seeded(
            adaln_dim,
            dim,
            param_seed(seed, 0, PARAM_DIT_FINAL),
            &device,
            true,
        )?;
        // Final linear: Linear(dim → pp, bias=True) — source: L287
        let (final_linear, final_linear_bias) =
            linear_seeded(dim, pp, param_seed(seed, 1, PARAM_DIT_FINAL), &device, true)?;

        let rope = AxialRoPE::new(config, &device)?;

        Ok(ZImageTransformer {
            x_embedder,
            x_embedder_bias,
            x_pad_token,
            cap_embedder_norm,
            cap_embedder_linear,
            cap_embedder_bias,
            cap_pad_token,
            t_embedder_mlp0: t_mlp0,
            t_embedder_mlp0_bias: t_mlp0_bias,
            t_embedder_mlp2: t_mlp2,
            t_embedder_mlp2_bias: t_mlp2_bias,
            noise_refiner,
            context_refiner,
            layers,
            final_norm: (),
            final_adaLN,
            final_adaLN_bias,
            final_linear,
            final_linear_bias,
            rope,
            config: config.clone(),
        })
    }

    /// Build from real safetensors tensors.
    pub fn from_weights(tensors: &HashMap<String, Tensor>) -> Result<Self, String> {
        let config = zimage_turbo_config();
        let device = Device::Cpu;

        let get = |name: &str, shape: &[usize]| -> Result<Tensor, String> {
            let t = tensors.get(name).ok_or_else(|| {
                format!(
                    "ZImageTransformer::from_weights: tensor '{}' not found",
                    name
                )
            })?;
            let t = t.to_device(&device).map_err(|e| format!("device: {}", e))?;
            let t = t
                .to_dtype(DType::F32)
                .map_err(|e| format!("dtype: {}", e))?;
            if t.dims() != shape {
                return Err(format!(
                    "ZImageTransformer: '{}' shape mismatch — expected {:?}, got {:?}",
                    name,
                    shape,
                    t.dims()
                ));
            }
            Ok(t)
        };

        let get_opt = |name: &str, shape: &[usize]| -> Result<Option<Tensor>, String> {
            match tensors.get(name) {
                None => Ok(None),
                Some(t) => {
                    let t = t.to_device(&device).map_err(|e| format!("device: {}", e))?;
                    let t = t
                        .to_dtype(DType::F32)
                        .map_err(|e| format!("dtype: {}", e))?;
                    if t.dims() != shape {
                        return Err(format!(
                            "ZImageTransformer: '{}' shape mismatch — expected {:?}, got {:?}",
                            name,
                            shape,
                            t.dims()
                        ));
                    }
                    Ok(Some(t))
                }
            }
        };

        let dim = config.dim;
        let pp = config.patch_size * config.patch_size * config.in_channels;
        let adaln_dim = config.adaln_dim();

        let x_embedder = get("all_x_embedder.2-1.weight", &[dim, pp])?;
        let x_embedder_bias = get_opt("all_x_embedder.2-1.bias", &[dim])?;
        let x_pad_token = get("x_pad_token", &[1, dim])?;
        let cap_embedder_norm = get("cap_embedder.0.weight", &[config.cap_feat_dim])?;
        let cap_embedder_linear = get("cap_embedder.1.weight", &[dim, config.cap_feat_dim])?;
        let cap_embedder_bias = get_opt("cap_embedder.1.bias", &[dim])?;
        let cap_pad_token = get("cap_pad_token", &[1, dim])?;

        let t_mlp0 = get(
            "t_embedder.mlp.0.weight",
            &[TIMESTEP_MID_SIZE, FREQ_EMBED_SIZE],
        )?;
        let t_mlp0_bias = get("t_embedder.mlp.0.bias", &[TIMESTEP_MID_SIZE])?;
        let t_mlp2 = get("t_embedder.mlp.2.weight", &[adaln_dim, TIMESTEP_MID_SIZE])?;
        let t_mlp2_bias = get("t_embedder.mlp.2.bias", &[adaln_dim])?;

        let mut noise_refiner = Vec::new();
        for i in 0..config.n_refiner_layers {
            noise_refiner.push(build_block_from_weights(
                tensors,
                &format!("noise_refiner.{}", i),
                &config,
                &device,
                true,
                &get,
                &get_opt,
            )?);
        }
        let mut context_refiner = Vec::new();
        for i in 0..config.n_refiner_layers {
            context_refiner.push(build_block_from_weights(
                tensors,
                &format!("context_refiner.{}", i),
                &config,
                &device,
                false,
                &get,
                &get_opt,
            )?);
        }
        let mut layers = Vec::new();
        for i in 0..config.n_layers {
            layers.push(build_block_from_weights(
                tensors,
                &format!("layers.{}", i),
                &config,
                &device,
                true,
                &get,
                &get_opt,
            )?);
        }

        let final_adaLN = get(
            "all_final_layer.2-1.adaLN_modulation.1.weight",
            &[dim, adaln_dim],
        )?;
        let final_adaLN_bias = get("all_final_layer.2-1.adaLN_modulation.1.bias", &[dim])?;
        let final_linear = get("all_final_layer.2-1.linear.weight", &[pp, dim])?;
        let final_linear_bias = get("all_final_layer.2-1.linear.bias", &[pp])?;

        let rope = AxialRoPE::new(&config, &device)?;

        // n233 Block 2: loader tensor-coverage guard.
        // Verify all loaded tensors are consumed and none are missing.
        // Expected count: 521 for Z-Image-Turbo (verified from index.json 2026-09-08).
        let loaded_count = tensors.len();
        // The expected count is: embedders(7) + t_embedder(4) + final_layer(4) +
        // per-block (13 for non-modulation, 15 for modulation) × blocks.
        // For Z-Image-Turbo: 30 layers (15 each) + 2 noise_refiner (15) + 2 context_refiner (13) = 521.
        // We check the exact count + key presence.
        let expected_count = 7 + 4 + 4
            + config.n_layers * 15
            + config.n_refiner_layers * 15  // noise_refiner (modulation=true)
            + config.n_refiner_layers * 13; // context_refiner (modulation=false)
        if loaded_count != expected_count {
            return Err(format!(
                "ZImageTransformer::from_weights: tensor count mismatch — expected {}, got {}",
                expected_count, loaded_count
            ));
        }

        Ok(ZImageTransformer {
            x_embedder,
            x_embedder_bias,
            x_pad_token,
            cap_embedder_norm,
            cap_embedder_linear,
            cap_embedder_bias,
            cap_pad_token,
            t_embedder_mlp0: t_mlp0,
            t_embedder_mlp0_bias: Some(t_mlp0_bias),
            t_embedder_mlp2: t_mlp2,
            t_embedder_mlp2_bias: Some(t_mlp2_bias),
            noise_refiner,
            context_refiner,
            layers,
            final_norm: (),
            final_adaLN,
            final_adaLN_bias: Some(final_adaLN_bias),
            final_linear,
            final_linear_bias: Some(final_linear_bias),
            rope,
            config,
        })
    }

    /// Forward pass: latent [1, C, H, W] + cap [cap_seq, cap_feat_dim] + timestep → velocity [1, C, H, W].
    pub fn forward(&self, latent: &Tensor, cap: &Tensor, t: f64) -> Result<Tensor, String> {
        let device = latent.device();
        let (b, c, h, w) = latent.dims4().map_err(|e| format!("DiT: dims4: {}", e))?;

        let ph = self.config.patch_size;
        let pw = self.config.patch_size;
        let nph = h / ph;
        let npw = w / pw;

        // t-embedding: t * t_scale → sinusoidal(256) → Linear(256→1024) → SiLU → Linear(1024→adaln_dim) — source: L945, L63-72
        let t_scaled = t * self.config.t_scale;
        let t_freq = sinusoidal_embedding(t_scaled, FREQ_EMBED_SIZE, MAX_PERIOD, device)
            .map_err(|e| format!("DiT: sinusoidal: {}", e))?;
        let t_hidden = linear_forward(
            &t_freq,
            &self.t_embedder_mlp0,
            self.t_embedder_mlp0_bias.as_ref(),
        )
        .map_err(|e| format!("DiT: t_mlp0: {}", e))?;
        let t_hidden = silu(&t_hidden).map_err(|e| format!("DiT: t silu: {}", e))?;
        let adaln_input = linear_forward(
            &t_hidden,
            &self.t_embedder_mlp2,
            self.t_embedder_mlp2_bias.as_ref(),
        )
        .map_err(|e| format!("DiT: t_mlp2: {}", e))?;
        // adaln_input: [adaln_dim] → unsqueeze to [1, adaln_dim] for batch
        let adaln_input = adaln_input
            .unsqueeze(0)
            .map_err(|e| format!("DiT: adaln.unsqueeze: {}", e))?;

        // Patchify: [1, C, H, W] → [nph*npw, pp] → x_embedder → [nph*npw, dim]
        let img_patches = patchify(latent, ph, pw).map_err(|e| format!("DiT: patchify: {}", e))?;
        let x = linear_forward(
            &img_patches,
            &self.x_embedder,
            self.x_embedder_bias.as_ref(),
        )
        .map_err(|e| format!("DiT: x_embed: {}", e))?;

        // x pos_ids: (cap_len+1, 0, 0) start → grid (1, nph, npw) — source: L608, L565
        // For tiny: cap_seq=4, so x_t_start=5, h/w=0..nph-1/npw-1
        // We need cap_len first; for the test, cap is [4, 32] so cap_len=4
        let cap_len = cap.dim(0).map_err(|e| format!("DiT: cap.dim(0): {}", e))?;
        let x_t_start = cap_len + 1;
        let x_pos_ids: Vec<(usize, usize, usize)> = (0..nph)
            .flat_map(|h_i| (0..npw).map(move |w_i| (x_t_start, h_i, w_i)))
            .collect();

        // Add batch dim to x
        let x = x
            .unsqueeze(0)
            .map_err(|e| format!("DiT: x.unsqueeze: {}", e))?; // [1, nph*npw, dim]

        // noise_refiner (modulation=True) on x — source: L985-992
        let mut x = x;
        for layer in &self.noise_refiner {
            x = layer
                .forward(&x, &x_pos_ids, Some(&adaln_input), &self.rope)
                .map_err(|e| format!("DiT: noise_refiner: {}", e))?;
        }

        // cap_embedder: RMSNorm(cap_feat_dim) → Linear(cap_feat_dim→dim) — source: L996, L439
        let cap_normed = rms_norm_last_dim(
            &cap.unsqueeze(0)
                .map_err(|e| format!("DiT: cap.unsqueeze: {}", e))?,
            &self.cap_embedder_norm,
            self.config.norm_eps,
        )
        .map_err(|e| format!("DiT: cap rms: {}", e))?;
        let cap_normed = cap_normed
            .squeeze(0)
            .map_err(|e| format!("DiT: cap.squeeze: {}", e))?; // [cap_seq, cap_feat_dim]
        let cap_emb = linear_forward(
            &cap_normed,
            &self.cap_embedder_linear,
            self.cap_embedder_bias.as_ref(),
        )
        .map_err(|e| format!("DiT: cap_embed: {}", e))?;
        // cap_emb: [cap_seq, dim]

        // cap pos_ids: start=(1, 0, 0), grid=(padded_cap_len, 1, 1) — source: L599, L565
        // n233: per-token (i+1, 0, 0) — create_coordinate_grid with start=(1,0,0)
        // gives axis 0 = arange(1, 1+padded_len), so each token gets incrementing t.
        let cap_pos_ids: Vec<(usize, usize, usize)> = (0..cap_len).map(|i| (i + 1, 0, 0)).collect();

        let cap_emb = cap_emb
            .unsqueeze(0)
            .map_err(|e| format!("DiT: cap_emb.unsqueeze: {}", e))?; // [1, cap_seq, dim]

        // context_refiner (modulation=False) on cap — source: L1001-1006
        let mut cap_emb = cap_emb;
        for layer in &self.context_refiner {
            cap_emb = layer
                .forward(&cap_emb, &cap_pos_ids, None, &self.rope)
                .map_err(|e| format!("DiT: context_refiner: {}", e))?;
        }

        // Build unified: [x, cap] — source: L858-860
        let unified =
            Tensor::cat(&[&x, &cap_emb], 1).map_err(|e| format!("DiT: unified cat: {}", e))?;
        let unified_pos_ids: Vec<(usize, usize, usize)> = x_pos_ids
            .iter()
            .chain(cap_pos_ids.iter())
            .copied()
            .collect();

        // Main layers — source: L1048-1055
        let mut unified = unified;
        for layer in &self.layers {
            unified = layer
                .forward(&unified, &unified_pos_ids, Some(&adaln_input), &self.rope)
                .map_err(|e| format!("DiT: layer: {}", e))?;
        }

        // Final layer: adaLN = Sequential(SiLU(), Linear(adaln_dim → dim)) — source: L289-292
        // SiLU is applied to adaln_input BEFORE Linear (it's part of the Sequential).
        let silu_adaln = silu(&adaln_input).map_err(|e| format!("DiT: final silu: {}", e))?;
        let scale_raw = linear_forward(
            &silu_adaln,
            &self.final_adaLN,
            self.final_adaLN_bias.as_ref(),
        )
        .map_err(|e| format!("DiT: final adaLN: {}", e))?;

        let one_t = Tensor::full(1.0f32, scale_raw.dims(), device)
            .map_err(|e| format!("DiT: final one_t: {}", e))?;
        let scale = (&scale_raw + &one_t).map_err(|e| format!("DiT: final scale: {}", e))?;
        let scale_b = scale
            .reshape({
                let mut s = vec![1; unified.dims().len()];
                s[unified.dims().len() - 1] = self.config.dim;
                s
            })
            .map_err(|e| format!("DiT: scale_b reshape: {}", e))?
            .broadcast_as(unified.dims())
            .map_err(|e| format!("DiT: scale_b broadcast: {}", e))?;

        let normed = layernorm_no_affine(&unified, 1e-6)
            .map_err(|e| format!("DiT: final layernorm: {}", e))?;
        let normed = (&normed * &scale_b).map_err(|e| format!("DiT: final normed*scale: {}", e))?;
        let out = linear_forward(&normed, &self.final_linear, self.final_linear_bias.as_ref())
            .map_err(|e| format!("DiT: final linear: {}", e))?;

        // Extract x part (first nph*npw tokens) and unpatchify — source: L1068
        let x_out = out
            .narrow(1, 0, nph * npw)
            .map_err(|e| format!("DiT: narrow: {}", e))?;
        let x_out = unpatchify(&x_out, nph, npw, ph, pw, self.config.in_channels, device)
            .map_err(|e| format!("DiT: unpatchify: {}", e))?;

        Ok(x_out)
    }

    pub fn config(&self) -> &ZImageConfig {
        &self.config
    }
}

// ── Block construction ──

fn build_block_seeded(
    config: &ZImageConfig,
    seed: u64,
    device: &Device,
    modulation: bool,
) -> Result<DiTBlock, String> {
    let dim = config.dim;
    let n_heads = config.n_heads;
    let n_kv_heads = config.n_kv_heads;
    let head_dim = config.head_dim;
    let q_dim = n_heads * head_dim;
    let kv_dim = n_kv_heads * head_dim;
    let inter = config.intermediate();
    let adaln_dim = config.adaln_dim();
    let eps = config.norm_eps;

    let adaLN = if modulation {
        // Linear(adaln_dim → 4*dim) — source: L224
        let (w, b) = linear_seeded(adaln_dim, 4 * dim, seed, device, true)?;
        Some((w, b))
    } else {
        None
    };

    let norm_w = |d: usize| -> Result<Tensor, String> {
        Tensor::ones((d,), DType::F32, device).map_err(|e| e.to_string())
    };
    let attention_norm1 = norm_w(dim)?;
    let attention_norm2 = norm_w(dim)?;
    let ffn_norm1 = norm_w(dim)?;
    let ffn_norm2 = norm_w(dim)?;

    let (to_q, _) = linear_seeded(dim, q_dim, seed.wrapping_add(10), device, false)?;
    let (to_k, _) = linear_seeded(dim, kv_dim, seed.wrapping_add(11), device, false)?;
    let (to_v, _) = linear_seeded(dim, kv_dim, seed.wrapping_add(12), device, false)?;
    let (to_out, _) = linear_seeded(q_dim, dim, seed.wrapping_add(13), device, false)?;

    let norm_q = norm_w(head_dim)?;
    let norm_k = norm_w(head_dim)?;

    let (w1, _) = linear_seeded(dim, inter, seed.wrapping_add(20), device, false)?;
    let (w2, _) = linear_seeded(inter, dim, seed.wrapping_add(22), device, false)?;
    let (w3, _) = linear_seeded(dim, inter, seed.wrapping_add(21), device, false)?;

    Ok(DiTBlock {
        modulation,
        adaLN,
        attention_norm1,
        attention_norm2,
        ffn_norm1,
        ffn_norm2,
        to_q,
        to_k,
        to_v,
        to_out,
        norm_q,
        norm_k,
        feed_forward: FeedForward { w1, w2, w3 },
        n_heads,
        n_kv_heads,
        head_dim,
        dim,
        eps,
    })
}

fn build_block_from_weights(
    tensors: &HashMap<String, Tensor>,
    prefix: &str,
    config: &ZImageConfig,
    device: &Device,
    modulation: bool,
    get: &impl Fn(&str, &[usize]) -> Result<Tensor, String>,
    get_opt: &impl Fn(&str, &[usize]) -> Result<Option<Tensor>, String>,
) -> Result<DiTBlock, String> {
    let dim = config.dim;
    let n_heads = config.n_heads;
    let n_kv_heads = config.n_kv_heads;
    let head_dim = config.head_dim;
    let q_dim = n_heads * head_dim;
    let kv_dim = n_kv_heads * head_dim;
    let inter = config.intermediate();
    let adaln_dim = config.adaln_dim();
    let eps = config.norm_eps;

    let adaLN = if modulation {
        let w = get(
            &format!("{}.adaLN_modulation.0.weight", prefix),
            &[4 * dim, adaln_dim],
        )?;
        let b = get(&format!("{}.adaLN_modulation.0.bias", prefix), &[4 * dim])?;
        Some((w, Some(b)))
    } else {
        None
    };

    let attention_norm1 = get(&format!("{}.attention_norm1.weight", prefix), &[dim])?;
    let attention_norm2 = get(&format!("{}.attention_norm2.weight", prefix), &[dim])?;
    let ffn_norm1 = get(&format!("{}.ffn_norm1.weight", prefix), &[dim])?;
    let ffn_norm2 = get(&format!("{}.ffn_norm2.weight", prefix), &[dim])?;

    let to_q = get(&format!("{}.attention.to_q.weight", prefix), &[q_dim, dim])?;
    let to_k = get(&format!("{}.attention.to_k.weight", prefix), &[kv_dim, dim])?;
    let to_v = get(&format!("{}.attention.to_v.weight", prefix), &[kv_dim, dim])?;
    let to_out = get(
        &format!("{}.attention.to_out.0.weight", prefix),
        &[dim, q_dim],
    )?;
    let norm_q = get(&format!("{}.attention.norm_q.weight", prefix), &[head_dim])?;
    let norm_k = get(&format!("{}.attention.norm_k.weight", prefix), &[head_dim])?;

    let w1 = get(&format!("{}.feed_forward.w1.weight", prefix), &[inter, dim])?;
    let w2 = get(&format!("{}.feed_forward.w2.weight", prefix), &[dim, inter])?;
    let w3 = get(&format!("{}.feed_forward.w3.weight", prefix), &[inter, dim])?;

    Ok(DiTBlock {
        modulation,
        adaLN,
        attention_norm1,
        attention_norm2,
        ffn_norm1,
        ffn_norm2,
        to_q,
        to_k,
        to_v,
        to_out,
        norm_q,
        norm_k,
        feed_forward: FeedForward { w1, w2, w3 },
        n_heads,
        n_kv_heads,
        head_dim,
        dim,
        eps,
    })
}

// ── Patchify / unpatchify ──

fn patchify(latent: &Tensor, ph: usize, pw: usize) -> CandleResult<Tensor> {
    // [1, C, H, W] → [H/ph * W/pw, C*ph*pw]
    let (b, c, h, w) = latent.dims4()?;
    let nph = h / ph;
    let npw = w / pw;
    if h % ph != 0 || w % pw != 0 {
        bail!(
            "patchify: H={} W={} not divisible by ph={} pw={}",
            h,
            w,
            ph,
            pw
        );
    }
    let _ = b;
    // Reshape: [1, C, nph, ph, npw, pw] → [nph, npw, C, ph, pw] → [nph*npw, C*ph*pw]
    let latent = latent.reshape((c, nph, ph, npw, pw))?;
    let latent = latent.permute((1, 3, 0, 2, 4))?;
    latent.reshape((nph * npw, c * ph * pw))
}

fn unpatchify(
    x: &Tensor,
    nph: usize,
    npw: usize,
    ph: usize,
    pw: usize,
    c: usize,
    device: &Device,
) -> CandleResult<Tensor> {
    // x: [1, nph*npw, pp] → [1, C, H, W]
    let pp = c * ph * pw;
    let x = x.reshape((nph, npw, c, ph, pw))?;
    // permute to [C, nph, ph, npw, pw] → [C, H, W]
    let x = x.permute((2, 0, 3, 1, 4))?;
    let h = nph * ph;
    let w = npw * pw;
    x.reshape((1, c, h, w))
}

// ── Sinusoidal embedding — source: transformer_z_image.py L51-61 ──

fn sinusoidal_embedding(
    t: f64,
    dim: usize,
    max_period: f64,
    device: &Device,
) -> CandleResult<Tensor> {
    let half = dim / 2;
    let freqs: Vec<f32> = (0..half)
        .map(|i| {
            let exponent = -(max_period.ln()) * (i as f64) / (half as f64);
            exponent.exp() as f32
        })
        .collect();
    let args: Vec<f32> = freqs.iter().map(|f| (t * *f as f64) as f32).collect();
    // embedding = cat([cos(args), sin(args)], dim=-1) — source: L58
    let cos_vals: Vec<f32> = args.iter().map(|a| a.cos()).collect();
    let sin_vals: Vec<f32> = args.iter().map(|a| a.sin()).collect();
    let mut vals = Vec::with_capacity(dim);
    vals.extend_from_slice(&cos_vals);
    vals.extend_from_slice(&sin_vals);
    Tensor::from_vec(vals, (dim,), device)
}
