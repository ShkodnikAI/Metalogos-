# Voice Pillar — Research A0 (Naryad #301, issue #369)

> **Date:** 2026-09-14. **Base:** main `b0dcd9a` (post-#300 merge). **Status:** done and verified by the owner.

## 1. Research goal

Determine the scope, wedge model, value-registry pattern, and security gates for the Voice pillar of Metalogos before any code implementation. Discipline: ADR before code (as with Reflex/Vision).

## 2. Existing infrastructure (reuse)

| Component | Source | Reuse for Voice |
|---|---|---|
| `euler_step` (ODE primitive) | `src/vision/sampler.rs` | Generalized, reusable for flow-matching TTS |
| `flow_match_euler_sample` | `src/vision/sampler.rs` | Hard-tied to `ZImageTransformer` — do NOT assume a shared denoiser extraction up front (rule of three; check in phase A2) |
| `encode_png` / `decode_png` | `src/vision/vae.rs` | Analogue: `encode_wav` / `decode_wav` (new, do not duplicate) |
| `VisionRegistry` / `VisionId` | `src/vision/mod.rs` (ADR-0124) | Template: `VoiceRegistry` / `AudioId` (ADR-0144) |
| `MODEL_WEIGHTS_UNSAFE` gate | `src/audit.rs` (#241/#300) | Generalized in #300 — `voice_fetch_weights` is covered by the suffix convention |
| `vision_fetch_weights` runtime | `src/builtins/vision.rs` (SSRF guard, allowlist, SHA pinning) | Template for the `voice_fetch_weights` runtime layers |
| `whisper_transcribe` | `src/builtins/llm.rs` (#279) | Reused for the ASR cross-check in the consent ritual |
| `secret()` / AES-256-GCM | Naryad #172 | Reused for encrypting the voiceprint at rest |
| `canary_insert` / `canary_check` | Naryad #284 | Template for the path-sensitive consent check |

## 3. Wedge table (September 2026 snapshot)

| Model | Code / Weights | Size | Cloning | Watermark | Status |
|---|---|---|---|---|---|
| **Chatterbox Multilingual V3** | MIT / MIT | 500M | ✅ | ✅ default | **Recommended** (ADR-0146) |
| **Kokoro-82M** | Apache-2.0 / Apache-2.0 | 82M | ❌ | ? | **A2 warm-up** (ADR-0146) |
| CosyVoice 3 | Apache-2.0 / Apache-2.0 | 0.5B | ✅ | ? | Future — ru/be languages not verified |
| NeuTTS Air | Apache-2.0 / Apache-2.0 | 748M | ✅ | ? | Future |
| GPT-SoVITS | MIT / MIT | ? | ✅ | ? | Future |
| F5-TTS | MIT / CC-BY-NC | ? | ? | ? | Research-profile only |
| Seed-VC | GPL-3.0 | ? | VC (not cloning) | ? | **Excluded** (GPL contamination) |

## 4. Linear/affine types and ConsentToken

The language has no linear/affine types (Option/Result were rejected by ADR-0106). ConsentToken is **not a type** but a static use-once check implemented as an extension of `src/audit.rs` (the same class of mechanism as `SECRET_LEAK`/`SQL_DYNAMIC`). Path-sensitive: `if consent.ok { voice_enroll(...) }` — enrollment only in the then-branch. Details: ADR-0145 D1.

## 5. Vocabulary boundary

The word "tripwire" is absent from ADR-0125 — the anti-spoof boundary must be phrased in the project's actual vocabulary: "MVP detector, not an adversarial guarantee" (ADR-0145 D3).

## 6. Licensing pattern of the niche

Permissive code + non-commercial weights — example: k2-fsa/OmniVoice "VoiceStudio": Apache-2.0 code, CC-BY-NC weights. For each candidate, check the weights license separately from the code license. Chatterbox and Kokoro are both fully permissive (MIT/MIT and Apache-2.0/Apache-2.0 respectively).

## 7. Cross-pillar summary

| Element | Vision (ADR-0122/0124/0125) | Voice (ADR-0143/0144/0145/0146) | Cross-pillar decision |
|---|---|---|---|
| `MODEL_WEIGHTS_UNSAFE` | #241 (vision_fetch_weights) | #300 (suffix `_fetch_weights`) | **Shared** — #300 generalized |
| `generative-declaration` grammar | `vision { }` (STRING name) | `voice { }` (STRING name) | Shared pattern, separate declarations |
| Registry license fields | Per-entry | Per-entry | Same structure (ADR-0144 D3) |
| Opaque handle | `Value::Vision(VisionId)` | `Value::Audio(AudioId)` | Separate variants, same pattern (ADR-0114) |
| Provenance gates | ADR-0125 (5 gates) | ADR-0145 (5 gates, 1 shared) | Separate gates, 1 shared (`MODEL_WEIGHTS_UNSAFE`) |
| Watermark | LSB (MLGV + 32-bit model hash) | TBD (AudioSeal-class — research backlog) | Different watermark class for audio |

No cross-pillar ADR is created — each pillar has its own scope/wedge/registry/gates ADR. The shared element (`MODEL_WEIGHTS_UNSAFE`) is already recorded in #300. If the Vision plan has not yet been adopted — a commitment is recorded to reconcile upon its adoption.

## 8. ADR summary

| ADR | Title | Status |
|---|---|---|
| 0143 | Voice scope — TTS, zero-shot cloning, voice-design | Accepted |
| 0144 | Voice value-registry — Value::Audio(AudioId) + VoiceRegistry | Accepted |
| 0145 | Voice security gates — consent, provenance, privacy, taint | Accepted |
| 0146 | Voice wedge — Chatterbox Multilingual V3 + Kokoro-82M | Accepted |
