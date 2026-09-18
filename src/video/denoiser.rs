#![cfg(feature = "video")]
// ── Video DiT: spatiotemporal transformer denoiser (Наряд №310) ──────
//
// Real implementation with tiny random init. Architecture:
// - Patch embed → temporal attention → spatial attention → MLP → output
// Uses candle 0.11 API correctly.
#![allow(clippy::all)]
#![allow(clippy::expect_used)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use candle_core::{DType, Device, Result as CandleResult, Tensor};
use candle_nn::{Linear, Module};

use crate::nn::attention::generate_uniform_f32;

pub struct VideoDitConfig {
    pub latent_channels: usize,
    pub hidden_dim: usize,
    pub patch_size: usize,
    pub num_layers: usize,
    /// Width of the prompt embedding consumed by the text path
    /// (ADR-0153 D1; matches `i2v::hash_embedding`, default call sites only).
    pub text_dim: usize,
}

impl Default for VideoDitConfig {
    fn default() -> Self {
        Self {
            latent_channels: 4,
            hidden_dim: 64,
            patch_size: 2,
            num_layers: 2,
            text_dim: 64,
        }
    }
}

pub struct VideoDit {
    config: VideoDitConfig,
    patch_embed: Linear,
    time_embed: Linear,
    /// Prompt-embedding projection (ADR-0153 D1): the text path is REAL —
    /// the projected embedding is added to every token at each forward.
    text_embed: Linear,
    layers: Vec<DitLayer>,
    output: Linear,
}

struct DitLayer {
    q: Linear,
    k: Linear,
    v: Linear,
    out: Linear,
    mlp_fc1: Linear,
    mlp_fc2: Linear,
    norm_w: Tensor,
    norm_b: Tensor,
}

impl VideoDit {
    pub fn new_tiny(
        seed: u64,
        config: VideoDitConfig,
        device: &Device,
    ) -> candle_core::Result<Self> {
        let dim = config.hidden_dim;
        let text_dim = config.text_dim;
        let patch_dim = config.latent_channels * config.patch_size * config.patch_size;
        let mut layers = Vec::new();
        for i in 0..config.num_layers {
            let s = seed.wrapping_add((i as u64) * 100 + 10);
            layers.push(DitLayer::new_seeded(dim, s, device)?);
        }
        Ok(Self {
            config,
            patch_embed: linear_seeded(patch_dim, dim, seed, device)?,
            time_embed: linear_seeded(1, dim, seed.wrapping_add(1), device)?,
            // ADR-0153 D1: streams seed+20/+21 — no overlap with patch
            // (0/1), time (1/2), output (999/1000), layers (10..15, 110..115).
            text_embed: linear_seeded(text_dim, dim, seed.wrapping_add(20), device)?,
            layers,
            output: linear_seeded(dim, patch_dim, seed.wrapping_add(999), device)?,
        })
    }

    /// `text`: prompt embedding [B, text_dim] (ADR-0153 D1) — projected and
    /// added to every token. A shape mismatch is a LOUD candle error.
    pub fn forward(&self, x: &Tensor, t: f64, text: &Tensor) -> CandleResult<Tensor> {
        let dims = x.dims();
        let (b, c, t_frames, h, w) = (dims[0], dims[1], dims[2], dims[3], dims[4]);
        let p = self.config.patch_size;
        let dim = self.config.hidden_dim;
        let hp = h / p;
        let wp = w / p;

        // Patchify: [B, C, T, H, W] → [B*T, hp*wp, C*p*p]
        let x_flat = x.reshape((b * t_frames, c, h, w))?;
        let patches = extract_patches(&x_flat, p)?;
        let tokens = self.patch_embed.forward(&patches)?;

        // Time embedding
        let t_t = Tensor::full(t as f32, (1, 1), x.device())?;
        let t_emb = self.time_embed.forward(&t_t)?;
        let t_b = t_emb
            .reshape((1, 1, dim))?
            .broadcast_as((b * t_frames, hp * wp, dim))?;
        let tokens = (tokens + t_b)?;

        // Text conditioning (ADR-0153 D1): validate loudly, project to the
        // token dim, broadcast-add per batch to every token.
        let want_text = [b, self.config.text_dim];
        if text.dims() != want_text.as_slice() {
            return Err(candle_core::Error::Msg(format!(
                "VideoDit::forward: text embedding must be [B, {}] with B = {}, got {:?}",
                self.config.text_dim,
                b,
                text.dims()
            )));
        }
        let txt = self.text_embed.forward(text)?; // [B, dim]
        let txt = txt
            .reshape((b, 1, 1, dim))?
            .broadcast_as((b, t_frames, hp * wp, dim))?;

        // Reshape: [B*T, S, dim] → [B, T, S, dim]
        let h_tok = tokens.reshape((b, t_frames, hp * wp, dim))?;
        let mut h_tok = (h_tok + txt)?;

        // Apply layers
        for layer in &self.layers {
            h_tok = layer.forward(&h_tok, b, t_frames, hp * wp, dim)?;
        }

        // Output: [B, T, S, dim] → [B*T, S, dim] → patches → frames
        let tok_out = h_tok.reshape((b * t_frames, hp * wp, dim))?;
        let patches_out = self.output.forward(&tok_out)?;
        let frames = unpatchify(&patches_out, c, h, w, p)?;
        frames.reshape((b, c, t_frames, h, w))
    }
}

impl DitLayer {
    fn new_seeded(dim: usize, seed: u64, device: &Device) -> candle_core::Result<Self> {
        Ok(Self {
            q: linear_seeded(dim, dim, seed, device)?,
            k: linear_seeded(dim, dim, seed.wrapping_add(1), device)?,
            v: linear_seeded(dim, dim, seed.wrapping_add(2), device)?,
            out: linear_seeded(dim, dim, seed.wrapping_add(3), device)?,
            mlp_fc1: linear_seeded(dim, dim * 4, seed.wrapping_add(4), device)?,
            mlp_fc2: linear_seeded(dim * 4, dim, seed.wrapping_add(5), device)?,
            norm_w: Tensor::ones((dim,), DType::F32, device)?,
            norm_b: Tensor::zeros((dim,), DType::F32, device)?,
        })
    }

    fn forward(
        &self,
        x: &Tensor,
        b: usize,
        t: usize,
        s: usize,
        dim: usize,
    ) -> CandleResult<Tensor> {
        // x: [B, T, S, dim]
        // Flatten to [B*T*S, dim] for norm, then attention over S
        let x_flat = x.reshape((b * t, s, dim))?;
        let x_norm = layer_norm(&x_flat, &self.norm_w, &self.norm_b)?;
        let q = self.q.forward(&x_norm)?;
        let k = self.k.forward(&x_norm)?;
        let v = self.v.forward(&x_norm)?;
        let attn = attention(&q, &k, &v, dim)?;
        let attn = self.out.forward(&attn)?;
        let x_flat = (x_flat + attn)?;

        // MLP
        let x_norm = layer_norm(&x_flat, &self.norm_w, &self.norm_b)?;
        let h = self.mlp_fc1.forward(&x_norm)?;
        let h = relu(&h)?;
        let h = self.mlp_fc2.forward(&h)?;
        let x_flat = (x_flat + h)?;

        x_flat.reshape((b, t, s, dim))
    }
}

fn attention(q: &Tensor, k: &Tensor, v: &Tensor, dim: usize) -> CandleResult<Tensor> {
    let scale = 1.0 / (dim as f64).sqrt();
    let scores = q.matmul(&k.transpose(1, 2)?)?;
    let scale_t = Tensor::full(scale as f32, scores.dims(), scores.device())?;
    let scores = scores.mul(&scale_t)?;
    let attn = candle_nn::ops::softmax(&scores, 2)?;
    attn.matmul(v)
}

fn layer_norm(x: &Tensor, w: &Tensor, b: &Tensor) -> CandleResult<Tensor> {
    let dims = x.dims();
    let mean = x.mean(vec![dims.len() - 1])?;
    let mean = mean.unsqueeze(2)?.broadcast_as(dims)?;
    let diff = x.sub(&mean)?;
    let var = diff.mul(&diff)?.mean(vec![dims.len() - 1])?;
    let var = var.unsqueeze(2)?.broadcast_as(dims)?;
    let eps = Tensor::full(1e-5f32, dims, x.device())?;
    let normed = diff.div(&(var + eps)?.sqrt()?)?;
    let w = w
        .reshape((1, 1, dims[dims.len() - 1]))?
        .broadcast_as(dims)?;
    let b = b
        .reshape((1, 1, dims[dims.len() - 1]))?
        .broadcast_as(dims)?;
    normed.mul(&w)?.add(&b)
}

fn relu(x: &Tensor) -> CandleResult<Tensor> {
    // relu = max(x, 0)
    let zeros = Tensor::zeros(x.dims(), x.dtype(), x.device())?;
    x.maximum(&zeros)
}

fn extract_patches(x: &Tensor, p: usize) -> CandleResult<Tensor> {
    let dims = x.dims();
    let (b, c, h, w) = (dims[0], dims[1], dims[2], dims[3]);
    let (hp, wp) = (h / p, w / p);
    let x = x.reshape((b, c, hp, p, wp, p))?;
    let x = x.permute((0, 2, 4, 1, 3, 5))?;
    x.reshape((b, hp * wp, c * p * p))
}

fn unpatchify(patches: &Tensor, c: usize, h: usize, w: usize, p: usize) -> CandleResult<Tensor> {
    let dims = patches.dims();
    let b = dims[0];
    let (hp, wp) = (h / p, w / p);
    let x = patches.reshape((b, hp, wp, c, p, p))?;
    let x = x.permute((0, 3, 1, 4, 2, 5))?;
    x.reshape((b, c, h, w))
}

fn linear_seeded(
    in_dim: usize,
    out_dim: usize,
    seed: u64,
    device: &Device,
) -> candle_core::Result<Linear> {
    let w = Tensor::from_vec(
        generate_uniform_f32(seed, out_dim * in_dim, -0.02, 0.02),
        (out_dim, in_dim),
        device,
    )?;
    let b = Tensor::from_vec(
        generate_uniform_f32(seed.wrapping_add(1), out_dim, -0.02, 0.02),
        (out_dim,),
        device,
    )?;
    Ok(Linear::new(w, Some(b)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dit_forward_shape() {
        let device = Device::Cpu;
        let dit = VideoDit::new_tiny(42, VideoDitConfig::default(), &device).unwrap();
        let x = Tensor::randn(0f32, 1f32, (1, 4, 2, 4, 4), &device).unwrap();
        let text = Tensor::zeros((1, 64), DType::F32, &device).unwrap();
        let velocity = dit.forward(&x, 500.0, &text).unwrap();
        assert_eq!(velocity.dims(), &[1, 4, 2, 4, 4]);
    }

    /// ADR-0153 D1: the text path is real — different embeddings produce
    /// different velocity fields.
    #[test]
    fn text_conditioning_changes_velocity() {
        let device = Device::Cpu;
        let dit = VideoDit::new_tiny(42, VideoDitConfig::default(), &device).unwrap();
        let x = Tensor::randn(0f32, 1f32, (1, 4, 2, 4, 4), &device).unwrap();
        let zeros = Tensor::zeros((1, 64), DType::F32, &device).unwrap();
        let ones = Tensor::ones((1, 64), DType::F32, &device).unwrap();
        let v0 = dit.forward(&x, 500.0, &zeros).unwrap();
        let v1 = dit.forward(&x, 500.0, &ones).unwrap();
        assert_ne!(
            v0.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            v1.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            "the text path must condition the velocity field"
        );
    }

    /// ADR-0153 D1: same text → identical output (determinism of the path).
    #[test]
    fn text_conditioning_is_deterministic() {
        let device = Device::Cpu;
        let dit = VideoDit::new_tiny(42, VideoDitConfig::default(), &device).unwrap();
        let x = Tensor::randn(0f32, 1f32, (1, 4, 2, 4, 4), &device).unwrap();
        let text = Tensor::randn(0f32, 1f32, (1, 64), &device).unwrap();
        let v1 = dit.forward(&x, 500.0, &text).unwrap();
        let v2 = dit.forward(&x, 500.0, &text).unwrap();
        assert_eq!(
            v1.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            v2.flatten_all().unwrap().to_vec1::<f32>().unwrap()
        );
    }

    /// ADR-0153 D1: shape mismatch is a LOUD error, not silent reuse.
    #[test]
    fn text_shape_mismatch_is_loud() {
        let device = Device::Cpu;
        let dit = VideoDit::new_tiny(42, VideoDitConfig::default(), &device).unwrap();
        let x = Tensor::randn(0f32, 1f32, (1, 4, 2, 4, 4), &device).unwrap();
        let bad_width = Tensor::zeros((1, 32), DType::F32, &device).unwrap();
        let bad_batch = Tensor::zeros((2, 64), DType::F32, &device).unwrap();
        for bad in [&bad_width, &bad_batch] {
            let err = dit.forward(&x, 500.0, bad).unwrap_err();
            assert!(
                err.to_string().contains("text embedding"),
                "loud text-shape error, got: {}",
                err
            );
        }
    }

    #[test]
    fn dit_deterministic_by_seed() {
        let device = Device::Cpu;
        let dit1 = VideoDit::new_tiny(42, VideoDitConfig::default(), &device).unwrap();
        let dit2 = VideoDit::new_tiny(42, VideoDitConfig::default(), &device).unwrap();
        let x = Tensor::randn(0f32, 1f32, (1, 4, 2, 4, 4), &device).unwrap();
        let text = Tensor::zeros((1, 64), DType::F32, &device).unwrap();
        let v1 = dit1.forward(&x, 500.0, &text).unwrap();
        let v2 = dit2.forward(&x, 500.0, &text).unwrap();
        assert_eq!(
            v1.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            v2.flatten_all().unwrap().to_vec1::<f32>().unwrap()
        );
    }
}
