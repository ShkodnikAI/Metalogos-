# ADR-0147: Video pillar — scope (T2V/I2V primary, v2v phase-gated)

**Status:** Accepted
**Date:** 2026-09-14
**Naryad:** #305 (issue #386, P1/research/docs — VIDEO-A0-RESEARCH-ADR)
**Supersedes (partial):** ADR-0122 §"images before video" — the "images before video" boundary is lifted by this ADR as a deliberate owner decision; ADR-0122's Vision-specific scope (images) remains unchanged.
**Precedent:** ADR-0122 (Vision scope), ADR-0143 (Voice scope — same pattern), ADR-0114 (opaque handle)

## Context

The Video pillar adds text-to-video (T2V) and image-to-video (I2V) generation to Metalogos. ADR-0122 explicitly deferred video to "a separate research cycle" — that cycle is now complete (plan v2 accepted 2026-09-14). This ADR fixes the scope before any code, following the established discipline (ADR-0122 Vision, ADR-0143 Voice).

## Decision

### In scope (v1)

1. **T2V (text-to-video)**: `video_generate(decl, prompt)` — synthesize video from text using a pinned model. Output is `Value::Video(VideoId)` — opaque handle (ADR-0148).
2. **I2V (image-to-video)**: `video_generate(decl, prompt, image_handle)` — animate a still image (Vision artifact or uploaded frame). Requires `LikenessToken` if the image depicts a real person (ADR-0149).
3. **Video design**: `video { }` declaration — model, resolution, fps, duration, policy, profile (mirrors `vision { }` / `voice { }`).
4. **Video export**: `video_export(handle)` — returns signed video bytes + manifest sidecar (provenance by construction, ADR-0125 pattern).
5. **Video persistence**: `video_save` / `video_load` — SQLite BLOB (mirrors `vision_save`/`voice_save`).
6. **av_mux (audio+video muxing)**: combine `Value::Audio` + `Value::Video` → `Value::Video` with audio track (cross-pillar, ADR-0149 §cross-pillar).

### Out of scope (explicit non-scope, phase 1)

1. **Singing / music videos** — different acoustic+visual model class.
2. **Long-form montage / editing** — timeline-based NLE, separate tool category.
3. **Real-time / streaming video** — v1 is batch (full clip → video artifact).
4. **v2v (video-to-video transformation)** — phase-gated: style transfer, frame interpolation, extension. Deferred to V7+ after V2 (real-weights run) proves the pipeline.
5. **Pre-training base models** — same as Vision/Voice: download pinned weights, don't train.

### Policy layer

- Feature `adult` — off-by-default. Official registries are safe-only (only architecture of the policy, not content moderation).
- `VIDEO_ADULT_POLICY` gate (ADR-0149) — if `video { }` declaration has `policy: adult` and feature `adult` is not enabled → compile error.

### Supersession note for ADR-0122

ADR-0122 §"images before video" stated: "Video is a separate, future phase." This ADR lifts that boundary — Video is now an active pillar. ADR-0122's Vision-specific scope (images, ADR-0125 gates, real-weights runbook) remains unchanged. The supersession is partial: only the "video is deferred" clause is lifted.

## Consequences

- Video pillar follows the same staged architecture: V1 (skeleton) → V2 (dispatch) → V3 (real weights) → V4 (security) → V5 (persistence) → V6 (advanced).
- `video { }` declaration grammar mirrors `vision { }` / `voice { }` — name as STRING, fields required.
- Feature gate: `--features video` (implies `candle`), not in default build.
- Cross-pillar: shares `MODEL_WEIGHTS_UNSAFE` gate (Наряд №300), grammar `generative-declaration` pattern, registry license-field structure with Vision/Voice.
- ADR-0122 is annotated with a supersession note pointing to this ADR.
