//! VAE decoder — flux-dev-style AutoencoderKL (naryad №212 Block 3).
//!
//! Implements the **decoder** path of the Z-Image-Turbo VAE (`vae/config.json`).
//! Encoder is unused (latents come from the sampler).
//!
//! ## Architecture (flux-dev style)
//!
//! Input: latent `[1, 16, H/8, W/8]` (latent_channels=16, e.g. 128×128 for 1024×1024 image).
//! Ritual: `z = latent / scaling_factor + shift_factor` (flux-dev: shift=0.1159, scaling=0.3611).
//! n232 fix: direction corrected per pipeline_z_image.py L589:
//!   `latents = (latents / scaling_factor) + shift_factor` — NOT `(latent - shift) / scaling`.
//! Output: `[3, H, W]` in [0,1] via `(sample / 2 + 0.5).clamp(0, 1)`.
//!
//! Decoder structure (config: block_out_channels=[128,256,512,512], layers_per_block=2,
//! norm_num_groups=32, mid_block_add_attention=true):
//! - conv_in:   latent 16ch → 4*base=512ch, 3×3 conv
//! - mid_block: 2× ResnetBlock2D + 1× Attention (GroupNorm + proj_in + q/k/v/out_proj + proj_out)
//! - up_blocks[3,2,1,0]: each 2 (last has 3) ResnetBlock2D + Upsample2D (conv 3×3) between blocks
//!   (block 0 has no upsampler — it's the final one).
//! - conv_out:  base=128ch → 3ch, 3×3 conv
//!
//! ## Two-tier tests
//!
//! - CI tiny-golden: `VaeDecoder::new_tiny(&TINY_VAE_CONFIG, seed)` — small dims, seeded init
//!   via SSOT PRNG (param_seed from №230 + generate_uniform_f32 from src/nn). 3 bit-identical
//!   runs, pinned SHA-256 + 4 anchor bits.
//! - env-gated real-weights: `VaeDecoder::from_weights(tensors)` — actual flux-dev VAE weights.

// Allow non-snake_case + selected clippy nits — the diffusers/HF naming convention
// (block_out_channels, scaling_factor, etc.) is preserved for traceability.
// Style nits are suppressed to keep the diffusers-source-comparable form.
#![allow(non_snake_case)]
#![allow(clippy::all)]
#![allow(clippy::expect_used)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::needless_late_init)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::useless_conversion)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use std::collections::HashMap;
use std::path::Path;

use candle_core::bail;
use candle_core::{DType, Device, Result as CandleResult, Tensor};
use candle_nn::{Conv2d, Conv2dConfig, Module};

use crate::nn::attention::generate_uniform_f32;
use crate::vision::text_encoder::param_seed;

/// VAE configuration — mirrors `vae/config.json` fields (relevant for decoder).
#[derive(Debug, Clone)]
pub struct VaeConfig {
    pub latent_channels: usize,
    pub block_out_channels: Vec<usize>,
    pub layers_per_block: usize,
    pub norm_num_groups: usize,
    pub mid_block_add_attention: bool,
    pub scaling_factor: f64,
    pub shift_factor: f64,
    pub out_channels: usize,
}

/// Pinned Z-Image-Turbo VAE config (from `vae/config.json`, fetched 2026-09-08).
pub fn zimage_turbo_vae_config() -> VaeConfig {
    VaeConfig {
        latent_channels: 16,
        block_out_channels: vec![128, 256, 512, 512],
        layers_per_block: 2,
        norm_num_groups: 32,
        mid_block_add_attention: true,
        scaling_factor: 0.3611,
        shift_factor: 0.1159,
        out_channels: 3,
    }
}

/// Tiny VAE config for CI goldens — same architecture, tiny dims.
pub fn tiny_vae_config() -> VaeConfig {
    VaeConfig {
        latent_channels: 4,
        block_out_channels: vec![8, 16, 32, 32],
        layers_per_block: 1,
        norm_num_groups: 8,
        mid_block_add_attention: false,
        scaling_factor: 0.3611,
        shift_factor: 0.1159,
        out_channels: 3,
    }
}

// ── Seed derivation constants ──

const PARAM_VAE_CONV_IN: u64 = 100;
const PARAM_VAE_CONV_OUT: u64 = 101;
const PARAM_VAE_MID_RESNET: u64 = 110;
const PARAM_VAE_UP_RESNET: u64 = 120;
const PARAM_VAE_UP_UPSAMPLE: u64 = 121;

// ── Encoder seed slots (Наряд №243 R6.2) ──
// Mirrors the decoder slots above: distinct slot numbers so encoder and
// decoder seeded inits never share parameter streams (PRNG hygiene, №230).
const PARAM_VAE_ENC_CONV_IN: u64 = 130;
const PARAM_VAE_ENC_DOWN_RESNET: u64 = 131;
const PARAM_VAE_ENC_DOWNSAMPLE: u64 = 132;
const PARAM_VAE_ENC_MID_RESNET: u64 = 133;
const PARAM_VAE_ENC_CONV_OUT: u64 = 134;
const PARAM_VAE_ENC_QUANT_CONV: u64 = 135;

/// Generate a seeded randn-like latent (n samples from a normal via Box-Muller).
///
/// Uses the SSOT `generate_uniform_f32` as the U(0,1) source; Box-Muller transforms
/// pairs to N(0,1). Determinism is bit-exact.
pub fn fixed_latent(seed: u64, channels: usize, h: usize, w: usize) -> Tensor {
    let n = channels * h * w;
    let u01 = generate_uniform_f32(seed, n * 2, 1e-9, 1.0 - 1e-9);
    let mut vals = Vec::with_capacity(n);
    let mut i = 0;
    while vals.len() < n {
        let u1 = u01[i] as f64;
        let u2 = u01[i + 1] as f64;
        i += 2;
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f64::consts::PI * u2;
        vals.push((r * theta.cos()) as f32);
        if vals.len() < n {
            vals.push((r * theta.sin()) as f32);
        }
    }
    Tensor::from_vec(vals, (1, channels, h, w), &Device::Cpu).expect("fixed_latent")
}

// ── Helper: scalar-as-tensor arithmetic ──

fn scalar_full(val: f32, dims: &[usize], device: &Device) -> CandleResult<Tensor> {
    Tensor::full(val, dims.to_vec(), device)
}

// ── GroupNorm helper ──

fn group_norm(
    x: &Tensor,
    num_groups: usize,
    channels: usize,
    weight: &Tensor,
    bias: &Tensor,
    eps: f64,
) -> CandleResult<Tensor> {
    let b = x.dim(0)?;
    let h = x.dim(2)?;
    let w = x.dim(3)?;
    if channels % num_groups != 0 {
        bail!(
            "group_norm: channels {} not divisible by num_groups {}",
            channels,
            num_groups
        );
    }
    let group_size = channels / num_groups;
    let x_f32 = x.to_dtype(DType::F32)?;
    // Reshape [B, C, H, W] → [B, G, C/G, H, W]
    let x_r = x_f32.reshape((b, num_groups, group_size, h, w))?;
    // Flatten for mean/var computation.
    let flat = x_r.reshape((b, num_groups, group_size * h * w))?;
    let mean = flat.mean_keepdim(2)?; // [B, G, 1]
    let mean_b = mean.broadcast_as(flat.dims())?;
    let centered = (&flat - &mean_b)?;
    let sq = (&centered * &centered)?;
    let var = sq.mean_keepdim(2)?; // [B, G, 1]
    let eps_t = scalar_full(eps as f32, var.dims(), x.device())?;
    let denom = (&var + eps_t)?.sqrt()?;
    let denom_b = denom.broadcast_as(flat.dims())?;
    let normed = centered.broadcast_div(&denom_b)?;
    let normed = normed.reshape((b, channels, h, w))?;
    // Apply per-channel weight/bias.
    let w_shape: Vec<usize> = vec![1, channels, 1, 1];
    let w = weight
        .reshape(w_shape.as_slice())?
        .broadcast_as(normed.dims())?;
    let b_shape: Vec<usize> = vec![1, channels, 1, 1];
    let b = bias
        .reshape(b_shape.as_slice())?
        .broadcast_as(normed.dims())?;
    Ok((&(&normed * &w)? + &b)?)
}

// ── SiLU ──

fn silu(x: &Tensor) -> CandleResult<Tensor> {
    candle_nn::ops::silu(x)
}

// ── Seeded Conv2d construction (bypasses VarBuilder) ──

fn conv2d_seeded(
    in_ch: usize,
    out_ch: usize,
    padding: usize,
    kernel: usize,
    seed: u64,
    device: &Device,
) -> Result<Conv2d, String> {
    let weight_init = generate_uniform_f32(seed, out_ch * in_ch * kernel * kernel, -0.02, 0.02);
    let bias_init = generate_uniform_f32(seed.wrapping_add(1), out_ch, -0.02, 0.02);
    let weight = Tensor::from_vec(weight_init, (out_ch, in_ch, kernel, kernel), device)
        .map_err(|e| format!("conv2d_seeded: weight init failed: {}", e))?;
    let bias = Tensor::from_vec(bias_init, (out_ch,), device)
        .map_err(|e| format!("conv2d_seeded: bias init failed: {}", e))?;
    Ok(Conv2d::new(
        weight,
        Some(bias),
        Conv2dConfig {
            padding,
            stride: 1,
            dilation: 1,
            groups: 1,
            cudnn_fwd_algo: None,
        },
    ))
}

/// Strided seeded Conv2d (Наряд №243 R6.2) — same init contour as
/// `conv2d_seeded`, with an explicit `stride` for the encoder's
/// downsamplers (diffusers `Downsample2D`: kernel 3, stride 2, padding 1).
fn conv2d_seeded_strided(
    in_ch: usize,
    out_ch: usize,
    padding: usize,
    kernel: usize,
    stride: usize,
    seed: u64,
    device: &Device,
) -> Result<Conv2d, String> {
    let weight_init = generate_uniform_f32(seed, out_ch * in_ch * kernel * kernel, -0.02, 0.02);
    let bias_init = generate_uniform_f32(seed.wrapping_add(1), out_ch, -0.02, 0.02);
    let weight = Tensor::from_vec(weight_init, (out_ch, in_ch, kernel, kernel), device)
        .map_err(|e| format!("conv2d_seeded_strided: weight init failed: {}", e))?;
    let bias = Tensor::from_vec(bias_init, (out_ch,), device)
        .map_err(|e| format!("conv2d_seeded_strided: bias init failed: {}", e))?;
    Ok(Conv2d::new(
        weight,
        Some(bias),
        Conv2dConfig {
            padding,
            stride,
            dilation: 1,
            groups: 1,
            cudnn_fwd_algo: None,
        },
    ))
}

// ── ResnetBlock2D ──

struct ResnetBlock2D {
    norm1_weight: Tensor,
    norm1_bias: Tensor,
    conv1: Conv2d,
    norm2_weight: Tensor,
    norm2_bias: Tensor,
    conv2: Conv2d,
    shortcut: Option<Conv2d>,
    num_groups: usize,
    in_ch: usize,
    out_ch: usize,
    eps: f64,
}

impl ResnetBlock2D {
    fn new_seeded(
        in_ch: usize,
        out_ch: usize,
        num_groups: usize,
        eps: f64,
        seed: u64,
        device: &Device,
    ) -> Result<Self, String> {
        let norm1_weight = Tensor::ones((in_ch,), DType::F32, device)
            .map_err(|e| format!("ResnetBlock2D: norm1_weight: {}", e))?;
        let norm1_bias = Tensor::zeros((in_ch,), DType::F32, device)
            .map_err(|e| format!("ResnetBlock2D: norm1_bias: {}", e))?;
        let conv1 = conv2d_seeded(in_ch, out_ch, 1, 3, seed, device)?;
        let norm2_weight = Tensor::ones((out_ch,), DType::F32, device)
            .map_err(|e| format!("ResnetBlock2D: norm2_weight: {}", e))?;
        let norm2_bias = Tensor::zeros((out_ch,), DType::F32, device)
            .map_err(|e| format!("ResnetBlock2D: norm2_bias: {}", e))?;
        let conv2 = conv2d_seeded(out_ch, out_ch, 1, 3, seed.wrapping_add(2), device)?;
        let shortcut = if in_ch != out_ch {
            Some(conv2d_seeded(
                in_ch,
                out_ch,
                0,
                1,
                seed.wrapping_add(3),
                device,
            )?)
        } else {
            None
        };
        Ok(ResnetBlock2D {
            norm1_weight,
            norm1_bias,
            conv1,
            norm2_weight,
            norm2_bias,
            conv2,
            shortcut,
            num_groups,
            in_ch,
            out_ch,
            eps,
        })
    }

    fn from_weights(
        tensors: &HashMap<String, Tensor>,
        prefix: &str,
        in_ch: usize,
        out_ch: usize,
        num_groups: usize,
        eps: f64,
        device: &Device,
    ) -> Result<Self, String> {
        let get = |name: &str, shape: &[usize]| -> Result<Tensor, String> {
            let t = tensors.get(name).ok_or_else(|| {
                format!("ResnetBlock2D::from_weights: tensor '{}' not found", name)
            })?;
            let t = t
                .to_device(device)
                .map_err(|e| format!("device transfer: {}", e))?;
            let t = t
                .to_dtype(DType::F32)
                .map_err(|e| format!("F32 cast: {}", e))?;
            if t.dims() != shape {
                return Err(format!(
                    "ResnetBlock2D: '{}' shape mismatch — expected {:?}, got {:?}",
                    name,
                    shape,
                    t.dims()
                ));
            }
            Ok(t)
        };

        let norm1_weight = get(&format!("{}.norm1.weight", prefix), &[in_ch])?;
        let norm1_bias = get(&format!("{}.norm1.bias", prefix), &[in_ch])?;
        let conv1_weight = get(&format!("{}.conv1.weight", prefix), &[out_ch, in_ch, 3, 3])?;
        let conv1_bias = get(&format!("{}.conv1.bias", prefix), &[out_ch])?;
        let norm2_weight = get(&format!("{}.norm2.weight", prefix), &[out_ch])?;
        let norm2_bias = get(&format!("{}.norm2.bias", prefix), &[out_ch])?;
        let conv2_weight = get(&format!("{}.conv2.weight", prefix), &[out_ch, out_ch, 3, 3])?;
        let conv2_bias = get(&format!("{}.conv2.bias", prefix), &[out_ch])?;

        let conv1 = Conv2d::new(
            conv1_weight,
            Some(conv1_bias),
            Conv2dConfig {
                padding: 1,
                ..Default::default()
            },
        );
        let conv2 = Conv2d::new(
            conv2_weight,
            Some(conv2_bias),
            Conv2dConfig {
                padding: 1,
                ..Default::default()
            },
        );

        let shortcut_weight_name = format!("{}.conv_shortcut.weight", prefix);
        let shortcut = if tensors.contains_key(&shortcut_weight_name) {
            let sw = get(&shortcut_weight_name, &[out_ch, in_ch, 1, 1])?;
            let sb = get(&format!("{}.conv_shortcut.bias", prefix), &[out_ch])?;
            Some(Conv2d::new(
                sw,
                Some(sb),
                Conv2dConfig {
                    padding: 0,
                    ..Default::default()
                },
            ))
        } else {
            None
        };

        Ok(ResnetBlock2D {
            norm1_weight,
            norm1_bias,
            conv1,
            norm2_weight,
            norm2_bias,
            conv2,
            shortcut,
            num_groups,
            in_ch,
            out_ch,
            eps,
        })
    }

    fn forward(&self, x: &Tensor) -> CandleResult<Tensor> {
        let h = group_norm(
            x,
            self.num_groups,
            self.in_ch,
            &self.norm1_weight,
            &self.norm1_bias,
            self.eps,
        )?;
        let h = silu(&h)?;
        let h = self.conv1.forward(&h)?;
        let h = group_norm(
            &h,
            self.num_groups,
            self.out_ch,
            &self.norm2_weight,
            &self.norm2_bias,
            self.eps,
        )?;
        let h = silu(&h)?;
        let h = self.conv2.forward(&h)?;

        let out = if let Some(ref sc) = self.shortcut {
            (h + sc.forward(x)?)?
        } else {
            (h + x)?
        };
        Ok(out)
    }
}

// ── Upsample2D ──

struct Upsample2D {
    conv: Conv2d,
}

impl Upsample2D {
    fn forward(&self, x: &Tensor) -> CandleResult<Tensor> {
        let (b, c, h, w) = x.dims4()?;
        // nearest-neighbor 2× upscale
        let x = x
            .reshape((b, c, h, 1, w, 1))?
            .broadcast_as((b, c, h, 2, w, 2))?
            .reshape((b, c, h * 2, w * 2))?;
        self.conv.forward(&x)
    }
}

// ── VaeDecoder ──

// ── VAE mid-block attention (n233 Block 3) ──
// Source: diffusers AutoencoderKL mid_block.attentions.0
// Structure: GroupNorm → spatial self-attention (q/k/v/out_proj) → residual
// Tensor names (verified from safetensors header 2026-09-08):
//   decoder.mid_block.attentions.0.group_norm.{weight,bias}  [512]
//   decoder.mid_block.attentions.0.to_{q,k,v}.{weight,bias}  [512, 512] / [512]
//   decoder.mid_block.attentions.0.to_out.0.{weight,bias}    [512, 512] / [512]

struct VaeAttention {
    group_norm_weight: Tensor, // [channels]
    group_norm_bias: Tensor,   // [channels]
    to_q: Tensor,              // [channels, channels]
    to_q_bias: Tensor,
    to_k: Tensor,
    to_k_bias: Tensor,
    to_v: Tensor,
    to_v_bias: Tensor,
    to_out: Tensor, // [channels, channels]
    to_out_bias: Tensor,
    channels: usize,
    num_groups: usize,
    eps: f64,
}

impl VaeAttention {
    fn from_weights(
        tensors: &HashMap<String, Tensor>,
        prefix: &str,
        channels: usize,
        num_groups: usize,
        eps: f64,
        device: &Device,
    ) -> Result<Self, String> {
        let get = |name: &str, shape: &[usize]| -> Result<Tensor, String> {
            let t = tensors.get(name).ok_or_else(|| {
                format!("VaeAttention::from_weights: tensor '{}' not found", name)
            })?;
            let t = t.to_device(device).map_err(|e| format!("device: {}", e))?;
            let t = t
                .to_dtype(DType::F32)
                .map_err(|e| format!("dtype: {}", e))?;
            if t.dims() != shape {
                return Err(format!(
                    "VaeAttention: '{}' shape mismatch — expected {:?}, got {:?}",
                    name,
                    shape,
                    t.dims()
                ));
            }
            Ok(t)
        };

        let group_norm_weight = get(&format!("{}.group_norm.weight", prefix), &[channels])?;
        let group_norm_bias = get(&format!("{}.group_norm.bias", prefix), &[channels])?;
        let to_q = get(&format!("{}.to_q.weight", prefix), &[channels, channels])?;
        let to_q_bias = get(&format!("{}.to_q.bias", prefix), &[channels])?;
        let to_k = get(&format!("{}.to_k.weight", prefix), &[channels, channels])?;
        let to_k_bias = get(&format!("{}.to_k.bias", prefix), &[channels])?;
        let to_v = get(&format!("{}.to_v.weight", prefix), &[channels, channels])?;
        let to_v_bias = get(&format!("{}.to_v.bias", prefix), &[channels])?;
        let to_out = get(
            &format!("{}.to_out.0.weight", prefix),
            &[channels, channels],
        )?;
        let to_out_bias = get(&format!("{}.to_out.0.bias", prefix), &[channels])?;

        Ok(VaeAttention {
            group_norm_weight,
            group_norm_bias,
            to_q,
            to_q_bias,
            to_k,
            to_k_bias,
            to_v,
            to_v_bias,
            to_out,
            to_out_bias,
            channels,
            num_groups,
            eps,
        })
    }

    fn forward(&self, x: &Tensor) -> CandleResult<Tensor> {
        // x: [B, C, H, W]
        let residual = x.clone();
        let b = x.dim(0)?;
        let c = x.dim(1)?;
        let h = x.dim(2)?;
        let w = x.dim(3)?;

        // GroupNorm
        let normed = group_norm(
            x,
            self.num_groups,
            c,
            &self.group_norm_weight,
            &self.group_norm_bias,
            self.eps,
        )?;

        // Reshape to [B, H*W, C] for spatial self-attention
        let normed_2d = normed.reshape((b, h * w, c))?;

        // q/k/v projections
        let q = linear_forward_2d(&normed_2d, &self.to_q, Some(&self.to_q_bias))?;
        let k = linear_forward_2d(&normed_2d, &self.to_k, Some(&self.to_k_bias))?;
        let v = linear_forward_2d(&normed_2d, &self.to_v, Some(&self.to_v_bias))?;

        // Single-head attention: scores = Q @ K^T / sqrt(C)
        let scale = 1.0 / (c as f64).sqrt();
        let scores = q.matmul(&k.transpose(1, 2)?)?;
        let scale_t = Tensor::full(scale as f32, scores.dims(), x.device())?;
        let scores = (scores * scale_t)?;
        let attn = candle_nn::ops::softmax_last_dim(&scores)?;
        let out = attn.matmul(&v)?; // [B, H*W, C]

        // to_out projection
        let out = linear_forward_2d(&out, &self.to_out, Some(&self.to_out_bias))?;

        // Reshape back to [B, C, H, W] and add residual
        let out = out.reshape((b, c, h, w))?;
        let result = (out + &residual)?;
        Ok(result)
    }
}

/// Linear forward for 2D inputs [B, seq, in] → [B, seq, out].
fn linear_forward_2d(x: &Tensor, weight: &Tensor, bias: Option<&Tensor>) -> CandleResult<Tensor> {
    let out = x.matmul(&weight.t()?)?;
    if let Some(b) = bias {
        let b_shape: Vec<usize> = vec![1, b.dim(0)?];
        let b_broadcast = b.reshape(b_shape.as_slice())?.broadcast_as(out.dims())?;
        Ok((out + b_broadcast)?)
    } else {
        Ok(out)
    }
}

// ── VaeDecoder ──

pub struct VaeDecoder {
    conv_in: Conv2d,
    mid_resnets: Vec<ResnetBlock2D>,
    mid_attn: Option<VaeAttention>,
    up_blocks: Vec<Vec<ResnetBlock2D>>,
    up_samplers: Vec<Option<Upsample2D>>,
    // n235: conv_norm_out = GroupNorm(base) → SiLU → conv_out — source: vae.py L271-273, L304-311
    conv_norm_out_weight: Tensor,
    conv_norm_out_bias: Tensor,
    conv_out: Conv2d,
    config: VaeConfig,
}

/// n236: VAE expected decoder key generator — single source of truth.
/// Builds the full expected tensor key set from the VAE config.
/// Real Z-Image-Turbo: 138 keys (verified from safetensors header, fetched 2026-09-09).
/// Breakdown: conv_in 2 + conv_norm_out 2 + conv_out 2 + mid 26 (2×8 resnets + 10 attn)
/// + up 106 (4 blocks × 3 resnets × 8 + 2 shortcuts on blocks 2/3 resnet 0 + 6 upsampler keys).
pub(crate) fn vae_expected_decoder_keys(config: &VaeConfig) -> Vec<String> {
    let base = config.block_out_channels[0];
    let mut keys = vec![
        "decoder.conv_in.weight".into(),
        "decoder.conv_in.bias".into(),
        "decoder.conv_norm_out.weight".into(),
        "decoder.conv_norm_out.bias".into(),
        "decoder.conv_out.weight".into(),
        "decoder.conv_out.bias".into(),
    ];
    // mid_block: 2 resnets × 8 tensors
    for i in 0..2 {
        let p = format!("decoder.mid_block.resnets.{}", i);
        for s in &[
            "norm1.weight",
            "norm1.bias",
            "conv1.weight",
            "conv1.bias",
            "norm2.weight",
            "norm2.bias",
            "conv2.weight",
            "conv2.bias",
        ] {
            keys.push(format!("{}.{}", p, s));
        }
    }
    // mid_block attention (if config has it): 10 tensors
    if config.mid_block_add_attention {
        let p = "decoder.mid_block.attentions.0";
        for s in &[
            "group_norm.weight",
            "group_norm.bias",
            "to_q.weight",
            "to_q.bias",
            "to_k.weight",
            "to_k.bias",
            "to_v.weight",
            "to_v.bias",
            "to_out.0.weight",
            "to_out.0.bias",
        ] {
            keys.push(format!("{}.{}", p, s));
        }
    }
    // up_blocks: layers_per_block+1 resnets per ALL blocks (source: vae.py L254).
    // Shortcuts only on resnet.0 when in_ch ≠ out_ch (n236 fix: in_ch updated INSIDE the loop).
    let rev: Vec<usize> = config.block_out_channels.iter().rev().copied().collect();
    let mut in_ch = 4 * base;
    for (i, &out_ch) in rev.iter().enumerate() {
        let num_resnets = config.layers_per_block + 1;
        for j in 0..num_resnets {
            let p = format!("decoder.up_blocks.{}.resnets.{}", i, j);
            for s in &[
                "norm1.weight",
                "norm1.bias",
                "conv1.weight",
                "conv1.bias",
                "norm2.weight",
                "norm2.bias",
                "conv2.weight",
                "conv2.bias",
            ] {
                keys.push(format!("{}.{}", p, s));
            }
            // n236 fix: shortcut ONLY when in_ch ≠ out_ch for THIS resnet.
            // in_ch is updated per-resnet, so only resnet.0 of a channel-changing block gets it.
            if in_ch != out_ch {
                keys.push(format!("{}.conv_shortcut.weight", p));
                keys.push(format!("{}.conv_shortcut.bias", p));
            }
            // n236: update in_ch INSIDE the loop (was outside → caused 146 instead of 138).
            in_ch = out_ch;
        }
        if i < rev.len() - 1 {
            let p = format!("decoder.up_blocks.{}.upsamplers.0.conv", i);
            keys.push(format!("{}.weight", p));
            keys.push(format!("{}.bias", p));
        }
    }
    keys
}

impl VaeDecoder {
    /// Seeded tiny-init for CI goldens (naryad №212 Block 3.2).
    pub fn new_tiny(config: &VaeConfig, seed: u64) -> Result<Self, String> {
        let device = Device::Cpu;
        let base = config.block_out_channels[0];

        let conv_in = conv2d_seeded(
            config.latent_channels,
            4 * base,
            1,
            3,
            param_seed(seed, 0, PARAM_VAE_CONV_IN),
            &device,
        )?;

        // mid_block: 2 resnets. (mid attention is None for tiny config — set mid_block_add_attention=false.)
        let mut mid_resnets = Vec::new();
        mid_resnets.push(ResnetBlock2D::new_seeded(
            4 * base,
            4 * base,
            config.norm_num_groups,
            1e-6,
            param_seed(seed, 1, PARAM_VAE_MID_RESNET),
            &device,
        )?);
        mid_resnets.push(ResnetBlock2D::new_seeded(
            4 * base,
            4 * base,
            config.norm_num_groups,
            1e-6,
            param_seed(seed, 2, PARAM_VAE_MID_RESNET),
            &device,
        )?);

        // up_blocks: reversed iteration to match diffusers layout.
        // n235: layers_per_block+1 resnets per ALL blocks (was: only last block).
        // Source: diffusers vae.py L254: num_layers = self.layers_per_block + 1.
        let mut up_blocks: Vec<Vec<ResnetBlock2D>> = Vec::new();
        let mut up_samplers: Vec<Option<Upsample2D>> = Vec::new();
        let mut in_ch = 4 * base;
        let rev_blocks: Vec<usize> = config.block_out_channels.iter().rev().copied().collect();
        for (i, &out_ch) in rev_blocks.iter().enumerate() {
            let mut resnets = Vec::new();
            let num_resnets = config.layers_per_block + 1;
            for j in 0..num_resnets {
                let r = ResnetBlock2D::new_seeded(
                    in_ch,
                    out_ch,
                    config.norm_num_groups,
                    1e-6,
                    param_seed(seed, (i as u64 + 10) * 100 + j as u64, PARAM_VAE_UP_RESNET),
                    &device,
                )?;
                resnets.push(r);
                in_ch = out_ch;
            }
            up_blocks.push(resnets);
            if i < rev_blocks.len() - 1 {
                let conv = conv2d_seeded(
                    out_ch,
                    out_ch,
                    1,
                    3,
                    param_seed(seed, i as u64, PARAM_VAE_UP_UPSAMPLE),
                    &device,
                )?;
                up_samplers.push(Some(Upsample2D { conv }));
            } else {
                up_samplers.push(None);
            }
        }

        // n235: conv_norm_out = GroupNorm(base) — source: vae.py L271-273
        let conv_norm_out_weight =
            Tensor::ones((base,), DType::F32, &device).map_err(|e| e.to_string())?;
        let conv_norm_out_bias =
            Tensor::zeros((base,), DType::F32, &device).map_err(|e| e.to_string())?;

        let conv_out = conv2d_seeded(
            base,
            config.out_channels,
            1,
            3,
            param_seed(seed, 999, PARAM_VAE_CONV_OUT),
            &device,
        )?;

        Ok(VaeDecoder {
            conv_in,
            mid_resnets,
            mid_attn: None, // tiny config uses mid_block_add_attention=false
            up_blocks,
            up_samplers,
            conv_norm_out_weight,
            conv_norm_out_bias,
            conv_out,
            config: config.clone(),
        })
    }

    /// Build from real safetensors tensors (naryad №212 Block 3.3).
    pub fn from_weights(tensors: &HashMap<String, Tensor>) -> Result<Self, String> {
        let config = zimage_turbo_vae_config();
        let device = Device::Cpu;
        let base = config.block_out_channels[0];

        let ci_w = tensors.get("decoder.conv_in.weight").ok_or_else(|| {
            "VaeDecoder::from_weights: missing decoder.conv_in.weight".to_string()
        })?;
        let ci_b = tensors
            .get("decoder.conv_in.bias")
            .ok_or_else(|| "VaeDecoder::from_weights: missing decoder.conv_in.bias".to_string())?;
        let ci_w = ci_w
            .to_dtype(DType::F32)
            .map_err(|e| format!("conv_in dtype: {}", e))?
            .to_device(&device)
            .map_err(|e| format!("conv_in device: {}", e))?;
        let ci_b = ci_b
            .to_dtype(DType::F32)
            .map_err(|e| format!("conv_in bias dtype: {}", e))?
            .to_device(&device)
            .map_err(|e| format!("conv_in bias device: {}", e))?;
        let conv_in = Conv2d::new(
            ci_w,
            Some(ci_b),
            Conv2dConfig {
                padding: 1,
                ..Default::default()
            },
        );

        let mid_resnets = vec![
            ResnetBlock2D::from_weights(
                tensors,
                "decoder.mid_block.resnets.0",
                4 * base,
                4 * base,
                config.norm_num_groups,
                1e-6,
                &device,
            )?,
            ResnetBlock2D::from_weights(
                tensors,
                "decoder.mid_block.resnets.1",
                4 * base,
                4 * base,
                config.norm_num_groups,
                1e-6,
                &device,
            )?,
        ];

        // n233 Block 3: load mid-block attention if config has it.
        let mid_attn = if config.mid_block_add_attention {
            Some(VaeAttention::from_weights(
                tensors,
                "decoder.mid_block.attentions.0",
                4 * base,
                config.norm_num_groups,
                1e-6,
                &device,
            )?)
        } else {
            None
        };

        let mut up_blocks: Vec<Vec<ResnetBlock2D>> = Vec::new();
        let mut up_samplers: Vec<Option<Upsample2D>> = Vec::new();
        let mut in_ch = 4 * base;
        let rev_blocks: Vec<usize> = config.block_out_channels.iter().rev().copied().collect();
        for (i, &out_ch) in rev_blocks.iter().enumerate() {
            let mut resnets = Vec::new();
            // n235: layers_per_block+1 for ALL blocks (was: only last).
            // Source: vae.py L254: num_layers = self.layers_per_block + 1.
            let num_resnets = config.layers_per_block + 1;
            for j in 0..num_resnets {
                let r = ResnetBlock2D::from_weights(
                    tensors,
                    &format!("decoder.up_blocks.{}.resnets.{}", i, j),
                    in_ch,
                    out_ch,
                    config.norm_num_groups,
                    1e-6,
                    &device,
                )?;
                resnets.push(r);
                in_ch = out_ch;
            }
            up_blocks.push(resnets);
            if i < rev_blocks.len() - 1 {
                let us_w = tensors
                    .get(&format!("decoder.up_blocks.{}.upsamplers.0.conv.weight", i))
                    .ok_or_else(|| {
                        format!("missing decoder.up_blocks.{}.upsamplers.0.conv.weight", i)
                    })?;
                let us_b = tensors
                    .get(&format!("decoder.up_blocks.{}.upsamplers.0.conv.bias", i))
                    .ok_or_else(|| {
                        format!("missing decoder.up_blocks.{}.upsamplers.0.conv.bias", i)
                    })?;
                let us_w = us_w
                    .to_dtype(DType::F32)
                    .map_err(|e| format!("upsampler {} dtype: {}", i, e))?
                    .to_device(&device)
                    .map_err(|e| format!("upsampler {} device: {}", i, e))?;
                let us_b = us_b
                    .to_dtype(DType::F32)
                    .map_err(|e| format!("upsampler {} bias dtype: {}", i, e))?
                    .to_device(&device)
                    .map_err(|e| format!("upsampler {} bias device: {}", i, e))?;
                let conv = Conv2d::new(
                    us_w,
                    Some(us_b),
                    Conv2dConfig {
                        padding: 1,
                        ..Default::default()
                    },
                );
                up_samplers.push(Some(Upsample2D { conv }));
            } else {
                up_samplers.push(None);
            }
        }

        let co_w = tensors.get("decoder.conv_out.weight").ok_or_else(|| {
            "VaeDecoder::from_weights: missing decoder.conv_out.weight".to_string()
        })?;
        let co_b = tensors
            .get("decoder.conv_out.bias")
            .ok_or_else(|| "VaeDecoder::from_weights: missing decoder.conv_out.bias".to_string())?;
        let co_w = co_w
            .to_dtype(DType::F32)
            .map_err(|e| format!("conv_out dtype: {}", e))?
            .to_device(&device)
            .map_err(|e| format!("conv_out device: {}", e))?;
        let co_b = co_b
            .to_dtype(DType::F32)
            .map_err(|e| format!("conv_out bias dtype: {}", e))?
            .to_device(&device)
            .map_err(|e| format!("conv_out bias device: {}", e))?;
        let conv_out = Conv2d::new(
            co_w,
            Some(co_b),
            Conv2dConfig {
                padding: 1,
                ..Default::default()
            },
        );

        // n235: conv_norm_out = GroupNorm(base) — source: vae.py L271-273
        let cno_w = tensors.get("decoder.conv_norm_out.weight").ok_or_else(|| {
            "VaeDecoder::from_weights: missing decoder.conv_norm_out.weight".to_string()
        })?;
        let cno_b = tensors.get("decoder.conv_norm_out.bias").ok_or_else(|| {
            "VaeDecoder::from_weights: missing decoder.conv_norm_out.bias".to_string()
        })?;
        let conv_norm_out_weight = cno_w
            .to_dtype(DType::F32)
            .map_err(|e| format!("conv_norm_out dtype: {}", e))?
            .to_device(&device)
            .map_err(|e| format!("conv_norm_out device: {}", e))?;
        let conv_norm_out_bias = cno_b
            .to_dtype(DType::F32)
            .map_err(|e| format!("conv_norm_out bias dtype: {}", e))?
            .to_device(&device)
            .map_err(|e| format!("conv_norm_out bias device: {}", e))?;

        // n236: key-level loader guard — calls extracted generator (single source of truth).
        // Наряд №243 (R6.2) truth-up: the pinned VAE safetensors file carries
        // BOTH sides of the AutoencoderKL — decoder.* (138 tensors) AND the
        // encoder.* / quant_conv.* / post_quant_conv.* prefixes (the manifest
        // row :158 lists them in one file). The decoder consumes ONLY the
        // `decoder.`-prefixed subset, so the coverage check must be run
        // against that subset — otherwise the honest 106 non-decoder tensors
        // of the SAME pinned file would fail the load as "extra". Loud
        // prerequisite of the edit path: the edit pipeline loads this file
        // ONCE and feeds BOTH the VaeEncoder (non-decoder prefixes) and this
        // decoder (decoder. prefix). Tiny goldens are unaffected (they use
        // `new_tiny`, not `from_weights`).
        let expected_keys = vae_expected_decoder_keys(&config);
        let loaded_keys: Vec<String> = tensors
            .keys()
            .filter(|k| k.starts_with("decoder."))
            .cloned()
            .collect();
        crate::vision::weights::check_tensor_coverage(&expected_keys, &loaded_keys)
            .map_err(|e| format!("VaeDecoder::from_weights: tensor coverage: {}", e))?;

        Ok(VaeDecoder {
            conv_in,
            mid_resnets,
            mid_attn,
            up_blocks,
            up_samplers,
            conv_norm_out_weight,
            conv_norm_out_bias,
            conv_out,
            config,
        })
    }

    /// Decode a latent `[1, latent_channels, H/8, W/8]` to an image `[3, H, W]` in [0,1].
    pub fn decode(&self, latent: &Tensor) -> Result<Tensor, String> {
        let device = latent.device();
        // n232 fix: VAE decode ritual = latent / scaling + shift (pipeline_z_image.py L589).
        // Was: (latent - shift) / scaling — WRONG direction.
        let shift = self.config.shift_factor as f32;
        let scale = self.config.scaling_factor as f32;
        let scale_t = scalar_full(scale, latent.dims(), device)
            .map_err(|e| format!("VAE decode: scale tensor: {}", e))?;
        let z = (latent / scale_t).map_err(|e| format!("VAE decode: divide scaling: {}", e))?;
        let shift_t = scalar_full(shift, z.dims(), device)
            .map_err(|e| format!("VAE decode: shift tensor: {}", e))?;
        let z = (z + shift_t).map_err(|e| format!("VAE decode: add shift: {}", e))?;

        let mut h = self
            .conv_in
            .forward(&z)
            .map_err(|e| format!("VAE decode: conv_in forward: {}", e))?;

        // Mid-block: resnets[0] → attention → resnets[1] — source: UNetMidBlock2D.forward
        // (diffusers unet_2d_blocks.py L737-748: resnets[0], then zip(attentions, resnets[1:])
        // → attn then resnet). Fetched 2026-09-08.
        // n234 fix: was applying attention AFTER both resnets — mathematically wrong
        // (nonlinear operations don't commute).
        h = self.mid_resnets[0]
            .forward(&h)
            .map_err(|e| format!("VAE decode: mid resnet 0: {}", e))?;
        if let Some(ref attn) = self.mid_attn {
            h = attn
                .forward(&h)
                .map_err(|e| format!("VAE decode: mid attn: {}", e))?;
        }
        h = self.mid_resnets[1]
            .forward(&h)
            .map_err(|e| format!("VAE decode: mid resnet 1: {}", e))?;

        for (i, resnets) in self.up_blocks.iter().enumerate() {
            for r in resnets {
                h = r
                    .forward(&h)
                    .map_err(|e| format!("VAE decode: up_block {} resnet: {}", i, e))?;
            }
            if let Some(Some(ref up)) = self.up_samplers.get(i) {
                h = up
                    .forward(&h)
                    .map_err(|e| format!("VAE decode: up_block {} upsampler: {}", i, e))?;
            }
        }

        // n235: conv_norm_out → SiLU → conv_out — source: vae.py L304-311
        // Was: conv_out immediately after up_blocks (missing norm + activation).
        let h = group_norm(
            &h,
            self.config.norm_num_groups,
            self.config.block_out_channels[0],
            &self.conv_norm_out_weight,
            &self.conv_norm_out_bias,
            1e-6,
        )
        .map_err(|e| format!("VAE decode: conv_norm_out: {}", e))?;
        let h = silu(&h).map_err(|e| format!("VAE decode: conv_norm_out silu: {}", e))?;
        let h = self
            .conv_out
            .forward(&h)
            .map_err(|e| format!("VAE decode: conv_out: {}", e))?;

        // Flux-dev style: image = (sample / 2 + 0.5).clamp(0, 1)
        let half_t = scalar_full(0.5f32, h.dims(), device)
            .map_err(|e| format!("VAE decode: half: {}", e))?;
        let out = (h * half_t.clone()).map_err(|e| format!("VAE decode: mul 0.5: {}", e))?;
        let out = (out + half_t).map_err(|e| format!("VAE decode: add 0.5: {}", e))?;
        let out = out
            .clamp(0.0f32, 1.0f32)
            .map_err(|e| format!("VAE decode: clamp: {}", e))?;

        out.squeeze(0)
            .map_err(|e| format!("VAE decode: squeeze: {}", e))
    }
}

// ═══════════════════════════════════════════════════════════════════════
// VaeEncoder (Наряд №243, R6.2) — the encode half of the flux-dev-style
// AutoencoderKL, mirroring `VaeDecoder` above.
//
// ## Where the weights come from
//
// The pinned `vae/diffusion_pytorch_model.safetensors` file carries BOTH
// sides of the AutoencoderKL (manifest №212, row `vae/…`): 138
// `decoder.`-prefixed tensors AND the non-decoder tensors
// (`encoder.*`, `quant_conv.*`, `post_quant_conv.*`). The encoder loads
// ONLY the non-decoder prefixes from that same map — the manifest is NOT
// extended (16 files / 32 848 304 654 B invariant), `tools/
// fetch_vision_weights.sh` is not touched, the VAE SHA stays `_TODO`
// (PARKED gap №237). The exact non-decoder key list must be verified
// against the real file header during the PARKED real-weights run
// (loud step in the runbook edit-e2e section) — until then the expected
// key set below mirrors the diffusers AutoencoderKL encoder layout
// (source: diffusers models/autoencoders/vae.py `Encoder` +
// unets/unet_2d_blocks.py `DownEncoderBlock2D`/`Downsample2D`).
//
// ## Architecture (mirrors diffusers `Encoder`)
//
// Input: image `[3, H, W]` F32 in **[-1, 1]** (the VAE convention; the
// caller maps [0,1] PNG pixels → [-1,1] via `x*2-1`).
// - conv_in: 3 → base (3×3, pad 1)
// - down_blocks[i] for i in 0..len(block_out_channels):
//     `layers_per_block` resnets each (NOTE: NOT layers_per_block+1 —
//     that is the DECODER's up-block count; diffusers vae.py Encoder
//     uses `layers_per_block` verbatim), first resnet of block 0 takes
//     in_channels=3, first resnet of block i>0 takes
//     block_out_channels[i-1]; conv_shortcut when in≠out;
//     downsampler (3×3 conv, stride 2, pad 1) after every block except
//     the last.
// - mid_block: resnets[0] → attention (if enabled) → resnets[1] — the
//   same UNetMidBlock2D shape the decoder uses.
// - conv_norm_out: GroupNorm(last) → SiLU → conv_out: last → 2*latent
//   (double_z — mean and logvar halves) → quant_conv (1×1).
// Output: posterior MODE = mean half `[1, latent_channels, H/f, W/f]`,
// then the pipeline ritual inverse of the decoder's
// `z_vae = z_model / scaling + shift` (n232, pipeline_z_image.py L589):
//   `z_model = (mean - shift_factor) * scaling_factor`
// The posterior is consumed at its MODE (mean) — deterministic by
// construction; sampling the posterior would require an extra random
// stream and would break "same source + same prompt + same weights →
// same seed → same output" (Block 2.3). Loud honest boundary: this is
// the standard deterministic encode used by in-context edit pipelines.
//
// ## Two-tier tests
//
// - CI tiny: `VaeEncoder::new_tiny(&tiny_vae_config(), seed)` — seeded
//   init via the SSOT PRNG, exercised by the №243 tiny contracts.
// - env-gated real weights: `VaeEncoder::from_weights(tensors)` — the
//   non-decoder prefixes of the pinned VAE file.
// ═══════════════════════════════════════════════════════════════════════

// ── Downsample2D (encoder-only; diffusers DownEncoderBlock2D downsampler)
// ──

struct Downsample2D {
    conv: Conv2d,
}

impl Downsample2D {
    fn forward(&self, x: &Tensor) -> CandleResult<Tensor> {
        // 3×3 conv, stride 2, padding 1 — halves H and W for even inputs
        // (odd inputs floor; the edit glue rejects non-divisible dims
        // BEFORE reaching the encoder, so this is defense-in-depth).
        self.conv.forward(x)
    }
}

/// VAE downsample factor: `2^(len(block_out_channels) - 1)` — one halving
/// per downsampler (every block except the last has one).
pub fn vae_downsample_factor(config: &VaeConfig) -> usize {
    1usize << (config.block_out_channels.len().saturating_sub(1))
}

/// n243: VAE expected ENCODER key generator — the encoder-side mirror of
/// `vae_expected_decoder_keys`. Keys live in the same pinned safetensors
/// file as the decoder's (no manifest change).
///
/// **Loud layout note (verified against the manifest arithmetic):** the
/// manifest row :158 records 244 tensors in the file, 138 of them
/// `decoder.`-prefixed → 106 non-decoder. The diffusers-style encoder
/// layout below produces EXACTLY 106 `encoder.*` keys for the pinned
/// config (2 conv_in + 18/20/20/16 down_blocks + 26 mid + 2 conv_norm_out
/// + 2 conv_out) — which means the file most likely carries NO
/// `quant_conv`/`post_quant_conv` tensors (138+106=244). The loader
/// therefore treats `quant_conv` as OPTIONAL (consumed when present,
/// loudly named when absent); `post_quant_conv.*`, when present, lands in
/// the loud-leftovers note. The actual header is verified during the
/// PARKED real-weights run (runbook edit-e2e checklist).
///
/// Channel progression: `conv_in` maps RGB → base, so the FIRST resnet of
/// block 0 takes base → base (in == out, no shortcut) — diffusers
/// `Encoder`: `input_channel = output_channel` where output starts at
/// `block_out_channels[0]`. Blocks 1..3 open with prev → out (shortcut
/// when in ≠ out).
pub(crate) fn vae_expected_encoder_keys(config: &VaeConfig) -> Vec<String> {
    let mut keys = vec![
        "encoder.conv_in.weight".to_string(),
        "encoder.conv_in.bias".to_string(),
        "encoder.conv_norm_out.weight".to_string(),
        "encoder.conv_norm_out.bias".to_string(),
        "encoder.conv_out.weight".to_string(),
        "encoder.conv_out.bias".to_string(),
    ];
    // down_blocks: layers_per_block resnets per block (Encoder — NOT +1),
    // shortcuts when in≠out, downsampler on every block except the last.
    let n_blocks = config.block_out_channels.len();
    for i in 0..n_blocks {
        let out_ch = config.block_out_channels[i];
        // conv_in already mapped RGB → base: block 0 opens at base.
        let block_in = if i == 0 {
            config.block_out_channels[0]
        } else {
            config.block_out_channels[i - 1]
        };
        for j in 0..config.layers_per_block {
            let in_ch = if j == 0 { block_in } else { out_ch };
            let p = format!("encoder.down_blocks.{}.resnets.{}", i, j);
            for s in &[
                "norm1.weight",
                "norm1.bias",
                "conv1.weight",
                "conv1.bias",
                "norm2.weight",
                "norm2.bias",
                "conv2.weight",
                "conv2.bias",
            ] {
                keys.push(format!("{}.{}", p, s));
            }
            if in_ch != out_ch {
                keys.push(format!("{}.conv_shortcut.weight", p));
                keys.push(format!("{}.conv_shortcut.bias", p));
            }
        }
        if i < n_blocks - 1 {
            keys.push(format!(
                "encoder.down_blocks.{}.downsamplers.0.conv.weight",
                i
            ));
            keys.push(format!(
                "encoder.down_blocks.{}.downsamplers.0.conv.bias",
                i
            ));
        }
    }
    // mid_block: 2 resnets × 8 tensors (+ attention when enabled).
    for i in 0..2 {
        let p = format!("encoder.mid_block.resnets.{}", i);
        for s in &[
            "norm1.weight",
            "norm1.bias",
            "conv1.weight",
            "conv1.bias",
            "norm2.weight",
            "norm2.bias",
            "conv2.weight",
            "conv2.bias",
        ] {
            keys.push(format!("{}.{}", p, s));
        }
    }
    if config.mid_block_add_attention {
        let p = "encoder.mid_block.attentions.0";
        for s in &[
            "group_norm.weight",
            "group_norm.bias",
            "to_q.weight",
            "to_q.bias",
            "to_k.weight",
            "to_k.bias",
            "to_v.weight",
            "to_v.bias",
            "to_out.0.weight",
            "to_out.0.bias",
        ] {
            keys.push(format!("{}.{}", p, s));
        }
    }
    keys
}

pub struct VaeEncoder {
    conv_in: Conv2d,
    down_blocks: Vec<Vec<ResnetBlock2D>>,
    downsamplers: Vec<Option<Downsample2D>>,
    mid_resnets: Vec<ResnetBlock2D>,
    mid_attn: Option<VaeAttention>,
    conv_norm_out_weight: Tensor,
    conv_norm_out_bias: Tensor,
    conv_out: Conv2d,
    // diffusers AutoencoderKL applies a 1×1 quant_conv to the encoder
    // output before the mean/logvar split — but the pinned Z-Image file
    // most likely does NOT carry it (244 = 138 decoder + 106 encoder;
    // see the key-generator note). Optional by design: consumed when
    // present, its absence is loud, never silent.
    quant_conv: Option<Conv2d>,
    config: VaeConfig,
}

impl VaeEncoder {
    /// Seeded tiny-init for the №243 tiny contracts (mirror of
    /// `VaeDecoder::new_tiny`; distinct PRNG slots — see the
    /// `PARAM_VAE_ENC_*` constants).
    pub fn new_tiny(config: &VaeConfig, seed: u64) -> Result<Self, String> {
        let device = Device::Cpu;
        let base = config.block_out_channels[0];
        let last = *config
            .block_out_channels
            .last()
            .ok_or_else(|| "VaeEncoder::new_tiny: empty block_out_channels".to_string())?;

        let conv_in = conv2d_seeded(
            3,
            base,
            1,
            3,
            param_seed(seed, 0, PARAM_VAE_ENC_CONV_IN),
            &device,
        )?;

        // down_blocks: layers_per_block resnets each + downsamplers between
        // blocks (channel progression mirrors the diffusers Encoder: conv_in
        // already mapped RGB → base, so block 0 opens at base).
        let n_blocks = config.block_out_channels.len();
        let mut down_blocks: Vec<Vec<ResnetBlock2D>> = Vec::new();
        let mut downsamplers: Vec<Option<Downsample2D>> = Vec::new();
        for i in 0..n_blocks {
            let out_ch = config.block_out_channels[i];
            let block_in = if i == 0 {
                config.block_out_channels[0]
            } else {
                config.block_out_channels[i - 1]
            };
            let mut resnets = Vec::new();
            for j in 0..config.layers_per_block {
                let in_ch = if j == 0 { block_in } else { out_ch };
                resnets.push(ResnetBlock2D::new_seeded(
                    in_ch,
                    out_ch,
                    config.norm_num_groups,
                    1e-6,
                    param_seed(
                        seed,
                        (i as u64 + 1) * 100 + j as u64,
                        PARAM_VAE_ENC_DOWN_RESNET,
                    ),
                    &device,
                )?);
            }
            down_blocks.push(resnets);
            if i < n_blocks - 1 {
                let conv = conv2d_seeded_strided(
                    out_ch,
                    out_ch,
                    1,
                    3,
                    2,
                    param_seed(seed, i as u64, PARAM_VAE_ENC_DOWNSAMPLE),
                    &device,
                )?;
                downsamplers.push(Some(Downsample2D { conv }));
            } else {
                downsamplers.push(None);
            }
        }

        // mid_block: 2 resnets; attention is None for the tiny config
        // (mid_block_add_attention=false — same as the decoder's new_tiny).
        let mid_resnets = vec![
            ResnetBlock2D::new_seeded(
                4 * base,
                4 * base,
                config.norm_num_groups,
                1e-6,
                param_seed(seed, 1, PARAM_VAE_ENC_MID_RESNET),
                &device,
            )?,
            ResnetBlock2D::new_seeded(
                4 * base,
                4 * base,
                config.norm_num_groups,
                1e-6,
                param_seed(seed, 2, PARAM_VAE_ENC_MID_RESNET),
                &device,
            )?,
        ];

        let conv_norm_out_weight =
            Tensor::ones((last,), DType::F32, &device).map_err(|e| e.to_string())?;
        let conv_norm_out_bias =
            Tensor::zeros((last,), DType::F32, &device).map_err(|e| e.to_string())?;

        let conv_out = conv2d_seeded(
            last,
            2 * config.latent_channels, // double_z: mean + logvar halves
            1,
            3,
            param_seed(seed, 998, PARAM_VAE_ENC_CONV_OUT),
            &device,
        )?;
        let quant_conv = conv2d_seeded(
            2 * config.latent_channels,
            2 * config.latent_channels,
            0,
            1,
            param_seed(seed, 999, PARAM_VAE_ENC_QUANT_CONV),
            &device,
        )?;

        Ok(VaeEncoder {
            conv_in,
            down_blocks,
            downsamplers,
            mid_resnets,
            mid_attn: None,
            conv_norm_out_weight,
            conv_norm_out_bias,
            conv_out,
            quant_conv: Some(quant_conv),
            config: config.clone(),
        })
    }

    /// Build from the real safetensors map (Наряд №243 R6.2): consumes the
    /// NON-decoder prefixes of the pinned VAE file (the same map that
    /// feeds `VaeDecoder::from_weights`). A missing expected key is a loud
    /// Err listing every missing tensor; unexpected non-decoder leftovers
    /// (`post_quant_conv.*` — decode-path tensors this MVP does not
    /// consume) are reported loudly as an informational note, NOT an
    /// error, and NOT silently swallowed.
    pub fn from_weights(tensors: &HashMap<String, Tensor>) -> Result<Self, String> {
        let config = zimage_turbo_vae_config();
        let device = Device::Cpu;
        let base = config.block_out_channels[0];
        let last = *config
            .block_out_channels
            .last()
            .ok_or_else(|| "VaeEncoder::from_weights: empty block_out_channels".to_string())?;

        let get = |name: &str, shape: &[usize]| -> Result<Tensor, String> {
            let t = tensors
                .get(name)
                .ok_or_else(|| format!("VaeEncoder::from_weights: tensor '{}' not found", name))?;
            let t = t
                .to_device(&device)
                .map_err(|e| format!("'{}' device: {}", name, e))?;
            let t = t
                .to_dtype(DType::F32)
                .map_err(|e| format!("'{}' dtype: {}", name, e))?;
            if t.dims() != shape {
                return Err(format!(
                    "VaeEncoder::from_weights: '{}' shape mismatch — expected {:?}, got {:?}",
                    name,
                    shape,
                    t.dims()
                ));
            }
            Ok(t)
        };

        let ci_w = get("encoder.conv_in.weight", &[base, 3, 3, 3])?;
        let ci_b = get("encoder.conv_in.bias", &[base])?;
        let conv_in = Conv2d::new(
            ci_w,
            Some(ci_b),
            Conv2dConfig {
                padding: 1,
                ..Default::default()
            },
        );

        let n_blocks = config.block_out_channels.len();
        let mut down_blocks: Vec<Vec<ResnetBlock2D>> = Vec::new();
        let mut downsamplers: Vec<Option<Downsample2D>> = Vec::new();
        for i in 0..n_blocks {
            let out_ch = config.block_out_channels[i];
            // conv_in already mapped RGB → base: block 0 opens at base.
            let block_in = if i == 0 {
                config.block_out_channels[0]
            } else {
                config.block_out_channels[i - 1]
            };
            let mut resnets = Vec::new();
            for j in 0..config.layers_per_block {
                let in_ch = if j == 0 { block_in } else { out_ch };
                resnets.push(ResnetBlock2D::from_weights(
                    tensors,
                    &format!("encoder.down_blocks.{}.resnets.{}", i, j),
                    in_ch,
                    out_ch,
                    config.norm_num_groups,
                    1e-6,
                    &device,
                )?);
            }
            down_blocks.push(resnets);
            if i < n_blocks - 1 {
                let ds_w = get(
                    &format!("encoder.down_blocks.{}.downsamplers.0.conv.weight", i),
                    &[out_ch, out_ch, 3, 3],
                )?;
                let ds_b = get(
                    &format!("encoder.down_blocks.{}.downsamplers.0.conv.bias", i),
                    &[out_ch],
                )?;
                let conv = Conv2d::new(
                    ds_w,
                    Some(ds_b),
                    Conv2dConfig {
                        padding: 1,
                        stride: 2,
                        ..Default::default()
                    },
                );
                downsamplers.push(Some(Downsample2D { conv }));
            } else {
                downsamplers.push(None);
            }
        }

        let mid_resnets = vec![
            ResnetBlock2D::from_weights(
                tensors,
                "encoder.mid_block.resnets.0",
                4 * base,
                4 * base,
                config.norm_num_groups,
                1e-6,
                &device,
            )?,
            ResnetBlock2D::from_weights(
                tensors,
                "encoder.mid_block.resnets.1",
                4 * base,
                4 * base,
                config.norm_num_groups,
                1e-6,
                &device,
            )?,
        ];
        let mid_attn = if config.mid_block_add_attention {
            Some(VaeAttention::from_weights(
                tensors,
                "encoder.mid_block.attentions.0",
                4 * base,
                config.norm_num_groups,
                1e-6,
                &device,
            )?)
        } else {
            None
        };

        let cno_w = get("encoder.conv_norm_out.weight", &[last])?;
        let cno_b = get("encoder.conv_norm_out.bias", &[last])?;
        let co_w = get(
            "encoder.conv_out.weight",
            &[2 * config.latent_channels, last, 3, 3],
        )?;
        let co_b = get("encoder.conv_out.bias", &[2 * config.latent_channels])?;
        let conv_out = Conv2d::new(
            co_w,
            Some(co_b),
            Conv2dConfig {
                padding: 1,
                ..Default::default()
            },
        );
        // quant_conv: OPTIONAL (see the key-generator note — the pinned
        // file most likely does not carry it: 138 + 106 = 244). Present →
        // loaded with the pinned shape; absent → None with a loud note.
        // Silence would hide an architecture divergence — forbidden.
        let quant_conv = match (
            tensors.get("quant_conv.weight"),
            tensors.get("quant_conv.bias"),
        ) {
            (Some(_), Some(_)) => {
                let qc_w = get(
                    "quant_conv.weight",
                    &[2 * config.latent_channels, 2 * config.latent_channels, 1, 1],
                )?;
                let qc_b = get("quant_conv.bias", &[2 * config.latent_channels])?;
                Some(Conv2d::new(qc_w, Some(qc_b), Conv2dConfig::default()))
            }
            (None, None) => {
                eprintln!(
                    "[VaeEncoder::from_weights] loud note: no quant_conv tensors in the \
                     file — the posterior split consumes conv_out directly (consistent \
                     with 138 decoder + 106 encoder = 244; verify against the header \
                     per the runbook edit-e2e checklist)"
                );
                None
            }
            _ => {
                return Err(
                    "VaeEncoder::from_weights: quant_conv is half-present (weight without \
                     bias or vice versa) — a loud refusal, not a guess"
                        .to_string(),
                )
            }
        };

        // Key-level guard: every expected encoder tensor must be present.
        // Missing = loud Err with the FULL missing list (№243 Block 1.1).
        // Non-decoder leftovers that this loader does not consume
        // (post_quant_conv.*) are named loudly — honest visibility without
        // failing the load (they belong to the decode path).
        let expected_keys = vae_expected_encoder_keys(&config);
        let loaded_non_decoder: Vec<String> = tensors
            .keys()
            .filter(|k| !k.starts_with("decoder."))
            .cloned()
            .collect();
        let loaded_set: std::collections::HashSet<&str> =
            loaded_non_decoder.iter().map(|s| s.as_str()).collect();
        let missing: Vec<&String> = expected_keys
            .iter()
            .filter(|k| !loaded_set.contains(k.as_str()))
            .collect();
        if !missing.is_empty() {
            let list = missing
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!(
                "VaeEncoder::from_weights: {} expected encoder tensor(s) missing from the \
                 pinned VAE file: [{}] — verify the file against the weights manifest \
                 (fetched 2026-09-08) and the runbook edit-e2e checklist",
                missing.len(),
                &list[..list.len().min(1200)]
            ));
        }
        let expected_set: std::collections::HashSet<&str> =
            expected_keys.iter().map(|s| s.as_str()).collect();
        let leftovers: Vec<&String> = loaded_non_decoder
            .iter()
            .filter(|k| !expected_set.contains(k.as_str()))
            .collect();
        if !leftovers.is_empty() {
            eprintln!(
                "[VaeEncoder::from_weights] loud note: {} non-decoder tensor(s) present in \
                 the file but not consumed by this encoder (decode-path tensors, e.g. \
                 post_quant_conv.*): {:?}",
                leftovers.len(),
                &leftovers[..leftovers.len().min(8)]
            );
        }

        Ok(VaeEncoder {
            conv_in,
            down_blocks,
            downsamplers,
            mid_resnets,
            mid_attn,
            conv_norm_out_weight: cno_w,
            conv_norm_out_bias: cno_b,
            conv_out,
            quant_conv,
            config,
        })
    }

    /// Encode an image `[3, H, W]` F32 in **[-1, 1]** to a model latent
    /// `[1, latent_channels, H/f, W/f]` (posterior MODE, then the ritual
    /// inverse of the decoder's `z_vae = z_model / scaling + shift`):
    /// `z_model = (mean - shift_factor) * scaling_factor`.
    ///
    /// The caller is responsible for the divisibility contract
    /// (H/W must be multiples of `vae_downsample_factor(&config)`) — the
    /// edit glue checks this loudly BEFORE calling encode; a silent floor
    /// here would be a silent resize (forbidden, №243 Block 1.4).
    pub fn encode(&self, img: &Tensor) -> Result<Tensor, String> {
        let device = img.device();
        let img = img
            .to_dtype(DType::F32)
            .map_err(|e| format!("VAE encode: dtype: {}", e))?;
        let dims = img.dims();
        if dims.len() != 3 || dims[0] != 3 {
            return Err(format!(
                "VAE encode: expected [3, H, W] image in [-1,1], got {:?}",
                dims
            ));
        }
        let x = img
            .unsqueeze(0)
            .map_err(|e| format!("VAE encode: unsqueeze: {}", e))?; // [1,3,H,W]

        let mut h = self
            .conv_in
            .forward(&x)
            .map_err(|e| format!("VAE encode: conv_in forward: {}", e))?;

        for (i, resnets) in self.down_blocks.iter().enumerate() {
            for (j, r) in resnets.iter().enumerate() {
                h = r
                    .forward(&h)
                    .map_err(|e| format!("VAE encode: down_block {} resnet {}: {}", i, j, e))?;
            }
            if let Some(Some(ref ds)) = self.downsamplers.get(i) {
                h = ds
                    .forward(&h)
                    .map_err(|e| format!("VAE encode: down_block {} downsample: {}", i, e))?;
            }
        }

        // Mid-block: resnets[0] → attention → resnets[1] (same order as the
        // decoder's mid-block, diffusers UNetMidBlock2D.forward).
        h = self.mid_resnets[0]
            .forward(&h)
            .map_err(|e| format!("VAE encode: mid resnet 0: {}", e))?;
        if let Some(ref attn) = self.mid_attn {
            h = attn
                .forward(&h)
                .map_err(|e| format!("VAE encode: mid attn: {}", e))?;
        }
        h = self.mid_resnets[1]
            .forward(&h)
            .map_err(|e| format!("VAE encode: mid resnet 1: {}", e))?;

        let h = group_norm(
            &h,
            self.config.norm_num_groups,
            *self.config.block_out_channels.last().expect("non-empty"),
            &self.conv_norm_out_weight,
            &self.conv_norm_out_bias,
            1e-6,
        )
        .map_err(|e| format!("VAE encode: conv_norm_out: {}", e))?;
        let h = silu(&h).map_err(|e| format!("VAE encode: conv_norm_out silu: {}", e))?;
        let h = self
            .conv_out
            .forward(&h)
            .map_err(|e| format!("VAE encode: conv_out: {}", e))?;

        // Split the double_z output into mean / logvar halves (channel dim)
        // and take the posterior MODE (mean) — deterministic encode.
        let h = match &self.quant_conv {
            Some(qc) => qc
                .forward(&h)
                .map_err(|e| format!("VAE encode: quant_conv: {}", e))?,
            None => h, // loud absence note at load time (from_weights)
        };
        let c = self.config.latent_channels;
        let mean = h
            .narrow(1, 0, c)
            .map_err(|e| format!("VAE encode: mean split: {}", e))?;

        // Ritual inverse of the decoder's n232 direction:
        // decode: z_vae = z_model / scaling + shift  →  z_model = (z_vae - shift) * scaling
        let shift = self.config.shift_factor as f32;
        let scale = self.config.scaling_factor as f32;
        let shift_t = scalar_full(shift, mean.dims(), device)
            .map_err(|e| format!("VAE encode: shift tensor: {}", e))?;
        let z = (&mean - &shift_t).map_err(|e| format!("VAE encode: sub shift: {}", e))?;
        let scale_t = scalar_full(scale, z.dims(), device)
            .map_err(|e| format!("VAE encode: scale tensor: {}", e))?;
        let z = (&z * &scale_t).map_err(|e| format!("VAE encode: mul scaling: {}", e))?;
        Ok(z)
    }

    /// The config this encoder was built for (the downsample factor and
    /// latent-channel contract derive from it).
    pub fn config(&self) -> &VaeConfig {
        &self.config
    }
}

/// Decode PNG bytes into a `[3, H, W]` F32 image tensor in [0,1]
/// (Наряд №243 R6.2) — the decode half of `encode_png`, used by the edit
/// path to turn the source artifact's PNG bytes back into the image the
/// VAE encoder consumes. RGB, 8-bit per channel (the same format
/// `encode_png` produces — the source artifacts were born here).
pub fn decode_png(png_bytes: &[u8]) -> Result<Tensor, String> {
    use image::ImageBuffer;
    let img = image::load_from_memory(png_bytes)
        .map_err(|e| format!("decode_png: PNG decode failed: {}", e))?;
    let rgb: ImageBuffer<image::Rgb<u8>, Vec<u8>> = img.to_rgb8();
    let (w, h) = (rgb.width() as usize, rgb.height() as usize);
    let mut vals = Vec::with_capacity(3 * h * w);
    for plane in 0..3 {
        for y in 0..h {
            for x in 0..w {
                let p = rgb.get_pixel(x as u32, y as u32).0;
                vals.push(p[plane] as f32 / 255.0);
            }
        }
    }
    Tensor::from_vec(vals, (3usize, h, w), &Device::Cpu)
        .map_err(|e| format!("decode_png: tensor: {}", e))
}

/// Save a `[3, H, W]` F32 image tensor (in [0,1]) as a PNG file.
///
/// Наряд №240 (R4.2): thin wrapper over `encode_png` — the encoding logic
/// is shared with the `vision_generate` dispatch (PNG bytes go into the
/// `VisionRegistry`), only the file write differs.
pub fn save_png(img: &Tensor, path: &Path) -> Result<(), String> {
    let bytes = encode_png(img)?;
    std::fs::write(path, &bytes)
        .map_err(|e| format!("save_png: write to {}: {}", path.display(), e))
}

/// Encode a `[3, H, W]` F32 image tensor (in [0,1]) as PNG bytes.
///
/// Наряд №240 (R4.2): encoding half of the former `save_png`, factored out
/// so the `vision_generate` dispatch can put real PNG bytes into the
/// `VisionRegistry` without touching the filesystem. Bit-identical output
/// to `save_png` (same encoder, same pixel math).
pub fn encode_png(img: &Tensor) -> Result<Vec<u8>, String> {
    use image::{ImageBuffer, Rgb};
    use std::io::Write as _;
    let img = img
        .to_dtype(DType::F32)
        .map_err(|e| format!("encode_png: dtype: {}", e))?;
    let img = img
        .contiguous()
        .map_err(|e| format!("encode_png: contiguous: {}", e))?;
    let dims = img.dims();
    if dims.len() != 3 || dims[0] != 3 {
        return Err(format!("encode_png: expected [3, H, W], got {:?}", dims));
    }
    let (h, w) = (dims[1], dims[2]);
    let vals = img
        .flatten_all()
        .map_err(|e| format!("encode_png: flatten: {}", e))?
        .to_vec1::<f32>()
        .map_err(|e| format!("encode_png: to_vec1: {}", e))?;

    let mut img_buf: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(w as u32, h as u32);
    let plane_size = h * w;
    for y in 0..h {
        for x in 0..w {
            let r = (vals[0 * plane_size + y * w + x] * 255.0).clamp(0.0, 255.0) as u8;
            let g = (vals[1 * plane_size + y * w + x] * 255.0).clamp(0.0, 255.0) as u8;
            let b = (vals[2 * plane_size + y * w + x] * 255.0).clamp(0.0, 255.0) as u8;
            img_buf.put_pixel(x as u32, y as u32, Rgb([r, g, b]));
        }
    }
    let mut cursor = std::io::Cursor::new(Vec::new());
    img_buf
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| format!("encode_png: PNG encode: {}", e))?;
    Ok(cursor.into_inner())
}
