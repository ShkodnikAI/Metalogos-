use metalogos::builtins::check_builtin_arity;

/// Exhaustive arity check: every non-variadic builtin (arity > 0) is tested
/// at its minimum, maximum, and one below/above. Variadic builtins (arity=0)
/// are verified to accept any count.
#[test]
fn registry_arity_exhaustive() {
    // Format: (name, min_arity, max_arity)
    // max_arity = min_arity means exact match
    let cases: &[(&str, usize, usize)] = &[
        // ── String builtins ──
        ("upper", 1, 1),
        ("lower", 1, 1),
        ("len", 1, 1),
        ("str", 1, 1),
        ("contains", 2, 2),
        ("index_of", 2, 2),
        ("substring", 3, 3),
        ("char_at", 2, 2),
        ("starts_with", 2, 2),
        ("ends_with", 2, 2),
        ("trim", 1, 1),
        ("replace", 3, 3),
        ("split", 2, 2),
        ("join", 2, 2),
        ("length", 1, 1),
        ("reverse", 1, 1),
        ("escape_html", 1, 1),
        ("escape_json", 1, 1),
        ("escape_js", 1, 1),
        ("fuzzy_match", 2, 2),
        ("strip", 2, 2),
        ("chomp", 1, 1),
        ("repeat", 2, 2),
        ("pad_left", 3, 3),
        ("pad_right", 3, 3),
        ("lines", 1, 1),
        ("words", 1, 1),
        ("token_count", 1, 1),
        ("type_of", 1, 1),
        // format is variadic (arity=0), tested below
        // ── Stdlib backing ──
        ("__trim", 1, 1),
        ("__replace", 3, 3),
        ("__split", 2, 2),
        ("__join", 2, 2),
        ("__abs", 1, 1),
        ("__min", 2, 2),
        ("__max", 2, 2),
        ("__clamp", 3, 3),
        ("__round", 1, 1),
        ("__first", 1, 1),
        ("__last", 1, 1),
        ("__push", 2, 2),
        ("__list_len", 1, 1),
        // ── Stub entries for VM coverage ──
        ("abs", 1, 1),
        ("min", 2, 2),
        ("max", 2, 2),
        ("clamp", 3, 3),
        ("round", 1, 1),
        ("if_eq", 3, 3),
        ("is_string_token", 1, 1),
        // ── Convert builtins ──
        ("float", 1, 1),
        ("to_string", 1, 1),
        ("to_float", 1, 1),
        // ── IO builtins ──
        ("print", 1, 1),
        ("read_file", 1, 1),
        ("write_file", 2, 2),
        ("append_file", 2, 2),
        ("delete_file", 1, 1),
        ("file_exists", 1, 1),
        ("list_dir", 1, 1),
        ("exec", 1, 1),
        ("git_push", 1, 1),
        // ── List builtins ──
        ("get", 2, 2),
        ("push", 2, 2),
        ("slice", 3, 3),
        ("zip", 2, 2),
        ("sort_by", 2, 3),
        ("filter", 3, 3),
        ("reduce", 3, 3),
        ("dedup", 1, 1),
        ("condense", 1, 1),
        ("first", 1, 1),
        ("last", 1, 1),
        ("matches_any", 2, 2),
        // ── JSON builtins ──
        ("parse_json", 1, 2),
        ("json_encode", 1, 1),
        ("json_get", 2, 3),
        ("has_field", 2, 2),
        ("dict_get", 3, 3),
        ("dict_set", 3, 3),
        ("dict_has", 2, 2),
        ("dict_keys", 1, 1),
        ("dict_values", 1, 1),
        // ── Web builtins ──
        ("respond", 1, 2),
        ("respond_html", 1, 1),
        ("form_data", 1, 1),
        ("query_param", 1, 1),
        ("render", 2, 3),
        ("http_get", 1, 3),
        ("http_post", 2, 5),
        ("http_post_multipart", 2, 4),
        ("require", 1, 2),
        ("web_search", 1, 2),
        ("geo_ip", 0, 1),
        ("weather", 2, 2),
        ("geo_distance", 2, 5),
        ("weather_forecast", 1, 3),
        // ── Crypto builtins ──
        ("hash_password", 1, 1),
        ("verify_password", 2, 2),
        ("encrypt", 2, 2),
        ("decrypt", 2, 2),
        ("base64_encode", 1, 1),
        ("base64_decode", 1, 1),
        ("sha256", 1, 1),
        ("hmac_sha256", 2, 2),
        ("hex_encode", 1, 1),
        ("hex_decode", 1, 1),
        // ── Auth stubs ──
        ("authenticate", 2, 2),
        ("session_login", 2, 2),
        ("session_logout", 1, 1),
        // ── Bot stubs ──
        ("send_message", 2, 3),
        ("answer_callback_query", 1, 3),
        ("edit_message_text", 3, 4),
        ("whisper_transcribe", 1, 1),
        ("tts_send", 4, 5),
        // ── System builtins ──
        ("env", 1, 1),
        // ── DB builtins ──
        ("query", 1, 2),
        ("db_execute", 1, 1),
        // ── LLM builtins ──
        ("call_llm", 1, 2),
        ("call_claude", 4, 4),
        // ── Memory builtins ──
        ("kv_set", 2, 2),
        ("kv_get", 1, 1),
        ("kv_delete", 1, 1),
        ("kv_exists", 1, 1),
        ("mem_set", 2, 2),
        ("mem_get", 1, 1),
        ("mem_delete", 1, 1),
        ("memorize", 2, 3),
        ("find", 4, 4),
        ("inspect", 1, 1),
        ("conv_start", 1, 1),
        ("conv_add", 3, 3),
        ("conv_history", 1, 1),
        ("conv_context", 1, 1),
        ("conv_end", 1, 1),
        ("session_set", 2, 2),
        ("session_get", 1, 1),
        ("ref", 1, 1),
        ("deref", 1, 1),
        // ── Time builtins ──
        ("add_days", 2, 2),
        ("add_hours", 2, 2),
        ("date_parts", 1, 1),
        ("format_date", 2, 2),
        ("days_between", 2, 2),
        ("days_in_month", 2, 2),
        ("is_leap_year", 1, 1),
        ("weekday_name", 1, 1),
        // ── Graph builtins ──
        ("graph_query", 1, 3),
        ("graph_path", 2, 2),
        ("trace_start", 0, 0), // variadic
        ("trace_end", 0, 0),   // variadic
        ("memory_score", 1, 1),
        ("mtree_retrieve", 1, 2),
        ("mtree_store", 2, 2),
        ("mtree_forget", 1, 1),
        // ── Cron builtins ──
        ("cron_mark_fired", 1, 1),
        // ── Event stubs ──
        ("events_since", 1, 1),
        ("event_sum", 2, 2),
        // ── Test builtins ──
        ("assert_eq", 2, 2),
        ("assert_contains", 2, 2),
        // ── Fluid builtins ──
        ("confidence", 1, 1),
        // ── Encoding builtins ──
        ("toon_encode", 1, 1),
        ("toon_decode", 1, 1),
        // ── Recipe / DAG / Orchestration builtins ──
        ("dag_phases", 1, 1),
        ("topo_sort", 1, 1),
        ("resolve_skill_index", 1, 1),
        // ── OpenPlanter-inspired ──
        ("fuzzy_find_best", 2, 2),
        ("hashline_read", 1, 1),
        ("hashline_edit", 2, 2),
        ("compact_list", 3, 3),
        ("budget_check", 2, 2),
        ("replay_snapshot", 1, 1),
        ("policy_check", 1, 1),
        // ── obsidian-mind ──
        ("semantic_search", 3, 3),
        ("config_load", 1, 1),
        ("vault_validate", 2, 2),
        // ── Bot — Telegram ──
        ("todo_add", 2, 2),
        ("todo_update", 2, 2),
        ("goal_set", 2, 2),
        ("goals_add", 1, 1),
        ("remind", 3, 3),
        ("human_mood", 3, 3),
        ("ask_approval", 1, 1),
        ("cancel_remind", 1, 1),
        ("human_create", 2, 2),
        ("human_delete", 1, 1),
        ("human_forget", 2, 2),
        ("human_recall", 3, 3),
        ("human_remember", 4, 4),
        ("human_respond", 2, 2),
        ("compress_html", 1, 1),
        ("estimate_tokens", 1, 1),
        ("extract_entities", 1, 1),
        ("extract_param", 2, 2),
        ("learn_preference", 2, 2),
        ("read_file_tokens", 1, 1),
        // ── sqz-inspired ──
        ("squeeze", 2, 2),
        // ── PDF processing ──
        ("pdf_classify", 1, 1),
        ("pdf_to_markdown", 1, 1),
        ("pdf_extract_regions", 2, 2),
        ("pdf_ocr", 1, 1),
    ];

    for &(name, min_a, max_a) in cases {
        if min_a == 0 {
            // Special case: geo_ip (0..1) — test 0 and 1
            assert!(
                check_builtin_arity(name, 0).is_ok(),
                "{}(0) should be ok (min=0)", name
            );
            assert!(
                check_builtin_arity(name, max_a).is_ok(),
                "{}({}) should be ok (max={})", name, max_a, max_a
            );
            assert!(
                check_builtin_arity(name, max_a + 1).is_err(),
                "{}({}) should be err (max={})", name, max_a + 1, max_a
            );
        } else {
            // Test at minimum arity
            assert!(
                check_builtin_arity(name, min_a).is_ok(),
                "{}({}) should be ok (min)", name, min_a
            );
            // Test at maximum arity
            assert!(
                check_builtin_arity(name, max_a).is_ok(),
                "{}({}) should be ok (max)", name, max_a
            );
            // Test below minimum
            assert!(
                check_builtin_arity(name, min_a - 1).is_err(),
                "{}({}) should be err (below min={})", name, min_a - 1, min_a
            );
            // Test above maximum
            assert!(
                check_builtin_arity(name, max_a + 1).is_err(),
                "{}({}) should be err (above max={})", name, max_a + 1, max_a
            );
            // Test between min and max (if range)
            if max_a > min_a {
                let mid = (min_a + max_a) / 2;
                assert!(
                    check_builtin_arity(name, mid).is_ok(),
                    "{}({}) should be ok (mid of {}..{})", name, mid, min_a, max_a
                );
            }
        }
    }

    // Variadic builtins (arity=0, max_arity=None): accept any count
    let variadic: &[&str] = &[
        "format",
        "newline", "stdin", "split_tokens", "db_insert",
        "json_body", "request_body",
        "generate_key", "session_clear",
        "llm_usage",
        "kv_list", "recall", "forget",
        "now", "time",
        "graph_neighbors", "memory_decay", "memory_boost",
        "memory_prune", "memory_revise",
        "subgraph_extract", "subgraph_nodes", "subgraph_json",
        "memory_score",
        "mtree_summarize", "mtree_stats",
        "event_count", "query_scalar", "query_row",
        "recipe_save", "recipe_search", "recipe_list",
        "fit_to_budget", "map",
        "todo_list", "goal_get", "goals_list",
        "goal_complete", "goals_reflect",
        "check_reminders", "list_reminders", "remind_recurring",
        "human_personas", "get_profile",
        "make_list",
    ];
    for &name in variadic {
        assert!(
            check_builtin_arity(name, 0).is_ok(),
            "{}(0) should be ok (variadic)", name
        );
        assert!(
            check_builtin_arity(name, 5).is_ok(),
            "{}(5) should be ok (variadic)", name
        );
    }
}
