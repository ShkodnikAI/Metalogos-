# ADR-0122: Vision pillar scope — inference-first over open weights, images before video

**Status:** Accepted
**Date:** 2026-09-07
**Naryad:** #209 (R0, research phase)
**Mandate:** owner directive of 2026-09-07 to begin implementation of
`Metalogos_Vision_Pillar_Plan.md`; this ADR formalizes that plan's scope decisions.

## Context

The owner requested a fourth generative capability pillar (images, later video) on top of
the existing `Reflex` pillar (ADR-0114..0121). The September 2026 state of open image
generation is consolidated around one architecture family: DiT/MMDiT backbones trained
with flow matching, distilled to few-step inference (8 NFE is normal, not an optimization).
Training such backbones from scratch is datacenter economics: pretraining corpora at
this scale do not exist in open access, and the best open training data is
non-commercially licensed.

At the same time, `candle` already carries full-pipeline precedents (SD 1.5/2.1, SDXL,
Wuerstchen): the "text encoder → denoiser → VAE decode" cycle in Rust has been walked
by someone. The realistic unit of value for Metalogos is the layer **above** the weights:
inference, adaptation, orchestration — and, as the differentiator, compiler-level
provenance and supply-chain security (ADR-0125).

## Decision

1. **Inference-first, no pretraining.** The pillar runs open checkpoints and may adapt
   them (LoRA-class, phase R7); it never pretrains base models. This mirrors Reflex's
   "real training, real accuracy — at local-machine scale" honesty contract (ADR-0115,
   ADR-0112).
2. **Images before video.** Image generation reaches reproducible quality first
   (phases R1–R6, наряды №210–215); video is a separate research cycle with its own ADR
   (**ADR-0126 reserved**). Video inference in pure Rust on consumer GPUs (3D VAE,
   temporal attention) is a distinct risk class and is not promised in this cycle.
3. **Explicit non-scope: NCII capability targets.** The language whose brand is
   security-by-design with compiler gates cannot ship a pillar whose primary community
   use-case is non-consensual synthetic imagery of real people. This is not enforced by
   prose but by mechanism: policy declarations and provenance gates (ADR-0125) make
   honest use explicit and dishonest use loud.
4. **Feature `vision`, off by default** (precedent: `candle`/ADR-0118), with a dedicated
   CI job (precedent: наряд №200's candle-CI job) and crosscheck exceptions referencing
   this ADR (precedent: n187).
5. **All pillar code lives in `src/vision/`, split from day one** under the existing
   module-size-guard discipline; it must not repeat the `diagrams.rs` god-file history.

## Consequences

- ADR-0123 (wedge choice), ADR-0124 (value/registry design), ADR-0125 (provenance gates)
  carry the technical detail; this ADR holds only scope.
- Reserves ADR numbers 0123–0126 for the Vision block; final numbers fixed in one pass
  with the Voice pillar plan (which reserves 0127+) to avoid parallel-ADR collisions —
  the same collision class as наряды 195–197 vs 200–203.
- Crosscheck exclusions for vision examples will grow in R1 and shrink only when
  examples genuinely pass on both backends, mirroring ADR-0121's staging discipline.
