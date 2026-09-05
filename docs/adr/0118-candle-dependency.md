# ADR-0118: `candle` as the tensor/autograd dependency for architecture blocks

**Status:** Accepted
**Date:** 2026-09-05
**Naryad:** #183 (blocks on this ADR)
**Pillar:** `Reflex` (stage 6+ — architecture blocks, separate owner decision from the initial rollout)
**Relates to:** naряды №175/176 (research reports — this ADR formalizes their conclusion into a binding decision)

## Context

Наряд №175 measured `candle` against `burn` directly (build time,
binary size, CPU epoch timing, dependency footprint) and recommended
`candle`. Наряд №176 confirmed `candle-transformers` ships a real,
working reference implementation (Llama) to follow as a template, and
independently verified deterministic forward-pass behavior on both
candidates. Neither наряд committed the project to a dependency —
both were explicitly scoped as research, no code, per the owner's own
instruction at the time.

This ADR is that commitment, now that the owner has explicitly
authorized proceeding to architecture blocks.

## Decision

`candle-core` + `candle-nn` are accepted as direct dependencies,
scoped to the architecture-block feature set only — the initial
`Reflex` rollout (naряды №177–182, `Dense`/`sigmoid`/`softmax`/SGD)
remains dependency-free by design (`ADR-0114`) and is **not**
retroactively migrated onto `candle`. The two coexist: simple
classifier/regressor heads keep the наряд №177-era hand-rolled path
(cheap, no dependency, already shipped); attention/transformer blocks
use `candle`.

Version pin: `candle-core = "0.11"` (наряд №175's measured version,
re-verify current `crates.io` state before naряд №183 vendors it —
do not assume six-week-old numbers are still current).

## Consequences

- The project's dependency-free claim for the base `Reflex` pillar
  stays true; only the architecture-block extension carries the new
  dependency, and that scope is documented as such everywhere the
  pillar is described (README, `docs/threat-model.md` if relevant to
  supply-chain surface).
- `cargo-audit` (already blocking in CI since наряд №137) now also
  covers `candle`'s transitive dependency tree — a real increase in
  supply-chain surface, accepted knowingly, not accidentally.
- Naряд №176 already confirmed candle's forward-pass determinism
  under a fixed seed — that finding is now load-bearing for this
  pillar's `seed`-determinism guarantee (наряд №177's contract),
  not merely informational.
