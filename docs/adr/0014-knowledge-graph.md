# ADR-0014: Knowledge Graph (Semantic Memory via Graph Structure)

**Status:** Implemented (Phase 2 Final)
**Date:** 2026-06-01
**Milestone:** Phase 2

---

## Context

Before this ADR, Metalogos memory was a flat list of entries (`memorize`/`recall`/`forget`).
`recall` found entries by substring matching and decay. There was no way to represent
relationships between facts — "Alice works at Acme" and "Bob works at Acme" were
independent strings with no connection, even though a human would immediately see
they share an employer.

The user requirement: `memory { semantic: { structure: KnowledgeGraph } }` — entities
connected through Relations, recall walks the graph.

## Prior Art

| Approach | Source | Trade-off |
|---|---|---|
| Flat key-value store | Redis, memcached | Fast, no relationships |
| Relational DB (SQL) | PostgreSQL, MySQL | Schema required, overhead |
| Property graph | Neo4j, JanusGraph | Full graph queries, external dependency |
| RDF/SPARQL | W3C semantic web | Standards-heavy, complex |
| Embedded adjacency list | In-memory graph (adjacency list) | Simple, no dependencies, limited scale |
| Conceptual graph | Sowa (1984) | Nodes + relations, reasoning support |

## Decision

### `relate` Declaration

New syntax for creating edges between memories:

```mlog
relate "Alice works at Acme" to "Bob works at Acme" as "coworker"
```

Three parts:
- **from**: expression evaluating to a string (matches a memorized fact)
- **to**: expression evaluating to a string (matches a memorized fact)
- **as**: relation type (string literal)

### In-Memory Graph Store

Simple adjacency list — no Neo4j, no external dependency:

```rust
struct Relation {
    from: String,
    to: String,
    relation: String,
    timestamp: i64,
}
```

Stored in `Interpreter.relations: Vec<Relation>`. Linear scan for graph traversal.
For the expected scale (dozens to hundreds of entries in .mlog programs), this is
sufficient. Neo4j migration deferred to Phase 3.

### Graph-Aware Recall

When `recall` finds a best match, it walks the graph to find all relations where
either `from` or `to` matches the found memory value. Related facts are appended
as `[GRAPH] relation_type -> related_value` lines:

```
Alice works at Acme Corp
[GRAPH] coworker -> Bob works at Acme Corp
```

This makes recall return both the direct match and its graph neighborhood, giving
the flow/pattern access to implicit connections.

### Limitations

1. **String matching only.** Relations are matched by exact string comparison
   between `from`/`to` and memory values. "Alice works at Acme" must match exactly.
2. **No transitive traversal.** `A → B → C` is not followed; only direct edges
   from the matched node are returned.
3. **No graph queries.** No Cypher/Gremlin. Just "find neighbors of X."
4. **Linear scan.** O(n) per recall for graph traversal. Adequate for <1000 edges.
5. **No persistence.** Relations live in memory. Serde persistence deferred.

## Rationale

- **Why adjacency list over property graph?** Adjacency list is the simplest
  data structure that captures edges. No schema, no indexing, no external process.
  For .mlog programs with tens of facts, a Vec<Relation> with linear scan is
  fast enough and zero-dependency.
- **Why `[GRAPH]` prefix?** Distinguishes graph-walk results from the direct
  recall match, making it clear to patterns/flows which facts are primary vs.
  discovered through relationships.
- **Why not embed relations in MemoryEntry?** Keeping relations separate allows
  `relate` to connect any two facts without modifying the entries themselves.
  It also makes forget/cleanup simpler — entries can be forgotten without
  orphaning graph data.

## Impact

- **`grammar.pest`:** New `relate_decl`, `RELATE_KW` keyword.
- **`ast.rs`:** `RelateDecl` struct, `Declaration::Relate` variant.
- **`parser.rs`:** `parse_relate_decl()`.
- **`interpreter.rs`:** `Relation` struct, `relations: Vec<Relation>` field,
  `invoke_recall()` modified for graph walk.
- **Backward compatible.** All 6 existing golden tests pass.
- **New test:** `p2_knowledge_graph.mlog` — relate + graph walk in recall.
