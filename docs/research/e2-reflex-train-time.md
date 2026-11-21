# E2 — the `reflex_train` time (№988, ADR-0200 §5)

- Status: measured (the round-2 E-line of the provable tier-lowering plan)
- Date: 2026-11-21
- Naryad: №988 (issue #2022, dispatch wave В99 — W-AC, №AC1)
- Harness: `tests/research_e1_e4.rs` (env-guarded — the tests pass as loud no-ops in CI and measure only under METALOGOS_RESEARCH_E1E4=1; the documented
  invocation `scripts/research/run_e1_e4.sh`); the machine-readable
  companion is `e2-reflex-train-time.json`, the raw run log is
  `e1_e4_raw_run.txt`.

## The question

`ADR-0200 §5`, the E2 row: the wall-clock time of `reflex_train` per
round/epoch on the same fixture, the spread over 3 runs (never one), the
machine facts recorded. E2 is a dependency of W5 (the states/hysteresis/
cooldown lane trains and re-trains on the hub's schedule) — the per-epoch
cost bounds what a cooldown budget can afford.

## The method

- The fixture: the SAME corpus as E1 (400 texts, 4 classes, seed
  `20261121`, the LCG generator recorded in the harness) — the distill
  pipeline's real shape: texts → `embed` (the in-process TF-IDF lane,
  dim 256) → features → `reflex_train`.
- The model: the Dense family the distill contour trains (№185/№530):
  `dense(256,64,relu)` + `dense(64,4,softmax)`, fresh per run, the
  dispatch's fixed learning rate 0.1, 30 epochs — the hub's №530
  training shape.
- The measurement: three runs of one `train(inputs, targets, 30, 0.1)`
  call each (wall-clock `Instant`), plus a per-epoch series — 30
  sequential `train(epochs=1)` calls timed individually on a fresh
  model. The per-call series is slightly ABOVE the true per-epoch cost
  of one `train(30)` call (each call re-runs the deterministic split
  and the holdout pass); both series are recorded so the reader sees
  the overhead, not just a quotient.
- Environment: `rustc 1.100.0 (0ef1bc386 2026-11-09)`, 2 CPUs, Intel
  Xeon, release profile (the JSON carries the facts verbatim).

## The results

| run | total (s) | per-epoch mean (s) | final loss | internal holdout acc |
|---|---|---|---|---|
| 0 | 0.652 | 0.0217 | 1.2618 | 0.86 |
| 1 | 0.651 | 0.0217 | 1.2849 | 0.82 |
| 2 | 0.645 | 0.0215 | 1.2342 | 0.95 |

- Total-time spread: 0.645–0.652 s (±0.5 % — tight).
- The per-epoch series (30 × `train(1)`): ~22 ms per call, flat across
  epochs (the first three points: 22.95 / 22.08 / 22.11 ms).

## The honest reading

One 30-epoch distill training on a 400-row/256-dim fixture costs
~0.65 s of wall time on this machine — the hub's background thread
(№489/№495 posture) never comes near a request path at this scale, and
a W5 cooldown window measured in seconds has orders of magnitude of
headroom. The holdout accuracy varies (0.82–0.95) across the three runs
because each run's model seed differs (seed + run index — recorded in
the JSON): the training is deterministic per seed, the SPREAD is the
per-seed variation, not noise. The numbers measure THIS machine, THIS
fixture and the Dense path only — not a universal statement (the
honest-limits register).
