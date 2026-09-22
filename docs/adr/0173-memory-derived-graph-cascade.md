# ADR-0173: The derived-from graph and cascading forgetting over Memory<K>

**Status:** Accepted
**Date:** 2026-09-22
**Naryad:** #351 (issue #594; dispatch #598, wave 9 — registry Phase 4 "Always-on, память, забывание")
**Pillar:** memory (cross-cutting: typed containers + grants + ledger + persistence)
**Consumers:** №350 (`Memory<K>` — the carrier this graph is built over; `derived_from` was landed there as the raw material), №425 (post-wave doc sync — the narrative REFERENCE sweep stays there), the office forget-workflows (the ADR-0155 grant path is the operator-facing deletion capability)

## 1. Context

`memory_forget` (№280) is controlled forgetting WITHOUT a derivative cascade: it forgets exactly the ids an operator pinned, in the vec0-lane (the embedding store), with the dry-run→ids→apply discipline. The typed lane (№350, `src/memory_typed.rs`) stores a `derived_from: Vec<String>` on every entry — fail-closed parents validated at `put` — and its module doc explicitly deferred the graph: "This is the raw material of the derived-graph + cascade naryad №351 (the graph itself is NOT built here)."

Two debts converge here. First, the Phase 8 "real DB" item: the typed lane is in-process only; a restart loses the containers. The legacy memory lane has file persistence (ADR-0041, Phase 7.6 `SqliteStore`), and the tree already carries rusqlite 0.40 bundled (the consent ledger, the FTS5 recall store ADR-0093) — sqlx is absent from the tree, so the registry plan's "SQLite/Postgres via sqlx" wording is re-stored by the snapshot reality (§16.9-бис): the real DB is a rusqlite persistent store; Postgres remains a loud boundary. Second, forgetting a derived entry without its derivatives contradicts provenance integrity: a summary that survives while its source is deleted is a lie the ledger cannot see.

Graph precedent in-tree: the perception provenance chain (№241, ADR-0125, `src/vision/provenance.rs`) — an append-only origin chain, not a queryable DAG; the legacy memory lane's knowledge graph (`src/memory_graph.rs`) is a petgraph-backed KG over the OTHER (memorize/recall) lane. Neither is the derived-from cascade this ADR builds.

## 2. Decision Drivers

1. **Provenance integrity is the invariant, not a nicety.** After any forgetting, every surviving entry's `derived_from` parents must themselves survive. A forget that dangles a survivor's provenance is refused BY CONSTRUCTION, not by a cleanup pass.
2. **Deletion is a capability, not a right.** The forget-cascade is an irreversible delete-class action (№316 classification): without a grant it is a typed error; with a grant it consumes (ADR-0155 algebra) and lands a post-success Action-Ledger record (ADR-0167 §3.4 — the `db_execute_with_grant` template, gh#489 dogfood precedent).
3. **The cascade must cost O(derivatives), not O(container).** A flat scan per forget would make forgetting accidentally quadratic in office workloads; the closure walks a maintained children index.
4. **Persistence must survive a restart without new crypto.** The at-rest discipline is №350's: per-subject AES-256-GCM blobs under a key derived from the process master. The store persists BLOBS, never plaintext; restart-stable decryption therefore requires the `METALOGOS_MEMORY_MASTER` anchor (loud otherwise — the №350 posture, pinned here as the persistent-store requirement).
5. **No new dependencies.** The cascade is a BFS over a `HashMap` children index — petgraph (legacy lane) and SQL recursive CTEs are both rejected for the in-memory lane: the pure plan function must be fuzzable with no DB and no graph crate in the typed path.
6. **Preview before apply.** The №280 discipline (dry-run → pinned ids → apply) transfers: the cascade is computable read-only BEFORE any grant is consumed.

## 3. Decision

### 3.1 Graph model

Nodes are entries `(container, key)`; edges are `derived_from` (child → parents), a DAG by construction — `memory_put` validates every parent exists BEFORE the child is inserted, so no cycle can ever be formed through the language surface. Multi-parent entries are №350's existing `Vec<String>`. The graph is per-container (edges never cross containers — №350 validated parents "inside the same container").

### 3.2 Adjacency index and closure

Each container maintains an incremental children index `HashMap<key, Vec<key>>` (parent → direct derivations), updated at `put` (append on insert; the overwritten key's stale out-edges are removed first) and rebuilt from the stored edges on DB load. The cascade closure of a root is a BFS over this index: **O(|closure| + |edges inside the closure|)** — the "O(производных)" registry requirement. The plan function is PURE (nodes/edges/retained in, plan out) so the fuzzer and the differential model can drive it without a registry, a DB, or locks.

### 3.3 Retain: a cascading pin whose protection is a deletion VETO

`retain(key)` pins the DESCENDANT CLOSURE of the key (the key included — the CASCADE retain: "retain с каскадом" over the children index); `release(key)` unpins the same closure (the surgical inverse; both audited, both idempotent, both reversible; a pin survives value overwrites — it belongs to the node identity, not the value; nodes put AFTER a retain are not auto-pinned — the pin set is an explicit materialized snapshot, not an implicit rule). The protection semantics of the FORGET are all-or-nothing: **a retained node inside a forget closure is a deletion VETO** — `forget_cascade` refuses loudly (`MEMORY_RETAIN_PROTECTED`, naming every pinned node in the closure), deletes nothing, consumes no grant. The delete set, when unblocked, is the FULL closure.

The cut-point alternative (a retained node spares only its descendant subtree while the rest of the closure dies) was REJECTED with a proof: descendant-closure is closed under CHILD-links, not parent-links — a survivor inside `subtree(r)` can have a parent in the deleted part (retain the leaf `report`, forget its parent `summary` → `report` survives with a dangling `derived_from`), which violates driver 1. Extending the spared set upward to keep P1 either still deletes the requested root's ancestors incoherently or degenerates; the veto is the only semantics where "forget the root" behaves coherently (the root always dies, or the whole forget is refused). This is the GC-root posture: pins are fail-closed obstacles an operator sees in the preview and releases deliberately. Invariants, fuzz-pinned against an independent model:

- **P1 provenance integrity** — after the forget, every surviving entry's parents all survive. Proof: the delete set is the full closure; a node outside `closure(root)` cannot have a parent inside it (a parent of a non-descendant would make it a descendant); nothing inside survives, so no survivor's parent is deleted.
- **P2 completeness** — when unblocked, every node of `closure(root)` is deleted (no partial ghosts).
- **P3 isolation** — nothing outside `closure(root)` changes.

### 3.4 forget_cascade — the ADR-0155 linear action

`memory_forget_cascade(handle, key, grant) -> Struct{root, deleted, batch_id}`. The grant is REQUIRED: calling without a `Value::Grant` handle is `GRANT_MISSING` (the ADR-0155 §3.5 vocabulary — never a panic, never a silent no-op). Enforcement order: `check_active` (state/TTL/revocation/exhaustion) → scope coverage (`scope_attenuates(grant.scope, "memory:forget:<container_id>")` — the canonical memory scope; `GRANT_SCOPE_MISMATCH` otherwise) → plan → retained-veto refusal → apply → `grant_use` (Once → consumed, N(n) → decrement, Unlimited → audited) → post-success ledger record `irreversible.memory_forget` carrying the grant id, the batch id, and a SHA-256 digest of the sorted deleted keys (the `db_execute_with_grant` template: consumption and journal are side effects of the SUCCESS path, never charged on refusal). Batch id: `MLOG-TFORGET-<16 hex>` (the №280 batch-id posture, typed-lane prefix).

### 3.5 Persistence — the Phase 8 "real DB" debt closes here

The store anchor is the env `METALOGOS_MEMORY_DB` (file path). Unset → the current in-process behavior byte-for-byte (tests and existing programs unchanged). Set → a lazily opened bundled-rusqlite file DB becomes the AUTHORITATIVE store; the in-process registry is a transparent write-through cache, and `memory_open` loads containers from the DB when they are not cached — a fresh process therefore sees the persisted state. Additive-only DDL (the ADR-0060 discipline; `CREATE TABLE IF NOT EXISTS`, no drops, no alters):

```sql
CREATE TABLE IF NOT EXISTS memtyped_containers (
  id TEXT PRIMARY KEY, subject TEXT NOT NULL,
  label TEXT NOT NULL, created_unix INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS memtyped_entries (
  container_id TEXT NOT NULL, key TEXT NOT NULL,
  stored BLOB NOT NULL, is_enc INTEGER NOT NULL,
  created_unix INTEGER NOT NULL, retained INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (container_id, key));
CREATE TABLE IF NOT EXISTS memtyped_edges (
  container_id TEXT NOT NULL, child TEXT NOT NULL, parent TEXT NOT NULL,
  PRIMARY KEY (container_id, child, parent));
```

Private entries persist as the EXACT №350 AES-GCM blobs (`is_enc` flag; public entries as UTF-8 text) — the store never sees plaintext, and restart-stable decryption requires `METALOGOS_MEMORY_MASTER` (loud stderr warning when absent, the №350 posture). The children index is rebuilt from `memtyped_edges` on load. Restart honesty: the criterion is verified by a TRUE cross-process test (the harness re-execs the test binary as a child against the same DB file twice — write, then read), not only by a cache-reset simulation.

### 3.6 Surfaces (registry 473→478, append-only)

| Builtin | Arity | Class (№316) | Semantics |
|---|---|---|---|
| `memory_cascade_preview(handle, key)` | 2 | Source/Internal/Pure | read-only plan: `{closure, blocked_by}` — the №280 dry-run discipline; no grant touched; when unblocked, the would-delete set IS the closure |
| `memory_retain(handle, key)` | 2 | Sink/Internal/Reversible | pin the descendant closure of the key (the CASCADE retain); audited |
| `memory_release(handle, key)` | 2 | Sink/Internal/Reversible | unpin the descendant closure (the surgical inverse); audited, idempotent |
| `memory_retained(handle)` | 1 | Source/Internal/Pure | list pinned keys (introspection) |
| `memory_forget_cascade(handle, key, grant)` | 3 | Sink/Internal/Irreversible | §3.4 — the grant-gated, ledger-recorded cascade delete; the retained veto refuses before anything is deleted |

`handle` is the opaque `Value::Memory` handle (№350). Unknown handle/key refuse with the typed `MEMORY_UNKNOWN`/`MEMORY_UNKNOWN_KEY` errors. REFERENCE §6/§7 are regenerated mechanically (the counts move with the registry — the readme/REFERENCE consistency contracts demand it); the narrative doc sweep stays with №425 per the wave rule.

### 3.7 Verification: two-tier fuzzing + deterministic O() proof

Tier 1 (BLOCKING, the grant_algebra_fuzz pattern): a deterministic xorshift-driven differential test builds random DAGs, random pins, random forget roots; the REAL plan output must equal an independently derived model on P1/P2/P3 and the veto rule, and the visited counters must be EXACTLY |closure| node pops + the closure's internal edge scans — the O(производных) claim pinned deterministically, no wall-clock flakiness. Tier 2 (fuzz-smoke, non-blocking): `fuzz/fuzz_targets/fuzz_target_memory_cascade.rs` (the `fuzz_target_ledger_tamper` precedent) drives the same pure plan with arbitrary bytes — panic-freedom plus the model invariants; the CI job runs it 120 s and stays non-blocking (the loud note lives in the completion report).

## 4. Consequences

- The Phase 8 "real DB" registry debt CLOSES for the typed lane: containers, entries, pins and edges survive restarts behind `METALOGOS_MEMORY_DB`; the office contour can anchor forgetting workflows on a file store without any new dependency.
- Deletion in the typed lane becomes a granted, ledgered, previewable operation — the operator story matches the vec-lane's №280 discipline and the grant algebra's refusal matrix.
- Postgres/sqlx stays a LOUD boundary (the path is this ADR, not a naryad): no new heavy dependencies enter the tree.
- Untouched by decision: the consent_ledger store (№350 only reads it), FTS5 recall (ADR-0093/0094), SMFS (ADR-0139), the legacy petgraph KG lane, the №398 thresholds, `memory_forget` №280.
- Honest boundaries: the DB store is single-writer by posture (SQLite's own locking is the concurrency story; no WAL tuning — the office agent is one process); `derived_from` parents are container-local keys, so cross-container derivation is refused exactly as №350 refused it; a WITHOUT-master restart persists blobs that cannot be decrypted (the data survives, the keys do not — loud, the №350 posture).
