# ADR-0149: Video security gates — likeness consent, provenance, taint, adult policy

**Status:** Accepted
**Date:** 2026-09-14
**Naryad:** #305 (issue #386, P1/research/docs — VIDEO-A0-RESEARCH-ADR)
**Precedent:** ADR-0125 (Vision provenance gates), ADR-0145 (Voice security gates — same pattern), Наряд №300 (shared `MODEL_WEIGHTS_UNSAFE`), ADR-0136 (redact taint)

## Context

The Video pillar introduces a new generative surface (synthetic video) with heightened risks: deepfake/likeness (GDPR Art. 9 + BIPA), provenance (EU AI Act Art. 50), and adult content policy. Security gates follow the established pattern: static checks in `audit.rs`, runtime backstops in builtins.

## Decision

### Five gates

| check_id | Severity | Category | Description |
|---|---|---|---|
| `VIDEO_LIKENESS_NO_CONSENT` | Error | A | I2V with `kind: "likeness"` without a `LikenessToken` — compile-time error |
| `VIDEO_UNSIGNED_EXPORT` | Error | A | `video_export` on an unsigned artifact — compile-time error; opt-out `video_export_raw` → loud Warning |
| `UNTRUSTED_FRAME` | Warning | advisory | Frame from `http_get`/`form_data`/`file` passed to `video_generate` (I2V) without screen — advisory, not a gate |
| `VIDEO_ADULT_POLICY` | Error | A | `video { }` with `policy: adult` but feature `adult` not enabled — compile-time error |
| `MODEL_WEIGHTS_UNSAFE` | Error | A | Shared with Vision/Voice (Наряд №300) — covers `video_fetch_weights` via suffix convention |

### D1. LikenessToken — static check via audit.rs extension

`LikenessToken` is a cross-pillar consent type (image_edit → I2V → av_mux). The static check verifies that `video_generate` with an I2V image handle depicting a real person is preceded by `likeness_challenge` / `likeness_verify` in the same scope. Path-sensitive, mirrors `ConsentToken` (ADR-0145 D1) and `CANARY_LEAK` (Наряд №284).

### D2. Honest boundary of static analysis

Gates check **presence of tokens/declarations on call-sites**, not content of pixels. The audit cannot determine whether an image actually depicts a real person — it checks whether the author declared `kind: "likeness"` and provided a `LikenessToken`. This is the same honest boundary as ADR-0125/0145: "MVP detector, not adversarial guarantee."

### D3. VIDEO_UNSIGNED_EXPORT — mirrors VISION/AUDIO pattern

Video artifacts produced by `video_generate` are always signed (watermark + manifest). `video_export` on an unsigned artifact is a compile-time error. `video_export_raw` is an explicit opt-out with a loud Warning — same pattern as `vision_export_raw` (ADR-0125) and `audio_export_raw` (ADR-0145).

### D4. VIDEO_ADULT_POLICY

Feature `adult` is off-by-default. If `video { }` declaration has `policy: adult` and the `adult` feature is not enabled at compile time → `VIDEO_ADULT_POLICY` Error. This is a compile-time gate, not runtime — the code literally does not compile with adult policy unless the feature is explicitly enabled.

### D5. UNTRUSTED_FRAME — advisory taint

Frame from untrusted sources (`http_get`, `form_data`, `file`) passed to `video_generate` (I2V mode) is flagged with a Warning (advisory). The author should run `frame_screen` before I2V. Mirrors `UNTRUSTED_AUDIO` (ADR-0145) and `VISION_PROMPT_USER_INPUT` (Наряд №240).

### D6. Cross-pillar — LikenessToken as shared consent type

`LikenessToken` is the cross-pillar consent mechanism:
- **Vision** (image_edit): `vision_edit(handle, prompt)` on a photo of a real person → requires `LikenessToken`.
- **Video** (I2V): `video_generate(decl, prompt, image_handle)` animating a real person → requires `LikenessToken`.
- **av_mux**: combining audio (voice clone) + video (likeness) → both `ConsentToken` (Voice) and `LikenessToken` (Video) required.

The token is path-sensitive (then-branch of `if likeness.ok { ... }`), same mechanism as `ConsentToken`. The four-layer ritual (challenge-response, ASR/speaker-verify, ECAPA, ledger) from ADR-0145 is reused — `LikenessToken` extends it with face-verification (future, phase V6+).

## Consequences

- `VIDEO_LIKENESS_NO_CONSENT` is a Category-A compile-time error.
- `VIDEO_UNSIGNED_EXPORT` is a Category-A compile-time error; `VIDEO_UNSIGNED_EXPORT_RAW` is advisory.
- `VIDEO_ADULT_POLICY` is a compile-time feature gate.
- Cross-pillar: `MODEL_WEIGHTS_UNSAFE` shared (one gate, three pillars). `LikenessToken` shared between Vision image_edit and Video I2V.
- Anti-spoof claims bounded ("MVP detector, not adversarial guarantee") — documented in `docs/limitations.md` and `docs/threat-model.md`.
