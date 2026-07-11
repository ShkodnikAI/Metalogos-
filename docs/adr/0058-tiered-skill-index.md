# ADR-0058: Tiered Skill Index — Structured Skill Loading

**Date:** 2026-07-12
**Status:** Accepted
**Context:** Наряд METALOGOS_4_PRIMITIVES v2, Problem A

## Problem

`SelectSkillsByKeywords` uses substring-matching with a 2000-char budget, cutting skills at 500 characters. Three defects: (1) no tier concept — critical skills not distinguished from niche ones, (2) substring match on first 50 chars of query and first 500 chars of skill — both random relative to meaning, (3) budget counted in characters not tokens.

## Decision

New `skill_index` declaration with explicit tiers, trigger-based matching, and token-aware budgeting:

```mlog
skill_index osp {
  tier 1 always ["deconstruct", "awareness-frame"]
  tier 2 when_matches [
    { skill: "cross-asset-divergence", triggers: ["рынок", "актив", "валют"] }
  ]
  tier 3 when_matches [
    { skill: "red-team", triggers: ["контр-анализ"] }
  ]
  budget: 25000 tokens
  truncation: whole_skill_only
}
```

### Builtins

| Builtin | Args | Returns | Description |
|---------|------|---------|-------------|
| `resolve_skill_index(dept)` | String | Struct | Returns registered index as struct with `tier1`, `tier2`/`tier3` (lists), `budget` (Float), `truncation` (String) |
| `fit_to_budget(list, budget, mode)` | List, Float, String | List | MVP: returns list as-is. Full implementation deferred (requires file I/O per skill) |

### Existing Builtins Used

- `matches_any(text, triggers_list)` — case-insensitive substring match (already exists)
- `estimate_tokens(text)` — heuristic `len/4` (already exists)
- `push(list, item)` — append to list (already exists)

### Design

- `skill_index` is a compile-time/init-time declaration, stored in the interpreter
- Tier 1 "always" skills are unconditionally loaded
- Tier 2/3 "when_matches" skills are loaded if any trigger matches the query
- `resolve_skill_index` returns the index as a Value::Struct for field access
- `fit_to_budget` MVP is a pass-through; full implementation needs file I/O and is deferred

### Backend Support

Same as Problem C: tree-walking only (requires interpreter state). VM/JIT deferred.

## Consequences

- Departments define their skill loading strategy declaratively, not via Python code
- No more substring hacks or character-budget cutting
- `skill_index`, `tier`, `when_matches`, `always`, `triggers`, `budget`, `tokens`, `truncation` added to step_ident negative lookahead