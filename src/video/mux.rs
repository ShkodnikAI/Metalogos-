#![cfg(feature = "video")]
// ── Video mux & export (Наряд №309, ADR-0151 D4-D5) ──────────────────
//
// av_mux: deterministic `.mlgv.av` sidecar container pairing VideoId ↔
// AudioId with frame-aligned timestamps (no MP4 muxing crate fits the
// constraints — the sidecar is a real, working artifact: JSON header +
// binary payload, ADR-0151 D4).
//
// video_export: signed-by-construction `.mlgv` container (manifest JSON +
// watermark + payload). Unsigned export does not exist — the runtime gate
// VIDEO_UNSIGNED_EXPORT refuses manifest-less artifacts (ADR-0149 D3,
// ADR-0151 D5; contract test pins it).
#![allow(clippy::all)]
#![allow(clippy::expect_used)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use sha2::Digest;

use crate::video::i2v::sha256_hex;
use crate::video::{VideoArtifact, VideoKind};

/// Magic of the `.mlgv.av` A/V sidecar container.
pub const MLGVAV_MAGIC: &[u8; 7] = b"MLGVAV\x01";
/// Magic of the `.mlgv` export container.
pub const MLGV_MAGIC: &[u8; 5] = b"MLGV\x01";

/// Parsed PCM WAV parameters (only what frame alignment needs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WavInfo {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub data_len: usize,
}

impl WavInfo {
    /// Audio duration in seconds (PCM data length / byte rate).
    pub fn duration_s(&self) -> f64 {
        let bytes_per_sample = (self.bits_per_sample as f64 / 8.0).max(1.0);
        let byte_rate = self.sample_rate as f64 * self.channels as f64 * bytes_per_sample;
        if byte_rate <= 0.0 {
            return 0.0;
        }
        self.data_len as f64 / byte_rate
    }
}

/// Parse a real PCM WAV (RIFF/WAVE) container: walk the chunk list, read
/// `fmt ` and `data` chunks. Loud errors on malformed input — no silent
/// fallbacks. (Compressed codecs, e.g. MP3, are a documented non-goal of
/// this phase — ADR-0151 D4.)
pub fn parse_wav(bytes: &[u8]) -> Result<WavInfo, String> {
    const WHO: &str = "parse_wav";
    if bytes.len() < 12 {
        return Err(format!("{}(): input too short for a RIFF header", WHO));
    }
    if &bytes[0..4] != b"RIFF" {
        return Err(format!("{}(): missing RIFF magic", WHO));
    }
    if &bytes[8..12] != b"WAVE" {
        return Err(format!("{}(): missing WAVE form type", WHO));
    }
    let mut pos = 12usize;
    let mut info = WavInfo {
        sample_rate: 0,
        channels: 0,
        bits_per_sample: 0,
        data_len: 0,
    };
    let mut saw_fmt = false;
    let mut saw_data = false;
    while pos + 8 <= bytes.len() {
        let chunk_id = &bytes[pos..pos + 4];
        let chunk_len = u32::from_le_bytes(
            bytes[pos + 4..pos + 8]
                .try_into()
                .map_err(|_| format!("{}(): chunk header truncated", WHO))?,
        ) as usize;
        let payload = pos + 8;
        let end = payload.saturating_add(chunk_len).min(bytes.len());
        match chunk_id {
            b"fmt " => {
                if end - payload < 16 {
                    return Err(format!("{}(): fmt chunk too short", WHO));
                }
                let p = &bytes[payload..end];
                let audio_format = u16::from_le_bytes([p[0], p[1]]);
                if audio_format != 1 {
                    return Err(format!(
                        "{}(): only PCM (format 1) is supported, got format {}",
                        WHO, audio_format
                    ));
                }
                info.channels = u16::from_le_bytes([p[2], p[3]]);
                info.sample_rate = u32::from_le_bytes([p[4], p[5], p[6], p[7]]);
                info.bits_per_sample = u16::from_le_bytes([p[14], p[15]]);
                saw_fmt = true;
            }
            b"data" => {
                info.data_len = end - payload;
                saw_data = true;
            }
            _ => {}
        }
        // Chunks are word-aligned.
        pos = payload + chunk_len + (chunk_len & 1);
    }
    if !saw_fmt {
        return Err(format!("{}(): missing fmt chunk", WHO));
    }
    if !saw_data {
        return Err(format!("{}(): missing data chunk", WHO));
    }
    if info.sample_rate == 0 || info.channels == 0 || info.bits_per_sample == 0 {
        return Err(format!("{}(): degenerate WAV parameters", WHO));
    }
    Ok(info)
}

/// Frame count of a render/interp/extend artifact: the VAE temporal decode
/// doubles the latent frames (№310 VideoVae::decode repeats each frame 2x).
fn decoded_frame_count(artifact: &VideoArtifact, who: &str) -> Result<usize, String> {
    let latent = artifact
        .latent
        .as_ref()
        .ok_or_else(|| format!("{}(): artifact has no latent data", who))?;
    Ok(latent.t() * 2)
}

/// `av_mux` core (ADR-0151 D4): build the deterministic `.mlgv.av` sidecar
/// pairing the video artifact with an AudioId's real PCM WAV bytes.
/// Timestamps are frame-aligned to the render fps; drift between A/V
/// durations is RECORDED, not hidden. Re-muxing an AvMux artifact is a
/// loud error (no nested containers).
pub fn mux_av(
    video: &VideoArtifact,
    audio_id: u32,
    audio_bytes: &[u8],
    timestamp: u64,
) -> Result<VideoArtifact, String> {
    const WHO: &str = "av_mux";
    let manifest = video.manifest.as_ref().ok_or_else(|| {
        format!(
            "{}(): VIDEO_UNSIGNED_EXPORT-class gate — video artifact has no provenance \
             manifest; muxing unsigned sources is refused (ADR-0151 D4)",
            WHO
        )
    })?;
    if manifest.kind == VideoKind::AvMux {
        return Err(format!(
            "{}(): nested mux is forbidden — the input is already an A/V container",
            WHO
        ));
    }
    let frames = decoded_frame_count(video, WHO)?;
    if frames == 0 {
        return Err(format!("{}(): video artifact has no frames", WHO));
    }
    let wav = parse_wav(audio_bytes).map_err(|e| format!("{}(): audio side: {}", WHO, e))?;

    let fps = manifest.fps.max(1);
    let video_duration_s = frames as f64 / fps as f64;
    let audio_duration_s = wav.duration_s();
    let drift_s = (video_duration_s - audio_duration_s).abs();
    let timestamps: Vec<f64> = (0..frames).map(|i| i as f64 / fps as f64).collect();

    let header = serde_json::json!({
        "version": 1,
        "video_id": null, // filled by the builtin layer (registry id), null at lib level
        "audio_id": audio_id,
        "fps": fps,
        "frames": frames,
        "video_sha256": manifest.video_sha,
        "audio_sha256": sha256_hex(audio_bytes),
        "video_duration_s": video_duration_s,
        "audio_duration_s": audio_duration_s,
        "drift_s": drift_s,
        "timestamp": timestamp,
        "timestamps": timestamps,
    });
    let header_bytes = serde_json::to_vec(&header)
        .map_err(|e| format!("{}(): header serialization failed: {}", WHO, e))?;

    let mut container = Vec::with_capacity(
        7 + 4 + header_bytes.len() + video.video_bytes.len() + audio_bytes.len(),
    );
    container.extend_from_slice(MLGVAV_MAGIC);
    container.extend_from_slice(&(header_bytes.len() as u32).to_le_bytes());
    container.extend_from_slice(&header_bytes);
    container.extend_from_slice(&video.video_bytes);
    container.extend_from_slice(audio_bytes);

    let out_manifest = crate::video::VideoManifest {
        model_id: manifest.model_id.clone(),
        weights_sha: manifest.weights_sha.clone(),
        kind: VideoKind::AvMux,
        ref_hash: manifest.ref_hash.clone(),
        ref_last_hash: manifest.ref_last_hash.clone(),
        consent_hash: manifest.consent_hash.clone(),
        prompt_hash: manifest.prompt_hash.clone(),
        seed: manifest.seed,
        timestamp,
        policy: manifest.policy.clone(),
        video_sha: sha256_hex(&container),
        audio_ref: Some(audio_id),
        fps: manifest.fps,
        source_sha: Some(manifest.video_sha.clone()),
    };

    Ok(VideoArtifact {
        video_bytes: container,
        manifest: Some(out_manifest),
        latent: None, // A/V container is a terminal artifact — no latent reuse
    })
}

/// `video_export` core (ADR-0151 D5): signed-by-construction `.mlgv`
/// container. Refuses (loud) artifacts without a manifest — unsigned export
/// does not exist. Writes the file at `path` and returns the exact bytes.
pub fn export_video(artifact: &VideoArtifact, path: &str) -> Result<Vec<u8>, String> {
    const WHO: &str = "video_export";
    let manifest = artifact.manifest.as_ref().ok_or_else(|| {
        "VIDEO_UNSIGNED_EXPORT: artifact has no provenance manifest — unsigned export does \
         not exist (ADR-0149 D3, ADR-0151 D5); every pipeline artifact is signed by \
         construction, so this state can only come from a manually constructed artifact"
            .to_string()
    })?;
    if manifest.kind == VideoKind::AvMux && artifact.latent.is_some() {
        return Err(format!("{}(): inconsistent AvMux artifact state", WHO));
    }

    let manifest_json = serde_json::to_vec(&manifest.to_json())
        .map_err(|e| format!("{}(): manifest serialization failed: {}", WHO, e))?;
    // Watermark: deterministic 16 bytes derived from the manifest content —
    // EU AI Act Art. 50 provenance marker embedded by construction.
    let watermark = Sha256Watermark::new(manifest).bytes();

    let mut container = Vec::with_capacity(
        MLGV_MAGIC.len() + 4 + manifest_json.len() + watermark.len() + artifact.video_bytes.len(),
    );
    container.extend_from_slice(MLGV_MAGIC);
    container.extend_from_slice(&(manifest_json.len() as u32).to_le_bytes());
    container.extend_from_slice(&manifest_json);
    container.extend_from_slice(&watermark);
    container.extend_from_slice(&artifact.video_bytes);

    std::fs::write(path, &container)
        .map_err(|e| format!("{}(): cannot write {}: {}", WHO, path, e))?;
    Ok(container)
}

/// Deterministic watermark: first 16 bytes of sha256(model|seed|video_sha).
struct Sha256Watermark([u8; 16]);

impl Sha256Watermark {
    fn new(manifest: &crate::video::VideoManifest) -> Self {
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, manifest.model_id.as_bytes());
        sha2::Digest::update(&mut hasher, b"|");
        sha2::Digest::update(&mut hasher, manifest.seed.to_le_bytes());
        sha2::Digest::update(&mut hasher, b"|");
        sha2::Digest::update(&mut hasher, manifest.video_sha.as_bytes());
        let digest = sha2::Digest::finalize(hasher);
        let mut out = [0u8; 16];
        out.copy_from_slice(&digest[..16]);
        Self(out)
    }

    fn bytes(&self) -> [u8; 16] {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::video::i2v::{render, TINY_FPS};
    use crate::video::interp::{extend_video_artifact, frame_interp_artifact};

    /// Build a real deterministic PCM WAV (8000 Hz, mono, 16-bit, 0.5 s).
    /// Triangle-wave samples via pure integer math — no libm, no drift.
    fn make_wav() -> Vec<u8> {
        let sample_rate: u32 = 8000;
        let n_samples: usize = 4000; // 0.5 s
        let mut data = Vec::with_capacity(n_samples * 2);
        for i in 0..n_samples {
            // Deterministic triangle in [-2048, 2047], period 160 samples.
            let phase = (i % 160) as i32;
            let tri = if phase < 80 {
                -2048 + phase * 51
            } else {
                2048 - (phase - 80) * 51
            };
            data.extend_from_slice(&(tri as i16).to_le_bytes());
        }
        let data_len = data.len() as u32;
        let mut wav = Vec::with_capacity(44 + data.len());
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_len).to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&1u16.to_le_bytes()); // mono
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
        wav.extend_from_slice(&2u16.to_le_bytes()); // block align
        wav.extend_from_slice(&16u16.to_le_bytes()); // bits
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        wav.extend_from_slice(&data);
        wav
    }

    fn i2v_source() -> VideoArtifact {
        let r = crate::nn::attention::generate_uniform_f32(77, 3 * 32 * 32, -1.0, 1.0);
        render("wan-2.2-ti2v-5b", "mux scene", Some(&r), None, 0).unwrap()
    }

    #[test]
    fn parse_wav_real_container() {
        let wav = make_wav();
        let info = parse_wav(&wav).unwrap();
        assert_eq!(info.sample_rate, 8000);
        assert_eq!(info.channels, 1);
        assert_eq!(info.bits_per_sample, 16);
        assert_eq!(info.data_len, 8000);
        assert!((info.duration_s() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn parse_wav_loud_on_garbage() {
        assert!(parse_wav(&[]).is_err());
        assert!(parse_wav(b"not a wav at all").is_err());
        let mut wav = make_wav();
        wav[8] = b'X'; // break WAVE magic
        assert!(parse_wav(&wav).is_err());
        // Truncated data chunk: loud, with best-effort bounds respected.
        let wav2 = &make_wav()[..30];
        assert!(parse_wav(wav2).is_err());
    }

    #[test]
    fn mux_sidecar_structure_and_determinism() {
        let src = i2v_source();
        let wav = make_wav();
        let m1 = mux_av(&src, 7, &wav, 0).unwrap();
        let m2 = mux_av(&src, 7, &wav, 0).unwrap();
        assert_eq!(m1.video_bytes, m2.video_bytes, "mux must be deterministic");

        // Container structure: magic | json_len | json | video | audio.
        let b = &m1.video_bytes;
        assert_eq!(&b[0..7], MLGVAV_MAGIC);
        let json_len = u32::from_le_bytes([b[7], b[8], b[9], b[10]]) as usize;
        let header: serde_json::Value = serde_json::from_slice(&b[11..11 + json_len]).unwrap();
        assert_eq!(header["audio_id"], 7);
        assert_eq!(header["fps"], TINY_FPS);
        assert_eq!(header["frames"], 4); // T_lat=2 → 4 decoded frames
        let video_dur = header["video_duration_s"].as_f64().unwrap();
        assert!((video_dur - 0.5).abs() < 1e-9, "4 frames @ 8 fps = 0.5 s");
        let audio_dur = header["audio_duration_s"].as_f64().unwrap();
        assert!((audio_dur - 0.5).abs() < 1e-9);
        assert!(header["drift_s"].as_f64().unwrap() < 1e-9);
        assert_eq!(header["timestamps"].as_array().unwrap().len(), 4);
        assert_eq!(header["timestamps"][0], 0.0);
        assert!((header["timestamps"][1].as_f64().unwrap() - 0.125).abs() < 1e-9);

        // Manifest: AvMux kind, audio_ref wired, provenance chain to source.
        let m = m1.manifest.as_ref().unwrap();
        assert_eq!(m.kind, VideoKind::AvMux);
        assert_eq!(m.audio_ref, Some(7));
        assert_eq!(
            m.source_sha.as_deref(),
            Some(src.manifest.as_ref().unwrap().video_sha.as_str())
        );
        assert_eq!(m.video_sha, sha256_hex(&m1.video_bytes));

        // Payload contains both streams verbatim.
        let payload = &b[11 + json_len..];
        assert!(payload.starts_with(&src.video_bytes));
        assert!(payload.ends_with(&wav));
    }

    #[test]
    fn mux_refuses_unsigned_and_nested() {
        let src = i2v_source();
        let wav = make_wav();
        let mut unsigned = src.clone();
        unsigned.manifest = None;
        assert!(mux_av(&unsigned, 1, &wav, 0).is_err());
        // Nested mux forbidden.
        let muxed = mux_av(&src, 1, &wav, 0).unwrap();
        assert!(mux_av(&muxed, 1, &wav, 0).is_err());
        // Non-WAV audio is a loud error (MP3-class unsupported this phase).
        assert!(mux_av(&src, 1, b"id3 not a wav", 0).is_err());
    }

    #[test]
    fn export_container_structure_and_watermark() {
        let src = i2v_source();
        let interp = frame_interp_artifact(&src, 2, 0).unwrap();
        let ext = extend_video_artifact(&interp, 2, 0).unwrap();
        let wav = make_wav();
        let muxed = mux_av(&ext, 42, &wav, 0).unwrap();

        let path = std::env::temp_dir().join("mlogos_n309_export_test.mlgv");
        let bytes = export_video(&muxed, &path.to_str().unwrap()).unwrap();

        assert_eq!(&bytes[0..5], MLGV_MAGIC);
        let json_len = u32::from_le_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]) as usize;
        let manifest_json: serde_json::Value =
            serde_json::from_slice(&bytes[9..9 + json_len]).unwrap();
        assert_eq!(manifest_json["kind"], "av_mux");
        assert_eq!(manifest_json["audio_ref"], 42);
        assert_eq!(manifest_json["model_id"], "wan-2.2-ti2v-5b");
        assert!(
            manifest_json["ref_hash"].is_string(),
            "ref-hash provenance survives"
        );
        // Watermark present between manifest and payload.
        let wm = &bytes[9 + json_len..9 + json_len + 16];
        assert!(wm.iter().any(|&b| b != 0));
        // Payload verbatim.
        assert_eq!(&bytes[9 + json_len + 16..], &muxed.video_bytes[..]);
        // Written file matches returned bytes.
        let on_disk = std::fs::read(&path).unwrap();
        assert_eq!(on_disk, bytes);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn unsigned_export_gate_contract() {
        // №309 A.3: unsigned export does not exist — the gate is loud.
        let unsigned = VideoArtifact {
            video_bytes: vec![1, 2, 3],
            manifest: None,
            latent: None,
        };
        let err = export_video(&unsigned, "/tmp/never_should_exist.mlgv").unwrap_err();
        assert!(
            err.contains("VIDEO_UNSIGNED_EXPORT"),
            "gate check-id must be named, got: {}",
            err
        );
        assert!(!std::path::Path::new("/tmp/never_should_exist.mlgv").exists());
    }

    #[test]
    fn export_is_deterministic() {
        let src = i2v_source();
        let wav = make_wav();
        let m1 = mux_av(&src, 3, &wav, 0).unwrap();
        let m2 = mux_av(&src, 3, &wav, 0).unwrap();
        let p1 = std::env::temp_dir().join("mlogos_n309_det1.mlgv");
        let p2 = std::env::temp_dir().join("mlogos_n309_det2.mlgv");
        let b1 = export_video(&m1, &p1.to_str().unwrap()).unwrap();
        let b2 = export_video(&m2, &p2.to_str().unwrap()).unwrap();
        assert_eq!(
            b1, b2,
            "same inputs + same timestamp ⇒ byte-identical export"
        );
        let _ = std::fs::remove_file(&p1);
        let _ = std::fs::remove_file(&p2);
    }
}
