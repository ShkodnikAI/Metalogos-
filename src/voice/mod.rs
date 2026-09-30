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

use crate::interpreter::Value;

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

/// №515 (issue #799; the consolidated audit 28.09 C-12): the hard cap on
/// the process-global audio artifact store — the same bound as
/// VIDEO_ARTIFACTS_MAX (nothing in production removes an artifact, so
/// repeated synth/mux without an explicit removal grew the store without
/// limit; the oldest handle — the lowest id — is evicted at the cap).
pub const VOICE_ARTIFACTS_MAX: usize = 64;

/// №529 (issue #838; the consolidated audit 30.09 Д-5+N-6): the SECOND
/// circuit — the byte ceiling on BOTH in-memory voice maps (the audio
/// artifacts and the voiceprints). The count cap (№515) bounds the
/// NUMBER of entries; large renders/embeddings could grow the footprint
/// unboundedly within it. Accounting: the raw audio bytes + the
/// manifest's text fields; a voiceprint counts its embedding (f32 × 4 ×
/// dim) + the model id. Deterministic by construction.
pub const VOICE_ARTIFACTS_MAX_BYTES: usize = 32 * 1024 * 1024; // 32 MiB

/// №529: the eviction metrics — the serve-visible observability of the
/// two-circuit bound (one loud `[REGISTRY_EVICTION]` line per victim is
/// the serve-report primitive; the counters make the totals observable).
static VOICE_EVICTED_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static VOICE_EVICTED_BYTES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// №529: the public eviction metrics for the voice registry (both maps
/// together) — `(victims_total, victim_bytes_total)` since process start.
#[doc(hidden)]
pub fn voice_registry_eviction_metrics() -> (u64, u64) {
    use std::sync::atomic::Ordering::Relaxed;
    (
        VOICE_EVICTED_COUNT.load(Relaxed),
        VOICE_EVICTED_BYTES.load(Relaxed),
    )
}

/// №529: the deterministic byte accounting of one audio artifact.
pub fn audio_artifact_bytes(a: &AudioArtifact) -> usize {
    const MANIFEST_STRUCT: usize = 48;
    a.audio_bytes.len()
        + a.manifest
            .as_ref()
            .map(|m| {
                m.model_id.len()
                    + m.weights_sha.len()
                    + m.prompt_hash.len()
                    + m.audio_sha.len()
                    + MANIFEST_STRUCT
            })
            .unwrap_or(0)
}

/// №529: the deterministic byte accounting of one voiceprint.
pub fn voiceprint_bytes(vp: &Voiceprint) -> usize {
    vp.embedding.len() * 4 + vp.model_id.len() + 16
}

impl VoiceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_artifact(&mut self, artifact: AudioArtifact) -> AudioId {
        use std::sync::atomic::Ordering::Relaxed;
        // №529: the two-circuit eviction — the COUNT circuit (№515) and the
        // BYTE circuit (№529); the victim order is the lowest id = least
        // recently created (ids are monotonic; the in-memory getters are
        // &self — the restamp boundary is documented in the inventory). A
        // single artifact larger than the byte cap is admitted into an
        // EMPTY store (the cap bounds accumulation, not one artifact).
        let incoming = audio_artifact_bytes(&artifact);
        let mut total: usize = self.artifacts.values().map(audio_artifact_bytes).sum();
        while !self.artifacts.is_empty()
            && (self.artifacts.len() >= VOICE_ARTIFACTS_MAX
                || total + incoming > VOICE_ARTIFACTS_MAX_BYTES)
        {
            let reason = if self.artifacts.len() >= VOICE_ARTIFACTS_MAX {
                "count"
            } else {
                "bytes"
            };
            let Some(&oldest) = self.artifacts.keys().min() else {
                break;
            };
            if let Some(victim) = self.artifacts.remove(&oldest) {
                let vb = audio_artifact_bytes(&victim);
                total -= vb;
                VOICE_EVICTED_COUNT.fetch_add(1, Relaxed);
                VOICE_EVICTED_BYTES.fetch_add(vb as u64, Relaxed);
                eprintln!(
                    "[REGISTRY_EVICTION] voice_registry artifact={} victim_bytes={} reason={} store_len={} store_bytes={} caps=({},{})",
                    oldest,
                    vb,
                    reason,
                    self.artifacts.len(),
                    total,
                    VOICE_ARTIFACTS_MAX,
                    VOICE_ARTIFACTS_MAX_BYTES
                );
            }
        }
        let id = AudioId(self.next_id);
        self.next_id += 1;
        self.artifacts.insert(id.0, artifact);
        id
    }

    pub fn insert_voiceprint(&mut self, vp: Voiceprint) -> VoiceId {
        use std::sync::atomic::Ordering::Relaxed;
        // №529: the same two-circuit bound on the voiceprint map (the mock
        // runtime's skeleton path is the only writer since №512's
        // fail-closed store; the byte circuit accounts the embedding).
        let incoming = voiceprint_bytes(&vp);
        let mut total: usize = self.voiceprints.values().map(voiceprint_bytes).sum();
        while !self.voiceprints.is_empty()
            && (self.voiceprints.len() >= VOICE_ARTIFACTS_MAX
                || total + incoming > VOICE_ARTIFACTS_MAX_BYTES)
        {
            let reason = if self.voiceprints.len() >= VOICE_ARTIFACTS_MAX {
                "count"
            } else {
                "bytes"
            };
            let Some(&oldest) = self.voiceprints.keys().min() else {
                break;
            };
            if let Some(victim) = self.voiceprints.remove(&oldest) {
                let vb = voiceprint_bytes(&victim);
                total -= vb;
                VOICE_EVICTED_COUNT.fetch_add(1, Relaxed);
                VOICE_EVICTED_BYTES.fetch_add(vb as u64, Relaxed);
                eprintln!(
                    "[REGISTRY_EVICTION] voice_registry voiceprint={} victim_bytes={} reason={} store_len={} store_bytes={} caps=({},{})",
                    oldest,
                    vb,
                    reason,
                    self.voiceprints.len(),
                    total,
                    VOICE_ARTIFACTS_MAX,
                    VOICE_ARTIFACTS_MAX_BYTES
                );
            }
        }
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

    /// №526: remove a voiceprint by handle — the RAM-side erasure.
    /// Returns whether the handle was present (the idempotency signal:
    /// `false` = already absent, the repeated erase is not an error).
    pub fn remove_voiceprint(&mut self, id: VoiceId) -> bool {
        self.voiceprints.remove(&id.0).is_some()
    }

    /// №526: list the held voiceprints — the informed-deletion basis at
    /// the runtime surface. Deterministic order (ascending id); the
    /// embedding bytes NEVER enter the result (id + model only — the
    /// metadata composition, mirroring the store's VoiceprintRecord
    /// posture: the listing never decrypts, never materializes bytes).
    pub fn list_voiceprints(&self) -> Vec<(VoiceId, String)> {
        let mut ids: Vec<&u32> = self.voiceprints.keys().collect();
        ids.sort();
        ids.iter()
            .map(|&k| {
                let vp = &self.voiceprints[k];
                (VoiceId(*k), vp.model_id.clone())
            })
            .collect()
    }

    /// №526: list the held audio artifacts — ascending id + the byte
    /// length (the size is the deletion-relevant fact; the bytes never
    /// enter the result).
    pub fn list_artifacts(&self) -> Vec<(AudioId, usize)> {
        let mut ids: Vec<&u32> = self.artifacts.keys().collect();
        ids.sort();
        ids.iter()
            .map(|&k| {
                let a = &self.artifacts[k];
                (AudioId(*k), a.audio_bytes.len())
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.artifacts.len() + self.voiceprints.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Audio artifact — encoded audio bytes + optional provenance manifest.
/// Clone (Наряд №309): av_mux resolves bytes against the global registry
/// without holding the lock during the mux computation.
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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

/// Process-global voice registry (Наряд №309) — the same `once_cell::Lazy`
/// pattern as `BPE_REGISTRY` / `VIDEO_REGISTRY`. av_mux resolves
/// `Value::Audio` handles to `AudioArtifact.audio_bytes` through it.
pub static VOICE_REGISTRY: once_cell::sync::Lazy<Mutex<VoiceRegistry>> =
    once_cell::sync::Lazy::new(|| Mutex::new(VoiceRegistry::new()));

/// SSOT list of voice models known to the language (ADR-0146).
/// NOT feature-gated — the list is available in all builds (mirrors
/// `KNOWN_VISION_MODELS` in `src/vision/mod.rs`).
pub const KNOWN_VOICE_MODELS: &[&str] = &["chatterbox-multilingual-v3", "koko-ro-82m"];

pub mod encoder;
pub mod store;
// №334: STT/omni backend wiring — the mock-first call contract over the
// №333 registry (not feature-gated: the mock path and the loud refusals
// are available in all builds, mirroring KNOWN_VOICE_MODELS).
pub mod backend;
//
// №526: voice_delete / voice_list are REAL (not stubs) — the GDPR Art. 17
// erasure path over the live runtime surfaces (the RAM registry + the
// store API), available in ALL builds (the erasure right is not
// feature-gated; VOICE_REGISTRY itself was never feature-gated).

/// №526 (issue #835; audit 30.09 N-2): `voice_delete(handle)` — the
/// erasure path over the live VOICE_REGISTRY. Accepts a Voice handle
/// (the voiceprint — the biometric embedding) or an Audio handle (the
/// audio artifact — the "artifact file" bytes of the registry). IDEMPOTENT:
/// an absent handle is `"absent"` (the repeated erase succeeds — GDPR
/// Art. 17(1): erasure without obstacles), never an error. The consent
/// ledger is untouched (the Art. 9 consent proof survives by design —
/// docs/privacy.md §2.1). Wrong types refuse loudly.
/// The PERSISTED (SQLite) prints are erased through the store API
/// `VoiceStore::delete_voiceprint` (the same secure zero-then-delete
/// path, tested at the store level) — a shipped runtime persists zero
/// voiceprints today (privacy.md §2.1), so the registry is the live
/// surface the builtin erases.
pub(crate) fn builtin_voice_delete(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "voice_delete(): expects 1 argument (voice|audio handle), got {}",
            args.len()
        ));
    }
    let mut reg = VOICE_REGISTRY
        .lock()
        .map_err(|_| "voice_delete(): VOICE_REGISTRY poisoned".to_string())?;
    let erased = match &args[0] {
        Value::Voice(id) => reg.remove_voiceprint(*id),
        Value::Audio(id) => {
            let present = reg.get_artifact(*id).is_some();
            reg.remove_artifact(*id);
            present
        }
        other => {
            return Err(format!(
                "voice_delete(): expects a Voice or Audio handle, got {}",
                other.type_name()
            ));
        }
    };
    Ok(Value::String(
        if erased { "deleted" } else { "absent" }.to_string(),
    ))
}

/// №526: `voice_list()` — the informed-deletion basis at the runtime
/// surface: the held voiceprints (id, model — the embedding NEVER enters
/// the result) and the held audio artifacts (id, byte length). Empty
/// registry → the empty string (the honest zero). Deterministic order
/// (ascending ids). The persisted-store listing (name/model/saved_at/
/// algo/ciphertext length, never decrypted) is `VoiceStore::list_voiceprints`
/// — the same №526 composition at the store level.
pub(crate) fn builtin_voice_list(args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!(
            "voice_list(): expects 0 arguments, got {}",
            args.len()
        ));
    }
    let reg = VOICE_REGISTRY
        .lock()
        .map_err(|_| "voice_list(): VOICE_REGISTRY poisoned".to_string())?;
    let mut out = String::new();
    for (id, model) in reg.list_voiceprints() {
        out.push_str(&format!("voice\t{}\t{}\n", id, model));
    }
    for (id, len) in reg.list_artifacts() {
        out.push_str(&format!("audio\t{}\t{} bytes\n", id, len));
    }
    Ok(Value::String(out))
}

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

    // ── №515 (issue #799; C-12): the registry is BOUNDED ──

    #[test]
    fn n515_voice_registry_bounded_1000_inserts() {
        let mut reg = VoiceRegistry::new();
        let mut first_id = None;
        for i in 0..1000 {
            let id = reg.insert_artifact(AudioArtifact {
                audio_bytes: vec![i as u8; 16],
                manifest: None,
            });
            if first_id.is_none() {
                first_id = Some(id);
            }
        }
        assert_eq!(
            reg.len(),
            VOICE_ARTIFACTS_MAX,
            "bounded by the constant (voiceprints empty in this test)"
        );
        assert!(
            reg.get_artifact(first_id.unwrap()).is_none(),
            "the oldest orphan was evicted"
        );
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

    // ── №526 (issue #835; N-2): the runtime erasure path ──

    #[test]
    fn n526_registry_delete_and_list_cycle() {
        let mut reg = VoiceRegistry::new();
        let id = reg.insert_voiceprint(Voiceprint {
            embedding: vec![0.1, 0.2],
            model_id: "koko-ro-82m".to_string(),
        });
        let listed = reg.list_voiceprints();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].0, id);
        assert_eq!(listed[0].1, "koko-ro-82m");
        assert!(reg.remove_voiceprint(id));
        assert!(
            !reg.remove_voiceprint(id),
            "idempotent: the second erase is false"
        );
        assert!(reg.list_voiceprints().is_empty());
    }

    #[test]
    fn n526_builtin_voice_delete_then_list_empty() {
        // the №526 language-path cycle: insert → voice_list shows it →
        // voice_delete → voice_list empty; a repeated erase is "absent"
        let id = {
            let mut reg = VOICE_REGISTRY.lock().unwrap();
            reg.insert_voiceprint(Voiceprint {
                embedding: vec![0.3, 0.4],
                model_id: "chatterbox-multilingual-v3".to_string(),
            })
        };
        let listed = builtin_voice_list(&[]).unwrap();
        match &listed {
            Value::String(s) => {
                assert!(s.contains("voice\t[Voice#"), "the print is listed: {s}");
                assert!(
                    s.contains("chatterbox-multilingual-v3"),
                    "the model is listed: {s}"
                );
                assert!(!s.contains("0.3"), "the embedding never enters the listing");
            }
            other => panic!("voice_list must return String, got {}", other.type_name()),
        }
        let erased = builtin_voice_delete(&[Value::Voice(id)]).unwrap();
        assert!(
            matches!(&erased, Value::String(s) if s == "deleted"),
            "the erase reports deleted, got {erased:?}"
        );
        let again = builtin_voice_delete(&[Value::Voice(id)]).unwrap();
        assert!(
            matches!(&again, Value::String(s) if s == "absent"),
            "idempotent: the repeated erase reports absent, got {again:?}"
        );
        let listed = builtin_voice_list(&[]).unwrap();
        match &listed {
            Value::String(s) => assert!(
                !s.contains("[Voice#") || !s.contains("chatterbox-multilingual-v3"),
                "the deleted print is not listed: {s}"
            ),
            other => panic!("voice_list must return String, got {}", other.type_name()),
        }
        // cleanup: the audio map is untouched; the voiceprint map is clean
        let reg = VOICE_REGISTRY.lock().unwrap();
        assert_eq!(reg.list_voiceprints().len(), 0);
    }

    #[test]
    fn n526_builtin_voice_delete_audio_artifact() {
        let id = {
            let mut reg = VOICE_REGISTRY.lock().unwrap();
            reg.insert_artifact(AudioArtifact {
                audio_bytes: vec![7u8; 32],
                manifest: None,
            })
        };
        let listed = builtin_voice_list(&[]).unwrap();
        match &listed {
            Value::String(s) => assert!(s.contains("audio\t[Audio#") && s.contains("32 bytes")),
            other => panic!("voice_list must return String, got {}", other.type_name()),
        }
        let erased = builtin_voice_delete(&[Value::Audio(id)]).unwrap();
        assert!(
            matches!(&erased, Value::String(s) if s == "deleted"),
            "the artifact erase reports deleted, got {erased:?}"
        );
        let reg = VOICE_REGISTRY.lock().unwrap();
        assert!(reg.get_artifact(id).is_none(), "the artifact is gone");
    }

    #[test]
    fn n526_builtin_voice_delete_loud_on_wrong_type() {
        let r = builtin_voice_delete(&[Value::Float(3.0)]);
        assert!(r.is_err(), "a non-handle argument refuses loudly");
        assert!(r.unwrap_err().contains("Voice or Audio handle"));
        let r = builtin_voice_delete(&[]);
        assert!(r.is_err(), "arity refuses loudly");
        let r = builtin_voice_list(&[Value::Float(1.0)]);
        assert!(r.is_err(), "voice_list takes no arguments");
    }
}
