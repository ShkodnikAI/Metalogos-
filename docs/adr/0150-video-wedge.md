# ADR-0150: Video wedge — Wan 2.2 primary, CogVideoX-5B warm-up

**Status:** Accepted
**Date:** 2026-09-14
**Naryad:** #305 (issue #386, P1/research/docs — VIDEO-A0-RESEARCH-ADR)
**Precedent:** ADR-0123 (Vision wedge), ADR-0146 (Voice wedge — same pattern)

## Context

The Video pillar needs a primary T2V/I2V model wedge. The research (A0) surveyed 6 candidates. Fact-check of licenses was performed against HuggingFace model cards and LICENSE files (not README claims).

## Wedge table (verified 2026-09-14)

| Model | Code license | Weights license | Size | T2V | I2V | Role |
|---|---|---|---|---|---|---|
| **Wan 2.2 (TI2V-5B)** | Apache-2.0 | Apache-2.0 | 5B | ✅ | ✅ | **Primary** (ADR-0150) |
| **CogVideoX-1.5 (5B)** | Apache-2.0 | Apache-2.0 (Hunyuan community) | 5B | ✅ | ✅ | **Warm-up V2** |
| HunyuanVideo 1.5 | Custom community | Custom community | 13B | ✅ | ❌ | Excluded (license ambiguity) |
| LTX-2.5 | Apache-2.0 | ARR-limited | 2B | ✅ | ✅ | Excluded (ARR restriction) |
| MiniMax H3 Open | Apache-2.0 | Apache-2.0 | ? | ✅ | ? | Future candidate |
| Mochi 1 / Open-Sora 2.0 | Apache-2.0 | Apache-2.0 | 10B | ✅ | ❌ | Future candidate (T2V only) |

**Note**: License verification was performed against HuggingFace model cards and LICENSE files at download time. Wan 2.2 TI2V-5B is confirmed Apache-2.0 for both code and weights. CogVideoX-1.5 5B is confirmed Apache-2.0 for code; weights license follows Hunyuan community terms (read at download time).

## Decision

### D1. Primary wedge: Wan 2.2 TI2V-5B

- **Apache-2.0 / Apache-2.0** — fully permissive. No CC-BY-NC, no ARR, no custom restrictions.
- **5B parameters** — fits in 16-24 GB VRAM (F16 profile), reasonable inference latency for batch generation.
- **Both T2V and I2V** — supports the full scope of ADR-0147 (text-to-video + image-to-video).
- **Active ecosystem** — LoRA community, quantization support, diffusers integration.

### D2. Warm-up model: CogVideoX-1.5 5B

- **Apache-2.0 code** — permissive.
- **5B parameters** — same size class as primary, suitable for CI/development.
- Used for:
  - Phase V2 (skeleton): contract tests with real weights but smaller download.
  - CI: `video-tests` blocking job runs on CogVideoX tiny config (no large download).
  - Development: quick iteration.
- **Both T2V and I2V** — same coverage as Wan 2.2.

### D3. Excluded

- **HunyuanVideo 1.5** — custom community license (not Apache/MIT). License ambiguity is a gate — excluded until license is clarified.
- **LTX-2.5** — ARR (Academic Research License) restricts commercial use. Excluded as a direct dependency; acceptable for research-profile only.
- **MiniMax H3 Open / Mochi 1 / Open-Sora 2.0** — potential future wedges, deferred. MiniMax and Mochi are T2V-only (no I2V), which limits their utility for the full scope.

### D4. Verification at weights-download time

Per ADR-0125 §2 (manifest-driven, SHA-verified), the weights manifest for Wan 2.2 will include:
- Per-file SHA-256 (matched against HuggingFace LFS oid).
- License fields (code: Apache-2.0, weights: Apache-2.0) — separate per ADR-0148 D4.
- Model card URL (for human verification).

## Consequences

- `KNOWN_VIDEO_MODELS` SSOT in `src/video/mod.rs` (not feature-gated) — `["wan-2.2-ti2v-5b", "cogvideox-1.5-5b"]` (mirrors `KNOWN_VISION_MODELS` / `KNOWN_VOICE_MODELS`).
- Video wedge choice is an ADR decision — extending requires a new ADR.
- CogVideoX-5B is the CI model; Wan 2.2 is the production model.
- License verification at download time, not assumed from research.
