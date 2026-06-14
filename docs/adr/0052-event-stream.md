# ADR-0052: Event Stream — Unified Log of All Operations

**Status:** Implemented
**Date:** 2026-06-11

## Context

As Metalogos programs grow in complexity — with learnable patterns making LLM calls, memory operations, rule engines, and HTTP handlers — there is no unified mechanism to observe what the interpreter is doing. The existing `audit_log` (Phase 7.5) is a flat `Vec<String>` with no structure, no timestamps, and no query API. `PatternStats` (ADR-0051) provides aggregate counters but not a time-ordered event log. For debugging, monitoring, audit compliance, and the Fosved Office learning loop, developers need a structured, queryable stream of all interpreter operations.

**Prior art:**
- **OpenHands Event Stream**: structured JSON events for every agent action, with types, sources, and metadata.
- **Temporal.io Event Sourcing**: append-only event log as the source of truth for workflow state.
- **Redux (single source of truth)**: every state mutation is described by an action with type and payload.

## Decision

Add a structured event stream to the interpreter. Every significant operation emits an `Event` with an auto-incrementing ID, Unix timestamp, type, source, arbitrary key-value data, and optional duration.

### Event Struct

```rust
Event {
    id: u64,                          // auto-increment
    timestamp: u64,                   // Unix milliseconds
    event_type: String,               // "pattern_call", "memory_store", etc.
    source: String,                   // pattern name or "system"
    data: HashMap<String, String>,    // arbitrary key-value metadata
    duration_ms: Option<u64>,         // measured latency
}
```

### Event Types

| event_type | source | data fields | instrumentation point |
|---|---|---|---|
| `memory_store` | `"system"` | `key_preview`, `priority` | `Declaration::Memorize` in `run()` and `load_module_inner()` |
| `memory_recall` | `"system"` | `query`, `results_count`, `best_score` | `invoke_recall()` (future) |
| `adapt` | pattern name | `pattern`, `action`, `examples_count` | `Declaration::Adapt` in `run()` |
| `pattern_call` | pattern name | `name`, `cache_hit` | `record_pattern_call()` (covers learnable + regular patterns) |
| `rule_fire` | rule text | `rule_text`, `priority` | rule evaluation (future) |
| `llm_call` | pattern name | `provider`, `model`, `latency_ms` | `invoke_learnable_with_env()` (future) |
| `error` | source | `message`, `context` | error handling (future) |

Events marked "future" are designed for but not yet instrumented. The infrastructure supports them.

### Storage

- **In-memory**: `event_log: Mutex<Vec<Event>>` on `Interpreter`. Thread-safe via Mutex.
- **Future**: SQLite table `events (id, timestamp, type, source, data_json, duration_ms)` when memory persist is enabled.

### Builtins for Querying

Three builtins provide read access to the event stream (all special-cased in FnCall dispatch because they need interpreter context):

1. **`event_count()`** → `Float`: Total number of events.
2. **`event_count(type: String)`** → `Float`: Count of events matching the given type.
3. **`events_since(seconds: Float)`** → `List<Struct>`: Events emitted in the last N seconds. Each struct has fields: `id`, `timestamp`, `event_type`, `source`, `data_json`, `duration_ms`.
4. **`event_sum(type: String, field: String)`** → `Float`: Sum of a numeric `data` field across all events of a given type. Useful for aggregating costs, latencies, priorities.

### Public Rust API

For programmatic/test access:
- `event_count(type: Option<&str>) -> usize`
- `events_since_ms(since_ms: u64) -> Vec<Event>`
- `get_events() -> Vec<Event>` (returns full log snapshot)
- `event_sum(type: &str, field: &str) -> f64`

### Implementation

- **Event struct**: Defined in `interpreter.rs` with `serde::Serialize` for future persistence.
- **Interpreter fields**: `event_log: Mutex<Vec<Event>>`, `event_next_id: AtomicU64`.
- **`emit_event()`**: Thread-safe method that increments ID, captures timestamp, appends to log.
- **Instrumentation**: Calls to `emit_event()` inserted at key operation sites in `run()`, `load_module_inner()`, and `record_pattern_call()`.
- **FnCall dispatch**: `event_count`, `events_since`, `event_sum` intercepted before generic builtin dispatch (like `inspect`, `query`).
- **No grammar/AST/parser changes**: The event stream is purely an interpreter-internal mechanism. No new .mlog syntax.

### Backward Compatibility

- No changes to the grammar, AST, parser, or CLI.
- The event stream is invisible to .mlog programs that don't call `event_count`/`events_since`/`event_sum`.
- Event emission is automatic but adds negligible overhead (HashMap insert + Mutex lock per event).
- If `event_count`/`events_since`/`event_sum` collide with user-defined pattern names, the builtins take priority (same as all other builtins).

## Consequences

- **Positive**: Provides a unified, structured, queryable log of all interpreter operations. The auto-increment IDs enable correlation. The `event_sum` builtin enables aggregation (e.g., total LLM cost, total memory priority). This is the foundation for audit trails, metrics dashboards, and the Fosved Office learning loop.
- **Negative**: In-memory only for now — events are lost on interpreter restart. SQLite persistence is deferred. Some event types (llm_call, rule_fire, error) are designed but not yet instrumented. The `data` field is `HashMap<String, String>` — numeric values must be parsed back when summing.
- **Neutral**: Event emission adds a small per-operation overhead (Mutex lock + Vec push). For typical workloads (< 10K events), this is negligible. For high-throughput scenarios, a ring buffer or async channel could replace the Vec.
