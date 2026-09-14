// ── Video pillar skeleton (Наряд №307, ADR-0147-0150) ───────────────
//
// Feature-gated under `video` (which implies `candle`).
// NOT in default/full — same pattern as Vision (ADR-0122) / Voice (ADR-0143).
//
// The skeleton provides:
// - VideoId opaque handle (ADR-0148, mirrors VisionId/AudioId)
// - VideoRegistry (mirrors VisionRegistry/VoiceRegistry)
// - KNOWN_VIDEO_MODELS SSOT (not feature-gated)
// - VideoManifest (ADR-0148)
// - 5 loud stub builtins (video_render, video_export, av_mux, frame_interp,
//   video_fetch_weights)

use std::collections::HashMap;
use std::sync::Mutex;

/// Opaque handle to a Video artifact in VideoRegistry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct VideoId(pub u32);

impl std::fmt::Display for VideoId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[Video#{}]", self.0)
    }
}

/// Registry of Video artifacts.
/// Mirrors VisionRegistry (ADR-0124) / VoiceRegistry (ADR-0144).
#[derive(Debug, Default)]
pub struct VideoRegistry {
    artifacts: HashMap<u32, VideoArtifact>,
    next_id: u32,
}

impl VideoRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_artifact(&mut self, artifact: VideoArtifact) -> VideoId {
        let id = VideoId(self.next_id);
        self.next_id += 1;
        self.artifacts.insert(id.0, artifact);
        id
    }

    pub fn get_artifact(&self, id: VideoId) -> Option<&VideoArtifact> {
        self.artifacts.get(&id.0)
    }

    pub fn remove_artifact(&mut self, id: VideoId) {
        self.artifacts.remove(&id.0);
    }

    pub fn len(&self) -> usize {
        self.artifacts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }
}

/// Video artifact — encoded video bytes + optional provenance manifest.
#[derive(Debug)]
pub struct VideoArtifact {
    pub video_bytes: Vec<u8>,
    pub manifest: Option<VideoManifest>,
}

/// Provenance manifest for video artifacts (ADR-0148).
#[derive(Debug)]
pub struct VideoManifest {
    pub model_id: String,
    pub weights_sha: String,
    pub kind: VideoKind,
    pub ref_hash: Option<String>,
    pub consent_hash: Option<String>,
    pub prompt_hash: String,
    pub seed: u64,
    pub timestamp: u64,
    pub policy: String,
    pub video_sha: String,
    pub audio_ref: Option<u32>, // AudioId.0 for av_mux
}

/// Kind of video generation (ADR-0147).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoKind {
    T2V,
    I2V,
    AvMux,
}

/// Convenience wrapper for `Mutex<VideoRegistry>`.
pub type SharedVideoRegistry = Mutex<VideoRegistry>;

/// SSOT list of video models known to the language (ADR-0150).
/// NOT feature-gated — mirrors `KNOWN_VISION_MODELS` / `KNOWN_VOICE_MODELS`.
pub const KNOWN_VIDEO_MODELS: &[&str] = &["wan-2.2-ti2v-5b", "cogvideox-1.5-5b"];

pub mod denoiser;
pub mod sampler;
pub mod vae;

// ── Builtin stubs (Наряд №307) ───────────────────────────────────────
// All video builtins are stubs — loud errors, no silent fallbacks.

#[cfg(feature = "video")]
use crate::interpreter::Value;

#[cfg(feature = "video")]
/// `video_render(decl, prompt)` stub — ADR-0147 T2V.
pub(crate) fn builtin_video_render_stub(_args: &[Value]) -> Result<Value, String> {
    Err("video_render(): feature video — builtin not implemented (phase V2, ADR-0147)".to_string())
}

#[cfg(feature = "video")]
/// `video_export(handle)` stub — ADR-0149.
pub(crate) fn builtin_video_export_stub(_args: &[Value]) -> Result<Value, String> {
    Err("video_export(): feature video — builtin not implemented (phase V4, ADR-0149)".to_string())
}

#[cfg(feature = "video")]
/// `av_mux(video, audio)` stub — ADR-0147 cross-pillar.
pub(crate) fn builtin_av_mux_stub(_args: &[Value]) -> Result<Value, String> {
    Err("av_mux(): feature video — builtin not implemented (phase V6, ADR-0147)".to_string())
}

#[cfg(feature = "video")]
/// `frame_interp(handle, count)` stub — ADR-0147 v2v (phase-gated).
pub(crate) fn builtin_frame_interp_stub(_args: &[Value]) -> Result<Value, String> {
    Err("frame_interp(): feature video — builtin not implemented (phase V7+, ADR-0147)".to_string())
}

#[cfg(feature = "video")]
/// `video_fetch_weights(url, dir)` stub — ADR-0148.
/// Note: the MODEL_WEIGHTS_UNSAFE static gate (audit.rs) already covers
/// this name via suffix convention `_fetch_weights` (Наряд №300).
pub(crate) fn builtin_video_fetch_weights_stub(_args: &[Value]) -> Result<Value, String> {
    Err(
        "video_fetch_weights(): feature video — builtin not implemented (phase V2, ADR-0148)"
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_id_display() {
        assert_eq!(format!("{}", VideoId(1)), "[Video#1]");
        assert_eq!(format!("{}", VideoId(42)), "[Video#42]");
    }

    #[test]
    fn registry_insert_returns_monotonic_ids() {
        let mut reg = VideoRegistry::new();
        let a1 = reg.insert_artifact(VideoArtifact {
            video_bytes: vec![1, 2, 3],
            manifest: None,
        });
        let a2 = reg.insert_artifact(VideoArtifact {
            video_bytes: vec![4, 5],
            manifest: None,
        });
        assert_ne!(a1, a2);
    }

    #[test]
    fn registry_get_artifact() {
        let mut reg = VideoRegistry::new();
        let id = reg.insert_artifact(VideoArtifact {
            video_bytes: vec![1, 2, 3],
            manifest: None,
        });
        assert!(reg.get_artifact(id).is_some());
        assert!(reg.get_artifact(VideoId(999)).is_none());
    }

    #[test]
    fn known_video_models_ssot() {
        assert!(KNOWN_VIDEO_MODELS.contains(&"wan-2.2-ti2v-5b"));
        assert!(KNOWN_VIDEO_MODELS.contains(&"cogvideox-1.5-5b"));
    }

    #[cfg(feature = "video")]
    #[test]
    fn stubs_are_loud() {
        assert!(builtin_video_render_stub(&[]).is_err());
        assert!(builtin_video_export_stub(&[]).is_err());
        assert!(builtin_av_mux_stub(&[]).is_err());
        assert!(builtin_frame_interp_stub(&[]).is_err());
        assert!(builtin_video_fetch_weights_stub(&[]).is_err());
    }
}
