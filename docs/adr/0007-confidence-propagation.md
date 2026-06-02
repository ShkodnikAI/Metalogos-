# ADR-0007: Confidence Propagation through Patterns

**Status:** Implemented
**Date:** 2026-05-31
**Milestone:** P1 (Phase 1 — Type System)

---

## Context

Fluid Types (ADR-0006) introduced superposition values that collapse lazily at use-sites.
However, after collapse the confidence metadata was discarded: a pattern receiving
`Fluid<Float[42.0], confidence=0.9>` would compute its result as a plain `Float(84.0)` with
no trace of the original uncertainty. This violates the principle that confidence should
flow through the system so downstream consumers (rules, branches, humans) can make
informed decisions.

Question: when a Fluid value passes through a pattern, what confidence does the output carry?

## Prior Art

| Approach | Source | Trade-off |
|---|---|---|
| Product of confidences | Markov Logic Networks (Richardson–Domingos) | Probabilistically motivated, but assumes independence of inputs |
| Minimum confidence | Fuzzy logic (Zadeh), Dempster–Shafer lower bound | Conservative; guarantees output confidence ≤ any input; simple |
| Bayesian updating | ProbLog, probabilistic programming | Computationally expensive; requires priors and likelihoods |
| No propagation (discard) | Naive gradual typing implementations | Loses information; downstream cannot reason about uncertainty |

## Decision

**Output confidence = min(confidence of all inputs).**

### Algorithm

1. When a pattern is invoked, each argument is collapsed via `collapse_with_confidence(arg, param_type)`, which returns both the concrete value and the confidence of the chosen variant.
2. `bind_and_collapse` computes `min_confidence = min(confidence_i)` across all arguments.
3. After pattern body execution, if `min_confidence < 1.0`, the result is wrapped in `Value::Fluid` with a single variant carrying that confidence.
4. Non-Fluid inputs contribute confidence 1.0 (full certainty), so they don't reduce the minimum.
5. A new builtin `confidence(v)` extracts the confidence of a Fluid value (or returns 1.0 for concrete values).

### Special case: `Fluid` as a parameter type

When a pattern declares a parameter of type `Fluid`, the Fluid value passes through
uncollapsed — the full superposition is available inside the pattern body. The confidence
is the best variant's confidence. This allows patterns like `GetConf(v: Fluid) -> Float`
to inspect confidence without forcing collapse.

### Soft-failure

If a Fluid value has no matching variant for the required type, or confidence is below the
collapse threshold (0.1), the value collapses to `Value::Unit` and the confidence is 0.0 or
the variant's actual confidence. The `min` rule then propagates this low confidence,
allowing downstream rules to detect degradation.

## Rationale

- **`min` over `product`:** Product assumes independence of input uncertainties, which is
  almost never true in practice (inputs often share sources of uncertainty). `min` is the
  safe, conservative bound — the output is no more certain than the least certain input.
  This is the same reasoning as in Dempster–Shafer theory for combining evidence.
- **Wrapping in Fluid (not adding a separate field):** Reusing the existing `Value::Fluid`
  type avoids changes to the core `Value` enum and makes confidence propagation transparent
  to the rest of the system. Any downstream consumer that already handles Fluid values
  automatically receives the propagated confidence.
- **`confidence()` builtin:** Provides an observation point for testing and for program
  logic that needs to branch on confidence levels.

## Limitations (Documented)

This is a **heuristic**, not a probabilistically correct inference procedure. It does not:
- Account for correlation between inputs
- Perform Bayesian updating with priors
- Distinguish between different sources of uncertainty (aleatoric vs. epistemic)
- Propagate confidence through arithmetic in a principled way (e.g., error propagation)

These are intentionally deferred to Phase 2 as research topics. The `min` rule is the
simplest defensible starting point, consistent with the Metalogos build-ladder principle
of "простейшая защитимая семантика" (simplest defensible semantics).

## Examples

```mlog
fluid x = Float[10.0][0.6]
pattern Double(n: Float) -> Float { return n + n }
pattern GetConf(v: Fluid) -> Float { return confidence(v) }
flow Main { input: Float = x -> Double -> GetConf -> output }
// Output: 0.6  (confidence propagated through Double)
```

## Impact

- **`builtins.rs`:** Added `confidence()` builtin.
- **`interpreter.rs`:** Added `collapse_with_confidence`, modified `bind_and_collapse` to
  return `(env, min_confidence)`, modified `invoke` and FnCall evaluation to wrap results
  in Fluid when confidence < 1.0.
- **Backward compatible:** Non-Fluid inputs produce confidence 1.0, so existing patterns
  return unwrapped results. P1 fluid_types test passes unchanged (Display of Fluid shows
  inner value).
