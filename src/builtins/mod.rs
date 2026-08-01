// ── Built-in functions for METALOGOS M1+M2 ────────────────────────────

use crate::interpreter::Value;
pub type BuiltinFn = fn(&[Value]) -> Result<Value, String>;

/// Registry of built-in functions.
pub struct Builtins {
    funcs: std::collections::HashMap<String, BuiltinFn>,
}

/// Metadata for a single builtin function.
/// This is the SINGLE SOURCE OF TRUTH for all builtin metadata.
/// Every consumer (compiler, VM, semantic) reads from here.
///
/// - `name`: function name as exposed to the DSL
/// - `arity`: exact argument count; 0 = variadic (skip arity check)
/// - `category`: logical group for documentation and error messages
#[derive(Debug, Clone)]
pub struct BuiltinSpec {
    pub name: &'static str,
    pub arity: usize, // 0 = variadic
    pub category: &'static str,
}

pub(crate) mod core;
use core::*;
pub(crate) mod io;
use io::*;
pub(crate) mod registry;
pub use registry::*;
pub(crate) mod math;
use math::*;
pub(crate) mod collections;
use collections::*;
pub(crate) mod string;
use string::*;
pub(crate) mod crypto;
use crypto::*;
pub(crate) mod json;
use json::*;
pub(crate) mod llm;
use llm::*;
pub(crate) mod http;
use http::*;
pub(crate) mod memory;
pub use memory::init_kv_persist;
use memory::*;
pub(crate) mod cron;
pub use cron::init_reminder_persist;
use cron::*;

impl Builtins {
    pub fn new() -> Self {
        let mut funcs = std::collections::HashMap::new();

        funcs.insert("upper".to_string(), builtin_upper as BuiltinFn);
        funcs.insert("lower".to_string(), builtin_lower as BuiltinFn);
        funcs.insert("len".to_string(), builtin_len as BuiltinFn);
        funcs.insert("str".to_string(), builtin_str as BuiltinFn);
        funcs.insert("print".to_string(), builtin_print as BuiltinFn);
        funcs.insert("contains".to_string(), builtin_contains as BuiltinFn);
        funcs.insert("float".to_string(), builtin_float as BuiltinFn);
        funcs.insert("to_string".to_string(), builtin_to_string as BuiltinFn);
        funcs.insert("get".to_string(), builtin_get as BuiltinFn);
        funcs.insert("push".to_string(), builtin_push as BuiltinFn);
        funcs.insert("slice".to_string(), builtin_slice as BuiltinFn);

        // Phase 7 — environment variable access
        funcs.insert("env".to_string(), builtin_env as BuiltinFn);

        // Phase 5.3 — string operations
        funcs.insert("index_of".to_string(), builtin_index_of as BuiltinFn);
        funcs.insert("substring".to_string(), builtin_substring as BuiltinFn);
        funcs.insert("char_at".to_string(), builtin_char_at as BuiltinFn);
        funcs.insert("starts_with".to_string(), builtin_starts_with as BuiltinFn);
        funcs.insert("ends_with".to_string(), builtin_ends_with as BuiltinFn);
        funcs.insert("to_float".to_string(), builtin_to_float as BuiltinFn);

        // Fluid confidence accessor
        funcs.insert("confidence".to_string(), builtin_confidence as BuiltinFn);

        // Phase 5.4 — stdlib backing builtins (double-underscore prefix)
        funcs.insert("__trim".to_string(), builtin_trim as BuiltinFn);
        funcs.insert("__replace".to_string(), builtin_replace as BuiltinFn);
        funcs.insert("__split".to_string(), builtin_split as BuiltinFn);
        funcs.insert("__join".to_string(), builtin_join as BuiltinFn);
        funcs.insert("__abs".to_string(), builtin_abs as BuiltinFn);
        funcs.insert("__min".to_string(), builtin_min as BuiltinFn);
        funcs.insert("__max".to_string(), builtin_max as BuiltinFn);
        funcs.insert("__clamp".to_string(), builtin_clamp as BuiltinFn);
        funcs.insert("__round".to_string(), builtin_round as BuiltinFn);
        funcs.insert("__first".to_string(), builtin_first as BuiltinFn);
        funcs.insert("__last".to_string(), builtin_last as BuiltinFn);

        // Phase 6.1 — HTTP server stubs
        funcs.insert("respond".to_string(), builtin_respond as BuiltinFn);
        funcs.insert(
            "respond_html".to_string(),
            builtin_respond_html as BuiltinFn,
        );
        funcs.insert("form_data".to_string(), builtin_form_data as BuiltinFn);
        funcs.insert("json_body".to_string(), builtin_json_body as BuiltinFn);
        funcs.insert("query_param".to_string(), builtin_query_param as BuiltinFn);

        // Phase 6.2 — Template stubs
        funcs.insert("render".to_string(), builtin_render as BuiltinFn);
        funcs.insert("escape_html".to_string(), builtin_escape_html as BuiltinFn);

        // Phase 6.3 — Database stubs
        funcs.insert("query".to_string(), builtin_query as BuiltinFn);
        funcs.insert("db_execute".to_string(), builtin_db_execute as BuiltinFn);

        // Phase 6.4 — Encryption stubs
        funcs.insert(
            "hash_password".to_string(),
            builtin_hash_password as BuiltinFn,
        );
        funcs.insert(
            "verify_password".to_string(),
            builtin_verify_password as BuiltinFn,
        );
        funcs.insert("encrypt".to_string(), builtin_encrypt as BuiltinFn);
        funcs.insert("decrypt".to_string(), builtin_decrypt as BuiltinFn);
        funcs.insert(
            "generate_key".to_string(),
            builtin_generate_key as BuiltinFn,
        );

        // Phase 6.5 — Auth stubs
        funcs.insert(
            "authenticate".to_string(),
            builtin_authenticate as BuiltinFn,
        );
        funcs.insert(
            "session_login".to_string(),
            builtin_session_login as BuiltinFn,
        );
        funcs.insert(
            "session_logout".to_string(),
            builtin_session_logout as BuiltinFn,
        );

        // Phase 6.6 — Bot stubs
        funcs.insert(
            "send_message".to_string(),
            builtin_send_message as BuiltinFn,
        );
        funcs.insert(
            "answer_callback_query".to_string(),
            builtin_answer_callback_query as BuiltinFn,
        );
        funcs.insert(
            "edit_message_text".to_string(),
            builtin_edit_message_text as BuiltinFn,
        );
        funcs.insert("require".to_string(), builtin_require as BuiltinFn);

        // Definition of Done — outgoing HTTP
        funcs.insert("http_post".to_string(), builtin_http_post as BuiltinFn);

        // v0.5.0 — top-level string builtins (aliases for __* + new)
        funcs.insert("trim".to_string(), builtin_trim as BuiltinFn);
        funcs.insert("replace".to_string(), builtin_replace as BuiltinFn);
        funcs.insert("split".to_string(), builtin_split as BuiltinFn);
        funcs.insert("join".to_string(), builtin_join as BuiltinFn);
        funcs.insert("length".to_string(), builtin_length as BuiltinFn);
        funcs.insert("to_int".to_string(), builtin_to_int as BuiltinFn);
        funcs.insert("reverse".to_string(), builtin_reverse as BuiltinFn);

        // v0.5.0 — LLM call builtin
        funcs.insert("call_llm".to_string(), builtin_call_llm as BuiltinFn);

        // v0.5.0 — KV memory builtins
        funcs.insert("kv_set".to_string(), builtin_kv_set as BuiltinFn);
        funcs.insert("kv_get".to_string(), builtin_kv_get as BuiltinFn);
        funcs.insert("kv_delete".to_string(), builtin_kv_delete as BuiltinFn);
        funcs.insert("kv_exists".to_string(), builtin_kv_exists as BuiltinFn);
        funcs.insert("kv_list".to_string(), builtin_kv_list as BuiltinFn);

        // Наряд №6 — exact key-value memory (mem_set/mem_get/mem_delete)
        funcs.insert("mem_set".to_string(), builtin_mem_set as BuiltinFn);
        funcs.insert("mem_get".to_string(), builtin_mem_get as BuiltinFn);
        funcs.insert("mem_delete".to_string(), builtin_mem_delete as BuiltinFn);

        // v0.5.0 — File I/O builtins (full set)
        funcs.insert("read_file".to_string(), builtin_read_file as BuiltinFn);
        funcs.insert("write_file".to_string(), builtin_write_file as BuiltinFn);
        funcs.insert("append_file".to_string(), builtin_append_file as BuiltinFn);
        funcs.insert("delete_file".to_string(), builtin_delete_file as BuiltinFn);
        funcs.insert("file_exists".to_string(), builtin_file_exists as BuiltinFn);
        funcs.insert("list_dir".to_string(), builtin_list_dir as BuiltinFn);

        // Anthropic Claude LLM integration (Phase 7.7)
        funcs.insert("call_claude".to_string(), builtin_call_claude as BuiltinFn);

        // Наряд №4: LLM usage tracking
        funcs.insert("llm_usage".to_string(), builtin_llm_usage as BuiltinFn);

        // JSON escape utility (Phase 7.7)
        funcs.insert("escape_json".to_string(), builtin_escape_json as BuiltinFn);

        // Phase 7.7 — new builtins for department modularity
        funcs.insert("parse_json".to_string(), builtin_parse_json as BuiltinFn);
        funcs.insert("json_encode".to_string(), builtin_json_encode as BuiltinFn);
        funcs.insert("json_get".to_string(), builtin_json_get as BuiltinFn);
        funcs.insert("has_field".to_string(), builtin_has_field as BuiltinFn);
        funcs.insert("http_get".to_string(), builtin_http_get as BuiltinFn);
        funcs.insert("now".to_string(), builtin_now as BuiltinFn);

        // ADR-0049 — session memory (temporary per-session KV store)
        funcs.insert("session_set".to_string(), builtin_session_set as BuiltinFn);
        funcs.insert("session_get".to_string(), builtin_session_get as BuiltinFn);
        funcs.insert(
            "session_clear".to_string(),
            builtin_session_clear as BuiltinFn,
        );

        // Voice pipeline builtins (Phase 7.8)
        funcs.insert(
            "http_post_multipart".to_string(),
            builtin_http_post_multipart as BuiltinFn,
        );
        funcs.insert(
            "whisper_transcribe".to_string(),
            builtin_whisper_transcribe as BuiltinFn,
        );
        funcs.insert("tts_send".to_string(), builtin_tts_send as BuiltinFn);

        // Наряд 17: utility builtins — base64, exec, escape_js, dict operations, type_of
        funcs.insert(
            "base64_encode".to_string(),
            builtin_base64_encode as BuiltinFn,
        );
        funcs.insert(
            "base64_decode".to_string(),
            builtin_base64_decode as BuiltinFn,
        );
        funcs.insert("exec".to_string(), builtin_exec as BuiltinFn);
        funcs.insert("escape_js".to_string(), builtin_escape_js as BuiltinFn);
        funcs.insert("dict_get".to_string(), builtin_json_get as BuiltinFn); // alias
        funcs.insert("dict_set".to_string(), builtin_dict_set as BuiltinFn);
        funcs.insert("dict_keys".to_string(), builtin_dict_keys as BuiltinFn);
        funcs.insert("dict_values".to_string(), builtin_dict_values as BuiltinFn);
        funcs.insert("dict_has".to_string(), builtin_dict_has as BuiltinFn);
        funcs.insert("type_of".to_string(), builtin_type_of as BuiltinFn);
        // Наряд №17 В.3: format() — positional string interpolation
        funcs.insert("format".to_string(), builtin_format as BuiltinFn);

        // Наряд №24: git_push, web_search, make_list
        funcs.insert("git_push".to_string(), builtin_git_push as BuiltinFn);
        funcs.insert("web_search".to_string(), builtin_web_search as BuiltinFn);
        funcs.insert("make_list".to_string(), builtin_make_list as BuiltinFn);
        // time() alias for now()
        funcs.insert("time".to_string(), builtin_now as BuiltinFn);
        // format_date() — format unix timestamp or current time (enhanced v0.8.0)
        funcs.insert("format_date".to_string(), builtin_format_date as BuiltinFn);
        // v0.8.0 — Time / Date / Calendar (additional)
        funcs.insert("date_parts".to_string(), builtin_date_parts as BuiltinFn);
        funcs.insert(
            "days_between".to_string(),
            builtin_days_between as BuiltinFn,
        );
        funcs.insert(
            "days_in_month".to_string(),
            builtin_days_in_month as BuiltinFn,
        );
        funcs.insert(
            "is_leap_year".to_string(),
            builtin_is_leap_year as BuiltinFn,
        );
        funcs.insert("add_days".to_string(), builtin_add_days as BuiltinFn);
        funcs.insert("add_hours".to_string(), builtin_add_hours as BuiltinFn);
        funcs.insert(
            "weekday_name".to_string(),
            builtin_weekday_name as BuiltinFn,
        );
        // v0.8.0 — Geolocation
        funcs.insert("geo_ip".to_string(), builtin_geo_ip as BuiltinFn);
        funcs.insert(
            "geo_distance".to_string(),
            builtin_geo_distance as BuiltinFn,
        );
        // v0.8.0 — Weather (Open-Meteo, free, no API key)
        funcs.insert("weather".to_string(), builtin_weather as BuiltinFn);
        funcs.insert(
            "weather_forecast".to_string(),
            builtin_weather_forecast as BuiltinFn,
        );
        // v0.8.0 — Reminders
        funcs.insert("remind".to_string(), builtin_remind as BuiltinFn);
        funcs.insert(
            "remind_recurring".to_string(),
            builtin_remind_recurring as BuiltinFn,
        );
        funcs.insert(
            "cancel_remind".to_string(),
            builtin_cancel_remind as BuiltinFn,
        );
        funcs.insert(
            "list_reminders".to_string(),
            builtin_list_reminders as BuiltinFn,
        );
        funcs.insert(
            "check_reminders".to_string(),
            builtin_check_reminders as BuiltinFn,
        );
        // request_body() alias for json_body() — common in web frameworks
        funcs.insert("request_body".to_string(), builtin_json_body as BuiltinFn);
        // Public first/last (without __ prefix)
        funcs.insert("first".to_string(), builtin_first as BuiltinFn);
        funcs.insert("last".to_string(), builtin_last as BuiltinFn);

        // v0.8.1 — OpenHuman-inspired Human Intelligence builtins
        funcs.insert(
            "human_create".to_string(),
            builtin_human_create as BuiltinFn,
        );
        funcs.insert("human_mood".to_string(), builtin_human_mood as BuiltinFn);
        funcs.insert(
            "human_remember".to_string(),
            builtin_human_remember as BuiltinFn,
        );
        funcs.insert(
            "human_forget".to_string(),
            builtin_human_forget as BuiltinFn,
        );
        funcs.insert(
            "human_recall".to_string(),
            builtin_human_recall as BuiltinFn,
        );
        funcs.insert(
            "human_respond".to_string(),
            builtin_human_respond as BuiltinFn,
        );
        funcs.insert(
            "human_personas".to_string(),
            builtin_human_personas as BuiltinFn,
        );
        funcs.insert(
            "human_delete".to_string(),
            builtin_human_delete as BuiltinFn,
        );

        // Problem B (Наряд reverse-iteration): list aggregation
        funcs.insert("zip".to_string(), builtin_zip as BuiltinFn);
        funcs.insert("sort_by".to_string(), builtin_sort_by as BuiltinFn);
        funcs.insert("filter".to_string(), builtin_filter as BuiltinFn);
        funcs.insert("reduce".to_string(), builtin_reduce as BuiltinFn);
        // Problem D helper + Problem A helper
        funcs.insert(
            "extract_param".to_string(),
            builtin_extract_param as BuiltinFn,
        );
        funcs.insert(
            "estimate_tokens".to_string(),
            builtin_estimate_tokens as BuiltinFn,
        );
        // Problem A (reverse-iteration): skill index helpers
        funcs.insert("matches_any".to_string(), builtin_matches_any as BuiltinFn);
        funcs.insert(
            "read_file_tokens".to_string(),
            builtin_read_file_tokens as BuiltinFn,
        );

        // ── OpenHuman-inspired: Scheduling (Tier 1 #1) ──
        funcs.insert("cron_add".to_string(), builtin_cron_add as BuiltinFn);
        funcs.insert("cron_list".to_string(), builtin_cron_list as BuiltinFn);
        funcs.insert("cron_remove".to_string(), builtin_cron_remove as BuiltinFn);
        funcs.insert("cron_run".to_string(), builtin_cron_run as BuiltinFn);
        funcs.insert(
            "cron_mark_fired".to_string(),
            builtin_cron_mark_fired as BuiltinFn,
        );

        // ── OpenHuman-inspired: Approval Gate (Tier 1 #10) ──
        funcs.insert(
            "ask_approval".to_string(),
            builtin_ask_approval as BuiltinFn,
        );

        // ── OpenHuman-inspired: Goals & Todos (Tier 1 #5, #6) ──
        funcs.insert("goal_set".to_string(), builtin_goal_set as BuiltinFn);
        funcs.insert("goal_get".to_string(), builtin_goal_get as BuiltinFn);
        funcs.insert(
            "goal_complete".to_string(),
            builtin_goal_complete as BuiltinFn,
        );
        funcs.insert("goals_list".to_string(), builtin_goals_list as BuiltinFn);
        funcs.insert("goals_add".to_string(), builtin_goals_add as BuiltinFn);
        funcs.insert(
            "goals_reflect".to_string(),
            builtin_goals_reflect as BuiltinFn,
        );
        funcs.insert("todo_add".to_string(), builtin_todo_add as BuiltinFn);
        funcs.insert("todo_update".to_string(), builtin_todo_update as BuiltinFn);
        funcs.insert("todo_list".to_string(), builtin_todo_list as BuiltinFn);

        // ── OpenHuman-inspired: Entity Extraction (Tier 1 #4) ──
        funcs.insert(
            "extract_entities".to_string(),
            builtin_extract_entities as BuiltinFn,
        );

        // ── OpenHuman-inspired: Memory Scoring (Tier 1 #3) ──
        funcs.insert(
            "memory_score".to_string(),
            builtin_memory_score as BuiltinFn,
        );

        // ── OpenHuman-inspired: Token Compression — HTML (Tier 1 #2) ──
        funcs.insert(
            "compress_html".to_string(),
            builtin_compress_html as BuiltinFn,
        );

        // ── OpenHuman-inspired: Personalization (Tier 2 #12) ──
        funcs.insert(
            "learn_preference".to_string(),
            builtin_learn_preference as BuiltinFn,
        );
        funcs.insert("get_profile".to_string(), builtin_get_profile as BuiltinFn);

        // ── Memory Tree (OpenHuman-inspired Tier 1 #9) ──
        funcs.insert("mtree_store".to_string(), builtin_mtree_store as BuiltinFn);
        funcs.insert(
            "mtree_retrieve".to_string(),
            builtin_mtree_retrieve as BuiltinFn,
        );
        funcs.insert(
            "mtree_forget".to_string(),
            builtin_mtree_forget as BuiltinFn,
        );
        funcs.insert(
            "mtree_summarize".to_string(),
            builtin_mtree_summarize as BuiltinFn,
        );
        funcs.insert("mtree_stats".to_string(), builtin_mtree_stats as BuiltinFn);
        funcs.insert("graph_query".to_string(), builtin_graph_query as BuiltinFn);
        funcs.insert("graph_path".to_string(), builtin_graph_path as BuiltinFn);
        funcs.insert(
            "graph_neighbors".to_string(),
            builtin_graph_neighbors as BuiltinFn,
        );
        funcs.insert(
            "memory_decay".to_string(),
            builtin_memory_decay as BuiltinFn,
        );
        funcs.insert(
            "memory_boost".to_string(),
            builtin_memory_boost as BuiltinFn,
        );
        funcs.insert(
            "memory_prune".to_string(),
            builtin_memory_prune as BuiltinFn,
        );
        funcs.insert(
            "memory_revise".to_string(),
            builtin_memory_revise as BuiltinFn,
        );
        funcs.insert(
            "subgraph_extract".to_string(),
            builtin_subgraph_extract as BuiltinFn,
        );
        funcs.insert(
            "subgraph_nodes".to_string(),
            builtin_subgraph_nodes as BuiltinFn,
        );
        funcs.insert(
            "subgraph_json".to_string(),
            builtin_subgraph_json as BuiltinFn,
        );
        funcs.insert("trace_start".to_string(), builtin_trace_start as BuiltinFn);
        funcs.insert("trace_end".to_string(), builtin_trace_end as BuiltinFn);
        funcs.insert("assert_eq".to_string(), builtin_assert_eq as BuiltinFn);
        funcs.insert(
            "assert_contains".to_string(),
            builtin_assert_contains as BuiltinFn,
        );
        // ── sqz-inspired: String/List utilities (P1) ──
        funcs.insert("squeeze".to_string(), builtin_squeeze as BuiltinFn);
        funcs.insert("dedup".to_string(), builtin_dedup as BuiltinFn);
        funcs.insert("condense".to_string(), builtin_condense as BuiltinFn);
        funcs.insert("strip".to_string(), builtin_strip as BuiltinFn);
        funcs.insert("chomp".to_string(), builtin_chomp as BuiltinFn);
        funcs.insert("repeat".to_string(), builtin_repeat as BuiltinFn);
        funcs.insert("pad_left".to_string(), builtin_pad_left as BuiltinFn);
        funcs.insert("pad_right".to_string(), builtin_pad_right as BuiltinFn);
        funcs.insert("lines".to_string(), builtin_lines as BuiltinFn);
        funcs.insert("words".to_string(), builtin_words as BuiltinFn);
        // ── sqz-inspired: TOON encoding (P2) ──
        funcs.insert("toon_encode".to_string(), builtin_toon_encode as BuiltinFn);
        funcs.insert("toon_decode".to_string(), builtin_toon_decode as BuiltinFn);
        // ── sqz-inspired: Content-addressed refs (P2) ──
        funcs.insert("ref".to_string(), builtin_content_ref as BuiltinFn);
        funcs.insert("deref".to_string(), builtin_content_deref as BuiltinFn);
        // ── sqz-inspired: Token awareness (P3) ──
        funcs.insert("token_count".to_string(), builtin_token_count as BuiltinFn);
        // ── AgentSkillOS-inspired: Recipe system + DAG orchestration (ADR-0062) ──
        funcs.insert("recipe_save".to_string(), builtin_recipe_save as BuiltinFn);
        funcs.insert(
            "recipe_search".to_string(),
            builtin_recipe_search as BuiltinFn,
        );
        funcs.insert("recipe_list".to_string(), builtin_recipe_list as BuiltinFn);
        funcs.insert("dag_phases".to_string(), builtin_dag_phases as BuiltinFn);
        funcs.insert("topo_sort".to_string(), builtin_topo_sort as BuiltinFn);
        // ── OpenPlanter-inspired: Fuzzy matching, safe editing, agent utilities (ADR-0063) ──
        funcs.insert("fuzzy_match".to_string(), builtin_fuzzy_match as BuiltinFn);
        funcs.insert(
            "fuzzy_find_best".to_string(),
            builtin_fuzzy_find_best as BuiltinFn,
        );
        funcs.insert(
            "hashline_read".to_string(),
            builtin_hashline_read as BuiltinFn,
        );
        funcs.insert(
            "hashline_edit".to_string(),
            builtin_hashline_edit as BuiltinFn,
        );
        funcs.insert(
            "compact_list".to_string(),
            builtin_compact_list as BuiltinFn,
        );
        funcs.insert(
            "budget_check".to_string(),
            builtin_budget_check as BuiltinFn,
        );
        funcs.insert(
            "replay_snapshot".to_string(),
            builtin_replay_snapshot as BuiltinFn,
        );
        funcs.insert(
            "policy_check".to_string(),
            builtin_policy_check as BuiltinFn,
        );

        // ── obsidian-mind inspired: Vault/memory (v0.10.0) ──
        funcs.insert(
            "semantic_search".to_string(),
            builtin_semantic_search as BuiltinFn,
        );
        funcs.insert("config_load".to_string(), builtin_config_load as BuiltinFn);
        funcs.insert(
            "vault_validate".to_string(),
            builtin_vault_validate as BuiltinFn,
        );
        Builtins { funcs }
    }

    /// Verify builtin registry consistency (debug builds).
    #[cfg(debug_assertions)]
    fn check_registry_sync(&self) {
        for spec in BUILTIN_REGISTRY.iter() {
            if spec.category != "stateful"
                && spec.category != "stub"
                && spec.category != "graph"
                && spec.category != "mtree"
                && spec.category != "cron"
                && spec.category != "test"
            {
                assert!(
                    self.funcs.contains_key(spec.name),
                    "BUILTIN_REGISTRY '{}' has no handler in Builtins::new()",
                    spec.name
                );
            }
        }
    }

    /// Look up a built-in by name.
    pub fn get(&self, name: &str) -> Option<&BuiltinFn> {
        self.funcs.get(name)
    }
}

pub(crate) mod server;
use server::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn is_string(val: &Value, expected: &str) -> bool {
        matches!(val, Value::String(s) if s == expected)
    }

    fn is_float(val: &Value, expected: f64) -> bool {
        matches!(val, Value::Float(f) if (*f - expected).abs() < 1e-9)
    }

    fn is_unit(val: &Value) -> bool {
        matches!(val, Value::Unit)
    }

    #[test]
    fn test_json_get_existing_field() {
        let obj = make_struct(
            "Test",
            vec![
                ("name", Value::String("Alice".to_string())),
                ("age", Value::Float(25.0)),
            ],
        );
        let result = builtin_json_get(&[obj, Value::String("name".to_string())]).unwrap();
        assert!(is_string(&result, "Alice"));
    }

    #[test]
    fn test_json_get_missing_field_returns_unit() {
        let obj = make_struct("Test", vec![("name", Value::String("Alice".to_string()))]);
        let result = builtin_json_get(&[obj, Value::String("voice".to_string())]).unwrap();
        assert!(
            is_unit(&result),
            "expected Unit, got {:?}",
            result.type_name()
        );
    }

    #[test]
    fn test_json_get_missing_field_returns_custom_default() {
        let obj = make_struct("Test", vec![("name", Value::String("Alice".to_string()))]);
        let result = builtin_json_get(&[
            obj,
            Value::String("voice".to_string()),
            Value::String("none".to_string()),
        ])
        .unwrap();
        assert!(is_string(&result, "none"));
    }

    #[test]
    fn test_json_get_nested_path() {
        let inner = make_struct(
            "Test",
            vec![("file_id", Value::String("abc123".to_string()))],
        );
        let obj = make_struct("Test", vec![("voice", inner)]);
        let result = builtin_json_get(&[obj, Value::String("voice.file_id".to_string())]).unwrap();
        assert!(is_string(&result, "abc123"));
    }

    #[test]
    fn test_json_get_nested_path_missing() {
        let obj = make_struct("Test", vec![("text", Value::String("hello".to_string()))]);
        let result = builtin_json_get(&[
            obj,
            Value::String("voice.file_id".to_string()),
            Value::String("default".to_string()),
        ])
        .unwrap();
        assert!(is_string(&result, "default"));
    }

    #[test]
    fn test_json_get_non_struct_returns_default() {
        let obj = Value::String("not a struct".to_string());
        let result =
            builtin_json_get(&[obj, Value::String("field".to_string()), Value::Float(42.0)])
                .unwrap();
        assert!(is_float(&result, 42.0));
    }

    #[test]
    fn test_has_field_existing() {
        let obj = make_struct("Test", vec![("voice", Value::String("data".to_string()))]);
        let result = builtin_has_field(&[obj, Value::String("voice".to_string())]).unwrap();
        assert!(is_float(&result, 1.0));
    }

    #[test]
    fn test_has_field_missing() {
        let obj = make_struct("Test", vec![("text", Value::String("hi".to_string()))]);
        let result = builtin_has_field(&[obj, Value::String("voice".to_string())]).unwrap();
        assert!(is_float(&result, 0.0));
    }

    #[test]
    fn test_has_field_nested() {
        let inner = make_struct("Test", vec![("file_id", Value::String("x".to_string()))]);
        let obj = make_struct("Test", vec![("voice", inner)]);
        let result = builtin_has_field(&[obj, Value::String("voice.file_id".to_string())]).unwrap();
        assert!(is_float(&result, 1.0));
    }

    #[test]
    fn test_json_encode_roundtrip() {
        let obj = make_struct(
            "Test",
            vec![
                ("key", Value::String("value".to_string())),
                ("num", Value::Float(42.0)),
            ],
        );
        let encoded = builtin_json_encode(&[obj]).unwrap();
        let decoded = builtin_parse_json(&[encoded]).unwrap();
        let key_val = decoded.get_field("key").unwrap();
        assert!(is_string(key_val, "value"));
        let num_val = decoded.get_field("num").unwrap();
        assert!(is_float(num_val, 42.0));
    }

    #[test]
    fn test_escape_json_handles_special_chars() {
        let result = builtin_escape_json(&[Value::String("hello\"world\n".to_string())]).unwrap();
        assert!(is_string(&result, "hello\\\"world\\n"));
    }

    // ── slice() unit tests (ADR-0069) ──────────────────────────
    fn make_list(items: Vec<&str>) -> Value {
        Value::List(items.iter().map(|s| Value::String(s.to_string())).collect())
    }

    fn list_strings(val: &Value) -> Vec<String> {
        match val {
            Value::List(items) => items
                .iter()
                .filter_map(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    _ => None,
                })
                .collect(),
            _ => vec![],
        }
    }

    #[test]
    fn test_slice_mid_range() {
        let list = make_list(vec!["a", "b", "c", "d", "e"]);
        let result = builtin_slice(&[list, Value::Float(1.0), Value::Float(3.0)]).unwrap();
        assert_eq!(list_strings(&result), vec!["b", "c"]);
    }

    #[test]
    fn test_slice_tail_to_end() {
        let list = make_list(vec!["a", "b", "c", "d", "e"]);
        let result = builtin_slice(&[list, Value::Float(2.0), Value::Float(5.0)]).unwrap();
        assert_eq!(list_strings(&result), vec!["c", "d", "e"]);
    }

    #[test]
    fn test_slice_end_past_len_clamps() {
        let list = make_list(vec!["a", "b", "c", "d", "e"]);
        let result = builtin_slice(&[list, Value::Float(3.0), Value::Float(99.0)]).unwrap();
        assert_eq!(list_strings(&result), vec!["d", "e"]);
    }

    #[test]
    fn test_slice_start_past_len_empty() {
        let list = make_list(vec!["a", "b", "c", "d", "e"]);
        let result = builtin_slice(&[list, Value::Float(9.0), Value::Float(10.0)]).unwrap();
        assert_eq!(list_strings(&result), Vec::<String>::new());
    }

    #[test]
    fn test_slice_start_ge_end_empty() {
        let list = make_list(vec!["a", "b", "c", "d", "e"]);
        let result = builtin_slice(&[list, Value::Float(3.0), Value::Float(1.0)]).unwrap();
        assert_eq!(list_strings(&result), Vec::<String>::new());
    }

    #[test]
    fn test_slice_non_list_errors() {
        let result = builtin_slice(&[
            Value::String("not a list".to_string()),
            Value::Float(0.0),
            Value::Float(1.0),
        ]);
        assert!(result.is_err());
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests_sqz_builtins {
    use super::*;

    /// Helper: compare two Values via their JSON representation
    /// (Value does not implement PartialEq due to Subgraph variant).
    fn assert_vals_eq(actual: &Value, expected: &Value, label: &str) {
        let a = serde_json::to_string(&mlog_value_to_json(actual)).unwrap();
        let e = serde_json::to_string(&mlog_value_to_json(expected)).unwrap();
        assert_eq!(a, e, "{}", label);
    }

    /// Helper: extract String from Value, panic if not String.
    fn as_str(v: &Value) -> &str {
        match v {
            Value::String(s) => s,
            other => panic!("expected String, got {}", other.type_name()),
        }
    }

    /// Helper: extract Float from Value, panic if not Float.
    fn as_f64(v: &Value) -> f64 {
        match v {
            Value::Float(f) => *f,
            other => panic!("expected Float, got {}", other.type_name()),
        }
    }

    // ── P1 tests ──

    #[test]
    fn test_squeeze_basic() {
        let r = builtin_squeeze(&[
            Value::String("aaabbbccc".into()),
            Value::String("abc".into()),
        ])
        .unwrap();
        assert_vals_eq(&r, &Value::String("abc".into()), "squeeze_basic");
    }

    #[test]
    fn test_squeeze_partial() {
        let r = builtin_squeeze(&[
            Value::String("aaabbbccc".into()),
            Value::String("ab".into()),
        ])
        .unwrap();
        assert_vals_eq(&r, &Value::String("abccc".into()), "squeeze_partial");
    }

    #[test]
    fn test_squeeze_empty_chars() {
        let r =
            builtin_squeeze(&[Value::String("hello".into()), Value::String("".into())]).unwrap();
        assert_vals_eq(&r, &Value::String("hello".into()), "squeeze_empty_chars");
    }

    #[test]
    fn test_squeeze_empty_string() {
        let r = builtin_squeeze(&[Value::String("".into()), Value::String("a".into())]).unwrap();
        assert_vals_eq(&r, &Value::String("".into()), "squeeze_empty_string");
    }

    #[test]
    fn test_dedup_basic() {
        let r = builtin_dedup(&[Value::List(vec![
            Value::Float(1.0),
            Value::Float(1.0),
            Value::Float(2.0),
            Value::Float(2.0),
            Value::Float(3.0),
        ])])
        .unwrap();
        assert_vals_eq(
            &r,
            &Value::List(vec![
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(3.0),
            ]),
            "dedup_basic",
        );
    }

    #[test]
    fn test_dedup_strings() {
        let r = builtin_dedup(&[Value::List(vec![
            Value::String("a".into()),
            Value::String("b".into()),
            Value::String("a".into()),
            Value::String("c".into()),
        ])])
        .unwrap();
        assert_vals_eq(
            &r,
            &Value::List(vec![
                Value::String("a".into()),
                Value::String("b".into()),
                Value::String("c".into()),
            ]),
            "dedup_strings",
        );
    }

    #[test]
    fn test_dedup_empty() {
        let r = builtin_dedup(&[Value::List(vec![])]).unwrap();
        assert_vals_eq(&r, &Value::List(vec![]), "dedup_empty");
    }

    #[test]
    fn test_condense_basic() {
        let r = builtin_condense(&[Value::List(vec![
            Value::String("error".into()),
            Value::String("error".into()),
            Value::String("error".into()),
            Value::String("warn".into()),
            Value::String("info".into()),
        ])])
        .unwrap();
        assert_vals_eq(
            &r,
            &Value::List(vec![
                Value::String("error".into()),
                Value::String("\u{00d7}3".into()),
                Value::String("warn".into()),
                Value::String("info".into()),
            ]),
            "condense_basic",
        );
    }

    #[test]
    fn test_condense_repeating_groups() {
        let r = builtin_condense(&[Value::List(vec![
            Value::String("a".into()),
            Value::String("a".into()),
            Value::String("b".into()),
            Value::String("b".into()),
            Value::String("b".into()),
            Value::String("a".into()),
        ])])
        .unwrap();
        assert_vals_eq(
            &r,
            &Value::List(vec![
                Value::String("a".into()),
                Value::String("\u{00d7}2".into()),
                Value::String("b".into()),
                Value::String("\u{00d7}3".into()),
                Value::String("a".into()),
            ]),
            "condense_repeating_groups",
        );
    }

    #[test]
    fn test_condense_single() {
        let r = builtin_condense(&[Value::List(vec![Value::String("single".into())])]).unwrap();
        assert_vals_eq(
            &r,
            &Value::List(vec![Value::String("single".into())]),
            "condense_single",
        );
    }

    #[test]
    fn test_strip_basic() {
        let r = builtin_strip(&[
            Value::String("///hello///".into()),
            Value::String("/".into()),
        ])
        .unwrap();
        assert_vals_eq(&r, &Value::String("hello".into()), "strip_basic");
    }

    #[test]
    fn test_strip_whitespace() {
        let r =
            builtin_strip(&[Value::String("  hello  ".into()), Value::String(" ".into())]).unwrap();
        assert_vals_eq(&r, &Value::String("hello".into()), "strip_whitespace");
    }

    #[test]
    fn test_strip_no_match() {
        let r = builtin_strip(&[Value::String("abc".into()), Value::String("x".into())]).unwrap();
        assert_vals_eq(&r, &Value::String("abc".into()), "strip_no_match");
    }

    #[test]
    fn test_chomp_newline() {
        let r = builtin_chomp(&[Value::String("hello\n".into())]).unwrap();
        assert_vals_eq(&r, &Value::String("hello".into()), "chomp_newline");
    }

    #[test]
    fn test_chomp_crlf() {
        let r = builtin_chomp(&[Value::String("hello\r\n".into())]).unwrap();
        assert_vals_eq(&r, &Value::String("hello".into()), "chomp_crlf");
    }

    #[test]
    fn test_chomp_no_newline() {
        let r = builtin_chomp(&[Value::String("hello".into())]).unwrap();
        assert_vals_eq(&r, &Value::String("hello".into()), "chomp_no_newline");
    }

    #[test]
    fn test_chomp_double_newline() {
        let r = builtin_chomp(&[Value::String("hello\n\n".into())]).unwrap();
        assert_vals_eq(&r, &Value::String("hello\n".into()), "chomp_double_newline");
    }

    #[test]
    fn test_repeat_basic() {
        let r = builtin_repeat(&[Value::String("-".into()), Value::Float(10.0)]).unwrap();
        assert_vals_eq(&r, &Value::String("----------".into()), "repeat_basic");
    }

    #[test]
    fn test_repeat_multiple() {
        let r = builtin_repeat(&[Value::String("ab".into()), Value::Float(3.0)]).unwrap();
        assert_vals_eq(&r, &Value::String("ababab".into()), "repeat_multiple");
    }

    #[test]
    fn test_repeat_zero() {
        let r = builtin_repeat(&[Value::String("x".into()), Value::Float(0.0)]).unwrap();
        assert_vals_eq(&r, &Value::String("".into()), "repeat_zero");
    }

    #[test]
    fn test_repeat_negative() {
        assert!(builtin_repeat(&[Value::String("x".into()), Value::Float(-1.0)]).is_err());
    }

    #[test]
    fn test_repeat_non_integer() {
        assert!(builtin_repeat(&[Value::String("x".into()), Value::Float(2.5)]).is_err());
    }

    #[test]
    fn test_pad_left_basic() {
        let r = builtin_pad_left(&[
            Value::String("42".into()),
            Value::Float(5.0),
            Value::String("0".into()),
        ])
        .unwrap();
        assert_vals_eq(&r, &Value::String("00042".into()), "pad_left_basic");
    }

    #[test]
    fn test_pad_left_noop() {
        let r = builtin_pad_left(&[
            Value::String("hello".into()),
            Value::Float(3.0),
            Value::String("x".into()),
        ])
        .unwrap();
        assert_vals_eq(&r, &Value::String("hello".into()), "pad_left_noop");
    }

    #[test]
    fn test_pad_right_basic() {
        let r = builtin_pad_right(&[
            Value::String("name".into()),
            Value::Float(10.0),
            Value::String(".".into()),
        ])
        .unwrap();
        assert_vals_eq(&r, &Value::String("name......".into()), "pad_right_basic");
        // 4+6=10
    }

    #[test]
    fn test_lines_basic() {
        let r = builtin_lines(&[Value::String("a\nb\nc".into())]).unwrap();
        assert_vals_eq(
            &r,
            &Value::List(vec![
                Value::String("a".into()),
                Value::String("b".into()),
                Value::String("c".into()),
            ]),
            "lines_basic",
        );
    }

    #[test]
    fn test_lines_trailing_newline() {
        let r = builtin_lines(&[Value::String("hello\nworld\n".into())]).unwrap();
        assert_vals_eq(
            &r,
            &Value::List(vec![
                Value::String("hello".into()),
                Value::String("world".into()),
            ]),
            "lines_trailing_newline",
        );
    }

    #[test]
    fn test_lines_empty() {
        let r = builtin_lines(&[Value::String("".into())]).unwrap();
        assert_vals_eq(&r, &Value::List(vec![]), "lines_empty");
    }

    #[test]
    fn test_words_basic() {
        let r = builtin_words(&[Value::String("hello world".into())]).unwrap();
        assert_vals_eq(
            &r,
            &Value::List(vec![
                Value::String("hello".into()),
                Value::String("world".into()),
            ]),
            "words_basic",
        );
    }

    #[test]
    fn test_words_extra_whitespace() {
        let r = builtin_words(&[Value::String("  a  b  c  ".into())]).unwrap();
        assert_vals_eq(
            &r,
            &Value::List(vec![
                Value::String("a".into()),
                Value::String("b".into()),
                Value::String("c".into()),
            ]),
            "words_extra_whitespace",
        );
    }

    #[test]
    fn test_words_empty() {
        let r = builtin_words(&[Value::String("".into())]).unwrap();
        assert_vals_eq(&r, &Value::List(vec![]), "words_empty");
    }

    // ── P2 tests: TOON ──

    #[test]
    fn test_toon_encode_string() {
        let r = builtin_toon_encode(&[Value::String("hello".into())]).unwrap();
        assert_eq!(as_str(&r), "TOON:s\"hello\"");
    }

    #[test]
    fn test_toon_encode_float() {
        let r = builtin_toon_encode(&[Value::Float(42.0)]).unwrap();
        assert_eq!(as_str(&r), "TOON:42");
    }

    #[test]
    fn test_toon_encode_list() {
        let r = builtin_toon_encode(&[Value::List(vec![
            Value::Float(1.0),
            Value::Float(2.0),
            Value::Float(3.0),
        ])])
        .unwrap();
        assert_eq!(as_str(&r), "TOON:[1,2,3]");
    }

    #[test]
    fn test_toon_encode_bool() {
        let r = builtin_toon_encode(&[Value::Bool(true)]).unwrap();
        assert_eq!(as_str(&r), "TOON:true");
    }

    #[test]
    fn test_toon_encode_null() {
        let r = builtin_toon_encode(&[Value::Unit]).unwrap();
        assert_eq!(as_str(&r), "TOON:null");
    }

    #[test]
    fn test_toon_encode_struct() {
        let mut fields = std::collections::HashMap::new();
        fields.insert("name".to_string(), Value::String("Alice".into()));
        fields.insert("age".to_string(), Value::Float(30.0));
        let r = builtin_toon_encode(&[Value::Struct {
            type_name: "Person".into(),
            fields,
        }])
        .unwrap();
        let s = as_str(&r);
        assert!(s.starts_with("TOON:{"));
        assert!(s.contains("name:s\"Alice\""));
        assert!(s.contains("age:30"));
        assert!(s.ends_with("}"));
    }

    #[test]
    fn test_toon_roundtrip_string() {
        let original = Value::String("hello world".into());
        let encoded = builtin_toon_encode(&[original.clone()]).unwrap();
        let decoded = builtin_toon_decode(&[encoded]).unwrap();
        assert_vals_eq(&original, &decoded, "roundtrip_string");
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_toon_roundtrip_float() {
        let original = Value::Float(3.14);
        let encoded = builtin_toon_encode(&[original.clone()]).unwrap();
        let decoded = builtin_toon_decode(&[encoded]).unwrap();
        assert_vals_eq(&original, &decoded, "roundtrip_float");
    }

    #[test]
    fn test_toon_roundtrip_list() {
        let original = Value::List(vec![
            Value::Float(1.0),
            Value::String("ok".into()),
            Value::Bool(false),
        ]);
        let encoded = builtin_toon_encode(&[original.clone()]).unwrap();
        let decoded = builtin_toon_decode(&[encoded]).unwrap();
        assert_vals_eq(&original, &decoded, "roundtrip_list");
    }

    #[test]
    fn test_toon_roundtrip_struct() {
        let mut fields = std::collections::HashMap::new();
        fields.insert("x".to_string(), Value::Float(1.0));
        let original = Value::Struct {
            type_name: "P".into(),
            fields,
        };
        let encoded = builtin_toon_encode(&[original.clone()]).unwrap();
        let decoded = builtin_toon_decode(&[encoded]).unwrap();
        // Compare JSON representations (type_name may differ: "P" vs "TOON")
        assert_eq!(
            serde_json::to_string(&mlog_value_to_json(&original)).unwrap(),
            serde_json::to_string(&mlog_value_to_json(&decoded)).unwrap(),
            "roundtrip_struct JSON mismatch"
        );
    }

    #[test]
    fn test_toon_roundtrip_cyrillic() {
        let original = Value::String("\u{041f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}".into());
        let encoded = builtin_toon_encode(&[original.clone()]).unwrap();
        let decoded = builtin_toon_decode(&[encoded]).unwrap();
        assert_vals_eq(&original, &decoded, "roundtrip_cyrillic");
    }

    #[test]
    fn test_toon_decode_no_prefix() {
        let r = builtin_toon_decode(&[Value::String("invalid".into())]);
        assert!(r.is_err());
    }

    #[test]
    fn test_toon_decode_invalid() {
        let r = builtin_toon_decode(&[Value::String("TOON:{{broken".into())]);
        assert!(r.is_err());
    }

    // ── P2 tests: ref/deref ──

    #[test]
    fn test_ref_deref_roundtrip() {
        let content = "hello world test roundtrip";
        let hash_val = builtin_content_ref(&[Value::String(content.into())]).unwrap();
        let hash_str = as_str(&hash_val);
        assert_eq!(hash_str.len(), 64); // SHA-256 hex = 64 chars
        let derefed = builtin_content_deref(&[Value::String(hash_str.to_string())]).unwrap();
        assert_eq!(as_str(&derefed), content);
    }

    #[test]
    fn test_ref_idempotent() {
        let content = "idempotent test content";
        let h1 = builtin_content_ref(&[Value::String(content.into())]).unwrap();
        let h2 = builtin_content_ref(&[Value::String(content.into())]).unwrap();
        assert_eq!(as_str(&h1), as_str(&h2)); // same hash
    }

    #[test]
    fn test_deref_not_found() {
        let r = builtin_content_deref(&[Value::String("a".repeat(64))]);
        assert!(r.is_err());
    }

    #[test]
    fn test_deref_invalid_format() {
        assert!(builtin_content_deref(&[Value::String("tooshort".into())]).is_err());
        assert!(builtin_content_deref(&[Value::String("zz".repeat(32).into())]).is_err());
    }

    // ── P3 tests: token_count ──

    #[test]
    fn test_token_count_ascii() {
        // "hello world" = 11 chars, /4 = 2.75, ceil = 3
        let r = builtin_token_count(&[Value::String("hello world".into())]).unwrap();
        assert_eq!(as_f64(&r), 3.0);
    }

    #[test]
    fn test_token_count_cyrillic() {
        // 10 cyrillic chars + 1 space, /2 = 5
        let r = builtin_token_count(&[Value::String(
            "\u{041f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442} \u{043c}\u{0438}\u{0440}".into(),
        )])
        .unwrap();
        assert_eq!(as_f64(&r), 5.0);
    }

    #[test]
    fn test_token_count_empty() {
        let r = builtin_token_count(&[Value::String("".into())]).unwrap();
        assert_eq!(as_f64(&r), 0.0);
    }

    #[test]
    fn test_token_count_mixed() {
        // "Hello мир" = 9 chars, 3 cyrillic = 33%, < 50% so /4 = 2.25, ceil = 3
        let r =
            builtin_token_count(&[Value::String("Hello \u{043c}\u{0438}\u{0440}".into())]).unwrap();
        assert_eq!(as_f64(&r), 3.0);
    }

    // ── AgentSkillOS-inspired tests (ADR-0062) ──

    // Helper: build a DAG node struct Value
    fn dag_node(id: &str, deps: &[&str]) -> Value {
        let mut fields = std::collections::HashMap::new();
        fields.insert("id".to_string(), Value::String(id.to_string()));
        fields.insert(
            "depends_on".to_string(),
            Value::List(deps.iter().map(|d| Value::String(d.to_string())).collect()),
        );
        Value::Struct {
            type_name: "DagNode".to_string(),
            fields,
        }
    }

    #[test]
    fn test_recipe_save_basic() {
        let r = builtin_recipe_save(&[
            Value::String("bug_report".into()),
            Value::String("Generate bug report from logs".into()),
            Value::List(vec![
                Value::String("log_analyze".into()),
                Value::String("report_gen".into()),
            ]),
            Value::List(vec![]),
        ])
        .unwrap();
        // Should return a struct with "key" and "recipe" fields
        match &r {
            Value::Struct { fields, .. } => {
                assert!(
                    fields.contains_key("key"),
                    "recipe_save result must have 'key'"
                );
                assert!(
                    fields.contains_key("recipe"),
                    "recipe_save result must have 'recipe'"
                );
                let key = &fields["key"];
                match key {
                    Value::String(s) => assert!(
                        s.starts_with("__recipe:bug_report"),
                        "key should start with __recipe:bug_report, got {}",
                        s
                    ),
                    _ => panic!("key should be String"),
                }
            }
            _ => panic!("recipe_save should return Struct, got {}", r.type_name()),
        }
    }

    #[test]
    fn test_recipe_save_too_few_args() {
        let r = builtin_recipe_save(&[Value::String("x".into())]);
        assert!(r.is_err(), "recipe_save with 1 arg should error");
    }

    #[test]
    fn test_recipe_search_placeholder() {
        let r = builtin_recipe_search(&[Value::String("bug".into())]).unwrap();
        assert_vals_eq(
            &r,
            &Value::List(vec![]),
            "recipe_search placeholder returns empty list",
        );
    }

    #[test]
    fn test_recipe_list_placeholder() {
        let r = builtin_recipe_list(&[]).unwrap();
        assert_vals_eq(
            &r,
            &Value::List(vec![]),
            "recipe_list placeholder returns empty list",
        );
    }

    #[test]
    fn test_dag_phases_simple_chain() {
        // A -> B -> C (linear chain: 3 phases, 1 node each)
        let dag = Value::List(vec![
            dag_node("a", &[]),
            dag_node("b", &["a"]),
            dag_node("c", &["b"]),
        ]);
        let r = builtin_dag_phases(&[dag]).unwrap();
        match &r {
            Value::List(phases) => {
                assert_eq!(phases.len(), 3, "linear chain should have 3 phases");
                // Phase 0: [a]
                match &phases[0] {
                    Value::List(ids) => {
                        assert_eq!(ids.len(), 1);
                        assert_eq!(as_str(&ids[0]), "a");
                    }
                    _ => panic!("phase should be List"),
                }
                // Phase 1: [b]
                match &phases[1] {
                    Value::List(ids) => {
                        assert_eq!(ids.len(), 1);
                        assert_eq!(as_str(&ids[0]), "b");
                    }
                    _ => panic!("phase should be List"),
                }
                // Phase 2: [c]
                match &phases[2] {
                    Value::List(ids) => {
                        assert_eq!(ids.len(), 1);
                        assert_eq!(as_str(&ids[0]), "c");
                    }
                    _ => panic!("phase should be List"),
                }
            }
            _ => panic!("dag_phases should return List of phases"),
        }
    }

    #[test]
    fn test_dag_phases_parallel() {
        // A (no deps), B (no deps), C (depends on A, B) — 2 phases
        let dag = Value::List(vec![
            dag_node("a", &[]),
            dag_node("b", &[]),
            dag_node("c", &["a", "b"]),
        ]);
        let r = builtin_dag_phases(&[dag]).unwrap();
        match &r {
            Value::List(phases) => {
                assert_eq!(phases.len(), 2, "diamond should have 2 phases");
                // Phase 0: [a, b] (parallel)
                match &phases[0] {
                    Value::List(ids) => {
                        assert_eq!(ids.len(), 2, "phase 0 should have 2 parallel nodes")
                    }
                    _ => panic!("phase should be List"),
                }
                // Phase 1: [c]
                match &phases[1] {
                    Value::List(ids) => {
                        assert_eq!(ids.len(), 1);
                        assert_eq!(as_str(&ids[0]), "c");
                    }
                    _ => panic!("phase should be List"),
                }
            }
            _ => panic!("dag_phases should return List"),
        }
    }

    #[test]
    fn test_dag_phases_cycle_detection() {
        // A -> B -> A (cycle)
        let dag = Value::List(vec![dag_node("a", &["b"]), dag_node("b", &["a"])]);
        let r = builtin_dag_phases(&[dag]);
        assert!(r.is_err(), "dag_phases should detect cycle");
        let err = r.unwrap_err();
        assert!(
            err.contains("cycle"),
            "error should mention 'cycle', got: {}",
            err
        );
    }

    #[test]
    fn test_dag_phases_empty() {
        let r = builtin_dag_phases(&[Value::List(vec![])]).unwrap();
        assert_vals_eq(&r, &Value::List(vec![]), "empty DAG returns empty phases");
    }

    #[test]
    fn test_dag_phases_missing_dep() {
        let dag = Value::List(vec![dag_node("a", &["nonexistent"])]);
        let r = builtin_dag_phases(&[dag]);
        assert!(r.is_err(), "dag_phases should error on unknown dependency");
    }

    #[test]
    fn test_topo_sort_simple() {
        let dag = Value::List(vec![
            dag_node("a", &[]),
            dag_node("b", &["a"]),
            dag_node("c", &["b"]),
        ]);
        let r = builtin_topo_sort(&[dag]).unwrap();
        match &r {
            Value::List(ids) => {
                assert_eq!(ids.len(), 3);
                // a must come before b, b before c
                let names: Vec<&str> = ids.iter().map(|v| as_str(v)).collect();
                let pos_a = names.iter().position(|&n| n == "a").unwrap();
                let pos_b = names.iter().position(|&n| n == "b").unwrap();
                let pos_c = names.iter().position(|&n| n == "c").unwrap();
                assert!(pos_a < pos_b, "a must come before b");
                assert!(pos_b < pos_c, "b must come before c");
            }
            _ => panic!("topo_sort should return List"),
        }
    }

    #[test]
    fn test_topo_sort_parallel() {
        let dag = Value::List(vec![
            dag_node("a", &[]),
            dag_node("b", &[]),
            dag_node("c", &["a", "b"]),
        ]);
        let r = builtin_topo_sort(&[dag]).unwrap();
        match &r {
            Value::List(ids) => {
                assert_eq!(ids.len(), 3);
                let names: Vec<&str> = ids.iter().map(|v| as_str(v)).collect();
                let pos_a = names.iter().position(|&n| n == "a").unwrap();
                let pos_b = names.iter().position(|&n| n == "b").unwrap();
                let pos_c = names.iter().position(|&n| n == "c").unwrap();
                assert!(pos_a < pos_c, "a must come before c");
                assert!(pos_b < pos_c, "b must come before c");
            }
            _ => panic!("topo_sort should return List"),
        }
    }

    #[test]
    fn test_topo_sort_cycle() {
        let dag = Value::List(vec![dag_node("x", &["y"]), dag_node("y", &["x"])]);
        let r = builtin_topo_sort(&[dag]);
        assert!(r.is_err(), "topo_sort should detect cycle");
    }

    #[test]
    fn test_topo_sort_empty() {
        let r = builtin_topo_sort(&[Value::List(vec![])]).unwrap();
        assert_vals_eq(&r, &Value::List(vec![]), "empty DAG returns empty list");
    }

    // ── OpenPlanter-inspired tests (ADR-0063) ──

    #[test]
    fn test_fuzzy_match_identical() {
        let r = builtin_fuzzy_match(&[
            Value::String("hello".to_string()),
            Value::String("hello".to_string()),
        ])
        .unwrap();
        assert_vals_eq(&r, &Value::Float(1.0), "identical strings = 1.0");
    }

    #[test]
    fn test_fuzzy_match_similar() {
        let r = builtin_fuzzy_match(&[
            Value::String("martin".to_string()),
            Value::String("martina".to_string()),
        ])
        .unwrap();
        if let Value::Float(score) = r {
            assert!(
                score > 0.9,
                "martin vs martina should be > 0.9, got {}",
                score
            );
            assert!(score < 1.0, "similar but not identical, got {}", score);
        } else {
            panic!("fuzzy_match should return Float");
        }
    }

    #[test]
    fn test_fuzzy_match_different() {
        let r = builtin_fuzzy_match(&[
            Value::String("abc".to_string()),
            Value::String("xyz".to_string()),
        ])
        .unwrap();
        if let Value::Float(score) = r {
            assert!(score < 0.5, "abc vs xyz should be < 0.5, got {}", score);
        } else {
            panic!("fuzzy_match should return Float");
        }
    }

    #[test]
    fn test_fuzzy_find_best() {
        let r = builtin_fuzzy_find_best(&[
            Value::String("Mikhail".to_string()),
            Value::List(vec![
                Value::String("Michele".to_string()),
                Value::String("Mikael".to_string()),
                Value::String("John".to_string()),
            ]),
        ])
        .unwrap();
        if let Value::Struct { fields, .. } = r {
            let score = match &fields["score"] {
                Value::Float(f) => *f,
                _ => panic!("score should be Float"),
            };
            assert!(
                score > 0.7,
                "best match for Mikhail should score > 0.7, got {}",
                score
            );
            assert_vals_eq(&fields["index"], &Value::Float(1.0), "Mikael at index 1");
        } else {
            panic!("fuzzy_find_best should return Struct");
        }
    }

    #[test]
    fn test_fuzzy_find_best_empty() {
        let r = builtin_fuzzy_find_best(&[Value::String("test".to_string()), Value::List(vec![])])
            .unwrap();
        assert_vals_eq(&r, &Value::Unit, "empty list returns Unit");
    }

    #[test]
    fn test_hashline_read() {
        let r =
            builtin_hashline_read(&[Value::String("hello world\nfoo bar".to_string())]).unwrap();
        if let Value::String(s) = r {
            let lines: Vec<&str> = s.lines().collect();
            assert_eq!(lines.len(), 2);
            // Line 1 should start with "1:" and contain "hello world"
            assert!(
                lines[0].starts_with("1:"),
                "line 1 should start with 1:, got: {}",
                lines[0]
            );
            assert!(
                lines[0].contains("hello world"),
                "should contain original content"
            );
            // Line 2 should start with "2:"
            assert!(lines[1].starts_with("2:"), "line 2 should start with 2:");
            assert!(lines[1].contains("foo bar"));
        } else {
            panic!("hashline_read should return String");
        }
    }

    #[test]
    fn test_hashline_read_empty() {
        let r = builtin_hashline_read(&[Value::String(String::new())]).unwrap();
        assert_vals_eq(
            &r,
            &Value::String(String::new()),
            "empty input = empty output",
        );
    }

    #[test]
    fn test_hashline_edit_set_line() {
        // First get hashline-annotated text
        let annotated =
            builtin_hashline_read(&[Value::String("line one\nline two\nline three".to_string())])
                .unwrap();
        let ann_str = format!("{}", annotated);
        // Extract ref for line 2 (e.g. "2:ab")
        let line2_ref = ann_str.lines().nth(1).unwrap();
        let hash_part: String = line2_ref.chars().take_while(|c| *c != '|').collect();
        // Now edit using that ref
        let r = builtin_hashline_edit(&[
            Value::String("line one\nline two\nline three".to_string()),
            Value::List(vec![Value::Struct {
                type_name: "Edit".to_string(),
                fields: vec![
                    ("op".to_string(), Value::String("set_line".to_string())),
                    (
                        "ref".to_string(),
                        Value::String(hash_part.trim().to_string()),
                    ),
                    (
                        "content".to_string(),
                        Value::String("replaced line".to_string()),
                    ),
                ]
                .into_iter()
                .collect(),
            }]),
        ])
        .unwrap();
        if let Value::String(s) = r {
            let lines: Vec<&str> = s.lines().collect();
            assert_eq!(lines[0], "line one");
            assert_eq!(lines[1], "replaced line");
            assert_eq!(lines[2], "line three");
        } else {
            panic!("hashline_edit should return String");
        }
    }

    #[test]
    fn test_hashline_edit_hash_mismatch() {
        let r = builtin_hashline_edit(&[
            Value::String("original line".to_string()),
            Value::List(vec![Value::Struct {
                type_name: "Edit".to_string(),
                fields: vec![
                    ("op".to_string(), Value::String("set_line".to_string())),
                    ("ref".to_string(), Value::String("1:ff".to_string())),
                    ("content".to_string(), Value::String("new".to_string())),
                ]
                .into_iter()
                .collect(),
            }]),
        ]);
        assert!(r.is_err(), "should error on hash mismatch");
        assert!(
            r.unwrap_err().contains("hash mismatch"),
            "error should mention hash mismatch"
        );
    }

    #[test]
    fn test_compact_list_no_compaction_needed() {
        let items = Value::List(vec![
            Value::String("a".to_string()),
            Value::String("b".to_string()),
        ]);
        let r = builtin_compact_list(&[items, Value::Float(5.0), Value::Float(5.0)]).unwrap();
        // Total (2) <= keep_first(5) + keep_last(5), so no compaction
        if let Value::List(items) = r {
            assert_eq!(items.len(), 2);
        } else {
            panic!("should return List");
        }
    }

    #[test]
    fn test_compact_list_compacts_middle() {
        let items = Value::List(vec![
            Value::Float(1.0),
            Value::Float(2.0),
            Value::Float(3.0),
            Value::Float(4.0),
            Value::Float(5.0),
        ]);
        let r = builtin_compact_list(&[items, Value::Float(1.0), Value::Float(1.0)]).unwrap();
        if let Value::List(list) = r {
            assert_eq!(list.len(), 3, "should have first + compacted + last");
            assert_vals_eq(&list[0], &Value::Float(1.0), "first item preserved");
            assert_vals_eq(&list[2], &Value::Float(5.0), "last item preserved");
            // Middle should be a Compacted struct
            if let Value::Struct { fields, .. } = &list[1] {
                assert_vals_eq(&fields["compacted"], &Value::Bool(true), "compacted flag");
                assert_vals_eq(
                    &fields["removed_count"],
                    &Value::Float(3.0),
                    "removed_count",
                );
            } else {
                panic!("middle item should be Compacted struct");
            }
        } else {
            panic!("should return List");
        }
    }

    #[test]
    fn test_budget_check_ok() {
        let r = builtin_budget_check(&[Value::Float(2.0), Value::Float(10.0)]).unwrap();
        if let Value::Struct { fields, .. } = r {
            assert_vals_eq(
                &fields["level"],
                &Value::String("ok".to_string()),
                "budget ok level",
            );
            assert_vals_eq(
                &fields["remaining"],
                &Value::Float(8.0),
                "budget ok remaining",
            );
        } else {
            panic!("should return struct");
        }
    }

    #[test]
    fn test_budget_check_warning() {
        let r = builtin_budget_check(&[Value::Float(6.0), Value::Float(10.0)]).unwrap();
        if let Value::Struct { fields, .. } = r {
            assert_vals_eq(
                &fields["level"],
                &Value::String("warning".to_string()),
                "budget warning level",
            );
        } else {
            panic!("should return struct");
        }
    }

    #[test]
    fn test_budget_check_critical() {
        let r = builtin_budget_check(&[Value::Float(9.0), Value::Float(10.0)]).unwrap();
        if let Value::Struct { fields, .. } = r {
            assert_vals_eq(
                &fields["level"],
                &Value::String("critical".to_string()),
                "budget critical level",
            );
        } else {
            panic!("should return struct");
        }
    }

    #[test]
    fn test_budget_check_zero_total() {
        let r = builtin_budget_check(&[Value::Float(1.0), Value::Float(0.0)]);
        assert!(r.is_err(), "total_steps=0 should error");
    }

    #[test]
    fn test_replay_snapshot() {
        let r = builtin_replay_snapshot(&[Value::List(vec![
            Value::String("msg1".to_string()),
            Value::String("msg2".to_string()),
        ])])
        .unwrap();
        if let Value::Struct { fields, .. } = r {
            assert_vals_eq(&fields["seq"], &Value::Float(0.0), "replay seq");
            assert_vals_eq(&fields["count"], &Value::Float(2.0), "replay count");
            assert!(fields.contains_key("snapshot"));
        } else {
            panic!("should return struct");
        }
    }

    #[test]
    fn test_policy_check_allowed() {
        let r = builtin_policy_check(&[Value::String("ls -la".to_string())]).unwrap();
        if let Value::Struct { fields, .. } = r {
            assert_vals_eq(&fields["allowed"], &Value::Bool(true), "policy allowed");
        } else {
            panic!("should return struct");
        }
    }

    #[test]
    fn test_policy_check_heredoc() {
        let r = builtin_policy_check(&[Value::String("cat << EOF".to_string())]).unwrap();
        if let Value::Struct { fields, .. } = r {
            assert_vals_eq(
                &fields["allowed"],
                &Value::Bool(false),
                "policy heredoc disallowed",
            );
            assert!(format!("{}", fields["reason"]).contains("heredoc"));
        } else {
            panic!("should return struct");
        }
    }

    #[test]
    fn test_policy_check_interactive() {
        let r = builtin_policy_check(&[Value::String("vim file.txt".to_string())]).unwrap();
        if let Value::Struct { fields, .. } = r {
            assert_vals_eq(
                &fields["allowed"],
                &Value::Bool(false),
                "policy interactive disallowed",
            );
            assert!(format!("{}", fields["reason"]).contains("interactive"));
        } else {
            panic!("should return struct");
        }
    }

    #[test]
    fn test_policy_check_whitespace() {
        // Leading/trailing whitespace should be trimmed
        let r = builtin_policy_check(&[Value::String("   echo hello   ".to_string())]).unwrap();
        if let Value::Struct { fields, .. } = r {
            assert_vals_eq(
                &fields["allowed"],
                &Value::Bool(true),
                "policy whitespace trim allowed",
            );
        } else {
            panic!("should return struct");
        }
    }

    #[test]
    fn test_replace_empty_pattern_returns_original() {
        let r = builtin_replace(&[
            Value::String("hello world".to_string()),
            Value::String("".to_string()),
            Value::String("x".to_string()),
        ])
        .unwrap();
        assert_vals_eq(
            &r,
            &Value::String("hello world".to_string()),
            "empty pattern",
        );
    }

    #[test]
    fn test_replace_normal() {
        let r = builtin_replace(&[
            Value::String("hello world".to_string()),
            Value::String("world".to_string()),
            Value::String("there".to_string()),
        ])
        .unwrap();
        assert_vals_eq(
            &r,
            &Value::String("hello there".to_string()),
            "normal replace",
        );
    }
}
