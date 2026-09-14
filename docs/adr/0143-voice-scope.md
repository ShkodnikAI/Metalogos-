# ADR-0143: Voice pillar — scope (TTS, zero-shot cloning, voice-design)

**Status:** Accepted
**Date:** 2026-09-14
**Naryad:** #301 (issue #369, P1/research/docs — VOICE_A0_ADR)
**Precedent:** ADR-0122 (Vision pillar scope — same staged architecture), ADR-0114 (opaque handle pattern), ADR-0125 (provenance gates)

## Context

The Voice pillar adds speech synthesis, voice cloning, and voice identity to Metalogos. Following the established discipline (ADR-0122 Vision, ADR-0114 Reflex), the pillar's scope must be fixed before any code — non-scope explicitly excluded.

## Decision

### In scope (v1)

1. **TTS (text-to-speech)**: `voice_generate(decl, text)` — synthesize speech from text using a pinned model. Output is `Value::Audio(AudioId)` — an opaque handle (ADR-0144).
2. **Zero-shot voice cloning**: `voice_enroll(decl, audio, kind: "cloned")` — enroll a speaker voiceprint from a short audio sample. Requires `ConsentToken` (ADR-0145, static check). Voiceprint is encrypted at rest (AES-256-GCM, `secret()` stack, Наряд №172).
3. **Voice design**: `voice { }` declaration — model, language, sample_rate, policy, profile (mirrors `vision { }` per ADR-0122).
4. **Audio export**: `audio_export(handle)` — returns signed audio bytes + manifest sidecar (provenance by construction, ADR-0125 pattern).
5. **Audio persistence**: `voice_save` / `voice_load` — SQLite BLOB (mirrors `vision_save`/`vision_load`, Наряд №242).

### Out of scope (explicit non-scope)

1. **Pre-training base models** — datacenter economics + CC-BY-NC licenses (Emilia-class). The project downloads pinned weights, does not train from scratch.
2. **Singing / music** — different acoustic model class, separate research cycle.
3. **Streaming TTS** — optional phase A7; v1 is batch (full utterance → audio).
4. **Voice conversion** (Seed-VC-class) — separate research cycle after A6. Seed-VC is GPL-3.0 — excluded as a direct dependency.
5. **Real-time / low-latency** — v1 targets offline synthesis, not live streaming.

## Consequences

- Voice pillar follows the same staged architecture as Vision (ADR-0122): R1 (skeleton) → R2 (dispatch) → R3 (real weights) → R4 (security) → R5 (persistence) → R6 (advanced).
- `voice { }` declaration grammar mirrors `vision { }` — name as STRING, fields required (no silent defaults).
- Feature gate: `--features voice` (implies `candle`), not in default build.
- Cross-pillar: shares `MODEL_WEIGHTS_UNSAFE` gate (Наряд №300), grammar `generative-declaration` pattern, registry license-field structure with Vision.
