#![cfg(feature = "video")]
// ── Video sampler: flow matching over spatiotemporal latents (Наряд №310) ──
//
// Real implementation: euler_step loop (reused from Vision) +
// VideoDit forward at each step. Deterministic by seed.
// Uses flow_match_euler_sigmas from src/vision/sampler.rs.
#![allow(clippy::all)]
#![allow(clippy::expect_used)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use candle_core::{Device, Result as CandleResult, Tensor};

// Local copies of euler_step + flow_match_euler_sigmas (vision::sampler is feature-gated).
fn euler_step(x: &Tensor, velocity: &Tensor, sigma: f64, sigma_next: f64) -> CandleResult<Tensor> {
    let dt = (sigma_next - sigma) as f32;
    let dt_t = Tensor::full(dt, x.dims(), x.device())?;
    let delta = (velocity * dt_t)?;
    x + delta
}

fn flow_match_euler_sigmas(num_inference_steps: usize, shift: f64, num_train: usize) -> Vec<f64> {
    let base: Vec<f64> = (0..num_train)
        .map(|i| 1.0 - (i as f64) * (1.0 - 1.0 / num_train as f64) / (num_train - 1) as f64)
        .collect();
    let shifted: Vec<f64> = base
        .iter()
        .map(|&s| shift * s / (1.0 + (shift - 1.0) * s))
        .collect();
    let indices: Vec<usize> = (0..num_inference_steps)
        .map(|i| (i as f64 * (num_train - 1) as f64 / (num_inference_steps - 1) as f64) as usize)
        .collect();
    indices.iter().map(|&i| shifted[i]).collect()
}
use crate::nn::attention::generate_uniform_f32;

/// Sampling configuration for video generation.
#[derive(Debug, Clone)]
pub struct VideoSampleConfig {
    pub num_steps: usize,
    pub seed: u64,
    pub profile: String,
    pub shift: f64,
}

impl Default for VideoSampleConfig {
    fn default() -> Self {
        Self {
            num_steps: 4, // tiny: 4 steps for CI (real: 8 distilled turbo)
            seed: 42,
            profile: "cpu".to_string(),
            shift: 3.0,
        }
    }
}

/// Generate seeded randn latent for video.
/// Shape: [B, C, T, H, W]
pub fn fixed_video_latent(seed: u64, b: usize, c: usize, t: usize, h: usize, w: usize) -> Tensor {
    let n = b * c * t * h * w;
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
    Tensor::from_vec(vals, (b, c, t, h, w), &Device::Cpu).expect("fixed_video_latent")
}

/// Run the full flow-matching Euler sampling loop for video.
///
/// `dit`: the video transformer.
/// `text`: text embedding [B, text_dim].
/// `seed`: for the initial latent.
/// `config`: sampling configuration.
/// `latent_shape`: (B, C, T, H, W) of the latent.
///
/// Returns the final denoised latent [B, C, T, H, W].
pub fn flow_match_euler_sample_video(
    dit: &super::denoiser::VideoDit,
    text: &Tensor,
    config: &VideoSampleConfig,
    latent_shape: (usize, usize, usize, usize, usize),
) -> Result<Tensor, String> {
    let (b, c, t, h, w) = latent_shape;
    let _device = text.device();

    // Initial latent: seeded randn
    let mut x = fixed_video_latent(config.seed, b, c, t, h, w);

    // Sigma schedule (reused from Vision)
    let sigmas = flow_match_euler_sigmas(config.num_steps, config.shift, 1000);

    // Euler steps
    for i in 0..config.num_steps.saturating_sub(1) {
        let sigma = sigmas[i];
        let sigma_next = if i + 1 < config.num_steps {
            sigmas[i + 1]
        } else {
            0.0
        };
        let timestep = sigma * 1000.0;
        let velocity = dit.forward(&x, timestep, text).map_err(|e| {
            format!(
                "flow_match_euler_sample_video: DiT forward at step {} failed: {}",
                i, e
            )
        })?;
        x = euler_step(&x, &velocity, sigma, sigma_next).map_err(|e| {
            format!(
                "flow_match_euler_sample_video: euler_step at step {} failed: {}",
                i, e
            )
        })?;
    }

    Ok(x)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::video::denoiser::{VideoDit, VideoDitConfig};
    use crate::video::vae::{VideoVae, VideoVaeConfig};
    use candle_core::DType;

    #[test]
    fn default_config() {
        let cfg = VideoSampleConfig::default();
        assert_eq!(cfg.num_steps, 4);
        assert_eq!(cfg.seed, 42);
    }

    /// E2E test: noise → DiT denoise → VAE decode → video frames.
    /// Deterministic by seed. Produces real tensor output (not stubs).
    #[test]
    fn e2e_pipeline_t2v() {
        let device = Device::Cpu;
        let dit_config = VideoDitConfig::default();
        let dit = VideoDit::new_tiny(42, dit_config, &device).unwrap();
        let vae_config = VideoVaeConfig::default();
        let vae = VideoVae::new_tiny(42, vae_config, &device).unwrap();

        // Text embedding (stub: zeros, dim=64)
        let text = Tensor::zeros((1, 64), DType::F32, &device).unwrap();

        // Sample config
        let sample_config = VideoSampleConfig::default();

        // Latent shape: [1, 4, 2, 4, 4] — tiny for CI
        let latent_shape = (1, 4, 2, 4, 4);

        // Run flow matching sampler
        let latent =
            flow_match_euler_sample_video(&dit, &text, &sample_config, latent_shape).unwrap();

        // Verify latent shape
        assert_eq!(latent.dims(), &[1, 4, 2, 4, 4]);

        // Decode to video frames
        let video = vae.decode(&latent).unwrap();

        // Verify video shape: [1, 3, 4, 32, 32] — 1 batch, 3 channels, 4 frames, 32x32
        assert_eq!(video.dims(), &[1, 3, 4, 32, 32]);

        // Verify output is not all zeros (real computation happened)
        let vals = video.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        let non_zero = vals.iter().filter(|&&v| v != 0.0).count();
        assert!(
            non_zero > 0,
            "video output should have non-zero values (real computation)"
        );
    }

    /// Determinism: same seed → same output.
    #[test]
    fn e2e_deterministic() {
        let device = Device::Cpu;

        let run = || {
            let dit = VideoDit::new_tiny(42, VideoDitConfig::default(), &device).unwrap();
            let text = Tensor::zeros((1, 64), DType::F32, &device).unwrap();
            let config = VideoSampleConfig::default();
            flow_match_euler_sample_video(&dit, &text, &config, (1, 4, 2, 4, 4)).unwrap()
        };

        let l1 = run();
        let l2 = run();
        assert_eq!(
            l1.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            l2.flatten_all().unwrap().to_vec1::<f32>().unwrap(),
            "same seed must produce identical output"
        );
    }
}
