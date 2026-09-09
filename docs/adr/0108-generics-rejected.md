# ADR-0108: Generics — not introduced, reaffirms the decision of ADR-0011

> **Status:** Rejected (reaffirmed)  
> **Date:** 2026-08-21  
> **Decision by:** owner  
> **Precedent:** ADR-0011 (type inference — original decision),
> ADR-0105 (VM experimental scope — same principle)

## Context

An external audit proposed introducing generics as a priority step
toward a "full" type system, unaware of the existing decision.

`ADR-0011` (Status: Implemented) already settled this question for
the current phase, with an explicit comparison of prior art
(Hindley-Milner vs constraint-based vs explicit annotations) and
the rationale:

> "Metalogos has explicit type annotations on patterns and entities,
> making forward-propagation through the flow pipeline sufficient for
> Phase 2."

This is not a gap — it is an accepted decision with status "Implemented".

## Decision

**Reaffirmed, not revisited.** Generics are not introduced.

The only basis found for a future revisit is a concrete, reproducible
case where explicit typing genuinely gets in the way (candidate
example: `std/collections.mlog` is hard-typed to `String`,
`first(items: List) -> String` — if the same pattern is needed for
numbers or structs, that would be a real trigger). The abstract
"modern languages need generics" is not such a basis.

## Consequences

- The grammar and AST remain without type parameters.
- Explicit types on patterns/entities plus forward-propagation
  remain the sole type-checking mechanism.
- Revisit only when a concrete case is demonstrated, not out of
  general completeness considerations (the same principle ADR-0105
  applied to the VM).

## Related

- ADR-0011 — the original decision and full rationale (not rewritten,
  this ADR only reaffirms its currency after the external audit)
- ADR-0105 — the same "don't fix without demonstrated demand" principle
