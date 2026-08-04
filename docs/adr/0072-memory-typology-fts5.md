# ADR-0072: Memory Typology & FTS5 Hybrid Search

**Status:** Accepted  
**Date:** 2026-08-04  

## Context

Metalogos has two memory paths:
1. **VM path** (bytecode): `VmMemoryEntry { value, priority, timestamp, decay_rate }` — in-memory Vec, substring recall
2. **Interpreter path** (Phase 7.6): `MemoryEntry` with SQLite persistence and semantic recall via embeddings

Both paths treat all memories identically — no type distinction. A user's
long-term preference ("likes spicy food") and a one-time event ("met with
Bob on Tuesday") decay at the same rate and cannot be recalled independently.

Additionally, recall uses only cosine similarity (embedding) or substring
match — no keyword/BM25 search. For short queries like "API key", BM25
outperforms embeddings which rely on distributional semantics.

TencentDB-Agent-Memory uses a 4-layer distillation pipeline (L0→L1→L2→L3)
with typed atoms and hybrid BM25+cosine+RRF search. This ADR implements
the two most impactful concepts: memory typology and FTS5/BM25.

## Decision

### 1. Memory Type Field

Added `mem_type: String` to both `VmMemoryEntry` and `MemoryEntry`.

**Type taxonomy** (inspired by TencentDB-Agent-Memory, simplified):
| Type | Description | Typical decay |
|------|-------------|---------------|
| `""` (empty) | Legacy/untyped — backward compatible | 0.01 (default) |
| `"persona"` | User profile, stable preferences, long-term traits | 0.001 (very slow) |
| `"episodic"` | Specific events, conversations, one-time occurrences | 0.05 (fast) |
| `"instruction"` | Rules, directives, standing orders | 0.005 (slow) |
| `"fact"` | Factual knowledge, objective information | 0.01 (default) |

### 2. FTS5 Full-Text Index

Added a `memories_fts` virtual table with content-synced triggers:
```sql
CREATE VIRTUAL TABLE memories_fts USING fts5(
    value,
    mem_type UNINDEXED,
    content=memories,
    content_rowid=id
);
```

Triggers on INSERT/UPDATE/DELETE keep FTS in sync with the base table.
The `mem_type` column is `UNINDEXED` — used for filtering, not BM25 scoring.

### 3. Backward Compatibility

- `mem_type` defaults to `""` in both VM and interpreter paths
- Existing SQLite databases get `mem_type TEXT NOT NULL DEFAULT ''` via ALTER
- FTS5 table is created with `IF NOT EXISTS` — no migration for existing DBs
  (existing rows are NOT retroactively indexed; only new/updated rows appear in FTS)
- All existing `.mlog` code continues to work — `memorize()` without type
  creates untyped entries

### 4. Future: Type-Aware Recall

The `mem_type` field is stored but not yet used in recall scoring.
Phase 5 (hybrid search) will use it for:
- Type-weighted decay rates (persona slower, episodic faster)
- Type-filtered recall ("recall only facts about X")
- Different search strategies per type (BM25 for facts, semantic for persona)

## Consequences

- **+**: Foundation for differentiated memory management
- **+**: FTS5 enables BM25 keyword search alongside cosine similarity
- **+**: Backward compatible — no breaking changes
- **-**: FTS5 only indexes new rows; existing data needs re-index to benefit
- **-**: `#[serde(default)]` on `MemoryEntry` is unused (no serde derives)
- **-**: Type-specific decay rates not yet implemented (data-only change)
