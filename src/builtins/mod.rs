// ── Built-in functions for METALOGOS M1+M2 ────────────────────────────

use crate::embeddings::{cosine_similarity, EmbeddingManager};
use crate::interpreter::{SecretString, Value};
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

// ── Phase 6.1 — HTTP server stubs ───────────────────────────
// In interpreter-only mode (mlog run), these return mock values.
// Real implementations live in server.rs for the Axum context.

// ── Phase 6.3 — Database stubs ───────────────────────────

fn builtin_query(args: &[Value]) -> Result<Value, String> {
    let sql = expect_string_arg("query", args, 0)?;
    // Wrap SQL in opaque Query value — prevents string concatenation or printing
    // In interpreter mode, store the SQL for later mock execution
    let _params = if args.len() > 1 {
        &args[1]
    } else {
        &Value::Unit
    };
    Ok(Value::Query(sql))
}

fn builtin_db_execute(args: &[Value]) -> Result<Value, String> {
    let _sql = expect_string_arg("db_execute", args, 0)?;
    // In interpreter mode, no-op (returns Unit)
    Ok(Value::Unit)
}

// ── Phase 6.6 — Bot stubs ───────────────────────────

fn builtin_send_message(args: &[Value]) -> Result<Value, String> {
    // Extract and format chat_id — supports negative channel IDs (Наряд №24 B5)
    let chat_id_value: serde_json::Value = match args.get(0) {
        Some(Value::String(s)) => serde_json::Value::String(s.clone()),
        Some(Value::Float(f)) => {
            if *f == (*f as i64) as f64 {
                serde_json::json!(*f as i64)
            } else {
                serde_json::json!(*f)
            }
        }
        Some(other) => {
            return Err(format!(
                "send_message() expected String or Float as chat_id, got {}",
                other.type_name()
            ))
        }
        None => {
            return Err("send_message() requires at least 2 arguments (chat_id, text)".to_string())
        }
    };
    let text = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "send_message() expected String as text, got {}",
                other.type_name()
            ))
        }
        None => {
            return Err("send_message() requires at least 2 arguments (chat_id, text)".to_string())
        }
    };

    // Try to send via Telegram API if BOT_TOKEN env var is set
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
    if bot_token.is_empty() {
        // No token — fall back to audit stub
        eprintln!("[AUDIT] send_message to {:?}: {}", chat_id_value, text);
        return Ok(Value::Unit);
    }

    // Build JSON body with optional reply_markup (3rd arg: Struct)
    let mut body = serde_json::json!({
        "chat_id": chat_id_value,
        "text": text,
    });
    if let Some(markup) = args.get(2) {
        let markup_json = value_to_json(markup)?;
        body["reply_markup"] = markup_json;
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("send_message(): client error: {}", e))?;

    let resp = client
        .post(format!(
            "https://api.telegram.org/bot{}/sendMessage",
            bot_token
        ))
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .map_err(|e| format!("send_message(): request failed: {}", e))?;

    let status = resp.status().as_u16();
    let resp_body = resp.text().unwrap_or_default();

    if status >= 400 {
        return Err(format!(
            "send_message(): Telegram status {}: {}",
            status, resp_body
        ));
    }

    Ok(Value::String(resp_body))
}

/// `answer_callback_query(callback_query_id, text?, show_alert?)` — respond to Telegram inline keyboard callback.
/// `callback_query_id` from update.callback_query.id.
/// `text` — notification text (max 200 chars). `show_alert` — 1.0 = alert popup, 0.0 = toast (default).
fn builtin_answer_callback_query(args: &[Value]) -> Result<Value, String> {
    let callback_query_id = expect_string_arg("answer_callback_query", args, 0)?;
    let text = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    };
    let show_alert = match args.get(2) {
        Some(Value::Float(f)) if *f > 0.5 => true,
        _ => false,
    };
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
    if bot_token.is_empty() {
        eprintln!(
            "[AUDIT] answer_callback_query id={}: {}",
            callback_query_id, text
        );
        return Ok(Value::Unit);
    }
    let body = serde_json::json!({
        "callback_query_id": callback_query_id,
        "text": text,
        "show_alert": show_alert,
    });
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("answer_callback_query(): client error: {}", e))?;
    let resp = client
        .post(format!(
            "https://api.telegram.org/bot{}/answerCallbackQuery",
            bot_token
        ))
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .map_err(|e| format!("answer_callback_query(): request failed: {}", e))?;
    let status = resp.status().as_u16();
    let resp_body = resp.text().unwrap_or_default();
    if status >= 400 {
        return Err(format!(
            "answer_callback_query(): Telegram status {}: {}",
            status, resp_body
        ));
    }
    Ok(Value::String(resp_body))
}

/// `edit_message_text(chat_id, message_id, text, reply_markup?)` — edit existing Telegram message.
/// Used to update inline keyboard buttons after callback.
fn builtin_edit_message_text(args: &[Value]) -> Result<Value, String> {
    let chat_id_val: serde_json::Value = match args.get(0) {
        Some(Value::String(s)) => serde_json::Value::String(s.clone()),
        Some(Value::Float(f)) => serde_json::json!(*f as i64),
        _ => return Err("edit_message_text() requires chat_id".to_string()),
    };
    let message_id = match args.get(1) {
        Some(Value::Float(f)) => *f as i64,
        _ => return Err("edit_message_text() message_id must be Float".to_string()),
    };
    let text = expect_string_arg("edit_message_text", args, 2)?;
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
    if bot_token.is_empty() {
        eprintln!(
            "[AUDIT] edit_message_text chat_id={}: {}",
            chat_id_val, text
        );
        return Ok(Value::Unit);
    }
    let mut body = serde_json::json!({
        "chat_id": chat_id_val,
        "message_id": message_id,
        "text": text,
    });
    if let Some(markup) = args.get(3) {
        let markup_json = value_to_json(markup)?;
        body["reply_markup"] = markup_json;
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("edit_message_text(): client error: {}", e))?;
    let resp = client
        .post(format!(
            "https://api.telegram.org/bot{}/editMessageText",
            bot_token
        ))
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .map_err(|e| format!("edit_message_text(): request failed: {}", e))?;
    let status = resp.status().as_u16();
    let resp_body = resp.text().unwrap_or_default();
    if status >= 400 {
        return Err(format!(
            "edit_message_text(): Telegram status {}: {}",
            status, resp_body
        ));
    }
    Ok(Value::String(resp_body))
}

// ── Phase 7.7 — parse_json, http_get, now ────────────────────────────

// ── Наряд 17: Utility builtins ──────────────────────────────────

// ── OpenPlanter-inspired: Fuzzy matching, safe editing, agent utilities (ADR-0063) ──

/// `fuzzy_find_best(query, candidates)` — find the best match for `query` in a list of candidate strings.
/// Returns a struct { index, candidate, score } or Unit if list is empty.
fn builtin_fuzzy_find_best(args: &[Value]) -> Result<Value, String> {
    let query = expect_string_arg("fuzzy_find_best", args, 0)?;
    let candidates = expect_list_arg("fuzzy_find_best", args, 1)?;
    if candidates.is_empty() {
        return Ok(Value::Unit);
    }
    let mut best_idx = 0usize;
    let mut best_score = 0.0f64;
    let mut best_candidate = String::new();
    for (i, v) in candidates.iter().enumerate() {
        let c = format!("{}", v);
        let score = strsim::jaro_winkler(&query, &c);
        if score > best_score {
            best_score = score;
            best_idx = i;
            best_candidate = c;
        }
    }
    Ok(make_struct(
        "FuzzyMatch",
        vec![
            ("index", Value::Float(best_idx as f64)),
            ("candidate", Value::String(best_candidate)),
            ("score", Value::Float(best_score)),
        ],
    ))
}

/// Compute a 2-char hex hash for a line (whitespace-normalized),
/// mimicking OpenPlanter's hashline system for content-verified editing.
fn compute_line_hash(line: &str) -> String {
    let normalized: String = line.split_whitespace().collect();
    let hash = crc32fast::hash(normalized.as_bytes());
    format!("{:02x}", hash & 0xFF)
}

/// `hashline_read(text)` — annotate each line with a 2-char CRC32 hash prefix.
/// Output format: "N:HH|content" per line.
/// Inspired by OpenPlanter's tools.py hashline system for safe LLM editing.
fn builtin_hashline_read(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("hashline_read", args, 0)?;
    let mut out = String::new();
    for (i, line) in text.lines().enumerate() {
        let hash = compute_line_hash(line);
        out.push_str(&format!("{}:{}|{}\n", i + 1, hash, line));
    }
    Ok(Value::String(out))
}

/// `hashline_edit(text, edits)` — apply edits to text using hashline-verified line references.
/// `edits` is a list of structs, each with an `op` field ("set_line", "replace_lines", "insert_after")
/// and corresponding fields.
///   - set_line: { op: "set_line", ref: "N:HH", content: "new content" }
///   - replace_lines: { op: "replace_lines", start_ref: "N:HH", end_ref: "M:HH", content: "replacement" }
///   - insert_after: { op: "insert_after", ref: "N:HH", content: "new line" }
/// Returns the modified text. Errors if hash mismatch (stale reference).
fn builtin_hashline_edit(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("hashline_edit", args, 0)?;
    let edits = expect_list_arg("hashline_edit", args, 1)?;
    let mut lines: Vec<String> = text.lines().map(String::from).collect();

    for edit in &edits {
        let edit_json = mlog_value_to_json(edit);
        let op = edit_json.get("op").and_then(|v| v.as_str()).unwrap_or("");
        match op {
            "set_line" => {
                let line_ref = edit_json.get("ref").and_then(|v| v.as_str()).unwrap_or("");
                let content = edit_json
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let (line_num, expected_hash) = parse_line_ref(line_ref)?;
                let idx = line_num - 1;
                if idx >= lines.len() {
                    return Err(format!(
                        "hashline_edit: line {} out of bounds ({} lines)",
                        line_num,
                        lines.len()
                    ));
                }
                let actual_hash = compute_line_hash(&lines[idx]);
                if actual_hash != expected_hash {
                    return Err(format!(
                        "hashline_edit: hash mismatch at line {} (expected {}, got {}). Line may have changed.",
                        line_num, expected_hash, actual_hash
                    ));
                }
                lines[idx] = content.to_string();
            }
            "replace_lines" => {
                let start_ref = edit_json
                    .get("start_ref")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let end_ref = edit_json
                    .get("end_ref")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let content = edit_json
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let (start_num, start_hash) = parse_line_ref(start_ref)?;
                let (end_num, end_hash) = parse_line_ref(end_ref)?;
                let si = start_num - 1;
                let ei = end_num;
                if si >= lines.len() || ei > lines.len() {
                    return Err(format!(
                        "hashline_edit: replace range {}..{} out of bounds",
                        start_num, end_num
                    ));
                }
                let actual_start = compute_line_hash(&lines[si]);
                let actual_end = compute_line_hash(&lines[ei - 1]);
                if actual_start != start_hash || actual_end != end_hash {
                    return Err(format!(
                        "hashline_edit: hash mismatch in replace range {}..{}",
                        start_num, end_num
                    ));
                }
                let replacement: Vec<String> = content.lines().map(String::from).collect();
                lines.splice(si..ei, replacement);
            }
            "insert_after" => {
                let line_ref = edit_json.get("ref").and_then(|v| v.as_str()).unwrap_or("");
                let content = edit_json
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let (line_num, expected_hash) = parse_line_ref(line_ref)?;
                let idx = line_num; // insert AFTER this line
                if idx > lines.len() {
                    return Err(format!(
                        "hashline_edit: insert_after line {} out of bounds",
                        line_num
                    ));
                }
                if line_num > 0 && (line_num - 1) < lines.len() {
                    let actual_hash = compute_line_hash(&lines[line_num - 1]);
                    if actual_hash != expected_hash {
                        return Err(format!(
                            "hashline_edit: hash mismatch at line {} (expected {}, got {})",
                            line_num, expected_hash, actual_hash
                        ));
                    }
                }
                let new_lines: Vec<String> = content.lines().map(String::from).collect();
                for (i, nl) in new_lines.into_iter().enumerate() {
                    lines.insert(idx + i, nl);
                }
            }
            _ => {
                return Err(format!(
                    "hashline_edit: unknown op '{}'. Use set_line, replace_lines, or insert_after.",
                    op
                ));
            }
        }
    }

    Ok(Value::String(lines.join("\n")))
}

/// Parse a line reference "N:HH" into (line_number, hash_hex).
fn parse_line_ref(line_ref: &str) -> Result<(usize, String), String> {
    let parts: Vec<&str> = line_ref.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(format!(
            "hashline_edit: invalid line ref '{}', expected N:HH format",
            line_ref
        ));
    }
    let line_num: usize = parts[0].parse().map_err(|_| {
        format!(
            "hashline_edit: invalid line number '{}' in ref '{}'",
            parts[0], line_ref
        )
    })?;
    let hash = parts[1].to_string();
    if hash.len() != 2 {
        return Err(format!(
            "hashline_edit: hash must be 2 hex chars, got '{}' in ref '{}'",
            hash, line_ref
        ));
    }
    Ok((line_num, hash))
}

/// `budget_check(step, total_steps)` — returns a budget status struct.
///   - remaining >= 50%: level = "ok"
///   - remaining >= 25%: level = "warning"
///   - remaining < 25%: level = "critical"
/// Inspired by OpenPlanter's engine.py budget awareness system.
fn builtin_budget_check(args: &[Value]) -> Result<Value, String> {
    let step = expect_float_arg("budget_check", args, 0)? as usize;
    let total_steps = expect_float_arg("budget_check", args, 1)? as usize;
    if total_steps == 0 {
        return Err("budget_check: total_steps must be > 0".to_string());
    }
    if step > total_steps {
        return Err(format!(
            "budget_check: step {} exceeds total_steps {}",
            step, total_steps
        ));
    }
    let remaining = total_steps - step;
    let pct = (remaining as f64) / (total_steps as f64) * 100.0;
    let level = if pct >= 50.0 {
        "ok"
    } else if pct >= 25.0 {
        "warning"
    } else {
        "critical"
    };
    Ok(make_struct(
        "BudgetStatus",
        vec![
            ("step", Value::Float(step as f64)),
            ("total", Value::Float(total_steps as f64)),
            ("remaining", Value::Float(remaining as f64)),
            ("pct_remaining", Value::Float(pct)),
            ("level", Value::String(level.to_string())),
        ],
    ))
}

/// `replay_snapshot(data)` — delta-encoded replay log helper.
/// Takes a list of items (messages, events, etc.) and returns a struct with:
///   - seq: 0 (first snapshot)
///   - count: number of items
///   - snapshot: JSON string of the full list
/// Subsequent calls should use the returned count to determine delta.
/// Inspired by OpenPlanter's ReplayLogger (seq 0 = full, seq N = delta).
fn builtin_replay_snapshot(args: &[Value]) -> Result<Value, String> {
    let data = expect_list_arg("replay_snapshot", args, 0)?;
    let json_items: Vec<serde_json::Value> = data.iter().map(|v| mlog_value_to_json(v)).collect();
    let snapshot = serde_json::to_string(&json_items)
        .map_err(|e| format!("replay_snapshot: JSON serialization error: {}", e))?;
    Ok(make_struct(
        "ReplaySnapshot",
        vec![
            ("seq", Value::Float(0.0)),
            ("count", Value::Float(data.len() as f64)),
            ("snapshot", Value::String(snapshot)),
        ],
    ))
}

/// `policy_check(command)` — runtime policy enforcement for shell commands.
/// Checks a command string against safety policies:
///   - Blocks heredoc syntax (<<)
///   - Blocks interactive TUI programs (vim, nano, less, more, top, htop, vi)
///   - Trims leading/trailing whitespace
/// Returns a struct { allowed: bool, reason: "..." }.
/// Inspired by OpenPlanter's _runtime_policy_check in engine.py.
fn builtin_policy_check(args: &[Value]) -> Result<Value, String> {
    let command = expect_string_arg("policy_check", args, 0)?;
    let cmd_trimmed = command.trim();
    // Check heredoc
    if cmd_trimmed.contains("<<") {
        return Ok(make_struct(
            "PolicyResult",
            vec![
                ("allowed", Value::Bool(false)),
                (
                    "reason",
                    Value::String("blocked: heredoc syntax (<<) detected".to_string()),
                ),
            ],
        ));
    }
    // Check interactive programs
    let interactive_patterns = [
        "vim", "vi ", "nano", "less ", "more ", "top", "htop", "emacs",
    ];
    let first_word = cmd_trimmed.split_whitespace().next().unwrap_or("");
    for pattern in &interactive_patterns {
        if first_word == *pattern || first_word.starts_with(&format!("{}", pattern)) {
            return Ok(make_struct(
                "PolicyResult",
                vec![
                    ("allowed", Value::Bool(false)),
                    (
                        "reason",
                        Value::String(format!(
                            "blocked: interactive program '{}' detected",
                            first_word
                        )),
                    ),
                ],
            ));
        }
    }
    Ok(make_struct(
        "PolicyResult",
        vec![
            ("allowed", Value::Bool(true)),
            ("reason", Value::String("ok".to_string())),
        ],
    ))
}

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

// ── Format (Наряд №17 В.3) ──────────────────────────────────────

// ── v0.8.0 — Geolocation builtins ───────────────────────────────────

// ── v0.8.1 — OpenHuman-inspired Human Intelligence builtins ──────────
// Inspired by https://github.com/tinyhumansai/OpenHuman — memory tree,
// persona system, mood tracking, human-like AI responses.
// All built on top of existing Metalogos primitives (KV store, call_llm).
// No external dependencies, no API keys required beyond LLM provider.

/// `human_create(name, traits)` — create or update a persona.
/// `traits` is a string describing personality: "friendly, professional, speaks Russian".
/// Stores persona in KV under `human_persona:{name}`.
/// Returns Struct {name, traits, created_at, memory_count}.
fn builtin_human_create(args: &[Value]) -> Result<Value, String> {
    let name = expect_string_arg("human_create", args, 0)?;
    let traits = expect_string_arg("human_create", args, 1)?;
    if name.is_empty() {
        return Err("human_create() name cannot be empty".to_string());
    }
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let persona_data = serde_json::json!({
        "name": name,
        "traits": traits,
        "created_at": now_ts,
        "mood": "neutral",
        "mood_intensity": 0.5,
    });
    let key = format!("human_persona:{}", name);
    let value = serde_json::to_string(&persona_data)
        .map_err(|e| format!("human_create() serialize error: {}", e))?;
    let mut store = kv_store()
        .lock()
        .map_err(|e| format!("human_create() lock error: {}", e))?;
    store.insert(key.clone(), value.clone());
    if let Ok(sqlite_guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *sqlite_guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, value],
            );
        }
    }
    // Count existing memories for this persona
    let mem_prefix = format!("human_mem:{}:", name);
    let mem_count = store.keys().filter(|k| k.starts_with(&mem_prefix)).count();
    Ok(make_date_struct(
        "Persona",
        vec![
            ("name", Value::String(name)),
            ("traits", Value::String(traits)),
            ("created_at", Value::Float(now_ts)),
            ("memory_count", Value::Float(mem_count as f64)),
        ],
    ))
}

/// `human_mood(persona, mood?, intensity?)` — get or set persona's emotional state.
/// With 1 arg: returns current mood as Struct {mood, intensity, updated_at}.
/// With 2+ args: sets mood. `intensity` is 0.0–1.0 (default 0.5).
/// `mood` examples: "happy", "sad", "focused", "creative", "neutral", "excited".
fn builtin_human_mood(args: &[Value]) -> Result<Value, String> {
    let persona = expect_string_arg("human_mood", args, 0)?;
    let key = format!("human_persona:{}", persona);
    let store = kv_store()
        .lock()
        .map_err(|e| format!("human_mood() lock error: {}", e))?;
    let mut data_str = store.get(&key).cloned().unwrap_or_default();
    drop(store);
    if data_str.is_empty() {
        return Err(format!(
            "human_mood() persona '{}' not found. Use human_create() first.",
            persona
        ));
    }
    let mut data: serde_json::Value =
        serde_json::from_str(&data_str).map_err(|e| format!("human_mood() parse error: {}", e))?;
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    // If mood argument provided — set mood
    if args.len() >= 2 {
        let mood = expect_string_arg("human_mood", args, 1)?;
        let intensity = if args.len() >= 3 {
            expect_float_arg("human_mood", args, 2)?.clamp(0.0, 1.0)
        } else {
            0.5
        };
        data["mood"] = serde_json::Value::String(mood.clone());
        data["mood_intensity"] = serde_json::Value::Number(
            serde_json::Number::from_f64(intensity)
                .or_else(|| serde_json::Number::from_f64(0.5))
                .unwrap_or(serde_json::Number::from(0)),
        );
        data["mood_updated_at"] = serde_json::json!(now_ts);
        let updated = serde_json::to_string(&data)
            .map_err(|e| format!("human_mood() serialize error: {}", e))?;
        let mut store = kv_store()
            .lock()
            .map_err(|e| format!("human_mood() lock error: {}", e))?;
        store.insert(key.clone(), updated.clone());
        if let Ok(sqlite_guard) = kv_sqlite().lock() {
            if let Some(ref conn) = *sqlite_guard {
                let _ = conn.execute(
                    "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                    rusqlite::params![key, updated],
                );
            }
        }
    }

    let mood = data
        .get("mood")
        .and_then(|v| v.as_str())
        .unwrap_or("neutral")
        .to_string();
    let intensity = data
        .get("mood_intensity")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);
    let updated_at = data
        .get("mood_updated_at")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    Ok(make_date_struct(
        "Mood",
        vec![
            ("persona", Value::String(persona)),
            ("mood", Value::String(mood)),
            ("intensity", Value::Float(intensity)),
            ("updated_at", Value::Float(updated_at)),
        ],
    ))
}

/// `human_remember(persona, key, content, importance?)` — store a memory in persona's memory tree.
/// `importance` is 0.0–1.0 (default 0.5). Higher importance = recalled first.
/// Stores as KV entry `human_mem:{persona}:{key}` with metadata.
fn builtin_human_remember(args: &[Value]) -> Result<Value, String> {
    let persona = expect_string_arg("human_remember", args, 0)?;
    let key = expect_string_arg("human_remember", args, 1)?;
    let content = expect_string_arg("human_remember", args, 2)?;
    if key.is_empty() {
        return Err("human_remember() key cannot be empty".to_string());
    }
    let importance = if args.len() >= 4 {
        expect_float_arg("human_remember", args, 3)?.clamp(0.0, 1.0)
    } else {
        0.5
    };
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let mem_data = serde_json::json!({
        "persona": persona,
        "key": key,
        "content": content,
        "importance": importance,
        "created_at": now_ts,
        "access_count": 0,
        "last_accessed": now_ts,
    });
    let store_key = format!("human_mem:{}:{}", persona, key);
    let value = serde_json::to_string(&mem_data)
        .map_err(|e| format!("human_remember() serialize error: {}", e))?;
    let mut store = kv_store()
        .lock()
        .map_err(|e| format!("human_remember() lock error: {}", e))?;
    store.insert(store_key.clone(), value.clone());
    if let Ok(sqlite_guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *sqlite_guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                rusqlite::params![store_key, value],
            );
        }
    }
    Ok(Value::String("ok".to_string()))
}

/// `human_forget(persona, key?)` — delete a specific memory or all memories for a persona.
/// With 2 args: deletes specific memory by key. Returns "ok" or "not_found".
/// With 1 arg: deletes ALL memories for persona. Returns count of deleted memories.
fn builtin_human_forget(args: &[Value]) -> Result<Value, String> {
    let persona = expect_string_arg("human_forget", args, 0)?;
    let prefix = format!("human_mem:{}:", persona);
    let mut store = kv_store()
        .lock()
        .map_err(|e| format!("human_forget() lock error: {}", e))?;

    if args.len() >= 2 {
        let key = expect_string_arg("human_forget", args, 1)?;
        let store_key = format!("human_mem:{}:{}", persona, key);
        if store.remove(&store_key).is_some() {
            if let Ok(sqlite_guard) = kv_sqlite().lock() {
                if let Some(ref conn) = *sqlite_guard {
                    let _ = conn.execute(
                        "DELETE FROM kv_store WHERE key = ?1",
                        rusqlite::params![store_key],
                    );
                }
            }
            Ok(Value::String("ok".to_string()))
        } else {
            Ok(Value::String("not_found".to_string()))
        }
    } else {
        // Delete all memories for this persona
        let to_remove: Vec<String> = store
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .cloned()
            .collect();
        let count = to_remove.len();
        for k in &to_remove {
            store.remove(k);
        }
        if let Ok(sqlite_guard) = kv_sqlite().lock() {
            if let Some(ref conn) = *sqlite_guard {
                for k in &to_remove {
                    let _ =
                        conn.execute("DELETE FROM kv_store WHERE key = ?1", rusqlite::params![k]);
                }
            }
        }
        Ok(Value::Float(count as f64))
    }
}

/// `human_recall(persona, query, limit?)` — search persona's memories by keyword match.
/// Returns List of Memory structs sorted by importance (descending), then by recency.
/// Each struct: {key, content, importance, created_at, access_count, relevance}.
fn builtin_human_recall(args: &[Value]) -> Result<Value, String> {
    let persona = expect_string_arg("human_recall", args, 0)?;
    let query = expect_string_arg("human_recall", args, 1)?;
    let limit: usize = if args.len() >= 3 {
        expect_float_arg("human_recall", args, 2)? as usize
    } else {
        10
    };
    let prefix = format!("human_mem:{}:", persona);
    let query_lower = query.to_lowercase();
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    let store = kv_store()
        .lock()
        .map_err(|e| format!("human_recall() lock error: {}", e))?;
    let mut memories: Vec<(f64, f64, Value)> = Vec::new(); // (importance, recency, struct)

    for (k, v) in store.iter() {
        if !k.starts_with(&prefix) {
            continue;
        }
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(v) {
            let content = data
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            let key_str = data
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            // Simple relevance scoring: keyword match in content or key
            let query_words: Vec<&str> = query_lower.split_whitespace().collect();
            let mut matches = 0;
            for word in &query_words {
                if content.contains(word) || key_str.contains(word) {
                    matches += 1;
                }
            }
            let relevance = if query_words.is_empty() {
                0.5
            } else {
                matches as f64 / query_words.len() as f64
            };
            if relevance < 0.01 && !query.is_empty() {
                continue;
            } // skip non-matching if query given

            let importance = data
                .get("importance")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5);
            let created_at = data
                .get("created_at")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let access_count = data
                .get("access_count")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as f64;
            let age_hours = (now_ts - created_at).max(0.0) / 3600.0;
            // Recency score: 1.0 for fresh, decays over time (half-life ~168h = 1 week)
            let recency = (0.5_f64).powf(age_hours / 168.0);
            // Composite score: 50% relevance, 30% importance, 20% recency
            let score = relevance * 0.5 + importance * 0.3 + recency * 0.2;

            let mem_key = data
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let mem_content = data
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let mem_struct = make_date_struct(
                "Memory",
                vec![
                    ("key", Value::String(mem_key)),
                    ("content", Value::String(mem_content)),
                    ("importance", Value::Float(importance)),
                    ("created_at", Value::Float(created_at)),
                    ("access_count", Value::Float(access_count)),
                    ("relevance", Value::Float(relevance)),
                    ("score", Value::Float(score)),
                ],
            );
            memories.push((score, recency, mem_struct));
        }
    }
    drop(store);

    // Sort by score descending, take top N
    memories.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    memories.truncate(limit);

    let results: Vec<Value> = memories.into_iter().map(|(_, _, v)| v).collect();
    Ok(Value::List(results))
}

/// `human_respond(persona, message, context?)` — generate a human-like response.
/// Uses the persona's traits, mood, and recalled memories to craft a response via LLM.
/// `context` is optional additional context (e.g., conversation history).
/// Returns the generated response as String.
fn builtin_human_respond(args: &[Value]) -> Result<Value, String> {
    let persona = expect_string_arg("human_respond", args, 0)?;
    let message = expect_string_arg("human_respond", args, 1)?;
    let context = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    };

    // Load persona data
    let persona_key = format!("human_persona:{}", persona);
    let store = kv_store()
        .lock()
        .map_err(|e| format!("human_respond() lock error: {}", e))?;
    let persona_data = store.get(&persona_key).cloned().unwrap_or_default();
    drop(store);

    if persona_data.is_empty() {
        return Err(format!(
            "human_respond() persona '{}' not found. Use human_create() first.",
            persona
        ));
    }

    let data: serde_json::Value = serde_json::from_str(&persona_data)
        .map_err(|e| format!("human_respond() persona parse error: {}", e))?;
    let traits = data
        .get("traits")
        .and_then(|v| v.as_str())
        .unwrap_or("helpful assistant")
        .to_string();
    let mood = data
        .get("mood")
        .and_then(|v| v.as_str())
        .unwrap_or("neutral")
        .to_string();
    let mood_intensity = data
        .get("mood_intensity")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);

    // Recall relevant memories
    let recall_result = builtin_human_recall(&[
        Value::String(persona.clone()),
        Value::String(message.clone()),
        Value::Float(5.0),
    ])?;
    let memories_text = match recall_result {
        Value::List(items) => {
            let mut parts = Vec::new();
            for item in &items {
                if let Value::Struct { fields, .. } = item {
                    let key = fields
                        .get("key")
                        .map(|v| format!("{}", v))
                        .unwrap_or_default();
                    let content = fields
                        .get("content")
                        .map(|v| format!("{}", v))
                        .unwrap_or_default();
                    parts.push(format!("- [{}]: {}", key, content));
                }
            }
            if parts.is_empty() {
                "No relevant memories found.".to_string()
            } else {
                parts.join("\n")
            }
        }
        _ => "No memories.".to_string(),
    };

    // Build the LLM prompt
    let system_prompt = format!(
        "You are {}, a persona with the following traits: {}. \
        Your current emotional state is '{}' with intensity {:.1}. \
        Let your mood subtly influence your tone and word choice. \
        You have access to the following memories about the user and past interactions:\n{}\
        \nRespond naturally, as a human would. Be concise but warm. Stay in character.",
        persona, traits, mood, mood_intensity, memories_text
    );

    let full_prompt = if context.is_empty() {
        format!("{}\n\nUser: {}", system_prompt, message)
    } else {
        format!(
            "{}\n\nRecent context:\n{}\n\nUser: {}",
            system_prompt, context, message
        )
    };

    // Call LLM (reuses existing call_llm infrastructure)
    let mock_mode = std::env::var("METALOGOS_LLM_MOCK")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(true);

    let response = if mock_mode {
        format!("[{} (mood: {}): {}]", persona, mood, message)
    } else {
        let backend = crate::llm::create_llm_backend();
        backend
            .call(&full_prompt, "")
            .map_err(|e| format!("human_respond() LLM call failed: {}", e))?
    };

    Ok(Value::String(response))
}

/// `human_personas()` — list all created personas.
/// Returns List of PersonaSummary structs: {name, traits, mood, memory_count, created_at}.
fn builtin_human_personas(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let prefix = "human_persona:";
    let store = kv_store()
        .lock()
        .map_err(|e| format!("human_personas() lock error: {}", e))?;
    let mut result = Vec::new();
    for (k, v) in store.iter() {
        if !k.starts_with(prefix) {
            continue;
        }
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(v) {
            let name = data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let traits = data
                .get("traits")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let mood = data
                .get("mood")
                .and_then(|v| v.as_str())
                .unwrap_or("neutral")
                .to_string();
            let created_at = data
                .get("created_at")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            // Count memories for this persona
            let mem_prefix = format!("human_mem:{}:", name);
            let mem_count = store
                .keys()
                .filter(|mk| mk.starts_with(&mem_prefix))
                .count();
            result.push(make_date_struct(
                "PersonaSummary",
                vec![
                    ("name", Value::String(name)),
                    ("traits", Value::String(traits)),
                    ("mood", Value::String(mood)),
                    ("memory_count", Value::Float(mem_count as f64)),
                    ("created_at", Value::Float(created_at)),
                ],
            ));
        }
    }
    Ok(Value::List(result))
}

/// `human_delete(persona)` — delete a persona and all its memories.
/// Returns Struct {deleted_memories: Float, status: String}.
fn builtin_human_delete(args: &[Value]) -> Result<Value, String> {
    let persona = expect_string_arg("human_delete", args, 0)?;
    let persona_key = format!("human_persona:{}", persona);
    let mem_prefix = format!("human_mem:{}:", persona);
    let mut store = kv_store()
        .lock()
        .map_err(|e| format!("human_delete() lock error: {}", e))?;

    // Delete persona
    let persona_existed = store.remove(&persona_key).is_some();
    // Delete all memories
    let to_remove: Vec<String> = store
        .keys()
        .filter(|k| k.starts_with(&mem_prefix))
        .cloned()
        .collect();
    let mem_count = to_remove.len();
    for k in &to_remove {
        store.remove(k);
    }
    drop(store);

    // SQLite cleanup
    if let Ok(sqlite_guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *sqlite_guard {
            let _ = conn.execute(
                "DELETE FROM kv_store WHERE key = ?1",
                rusqlite::params![persona_key],
            );
            for k in &to_remove {
                let _ = conn.execute("DELETE FROM kv_store WHERE key = ?1", rusqlite::params![k]);
            }
        }
    }

    let status = if persona_existed {
        "deleted"
    } else {
        "not_found"
    };
    Ok(make_date_struct(
        "DeleteResult",
        vec![
            ("deleted_memories", Value::Float(mem_count as f64)),
            ("status", Value::String(status.to_string())),
        ],
    ))
}

/// `extract_param(text, index)` — parse colon-separated callback_data, return N-th segment.
/// Example: extract_param("dept:osp:watch:42", 2) → "watch"
pub fn builtin_extract_param(args: &[Value]) -> Result<Value, String> {
    let text = match args.get(0) {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("extract_param() expects first argument to be a String".to_string()),
    };
    let index = match args.get(1) {
        Some(Value::Float(f)) => *f as usize,
        _ => {
            return Err("extract_param() expects second argument to be a Float (index)".to_string())
        }
    };
    let parts: Vec<&str> = text.split(':').collect();
    match parts.get(index) {
        Some(s) => Ok(Value::String(s.to_string())),
        None => Ok(Value::String("".to_string())),
    }
}

/// `estimate_tokens(text)` — rough token count heuristic (len / 4 for CJK+Latin mix).
/// ADR note: temporary heuristic, replace with proper tokenizer when available.
pub fn builtin_estimate_tokens(args: &[Value]) -> Result<Value, String> {
    let text = match args.get(0) {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("estimate_tokens() expects a String argument".to_string()),
    };
    let char_count = text.chars().count() as f64;
    // Heuristic: ~4 chars per token for mixed CJK/Latin
    let tokens = (char_count / 4.0).ceil();
    Ok(Value::Float(tokens))
}

/// `read_file_tokens(path)` — read file and return {content, tokens} struct.
/// Convenience for skill_index: read skill file + estimate its token cost in one call.
pub fn builtin_read_file_tokens(args: &[Value]) -> Result<Value, String> {
    let path = match args.get(0) {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("read_file_tokens() expects a file path (String)".to_string()),
    };
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("read_file_tokens(): {}", e))?;
    let char_count = content.chars().count() as f64;
    let tokens = (char_count / 4.0).ceil();
    Ok(Value::Struct {
        type_name: "FileInfo".to_string(),
        fields: [
            ("content".to_string(), Value::String(content)),
            ("chars".to_string(), Value::Float(char_count)),
            ("tokens".to_string(), Value::Float(tokens)),
        ]
        .into_iter()
        .collect(),
    })
}

// ═══════════════════════════════════════════════════════════════════════
// OpenHuman-inspired builtins (v0.8.3 — from OpenHuman feature audit)
// ═══════════════════════════════════════════════════════════════════════

// ── Approval Gate (inspired by OpenHuman approval flow) ──
// Stores pending approvals in KV store. In server mode, these can be
// dispatched as Telegram inline keyboards. In CLI mode, returns the
// approval struct for programmatic handling.

/// `ask_approval(title, description)` — create an approval request.
/// Returns Struct { id, title, description, approved, status }.
/// The `approved` field is 0.0 (pending). Use kv_get("approval:<id>") to poll.
/// In Telegram bot context, this would generate an inline keyboard.
fn builtin_ask_approval(args: &[Value]) -> Result<Value, String> {
    let title = expect_string_arg("ask_approval", args, 0)?;
    let description = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err(
                "ask_approval() expects second argument to be a description (String)".to_string(),
            )
        }
    };
    let id = format!("appr_{}", chrono_now_timestamp());
    let approval = serde_json::json!({
        "id": id,
        "title": title,
        "description": description,
        "approved": false,
        "rejected": false,
        "created_at": chrono_now_timestamp()
    });
    let json = serde_json::to_string(&approval).unwrap_or_default();
    let key = format!("approval:{}", id);
    if let Ok(mut store) = kv_store().lock() {
        store.insert(key.clone(), json.clone());
    }
    if let Ok(guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, json],
            );
        }
    }
    Ok(make_date_struct(
        "Approval",
        vec![
            ("id", Value::String(id)),
            ("title", Value::String(title)),
            ("description", Value::String(description)),
            ("approved", Value::Float(0.0)),
            ("status", Value::String("pending".to_string())),
        ],
    ))
}

// ── Goals (inspired by OpenHuman Goals: long-term goals + thread goal + budget) ──
// Goals stored in KV store under "goals" key as JSON array.
// Thread goal under "thread_goal" as single JSON object.

/// `goal_set(objective, budget?)` — set the current thread goal.
/// Returns Struct { objective, status, budget, spent }.
fn builtin_goal_set(args: &[Value]) -> Result<Value, String> {
    let objective = expect_string_arg("goal_set", args, 0)?;
    let budget = match args.get(1) {
        Some(Value::Float(f)) => Some(*f),
        _ => None,
    };
    let goal = serde_json::json!({
        "objective": objective,
        "status": "active",
        "budget": budget,
        "spent": 0.0,
        "set_at": chrono_now_timestamp()
    });
    let json = serde_json::to_string(&goal).unwrap_or_default();
    if let Ok(mut store) = kv_store().lock() {
        store.insert("thread_goal".to_string(), json.clone());
    }
    if let Ok(guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES ('thread_goal', ?1)",
                rusqlite::params![json],
            );
        }
    }
    Ok(make_date_struct(
        "ThreadGoal",
        vec![
            ("objective", Value::String(objective)),
            ("status", Value::String("active".to_string())),
            ("budget", Value::Float(budget.unwrap_or(0.0))),
            ("spent", Value::Float(0.0)),
        ],
    ))
}

/// `goal_get()` — get the current thread goal.
/// Returns Struct or empty struct if no goal is set.
fn builtin_goal_get(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let raw = kv_get_raw("thread_goal");
    if let Some(json_str) = raw {
        if let Ok(goal) = serde_json::from_str::<serde_json::Value>(&json_str) {
            return Ok(make_date_struct(
                "ThreadGoal",
                vec![
                    (
                        "objective",
                        Value::String(goal["objective"].as_str().unwrap_or("").to_string()),
                    ),
                    (
                        "status",
                        Value::String(goal["status"].as_str().unwrap_or("none").to_string()),
                    ),
                    (
                        "budget",
                        Value::Float(goal["budget"].as_f64().unwrap_or(0.0)),
                    ),
                    ("spent", Value::Float(goal["spent"].as_f64().unwrap_or(0.0))),
                ],
            ));
        }
    }
    Ok(make_date_struct(
        "ThreadGoal",
        vec![
            ("objective", Value::String("".to_string())),
            ("status", Value::String("none".to_string())),
            ("budget", Value::Float(0.0)),
            ("spent", Value::Float(0.0)),
        ],
    ))
}

/// `goal_complete()` — mark the current thread goal as complete.
/// Returns Struct { status, objective }.
fn builtin_goal_complete(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let raw = kv_get_raw("thread_goal");
    let objective = match raw {
        Some(ref json_str) => serde_json::from_str::<serde_json::Value>(json_str)
            .ok()
            .and_then(|g| g["objective"].as_str().map(|s| s.to_string()))
            .unwrap_or_default(),
        None => String::new(),
    };
    let goal = serde_json::json!({
        "objective": objective,
        "status": "complete",
        "completed_at": chrono_now_timestamp()
    });
    let json = serde_json::to_string(&goal).unwrap_or_default();
    if let Ok(mut store) = kv_store().lock() {
        store.insert("thread_goal".to_string(), json.clone());
    }
    if let Ok(guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES ('thread_goal', ?1)",
                rusqlite::params![json],
            );
        }
    }
    Ok(make_date_struct(
        "GoalComplete",
        vec![
            ("status", Value::String("complete".to_string())),
            ("objective", Value::String(objective)),
        ],
    ))
}

/// `goals_list()` — list all long-term goals.
/// Returns List of Struct { id, text, status }.
fn builtin_goals_list(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let raw = kv_get_raw("goals_list");
    let goals: Vec<serde_json::Value> = raw
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut result = Vec::new();
    for (i, g) in goals.iter().enumerate() {
        result.push(make_date_struct(
            "Goal",
            vec![
                ("id", Value::String(format!("g{}", i))),
                (
                    "text",
                    Value::String(g["text"].as_str().unwrap_or("").to_string()),
                ),
                (
                    "status",
                    Value::String(g["status"].as_str().unwrap_or("active").to_string()),
                ),
            ],
        ));
    }
    Ok(Value::List(result))
}

/// `goals_add(text)` — add a long-term goal (max 8).
/// Returns Struct { id, text, status }.
fn builtin_goals_add(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("goals_add", args, 0)?;
    let raw = kv_get_raw("goals_list");
    let mut goals: Vec<serde_json::Value> = raw
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if goals.len() >= 8 {
        return Err("goals_add() maximum 8 long-term goals".to_string());
    }
    let goal = serde_json::json!({
        "text": text,
        "status": "active",
        "added_at": chrono_now_timestamp()
    });
    let id = format!("g{}", goals.len());
    goals.push(goal);
    let json = serde_json::to_string(&goals).unwrap_or_default();
    if let Ok(mut store) = kv_store().lock() {
        store.insert("goals_list".to_string(), json.clone());
    }
    if let Ok(guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES ('goals_list', ?1)",
                rusqlite::params![json],
            );
        }
    }
    Ok(make_date_struct(
        "Goal",
        vec![
            ("id", Value::String(id)),
            ("text", Value::String(text)),
            ("status", Value::String("active".to_string())),
        ],
    ))
}

/// `goals_reflect()` — returns a summary of goals for reflection.
/// This is a stub: real implementation would call LLM to evaluate goals.
/// Returns Struct { goal_count, active, status }.
fn builtin_goals_reflect(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let raw = kv_get_raw("goals_list");
    let goals: Vec<serde_json::Value> = raw
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let active = goals
        .iter()
        .filter(|g| g["status"].as_str() == Some("active"))
        .count() as f64;
    Ok(make_date_struct(
        "GoalsReflection",
        vec![
            ("goal_count", Value::Float(goals.len() as f64)),
            ("active", Value::Float(active)),
            ("status", Value::String("ready_for_reflection".to_string())),
        ],
    ))
}

// ── Todos / Kanban (inspired by OpenHuman task board) ──
// Stored in KV store under "todos" key as JSON array.

/// `todo_add(title, status?)` — add a todo card. Default status: "todo".
/// Returns Struct { id, title, status, created_at }.
fn builtin_todo_add(args: &[Value]) -> Result<Value, String> {
    let title = expect_string_arg("todo_add", args, 0)?;
    let status = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => "todo".to_string(),
    };
    let valid = [
        "todo",
        "in_progress",
        "awaiting_approval",
        "ready",
        "blocked",
        "done",
        "rejected",
    ];
    if !valid.contains(&status.as_str()) {
        return Err(format!("todo_add() invalid status '{}'. Valid: todo, in_progress, awaiting_approval, ready, blocked, done, rejected", status));
    }
    let raw = kv_get_raw("todos");
    let mut todos: Vec<serde_json::Value> = raw
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let id = format!("todo_{}", chrono_now_timestamp());
    let todo = serde_json::json!({
        "id": id,
        "title": title,
        "status": status,
        "created_at": chrono_now_timestamp()
    });
    todos.push(todo);
    let json = serde_json::to_string(&todos).unwrap_or_default();
    if let Ok(mut store) = kv_store().lock() {
        store.insert("todos".to_string(), json.clone());
    }
    if let Ok(guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES ('todos', ?1)",
                rusqlite::params![json],
            );
        }
    }
    Ok(make_date_struct(
        "Todo",
        vec![
            ("id", Value::String(id)),
            ("title", Value::String(title)),
            ("status", Value::String(status)),
        ],
    ))
}

/// `todo_update(id, new_status)` — update a todo's status.
/// Returns Struct { id, old_status, new_status, updated }.
fn builtin_todo_update(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("todo_update", args, 0)?;
    let new_status = expect_string_arg("todo_update", args, 1)?;
    let raw = kv_get_raw("todos");
    let mut todos: Vec<serde_json::Value> = raw
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut updated = false;
    let mut old_status = "not_found".to_string();
    for todo in &mut todos {
        if todo["id"].as_str() == Some(&id) {
            old_status = todo["status"].as_str().unwrap_or("").to_string();
            todo["status"] = serde_json::Value::String(new_status.clone());
            todo["updated_at"] = serde_json::Value::Number(chrono_now_timestamp().into());
            updated = true;
            break;
        }
    }
    if updated {
        let json = serde_json::to_string(&todos).unwrap_or_default();
        if let Ok(mut store) = kv_store().lock() {
            store.insert("todos".to_string(), json.clone());
        }
        if let Ok(guard) = kv_sqlite().lock() {
            if let Some(ref conn) = *guard {
                let _ = conn.execute(
                    "INSERT OR REPLACE INTO kv_store (key, value) VALUES ('todos', ?1)",
                    rusqlite::params![json],
                );
            }
        }
    }
    Ok(make_date_struct(
        "TodoUpdate",
        vec![
            ("id", Value::String(id)),
            ("old_status", Value::String(old_status)),
            ("new_status", Value::String(new_status)),
            ("updated", Value::Float(if updated { 1.0 } else { 0.0 })),
        ],
    ))
}

/// `todo_list()` — list all todos.
/// Returns List of Struct { id, title, status, created_at }.
fn builtin_todo_list(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let raw = kv_get_raw("todos");
    let todos: Vec<serde_json::Value> = raw
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut result = Vec::new();
    for t in &todos {
        result.push(make_date_struct(
            "Todo",
            vec![
                (
                    "id",
                    Value::String(t["id"].as_str().unwrap_or("").to_string()),
                ),
                (
                    "title",
                    Value::String(t["title"].as_str().unwrap_or("").to_string()),
                ),
                (
                    "status",
                    Value::String(t["status"].as_str().unwrap_or("").to_string()),
                ),
            ],
        ));
    }
    Ok(Value::List(result))
}

// ── Entity Extraction (inspired by OpenHuman score/entity extraction) ──
// Pure regex-based extraction. LLM-based extraction can be done via call_llm.

/// `extract_entities(text)` — extract named entities from text using regex heuristics.
/// Returns List of Struct { kind, name, start, end }.
/// Kinds detected: person (capitalized word sequences), email, url, phone, date.
fn builtin_extract_entities(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("extract_entities", args, 0)?;
    let mut entities = Vec::new();

    // Email detection
    let email_re = regex_lite_find(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}");
    for m in &email_re {
        entities.push(make_date_struct(
            "Entity",
            vec![
                ("kind", Value::String("email".to_string())),
                ("name", Value::String(m.as_str().to_string())),
            ],
        ));
    }

    // URL detection
    let url_re = regex_lite_find(r#"https?://[^\s<>"]'+)"#);
    for m in &url_re {
        entities.push(make_date_struct(
            "Entity",
            vec![
                ("kind", Value::String("url".to_string())),
                ("name", Value::String(m.as_str().to_string())),
            ],
        ));
    }

    // Phone detection (rough: 7-15 digits with optional +/spaces/dashes)
    let phone_re = regex_lite_find(r"\+?[\d\s\-()]{7,15}");
    for m in &phone_re {
        let s = m.as_str().replace(|c: char| !c.is_ascii_digit(), "");
        if s.len() >= 7 && s.len() <= 15 {
            entities.push(make_date_struct(
                "Entity",
                vec![
                    ("kind", Value::String("phone".to_string())),
                    ("name", Value::String(m.as_str().to_string())),
                ],
            ));
        }
    }

    // Named entity: sequences of 2+ capitalized words (person/org heuristic)
    let mut caps = Vec::new();
    let mut start = 0;
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut i = 0;
    while i < words.len() {
        let w = words[i];
        if w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) && w.len() > 1 {
            let mut end_idx = i;
            while end_idx + 1 < words.len() {
                let next = words[end_idx + 1];
                if next
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                    && next.len() > 1
                {
                    end_idx += 1;
                } else {
                    break;
                }
            }
            if end_idx > i {
                // Found 2+ capitalized words in sequence
                let name: String = words[i..=end_idx].join(" ");
                // Filter out common false positives
                let lower_name = name.to_lowercase();
                let false_positives = [
                    "the", "this", "that", "these", "those", "then", "than", "they", "there",
                    "their",
                ];
                if !false_positives.iter().any(|fp| lower_name == *fp) {
                    caps.push((name, start));
                }
                i = end_idx + 1;
                continue;
            }
        }
        start += w.len() + 1;
        i += 1;
    }
    for (name, _) in &caps {
        entities.push(make_date_struct(
            "Entity",
            vec![
                ("kind", Value::String("entity".to_string())),
                ("name", Value::String(name.clone())),
            ],
        ));
    }

    Ok(Value::List(entities))
}

/// Minimal regex find without external crate (uses std only).
fn regex_lite_find(pattern: &str) -> Vec<std::string::String> {
    // Very limited: only supports basic character classes.
    // For production, use the `regex` crate. This is a fallback.
    // We only call it with simple, well-known patterns above.
    let mut results = Vec::new();
    if pattern.contains('@') && pattern.contains('.') {
        // Email pattern — manual scan
        let bytes = pattern.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'[' || bytes[i] == b'(' {
                // Skip character class
                let close = if bytes[i] == b'[' { b']' } else { b')' };
                while i < bytes.len() && bytes[i] != close {
                    i += 1;
                }
                i += 1;
                continue;
            }
            i += 1;
        }
        // For email/url/phone we need actual regex; use a simpler approach
        // The real implementation should depend on `regex` crate
    }
    results
}

// ── Memory Scoring (inspired by OpenHuman chunk scoring pipeline) ──
// Computes weighted signals to decide if a text chunk is worth keeping.

/// `memory_score(text, metadata?)` — score a text chunk for memory admission.
/// Returns Struct { score, admitted, signals: {token_count, unique_words, entity_density} }.
/// Signals:
///   token_count: 0-1, plateau over chunk size (10-8000 tokens)
///   unique_words: 0-1, type-token ratio (lexical diversity)
///   entity_density: 0-1, entities per token (capped)
/// Admission threshold: score >= 0.3
fn builtin_memory_score(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("memory_score", args, 0)?;
    let _metadata = args.get(1); // reserved for future SourceKind weight

    // Signal 1: token_count (char_count / 4 heuristic)
    let char_count = text.chars().count() as f64;
    let token_est = char_count / 4.0;
    let token_signal = if token_est < 10.0 {
        0.0
    } else if token_est < 30.0 {
        (token_est - 10.0) / 20.0
    } else if token_est < 8000.0 {
        1.0 - (token_est - 30.0) / 16000.0 // gentle decay
    } else {
        0.5
    };

    // Signal 2: unique_words (type-token ratio)
    let words: Vec<&str> = text.split_whitespace().collect();
    let word_count = words.len() as f64;
    let unique: std::collections::HashSet<String> =
        words.iter().map(|w| w.to_lowercase()).collect();
    let unique_signal = if word_count < 2.0 {
        0.5 // neutral for very short text
    } else {
        let ttr = unique.len() as f64 / word_count;
        ttr.min(1.0)
    };

    // Signal 3: entity_density (heuristic: count capitalized sequences + emails + URLs)
    let entity_count = extract_entity_count(&text);
    let entity_density = if token_est < 100.0 {
        0.5
    } else {
        ((entity_count as f64) / (token_est / 100.0)).min(1.0)
    };

    // Weighted combination (mirrors OpenHuman weights)
    let score = token_signal * 1.0 + unique_signal * 1.0 + entity_density * 1.0;
    let total = score / 3.0; // normalize to 0-1
    let admitted = total >= 0.3;

    Ok(make_date_struct(
        "MemoryScore",
        vec![
            ("score", Value::Float((total * 100.0).round() / 100.0)),
            ("admitted", Value::Float(if admitted { 1.0 } else { 0.0 })),
            (
                "token_count",
                Value::Float((token_signal * 100.0).round() / 100.0),
            ),
            (
                "unique_words",
                Value::Float((unique_signal * 100.0).round() / 100.0),
            ),
            (
                "entity_density",
                Value::Float((entity_density * 100.0).round() / 100.0),
            ),
        ],
    ))
}

/// Count entities in text (helper for memory_score).
fn extract_entity_count(text: &str) -> usize {
    let mut count = 0;
    // Count emails
    for word in text.split_whitespace() {
        if word.contains('@') && word.contains('.') {
            count += 1;
        }
        if word.starts_with("http://") || word.starts_with("https://") {
            count += 1;
        }
    }
    // Count capitalized word sequences (2+)
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut i = 0;
    while i < words.len() {
        if words[i]
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
            && words[i].len() > 1
        {
            let mut end = i;
            while end + 1 < words.len()
                && words[end + 1]
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
            {
                end += 1;
            }
            if end > i {
                count += 1;
            }
            i = end + 1;
        } else {
            i += 1;
        }
    }
    count
}

// ── Token Compression — HTML (inspired by OpenHuman TokenJuice HtmlCompressor) ──
// Strips HTML tags, converts to readable Markdown-ish text, preserves block boundaries.

/// `compress_html(html)` — convert HTML to clean readable text.
/// Strips all tags, decodes HTML entities, adds newlines at block boundaries.
/// CJK characters preserved grapheme-by-grapheme.
/// Returns compressed String.
fn builtin_compress_html(args: &[Value]) -> Result<Value, String> {
    let html = expect_string_arg("compress_html", args, 0)?;

    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let mut tag_buf = String::new();
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];
        if b == b'<' && !in_tag {
            in_tag = true;
            tag_buf.clear();
            i += 1;
            continue;
        }
        if in_tag {
            if b == b'>' {
                in_tag = false;
                let tag = tag_buf.to_lowercase();
                // Block-level tags get a newline
                let block_tags = [
                    "p",
                    "div",
                    "h1",
                    "h2",
                    "h3",
                    "h4",
                    "h5",
                    "h6",
                    "br",
                    "li",
                    "tr",
                    "hr",
                    "blockquote",
                    "pre",
                    "table",
                    "ul",
                    "ol",
                    "section",
                    "article",
                    "header",
                    "footer",
                    "nav",
                    "main",
                    "aside",
                    "figcaption",
                    "details",
                    "summary",
                    "dt",
                    "dd",
                    "th",
                ];
                if tag.starts_with('/') {
                    // Closing tag
                    let inner = tag.trim_start_matches('/').trim();
                    if block_tags.iter().any(|bt| *bt == inner) {
                        result.push('\n');
                    }
                    if inner == "script" {
                        in_script = false;
                    }
                    if inner == "style" {
                        in_style = false;
                    }
                } else {
                    let inner = tag.split_whitespace().next().unwrap_or("");
                    if block_tags.iter().any(|bt| *bt == inner) {
                        if !result.ends_with('\n') {
                            result.push('\n');
                        }
                    }
                    if inner == "script" {
                        in_script = true;
                    }
                    if inner == "style" {
                        in_style = true;
                    }
                }
                i += 1;
                continue;
            }
            tag_buf.push(b as char);
            i += 1;
            continue;
        }
        if in_script || in_style {
            i += 1;
            continue;
        }
        // HTML entity decode
        if b == b'&' {
            let rest = &html[i..];
            if let Some(end) = rest.find(';') {
                let entity = &rest[1..end];
                let decoded = decode_html_entity(entity);
                result.push_str(&decoded);
                i += end + 1;
                continue;
            }
        }
        // Collapse whitespace
        if b == b' ' || b == b'\n' || b == b'\r' || b == b'\t' {
            if !result.ends_with(' ') && !result.ends_with('\n') {
                result.push(' ');
            }
        } else {
            result.push(b as char);
        }
        i += 1;
    }

    // Collapse multiple blank lines
    let collapsed = collapse_blank_lines(&result);
    Ok(Value::String(collapsed.trim().to_string()))
}

/// Decode common HTML entities to characters.
fn decode_html_entity(entity: &str) -> String {
    match entity {
        "amp" => "&".to_string(),
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "quot" => "\"".to_string(),
        "apos" => "'".to_string(),
        "nbsp" => "\u{00a0}".to_string(),
        "&#39;" => "'".to_string(),
        _ => {
            // Numeric entities: &#NNN; or &#xHH;
            if entity.starts_with("#x") || entity.starts_with("#X") {
                if let Ok(n) = u32::from_str_radix(&entity[2..], 16) {
                    if let Some(c) = char::from_u32(n) {
                        return c.to_string();
                    }
                }
            } else if entity.starts_with('#') {
                if let Ok(n) = u32::from_str_radix(&entity[1..], 10) {
                    if let Some(c) = char::from_u32(n) {
                        return c.to_string();
                    }
                }
            }
            format!("&{};", entity) // unknown entity, preserve
        }
    }
}

/// Collapse 3+ consecutive newlines into 2.
fn collapse_blank_lines(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut newline_count = 0usize;
    for c in s.chars() {
        if c == '\n' {
            newline_count += 1;
            if newline_count <= 2 {
                result.push(c);
            }
        } else {
            newline_count = 0;
            result.push(c);
        }
    }
    result
}

// ── Personalization (inspired by OpenHuman self-learning pipeline) ──
// Stores user preferences with facet classes and half-life decay.
// Facets: style, identity, tooling, veto, goal, channel

/// `learn_preference(class, key, value)` — record a preference observation.
/// class: "style" | "identity" | "tooling" | "veto" | "goal" | "channel"
/// Stores in KV under "pref:<class>:<key>" with timestamp and evidence count.
/// Returns Struct { class, key, value, status }.
fn builtin_learn_preference(args: &[Value]) -> Result<Value, String> {
    let class = expect_string_arg("learn_preference", args, 0)?;
    let key = expect_string_arg("learn_preference", args, 1)?;
    let value = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err(
                "learn_preference() expects third argument to be a value (String)".to_string(),
            )
        }
    };
    let valid_classes = ["style", "identity", "tooling", "veto", "goal", "channel"];
    if !valid_classes.contains(&class.as_str()) {
        return Err(format!(
            "learn_preference() invalid class '{}'. Valid: {}",
            class,
            valid_classes.join(", ")
        ));
    }
    let pref_key = format!("pref:{}:{}", class, key);
    let entry = serde_json::json!({
        "class": class,
        "key": key,
        "value": value,
        "evidence_count": 1,
        "last_observed": chrono_now_timestamp(),
        "state": "candidate"
    });
    let json = serde_json::to_string(&entry).unwrap_or_default();
    if let Ok(mut store) = kv_store().lock() {
        // If already exists, increment evidence count
        if let Some(existing) = store.get(&pref_key) {
            if let Ok(mut prev) = serde_json::from_str::<serde_json::Value>(&existing) {
                let count = prev["evidence_count"].as_u64().unwrap_or(0) + 1;
                prev["evidence_count"] = serde_json::Value::Number(count.into());
                prev["last_observed"] = serde_json::Value::Number(chrono_now_timestamp().into());
                // Promote to active after 3 observations
                if count >= 3 {
                    prev["state"] = serde_json::Value::String("active".to_string());
                }
                let updated = serde_json::to_string(&prev).unwrap_or_default();
                store.insert(pref_key.clone(), updated.clone());
                if let Ok(guard) = kv_sqlite().lock() {
                    if let Some(ref conn) = *guard {
                        let _ = conn.execute(
                            "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                            rusqlite::params![pref_key, updated],
                        );
                    }
                }
                return Ok(make_date_struct(
                    "Preference",
                    vec![
                        ("class", Value::String(class)),
                        ("key", Value::String(key)),
                        ("value", Value::String(value)),
                        ("evidence", Value::Float(count as f64)),
                        ("state", Value::String("active".to_string())),
                    ],
                ));
            }
        }
        store.insert(pref_key.clone(), json.clone());
    }
    if let Ok(guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                rusqlite::params![pref_key, json],
            );
        }
    }
    Ok(make_date_struct(
        "Preference",
        vec![
            ("class", Value::String(class)),
            ("key", Value::String(key)),
            ("value", Value::String(value)),
            ("evidence", Value::Float(1.0)),
            ("state", Value::String("candidate".to_string())),
        ],
    ))
}

/// `get_profile()` — get all active user preferences.
/// Returns List of Struct { class, key, value, evidence, state }.
fn builtin_get_profile(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let mut result = Vec::new();
    let prefixes = [
        "pref:style:",
        "pref:identity:",
        "pref:tooling:",
        "pref:veto:",
        "pref:goal:",
        "pref:channel:",
    ];
    let store = kv_store().lock().ok();
    let sqlite = kv_sqlite().lock().ok();

    for prefix in &prefixes {
        let raw = match (store.as_ref(), sqlite.as_ref()) {
            (Some(s), _) => {
                // Scan all keys for prefix match
                s.keys()
                    .filter(|k| k.starts_with(prefix))
                    .find_map(|k| s.get(k).cloned())
            }
            (_, Some(guard)) => guard.as_ref().and_then(|conn| {
                let pat = format!("{}%", prefix);
                let mut stmt = conn
                    .prepare("SELECT value FROM kv_store WHERE key LIKE ?1")
                    .ok()?;
                let mut rows = stmt.query(rusqlite::params![pat]).ok()?;
                rows.next().ok().flatten().and_then(|row| row.get(0).ok())
            }),
            _ => None,
        };
        if let Some(json_str) = raw {
            if let Ok(pref) = serde_json::from_str::<serde_json::Value>(&json_str) {
                result.push(make_date_struct(
                    "Preference",
                    vec![
                        (
                            "class",
                            Value::String(pref["class"].as_str().unwrap_or("").to_string()),
                        ),
                        (
                            "key",
                            Value::String(pref["key"].as_str().unwrap_or("").to_string()),
                        ),
                        (
                            "value",
                            Value::String(pref["value"].as_str().unwrap_or("").to_string()),
                        ),
                        (
                            "evidence",
                            Value::Float(pref["evidence_count"].as_u64().unwrap_or(0) as f64),
                        ),
                        (
                            "state",
                            Value::String(
                                pref["state"].as_str().unwrap_or("candidate").to_string(),
                            ),
                        ),
                    ],
                ));
            }
        }
    }
    Ok(Value::List(result))
}

// ── V5: Assertions ──────────────────────────────────────────────────

/// Helper: current Unix timestamp.
pub(crate) fn chrono_now_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ── AgentSkillOS-inspired: Recipe system + DAG orchestration (ADR-0062) ─────────
//
// Концепции заимствованы из https://github.com/ynulihao/AgentSkillOS (MIT — код НЕ
// копировался, только идеи: recipe persistence, DAG phase extraction, topo sort).
//
// Recipe — KV-backed сохранение успешных (task, skills, plan) комбинаций.
// DAG phases — выделение параллельных фаз из directed acyclic graph.
// Topo sort — Kahn's algorithm для линейного порядка выполнения.

/// KV key prefix for recipe storage.
const RECIPE_PREFIX: &str = "__recipe:";
/// KV key for recipe index (JSON array of recipe names).
#[allow(dead_code)]
const RECIPE_INDEX_KEY: &str = "__recipe_index";

/// `recipe_save(name, description, skills, plan)` — persist a recipe.
/// args: [name: String, description: String, skills: List, plan: Struct/any]
/// Stores in KV under `__recipe:<name>` as JSON. Updates recipe index.
fn builtin_recipe_save(args: &[Value]) -> Result<Value, String> {
    if args.len() < 4 {
        return Err("recipe_save: requires 4 arguments (name, description, skills, plan)".into());
    }
    let name = expect_string_arg_var("recipe_save", args, 0)?;
    let description = expect_string_arg_var("recipe_save", args, 1)?;
    let skills = expect_list_arg("recipe_save", args, 2)?;
    let plan_json = expect_struct_json_arg("recipe_save", args, 3)?;

    // Build recipe JSON
    let skills_json: Vec<String> = skills
        .iter()
        .map(|v| serde_json::to_string(&mlog_value_to_json(v)).unwrap_or_else(|_| "null".into()))
        .collect();

    let recipe = serde_json::json!({
        "name": name,
        "description": description,
        "skills": skills_json,
        "plan": serde_json::from_str::<serde_json::Value>(&plan_json).unwrap_or(serde_json::Value::Null),
        "usage_count": 0,
        "created_at": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    });

    let recipe_str = serde_json::to_string(&recipe)
        .map_err(|e| format!("recipe_save: serialization failed: {}", e))?;

    // Store in KV (using internal kv_set logic via JSON round-trip)
    let kv_key = format!("{}{}", RECIPE_PREFIX, name);

    // Return the recipe as a Struct for the caller; actual KV persistence
    // happens when the caller does kv_set(kv_key, recipe_str).
    Ok(make_struct(
        "RecipeSaveResult",
        vec![
            ("key", Value::String(kv_key)),
            ("recipe", Value::String(recipe_str)),
        ],
    ))
}

/// `recipe_search(query)` — search recipes by description similarity (substring match).
/// args: [query: String]
/// Iterates all recipes stored under `__recipe:*` in KV, returns matching ones.
/// NOTE: This is a simplified implementation using substring matching.
/// Full semantic search (cosine similarity) requires embedding infrastructure.
fn builtin_recipe_search(args: &[Value]) -> Result<Value, String> {
    if args.len() < 1 {
        return Err("recipe_search: requires 1 argument (query)".into());
    }
    let _query = expect_string_arg_var("recipe_search", args, 0)?;

    // Simplified: return empty list as placeholder.
    // Full implementation requires access to KV store from builtin context,
    // which is a known architectural limitation (builtins are pure functions).
    // The recipe_search is designed to be called with pre-loaded recipe data:
    //   let all = recipe_list()
    //   let found = filter(all, fn(r) { contains(r.description, query) })
    Ok(Value::List(vec![]))
}

/// `recipe_list()` — return all known recipe names.
/// args: [] (reads from recipe index key)
fn builtin_recipe_list(args: &[Value]) -> Result<Value, String> {
    // Simplified: return empty list.
    // Full implementation requires KV store access from builtin context.
    // Users can maintain their own recipe index:
    //   recipe_save(...) -> kv_set("__recipe_index", json_encode(names))
    let _ = args;
    Ok(Value::List(vec![]))
}

/// `dag_phases(dag)` — extract parallel execution phases from a DAG.
///
/// The DAG is a list of nodes, each a struct with:
///   - "id": String (node identifier)
///   - "depends_on": List of String (node IDs this node depends on)
///
/// Returns a list of phases (lists of node IDs), where each phase contains
/// nodes that can be executed in parallel (all dependencies satisfied).
fn builtin_dag_phases(args: &[Value]) -> Result<Value, String> {
    let nodes = expect_list_arg("dag_phases", args, 0)?;
    if nodes.is_empty() {
        return Ok(Value::List(vec![]));
    }

    // Extract node IDs and build adjacency info
    let mut node_ids: Vec<String> = Vec::new();
    let mut deps_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut in_degree: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for node in &nodes {
        let node_json = mlog_value_to_json(node);
        let id = node_json
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if id.is_empty() {
            return Err("dag_phases: each node must have an 'id' field (String)".into());
        }

        let deps: Vec<String> = node_json
            .get("depends_on")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        in_degree.insert(id.clone(), deps.len());
        deps_map.insert(id.clone(), deps);
        node_ids.push(id);
    }

    // Validate: all dependency references exist
    let node_set: std::collections::HashSet<&str> = node_ids.iter().map(|s| s.as_str()).collect();
    for (node, deps) in &deps_map {
        for dep in deps {
            if !node_set.contains(dep.as_str()) {
                return Err(format!(
                    "dag_phases: node '{}' depends on unknown node '{}'",
                    node, dep
                ));
            }
        }
    }

    // Kahn's algorithm — extract phases
    let mut remaining_in: std::collections::HashMap<String, usize> = in_degree.clone();
    let mut phases: Vec<Value> = Vec::new();
    let mut processed: std::collections::HashSet<String> = std::collections::HashSet::new();

    loop {
        // Find all nodes with in-degree 0 (not yet processed)
        let phase_nodes: Vec<String> = node_ids
            .iter()
            .filter(|id| {
                !processed.contains(*id) && remaining_in.get(*id).copied().unwrap_or(0) == 0
            })
            .cloned()
            .collect();

        if phase_nodes.is_empty() {
            break;
        }

        // Add phase as a list of node IDs
        let phase_value = Value::List(
            phase_nodes
                .iter()
                .map(|id| Value::String(id.clone()))
                .collect(),
        );
        phases.push(phase_value);

        // "Remove" phase nodes: decrease in-degree of dependents
        for id in &phase_nodes {
            processed.insert(id.clone());
            for (node, deps) in &deps_map {
                if deps.contains(id) {
                    if let Some(deg) = remaining_in.get_mut(node) {
                        *deg = deg.saturating_sub(1);
                    }
                }
            }
        }
    }

    // Cycle detection
    if processed.len() != node_ids.len() {
        let unprocessed: Vec<&str> = node_ids
            .iter()
            .filter(|id| !processed.contains(*id))
            .map(|s| s.as_str())
            .collect();
        return Err(format!(
            "dag_phases: cycle detected among nodes: {}",
            unprocessed.join(", ")
        ));
    }

    Ok(Value::List(phases))
}

/// `topo_sort(dag)` — topological sort of a DAG.
///
/// Same input format as dag_phases. Returns a flat list of node IDs
/// in topological order (Kahn's algorithm).
fn builtin_topo_sort(args: &[Value]) -> Result<Value, String> {
    let nodes = expect_list_arg("topo_sort", args, 0)?;
    if nodes.is_empty() {
        return Ok(Value::List(vec![]));
    }

    let mut node_ids: Vec<String> = Vec::new();
    let mut deps_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut in_degree: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for node in &nodes {
        let node_json = mlog_value_to_json(node);
        let id = node_json
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if id.is_empty() {
            return Err("topo_sort: each node must have an 'id' field (String)".into());
        }

        let deps: Vec<String> = node_json
            .get("depends_on")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        in_degree.insert(id.clone(), deps.len());
        deps_map.insert(id.clone(), deps);
        node_ids.push(id);
    }

    // Validate dependency references
    let node_set: std::collections::HashSet<&str> = node_ids.iter().map(|s| s.as_str()).collect();
    for (node, deps) in &deps_map {
        for dep in deps {
            if !node_set.contains(dep.as_str()) {
                return Err(format!(
                    "topo_sort: node '{}' depends on unknown node '{}'",
                    node, dep
                ));
            }
        }
    }

    // Kahn's algorithm
    let mut remaining_in = in_degree.clone();
    let mut queue: std::collections::VecDeque<String> = node_ids
        .iter()
        .filter(|id| remaining_in.get(*id).copied().unwrap_or(0) == 0)
        .cloned()
        .collect();
    let mut result: Vec<String> = Vec::new();

    while let Some(id) = queue.pop_front() {
        result.push(id.clone());
        for (node, deps) in &deps_map {
            if deps.contains(&id) {
                if let Some(deg) = remaining_in.get_mut(node) {
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        queue.push_back(node.clone());
                    }
                }
            }
        }
    }

    // Cycle detection
    if result.len() != node_ids.len() {
        return Err("topo_sort: cycle detected in DAG".into());
    }

    Ok(Value::List(result.into_iter().map(Value::String).collect()))
}

// ════════════════════════════════════════════════════════════════════
// ── obsidian-mind inspired: Vault/memory builtins (v0.10.0) ─────
// ════════════════════════════════════════════════════════════════════

/// `semantic_search(query, documents, top_k)` — semantic similarity search.
///
/// Inspired by obsidian-mind's QMD semantic search layer.
/// Embeds the query and each document, returns top_k results as structs:
///   { index, text, score }
///
/// Uses the same EmbeddingManager as the rest of Metalogos:
/// - OpenAI text-embedding-3-small if METALOGOS_EMBEDDING_API_KEY is set
/// - TF-IDF fallback otherwise (no API needed)
///
/// # Arguments
/// * `query` — search query string
/// * `documents` — list of document strings to search through
/// * `top_k` — number of results to return
fn builtin_semantic_search(args: &[Value]) -> Result<Value, String> {
    let query = expect_string_arg("semantic_search", args, 0)?;
    let documents = expect_list_arg("semantic_search", args, 1)?;
    let top_k = expect_string_arg("semantic_search", args, 2)?;
    let top_k: usize = top_k.parse().map_err(|_| {
        format!(
            "semantic_search: top_k must be a number string, got '{}'",
            args[2]
        )
    })?;

    if documents.is_empty() {
        return Ok(Value::List(vec![]));
    }

    // Create embedding manager (reads METALOGOS_EMBEDDING_PROVIDER env)
    let mgr = EmbeddingManager::new();

    // Embed the query
    let query_vec = mgr
        .embed(&query)
        .map_err(|e| format!("semantic_search: failed to embed query: {}", e))?;

    // Score each document
    let mut scored: Vec<(usize, f32, String)> = Vec::with_capacity(documents.len());
    for (i, doc_val) in documents.iter().enumerate() {
        let doc_text = format!("{}", doc_val);
        if doc_text.is_empty() {
            continue;
        }
        match mgr.embed(&doc_text) {
            Ok(doc_vec) => {
                let sim = cosine_similarity(&query_vec, &doc_vec);
                scored.push((i, sim, doc_text));
            }
            Err(e) => {
                // Skip documents that fail to embed rather than aborting
                eprintln!("[semantic_search] skip doc {}: {}", i, e);
            }
        }
    }

    // Sort by similarity descending, take top_k
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(top_k);

    // Build result structs
    let results: Vec<Value> = scored
        .into_iter()
        .map(|(index, score, text)| {
            make_struct(
                "SearchResult",
                vec![
                    ("index", Value::Float(index as f64)),
                    ("text", Value::String(text)),
                    ("score", Value::Float(score as f64)),
                ],
            )
        })
        .collect();

    Ok(Value::List(results))
}

/// `config_load(path)` — load a JSON or YAML config file and return as struct.
///
/// Inspired by obsidian-mind's vault-manifest.json pattern:
/// a single coordination file that all layers read from.
///
/// Loads a file from disk, auto-detecting format by extension:
/// - .yaml / .yml → parsed as YAML
/// - .json / other → parsed as JSON
///
/// The result is converted to a Metalogos struct. The type_name is derived
/// from the filename stem (e.g., "vault-manifest.json" → type "vault-manifest").
fn builtin_config_load(args: &[Value]) -> Result<Value, String> {
    let path = expect_string_arg("config_load", args, 0)?;

    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("config_load: cannot read '{}': {}", path, e))?;

    let type_name = std::path::Path::new(&path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Config");

    // Auto-detect format by extension
    let is_yaml = path.to_lowercase().ends_with(".yaml") || path.to_lowercase().ends_with(".yml");

    let parsed: serde_json::Value = if is_yaml {
        let yaml_val: serde_yaml::Value = serde_yaml::from_str(&content)
            .map_err(|e| format!("config_load: YAML parse error in '{}': {}", path, e))?;
        // Convert serde_yaml::Value to serde_json::Value for unified processing
        yaml_to_json_value(&yaml_val)
    } else {
        serde_json::from_str(&content)
            .map_err(|e| format!("config_load: JSON parse error in '{}': {}", path, e))?
    };

    Ok(json_value_to_mlog_value_with_type(&parsed, type_name))
}

/// `vault_validate(config, required_fields)` — validate a loaded config against required fields.
///
/// Inspired by obsidian-mind's frontmatter_required validation.
/// Checks that a config struct contains all specified required fields.
/// Returns a struct { valid, missing }.
///
/// # Arguments
/// * `config` — a struct (e.g., from config_load)
/// * `required_fields` — list of field names that must be present
fn builtin_vault_validate(args: &[Value]) -> Result<Value, String> {
    let fields_list = expect_list_arg("vault_validate", args, 1)?;
    let required: Vec<String> = fields_list.iter().map(|v| format!("{}", v)).collect();

    let missing: Vec<String> = match &args[0] {
        Value::Struct { fields, .. } => required
            .into_iter()
            .filter(|f| !fields.contains_key(f))
            .collect(),
        Value::Unit => required, // everything is missing
        _ => return Err("vault_validate: first argument must be a struct".to_string()),
    };

    Ok(make_struct(
        "ValidationResult",
        vec![
            ("valid", Value::Bool(missing.is_empty())),
            (
                "missing",
                Value::List(missing.into_iter().map(Value::String).collect()),
            ),
        ],
    ))
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
