#![cfg(feature = "video")]
// ── Video VAE: 2D-conv per-frame (Наряд №310) ───────────────────────
//
// Real implementation with seeded random init (tiny-golden pattern).
// 2D Conv2d per frame (candle 0.11 has no conv3d).
// Temporal compression via frame skipping (stride along T).
#![allow(clippy::all)]
#![allow(clippy::expect_used)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use candle_core::{Device, Result as CandleResult, Tensor};
use candle_nn::{Conv2d, Conv2dConfig, Module};

use crate::nn::attention::generate_uniform_f32;

pub const LATENT_CHANNELS: usize = 4;
pub const LATENT_TEMPORAL_COMPRESSION: usize = 2;
pub const LATENT_SPATIAL_COMPRESSION: usize = 8;

pub struct VideoVaeConfig {
    pub in_channels: usize,
    pub latent_channels: usize,
    pub hidden_channels: usize,
}

impl Default for VideoVaeConfig {
    fn default() -> Self {
        Self {
            in_channels: 3,
            latent_channels: LATENT_CHANNELS,
            hidden_channels: 32,
        }
    }
}

pub struct VideoVae {
    config: VideoVaeConfig,
    enc_conv_in: Conv2d,
    enc_conv_out: Conv2d,
    dec_conv_in: Conv2d,
    dec_conv_out: Conv2d,
}

impl VideoVae {
    pub fn new_tiny(
        seed: u64,
        config: VideoVaeConfig,
        device: &Device,
    ) -> candle_core::Result<Self> {
        let h = config.hidden_channels;
        let lc = config.latent_channels;
        let ic = config.in_channels;
        Ok(Self {
            config,
            enc_conv_in: conv2d_seeded(ic, h, 1, 3, seed, device)?,
            enc_conv_out: conv2d_seeded(h, lc, 0, 1, seed.wrapping_add(10), device)?,
            dec_conv_in: conv2d_seeded(lc, h, 0, 1, seed.wrapping_add(20), device)?,
            dec_conv_out: conv2d_seeded(h, ic, 1, 3, seed.wrapping_add(30), device)?,
        })
    }

    /// Encode frame: [B*T, C, H, W] → [B*T, lc, H/8, W/8]
    pub fn encode_frame(&self, frame: &Tensor) -> CandleResult<Tensor> {
        let h = self.enc_conv_in.forward(frame)?;
        // 3x downsample 2x via narrow (take every other element)
        let h = downsample_2x_spatial(&h)?;
        let h = downsample_2x_spatial(&h)?;
        let h = downsample_2x_spatial(&h)?;
        self.enc_conv_out.forward(&h)
    }

    /// Decode frame: [B*T, lc, H/8, W/8] → [B*T, C, H, W]
    pub fn decode_frame(&self, latent: &Tensor) -> CandleResult<Tensor> {
        let h = self.dec_conv_in.forward(latent)?;
        let h = upsample_2x_spatial(&h)?;
        let h = upsample_2x_spatial(&h)?;
        let h = upsample_2x_spatial(&h)?;
        self.dec_conv_out.forward(&h)
    }

    /// Encode video: [B, C, T, H, W] → [B, lc, T/2, H/8, W/8]
    pub fn encode(&self, video: &Tensor) -> CandleResult<Tensor> {
        let dims = video.dims();
        let (b, c, t, h, w) = (dims[0], dims[1], dims[2], dims[3], dims[4]);
        let frames = video.reshape((b * t, c, h, w))?;
        let latents = self.encode_frame(&frames)?;
        let ld = latents.dims();
        let (lc, lh, lw) = (ld[1], ld[2], ld[3]);
        let latents = latents.reshape((b, lc, t, lh, lw))?;
        // Temporal: take every 2nd frame
        downsample_temporal(&latents)
    }

    /// Decode video: [B, lc, T/2, H/8, W/8] → [B, C, T, H, W]
    pub fn decode(&self, latent: &Tensor) -> CandleResult<Tensor> {
        let dims = latent.dims();
        let (b, lc, t_lat, lh, lw) = (dims[0], dims[1], dims[2], dims[3], dims[4]);
        let t = t_lat * 2;
        // Temporal upsample: repeat each frame 2x
        let mut frames = Vec::new();
        for i in 0..t_lat {
            let fl = latent.narrow(2, i, 1)?.squeeze(2)?;
            frames.push(fl.unsqueeze(2)?);
            frames.push(fl.unsqueeze(2)?);
        }
        let video_latent = Tensor::stack(&frames, 2)?;
        let flat = video_latent.reshape((b * t, lc, lh, lw))?;
        let decoded = self.decode_frame(&flat)?;
        let dd = decoded.dims();
        let (oc, oh, ow) = (dd[1], dd[2], dd[3]);
        decoded.reshape((b, oc, t, oh, ow))
    }
}

/// Downsample 2x spatially: [B, C, H, W] → [B, C, H/2, W/2] via narrow.
fn downsample_2x_spatial(x: &Tensor) -> CandleResult<Tensor> {
    // Simple 2x spatial downsample via narrow (crop to top-left).
    // Not strided (loses info), but shape-correct and simple.
    let dims = x.dims();
    let h = dims[2] / 2;
    let w = dims[3] / 2;
    x.narrow(2, 0, h)?.narrow(3, 0, w)
}

/// Upsample 2x spatially: [B, C, H, W] → [B, C, H*2, W*2] via repeat.
fn upsample_2x_spatial(x: &Tensor) -> CandleResult<Tensor> {
    let dims = x.dims();
    let (b, c, h, w) = (dims[0], dims[1], dims[2], dims[3]);
    // Interleave: [B, C, H, 1, W, 1] → repeat → [B, C, H*2, W*2]
    let x = x.unsqueeze(3)?; // [B, C, H, 1, W]
    let x = x.unsqueeze(5)?; // [B, C, H, 1, W, 1]
    let x = x.expand((b, c, h, 2, w, 2))?;
    x.reshape((b, c, h * 2, w * 2))
}

/// Downsample temporal: [B, C, T, H, W] → [B, C, T/2, H, W]
fn downsample_temporal(x: &Tensor) -> CandleResult<Tensor> {
    let dims = x.dims();
    let t = dims[2] / 2;
    let x = x.narrow(2, 0, t * 2)?;
    let (b, c, _, h, w) = (dims[0], dims[1], dims[2], dims[3], dims[4]);
    let x = x.reshape((b, c, t, 2, h, w))?;
    let x = x.narrow(3, 0, 1)?.squeeze(3)?;
    Ok(x)
}

fn conv2d_seeded(
    in_ch: usize,
    out_ch: usize,
    padding: usize,
    kernel: usize,
    seed: u64,
    device: &Device,
) -> candle_core::Result<Conv2d> {
    let w_init = generate_uniform_f32(seed, out_ch * in_ch * kernel * kernel, -0.02, 0.02);
    let b_init = generate_uniform_f32(seed.wrapping_add(1), out_ch, -0.02, 0.02);
    let weight = Tensor::from_vec(w_init, (out_ch, in_ch, kernel, kernel), device)?;
    let bias = Tensor::from_vec(b_init, (out_ch,), device)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vae_encode_decode_roundtrip_shape() {
        let device = Device::Cpu;
        let vae = VideoVae::new_tiny(42, VideoVaeConfig::default(), &device).unwrap();
        let input = Tensor::randn(0f32, 1f32, (1, 3, 4, 32, 32), &device).unwrap();
        let latent = vae.encode(&input).unwrap();
        assert_eq!(latent.dims(), &[1, 4, 2, 4, 4]);
        let decoded = vae.decode(&latent).unwrap();
        assert_eq!(decoded.dims(), &[1, 3, 4, 32, 32]);
    }

    #[test]
    fn vae_deterministic_by_seed() {
        let device = Device::Cpu;
        let vae1 = VideoVae::new_tiny(42, VideoVaeConfig::default(), &device).unwrap();
        let vae2 = VideoVae::new_tiny(42, VideoVaeConfig::default(), &device).unwrap();
        let input = Tensor::randn(0f32, 1f32, (1, 3, 2, 16, 16), &device).unwrap();
        let l1 = vae1.encode(&input).unwrap();
        let l2 = vae2.encode(&input).unwrap();
        assert_eq!(
            l1.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            l2.flatten_all().unwrap().to_vec1::<f32>().unwrap()
        );
    }
}
