#![cfg(feature = "video")]
// ── Video I2V: first/last-frame anchored rendering (Наряд №309, ADR-0151 D1) ──
//
// I2V = T2V with pinned latent anchors: the flow-matching Euler loop runs
// on the №310 machinery and the anchor latent frames (encoded from the
// reference pixels by the №310 VideoVae) are re-pinned after every step.
// Deterministic by construction: seed = sha256(model_id | prompt).
#![allow(clippy::all)]
#![allow(clippy::expect_used)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use candle_core::Tensor;
use sha2::{Digest, Sha256};

use crate::video::denoiser::{VideoDit, VideoDitConfig};
use crate::video::sampler::{flow_match_euler_sample_video_anchored, VideoSampleConfig};
use crate::video::vae::{VideoVae, VideoVaeConfig};
use crate::video::{LatentData, VideoArtifact, VideoKind, VideoManifest, KNOWN_VIDEO_MODELS};

/// Reference frame pixel count: 3 channels x 32 x 32 (tiny pipeline frame).
pub const REF_PIXELS_LEN: usize = 3 * 32 * 32;

/// Render frame rate of the tiny pipeline (fps) — mux timestamps are
/// frame-aligned to it (ADR-0151 D4).
pub const TINY_FPS: u32 = 8;

/// Latent frames per render (№310 tiny shape [1, 4, 2, 4, 4]).
const LATENT_T: usize = 2;
const LATENT_H: usize = 4; // 32 / LATENT_SPATIAL_COMPRESSION
const LATENT_W: usize = 4;

/// Deterministic seed derivation: sha256(model_id | prompt), first 8 bytes
/// LE (ADR-0151 D1). Same model+prompt ⇒ same video; different prompt ⇒
/// different latent init.
pub fn prompt_seed(model_id: &str, prompt: &str) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(model_id.as_bytes());
    hasher.update(b"|");
    hasher.update(prompt.as_bytes());
    let digest = hasher.finalize();
    let mut seed: u64 = 0;
    for (i, b) in digest.iter().take(8).enumerate() {
        seed |= (*b as u64) << (8 * i);
    }
    seed
}

/// SHA-256 hex of arbitrary bytes (prompt_hash / video_sha / audio side).
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// SHA-256 hex of the reference frame pixels (f32 LE byte stream) —
/// the `ref_hash` / `ref_last_hash` provenance fields (№309 A.1).
pub fn ref_hash(ref_pixels: &[f32]) -> String {
    let mut bytes = Vec::with_capacity(ref_pixels.len() * 4);
    for v in ref_pixels {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    sha256_hex(&bytes)
}

/// Text conditioning for the tiny pipeline: deterministic hash embedding
/// [1, 64] derived from the render seed (ADR-0151 D7 — prompt conditioning
/// operates through seed derivation; the DiT text path is a V4+ item).
pub(crate) fn hash_embedding(seed: u64) -> Tensor {
    let vals = crate::nn::attention::generate_uniform_f32(seed.wrapping_add(7), 64, -1.0, 1.0);
    Tensor::from_vec(vals, (1, 64), &candle_core::Device::Cpu).expect("hash_embedding")
}

fn validate_model(model_id: &str) -> Result<(), String> {
    if KNOWN_VIDEO_MODELS.contains(&model_id) {
        Ok(())
    } else {
        Err(format!(
            "video_render(): unknown model id '{}' (KNOWN_VIDEO_MODELS: {:?})",
            model_id, KNOWN_VIDEO_MODELS
        ))
    }
}

fn validate_prompt(prompt: &str) -> Result<(), String> {
    if prompt.trim().is_empty() {
        Err("video_render(): prompt must be non-empty".to_string())
    } else {
        Ok(())
    }
}

fn validate_ref(ref_pixels: Option<&[f32]>, name: &str) -> Result<(), String> {
    match ref_pixels {
        None => Ok(()),
        Some(r) => {
            if r.len() != REF_PIXELS_LEN {
                return Err(format!(
                    "video_render(): {} must contain exactly {} RGB f32 pixels (3x32x32), got {}",
                    name,
                    REF_PIXELS_LEN,
                    r.len()
                ));
            }
            if r.iter().any(|v| !v.is_finite()) {
                return Err(format!(
                    "video_render(): {} contains a non-finite pixel value",
                    name
                ));
            }
            Ok(())
        }
    }
}

/// Serialize decoded frames [1, C, T, H, W] as raw little-endian f32
/// (`MLGV-RAW-F32` payload class, ADR-0151 D7 — deterministic, hashable).
pub fn serialize_video(video: &Tensor) -> Result<Vec<u8>, String> {
    let vals = video
        .flatten_all()
        .and_then(|t| t.to_vec1::<f32>())
        .map_err(|e| format!("video render: frame serialization failed: {}", e))?;
    let mut bytes = Vec::with_capacity(vals.len() * 4);
    for v in vals {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    Ok(bytes)
}

/// Encode a reference frame to a latent anchor [1, lc, H/8, W/8] via the
/// №310 VideoVae encoder.
fn encode_anchor(vae: &VideoVae, ref_pixels: &[f32]) -> Result<Tensor, String> {
    let frame = Tensor::from_vec(
        ref_pixels.to_vec(),
        (1, 3, 32, 32),
        &candle_core::Device::Cpu,
    )
    .map_err(|e| format!("video render: reference frame tensor failed: {}", e))?;
    vae.encode_frame(&frame)
        .map_err(|e| format!("video render: reference encode failed: {}", e))
}

/// One pipeline render (T2V / I2V / two-anchor — chosen by the anchor
/// arguments, ADR-0151 D1). Returns a fully-provenanced artifact.
///
/// * `ref_first` — I2V first-frame anchor (None ⇒ T2V).
/// * `ref_last`  — two-anchor first–last contract (requires `ref_first`).
pub fn render(
    model_id: &str,
    prompt: &str,
    ref_first: Option<&[f32]>,
    ref_last: Option<&[f32]>,
    timestamp: u64,
) -> Result<VideoArtifact, String> {
    validate_model(model_id)?;
    validate_prompt(prompt)?;
    validate_ref(ref_first, "ref_first")?;
    validate_ref(ref_last, "ref_last")?;
    if ref_last.is_some() && ref_first.is_none() {
        return Err(
            "video_render(): ref_last requires ref_first — the two-anchor contract is \
             first–last, never last-only"
                .to_string(),
        );
    }

    let seed = prompt_seed(model_id, prompt);
    let device = candle_core::Device::Cpu;
    let dit = VideoDit::new_tiny(seed, VideoDitConfig::default(), &device)
        .map_err(|e| format!("video render: DiT init failed: {}", e))?;
    let vae = VideoVae::new_tiny(seed, VideoVaeConfig::default(), &device)
        .map_err(|e| format!("video render: VAE init failed: {}", e))?;
    let text = hash_embedding(seed);
    let config = VideoSampleConfig {
        seed,
        ..VideoSampleConfig::default()
    };

    let first_anchor = match ref_first {
        Some(r) => Some(encode_anchor(&vae, r)?),
        None => None,
    };
    let last_anchor = match ref_last {
        Some(r) => Some(encode_anchor(&vae, r)?),
        None => None,
    };

    let latent = flow_match_euler_sample_video_anchored(
        &dit,
        &text,
        &config,
        (1, 4, LATENT_T, LATENT_H, LATENT_W),
        first_anchor.as_ref(),
        last_anchor.as_ref(),
    )?;
    let video = vae
        .decode(&latent)
        .map_err(|e| format!("video render: VAE decode failed: {}", e))?;
    let video_bytes = serialize_video(&video)?;
    // Frame-major layout contract: [B, C, T, H, W] → [B, T, C, H, W] flatten.
    let latent_data = LatentData::from_tensor(&latent)?;

    let manifest = VideoManifest {
        model_id: model_id.to_string(),
        // Honest provenance: no external weights — the "weights" are the
        // seeded tiny init itself (№310 template, ADR-0151 D7).
        weights_sha: format!("seeded-tiny:{:016x}", seed),
        kind: if ref_first.is_some() {
            VideoKind::I2V
        } else {
            VideoKind::T2V
        },
        ref_hash: ref_first.map(ref_hash),
        ref_last_hash: ref_last.map(ref_hash),
        consent_hash: None,
        prompt_hash: sha256_hex(prompt.as_bytes()),
        seed,
        timestamp,
        policy: "default".to_string(),
        video_sha: sha256_hex(&video_bytes),
        audio_ref: None,
        fps: TINY_FPS,
        source_sha: None,
    };

    Ok(VideoArtifact {
        video_bytes,
        manifest: Some(manifest),
        latent: Some(latent_data),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_ref(seed: u64) -> Vec<f32> {
        crate::nn::attention::generate_uniform_f32(seed, REF_PIXELS_LEN, -1.0, 1.0)
    }

    #[test]
    fn prompt_seed_is_deterministic_and_sensitive() {
        let a = prompt_seed("wan-2.2-ti2v-5b", "scene one");
        let b = prompt_seed("wan-2.2-ti2v-5b", "scene one");
        let c = prompt_seed("wan-2.2-ti2v-5b", "scene two");
        let d = prompt_seed("cogvideox-1.5-5b", "scene one");
        assert_eq!(a, b, "same model+prompt must give the same seed");
        assert_ne!(a, c, "different prompt must change the seed");
        assert_ne!(a, d, "different model must change the seed");
    }

    #[test]
    fn ref_hash_is_deterministic_byte_level() {
        let r = fake_ref(11);
        assert_eq!(ref_hash(&r), ref_hash(&r));
        let mut r2 = r.clone();
        r2[0] += 0.001;
        assert_ne!(ref_hash(&r), ref_hash(&r2), "pixel change must change hash");
    }

    #[test]
    fn render_t2v_shape_and_manifest() {
        let artifact = render("wan-2.2-ti2v-5b", "silent scene", None, None, 42).unwrap();
        let m = artifact.manifest.as_ref().unwrap();
        assert_eq!(m.kind, VideoKind::T2V);
        assert!(m.ref_hash.is_none());
        assert!(m.ref_last_hash.is_none());
        assert_eq!(m.seed, prompt_seed("wan-2.2-ti2v-5b", "silent scene"));
        assert_eq!(m.fps, TINY_FPS);
        assert_eq!(m.timestamp, 42);
        assert_eq!(
            m.video_sha,
            sha256_hex(&artifact.video_bytes),
            "video_sha must hash the real payload"
        );
        let latent = artifact.latent.as_ref().unwrap();
        assert_eq!(latent.dims, [1, 4, LATENT_T, LATENT_H, LATENT_W]);
        // Real computation: decoded 4 frames x 3 x 32 x 32, non-trivial bytes.
        assert_eq!(artifact.video_bytes.len(), 4 * 3 * 32 * 32 * 4);
        assert!(artifact.video_bytes.iter().any(|&b| b != 0));
    }

    #[test]
    fn render_i2v_records_ref_hash() {
        let r = fake_ref(7);
        let artifact = render("wan-2.2-ti2v-5b", "scene", Some(&r), None, 0).unwrap();
        let m = artifact.manifest.as_ref().unwrap();
        assert_eq!(m.kind, VideoKind::I2V);
        assert_eq!(m.ref_hash.as_deref(), Some(ref_hash(&r).as_str()));
    }

    #[test]
    fn anchors_are_pinned_exactly_in_final_latent() {
        // The two-anchor contract (№309 A.1): after the last Euler step the
        // anchors are re-pinned, so latent frames 0 and T-1 must equal the
        // encoded references EXACTLY (not approximately).
        let r_first = fake_ref(101);
        let r_last = fake_ref(202);
        let artifact = render(
            "wan-2.2-ti2v-5b",
            "two anchors",
            Some(&r_first),
            Some(&r_last),
            0,
        )
        .unwrap();
        let m = artifact.manifest.as_ref().unwrap();
        assert_eq!(m.kind, VideoKind::I2V);
        assert_eq!(m.ref_hash.as_deref(), Some(ref_hash(&r_first).as_str()));
        assert_eq!(m.ref_last_hash.as_deref(), Some(ref_hash(&r_last).as_str()));

        let vae = VideoVae::new_tiny(m.seed, VideoVaeConfig::default(), &candle_core::Device::Cpu)
            .unwrap();
        let want_first = vae.encode_frame(
            &Tensor::from_vec(r_first, (1, 3, 32, 32), &candle_core::Device::Cpu).unwrap(),
        );
        let want_last = vae.encode_frame(
            &Tensor::from_vec(r_last, (1, 3, 32, 32), &candle_core::Device::Cpu).unwrap(),
        );
        let latent = artifact.latent.as_ref().unwrap();
        let got_first = latent.frame_vec(0).unwrap();
        let got_last = latent.frame_vec(latent.t() - 1).unwrap();
        let want_first = want_first
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let want_last = want_last
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        assert_eq!(got_first, want_first, "first anchor must be pinned exactly");
        assert_eq!(got_last, want_last, "last anchor must be pinned exactly");
        // Middle frames are NOT the anchors (real generation happened between).
        if latent.t() > 2 {
            let mid = latent.frame_vec(1).unwrap();
            assert_ne!(mid, got_first);
        }
    }

    #[test]
    fn render_is_seed_deterministic() {
        let r = fake_ref(5);
        let a1 = render("cogvideox-1.5-5b", "determinism", Some(&r), None, 0).unwrap();
        let a2 = render("cogvideox-1.5-5b", "determinism", Some(&r), None, 0).unwrap();
        assert_eq!(a1.video_bytes, a2.video_bytes);
        assert_eq!(
            a1.latent.as_ref().unwrap().vals,
            a2.latent.as_ref().unwrap().vals
        );
        assert_eq!(
            a1.manifest.as_ref().unwrap().video_sha,
            a2.manifest.as_ref().unwrap().video_sha
        );
    }

    #[test]
    fn i2v_differs_from_t2v_same_prompt() {
        let r = fake_ref(9);
        let t2v = render("wan-2.2-ti2v-5b", "same prompt", None, None, 0).unwrap();
        let i2v = render("wan-2.2-ti2v-5b", "same prompt", Some(&r), None, 0).unwrap();
        assert_ne!(
            t2v.manifest.as_ref().unwrap().video_sha,
            i2v.manifest.as_ref().unwrap().video_sha,
            "the reference anchor must change the generated latent"
        );
    }

    #[test]
    fn render_validates_inputs_loudly() {
        assert!(render("unknown-model", "p", None, None, 0).is_err());
        assert!(render("wan-2.2-ti2v-5b", "  ", None, None, 0).is_err());
        assert!(render("wan-2.2-ti2v-5b", "p", Some(&fake_ref(1)[..10]), None, 0).is_err());
        // last-only is forbidden — the contract is first–last.
        assert!(render("wan-2.2-ti2v-5b", "p", None, Some(&fake_ref(2)), 0).is_err());
        // NaN pixels are rejected loudly.
        let mut nan_ref = fake_ref(3);
        nan_ref[0] = f32::NAN;
        assert!(render("wan-2.2-ti2v-5b", "p", Some(&nan_ref), None, 0).is_err());
    }
}
