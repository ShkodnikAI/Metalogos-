# ADR-0112: `adapt` quality metric — current mock, not an implemented function

> **Status:** Accepted + IMPLEMENTED (реализовано в наряде №375, 2026-09-16 — see the Implementation addendum at the bottom)
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


---

## Implementation addendum (2026-09-16, Наряд №375 — the revisit condition fired)

The revisit condition was "a real `mutate` use case where the mock value of
0.95 creates a concrete problem" — the external audit of 2026-09-15 marked
the constant-driven keep/rollback decision of a self-modifying system as P0.
Наряд №375 implemented the real metric. Methodology:

**Battery assembly.** Golden tasks are `(input, expected)` pairs gathered
from two sources, deduped by input (first source wins):
1. the eval-block datasets registered for the mutated pattern (ADR-0050) —
   the user-declared golden set;
2. the pattern's pre-mutation few-shot — the pattern's established Q→A
   behavior.

**Held-out split.** A battery task is held out iff its input is NOT among the
mutation's own new-example inputs (the build set). Accuracy is NEVER measured
on the tasks the mutation was built from.

**Deterministic seeds.** The held-out tasks are measured in an order given by
a seeded FNV-1a hash (fixed constant `0x9E3779B97F4A7C15` — the same constant
seeds `seed_to_state` in the PRNG). The same battery + the same mutation →
the same measurement, byte-for-byte, across runs and processes.

**Measurement.** Each held-out task is answered by the pattern's REAL answer
path (the LLM backend; TW uses the full effective-prompt call, VM the base
prompt — the difference is documented). `answer.trim() == expected.trim()` →
correct; a backend error counts as incorrect. `accuracy = correct /
held_out`; an EMPTY held-out set scores 0.0 — no evidence, no keep.

**Minimum.** A battery below 20 tasks carries a loud `BELOW MINIMUM 20`
marker in the mutate log; the measurement still runs (honesty over comfort).

**Mock mode.** `METALOGOS_MOCK_LLM` (default-on — the codebase-wide
test-mode convention) keeps the 0.95 stub for the rollback-mechanism tests;
this is the ONLY place the stub survives, loudly documented here, in the
code comment and in the mutate-log contract tests.

**rollback_if semantics.** The threshold comparison (CompareOp/ConditionOp
mapping) is UNCHANGED — only the input value stopped being a constant.

**Where:** `src/interpreter/learnable.rs` (`measure_battery_accuracy`,
`MIN_BATTERY_TASKS`, `call_llm_for_battery`), `src/interpreter/hooks.rs`
(TW mutate path), `src/vm.rs` (VM mutate path).
