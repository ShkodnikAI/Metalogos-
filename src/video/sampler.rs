// ── Video sampler: flow matching over spatiotemporal latents (Наряд №308) ──
//
// The video sampler orchestrates the denoising loop:
//   1. Start with random noise in latent space
//   2. Apply euler_step (shared ODE primitive from src/vision/sampler.rs)
//   3. Call VideoDit::forward at each step
//   4. Decode final latent to video frames via VideoVae
//
// This module is the integration point — VAE and DiT are separate modules.
// The euler_step ODE primitive is reused from Vision (rule of three
// assessment in docs/research/video-rule-of-three.md).

/// Sampling configuration for video generation.
#[derive(Debug, Clone)]
pub struct VideoSampleConfig {
    /// Number of denoising steps (distilled turbo: 4-8; full: 20-50).
    pub num_steps: usize,
    /// Random seed for reproducibility.
    pub seed: u64,
    /// Profile: "cpu" | "gpu" | "fp8" | "gguf-q4" (ADR-0147).
    pub profile: String,
}

impl Default for VideoSampleConfig {
    fn default() -> Self {
        Self {
            num_steps: 8, // distilled turbo default
            seed: 42,
            profile: "cpu".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let cfg = VideoSampleConfig::default();
        assert_eq!(cfg.num_steps, 8);
        assert_eq!(cfg.seed, 42);
        assert_eq!(cfg.profile, "cpu");
    }

    #[test]
    fn custom_config() {
        let cfg = VideoSampleConfig {
            num_steps: 50,
            seed: 123,
            profile: "gpu".to_string(),
        };
        assert_eq!(cfg.num_steps, 50);
    }
}
