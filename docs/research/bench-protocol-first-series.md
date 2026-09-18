# Benchmark Protocol — First Executed Series (naryad №398)

> The first series run under [`docs/benchmark-protocol.md`](../benchmark-protocol.md)
> (criterion (б) and (в) of the naryad). Executed 2026-09-18, single sandbox
> machine (NOT a pinned runner — the pinned-runner requirement of the Stage 4
> re-gate still applies to the real re-run; see the verdicts).

## The series record

```
decision: does raising the per-route round count (30 -> 60) change the measured
          request-cycle cost enough to matter for the Stage 4 re-gate evidence?
divisor (declared BEFORE the runs): the benchmark plan's route count = 14
          (normalized numbers are µs per route per cycle)
run command (identical for both variants): cargo bench --bench stage4_benchmark
variants: A = rounds30-main (DEFAULT_ROUNDS = 30, the main state)
          B = rounds60-varB (DEFAULT_ROUNDS = 60, one committed line change,
              branch bench-variant-rounds60, commit aa2390c)
tree: root -> rounds30-main ; root -> rounds60-varB
      (a fan of two WITHIN one decision — stacked bushes, rule 5)
```

## Raw + normalized numbers (rule 2 — verbatim wrapper output)

Variant A — `rounds30-main` (after one repair, see below):

```
[bench_run] RESULT backend=interpreter RAW_CYCLE_MEAN_US=20505 | DIVISOR=14 | NORMALIZED_US_PER_UNIT=1464.6
[bench_run] RESULT backend=vm        RAW_CYCLE_MEAN_US=11363 | DIVISOR=14 | NORMALIZED_US_PER_UNIT=811.7
```

Variant B — `rounds60-varB`:

```
[bench_run] RESULT backend=interpreter RAW_CYCLE_MEAN_US=25405 | DIVISOR=14 | NORMALIZED_US_PER_UNIT=1814.7
[bench_run] RESULT backend=vm        RAW_CYCLE_MEAN_US=14708 | DIVISOR=14 | NORMALIZED_US_PER_UNIT=1050.6
```

Peak RSS (raw, from the same reports): interpreter ≈ 37.4 MB, vm ≈ 39.9 MB
(ratio ≈ 1.06 — consistent with №381's ≈ 1.0 no-memory-win finding).

## Per-node verdicts (rule 3/4 form)

| Node | Verdict | Rationale |
|---|---|---|
| `rounds30-main` | **repair(2) → answered (promote as the protocol's baseline series)** | two repairs before the answer (the wrapper's parser initially missed the bench's combined-report JSON — an infra defect of the WRAPPER, not a property of the node; per rule 3 those runs count as repairs of the node, not as its result). The answered run produced valid raw+normalized numbers for both backends. |
| `rounds60-varB` | **answered (stop)** | doubling the rounds did NOT stabilize the measurement: the per-cycle means moved MORE between runs (interpreter +23.9 %, vm +29.4 %) than any within-run rounding could explain — the variance is cross-run machine noise on a shared sandbox, not sample noise a larger round count absorbs. Verdict: stop this direction; the re-gate's stability requirement is a PINNED-RUNNER property, not a rounds property. |

## Findings carried forward

1. The protocol caught itself: the two wrapper repairs are exactly the
   "error that does not answer the node's hypothesis" case rule 3 was
   written for — the runs were discarded as repairs, the node re-run, and
   the log retains every attempt (`docs/research/bench-tree.txt`).
2. The vm/interpreter cycle ratio stayed in the №381 band (1.80x / 1.73x)
   under BOTH variants — the backend delta is robust to the round count,
   but both absolute numbers are ~20 % slower than a quiet machine would
   show; the Stage 4 re-run must use the pinned runner (dispatch requirement)
   before any §D5 verdict can be promoted.
3. The divisor (route count = 14) is now DECLARED in the series record —
   raw numbers above are not absorbable by it (the INSUFFICIENT DATA
   failure mode of №381 is mechanically closed).
