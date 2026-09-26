# The Crate Split Roadmap (№472, decision 2-B)

The audit 25.09 §4.2 finding: 128k+ LOC in one crate (×2.1 to v0.17) —
the interpreter, the compiler/VM, the server, MCP, the SQLite memory,
SVG/diagrams, PDF, email/calendar, the NN module, the image generation,
video, voice, TimesFM, embodied, the Ed25519 ledger. The language is
"core + std; everything else — libraries". The full split is **0.27+**
(after the completed dedup №466: "the split cuts the already-deduplicated");
this document is the roadmap the owner's decision 2-B fixed.

## The stage that already happened (this document's baseline, №472)

- **The physical core→media ban is a CI fact**: `scripts/ci/core_media_gate.py`
  (the `core-media-gate` blocking job) enforces that the core files
  (parser, ast, compiler, bytecode, the interpreter tree, the VM, the
  pool, audit, semantic) touch the media modules ONLY through the
  handle/registry tier (`MediaStore`/`MediaHandle`/`MediaKind`,
  `VisionRegistry`/`VisionId`, `VideoId`, `VoiceId`/`AudioId` and the
  media-metadata helpers). The heavy tier (encoders, VAEs, samplers,
  the pipelines, the weights machinery) is feature-gated inside the
  media modules and never imported by core.
- **The feature boundary**: the generative pillars (`vision`, `voice`,
  `video`, `candle`) are off by default; `cargo build
  --no-default-features` is green and enforced as a blocking CI job —
  the core builds without media.
- **The semantics did not change**: the media-handle behavior, the
  registry shapes and the builtin surfaces are exactly as before
  (the boundary rule of №472).

## 0.27 — the first physical split: `metalogos-core` + `metalogos-reflex`

- `metalogos-core`: the language — parser, ast, compiler, bytecode, the
  TW/VM backends (deduplicated by №466 by then), the runtime values,
  audit/semantic lanes, the builtin registry shell. No media, no server,
  no NN.
- `metalogos-reflex`: the generative contour's home (the №464 ADR-0178
  boundary made physical) — the NN module, the generative-model
  machinery, the candle/tokenizers surfaces. Depends on core for the
  value/handle types (or on a tiny `metalogos-values` crate if the
  dependency direction demands it — the split PR decides with the
  owner's gate).
- **Lockstep versioning**: one version number and one CHANGELOG across
  all crates — the release overhead must not eat the split's gain
  (the decision 2-B wording). Crates version-bump together; the
  CHANGELOG stays single.
- The server/memory/media/domain crates follow **0.27+**, by stability:
  each new crate takes its media pillars out of the feature-gated tier
  only after its types stabilized (the decision 3-A type line first —
  the enum Type → the typed signatures → the API boundaries).

## The rules the split inherits (from this naryad)

1. The core→media import ban survives the split verbatim — it becomes
   a Cargo dependency edge (`metalogos-core` must not depend on any
   media crate; the CI gate switches from the import scan to the
   dependency graph check, the allowlist concept stays).
2. The `Closes`-semantics, the gates and the baselines move with their
   code: each gate script stays in the repo's `scripts/ci/` (the
   supply-chain perimeter of №471) until the split, then follows its
   crate.
3. No crate split without the owner's gate — the roadmap is the plan,
   each step is a wave-sized decision (the canon §13 discipline).
