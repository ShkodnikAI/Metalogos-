# ADR-0112: `adapt` quality metric — current mock, not an implemented function

> **Status:** Accepted
> **Date:** 2026-08-28
> **Naryads:** #124 (documentation)
> **Precedent:** ADR-0105 (honest-gap-documentation), ADR-0110
>  §1 (semantic question requires prior art, not mechanical addition)

## Context

The `adapt` statement (few-shot mutation with sandboxing and
rollback) uses `accuracy` to decide whether to keep a mutation or
roll it back. In the current implementation, this value is a
hardcoded stub:

```rust
// src/vm.rs:2433-2434 and src/interpreter/hooks.rs:60-61
let accuracy: f64 = 0.95;  // Mock accuracy (always 0.95 for MockLlm)
```

The rollback mechanism (the logic comparing `accuracy` against a
threshold, restoring the pattern's prior version) is real and covered
by tests. But it **does not respond to a mutation's actual quality**,
because the input value is always the same.

README (before this naryad) claimed: *"quality metrics, and automatic
rollback on degradation"* — without qualification, creating the
impression of a working metric.

## Decision

**Do not implement a real quality metric in this naryad.**

The question of *what "accuracy" means for an LLM pattern without an
explicit test set* is semantic (ADR-0110 §1), not mechanical. It
requires separate research: prior art (DSPy, eval frameworks for
LLMs), a definition of the comparison baseline, and a dataset for
evaluation.

**Revisit only on a real `mutate` use case where the mock value of
0.95 creates a concrete problem**, not abstractly.

## Consequences

- README honestly describes the current state: the rollback logic
  exists and works, but the quality metric is a mock.
- The mock value `0.95` stays in the code — it is correct for
  testing the rollback mechanism without a real LLM.
- When real demand for `adapt` with genuine quality assessment
  appears — a separate naryad with an ADR defining the approach.

## Related

- ADR-0105 — precedent for honest gap documentation
- ADR-0110 §1 — protocol: semantic questions require prior art
- Naryad #124 — this naryad (documentation)

## Revisit point (2026-09-10)

The external audit of 2026-09-10 recorded the mock accuracy as an open
finding ("not fixed"). Coordinator verification confirmed the fact itself
and the status of the recorded decision. The decision is **reaffirmed**:
no real quality metric is implemented, and the revisit condition stays
verbatim — "Revisit only on a real `mutate` use case where the mock value
of 0.95 creates a concrete problem", not abstractly.

Current code addresses of the mock: `src/interpreter/hooks.rs:60-61` and
`src/vm.rs:2946-2947` (the historical citation `src/vm.rs:2433-2434` in the
Context section above has drifted — the lines moved with the growth of
vm.rs; the fact did not change).

Positioning: README (§5) and REFERENCE (§5.15) mark both the mock value and
the revisit point (naryad #247, Block 2).
