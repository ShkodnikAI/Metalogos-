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
pub use memory::{reset_session_store, session_key_count, session_store_count};
pub(crate) mod cron;
pub use cron::init_reminder_persist;
use cron::*;
pub mod pdf;
pub use pdf::*;

impl Default for Builtins {
    fn default() -> Self {
        Self::new()
    }
}

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

        // Наряд №48: PDF processing (native Rust via pdf-inspector)
        funcs.insert(
            "pdf_classify".to_string(),
            builtin_pdf_classify as BuiltinFn,
        );
        funcs.insert(
            "pdf_to_markdown".to_string(),
            builtin_pdf_to_markdown as BuiltinFn,
        );
        funcs.insert(
            "pdf_extract_regions".to_string(),
            builtin_pdf_extract_regions as BuiltinFn,
        );
        funcs.insert("pdf_ocr".to_string(), builtin_pdf_ocr as BuiltinFn);

        Builtins { funcs }
    }

    /// Verify builtin registry consistency (debug builds).
    #[cfg(debug_assertions)]
    #[allow(dead_code)]
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
pub(crate) mod office;
use office::*;

#[cfg(test)]
mod tests;
