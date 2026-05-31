# ADR 0004: M4 — Memory: Memorize, Recall, Forget, Decay

**Status:** Accepted
**Date:** 2026-05-31
**Milestone:** M4 — In-memory store with decay and similarity recall

## Context

M4 introduces persistent memory to METALOGOS programs. Until now, all state was
confined to entities and variables within a single execution. M4 adds the ability
to store facts in memory, retrieve them by similarity, and remove them by age or
explicitly — the foundation for any system that learns from experience.

The contract program is `examples/m4_memory.mlog`:
```mlog
memorize "user likes spicy food" with priority=0.9
memorize "user hates cold soup" with priority=0.7

pattern FindFood(query: String) -> String {
  return recall(query)
}

flow Main { input: String = "spicy" -> FindFood -> output }
```

**Done when:** `mlog run m4_memory.mlog` prints `user likes spicy food`.

## Decision

### Memory model: in-memory Vec with activation-based recall

Per the `metalogos-language-semantics` skill, grounded in ACT-R activation memory:

Each memory entry stores four fields:
- **value** (String): the fact itself
- **priority** (Float 0.0..1.0): initial confidence/activation
- **timestamp** (i64 Unix seconds): when the fact was memorized
- **decay_rate** (Float/day): exponential decay coefficient

The activation of a memory entry at recall time is:
```
activation = priority * exp(-decay_rate * age_in_days)
```

This is the ACT-R base-level learning equation simplified to a single exponential
decay. We document this as an intentional simplification: real ACT-R uses a more
complex formula with decay parameters estimated from empirical data.

### Memorize syntax

```mlog
memorize "user likes spicy food" with priority=0.9
```

A top-level declaration. The expression is evaluated, converted to string, and
stored in the interpreter's memory `Vec<MemoryEntry>`. Default priority is 0.5.

### Recall: substring similarity + activation-based ranking

```mlog
pattern FindFood(query: String) -> String {
  return recall(query)
}
```

`recall` is implemented as a special case in the interpreter's `invoke()` method
(rather than a simple builtin function) because it needs access to the memory store.

The recall algorithm:
1. Filter memory entries whose value **contains** the query string (substring match).
2. Compute activation = priority * exp(-decay_rate * age_in_days) for each match.
3. Filter entries with activation >= min_confidence (default: 0.0).
4. Return the entry with the highest activation.

**No match → soft-failure:** returns empty string `""` rather than an error. This
follows the soft-failure principle from `metalogos-language-semantics`: degraded
results rather than crashes.

**Growth path:** Replace substring matching with embedding-based similarity
(vectors from sentence-transformers, cosine distance). This requires a vector
index (qdrant/hnswlib) and is deferred to Phase 2.

### Forget: time-based removal

```mlog
forget "cold" after 30.days
```

Removes all memory entries whose value contains the query string AND whose
timestamp is older than `now - (days * 86400)`. This is irreversible within
a single execution — forgotten entries cannot be recalled.

### Decay semantics (honest limitations)

- M4 does NOT implement vector similarity or semantic recall. Substring
  matching is a deliberate MVP simplification.
- The decay formula is a single-parameter exponential. Real ACT-R uses
  `d * log(n+1) - d * log(n)` where `n` is the number of presentations.
  We defer this to Phase 2 when we have access to usage counts.
- Memory is in-process only — no persistence across executions. `serde`-based
  persistence is a straightforward addition but is not part of the M4 contract.
- There is no `memory { ... }` configuration block yet (shown in the build-ladder
  contract). Default decay_rate=0.01 (slow) applies to all entries. Per-type
  configuration (episodic vs semantic with different retention/decay) is deferred.

## Consequences

- M4 proves that METALOGOS programs can store and retrieve facts: the language
  now has a rudimentary form of experience.
- The activation-based decay formula is grounded in ACT-R and provides
  deterministic, testable behavior. The default decay_rate=0.01 means entries
  lose ~1% activation per day — effectively no practical decay for golden tests
  that run in milliseconds.
- The `recall` function is a special interpreter method, not a builtin. This
  architectural decision keeps builtins as pure functions while giving memory
  operations access to interpreter state.
- M1, M2, and M3 tests remain green — no regressions from M4 changes.
- Future milestones will add: serde persistence, embedding-based recall,
  per-type memory configuration, and the `memory { ... }` block.
