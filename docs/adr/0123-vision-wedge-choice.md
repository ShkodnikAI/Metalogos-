# ADR-0123: Vision wedge — Z-Image-Turbo primary, FLUX.2 [klein] fallback

**Status:** Accepted
**Date:** 2026-09-07
**Naryad:** #209 (R0)
**Depends on:** ADR-0122 (scope), research report
`docs/research/naryad-209-vision-model-choice.md`

## Context

The wedge (first end-to-end target model) determines what R2 (text encoder) and R3
(FlowDiT + VAE decode) actually build against. Selection criteria, in weight order:
(1) checkpoint license — Apache-first, learned from the F5-TTS/Emilia case where code
is MIT but weights are CC-BY-NC because training data is; (2) quality/compute on
consumer GPUs; (3) reuse of already-implemented nn-blocks (`src/nn/`: attention/GQA/
RmsNorm/SwiGLU/transformer_block, KV-cache from наряд №193) — modern generators use
LLM-class text encoders, which is exactly the Reflex block zoo; (4) a reproducible
quantization path for consumer VRAM; (5) edit capabilities for phase R6.

## Decision

- **Primary wedge: Z-Image-Turbo.** Apache-2.0 (verified via search 2026-09-07), 6B,
  distilled to 8 NFE, consumer VRAM profiles 16/8/6 GB (BF16/FP8/GGUF — verified;
  community quantized checkpoints exist, e.g. lightx2v). Its text encoder is LLM-class —
  the second reuse of the Reflex nn stack (after sequence blocks, before speech).
- **Fallback and edit-first alternative: FLUX.2 [klein]** (Apache-2.0, 4B, verified
  publication, official FP8 path, native edit/multi-reference in the checkpoint).
- **Go/No-Go after R3 (наряд №212):** full path "text encoder → 8-step flow → VAE →
  PNG" must produce a correct image on one consumer machine within budgets fixed in
  the R3 report. On failure — pivot to klein, per the ADR-0106/0107 "recognize and
  turn" precedent, not silent phase-stretching.
- **Licensing is part of the system:** the model registry carries the license as a
  field; the compiler warns on a commercial-profile program loading an NC-licensed
  checkpoint (criterion fixed for all future wedge choices in this pillar).

## Fact-check pending (blocks R2/R3, not this decision)

Recorded explicitly so the decision is not mistaken for completed verification:

1. **RESOLVED (Наряд №211, 2026-09-07):** Z-Image text-encoder identity confirmed as
   Qwen3-4B (pure text decoder-only LLM, 36 layers, GQA 40/8, head_dim=64, SwiGLU,
   RmsNorm eps=1e-6, RoPE theta=1e6). See `docs/research/naryad-211-text-encoder-facts.md`
   for 3 independent sources and pinned config.json values. The R2 estimate "encoder =
   wiring existing blocks" is confirmed: Qwen3-4B is exactly the src/nn/ zoo (GQA +
   RmsNorm + SwiGLU) plus RoPE and QK-norm (implemented in src/vision/).
2. Availability of edit weights in the Z-Image family; if absent, R6's edit scenario
   routes to FLUX.2-klein.
3. Final license-text check on the HF model cards (no additional conditions).
4. Reference sampler implementation to treat as the golden source.

## Consequences

- Checkpoints are never committed to the repo or git history: the registry pins
  SHA-256 + source URL; first-run download goes through the SSRF-guarded HTTP client
  (наряд №130) with mandatory checksum verification (ADR-0125's supply-chain gate).
- R2/R3 estimates assume the fact-check list above resolves favorably; an adverse
  result on item 1 or 2 re-routes phases, not the wedge decision itself.
