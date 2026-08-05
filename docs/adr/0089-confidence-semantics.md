# ADR-0089: Confidence Semantics — Actual State

## Status

accepted

## Context

The Metalogos language has two confidence-related concepts:

1. **Fluid Types** — values exist as superpositions of typed variants with
   confidence scores. Example: `fluid x = Float[42.0][0.9] or String["answer"][0.1]`.
2. **LLM memory confidence** — memorized entries carry a `confidence` field
   equal to their priority at store time (`src/interpreter/execution.rs:76`).

The existing example `p1_confidence_propagation.mlog` (renamed to
`p1_fluid_collapse.mlog` in Наряд №42) demonstrated that confidence does
**not** propagate through pattern calls: a Fluid value collapsed to a concrete
Float, passed through `Double`, then `confidence()` returned `1.0`.

This was an opportunity for confusion — the original name "confidence
propagation" documented the *absence* of propagation.

## Decision

### What exists (as of Наряд №42)

1. **Fluid collapse** (`maybe_collapse` in `src/interpreter/execution.rs:1472`):
   - At point of use (pattern binding), Fluid values collapse lazily.
   - The variant matching the required type with the **highest confidence** is
     selected.
   - If confidence >= `COLLAPSE_THRESHOLD` (0.1), the concrete value is
     returned.
   - Below threshold → `Unit` (soft-failure).
   - No matching type variant → `Unit` (soft-failure).
   - Non-Fluid values pass through unchanged.

2. **`confidence()` builtin** (`src/builtins/math.rs:38`):
   - On Fluid values: returns the **max** confidence across all variants.
   - On concrete values: returns `1.0` (fully confident).

3. **After collapse, confidence is lost.** The collapsed value is a plain
   `Float`, `String`, etc. — no confidence metadata. Any subsequent
   `confidence()` call on it returns `1.0`.

### What does NOT exist

**Confidence propagation through pattern calls.** When a Fluid value is
collapsed to pass through a pattern, the output of that pattern is a
concrete value with no confidence tracking. There is no mechanism to carry
input confidence to output confidence via `min`, product, or any other
combining function.

This is consistent with the current language design: collapse is a
one-way conversion that consumes the Fluid superposition and produces a
concrete value.

## Open Question: Future Propagation

If confidence propagation is added in a future наряд, prior art and
recommended starting approaches include:

| Approach | Source | Notes |
|----------|--------|-------|
| `min(confidences)` of inputs | `metalogos-language-semantics` skill | Simplest. Honest heuristic — "weakest link". Not probabilistic inference. |
| Product of confidences | ProbLog, fuzzy logic | Only valid for independent evidence. |
| Dempster–Shafer combination | MLN, uncertainty reasoning | Most principled but complex. |
| Bayesian update | Probabilistic programming | Requires priors and likelihoods. |

Recommended start: `min()` of input confidences, explicitly documented as
an **heuristic for ordering alternatives**, not a sound probabilistic
inference mechanism.

## Consequences

- The example `p1_fluid_collapse.mlog` (formerly `p1_confidence_propagation`)
  now accurately describes what it tests: Fluid collapse, not propagation.
- README does not claim confidence propagation as a working feature.
  "confidence-based branching" refers to branch conditions on entity fields,
  which is correct.
- **Implemented (session 0805)**: Confidence propagation via `min()` heuristic
  (ADR-0089 recommended start). When a Fluid value is collapsed to pass through
  a pattern call, the collapsed confidence is tracked via `propagated_confidence`
  (Interpreter) / `propagated_confidence` (VM). After pattern execution, if
  confidence < 1.0, the result is wrapped as `Fluid` with a single variant
  carrying the min confidence. This allows downstream `confidence()` calls to
  return the propagated value instead of the default 1.0.
  - `p1_fluid_collapse.expected` updated: `1` → `0.6` (confidence now propagates
    through Double pattern call).
  - Both interpreter (`execution.rs`) and VM (`vm.rs`) updated for parity.
