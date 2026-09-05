# ADR-0116: Weight persistence for `Reflex` — SQLite BLOB, not a new file format

**Status:** Accepted
**Date:** 2026-09-05
**Naryad:** #180 (blocks on this ADR)
**Pillar:** `Reflex` (eighth semantic pillar)
**Relates to:** `ADR-0114` (opaque handle), `наряд №178` (binary
serialization format already implemented in `src/nn/serde_weights.rs`)

## Context

Trained `Reflex` weights must survive process restarts — a model
trained once should not require retraining every time `mlog serve`
restarts. The binary serialization format itself (`REFLEX_MAGIC`,
`REFLEX_VERSION`, length-prefixed per-layer blobs) already exists,
built in наряд №178 ahead of this ADR. What remains is **where that
byte blob lives** and **how `.mlog` source accesses it**.

Two storage locations were considered.

### Option A — a dedicated `.mlm` (or similar) file format

Each model saved as its own file, application manages paths.

**Rejected.** The project already carries one versionable binary
format end-to-end (`.mbc` bytecode, `ADR-0146`-era migration work —
size limits, version checks, real test coverage for corrupted input).
A second, independent binary file format duplicates that entire
concern (path traversal risk identical to naряд №150's `mlog check`
audit finding for `model_load` in the original distillation draft,
version-skew handling, corruption handling) for no benefit — the
data is not large enough or accessed differently enough to justify a
second format.

### Option B — SQLite BLOB in the existing memory store

Weights live in a table in the same SQLite database
`memory_store.rs` already opens and manages (`memories`, `kg_nodes`,
`kg_edges` tables already follow this pattern).

**Accepted.**

## Decision

New table, same database as `memory_store.rs`:

```sql
CREATE TABLE IF NOT EXISTS reflex_models (
  name        TEXT PRIMARY KEY,
  weights     BLOB NOT NULL,      -- output of serialize_model()
  input_size  INTEGER NOT NULL,
  labels      TEXT NOT NULL,      -- JSON array
  seed        INTEGER NOT NULL,
  last_metric REAL,
  updated_at  INTEGER NOT NULL
);
```

`weights` is exactly the byte vector `serialize_model()`
(`src/nn/serde_weights.rs`, наряд №178) already produces — no new
serialization logic, this ADR decides storage, not format.

**Builtins:**
```
reflex_save(model)               -- writes current weights to reflex_models
reflex_load(name) -> Value::Reflex  -- reads and reconstructs a ReflexModel
```

`reflex_load` on a version mismatch (`REFLEX_VERSION` in the stored
blob differs from the running binary's constant) fails with an
explicit, actionable error — the same principle naряд №146 applied to
`.mbc`: a stale format is a loud failure, never a silent
misinterpretation of bytes.

## Consequences

- No new file-format surface, no new path-traversal concern — SQLite
  access already goes through `memory_store.rs`'s existing,
  audited connection handling.
- A `Reflex` model's row is keyed by `name` (the identifier in the
  `reflex` declaration) — saving twice under the same name overwrites,
  matching how `memorize`/`recall` already behave for ordinary memory
  entries. No versioned-history-of-a-single-model in this ADR; if that
  becomes a real need later, it is a new column/table, not a reason to
  revisit the storage choice made here.
- `reflex_save`/`reflex_load` are the only new builtins this ADR
  requires — everything else (the binary format itself, the
  `ReflexModel`/`ReflexRegistry` types) already exists from earlier
  stages.
