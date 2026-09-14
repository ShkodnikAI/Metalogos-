# ADR-0148: Video value-registry — `Value::Video(VideoId)` opaque handle + VideoRegistry

**Status:** Accepted
**Date:** 2026-09-14
**Naryad:** #305 (issue #386, P1/research/docs — VIDEO-A0-RESEARCH-ADR)
**Precedent:** ADR-0114 (`Value::Reflex`), ADR-0124 (`Value::Vision`), ADR-0144 (`Value::Audio`/`Value::Voice`), Наряд №300 (shared `MODEL_WEIGHTS_UNSAFE`)

## Context

The Video pillar produces video artifacts (generated clips, I2V animations) that must be addressable from `.mlog` source but must NOT enter `Value` directly — video bytes are too large, and video manifests carry provenance that must be tamper-evident.

## Decision

### D1. `Value::Video(VideoId)` — opaque handle

`VideoId = u32` — index into `VideoRegistry` (process-global, `Lazy<Mutex<VideoRegistry>>` in `crate::video`, mirrors `VoiceRegistry` from ADR-0144). Video bytes and manifests never enter `Value`.

`Display` = `[Video#N]`. `Debug` — manual, prints only duration + model + resolution, never video bytes.

### D2. VideoRegistry

```rust
pub struct VideoRegistry {
    artifacts: HashMap<u32, VideoArtifact>,
    next_id: u32,
}

pub struct VideoArtifact {
    pub video_bytes: Vec<u8>,        // MP4/WebM encoded
    pub manifest: Option<VideoManifest>,
}
```

### D3. VideoManifest — provenance sidecar

```rust
pub struct VideoManifest {
    pub model_id: String,
    pub weights_sha: String,
    pub kind: VideoKind,            // T2V | I2V | AvMux
    pub ref_hash: Option<String>,   // I2V: hash of source image
    pub consent_hash: Option<String>, // LikenessToken hash (ADR-0149)
    pub prompt_hash: String,
    pub seed: u64,
    pub timestamp: u64,
    pub policy: String,             // "safe" | "adult"
    pub video_sha: String,          // SHA-256 of final video bytes
    pub audio_ref: Option<AudioId>, // av_mux: reference to audio track
}
```

### D4. License fields

Each registry entry carries separate fields for code license and weights license (precedent: ADR-0144 D3 Voice, ADR-0124 Vision). The permissive-code + non-commercial-weights pattern requires this distinction for each candidate wedge model.

### D5. SSRF-guarded download

`video_fetch_weights` reuses the same runtime layers as `vision_fetch_weights` / `voice_fetch_weights` (Наряд №130 SSRF guard, №300 `MODEL_WEIGHTS_UNSAFE` static gate). The suffix convention `_fetch_weights` (Наряд №300) covers `video_fetch_weights` automatically — no new gate needed.

## Consequences

- `Value::Video(VideoId)` is a new `Value` variant — needs match arms in all exhaustive positions. Precedent: `Value::Vision` (ADR-0124), `Value::Voice`/`Value::Audio` (ADR-0144).
- `VideoRegistry` lives in `crate::video` (new module, feature-gated under `video` which implies `candle`).
- Cross-pillar: shares `MODEL_WEIGHTS_UNSAFE` with Vision/Voice (one gate, three pillars — №300 generalized it).
- `av_mux` creates `Value::Video` with `audio_ref` pointing to a `Value::Audio` handle — cross-pillar composition (ADR-0149 §cross-pillar).
