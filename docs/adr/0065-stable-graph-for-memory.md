# ADR-0065: Use StableDiGraph for MemoryGraph

## Status
Accepted

## Context

`MemoryGraph` stored a `petgraph::graph::DiGraph` with an external
`HashMap<String, NodeIndex>` (`id_index`) mapping node IDs to their graph
indices.

`DiGraph::remove_node` swaps the last node into the removed slot to
maintain compact storage. This invalidates all `NodeIndex` values above the
removed index. The `id_index` map continued holding stale indices for
surviving nodes, causing `get_node()` and `boost()` to return wrong data
or panic with index-out-of-bounds.

The bug manifested in `prune()`, which removes low-score nodes during
regular maintenance. Every prune corrupted the graph.

## Decision

Replace `DiGraph` with `StableDiGraph` from `petgraph::stable_graph`.
`StableDiGraph` uses a free-list for removed slots and never reassigns
indices of surviving nodes. The `id_index` map remains valid after any
sequence of removals.

## Alternatives considered

1. **Rebuild `id_index` after every removal** — O(n) per deletion, and
   easy to forget at every call site. One missed rebuild = silent corruption.

2. **StableDiGraph (chosen)** — O(1) removal, no bookkeeping required.
   Slightly higher memory usage (free-list), negligible for the expected
   graph size (hundreds to low thousands of nodes).

3. **Use node IDs as keys everywhere, never cache NodeIndex** — would
   require O(log n) or O(n) lookup at every graph operation instead of O(1).

## Prior art

- petgraph documentation, `stable_graph` module: "This graph is similar to
  `Graph`, but it uses a different internal representation that preserves
  node indices even after removal."
- The `StableGraph` pattern is the standard solution for graph structures
  with external index maps (e.g., compiler IRs, game entity graphs).
