# ADR-0117: Semantics of distillation — mode switching, backward compatibility, and why generation stays out of scope

**Status:** Accepted
**Date:** 2026-09-05
**Naryad:** #181 (blocks on this ADR)
**Pillar:** `Reflex` (eighth semantic pillar, final stage of the initial rollout)
**Relates to:** `ADR-0112` (mock accuracy — this ADR is the pillar's
completion of that deferred work), `ADR-0114`–`0116` (representation,
metric, persistence — all prerequisites this ADR assembles)

## Context

Наряды №177–180 gave the language a real, working `Reflex`: declare,
train, predict, persist. What remains is the point of contact with
`learnable pattern` — the construct that makes distillation *useful*
rather than merely possible: a pattern starts by calling an LLM on
every input, and should be able to switch to a locally-`Reflex`ed
head for inputs it has already learned to classify confidently,
without the program's author writing that switching logic by hand.

Three semantic questions need answers before code:

### 1. What does "switching modes" mean for observable program behavior?

**Decision:** mode switching is an **internal execution-strategy
change**, not an observable change in the pattern's contract. A
`learnable pattern` with `distill_to` set still has the same
signature, same return type, same set of possible outputs as one
without it. The only observable differences are: latency (lower once
distilled), and — honestly, not hidden — a `Fluid` confidence value
that now comes from a calibrated softmax rather than being absent or
approximated by the LLM call's own self-reported confidence.

### 2. Backward compatibility guarantee

**Decision:** a `learnable pattern` **without** `distill_to` behaves
identically before and after this наряд — same grammar, same
execution path, same output. This is not a "should mostly work"
guarantee; наряд №181's contracts must include an explicit
regression test proving byte-identical output for an undistilled
pattern, matching the same discipline наряд №169 applied when
splitting `diagrams.rs` (mechanical changes verified not to alter
existing behavior).

### 3. Why free-form generation is out of scope, not "future work"

**Decision:** distillation applies only to patterns whose output is
a closed label set or a numeric value — never free text. This is not
a temporary limitation to be lifted later; it is a structural
property of what `Reflex` *is* (`ADR-0114`: a small classifier/
regressor head over an embedding, not a language model). Generating
open-ended text would require the architecture-block work наряд №176
scoped separately (attention, transformer blocks, `candle`) — a
distinct, larger decision the project owner has not yet made. This
ADR does not gesture at that as an eventual target; conflating the
two would misrepresent what this наряд delivers.

**Enforcement:** `distill_to` is only valid on a `learnable pattern`
whose declared return type is `String` used as a closed label (the
`reflex`'s own `labels` list) or a numeric type — `mlog check` must
reject `distill_to` on a pattern whose historical outputs (or
declared intent) suggest open-ended text, at minimum by requiring the
referenced `reflex`'s `labels` list to be non-empty for
`String`-returning patterns. A pattern author asking for free-text
distillation gets a clear compile-time error naming why, not silent
LLM-only fallback forever.

## Grammar note (not this ADR's core decision, but binding for наряд №181)

`learnable_body`'s existing grammar (`src/grammar.pest`) is a fixed-
order sequence of optional fields — the same rigidity `ADR-0114`'s
addendum already criticized for other declarations (`server { middleware:
[...] port: 8080 }` fails to parse if fields are reordered). Наряд
№181 must not add `distill_to`/`distill_after`/`fallback_if` as three
more entries in that same rigid sequence, compounding a known,
already-flagged defect. At minimum, the three new fields should parse
in any order relative to each other, even if the наряд does not fix
the pre-existing fields' ordering as a separate concern.

## Consequences

- `ADR-0112`'s deferred question is now fully closed: `accuracy` is
  real (`ADR-0115`), and the mechanism that consumes it (`rollback_if`
  via `reflex_train`'s returned `threshold_met`, наряд №179b) already
  works.
- A pattern author opts into distillation by adding three lines;
  removing them returns to LLM-only behavior with zero other changes
  — the pillar is additive, not a fork of `learnable pattern`'s
  semantics.
- Free-text generation remains explicitly, permanently out of this
  pillar's initial scope — any future work in that direction is
  `наряд №176`'s domain (architecture blocks) and requires its own,
  separate owner decision, not an incremental extension of this ADR.
