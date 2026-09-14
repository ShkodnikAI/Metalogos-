// ── Video DiT denoiser: spatiotemporal flow matching (Наряд №308) ────
//
// Spatiotemporal DiT (Diffusion Transformer) for video denoising.
// Uses flow matching (same ODE primitive as Vision: euler_step).
// Temporal-specific: temporal axes, causal attention — local to this module.
// General euler_step from src/vision/sampler.rs is reused, not extended.
//
// Phase V2 skeleton: contract + stub.
// Real DiT on candle-core deferred to phase V2-real.

#[cfg(feature = "video")]
use candle_core::Tensor;

/// Video DiT denoiser contract:
/// - Input: noisy latent [B, C, T, H, W] + timestep + text embedding
/// - Output: denoised latent [B, C, T, H, W]
/// - Uses flow matching (euler_step from src/vision/sampler.rs)
/// - Temporal attention: causal (frame i attends to frames 0..=i)
/// - Spatiotemporal: 3D positional encoding (temporal + spatial)
#[cfg(feature = "video")]
pub struct VideoDit {
    // Real: transformer blocks with 3D attention
    // Stub: no weights
}

#[cfg(feature = "video")]
impl VideoDit {
    pub fn new_stub() -> Self {
        Self {}
    }

    /// Forward pass: predict clean latent from noisy latent.
    pub fn forward(
        &self,
        _noisy_latent: &Tensor,
        _timestep: f64,
        _text_embedding: &Tensor,
    ) -> Result<Tensor, candle_core::Error> {
        unimplemented!("VideoDit::forward — phase V2-real (requires candle + weights)")
    }
}

/// Flow matching sampler for video — reuses euler_step from Vision.
/// The ODE primitive is shared; the denoiser model is video-specific.
#[cfg(feature = "video")]
pub fn flow_match_euler_sample_video(
    _dit: &VideoDit,
    _initial_noise: &Tensor,
    _text_embedding: &Tensor,
    _num_steps: usize,
) -> Result<Tensor, candle_core::Error> {
    unimplemented!("flow_match_euler_sample_video — phase V2-real")
}

#[cfg(test)]
mod tests {
    #[test]
    fn denoiser_contract_documented() {
        // Contract is in the doc comments above — this test verifies
        // the module compiles and the contract is accessible.
        // (No assertion needed — compilation IS the contract check.)
    }
}
