# ADR-0153: Video DiT text path wired — prompt embedding genuinely conditions the denoiser

**Status:** Accepted
**Date:** 2026-09-14
**Naryad:** owner directive 2026-09-14 (video pillar no-stubs re-audit; pre-wave cleanup order)
**Precedent:** ADR-0151 D7 (honest boundaries — the boundary being amended), №310 (tiny-tensor template), №294 (production-weights No-Go class)

## Context

The owner ordered a re-audit of the Video pillar for placeholders ("заглушки"): none may remain. The re-audit found the pillar substantively real (DiT/VAE/Euler/anchors/mux/export/taint all implemented; `video_fetch_weights` is the formally recorded loud No-Go), with one residual semi-stub: `VideoDit::forward(x, t, _text)` accepted the prompt-derived text embedding and **silently dropped it**. Prompt conditioning operated only through seed derivation (different prompt → different seed → different tiny init + latent init) and the manifest `prompt_hash`. ADR-0151 D7 recorded this as "the DiT text path is the №310 API surface (its use is a V4+ item)" — a recorded boundary, but one that leaves a dead parameter in the forward signature: the model receives prompt data and discards it. That is exactly the shape of a hidden stub the owner's standard forbids.

## Decision

### D1. The text path is wired (real conditioning, tiny scale)

`VideoDit` gains a seeded text projection `Linear(text_dim → hidden_dim)` (`text_embed`, weight stream `seed+20`, bias `seed+21` — verified free of overlap with the existing streams: patch `seed+0/1`, time `seed+1/2`, output `seed+999/1000`, layers `seed+10..15, +110..115`). `forward(x, t, text)` now:

1. validates the embedding shape loudly — `[B, text_dim]` with `B` equal to the latent batch; mismatch is a `candle_core::Error` (no silent reinterpretation);
2. projects the embedding to `[B, dim]`, reshapes to `[B, 1, 1, dim]` and broadcast-adds it to every token of `[B, T, S, dim]` after the time-embedding add, before the DiT layers.

Prompt conditioning therefore operates through **two real paths**: seed derivation (latent init + tiny init) and the projected embedding inside every denoising step. The embedding itself remains `hash_embedding(seed)` — a deterministic hash-derived vector, NOT a learned text encoder; the learned-encoder boundary is restated in D2.

`VideoDitConfig` gains an additive field `text_dim: usize` (default 64, matching `hash_embedding` and all existing call sites, which use `VideoDitConfig::default()`).

### D2. Boundary restated (no change to the No-Go)

What changed: the DiT genuinely consumes the embedding it is given. What did NOT change: the embedding is a hash-derived deterministic vector, not a learned text encoder (umT5-class for Wan 2.2). Production text encoders remain under the №294-class No-Go with `video_fetch_weights` as the loud error behind `MODEL_WEIGHTS_UNSAFE`. `docs/limitations.md` carries the row.

### D3. Seed-stream and determinism contracts preserved

- No stream overlap (allocation above); `new_tiny` remains fully seed-deterministic.
- No test pins absolute video hashes (re-audit confirmed: golden.rs and all video tests assert shape/determinism/anchor/provenance contracts, not byte values), so the changed forward does not break any pinned expectation. All pipeline contracts (two-anchor exactness, endpoint preservation, seed determinism, mux/export byte-determinism) are unaffected by construction: anchors are re-pinned on the latent after each Euler step regardless of model internals.

### D4. "Stub" wording hygiene

The word "stub" is reserved for formally recorded loud-error boundaries (`builtin_video_fetch_weights_stub`). Test fixtures that pass a zero embedding are labeled "fixture", not "stub" (`sampler.rs` e2e test); the №307 historical note in `src/video/mod.rs` is marked as superseded by the №309 real pipeline. No behavioral change — documentation-only, so a re-audit for "заглушки" reads zero false positives.

## Consequences

- `VideoDit::forward` consumes `text`; the `_text` dead parameter is gone. New tests: text conditioning changes the velocity field; same text → same output (determinism); shape mismatch is a loud error; `hash_embedding` is seed-sensitive.
- All video outputs of the same seed differ from pre-ADR-0153 builds (the conditioning add is real) — acceptable: no absolute hashes are pinned anywhere; determinism is per-build.
- ADR-0151 D7's clause "its use is a V4+ item" is amended by this ADR: the wiring landed at tiny scale; the V4+ item that remains is the learned text encoder (No-Go class), not the wiring.
- REFERENCE.md unchanged (no new builtins); README counts change only for ADRs (144 → 145).
