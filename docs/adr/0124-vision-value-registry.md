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
   the ADR-0121 pattern (naryad №67/72 memory pattern, generalized), not shared
   `RuntimeContext` state.
3. **Builtins via SSOT.** `vision_generate`, `vision_edit`, `vision_export`,
   `vision_list`, `vision_load`/`vision_save` register in `BUILTIN_REGISTRY`
   (naryad №170 discipline). VM stubs fail loudly with an ADR reference until their
   parity stage lands (Reflex stub precedent). Expected family size: ~8–10 functions —
   a hard counter against builtins bloat.
4. **Declarative block.** The `vision { }` declaration grammar mirrors `reflex { }`
   (and the future `voice { }`): model id from `VisionRegistry`, pinned checkpoint,
   fixed seed (reproducibility by construction), VRAM profile (`fp16 | fp8 | gguf-q4`),
   policy field (ADR-0125). Three generative declarations sharing one grammatical
   shape invites a future "generative declaration" grammar unification — noted, not
   committed here.
5. **Feature gating.** `vision` cargo feature, off by default (ADR-0118 precedent);
   dedicated CI job (naryad №200's candle-job precedent); crosscheck exclusions carry
   ADR references (n187 precedent) and are removed per R4's actual parity, not in bulk.
6. **Persistence.** Checkpoints are never in the repo or git history; registry pins
   SHA-256 + source URL, downloads via the SSRF-guarded client (naryad №130) with
   mandatory checksum verification. LoRA adapters persist as SQLite BLOBs — the
   ADR-0116 pattern, not a new file format.

## Consequences

- R1 (naryad №210) delivers the skeleton in exactly this shape: feature, `src/vision/`,
  registry, SSOT stubs, VM stubs, CI job, crosscheck exceptions — measurable against
  the Reflex skeleton (naryads №177+), not against a new spec.
- Determinism contract: fixed seed ⇒ same image, regardless of backend, becomes an
  explicit crosscheck requirement (ADR-0121's determinism consequence, extended to
  image generation).

## Update (R2, naryad №211, 2026-09-07; fix-forward 2026-09-08)

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
  overlap — all scheduled for the R2 hotfix (naryad №230) before R3 starts.

## Update (R3, naryad №212, 2026-09-08)

- New `vision`-gated dependencies added (NOT in `default`/`full`):
  - `tokenizers` = 0.22 (optional, dep:tokenizers) — HF canonical BPE
    implementation. **Rationale:** Qwen2Tokenizer uses byte-level BPE with
    151,643-token base vocab + 119 special tokens + GPT-2-style regex
    pre-tokenizer. Hand-rolling this is high-risk for silent mis-tokenization
    (wrong regex → wrong token IDs for non-ASCII; wrong merges order → wrong
    IDs globally). The `tokenizers` crate is HF's verified reference and is
    deterministic (no RNG). Accepting this dep is the lowest-risk path;
    alternative was a hand-rolled BPE that would have required extensive
    test-fixture coverage to match HF byte-for-byte.
  - `image` = 0.25 (optional, dep:image, default-features=false,
    features=["png"]) — PNG encode of VAE decoder output. Minimal feature
    set (png only — no JPEG/GIF/BMP), keeps dep surface small.
- `vision` feature extended: `vision = ["candle", "dep:tokenizers", "dep:image"]`.
  Both new deps remain off-by-default (CI guard `vision ∉ default/full`
  continues to pass).
- New `vision`-gated modules in R3:
  - `src/vision/weights.rs` — `WeightsManifest` + sharded/single safetensors
    loaders with mandatory SHA-256 verification when manifest present.
  - `src/vision/tokenizer.rs` — thin wrapper around `tokenizers::Tokenizer`.
  - `src/vision/vae.rs` — `VaeDecoder` (flux-dev-style AutoencoderKL decoder).
  - `src/vision/dit.rs` — `ZImageTransformer` (30 DiT layers + 2 refiner +
    adaLN + axial RoPE + cap_embedder + t_embedder + patchify/unpatchify).
  - `src/vision/sampler.rs` — `FlowMatchEuler` scheduler + sampling loop.
- `TextEncoder::from_weights` added as a parallel construction path (R2's
  `new(config, seed)` and its golden SHA-256 records remain UNTOUCHED — R2
  contract invariant respected per §3.3 of naryad №212).
- **Weights policy** (per §3.1 of naryad №212):
  - Weights NEVER enter the repo or git history (no LFS, no fixtures).
  - SHA-256 manifest is the only weights-related artifact committed (template
    at `docs/research/naryad-212-weights-manifest.md`; executor fills SHAs
    after manual download).
  - SHA-256 verification is MANDATORY when a manifest is present; silent
    fallback to "trust the file" is FORBIDDEN.
  - Auto-download is R5 (ADR-0125) territory — R3 weights.rs performs ZERO
    network operations.
- **Two-tier test architecture** (per §0 of naryad №212):
  - CI-visible: tiny-fixture goldens (VaeDecoder, sampler) — bit-exact,
    no weights, no network. VAE tiny golden pinned after 3 bit-identical runs.
  - env-gated: real-weights tests run only when `MLOG_VISION_WEIGHTS_DIR` is
    set. Otherwise they SKIP loudly (NOT `#[ignore]`) — the §3 form of
    permitted unfinishedness.
- ADR-0116 (persistence pattern) is NOT yet extended to vision artifacts —
  that's R5/R6 territory (manifest, watermark, SQLite BLOB for vision_save/
  load). R3 delivers the inference path only.
