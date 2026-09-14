// ── Voice pillar skeleton (Наряд №302, ADR-0143-0146) ───────────────
//
// Feature-gated under `voice` (which implies `candle`).
// NOT in default/full — same pattern as Vision (ADR-0122).
//
// The skeleton provides:
// - VoiceId / AudioId opaque handles (ADR-0114 pattern, mirrors VisionId)
// - VoiceRegistry (mirrors VisionRegistry from ADR-0124)
// - KNOWN_VOICE_MODELS SSOT (not feature-gated — mirrors KNOWN_VISION_MODELS)
//
// All builtins are stubs — loud errors, no silent fallbacks.

use std::collections::HashMap;
use std::sync::Mutex;

/// Opaque handle to a Voice artifact (voiceprint) in VoiceRegistry.
/// Weights/embeddings never enter Value — only the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct VoiceId(pub u32);

impl std::fmt::Display for VoiceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[Voice#{}]", self.0)
    }
}

/// Opaque handle to an Audio artifact (generated speech) in VoiceRegistry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AudioId(pub u32);

impl std::fmt::Display for AudioId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[Audio#{}]", self.0)
    }
}

/// Registry of Voice artifacts — voiceprints and audio.
/// Mirrors VisionRegistry (ADR-0124, src/vision/mod.rs).
#[derive(Debug, Default)]
pub struct VoiceRegistry {
    artifacts: HashMap<u32, AudioArtifact>,
    voiceprints: HashMap<u32, Voiceprint>,
    next_id: u32,
}

impl VoiceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_artifact(&mut self, artifact: AudioArtifact) -> AudioId {
        let id = AudioId(self.next_id);
        self.next_id += 1;
        self.artifacts.insert(id.0, artifact);
        id
    }

    pub fn insert_voiceprint(&mut self, vp: Voiceprint) -> VoiceId {
        let id = VoiceId(self.next_id);
        self.next_id += 1;
        self.voiceprints.insert(id.0, vp);
        id
    }

    pub fn get_artifact(&self, id: AudioId) -> Option<&AudioArtifact> {
        self.artifacts.get(&id.0)
    }

    pub fn get_voiceprint(&self, id: VoiceId) -> Option<&Voiceprint> {
        self.voiceprints.get(&id.0)
    }

    pub fn remove_artifact(&mut self, id: AudioId) {
        self.artifacts.remove(&id.0);
    }

    pub fn len(&self) -> usize {
        self.artifacts.len() + self.voiceprints.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Audio artifact — encoded audio bytes + optional provenance manifest.
#[derive(Debug)]
pub struct AudioArtifact {
    pub audio_bytes: Vec<u8>,
    pub manifest: Option<VoiceManifest>,
}

/// Voiceprint — biometric embedding (encrypted at rest per ADR-0145 D4).
#[derive(Debug)]
pub struct Voiceprint {
    pub embedding: Vec<f32>,
    pub model_id: String,
}

/// Provenance manifest for audio artifacts (ADR-0145, mirrors VisionManifest).
#[derive(Debug)]
pub struct VoiceManifest {
    pub model_id: String,
    pub weights_sha: String,
    pub seed: u64,
    pub prompt_hash: String,
    pub timestamp: u64,
    pub audio_sha: String,
}

/// Convenience wrapper for `Mutex<VoiceRegistry>`.
pub type SharedVoiceRegistry = Mutex<VoiceRegistry>;

/// SSOT list of voice models known to the language (ADR-0146).
/// NOT feature-gated — the list is available in all builds (mirrors
/// `KNOWN_VISION_MODELS` in `src/vision/mod.rs`).
pub const KNOWN_VOICE_MODELS: &[&str] = &["chatterbox-multilingual-v3", "koko-ro-82m"];

// ── Builtin stubs (Наряд №302) ───────────────────────────────────────
//
// All voice builtins are stubs in the skeleton phase (A1). They return
// loud errors — no silent fallbacks. Real implementation in A2/A3/A5+.

#[cfg(feature = "voice")]
use crate::interpreter::Value;

#[cfg(feature = "voice")]
/// `voice_enroll(decl, audio, kind)` stub — ADR-0145.
pub(crate) fn builtin_voice_enroll_stub(_args: &[Value]) -> Result<Value, String> {
    Err(
        "voice_enroll(): feature voice — builtin not implemented (phase A2/A3, ADR-0145)"
            .to_string(),
    )
}

#[cfg(feature = "voice")]
/// `tts_speak(decl, text)` stub — ADR-0143.
pub(crate) fn builtin_tts_speak_stub(_args: &[Value]) -> Result<Value, String> {
    Err("tts_speak(): feature voice — builtin not implemented (phase A2, ADR-0143)".to_string())
}

#[cfg(feature = "voice")]
/// `audio_export(handle)` stub — ADR-0145.
pub(crate) fn builtin_audio_export_stub(_args: &[Value]) -> Result<Value, String> {
    Err("audio_export(): feature voice — builtin not implemented (phase A4, ADR-0145)".to_string())
}

#[cfg(feature = "voice")]
/// `voice_design(text, voice)` stub — ADR-0143.
pub(crate) fn builtin_voice_design_stub(_args: &[Value]) -> Result<Value, String> {
    Err("voice_design(): feature voice — builtin not implemented (phase A6, ADR-0143)".to_string())
}

#[cfg(feature = "voice")]
/// `voice_save(handle, name)` stub — ADR-0143 (persistence).
pub(crate) fn builtin_voice_save_stub(_args: &[Value]) -> Result<Value, String> {
    Err("voice_save(): feature voice — builtin not implemented (phase A5, ADR-0143)".to_string())
}

#[cfg(feature = "voice")]
/// `voice_load(name)` stub — ADR-0143 (persistence).
pub(crate) fn builtin_voice_load_stub(_args: &[Value]) -> Result<Value, String> {
    Err("voice_load(): feature voice — builtin not implemented (phase A5, ADR-0143)".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_id_display() {
        assert_eq!(format!("{}", VoiceId(1)), "[Voice#1]");
        assert_eq!(format!("{}", VoiceId(42)), "[Voice#42]");
    }

    #[test]
    fn audio_id_display() {
        assert_eq!(format!("{}", AudioId(1)), "[Audio#1]");
    }

    #[test]
    fn registry_insert_returns_monotonic_ids() {
        let mut reg = VoiceRegistry::new();
        let a1 = reg.insert_artifact(AudioArtifact {
            audio_bytes: vec![1, 2, 3],
            manifest: None,
        });
        let a2 = reg.insert_artifact(AudioArtifact {
            audio_bytes: vec![4, 5],
            manifest: None,
        });
        assert_ne!(a1, a2);
    }

    #[test]
    fn registry_get_artifact() {
        let mut reg = VoiceRegistry::new();
        let id = reg.insert_artifact(AudioArtifact {
            audio_bytes: vec![1, 2, 3],
            manifest: None,
        });
        assert!(reg.get_artifact(id).is_some());
        assert!(reg.get_artifact(AudioId(999)).is_none());
    }

    #[test]
    fn known_voice_models_ssot() {
        assert!(KNOWN_VOICE_MODELS.contains(&"chatterbox-multilingual-v3"));
        assert!(KNOWN_VOICE_MODELS.contains(&"koko-ro-82m"));
    }

    #[test]
    #[cfg(feature = "voice")]
    fn stubs_are_loud() {
        assert!(builtin_voice_enroll_stub(&[]).is_err());
        assert!(builtin_tts_speak_stub(&[]).is_err());
        assert!(builtin_audio_export_stub(&[]).is_err());
        assert!(builtin_voice_design_stub(&[]).is_err());
        assert!(builtin_voice_save_stub(&[]).is_err());
        assert!(builtin_voice_load_stub(&[]).is_err());
    }
}
