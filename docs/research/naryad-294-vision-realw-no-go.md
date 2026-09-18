# Naryad #294 (issue #357) — VISION_REALW — formal No-Go

> **Date:** 2026-09-14 (UTC+8).
> **Verdict:** **No-Go** — executing runbook #237 is impossible in the current environment; the hardware gate is not passed.
> **Review date:** upon hardware allocation (≥64 GB RAM, ≥40 GB disk, a GPU environment). Open item outside the repo — the owner's decision on the allocation.
> **Owner decision 2026-09-14** (in the issue #357 body): "conditional Go — if a machine is allocated, the run per the preflight of runbook #237 is executed verbatim; without a machine — a formal No-Go with an explicit review date. The audit's 'tiny model' option is rejected".

## Context

Runbook #237 (`docs/research/naryad-237-real-weights-runbook.md`, 226 lines) is ready to be executed verbatim:
- The weights manifest is pinned and verifiable (16 files, 32.85 GB; SHA check against HF LFS oids; post-download refusal on mismatch; `--dry-run`, `--only`).
- `tools/fetch_vision_weights.sh` is ready.
- The Go/No-Go criteria are stated verbatim.
- Preflight: ≥40 GB disk, ≥64 GB RAM for the F32 policy, ~62 GB peak.
- The code is GO-ready after #236/#243 (zero diff in `src/**`).
- The env-gated tests of #212/#243 SKIP loudly when `MLOG_VISION_WEIGHTS_DIR` is unset — this is working behavior, not a blocker.

## Preflight check (2026-09-14, agent container)

| Runbook requirement | Actual availability | Status |
|---|---|---|
| ≥64 GB RAM | 4.1 GB total (3.2 GB free) | ❌ FAIL (64 GB needed) |
| ≥40 GB disk | 9.9 GB total (7.1 GB free) | ❌ FAIL (40 GB needed; weights alone — 32.85 GB) |
| GPU environment (CUDA, for candle) | /dev/nvidia* does not exist | ❌ FAIL (no GPU) |
| `MLOG_VISION_WEIGHTS_DIR` | unset | ⏸ SKIP (env-gated; working behavior) |

**Preflight result**: 3 of 3 hardware requirements NOT passed. Runbook #237 §preflight blocks execution.

## No-Go reasons

1. **RAM**: 4.1 GB vs the required 64 GB (the F32 policy, 62 GB peak). 16× below the minimum. Without a GPU + candle offload, F16 will not come up either (~32 GB RAM needed).
2. **Disk**: 9.9 GB total vs the required 40 GB+ (weights alone — 32.85 GB). 4× below the minimum. cargo artifacts + the repo clone already take ~2 GB.
3. **GPU**: no `/dev/nvidia*` devices. Candle can run on CPU, but Z-Image-Turbo (4B parameters) inference on CPU without substantial RAM (see item 1) is unacceptably slow (hours per step).

## What was NOT done (because it is impossible without hardware)

- ❌ Downloading the 16 manifest files (32.85 GB) — `tools/fetch_vision_weights.sh` was not run (nothing to download into 7 GB).
- ❌ SHA verification of the downloaded files against HF LFS oids — there are no downloaded files.
- ❌ The env-gated tests of #212 (`naryad_212_wedge_e2e`) / #243 (`mlog_vision_edit_export_e2e`) with `MLOG_VISION_WEIGHTS_DIR` — SKIP loudly (expected behavior; not executed in substance).
- ❌ Capture: golden PNG, timings, determinism (a repeat run — byte-for-byte).
- ❌ Lifting PARKED in ADR-0122 / README / threat-model — the Parked status remains (No-Go → Parked is not lifted).

## What is available without hardware

- ✅ The vision pillar code — feature-gated (`--features vision`, implies `candle`), not enabled in the default build. CI on PR: `vision-tests (blocking)` runs only the tiny-golden tests (no real weights). This is a working state.
- ✅ Runbook, manifest, `tools/fetch_vision_weights.sh` — ready for execution once hardware is allocated.
- ✅ Tiny-golden pinned models (#231, #232) — run on tiny pinned weights (megabytes, not gigabytes), CI green. This does not replace the real-weights run, but confirms architectural correctness.

## Verdict

**No-Go** — with an explicit review date: upon hardware allocation (≥64 GB RAM, ≥40 GB disk, a GPU environment) the run is executed verbatim per runbook #237. Without hardware — the pillar stays PARKED (as it has been since the owner's decision of 2026-09-09).

## What changes in the repo

Minimal docs-only changes (zero diff in `src/**`):
1. **ADR-0122 map row #237** — add an entry for the No-Go verdict of 2026-09-14 + the review date (explicit, not "TBD"). The Parked status remains.
2. **README** — the `Weights run parked` line is updated: a link to this report + the explicit date of the No-Go verdict.
3. **docs/threat-model.md** — wherever PARKED is mentioned — a link to this report.
4. **CHANGELOG** — an entry for #294.

## Related

- Issue #357 (naryad).
- Runbook: `docs/research/naryad-237-real-weights-runbook.md`.
- ADR-0122 (Vision pillar scope — map row #237).
- Naryad #237 (runbook prep — merged PR #235, 2026-09-09).
- Naryads #212/#243 (env-gated tests, SKIP loudly when `MLOG_VISION_WEIGHTS_DIR` is unset — working behavior, not a blocker).
- Owner decision 2026-09-14 (in the issue #357 body): "conditional Go; without a machine — a formal No-Go with an explicit review date. The audit's 'tiny model' option is rejected".
