# Video Pillar — Research A0 (Naryad #305, issue #386)

> **Date:** 2026-09-14. **Base:** main `420384e`. **Status:** done.

## 1. Research goal

Determine the scope, wedge model, value-registry pattern, security gates, and cross-pillar summary for the Video pillar of Metalogos before any code implementation.

## 2. Existing infrastructure (reuse)

| Component | Source | Reuse for Video |
|---|---|---|
| `euler_step` (ODE primitive) | `src/vision/sampler.rs` | Generalized, reusable for flow-matching video |
| `flow_match_euler_sample` | `src/vision/sampler.rs` | Hard-tied to `ZImageTransformer` — do NOT assume a shared denoiser extraction (rule of three; decision AFTER V2) |
| `VisionRegistry` / `VisionId` | `src/vision/mod.rs` (ADR-0124) | Template: `VideoRegistry` / `VideoId` (ADR-0148) |
| `VoiceRegistry` / `AudioId` | `src/voice/mod.rs` (ADR-0144) | Template: opaque handle + encrypted store |
| `MODEL_WEIGHTS_UNSAFE` gate | `src/audit.rs` (#241/#300) | Generalized in #300 — `video_fetch_weights` is covered by the suffix convention automatically |
| `vision_fetch_weights` runtime | `src/builtins/vision.rs` | Template for the `video_fetch_weights` runtime layers |
| `VisionManifest` | `src/vision/provenance.rs` | Template: `VideoManifest` (ADR-0148) |
| `secret()` / AES-256-GCM | Naryad #172 | Reused if video artifacts need encryption |
| `canary_insert` / `canary_check` | Naryad #284 | Template for the path-sensitive consent check |

## 3. Wedge table (September 2026 snapshot)

| Model | Code / Weights | Size | T2V | I2V | Status |
|---|---|---|---|---|---|
| **Wan 2.2 (TI2V-5B)** | Apache-2.0 / Apache-2.0 | 5B | ✅ | ✅ | **Recommended** (ADR-0150) |
| **CogVideoX-1.5 (5B)** | Apache-2.0 / Apache-2.0 (Hunyuan community) | 5B | ✅ | ✅ | **V2 warm-up** (ADR-0150) |
| HunyuanVideo 1.5 | Custom community / Custom community | 13B | ✅ | ❌ | **Excluded** (license) |
| LTX-2.5 | Apache-2.0 / ARR-limited | 2B | ✅ | ✅ | **Excluded** (ARR restriction) |
| MiniMax H3 Open | Apache-2.0 / Apache-2.0 | ? | ✅ | ? | Future |
| Mochi 1 / Open-Sora 2.0 | Apache-2.0 / Apache-2.0 | 10B | ✅ | ❌ | Future (T2V only) |

## 4. Cross-pillar summary — three pillars

| Element | Vision (ADR-0122/0124/0125) | Voice (ADR-0143/0144/0145/0146) | Video (ADR-0147/0148/0149/0150) | Cross-pillar |
|---|---|---|---|---|
| `MODEL_WEIGHTS_UNSAFE` | #241 | #300 (suffix) | #300 (suffix, auto) | **Shared** — one gate, three pillars |
| Opaque handle | `Value::Vision(VisionId)` | `Value::Audio/Voice` | `Value::Video(VideoId)` | Same pattern (ADR-0114) |
| Provenance gates | ADR-0125 (5 gates) | ADR-0145 (5 gates) | ADR-0149 (5 gates) | Separate gates, 1 shared (`MODEL_WEIGHTS_UNSAFE`) |
| Consent | — | `ConsentToken` (voice clone) | `LikenessToken` (I2V likeness) | Cross-pillar: LikenessToken shared Vision↔Video |
| Watermark | LSB (MLGV) | TBD (AudioSeal-class) | TBD (VideoSeal-class) | Different watermark class per medium |
| `generative-declaration` | `vision { }` | `voice { }` | `video { }` | Shared pattern, separate declarations |
| Registry license fields | Per-entry | Per-entry | Per-entry | Same structure |
| General denoiser | `euler_step` (vision-specific) | Not extracted (Voice-A2 not started) | Not assumed (rule of three) | Decision AFTER V2, not before |

### Rule of three for general denoiser

`euler_step` is a shared ODE primitive. The full denoising contour (`flow_match_euler_sample`) is vision-specific. Voice-A2 (general denoiser extraction) has not been published. Video's spatiotemporal DiT is a third contour. **Decision: do NOT assume a general denoiser exists** — extract only when all three contours are proven (rule of three). This is deferred to post-V2.

## 5. Supersession note for ADR-0122

ADR-0122 §"images before video": "Video is a separate, future phase." → **Superseded by ADR-0147**: Video is now an active pillar. The supersession is partial — only the "video is deferred" clause is lifted; Vision-specific scope remains unchanged.

## 6. ADR summary

| ADR | Title | Status |
|---|---|---|
| 0147 | Video scope — T2V/I2V primary, v2v phase-gated | Accepted |
| 0148 | Video value-registry — Value::Video(VideoId) + VideoRegistry | Accepted |
| 0149 | Video security gates — likeness, provenance, taint, adult | Accepted |
| 0150 | Video wedge — Wan 2.2 TI2V-5B + CogVideoX-1.5 5B | Accepted |

## 7. Disk hygiene

- Before: 13M repo (no target/, no weights).
- After: 13M — an ADR-only naryad, no heavy files.
- Downloaded cards/licenses: none — the fact-check was done against HuggingFace model cards via web (text in this document).
