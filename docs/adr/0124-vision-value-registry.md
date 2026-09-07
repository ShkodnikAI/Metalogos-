# ADR-0124: `Value::Vision` as opaque handle + `VisionRegistry` — Reflex patterns, VM-owned state

**Status:** Accepted
**Date:** 2026-09-07
**Naryad:** #209 (R0)
**Depends on:** ADR-0122 (scope), ADR-0114 (opaque handles), ADR-0121 (VM-owned state),
ADR-0116 (persistence pattern), ADR-0118 (feature gating)

## Context

The Vision pillar needs a language-level representation for generated images (and later
video) that keeps tensors out of `Value`, keeps the registry as the single source of
truth for models, and reaches TW/VM parity without inventing a new architecture.
Every one of these questions has already been answered by Reflex; this ADR records the
reapplication, not a new design.

## Decision

1. **Opaque handles, not tensors.** `Value::Vision(VisionId)` and (phase V)
   `Value::Video(VideoId)` — `Display` renders `[Vision#N]`. Weights/tensors never
   enter `Value` (precedent: `Value::Reflex`/`ReflexId`, ADR-0114).
2. **Registry owns state.** `VisionRegistry` owns tensors, checkpoints and adapters,
   exactly as `ReflexRegistry` does. For VM parity, `Vm` owns its **own** registry
   instance and routes builtins to the same underlying `src/vision/*` functions —
   the ADR-0121 pattern (naряд №67/72 memory pattern, generalized), not shared
   `RuntimeContext` state.
3. **Builtins via SSOT.** `vision_generate`, `vision_edit`, `vision_export`,
   `vision_list`, `vision_load`/`vision_save` register in `BUILTIN_REGISTRY`
   (naряд №170 discipline). VM stubs fail loudly with an ADR reference until their
   parity stage lands (Reflex stub precedent). Expected family size: ~8–10 functions —
   a hard counter against builtins bloat.
4. **Declarative block.** The `vision { }` declaration grammar mirrors `reflex { }`
   (and the future `voice { }`): model id from `VisionRegistry`, pinned checkpoint,
   fixed seed (reproducibility by construction), VRAM profile (`fp16 | fp8 | gguf-q4`),
   policy field (ADR-0125). Three generative declarations sharing one grammatical
   shape invites a future "generative declaration" grammar unification — noted, not
   committed here.
5. **Feature gating.** `vision` cargo feature, off by default (ADR-0118 precedent);
   dedicated CI job (naряд №200's candle-job precedent); crosscheck exclusions carry
   ADR references (n187 precedent) and are removed per R4's actual parity, not in bulk.
6. **Persistence.** Checkpoints are never in the repo or git history; registry pins
   SHA-256 + source URL, downloads via the SSRF-guarded client (naряд №130) with
   mandatory checksum verification. LoRA adapters persist as SQLite BLOBs — the
   ADR-0116 pattern, not a new file format.

## Consequences

- R1 (naряд №210) delivers the skeleton in exactly this shape: feature, `src/vision/`,
  registry, SSOT stubs, VM stubs, CI job, crosscheck exceptions — measurable against
  the Reflex skeleton (naряды №177+), not against a new spec.
- Determinism contract: fixed seed ⇒ same image, regardless of backend, becomes an
  explicit crosscheck requirement (ADR-0121's determinism consequence, extended to
  image generation).

## Update (R2, наряд №211, 2026-09-07; fix-forward 2026-09-08)

- The `vision` feature now **implies `candle`**: `vision = ["dep:candle-core",
  "dep:candle-nn"]`. The R2 text encoder requires tensor operations. ADR-0118
  is not violated — both features remain off-by-default, `default`/`full` are
  unchanged, and the CI guard continues to enforce `vision ∉ default/full`.
- The R1 skeleton tests (naryad_210) do not depend on candle and keep passing
  unchanged under the new feature dependency.
- `src/vision/text_encoder.rs` (Qwen3-architecture text encoder on Reflex
  primitives) is feature-gated behind `vision`. Known debts recorded loudly in
  the module docs and the ADR-0122 map: golden SHA-256 records not yet pinned,
  local PRNG copy divergent from the `src/nn` SSOT contract, seed-stream
  overlap — all scheduled for the R2 hotfix (наряд №230) before R3 starts.
