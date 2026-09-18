#![cfg(feature = "video")]
// ── Video interpolation & extension (Наряд №309, ADR-0151 D2-D3) ─────
//
// RIFE-class frame interpolation: linear latent blending between
// consecutive latent frames (2x, 4x) with exact endpoint preservation.
// Extension: anchored continuation sampling — new latent frames are
// generated with the first anchor pinned to the source's last frame.
// Both are real deterministic algorithms on the №310 tiny machinery.
#![allow(clippy::all)]
#![allow(clippy::expect_used)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use candle_core::Tensor;

use crate::video::denoiser::{VideoDit, VideoDitConfig};
use crate::video::i2v::{hash_embedding, serialize_video, sha256_hex};
use crate::video::sampler::{flow_match_euler_sample_video_anchored, VideoSampleConfig};
use crate::video::vae::{VideoVae, VideoVaeConfig};
use crate::video::{LatentData, VideoArtifact, VideoKind, VideoManifest};

/// Decode a latent snapshot through a same-seed tiny VAE and serialize.
fn decode_and_serialize(latent: &LatentData, seed: u64) -> Result<Vec<u8>, String> {
    let vae = VideoVae::new_tiny(seed, VideoVaeConfig::default(), &candle_core::Device::Cpu)
        .map_err(|e| format!("frame_interp: VAE init failed: {}", e))?;
    let tensor = latent
        .to_tensor()
        .map_err(|e| format!("frame_interp: latent tensor failed: {}", e))?;
    let video = vae
        .decode(&tensor)
        .map_err(|e| format!("frame_interp: VAE decode failed: {}", e))?;
    serialize_video(&video)
}

/// Source manifest accessor shared by interp/extend (both require a signed
/// source: provenance chain by construction).
fn source_manifest<'a>(
    src: &'a VideoArtifact,
    who: &str,
) -> Result<(&'a VideoManifest, &'a LatentData), String> {
    let m = src.manifest.as_ref().ok_or_else(|| {
        format!(
            "{}(): source artifact has no provenance manifest — unsigned sources are refused",
            who
        )
    })?;
    if m.kind == VideoKind::AvMux {
        return Err(format!(
            "{}(): muxed (A/V container) artifacts cannot be re-processed — only \
             render/interp/extend outputs",
            who
        ));
    }
    let latent = src
        .latent
        .as_ref()
        .ok_or_else(|| format!("{}(): source artifact has no latent data", who))?;
    Ok((m, latent))
}

/// Derive an interp/extend manifest from the source (provenance chain:
/// kind changes, source_sha records the parent, ref/prompt/seed inherit).
fn derived_manifest(
    src: &VideoManifest,
    kind: VideoKind,
    video_sha: String,
    timestamp: u64,
) -> VideoManifest {
    VideoManifest {
        model_id: src.model_id.clone(),
        weights_sha: src.weights_sha.clone(),
        kind,
        ref_hash: src.ref_hash.clone(),
        ref_last_hash: src.ref_last_hash.clone(),
        consent_hash: src.consent_hash.clone(),
        prompt_hash: src.prompt_hash.clone(),
        seed: src.seed,
        timestamp,
        policy: src.policy.clone(),
        video_sha,
        audio_ref: None,
        fps: src.fps,
        source_sha: Some(src.video_sha.clone()),
    }
}

/// RIFE-class frame interpolation (ADR-0151 D2): between every consecutive
/// latent frame pair, (factor-1) blends `lerp(l_t, l_{t+1}, k/factor)` are
/// inserted. `factor ∈ {2, 4}`. Endpoints are preserved EXACTLY:
/// out[0] = in[0], out[T'] = in[T-1] (the RIFE-class contract).
pub fn frame_interp_artifact(
    src: &VideoArtifact,
    factor: usize,
    timestamp: u64,
) -> Result<VideoArtifact, String> {
    const WHO: &str = "frame_interp";
    let (manifest, latent) = source_manifest(src, WHO)?;
    match factor {
        2 | 4 => {}
        other => return Err(format!("{}(): factor must be 2 or 4, got {}", WHO, other)),
    }
    let (b, c, t, h, w) = (
        latent.dims[0],
        latent.dims[1],
        latent.dims[2],
        latent.dims[3],
        latent.dims[4],
    );
    if t < 2 {
        return Err(format!("{}(): source latent must have T >= 2 frames", WHO));
    }
    let per_frame = c * h * w;
    let t_out = (t - 1) * factor + 1;
    let mut vals = Vec::with_capacity(b * c * t_out * h * w);

    let lerp_frame = |a: &[f32], bframe: &[f32], alpha: f32, out: &mut Vec<f32>| {
        for i in 0..per_frame {
            out.push(a[i] * (1.0 - alpha) + bframe[i] * alpha);
        }
    };

    for ti in 0..t {
        let cur = latent.frame(ti).unwrap_or_default();
        if cur.is_empty() {
            return Err(format!("{}(): source latent frame {} is empty", WHO, ti));
        }
        vals.extend_from_slice(cur);
        if ti + 1 < t {
            let next = latent.frame(ti + 1).unwrap_or_default();
            if next.is_empty() {
                return Err(format!(
                    "{}(): source latent frame {} is empty",
                    WHO,
                    ti + 1
                ));
            }
            for k in 1..factor {
                let alpha = k as f32 / factor as f32;
                lerp_frame(cur, next, alpha, &mut vals);
            }
        }
    }

    let out_latent = LatentData {
        dims: [b, c, t_out, h, w],
        vals,
    };
    let video_bytes = decode_and_serialize(&out_latent, manifest.seed)?;
    let out_manifest = derived_manifest(
        manifest,
        VideoKind::Interp,
        sha256_hex(&video_bytes),
        timestamp,
    );

    Ok(VideoArtifact {
        video_bytes,
        manifest: Some(out_manifest),
        latent: Some(out_latent),
    })
}

/// Clip extension (ADR-0151 D3): sample `extra` NEW latent frames with the
/// first anchor pinned to the source's LAST latent frame, seed+1. The
/// anchor frame itself is the duplicate boundary and is dropped, so the
/// total latent temporal size becomes T + extra (all appended frames are
/// new). Decoded with the same-seed VAE.
pub fn extend_video_artifact(
    src: &VideoArtifact,
    extra: usize,
    timestamp: u64,
) -> Result<VideoArtifact, String> {
    const WHO: &str = "video_extend";
    let (manifest, latent) = source_manifest(src, WHO)?;
    if !(1..=8).contains(&extra) {
        return Err(format!(
            "{}(): extra must be in 1..=8 additional latent frames, got {}",
            WHO, extra
        ));
    }
    let (b, c, t, h, w) = (
        latent.dims[0],
        latent.dims[1],
        latent.dims[2],
        latent.dims[3],
        latent.dims[4],
    );
    let seed = manifest.seed.wrapping_add(1);
    let device = candle_core::Device::Cpu;
    let dit = VideoDit::new_tiny(seed, VideoDitConfig::default(), &device)
        .map_err(|e| format!("{}(): DiT init failed: {}", WHO, e))?;
    let text = hash_embedding(seed);

    // Continuation anchor: the source's LAST latent frame passed directly
    // as the anchor — it already lives in latent space, and the continuation
    // sampler re-pins it after every Euler step (ADR-0151 D3).
    let last_frame = latent.frame_vec(t - 1).unwrap_or_default();
    if last_frame.is_empty() {
        return Err(format!("{}(): source latent last frame is empty", WHO));
    }
    let anchor = Tensor::from_vec(last_frame, (b, c, h, w), &device)
        .map_err(|e| format!("{}(): anchor tensor failed: {}", WHO, e))?;
    let config = VideoSampleConfig {
        seed,
        ..VideoSampleConfig::default()
    };
    // Sample extra+1 latent frames; frame 0 is the pinned anchor (duplicate
    // of the source's last frame) and is dropped — appended frames are new.
    let cont = flow_match_euler_sample_video_anchored(
        &dit,
        &text,
        &config,
        (b, c, extra + 1, h, w),
        Some(&anchor),
        None,
    )?;
    let cont_vals = cont
        .narrow(2, 1, extra)
        // Frame-major layout contract: [B, C, T', H, W] → [B, T', C, H, W].
        .and_then(|t| t.permute((0, 2, 1, 3, 4)))
        .and_then(|t| t.flatten_all().and_then(|f| f.to_vec1::<f32>()))
        .map_err(|e| format!("{}(): continuation sampling failed: {}", WHO, e))?;

    let t_out = t + extra;
    let mut vals = latent.vals.clone();
    vals.extend_from_slice(&cont_vals);
    let out_latent = LatentData {
        dims: [b, c, t_out, h, w],
        vals,
    };
    let video_bytes = decode_and_serialize(&out_latent, manifest.seed)?;
    let out_manifest = derived_manifest(
        manifest,
        VideoKind::Extend,
        sha256_hex(&video_bytes),
        timestamp,
    );

    Ok(VideoArtifact {
        video_bytes,
        manifest: Some(out_manifest),
        latent: Some(out_latent),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::video::i2v::render;

    fn i2v_source() -> VideoArtifact {
        let r = crate::nn::attention::generate_uniform_f32(33, 3 * 32 * 32, -1.0, 1.0);
        render("wan-2.2-ti2v-5b", "interp scene", Some(&r), None, 0).unwrap()
    }

    #[test]
    fn interp_2x_frame_count_and_endpoints() {
        let src = i2v_source();
        let out = frame_interp_artifact(&src, 2, 0).unwrap();
        let li = src.latent.as_ref().unwrap();
        let lo = out.latent.as_ref().unwrap();
        assert_eq!(lo.dims[2], (li.dims[2] - 1) * 2 + 1, "2x: T' = (T-1)*2+1");
        // RIFE-class endpoint contract: first/last frames never move.
        assert_eq!(lo.frame_vec(0).unwrap(), li.frame_vec(0).unwrap());
        let lt = li.dims[2];
        assert_eq!(
            lo.frame_vec(lo.dims[2] - 1).unwrap(),
            li.frame_vec(lt - 1).unwrap()
        );
        // Manifest: interp kind + provenance chain to the source.
        let m = out.manifest.as_ref().unwrap();
        assert_eq!(m.kind, VideoKind::Interp);
        assert_eq!(
            m.source_sha.as_deref(),
            Some(src.manifest.as_ref().unwrap().video_sha.as_str())
        );
        assert_eq!(m.ref_hash, src.manifest.as_ref().unwrap().ref_hash);
    }

    #[test]
    fn interp_4x_frame_count() {
        let src = i2v_source();
        let out = frame_interp_artifact(&src, 4, 0).unwrap();
        assert_eq!(out.latent.as_ref().unwrap().dims[2], (2 - 1) * 4 + 1);
    }

    #[test]
    fn interp_midpoint_blend_is_exact_average() {
        // 2x midpoint must be the exact f32 mean of the neighbours.
        let src = i2v_source();
        let li = src.latent.as_ref().unwrap();
        let lo = frame_interp_artifact(&src, 2, 0).unwrap().latent.unwrap();
        let a = li.frame_vec(0).unwrap();
        let b = li.frame_vec(1).unwrap();
        let mid = lo.frame_vec(1).unwrap();
        let per_frame = a.len();
        for i in 0..per_frame {
            let want = a[i] * 0.5 + b[i] * 0.5;
            assert_eq!(mid[i], want, "midpoint blend at {}", i);
        }
    }

    #[test]
    fn interp_deterministic_and_real_bytes() {
        let src = i2v_source();
        let o1 = frame_interp_artifact(&src, 2, 0).unwrap();
        let o2 = frame_interp_artifact(&src, 2, 0).unwrap();
        assert_eq!(o1.video_bytes, o2.video_bytes);
        assert!(o1.video_bytes.iter().any(|&b| b != 0));
    }

    #[test]
    fn interp_rejects_bad_factor_and_unsigned() {
        let src = i2v_source();
        assert!(frame_interp_artifact(&src, 3, 0).is_err());
        assert!(frame_interp_artifact(&src, 0, 0).is_err());
        let mut unsigned = src.clone();
        unsigned.manifest = None;
        assert!(frame_interp_artifact(&unsigned, 2, 0).is_err());
    }

    #[test]
    fn extend_appends_new_frames_anchored_on_source_last() {
        let src = i2v_source();
        let li = src.latent.as_ref().unwrap();
        let out = extend_video_artifact(&src, 3, 0).unwrap();
        let lo = out.latent.as_ref().unwrap();
        assert_eq!(
            lo.dims[2],
            li.dims[2] + 3,
            "extend appends exactly `extra` frames"
        );
        // Source frames are copied unchanged at the head.
        assert_eq!(lo.frame_vec(0).unwrap(), li.frame_vec(0).unwrap());
        assert_eq!(lo.frame_vec(1).unwrap(), li.frame_vec(1).unwrap());
        // Appended frames are REAL continuation: none of them equals the
        // source's last frame (the anchor is pinned at cont frame 0, which
        // is dropped; frames 1.. are fresh generation conditioned on it).
        let lt = li.dims[2];
        let src_last = li.frame_vec(lt - 1).unwrap();
        for k in 0..3 {
            let appended = lo.frame_vec(lt + k).unwrap();
            assert_ne!(appended, src_last, "appended frame {} must be new", k);
            assert!(appended.iter().any(|&v| v != 0.0));
        }
        // Manifest chain.
        let m = out.manifest.as_ref().unwrap();
        assert_eq!(m.kind, VideoKind::Extend);
        assert_eq!(
            m.source_sha.as_deref(),
            Some(src.manifest.as_ref().unwrap().video_sha.as_str())
        );
        assert!(
            m.seed == src.manifest.as_ref().unwrap().seed,
            "derived manifest inherits the source seed (same-seed VAE decode + provenance); \
             the continuation sampler uses seed+1 internally"
        );
        // Real bytes.
        assert!(out.video_bytes.iter().any(|&b| b != 0));
    }

    #[test]
    fn extend_rejects_bad_counts_and_unsigned() {
        let src = i2v_source();
        assert!(extend_video_artifact(&src, 0, 0).is_err());
        assert!(extend_video_artifact(&src, 9, 0).is_err());
        let mut unsigned = src.clone();
        unsigned.manifest = None;
        assert!(extend_video_artifact(&unsigned, 2, 0).is_err());
    }

    #[test]
    fn interp_then_extend_chain_provenance() {
        let src = i2v_source();
        let interp = frame_interp_artifact(&src, 2, 0).unwrap();
        let ext = extend_video_artifact(&interp, 1, 0).unwrap();
        assert_eq!(
            ext.manifest.as_ref().unwrap().source_sha.as_deref(),
            Some(interp.manifest.as_ref().unwrap().video_sha.as_str()),
            "extend must chain from the interp artifact"
        );
        assert_eq!(
            ext.manifest.as_ref().unwrap().ref_hash,
            src.manifest.as_ref().unwrap().ref_hash,
            "ref_hash provenance survives the whole chain"
        );
    }
}
