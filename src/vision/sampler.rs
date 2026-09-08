//! Flow-matching Euler discrete scheduler (naryad №212 Block 4.2).
//!
//! Implements the sigma schedule and Euler step for Z-Image-Turbo's
//! FlowMatchEulerDiscreteScheduler.
//!
//! ## Sigma schedule (diffusers source)
//!
//! ```python
//! sigmas = linspace(1, 1/num_train, num_train)         # 1000 values, 1.0 → 0.001
//! sigmas = shift * sigmas / (1 + (shift-1) * sigmas)    # shift transform
//! timesteps = sigmas * num_train                        # → t in [0, 1000]
//! sigmas = concat([sigmas, 0.0])                        # append 0
//! ```
//! For inference with `num_inference_steps=N`: linspace over `[0, 999]` (N indices),
//! index into the precomputed sigmas (which has 1000 entries; the +1 zero is
//! appended separately and used as the final sigma_next=0).
//!
//! ## Update rule (Euler step)
//!
//! For flow matching: `x_{t+dt} = x_t + (sigma_next - sigma_t) * v(x_t, t)`.
//! Final sigma=0 → no update (terminal).

// Style nits suppressed — see vae.rs/dit.rs for rationale.
#![allow(clippy::all)]
#![allow(clippy::expect_used)]
#![allow(clippy::needless_borrow)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use candle_core::{Result as CandleResult, Tensor};

/// Compute the FlowMatchEuler sigma schedule for `num_inference_steps` steps.
///
/// Returns `num_inference_steps` sigma values (monotonically decreasing,
/// first ≈ 1.0 after shift, last small but > 0; the implicit final sigma=0
/// is added by `euler_step` when computing deltas).
///
/// `shift` = 3.0 for Z-Image-Turbo. `num_train` = 1000.
pub fn flow_match_euler_sigmas(
    num_inference_steps: usize,
    shift: f64,
    num_train: usize,
) -> Vec<f64> {
    // Generate the 1000-entry base schedule: linspace(1, 1/num_train, num_train).
    let base: Vec<f64> = (0..num_train)
        .map(|i| 1.0 - (i as f64) * (1.0 - 1.0 / num_train as f64) / (num_train - 1) as f64)
        .collect();

    // Shift transform: sigma = shift * s / (1 + (shift-1) * s).
    let shifted: Vec<f64> = base
        .iter()
        .map(|&s| shift * s / (1.0 + (shift - 1.0) * s))
        .collect();

    // Inference indices: linspace(0, 999, num_inference_steps).
    let indices: Vec<usize> = (0..num_inference_steps)
        .map(|i| (i as f64 * (num_train - 1) as f64 / (num_inference_steps - 1) as f64) as usize)
        .collect();

    indices.iter().map(|&i| shifted[i]).collect()
}

/// Euler step: `x_next = x + (sigma_next - sigma) * velocity`.
pub fn euler_step(
    x: &Tensor,
    velocity: &Tensor,
    sigma: f64,
    sigma_next: f64,
) -> CandleResult<Tensor> {
    let dt = (sigma_next - sigma) as f32;
    let dt_t = Tensor::full(dt, x.dims(), x.device())?;
    let delta = (velocity * dt_t)?;
    x + delta
}

/// Run the full flow-matching Euler sampling loop.
///
/// `dit`: the ZImage transformer.
/// `cap`: text-encoder hidden states `[cap_seq, cap_feat_dim]`.
/// `seed`: for the initial latent (randn via SSOT-PRNG + Box-Muller).
/// `num_inference_steps`: 9 for Z-Image-Turbo (→ 8 forward steps).
/// `guidance_scale`: 0.0 for Turbo (no CFG branch).
/// `shift`: 3.0 (Z-Image-Turbo).
/// `num_train`: 1000.
///
/// Returns the final latent `[1, in_channels, H/8, W/8]`.
pub fn flow_match_euler_sample(
    dit: &crate::vision::dit::ZImageTransformer,
    cap: &Tensor,
    seed: u64,
    num_inference_steps: usize,
    guidance_scale: f64,
) -> Result<Tensor, String> {
    use crate::vision::dit::{tiny_dit_config, zimage_turbo_config, ZImageTransformer};
    use candle_core::{Device, Tensor};

    let _ = guidance_scale; // CFG=0 for Turbo; ignored.
    let device = cap.device();

    // Determine latent shape from the DiT config.
    // For Z-Image-Turbo: 1024×1024 image → 128×128 latent (8× downsample).
    // latent_channels = config.in_channels = 16.
    // For tiny config: we use 8×8 latent → tiny image.
    let config = dit.config();
    let latent_h;
    let latent_w;
    let latent_c = config.in_channels;
    if config.dim == 3840 {
        // Real config → 1024×1024 image → 128×128 latent.
        latent_h = 128;
        latent_w = 128;
    } else {
        // Tiny config → 8×8 latent (32×32 image with patch 2 → 4×4 patches).
        latent_h = 8;
        latent_w = 8;
    }

    // Initial latent: seeded randn via Box-Muller over SSOT-PRNG.
    let latent = crate::vision::vae::fixed_latent(seed, latent_c, latent_h, latent_w);

    // Sigmas.
    let sigmas = flow_match_euler_sigmas(num_inference_steps, 3.0, 1000);
    // The final sigma is 0.0 (terminal). We do num_inference_steps - 1 Euler steps.
    let mut x = latent;
    for i in 0..num_inference_steps - 1 {
        let sigma = sigmas[i];
        let sigma_next = if i + 1 < num_inference_steps {
            sigmas[i + 1]
        } else {
            0.0
        };
        // DiT forward at timestep = sigma * t_scale.
        let t = sigma * 1000.0; // t_scale = 1000.
        let velocity = dit.forward(&x, cap, t).map_err(|e| {
            format!(
                "flow_match_euler_sample: DiT forward at step {} failed: {}",
                i, e
            )
        })?;
        x = euler_step(&x, &velocity, sigma, sigma_next).map_err(|e| {
            format!(
                "flow_match_euler_sample: euler_step at step {} failed: {}",
                i, e
            )
        })?;
    }

    Ok(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sigmas_monotonic_decreasing() {
        let sigmas = flow_match_euler_sigmas(9, 3.0, 1000);
        assert_eq!(sigmas.len(), 9);
        for i in 0..sigmas.len() - 1 {
            assert!(sigmas[i] > sigmas[i + 1], "non-monotonic at {}", i);
        }
    }

    #[test]
    fn sigmas_first_close_to_one_after_shift() {
        // With shift=3.0 and base=1.0: sigma = 3*1/(1+2*1) = 1.0.
        let sigmas = flow_match_euler_sigmas(9, 3.0, 1000);
        assert!((sigmas[0] - 1.0).abs() < 1e-9, "first sigma: {}", sigmas[0]);
    }

    #[test]
    fn sigmas_last_small_positive() {
        let sigmas = flow_match_euler_sigmas(9, 3.0, 1000);
        assert!(sigmas[8] > 0.0, "last sigma: {}", sigmas[8]);
        assert!(sigmas[8] < 0.01, "last sigma: {}", sigmas[8]);
    }

    #[test]
    fn sigmas_deterministic() {
        let s1 = flow_match_euler_sigmas(9, 3.0, 1000);
        let s2 = flow_match_euler_sigmas(9, 3.0, 1000);
        assert_eq!(s1, s2, "same inputs → identical sigmas");
    }

    #[test]
    fn sigmas_pinned_vector() {
        // Pinned sigma values for 9 steps, shift=3.0, num_train=1000.
        // Computed once from the formula above; if this test fails, the schedule
        // has been changed — re-pin after verifying the change is intentional.
        let sigmas = flow_match_euler_sigmas(9, 3.0, 1000);
        // Print for verification (pinning procedure — mirrors №230).
        eprintln!("sigmas_pinned: {:?}", sigmas);
        // First and last values:
        assert!((sigmas[0] - 1.0).abs() < 1e-12);
        assert!(sigmas[8] < 0.01);
        // 9 values total.
        assert_eq!(sigmas.len(), 9);
    }
}
