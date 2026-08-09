use super::*;

/// Master registry of ALL builtin functions.
/// Order determines bytecode indices — DO NOT reorder existing entries.
/// To add a new builtin: append a `spec!` row here,
/// add the handler in Builtins::new(), and you're done.
pub const BUILTIN_REGISTRY: &[BuiltinSpec] = &[
    // ── String builtins ──
    spec!("upper", 1, "string"),
    spec!("lower", 1, "string"),
    spec!("len", 1, "string"),
    spec!("str", 1, "string"),
    spec!("contains", 2, "string"),
    spec!("index_of", 2, "string"),
    spec!("substring", 3, "string"),
    spec!("char_at", 2, "string"),
    spec!("starts_with", 2, "string"),
    spec!("ends_with", 2, "string"),
    spec!("trim", 1, "string"),
    spec!("replace", 3, "string"),
    spec!("split", 2, "string"),
    spec!("join", 2, "string"),
    spec!("length", 1, "string"),
    spec!("reverse", 1, "string"),
    spec!("escape_html", 1, "string"),
    spec!("escape_json", 1, "string"),
    spec!("escape_js", 1, "string"),
    spec!("fuzzy_match", 2, "string"),
    spec!("strip", 2, "string"),
    spec!("chomp", 1, "string"),
    spec!("repeat", 2, "string"),
    spec!("pad_left", 3, "string"),
    spec!("pad_right", 3, "string"),
    spec!("lines", 1, "string"),
    spec!("words", 1, "string"),
    spec!("token_count", 1, "string"),
    spec!("type_of", 1, "string"),
    spec!("format", 0, "string"), // variadic: 1 template + N fill args
    // ── Stdlib backing (double-underscore prefix) ──
    spec!("__trim", 1, "std"),
    spec!("__replace", 3, "std"),
    spec!("__split", 2, "std"),
    spec!("__join", 2, "std"),
    spec!("__abs", 1, "std"),
    spec!("__min", 2, "std"),
    spec!("__max", 2, "std"),
    spec!("__clamp", 3, "std"),
    spec!("__round", 1, "std"),
    spec!("__first", 1, "std"),
    spec!("__last", 1, "std"),
    spec!("__push", 2, "stub"), // no handler; planned stdlib helper, superseded by 'push' builtin
    spec!("__list_len", 1, "stub"), // no handler; planned stdlib helper, use 'len' instead
    // ── Stub entries for VM coverage ──
    // abs/min/max/clamp/round: public aliases for __abs/__min/__max/__clamp/__round;
    // no handler registered yet (double-underscore versions are wired instead)
    spec!("abs", 1, "stub"),
    spec!("min", 2, "stub"),
    spec!("max", 2, "stub"),
    spec!("clamp", 3, "stub"),
    spec!("round", 1, "stub"),
    // newline/stdin: not yet implemented; no handler
    spec!("newline", 0, "stub"),
    spec!("stdin", 0, "stub"),
    // split_tokens/if_eq/is_string_token: planned compiler helpers; no handler
    spec!("split_tokens", 0, "stub"),
    spec!("if_eq", 3, "stub"),
    spec!("is_string_token", 1, "stub"),
    // db_insert: planned convenience wrapper for INSERT; no handler (use db_execute instead)
    spec!("db_insert", 0, "stub"),
    // ── Convert builtins ──
    spec!("float", 1, "convert"),
    spec!("to_string", 1, "convert"),
    spec!("to_float", 1, "convert"),
    // ── IO builtins ──
    spec!("print", 1, "io"),
    spec!("read_file", 1, "io"),
    spec!("write_file", 2, "io"),
    spec!("append_file", 2, "io"),
    spec!("delete_file", 1, "io"),
    spec!("file_exists", 1, "io"),
    spec!("list_dir", 1, "io"),
    spec!("exec", 1, "io"),
    spec!("git_push", 1, "io"),
    // ── List builtins ──
    spec!("get", 2, "list"),
    spec!("push", 2, "list"),
    spec!("slice", 3, "list"),
    spec!("zip", 2, "list"),
    spec!("sort_by", 2, 3, "list"),
    spec!("filter", 3, "list"),
    spec!("reduce", 3, "list"),
    spec!("dedup", 1, "list"),
    spec!("condense", 1, "list"),
    spec!("first", 1, "list"),
    spec!("last", 1, "list"),
    spec!("make_list", 0, "list"),
    spec!("matches_any", 2, "list"),
    // ── JSON builtins ──
    spec!("parse_json", 1, 2, "json"),
    spec!("json_encode", 1, "json"),
    spec!("json_get", 2, 3, "json"),
    spec!("has_field", 2, "json"),
    spec!("dict_get", 3, "json"),
    spec!("dict_set", 3, "json"),
    spec!("dict_has", 2, "json"),
    spec!("dict_keys", 1, "json"),
    spec!("dict_values", 1, "json"),
    // ── Web builtins ──
    spec!("respond", 1, 2, "web"),
    spec!("respond_html", 1, "web"),
    spec!("form_data", 1, "web"),
    spec!("json_body", 0, "web"),
    spec!("query_param", 1, "web"),
    spec!("render", 2, 3, "web"),
    spec!("http_get", 1, 3, "web"), // url | url,headers | url,headers,timeout
    spec!("http_post", 2, 5, "web"),
    spec!("http_post_multipart", 2, 4, "web"),
    spec!("require", 1, 2, "web"),
    spec!("request_body", 0, "web"),
    spec!("web_search", 1, 2, "web"), // query | query,num
    spec!("geo_ip", 0, 1, "web"),     // ip? (omit = caller IP)
    spec!("weather", 2, "web"),
    spec!("geo_distance", 2, 5, "web"),
    spec!("weather_forecast", 1, 3, "web"), // city | lat,lon | lat,lon,days
    // ── Crypto builtins ──
    spec!("hash_password", 1, "crypto"),
    spec!("verify_password", 2, "crypto"),
    spec!("encrypt", 2, "crypto"),
    spec!("decrypt", 2, "crypto"),
    spec!("generate_key", 0, "crypto"),
    spec!("base64_encode", 1, "encoding"),
    spec!("base64_decode", 1, "encoding"),
    // ── Auth stubs (interpreter-mode mocks; real auth requires server mode) ──
    // authenticate: always returns Unit — mock; no user database in interpreter
    spec!("authenticate", 2, "stub"),
    // session_login: returns empty Session HashMap — mock; no auth backend in interpreter
    spec!("session_login", 2, "stub"),
    // session_logout: no-op — mock; no sessions to invalidate in interpreter
    spec!("session_logout", 1, "stub"),
    spec!("session_clear", 0, "memory"),
    // ── Bot — Telegram messaging ──
    spec!("send_message", 2, 3, "bot"), // chat_id,text | +reply_markup
    spec!("answer_callback_query", 1, 3, "bot"), // id | id,text | id,text,show_alert
    spec!("edit_message_text", 3, 4, "bot"), // chat_id,message_id,text | +reply_markup
    // ── Voice / transcription ──
    spec!("whisper_transcribe", 1, "voice"),
    spec!("tts_send", 4, 5, "voice"), // text,voice,bot_token,chat_id | +mode
    // ── System builtins ──
    spec!("env", 1, "system"),
    // ── DB builtins ──
    spec!("query", 1, 2, "db"),
    spec!("db_execute", 1, 2, "db"), // ADR-0068: optional params list
    // ── LLM builtins ──
    spec!("call_llm", 1, 2, "llm"), // prompt | prompt,input
    spec!("call_claude", 4, "llm"), // api_key,model,system,user
    spec!("llm_usage", 0, "llm"),
    // ── Memory builtins ──
    spec!("kv_set", 2, "memory"),
    spec!("kv_get", 1, "memory"),
    spec!("kv_delete", 1, "memory"),
    spec!("kv_exists", 1, "memory"),
    spec!("kv_list", 0, "memory"),
    spec!("mem_set", 2, "memory"),
    spec!("mem_get", 1, "memory"),
    spec!("mem_delete", 1, "memory"),
    spec!("memorize", 2, 3, "memory"),
    // recall/forget/find/inspect: planned high-level memory API; no handler (use kv_*/mem_* instead)
    spec!("recall", 0, "stub"),
    spec!("forget", 0, "stub"),
    spec!("find", 4, "stub"),
    spec!("inspect", 1, "stub"),
    // conv_start/add/history/context/end: conversation lifecycle management; not yet implemented
    spec!("conv_start", 1, "stub"),
    spec!("conv_add", 3, "stub"),
    spec!("conv_history", 1, "stub"),
    spec!("conv_context", 1, "stub"),
    spec!("conv_end", 1, "stub"),
    spec!("session_set", 2, "memory"),
    spec!("session_get", 1, "memory"),
    spec!("ref", 1, "memory"),
    spec!("deref", 1, "memory"),
    // ── Time builtins ──
    spec!("now", 0, "time"),
    spec!("time", 0, "time"),
    spec!("add_days", 2, "time"),
    spec!("add_hours", 2, "time"),
    spec!("date_parts", 1, "time"),
    spec!("format_date", 2, "time"),
    spec!("days_between", 2, "time"),
    spec!("days_in_month", 2, "time"),
    spec!("is_leap_year", 1, "time"),
    spec!("weekday_name", 1, "time"),
    // ── Graph builtins ──
    spec!("graph_query", 1, 3, "graph"), // query | query,limit | query,limit,level
    spec!("graph_path", 2, "graph"),     // from_id,to_id
    spec!("graph_neighbors", 0, "graph"),
    spec!("memory_decay", 0, "memory"),
    spec!("memory_boost", 0, "memory"),
    spec!("memory_prune", 0, "memory"),
    spec!("memory_revise", 0, "memory"),
    spec!("subgraph_extract", 0, "graph"),
    spec!("subgraph_nodes", 0, "graph"),
    spec!("subgraph_json", 0, "graph"),
    spec!("trace_start", 0, "graph"),
    spec!("trace_end", 0, "graph"),
    spec!("memory_score", 1, "bot" => "ext"),
    spec!("mtree_summarize", 0, "mtree"),
    spec!("mtree_retrieve", 1, 2, "mtree"), // query | query,limit
    spec!("mtree_store", 2, "mtree"),
    spec!("mtree_stats", 0, "mtree"),
    spec!("mtree_forget", 1, "mtree"),
    // ── Cron builtins ──
    spec!("cron_mark_fired", 1, "cron"),
    spec!("cron_add", 2, "cron"), // cron_expr, prompt
    spec!("cron_list", 0, "cron"),
    spec!("cron_remove", 1, "cron"),
    spec!("cron_run", 1, "cron"),
    // ── Event / query analytics stubs ──
    // event_count/events_since/event_sum: planned event analytics; no handler (use query with SQL instead)
    spec!("event_count", 0, "stub"),
    spec!("events_since", 1, "stub"),
    spec!("event_sum", 2, "stub"),
    // query_scalar/query_row: planned convenience wrappers; no handler (use query + json_get instead)
    spec!("query_scalar", 0, "stub"),
    spec!("query_row", 0, "stub"),
    // ── Test builtins ──
    spec!("assert_eq", 2, "test"),
    spec!("assert_contains", 2, "test"),
    // ── Fluid builtins ──
    spec!("confidence", 1, "fluid"),
    // ── Encoding builtins ──
    spec!("toon_encode", 1, "encoding"),
    spec!("toon_decode", 1, "encoding"),
    // ── Recipe / DAG / Orchestration builtins ──
    spec!("recipe_save", 0, "recipe" => "ext"),
    spec!("recipe_search", 0, "stub" => "ext"), // returns empty list; KV access limited from pure builtins
    spec!("recipe_list", 0, "recipe" => "ext"),
    spec!("dag_phases", 1, "orchestration" => "ext"),
    spec!("topo_sort", 1, "orchestration" => "ext"),
    spec!("resolve_skill_index", 1, "stub"), // planned skill resolution; no handler
    spec!("fit_to_budget", 0, "stub"),       // planned budget planner; no handler
    spec!("map", 0, "stub"), // planned list mapper; no handler (use filter+reduce instead)
    // ── OpenPlanter-inspired: fuzzy / safe editing / agent utilities ──
    spec!("fuzzy_find_best", 2, "string"),
    spec!("hashline_read", 1, "string"),
    spec!("hashline_edit", 2, "string"),
    spec!("compact_list", 3, "list"),
    spec!("budget_check", 2, "fluid"),
    spec!("replay_snapshot", 1, "system"),
    spec!("policy_check", 1, "system"),
    // ── obsidian-mind: Vault / semantic search ──
    spec!("semantic_search", 3, "vault" => "ext"),
    spec!("config_load", 1, "vault" => "ext"),
    spec!("vault_validate", 2, "vault" => "ext"),
    // ── Bot — Telegram ──
    spec!("todo_add", 2, "bot" => "ext"),
    spec!("todo_list", 0, "bot" => "ext"),
    spec!("todo_update", 2, "bot" => "ext"),
    spec!("goal_get", 0, "bot" => "ext"),
    spec!("goal_set", 2, "bot" => "ext"),
    spec!("goals_add", 1, "bot" => "ext"),
    spec!("goals_list", 0, "bot" => "ext"),
    spec!("remind", 3, "bot"),
    spec!("get_profile", 0, "bot" => "ext"),
    spec!("human_mood", 3, "bot"),
    spec!("ask_approval", 1, "bot" => "ext"),
    spec!("goal_complete", 0, "bot" => "ext"),
    spec!("goals_reflect", 0, "bot" => "ext"),
    spec!("cancel_remind", 1, "bot"),
    spec!("check_reminders", 0, "bot"),
    spec!("list_reminders", 0, "bot"),
    spec!("remind_recurring", 2, "bot"),
    spec!("human_create", 2, "bot"),
    spec!("human_delete", 1, "bot" => "ext"),
    spec!("human_forget", 2, "bot"),
    spec!("human_personas", 0, "bot" => "ext"),
    spec!("human_recall", 3, "bot"),
    spec!("human_remember", 4, "bot"),
    spec!("human_respond", 2, "bot" => "ext"),
    spec!("compress_html", 1, "bot" => "ext"),
    spec!("estimate_tokens", 1, "bot" => "ext"),
    spec!("extract_entities", 1, "bot" => "ext"),
    spec!("extract_param", 2, "bot" => "ext"), // text,index
    spec!("learn_preference", 2, "bot" => "ext"),
    spec!("read_file_tokens", 1, "bot" => "ext"),
    // ── sqz-inspired: string/list utilities ──
    spec!("squeeze", 2, "string"),
    spec!("to_int", 1, "string"), // parse string/float to integer
    // ── PDF processing (Наряд №48) ──
    spec!("pdf_classify", 1, "pdf"),
    spec!("pdf_to_markdown", 1, "pdf"),
    spec!("pdf_extract_regions", 2, "pdf"),
    spec!("pdf_ocr", 1, "pdf"),
    // ── Crypto: SHA-256 / HMAC (Наряд №50 Block 3) ──
    spec!("sha256", 1, "crypto"),
    spec!("hmac_sha256", 2, "crypto"),
    spec!("hex_encode", 1, "crypto"),
    spec!("hex_decode", 1, "crypto"),
    // ── Regex (Наряд №54) ──
    spec!("regex_match", 2, "string"),
    spec!("regex_captures", 2, "string"),
    spec!("regex_replace", 3, "string"),
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

/// Check if calling a builtin with the given argument count is valid.
pub fn check_builtin_arity(name: &str, arg_count: usize) -> Result<(), String> {
    for spec in BUILTIN_REGISTRY {
        if spec.name == name {
            if spec.arity == 0 && spec.max_arity.is_none() {
                return Ok(()); // truly variadic (no bounds)
            }
            let max = spec.max_arity.unwrap_or(spec.arity);
            if arg_count >= spec.arity && arg_count <= max {
                return Ok(());
            }
            return Err(format!(
                "function '{}' expects {}{} argument(s), got {}",
                name,
                spec.arity,
                if let Some(m) = spec.max_arity {
                    if m != spec.arity {
                        format!("..{}", m)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                },
                arg_count
            ));
        }
    }
    Ok(())
}
