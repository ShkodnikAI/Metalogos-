# ADR-0144: Voice value-registry — `Value::Audio(AudioId)` opaque handle + VoiceRegistry

**Status:** Accepted
**Date:** 2026-09-14
**Naryad:** #301 (issue #369, P1/research/docs — VOICE_A0_ADR)
**Precedent:** ADR-0114 (`Value::Reflex` opaque handle), ADR-0124 (`Value::Vision` + VisionRegistry), Наряд №300 (shared `MODEL_WEIGHTS_UNSAFE` gate)

## Context

The Voice pillar produces audio artifacts (generated speech, enrolled voiceprints) that must be addressable from `.mlog` source but must NOT enter `Value` directly — audio bytes and voiceprint tensors are too large for the Value enum, and voiceprints are privacy-sensitive (biometric data).

## Decision

### D1. `Value::Audio(AudioId)` — opaque handle

`AudioId = u32` — index into `VoiceRegistry` (process-global, `Lazy<Mutex<VoiceRegistry>>` in `crate::voice`, mirrors `VisionRegistry` from ADR-0124). Audio bytes, voiceprint tensors, and manifests never enter `Value`.

`Display` = `[Audio#N]`. `Debug` — manual, prints only duration + model, never audio bytes.

### D2. VoiceRegistry

```rust
pub struct VoiceRegistry {
    artifacts: HashMap<u32, AudioArtifact>,
    voiceprints: HashMap<u32, Voiceprint>,
    next_id: u32,
}

pub struct AudioArtifact {
    pub audio_bytes: Vec<u8>,     // WAV/MP3/FLAC encoded
    pub manifest: Option<VoiceManifest>,  // provenance sidecar
}

pub struct Voiceprint {
    pub embedding: Vec<f32>,      // ECAPA-class embedding (encrypted at rest)
    pub model_id: String,
    pub consent_ledger: Vec<ConsentRecord>,
}
```

### D3. License fields

Each registry entry carries separate fields for code license and weights license (precedent: ADR-0124 `vision_ident_val` with hyphens). The permissive-code + non-commercial-weights pattern (k2-fsa/OmniVoice: Apache-2.0 code, CC-BY-NC weights) requires this distinction.

### D4. Privacy — voiceprint never in Value

Voiceprints (biometric embeddings) are `Vec<f32>` stored in `VoiceRegistry` behind AES-256-GCM encryption (`secret()` stack, Наряд №172). They are:
- Never serialized to JSON.
- Never exported through `audio_export` (only audio artifacts are exported).
- Accessible only through `voice_generate` with an enrolled voiceprint handle (which requires consent per ADR-0145).

### D5. SSRF-guarded download

`voice_fetch_weights` reuses the same runtime layers as `vision_fetch_weights` (Наряд №130 SSRF guard, №300 `MODEL_WEIGHTS_UNSAFE` static gate). The suffix convention `_fetch_weights` (Наряд №300) covers `voice_fetch_weights` automatically — no new gate needed.

## Consequences

- `Value::Audio(AudioId)` is a new `Value` variant — needs match arms in all exhaustive positions (Display, type_name, is_nonprintable, template rendering, etc.). Precedent: `Value::Vision` from ADR-0124.
- `VoiceRegistry` lives in `crate::voice` (new module, feature-gated under `voice` which implies `candle`).
- Cross-pillar: shares `MODEL_WEIGHTS_UNSAFE` with Vision (one gate, two pillars — №300 generalized it).
- Privacy: voiceprint embeddings are never accessible through `Value` — only through controlled `voice_generate` / `voice_enroll` builtins.
