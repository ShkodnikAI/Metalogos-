# ADR-0041: Memory Persistence via SQLite (Phase 7.6)

## Status: Accepted

## Context

Memory in METALOGOS (ADR-0004, ADR-0012) was entirely in-process — `memorize` stored facts in a
`Vec<MemoryEntry>`, and all data was lost on interpreter shutdown. ADR-0004 explicitly noted:
*"Memory is in-process only — no persistence across executions."*

For production use-cases (chatbots, long-running agents, knowledge assistants), memory must survive
process restarts. Additionally, the knowledge graph (ADR-0014) suffered the same limitation:
`relate` edges were lost on every restart.

Previous phases added SQLite for sessions (Phase 7.4) but memory remained in-memory.

## Decision

### 1. MemoryStore Trait (abstracts backend)

Created `src/memory_store.rs` with a trait-based architecture:

```rust
pub trait MemoryStore: Send + Sync {
    fn memorize(&mut self, entry: MemoryEntry) -> Result<i64, String>;
    fn recall(&self, query: &str, query_embedding: &[f32], min_confidence: f32) -> Option<(MemoryEntry, f32)>;
    fn forget(&mut self, query: &str, cutoff: i64);
    fn decay(&mut self) -> usize;
    fn all_entries(&self) -> Vec<MemoryEntry>;
    fn count(&self) -> usize;
}
```

Two implementations:
- **InMemoryStore**: `Vec<MemoryEntry>` — identical to pre-7.6 behavior, backward compatible
- **SqliteStore**: SQLite-backed with `std::sync::Mutex<Connection>` for thread safety

### 2. KgStore Trait (abstracts knowledge graph)

```rust
pub trait KgStore: Send + Sync {
    fn relate(&mut self, from: &str, to: &str, relation: &str, weight: f64) -> Result<(), String>;
    fn edges_for(&self, value: &str) -> Vec<(String, String, f64)>;
    fn walk(&self, value: &str, max_depth: usize) -> Vec<(String, String, f64)>;
    fn edge_count(&self) -> usize;
    fn all_edges(&self) -> Vec<(String, String, String, f64)>;
}
```

Two implementations:
- **InMemoryKg**: `Vec<(from, to, relation, weight)>` — backward compatible
- **SqliteKg**: SQLite-backed with `kg_nodes` + `kg_edges` tables, node deduplication

### 3. SQLite Schema

```sql
CREATE TABLE memories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key TEXT,
    value TEXT NOT NULL,
    priority REAL NOT NULL DEFAULT 1.0,
    confidence REAL NOT NULL DEFAULT 1.0,
    decay_rate REAL NOT NULL DEFAULT 0.01,
    created_at INTEGER NOT NULL,
    embedding BLOB
);
CREATE INDEX idx_memories_value ON memories(value);
CREATE INDEX idx_memories_created ON memories(created_at);

CREATE TABLE kg_nodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    value TEXT NOT NULL UNIQUE,
    type TEXT NOT NULL DEFAULT 'fact'
);
CREATE TABLE kg_edges (
    from_id INTEGER NOT NULL REFERENCES kg_nodes(id),
    to_id INTEGER NOT NULL REFERENCES kg_nodes(id),
    relation TEXT NOT NULL,
    weight REAL NOT NULL DEFAULT 1.0
);
CREATE INDEX idx_kg_nodes_value ON kg_nodes(value);
CREATE INDEX idx_kg_edges_from ON kg_edges(from_id);
CREATE INDEX idx_kg_edges_to ON kg_edges(to_id);
CREATE INDEX idx_kg_edges_rel ON kg_edges(relation);
```

Embedding vectors are serialized as little-endian `f32` bytes (4 bytes per dimension) in the
`embedding BLOB` column, enabling efficient cosine similarity recall.

### 4. Decay Formula

```
activation = priority * exp(-decay_rate * age_days)
```

Implemented both in-memory (Rust) and SQLite (`exp()` function). The `decay()` method updates
`priority` in-place for all entries.

### 5. Configuration

```mlog
memory { persist: "./data/memory.db" }
```

- With `persist`: switches to `SqliteStore` + `SqliteKg`, auto-creates directories and tables
- Without `persist`: keeps `InMemoryStore` + `InMemoryKg` (default, all old tests pass)
- Existing in-memory data is migrated to SQLite on config declaration
- Graceful fallback: if SQLite open fails, keeps in-memory with a warning

### 6. Thread Safety

Both `SqliteStore` and `SqliteKg` use `std::sync::Mutex<rusqlite::Connection>`:
- Safe for `Send + Sync` requirement (interpreter lives in `Arc<RwLock<>>` on server)
- Each store owns its own connection (no cross-store locking)
- KG shares the same database file as memories via separate connection

## Contract Tests (8 tests)

| Contract | Test |
|----------|------|
| SQLite memorize + recall | `test_76_sqlite_memorize_and_recall` |
| Persistence across reopen | `test_76_persistence_across_restart` |
| In-memory default (backward compat) | `test_76_inmemory_default_no_persist` |
| Decay formula correctness | `test_76_decay_formula` |
| Forget removes entries | `test_76_forget_removes_entries` |
| KG persist + walk | `test_76_kg_persistence_and_walk` |
| Embedding roundtrip via BLOB | `test_76_embedding_blob_roundtrip` |
| No persist → data lost on restart | `test_76_no_persist_data_lost` |

## Consequences

- No breaking changes: without `memory { persist: ... }`, behavior is identical to pre-7.6
- `src/memory_store.rs` is a new 1100-line module with 14 unit tests
- `Interpreter` now uses `Box<dyn MemoryStore>` and `Box<dyn KgStore>` (trait objects)
- SQLite file is auto-created on first `memory { persist }` declaration
- KG edges are now fully migrated to SQLite alongside memories (same DB file)
