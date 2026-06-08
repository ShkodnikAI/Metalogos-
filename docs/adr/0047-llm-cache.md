# ADR-0047: LLM Response Caching for Learnable Patterns

**Status:** Implemented
**Date:** 2026-06-08

## Context

Learnable patterns call an LLM on every invocation. For applications that make repeated calls with the same inputs (e.g., classification of recurring messages, repeated translations), this results in redundant LLM API calls with associated latency and cost. A response caching mechanism allows identical (prompt + args) calls to return the cached response without hitting the LLM backend.

## Decision

Add two optional fields to `learnable pattern` declarations:

```mlog
learnable pattern Translate(text: String) -> String {
  prompt: "Translate to French."
  cache: true
  cache_ttl: 60.minutes
}
```

### Semantics

1. **`cache: true/false`**: Enables or disables response caching for this pattern. Default is `false` (backward compatible — no existing patterns are cached).

2. **`cache_ttl: N.minutes`**: Time-to-live for cached entries. Supports units: `seconds`, `minutes`, `hours`, `days`. Default is 3600 (1 hour) when `cache: true` is set without an explicit TTL.

3. **Cache key**: `hash(effective_prompt + input)` using Rust's `DefaultHasher` (SipHash). The effective prompt includes any context auto-loading from `context: recall(...)`.

4. **In-memory cache**: `HashMap<u64, LlmCacheEntry>` protected by `Mutex`. On cache hit (key exists and TTL not expired), the cached response is returned directly without calling the LLM. On TTL expiry, the entry is evicted and the LLM is called again.

5. **SQLite persistence**: When `memory { persist: "path" }` is enabled, cache entries are also written to an `llm_cache` table in the same SQLite database:
   ```sql
   CREATE TABLE llm_cache (
     hash INTEGER PRIMARY KEY,
     response TEXT NOT NULL,
     created_at INTEGER NOT NULL,
     ttl INTEGER NOT NULL
   );
   ```
   This enables cache survival across interpreter restarts (e.g., server hot-reloads).

6. **Cache checking order**: The cache is checked after few-shot matching but before the LLM call. This means few-shot exact matches still take priority over the cache.

### Implementation

- **Grammar**: `cache_line` (`cache: true/false`) and `cache_ttl_line` (`cache_ttl: N.minutes`) rules in `learnable_body`.
- **AST**: `LearnablePatternDecl` gains `cache: bool` and `cache_ttl: u64` fields.
- **Parser**: Extracts cache boolean and TTL with unit conversion (minutes → seconds, etc.).
- **Interpreter**:
  - `LlmCacheEntry` struct stores response, created_at timestamp, and TTL.
  - `llm_cache: Mutex<HashMap<u64, LlmCacheEntry>>` field on `Interpreter`.
  - `compute_cache_key()` hashes effective_prompt + input.
  - `llm_cache_get()` checks TTL, evicts expired entries, parses JSON responses.
  - `llm_cache_persist()` writes to SQLite when persistence is enabled.
  - `invoke_learnable_with_env()` checks cache before LLM call, stores after.
  - `CompiledLearnable` and both registration sites updated.

### Backward Compatibility

- `cache` defaults to `false` — no existing patterns are affected.
- `cache_ttl` defaults to 3600 seconds (1 hour) when cache is enabled.
- The `LlmBackend::call()` trait is unchanged.
- MockLlm behavior unchanged — returns the effective prompt, which is the same on cache hit (consistent result).

## Consequences

- **Positive**: Eliminates redundant LLM calls for repeated inputs. Reduces latency (instant cache hit vs network round-trip) and cost (fewer API tokens consumed). SQLite persistence enables cross-restart caching.
- **Negative**: In-memory cache grows without bound (no eviction policy beyond TTL). For very large caches, memory usage could become significant. A future enhancement could add a max-entries LRU eviction.
- **Neutral**: The SQLite `llm_cache` table is created lazily on first cache write. If persistence is not enabled, no SQLite overhead occurs.
