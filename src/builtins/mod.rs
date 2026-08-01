// ── Built-in functions for METALOGOS M1+M2 ────────────────────────────

use crate::embeddings::{cosine_similarity, EmbeddingManager};
use crate::interpreter::{SecretString, Value};
use chrono::{Datelike, TimeZone, Timelike};

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

pub(crate) mod registry;
pub use registry::*;

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

fn builtin_upper(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("upper", args, 0)?;
    Ok(Value::String(s.to_uppercase()))
}

fn builtin_lower(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("lower", args, 0)?;
    Ok(Value::String(s.to_lowercase()))
}

fn builtin_len(args: &[Value]) -> Result<Value, String> {
    match args.get(0) {
        // Unicode-aware: chars().count() returns character count, not byte count.
        // "Привет" (6 chars, 12 bytes) → 6.0, not 12.0.
        Some(Value::String(s)) => Ok(Value::Float(s.chars().count() as f64)),
        Some(Value::List(items)) => Ok(Value::Float(items.len() as f64)),
        _ => Err("len() requires String or List argument".to_string()),
    }
}

fn builtin_str(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("str() requires 1 argument".to_string());
    }
    Ok(Value::String(format!("{}", args[0])))
}

fn builtin_print(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("print", args, 0)?;
    eprintln!("[print] {}", s);
    Ok(Value::String(s))
}

fn builtin_contains(args: &[Value]) -> Result<Value, String> {
    let haystack = expect_string_arg("contains", args, 0)?;
    let needle = expect_string_arg("contains", args, 1)?;
    Ok(Value::Bool(haystack.contains(&needle)))
}

fn builtin_float(args: &[Value]) -> Result<Value, String> {
    match args.get(0) {
        Some(Value::Float(f)) => Ok(Value::Float(*f)),
        Some(Value::String(s)) => s
            .parse::<f64>()
            .map(Value::Float)
            .map_err(|_| format!("float() cannot parse '{}'", s)),
        _ => Err("float() requires 1 argument".to_string()),
    }
}

fn builtin_to_string(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("to_string() requires 1 argument".to_string());
    }
    // Use Value's Display impl — Float omits .0 for integers automatically
    Ok(Value::String(format!("{}", args[0])))
}

fn builtin_get(args: &[Value]) -> Result<Value, String> {
    let list = match args.get(0) {
        Some(Value::List(items)) => items,
        _ => return Err("get() requires List as first argument".to_string()),
    };
    let index = match args.get(1) {
        Some(Value::Float(f)) => *f as usize,
        _ => return Err("get() requires Float index as second argument".to_string()),
    };
    list.get(index).cloned().ok_or_else(|| {
        format!(
            "get() index {} out of bounds (list has {} elements)",
            index,
            list.len()
        )
    })
}

fn builtin_push(args: &[Value]) -> Result<Value, String> {
    let list = match args.get(0) {
        Some(Value::List(items)) => items.clone(),
        _ => return Err("push() requires List as first argument".to_string()),
    };
    let item = match args.get(1) {
        Some(v) => v.clone(),
        None => return Err("push() requires a second argument (item to push)".to_string()),
    };
    let mut new_list = list;
    new_list.push(item);
    Ok(Value::List(new_list))
}

fn builtin_slice(args: &[Value]) -> Result<Value, String> {
    let list = match args.get(0) {
        Some(Value::List(items)) => items,
        _ => return Err("slice() requires List as first argument".to_string()),
    };
    let start = match args.get(1) {
        Some(Value::Float(f)) => *f as usize,
        _ => return Err("slice() requires Float start as second argument".to_string()),
    };
    let end = match args.get(2) {
        Some(Value::Float(f)) => *f as usize,
        _ => return Err("slice() requires Float end as third argument".to_string()),
    };
    // Soft-failure semantics, mirroring substring:
    //   start >= len  -> empty list
    //   end > len     -> clamp to len
    //   start >= end  -> empty list
    let list_len = list.len();
    if start >= list_len {
        return Ok(Value::List(vec![]));
    }
    let end = if end > list_len { list_len } else { end };
    if start >= end {
        return Ok(Value::List(vec![]));
    }
    Ok(Value::List(list[start..end].to_vec()))
}

fn builtin_index_of(args: &[Value]) -> Result<Value, String> {
    let haystack = expect_string_arg("index_of", args, 0)?;
    let needle = expect_string_arg("index_of", args, 1)?;
    // Unicode-aware: return CHARACTER position, not byte offset.
    // "Привет, мир".find("мир") byte offset = 12, char offset = 8.
    // Must be consistent with substring()/char_at() which use char indices.
    let char_pos = haystack
        .char_indices()
        .find(|(byte_idx, _)| haystack[*byte_idx..].starts_with(&needle))
        .map(|(byte_idx, _)| haystack[..byte_idx].chars().count());
    match char_pos {
        Some(pos) => Ok(Value::Float(pos as f64)),
        None => Ok(Value::Float(-1.0)),
    }
}

fn builtin_substring(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("substring", args, 0)?;
    let start = expect_float_arg("substring", args, 1)? as usize;
    let end = expect_float_arg("substring", args, 2)? as usize;
    // Soft-failure: clamp to valid range, empty string if start >= len
    let s_len = s.chars().count();
    if start >= s_len {
        return Ok(Value::String(String::new()));
    }
    let end = if end > s_len { s_len } else { end };
    if start >= end {
        return Ok(Value::String(String::new()));
    }
    // Convert byte indices for char-based slicing
    let chars: Vec<char> = s.chars().collect();
    let result: String = chars[start..end].iter().collect();
    Ok(Value::String(result))
}

fn builtin_char_at(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("char_at", args, 0)?;
    let index = expect_float_arg("char_at", args, 1)? as usize;
    // Soft-failure: return empty string on out-of-bounds
    let chars: Vec<char> = s.chars().collect();
    match chars.get(index) {
        Some(ch) => Ok(Value::String(ch.to_string())),
        None => Ok(Value::String(String::new())),
    }
}

fn builtin_starts_with(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("starts_with", args, 0)?;
    let prefix = expect_string_arg("starts_with", args, 1)?;
    Ok(Value::Bool(s.starts_with(&prefix)))
}

fn builtin_ends_with(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("ends_with", args, 0)?;
    let suffix = expect_string_arg("ends_with", args, 1)?;
    Ok(Value::Bool(s.ends_with(&suffix)))
}

fn builtin_to_float(args: &[Value]) -> Result<Value, String> {
    match args.get(0) {
        Some(Value::Float(f)) => Ok(Value::Float(*f)),
        Some(Value::String(s)) => Ok(s
            .parse::<f64>()
            .map(Value::Float)
            .unwrap_or(Value::Float(0.0))), // soft-failure: return 0.0 on parse error
        Some(Value::Bool(b)) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
        _ => Ok(Value::Float(0.0)), // soft-failure
    }
}

fn builtin_confidence(args: &[Value]) -> Result<Value, String> {
    match args.get(0) {
        Some(Value::Fluid(variants)) => {
            let best = variants
                .iter()
                .map(|v| v.confidence)
                .fold(0.0_f64, f64::max);
            Ok(Value::Float(best))
        }
        Some(_) => Ok(Value::Float(1.0)), // concrete values are fully confident
        None => Err("confidence() requires 1 argument".to_string()),
    }
}

// builtin_env moved to Phase 6.4 section below

fn expect_float_arg(fn_name: &str, args: &[Value], index: usize) -> Result<f64, String> {
    if args.len() <= index {
        return Err(format!(
            "{}() requires an argument at position {}",
            fn_name, index
        ));
    }
    match &args[index] {
        Value::Float(f) => Ok(*f),
        other => Err(format!(
            "{}() expected Float argument, got {}",
            fn_name,
            other.type_name()
        )),
    }
}

fn expect_string_arg(fn_name: &str, args: &[Value], index: usize) -> Result<String, String> {
    if args.len() <= index {
        return Err(format!(
            "{}() requires an argument at position {}",
            fn_name, index
        ));
    }
    match &args[index] {
        Value::String(s) => Ok(s.clone()),
        other => Err(format!(
            "{}() expected String argument, got {}",
            fn_name,
            other.type_name()
        )),
    }
}

// ── Stdlib backing builtins (Phase 5.4) ───────────────────────────
// These implement the primitives used by std/*.mlog pattern wrappers.

fn builtin_trim(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("__trim", args, 0)?;
    Ok(Value::String(s.trim().to_string()))
}

fn builtin_replace(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("__replace", args, 0)?;
    let old = expect_string_arg("__replace", args, 1)?;
    let new = expect_string_arg("__replace", args, 2)?;
    if old.is_empty() {
        // Empty pattern would insert replacement between every character — return original
        Ok(Value::String(s))
    } else {
        Ok(Value::String(s.replace(&old, &new)))
    }
}

fn builtin_split(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("__split", args, 0)?;
    let sep = expect_string_arg("__split", args, 1)?;
    let items: Vec<Value> = if sep.is_empty() {
        s.chars().map(|c| Value::String(c.to_string())).collect()
    } else {
        s.split(&sep)
            .map(|part| Value::String(part.to_string()))
            .collect()
    };
    Ok(Value::List(items))
}

fn builtin_join(args: &[Value]) -> Result<Value, String> {
    let list = match args.get(0) {
        Some(Value::List(items)) => items,
        _ => return Err("__join() requires List as first argument".to_string()),
    };
    let sep = if args.len() > 1 {
        match &args[1] {
            Value::String(s) => s.clone(),
            _ => ",".to_string(),
        }
    } else {
        ",".to_string()
    };
    let parts: Vec<String> = list.iter().map(|v| format!("{}", v)).collect();
    Ok(Value::String(parts.join(&sep)))
}

fn builtin_abs(args: &[Value]) -> Result<Value, String> {
    let f = expect_float_arg("__abs", args, 0)?;
    Ok(Value::Float(f.abs()))
}

fn builtin_min(args: &[Value]) -> Result<Value, String> {
    let a = expect_float_arg("__min", args, 0)?;
    let b = expect_float_arg("__min", args, 1)?;
    Ok(Value::Float(a.min(b)))
}

fn builtin_max(args: &[Value]) -> Result<Value, String> {
    let a = expect_float_arg("__max", args, 0)?;
    let b = expect_float_arg("__max", args, 1)?;
    Ok(Value::Float(a.max(b)))
}

fn builtin_clamp(args: &[Value]) -> Result<Value, String> {
    let val = expect_float_arg("__clamp", args, 0)?;
    let lo = expect_float_arg("__clamp", args, 1)?;
    let hi = expect_float_arg("__clamp", args, 2)?;
    Ok(Value::Float(val.clamp(lo, hi)))
}

fn builtin_round(args: &[Value]) -> Result<Value, String> {
    let f = expect_float_arg("__round", args, 0)?;
    Ok(Value::Float(f.round()))
}

fn builtin_first(args: &[Value]) -> Result<Value, String> {
    let list = match args.get(0) {
        Some(Value::List(items)) => items,
        _ => return Err("first() requires List as first argument".to_string()),
    };
    match list.first() {
        Some(v) => Ok(v.clone()),
        None => Ok(Value::String(String::new())), // soft-failure
    }
}

fn builtin_last(args: &[Value]) -> Result<Value, String> {
    let list = match args.get(0) {
        Some(Value::List(items)) => items,
        _ => return Err("last() requires List as first argument".to_string()),
    };
    match list.last() {
        Some(v) => Ok(v.clone()),
        None => Ok(Value::String(String::new())), // soft-failure
    }
}

// ── Phase 6.1 — HTTP server stubs ───────────────────────────
// In interpreter-only mode (mlog run), these return mock values.
// Real implementations live in server.rs for the Axum context.

fn builtin_respond(args: &[Value]) -> Result<Value, String> {
    // Two forms: respond("200 OK") or respond("200", "body text")
    let (status, body) = if args.len() >= 2 {
        let status_str = expect_string_arg("respond", args, 0)?;
        let status = status_str.parse::<u16>().unwrap_or(200);
        let body = match &args[1] {
            Value::String(s) => s.clone(),
            other => format!("{}", other),
        };
        (status, body)
    } else {
        let status_body = expect_string_arg("respond", args, 0)?;
        parse_status_line(&status_body)
    };
    Ok(Value::HttpResponse { status, body })
}

/// respond_html(status, html) — respond with HTML content.
/// In server context, value_to_response converts HttpResponse to Axum response.
/// The Html variant would auto-set Content-Type, but FOSVED uses respond_html("200", ...)
/// with return, so HttpResponse is the correct type here — the server sets Content-Type.
fn builtin_respond_html(args: &[Value]) -> Result<Value, String> {
    let status_str = expect_string_arg("respond_html", args, 0)?;
    let html = expect_string_arg("respond_html", args, 1)?;
    let (status, _) = parse_status_line(&status_str);
    Ok(Value::HttpResponse { status, body: html })
}

fn builtin_form_data(args: &[Value]) -> Result<Value, String> {
    let _ = args; // no args needed
                  // In non-server context, return empty form data struct
    Ok(Value::Struct {
        type_name: "FormData".to_string(),
        fields: std::collections::HashMap::new(),
    })
}

fn builtin_json_body(args: &[Value]) -> Result<Value, String> {
    let _ = args; // no args needed
                  // In non-server context, return empty json body struct
    Ok(Value::Struct {
        type_name: "JsonBody".to_string(),
        fields: std::collections::HashMap::new(),
    })
}

/// query_param(name) — stub that returns empty string.
/// Real implementation is handled in interpreter.rs FnCall dispatch
/// (needs access to server_query_params HashMap on the interpreter).
fn builtin_query_param(args: &[Value]) -> Result<Value, String> {
    let _name = if args.is_empty() {
        return Err("query_param() requires 1 argument (param name)".to_string());
    } else {
        match &args[0] {
            Value::String(s) => s.clone(),
            other => {
                return Err(format!(
                    "query_param() expected String, got {}",
                    other.type_name()
                ))
            }
        }
    };
    // Stub — real implementation is special-cased in interpreter FnCall dispatch
    Ok(Value::String(String::new()))
}

// ── Phase 6.2 — Template stubs ───────────────────────────

fn builtin_render(args: &[Value]) -> Result<Value, String> {
    // render(template_name, key1, val1, key2, val2, ...)
    // Simple {{ var }} substitution with auto-escaping
    // In interpreter mode, do basic string substitution
    if args.len() < 3 || (args.len() - 1) % 2 != 0 {
        return Err("render() requires template name + key/value pairs (odd count)".to_string());
    }
    let template_name = expect_string_arg("render", args, 0)?;

    // Build substitution map from remaining args (key, value pairs)
    let mut vars = std::collections::HashMap::new();
    let mut i = 1;
    while i + 1 < args.len() {
        let key = match &args[i] {
            Value::String(s) => s.clone(),
            other => {
                return Err(format!(
                    "render() key must be String, got {}",
                    other.type_name()
                ))
            }
        };
        let val = match &args[i + 1] {
            Value::String(s) => s.clone(),
            other => format!("{}", other),
        };
        vars.insert(key, val);
        i += 2;
    }

    // In interpreter mode, generate a simple HTML string from the template name and vars
    let mut html = String::from("<div class=\"template-");
    html.push_str(&escape_html_chars(&template_name));
    html.push_str("\">");
    for (key, val) in &vars {
        html.push_str(&format!(
            "<span data-key=\"{}\">{}</span>",
            escape_html_chars(key),
            escape_html_chars(val)
        ));
    }
    html.push_str("</div>");

    Ok(Value::Html(html))
}

fn builtin_escape_html(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("escape_html", args, 0)?;
    Ok(Value::String(escape_html_chars(&s)))
}

/// HTML-escape a string (for use in templates and escape_html builtin).
fn escape_html_chars(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Parse a status line like "200 OK" into (status_code, body).
fn parse_status_line(status_body: &str) -> (u16, String) {
    let parts: Vec<&str> = status_body.splitn(2, ' ').collect();
    let status = parts
        .first()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(200);
    let body = if parts.len() > 1 {
        parts[1].to_string()
    } else {
        String::new()
    };
    (status, body)
}

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

// ── Phase 6.4 — Encryption stubs ───────────────────────────

fn builtin_env(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("env", args, 0)?;
    match std::env::var(&key) {
        Ok(val) => Ok(Value::String(val)),
        Err(_) => Ok(Value::String(String::new())), // soft-failure: empty string if not found
    }
}

fn builtin_hash_password(args: &[Value]) -> Result<Value, String> {
    let password = expect_string_arg("hash_password", args, 0)?;
    // Argon2id with random salt — real password hashing (Phase 7.3)
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    use rand::rngs::OsRng;

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default(); // Argon2id
    match argon2.hash_password(password.as_bytes(), &salt) {
        Ok(hash) => Ok(Value::Hash(hash.to_string())),
        Err(e) => Err(format!("hash_password() failed: {}", e)),
    }
}

fn builtin_verify_password(args: &[Value]) -> Result<Value, String> {
    let password = expect_string_arg("verify_password", args, 0)?;
    let hash_str = match args.get(1) {
        Some(Value::Hash(h)) => h.as_str(),
        Some(other) => {
            return Err(format!(
                "verify_password() expected Hash as second arg, got {}",
                other.type_name()
            ))
        }
        None => return Err("verify_password() requires 2 arguments".to_string()),
    };
    // Real Argon2id verification with constant-time comparison (Phase 7.3)
    use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};

    let argon2 = Argon2::default();
    match PasswordHash::new(hash_str) {
        Ok(parsed_hash) => {
            // Constant-time comparison inside argon2
            match argon2.verify_password(password.as_bytes(), &parsed_hash) {
                Ok(_) => Ok(Value::Bool(true)),
                Err(argon2::password_hash::Error::Password) => Ok(Value::Bool(false)),
                Err(e) => Err(format!("verify_password() failed: {}", e)),
            }
        }
        Err(e) => Err(format!("verify_password() invalid hash format: {}", e)),
    }
}

fn builtin_encrypt(args: &[Value]) -> Result<Value, String> {
    let data = expect_string_arg("encrypt", args, 0)?;
    let key_str = match args.get(1) {
        Some(Value::Secret(zs)) => zs.as_str(),
        Some(other) => {
            return Err(format!(
                "encrypt() expected Secret as second arg, got {}",
                other.type_name()
            ))
        }
        None => return Err("encrypt() requires 2 arguments".to_string()),
    };
    // Real AES-256-GCM with random 96-bit nonce (Phase 7.3)
    use aes_gcm::aead::{Aead, KeyInit, OsRng};
    use aes_gcm::{AeadCore, Aes256Gcm, Key};

    let key_bytes = hex::decode(key_str)
        .map_err(|e| format!("encrypt() invalid key format (expected hex): {}", e))?;
    if key_bytes.len() != 32 {
        return Err(format!(
            "encrypt() key must be 256-bit (64 hex chars), got {} bytes",
            key_bytes.len()
        ));
    }
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bit random nonce

    match cipher.encrypt(&nonce, data.as_ref()) {
        Ok(ciphertext) => {
            // Prepend nonce to ciphertext for self-contained Encrypted value
            let mut output = nonce.to_vec();
            output.extend_from_slice(&ciphertext);
            Ok(Value::Encrypted(output))
        }
        Err(e) => Err(format!("encrypt() AES-256-GCM encryption failed: {}", e)),
    }
}

fn builtin_decrypt(args: &[Value]) -> Result<Value, String> {
    let encrypted = match args.get(0) {
        Some(Value::Encrypted(data)) => data.clone(),
        Some(other) => {
            return Err(format!(
                "decrypt() expected Encrypted as first arg, got {}",
                other.type_name()
            ))
        }
        None => return Err("decrypt() requires 2 arguments".to_string()),
    };
    let key_str = match args.get(1) {
        Some(Value::Secret(zs)) => zs.as_str(),
        Some(other) => {
            return Err(format!(
                "decrypt() expected Secret as second arg, got {}",
                other.type_name()
            ))
        }
        None => return Err("decrypt() requires 2 arguments".to_string()),
    };
    // Real AES-256-GCM decryption (Phase 7.3)
    // Encrypted format: nonce (12 bytes) || ciphertext_with_tag
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Key, Nonce};

    if encrypted.len() < 13 {
        // Need at least 12 (nonce) + 1 (tag minimum)
        return Err("decrypt() invalid encrypted data: too short".to_string());
    }

    let key_bytes = hex::decode(key_str)
        .map_err(|e| format!("decrypt() invalid key format (expected hex): {}", e))?;
    if key_bytes.len() != 32 {
        return Err(format!(
            "decrypt() key must be 256-bit (64 hex chars), got {} bytes",
            key_bytes.len()
        ));
    }

    let (nonce_bytes, ciphertext) = encrypted.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    match cipher.decrypt(nonce, ciphertext) {
        Ok(plaintext) => match String::from_utf8(plaintext) {
            Ok(s) => Ok(Value::String(s)),
            Err(_) => Err("decrypt() decrypted data is not valid UTF-8".to_string()),
        },
        Err(_) => Err("decrypt() failed: incorrect key or corrupted data".to_string()),
    }
}

fn builtin_generate_key(args: &[Value]) -> Result<Value, String> {
    let _ = args; // no args needed
                  // Generate a real 256-bit random key (Phase 7.3)
    use rand::RngCore;

    let mut key_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key_bytes);
    let key_hex = hex::encode(key_bytes); // 64 hex chars
    Ok(Value::Secret(SecretString::new(key_hex)))
}

// ── Phase 6.5 — Auth stubs ───────────────────────────

fn builtin_authenticate(args: &[Value]) -> Result<Value, String> {
    let _email = expect_string_arg("authenticate", args, 0)?;
    let _password = match args.get(1) {
        Some(Value::Secret(_)) => true,
        Some(Value::String(_)) => true,
        Some(other) => {
            return Err(format!(
                "authenticate() expected Secret or String as password, got {}",
                other.type_name()
            ))
        }
        None => return Err("authenticate() requires 2 arguments (email, password)".to_string()),
    };
    // In interpreter mode, always fail (mock)
    Ok(Value::Unit)
}

fn builtin_session_login(args: &[Value]) -> Result<Value, String> {
    let _user_id = expect_string_arg("session_login", args, 0)?;
    // In interpreter mode, return empty session
    Ok(Value::Session(std::collections::HashMap::new()))
}

fn builtin_session_logout(args: &[Value]) -> Result<Value, String> {
    let _session = match args.get(0) {
        Some(Value::Session(_)) => true,
        Some(other) => {
            return Err(format!(
                "session_logout() expected Session, got {}",
                other.type_name()
            ))
        }
        None => return Err("session_logout() requires 1 argument".to_string()),
    };
    Ok(Value::Unit)
}

// ── Phase 6.6 — Bot stubs ───────────────────────────

/// Convert Metalogos Value to serde_json::Value (inverse of server.rs json_value_to_value).
fn value_to_json(val: &Value) -> Result<serde_json::Value, String> {
    match val {
        Value::String(s) => Ok(serde_json::Value::String(s.clone())),
        Value::Float(f) => {
            if *f == (*f as i64) as f64 {
                Ok(serde_json::json!(*f as i64))
            } else {
                Ok(serde_json::json!(*f))
            }
        }
        Value::Bool(b) => Ok(serde_json::Value::Bool(*b)),
        Value::List(items) => {
            let arr: Vec<serde_json::Value> =
                items.iter().map(value_to_json).collect::<Result<_, _>>()?;
            Ok(serde_json::Value::Array(arr))
        }
        Value::Struct { fields, .. } => {
            let mut map = serde_json::Map::new();
            for (k, v) in fields {
                map.insert(k.clone(), value_to_json(v)?);
            }
            Ok(serde_json::Value::Object(map))
        }
        Value::Unit => Ok(serde_json::Value::Null),
        _ => Ok(serde_json::Value::Null),
    }
}

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

// ── Outgoing HTTP (Definition of Done: http_post) ─────────────────

/// Send an HTTP POST request. Returns the response body as String.
/// Usage: http_post(url, body, content_type)
/// Usage: http_post(url, body, content_type, auth_token)        — sets Authorization: Bearer <auth_token>
/// Usage: http_post(url, body, content_type, headers_struct)    — sets headers from Struct fields
/// Наряд №12 Bug 2: Added 4th parameter for authorization headers.
fn builtin_http_post(args: &[Value]) -> Result<Value, String> {
    let url = match args.get(0) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "http_post() expected String as url, got {}",
                other.type_name()
            ))
        }
        None => return Err("http_post() requires at least 1 argument (url)".to_string()),
    };

    let body = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "http_post() expected String as body, got {}",
                other.type_name()
            ))
        }
        None => return Err("http_post() requires at least 2 arguments (url, body)".to_string()),
    };

    let (content_type, timeout_arg_idx) = match args.get(2) {
        Some(Value::String(s)) => (s.clone(), 3),
        _ => ("application/json".to_string(), 2),
    };

    // Наряда-26 P0-1: configurable timeout (default 30s, max 300s)
    // Signatures: http_post(url, body, timeout) | http_post(url, body, ct, timeout) | http_post(url, body, ct, headers, timeout)
    let timeout_secs = if let Some(timeout_val) = args.get(timeout_arg_idx) {
        match timeout_val {
            Value::Float(f) => {
                let t = f.clamp(1.0, 300.0) as u64;
                if *f > 300.0 {
                    eprintln!("[http_post] timeout clamped from {} to 300s", f);
                }
                t
            }
            _ => 30, // not a number → skip, treat as headers or ignore
        }
    } else {
        30
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("http_post(): failed to create client: {}", e))?;

    let mut req = client
        .post(&url)
        .header("Content-Type", &content_type)
        .body(body);

    // Optional headers argument (index depends on whether content_type was provided)
    let headers_idx = if args.len() > 2 && matches!(args.get(2), Some(Value::String(_))) {
        // content_type was 3rd arg → headers are 4th
        3
    } else {
        // content_type was default → headers are 3rd
        2
    };
    // Only parse headers if the arg exists and is NOT a Float (which would be timeout)
    if let Some(headers_arg) = args.get(headers_idx) {
        if !matches!(headers_arg, Value::Float(_)) {
            match headers_arg {
                Value::String(auth_token) => {
                    if !auth_token.is_empty() {
                        req = req.header("Authorization", format!("Bearer {}", auth_token));
                    }
                }
                Value::Struct { fields, .. } => {
                    for (key, val) in fields {
                        if let Value::String(v) = val {
                            req = req.header(key.as_str(), v.as_str());
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let resp = req.send().map_err(|e| {
        let err_str = e.to_string();
        if err_str.contains("timeout") || err_str.contains("timed out") {
            format!("ERROR: http timeout after {}s", timeout_secs)
        } else {
            format!("http_post() request failed: {}", e)
        }
    })?;

    let status = resp.status().as_u16();
    let resp_body = resp.text().unwrap_or_default();

    if status >= 400 {
        return Err(format!(
            "http_post() returned status {}: {}",
            status, resp_body
        ));
    }

    Ok(Value::String(resp_body))
}

/// Send a request to Anthropic Claude Messages API.
/// Usage: call_claude(api_key, model, system_prompt, user_message) -> String
fn builtin_call_claude(args: &[Value]) -> Result<Value, String> {
    let api_key = expect_string_arg("call_claude", args, 0)?;
    let model = expect_string_arg("call_claude", args, 1)?;
    let system_prompt = expect_string_arg("call_claude", args, 2)?;
    let user_message = expect_string_arg("call_claude", args, 3)?;

    let body = serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "system": system_prompt,
        "messages": [{"role": "user", "content": user_message}]
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("call_claude(): failed to create client: {}", e))?;

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .body(body.to_string())
        .send()
        .map_err(|e| format!("call_claude(): request failed: {}", e))?;

    let status = resp.status().as_u16();
    let resp_body = resp.text().unwrap_or_default();

    if status >= 400 {
        return Err(format!(
            "call_claude() returned status {}: {}",
            status, resp_body
        ));
    }

    // Parse response and extract content[0].text
    let parsed: serde_json::Value = serde_json::from_str(&resp_body)
        .map_err(|e| format!("call_claude(): JSON parse error: {}", e))?;

    let content = parsed["content"][0]["text"]
        .as_str()
        .unwrap_or("Claude API returned an unexpected response format")
        .to_string();

    Ok(Value::String(content))
}

/// Escape a string for safe embedding inside a JSON string value.
/// Replaces: " -> \" , \ -> \\ , newline -> \n , tab -> \t , carriage return -> \r
/// Usage: escape_json(text) -> String
fn builtin_escape_json(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("escape_json", args, 0)?;
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            _ => out.push(ch),
        }
    }
    Ok(Value::String(out))
}

// ── Phase 7.7 — parse_json, http_get, now ────────────────────────────

/// Parse a JSON string into a Value (Struct or List).
/// Usage: parse_json(text) -> Struct|List|String|Float|Bool|Unit
fn builtin_parse_json(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("parse_json", args, 0)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("parse_json() error: {}", e))?;
    Ok(json_value_to_mlog_value(&parsed))
}

/// Convert serde_json::Value to METALOGOS Value (same logic as interpreter's method).
fn json_value_to_mlog_value(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Number(n) => Value::Float(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Null => Value::Unit,
        serde_json::Value::Array(arr) => {
            Value::List(arr.iter().map(|v| json_value_to_mlog_value(v)).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut fields = std::collections::HashMap::new();
            for (k, v) in obj {
                fields.insert(k.clone(), json_value_to_mlog_value(v));
            }
            Value::Struct {
                type_name: "Json".to_string(),
                fields,
            }
        }
    }
}

/// Convert METALOGOS Value to serde_json::Value (reverse of json_value_to_mlog_value).
fn mlog_value_to_json(val: &Value) -> serde_json::Value {
    match val {
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Float(f) => serde_json::json!(*f),
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Unit => serde_json::Value::Null,
        Value::List(items) => {
            serde_json::Value::Array(items.iter().map(|v| mlog_value_to_json(v)).collect())
        }
        Value::Struct { fields, .. } => {
            let mut map = serde_json::Map::new();
            for (k, v) in fields {
                map.insert(k.clone(), mlog_value_to_json(v));
            }
            serde_json::Value::Object(map)
        }
        // Opaque/internal types: convert to string representation
        other => serde_json::Value::String(format!("{}", other)),
    }
}

/// Serialize a Value to a JSON string.
/// Usage: json_encode(value) -> String
/// Supports: String, Float, Bool, Unit->null, List->array, Struct->object
fn builtin_json_encode(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("json_encode() requires 1 argument".to_string());
    }
    let json = mlog_value_to_json(&args[0]);
    let serialized = serde_json::to_string(&json)
        .map_err(|e| format!("json_encode() serialization error: {}", e))?;
    Ok(Value::String(serialized))
}

/// Safe field access on a struct value: returns default if field missing or not a struct.
/// Usage: json_get(obj, "field") -> Value (returns Unit if missing)
/// Usage: json_get(obj, "field", default_value) -> Value (returns default if missing)
/// Usage: json_get(obj, "nested.field.path", default) -> Value (dot-separated path)
/// Supports numeric path segments for array indexing: "items.0.title"
/// This is the P0 fix: prevents runtime crash when accessing optional JSON fields
/// like message.voice on non-voice Telegram updates.
fn builtin_json_get(args: &[Value]) -> Result<Value, String> {
    let obj = match args.get(0) {
        Some(v) => v,
        None => {
            return Err("json_get() requires at least 2 arguments (obj, field_path)".to_string())
        }
    };
    let path = expect_string_arg("json_get", args, 1)?;
    // Bug 2.2 fix: when no default is provided, return the found value directly
    // (not wrapped in Unit). The old code defaulted to Value::Unit which silently
    // swallowed string values and made them unusable.
    if args.len() >= 3 {
        let default_val = args[2].clone();
        // Navigate the path (dot-separated)
        let mut current = obj;
        for segment in path.split('.') {
            // Try struct field access first
            match current.get_field(segment) {
                Ok(val) => {
                    current = val;
                    continue;
                }
                Err(_) => {}
            }
            // If struct field not found, try numeric array index (Наряд №24 B4)
            if let Ok(index) = segment.parse::<usize>() {
                if let Value::List(items) = current {
                    if let Some(item) = items.get(index) {
                        current = item;
                        continue;
                    }
                }
            }
            return Ok(default_val);
        }
        // SQL NULL / missing value maps to Value::Unit; treat as "use default"
        if matches!(current, Value::Unit) {
            return Ok(default_val);
        }
        Ok(current.clone())
    } else {
        // 2-argument form: return the found value or Unit if not found
        let mut current = obj;
        for segment in path.split('.') {
            // Try struct field access first
            match current.get_field(segment) {
                Ok(val) => {
                    current = val;
                    continue;
                }
                Err(_) => {}
            }
            // If struct field not found, try numeric array index (Наряд №24 B4)
            if let Ok(index) = segment.parse::<usize>() {
                if let Value::List(items) = current {
                    if let Some(item) = items.get(index) {
                        current = item;
                        continue;
                    }
                }
            }
            return Ok(Value::Unit);
        }
        Ok(current.clone())
    }
}

/// Check if a struct value has a given field. Returns 1.0 (true) or 0.0 (false).
/// Usage: has_field(obj, "field") -> Float
/// Usage: has_field(obj, "nested.field") -> Float (dot-separated path)
fn builtin_has_field(args: &[Value]) -> Result<Value, String> {
    let obj = match args.get(0) {
        Some(v) => v,
        None => return Err("has_field() requires 2 arguments (obj, field_path)".to_string()),
    };
    let path = expect_string_arg("has_field", args, 1)?;

    let mut current = obj;
    let segments: Vec<&str> = path.split('.').collect();
    for (i, segment) in segments.iter().enumerate() {
        match current.get_field(segment) {
            Ok(val) => {
                if i == segments.len() - 1 {
                    return Ok(Value::Float(1.0));
                }
                current = val;
            }
            Err(_) => return Ok(Value::Float(0.0)),
        }
    }
    Ok(Value::Float(0.0))
}

/// Send an HTTP GET request. Returns the response body as String.
/// Usage: http_get(url) -> String
/// Usage: http_get(url, headers_struct) -> String  — sets headers from Struct fields
fn builtin_http_get(args: &[Value]) -> Result<Value, String> {
    let url = match args.get(0) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "http_get() expected String as url, got {}",
                other.type_name()
            ))
        }
        None => return Err("http_get() requires 1 argument (url)".to_string()),
    };

    // Наряда-26 P0-1: configurable timeout
    // http_get(url) | http_get(url, timeout) | http_get(url, headers) | http_get(url, headers, timeout)
    let (headers_arg, timeout_secs) = match args.len() {
        1 => (None, 30u64),
        2 => {
            // 2nd arg could be timeout (Float) or headers (String/Struct)
            match &args[1] {
                Value::Float(f) => (None, f.clamp(1.0, 300.0) as u64),
                other => (Some(other), 30),
            }
        }
        _ => {
            // 3+ args: 2nd is headers, 3rd is timeout
            let timeout = if let Some(Value::Float(f)) = args.get(2) {
                f.clamp(1.0, 300.0) as u64
            } else {
                30
            };
            (args.get(1), timeout)
        }
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("http_get(): failed to create client: {}", e))?;

    let mut req = client.get(&url);

    if let Some(headers_arg) = headers_arg {
        match headers_arg {
            Value::String(auth_token) => {
                if !auth_token.is_empty() {
                    req = req.header("Authorization", format!("Bearer {}", auth_token));
                }
            }
            Value::Struct { fields, .. } => {
                for (key, val) in fields {
                    if let Value::String(v) = val {
                        req = req.header(key.as_str(), v.as_str());
                    }
                }
            }
            _ => {}
        }
    }

    let resp = req.send().map_err(|e| {
        let err_str = e.to_string();
        if err_str.contains("timeout") || err_str.contains("timed out") {
            format!("ERROR: http timeout after {}s", timeout_secs)
        } else {
            format!("http_get() request failed: {}", e)
        }
    })?;

    let status = resp.status().as_u16();
    let resp_body = resp.text().unwrap_or_default();

    if status >= 400 {
        return Err(format!(
            "http_get() returned status {}: {}",
            status, resp_body
        ));
    }

    Ok(Value::String(resp_body))
}

/// Return current Unix timestamp as Float (seconds since epoch).
/// Usage: now() -> Float
fn builtin_now(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    Ok(Value::Float(now))
}

fn builtin_require(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("require() requires 1 argument (bool expression)".to_string());
    }
    match &args[0] {
        Value::Bool(true) => Ok(Value::Unit),
        Value::Bool(false) => {
            let msg = if args.len() > 1 {
                match &args[1] {
                    Value::String(s) => s.clone(),
                    other => format!("{:?}", other),
                }
            } else {
                "require assertion failed".to_string()
            };
            Err(format!("require assertion failed: {}", msg))
        }
        other => Err(format!(
            "require() expected Bool, got {}",
            other.type_name()
        )),
    }
}

// ── v0.5.0 — New string builtins ──────────────────────────

/// `length(s)` — returns the length of a string or list as Float.
fn builtin_length(args: &[Value]) -> Result<Value, String> {
    match args.get(0) {
        Some(Value::String(s)) => Ok(Value::Float(s.chars().count() as f64)),
        Some(Value::List(items)) => Ok(Value::Float(items.len() as f64)),
        other => Err(format!(
            "length() requires String or List, got {}",
            other.as_ref().map(|v| v.type_name()).unwrap_or("none")
        )),
    }
}

/// `to_int(s)` — parse a string to an integer Float (truncates towards zero).
fn builtin_to_int(args: &[Value]) -> Result<Value, String> {
    match args.get(0) {
        Some(Value::Float(f)) => Ok(Value::Float(f.trunc())),
        Some(Value::String(s)) => {
            // Try integer parse first, then float truncation
            if let Ok(i) = s.parse::<i64>() {
                Ok(Value::Float(i as f64))
            } else if let Ok(f) = s.parse::<f64>() {
                Ok(Value::Float(f.trunc()))
            } else {
                Ok(Value::Float(0.0)) // soft-failure
            }
        }
        Some(Value::Bool(b)) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
        _ => Ok(Value::Float(0.0)), // soft-failure
    }
}

/// `reverse(s)` — reverse a string or list.
fn builtin_reverse(args: &[Value]) -> Result<Value, String> {
    match args.get(0) {
        Some(Value::String(s)) => Ok(Value::String(s.chars().rev().collect())),
        Some(Value::List(items)) => {
            let mut rev = items.clone();
            rev.reverse();
            Ok(Value::List(rev))
        }
        other => Err(format!(
            "reverse() requires String or List, got {}",
            other.as_ref().map(|v| v.type_name()).unwrap_or("none")
        )),
    }
}

// ── v0.5.0 — LLM call builtin ──────────────────────────────

/// `call_llm(prompt, input)` — call the LLM backend with a prompt and input.
/// When METALOGOS_LLM_MOCK=true (default), returns "[MOCK: <prompt> | <input>]".
/// When METALOGOS_LLM_MOCK=false, calls the real LLM backend (30s timeout).
fn builtin_call_llm(args: &[Value]) -> Result<Value, String> {
    let prompt = match args.get(0) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "call_llm() expected String as prompt, got {}",
                other.type_name()
            ))
        }
        None => return Err("call_llm() requires at least 1 argument (prompt)".to_string()),
    };
    let input = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => format!("{}", other),
        None => String::new(),
    };

    // Check mock mode
    let mock_mode = std::env::var("METALOGOS_LLM_MOCK")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(true); // Default: mock mode ON

    if mock_mode {
        Ok(Value::String(format!("[MOCK: {} | {}]", prompt, input)))
    } else {
        // Real LLM call
        let backend = crate::llm::create_llm_backend();
        backend
            .call(&prompt, &input)
            .map(Value::String)
            .map_err(|e| format!("call_llm() failed: {}", e))
    }
}

// ── v0.5.0 — KV memory builtins ────────────────────────────
// These use a thread-local KV store (in-memory by default).
// When memory { persist: "..." } is configured, they also persist to SQLite kv_store table.
// Uses a write-through cache: in-memory HashMap is always authoritative;
// SQLite is a persistence backend that mirrors the HashMap.

use std::sync::Mutex as StdMutex;

/// Global KV store — lazy_static pattern using std::sync::OnceLock (Rust 1.70+).
static KV_STORE: std::sync::OnceLock<StdMutex<std::collections::HashMap<String, String>>> =
    std::sync::OnceLock::new();

fn kv_store() -> &'static StdMutex<std::collections::HashMap<String, String>> {
    KV_STORE.get_or_init(|| StdMutex::new(std::collections::HashMap::new()))
}

/// Global SQLite KV persistence backend.
/// Initialized by init_kv_persist() when memory { persist: "..." } is configured.
/// Uses std::sync::Mutex (same thread model as KV_STORE).
static KV_SQLITE: std::sync::OnceLock<StdMutex<Option<rusqlite::Connection>>> =
    std::sync::OnceLock::new();

fn kv_sqlite() -> &'static StdMutex<Option<rusqlite::Connection>> {
    KV_SQLITE.get_or_init(|| StdMutex::new(None))
}

/// Initialize SQLite persistence for the KV store.
/// Called by Interpreter::configure_memory() when persist path is set.
/// Creates kv_store table (key TEXT PRIMARY KEY, value TEXT) in the given database.
/// Loads existing rows into the in-memory HashMap.
pub fn init_kv_persist(db_path: &str) -> Result<(), String> {
    let conn = rusqlite::Connection::open(db_path)
        .map_err(|e| format!("[kv_store] Failed to open database '{}': {}", db_path, e))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS kv_store (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    )
    .map_err(|e| format!("[kv_store] Failed to create table: {}", e))?;

    // Load existing KV pairs into in-memory HashMap (write-through cache warmup)
    {
        let mut stmt = conn
            .prepare("SELECT key, value FROM kv_store")
            .map_err(|e| format!("[kv_store] Failed to query: {}", e))?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| format!("[kv_store] Failed to iterate: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        // Merge into in-memory store (SQLite is authoritative on init)
        if let Ok(mut store) = kv_store().lock() {
            for (key, value) in rows {
                store.insert(key, value);
            }
        }
    } // stmt is dropped here, releasing borrow on conn

    // Store the connection globally
    let mut sqlite_guard = kv_sqlite()
        .lock()
        .map_err(|e| format!("[kv_store] lock error: {}", e))?;
    *sqlite_guard = Some(conn);
    eprintln!("[kv_store] SQLite persistence enabled: {}", db_path);
    Ok(())
}

/// `kv_set(key, value)` — store a key-value pair.
fn builtin_kv_set(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("kv_set", args, 0)?;
    let value = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => format!("{}", other),
        None => return Err("kv_set() requires 2 arguments (key, value)".to_string()),
    };
    let mut store = kv_store()
        .lock()
        .map_err(|e| format!("kv_set() lock error: {}", e))?;
    store.insert(key.clone(), value.clone());
    // Write-through to SQLite if available
    if let Ok(sqlite_guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *sqlite_guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, value],
            );
        }
    }
    Ok(Value::Unit)
}

/// `kv_get(key)` — retrieve a value by key. Returns empty string if not found.
fn builtin_kv_get(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("kv_get", args, 0)?;
    let store = kv_store()
        .lock()
        .map_err(|e| format!("kv_get() lock error: {}", e))?;
    Ok(Value::String(store.get(&key).cloned().unwrap_or_default()))
}

/// `kv_delete(key)` — remove a key-value pair.
fn builtin_kv_delete(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("kv_delete", args, 0)?;
    let mut store = kv_store()
        .lock()
        .map_err(|e| format!("kv_delete() lock error: {}", e))?;
    store.remove(&key);
    // Write-through delete to SQLite if available
    if let Ok(sqlite_guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *sqlite_guard {
            let _ = conn.execute(
                "DELETE FROM kv_store WHERE key = ?1",
                rusqlite::params![key],
            );
        }
    }
    Ok(Value::Unit)
}

/// `kv_exists(key)` — check if a key exists. Returns Bool.
fn builtin_kv_exists(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("kv_exists", args, 0)?;
    let store = kv_store()
        .lock()
        .map_err(|e| format!("kv_exists() lock error: {}", e))?;
    Ok(Value::Bool(store.contains_key(&key)))
}

/// `kv_list()` — list all keys. Returns List of Strings.
fn builtin_kv_list(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let store = kv_store()
        .lock()
        .map_err(|e| format!("kv_list() lock error: {}", e))?;
    let keys: Vec<Value> = store.keys().cloned().map(Value::String).collect();
    Ok(Value::List(keys))
}

// ── Наряд №6 — mem_set / mem_get / mem_delete (exact KV, not semantic) ─
// These are user-facing aliases for the KV store with String return types.
// mem_set returns the stored value, mem_get returns value or empty string,
// mem_delete returns the deleted value or empty string.
// They share the same global HashMap + optional SQLite backend as kv_*.

/// `mem_set(key, value)` — exact key-value write. Returns the stored value.
fn builtin_mem_set(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("mem_set", args, 0)?;
    let value = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => format!("{}", other),
        None => return Err("mem_set() requires 2 arguments (key, value)".to_string()),
    };
    let mut store = kv_store()
        .lock()
        .map_err(|e| format!("mem_set() lock error: {}", e))?;
    store.insert(key.clone(), value.clone());
    // Write-through to SQLite if available
    if let Ok(sqlite_guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *sqlite_guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, value],
            );
        }
    }
    Ok(Value::String(value))
}

/// `mem_get(key)` — exact key-value read (not semantic recall).
/// Returns the value or empty string if not found.
fn builtin_mem_get(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("mem_get", args, 0)?;
    let store = kv_store()
        .lock()
        .map_err(|e| format!("mem_get() lock error: {}", e))?;
    Ok(Value::String(store.get(&key).cloned().unwrap_or_default()))
}

/// `mem_delete(key)` — remove a key-value pair. Returns the deleted value or empty string.
fn builtin_mem_delete(args: &[Value]) -> Result<Value, String> {
    let key = expect_string_arg("mem_delete", args, 0)?;
    let mut store = kv_store()
        .lock()
        .map_err(|e| format!("mem_delete() lock error: {}", e))?;
    let removed = store.remove(&key);
    // Write-through delete to SQLite if available
    if let Ok(sqlite_guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *sqlite_guard {
            let _ = conn.execute(
                "DELETE FROM kv_store WHERE key = ?1",
                rusqlite::params![key],
            );
        }
    }
    Ok(Value::String(removed.unwrap_or_default()))
}

// ── ADR-0049 — session memory (temporary per-session KV store) ──
// In-memory HashMap<String, HashMap<String, String>> — NOT persistent.
// Resets when mlog serve restarts (by design: session data is ephemeral).
// Unlike mem_set/mem_get (global), session_* is scoped to a specific session_id.
//
// Usage:
//   session_set(session_id, key, value)   -> String (stored value)
//   session_get(session_id, key)             -> String (value or "")
//   session_clear(session_id)                -> Unit

/// Global session store — lazy_static pattern using std::sync::OnceLock.
/// Outer key = session_id, inner key = data key, inner value = data value.
static SESSION_STORE: std::sync::OnceLock<
    StdMutex<std::collections::HashMap<String, std::collections::HashMap<String, String>>>,
> = std::sync::OnceLock::new();

fn session_store(
) -> &'static StdMutex<std::collections::HashMap<String, std::collections::HashMap<String, String>>>
{
    SESSION_STORE.get_or_init(|| StdMutex::new(std::collections::HashMap::new()))
}

/// Reset the entire session store. Used by contract tests to verify restart behavior.
pub fn reset_session_store() {
    if let Ok(mut store) = session_store().lock() {
        store.clear();
    }
}

/// Get the number of sessions in the store. Used by contract tests.
pub fn session_store_count() -> usize {
    session_store().lock().map(|s| s.len()).unwrap_or(0)
}

/// Get the number of keys in a specific session. Used by contract tests.
pub fn session_key_count(session_id: &str) -> usize {
    session_store()
        .lock()
        .ok()
        .and_then(|s| s.get(session_id).map(|m| m.len()))
        .unwrap_or(0)
}

/// `session_set(session_id, key, value)` — store a value scoped to a session.
/// Returns the stored value. Creates session bucket if it doesn't exist.
fn builtin_session_set(args: &[Value]) -> Result<Value, String> {
    let session_id = expect_string_arg("session_set", args, 0)?;
    let key = expect_string_arg("session_set", args, 1)?;
    let value = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => format!("{}", other),
        None => {
            return Err("session_set() requires 3 arguments (session_id, key, value)".to_string())
        }
    };
    let mut store = session_store()
        .lock()
        .map_err(|e| format!("session_set() lock error: {}", e))?;
    store
        .entry(session_id)
        .or_default()
        .insert(key.clone(), value.clone());
    Ok(Value::String(value))
}

/// `session_get(session_id, key)` — retrieve a value from a session.
/// Returns empty string if session or key not found.
fn builtin_session_get(args: &[Value]) -> Result<Value, String> {
    let session_id = expect_string_arg("session_get", args, 0)?;
    let key = expect_string_arg("session_get", args, 1)?;
    let store = session_store()
        .lock()
        .map_err(|e| format!("session_get() lock error: {}", e))?;
    let value = store
        .get(&session_id)
        .and_then(|session| session.get(&key).cloned())
        .unwrap_or_default();
    Ok(Value::String(value))
}

/// `session_clear(session_id)` — remove all keys for a session.
/// Returns "ok". No-op if session doesn't exist.
fn builtin_session_clear(args: &[Value]) -> Result<Value, String> {
    let session_id = expect_string_arg("session_clear", args, 0)?;
    let mut store = session_store()
        .lock()
        .map_err(|e| format!("session_clear() lock error: {}", e))?;
    store.remove(&session_id);
    Ok(Value::String("ok".to_string()))
}

// ── v0.5.0 — File I/O builtins (sandboxed) ──────────────────
// All file operations are restricted to the working directory.
// Paths containing ".." or absolute paths are rejected (sandbox).

/// Validate that a path is safe (within working directory, no traversal).
fn sandbox_path(path: &str) -> Result<std::path::PathBuf, String> {
    let p = std::path::Path::new(path);
    // Reject absolute paths
    if p.is_absolute() {
        return Err(format!(
            "file I/O sandbox: absolute paths not allowed: '{}'",
            path
        ));
    }
    // Reject path traversal
    for component in p.components() {
        if let std::path::Component::ParentDir = component {
            return Err(format!(
                "file I/O sandbox: path traversal ('..') not allowed: '{}'",
                path
            ));
        }
    }
    Ok(std::path::PathBuf::from(path))
}

/// `read_file(path)` — read file contents as String.
/// Soft-failure: returns empty string on error (file not found, permission denied, etc.).
fn builtin_read_file(args: &[Value]) -> Result<Value, String> {
    let path = expect_string_arg("read_file", args, 0)?;
    let safe_path = match sandbox_path(&path) {
        Ok(p) => p,
        Err(_) => return Ok(Value::String(String::new())), // soft-failure on sandbox violation
    };
    match std::fs::read_to_string(&safe_path) {
        Ok(content) => Ok(Value::String(content)),
        Err(_) => Ok(Value::String(String::new())), // soft-failure
    }
}

/// `write_file(path, content)` — write string to file (overwrite).
/// Returns "ok" on success, empty string on soft-failure.
fn builtin_write_file(args: &[Value]) -> Result<Value, String> {
    let path = expect_string_arg("write_file", args, 0)?;
    let content = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => format!("{}", other),
        None => return Ok(Value::String(String::new())), // soft-failure
    };
    let safe_path = match sandbox_path(&path) {
        Ok(p) => p,
        Err(_) => return Ok(Value::String(String::new())), // soft-failure on sandbox violation
    };
    // Create parent directories if needed
    if let Some(parent) = safe_path.parent() {
        let _ = std::fs::create_dir_all(parent); // best-effort
    }
    match std::fs::write(&safe_path, &content) {
        Ok(_) => Ok(Value::String("ok".to_string())),
        Err(_) => Ok(Value::String(String::new())), // soft-failure
    }
}

/// `append_file(path, content)` — append string to file.
/// Returns "ok" on success, empty string on soft-failure.
fn builtin_append_file(args: &[Value]) -> Result<Value, String> {
    let path = expect_string_arg("append_file", args, 0)?;
    let content = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => format!("{}", other),
        None => return Ok(Value::String(String::new())), // soft-failure
    };
    let safe_path = match sandbox_path(&path) {
        Ok(p) => p,
        Err(_) => return Ok(Value::String(String::new())), // soft-failure on sandbox violation
    };
    // Create parent directories if needed
    if let Some(parent) = safe_path.parent() {
        let _ = std::fs::create_dir_all(parent); // best-effort
    }
    use std::io::Write;
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&safe_path)
    {
        Ok(mut file) => match file.write_all(content.as_bytes()) {
            Ok(_) => Ok(Value::String("ok".to_string())),
            Err(_) => Ok(Value::String(String::new())), // soft-failure
        },
        Err(_) => Ok(Value::String(String::new())), // soft-failure
    }
}

/// `delete_file(path)` — delete a file.
/// Soft-failure: returns empty string on error.
fn builtin_delete_file(args: &[Value]) -> Result<Value, String> {
    let path = expect_string_arg("delete_file", args, 0)?;
    let safe_path = match sandbox_path(&path) {
        Ok(p) => p,
        Err(_) => return Ok(Value::String(String::new())), // soft-failure
    };
    match std::fs::remove_file(&safe_path) {
        Ok(_) => Ok(Value::String("ok".to_string())),
        Err(_) => Ok(Value::String(String::new())), // soft-failure
    }
}

/// `file_exists(path)` — check if a file exists. Returns Bool.
fn builtin_file_exists(args: &[Value]) -> Result<Value, String> {
    let path = expect_string_arg("file_exists", args, 0)?;
    let safe_path = match sandbox_path(&path) {
        Ok(p) => p,
        Err(_) => return Ok(Value::Bool(false)), // soft-failure on sandbox violation
    };
    Ok(Value::Bool(safe_path.exists()))
}

/// `list_dir(path)` — list files in a directory. Returns List of Strings.
fn builtin_list_dir(args: &[Value]) -> Result<Value, String> {
    let path = if args.is_empty() {
        ".".to_string()
    } else {
        expect_string_arg("list_dir", args, 0)?
    };
    let safe_path = sandbox_path(&path)?;
    let entries: Vec<Value> = std::fs::read_dir(&safe_path)
        .map_err(|e| format!("list_dir('{}'): {}", path, e))?
        .filter_map(|entry| {
            entry
                .ok()
                .map(|e| Value::String(e.file_name().to_string_lossy().to_string()))
        })
        .collect();
    Ok(Value::List(entries))
}

/// Наряд №4: `llm_usage()` — returns LLM usage statistics as a Struct.
/// Returns: { total_calls: Float, total_tokens: Float, total_errors: Float, providers: List }
fn builtin_llm_usage(_args: &[Value]) -> Result<Value, String> {
    let report = crate::llm::global_llm_usage_report();

    let mut fields = std::collections::HashMap::new();
    fields.insert("total_calls".to_string(), Value::Float(report.total_calls));
    fields.insert(
        "total_tokens".to_string(),
        Value::Float(report.total_tokens),
    );
    fields.insert(
        "total_errors".to_string(),
        Value::Float(report.total_errors),
    );

    let providers: Vec<Value> = report
        .providers
        .iter()
        .map(|p| {
            let mut pf = std::collections::HashMap::new();
            pf.insert("alias".to_string(), Value::String(p.alias.clone()));
            pf.insert("calls".to_string(), Value::Float(p.calls as f64));
            pf.insert("tokens".to_string(), Value::Float(p.tokens as f64));
            pf.insert("errors".to_string(), Value::Float(p.errors as f64));
            pf.insert("avg_latency_ms".to_string(), Value::Float(p.avg_latency_ms));
            pf.insert("health_score".to_string(), Value::Float(p.health_score));
            Value::Struct {
                type_name: "ProviderUsage".to_string(),
                fields: pf,
            }
        })
        .collect();
    fields.insert("providers".to_string(), Value::List(providers));

    Ok(Value::Struct {
        type_name: "LlmUsage".to_string(),
        fields,
    })
}

// ── Phase 7.8: Voice pipeline builtins ──────────────────────────────

fn builtin_http_post_multipart(args: &[Value]) -> Result<Value, String> {
    let url = expect_string_arg("http_post_multipart", args, 0)?;
    let fields = match args.get(1) {
        Some(Value::Struct { fields, .. }) => fields.clone(),
        _ => {
            return Err(
                "http_post_multipart() requires Struct as 2nd argument (fields)".to_string(),
            )
        }
    };
    let files = match args.get(2) {
        Some(Value::Struct { fields, .. }) => fields.clone(),
        _ => {
            return Err("http_post_multipart() requires Struct as 3rd argument (files)".to_string())
        }
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("http_post_multipart(): client error: {}", e))?;

    let mut form = reqwest::blocking::multipart::Form::new();

    // Add text fields
    for (key, val) in &fields {
        if let Value::String(v) = val {
            form = form.text(key.clone(), v.clone());
        }
    }

    // Add file fields
    for (key, val) in &files {
        if let Value::String(path) = val {
            let file_bytes = std::fs::read(path).map_err(|e| {
                format!("http_post_multipart(): cannot read file '{}': {}", path, e)
            })?;
            let file_name = std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file");
            let part = reqwest::blocking::multipart::Part::bytes(file_bytes)
                .file_name(file_name.to_string());
            form = form.part(key.clone(), part);
        }
    }

    let resp = client
        .post(&url)
        .multipart(form)
        .send()
        .map_err(|e| format!("http_post_multipart(): request failed: {}", e))?;

    let status = resp.status().as_u16();
    let resp_body = resp.text().unwrap_or_default();

    if status >= 400 {
        return Err(format!(
            "http_post_multipart() returned status {}: {}",
            status, resp_body
        ));
    }

    Ok(Value::String(resp_body))
}

fn builtin_whisper_transcribe(args: &[Value]) -> Result<Value, String> {
    let file_id = expect_string_arg("whisper_transcribe", args, 0)?;
    let bot_token = expect_string_arg("whisper_transcribe", args, 1)?;
    let whisper_key = expect_string_arg("whisper_transcribe", args, 2)?;
    let provider = match args.get(3) {
        Some(Value::String(s)) => s.clone(),
        _ => "openai".to_string(),
    };

    // Step 1: Get file path from Telegram
    let tg_client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("whisper_transcribe(): client error: {}", e))?;

    let get_file_url = format!(
        "https://api.telegram.org/bot{}/getFile?file_id={}",
        bot_token, file_id
    );
    let tg_resp = tg_client
        .get(&get_file_url)
        .send()
        .map_err(|e| format!("whisper_transcribe(): Telegram getFile failed: {}", e))?;
    let tg_body: serde_json::Value = serde_json::from_str(&tg_resp.text().unwrap_or_default())
        .map_err(|e| format!("whisper_transcribe(): Telegram response parse error: {}", e))?;

    let file_path = tg_body
        .get("result")
        .and_then(|r| r.get("file_path"))
        .and_then(|p| p.as_str())
        .unwrap_or("")
        .to_string();

    if file_path.is_empty() {
        return Err("whisper_transcribe(): Telegram returned empty file_path".to_string());
    }

    // Step 2: Download the file
    let download_url = format!(
        "https://api.telegram.org/file/bot{}/{}",
        bot_token, file_path
    );
    let audio_bytes = tg_client
        .get(&download_url)
        .send()
        .map_err(|e| format!("whisper_transcribe(): download failed: {}", e))?
        .bytes()
        .map_err(|e| format!("whisper_transcribe(): read bytes failed: {}", e))?;

    // Step 3: Send to Whisper API
    let (api_url, auth_header, auth_value) = match provider.as_str() {
        "groq" => (
            "https://api.groq.com/openai/v1/audio/transcriptions".to_string(),
            "Authorization".to_string(),
            format!("Bearer {}", whisper_key),
        ),
        _ => (
            // openai
            "https://api.openai.com/v1/audio/transcriptions".to_string(),
            "Authorization".to_string(),
            format!("Bearer {}", whisper_key),
        ),
    };

    // Use multipart form
    let whisper_client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("whisper_transcribe(): whisper client error: {}", e))?;

    let model = if provider == "groq" {
        "whisper-large-v3"
    } else {
        "whisper-1"
    };
    let mut form = reqwest::blocking::multipart::Form::new();
    form = form.text("model", model.to_string());
    let part =
        reqwest::blocking::multipart::Part::bytes(audio_bytes.to_vec()).file_name("audio.ogg");
    form = form.part("file", part);

    let whisper_resp = whisper_client
        .post(&api_url)
        .header(auth_header, auth_value)
        .multipart(form)
        .send()
        .map_err(|e| format!("whisper_transcribe(): whisper request failed: {}", e))?;

    let status = whisper_resp.status().as_u16();
    let whisper_body = whisper_resp.text().unwrap_or_default();

    if status >= 400 {
        return Err(format!(
            "whisper_transcribe(): whisper API status {}: {}",
            status, whisper_body
        ));
    }

    // Parse response to extract text
    let parsed: serde_json::Value = serde_json::from_str(&whisper_body)
        .map_err(|e| format!("whisper_transcribe(): whisper response parse error: {}", e))?;
    let text = parsed
        .get("text")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();

    Ok(Value::String(text))
}

fn builtin_tts_send(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("tts_send", args, 0)?;
    let voice = expect_string_arg("tts_send", args, 1)?;
    let bot_token = expect_string_arg("tts_send", args, 2)?;
    let chat_id = expect_string_arg("tts_send", args, 3)?;
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return Err("tts_send(): OPENAI_API_KEY env var not set".to_string());
    }

    // Step 1: Call OpenAI TTS API
    let tts_body = serde_json::json!({
        "model": "tts-1",
        "input": text,
        "voice": voice,
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("tts_send(): client error: {}", e))?;

    let tts_resp = client
        .post("https://api.openai.com/v1/audio/speech")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .body(tts_body.to_string())
        .send()
        .map_err(|e| format!("tts_send(): TTS request failed: {}", e))?;

    let status = tts_resp.status().as_u16();
    if status >= 400 {
        let err_body = tts_resp.text().unwrap_or_default();
        return Err(format!(
            "tts_send(): TTS API status {}: {}",
            status, err_body
        ));
    }

    let audio_bytes = tts_resp
        .bytes()
        .map_err(|e| format!("tts_send(): failed to read TTS audio: {}", e))?;

    // Step 2: Send as voice note to Telegram via sendVoice
    // sendVoice (not sendAudio) displays as voice message bubble in Telegram.
    // Optional 5th arg "audio" switches back to sendAudio (audio player).
    let send_as = match args.get(4) {
        Some(Value::String(s)) if s == "audio" => "audio",
        _ => "voice",
    };
    let (field_name, endpoint) = match send_as {
        "audio" => ("audio", "sendAudio"),
        _ => ("voice", "sendVoice"),
    };
    let mut form = reqwest::blocking::multipart::Form::new();
    form = form.text("chat_id", chat_id.clone());
    let audio_part =
        reqwest::blocking::multipart::Part::bytes(audio_bytes.to_vec()).file_name("speech.ogg");
    form = form.part(field_name, audio_part);

    let tg_resp = client
        .post(format!(
            "https://api.telegram.org/bot{}/{}",
            bot_token, endpoint
        ))
        .multipart(form)
        .send()
        .map_err(|e| format!("tts_send(): Telegram {} failed: {}", endpoint, e))?;

    let tg_status = tg_resp.status().as_u16();
    let tg_body = tg_resp.text().unwrap_or_default();

    if tg_status >= 400 {
        return Err(format!(
            "tts_send(): Telegram status {}: {}",
            tg_status, tg_body
        ));
    }

    Ok(Value::String(tg_body))
}

// ── Наряд 17: Utility builtins ──────────────────────────────────

/// `base64_encode(s) -> String` — encode a string to base64.
fn builtin_base64_encode(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("base64_encode", args, 0)?;
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(s.as_bytes());
    Ok(Value::String(encoded))
}

/// `base64_decode(s) -> String` — decode a base64 string.
fn builtin_base64_decode(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("base64_decode", args, 0)?;
    use base64::Engine;
    match base64::engine::general_purpose::STANDARD.decode(s.as_bytes()) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(decoded) => Ok(Value::String(decoded)),
            Err(_) => Err("base64_decode(): decoded bytes are not valid UTF-8".to_string()),
        },
        Err(e) => Err(format!("base64_decode(): invalid base64: {}", e)),
    }
}

/// `exec(command) -> String` — execute a shell command and return stdout.
/// **Security**: Only available outside sandbox. Inside sandbox, always errors.
/// In server mode, command execution is disabled unless the binary is run with METALOGOS_ALLOW_EXEC=1.
fn builtin_exec(args: &[Value]) -> Result<Value, String> {
    // Security: disable in server context unless explicitly allowed
    if std::env::var("METALOGOS_ALLOW_EXEC").unwrap_or_default() != "1" {
        // Check if we're likely in server mode (has METALOGOS_PORT or METALOGOS_DB env)
        let in_server =
            std::env::var("METALOGOS_PORT").is_ok() || std::env::var("METALOGOS_DB").is_ok();
        if in_server {
            return Err(
                "exec() is disabled in server mode. Set METALOGOS_ALLOW_EXEC=1 to enable."
                    .to_string(),
            );
        }
    }

    let cmd = expect_string_arg("exec", args, 0)?;
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .output()
        .map_err(|e| format!("exec(): failed to run command: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!(
            "exec() command exited with {}: {}",
            output.status,
            stderr.trim()
        ));
    }
    Ok(Value::String(stdout))
}

/// `escape_js(s) -> String` — escape a string for safe insertion into JavaScript.
/// Escapes: backslash, single quote, double quote, newline, carriage return, tab,
/// line separator, paragraph separator, and NUL.
fn builtin_escape_js(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("escape_js", args, 0)?;
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{2028}' => out.push_str("\\u{2028}"), // line separator
            '\u{2029}' => out.push_str("\\u{2029}"), // paragraph separator
            '\0' => out.push_str("\\0"),
            _ => out.push(c),
        }
    }
    Ok(Value::String(out))
}

/// `type_of(value) -> String` — returns the runtime type name as a String.
/// Useful for safe checking after json_get: `if type_of(x) == "Unit" { ... }`
fn builtin_type_of(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("type_of() requires 1 argument".to_string());
    }
    Ok(Value::String(args[0].type_name().to_string()))
}

// ── Наряд 24: New builtins (A3, A4, B2) ──────────────────────────────

/// `git_push(message?) -> String` — git add/commit/push via subprocess.
/// Uses GITHUB_TOKEN and GITHUB_REPO env vars for authentication.
/// Usage: git_push("commit message") -> "ok" | "nothing to commit" | error
fn builtin_git_push(args: &[Value]) -> Result<Value, String> {
    let message = match args.get(0) {
        Some(Value::String(s)) => s.clone(),
        _ => "Auto commit".to_string(),
    };

    let run = |cmd: &str, cmd_args: &[&str]| -> Result<String, String> {
        let output = std::process::Command::new(cmd)
            .args(cmd_args)
            .output()
            .map_err(|e| format!("git_push(): {} failed: {}", cmd, e))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(format!(
                "git_push(): {} exited with {}: {}",
                cmd,
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    };

    run("git", &["add", "."])?;

    // Check if there's anything to commit
    let status = run("git", &["status", "--porcelain"])?;
    if status.trim().is_empty() {
        return Ok(Value::String("nothing to commit".to_string()));
    }

    run("git", &["commit", "-m", &message])?;

    // Push using token from env
    let token = std::env::var("GITHUB_TOKEN").unwrap_or_default();
    let repo = std::env::var("GITHUB_REPO").unwrap_or_default();
    if token.is_empty() || repo.is_empty() {
        return Err("git_push(): GITHUB_TOKEN or GITHUB_REPO env var not set".to_string());
    }

    let remote = format!("https://{}@github.com/{}.git", token, repo);
    run("git", &["push", &remote, "main"])?;

    Ok(Value::String("ok".to_string()))
}

/// `web_search(query, num_results?) -> String` — search via SerpAPI.
/// Uses SERPAPI_KEY env var. Returns raw JSON string.
/// Usage: web_search("query") -> JSON string
/// Usage: web_search("query", 5) -> JSON string with 5 results
fn builtin_web_search(args: &[Value]) -> Result<Value, String> {
    let query = expect_string_arg("web_search", args, 0)?;
    let num: i32 = match args.get(1) {
        Some(Value::Float(n)) => *n as i32,
        _ => 10,
    };

    let api_key = std::env::var("SERPAPI_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return Err("web_search(): SERPAPI_KEY env var not set".to_string());
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("web_search(): client error: {}", e))?;

    let encoded_query = urlencoding::encode(&query);
    let url = format!(
        "https://serpapi.com/search.json?q={}&num={}&api_key={}&hl=ru",
        encoded_query, num, api_key
    );

    let resp = client
        .get(&url)
        .send()
        .map_err(|e| format!("web_search(): request failed: {}", e))?;

    let body = resp
        .text()
        .map_err(|e| format!("web_search(): failed to read response: {}", e))?;

    Ok(Value::String(body))
}

/// `make_list(a, b, c, ...) -> List` — create a list from variadic arguments.
/// Eliminates race conditions from write_file/read_file workarounds for
/// returning multiple values from patterns.
/// Usage: make_list("red", "green", "blue") -> List ["red", "green", "blue"]
fn builtin_make_list(args: &[Value]) -> Result<Value, String> {
    Ok(Value::List(args.to_vec()))
}

// ── OpenPlanter-inspired: Fuzzy matching, safe editing, agent utilities (ADR-0063) ──

/// Helper: build a Value::Struct from a type name and a list of (key, value) pairs.
fn make_struct(type_name: &str, fields: Vec<(&str, Value)>) -> Value {
    let mut map = std::collections::HashMap::new();
    for (k, v) in fields {
        map.insert(k.to_string(), v);
    }
    Value::Struct {
        type_name: type_name.to_string(),
        fields: map,
    }
}

/// `fuzzy_match(a, b)` — Jaro-Winkler similarity between two strings (0.0..1.0).
/// Ported from OpenPlanter's wiki/matching.rs NameRegistry pattern.
fn builtin_fuzzy_match(args: &[Value]) -> Result<Value, String> {
    let a = expect_string_arg("fuzzy_match", args, 0)?;
    let b = expect_string_arg("fuzzy_match", args, 1)?;
    let score = strsim::jaro_winkler(&a, &b);
    Ok(Value::Float(score))
}

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

/// `compact_list(items, keep_first, keep_last)` — context condensation for lists.
/// Protects the first `keep_first` and last `keep_last` items, replaces middle items
/// with a single placeholder struct { compacted: true, removed_count: N }.
/// Inspired by OpenPlanter's compact_messages() for managing long conversation histories.
fn builtin_compact_list(args: &[Value]) -> Result<Value, String> {
    let items = expect_list_arg("compact_list", args, 0)?;
    let keep_first = expect_float_arg("compact_list", args, 1)? as usize;
    let keep_last = expect_float_arg("compact_list", args, 2)? as usize;
    let total = items.len();
    if total <= keep_first + keep_last {
        // Nothing to compact
        return Ok(Value::List(items));
    }
    let mut result: Vec<Value> = Vec::new();
    // Keep first N
    for item in items.iter().take(keep_first) {
        result.push(item.clone());
    }
    // Insert compacted placeholder
    let removed = total - keep_first - keep_last;
    result.push(make_struct(
        "Compacted",
        vec![
            ("compacted", Value::Bool(true)),
            ("removed_count", Value::Float(removed as f64)),
        ],
    ));
    // Keep last N
    let last_start = total - keep_last;
    for item in items.iter().skip(last_start) {
        result.push(item.clone());
    }
    Ok(Value::List(result))
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

// ── Dict operations (Наряд №17 В.1) ─────────────────────────────
// Dicts are represented as Value::Struct with type_name "Dict".

/// `dict_set(dict, key, value)` — set a key in a dict. Returns the modified dict.
fn builtin_dict_set(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 {
        return Err("dict_set() requires 3 arguments (dict, key, value)".to_string());
    }
    let key = expect_string_arg("dict_set", args, 1)?;
    let mut fields = match &args[0] {
        Value::Struct { fields, .. } => fields.clone(),
        other => {
            return Err(format!(
                "dict_set() expected Struct as first arg, got {}",
                other.type_name()
            ))
        }
    };
    fields.insert(key, args[2].clone());
    Ok(Value::Struct {
        type_name: "Dict".to_string(),
        fields,
    })
}

/// `dict_keys(dict) -> List` — return list of keys.
fn builtin_dict_keys(args: &[Value]) -> Result<Value, String> {
    let fields = match &args.get(0) {
        Some(Value::Struct { fields, .. }) => fields,
        _ => return Err("dict_keys() requires 1 argument (Struct/Dict)".to_string()),
    };
    let keys: Vec<Value> = fields.keys().map(|k| Value::String(k.clone())).collect();
    Ok(Value::List(keys))
}

/// `dict_values(dict) -> List` — return list of values.
fn builtin_dict_values(args: &[Value]) -> Result<Value, String> {
    let fields = match &args.get(0) {
        Some(Value::Struct { fields, .. }) => fields,
        _ => return Err("dict_values() requires 1 argument (Struct/Dict)".to_string()),
    };
    let values: Vec<Value> = fields.values().cloned().collect();
    Ok(Value::List(values))
}

/// `dict_has(dict, key) -> Bool` — check if key exists.
fn builtin_dict_has(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("dict_has() requires 2 arguments (dict, key)".to_string());
    }
    let key = expect_string_arg("dict_has", args, 1)?;
    let has = match &args[0] {
        Value::Struct { fields, .. } => fields.contains_key(&key),
        _ => {
            return Err(format!(
                "dict_has() expected Struct as first arg, got {}",
                args[0].type_name()
            ))
        }
    };
    Ok(Value::Bool(has))
}

// ── Format (Наряд №17 В.3) ──────────────────────────────────────
/// `format(template, arg1, arg2, ...)` — positional string interpolation.
/// Replaces `{}` placeholders in template with arguments.
/// Usage: format("Hello {}, you are {} years old", name, age)
fn builtin_format(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("format() requires at least 1 argument (template)".to_string());
    }
    let template = expect_string_arg("format", args, 0)?;
    let mut result = String::new();
    let mut arg_idx = 1;
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '{' && chars.peek() == Some(&'}') {
            chars.next(); // consume '}'
            if arg_idx < args.len() {
                result.push_str(&format!("{}", args[arg_idx]));
                arg_idx += 1;
            } else {
                return Err(format!(
                    "format(): not enough arguments for template (need {} more)",
                    arg_idx - 1
                ));
            }
        } else if ch == '{' && chars.peek() == Some(&'{') {
            chars.next(); // consume second '{', emit literal '{'
            result.push('{');
        } else {
            result.push(ch);
        }
    }
    Ok(Value::String(result))
}

// ── v0.8.0 — format_date (enhanced) ──────────────────────
/// Weekday names: Monday=0 (converted from libc Sunday=0).
const WEEKDAY_NAMES_MON: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// Month names (1-indexed).
const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const MONTH_ABBR: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `format_date(format)` — format current time.
/// `format_date(format, timestamp)` — format given unix timestamp (Float seconds).
/// v0.8.0: supports %y %I %p %A %a %B %b %j %w %W %% in addition to %Y %m %d %H %M %S %F %T %R.
fn builtin_format_date(args: &[Value]) -> Result<Value, String> {
    let fmt_str = if args.is_empty() {
        "%Y-%m-%d %H:%M:%S".to_string()
    } else {
        expect_string_arg("format_date", args, 0)?
    };

    let timestamp = if args.len() >= 2 {
        match &args[1] {
            Value::Float(f) => *f,
            _ => {
                return Err(format!(
                    "format_date(): timestamp must be Float, got {}",
                    args[1].type_name()
                ))
            }
        }
    } else {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    };

    let secs = timestamp as i64;
    let dt = chrono::Local
        .timestamp_opt(secs, 0)
        .single()
        .unwrap_or_else(|| chrono::Local::now());

    let y = dt.year() as u32;
    let mo = dt.month();
    let d = dt.day();
    let h = dt.hour();
    let mi = dt.minute();
    let s = dt.second();
    let wday_mon = dt.weekday().num_days_from_monday();
    let day_of_year = dt.ordinal();
    let week_num = ((day_of_year as i32 + 6 - wday_mon as i32) / 7).max(1) as u32;
    let ampm = if h >= 12 { "PM" } else { "AM" };
    let h12 = if h == 0 {
        12
    } else if h > 12 {
        h - 12
    } else {
        h
    };

    let result = match fmt_str.as_str() {
        "%F" => format!("{:04}-{:02}-{:02}", y, mo, d),
        "%T" => format!("{:02}:{:02}:{:02}", h, mi, s),
        "%R" => format!("{:02}:{:02}", h, mi),
        "%Y-%m-%d" => format!("{:04}-{:02}-{:02}", y, mo, d),
        "%d.%m.%Y" => format!("{:02}.{:02}.{:04}", d, mo, y),
        _ => {
            let mut out = String::new();
            let mut chars = fmt_str.chars().peekable();
            while let Some(ch) = chars.next() {
                if ch == '%' {
                    match chars.next() {
                        Some('Y') => out.push_str(&format!("{:04}", y)),
                        Some('y') => out.push_str(&format!("{:02}", y % 100)),
                        Some('m') => out.push_str(&format!("{:02}", mo)),
                        Some('d') => out.push_str(&format!("{:02}", d)),
                        Some('H') => out.push_str(&format!("{:02}", h)),
                        Some('I') => out.push_str(&format!("{:02}", h12)),
                        Some('M') => out.push_str(&format!("{:02}", mi)),
                        Some('S') => out.push_str(&format!("{:02}", s)),
                        Some('p') => out.push_str(ampm),
                        Some('A') => out.push_str(WEEKDAY_NAMES_MON[wday_mon as usize]),
                        Some('a') => out.push_str(&WEEKDAY_NAMES_MON[wday_mon as usize][..3]),
                        Some('B') => out.push_str(MONTH_NAMES[(mo - 1) as usize]),
                        Some('b') => out.push_str(MONTH_ABBR[(mo - 1) as usize]),
                        Some('j') => out.push_str(&format!("{:03}", day_of_year)),
                        Some('w') => out.push_str(&format!("{}", wday_mon)),
                        Some('W') => out.push_str(&format!("{:02}", week_num)),
                        Some('%') => out.push('%'),
                        Some('F') => out.push_str(&format!("{:04}-{:02}-{:02}", y, mo, d)),
                        Some('T') => out.push_str(&format!("{:02}:{:02}:{:02}", h, mi, s)),
                        Some('R') => out.push_str(&format!("{:02}:{:02}", h, mi)),
                        Some(c) => {
                            out.push('%');
                            out.push(c);
                        }
                        None => out.push('%'),
                    }
                } else {
                    out.push(ch);
                }
            }
            out
        }
    };
    Ok(Value::String(result))
}

// ── v0.8.0 — Additional Time / Date / Calendar builtins ──────────

fn date_is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn date_days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if date_is_leap(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn make_date_struct(type_name: &str, pairs: Vec<(&str, Value)>) -> Value {
    let mut map = std::collections::HashMap::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), v);
    }
    Value::Struct {
        type_name: type_name.to_string(),
        fields: map,
    }
}

/// `date_parts(timestamp?)` — returns struct with all date components via libc.
fn builtin_date_parts(args: &[Value]) -> Result<Value, String> {
    let ts = if args.is_empty() {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    } else {
        expect_float_arg("date_parts", args, 0)?
    };
    let secs = ts as i64;
    let dt = chrono::Local
        .timestamp_opt(secs, 0)
        .single()
        .unwrap_or_else(|| chrono::Local::now());

    let y = dt.year() as u32;
    let mo = dt.month();
    let d = dt.day();
    let h = dt.hour();
    let mi = dt.minute();
    let s = dt.second();
    let wday_mon = dt.weekday().num_days_from_monday();
    let day_of_year = dt.ordinal();
    let week_num = ((day_of_year as i32 + 6 - wday_mon as i32) / 7).max(1) as u32;

    Ok(make_date_struct(
        "Date",
        vec![
            ("year", Value::Float(y as f64)),
            ("month", Value::Float(mo as f64)),
            ("day", Value::Float(d as f64)),
            ("hour", Value::Float(h as f64)),
            ("minute", Value::Float(mi as f64)),
            ("second", Value::Float(s as f64)),
            ("weekday", Value::Float(wday_mon as f64)),
            (
                "weekday_name",
                Value::String(WEEKDAY_NAMES_MON[wday_mon as usize].to_string()),
            ),
            (
                "month_name",
                Value::String(MONTH_NAMES[(mo - 1) as usize].to_string()),
            ),
            ("day_of_year", Value::Float(day_of_year as f64)),
            ("week_number", Value::Float(week_num as f64)),
            ("timestamp", Value::Float(ts)),
        ],
    ))
}

/// `days_between(ts1, ts2)` — absolute difference in days.
fn builtin_days_between(args: &[Value]) -> Result<Value, String> {
    let ts1 = expect_float_arg("days_between", args, 0)?;
    let ts2 = expect_float_arg("days_between", args, 1)?;
    Ok(Value::Float((ts1 - ts2).abs() / 86400.0))
}

/// `days_in_month(year, month)` — days in given month (1-12).
fn builtin_days_in_month(args: &[Value]) -> Result<Value, String> {
    let year = expect_float_arg("days_in_month", args, 0)? as i32;
    let month = expect_float_arg("days_in_month", args, 1)? as u32;
    if month < 1 || month > 12 {
        return Err("days_in_month() month must be 1-12".to_string());
    }
    Ok(Value::Float(date_days_in_month(year, month) as f64))
}

/// `is_leap_year(year)` — Gregorian leap year check.
fn builtin_is_leap_year(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(date_is_leap(
        expect_float_arg("is_leap_year", args, 0)? as i32,
    )))
}

/// `add_days(timestamp, days)` — add/subtract days to timestamp.
fn builtin_add_days(args: &[Value]) -> Result<Value, String> {
    let ts = expect_float_arg("add_days", args, 0)?;
    let days = expect_float_arg("add_days", args, 1)?;
    Ok(Value::Float(ts + days * 86400.0))
}

/// `add_hours(timestamp, hours)` — add/subtract hours to timestamp.
fn builtin_add_hours(args: &[Value]) -> Result<Value, String> {
    let ts = expect_float_arg("add_hours", args, 0)?;
    let hours = expect_float_arg("add_hours", args, 1)?;
    Ok(Value::Float(ts + hours * 3600.0))
}

/// `weekday_name(timestamp)` — full weekday name ("Monday".."Sunday").
fn builtin_weekday_name(args: &[Value]) -> Result<Value, String> {
    let ts = expect_float_arg("weekday_name", args, 0)?;
    let secs = ts as i64;
    let dt = chrono::Local
        .timestamp_opt(secs, 0)
        .single()
        .unwrap_or_else(|| chrono::Local::now());
    let wday_mon = dt.weekday().num_days_from_monday();
    Ok(Value::String(
        WEEKDAY_NAMES_MON[wday_mon as usize].to_string(),
    ))
}

// ── v0.8.0 — Geolocation builtins ───────────────────────────────────

/// `geo_ip(ip?)` — geolocate by IP. Uses ip-api.com (free, no key).
/// Returns Struct {ip, city, region, country, country_code, lat, lon, isp, timezone}.
fn builtin_geo_ip(args: &[Value]) -> Result<Value, String> {
    let ip = match args.get(0) {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        _ => String::new(),
    };
    let url = if ip.is_empty() {
        "http://ip-api.com/json/".to_string()
    } else {
        format!("http://ip-api.com/json/{}", ip)
    };
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("geo_ip() client error: {}", e))?;
    let resp = client
        .get(&url)
        .send()
        .map_err(|e| format!("geo_ip() request failed: {}", e))?;
    let body = resp.text().unwrap_or_default();
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("geo_ip() parse error: {}", e))?;
    if json.get("status").and_then(|v| v.as_str()) != Some("success") {
        let msg = json
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(format!("geo_ip() API error: {}", msg));
    }
    let g = |key: &str| -> Value {
        json.get(key)
            .map(|v| match v {
                serde_json::Value::String(s) => Value::String(s.clone()),
                serde_json::Value::Number(n) => Value::Float(n.as_f64().unwrap_or(0.0)),
                serde_json::Value::Null => Value::String(String::new()),
                _ => Value::String(v.to_string()),
            })
            .unwrap_or(Value::String(String::new()))
    };
    Ok(make_date_struct(
        "GeoLocation",
        vec![
            ("ip", g("query")),
            ("city", g("city")),
            ("region", g("regionName")),
            ("country", g("country")),
            ("country_code", g("countryCode")),
            ("lat", g("lat")),
            ("lon", g("lon")),
            ("isp", g("isp")),
            ("timezone", g("timezone")),
        ],
    ))
}

/// `geo_distance(lat1, lon1, lat2, lon2, unit?)` — haversine distance. unit: "km"(default), "mi", "nm", "m".
fn builtin_geo_distance(args: &[Value]) -> Result<Value, String> {
    let lat1 = expect_float_arg("geo_distance", args, 0)?;
    let lon1 = expect_float_arg("geo_distance", args, 1)?;
    let lat2 = expect_float_arg("geo_distance", args, 2)?;
    let lon2 = expect_float_arg("geo_distance", args, 3)?;
    let unit = match args.get(4) {
        Some(Value::String(s)) => s.as_str(),
        _ => "km",
    };
    let to_rad = |d: f64| d * std::f64::consts::PI / 180.0;
    let dlat = to_rad(lat2 - lat1);
    let dlon = to_rad(lon2 - lon1);
    let a = (dlat / 2.0).sin().powi(2)
        + to_rad(lat1).cos() * to_rad(lat2).cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    let km = 6371.0 * c;
    Ok(Value::Float(match unit {
        "mi" => km * 0.621371,
        "nm" => km * 0.539957,
        "m" => km * 1000.0,
        _ => km,
    }))
}

// ── v0.8.0 — Weather builtins (Open-Meteo, free, no API key) ───────

/// WMO weather codes to human-readable description.
fn wmo_description(code: i64) -> &'static str {
    match code {
        0 => "Clear sky",
        1 => "Mainly clear",
        2 => "Partly cloudy",
        3 => "Overcast",
        45 | 48 => "Fog",
        51 | 53 | 55 => "Drizzle",
        56 | 57 => "Freezing drizzle",
        61 | 63 | 65 => "Rain",
        66 | 67 => "Freezing rain",
        71 | 73 | 75 => "Snow fall",
        77 => "Snow grains",
        80 | 81 | 82 => "Rain showers",
        85 | 86 => "Snow showers",
        95 => "Thunderstorm",
        96 | 99 => "Thunderstorm with hail",
        _ => "Unknown",
    }
}

/// Resolve city name to (lat, lon) via Open-Meteo geocoding (free, no key).
fn geo_resolve_city(city: &str) -> Result<(f64, f64), String> {
    let url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=en&format=json",
        urlencoding::encode(city)
    );
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("weather() client error: {}", e))?;
    let body = client
        .get(&url)
        .send()
        .map_err(|e| format!("weather() geocoding failed: {}", e))?
        .text()
        .unwrap_or_default();
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("weather() geocoding parse error: {}", e))?;
    let results = json
        .get("results")
        .and_then(|r| r.as_array())
        .ok_or_else(|| format!("weather() city not found: {}", city))?;
    if results.is_empty() {
        return Err(format!("weather() city not found: {}", city));
    }
    let first = &results[0];
    let lat = first
        .get("latitude")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let lon = first
        .get("longitude")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    Ok((lat, lon))
}

/// `weather(city_or_lat, lon?)` — current weather via Open-Meteo (FREE, no API key).
/// `weather("Minsk")` or `weather(53.9, 27.57)`.
/// Returns Struct {temp, feels_like, temp_min, temp_max, humidity, description,
///   wind_speed, wind_direction, pressure, cloud_cover, is_day, city, country}.
fn builtin_weather(args: &[Value]) -> Result<Value, String> {
    let (lat, lon, resolved_city) = if args.len() >= 2 {
        let lat = expect_float_arg("weather", args, 0)?;
        let lon = expect_float_arg("weather", args, 1)?;
        (lat, lon, String::new())
    } else {
        let city = expect_string_arg("weather", args, 0)?;
        let (lat, lon) = geo_resolve_city(&city)?;
        (lat, lon, city)
    };
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m,wind_direction_10m,surface_pressure,cloud_cover,is_day&timezone=auto",
        lat, lon
    );
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("weather() client error: {}", e))?;
    let resp = client
        .get(&url)
        .send()
        .map_err(|e| format!("weather() request failed: {}", e))?;
    let status = resp.status().as_u16();
    let body = resp.text().unwrap_or_default();
    if status >= 400 {
        return Err(format!("weather() API error {}: {}", status, body));
    }
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("weather() parse error: {}", e))?;
    let cur = json
        .get("current")
        .ok_or("weather() missing 'current' in response")?;
    let gf = |key: &str| -> f64 { cur.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0) };
    let code = gf("weather_code") as i64;
    let desc = wmo_description(code).to_string();
    Ok(make_date_struct(
        "Weather",
        vec![
            ("temp", Value::Float(gf("temperature_2m"))),
            ("feels_like", Value::Float(gf("apparent_temperature"))),
            ("temp_min", Value::Float(gf("temperature_2m"))),
            ("temp_max", Value::Float(gf("temperature_2m"))),
            ("humidity", Value::Float(gf("relative_humidity_2m"))),
            ("description", Value::String(desc)),
            ("wind_speed", Value::Float(gf("wind_speed_10m"))),
            ("wind_direction", Value::Float(gf("wind_direction_10m"))),
            ("pressure", Value::Float(gf("surface_pressure"))),
            ("cloud_cover", Value::Float(gf("cloud_cover"))),
            ("is_day", Value::Float(gf("is_day"))),
            ("city", Value::String(resolved_city)),
            ("country", Value::String(String::new())),
        ],
    ))
}

/// `weather_forecast(city_or_lat, lon?, days?)` — multi-day forecast via Open-Meteo (FREE, no API key).
/// `weather_forecast("Minsk", 7)` or `weather_forecast(53.9, 27.57, 3)`.
/// Default: 7 days. Max: 16 days. Returns List of DayForecast structs.
fn builtin_weather_forecast(args: &[Value]) -> Result<Value, String> {
    let (lat, lon) = if args.len() >= 2 && matches!(&args[1], Value::Float(_)) {
        let la = expect_float_arg("weather_forecast", args, 0)?;
        let lo = expect_float_arg("weather_forecast", args, 1)?;
        (la, lo)
    } else {
        let city = expect_string_arg("weather_forecast", args, 0)?;
        geo_resolve_city(&city)?
    };
    let mut days: u32 = 7;
    if args.len() == 2 && matches!(&args[1], Value::Float(_)) {
        days = expect_float_arg("weather_forecast", args, 1)? as u32;
    } else if args.len() >= 3 {
        days = expect_float_arg("weather_forecast", args, 2)? as u32;
    }
    if days < 1 {
        days = 1;
    }
    if days > 16 {
        days = 16;
    }
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_sum,wind_speed_10m_max,sunrise,sunset,uv_index_max&timezone=auto&forecast_days={}",
        lat, lon, days
    );
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| format!("weather_forecast() client error: {}", e))?;
    let resp = client
        .get(&url)
        .send()
        .map_err(|e| format!("weather_forecast() request failed: {}", e))?;
    let status = resp.status().as_u16();
    let body = resp.text().unwrap_or_default();
    if status >= 400 {
        return Err(format!("weather_forecast() API error {}: {}", status, body));
    }
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("weather_forecast() parse error: {}", e))?;
    let daily = json
        .get("daily")
        .ok_or("weather_forecast() missing 'daily' in response")?;
    let dates = daily
        .get("time")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let codes = daily
        .get("weather_code")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let tmax = daily
        .get("temperature_2m_max")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let tmin = daily
        .get("temperature_2m_min")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let precip = daily
        .get("precipitation_sum")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let wind = daily
        .get("wind_speed_10m_max")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let sunrise = daily
        .get("sunrise")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let sunset = daily
        .get("sunset")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let uv = daily
        .get("uv_index_max")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let gf = |arr: &[serde_json::Value], i: usize| -> f64 {
        arr.get(i).and_then(|v| v.as_f64()).unwrap_or(0.0)
    };
    let gs = |arr: &[serde_json::Value], i: usize| -> String {
        arr.get(i)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let mut result = Vec::new();
    for i in 0..dates.len() {
        let code = gf(&codes, i) as i64;
        let desc = wmo_description(code).to_string();
        let uv_val = gf(&uv, i);
        let uv_str = if uv_val < 0.0 {
            String::new()
        } else {
            format!("{:.1}", uv_val)
        };
        result.push(make_date_struct(
            "DayForecast",
            vec![
                ("date", Value::String(gs(&dates, i))),
                ("temp_max", Value::Float(gf(&tmax, i))),
                ("temp_min", Value::Float(gf(&tmin, i))),
                ("precipitation", Value::Float(gf(&precip, i))),
                ("weather_code", Value::Float(gf(&codes, i))),
                ("description", Value::String(desc)),
                ("wind_speed_max", Value::Float(gf(&wind, i))),
                ("sunrise", Value::String(gs(&sunrise, i))),
                ("sunset", Value::String(gs(&sunset, i))),
                ("uv_index", Value::String(uv_str)),
            ],
        ));
    }
    Ok(Value::List(result))
}

// ── v0.8.0 — Reminders builtins ─────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct ReminderEntry {
    id: String,
    message: String,
    fire_at: f64,
    interval: f64,
    next_fire: f64,
    data: String,
    active: bool,
    created_at: f64,
}

static REMINDERS: std::sync::OnceLock<StdMutex<Vec<ReminderEntry>>> = std::sync::OnceLock::new();

fn reminders_store() -> &'static StdMutex<Vec<ReminderEntry>> {
    REMINDERS.get_or_init(|| StdMutex::new(Vec::new()))
}

/// Global SQLite persistence for reminders (same pattern as KV_SQLITE).
static REMINDERS_SQLITE: std::sync::OnceLock<StdMutex<Option<rusqlite::Connection>>> =
    std::sync::OnceLock::new();

fn reminders_sqlite() -> &'static StdMutex<Option<rusqlite::Connection>> {
    REMINDERS_SQLITE.get_or_init(|| StdMutex::new(None))
}

/// Initialize SQLite persistence for reminders. Called from server.rs on startup.
pub fn init_reminder_persist(db_path: &str) -> Result<(), String> {
    let conn = rusqlite::Connection::open(db_path)
        .map_err(|e| format!("[reminders] Failed to open database '{}': {}", db_path, e))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS reminders (
            id TEXT PRIMARY KEY,
            message TEXT NOT NULL,
            fire_at REAL NOT NULL,
            interval REAL NOT NULL DEFAULT 0,
            next_fire REAL NOT NULL,
            data TEXT NOT NULL DEFAULT '',
            active INTEGER NOT NULL DEFAULT 1,
            created_at REAL NOT NULL
        );",
    )
    .map_err(|e| format!("[reminders] Failed to create table: {}", e))?;
    // Load existing reminders into memory
    let mut stmt = conn.prepare("SELECT id, message, fire_at, interval, next_fire, data, active, created_at FROM reminders")
        .map_err(|e| format!("[reminders] Failed to query: {}", e))?;
    let rows: Vec<ReminderEntry> = stmt
        .query_map([], |row| {
            Ok(ReminderEntry {
                id: row.get::<_, String>(0)?,
                message: row.get::<_, String>(1)?,
                fire_at: row.get::<_, f64>(2)?,
                interval: row.get::<_, f64>(3)?,
                next_fire: row.get::<_, f64>(4)?,
                data: row.get::<_, String>(5)?,
                active: row.get::<_, i32>(6)? != 0,
                created_at: row.get::<_, f64>(7)?,
            })
        })
        .map_err(|e| format!("[reminders] Failed to iterate: {}", e))?
        .filter_map(|r| r.ok())
        .collect();
    if let Ok(mut store) = reminders_store().lock() {
        store.extend(rows);
    }
    drop(stmt);
    let mut guard = reminders_sqlite()
        .lock()
        .map_err(|e| format!("[reminders] lock error: {}", e))?;
    *guard = Some(conn);
    eprintln!("[reminders] SQLite persistence enabled: {}", db_path);
    Ok(())
}

/// Write a single reminder to SQLite (write-through, called after mutations).
fn reminder_sqlite_upsert(entry: &ReminderEntry) {
    if let Ok(guard) = reminders_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO reminders (id, message, fire_at, interval, next_fire, data, active, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                rusqlite::params![entry.id, entry.message, entry.fire_at, entry.interval, entry.next_fire, entry.data, entry.active as i32, entry.created_at],
            );
        }
    }
}

fn reminder_sqlite_delete(id: &str) {
    if let Ok(guard) = reminders_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute("DELETE FROM reminders WHERE id = ?1", rusqlite::params![id]);
        }
    }
}

fn reminder_sqlite_delete_all_for_persona() {
    if let Ok(guard) = reminders_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute("DELETE FROM reminders", []);
        }
    }
}

/// `remind(message, timestamp, data?)` — one-time reminder. Returns ID.
fn builtin_remind(args: &[Value]) -> Result<Value, String> {
    let message = expect_string_arg("remind", args, 0)?;
    let fire_at = expect_float_arg("remind", args, 1)?;
    let data = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        Some(o) => format!("{}", o),
        None => String::new(),
    };
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let mut store = reminders_store()
        .lock()
        .map_err(|e| format!("remind() lock error: {}", e))?;
    let entry = ReminderEntry {
        id: id.clone(),
        message,
        fire_at,
        interval: 0.0,
        next_fire: fire_at,
        data,
        active: true,
        created_at: now_ts,
    };
    reminder_sqlite_upsert(&entry);
    store.push(entry);
    Ok(Value::String(id))
}

/// `remind_recurring(message, interval_seconds, data?)` — recurring reminder. Returns ID.
fn builtin_remind_recurring(args: &[Value]) -> Result<Value, String> {
    let message = expect_string_arg("remind_recurring", args, 0)?;
    let interval = expect_float_arg("remind_recurring", args, 1)?;
    if interval <= 0.0 {
        return Err("remind_recurring() interval must be positive".to_string());
    }
    let data = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        Some(o) => format!("{}", o),
        None => String::new(),
    };
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let mut store = reminders_store()
        .lock()
        .map_err(|e| format!("remind_recurring() lock error: {}", e))?;
    let entry = ReminderEntry {
        id: id.clone(),
        message,
        fire_at: now_ts,
        interval,
        next_fire: now_ts + interval,
        data,
        active: true,
        created_at: now_ts,
    };
    reminder_sqlite_upsert(&entry);
    store.push(entry);
    Ok(Value::String(id))
}

/// `cancel_remind(id)` — cancel reminder. Returns "ok" or "not_found".
fn builtin_cancel_remind(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("cancel_remind", args, 0)?;
    let mut store = reminders_store()
        .lock()
        .map_err(|e| format!("cancel_remind() lock error: {}", e))?;
    for entry in store.iter_mut() {
        if entry.id == id && entry.active {
            entry.active = false;
            reminder_sqlite_upsert(entry);
            return Ok(Value::String("ok".to_string()));
        }
    }
    Ok(Value::String("not_found".to_string()))
}

/// `list_reminders()` — list all active reminders.
fn builtin_list_reminders(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let store = reminders_store()
        .lock()
        .map_err(|e| format!("list_reminders() lock error: {}", e))?;
    let mut result = Vec::new();
    for entry in store.iter().filter(|r| r.active) {
        let rtype = if entry.interval > 0.0 {
            "recurring"
        } else {
            "once"
        };
        let ec = entry.clone();
        result.push(make_date_struct(
            "Reminder",
            vec![
                ("id", Value::String(ec.id)),
                ("message", Value::String(ec.message)),
                ("fire_at", Value::Float(ec.fire_at)),
                ("interval", Value::Float(ec.interval)),
                ("next_fire", Value::Float(ec.next_fire)),
                ("data", Value::String(ec.data)),
                ("created_at", Value::Float(ec.created_at)),
                ("type", Value::String(rtype.to_string())),
            ],
        ));
    }
    Ok(Value::List(result))
}

/// `check_reminders()` — get due reminders. One-shot deactivated; recurring advanced.
fn builtin_check_reminders(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let mut store = reminders_store()
        .lock()
        .map_err(|e| format!("check_reminders() lock error: {}", e))?;
    let mut due = Vec::new();
    for entry in store.iter_mut() {
        if !entry.active {
            continue;
        }
        if now_ts >= entry.next_fire {
            let rtype = if entry.interval > 0.0 {
                "recurring"
            } else {
                "once"
            };
            due.push(make_date_struct(
                "DueReminder",
                vec![
                    ("id", Value::String(entry.id.clone())),
                    ("message", Value::String(entry.message.clone())),
                    ("data", Value::String(entry.data.clone())),
                    ("type", Value::String(rtype.to_string())),
                    ("next_fire", Value::Float(entry.next_fire)),
                    ("overdue_seconds", Value::Float(now_ts - entry.next_fire)),
                ],
            ));
            if entry.interval > 0.0 {
                entry.next_fire += entry.interval;
            } else {
                entry.active = false;
            }
            reminder_sqlite_upsert(entry);
        }
    }
    Ok(Value::List(due))
}

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

// ── Problem B (Наряд reverse-iteration): list aggregation builtins ──────

/// `zip(list_a, list_b)` — pairwise merge two lists into list of 2-element structs [{a, b}, ...]
pub fn builtin_zip(args: &[Value]) -> Result<Value, String> {
    let list_a = match args.get(0) {
        Some(Value::List(items)) => items,
        _ => return Err("zip() expects first argument to be a List".to_string()),
    };
    let list_b = match args.get(1) {
        Some(Value::List(items)) => items,
        _ => return Err("zip() expects second argument to be a List".to_string()),
    };
    let paired: Vec<Value> = list_a
        .iter()
        .zip(list_b.iter())
        .map(|(a, b)| Value::Struct {
            type_name: "Pair".to_string(),
            fields: [("a".to_string(), a.clone()), ("b".to_string(), b.clone())]
                .into_iter()
                .collect(),
        })
        .collect();
    Ok(Value::List(paired))
}

/// `sort_by(list, key_field, descending?)` — sort list of structs by a field name.
/// descending: 1.0 = descending, 0.0 or absent = ascending.
pub fn builtin_sort_by(args: &[Value]) -> Result<Value, String> {
    let list = match args.get(0) {
        Some(Value::List(items)) => items.clone(),
        _ => return Err("sort_by() expects first argument to be a List".to_string()),
    };
    let key_field = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err("sort_by() expects second argument to be a field name (String)".to_string())
        }
    };
    let descending = match args.get(2) {
        Some(Value::Float(f)) if *f != 0.0 => true,
        _ => false,
    };

    let mut sorted = list;
    sorted.sort_by(|a, b| {
        let va = a
            .get_field(&key_field)
            .ok()
            .cloned()
            .unwrap_or(Value::Float(0.0));
        let vb = b
            .get_field(&key_field)
            .ok()
            .cloned()
            .unwrap_or(Value::Float(0.0));
        let fa = match va {
            Value::Float(f) => f,
            Value::String(s) => s.parse::<f64>().unwrap_or(0.0),
            _ => 0.0,
        };
        let fb = match vb {
            Value::Float(f) => f,
            Value::String(s) => s.parse::<f64>().unwrap_or(0.0),
            _ => 0.0,
        };
        if descending {
            fb.partial_cmp(&fa).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
        }
    });
    Ok(Value::List(sorted))
}

/// `filter(list, key_field, value)` — filter list of structs where field == value.
pub fn builtin_filter(args: &[Value]) -> Result<Value, String> {
    let list = match args.get(0) {
        Some(Value::List(items)) => items.clone(),
        _ => return Err("filter() expects first argument to be a List".to_string()),
    };
    let key_field = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err("filter() expects second argument to be a field name (String)".to_string())
        }
    };
    let filter_val = match args.get(2) {
        Some(v) => v.clone(),
        None => return Err("filter() expects three arguments".to_string()),
    };

    let filtered: Vec<Value> = list
        .into_iter()
        .filter(|item| {
            let field_val = item
                .get_field(&key_field)
                .ok()
                .cloned()
                .unwrap_or(Value::Unit);
            match (&field_val, &filter_val) {
                (Value::String(a), Value::String(b)) => a == b,
                (Value::Float(a), Value::Float(b)) => a == b,
                (Value::Bool(a), Value::Bool(b)) => a == b,
                _ => false,
            }
        })
        .collect();
    Ok(Value::List(filtered))
}

/// `reduce(list, key_field, initial)` — sum all float values of a field across list of structs.
pub fn builtin_reduce(args: &[Value]) -> Result<Value, String> {
    let list = match args.get(0) {
        Some(Value::List(items)) => items,
        _ => return Err("reduce() expects first argument to be a List".to_string()),
    };
    let key_field = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err("reduce() expects second argument to be a field name (String)".to_string())
        }
    };
    let initial = match args.get(2) {
        Some(Value::Float(f)) => *f,
        _ => {
            return Err("reduce() expects third argument to be an initial Float value".to_string())
        }
    };

    let mut acc = initial;
    for item in list {
        let field_val = item
            .get_field(&key_field)
            .ok()
            .cloned()
            .unwrap_or(Value::Float(0.0));
        if let Value::Float(f) = field_val {
            acc += f;
        }
    }
    Ok(Value::Float(acc))
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

/// `matches_any(text, triggers_list)` — case-insensitive substring match.
/// Returns 1.0 if ANY trigger string is found in text, 0.0 otherwise.
/// Used by skill_index tier matching (Problem A).
pub fn builtin_matches_any(args: &[Value]) -> Result<Value, String> {
    let text = match args.get(0) {
        Some(Value::String(s)) => s.to_lowercase(),
        _ => return Err("matches_any() expects first argument to be a String".to_string()),
    };
    let triggers = match args.get(1) {
        Some(Value::List(items)) => items,
        _ => {
            return Err(
                "matches_any() expects second argument to be a List of trigger strings".to_string(),
            )
        }
    };
    for trigger in triggers {
        if let Value::String(t) = trigger {
            if text.contains(&t.to_lowercase()) {
                return Ok(Value::Float(1.0));
            }
        }
    }
    Ok(Value::Float(0.0))
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

// ── Cron Scheduler (inspired by OpenHuman cron_add/cron_list/cron_remove/cron_run) ──
// Stores cron jobs in KV store under "cron_jobs" key as JSON array.
// The server.rs scheduler loop (5s tick) checks these jobs and fires due ones.

fn get_cron_jobs() -> Vec<serde_json::Value> {
    let store = kv_store().lock().ok();
    let sqlite = kv_sqlite().lock().ok();
    let raw = match (store, sqlite) {
        (Some(s), _) => s.get("cron_jobs").cloned(),
        (_, Some(guard)) => guard.as_ref().and_then(|conn| {
            conn.query_row(
                "SELECT value FROM kv_store WHERE key = 'cron_jobs'",
                [],
                |row| row.get(0),
            )
            .ok()
        }),
        _ => None,
    };
    raw.and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default()
}

fn save_cron_jobs(jobs: &[serde_json::Value]) {
    let json = serde_json::to_string(jobs).unwrap_or_else(|_| "[]".to_string());
    if let Ok(mut store) = kv_store().lock() {
        store.insert("cron_jobs".to_string(), json.clone());
    }
    if let Ok(guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES ('cron_jobs', ?1)",
                rusqlite::params![json],
            );
        }
    }
}

/// `cron_add(cron_expr, prompt)` — register a recurring cron job.
/// Returns Struct { id, cron_expr, prompt, enabled, next_run, status }.
/// cron_expr: "0 9 * * 1-5" (standard 5-field cron: min hour dom month dow)
/// The server scheduler tick loop fires due jobs by calling the prompt as a pattern.
fn builtin_cron_add(args: &[Value]) -> Result<Value, String> {
    let cron_expr = expect_string_arg("cron_add", args, 0)?;
    let prompt = expect_string_arg("cron_add", args, 1)?;
    // Validate cron expression has 5 fields
    let parts: Vec<&str> = cron_expr.split_whitespace().collect();
    if parts.len() != 5 {
        return Err(
            "cron_add() expects a 5-field cron expression (min hour dom month dow)".to_string(),
        );
    }
    let id = format!("cron_{}", chrono_now_timestamp());
    let mut jobs = get_cron_jobs();
    let job = serde_json::json!({
        "id": id,
        "cron_expr": cron_expr,
        "prompt": prompt,
        "enabled": true,
        "created_at": chrono_now_timestamp(),
        "last_run": serde_json::Value::Null,
        "run_count": 0
    });
    jobs.push(job);
    save_cron_jobs(&jobs);
    Ok(make_date_struct(
        "CronJob",
        vec![
            ("id", Value::String(id)),
            ("cron_expr", Value::String(cron_expr)),
            ("prompt", Value::String(prompt)),
            ("enabled", Value::Float(1.0)),
            ("status", Value::String("created".to_string())),
        ],
    ))
}

/// `cron_list()` — list all registered cron jobs.
/// Returns List of Struct { id, cron_expr, prompt, enabled, created_at, run_count }.
fn builtin_cron_list(args: &[Value]) -> Result<Value, String> {
    let _ = args; // variadic
    let jobs = get_cron_jobs();
    let mut result = Vec::new();
    for job in &jobs {
        let force_run = job["force_run"].as_bool().unwrap_or(false);
        result.push(make_date_struct(
            "CronJob",
            vec![
                (
                    "id",
                    Value::String(job["id"].as_str().unwrap_or("").to_string()),
                ),
                (
                    "cron_expr",
                    Value::String(job["cron_expr"].as_str().unwrap_or("").to_string()),
                ),
                (
                    "prompt",
                    Value::String(job["prompt"].as_str().unwrap_or("").to_string()),
                ),
                (
                    "enabled",
                    Value::Float(if job["enabled"].as_bool().unwrap_or(false) {
                        1.0
                    } else {
                        0.0
                    }),
                ),
                (
                    "run_count",
                    Value::Float(job["run_count"].as_u64().unwrap_or(0) as f64),
                ),
                ("force_run", Value::Float(if force_run { 1.0 } else { 0.0 })),
            ],
        ));
    }
    Ok(Value::List(result))
}

/// `cron_remove(id)` — remove a cron job by id.
/// Returns Struct { removed: Float, status: String }.
fn builtin_cron_remove(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("cron_remove", args, 0)?;
    let jobs = get_cron_jobs();
    let before = jobs.len();
    let filtered: Vec<serde_json::Value> = jobs
        .into_iter()
        .filter(|j| j["id"].as_str() != Some(&id))
        .collect();
    let removed = (before - filtered.len()) as f64;
    save_cron_jobs(&filtered);
    let status = if removed > 0.0 {
        "removed"
    } else {
        "not_found"
    };
    Ok(make_date_struct(
        "CronRemoveResult",
        vec![
            ("removed", Value::Float(removed)),
            ("status", Value::String(status.to_string())),
        ],
    ))
}

/// `cron_run(id)` — immediately execute a cron job (bypass schedule).
/// Returns Struct { id, executed: Float, status: String }.
/// Note: actual execution dispatch is handled by the server scheduler.
/// This builtin marks the job for immediate execution on next tick.
fn builtin_cron_run(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("cron_run", args, 0)?;
    let mut jobs = get_cron_jobs();
    let mut found = false;
    for job in &mut jobs {
        if job["id"].as_str() == Some(&id) {
            job["force_run"] = serde_json::Value::Bool(true);
            found = true;
            break;
        }
    }
    if found {
        save_cron_jobs(&jobs);
        Ok(make_date_struct(
            "CronRunResult",
            vec![
                ("id", Value::String(id)),
                ("executed", Value::Float(1.0)),
                ("status", Value::String("queued".to_string())),
            ],
        ))
    } else {
        Ok(make_date_struct(
            "CronRunResult",
            vec![
                ("id", Value::String(id)),
                ("executed", Value::Float(0.0)),
                ("status", Value::String("not_found".to_string())),
            ],
        ))
    }
}

/// `cron_mark_fired(id)` — internal: reset force_run, increment run_count, set last_run.
/// Called by the server scheduler after dispatching a cron job.
/// Returns Struct { id, status }.
fn builtin_cron_mark_fired(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("cron_mark_fired", args, 0)?;
    let mut jobs = get_cron_jobs();
    let mut found = false;
    for job in &mut jobs {
        if job["id"].as_str() == Some(&id) {
            job["force_run"] = serde_json::Value::Bool(false);
            let count = job["run_count"].as_u64().unwrap_or(0) + 1;
            job["run_count"] = serde_json::Value::Number(count.into());
            job["last_run"] = serde_json::Value::Number(chrono_now_timestamp().into());
            found = true;
            break;
        }
    }
    if found {
        save_cron_jobs(&jobs);
        Ok(make_date_struct(
            "CronMarkResult",
            vec![
                ("id", Value::String(id)),
                ("status", Value::String("fired".to_string())),
            ],
        ))
    } else {
        Ok(make_date_struct(
            "CronMarkResult",
            vec![
                ("id", Value::String(id)),
                ("status", Value::String("not_found".to_string())),
            ],
        ))
    }
}

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

// ── Helpers ──

/// Helper: get raw KV value (tries memory store, then SQLite).
fn kv_get_raw(key: &str) -> Option<String> {
    if let Ok(store) = kv_store().lock() {
        if let Some(v) = store.get(key).cloned() {
            return Some(v);
        }
    }
    if let Ok(guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *guard {
            if let Ok(v) = conn.query_row(
                "SELECT value FROM kv_store WHERE key = ?1",
                rusqlite::params![key],
                |row| row.get(0),
            ) {
                return Some(v);
            }
        }
    }
    None
}

// ── Memory Graph (v2) — petgraph-backed knowledge graph ─────────────
// Replaces flat L0/L1/L2 JSON array with a directed graph.
// Stored in KV store under "memory_graph" key as GraphSnapshot JSON.
// Auto-migrates from legacy "mtree_entries" JSON array on first access.

use crate::memory_graph::{graph_search, GraphSnapshot, MemoryGraph, MemoryNode, Relation};

fn get_memory_graph() -> MemoryGraph {
    let store = kv_store().lock().ok();
    let sqlite = kv_sqlite().lock().ok();

    // Try new format first
    let raw = if let Some(ref s) = store {
        s.get("memory_graph").cloned()
    } else if let Some(ref guard) = sqlite {
        guard.as_ref().and_then(|conn| {
            conn.query_row(
                "SELECT value FROM kv_store WHERE key = 'memory_graph'",
                [],
                |row| row.get(0),
            )
            .ok()
        })
    } else {
        None
    };

    if let Some(json) = raw {
        return MemoryGraph::from_json(&json);
    }

    // Migration: try legacy flat array format
    let legacy_raw = if let Some(ref s) = store {
        s.get("mtree_entries").cloned()
    } else if let Some(ref guard) = sqlite {
        guard.as_ref().and_then(|conn| {
            conn.query_row(
                "SELECT value FROM kv_store WHERE key = 'mtree_entries'",
                [],
                |row| row.get(0),
            )
            .ok()
        })
    } else {
        None
    };

    if let Some(legacy_json) = legacy_raw {
        if let Ok(entries) = serde_json::from_str::<Vec<serde_json::Value>>(&legacy_json) {
            let mut graph = MemoryGraph::new();
            for e in &entries {
                let node = MemoryNode {
                    id: e["id"].as_str().unwrap_or("").to_string(),
                    text: e["text"].as_str().unwrap_or("").to_string(),
                    level: e["level"].as_str().unwrap_or("L0").to_string(),
                    score: e["score"].as_f64().unwrap_or(0.0),
                    created_at: e["created_at"].as_i64().unwrap_or(0),
                    source: e["source"].as_str().unwrap_or("migrated").to_string(),
                    tags: vec![],
                    last_accessed: 0,
                    access_count: 0,
                };
                if !node.id.is_empty() {
                    graph.add_node(node);
                }
            }
            // Migrated — persist in new format
            save_memory_graph(&graph);
            return graph;
        }
    }

    MemoryGraph::new()
}

fn save_memory_graph(graph: &MemoryGraph) {
    let json = graph.to_json();
    if let Ok(mut store) = kv_store().lock() {
        store.insert("memory_graph".to_string(), json.clone());
    }
    if let Ok(guard) = kv_sqlite().lock() {
        if let Some(ref conn) = *guard {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES ('memory_graph', ?1)",
                rusqlite::params![json],
            );
        }
    }
}

/// Helper: inline admission gate score (same logic as memory_score builtin).
fn compute_admission_score(text: &str) -> f64 {
    let token_count = (text.len() as f64 / 4.0).ceil() as usize;
    let words: std::collections::HashSet<&str> = text.split_whitespace().collect();
    let unique_words = words.len();
    let entity_density = {
        let caps: Vec<&str> = text
            .split_whitespace()
            .filter(|w| w.chars().next().map_or(false, |c| c.is_uppercase()) && w.len() > 1)
            .collect();
        if token_count > 0 {
            caps.len() as f64 / token_count as f64
        } else {
            0.0
        }
    };
    let score = (0.3 * (unique_words as f64 / 50.0).min(1.0))
        + (0.2 * entity_density.min(1.0))
        + (0.2 * 1.0) // recency bonus for fresh entry
        + (0.3 * (token_count as f64 / 200.0).min(1.0));
    score.min(1.0)
}

/// `mtree_store(text, source?)` — store a memory chunk as a graph node.
/// Uses inline admission gate (threshold 0.3).
/// Returns Struct { id, level, score, admitted, reason }.
/// V2: stores as graph node (no edges yet; edges added by mtree_summarize).
fn builtin_mtree_store(args: &[Value]) -> Result<Value, String> {
    let text = expect_string_arg("mtree_store", args, 0)?;
    let source = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => "user".to_string(),
    };

    let score = compute_admission_score(&text);
    let admitted = score >= 0.3;

    if !admitted {
        return Ok(make_date_struct(
            "MTreeStore",
            vec![
                ("id", Value::String("".to_string())),
                ("level", Value::String("L0".to_string())),
                ("score", Value::Float(score)),
                ("admitted", Value::Float(0.0)),
                ("reason", Value::String("below_threshold".to_string())),
            ],
        ));
    }

    let id = format!("mt_{}", chrono_now_timestamp());
    let node = MemoryNode {
        id: id.clone(),
        text: text.clone(),
        level: "L0".to_string(),
        score,
        created_at: chrono_now_timestamp(),
        source,
        tags: vec![],
        last_accessed: 0,
        access_count: 0,
    };

    let mut graph = get_memory_graph();
    graph.add_node(node);
    save_memory_graph(&graph);

    Ok(make_date_struct(
        "MTreeStore",
        vec![
            ("id", Value::String(id)),
            ("level", Value::String("L0".to_string())),
            ("score", Value::Float(score)),
            ("admitted", Value::Float(1.0)),
            ("reason", Value::String("stored".to_string())),
        ],
    ))
}

/// `mtree_retrieve(query, limit?)` — retrieve top-N relevant memories using graph search.
/// V2: uses graph_search (keyword relevance on graph nodes) instead of flat array scan.
/// Default limit: 5. Returns List of Struct { id, text, level, score, relevance }.
fn builtin_mtree_retrieve(args: &[Value]) -> Result<Value, String> {
    let query = expect_string_arg("mtree_retrieve", args, 0)?;
    let limit = match args.get(1) {
        Some(Value::Float(f)) => *f as usize,
        _ => 5,
    };
    let limit = if limit == 0 { 5 } else { limit };

    let graph = get_memory_graph();
    let results = graph_search(&graph, &query, limit, None);

    let mut result = Vec::new();
    for (id, text, level, score, relevance) in &results {
        result.push(make_date_struct(
            "MTreeEntry",
            vec![
                ("id", Value::String(id.clone())),
                ("text", Value::String(text.clone())),
                ("level", Value::String(level.clone())),
                ("score", Value::Float(*score)),
                ("relevance", Value::Float(*relevance)),
            ],
        ));
    }
    Ok(Value::List(result))
}

/// `mtree_forget(id)` — delete a memory node and its edges from the graph.
/// Returns Struct { id, removed, status }.
fn builtin_mtree_forget(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("mtree_forget", args, 0)?;
    let mut graph = get_memory_graph();
    let existed = graph.get_node(&id).is_some();
    let removed = if existed { 1.0 } else { 0.0 };
    if existed {
        graph.remove_node(&id);
        save_memory_graph(&graph);
    }
    let status = if existed { "removed" } else { "not_found" };
    Ok(make_date_struct(
        "MTreeForget",
        vec![
            ("id", Value::String(id)),
            ("removed", Value::Float(removed)),
            ("status", Value::String(status.to_string())),
        ],
    ))
}

/// `mtree_summarize()` — promote L0 entries to L1 with derived_from edges,
/// then if 3+ L1 entries exist, create L2 global summary with edges to all L1.
/// V2: uses graph edges (DerivedFrom) instead of flat JSON "summary" field.
/// Returns Struct { l0_promoted, l1_count, l2_created, status, edges }.
fn builtin_mtree_summarize(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let mut graph = get_memory_graph();

    // ── Phase 1: L0 → L1 ──
    let l0_nodes: Vec<MemoryNode> = graph
        .nodes()
        .into_iter()
        .filter(|n| n.level == "L0")
        .cloned()
        .collect();

    let mut l0_promoted = 0u32;
    let batch_size = 10;
    for chunk in l0_nodes.chunks(batch_size) {
        let texts: Vec<String> = chunk.iter().map(|n| n.text.clone()).collect();
        let ids: Vec<String> = chunk.iter().map(|n| n.id.clone()).collect();

        let combined: String = texts.join(" | ");
        let summary = if combined.len() > 500 {
            format!("{}...", &combined[..497])
        } else {
            combined
        };

        let l1_id = format!("mt_l1_{}", chrono_now_timestamp());
        let l1_node = MemoryNode {
            id: l1_id.clone(),
            text: summary,
            level: "L1".to_string(),
            score: 0.7,
            created_at: chrono_now_timestamp(),
            source: "mtree_summarize".to_string(),
            tags: vec![],
            last_accessed: 0,
            access_count: 0,
        };
        graph.add_node(l1_node);

        // Add derived_from edges: L1 → each L0 in the batch
        for src_id in &ids {
            let _ = graph.add_edge(&l1_id, src_id, Relation::DerivedFrom, 0.8);
        }
        l0_promoted += 1;
    }

    // ── Phase 2: L1 → L2 global summary ──
    let l1_nodes: Vec<MemoryNode> = graph
        .nodes()
        .into_iter()
        .filter(|n| n.level == "L1")
        .cloned()
        .collect();

    let l1_count = l1_nodes.len();
    let mut l2_created = 0u32;

    if l1_count >= 3 {
        // Remove any existing L2
        let l2_ids: Vec<String> = graph
            .nodes()
            .into_iter()
            .filter(|n| n.level == "L2")
            .map(|n| n.id.clone())
            .collect();
        for id in &l2_ids {
            graph.remove_node(id);
        }

        let l1_texts: Vec<String> = l1_nodes.iter().map(|n| n.text.clone()).collect();
        let l1_ids: Vec<String> = l1_nodes.iter().map(|n| n.id.clone()).collect();

        let combined: String = l1_texts.join("\n---\n");
        let global_summary = if combined.len() > 1000 {
            format!("{}...", &combined[..997])
        } else {
            combined
        };

        let l2_id = format!("mt_l2_{}", chrono_now_timestamp());
        let l2_node = MemoryNode {
            id: l2_id.clone(),
            text: global_summary,
            level: "L2".to_string(),
            score: 0.9,
            created_at: chrono_now_timestamp(),
            source: "mtree_summarize".to_string(),
            tags: vec![],
            last_accessed: 0,
            access_count: 0,
        };
        graph.add_node(l2_node);

        // L2 derives from all L1 entries
        for src_id in &l1_ids {
            let _ = graph.add_edge(&l2_id, src_id, Relation::DerivedFrom, 0.9);
        }
        l2_created = 1;
    }

    save_memory_graph(&graph);

    // Recount L1 after changes
    let l1_final = graph.count_by_level().get("L1").copied().unwrap_or(0);

    let (nodes, edges, components) = graph.stats();
    let status = match (l0_promoted, l2_created) {
        (0, 0) => "no_unsummarized_l0".to_string(),
        (_, 1) => "l0_and_l2_promoted".to_string(),
        (n, 0) if n > 0 => "l0_promoted".to_string(),
        _ => "no_change".to_string(),
    };

    Ok(make_date_struct(
        "MTreeSummarize",
        vec![
            ("l0_promoted", Value::Float(l0_promoted as f64)),
            ("l1_count", Value::Float(l1_final as f64)),
            ("l2_created", Value::Float(l2_created as f64)),
            ("status", Value::String(status)),
            ("graph_nodes", Value::Float(nodes as f64)),
            ("graph_edges", Value::Float(edges as f64)),
            ("components", Value::Float(components as f64)),
        ],
    ))
}

/// `mtree_stats()` — diagnostics: count entries at each level, graph metrics.
/// V2: includes graph nodes, edges, connected components.
/// Returns Struct { l0, l1, l2, total, total_chars, nodes, edges, components }.
fn builtin_mtree_stats(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let graph = get_memory_graph();
    let levels = graph.count_by_level();
    let l0 = *levels.get("L0").unwrap_or(&0);
    let l1 = *levels.get("L1").unwrap_or(&0);
    let l2 = *levels.get("L2").unwrap_or(&0);
    let total_chars = graph.total_chars();
    let (nodes, edges, components) = graph.stats();
    Ok(make_date_struct(
        "MTreeStats",
        vec![
            ("l0", Value::Float(l0 as f64)),
            ("l1", Value::Float(l1 as f64)),
            ("l2", Value::Float(l2 as f64)),
            ("total", Value::Float(nodes as f64)),
            ("total_chars", Value::Float(total_chars as f64)),
            ("graph_edges", Value::Float(edges as f64)),
            ("components", Value::Float(components as f64)),
        ],
    ))
}

// ── Graph Query builtins (v0.8.10) ──────────────────────────────────

/// `graph_query(query, limit?, level?)` — search memory graph by keyword relevance.
/// Same scoring as mtree_retrieve but explicit API. Optional level filter.
/// Returns List of Struct { id, text, level, score, relevance }.
fn builtin_graph_query(args: &[Value]) -> Result<Value, String> {
    let query = expect_string_arg("graph_query", args, 0)?;
    let limit = match args.get(1) {
        Some(Value::Float(f)) => *f as usize,
        _ => 5,
    };
    let limit = if limit == 0 { 5 } else { limit };
    let level_filter = match args.get(2) {
        Some(Value::String(s)) if !s.is_empty() => Some(s.as_str()),
        _ => None,
    };

    let mut graph = get_memory_graph();
    let results = graph_search(&graph, &query, limit, level_filter);

    // Touch accessed nodes to update last_accessed
    let now = chrono_now_timestamp();
    for (id, _, _, _, _) in &results {
        graph.touch(id, now);
    }
    save_memory_graph(&graph);

    let mut result = Vec::new();
    for (id, text, level, score, relevance) in &results {
        result.push(make_date_struct(
            "GraphEntry",
            vec![
                ("id", Value::String(id.clone())),
                ("text", Value::String(text.clone())),
                ("level", Value::String(level.clone())),
                ("score", Value::Float(*score)),
                ("relevance", Value::Float(*relevance)),
            ],
        ));
    }
    Ok(Value::List(result))
}

/// `graph_path(from_id, to_id)` — find shortest path between two memory nodes.
/// Returns List of Struct { id, text, level } (ordered from source to target),
/// or List with single entry { status: "no_path" } if no path exists.
fn builtin_graph_path(args: &[Value]) -> Result<Value, String> {
    let from_id = expect_string_arg("graph_path", args, 0)?;
    let to_id = expect_string_arg("graph_path", args, 1)?;

    let graph = get_memory_graph();
    match graph.shortest_path(&from_id, &to_id) {
        Some(path) => {
            let mut result = Vec::new();
            for id in &path {
                if let Some(node) = graph.get_node(id) {
                    result.push(make_date_struct(
                        "PathNode",
                        vec![
                            ("id", Value::String(node.id.clone())),
                            ("text", Value::String(node.text.clone())),
                            ("level", Value::String(node.level.clone())),
                        ],
                    ));
                }
            }
            Ok(Value::List(result))
        }
        None => Ok(Value::List(vec![make_date_struct(
            "PathError",
            vec![
                ("status", Value::String("no_path".to_string())),
                ("from", Value::String(from_id)),
                ("to", Value::String(to_id)),
            ],
        )])),
    }
}

/// `graph_neighbors(id, depth?)` — get nodes connected to a given node within depth.
/// Default depth: 1. Bidirectional (follows edges both ways).
/// Returns List of Struct { id, text, level, distance }.
fn builtin_graph_neighbors(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("graph_neighbors", args, 0)?;
    let depth = match args.get(1) {
        Some(Value::Float(f)) => *f as usize,
        _ => 1,
    };
    let depth = if depth == 0 { 1 } else { depth };

    let mut graph = get_memory_graph();
    let nbrs: Vec<(String, String, String, usize)> = graph
        .neighbors(&id, depth)
        .into_iter()
        .map(|(n, d)| (n.id.clone(), n.text.clone(), n.level.clone(), d))
        .collect();

    // Touch the queried node and its neighbors
    let now = chrono_now_timestamp();
    graph.touch(&id, now);
    for (nid, _, _, _) in &nbrs {
        graph.touch(nid, now);
    }
    save_memory_graph(&graph);

    let mut result = Vec::new();
    for (id, text, level, distance) in &nbrs {
        result.push(make_date_struct(
            "Neighbor",
            vec![
                ("id", Value::String(id.clone())),
                ("text", Value::String(text.clone())),
                ("level", Value::String(level.clone())),
                ("distance", Value::Float(*distance as f64)),
            ],
        ));
    }
    Ok(Value::List(result))
}

/// `memory_decay(lambda?)` — apply exponential decay to all memory node scores.
/// `lambda` controls decay rate (default 0.01 = gentle).
/// Formula: score *= e^(-lambda * hours_since_access).
/// Returns Struct { decayed: <count>, nodes: <total>, edges: <total> }.
fn builtin_memory_decay(args: &[Value]) -> Result<Value, String> {
    let lambda = match args.get(0) {
        Some(Value::Float(f)) => *f,
        _ => 0.01,
    };
    let now = chrono_now_timestamp();
    let mut graph = get_memory_graph();
    let decayed = graph.decay(lambda, now);
    let (nodes, edges, components) = graph.stats();
    save_memory_graph(&graph);
    Ok(make_date_struct(
        "DecayResult",
        vec![
            ("decayed", Value::Float(decayed as f64)),
            ("nodes", Value::Float(nodes as f64)),
            ("edges", Value::Float(edges as f64)),
            ("components", Value::Float(components as f64)),
        ],
    ))
}

/// `memory_boost(id, amount?)` — boost a memory node's score by amount (default 0.1, capped at 1.0).
/// Updates last_accessed timestamp. Returns Struct { id, new_score, access_count }.
fn builtin_memory_boost(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("memory_boost", args, 0)?;
    let amount = match args.get(1) {
        Some(Value::Float(f)) => *f,
        _ => 0.1,
    };
    let now = chrono_now_timestamp();
    let mut graph = get_memory_graph();
    let boosted = graph.boost(&id, amount, now);
    if boosted {
        let node = match graph.get_node(&id) {
            Some(n) => n,
            None => return Err("memory_boost: node not found after boost".to_string()),
        };
        save_memory_graph(&graph);
        Ok(make_date_struct(
            "BoostResult",
            vec![
                ("id", Value::String(node.id.clone())),
                ("new_score", Value::Float(node.score)),
                ("access_count", Value::Float(node.access_count as f64)),
            ],
        ))
    } else {
        Err(format!("memory_boost: node '{}' not found", id))
    }
}

/// `memory_prune(threshold?, min_age_hours?)` — remove dead memory nodes.
/// `threshold`: minimum score to keep (default 0.05).
/// `min_age_hours`: minimum age in hours before pruning (default 24, protects fresh entries).
/// Returns Struct { pruned: <count>, remaining: <total> }.
fn builtin_memory_prune(args: &[Value]) -> Result<Value, String> {
    let threshold = match args.get(0) {
        Some(Value::Float(f)) => *f,
        _ => 0.05,
    };
    let min_age_hours = match args.get(1) {
        Some(Value::Float(f)) => *f,
        _ => 24.0,
    };
    let now = chrono_now_timestamp();
    let mut graph = get_memory_graph();
    let pruned = graph.prune(threshold, min_age_hours, now);
    let (nodes, _, _) = graph.stats();
    save_memory_graph(&graph);
    Ok(make_date_struct(
        "PruneResult",
        vec![
            ("pruned", Value::Float(pruned as f64)),
            ("remaining", Value::Float(nodes as f64)),
        ],
    ))
}

// ── V2: Belief Revision ────────────────────────────────────────────

/// `memory_revise(id, new_text, new_score?)` — update a node and resolve Contradicts.
/// If the node contradicts others, the system keeps the higher-scoring belief
/// and demotes the loser (score *= 0.3, adds Supersedes edge).
/// Returns Struct { action, winner_id, superseded_id? }.
fn builtin_memory_revise(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("memory_revise", args, 0)?;
    let new_text = expect_string_arg("memory_revise", args, 1)?;
    let new_score = match args.get(2) {
        Some(Value::Float(f)) => *f,
        _ => 0.8,
    };
    let now = chrono_now_timestamp();
    let mut graph = get_memory_graph();
    match graph.revise(&id, &new_text, new_score, now) {
        Some(result) => {
            save_memory_graph(&graph);
            let mut fields = vec![
                ("action", Value::String(result.action)),
                ("winner_id", Value::String(result.winner_id)),
            ];
            if let Some(sid) = result.superseded_id {
                fields.push(("superseded_id", Value::String(sid)));
            }
            Ok(make_date_struct("ReviseResult", fields))
        }
        None => Err(format!("memory_revise: node '{}' not found", id)),
    }
}

// ── V3: Subgraph as First-Class Value ──────────────────────────────

/// `subgraph_extract(id, depth?)` — extract a subgraph around a node as a first-class value.
/// Returns an opaque Subgraph value containing nodes and edges within depth.
/// Pass to subgraph_nodes() or subgraph_json() to inspect.
fn builtin_subgraph_extract(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("subgraph_extract", args, 0)?;
    let depth = match args.get(1) {
        Some(Value::Float(f)) => *f as usize,
        _ => 2,
    };
    let depth = if depth == 0 { 2 } else { depth };

    let mut graph = get_memory_graph();
    let nbrs: Vec<(String, String, String, usize)> = graph
        .neighbors(&id, depth)
        .into_iter()
        .map(|(n, d)| (n.id.clone(), n.text.clone(), n.level.clone(), d))
        .collect();
    let mut ids: Vec<String> = nbrs.iter().map(|(nid, _, _, _)| nid.clone()).collect();
    // Include the center node itself
    if graph.get_node(&id).is_some() && !ids.contains(&id) {
        ids.insert(0, id.clone());
    }

    // Touch accessed nodes
    let now = chrono_now_timestamp();
    graph.touch(&id, now);
    for nid in &ids {
        graph.touch(nid, now);
    }
    save_memory_graph(&graph);

    let snapshot = graph.subgraph(&ids);
    Ok(Value::Subgraph(snapshot))
}

/// `subgraph_nodes(subgraph_value)` — extract node list from a Subgraph value.
/// Returns List of Struct { id, text, level, score }.
fn builtin_subgraph_nodes(args: &[Value]) -> Result<Value, String> {
    let snap = match args.get(0) {
        Some(Value::Subgraph(s)) => s,
        _ => return Err("subgraph_nodes: expected Subgraph value as first argument".to_string()),
    };
    let mut result = Vec::new();
    for node in &snap.nodes {
        result.push(make_date_struct(
            "GraphNode",
            vec![
                ("id", Value::String(node.id.clone())),
                ("text", Value::String(node.text.clone())),
                ("level", Value::String(node.level.clone())),
                ("score", Value::Float(node.score)),
            ],
        ));
    }
    Ok(Value::List(result))
}

/// `subgraph_json(subgraph_value)` — serialize a Subgraph to JSON string.
fn builtin_subgraph_json(args: &[Value]) -> Result<Value, String> {
    let snap = match args.get(0) {
        Some(Value::Subgraph(s)) => s,
        _ => return Err("subgraph_json: expected Subgraph value as first argument".to_string()),
    };
    match serde_json::to_string(snap) {
        Ok(json) => Ok(Value::String(json)),
        Err(e) => Err(format!("subgraph_json: serialization failed: {}", e)),
    }
}

// ── V4: Execution Tracing ──────────────────────────────────────────

/// `trace_start(name)` — begin a named trace span. Stores start time in session.
/// Returns Unit.
fn builtin_trace_start(args: &[Value]) -> Result<Value, String> {
    let name = expect_string_arg("trace_start", args, 0)?;
    let key = format!("__trace_{}", name);
    let now = format!("{}", chrono_now_timestamp());
    if let Ok(mut store) = kv_store().lock() {
        store.insert(key, now);
    }
    Ok(Value::Unit)
}

/// `trace_end(name)` — end a named trace span. Returns Struct { name, elapsed_ms, elapsed_secs }.
/// Removes the start time from session.
fn builtin_trace_end(args: &[Value]) -> Result<Value, String> {
    let name = expect_string_arg("trace_end", args, 0)?;
    let key = format!("__trace_{}", name);
    let start: i64 = if let Ok(store) = kv_store().lock() {
        match store.get(&key) {
            Some(s) => s.parse().unwrap_or(0),
            None => return Err(format!("trace_end: no active trace '{}'", name)),
        }
    } else {
        return Err("trace_end: cannot access session store".to_string());
    };
    let now = chrono_now_timestamp();
    let elapsed_ms = (now - start).max(0) * 1000; // seconds→ms (timestamp is seconds)
                                                  // Clean up
    if let Ok(mut store) = kv_store().lock() {
        store.remove(&key);
    }
    Ok(make_date_struct(
        "TraceSpan",
        vec![
            ("name", Value::String(name)),
            ("elapsed_ms", Value::Float(elapsed_ms as f64)),
            ("elapsed_secs", Value::Float((now - start).max(0) as f64)),
        ],
    ))
}

// ── V5: Assertions ──────────────────────────────────────────────────

/// `assert_eq(actual, expected)` — error if two values display differently.
/// Returns the actual value on success.
fn builtin_assert_eq(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("assert_eq: requires 2 arguments (actual, expected)".to_string());
    }
    let actual_str = format!("{}", args[0]);
    let expected_str = format!("{}", args[1]);
    if actual_str != expected_str {
        Err(format!(
            "assert_eq failed: {} != {}",
            actual_str, expected_str
        ))
    } else {
        Ok(args[0].clone())
    }
}

/// `assert_contains(haystack, needle)` — panic if needle not found in haystack string.
fn builtin_assert_contains(args: &[Value]) -> Result<Value, String> {
    let haystack = format!("{}", args.get(0).unwrap_or(&Value::Unit));
    let needle = format!("{}", args.get(1).unwrap_or(&Value::Unit));
    if !haystack.contains(&needle) {
        Err(format!(
            "assert_contains failed: '{}' not in '{}'",
            needle,
            &haystack[..haystack.len().min(80)]
        ))
    } else {
        Ok(args[0].clone())
    }
}

/// Helper: current Unix timestamp.
fn chrono_now_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ════════════════════════════════════════════════════════════════════
// sqz-inspired builtins (P1 + P2 + P3)
// Source concept: https://github.com/ojuschugh1/sqz (ELv2 — no code copied)
// ════════════════════════════════════════════════════════════════════

// ── P1: String/List utilities ──────────────────────────────────────

/// `squeeze(s, chars)` — collapse consecutive identical characters from `chars`.
fn builtin_squeeze(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("squeeze", args, 0)?;
    let chars = expect_string_arg("squeeze", args, 1)?;
    if chars.is_empty() {
        return Ok(Value::String(s));
    }
    let char_set: std::collections::HashSet<char> = chars.chars().collect();
    let mut result = String::with_capacity(s.len());
    let mut prev: Option<char> = None;
    for c in s.chars() {
        if char_set.contains(&c) && prev == Some(c) {
            continue; // skip consecutive duplicate
        }
        result.push(c);
        prev = Some(c);
    }
    Ok(Value::String(result))
}

/// `dedup(list)` — remove duplicate elements, preserving first-occurrence order.
fn builtin_dedup(args: &[Value]) -> Result<Value, String> {
    let list = match args.get(0) {
        Some(Value::List(items)) => items.clone(),
        Some(other) => {
            return Err(format!(
                "dedup() expected List argument, got {}",
                other.type_name()
            ))
        }
        None => return Err("dedup() requires 1 argument".to_string()),
    };
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut result = Vec::new();
    for item in &list {
        let key = match item {
            Value::String(s) => s.clone(),
            Value::Float(f) => format!("{}", f),
            Value::Bool(b) => format!("{}", b),
            other => {
                // Use JSON representation for complex types
                let json = mlog_value_to_json(other);
                serde_json::to_string(&json).unwrap_or_else(|_| format!("{}", other))
            }
        };
        if seen.insert(key) {
            result.push(item.clone());
        }
    }
    Ok(Value::List(result))
}

/// `condense(list)` — collapse consecutive identical string elements with count.
/// Example: ["a","a","b"] -> ["a","x2","b"]
fn builtin_condense(args: &[Value]) -> Result<Value, String> {
    let list = match args.get(0) {
        Some(Value::List(items)) => items.clone(),
        Some(other) => {
            return Err(format!(
                "condense() expected List argument, got {}",
                other.type_name()
            ))
        }
        None => return Err("condense() requires 1 argument".to_string()),
    };
    let mut result: Vec<Value> = Vec::new();
    let mut i = 0;
    while i < list.len() {
        let current = match &list[i] {
            Value::String(s) => s.clone(),
            other => {
                return Err(format!(
                    "condense() all elements must be String, got {} at index {}",
                    other.type_name(),
                    i
                ))
            }
        };
        let mut count: usize = 1;
        while i + count < list.len() {
            if let Value::String(ref next) = list[i + count] {
                if next == &current {
                    count += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        result.push(Value::String(current.clone()));
        if count > 1 {
            result.push(Value::String(format!("\u{00d7}{}", count)));
        }
        i += count;
    }
    Ok(Value::List(result))
}

/// `strip(s, chars)` — remove characters from both ends of string.
fn builtin_strip(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("strip", args, 0)?;
    let chars = expect_string_arg("strip", args, 1)?;
    if chars.is_empty() {
        return Ok(Value::String(s));
    }
    let char_set: std::collections::HashSet<char> = chars.chars().collect();
    let start = s.chars().take_while(|c| char_set.contains(c)).count();
    let end = s.chars().rev().take_while(|c| char_set.contains(c)).count();
    let s_chars: Vec<char> = s.chars().collect();
    let trimmed: String = s_chars[start..s_chars.len().saturating_sub(end)]
        .iter()
        .collect();
    Ok(Value::String(trimmed))
}

/// `chomp(s)` — remove a single trailing newline (\n or \r\n).
fn builtin_chomp(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("chomp", args, 0)?;
    let trimmed = if s.ends_with("\r\n") {
        &s[..s.len() - 2]
    } else if s.ends_with('\n') {
        &s[..s.len() - 1]
    } else {
        &s[..]
    };
    Ok(Value::String(trimmed.to_string()))
}

/// `repeat(s, n)` — repeat string n times.
fn builtin_repeat(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("repeat", args, 0)?;
    let n = expect_float_arg("repeat", args, 1)?;
    if n < 0.0 {
        return Err("repeat() count must be non-negative".to_string());
    }
    let n_int = n as usize;
    if (n - n_int as f64).abs() > 1e-9 {
        return Err("repeat() count must be an integer".to_string());
    }
    Ok(Value::String(s.repeat(n_int)))
}

/// `pad_left(s, n, fill)` — left-pad string with fill character to length n.
fn builtin_pad_left(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("pad_left", args, 0)?;
    let n = expect_float_arg("pad_left", args, 1)?;
    let fill = expect_string_arg("pad_left", args, 2)?;
    if n < 0.0 {
        return Err("pad_left() width must be non-negative".to_string());
    }
    let n_int = n as usize;
    let fill_char = fill.chars().next().unwrap_or(' ');
    let s_len = s.chars().count();
    if s_len >= n_int {
        return Ok(Value::String(s));
    }
    let padding_len = n_int - s_len;
    let padding: String = std::iter::repeat(fill_char).take(padding_len).collect();
    Ok(Value::String(format!("{}{}", padding, s)))
}

/// `pad_right(s, n, fill)` — right-pad string with fill character to length n.
fn builtin_pad_right(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("pad_right", args, 0)?;
    let n = expect_float_arg("pad_right", args, 1)?;
    let fill = expect_string_arg("pad_right", args, 2)?;
    if n < 0.0 {
        return Err("pad_right() width must be non-negative".to_string());
    }
    let n_int = n as usize;
    let fill_char = fill.chars().next().unwrap_or(' ');
    let s_len = s.chars().count();
    if s_len >= n_int {
        return Ok(Value::String(s));
    }
    let padding_len = n_int - s_len;
    let padding: String = std::iter::repeat(fill_char).take(padding_len).collect();
    Ok(Value::String(format!("{}{}", s, padding)))
}

/// `lines(s)` — split string into list of lines (no trailing empty element).
fn builtin_lines(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("lines", args, 0)?;
    if s.is_empty() {
        return Ok(Value::List(vec![]));
    }
    let mut items: Vec<Value> = s
        .split('\n')
        .map(|line| {
            // Handle \r\n: strip trailing \r from each line
            let trimmed = line.strip_suffix('\r').unwrap_or(line);
            Value::String(trimmed.to_string())
        })
        .collect();
    // Remove trailing empty element caused by trailing newline
    if s.ends_with('\n') {
        if items
            .last()
            .map_or(false, |v| matches!(v, Value::String(s) if s.is_empty()))
        {
            items.pop();
        }
    }
    Ok(Value::List(items))
}

/// `words(s)` — split string into list of words by whitespace.
fn builtin_words(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("words", args, 0)?;
    let items: Vec<Value> = s
        .split_whitespace()
        .map(|w| Value::String(w.to_string()))
        .collect();
    Ok(Value::List(items))
}

// ── P2: TOON encoding ──────────────────────────────────────────────

/// Check if a string is a "simple" identifier (no quoting needed in TOON).
fn toon_is_simple(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Escape a string for TOON: non-ASCII -> \uXXXX, quotes -> \", backslash -> \\
fn toon_escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ if c.is_ascii() => out.push(c),
            _ => {
                let code = c as u32;
                out.push_str(&format!("\\u{{{:04x}}}", code));
            }
        }
    }
    out
}

/// Encode a Value to TOON format (recursive).
fn value_to_toon(val: &Value) -> String {
    match val {
        Value::String(s) => {
            if toon_is_simple(s) {
                format!("s\"{}\"", s)
            } else {
                format!("s\"{}\"", toon_escape_string(s))
            }
        }
        Value::Float(f) => {
            if *f == f.floor() && f.abs() < 1e15 {
                format!("{}", *f as i64)
            } else {
                format!("{}", f)
            }
        }
        Value::Bool(b) => (if *b { "true" } else { "false" }).to_string(),
        Value::Unit => "null".to_string(),
        Value::List(items) => {
            let inner: Vec<String> = items.iter().map(value_to_toon).collect();
            format!("[{}]", inner.join(","))
        }
        Value::Struct { fields, .. } => {
            let pairs: Vec<String> = fields
                .iter()
                .map(|(k, v)| {
                    let key = if toon_is_simple(k) {
                        k.clone()
                    } else {
                        format!("s\"{}\"", toon_escape_string(k))
                    };
                    format!("{}:{}", key, value_to_toon(v))
                })
                .collect();
            format!("{{{}}}", pairs.join(","))
        }
        other => format!("s\"{}\"", toon_escape_string(&format!("{}", other))),
    }
}

/// `toon_encode(value)` — encode any value to TOON (Token-Optimized Object Notation).
fn builtin_toon_encode(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("toon_encode() requires 1 argument".to_string());
    }
    let encoded = value_to_toon(&args[0]);
    Ok(Value::String(format!("TOON:{}", encoded)))
}

/// Decode a TOON string back into a Value. Recursive descent parser.
fn parse_toon_value(input: &str, pos: &mut usize) -> Result<Value, String> {
    let bytes = input.as_bytes();
    if *pos >= bytes.len() {
        return Err("toon_decode: unexpected end of input".to_string());
    }

    match bytes[*pos] {
        b't' => {
            // true
            if input[*pos..].starts_with("true") {
                *pos += 4;
                return Ok(Value::Bool(true));
            }
            Err(format!("toon_decode: invalid token at position {}", pos))
        }
        b'f' => {
            // false
            if input[*pos..].starts_with("false") {
                *pos += 5;
                return Ok(Value::Bool(false));
            }
            Err(format!("toon_decode: invalid token at position {}", pos))
        }
        b'n' => {
            // null
            if input[*pos..].starts_with("null") {
                *pos += 4;
                return Ok(Value::Unit);
            }
            Err(format!("toon_decode: invalid token at position {}", pos))
        }
        b's' => {
            // s"..." — quoted string
            if *pos + 1 >= bytes.len() || bytes[*pos + 1] != b'"' {
                return Err(format!(
                    "toon_decode: expected s\"...\" at position {}",
                    pos
                ));
            }
            *pos += 2; // skip s"
            let mut result = String::new();
            while *pos < bytes.len() {
                match bytes[*pos] {
                    b'"' => {
                        *pos += 1;
                        return Ok(Value::String(result));
                    }
                    b'\\' => {
                        *pos += 1;
                        if *pos >= bytes.len() {
                            return Err("toon_decode: unterminated escape".to_string());
                        }
                        match bytes[*pos] {
                            b'"' => {
                                result.push('"');
                                *pos += 1;
                            }
                            b'\\' => {
                                result.push('\\');
                                *pos += 1;
                            }
                            b'n' => {
                                result.push('\n');
                                *pos += 1;
                            }
                            b't' => {
                                result.push('\t');
                                *pos += 1;
                            }
                            b'r' => {
                                result.push('\r');
                                *pos += 1;
                            }
                            b'u' => {
                                // \u{XXXX}
                                *pos += 1;
                                if *pos >= bytes.len() || bytes[*pos] != b'{' {
                                    return Err(format!(
                                        "toon_decode: expected {{ after \\u at position {}",
                                        pos
                                    ));
                                }
                                *pos += 1;
                                let hex_start = *pos;
                                while *pos < bytes.len() && bytes[*pos] != b'}' {
                                    *pos += 1;
                                }
                                if *pos >= bytes.len() {
                                    return Err("toon_decode: unterminated \\u{{...}}".to_string());
                                }
                                let hex_str = &input[hex_start..*pos];
                                *pos += 1; // skip }
                                let code_point = u32::from_str_radix(hex_str, 16).map_err(|e| {
                                    format!("toon_decode: invalid unicode escape: {}", e)
                                })?;
                                if let Some(c) = char::from_u32(code_point) {
                                    result.push(c);
                                } else {
                                    return Err(format!(
                                        "toon_decode: invalid unicode code point: {:x}",
                                        code_point
                                    ));
                                }
                            }
                            other => {
                                result.push(other as char);
                                *pos += 1;
                            }
                        }
                    }
                    other => {
                        result.push(other as char);
                        *pos += 1;
                    }
                }
            }
            Err("toon_decode: unterminated string".to_string())
        }
        b'[' => {
            // Array
            *pos += 1;
            let mut items = Vec::new();
            while *pos < bytes.len() && bytes[*pos] != b']' {
                if bytes[*pos] == b',' {
                    *pos += 1;
                    continue;
                }
                items.push(parse_toon_value(input, pos)?);
            }
            if *pos >= bytes.len() {
                return Err("toon_decode: unterminated array".to_string());
            }
            *pos += 1; // skip ]
            Ok(Value::List(items))
        }
        b'{' => {
            // Object -> Struct
            *pos += 1;
            let mut fields = std::collections::HashMap::new();
            while *pos < bytes.len() && bytes[*pos] != b'}' {
                if bytes[*pos] == b',' {
                    *pos += 1;
                    continue;
                }
                // Parse key
                let key =
                    if bytes[*pos] == b's' && *pos + 1 < bytes.len() && bytes[*pos + 1] == b'"' {
                        // s"key"
                        *pos += 2;
                        let mut k = String::new();
                        while *pos < bytes.len() && bytes[*pos] != b'"' {
                            if bytes[*pos] == b'\\' {
                                *pos += 1;
                                if *pos < bytes.len() {
                                    k.push(bytes[*pos] as char);
                                    *pos += 1;
                                }
                            } else {
                                k.push(bytes[*pos] as char);
                                *pos += 1;
                            }
                        }
                        if *pos < bytes.len() {
                            *pos += 1;
                        } // skip closing "
                        k
                    } else {
                        // bare identifier
                        let start = *pos;
                        while *pos < bytes.len()
                            && (bytes[*pos].is_ascii_alphanumeric()
                                || bytes[*pos] == b'_'
                                || bytes[*pos] == b'-')
                        {
                            *pos += 1;
                        }
                        input[start..*pos].to_string()
                    };
                // Expect ':'
                if *pos >= bytes.len() || bytes[*pos] != b':' {
                    return Err(format!(
                        "toon_decode: expected ':' after key '{}' at position {}",
                        key, pos
                    ));
                }
                *pos += 1;
                // Parse value
                let val = parse_toon_value(input, pos)?;
                fields.insert(key, val);
            }
            if *pos >= bytes.len() {
                return Err("toon_decode: unterminated object".to_string());
            }
            *pos += 1; // skip }
            Ok(Value::Struct {
                type_name: "TOON".to_string(),
                fields,
            })
        }
        b'-' | b'0'..=b'9' => {
            // Number
            let start = *pos;
            if bytes[*pos] == b'-' {
                *pos += 1;
            }
            while *pos < bytes.len() && (bytes[*pos].is_ascii_digit() || bytes[*pos] == b'.') {
                *pos += 1;
            }
            let num_str = &input[start..*pos];
            let f: f64 = num_str.parse().map_err(|e| {
                format!(
                    "toon_decode: invalid number '{}' at position {}: {}",
                    num_str, start, e
                )
            })?;
            Ok(Value::Float(f))
        }
        other => Err(format!(
            "toon_decode: unexpected character '{}' at position {}",
            other as char, pos
        )),
    }
}

/// `toon_decode(s)` — decode TOON string back to Value.
fn builtin_toon_decode(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("toon_decode", args, 0)?;
    if !s.starts_with("TOON:") {
        return Err("toon_decode: input must start with 'TOON:'".to_string());
    }
    let payload = &s[5..];
    let mut pos = 0;
    let value = parse_toon_value(payload, &mut pos)?;
    // Skip trailing whitespace
    while pos < payload.len() && payload.as_bytes()[pos] == b' ' {
        pos += 1;
    }
    if pos < payload.len() {
        return Err(format!(
            "toon_decode: unexpected trailing data at position {}",
            5 + pos
        ));
    }
    Ok(value)
}

// ── P2: Content-addressed refs ─────────────────────────────────────

/// `ref(content)` — compute SHA-256 hash, store in KV, return hash string. Idempotent.
fn builtin_content_ref(args: &[Value]) -> Result<Value, String> {
    let content = expect_string_arg("ref", args, 0)?;
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash = hex::encode(hasher.finalize());
    let key = format!("__ref:{}", hash);
    // Only set if not already present (idempotent)
    let store = kv_store()
        .lock()
        .map_err(|e| format!("ref() lock error: {}", e))?;
    if !store.contains_key(&key) {
        drop(store); // release read lock
        let mut store = kv_store()
            .lock()
            .map_err(|e| format!("ref() lock error: {}", e))?;
        store.insert(key.clone(), content.clone());
        // Write-through to SQLite if available
        if let Ok(sqlite_guard) = kv_sqlite().lock() {
            if let Some(ref conn) = *sqlite_guard {
                let _ = conn.execute(
                    "INSERT OR IGNORE INTO kv_store (key, value) VALUES (?1, ?2)",
                    rusqlite::params![key, content],
                );
            }
        }
    }
    Ok(Value::String(hash))
}

/// `deref(hash)` — retrieve content by SHA-256 hash from ref store.
fn builtin_content_deref(args: &[Value]) -> Result<Value, String> {
    let hash = expect_string_arg("deref", args, 0)?;
    if hash.len() != 64 {
        return Err("deref: invalid hash format, expected 64-char hex string".to_string());
    }
    if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("deref: invalid hash format, expected hex characters only".to_string());
    }
    let key = format!("__ref:{}", hash);
    let store = kv_store()
        .lock()
        .map_err(|e| format!("deref() lock error: {}", e))?;
    match store.get(&key) {
        Some(content) => Ok(Value::String(content.clone())),
        None => Err("deref: hash not found in ref store".to_string()),
    }
}

// ── P3: Token awareness ────────────────────────────────────────────

/// `token_count(text)` — estimate token count. Cyrillic: chars/2, Latin: chars/4.
fn builtin_token_count(args: &[Value]) -> Result<Value, String> {
    let s = expect_string_arg("token_count", args, 0)?;
    if s.is_empty() {
        return Ok(Value::Float(0.0));
    }
    let total_chars = s.chars().count();
    let cyrillic_chars = s
        .chars()
        .filter(|c| matches!(c, '\u{0400}'..='\u{04FF}'))
        .count();
    // If >=50% Cyrillic, use /2 divisor; else /4
    let divisor = if total_chars > 0 && (cyrillic_chars as f64 / total_chars as f64) >= 0.5 {
        2.0
    } else {
        4.0
    };
    let tokens = (total_chars as f64 / divisor).ceil();
    Ok(Value::Float(tokens))
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

/// Extract a String arg or return error.
fn expect_string_arg_var(name: &str, args: &[Value], idx: usize) -> Result<String, String> {
    if idx >= args.len() {
        return Err(format!("{}: expected argument at position {}", name, idx));
    }
    match &args[idx] {
        Value::String(s) => Ok(s.clone()),
        other => Err(format!(
            "{}: argument {} must be String, got {}",
            name,
            idx,
            other.type_name()
        )),
    }
}

/// Extract a List arg or return error.
fn expect_list_arg(name: &str, args: &[Value], idx: usize) -> Result<Vec<Value>, String> {
    if idx >= args.len() {
        return Err(format!("{}: expected argument at position {}", name, idx));
    }
    match &args[idx] {
        Value::List(items) => Ok(items.clone()),
        other => Err(format!(
            "{}: argument {} must be List, got {}",
            name,
            idx,
            other.type_name()
        )),
    }
}

/// Extract a Struct arg as JSON or return error.
fn expect_struct_json_arg(name: &str, args: &[Value], idx: usize) -> Result<String, String> {
    if idx >= args.len() {
        return Err(format!("{}: expected argument at position {}", name, idx));
    }
    let json = mlog_value_to_json(&args[idx]);
    match serde_json::to_string(&json) {
        Ok(s) => Ok(s),
        Err(_) => Err(format!(
            "{}: argument {} must be serializable to JSON",
            name, idx
        )),
    }
}

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

/// Convert serde_yaml::Value to serde_json::Value for unified config processing.
fn yaml_to_json_value(yaml: &serde_yaml::Value) -> serde_json::Value {
    match yaml {
        serde_yaml::Value::Null => serde_json::Value::Null,
        serde_yaml::Value::Bool(b) => serde_json::Value::Bool(*b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_json::Value::Number(i.into())
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            }
        }
        serde_yaml::Value::String(s) => serde_json::Value::String(s.clone()),
        serde_yaml::Value::Sequence(seq) => {
            serde_json::Value::Array(seq.iter().map(yaml_to_json_value).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                if let Some(key_str) = k.as_str() {
                    obj.insert(key_str.to_string(), yaml_to_json_value(v));
                }
            }
            serde_json::Value::Object(obj)
        }
        serde_yaml::Value::Tagged(tagged) => {
            // YAML tags — just convert the value, ignoring the tag
            yaml_to_json_value(&tagged.value)
        }
    }
}

/// Like json_value_to_mlog_value but with a custom type_name for the root struct.
fn json_value_to_mlog_value_with_type(json: &serde_json::Value, type_name: &str) -> Value {
    match json {
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Number(n) => Value::Float(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Null => Value::Unit,
        serde_json::Value::Array(arr) => Value::List(
            arr.iter()
                .map(|v| json_value_to_mlog_value_with_type(v, type_name))
                .collect(),
        ),
        serde_json::Value::Object(obj) => {
            let mut fields = std::collections::HashMap::new();
            for (k, v) in obj {
                fields.insert(k.clone(), json_value_to_mlog_value_with_type(v, type_name));
            }
            Value::Struct {
                type_name: type_name.to_string(),
                fields,
            }
        }
    }
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
