use super::*;

/// Master registry of ALL builtin functions.
/// Order determines bytecode indices — DO NOT reorder existing entries.
/// To add a new builtin: append a `BuiltinSpec` row here,
/// add the handler in Builtins::new(), and you're done.
pub const BUILTIN_REGISTRY: &[BuiltinSpec] = &[
    BuiltinSpec {
        name: "upper",
        arity: 1,
        category: "string",
    },
    BuiltinSpec {
        name: "lower",
        arity: 1,
        category: "string",
    },
    BuiltinSpec {
        name: "len",
        arity: 1,
        category: "string",
    },
    BuiltinSpec {
        name: "str",
        arity: 1,
        category: "string",
    },
    BuiltinSpec {
        name: "print",
        arity: 1,
        category: "io",
    },
    BuiltinSpec {
        name: "contains",
        arity: 2,
        category: "string",
    },
    BuiltinSpec {
        name: "float",
        arity: 1,
        category: "convert",
    },
    BuiltinSpec {
        name: "to_string",
        arity: 1,
        category: "convert",
    },
    BuiltinSpec {
        name: "get",
        arity: 2,
        category: "list",
    },
    BuiltinSpec {
        name: "push",
        arity: 2,
        category: "list",
    },
    BuiltinSpec {
        name: "slice",
        arity: 3,
        category: "list",
    },
    BuiltinSpec {
        name: "index_of",
        arity: 2,
        category: "string",
    },
    BuiltinSpec {
        name: "substring",
        arity: 3,
        category: "string",
    },
    BuiltinSpec {
        name: "char_at",
        arity: 2,
        category: "string",
    },
    BuiltinSpec {
        name: "starts_with",
        arity: 2,
        category: "string",
    },
    BuiltinSpec {
        name: "ends_with",
        arity: 2,
        category: "string",
    },
    BuiltinSpec {
        name: "to_float",
        arity: 1,
        category: "convert",
    },
    BuiltinSpec {
        name: "confidence",
        arity: 1,
        category: "fluid",
    },
    BuiltinSpec {
        name: "trim",
        arity: 1,
        category: "string",
    },
    BuiltinSpec {
        name: "replace",
        arity: 3,
        category: "string",
    },
    BuiltinSpec {
        name: "split",
        arity: 2,
        category: "string",
    },
    BuiltinSpec {
        name: "join",
        arity: 2,
        category: "string",
    },
    BuiltinSpec {
        name: "length",
        arity: 1,
        category: "string",
    },
    BuiltinSpec {
        name: "to_int",
        arity: 1,
        category: "convert",
    },
    BuiltinSpec {
        name: "reverse",
        arity: 1,
        category: "string",
    },
    BuiltinSpec {
        name: "escape_html",
        arity: 1,
        category: "string",
    },
    BuiltinSpec {
        name: "escape_json",
        arity: 1,
        category: "string",
    },
    BuiltinSpec {
        name: "__trim",
        arity: 1,
        category: "std",
    },
    BuiltinSpec {
        name: "__replace",
        arity: 3,
        category: "std",
    },
    BuiltinSpec {
        name: "__split",
        arity: 2,
        category: "std",
    },
    BuiltinSpec {
        name: "__join",
        arity: 2,
        category: "std",
    },
    BuiltinSpec {
        name: "__abs",
        arity: 1,
        category: "std",
    },
    BuiltinSpec {
        name: "__min",
        arity: 2,
        category: "std",
    },
    BuiltinSpec {
        name: "__max",
        arity: 2,
        category: "std",
    },
    BuiltinSpec {
        name: "__clamp",
        arity: 3,
        category: "std",
    },
    BuiltinSpec {
        name: "__round",
        arity: 1,
        category: "std",
    },
    BuiltinSpec {
        name: "__first",
        arity: 1,
        category: "std",
    },
    BuiltinSpec {
        name: "__last",
        arity: 1,
        category: "std",
    },
    BuiltinSpec {
        name: "__push",
        arity: 2,
        category: "stub",
    },
    BuiltinSpec {
        name: "__list_len",
        arity: 1,
        category: "stub",
    },
    BuiltinSpec {
        name: "abs",
        arity: 1,
        category: "stub",
    },
    BuiltinSpec {
        name: "min",
        arity: 2,
        category: "stub",
    },
    BuiltinSpec {
        name: "max",
        arity: 2,
        category: "stub",
    },
    BuiltinSpec {
        name: "clamp",
        arity: 3,
        category: "stub",
    },
    BuiltinSpec {
        name: "round",
        arity: 1,
        category: "stub",
    },
    BuiltinSpec {
        name: "respond",
        arity: 1,
        category: "web",
    },
    BuiltinSpec {
        name: "respond_html",
        arity: 1,
        category: "stub",
    },
    BuiltinSpec {
        name: "form_data",
        arity: 1,
        category: "web",
    },
    BuiltinSpec {
        name: "json_body",
        arity: 0,
        category: "web",
    },
    BuiltinSpec {
        name: "query_param",
        arity: 1,
        category: "web",
    },
    BuiltinSpec {
        name: "render",
        arity: 2,
        category: "web",
    },
    BuiltinSpec {
        name: "http_get",
        arity: 1,
        category: "web",
    },
    BuiltinSpec {
        name: "http_post",
        arity: 2,
        category: "web",
    },
    BuiltinSpec {
        name: "http_post_multipart",
        arity: 2,
        category: "stub",
    },
    BuiltinSpec {
        name: "require",
        arity: 0,
        category: "web",
    },
    BuiltinSpec {
        name: "parse_json",
        arity: 1,
        category: "json",
    },
    BuiltinSpec {
        name: "json_encode",
        arity: 1,
        category: "json",
    },
    BuiltinSpec {
        name: "json_get",
        arity: 2,
        category: "json",
    },
    BuiltinSpec {
        name: "has_field",
        arity: 2,
        category: "json",
    },
    BuiltinSpec {
        name: "env",
        arity: 1,
        category: "system",
    },
    BuiltinSpec {
        name: "hash_password",
        arity: 1,
        category: "stub",
    },
    BuiltinSpec {
        name: "verify_password",
        arity: 2,
        category: "stub",
    },
    BuiltinSpec {
        name: "encrypt",
        arity: 2,
        category: "crypto",
    },
    BuiltinSpec {
        name: "decrypt",
        arity: 2,
        category: "crypto",
    },
    BuiltinSpec {
        name: "generate_key",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "authenticate",
        arity: 2,
        category: "stub",
    },
    BuiltinSpec {
        name: "session_login",
        arity: 2,
        category: "stub",
    },
    BuiltinSpec {
        name: "session_logout",
        arity: 1,
        category: "stub",
    },
    BuiltinSpec {
        name: "query",
        arity: 1,
        category: "db",
    },
    BuiltinSpec {
        name: "db_execute",
        arity: 1,
        category: "db",
    },
    BuiltinSpec {
        name: "call_llm",
        arity: 0,
        category: "llm",
    },
    BuiltinSpec {
        name: "call_claude",
        arity: 0,
        category: "llm",
    },
    BuiltinSpec {
        name: "llm_usage",
        arity: 0,
        category: "llm",
    },
    BuiltinSpec {
        name: "kv_set",
        arity: 2,
        category: "memory",
    },
    BuiltinSpec {
        name: "kv_get",
        arity: 1,
        category: "memory",
    },
    BuiltinSpec {
        name: "kv_delete",
        arity: 1,
        category: "memory",
    },
    BuiltinSpec {
        name: "kv_exists",
        arity: 1,
        category: "memory",
    },
    BuiltinSpec {
        name: "kv_list",
        arity: 0,
        category: "memory",
    },
    BuiltinSpec {
        name: "mem_set",
        arity: 2,
        category: "memory",
    },
    BuiltinSpec {
        name: "mem_get",
        arity: 1,
        category: "memory",
    },
    BuiltinSpec {
        name: "mem_delete",
        arity: 1,
        category: "memory",
    },
    BuiltinSpec {
        name: "session_set",
        arity: 2,
        category: "memory",
    },
    BuiltinSpec {
        name: "session_get",
        arity: 1,
        category: "memory",
    },
    BuiltinSpec {
        name: "session_clear",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "read_file",
        arity: 1,
        category: "io",
    },
    BuiltinSpec {
        name: "write_file",
        arity: 2,
        category: "io",
    },
    BuiltinSpec {
        name: "append_file",
        arity: 2,
        category: "io",
    },
    BuiltinSpec {
        name: "delete_file",
        arity: 1,
        category: "io",
    },
    BuiltinSpec {
        name: "file_exists",
        arity: 1,
        category: "io",
    },
    BuiltinSpec {
        name: "list_dir",
        arity: 1,
        category: "io",
    },
    BuiltinSpec {
        name: "now",
        arity: 0,
        category: "time",
    },
    BuiltinSpec {
        name: "send_message",
        arity: 2,
        category: "stub",
    },
    BuiltinSpec {
        name: "whisper_transcribe",
        arity: 1,
        category: "stub",
    },
    BuiltinSpec {
        name: "tts_send",
        arity: 2,
        category: "voice",
    },
    BuiltinSpec {
        name: "recall",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "memorize",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "forget",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "find",
        arity: 4,
        category: "stub",
    },
    BuiltinSpec {
        name: "inspect",
        arity: 1,
        category: "stub",
    },
    BuiltinSpec {
        name: "conv_start",
        arity: 1,
        category: "stub",
    },
    BuiltinSpec {
        name: "conv_add",
        arity: 3,
        category: "stub",
    },
    BuiltinSpec {
        name: "conv_history",
        arity: 1,
        category: "stub",
    },
    BuiltinSpec {
        name: "conv_context",
        arity: 1,
        category: "stub",
    },
    BuiltinSpec {
        name: "conv_end",
        arity: 1,
        category: "stub",
    },
    BuiltinSpec {
        name: "event_count",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "events_since",
        arity: 1,
        category: "stub",
    },
    BuiltinSpec {
        name: "event_sum",
        arity: 2,
        category: "stub",
    },
    BuiltinSpec {
        name: "query_scalar",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "query_row",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "graph_query",
        arity: 0,
        category: "graph",
    },
    BuiltinSpec {
        name: "graph_path",
        arity: 0,
        category: "graph",
    },
    BuiltinSpec {
        name: "graph_neighbors",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "memory_decay",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "memory_boost",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "memory_prune",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "memory_revise",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "subgraph_extract",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "subgraph_nodes",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "subgraph_json",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "trace_start",
        arity: 0,
        category: "graph",
    },
    BuiltinSpec {
        name: "trace_end",
        arity: 0,
        category: "graph",
    },
    BuiltinSpec {
        name: "mtree_summarize",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "mtree_retrieve",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "mtree_stats",
        arity: 0,
        category: "mtree",
    },
    BuiltinSpec {
        name: "cron_mark_fired",
        arity: 1,
        category: "stub",
    },
    BuiltinSpec {
        name: "request_body",
        arity: 0,
        category: "web",
    },
    BuiltinSpec {
        name: "stdin",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "split_tokens",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "if_eq",
        arity: 3,
        category: "stub",
    },
    BuiltinSpec {
        name: "newline",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "is_string_token",
        arity: 1,
        category: "stub",
    },
    BuiltinSpec {
        name: "assert_eq",
        arity: 2,
        category: "test",
    },
    BuiltinSpec {
        name: "assert_contains",
        arity: 2,
        category: "stub",
    },
    BuiltinSpec {
        name: "base64_encode",
        arity: 1,
        category: "stub",
    },
    BuiltinSpec {
        name: "base64_decode",
        arity: 1,
        category: "stub",
    },
    BuiltinSpec {
        name: "db_insert",
        arity: 0,
        category: "stub",
    },
    // Problem B: collection ops — variadic arity (compiler uses actual arg count)
    BuiltinSpec {
        name: "map",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "zip",
        arity: 0,
        category: "list",
    },
    BuiltinSpec {
        name: "sort_by",
        arity: 0,
        category: "list",
    },
    BuiltinSpec {
        name: "filter",
        arity: 0,
        category: "list",
    },
    BuiltinSpec {
        name: "reduce",
        arity: 0,
        category: "list",
    },
    BuiltinSpec {
        name: "resolve_skill_index",
        arity: 1,
        category: "stub",
    },
    BuiltinSpec {
        name: "fit_to_budget",
        arity: 0,
        category: "stub",
    },
    // ── sqz-inspired: String/List utilities (P1) ──
    BuiltinSpec {
        name: "squeeze",
        arity: 2,
        category: "string",
    },
    BuiltinSpec {
        name: "dedup",
        arity: 1,
        category: "list",
    },
    BuiltinSpec {
        name: "condense",
        arity: 1,
        category: "list",
    },
    BuiltinSpec {
        name: "strip",
        arity: 2,
        category: "string",
    },
    BuiltinSpec {
        name: "chomp",
        arity: 1,
        category: "string",
    },
    BuiltinSpec {
        name: "repeat",
        arity: 2,
        category: "string",
    },
    BuiltinSpec {
        name: "pad_left",
        arity: 3,
        category: "string",
    },
    BuiltinSpec {
        name: "pad_right",
        arity: 3,
        category: "string",
    },
    BuiltinSpec {
        name: "lines",
        arity: 1,
        category: "string",
    },
    BuiltinSpec {
        name: "words",
        arity: 1,
        category: "string",
    },
    // ── sqz-inspired: TOON encoding (P2) ──
    BuiltinSpec {
        name: "toon_encode",
        arity: 1,
        category: "encoding",
    },
    BuiltinSpec {
        name: "toon_decode",
        arity: 1,
        category: "encoding",
    },
    // ── sqz-inspired: Content-addressed refs (P2) ──
    BuiltinSpec {
        name: "ref",
        arity: 1,
        category: "memory",
    },
    BuiltinSpec {
        name: "deref",
        arity: 1,
        category: "memory",
    },
    // ── sqz-inspired: Token awareness (P3) ──
    BuiltinSpec {
        name: "token_count",
        arity: 1,
        category: "string",
    },
    // ── AgentSkillOS-inspired: Recipe system + DAG orchestration (ADR-0062) ──
    BuiltinSpec {
        name: "recipe_save",
        arity: 0,
        category: "recipe",
    },
    BuiltinSpec {
        name: "recipe_search",
        arity: 0,
        category: "stub",
    },
    BuiltinSpec {
        name: "recipe_list",
        arity: 0,
        category: "recipe",
    },
    BuiltinSpec {
        name: "dag_phases",
        arity: 1,
        category: "orchestration",
    },
    BuiltinSpec {
        name: "topo_sort",
        arity: 1,
        category: "orchestration",
    },
    // ── OpenPlanter-inspired: Fuzzy matching, safe editing, agent utilities (ADR-0063) ──
    BuiltinSpec {
        name: "fuzzy_match",
        arity: 2,
        category: "string",
    },
    BuiltinSpec {
        name: "fuzzy_find_best",
        arity: 2,
        category: "stub",
    },
    BuiltinSpec {
        name: "hashline_read",
        arity: 1,
        category: "stub",
    },
    BuiltinSpec {
        name: "hashline_edit",
        arity: 2,
        category: "stub",
    },
    BuiltinSpec {
        name: "compact_list",
        arity: 3,
        category: "stub",
    },
    BuiltinSpec {
        name: "budget_check",
        arity: 2,
        category: "stub",
    },
    BuiltinSpec {
        name: "replay_snapshot",
        arity: 1,
        category: "stub",
    },
    BuiltinSpec {
        name: "policy_check",
        arity: 1,
        category: "stub",
    },
    // ── obsidian-mind inspired: Vault/memory (v0.10.0) ──
    BuiltinSpec {
        name: "semantic_search",
        arity: 3,
        category: "stub",
    },
    BuiltinSpec {
        name: "config_load",
        arity: 1,
        category: "vault",
    },
    BuiltinSpec {
        name: "vault_validate",
        arity: 2,
        category: "stub",
    },
    // ── Missing entries added by Наряд №32 Block 3.2 ──
    // Functions present in dispatcher (funcs.insert) but absent from REGISTRY.
    // Categories assigned by function purpose.

    // time (builtins that were missing from registry)
    BuiltinSpec {
        name: "time",
        arity: 0,
        category: "time",
    },
    BuiltinSpec {
        name: "add_days",
        arity: 2,
        category: "time",
    },
    BuiltinSpec {
        name: "add_hours",
        arity: 2,
        category: "time",
    },
    BuiltinSpec {
        name: "date_parts",
        arity: 1,
        category: "time",
    },
    BuiltinSpec {
        name: "format_date",
        arity: 2,
        category: "time",
    },
    // cron
    BuiltinSpec {
        name: "cron_add",
        arity: 2,
        category: "cron",
    },
    BuiltinSpec {
        name: "cron_list",
        arity: 0,
        category: "cron",
    },
    BuiltinSpec {
        name: "cron_remove",
        arity: 1,
        category: "cron",
    },
    BuiltinSpec {
        name: "cron_run",
        arity: 1,
        category: "cron",
    },
    // json/dict (dict_get is alias for json_get)
    BuiltinSpec {
        name: "dict_get",
        arity: 3,
        category: "json",
    },
    BuiltinSpec {
        name: "dict_set",
        arity: 3,
        category: "json",
    },
    BuiltinSpec {
        name: "dict_has",
        arity: 2,
        category: "json",
    },
    BuiltinSpec {
        name: "dict_keys",
        arity: 1,
        category: "json",
    },
    BuiltinSpec {
        name: "dict_values",
        arity: 1,
        category: "json",
    },
    // list
    BuiltinSpec {
        name: "first",
        arity: 1,
        category: "list",
    },
    BuiltinSpec {
        name: "last",
        arity: 1,
        category: "list",
    },
    BuiltinSpec {
        name: "make_list",
        arity: 0,
        category: "list",
    },
    BuiltinSpec {
        name: "matches_any",
        arity: 2,
        category: "list",
    },
    // string
    BuiltinSpec {
        name: "format",
        arity: 1,
        category: "string",
    },
    BuiltinSpec {
        name: "escape_js",
        arity: 1,
        category: "string",
    },
    BuiltinSpec {
        name: "type_of",
        arity: 1,
        category: "string",
    },
    // io/system
    BuiltinSpec {
        name: "exec",
        arity: 1,
        category: "io",
    },
    BuiltinSpec {
        name: "web_search",
        arity: 2,
        category: "web",
    },
    BuiltinSpec {
        name: "geo_ip",
        arity: 1,
        category: "web",
    },
    BuiltinSpec {
        name: "weather",
        arity: 2,
        category: "web",
    },
    BuiltinSpec {
        name: "git_push",
        arity: 1,
        category: "io",
    },
    // bot/todo/goals
    BuiltinSpec {
        name: "todo_add",
        arity: 2,
        category: "bot",
    },
    BuiltinSpec {
        name: "todo_list",
        arity: 0,
        category: "bot",
    },
    BuiltinSpec {
        name: "todo_update",
        arity: 2,
        category: "bot",
    },
    BuiltinSpec {
        name: "goal_get",
        arity: 0,
        category: "bot",
    },
    BuiltinSpec {
        name: "goal_set",
        arity: 2,
        category: "bot",
    },
    BuiltinSpec {
        name: "goals_add",
        arity: 1,
        category: "bot",
    },
    BuiltinSpec {
        name: "goals_list",
        arity: 0,
        category: "bot",
    },
    BuiltinSpec {
        name: "remind",
        arity: 3,
        category: "bot",
    },
    BuiltinSpec {
        name: "get_profile",
        arity: 0,
        category: "bot",
    },
    BuiltinSpec {
        name: "human_mood",
        arity: 3,
        category: "bot",
    },
    // mtree
    BuiltinSpec {
        name: "mtree_store",
        arity: 2,
        category: "mtree",
    },
    // ── P33 sync: 30 missing entries ──
    // bot — Telegram
    BuiltinSpec {
        name: "answer_callback_query",
        arity: 0,
        category: "bot",
    },
    BuiltinSpec {
        name: "edit_message_text",
        arity: 0,
        category: "bot",
    },
    // bot — approvals / goals
    BuiltinSpec {
        name: "ask_approval",
        arity: 1,
        category: "bot",
    },
    BuiltinSpec {
        name: "goal_complete",
        arity: 0,
        category: "bot",
    },
    BuiltinSpec {
        name: "goals_reflect",
        arity: 0,
        category: "bot",
    },
    // bot — reminders
    BuiltinSpec {
        name: "cancel_remind",
        arity: 1,
        category: "bot",
    },
    BuiltinSpec {
        name: "check_reminders",
        arity: 0,
        category: "bot",
    },
    BuiltinSpec {
        name: "list_reminders",
        arity: 0,
        category: "bot",
    },
    BuiltinSpec {
        name: "remind_recurring",
        arity: 2,
        category: "bot",
    },
    // bot — human intelligence
    BuiltinSpec {
        name: "human_create",
        arity: 2,
        category: "bot",
    },
    BuiltinSpec {
        name: "human_delete",
        arity: 1,
        category: "bot",
    },
    BuiltinSpec {
        name: "human_forget",
        arity: 2,
        category: "bot",
    },
    BuiltinSpec {
        name: "human_personas",
        arity: 0,
        category: "bot",
    },
    BuiltinSpec {
        name: "human_recall",
        arity: 3,
        category: "bot",
    },
    BuiltinSpec {
        name: "human_remember",
        arity: 4,
        category: "bot",
    },
    BuiltinSpec {
        name: "human_respond",
        arity: 2,
        category: "bot",
    },
    // bot — OpenHuman helpers
    BuiltinSpec {
        name: "compress_html",
        arity: 1,
        category: "bot",
    },
    BuiltinSpec {
        name: "estimate_tokens",
        arity: 0,
        category: "bot",
    },
    BuiltinSpec {
        name: "extract_entities",
        arity: 1,
        category: "bot",
    },
    BuiltinSpec {
        name: "extract_param",
        arity: 0,
        category: "bot",
    },
    BuiltinSpec {
        name: "learn_preference",
        arity: 2,
        category: "bot",
    },
    BuiltinSpec {
        name: "memory_score",
        arity: 1,
        category: "bot",
    },
    BuiltinSpec {
        name: "read_file_tokens",
        arity: 0,
        category: "bot",
    },
    // web — geolocation / weather
    BuiltinSpec {
        name: "geo_distance",
        arity: 0,
        category: "web",
    },
    BuiltinSpec {
        name: "weather_forecast",
        arity: 0,
        category: "web",
    },
    // time — date helpers
    BuiltinSpec {
        name: "days_between",
        arity: 2,
        category: "time",
    },
    BuiltinSpec {
        name: "days_in_month",
        arity: 2,
        category: "time",
    },
    BuiltinSpec {
        name: "is_leap_year",
        arity: 1,
        category: "time",
    },
    BuiltinSpec {
        name: "weekday_name",
        arity: 1,
        category: "time",
    },
    // mtree
    BuiltinSpec {
        name: "mtree_forget",
        arity: 1,
        category: "mtree",
    },
];

/// Total number of registered builtins.
pub fn builtin_count() -> usize {
    BUILTIN_REGISTRY.len()
}

/// Ordered list of builtin names (parallel to compiler index table).
pub fn builtin_names() -> Vec<String> {
    BUILTIN_REGISTRY
        .iter()
        .map(|s| s.name.to_string())
        .collect()
}

/// Name → bytecode index mapping for the compiler.
pub fn builtin_indices() -> std::collections::HashMap<String, usize> {
    BUILTIN_REGISTRY
        .iter()
        .enumerate()
        .map(|(i, s)| (s.name.to_string(), i))
        .collect()
}

/// Set of all builtin names for semantic validation.
pub fn builtin_name_set() -> std::collections::HashSet<String> {
    BUILTIN_REGISTRY
        .iter()
        .map(|s| s.name.to_string())
        .collect()
}

/// Name → arity mapping. 0 = variadic (skip check).
pub fn builtin_arity_map() -> std::collections::HashMap<&'static str, usize> {
    BUILTIN_REGISTRY.iter().map(|s| (s.name, s.arity)).collect()
}

/// Check if a name is a known builtin.
pub fn is_builtin(name: &str) -> bool {
    BUILTIN_REGISTRY.iter().any(|s| s.name == name)
}
