# ADR-0008: Entity Store

**Status:** Implemented
**Date:** 2026-05-31
**Milestone:** P1 (Phase 1 — Type System)

---

## Context

Before this ADR, entities in Metalogos were stored as individual entries in a flat `HashMap<String, Value>`.
The only way to access an entity was by its variable name (`m1`, `m2`, etc.). This is equivalent to a
simple key-value store — it works for direct references but provides no way to:

1. **Query entities by predicate** — "find all Messages with urgency > 0.5"
2. **Count entities by type** — "how many Message instances exist?"
3. **Establish identity** — entities have no first-class identity beyond their variable name

The user requirement: "Entity Store — это хранилище с идентичностью, а не просто HashMap."

Question: what is the minimal runtime structure that gives entities identity, queryability, and a clear
distinction from a flat variable map?

## Prior Art

| Approach | Source | Trade-off |
|---|---|---|
| Relational table with index | SQL databases | Powerful queries, but heavyweight; requires schema migration |
| Document store with collection | MongoDB, Firestore | Schema-flexible, collection-scoped queries |
| In-memory entity index with identity | ECS (Entity-Component-System), DDD (Domain-Driven Design) | Lightweight, type-scoped, identity-first |
| HashMap with metadata | Current Metalogos `variables` | Too flat — no collection semantics |

## Decision

**Entity Store = type-scoped index into `variables`.** A lightweight `HashMap<String, Vec<EntityRecord>>`
maps type names to lists of entity identity records. The records hold only the entity ID (variable name)
and type — actual values remain in `variables` and are looked up by ID at query time.

### Data structure

```rust
struct EntityRecord {
    id: String,        // variable name (identity)
    type_name: String, // struct type name
}

entity_store: HashMap<String, Vec<EntityRecord>>  // type → [records]
```

### CRUD operations

| Operation | Mechanism | Example |
|---|---|---|
| **Create** | `entity m1: Message = {...}` — automatically registers in store | Implicit on `EntityRecord` declaration |
| **Read (by id)** | Direct variable access via `variables["m1"]` | Unchanged from pre-store behavior |
| **Read (by predicate)** | `find("Message", "urgency", "gt", 0.5)` — scans records, checks field | New builtin-like function |
| **Update** | `rule If(...) then m1.urgency = 0.9` — mutates `variables`, store sees changes | Unchanged — store reads from `variables` |
| **Delete** | Not implemented in MVP | Future: `delete("m1")` or `forget`-style |
| **Count** | `count("Message")` — returns number of entities of given type | New builtin-like function |

### `find` semantics

```
find(type_name: String, field: String, op: String, threshold: Float) -> Value
```

1. Look up `entity_store[type_name]` → list of records
2. For each record, get current value from `variables[record.id]`
3. Access `value.field`, compare with threshold using `op` (gt/lt/ge/le/eq)
4. Return first matching entity as `Value::Struct` (clone from variables)
5. No match → `Value::Unit` (soft-failure)

The scan is linear (O(n) per type). This is intentional for MVP — no indexing needed when entity counts
are small. B-tree or hash index can be added in Phase 2 when performance becomes a concern.

### Separation of store and values

The store is an **index**, not a copy of entity data. This design:
- Avoids synchronization issues: rules mutate `variables` directly, store always sees latest state
- Keeps the store lightweight: only IDs, no deep-cloned values
- Allows the same entity to be accessed both by name (variable) and by predicate (store)

## Rationale

- **Why not a full database?** Metalogos is a language runtime, not a database. For MVP, an in-memory
  index with linear scan is sufficient. When entity counts grow, a B-tree index can be layered in
  without changing the `find` API.
- **Why index-by-type?** This is the natural query dimension: most entity queries are scoped to a type
  ("find Messages with urgency > 0.5"), not cross-type. This mirrors ECS archetypes and DDD aggregates.
- **Why `find` returns first match?** MVP simplicity. A `find_all` returning a collection requires a
  new `Value::List` type, which is a separate feature. `find` (first match) is sufficient to prove the
  concept and is backward-compatible.
- **Why no version history in MVP?** The user acknowledged this as optional. Versioning requires a
  `Vec<Value>` per entity and a `revert_to(version)` operation — substantial complexity. Deferred to
  Phase 2 when the semantics of "undo" in an AI-agent context are clearer.

## Limitations (Documented)

1. **No `delete` operation.** Entities can be created and updated but not removed from the store.
2. **No `find_all` / collection return type.** `find` returns only the first match.
3. **Linear scan.** O(n) per query; no indexing.
4. **No version history.** Changes overwrite in-place; no undo.
5. **No cross-type queries.** `find` is scoped to a single type.

These are all acceptable for MVP and have clear Phase 2 upgrade paths.

## Examples

```mlog
entity Message { text: String, urgency: Float = 0.0 }

entity m1: Message = { text: "срочно нужна помощь", urgency: 0.0 }
entity m2: Message = { text: "обычный вопрос", urgency: 0.0 }
entity m3: Message = { text: "срочно нужна консультация", urgency: 0.0 }

rule If(m1.text contains "срочно") then m1.urgency = 0.9 with priority=10

pattern GetUrgency(msg: Message) -> Float { return msg.urgency }

flow Main { input: Message = find("Message", "urgency", "gt", 0.5) -> GetUrgency -> output }
// Output: 0.9
```

Key insight: `find("Message", "urgency", "gt", 0.5)` finds m1 **without mentioning its name** — this
is what distinguishes Entity Store from a flat HashMap.

## Impact

- **`interpreter.rs`:** Added `EntityRecord` struct, `entity_store` field in `Interpreter`.
  `EntityRecord` declarations now register in the store. Added `invoke_find` and `invoke_count`
  methods. Special-cased `find`/`count` in `invoke` and `eval_expr_with_env` FnCall arm.
- **No grammar changes.** `find` and `count` are ordinary function-call expressions parsed by the
  existing `call_expr` rule.
- **No AST changes.** Entity Store is purely a runtime feature.
- **Backward compatible.** All existing tests pass. `find`/`count` are additive — no existing behavior changes.
