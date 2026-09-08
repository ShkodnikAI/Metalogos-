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
    conv_out: Conv2d,
    config: VaeConfig,
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
        let mut up_blocks: Vec<Vec<ResnetBlock2D>> = Vec::new();
        let mut up_samplers: Vec<Option<Upsample2D>> = Vec::new();
        let mut in_ch = 4 * base;
        let rev_blocks: Vec<usize> = config.block_out_channels.iter().rev().copied().collect();
        for (i, &out_ch) in rev_blocks.iter().enumerate() {
            let mut resnets = Vec::new();
            let num_resnets = if i == rev_blocks.len() - 1 {
                config.layers_per_block + 1
            } else {
                config.layers_per_block
            };
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
            let num_resnets = if i == rev_blocks.len() - 1 {
                config.layers_per_block + 1
            } else {
                config.layers_per_block
            };
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

        // n233 Block 2: loader tensor-coverage guard.
        // VAE total: 244 (encoder + decoder). Decoder-only: 138.
        // We only load decoder.* tensors, so verify count matches.
        let loaded_decoder_count = tensors.keys().filter(|k| k.starts_with("decoder.")).count();
        let expected_decoder_count = 138; // verified from safetensors header 2026-09-08
        if loaded_decoder_count != expected_decoder_count {
            return Err(format!(
                "VaeDecoder::from_weights: decoder tensor count mismatch — expected {}, got {}",
                expected_decoder_count, loaded_decoder_count
            ));
        }

        Ok(VaeDecoder {
            conv_in,
            mid_resnets,
            mid_attn,
            up_blocks,
            up_samplers,
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

/// Save a `[3, H, W]` F32 image tensor (in [0,1]) as a PNG file.
pub fn save_png(img: &Tensor, path: &Path) -> Result<(), String> {
    use image::{ImageBuffer, Rgb};
    let img = img
        .to_dtype(DType::F32)
        .map_err(|e| format!("save_png: dtype: {}", e))?;
    let img = img
        .contiguous()
        .map_err(|e| format!("save_png: contiguous: {}", e))?;
    let dims = img.dims();
    if dims.len() != 3 || dims[0] != 3 {
        return Err(format!("save_png: expected [3, H, W], got {:?}", dims));
    }
    let (h, w) = (dims[1], dims[2]);
    let vals = img
        .flatten_all()
        .map_err(|e| format!("save_png: flatten: {}", e))?
        .to_vec1::<f32>()
        .map_err(|e| format!("save_png: to_vec1: {}", e))?;

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
    img_buf
        .save(path)
        .map_err(|e| format!("save_png: save to {}: {}", path.display(), e))
}
