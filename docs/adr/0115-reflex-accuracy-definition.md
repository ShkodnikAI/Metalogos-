# ADR-0115: What "accuracy" means for `Reflex`

**Status:** Accepted
**Date:** 2026-09-05
**Naryad:** #179 (blocks on this ADR)
**Pillar:** `Reflex` (eighth semantic pillar)
**Relates to:** `ADR-0112` (mock accuracy for `adapt`), `ADR-0114`
(registry mechanism for extensible metrics)

## Context

`ADR-0112` documented that `accuracy` for `adapt`/`mutate` is
hardcoded to `0.95` (`src/vm.rs:2434`, `src/interpreter/hooks.rs:61`)
and explicitly deferred defining a real metric until a concrete case
existed, requiring prior art rather than a mechanical guess. `Reflex`
is that case: `reflex_train` needs a real, measured number for
`rollback_if: accuracy < threshold` to mean anything.

This ADR does not decide *how metrics are registered* — that
mechanism (a registry mirroring `BUILTIN_REGISTRY`, checked at
`mlog check`-time, extensible without touching `grammar.pest`) is
already decided in `ADR-0114`'s addendum. This ADR decides what the
first, built-in `accuracy` metric actually computes.

## Prior art considered

- **DSPy** (Khattab et al.) evaluates a compiled program against a
  held-out set using a user-supplied metric function, never the
  training set itself — the split is structural, not incidental.
- **scikit-learn's `train_test_split` + `accuracy_score`** — the
  standard, uncontroversial definition: fraction of predictions
  matching the true label on data the model did not train on.
- **k-fold cross-validation** — considered and rejected for the
  first metric: requires k full training runs, adds real wall-clock
  cost for a first implementation where a single held-out split is
  sufficient to make `rollback_if` meaningful. Not precluded for a
  later metric registry entry.

## Decision

`accuracy` = fraction of correct predictions on a **held-out split**
the model did not see during training, computed after the current
training run completes — not a running average during training, not
a training-set score.

**Split:** deterministic, seeded (same `seed` field already required
on `reflex` declarations per the pillar's math-foundation contract,
наряд №177). 80/20 train/holdout by default, not configurable in this
first metric — a configurable split ratio is a mechanical, later
addition to the metric's own parameters, not a reason to block this
ADR.

**Computation:** `correct_predictions / total_holdout_samples`,
computed once, immediately after training, stored on the `Reflex`
handle's metadata (accessible via `reflex_predict`'s confidence path
and via a future `reflex_metrics` builtin — not scoped in this ADR).

**Interaction with `rollback_if`:** `naряд №148` already fixed the
comparison-operator bug for `Gt`/`Ge` in the rollback logic — this
ADR supplies the previously-missing real left-hand-side value that
bug fix was waiting for. No further change to the comparison logic
itself is needed.

## Consequences

- `rollback_if: accuracy < 0.85` genuinely reflects held-out
  performance, closing the gap `ADR-0112` left open.
- A `Reflex` with too little data to form a meaningful holdout split
  (fewer than a minimum sample count — exact threshold decided in
  наряд №179's implementation, not this ADR) must fail training with
  an explicit error, not silently report a meaningless accuracy on
  an empty or near-empty split.
- Future metrics (calibration, F1, regression MSE) are registry
  entries per `ADR-0114`'s addendum — this ADR's split methodology
  (deterministic, held-out, seeded) is the expected pattern new
  metric entries follow, not a one-off special case for `accuracy`
  alone.
