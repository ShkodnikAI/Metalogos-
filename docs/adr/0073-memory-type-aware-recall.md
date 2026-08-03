# ADR-0073: Type-Aware Recall & Hybrid Search (Memory Phase 3)

**Status:** Accepted
**Date:** 2026-08-04
**Supersedes:** ADR-0072 (activates mem_type and FTS5 infrastructure added there)

## Context

ADR-0072 added `mem_type` field to `MemoryEntry` and FTS5 virtual table with content-synced triggers.
However, both were dead infrastructure:
- `mem_type` was stored and retrieved but never used in recall scoring or filtering.
- FTS5 table was populated via triggers but never queried — `recall()` did full table scans.
- `memorize()` callable did not accept a type argument — all entries created as untyped.
- `load_all()` had a bug: SELECT omitted `mem_type`, hardcoding `String::new()`.
- `recall()` returned only the single best match — no top-K support for context building.

Production FOSVED bot already uses a separate Python memory system (`FosvedMemory` with 13 types,
`LIKE` search, no FTS5). This ADR brings equivalent capabilities into Metalogos-native memory.

## Decision

### 1. `memorize()` gains 3rd argument: type

```mlog
// Before: memorize("user likes spicy food", 0.9)
// After:  memorize("user likes spicy food", 0.9, "persona")
```

Argument parsing in `invoke_memorize_fn()`:
- `args[0]` = text (String, required)
- `args[1]` = priority (Float, optional, default 1.0)
- `args[2]` = mem_type (String, optional, default "")

Fully backward compatible: existing `memorize("text")` and `memorize("text", 0.9)` continue to work.

### 2. New builtin: `recall_top_k()`

```mlog
let results = recall_top_k("API key")           // top 5, any type
let results = recall_top_k("preferences", 10)    // top 10, any type
let results = recall_top_k("food", 5, "persona") // top 5, persona type only
```

Returns JSON array of objects:
```json
[
  {"value":"user likes spicy food","score":0.8542,"type":"persona","priority":0.90},
  {"value":"user hates cold soup","score":0.6231,"type":"persona","priority":0.70}
]
```

Dispatched from both `invoke()` (flow-step context) and `eval_expr_with_env()` (expression context).

### 3. `recall_top_k` on MemoryStore trait

Added as a trait method with default implementation (InMemoryStore path: scan all_entries + in-Rust scoring).
SqliteStore overrides with FTS5 BM25 + cosine hybrid.

```rust
fn recall_top_k(
    &self, query: &str, query_embedding: &[f32],
    min_confidence: f32, limit: usize, type_filter: &str,
) -> Vec<(MemoryEntry, f32)>;
```

### 4. SqliteStore: FTS5 BM25 + cosine hybrid

The override implementation in SqliteStore:
1. **BM25 candidates**: `SELECT rowid, rank FROM memories_fts WHERE memories_fts MATCH ?` — keyword matching via FTS5. Optional `AND mem_type = ?` for type filtering.
2. **Cosine scoring**: For each BM25 candidate, compute cosine similarity between query and entry embeddings.
3. **Fallback**: If no BM25 hits (e.g., empty query), fall back to cosine/substring scan of all entries.
4. **Merge**: Weighted blend — 40% BM25 + 60% cosine*decay*priority. When only cosine matches exist (no BM25), use cosine-only score.
5. **Top-K**: Sort by combined score descending, truncate to limit.

Scoring formula:
```
bm25_score > 0:
  combined = 0.4 * normalize(bm25) + 0.6 * cosine * exp(-decay_rate * age_days) * priority
bm25_score == 0:
  combined = cosine * exp(-decay_rate * age_days) * priority
```

### 5. Bug fix: `load_all()` missing `mem_type`

Before: `SELECT id, value, priority, confidence, decay_rate, created_at, embedding FROM memories` (7 columns)
After:  `SELECT id, value, priority, confidence, decay_rate, created_at, embedding, mem_type FROM memories` (8 columns)

The `mem_type: String::new()` hardcode was replaced with `row.get::<_, String>(7).unwrap_or_default()`.

This was a latent bug — `decay()` uses `load_all()` but only writes back `priority`, so no data corruption occurred. However, future code using `load_all()` results would lose type information.

### 6. Declaration-level `memorize` unchanged

The AST-level `memorize "text" with priority=0.5` (top-level declaration) still creates untyped entries.
Extending the declaration syntax to `memorize "text" with priority=0.5 type="persona"` requires
parser/compiler changes and is deferred to a follow-up ADR.

## Consequences

- **+**: `memorize("text", 0.9, "persona")` — typed memory creation from .mlog code
- **+**: `recall_top_k("query", 5, "fact")` — type-filtered recall returning top-K results
- **+**: FTS5 BM25 finally queried — keyword search for short queries like "API key"
- **+**: Hybrid BM25+cosine scoring — both exact keyword matches and semantic similarity contribute
- **+**: `load_all()` bug fixed — mem_type preserved during decay operations
- **+**: Fully backward compatible — no breaking changes to existing .mlog code
- **-**: No compilation verification (no Rust toolchain in sandbox)
- **-**: Declaration-level `memorize` still untyped (parser extension needed)
- **-**: BM25 weights (40/60 blend) are hardcoded — future: configurable per type
- **-**: `recall_top_k` default implementation on InMemoryStore scans all entries (O(n))

## Future Phases

- **Phase 3b**: Type-aware decay rates — persona=0.001, episodic=0.05, instruction=0.005
- **Phase 4**: L2 scenario grouping — aggregate L1 facts into scenario .md files
- **Phase 5**: Full RRF merge with rank-based scoring (k=60)
- **Phase 6**: L3 persona aggregation — consolidate persona-type records
