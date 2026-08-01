# Refactoring Split Plan

**Created:** 2026-08-02
**Context:** Наряд №37 Block 2

## Current State

| File | Lines | Target |
|------|-------|--------|
| src/builtins.rs | 10 447 | ~14 modules |
| src/interpreter.rs | 5 073 | ~8 modules |
| src/parser.rs | 4 919 | ~4 modules |
| src/vm.rs | 1 387 | not touched |

## Rule

- One module = one commit = green `test-lib` (377 pass).
- No logic changes during split. Pure move.
- No file in `src/` exceeds 800 lines after split.

---

## builtins.rs → 14 modules

### Target structure

```
src/builtins/
  mod.rs          — Builtins struct, get(), check_registry_sync(), public query fns
  registry.rs     — BUILTIN_REGISTRY table (1235 lines), builtin_count/names/indices
  core.rs         — expect_* helpers, make_struct, assert_eq/contains, type_of
  string.rs       — upper/lower/len/contains/trim/replace/split/join/index_of/
                     substring/char_at/starts_with/ends_with/reverse/escape_html/
                     escape_json/escape_js/format/squeeze/strip/chomp/repeat/
                     pad_left/pad_right/lines/words/fuzzy_match/token_count/
                     base64_encode/decode/toon_encode/decode
  math.rs         — abs/min/max/clamp/round/confidence/first/last/float/str/
                     to_string/to_float/to_int/length
  collections.rs  — get/push/slice/zip/sort_by/filter/reduce/make_list/
                     dedup/condense/compact_list/matches_any
  http.rs         — respond/respond_html/form_data/json_body/query_param/
                     render/require/http_get/http_post/http_post_multipart/
                     web_search/geo_ip/geo_distance/weather/weather_forecast
  json.rs         — parse_json/json_encode/json_get/has_field/dict_set/keys/
                     values/has/value_to_json helpers
  io.rs           — print/read_file/write_file/append_file/delete_file/
                     file_exists/list_dir/exec/git_push/sandbox_path
  crypto.rs       — encrypt/decrypt/hash_password/verify_password/generate_key/
                     authenticate/session_login/session_logout
  llm.rs          — call_llm/call_claude/llm_usage/whisper_transcribe/tts_send
  memory.rs       — kv_store/kv_sqlite/init_kv_persist/kv_set/get/delete/
                     exists/list/mem_set/get/delete/session_store/set/get/
                     clear/content_ref/deref + memory_decay/boost/prune/
                     revise/graph_query/path/neighbors/mtree_store/retrieve/
                     forget/summarize/stats/subgraph_*/trace_start/end
  cron.rs         — ReminderEntry/cron_add/list/remove/run/mark_fired/
                     remind/remind_recurring/cancel_remind/list_reminders/
                     check_reminders/init_reminder_persist + helper fns
  server.rs       — send_message/answer_callback_query/edit_message_text/
                     human_create/mood/remember/forget/recall/respond/
                     personas/delete/goal_set/get/complete/goals_list/
                     goals_add/goals_reflect/todo_add/update/list/
                     ask_approval/extract_entities/regex_lite/memory_score/
                     compress_html/learn_preference/get_profile/
                     extract_param/estimate_tokens/read_file_tokens/
                     recipe_save/search/list/dag_phases/topo_sort/
                     hashline_read/edit/budget_check/replay_snapshot/
                     policy_check/fuzzy_find_best/semantic_search/
                     config_load/vault_validate
  tests.rs        — all #[cfg(test)] code (mod tests + mod tests_sqz_builtins)
```

### Estimated line counts (code only, excluding tests)

| Module | Est. Lines |
|--------|-----------|
| mod.rs | ~200 |
| registry.rs | ~1 350 |
| core.rs | ~200 |
| string.rs | ~1 250 |
| math.rs | ~200 |
| collections.rs | ~400 |
| http.rs | ~750 |
| json.rs | ~350 |
| io.rs | ~150 |
| crypto.rs | ~300 |
| llm.rs | ~350 |
| memory.rs | ~1 100 |
| cron.rs | ~500 |
| server.rs | ~1 700 |
| tests.rs | ~6 200 (exempt from 800-line limit) |

NOTE: registry.rs (1350), memory.rs (1100), server.rs (1700), string.rs (1250)
exceed 800 lines. These need further subdivision or the 800-line target
must be relaxed for data-heavy files.

### Split order

1. `registry.rs` — extract BUILTIN_REGISTRY + query fns (no dependencies)
2. `core.rs` — extract helpers (used by all other modules)
3. `io.rs` — simplest domain, few deps
4. `math.rs` — simple, uses only core
5. `collections.rs` — uses core
6. `string.rs` — uses core, collections
7. `convert.rs` — merge into math.rs (small)
8. `crypto.rs` — self-contained
9. `json.rs` — uses core
10. `llm.rs` — uses core, crypto (SecretString)
11. `http.rs` — uses core, json
12. `memory.rs` — uses core (kv_store statics)
13. `cron.rs` — uses memory (kv_store)
14. `server.rs` — uses everything
15. `tests.rs` — last, after all modules exist
16. `mod.rs` — becomes re-export facade

---

## interpreter.rs → 8 modules

### Target structure

```
src/interpreter/
  mod.rs          — Interpreter struct, new(), accessors, re-exports
  values.rs       — Value enum, SecretString, FluidValueVariant, impl Display, impl Value
  types.rs        — CompiledPattern, CompiledLearnable, EvalResult, PatternStats,
                     ControlFlow, Event, Conversation, etc.
  execution.rs    — run(), eval_statements(), eval_statements_cf(),
                     eval_expr(), eval_expr_with_env(), eval_binop(),
                     compare_values(), is_truthy(), helpers
  flow.rs         — execute_rules(), eval_condition(), run_flow(),
                     save/load/delete_checkpoint(), lifecycle control
  memory.rs       — configure_memory(), invoke_recall/find/memorize/forget()
  db.rs           — init_db_connection(), invoke_query/db_execute/query_scalar/
                     query_row(), apply_schema(), mlog_type_to_sql()
  learnable.rs    — build_effective_prompt(), invoke_learnable_with_env(),
                     llm_cache_get/persist(), compress_context()
  conversations.rs — conv_start/add/history/context/end(),
                     compress_conversation(), get_conversation_for_llm()
  events.rs       — emit_event(), event_count(), events_since(),
                     get_events(), event_sum()
  modules.rs      — handle_import(), load_module(), load_module_inner()
  hooks.rs        — invoke(), fire_on_write_hooks(), invoke_pattern_with_hooks()
  eval_harness.rs — run_eval_blocks(), run_single_eval(), invoke_inspect(),
                     inspect_pattern(), handle_mutate()
```

### Estimated line counts

| Module | Est. Lines |
|--------|-----------|
| mod.rs | ~300 |
| values.rs | ~250 |
| types.rs | ~450 |
| execution.rs | ~1 100 (includes dispatch) |
| flow.rs | ~350 |
| memory.rs | ~350 |
| db.rs | ~450 |
| learnable.rs | ~400 |
| conversations.rs | ~250 |
| events.rs | ~100 |
| modules.rs | ~300 |
| hooks.rs | ~300 |
| eval_harness.rs | ~250 |

NOTE: execution.rs at ~1100 lines needs further splitting. The FnCall
dispatch arm (~407 lines) should be extracted to `dispatch.rs`.

### Prerequisite refactoring

Before splitting, extract from the `impl Interpreter` block:
1. `dispatch_fn_call()` — 407-line FnCall arm from `eval_expr_with_env`
2. `process_declaration()` — shared between `run()` and `load_module_inner()`

### Split order

1. `values.rs` — standalone types, no deps
2. `types.rs` — depends on values
3. `events.rs` — depends on types::Event
4. `memory.rs` — depends on types
5. `db.rs` — depends on values
6. `conversations.rs` — depends on types
7. `learnable.rs` — depends on types
8. `flow.rs` — depends on types
9. `hooks.rs` — depends on types
10. `eval_harness.rs` — depends on types, learnable
11. `modules.rs` — depends on everything
12. `execution.rs` — core, depends on all above
13. `mod.rs` — struct + new() + re-exports

---

## parser.rs → 4 modules

### Target structure

```
src/parser/
  mod.rs          — parse() entry point, parse_inner(), error types, re-exports
  expr.rs         — parse_expression(), parse_binop(), parse_block_if_else_expr()
  stmt.rs         — parse_single_statement(), parse_pattern_body(),
                     parse_match_stmt(), parse_if_block_stmt(), parse_params()
  decl.rs         — all parse_*_decl functions (pattern, learnable, entity,
                     rule, schema, db, skill_index, memory, import, flow,
                     llm, tool, memorize, forget, eval, fluid, adapt, relate,
                     hook, sandbox, mutate, conversation, context_budget,
                     mlogserver, route, template)
  helpers.rs      — pair_str(), children_of(), find_child_str(), find_child(),
                     unescape_string(), parse_literal_to_expr(),
                     extract_balanced_braces(), preprocess_templates()
  tests.rs        — #[cfg(test)] mod tests (~2070 lines)
```

### Estimated line counts

| Module | Est. Lines |
|--------|-----------|
| mod.rs | ~120 |
| expr.rs | ~400 |
| stmt.rs | ~500 |
| decl.rs | ~1 600 |
| helpers.rs | ~200 |
| tests.rs | ~2 070 |

NOTE: decl.rs at ~1600 lines exceeds 800. Can split into:
- `decl.rs` — top-level dispatch + simpler decls (~800)
- `decl_ai.rs` — llm, tool, eval, fluid, adapt, relate (~600)

### Split order

1. `helpers.rs` — standalone, no deps
2. `expr.rs` — depends on helpers
3. `stmt.rs` — depends on helpers, expr
4. `decl.rs` — depends on helpers, expr, stmt
5. `tests.rs` — last
6. `mod.rs` — facade + re-exports

---

## Verification

After each module extraction:
```bash
cargo test --lib --no-fail-fast  # must be 377 pass
cargo fmt -- --check             # must be clean
wc -l src/<file>                 # must be <= 800 (except tests)
```
