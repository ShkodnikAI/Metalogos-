# ADR 0002: M2 — Rule Engine, Confidence, Flow Branching

**Status:** Accepted
**Date:** 2026-05-31
**Milestone:** M2 — Confidence, rules, branching

## Context

M2 introduces three capabilities that make the language feel probabilistic:
1. **Struct entities** with typed fields and defaults (`entity Message { text: String, urgency: Float = 0.0 }`)
2. **Rules** with conditions and priority (`rule If(...) then ... with priority=10`)
3. **Flow branching** by confidence thresholds (`Classify { high (...) -> Escalate, low (...) -> Ignore }`)

The contract program is `examples/m2_triage.mlog`:
```mlog
entity Message { text: String, urgency: Float = 0.0 }
entity m: Message = { text: "срочно нужна помощь", urgency: 0.0 }

rule If(m.text contains "срочно") then m.urgency = 0.9 with priority=10

pattern Escalate(msg: Message) -> String { return "ESCALATE" }
pattern Queue(msg: Message) -> String { return "QUEUE" }
pattern Ignore(msg: Message) -> String { return "IGNORE" }

flow Main {
  input: Message = m -> Classify -> output
  Classify {
    high (m.urgency > 0.8)        -> Escalate
    medium (m.urgency < 0.8)       -> Queue
    low  (m.urgency < 0.4)        -> Ignore
  }
}
```

**Done when:** `mlog run m2_triage.mlog` prints `ESCALATE`.

## Decision

### Rule engine: priority-ordered, first-wins

Rules are stored in a `Vec<RuleDecl>` and executed before the flow runs. The
resolution strategy, grounded in production systems (Rete, CLIPS) and
recommended by the `metalogos-language-semantics` skill:

1. **Sort by priority descending.** Higher priority rules fire first.
2. **Stable sort preserves declaration order** for rules with equal priority.
3. **First match wins.** When multiple rules match, the highest-priority rule
   sets the value. No chaining, no conflict detection — just first-wins.

This is the simplest defensible semantics. It avoids the complexity of
conflict resolution (as in production systems like Rete) while still
providing deterministic behavior. We document this as an intentional
simplification: real production systems use forward-chaining to a fixpoint,
but M2 only needs single-pass rule application.

**Growth path (not now):** Forward-chaining to fixpoint in later milestones,
then weighted inference in the style of Markov Logic Networks (Phase 2).

### Rule conditions

Two condition forms are supported:
- **`contains`**: `m.text contains "срочно"` — string substring check.
- **Comparison**: `m.urgency > 0.8` — numeric comparison with `>`, `<`, `>=`, `<=`, `==`.

Both are evaluated against the current variable environment (entities and their
fields). The expression evaluator already handles `FieldAccess` for `m.text`.

### Assignment in rules

Rule syntax: `rule If(condition) then target.field = value with priority=N`.

In the grammar, `assignment = { IDENT ~ "." ~ IDENT ~ "=" ~ expression }` uses
IDENT rather than a full expression for the target. This prevents the expression
parser from greedily consuming `m.urgency` as a single `field_expr`, which would
leave no tokens for the `.` separator in the assignment rule. This is a pest
PEG limitation: the expression grammar is too permissive for use as a
sub-component of assignment patterns.

### Confidence propagation (honest semantics)

Per the `metalogos-language-semantics` skill, confidence propagation through
patterns uses a simple heuristic: output confidence = min (or product) of input
confidences. For M2, we do NOT implement a full Fluid Type system. Instead,
Float fields on struct entities serve as explicit confidence values. The
`urgency: Float = 0.0` field IS the confidence — it is set by rules, read by
branch conditions, and used to route flow execution.

**Documented limitation:** This is not probabilistic reasoning. It is
deterministic field assignment with numeric thresholds. We do NOT claim
mathematical rigor for confidence propagation. That is research-grade work
deferred to Phase 2.

### Flow architecture: pipeline + branch definitions

The M2 contract reveals that `Classify { branches }` blocks appear AFTER the
`-> output` line in a flow, not inline in the pipeline. This required a
fundamental restructuring of the flow grammar:

```
flow_decl     = { "flow" ~ IDENT ~ "{" ~ flow_pipeline ~ branch_def* ~ "}" }
flow_pipeline = { "input" ~ ":" ~ type_name ~ "=" ~ expression ~ (ARROW ~ step_ident)* ~ ARROW ~ "output" }
branch_def    = { step_ident ~ "{" ~ branch* ~ "}" }
```

The pipeline is now purely linear: a sequence of step names (IDENT) from
`input` to `output`. Branch definitions are separate blocks that follow the
pipeline. When the interpreter encounters a step name that has a branch
definition, it evaluates the branch conditions against the current value and
dispatches to the matching target.

This separation has several advantages:
1. **Clean PEG parsing.** No ambiguity between `->` in the pipeline and `->`
   inside branch blocks.
2. **Orthogonal extension.** Future flow features (parallel steps, error
   handling) can be added to the pipeline without touching branch definitions.
3. **Clear semantics.** The pipeline is a simple dispatch chain; branch
   definitions are pattern-matching tables.

### pest grammar lessons learned

Several pest-specific issues were discovered during M2:

1. **Missing `~` after `*`/`?`/`+`.** In pest 2.8, implicit `~` (sequence
   operator) is NOT inserted after postfix repetition operators before a
   string literal containing `}`. `field_decl* "}"` fails to parse; you must
   write `field_decl* ~ "}"`. This is a pest grammar parser limitation with
   brace counting inside string literals.

2. **Silent rules unwrap their inner pair.** `_{ a | b }` does not produce a
   `flow_step` pair — it produces the inner `a` or `b` pair directly. The
   parser must check for the concrete rule variants (`linear_step`,
   `branch_step`), not the wrapper.

3. **Named rule wrappers add pair layers.** `type_name = { IDENT }` produces
   a `type_name` pair whose inner child is `IDENT`. The parser must unwrap
   with `into_inner()` to get the actual IDENT string.

4. **Negative lookahead syntax.** `!":"` works in pest 2.8 for negative
   lookahead of a single string literal. The `!("pattern1" | "pattern2")`
   syntax with alternatives is used in `step_ident`.

### Parser migration: hand-rolled → pest

The Agent Builder Charter asked whether to migrate from hand-rolled parsing
to pest/chumsky. **Decision: stay with pest.** pest was already adopted in
M1 (ADR 0001) and works well for M2. The grammar has grown to ~100 lines but
remains readable and declarative. Chumsky would provide better error messages
but would require a full rewrite of the parser with no functional benefit.

## Consequences

- M2 proves that METALOGOS can express probabilistic-style reasoning:
  rules elevate confidence, flow branches on thresholds.
- The `priority-ordered, first-wins` rule strategy is simple but sufficient.
  It will need upgrading when we add forward-chaining (M3+).
- The flow architecture change (pipeline + branch_def) is a permanent
  structural decision. All future flow features must respect this separation.
- pest grammar maintenance requires careful attention to `~` operators and
  silent rule unwrapping. These are documented here to prevent future
  confusion.
- M1 tests remain green — no regressions from M2 changes.
