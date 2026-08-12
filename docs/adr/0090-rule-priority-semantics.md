# ADR-0090: Rule Priority Semantics — First-Wins

## Status

accepted

## Context

Naryad №42 discovered that `execute_rules()` in both interpreter
(`src/interpreter/flow.rs`) and VM (`src/vm.rs`) had inverted priority
semantics. All matching rules executed sequentially in priority-descending
order, with each rule overwriting the previous. The result was **last-wins**
— the rule with the *lowest* priority determined the final field value.

This contradicted the language semantics specification (`metalogos-language-semantics`
skill) which prescribes "priority-ordered, first-wins": the rule with the
highest priority wins the conflict for a field, and at equal priority the
rule declared earlier wins.

FOSVED does not use `rule` at all (zero occurrences across 23 files), so
fixing this has zero production impact — a rare opportunity to correct
language semantics without migration.

## Decision

### Semantics: priority-ordered, first-wins

1. Rules are sorted by **priority descending** (highest first).
2. For each `(entity_name, field_name)` pair, only the **first** matching
   rule writes the field. Subsequent rules targeting the same field are
   **skipped** (tracked via `HashSet<(String, String)>`).
3. At equal priority, **stable sort** preserves declaration order, so the
   rule declared earlier wins.
4. Rules targeting **different fields** of the same entity **all fire** —
   deduplication is per-field, not per-entity.

### Implementation: variant (a)

Chosen variant (a): keep descending sort, skip already-written fields.
This preserves the ability for multiple rules to modify different fields
of the same entity in a single rule pass.

Variant (b) was considered (sort ascending so highest runs last) but
rejected because it would give "last-declared wins" at equal priority,
contradicting the spec.

Variant (c) (pre-select one winner per field) was considered but adds
complexity without benefit over variant (a).

### Prior art

- **CLIPS/Rete**: rules have salience (priority), conflict resolution
  strategies include "salience" (highest first) and "MEA" (recency).
  CLIPS fires one rule per cycle; Metalogos fires all non-conflicting rules.
- **Drools**: similar salience-based conflict resolution, first-wins
  when multiple rules can fire.
- **Prolog**: clauses are tried in order; first match wins (closer to
  Metalogos' equal-priority behavior).

## Consequences

- `p42_rule_priority_order` updated: expected `0.5` → `0.9`.
- `p42_rule_equal_priority` updated: expected `0.3` → `0.9`.
- `p43_rule_different_fields` added: verifies different-field rules all fire.
- Both interpreter and VM fixed in same commit for parity.
- Crosscheck asserts remain green.
