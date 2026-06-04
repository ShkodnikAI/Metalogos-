# ADR 0005: M5 — Adapt: Few-Shot Self-Modification

**Status:** Accepted
**Date:** 2026-05-31
**Milestone:** M5 — Adapt: safe self-modification of learnable patterns

## Context

M5 introduces the first form of self-modification to METALOGOS: an `adapt` declaration
can add few-shot examples to a `learnable pattern` at runtime. When the pattern is
subsequently invoked, it checks the few-shot examples first (exact-match cache) and
returns the cached output without calling the LLM. This is the foundation for
in-context learning — the simplest defensible form of program self-modification.

The contract program is `examples/m5_adapt.mlog`:
```mlog
learnable pattern Greet(name: String) -> String {
  prompt: "hello"
}

adapt Greet add_example("world", "Hello, world!")

pattern RunGreet(input: String) -> String {
  return Greet(input)
}

flow Main { input: String = "world" -> RunGreet -> output }
```

**Done when:** `mlog run m5_adapt.mlog` prints `Hello, world!` (cached, not from LLM).

## Decision

### What M5 implements (minimal subset)

Per the `metalogos-language-semantics` skill, M5's full scope includes `adapt`, `mutate`,
and `sandbox` with rollback. For the MVP milestone, only the simplest useful form
is implemented:

```mlog
adapt PatternName add_example("input_value", "output_value")
```

This adds a single `(input, output)` pair to the learnable pattern's few-shot example
set. When the pattern is invoked with an input that exactly matches a cached example,
the cached output is returned immediately — no LLM call is made.

### What M5 defers (documented as Phase 2)

The build-ladder contract shows richer syntax that is intentionally not implemented:

1. **`mutate pattern Classify { add_example: ... rollback_if: accuracy < 0.9 }`**
   — Requires a test-suite runner, accuracy metric computation, and rollback
   mechanism. This is complex and requires M4 memory for test storage.

2. **`sandbox experimental { allowed: [...], forbidden: [...], timeout: 60.seconds }`**
   — Requires capability-based execution sandboxing. This is research-grade work
   requiring OS-level isolation or WASM sandboxing.

3. **`adapt Classify with new_example feedback=user_correction`** — Alternative
   syntax for implicit example extraction from feedback. Deferred until the feedback
   model is defined.

These are documented here as intentional simplifications. The build-ladder contract
serves as the roadmap, not the MVP spec.

### Few-shot match semantics

The match strategy is deliberately simple: **exact string equality** on the
formatted input. The input is formatted by joining all arguments with `", "`
(the same format used for LLM calls). This means:

- `Greet("world")` checks if `"world"` matches any cached input → yes → returns `"Hello, world!"`
- `Greet("stranger")` checks if `"stranger"` matches → no → calls LLM mock → returns `"hello"`

**Growth path:** Replace exact match with:
- Substring/contains matching (M4-style)
- Embedding-based similarity (Phase 2, vector index)
- Fuzzy matching with configurable threshold

### Safety invariants

Per the `metalogos-language-semantics` skill:

1. **Adapt only modifies learnable patterns.** It cannot modify pure patterns,
   rules, entities, or flows. This is enforced at the AST level — `AdaptDecl`
   only references a pattern name, and the interpreter only looks up
   `learnable_patterns`.

2. **No arbitrary code modification.** Adapt adds data (examples) to a pattern,
   it does not rewrite the pattern's logic. This is the key safety invariant
   that distinguishes METALOGOS adapt from arbitrary self-modification.

3. **Rollback is deferred.** In the full design, `mutate` with `rollback_if`
   would remove the example if accuracy drops below threshold. In M5, there
   is no rollback — examples are added permanently within a single execution.

4. **Safety-critical rules are protected.** The semantics skill states that
   safety-critical rules cannot be touched by mutations. In M5, this is
   trivially true because adapt only modifies learnable patterns, never rules.

### Interpreter changes

- `CompiledLearnable` gains a `few_shot: Vec<(String, String)>` field.
- `Declaration::Adapt(a)` is processed after `LearnablePattern` registration,
   pushing `(input_str, output_str)` onto the target pattern's few-shot list.
- `invoke_learnable()` checks few-shot examples before calling the LLM.
  Exact match → cache hit → return cached output. No match → LLM fallback.

## Consequences

- M5 proves that METALOGOS programs can modify their own behavior at runtime:
  adapt adds knowledge, and subsequent invocations use that knowledge.
- The exact-match cache is deterministic and testable. Golden tests prove the
  contract without requiring a real LLM.
- The safety invariant (adapt only touches learnable patterns) is enforced at
  the type system level, not just by convention. This is a strong guarantee.
- M1 through M4 tests remain green — no regressions.
- The deferred features (mutate, sandbox, rollback) are documented in this ADR
  as the explicit roadmap for post-M5 development. They should be implemented
  in the same contract-first style when the time comes.
