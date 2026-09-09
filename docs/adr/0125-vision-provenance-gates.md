# ADR-0125: Provenance and supply-chain gates for generated media

**Status:** Accepted
**Date:** 2026-09-07
**Naryad:** #209 (R0)
**Depends on:** ADR-0122 (scope), ADR-0124 (value/registry)
**Precedents:** SECRET_LEAK (naryad №172), SSRF-guard (naryad №130), sandbox canonicalize
(naryad №131), positional taint logic (naryad №157), Category-A discipline (SQL_DYNAMIC)

## Context

diffusers, ComfyUI and similar tooling treat generated media as raw pixels: provenance
is an opt-in afterthought, model weights are downloaded from wherever the workflow
points, and nothing in the toolchain distinguishes a signed export from an unsigned
one. Metalogos's differentiator is exactly here: make honest use explicit and dishonest
use loud — at compile time, by construction. This is also the direct answer to the
non-scope decision in ADR-0122 (NCII capability targets are excluded not by prose but
by mechanism) and the prepared ground for the Voice pillar's equivalent gates.

## Decision

### Category-A gates (compile errors, by precedence)

| Gate | Rule | Precedent |
|---|---|---|
| `VISION_UNSIGNED_EXPORT` | `vision_export` without watermark + manifest — compile error. Opt-out is a separate explicit form `export_raw` with a loud audit warning | SECRET_LEAK / HTML_INJECTION |
| `MODEL_WEIGHTS_UNSAFE` | Loading non-safetensors (pickle RCE class), missing pinned SHA-256, or URL outside the allowlist — compile error. **Shared SSOT gate across generative pillars** (Voice reuses it verbatim) | SSRF-guard (№130), sandbox canonicalize (№131) |
| `VISION_POLICY_MISSING` | `vision { }` block without `policy:` — warning; policy validated statically | RATE_LIMIT / CSRF checks |

### Taint

`UserInput → prompt` is **allowed** (legitimate web-service case: prompt from
`form_data`/`json_body`) but is recorded into the manifest; the position-based wiring
follows naryad №157's logic. `Value::Vision` itself carries no taint (opaque handle,
ADR-0114 semantics).

### Provenance MVP (honest boundary)

- **Manifest JSON**: model-id + SHA, seed, prompt-hash, policy, timestamp. Written on
  every default export.
- **Watermark MVP**: steganographic LSB mark — mechanically verifiable in tests.
- **Research backlog, not promised**: robust watermarking surviving resize/JPEG;
  C2PA-compatible manifests (specification work, not cryptography) — phase 2.

## Consequences

- The agent (the language's primary user) **cannot accidentally** ship unsigned media
  or pull poisoned weights: security is a type, not a procedure.
- Wiring lands in `src/audit.rs` during R5 (naryad №214) with contract tests per gate;
  the gate names above are the SSOT for those tests and for the Voice pillar's
  `AUDIO_UNSIGNED_EXPORT`/`VOICE_CLONE_NO_CONSENT` extensions.
- The MVP watermark is detectable-by-us but not adversarially robust — the threat
  model states this plainly (ADR-0011 lesson: the threat model claims only what is
  actually delivered).
