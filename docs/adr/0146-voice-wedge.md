# ADR-0146: Voice wedge — Chatterbox Multilingual V3 primary, Kokoro-82M warm-up

**Status:** Accepted
**Date:** 2026-09-14
**Naryad:** #301 (issue #369, P1/research/docs — VOICE_A0_ADR)
**Precedent:** ADR-0123 (Vision wedge — Z-Image-Turbo primary, FLUX.2 [klein] fallback)

## Context

The Voice pillar needs a primary TTS/voice-cloning model wedge. The research (A0) surveyed 6 candidates:

| Model | Code license | Weights license | Size | Languages | Cloning | Watermark |
|---|---|---|---|---|---|---|
| **Chatterbox Multilingual V3** | MIT | MIT | 500M | 25 | ✅ | by default |
| CosyVoice 3 | Apache-2.0 | Apache-2.0 | 0.5B | ? | ✅ | ? |
| NeuTTS Air | Apache-2.0 | Apache-2.0 | 748M | ? | ✅ | ? |
| GPT-SoVITS | MIT | MIT | ? | ? | ✅ | ? |
| F5-TTS | MIT | CC-BY-NC | ? | ? | ? | ? |
| Kokoro-82M | Apache-2.0 | Apache-2.0 | 82M | ? | ❌ | ? |

**Note**: language coverage and exact model names need fact-check at execution time (A0 research did not have HuggingFace access — verified when downloading weights via runbook).

## Decision

### D1. Primary wedge: Chatterbox Multilingual V3

- **MIT/MIT** — permissive both code and weights. No CC-BY-NC restriction.
- **500M parameters** — fits in 4-8 GB RAM (F16 profile), reasonable inference latency.
- **25 languages** — broad coverage including Russian (Fosved primary).
- **Watermark by default** — EU AI Act Art. 50 compliance out of the box.
- **Zero-shot cloning** — core feature of the Voice pillar.

### D2. Warm-up model: Kokoro-82M

- **Apache-2.0/Apache-2.0** — fully permissive.
- **82M parameters** — extremely lightweight, fits any container. Used for:
  - Phase A2 (skeleton): contract tests with real weights but tiny model.
  - CI: `voice-tests` blocking job runs on Kokoro (no large download).
  - Development: quick iteration without GPU.
- **No cloning** — Kokoro is TTS-only (no zero-shot voice cloning). This is acceptable for the warm-up phase; cloning is tested with Chatterbox in env-gated tests (skipped loudly in CI, like `naryad_212_wedge_e2e`).

### D3. Excluded

- **Seed-VC** (GPL-3.0) — excluded as direct dependency (GPL contamination).
- **F5-TTS** (CC-BY-NC weights) — research-profile only, not production.
- **CosyVoice 3 / NeuTTS Air / GPT-SoVITS** — potential future wedges, but A0 did not verify ru/be quality, language coverage, or exact model names. Deferred to a future wedge-replacement ADR if Chatterbox proves insufficient.

### D4. Verification at weights-download time

Per ADR-0125 §2 (manifest-driven, SHA-verified, post-download refusal), the weights manifest for Chatterbox will include:
- Per-file SHA-256 (matched against HuggingFace LFS oid).
- License fields (code: MIT, weights: MIT) — separate per ADR-0144 D3.
- Language list (verified at download time, not from research).

## Consequences

- `KNOWN_VOICE_MODELS` SSOT in `src/voice/mod.rs` (not feature-gated) — `["chatterbox-multilingual-v3", "koko-ro-82m"]` (mirrors `KNOWN_VISION_MODELS`).
- Voice wedge choice is an ADR decision, not a code-level edit — extending requires a new ADR.
- Kokoro-82M is the CI model (like DiT tiny-golden in Vision, ADR-0123); Chatterbox is the production model (like Z-Image-Turbo).
- Language coverage must be fact-checked at weights-download time, not assumed from research.
