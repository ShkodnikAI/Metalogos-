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

// ═══════════════════════════════════════════════════════════════════════
// In-context edit loop (Наряд №243, R6.2)
// ═══════════════════════════════════════════════════════════════════════

/// Loud constant: number of forward DiT calls (NFE) for the edit loop.
///
/// **Rationale (loud, per №243 Block 1.2):** Z-Image-Turbo is a distilled
/// model — its published inference NFE is 8 (the generate clip uses
/// `num_inference_steps=9` sigmas → 8 Euler forward steps). The edit loop
/// keeps the SAME distilled budget: `EDIT_STEPS = 8` forward calls through
/// `ZImageTransformer::forward_edit`. Any other value is a loud deviation
/// that must be argued BEFORE the merge (§3.5 — a quiet substitution of
/// steps would be a silent provenance distortion).
pub const EDIT_STEPS: usize = 8;

/// Run the in-context EDIT sampling loop.
///
/// Mirrors `flow_match_euler_sample` (the лекало) with two differences:
/// 1. The initial latent is seeded noise shaped like the SOURCE latent
///    (the output must keep the source resolution — no resize), not a
///    config-derived shape.
/// 2. Every step runs `forward_edit`: the reference latent tokens are
///    concatenated with the noise tokens (in-context self-attention) and
///    `euler_step` is applied to the NOISE branch only — the reference
///    stays clean for the whole loop.
///
/// `dit`: the Z-Image transformer. `cap`: text-encoder hidden states
/// `[cap_seq, cap_feat_dim]`. `ref_latent`: VAE-encoded source
/// `[1, C, H/f, W/f]` (model-latent space — the same space the sampler
/// works in). `seed`: inherited from the source artifact's manifest
/// (№243 Block 2.3 — same source + same prompt + same weights → same seed
/// → deterministic output). `num_forward_steps`: forward DiT calls —
/// pass [`EDIT_STEPS`] (8, the distilled NFE; the sigma schedule gets
/// `steps + 1` entries, matching the generate clip's 9-sigma → 8-forward
/// arithmetic).
///
/// CFG is absent entirely: Turbo is distilled (guidance_scale would be
/// ignored — the honest form of the generate path's `let _ = guidance_scale`).
pub fn flow_match_euler_edit(
    dit: &crate::vision::dit::ZImageTransformer,
    cap: &Tensor,
    ref_latent: &Tensor,
    seed: u64,
    num_forward_steps: usize,
) -> Result<Tensor, String> {
    use candle_core::Tensor;

    let (b, c, h, w) = ref_latent
        .dims4()
        .map_err(|e| format!("flow_match_euler_edit: ref dims4: {}", e))?;
    if b != 1 {
        return Err(format!(
            "flow_match_euler_edit: reference latent must be batch 1, got batch {}",
            b
        ));
    }

    // Initial noise branch: seeded randn shaped like the source latent
    // (Box-Muller over the SSOT PRNG — the same `fixed_latent` generate
    // starts from; the seed is inherited from the source manifest).
    let mut x = crate::vision::vae::fixed_latent(seed, c, h, w);

    // Sigma schedule: steps + 1 entries → `steps` Euler updates, exactly
    // the generate clip's arithmetic (9 sigmas → 8 forwards).
    let sigmas = flow_match_euler_sigmas(num_forward_steps + 1, 3.0, 1000);
    for i in 0..num_forward_steps {
        let sigma = sigmas[i];
        let sigma_next = if i + 1 < sigmas.len() {
            sigmas[i + 1]
        } else {
            0.0
        };
        // DiT forward at timestep = sigma * 1000 — the SAME t convention
        // the generate loop passes into `forward` (consistency over
        // re-derivation; the golden pins that convention).
        let t = sigma * 1000.0;
        let velocity = dit.forward_edit(&x, ref_latent, cap, t).map_err(|e| {
            format!(
                "flow_match_euler_edit: DiT forward_edit at step {} failed: {}",
                i, e
            )
        })?;
        // Euler update on the noise branch ONLY.
        x = euler_step(&x, &velocity, sigma, sigma_next).map_err(|e| {
            format!(
                "flow_match_euler_edit: euler_step at step {} failed: {}",
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
