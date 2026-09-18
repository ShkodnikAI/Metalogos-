# ADR-0145: Voice security gates — consent, provenance, privacy, taint

**Status:** Accepted
**Date:** 2026-09-14
**Naryad:** #301 (issue #369, P1/research/docs — VOICE_A0_ADR)
**Precedent:** ADR-0125 (Vision provenance gates — same pattern), ADR-0136 (redact taint-sanitizer), Наряд №284 (canary), Наряд №300 (shared MODEL_WEIGHTS_UNSAFE)

## Context

The Voice pillar introduces a new biometric surface (voiceprints) and a new generative surface (synthetic audio). Both require security gates: consent for cloning, provenance for export, privacy for voiceprints, and taint for untrusted audio input.

## Decision

### Five gates

| check_id | Severity | Category | Description |
|---|---|---|---|
| `VOICE_CLONE_NO_CONSENT` | Error | A | `voice_enroll` with `kind: "cloned"` without a `ConsentToken` — compile-time error |
| `AUDIO_UNSIGNED_EXPORT` | Error | A | `audio_export` on an unsigned artifact — compile-time error; opt-out `export_raw` → loud Warning |
| `VOICEPRINT_OPAQUE` | Error | A | Voiceprint value used in a non-voice builtin (print, http_post, etc.) — leak prevention |
| `UNTRUSTED_AUDIO` | Warning | advisory | Audio from `http_get`/`form_data`/`file` passed to `voice_enroll` without spoof-screen — advisory, not a gate |
| `MODEL_WEIGHTS_UNSAFE` | Error | A | Shared with Vision (Наряд №300) — same static check, covers `voice_fetch_weights` via suffix convention |

### D1. ConsentToken — static check via audit.rs extension

ConsentToken is not a new type or move-semantics construct — it's a static audit check that `voice_enroll` with `kind: "cloned"` is preceded by a `consent_challenge` / `consent_verify` pair in the same scope. The check is path-sensitive (then-branch of `if consent.ok { ... }`), mirroring the `CANARY_LEAK` pattern (Наряд №284).

### D2. Four-layer consent ritual

1. **Challenge-response**: nonce-phrase spoken by the enrolllee in the declaration's `language:` field.
2. **ASR verification**: the nonce-phrase is transcribed and compared (reuses existing `whisper_transcribe` builtin, Наряд №279).
3. **Speaker verification**: ECAPA-class embedding cosine similarity (Наряд #371).
4. **SQLite ledger**: `hash(voiceprint, nonce, date, model)` stored in every `VoiceManifest`.

### D3. Anti-spoof — MVP detector, not adversarial guarantee

Following the vocabulary of ADR-0125 (not "tripwire" — "MVP detector, not adversarial guarantee"). The anti-spoof layer is:
- Watermark + manifest by construction (EU AI Act Art. 50).
- Robust watermarking (AudioSeal-class) — research backlog, not v1.
- The four-layer consent ritual is the primary defense; spoofing defeats are documented limitations, not claims.

### D4. AUDIO_UNSIGNED_EXPORT — mirrors VISION_UNSIGNED_EXPORT

Audio artifacts produced by `voice_generate` are always signed (LSB watermark + manifest). `audio_export` on an unsigned artifact is a compile-time error. `audio_export_raw` is an explicit opt-out with a loud Warning — same pattern as `vision_export_raw` (ADR-0125).

### D5. VOICEPRINT_OPAQUE — leak prevention

Voiceprint values (`Value::Voiceprint` if introduced, or checked at the `voice_enroll` call site) must not flow to non-voice sinks (`print`, `http_post`, `write_file`, `call_llm`). The static check mirrors `SECRET_LEAK` — if a voiceprint handle reaches a sink, it's an Error.

### D6. UNTRUSTED_AUDIO — advisory taint

Audio from untrusted sources (`http_get`, `form_data`, `file`) passed to `voice_enroll` is flagged with a Warning (advisory, not a gate). The author should run `audio_screen` (spoof-detection MVP) before enrollment. This mirrors `VISION_PROMPT_USER_INPUT` (Наряд №240) — advisory taint, not a compile error.

## Consequences

- `VOICE_CLONE_NO_CONSENT` is a Category-A compile-time error (via `audit_category_a` → semantic №98 promotion).
- `AUDIO_UNSIGNED_EXPORT` is a Category-A compile-time error; `AUDIO_UNSIGNED_EXPORT_RAW` is advisory.
- ConsentToken is a static check, not a type-system construct — consistent with ADR-0106 (no Option/Result).
- Anti-spoof claims are bounded ("MVP detector", not "adversarial guarantee") — the boundary is documented in `docs/limitations.md` and `docs/threat-model.md`.
- Cross-pillar: `MODEL_WEIGHTS_UNSAFE` is shared with Vision (one gate, №300 generalized it). No duplication.
