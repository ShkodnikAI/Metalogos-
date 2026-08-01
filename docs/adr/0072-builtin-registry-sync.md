# ADR-0072: BUILTIN_REGISTRY / dispatcher synchronization

**Date:** 2026-08-01
**Status:** accepted
**Context:** Наряд №32 Block 3.2

## Problem

Metalogos has two parallel lists of builtins:

1. **BUILTIN_REGISTRY** — declarative spec table (name, arity, category).
   Used by `mlog check` for arity validation and by compiler index table.

2. **Builtins::new() dispatcher** — `funcs.insert(name, handler)` map.
   Used at runtime to dispatch builtin calls.

These lists diverged significantly:
- **37 functions in dispatcher but missing from registry** (arity not checked by `mlog check`).
- **66 entries in registry but without dispatcher handlers** (planned stubs).

Total: 103 discrepancies out of 211 entries.

## Decision

1. **Add all 37 missing entries to BUILTIN_REGISTRY** with correct arity and category.
2. **Reclassify 66 dead entries as `category: "stub"`** — `check_registry_sync()` already
   skips stub entries, so they don't cause assertion failures.
3. **Establish the one-source-of-truth rule:**
   - To add a new builtin: add BOTH a `BuiltinSpec` row AND a `funcs.insert()` line.
   - To remove a builtin: remove BOTH.
   - `check_registry_sync()` runs at `Builtins::new()` time in debug builds.

## Categories assigned to the 37 added entries

| Category | Functions |
|----------|-----------|
| time | time, add_days, add_hours, date_parts, format_date |
| cron | cron_add, cron_list, cron_remove, cron_run |
| json | dict_get, dict_set, dict_has, dict_keys, dict_values |
| list | first, last, make_list, matches_any |
| string | format, escape_js, type_of |
| io | exec, git_push |
| web | web_search, geo_ip, weather |
| bot | todo_add, todo_list, todo_update, goal_get, goal_set, goals_add, goals_list, remind, get_profile, human_mood |
| mtree | mtree_store |

## Stub cleanup path

The 66 stub entries remain in the registry so that `mlog check` recognizes them
and gives a meaningful "builtin X is not yet implemented" error instead of
"unknown function X". Each stub should be either:
- Implemented (add handler in Builtins::new, change category from stub), or
- Removed if no longer planned.

## Consequences

- `mlog check` now validates arity for 145 builtins (up from 108).
- `cargo test --lib` passes with 0 assertion failures from `check_registry_sync()`.
- Adding a new builtin without a registry entry triggers an assertion in debug builds.
