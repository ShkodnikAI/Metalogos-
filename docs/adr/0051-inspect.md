# ADR-0051: inspect() — Pattern Metadata Builtin

**Status:** Implemented
**Date:** 2026-06-09

## Context

As learnable patterns accumulate invocations, few-shot examples (via `adapt`), and cached responses, developers need observability into their runtime behavior. Without instrumentation, it is impossible to answer questions like "how many times has Classify been called?", "what's the cache hit rate?", or "when was the last adapt?". The `eval` harness (ADR-0050) provides batch accuracy testing, but lacks per-invocation telemetry. A lightweight `inspect()` builtin gives developers and operators real-time pattern metadata without external monitoring tools.

## Decision

Add a new builtin `inspect(pattern_name)` that returns a `Struct` with per-pattern runtime statistics:

```mlog
let stats = inspect("Classify")
// stats.calls           → Float (total invocations)
// stats.avg_confidence  → Float (average confidence across invocations)
// stats.cache_hits      → Float (few-shot + LLM cache hits)
// stats.last_adapt      → Float (Unix timestamp of last adapt, 0 if never)
// stats.examples_count  → Float (current few-shot example count)
```

### Semantics

1. **`inspect("PatternName")`**: Takes a single String argument — the name of a learnable pattern. Returns a `Value::Struct { type_name: "PatternStats", fields: { ... } }`.

2. **Per-pattern tracking**: The interpreter maintains a `HashMap<String, PatternStats>` behind a `Mutex`. Each learnable pattern has its own entry, created lazily on first invocation or adapt.

3. **Automatic stats recording**:
   - `calls`: Incremented on every learnable pattern invocation (few-shot match, cache hit, or LLM call).
   - `avg_confidence`: Computed as `confidence_sum / calls`. Currently defaults to 1.0 per invocation (non-Fluid results are fully confident). Future enhancement: extract actual confidence from Fluid-typed responses.
   - `cache_hits`: Incremented when a response is served from few-shot exact match or LLM response cache (ADR-0047). Regular LLM calls (cache miss) do not increment this counter.
   - `last_adapt`: Set to the current Unix timestamp whenever an `adapt` declaration is processed. Remains 0 if the pattern has never been adapted.
   - `examples_count`: Reflects the current number of few-shot examples in the `CompiledLearnable.few_shot` vector. Incremented by each `adapt` declaration.

4. **Scope**: `inspect()` works for any learnable pattern registered in the current interpreter. For non-existent or never-invoked patterns, it returns a struct with all zero values (except `examples_count` which reflects the current few-shot count from adapt).

5. **Non-persistent**: Stats are in-memory only. They reset on interpreter restart (same design as session memory, ADR-0049). This is intentional — inspect is for live debugging and monitoring, not historical analytics.

### Implementation

- **Interpreter**:
  - `PatternStats` struct: `calls: u64`, `confidence_sum: f64`, `cache_hits: u64`, `last_adapt: i64`, `examples_count: u64`. Public, with `avg_confidence()` method.
  - `pattern_stats: Mutex<HashMap<String, PatternStats>>` field on `Interpreter`.
  - `record_pattern_call(name, cache_hit)`: Increments `calls`, adds to `confidence_sum`, and conditionally increments `cache_hits`. Called from `invoke_learnable_with_env()`.
  - `invoke_inspect(args)`: Looks up stats, returns `Value::Struct` with 5 Float fields. Special-cased in FnCall dispatch (like `query()` and `json_body()`) because it needs interpreter context.
  - `invoke_learnable_with_env()` signature changed: added `pattern_name: &str` parameter.
  - Adapt handling in `run()` and `load_module_inner()`: updates `last_adapt` and `examples_count` in pattern stats.

### Backward Compatibility

- `inspect` is not a reserved keyword in the grammar — it is handled as a special builtin in the interpreter's FnCall dispatch. If a user has a pattern named `inspect`, the builtin takes priority (same behavior as other builtin names like `len`, `contains`, etc.).
- `invoke_learnable_with_env()` gained a `pattern_name` parameter — all internal call sites updated.
- No changes to the grammar, AST, parser, or CLI.
- No new dependencies.

## Consequences

- **Positive**: Provides zero-config runtime observability for learnable patterns. Developers can monitor call volume, cache effectiveness, and adapt frequency in production and development. Integrates naturally with the `eval` harness (ADR-0050) for comprehensive quality monitoring.
- **Negative**: `inspect()` is an approximation: `avg_confidence` defaults to 1.0 because learnable patterns currently return `Value::String` (not `Value::Fluid` with explicit confidence). A future enhancement would propagate confidence from the LLM response or use Fluid-typed returns.
- **Neutral**: The stats are non-persistent and scoped to a single interpreter instance. For persistent monitoring, stats can be periodically exported via `http_post()` to an external service.
