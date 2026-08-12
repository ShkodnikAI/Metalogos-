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

## The `stub` category

66 entries are classified as `category: "stub"`. The `check_registry_sync()`
assertion skips them because they have no `funcs.insert()` handler.

### What is a stub?

A stub is a **planned but unimplemented builtin** that lives in the registry
so that `mlog check` recognizes the name and can emit a meaningful
"builtin X is not yet implemented" diagnostic instead of a generic
"unknown function X" error. This makes `mlog check` useful for programs
that reference future builtins.

### Why are they not deleted?

1. **User programs already reference them.** Golden test contracts, examples,
   and user code may call `send_message`, `graph_neighbors`, `inspect`, etc.
   Deleting the registry entry would turn a "not implemented" soft error into
   a "unknown function" hard parse error.

2. **Some have partial implementations.** `cron_mark_fired`, `mtree_retrieve`,
   `mtree_summarize` have handlers that are registered in the dispatcher under
   different conditions (feature flags, optional deps). The stub entry ensures
   they are recognized even when the handler is not compiled in.

3. **Scheduling intent.** Entries like `semantic_search`, `respond_html`,
   `recipe_search` document planned functionality. Removing them loses that
   signal.

### When should a stub entry be removed?

A stub entry MUST be removed when:
- The builtin is **officially abandoned** (decided not to implement).
- The builtin was **renamed** — old name removed, new name added as non-stub.

A stub entry SHOULD be converted to a real entry when:
- A handler is added in `Builtins::new()`.
- The category is changed from `stub` to the appropriate group.

### List of 66 stub entries

```
__list_len              __push                 abs
assert_contains         authenticate            base64_decode
base64_encode           budget_check            clamp
compact_list            conv_add                conv_context
conv_end                conv_history            conv_start
cron_mark_fired         db_insert               event_count
event_sum               events_since            find
fit_to_budget           forget                  fuzzy_find_best
generate_key            graph_neighbors         hash_password
hashline_edit           hashline_read           http_post_multipart
if_eq                   inspect                 is_string_token
map                     max                     memorize
memory_boost            memory_decay            memory_prune
memory_revise           min                     mtree_retrieve
mtree_summarize          newline                 policy_check
query_row               query_scalar            recall
recipe_search           replay_snapshot          resolve_skill_index
respond_html            round                   semantic_search
send_message            session_clear           session_login
session_logout          split_tokens            stdin
subgraph_extract        subgraph_json           subgraph_nodes
vault_validate          verify_password         whisper_transcribe
```

### Stub cleanup path

Each stub should be either:
- Implemented (add handler in Builtins::new, change category from stub), or
- Removed if no longer planned.

## Update: P33 additional sync (2026-08-01)

30 more callable functions were found outside the registry.
All added with correct arity from implementation:

| Category | Functions |
|----------|-----------|
| bot/Telegram | answer_callback_query, edit_message_text |
| bot/goals | ask_approval, goal_complete, goals_reflect |
| bot/reminders | cancel_remind, check_reminders, list_reminders, remind_recurring |
| bot/human | human_create, human_delete, human_forget, human_personas, human_recall, human_remember, human_respond |
| bot/OpenHuman | compress_html, estimate_tokens, extract_entities, extract_param, learn_preference, memory_score, read_file_tokens |
| web | geo_distance, weather_forecast |
| time | days_between, days_in_month, is_leap_year, weekday_name |
| mtree | mtree_forget |

Acceptance: `callable but not in registry` = 0.

## Consequences

- `mlog check` now validates arity for 241 builtins (up from 211).
- `cargo test --lib` passes with 0 assertion failures from `check_registry_sync()`.
- Adding a new builtin without a registry entry triggers an assertion in debug builds.
- 66 stub entries remain, documented with removal criteria above.
