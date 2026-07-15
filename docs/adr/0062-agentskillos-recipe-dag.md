# ADR-0062: AgentSkillOS-inspired Recipe System + DAG Orchestration

**Date:** 2026-07-16
**Status:** Accepted
**Source:** https://github.com/ynulihao/AgentSkillOS (MIT license)

## Context

AgentSkillOS introduces two concepts valuable for Metalogos agent orchestration:

1. **Recipe System** — persistent storage of successful (task, skills, plan) combinations for reuse. Analogous to caching successful strategies.
2. **DAG Phase Extraction** — decomposing a directed acyclic graph into parallel execution phases using topological analysis (Kahn's algorithm).

Metalogos already has `flow` for linear pipelines and `skill_index` for skill retrieval, but lacks:
- A mechanism to save and recall successful multi-skill compositions
- Graph-based parallel execution planning

## Decision

### New Builtins (5)

| Builtin | Arity | Category | Description |
|---------|-------|----------|-------------|
| `recipe_save` | 0 (variadic: 4) | recipe | Build a recipe struct from (name, description, skills, plan). Returns `{key, recipe}` for KV persistence. |
| `recipe_search` | 0 (variadic: 1) | recipe | Placeholder for semantic recipe search. Returns empty list (full implementation requires embedding infrastructure). |
| `recipe_list` | 0 (variadic) | recipe | Placeholder for recipe index listing. Returns empty list (full implementation requires KV access from builtin context). |
| `dag_phases` | 1 | orchestration | Extract parallel execution phases from a DAG. Input: list of `{id, depends_on}` structs. Returns list of phases (lists of node IDs). Uses Kahn's algorithm with cycle detection. |
| `topo_sort` | 1 | orchestration | Topological sort of a DAG. Same input format. Returns flat list of node IDs in dependency order. Cycle detection included. |

### Recipe Architecture

`recipe_save` returns a struct with a `key` field (`__recipe:<name>`) so the caller can persist via `kv_set`:

```mlog
let result = recipe_save("bug_report", "Analyze logs and generate report", ["log_analyze", "report_gen"], plan)
kv_set(result.key, json_encode(result.recipe))
```

This two-step pattern (builtin builds struct → user persists via kv_set) avoids the architectural limitation of builtins being pure functions without access to the interpreter's KV store.

### DAG Input Format

```mlog
let dag = [
  { id: "extract", depends_on: [] },
  { id: "load", depends_on: [] },
  { id: "transform", depends_on: ["extract", "load"] },
  { id: "validate", depends_on: ["transform"] }
]
let phases = dag_phases(dag)
// phases = [["extract", "load"], ["transform"], ["validate"]]
let order = topo_sort(dag)
// order = ["extract", "load", "transform", "validate"]  (or ["load", "extract", ...])
```

### Cycle Detection

Both `dag_phases` and `topo_sort` detect cycles and return an error listing the unprocessed nodes.

## Consequences

### Positive
- `dag_phases` enables planning parallel agent execution directly in .mlog programs
- `topo_sort` provides deterministic execution ordering
- Recipe system creates a foundation for caching successful multi-skill strategies
- All 5 builtins follow the existing `BUILTIN_REGISTRY` pattern (1 row + 1 handler)
- 13 unit tests cover happy paths, edge cases, and error conditions

### Limitations
- `recipe_search` and `recipe_list` are placeholders — full implementation requires either: (a) passing KV store reference to builtins, or (b) user-managed recipe index via `kv_set`/`kv_get`
- No semantic similarity search (requires embedding infrastructure)
- DAG builtins operate on in-memory data structures — no persistence

### Deferred
- `capability_tree` declaration (LLM-built hierarchical skill index) — requires LLM calls during tree construction
- `plan_strategies` (multi-strategy DAG planning) — requires LLM planner integration
- `collaboration_hints` (inter-node output annotations) — requires DAG execution engine
- Adaptive throttler — requires async runtime integration

## Changed Files

- `src/builtins.rs` — 5 new builtins, 13 tests, ~300 lines of implementation
- `examples/dag_demo.mlog` + `.expected` — golden test for dag_phases/topo_sort