use super::*;

/// Master registry of ALL builtin functions.
/// Order determines bytecode indices — DO NOT reorder existing entries.
/// To add a new builtin: append a `spec!` row here,
/// add the handler in Builtins::new(), and you're done.
pub const BUILTIN_REGISTRY: &[BuiltinSpec] = &[
    // ── String builtins ──
    spec!("upper", 1, "string"; builtin_upper),
    spec!("lower", 1, "string"; builtin_lower),
    spec!("len", 1, "string"; builtin_len),
    spec!("str", 1, "string"; builtin_str),
    spec!("contains", 2, "string"; builtin_contains),
    spec!("index_of", 2, "string"; builtin_index_of),
    spec!("substring", 3, "string"; builtin_substring),
    spec!("char_at", 2, "string"; builtin_char_at),
    spec!("starts_with", 2, "string"; builtin_starts_with),
    spec!("ends_with", 2, "string"; builtin_ends_with),
    spec!("trim", 1, "string"; builtin_trim),
    spec!("replace", 3, "string"; builtin_replace),
    spec!("split", 2, "string"; builtin_split),
    spec!("join", 2, "string"; builtin_join),
    spec!("length", 1, "string"; builtin_length),
    spec!("reverse", 1, "string"; builtin_reverse),
    spec!("escape_html", 1, "string"; builtin_escape_html),
    spec!("escape_json", 1, "string"; builtin_escape_json),
    spec!("escape_js", 1, "string"; builtin_escape_js),
    spec!("fuzzy_match", 2, "string"; builtin_fuzzy_match),
    spec!("strip", 2, "string"; builtin_strip),
    spec!("chomp", 1, "string"; builtin_chomp),
    spec!("repeat", 2, "string"; builtin_repeat),
    spec!("pad_left", 3, "string"; builtin_pad_left),
    spec!("pad_right", 3, "string"; builtin_pad_right),
    spec!("lines", 1, "string"; builtin_lines),
    spec!("words", 1, "string"; builtin_words),
    spec!("token_count", 1, "string"; builtin_token_count),
    spec!("type_of", 1, "string"; builtin_type_of),
    spec!("format", 0, "string"; builtin_format), // variadic: 1 template + N fill args
    // НАРЯД №117: missing string utilities
    spec!("trim_start", 1, "string"; builtin_trim_start),
    spec!("trim_end", 1, "string"; builtin_trim_end),
    spec!("truncate", 2, "string"; builtin_truncate),
    spec!("slugify", 1, "string"; builtin_slugify),
    spec!("word_wrap", 2, "string"; builtin_word_wrap),
    spec!("capitalize", 1, "string"; builtin_capitalize),
    spec!("title_case", 1, "string"; builtin_title_case),
    // ── Stdlib backing (double-underscore prefix) ──
    spec!("__trim", 1, "std"; builtin_trim),
    spec!("__replace", 3, "std"; builtin_replace),
    spec!("__split", 2, "std"; builtin_split),
    spec!("__join", 2, "std"; builtin_join),
    spec!("__abs", 1, "std"; builtin_abs),
    spec!("__min", 2, "std"; builtin_min),
    spec!("__max", 2, "std"; builtin_max),
    spec!("__clamp", 3, "std"; builtin_clamp),
    spec!("__round", 1, "std"; builtin_round),
    spec!("__first", 1, "std"; builtin_first),
    spec!("__last", 1, "std"; builtin_last),
    // ── Math builtins (public aliases for __abs/__min/__max/__clamp/__round) ──
    spec!("abs", 1, "math"; builtin_abs),
    spec!("min", 2, "math"; builtin_min),
    spec!("max", 2, "math"; builtin_max),
    spec!("clamp", 3, "math"; builtin_clamp),
    spec!("round", 1, "math"; builtin_round),
    // Наряд №177: Math foundation for Reflex (stage 1/6)
    spec!("exp", 1, "math"; builtin_exp),
    spec!("ln", 1, "math"; builtin_ln),
    spec!("sqrt", 1, "math"; builtin_sqrt),
    spec!("pow", 2, "math"; builtin_pow),
    spec!("tanh", 1, "math"; builtin_tanh),
    spec!("sigmoid", 1, "math"; builtin_sigmoid),
    spec!("softmax", 1, "math"; builtin_softmax),
    spec!("random_seed", 1, "math"; builtin_random_seed),
    spec!("random", 0, "math"; builtin_random), // ── Phase 4.4 self-hosting — historical placeholders, never implemented ──
    // ADR-0023 described a hybrid lexer approach using 5 builtins (stdin,
    // split_tokens, if_eq, newline, is_string_token). Handler functions were
    // never committed to main — only the builtin names were registered as
    // bytecode opcode indices (commit b3f5921, 2026-06-02). The actual
    // self-host/lexer.mlog was rewritten to use pure Metalogos constructs
    // (if/then/else, literal "\n", char_at/index_of/substring) and does
    // not depend on these builtins.
    //
    // The lexer.mlog itself remains non-functional (test self_host_lexer
    // is #[ignore] since commit e61bd66, reason: "produces no output —
    // needs investigation"). That failure is unrelated to these 5 stubs.
    //
    // These spec! entries are kept ONLY for bytecode index stability.
    // DO NOT remove without .mbc format version bump.
    // See ADR-0023 (naряд №73 re-measurement) for full history.
    spec!("newline", 0, "stub"),
    spec!("stdin", 0, "stub"),
    spec!("split_tokens", 0, "stub"),
    spec!("if_eq", 3, "stub"),
    spec!("is_string_token", 1, "stub"), // db_insert: planned convenience wrapper for INSERT; no handler (use db_execute instead)
    spec!("db_insert", 0, "stub"),       // ── Convert builtins ──
    spec!("float", 1, "convert"; builtin_float),
    spec!("to_string", 1, "convert"; builtin_to_string),
    spec!("to_float", 1, "convert"; builtin_to_float), // ── IO builtins ──
    spec!("print", 1, "io"; builtin_print),
    spec!("read_file", 1, "io"; builtin_read_file),
    spec!("write_file", 2, "io"; builtin_write_file),
    spec!("append_file", 2, "io"; builtin_append_file),
    spec!("delete_file", 1, "io"; builtin_delete_file),
    spec!("file_exists", 1, "io"; builtin_file_exists),
    spec!("list_dir", 1, "io"; builtin_list_dir),
    spec!("exec", 1, "io"; builtin_exec),
    spec!("exec_argv", 1, 2, "io"; builtin_exec_argv), // binary required, args list optional
    spec!("git_push", 1, "io"; builtin_git_push),      // ── List builtins ──
    spec!("get", 2, "list"; builtin_get),
    spec!("push", 2, "list"; builtin_push),
    spec!("slice", 3, "list"; builtin_slice),
    spec!("zip", 2, "list"; builtin_zip),
    spec!("sort_by", 2, 3, "list"; builtin_sort_by),
    spec!("filter", 3, "list"; builtin_filter),
    spec!("reduce", 3, "list"; builtin_reduce),
    spec!("dedup", 1, "list"; builtin_dedup),
    spec!("condense", 1, "list"; builtin_condense),
    // НАРЯД №118: collection utilities (unique, chunk, sort)
    spec!("unique", 1, "list"; builtin_unique),
    spec!("chunk", 2, "list"; builtin_chunk),
    spec!("sort", 1, "list"; builtin_sort),
    spec!("first", 1, "list"; builtin_first),
    spec!("last", 1, "list"; builtin_last),
    spec!("make_list", 0, "list"; builtin_make_list),
    spec!("matches_any", 2, "list"; builtin_matches_any), // ── JSON builtins ──
    spec!("parse_json", 1, 2, "json"; builtin_parse_json),
    spec!("json_encode", 1, "json"; builtin_json_encode),
    spec!("json_get", 2, 3, "json"; builtin_json_get),
    spec!("has_field", 2, "json"; builtin_has_field),
    spec!("dict_get", 3, "json"; builtin_json_get),
    spec!("dict_set", 3, "json"; builtin_dict_set),
    spec!("dict_has", 2, "json"; builtin_dict_has),
    spec!("dict_keys", 1, "json"; builtin_dict_keys),
    spec!("dict_values", 1, "json"; builtin_dict_values), // ── Web builtins ──
    spec!("respond", 1, 2, "web"; builtin_respond),
    spec!("respond_html", 1, "web"; builtin_respond_html),
    spec!("form_data", 1, "web"; builtin_form_data),
    spec!("json_body", 0, "web"; builtin_json_body),
    spec!("query_param", 1, "web"; builtin_query_param),
    spec!("render", 2, 3, "web"; builtin_render),
    spec!("http_get", 1, 4, "web"; builtin_http_get), // url | url,headers | url,headers,timeout | ...,{max_retries:N,base_delay:N}
    spec!("http_post", 2, 6, "web"; builtin_http_post), // up to +retry_config Struct
    spec!("http_post_multipart", 2, 4, "web"; builtin_http_post_multipart),
    spec!("http_download", 2, 3, "web"; builtin_http_download), // Наряд №76: url,dest_path | url,dest_path,headers
    spec!("require", 1, 2, "web"; builtin_require),
    spec!("request_body", 0, "web"; builtin_json_body),
    spec!("web_search", 1, 2, "web"; builtin_web_search), // query | query,num
    spec!("geo_ip", 0, 1, "web"; builtin_geo_ip),         // ip? (omit = caller IP; builtin_geo_ip)
    spec!("weather", 2, "web"; builtin_weather),
    spec!("geo_distance", 2, 5, "web"; builtin_geo_distance),
    spec!("weather_forecast", 1, 3, "web"; builtin_weather_forecast), // city | lat,lon | lat,lon,days
    // ── Crypto builtins ──
    spec!("hash_password", 1, "crypto"; builtin_hash_password),
    spec!("verify_password", 2, "crypto"; builtin_verify_password),
    spec!("encrypt", 2, "crypto"; builtin_encrypt),
    spec!("decrypt", 2, "crypto"; builtin_decrypt),
    spec!("generate_key", 0, "crypto"; builtin_generate_key),
    spec!("base64_encode", 1, "encoding"; builtin_base64_encode),
    spec!("base64_decode", 1, "encoding"; builtin_base64_decode), // ── Auth stubs (interpreter-mode mocks; real auth requires server mode; builtin_base64_decode) ──
    // authenticate: always returns Unit — mock; no user database in interpreter
    spec!("authenticate", 2, "stub"; builtin_authenticate), // session_login: returns empty Session HashMap — mock; no auth backend in interpreter
    spec!("session_login", 2, "stub"; builtin_session_login), // session_logout: no-op — mock; no sessions to invalidate in interpreter
    spec!("session_logout", 1, "stub"; builtin_session_logout),
    spec!("session_clear", 0, "memory"; builtin_session_clear), // ── Bot — Telegram messaging ──
    spec!("send_message", 2, 3, "bot"; builtin_send_message),   // chat_id,text | +reply_markup
    spec!("answer_callback_query", 1, 3, "bot"; builtin_answer_callback_query), // id | id,text | id,text,show_alert
    spec!("edit_message_text", 3, 4, "bot"; builtin_edit_message_text), // chat_id,message_id,text | +reply_markup
    // ── Voice / transcription ──
    spec!("whisper_transcribe", 1, "voice"; builtin_whisper_transcribe),
    spec!("tts_send", 4, 5, "voice"; builtin_tts_send), // text,voice,bot_token,chat_id | +mode
    // ── System builtins ──
    spec!("env", 1, "system"; builtin_env), // ── DB builtins ──
    spec!("query", 1, 2, "db"; builtin_query),
    spec!("db_execute", 1, 2, "db"; builtin_db_execute), // ADR-0068: optional params list
    // ── LLM builtins ──
    #[cfg(feature = "llm")]
    spec!("call_llm", 1, 2, "llm"; builtin_call_llm), // prompt | prompt,input
    #[cfg(feature = "llm")]
    spec!("call_claude", 4, "llm"; builtin_call_claude), // api_key,model,system,user
    #[cfg(feature = "llm")]
    spec!("llm_usage", 0, "llm"; builtin_llm_usage),
    // ── Memory builtins ──
    spec!("kv_set", 2, "memory"; builtin_kv_set),
    spec!("kv_get", 1, "memory"; builtin_kv_get),
    spec!("kv_delete", 1, "memory"; builtin_kv_delete),
    spec!("kv_exists", 1, "memory"; builtin_kv_exists),
    spec!("kv_list", 0, "memory"; builtin_kv_list),
    spec!("mem_set", 2, "memory"; builtin_mem_set),
    spec!("mem_get", 1, "memory"; builtin_mem_get),
    spec!("mem_delete", 1, "memory"; builtin_mem_delete),
    spec!("memorize", 2, 3, "memory"; builtin_kv_set),
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
    spec!("session_set", 2, "memory"; builtin_session_set),
    spec!("session_get", 1, "memory"; builtin_session_get),
    spec!("ref", 1, "memory"; builtin_content_ref),
    spec!("deref", 1, "memory"; builtin_content_deref), // ── Time builtins ──
    spec!("now", 0, "time"; builtin_now),
    spec!("sleep", 1, "time"; builtin_sleep),
    spec!("time", 0, "time"; builtin_now),
    spec!("add_days", 2, "time"; builtin_add_days),
    spec!("add_hours", 2, "time"; builtin_add_hours),
    spec!("date_parts", 1, "time"; builtin_date_parts),
    spec!("format_date", 2, "time"; builtin_format_date),
    spec!("days_between", 2, "time"; builtin_days_between),
    spec!("days_in_month", 2, "time"; builtin_days_in_month),
    spec!("is_leap_year", 1, "time"; builtin_is_leap_year),
    spec!("weekday_name", 1, "time"; builtin_weekday_name), // ── Graph builtins ──
    spec!("graph_query", 1, 3, "graph"; builtin_graph_query), // query | query,limit | query,limit,level
    spec!("graph_path", 2, "graph"; builtin_graph_path),      // from_id,to_id
    spec!("graph_neighbors", 0, "graph"; builtin_graph_neighbors),
    spec!("memory_decay", 0, "memory"; builtin_memory_decay),
    spec!("memory_boost", 0, "memory"; builtin_memory_boost),
    spec!("memory_prune", 0, "memory"; builtin_memory_prune),
    spec!("memory_revise", 0, "memory"; builtin_memory_revise),
    spec!("subgraph_extract", 0, "graph"; builtin_subgraph_extract),
    spec!("subgraph_nodes", 0, "graph"; builtin_subgraph_nodes),
    spec!("subgraph_json", 0, "graph"; builtin_subgraph_json),
    spec!("trace_start", 0, "graph"; builtin_trace_start),
    spec!("trace_end", 0, "graph"; builtin_trace_end),
    spec!("memory_score", 1, "bot" => "ext"; builtin_memory_score),
    spec!("mtree_summarize", 0, "mtree"; builtin_mtree_summarize),
    spec!("mtree_retrieve", 1, 2, "mtree"; builtin_mtree_retrieve), // query | query,limit
    spec!("mtree_store", 2, "mtree"; builtin_mtree_store),
    spec!("mtree_stats", 0, "mtree"; builtin_mtree_stats),
    spec!("mtree_forget", 1, "mtree"; builtin_mtree_forget), // ── Cron builtins ──
    spec!("cron_mark_fired", 1, "cron"; builtin_cron_mark_fired),
    spec!("cron_add", 2, "cron"; builtin_cron_add), // cron_expr, prompt
    spec!("cron_list", 0, "cron"; builtin_cron_list),
    spec!("cron_remove", 1, "cron"; builtin_cron_remove),
    spec!("cron_run", 1, "cron"; builtin_cron_run), // ── Event / query analytics stubs ──
    // event_count/events_since/event_sum: planned event analytics; no handler (use query with SQL instead)
    spec!("event_count", 0, "stub"),
    spec!("events_since", 1, "stub"),
    spec!("event_sum", 2, "stub"), // query_scalar/query_row: planned convenience wrappers; no handler (use query + json_get instead)
    spec!("query_scalar", 0, "stub"),
    spec!("query_row", 0, "stub"), // ── Test builtins ──
    spec!("assert_eq", 2, "test"; builtin_assert_eq),
    spec!("assert_contains", 2, "test"; builtin_assert_contains), // ── Fluid builtins ──
    spec!("confidence", 1, "fluid"; builtin_confidence),          // ── Encoding builtins ──
    spec!("toon_encode", 1, "encoding"; builtin_toon_encode),
    spec!("toon_decode", 1, "encoding"; builtin_toon_decode), // ── Recipe / DAG / Orchestration builtins ──
    spec!("recipe_save", 0, "recipe" => "ext"; builtin_recipe_save),
    spec!("recipe_search", 1, 2, "recipe" => "ext"; builtin_recipe_search), // semantic search via recall_top_k + kv_get
    spec!("recipe_list", 0, "recipe" => "ext"; builtin_recipe_list),
    spec!("dag_phases", 1, "orchestration" => "ext"; builtin_dag_phases),
    spec!("topo_sort", 1, "orchestration" => "ext"; builtin_topo_sort),
    spec!("resolve_skill_index", 1, "stub"), // planned skill resolution; no handler
    spec!("fit_to_budget", 0, "stub"),       // planned budget planner; no handler
    spec!("map", 0, "stub"), // planned list mapper; no handler (use filter+reduce instead)
    // ── OpenPlanter-inspired: fuzzy / safe editing / agent utilities ──
    spec!("fuzzy_find_best", 2, "string"; builtin_fuzzy_find_best),
    spec!("hashline_read", 1, "string"; builtin_hashline_read),
    spec!("hashline_edit", 2, "string"; builtin_hashline_edit),
    spec!("compact_list", 3, "list"; builtin_compact_list),
    spec!("budget_check", 2, "fluid"; builtin_budget_check),
    spec!("replay_snapshot", 1, "system"; builtin_replay_snapshot),
    spec!("policy_check", 1, "system"; builtin_policy_check), // ── obsidian-mind: Vault / semantic search ──
    spec!("semantic_search", 3, "vault" => "ext"; builtin_semantic_search),
    spec!("config_load", 1, "vault" => "ext"; builtin_config_load),
    spec!("vault_validate", 2, "vault" => "ext"; builtin_vault_validate),
    // ── Bot — Telegram ──
    spec!("todo_add", 2, "bot" => "ext"; builtin_todo_add),
    spec!("todo_list", 0, "bot" => "ext"; builtin_todo_list),
    spec!("todo_update", 2, "bot" => "ext"; builtin_todo_update),
    spec!("goal_get", 0, "bot" => "ext"; builtin_goal_get),
    spec!("goal_set", 2, "bot" => "ext"; builtin_goal_set),
    spec!("goals_add", 1, "bot" => "ext"; builtin_goals_add),
    spec!("goals_list", 0, "bot" => "ext"; builtin_goals_list),
    spec!("remind", 3, "bot"; builtin_remind),
    spec!("get_profile", 0, "bot" => "ext"; builtin_get_profile),
    spec!("human_mood", 3, "bot"; builtin_human_mood),
    spec!("ask_approval", 1, "bot" => "ext"; builtin_ask_approval),
    spec!("goal_complete", 0, "bot" => "ext"; builtin_goal_complete),
    spec!("goals_reflect", 0, "bot" => "ext"; builtin_goals_reflect),
    spec!("cancel_remind", 1, "bot"; builtin_cancel_remind),
    spec!("check_reminders", 0, "bot"; builtin_check_reminders),
    spec!("list_reminders", 0, "bot"; builtin_list_reminders),
    spec!("remind_recurring", 2, "bot"; builtin_remind_recurring),
    spec!("human_create", 2, "bot"; builtin_human_create),
    spec!("human_delete", 1, "bot" => "ext"; builtin_human_delete),
    spec!("human_forget", 2, "bot"; builtin_human_forget),
    spec!("human_personas", 0, "bot" => "ext"; builtin_human_personas),
    spec!("human_recall", 3, "bot"; builtin_human_recall),
    spec!("human_remember", 4, "bot"; builtin_human_remember),
    spec!("human_respond", 2, "bot" => "ext"; builtin_human_respond),
    spec!("compress_html", 1, "bot" => "ext"; builtin_compress_html),
    spec!("estimate_tokens", 1, "bot" => "ext"; builtin_estimate_tokens),
    spec!("extract_entities", 1, "bot" => "ext"; builtin_extract_entities),
    spec!("extract_param", 2, "bot" => "ext"; builtin_extract_param), // text,index
    spec!("learn_preference", 2, "bot" => "ext"; builtin_learn_preference),
    spec!("read_file_tokens", 1, "bot" => "ext"; builtin_read_file_tokens),
    // ── sqz-inspired: string/list utilities ──
    spec!("squeeze", 2, "string"; builtin_squeeze),
    spec!("to_int", 1, "string"; builtin_to_int), // parse string/float to integer
    // ── PDF processing (Наряд №48) ──
    spec!("pdf_classify", 1, "pdf"; builtin_pdf_classify),
    spec!("pdf_to_markdown", 1, "pdf"; builtin_pdf_to_markdown),
    spec!("pdf_extract_regions", 2, "pdf"; builtin_pdf_extract_regions),
    spec!("pdf_ocr", 1, "pdf"; builtin_pdf_ocr), // ── PDF creation & manipulation (Наряд MLG-1; builtin_pdf_ocr) ──
    spec!("pdf_create", 0, "pdf"; builtin_pdf_create), // → { id }
    spec!("pdf_add_page", 3, "pdf"; builtin_pdf_add_page), // id, width, height
    spec!("pdf_write_text", 4, 6, "pdf"; builtin_pdf_write_text), // id, x, y, text [,font, size]
    spec!("pdf_draw_line", 5, 6, "pdf"; builtin_pdf_draw_line), // id, x1, y1, x2, y2 [,width]
    spec!("pdf_draw_rect", 5, 7, "pdf"; builtin_pdf_draw_rect), // id, x, y, w, h [,stroke, fill]
    spec!("pdf_save", 2, "pdf"; builtin_pdf_save), // id, path
    spec!("pdf_merge", 2, "pdf"; builtin_pdf_merge), // paths_json, output
    spec!("pdf_split", 3, "pdf"; builtin_pdf_split), // path, ranges_json, output_dir
    spec!("pdf_metadata", 1, "pdf"; builtin_pdf_metadata), // path
    spec!("pdf_set_metadata", 3, "pdf"; builtin_pdf_set_metadata), // path, key, value
    spec!("html_to_pdf", 2, "pdf"; builtin_html_to_pdf), // html, path
    spec!("send_document", 2, 3, "bot" => "ext"; builtin_send_document), // chat_id, file_path [,caption]
    // ── Crypto: SHA-256 / HMAC (Наряд №50 Block 3) ──
    spec!("sha256", 1, "crypto"; builtin_sha256),
    spec!("hmac_sha256", 2, "crypto"; builtin_hmac_sha256),
    spec!("hex_encode", 1, "crypto"; builtin_hex_encode),
    spec!("hex_decode", 1, "crypto"; builtin_hex_decode),
    // Наряд №172: secret() — reads env var as Value::Secret directly
    // (hard-failure if missing, unlike env() which returns empty string).
    spec!("secret", 1, "crypto"; builtin_secret), // ── Regex (Наряд №54; builtin_hex_decode) ──
    spec!("regex_match", 2, "string"; builtin_regex_match),
    spec!("regex_captures", 2, "string"; builtin_regex_captures),
    spec!("regex_replace", 3, "string"; builtin_regex_replace), // ── PDF office automation (Наряд MLG-3; builtin_regex_replace) ──
    spec!("pdf_draw_table", 5, 6, "pdf"; builtin_pdf_draw_table), // id, x, y, col_widths_json, rows_json [,style_json]
    spec!("pdf_add_image", 4, 6, "pdf"; builtin_pdf_add_image), // id, x, y, image_path [,width, height]
    spec!("pdf_set_page_header", 2, 4, "pdf"; builtin_pdf_set_page_header), // id, text [,font, size]
    spec!("pdf_set_page_footer", 2, 4, "pdf"; builtin_pdf_set_page_footer), // id, text [,font, size]
    spec!("pdf_page_numbers", 1, 4, "pdf"; builtin_pdf_page_numbers),       // id [,format, x, y]
    spec!("pdf_watermark", 2, 5, "pdf"; builtin_pdf_watermark), // id, text [,font, size, opacity]
    spec!("pdf_fill_form", 3, "pdf"; builtin_pdf_fill_form),    // path, fields_json, output_path
    spec!("pdf_rotate_page", 4, "pdf"; builtin_pdf_rotate_page), // path, page_number, degrees, output_path
    spec!("pdf_delete_pages", 3, "pdf"; builtin_pdf_delete_pages), // path, pages_json, output_path
    spec!("pdf_extract_images", 1, 2, "pdf"; builtin_pdf_extract_images), // path [,output_dir]
    // ── Email: SMTP + IMAP (Наряд MLG-4) ──
    spec!("smtp_send", 3, 6, "email"; builtin_smtp_send), // to, subject, body [,attachments_json, from, reply_to]
    spec!("smtp_send_html", 3, 4, "email"; builtin_smtp_send_html), // to, subject, html [,attachments_json]
    spec!("imap_list", 2, 3, "email"; builtin_imap_list),           // folder, limit [,since_date]
    spec!("imap_read", 1, "email"; builtin_imap_read),              // uid
    spec!("imap_search", 2, "email"; builtin_imap_search),          // query, folder
    spec!("imap_mark_read", 1, "email"; builtin_imap_mark_read),    // uid
    spec!("imap_move", 2, "email"; builtin_imap_move),              // uid, dest_folder
    // ── Наряд MLG-5: Calendar (CalDAV + iCal) ──
    spec!("cal_connect", 3, "calendar"; builtin_cal_connect), // url, user, pass
    spec!("cal_list", 1, "calendar"; builtin_cal_list),       // session_id
    spec!("cal_events", 3, "calendar"; builtin_cal_events),   // calendar_id, start, end
    spec!("cal_read", 1, "calendar"; builtin_cal_read),       // event_uid
    spec!("cal_create", 4, 7, "calendar"; builtin_cal_create), // cal_id, summary, start, end [,desc, location, attendees_json]
    spec!("cal_update", 2, "calendar"; builtin_cal_update),    // event_uid, fields_json
    spec!("cal_delete", 1, "calendar"; builtin_cal_delete),    // event_uid
    spec!("cal_freebusy", 3, "calendar"; builtin_cal_freebusy), // calendar_id, start, end
    spec!("ical_parse", 1, "calendar"; builtin_ical_parse),    // text
    spec!("ical_generate", 1, "calendar"; builtin_ical_generate), // event_json
    // ── Наряд MLG-6: Contacts (CardDAV + vCard) ──
    spec!("card_connect", 3, "contacts"; builtin_card_connect), // url, user, pass
    spec!("card_list", 1, "contacts"; builtin_card_list),       // session_id
    spec!("card_contacts", 2, "contacts"; builtin_card_contacts), // addressbook_id, query
    spec!("card_read", 1, "contacts"; builtin_card_read),       // contact_uid
    spec!("card_create", 3, 7, "contacts"; builtin_card_create), // addressbook_id, fn, email [,tel, org, title, note]
    spec!("card_update", 2, "contacts"; builtin_card_update),    // contact_uid, fields_json
    spec!("card_delete", 1, "contacts"; builtin_card_delete),    // contact_uid
    spec!("card_search", 2, "contacts"; builtin_card_search),    // session_id, query
    spec!("vcard_parse", 1, "contacts"; builtin_vcard_parse),    // text
    spec!("vcard_generate", 1, "contacts"; builtin_vcard_generate), // contact_json
    // ── Наряд №74: Native SVG Graphics & Diagrams (ADR-0102) ──
    // Level 1: SVG primitives — return XML fragments
    #[cfg(feature = "svg")]
    spec!("svg_rect", 5, 6, "svg"; builtin_svg_rect), // x, y, w, h, fill [, stroke]
    #[cfg(feature = "svg")]
    spec!("svg_circle", 4, "svg"; builtin_svg_circle), // cx, cy, r, fill
    #[cfg(feature = "svg")]
    spec!("svg_line", 5, 6, "svg"; builtin_svg_line), // x1, y1, x2, y2, stroke [, width]
    #[cfg(feature = "svg")]
    spec!("svg_text", 5, 6, "svg"; builtin_svg_text), // x, y, content, font_size, fill [, anchor]
    #[cfg(feature = "svg")]
    spec!("svg_path", 2, 3, "svg"; builtin_svg_path), // d, fill [, stroke]
    #[cfg(feature = "svg")]
    spec!("svg_group", 1, 2, "svg"; builtin_svg_group), // children [, transform]
    #[cfg(feature = "svg")]
    spec!("svg_canvas", 4, "svg"; builtin_svg_canvas), // width, height, viewbox, children
    // Level 2: design tokens
    #[cfg(feature = "svg")]
    spec!("diagram_style", 1, "tokens"; builtin_diagram_style), // {paper, ink, accent, muted, rule}
    // Level 2.5: wow-effects
    #[cfg(feature = "svg")]
    spec!("svg_sketchy_filter", 1, 5, "svg"; builtin_svg_sketchy_filter), // id [, base_freq, octaves, scale, seed]
    #[cfg(feature = "svg")]
    spec!("svg_icon", 5, "svg"; builtin_svg_icon), // name, x, y, size, color
    #[cfg(feature = "svg")]
    spec!("svg_callout", 5, 6, "svg"; builtin_svg_callout), // text, from_x, from_y, to_x, to_y [, intent]
    // Level 3: high-level chart types
    #[cfg(feature = "chart")]
    spec!("chart_bar", 2, "chart"; builtin_chart_bar), // data, style
    #[cfg(feature = "chart")]
    spec!("chart_donut", 2, "chart"; builtin_chart_donut), // data, style — Наряд №77 Block 2
    #[cfg(feature = "chart")]
    spec!("chart_line", 2, "chart"; builtin_chart_line), // data, style — Наряд №78 Block 1
    #[cfg(feature = "chart")]
    spec!("chart_scatter", 2, "chart"; builtin_chart_scatter), // data, style — Наряд №78 Block 2
    #[cfg(feature = "chart")]
    spec!("chart_area", 2, "chart"; builtin_chart_area), // data, style — Наряд №78 Block 3
    #[cfg(feature = "chart")]
    spec!("chart_radar", 2, "chart"; builtin_chart_radar), // data, style — Наряд №79 Block 1
    #[cfg(feature = "chart")]
    spec!("chart_heatmap", 2, "chart"; builtin_chart_heatmap), // data, style — Наряд №79 Block 2
    #[cfg(feature = "chart")]
    spec!("chart_boxplot", 2, "chart"; builtin_chart_boxplot), // data, style — Наряд №79 Block 3
    // Level 2.6: derived palette (Наряд №77 Block 1)
    #[cfg(feature = "svg")]
    spec!("color_palette", 2, "svg"; builtin_color_palette), // intent, mode → DiagramStyle
    // Level 2.6/2.7: procedural backgrounds + canvas presets (Наряд №80)
    #[cfg(feature = "svg")]
    spec!("svg_generate", 4, "svg"; builtin_svg_generate), // kind, intent, w, h → SVG fragment
    #[cfg(feature = "svg")]
    spec!("svg_canvas_preset", 3, "svg"; builtin_svg_canvas_preset), // preset_name, viewbox, children
    // Level 3.1: diagrams (Наряд №81) — hierarchies & flows
    #[cfg(feature = "diagram")]
    spec!("diagram_tree", 2, "diagram"; builtin_diagram_tree), // data, style — recursive tree
    #[cfg(feature = "diagram")]
    spec!("diagram_org_chart", 2, "diagram"; builtin_diagram_org_chart), // data, style — tree with title field
    #[cfg(feature = "diagram")]
    spec!("diagram_flowchart", 2, "diagram"; builtin_diagram_flowchart), // data, style — layered DAG
    #[cfg(feature = "diagram")]
    spec!("diagram_layers", 2, "diagram"; builtin_diagram_layers), // data, style — horizontal stripes
    // Level 3.2: diagrams (Наряд №82) — temporal & process
    #[cfg(feature = "diagram")]
    spec!("diagram_sequence", 2, "diagram"; builtin_diagram_sequence), // data, style — UML sequence (lifelines + messages; builtin_diagram_sequence)
    #[cfg(feature = "diagram")]
    spec!("diagram_timeline", 2, "diagram"; builtin_diagram_timeline), // data, style — horizontal axis with event dots
    #[cfg(feature = "diagram")]
    spec!("diagram_gantt", 2, "diagram"; builtin_diagram_gantt), // data, style — horizontal bars per task
    #[cfg(feature = "diagram")]
    spec!("diagram_process", 2, "diagram"; builtin_diagram_process), // data, style — linear numbered step chain
    #[cfg(feature = "diagram")]
    spec!("diagram_loop", 2, "diagram"; builtin_diagram_loop), // data, style — closed-loop circular steps
    // Level 3.3: diagrams (Наряд №83) — sets & comparisons
    #[cfg(feature = "diagram")]
    spec!("diagram_venn", 2, "diagram"; builtin_diagram_venn), // data, style — 2 or 3 overlapping circles
    #[cfg(feature = "diagram")]
    spec!("diagram_quadrant", 2, "diagram"; builtin_diagram_quadrant), // data, style — 2x2 strategic quadrant
    #[cfg(feature = "diagram")]
    spec!("diagram_pyramid", 2, "diagram"; builtin_diagram_pyramid), // data, style — stacked trapezoids (top=apex; builtin_diagram_pyramid)
    #[cfg(feature = "diagram")]
    spec!("diagram_nested", 2, "diagram"; builtin_diagram_nested), // data, style — concentric circles
    #[cfg(feature = "diagram")]
    spec!("diagram_medallion", 2, "diagram"; builtin_diagram_medallion), // data, style — row of round badges w/ icons
    // Level 3.4: diagrams (Наряд №84) — data & state
    //   diagram_er         — Struct{entities: [{name, fields: [String]}], relations: [{from,to,label?}]}
    //                       simple grid layout (no graph analysis), entities ≤ 12, fields ≤ 8.
    //   diagram_state      — Struct{states: [String], transitions: [{from,to,label?}], initial?}
    //                       BFS layout tolerating cycles + self-loops (state machines are cyclic).
    //   diagram_swimlane   — Struct{lanes: [String], steps: [{lane,label,order}]}
    //                       vertical stack of lanes, steps positioned by Float `order` (not list idx).
    //   diagram_data_flow  — Struct{nodes:[{id,label}], edges:[{from,to,label?}]}
    //                       same shape as flowchart, but cycles VALID (uses bfs_layers_with_cycles).
    //   diagram_high_level — same shape, NO cycles (topological), larger bolder blocks.
    //   diagram_architecture — same shape + optional `icon` per node (reuses svg_icon's 10 names).
    #[cfg(feature = "diagram")]
    spec!("diagram_er", 2, "diagram"; builtin_diagram_er), // data, style — entity boxes on a grid w/ relations
    #[cfg(feature = "diagram")]
    spec!("diagram_state", 2, "diagram"; builtin_diagram_state), // data, style — state machine (cycles OK; builtin_diagram_state)
    #[cfg(feature = "diagram")]
    spec!("diagram_swimlane", 2, "diagram"; builtin_diagram_swimlane), // data, style — lanes × steps positioned by order
    #[cfg(feature = "diagram")]
    spec!("diagram_data_flow", 2, "diagram"; builtin_diagram_data_flow), // data, style — graph w/ cycles OK
    #[cfg(feature = "diagram")]
    spec!("diagram_high_level", 2, "diagram"; builtin_diagram_high_level), // data, style — large bolder blocks, no cycles
    #[cfg(feature = "diagram")]
    spec!("diagram_architecture", 2, "diagram"; builtin_diagram_architecture), // data, style — high_level + svg_icon per node
    // ── Наряд №86: Mini template engine ──
    //   template_render(template, data) -> Html
    //   Parses Mustache/Handlebars-like subset: {{ var }} (auto-escaped),
    //   {{{ var }}} (raw), {{#if cond}}...{{else}}...{{/if}},
    //   {{#each items}}...{{/each}}. Returns opaque Value::Html.
    //   INTENTIONALLY NOT in SVG_AUTO_ESCAPE_BUILTINS — the template is
    //   trusted code (written by the .mlog programmer, not user input);
    //   data substitution is escaped at runtime via escape_html_chars.
    #[cfg(feature = "template")]
    spec!("template_render", 2, "template"; builtin_template_render), // ── Наряд №88: HTML rendering via headless browser ──
    //   html_render(html, width, height) -> String (path to PNG)
    //   Renders self-contained HTML to a PNG screenshot using
    //   Chromium/Chrome (configured via METALOGOS_BROWSER_BIN env var).
    //   NO shell interpretation — uses exec_restricted internally.
    //   Network isolation: caller's responsibility (self-contained HTML
    //   with data: URIs; external resources NOT blocked at OS level).
    spec!("html_render", 3, "web"; builtin_html_render), // ── Наряд №89: Infographic quality assurance ──
    //   infographic_qa(svg_string) -> Struct { passed, warnings, checks_run }
    //   Three mechanical checks: contrast (WCAG), saturation discipline,
    //   element density. Advisory — passed:false means "review", not "broken".
    #[cfg(feature = "diagram")]
    spec!("infographic_qa", 1, "diagram"; builtin_infographic_qa),
    // ── Наряд №179b: Reflex training/prediction builtins ──
    // Stub handlers — the real dispatch lives in interpreter::execution::invoke()
    // because reflex_train/reflex_predict need access to ReflexRegistry (which
    // lives on the Interpreter struct). The stubs produce a clean "VM not yet
    // supported" error if the VM backend somehow reaches them directly.
    // When VM gains Reflex support (future naryad), the same dispatch logic
    // in src/builtins/reflex.rs will be reused — see reflex_train_dispatch /
    // reflex_predict_dispatch.
    spec!("reflex_train", 5, "reflex"; builtin_reflex_train_stub),
    spec!("reflex_predict", 2, "reflex"; builtin_reflex_predict_stub),
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
