use super::reflex::{
    builtin_reflex_bpe_decode, builtin_reflex_bpe_encode, builtin_reflex_bpe_load,
    builtin_reflex_bpe_save, builtin_reflex_bpe_train, builtin_reflex_detokenize,
    builtin_reflex_generate_stub, builtin_reflex_list_stub, builtin_reflex_load_stub,
    builtin_reflex_metrics_stub, builtin_reflex_predict_stub, builtin_reflex_save_stub,
    builtin_reflex_tokenize, builtin_reflex_train_stub,
};
// Наряд №210: Vision pillar stub handlers (ADR-0124).
use super::vision::{
    builtin_vision_edit_stub, builtin_vision_export_raw_stub, builtin_vision_export_stub,
    builtin_vision_fetch_weights, builtin_vision_generate_stub, builtin_vision_list_stub,
    builtin_vision_load_stub, builtin_vision_lora_generate_stub, builtin_vision_lora_load_stub,
    builtin_vision_save_stub,
};
// Наряд №307 (ADR-0147-0150): Video pillar builtins. Наряд №309 (ADR-0151):
// real pipeline implementations — I2V anchors, interp, extend, mux, export.
#[cfg(feature = "video")]
use crate::video::{
    builtin_av_mux, builtin_frame_interp, builtin_video_export, builtin_video_extend,
    builtin_video_fetch_weights_stub, builtin_video_render, builtin_video_understand,
};

// Наряд №275 (ADR-0137): LLM streaming builtins — llm_stream_open/next/close.
use super::llm_stream::{
    builtin_llm_stream_close, builtin_llm_stream_next, builtin_llm_stream_open,
};
// №757: the mlog-visible truncation probe (llm_last_finish_reason).
#[cfg(feature = "llm")]
use super::llm::builtin_llm_last_finish_reason;
// Наряд №331 (ADR-0162): unified media layer — last-resort stubs for the
// state-carrying media builtins (pub(crate); real paths are interception).
use super::media::{
    builtin_media_bind_origin_stub, builtin_media_meta_stub, builtin_media_release_stub,
    builtin_media_retain_stub, builtin_media_save_stub, builtin_media_source_capture_stub,
    builtin_media_store_audio_stub, builtin_media_store_image_stub,
    builtin_media_store_video_frame_stub, builtin_media_store_video_segment_stub,
};
use super::media::{builtin_media_manifest_read, builtin_media_manifest_stub};
// Наряд №333 (ADR-0163): backend registry listing (stateless).
use super::backends::{builtin_backend_list, builtin_backend_select};
use super::forecast::{
    builtin_forecast_next, builtin_forecast_points, builtin_forecast_state, builtin_series_make,
    builtin_series_pull,
};
// Наряд №591 (Волна 30): the spectral contour (Lomb–Scargle).
use super::spectral::{builtin_lomb_scargle, builtin_spectral_peak};
// Наряд №595 (Волна 30): the UTC calendar arithmetic.
use super::calendar_utc::{
    builtin_date_diff_days, builtin_date_format_iso, builtin_date_parse_iso, builtin_now_unix,
};
// Наряд №334: real STT/omni/vision-understanding backends — the mock-first
// call surface over the №333 registry (SHA-pin path, ADR-0163 §2.1).
use crate::vision::ocr::builtin_ocr_extract;
use crate::vision::understand::builtin_vision_understand;
use crate::voice::backend::{builtin_omni_ask, builtin_stt_transcribe};
// Наряд №335 (spec §7.2 v2): consent surface — grant/revoke/quarantine/ledger.
use super::consent::{
    builtin_consent_grant, builtin_consent_ledger_export, builtin_consent_revoke,
    builtin_quarantine_write,
};
// Naryad #390 (ADR-0155): Grant algebra builtins — issue/subgrant/revoke/use
// + the granted destructive-SQL action surface.
use super::grants::{
    builtin_db_execute_with_grant, builtin_grant_issue, builtin_grant_revoke,
    builtin_grant_subgrant, builtin_grant_use,
};
// Naryad #393 (ADR-0167): Action Ledger v1 surface — count/head/export/
// export_intoto/rotate/snapshot.
use super::ledger::{
    builtin_ledger_count, builtin_ledger_export, builtin_ledger_export_intoto, builtin_ledger_head,
    builtin_ledger_rotate, builtin_ledger_snapshot, builtin_ledger_verify,
};
// Naryad #387 (ADR-0149 D1/D6): the likeness ritual — challenge/verify
// over the opaque LikenessToken.
use super::likeness::{builtin_likeness_challenge, builtin_likeness_verify};
// Наряд №302 (ADR-0143-0146): Voice pillar skeleton builtins — stubs.
use super::*;
#[cfg(feature = "voice")]
use crate::voice::{
    builtin_audio_export_stub, builtin_tts_speak_stub, builtin_voice_design_stub,
    builtin_voice_enroll_stub, builtin_voice_load_stub, builtin_voice_save_stub,
};
// №526 (issue #835): the erasure path is NOT feature-gated — the GDPR
// Art. 17 right cannot depend on a build flag; VOICE_REGISTRY itself was
// never feature-gated.
use crate::voice::{builtin_voice_delete, builtin_voice_list};

/// Master registry of ALL builtin functions.
/// Order determines bytecode indices — DO NOT reorder existing entries.
/// To add a new builtin: append a `spec!` row here,
/// add the handler in Builtins::new(), and you're done.
pub const BUILTIN_REGISTRY: &[BuiltinSpec] = &[
    // ── String builtins ──
    spec!("upper", 1, "string"; builtin_upper, "String"),
    spec!("lower", 1, "string"; builtin_lower, "String"),
    spec!("len", 1, "string"; builtin_len, "Float"),
    spec!("str", 1, "string"; builtin_str, "String"),
    spec!("contains", 2, "string"; builtin_contains, "Bool"),
    spec!("index_of", 2, "string"; builtin_index_of, "Float"),
    spec!("substring", 3, "string"; builtin_substring, "String"),
    spec!("char_at", 2, "string"; builtin_char_at, "String"),
    spec!("starts_with", 2, "string"; builtin_starts_with, "Bool"),
    spec!("ends_with", 2, "string"; builtin_ends_with, "Bool"),
    spec!("trim", 1, "string"; builtin_trim, "String"),
    spec!("replace", 3, "string"; builtin_replace, "String"),
    spec!("split", 2, "string"; builtin_split, "List<String>"), // №631: the String parts (string.rs: Value::String elements)
    spec!("join", 2, "string"; builtin_join, "String"),
    spec!("length", 1, "string"; builtin_length, "Float"),
    spec!("reverse", 1, "string"; builtin_reverse), // №536: Unknown honest — polymorphic (String → String, List → List); the flat vocabulary has no union
    spec!("escape_html", 1, "string"; builtin_escape_html, "String"),
    spec!("escape_json", 1, "string"; builtin_escape_json, "String"),
    // Наряд №274 (ADR-0136): redact(text, mode) — PII/секреты как
    // taint-санитайзер («mask before sink»). Единственный легальный путь
    // снять Secret-taint; семантика снятия — в src/audit.rs + ADR-0136.
    spec!("redact", 2, "string"; builtin_redact, "String"),
    spec!("escape_js", 1, "string"; builtin_escape_js, "String"),
    spec!("fuzzy_match", 2, "string"; builtin_fuzzy_match, "Float"),
    spec!("strip", 2, "string"; builtin_strip, "String"),
    spec!("chomp", 1, "string"; builtin_chomp, "String"),
    spec!("repeat", 2, "string"; builtin_repeat, "String"),
    spec!("pad_left", 3, "string"; builtin_pad_left, "String"),
    spec!("pad_right", 3, "string"; builtin_pad_right, "String"),
    spec!("lines", 1, "string"; builtin_lines, "List<String>"), // №631: the String lines (string.rs: Value::String elements)
    spec!("words", 1, "string"; builtin_words, "List<String>"), // №631: the String words (string.rs: Value::String elements)
    spec!("token_count", 1, "string"; builtin_token_count, "Float"),
    spec!("type_of", 1, "string"; builtin_type_of, "String"),
    spec!("format", 0, "string"; builtin_format, "String"), // variadic: 1 template + N fill args
    // НАРЯД №117: missing string utilities
    spec!("trim_start", 1, "string"; builtin_trim_start, "String"),
    spec!("trim_end", 1, "string"; builtin_trim_end, "String"),
    spec!("truncate", 2, "string"; builtin_truncate, "String"),
    spec!("slugify", 1, "string"; builtin_slugify, "String"),
    spec!("word_wrap", 2, "string"; builtin_word_wrap, "String"),
    spec!("capitalize", 1, "string"; builtin_capitalize, "String"),
    spec!("title_case", 1, "string"; builtin_title_case, "String"),
    // ── Stdlib backing (double-underscore prefix) ──
    spec!("__trim", 1, "std"; builtin_trim, "String"),
    spec!("__replace", 3, "std"; builtin_replace, "String"),
    spec!("__split", 2, "std"; builtin_split),
    spec!("__join", 2, "std"; builtin_join, "String"),
    spec!("__abs", 1, "std"; builtin_abs, "Float"),
    spec!("__min", 2, "std"; builtin_min, "Float"),
    spec!("__max", 2, "std"; builtin_max, "Float"),
    spec!("__clamp", 3, "std"; builtin_clamp, "Float"),
    spec!("__round", 1, "std"; builtin_round, "Float"),
    spec!("__first", 1, "std"; builtin_first),
    spec!("__last", 1, "std"; builtin_last),
    // ── Math builtins (public aliases for __abs/__min/__max/__clamp/__round) ──
    spec!("abs", 1, "math"; builtin_abs, "Float"),
    spec!("min", 2, "math"; builtin_min, "Float"),
    spec!("max", 2, "math"; builtin_max, "Float"),
    spec!("clamp", 3, "math"; builtin_clamp, "Float"),
    spec!("round", 1, "math"; builtin_round, "Float"),
    // Наряд №177: Math foundation for Reflex (stage 1/6)
    spec!("exp", 1, "math"; builtin_exp, "Float"),
    spec!("ln", 1, "math"; builtin_ln, "Float"),
    spec!("sqrt", 1, "math"; builtin_sqrt, "Float"),
    spec!("pow", 2, "math"; builtin_pow, "Float"),
    spec!("tanh", 1, "math"; builtin_tanh, "Float"),
    spec!("sigmoid", 1, "math"; builtin_sigmoid, "Float"),
    spec!("softmax", 1, "math"; builtin_softmax),
    spec!("random_seed", 1, "math"; builtin_random_seed, "Unit"),
    spec!("random", 0, "math"; builtin_random, "Float"), // ── Phase 4.4 self-hosting — historical placeholders, never implemented ──
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
    spec!("float", 1, "convert"; builtin_float, "Float"),
    spec!("to_string", 1, "convert"; builtin_to_string, "String"),
    spec!("to_float", 1, "convert"; builtin_to_float, "Float"), // ── IO builtins ──
    spec!("print", 1, "io"; builtin_print, "String"), // №613: returns the echoed string (the handler fact), not Unit
    spec!("read_file", 1, "io"; builtin_read_file, "String"),
    spec!("write_file", 2, "io"; builtin_write_file, "String"),
    spec!("append_file", 2, "io"; builtin_append_file, "String"),
    spec!("delete_file", 1, "io"; builtin_delete_file, "String"),
    spec!("file_exists", 1, "io"; builtin_file_exists, "Bool"),
    spec!("list_dir", 1, "io"; builtin_list_dir),
    spec!("exec", 1, "io"; builtin_exec, "String"),
    spec!("exec_argv", 1, 2, "io"; builtin_exec_argv, "String"), // binary required, args list optional
    spec!("git_push", 1, "io"; builtin_git_push, "String"),
    // Наряд №268 (ADR-0132): MCP stdio-клиент — stateless, exec-гейт +
    // METALOGOS_MCP_ALLOWLIST + taint UserInput на выводе mcp_call.
    spec!("mcp_call", 4, "io"; builtin_mcp_call, "String"),
    spec!("mcp_list_tools", 2, "io"; builtin_mcp_list_tools, "List<Tool>"),
    // ── List builtins ──
    spec!("get", 2, "list"; builtin_get), // №537: Unknown honest — returns the ELEMENT; a heterogeneous list has no fixed element type
    spec!("push", 2, "list"; builtin_push, "List"),
    spec!("slice", 3, "list"; builtin_slice, "List"),
    spec!("zip", 2, "list"; builtin_zip, "List"),
    spec!("sort_by", 2, 3, "list"; builtin_sort_by, "List"),
    spec!("filter", 3, "list"; builtin_filter, "List"),
    spec!("reduce", 3, "list"; builtin_reduce, "Float"), // №537: Unknown honest — returns the ACCUMULATOR; its type is the caller's choice
    spec!("dedup", 1, "list"; builtin_dedup, "List"),
    spec!("condense", 1, "list"; builtin_condense, "List"),
    // НАРЯД №118: collection utilities (unique, chunk, sort)
    spec!("unique", 1, "list"; builtin_unique, "List"),
    spec!("chunk", 2, "list"; builtin_chunk, "List"),
    spec!("sort", 1, "list"; builtin_sort, "List"),
    spec!("first", 1, "list"; builtin_first), // №537: Unknown honest — the element + the soft-failure "" path (ADR-0180 posture)
    spec!("last", 1, "list"; builtin_last), // №537: Unknown honest — the element + the soft-failure "" path (ADR-0180 posture)
    spec!("make_list", 0, "list"; builtin_make_list, "List"),
    spec!("matches_any", 2, "list"; builtin_matches_any, "Float"), // 1.0/0.0 — the numeric verdict form // ── JSON builtins ──
    spec!("parse_json", 1, 2, "json"; builtin_parse_json),
    spec!("json_encode", 1, "json"; builtin_json_encode, "String"),
    spec!("json_get", 2, 3, "json"; builtin_json_get),
    spec!("has_field", 2, "json"; builtin_has_field, "Float"),
    spec!("dict_get", 3, "json"; builtin_json_get),
    spec!("dict_set", 3, "json"; builtin_dict_set),
    spec!("dict_has", 2, "json"; builtin_dict_has, "Bool"),
    spec!("dict_keys", 1, "json"; builtin_dict_keys),
    spec!("dict_values", 1, "json"; builtin_dict_values), // ── Web builtins ──
    spec!("respond", 1, 2, "web"; builtin_respond),
    // №523: the spec followed the then-implementation (status, html) — and
    // broke the 1-arg office corpus form (issue #892: 500 on every HTML route).
    // #892 restores the full contract: 1 arg (office form) / 2 args (documented
    // (status, html) OR office (title, body) — the builtin disambiguates).
    spec!("respond_html", 1, 2, "web"; builtin_respond_html), // №565: the 2-arg form is DEPRECATED (the sense-by-content hazard) — migrate to respond_html_status/respond_html_doc; the 1-arg form is the gh#899 SSOT
    spec!("respond_html_status", 2, 2, "web"; builtin_respond_html_status, "Struct"), // №565: the explicit (status, body) form — the HttpResponse struct shape; the HTML egress gates treat it like respond_html
    spec!("respond_html_doc", 2, 2, "web"; builtin_respond_html_doc, "Struct"), // №565: the explicit (title, body) form — the title is never a status
    spec!("form_data", 1, "web"; builtin_form_data, "Struct<FormData>"),
    spec!("json_body", 0, "web"; builtin_json_body, "Struct<JsonBody>"),
    spec!("query_param", 1, "web"; builtin_query_param, "String"), // №565 top-up (the №543 line): both paths verified — the handler stub and the db_ops::query_param dispatch return Value::String
    // №523: the 2..3 range is NOMINAL — render's real contract is dynamic
    // (the template's parameter list is data, №115; the 1-arg form is the
    // №448 taint-lift surface). The builtin validates loudly at runtime;
    // the semantic arity check exempts the name (see check_expr_calls).
    spec!("render", 2, 3, "web"; builtin_render),
    spec!("http_get", 1, 4, "web"; builtin_http_get, "String"), // url | url,headers | url,headers,timeout | ...,{max_retries:N,base_delay:N}
    spec!("http_post", 2, 6, "web"; builtin_http_post, "String"), // up to +retry_config Struct
    spec!("http_post_multipart", 2, 4, "web"; builtin_http_post_multipart, "String"),
    spec!("http_download", 2, 3, "web"; builtin_http_download), // Наряд №76: url,dest_path | url,dest_path,headers
    spec!("require", 1, 2, "web"; builtin_require, "Unit"),
    spec!("request_body", 0, "web"; builtin_json_body),
    spec!("web_search", 1, 2, "web"; builtin_web_search, "String"), // query | query,num
    // №627 (gh#1110): the field-label tables — the FIRST honest package
    // (the handler-read verification table in the naryad thread; the file
    // facts: geo_ip — every field parsed from the external ip-api.com
    // body (the g() closure, http.rs), the ip field additionally the echo
    // of the caller's argument; weather — the ten wire numbers from the
    // Open-Meteo body (serde parse), city the echo of the user's argument
    // (expect_string_arg → resolved_city), description the in-tree WMO
    // table (wmo_description, &'static str) and country the in-tree
    // constant ""; the UserInput-sourced echo is untrusted per
    // labels.rs:509); llm_usage — the in-process Rust counters
    // (LlmUsageReport, llm.rs), no wire, no user input → internal.
    // The honest exclusions: json_body/form_data (dynamic user shapes —
    // no fixed field vocabulary exists), Tool/DayForecast (List elements —
    // the section is a Struct form at stage 2).
    spec!("geo_ip", 0, 1, "web"; builtin_geo_ip, "Struct<GeoLocation>{ip:untrusted,city:untrusted,region:untrusted,country:untrusted,country_code:untrusted,lat:untrusted,lon:untrusted,isp:untrusted,timezone:untrusted}"), // ip? (omit = caller IP; builtin_geo_ip)
    spec!("weather", 2, "web"; builtin_weather, "Struct<Weather>{temp:untrusted,feels_like:untrusted,temp_min:untrusted,temp_max:untrusted,humidity:untrusted,description:internal,wind_speed:untrusted,wind_direction:untrusted,pressure:untrusted,cloud_cover:untrusted,is_day:untrusted,city:untrusted,country:internal}"),
    spec!("geo_distance", 2, 5, "web"; builtin_geo_distance, "Float"),
    spec!("weather_forecast", 1, 3, "web"; builtin_weather_forecast, "List<DayForecast>"), // city | lat,lon | lat,lon,days
    // ── Crypto builtins ──
    spec!("hash_password", 1, "crypto"; builtin_hash_password),
    spec!("verify_password", 2, "crypto"; builtin_verify_password, "Bool"),
    spec!("encrypt", 2, "crypto"; builtin_encrypt),
    spec!("decrypt", 2, "crypto"; builtin_decrypt),
    spec!("generate_key", 0, "crypto"; builtin_generate_key),
    spec!("base64_encode", 1, "encoding"; builtin_base64_encode, "String"),
    spec!("base64_decode", 1, "encoding"; builtin_base64_decode, "String"),
    // ── Auth stubs (interpreter-mode mocks; real auth requires server mode; builtin_base64_decode) ──
    // authenticate: always returns Unit — mock; no user database in interpreter
    spec!("authenticate", 2, "stub"; builtin_authenticate, "Unit"), // №613: the registered handler IS the interpreter surface — the mock contract is Ok(Unit)
    // ── Session (№348, ADR-0172): real session surface over the process-global
    // registry (src/session.rs) — the pre-№348 mock handlers lived above in
    // crypto.rs. Every transition is an Action-Ledger record (ADR-0167 §3.4).
    spec!("session_login", 2, "session"; builtin_session_login),
    spec!("session_logout", 1, "session"; builtin_session_logout, "Unit"),
    spec!("session_duty_enter", 1, "session"; builtin_session_duty_enter, "Bool"),
    spec!("session_duty_exit", 1, "session"; builtin_session_duty_exit, "Bool"),
    spec!("session_wake", 2, 3, "session"; builtin_session_wake, "String"), // №613: returns the delivered source (String) so programs can branch
    spec!("session_poll_wake", 1, "session"; builtin_session_poll_wake),
    spec!("session_interrupt", 2, 3, "session"; builtin_session_interrupt, "String"), // №613: returns the accepted priority (String)
    spec!("session_take_interrupt", 1, "session"; builtin_session_take_interrupt),
    // №539: String — the literal "ok" return (verified handler; not Unit).
    spec!("session_clear", 1, "memory"; builtin_session_clear, "String"), // ── Bot — Telegram messaging ──
    spec!("send_message", 2, 3, "bot"; builtin_send_message), // №543: Unknown honest — String (delivered) | Unit (no TELEGRAM_BOT_TOKEN fallback); the flat vocabulary has no env-dependent union
    spec!("answer_callback_query", 1, 3, "bot"; builtin_answer_callback_query), // №543: Unknown honest — String | Unit (no-token fallback), same env-dependent split as send_message
    spec!("edit_message_text", 3, 4, "bot"; builtin_edit_message_text), // №543: Unknown honest — String | Unit (no-token fallback), same env-dependent split as send_message
    // ── Voice / transcription ──
    // Naryad #279 fact-check: registry said min=1, implementation has always
    // required 3 strings (file_id, bot_token, whisper_key) + optional provider.
    // A 1-arg call passed mlog check and exploded at runtime — fixed to 3..4.
    spec!("whisper_transcribe", 3, 4, "voice"; builtin_whisper_transcribe, "String"), // file_id,bot_token,whisper_key | +provider
    spec!("tts_send", 4, 5, "voice"; builtin_tts_send, "String"), // text,voice,bot_token,chat_id | +mode — delivery convenience (delegates synthesis to tts_synth, Naryad #279)
    spec!("tts_generate", 2, 4, "voice"; builtin_tts_generate, "String"), // Naryad #279: text,voice | +provider | +model — synthesis to sandbox file, no delivery (APPENDED: bytecode indices must not shift)
    // ── System builtins ──
    spec!("env", 1, "system"; builtin_env, "String"),
    // №481: the EXPLICIT-silence twin of env — the `_or` suffix carries the
    // silent-default semantics in the name (audit 25.09 §3.9 naming rule).
    spec!("env_or", 2, "system"; builtin_env_or, "String"), // ── DB builtins ──
    spec!("query", 1, 2, "db"; builtin_query), // №538: Unknown honest — returns the opaque Query wrapper (Value::Query); the flat vocabulary has no Query entry, and the EXECUTION result type depends on the backend
    spec!("db_execute", 1, 2, "db"; builtin_db_execute, "Unit"), // ADR-0068: optional params list — the interpreter path returns Unit; the executed surface is the Query pipeline (№484 DbAccess)
    // ── LLM builtins ──
    #[cfg(feature = "llm")]
    spec!("call_llm", 1, 2, "llm"; builtin_call_llm, "String"), // prompt | prompt,input
    #[cfg(feature = "llm")]
    spec!("call_claude", 4, "llm"; builtin_call_claude, "String"), // api_key,model,system,user
    #[cfg(feature = "llm")]
    spec!("llm_usage", 0, "llm"; builtin_llm_usage, "Struct<LlmUsage>{total_calls:internal,total_tokens:internal,total_errors:internal,cache_hits_semantic:internal,canary_leaks:internal,providers:internal}"),
    #[cfg(feature = "llm")]
    spec!("call_llm_schema", 2, 3, "llm"; builtin_call_llm_schema), // prompt,schema | prompt,input,schema (Наряд №269, ADR-0133)
    // ── Memory builtins ──
    spec!("kv_set", 2, "memory"; builtin_kv_set, "Unit"),
    spec!("kv_get", 1, "memory"; builtin_kv_get, "String"),
    spec!("kv_delete", 1, "memory"; builtin_kv_delete, "Unit"),
    spec!("kv_exists", 1, "memory"; builtin_kv_exists, "Bool"),
    spec!("kv_list", 0, "memory"; builtin_kv_list, "List<String>"), // №631: the String keys (memory.rs: Value::String map)
    // №539: the mem_* twins RETURN the value (String) — the honest
    // asymmetry vs kv_set/kv_delete (Unit), verified in the handlers.
    spec!("mem_set", 2, "memory"; builtin_mem_set, "String"),
    spec!("mem_get", 1, "memory"; builtin_mem_get, "String"),
    spec!("mem_delete", 1, "memory"; builtin_mem_delete, "String"),
    spec!("memorize", 2, 3, "memory"; builtin_kv_set, "Unit"), // №539: the kv_set handler — Unit
    // Наряд №442: recall — the front door of memory, a REAL handler
    // (zero stub-spec on the name). The registry-level handler serves
    // the TYPED lane (the only state a bare fn can reach): consent-
    // gated private containers (fail-closed MEMORY_RECALL_CONSENT_
    // REQUIRED), provenance via the [MEM] suffix, the memory.recall
    // ledger family. The TW/VM state-carrying blocks intercept by name
    // BEFORE this fallback and add their store lanes (the bug #530
    // twin pattern) — same gate, same suffix, same ledger.
    // Bug #530 (FO-050 / office #182): recall_top_k is the memory READ the
    // interpreter dispatches through its interception table
    // (interpreter::memory::invoke_recall_top_k_fn — hybrid search over the
    // interpreter's memory store); it had NO registry entry, so the VM
    // compiler refused every program calling it ("undefined function") while
    // the TW ran it — a TW/VM registry disagreement. The name is now
    // registered (compile parity); the VM dispatches it through its own
    // state-carrying block (vm.rs call_builtin) against the VM's memory —
    // handler stays None here because both backends intercept by name before
    // the generic fallback (the media_store_* stub pattern).
    spec!("recall_top_k", 1, 3, "memory"), // №539: handlerless row — the spec! macro keeps it untyped (no honest fill-site type for a no-handler row); the verified intercept fact for stage 1: BOTH backends return the serialized JSON hit array as a String (memory_ops recall_top_k_tw / recall_top_k_vm)
    // Наряд №272 (ADR-0134): векторный контур поверх sqlite-vec — KNN
    // (distance_metric=cosine), песочница через sandbox_path_ex, dim-гейт.
    #[cfg(feature = "vec")]
    spec!("embed", 1, "memory"; builtin_embed, "List<Float>"), // №539: the Float embedding vector; №631: the Float element read off the handler (Value::Float map)
    #[cfg(feature = "vec")]
    spec!("vec_store", 4, 5, "memory"; builtin_vec_store, "Struct<VecStoreResult>{stored:internal,table:untrusted,id:untrusted,dim:internal,rowid:internal}"), // db_path,table,id,embedding | +text|opts{text,scope} (№281) — №539: the VecStoreResult; №631: the name read off make_struct; №638: the fields read off make_struct (vector.rs:360-368) — table/id echo the caller's arguments → untrusted, stored/dim/rowid computed in-tree → internal
    #[cfg(feature = "vec")]
    spec!("vec_search", 4, 5, "memory"; builtin_vec_search, "List<VecSearchHit>"), // db_path,table,query,k | +include_forgotten (№280, дефолт false) — №539: the hit structs (the post-filter may shrink below k); №631: the VecSearchHit element read off make_struct (all three arms)
    // Наряд №442: the registry-level recall row (see the №442 comment at
    // the memory block head) — a REAL handler over the typed lane; the
    // stub-spec row is gone. forget/find/inspect remain the planned
    // high-level memory API rows (use kv_*/mem_* instead).
    spec!("recall", 1, 2, "memory"; builtin_recall, "String"), // №539: the best hit's text + the [MEM] provenance suffix; the miss is ""
    // ── Typed Memory<K> (№350): label-typed containers over the
    // process-global registry (src/memory_typed.rs) — private is
    // consent-gated + encrypted at rest, reads/exports are audited
    // sinks, redact is the only private egress. Category "memory".
    spec!("memory_open", 2, "memory"; builtin_memory_open), // №539: Unknown honest — the opaque Value::Memory handle; the flat vocabulary has no Memory spelling (the №538 Query posture)
    spec!("memory_put", 3, 5, "memory"; builtin_memory_put, "Unit"),
    spec!("memory_read", 2, "memory"; builtin_memory_read), // №539: Unknown honest — String|Secret by the container label (public → String, private → the gated Secret); typing String would false-warn the private lane
    spec!("memory_keys", 1, "memory"; builtin_memory_keys, "List<String>"), // №631: the String keys (memory_typed.rs)
    spec!("memory_provenance", 2, "memory"; builtin_memory_provenance, "List<String>"), // №631: the String parents (memory_typed.rs)
    spec!("memory_export", 3, "memory"; builtin_memory_export, "String"), // №539: the exported path
    // №351 (ADR-0173): the derived-graph surfaces — the cascade preview
    // (the №280 dry-run discipline), the retain pins, and the grant-gated
    // cascading forget (the ADR-0155 linear action, `irreversible.
    // memory_forget` ledger record; scope `memory:forget:<container>`).
    spec!("memory_cascade_preview", 2, "memory"; builtin_memory_cascade_preview, "Struct<MemoryCascadePlan>{closure:internal,blocked_by:internal}"), // №539: the MemoryCascadePlan; №631: the name read off make_struct; №638: both fields are the in-tree graph computation (memory_typed.rs:286-294) → internal
    spec!("memory_retain", 2, "memory"; builtin_memory_retain, "Unit"),
    spec!("memory_release", 2, "memory"; builtin_memory_release, "Unit"),
    spec!("memory_retained", 1, "memory"; builtin_memory_retained, "List<String>"), // №631: the String keys (memory_typed.rs)
    spec!("memory_forget_cascade", 3, "memory"; builtin_memory_forget_cascade, "Struct<MemoryForgetResult>{root:untrusted,deleted:internal,batch_id:internal}"), // №539: the MemoryForgetResult; №631: the name read off forget_outcome_value; №638: the fields read off forget_outcome_value (memory_typed.rs:249-268) — root echoes the caller's key → untrusted, deleted/batch_id computed → internal (NOTE: the SAME type_name carries a DIFFERENT field set on the forget row — the two-shapes observation recorded in the PR)
    // №352 (ADR-0174): the duplex channel — barge-in over the №348
    // session priority ladder. NOT feature-gated: the CI contour
    // exercises the same state machine as the serve contour. The
    // direction is STATIC (separate *_start/*_stop surfaces) so the
    // №324 effect trail can type it (speak_start → ⟨speak⟩ etc.).
    spec!("duplex_open", 1, 2, "voice"; builtin_duplex_open),
    spec!("speak_start", 2, 3, "voice"; builtin_speak_start),
    spec!("listen_start", 1, 2, "voice"; builtin_listen_start),
    spec!("speak_stop", 1, "voice"; builtin_speak_stop),
    spec!("listen_stop", 1, "voice"; builtin_listen_stop),
    spec!("duplex_state", 1, "voice"; builtin_duplex_state),
    // №355 (registry В5 — Phase 5 «Embodied, sim-only», ADR-0159): the
    // embodied surface over the sim contour. NOT feature-gated: the
    // contour is sim-only (in-tree deterministic records — no hardware,
    // no GPU, no weights); the monitor/state evolution lands with №356
    // BEHIND the GPU-budget gate, the TYPES are live now.
    spec!("device_open", 1, 2, "embodied"; builtin_device_open),
    spec!("bounds_attach", 2, "embodied"; builtin_bounds_attach),
    spec!("device_state", 1, "embodied"; builtin_device_state),
    spec!("world_state", 1, "embodied"; builtin_world_state),
    spec!("pose_make", 4, "embodied"; builtin_pose_make),
    spec!("trajectory_make", 1, "embodied"; builtin_trajectory_make),
    spec!("goal_make", 1, "embodied"; builtin_goal_make),
    spec!("chunk_make", 2, 3, "embodied"; builtin_chunk_make),
    spec!("proof_seal", 2, 3, "embodied"; builtin_proof_seal),
    spec!("proof_verify", 1, "embodied"; builtin_proof_verify),
    // ── Naryad #445 (P1, feature/memory): the forgetting memory — the
    // canon §10.3 front door on the name forget (the stub-spec row is
    // GONE; the row keeps its position — bytecode indices stable).
    // forget(handle, key, grant, dry_run?) — the ADR-0155 linear action
    // (scope `memory:forget:<container>`), the dry_run preview → apply,
    // the derived-from cascade → poisoned with closed sinks, the
    // consent fail-closed gate (MEMORY_FORGET_CONSENT_REQUIRED) and the
    // ledger family memory.forget / memory.forget.denied /
    // irreversible.memory_forget. The legacy 1..2-argument
    // forget(query, days?) surface (№72) is intact — the TW/VM
    // intercepts keep it; 3..4 arguments fall through to the handler.
    spec!("forget", 3, 4, "memory"; builtin_forget, "Struct<MemoryForgetResult>{candidates:internal,applied:internal,batch_id:internal}"), // №539: the 3..4-argument typed front door (MemoryForgetResult); №631: the name read off make_struct — the legacy 1..2-argument forget(query, days?) intercepts first and returns Unit — a different arity surface; №638: the fields read off struct_value BOTH arms (memory_forget.rs:464, 538) — candidates/applied/batch_id computed in-tree → internal (NOTE: the type_name collides with the cascade row's shape — see the PR)
    spec!("find", 4, "stub"),
    spec!("inspect", 1, "stub"),
    // conv_start/add/history/context/end: conversation lifecycle management; not yet implemented
    spec!("conv_start", 1, "stub"),
    spec!("conv_add", 3, "stub"),
    spec!("conv_history", 1, "stub"),
    spec!("conv_context", 1, "stub"),
    spec!("conv_end", 1, "stub"),
    // №523: the spec is 3 — the implementation (builtins/memory.rs
    // builtin_session_set) hard-requires (session_id, key, value) and
    // refuses 2 arguments loudly at runtime; the 2 here was a stale
    // pre-session-id spec the №523 run gate surfaced (examples/p8 calls
    // with 3 on every line).
    spec!("session_set", 3, "memory"; builtin_session_set, "String"), // №539: the stored value
    // №523: the specs are the implementation's truth (builtins/memory.rs):
    // session_get hard-requires (session_id, key); session_clear requires
    // exactly (session_id). The stale 1/0 specs predated the session-id
    // parameter; the №523 run gate surfaced the drift (examples/p8).
    spec!("session_get", 2, "memory"; builtin_session_get, "String"),
    spec!("ref", 1, "memory"; builtin_content_ref, "String"), // №539: the SHA-256 hex hash
    spec!("deref", 1, "memory"; builtin_content_deref, "String"), // ── Time builtins ──
    spec!("now", 0, "time"; builtin_now, "Float"),
    spec!("sleep", 1, "time"; builtin_sleep, "Unit"),
    spec!("time", 0, "time"; builtin_now, "Float"),
    spec!("add_days", 2, "time"; builtin_add_days, "Float"),
    spec!("add_hours", 2, "time"; builtin_add_hours, "Float"),
    spec!("date_parts", 1, "time"; builtin_date_parts),
    spec!("format_date", 2, "time"; builtin_format_date, "String"),
    spec!("days_between", 2, "time"; builtin_days_between, "Float"),
    spec!("days_in_month", 2, "time"; builtin_days_in_month, "Float"),
    spec!("is_leap_year", 1, "time"; builtin_is_leap_year, "Bool"),
    // №591: typed PRECISE ("String" — the verified single-path handler
    // fact: the only Ok arm is Value::String(WEEKDAY_NAMES_MON[...]); the
    // №565 procedure) — compensates the typed-but-coarse "Struct" of the
    // spectral rows so the precise share keeps moving only up (№560).
    spec!("weekday_name", 1, "time"; builtin_weekday_name, "String"), // ── Graph builtins ──
    spec!("graph_query", 1, 3, "graph"; builtin_graph_query), // query | query,limit | query,limit,level
    spec!("graph_path", 2, "graph"; builtin_graph_path),      // from_id,to_id
    spec!("graph_neighbors", 0, "graph"; builtin_graph_neighbors),
    spec!("memory_decay", 0, "memory"; builtin_memory_decay, "Struct<DecayResult>{decayed:internal,nodes:internal,edges:internal,components:internal}"), // №539: the DecayResult; №631: the name read off make_date_struct; №638: all four fields are the in-tree graph computation (memory.rs:865-871) → internal
    spec!("memory_boost", 0, "memory"; builtin_memory_boost, "Struct<BoostResult>{id:untrusted,new_score:internal,access_count:internal}"), // №539: the BoostResult; №631: the name read off make_date_struct; №638: the fields read off make_date_struct (memory.rs:893-902) — id echoes the caller's argument → untrusted, the metrics computed → internal
    spec!("memory_prune", 0, "memory"; builtin_memory_prune, "Struct<PruneResult>{pruned:internal,remaining:internal}"), // №539: the PruneResult; №631: the name read off make_date_struct; №638: both fields computed in-tree (memory.rs:924-929) → internal
    spec!("memory_revise", 0, "memory"; builtin_memory_revise, "Struct<ReviseResult>{action:internal,winner_id:internal,superseded_id:internal}"), // №539: the ReviseResult; №631: the name read off make_date_struct; №638: the fields read off make_date_struct (memory.rs:957-964) — all computed in-tree; superseded_id is OPTIONAL (present only when a contradiction resolved) → internal
    spec!("subgraph_extract", 0, "graph"; builtin_subgraph_extract),
    spec!("subgraph_nodes", 0, "graph"; builtin_subgraph_nodes),
    spec!("subgraph_json", 0, "graph"; builtin_subgraph_json),
    spec!("trace_start", 0, "graph"; builtin_trace_start),
    spec!("trace_end", 0, "graph"; builtin_trace_end),
    spec!("memory_score", 1, "bot" => "ext"; builtin_memory_score, "Struct<MemoryScore>{score:internal,admitted:internal,token_count:internal,unique_words:internal,entity_density:internal}"), // №543: the MemoryScore make_date_struct (verified office/text.rs); №631: the name read off the handler; №638: all five fields are the in-tree weighted computation (office/text.rs:424-441) → internal
    spec!("mtree_summarize", 0, "mtree"; builtin_mtree_summarize),
    spec!("mtree_retrieve", 1, 2, "mtree"; builtin_mtree_retrieve), // query | query,limit
    spec!("mtree_store", 2, "mtree"; builtin_mtree_store),
    spec!("mtree_stats", 0, "mtree"; builtin_mtree_stats),
    spec!("mtree_forget", 1, "mtree"; builtin_mtree_forget), // ── Cron builtins ──
    spec!("cron_mark_fired", 1, "cron"; builtin_cron_mark_fired_stamped),
    // №418: 2..5 — the optional tz / catch_up / payload extensions
    // (additive arity widening; the 2-arg 0.20.x call shape is unchanged).
    spec!("cron_add", 2, 5, "cron"; builtin_cron_add_stamped), // cron_expr, prompt, tz?, catch_up?, payload?
    spec!("cron_list", 0, "cron"; builtin_cron_list_stamped),
    spec!("cron_remove", 1, "cron"; builtin_cron_remove_stamped),
    spec!("cron_run", 1, "cron"; builtin_cron_run_stamped), // ── Event / query analytics stubs ──
    // event_count/events_since/event_sum: planned event analytics; no handler (use query with SQL instead)
    spec!("event_count", 0, "stub"),
    spec!("events_since", 1, "stub"),
    spec!("event_sum", 2, "stub"), // query_scalar/query_row: planned convenience wrappers; no handler (use query + json_get instead)
    spec!("query_scalar", 0, "stub"),
    spec!("query_row", 0, "stub"), // ── Test builtins ──
    spec!("assert_eq", 2, "test"; builtin_assert_eq),
    spec!("assert_contains", 2, "test"; builtin_assert_contains), // ── Fluid builtins ──
    spec!("confidence", 1, "fluid"; builtin_confidence, "Float"), // ── Encoding builtins ──
    spec!("toon_encode", 1, "encoding"; builtin_toon_encode, "String"),
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
    spec!("fuzzy_find_best", 2, "string"; builtin_fuzzy_find_best), // №536: Unknown honest — Unit (no candidates) | struct{index, candidate, score}
    spec!("hashline_read", 1, "string"; builtin_hashline_read, "String"),
    spec!("hashline_edit", 2, "string"; builtin_hashline_edit, "String"),
    spec!("compact_list", 3, "list"; builtin_compact_list), // №537: Unknown honest — List | the struct report{compacted, removed_count}
    spec!("budget_check", 2, "fluid"; builtin_budget_check),
    spec!("replay_snapshot", 1, "system"; builtin_replay_snapshot),
    spec!("policy_check", 1, "system"; builtin_policy_check), // ── obsidian-mind: Vault / semantic search ──
    spec!("semantic_search", 3, "vault" => "ext"; builtin_semantic_search),
    spec!("config_load", 1, "vault" => "ext"; builtin_config_load),
    spec!("vault_validate", 2, "vault" => "ext"; builtin_vault_validate),
    // ── Bot — Telegram ──
    spec!("todo_add", 2, "bot" => "ext"; builtin_todo_add, "Struct<Todo>{id:internal,title:untrusted,status:untrusted}"), // №543: the Todo make_date_struct (verified goals.rs); №631: the name read off the handler; №638: the fields read off make_date_struct (goals.rs:265-270) — id computed → internal; title echoes the argument, status echoes the optional argument (default const) → untrusted
    spec!("todo_list", 0, "bot" => "ext"; builtin_todo_list, "List<Todo>"), // №543: the todo rows vector (verified goals.rs); №631: the Todo element read off the handler
    spec!("todo_update", 2, "bot" => "ext"; builtin_todo_update, "Struct<TodoUpdate>{id:untrusted,old_status:internal,new_status:untrusted,updated:internal}"), // №543: the TodoUpdate make_date_struct (verified goals.rs); №631: the name read off the handler; №638: the fields read off make_date_struct (goals.rs:308-315) — id/new_status echo the arguments → untrusted; old_status read from the in-tree store, updated computed → internal
    spec!("goal_get", 0, "bot" => "ext"; builtin_goal_get, "Struct<ThreadGoal>{objective:internal,status:internal,budget:internal,spent:internal}"), // №543: the ThreadGoal make_date_struct (verified goals.rs); №631: the name read off the handler (both paths); №638: the fields read off make_date_struct ALL 3 arms (goals.rs:38-43, 56-62, 76-81) — reads from the in-tree kv store, no call-argument echo (0-arg row) → internal
    spec!("goal_set", 2, "bot" => "ext"; builtin_goal_set, "Struct<ThreadGoal>{objective:untrusted,status:internal,budget:untrusted,spent:internal}"), // №543: the ThreadGoal make_date_struct (verified goals.rs); №631: the name read off the handler; №638: the fields read off make_date_struct (goals.rs:38-43) — objective/budget echo the arguments → untrusted (the geo_ip ip-field precedent); status const, spent const → internal
    spec!("goals_add", 1, "bot" => "ext"; builtin_goals_add, "Struct<Goal>{id:internal,text:untrusted,status:internal}"), // №543: the Goal make_date_struct (verified goals.rs); №631: the name read off the handler; №638: the fields read off make_date_struct (goals.rs:172-180) — id computed (g{n}) → internal; text echoes the argument → untrusted; status const → internal
    spec!("goals_list", 0, "bot" => "ext"; builtin_goals_list, "List<Goal>"), // №543: the goal rows vector (verified goals.rs); №631: the Goal element read off the handler
    spec!("remind", 3, "bot"; builtin_remind_stamped, "String"), // №543: the reminder id (verified cron.rs builtin_remind; the stamped wrapper is pass-through)
    spec!("get_profile", 0, "bot" => "ext"; builtin_get_profile, "List<Preference>"), // №543: the profile rows vector (verified); №631: the Preference element read off the handler
    spec!("human_mood", 3, "bot"; builtin_human_mood, "Struct<Mood>{persona:untrusted,mood:internal,intensity:internal,updated_at:internal}"), // №543: the Mood make_date_struct (verified server.rs); №631: the name read off the handler; №638: the fields read off make_date_struct (server.rs:654-663) — persona echoes the caller's argument → untrusted; mood/intensity/updated_at read from the in-tree persona store → internal
    spec!("ask_approval", 1, "bot" => "ext"; builtin_ask_approval, "Struct"), // №543: the Approval make_date_struct (verified config.rs)
    spec!("goal_complete", 0, "bot" => "ext"; builtin_goal_complete, "Struct"), // №543: the GoalComplete make_date_struct (verified goals.rs)
    spec!("goals_reflect", 0, "bot" => "ext"; builtin_goals_reflect, "Struct"), // №543: the GoalsReflection make_date_struct (verified goals.rs)
    spec!("cancel_remind", 1, "bot"; builtin_cancel_remind_stamped, "String"), // №543: the literal "ok"/"not_found" (verified cron.rs)
    spec!("check_reminders", 0, "bot"; builtin_check_reminders_stamped, "List"), // №543: the DueReminder rows vector (verified cron.rs)
    spec!("list_reminders", 0, "bot"; builtin_list_reminders_stamped, "List"), // №543: the Reminder rows vector (verified cron.rs)
    spec!("remind_recurring", 2, "bot"; builtin_remind_recurring_stamped, "String"), // №543: the reminder id (verified cron.rs)
    spec!("human_create", 2, "bot"; builtin_human_create, "Struct"), // №543: the Persona make_date_struct (verified server.rs)
    spec!("human_delete", 1, "bot" => "ext"; builtin_human_delete, "Struct"), // №543: the DeleteResult make_date_struct (verified office/human.rs)
    spec!("human_forget", 2, "bot"; builtin_human_forget), // №543: Unknown honest — Float (deleted count, 1-arg all-persona path) | String ("ok"/"not_found", 2-arg path); arity-dependent union
    spec!("human_personas", 0, "bot" => "ext"; builtin_human_personas, "List"), // №543: the persona rows vector (verified)
    spec!("human_recall", 3, "bot"; builtin_human_recall, "List"), // №543: the matched memory rows vector (verified server.rs)
    spec!("human_remember", 4, "bot"; builtin_human_remember, "String"), // №543: the literal "ok" (verified server.rs)
    spec!("human_respond", 2, "bot" => "ext"; builtin_human_respond, "String"), // №543: the reply text (delegates to human_recall; verified)
    spec!("compress_html", 1, "bot" => "ext"; builtin_compress_html, "String"), // №543: the compressed HTML (verified)
    spec!("estimate_tokens", 1, "bot" => "ext"; builtin_estimate_tokens, "Float"), // №543: the token estimate (verified)
    spec!("extract_entities", 1, "bot" => "ext"; builtin_extract_entities, "List"), // №543: the entity rows vector (verified)
    spec!("extract_param", 2, "bot" => "ext"; builtin_extract_param, "String"), // text,index — №543: the extracted substring (verified)
    spec!("learn_preference", 3, "bot" => "ext"; builtin_learn_preference, "Struct"), // №543: the Preference make_date_struct (verified office/human.rs)
    spec!("read_file_tokens", 1, "bot" => "ext"; builtin_read_file_tokens, "Struct"), // №543: the token-accounting struct (verified)
    // ── sqz-inspired: string/list utilities ──
    spec!("squeeze", 2, "string"; builtin_squeeze, "String"),
    spec!("to_int", 1, "string"; builtin_to_int, "Float"), // parse string/float to integer
    // ── PDF processing (Наряд №48) ──
    spec!("pdf_classify", 1, "pdf"; builtin_pdf_classify, "List"), // №543: the pdf.rs make_dict — a List of String keys/values (verified pdf.rs:363-371); №637: stays bare — the flat [String,Float,List,Float] is heterogeneous, no honest element type (№616 honest Unknown)
    spec!("pdf_to_markdown", 1, "pdf"; builtin_pdf_to_markdown, "List"), // №543: the same make_dict shape (verified); №637: stays bare — the flat mixed-type pairs, no honest element type (№616)
    spec!("pdf_extract_regions", 2, "pdf"; builtin_pdf_extract_regions, "List"), // №543: the region structs vector (verified); №637: stays bare — a List of flat pseudo-dict lists (heterogeneous key/value pairs), no honest element type (№616)
    spec!("pdf_ocr", 1, "pdf"; builtin_pdf_ocr, "List"), // №543: the make_dict shape (verified); №637: stays bare — the flat mixed-type pairs, no honest element type (№616) // ── PDF creation & manipulation (Наряд MLG-1; builtin_pdf_ocr) ──
    spec!("pdf_create", 0, "pdf"; builtin_pdf_create, "Struct<PdfDocId>{id:internal}"), // → { id } — №543: the PdfDocId make_struct (verified); №637: PdfDocId{ id } (pdf.rs:591) — the id computed in-tree (uuid v4) → internal
    spec!("pdf_add_page", 3, "pdf"; builtin_pdf_add_page, "Struct<PdfPage>{page:internal}"), // id, width, height — №543: the PdfPage make_struct (verified); №637: PdfPage{ page } (pdf.rs:629-632) — the 1-based page count computed in-tree → internal
    spec!("pdf_write_text", 4, 6, "pdf"; builtin_pdf_write_text, "Struct<PdfResult>{ok:internal}"), // id, x, y, text [,font, size] — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:687) — the constant true → internal
    spec!("pdf_draw_line", 5, 6, "pdf"; builtin_pdf_draw_line, "Struct<PdfResult>{ok:internal}"), // id, x1, y1, x2, y2 [,width] — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:730)
    spec!("pdf_draw_rect", 5, 7, "pdf"; builtin_pdf_draw_rect, "Struct<PdfResult>{ok:internal}"), // id, x, y, w, h [,stroke, fill] — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:780)
    spec!("pdf_save", 2, "pdf"; builtin_pdf_save, "Struct<PdfFile>{path:untrusted,size:internal}"), // id, path — №543: the PdfFile make_struct (verified); №637: PdfFile{ path, size } (pdf.rs:818-821) — path echoes the user argument → untrusted, size computed in-tree → internal
    spec!("pdf_merge", 2, "pdf"; builtin_pdf_merge, "Struct<PdfMerge>{path:untrusted,pages:internal,size:internal}"), // paths_json, output — №543: the PdfMerge make_struct (verified); №637: PdfMerge{ path, pages, size } both arms (pdf.rs:1989-1994, 2164-2170) — path echoes the output argument, pages/size computed
    spec!("pdf_split", 3, "pdf"; builtin_pdf_split, "Struct<PdfSplit>{files:untrusted,pages:internal}"), // path, ranges_json, output_dir — №543: the PdfSplit make_struct (verified); №637: PdfSplit{ files, pages } (pdf.rs:2261-2264) — the file paths embed the user-supplied output_dir → untrusted, pages computed
    spec!("pdf_metadata", 1, "pdf"; builtin_pdf_metadata, "Struct<PdfMetadata>{title:untrusted,author:untrusted,subject:untrusted,creator:untrusted,producer:untrusted,pages:untrusted,created:untrusted,modified:untrusted}"), // path — №543: the PdfMetadata make_struct (verified); №637: PdfMetadata{ 8 fields } (pdf.rs:2345-2356) — every field read off the EXTERNAL PDF body → untrusted
    spec!("pdf_set_metadata", 3, "pdf"; builtin_pdf_set_metadata, "Struct<PdfResult>{ok:internal}"), // path, key, value — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:2431)
    spec!("html_to_pdf", 2, "pdf"; builtin_html_to_pdf, "Struct<PdfFile>{path:untrusted,size:internal}"), // html, path — №543: the PdfFile make_struct (verified); №637: PdfFile{ path, size } both arms (pdf.rs:2458-2468) — path echoes the user argument, size computed
    spec!("send_document", 2, 3, "bot" => "ext"; builtin_send_document), // chat_id, file_path [,caption] — №543: Unknown honest — String | Unit (no-token fallback), same env-dependent split as send_message
    // ── Crypto: SHA-256 / HMAC (Наряд №50 Block 3) ──
    spec!("sha256", 1, "crypto"; builtin_sha256, "String"),
    spec!("hmac_sha256", 2, "crypto"; builtin_hmac_sha256, "String"),
    spec!("hex_encode", 1, "crypto"; builtin_hex_encode, "String"),
    spec!("hex_decode", 1, "crypto"; builtin_hex_decode),
    // Наряд №172: secret() — reads env var as Value::Secret directly
    // (hard-failure if missing, unlike env() which returns empty string).
    spec!("secret", 1, "crypto"; builtin_secret), // ── Regex (Наряд №54; builtin_hex_decode) ──
    spec!("regex_match", 2, "string"; builtin_regex_match, "Bool"),
    spec!("regex_captures", 2, "string"; builtin_regex_captures, "List"),
    spec!("regex_replace", 3, "string"; builtin_regex_replace, "String"), // ── PDF office automation (Наряд MLG-3; builtin_regex_replace) ──
    spec!("pdf_draw_table", 5, 6, "pdf"; builtin_pdf_draw_table, "Struct<PdfResult>{ok:internal}"), // id, x, y, col_widths_json, rows_json [,style_json] — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:1385)
    spec!("pdf_add_image", 4, 6, "pdf"; builtin_pdf_add_image, "Struct<PdfResult>{ok:internal}"), // id, x, y, image_path [,width, height] — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:1442)
    spec!("pdf_set_page_header", 2, 4, "pdf"; builtin_pdf_set_page_header, "Struct<PdfResult>{ok:internal}"), // id, text [,font, size] — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:1516)
    spec!("pdf_set_page_footer", 2, 4, "pdf"; builtin_pdf_set_page_footer, "Struct<PdfResult>{ok:internal}"), // id, text [,font, size] — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:1558)
    spec!("pdf_page_numbers", 1, 4, "pdf"; builtin_pdf_page_numbers, "Struct<PdfResult>{ok:internal}"), // id [,format, x, y] — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:1595)
    spec!("pdf_watermark", 2, 5, "pdf"; builtin_pdf_watermark, "Struct<PdfResult>{ok:internal}"), // id, text [,font, size, opacity] — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:1645)
    spec!("pdf_fill_form", 3, "pdf"; builtin_pdf_fill_form, "Struct<PdfFillForm>{path:untrusted,fields_filled:internal}"), // path, fields_json, output_path — №543: the PdfFillForm make_struct (verified); №637: PdfFillForm{ path, fields_filled } (pdf.rs:1737-1744) — path echoes output_path → untrusted, the count computed
    spec!("pdf_rotate_page", 4, "pdf"; builtin_pdf_rotate_page, "Struct<PdfResult>{ok:internal}"), // path, page_number, degrees, output_path — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:1799)
    spec!("pdf_delete_pages", 3, "pdf"; builtin_pdf_delete_pages, "Struct<PdfDeletePages>{ok:internal,pages_remaining:internal}"), // path, pages_json, output_path — №543: the PdfDeletePages make_struct (verified); №637: PdfDeletePages{ ok, pages_remaining } (pdf.rs:1852-1855) — both computed in-tree
    spec!("pdf_extract_images", 1, 2, "pdf"; builtin_pdf_extract_images, "List<String>"), // path [,output_dir] — №543: the extracted paths vector (verified); №637: the String path element read off the handler (extracted_paths.push(Value::String(out_path)), pdf.rs:1938)
    // ── Email: SMTP + IMAP (Наряд MLG-4) ──
    spec!("smtp_send", 3, 6, "email"; builtin_smtp_send), // to, subject, body [,attachments_json, from, reply_to]
    spec!("smtp_send_html", 3, 4, "email"; builtin_smtp_send_html), // to, subject, html [,attachments_json]
    spec!("imap_list", 2, 3, "email"; builtin_imap_list),           // folder, limit [,since_date]
    spec!("imap_read", 1, "email"; builtin_imap_read),              // uid
    spec!("imap_search", 2, "email"; builtin_imap_search),          // query, folder
    spec!("imap_mark_read", 1, "email"; builtin_imap_mark_read),    // uid
    spec!("imap_move", 2, "email"; builtin_imap_move),              // uid, dest_folder
    // ── Наряд MLG-5: Calendar (CalDAV + iCal) ──
    spec!("cal_connect", 3, "calendar"; builtin_cal_connect, "String"), // url, user, pass
    spec!("cal_list", 1, "calendar"; builtin_cal_list, "String"),       // session_id
    spec!("cal_events", 3, "calendar"; builtin_cal_events),             // calendar_id, start, end
    spec!("cal_read", 1, "calendar"; builtin_cal_read),                 // event_uid
    spec!("cal_create", 4, 7, "calendar"; builtin_cal_create), // cal_id, summary, start, end [,desc, location, attendees_json]
    spec!("cal_update", 2, "calendar"; builtin_cal_update, "String"), // event_uid, fields_json
    spec!("cal_delete", 1, "calendar"; builtin_cal_delete, "String"), // event_uid
    spec!("cal_freebusy", 3, "calendar"; builtin_cal_freebusy, "String"), // calendar_id, start, end
    spec!("ical_parse", 1, "calendar"; builtin_ical_parse),    // text
    spec!("ical_generate", 1, "calendar"; builtin_ical_generate, "String"), // event_json
    // ── Наряд MLG-6: Contacts (CardDAV + vCard) ──
    spec!("card_connect", 3, "contacts"; builtin_card_connect, "String"), // url, user, pass
    spec!("card_list", 1, "contacts"; builtin_card_list, "String"),       // session_id
    spec!("card_contacts", 2, "contacts"; builtin_card_contacts),         // addressbook_id, query
    spec!("card_read", 1, "contacts"; builtin_card_read, "String"),       // contact_uid
    spec!("card_create", 3, 7, "contacts"; builtin_card_create, "String"), // addressbook_id, fn, email [,tel, org, title, note]
    spec!("card_update", 2, "contacts"; builtin_card_update, "String"), // contact_uid, fields_json
    spec!("card_delete", 1, "contacts"; builtin_card_delete, "String"), // contact_uid
    spec!("card_search", 2, "contacts"; builtin_card_search),           // session_id, query
    spec!("vcard_parse", 1, "contacts"; builtin_vcard_parse, "String"), // text
    spec!("vcard_generate", 1, "contacts"; builtin_vcard_generate, "String"), // contact_json
    // ── Наряд №74: Native SVG Graphics & Diagrams (ADR-0102) ──
    // Level 1: SVG primitives — return XML fragments
    #[cfg(feature = "svg")]
    spec!("svg_rect", 5, 6, "svg"; builtin_svg_rect, "String"), // x, y, w, h, fill [, stroke]
    #[cfg(feature = "svg")]
    spec!("svg_circle", 4, "svg"; builtin_svg_circle, "String"), // cx, cy, r, fill
    #[cfg(feature = "svg")]
    spec!("svg_line", 5, 6, "svg"; builtin_svg_line, "String"), // x1, y1, x2, y2, stroke [, width]
    #[cfg(feature = "svg")]
    spec!("svg_text", 5, 6, "svg"; builtin_svg_text, "String"), // x, y, content, font_size, fill [, anchor]
    #[cfg(feature = "svg")]
    spec!("svg_path", 2, 3, "svg"; builtin_svg_path, "String"), // d, fill [, stroke]
    #[cfg(feature = "svg")]
    spec!("svg_group", 1, 2, "svg"; builtin_svg_group, "String"), // children [, transform]
    #[cfg(feature = "svg")]
    spec!("svg_canvas", 4, "svg"; builtin_svg_canvas, "String"), // width, height, viewbox, children
    // Level 2: design tokens
    #[cfg(feature = "svg")]
    spec!("diagram_style", 1, "tokens"; builtin_diagram_style, "Struct"), // {paper, ink, accent, muted, rule}
    // Level 2.5: wow-effects
    #[cfg(feature = "svg")]
    spec!("svg_sketchy_filter", 1, 5, "svg"; builtin_svg_sketchy_filter, "String"), // id [, base_freq, octaves, scale, seed]
    #[cfg(feature = "svg")]
    spec!("svg_icon", 5, "svg"; builtin_svg_icon, "String"), // name, x, y, size, color
    #[cfg(feature = "svg")]
    spec!("svg_callout", 5, 6, "svg"; builtin_svg_callout, "String"), // text, from_x, from_y, to_x, to_y [, intent]
    // Level 3: high-level chart types
    #[cfg(feature = "chart")]
    spec!("chart_bar", 2, "chart"; builtin_chart_bar, "String"), // data, style
    #[cfg(feature = "chart")]
    spec!("chart_donut", 2, "chart"; builtin_chart_donut, "String"), // data, style — Наряд №77 Block 2
    #[cfg(feature = "chart")]
    spec!("chart_line", 2, "chart"; builtin_chart_line, "String"), // data, style — Наряд №78 Block 1
    #[cfg(feature = "chart")]
    spec!("chart_scatter", 2, "chart"; builtin_chart_scatter, "String"), // data, style — Наряд №78 Block 2
    #[cfg(feature = "chart")]
    spec!("chart_area", 2, "chart"; builtin_chart_area, "String"), // data, style — Наряд №78 Block 3
    #[cfg(feature = "chart")]
    spec!("chart_radar", 2, "chart"; builtin_chart_radar, "String"), // data, style — Наряд №79 Block 1
    #[cfg(feature = "chart")]
    spec!("chart_heatmap", 2, "chart"; builtin_chart_heatmap, "String"), // data, style — Наряд №79 Block 2
    #[cfg(feature = "chart")]
    spec!("chart_boxplot", 2, "chart"; builtin_chart_boxplot, "String"), // data, style — Наряд №79 Block 3
    // Level 2.6: derived palette (Наряд №77 Block 1)
    #[cfg(feature = "svg")]
    spec!("color_palette", 2, "svg"; builtin_color_palette), // intent, mode → DiagramStyle
    // Level 2.6/2.7: procedural backgrounds + canvas presets (Наряд №80)
    #[cfg(feature = "svg")]
    spec!("svg_generate", 4, "svg"; builtin_svg_generate, "String"), // kind, intent, w, h → SVG fragment
    #[cfg(feature = "svg")]
    spec!("svg_canvas_preset", 3, "svg"; builtin_svg_canvas_preset), // preset_name, viewbox, children
    // Level 3.1: diagrams (Наряд №81) — hierarchies & flows
    #[cfg(feature = "diagram")]
    spec!("diagram_tree", 2, "diagram"; builtin_diagram_tree, "String"), // data, style — recursive tree
    #[cfg(feature = "diagram")]
    spec!("diagram_org_chart", 2, "diagram"; builtin_diagram_org_chart, "String"), // data, style — tree with title field
    #[cfg(feature = "diagram")]
    spec!("diagram_flowchart", 2, "diagram"; builtin_diagram_flowchart, "String"), // data, style — layered DAG
    #[cfg(feature = "diagram")]
    spec!("diagram_layers", 2, "diagram"; builtin_diagram_layers, "String"), // data, style — horizontal stripes
    // Level 3.2: diagrams (Наряд №82) — temporal & process
    #[cfg(feature = "diagram")]
    spec!("diagram_sequence", 2, "diagram"; builtin_diagram_sequence, "String"), // data, style — UML sequence (lifelines + messages; builtin_diagram_sequence)
    #[cfg(feature = "diagram")]
    spec!("diagram_timeline", 2, "diagram"; builtin_diagram_timeline, "String"), // data, style — horizontal axis with event dots
    #[cfg(feature = "diagram")]
    spec!("diagram_gantt", 2, "diagram"; builtin_diagram_gantt, "String"), // data, style — horizontal bars per task
    #[cfg(feature = "diagram")]
    spec!("diagram_process", 2, "diagram"; builtin_diagram_process, "String"), // data, style — linear numbered step chain
    #[cfg(feature = "diagram")]
    spec!("diagram_loop", 2, "diagram"; builtin_diagram_loop, "String"), // data, style — closed-loop circular steps
    // Level 3.3: diagrams (Наряд №83) — sets & comparisons
    #[cfg(feature = "diagram")]
    spec!("diagram_venn", 2, "diagram"; builtin_diagram_venn, "String"), // data, style — 2 or 3 overlapping circles
    #[cfg(feature = "diagram")]
    spec!("diagram_quadrant", 2, "diagram"; builtin_diagram_quadrant, "String"), // data, style — 2x2 strategic quadrant
    #[cfg(feature = "diagram")]
    spec!("diagram_pyramid", 2, "diagram"; builtin_diagram_pyramid, "String"), // data, style — stacked trapezoids (top=apex; builtin_diagram_pyramid)
    #[cfg(feature = "diagram")]
    spec!("diagram_nested", 2, "diagram"; builtin_diagram_nested, "String"), // data, style — concentric circles
    #[cfg(feature = "diagram")]
    spec!("diagram_medallion", 2, "diagram"; builtin_diagram_medallion, "String"), // data, style — row of round badges w/ icons
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
    spec!("diagram_er", 2, "diagram"; builtin_diagram_er, "String"), // data, style — entity boxes on a grid w/ relations
    #[cfg(feature = "diagram")]
    spec!("diagram_state", 2, "diagram"; builtin_diagram_state, "String"), // data, style — state machine (cycles OK; builtin_diagram_state)
    #[cfg(feature = "diagram")]
    spec!("diagram_swimlane", 2, "diagram"; builtin_diagram_swimlane, "String"), // data, style — lanes × steps positioned by order
    #[cfg(feature = "diagram")]
    spec!("diagram_data_flow", 2, "diagram"; builtin_diagram_data_flow, "String"), // data, style — graph w/ cycles OK
    #[cfg(feature = "diagram")]
    spec!("diagram_high_level", 2, "diagram"; builtin_diagram_high_level, "String"), // data, style — large bolder blocks, no cycles
    #[cfg(feature = "diagram")]
    spec!("diagram_architecture", 2, "diagram"; builtin_diagram_architecture, "String"), // data, style — high_level + svg_icon per node
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
    // ── Наряд №180: Reflex persistence (ADR-0116) ──
    // Same pattern as reflex_train/reflex_predict: stub handlers — the
    // real dispatch lives in interpreter::execution::invoke() (and
    // interpreter::reflex_builtin.rs for FnCall expressions) because
    // reflex_save/reflex_load need access to both the ReflexRegistry
    // and the SQLite persist path (set by `memory { persist: "..." }`).
    spec!("reflex_save", 1, "reflex"; builtin_reflex_save_stub),
    spec!("reflex_load", 1, "reflex"; builtin_reflex_load_stub),
    // ── Наряд №187: Reflex introspection (read-only, ADR-0114) ──
    // Same stub pattern. reflex_metrics returns metadata (no weights);
    // reflex_list returns names of all declared reflex/reflex_seq models.
    spec!("reflex_metrics", 1, "reflex"; builtin_reflex_metrics_stub),
    spec!("reflex_list", 0, "reflex"; builtin_reflex_list_stub),
    // ── Наряд №193: text generation (ADR-0120) ──
    spec!("reflex_generate", 4, "reflex"; builtin_reflex_generate_stub),
    // ── Наряд №194: tokenization (ADR-0120 follow-up) ──
    // These are pure functions (no registry access) — real handlers, not stubs.
    spec!("reflex_tokenize", 1, "reflex"; builtin_reflex_tokenize),
    spec!("reflex_detokenize", 1, "reflex"; builtin_reflex_detokenize, "String"),
    // ── Наряд №195: BPE tokenization ──
    spec!("reflex_bpe_train", 2, "reflex"; builtin_reflex_bpe_train),
    spec!("reflex_bpe_encode", 2, "reflex"; builtin_reflex_bpe_encode),
    spec!("reflex_bpe_decode", 2, "reflex"; builtin_reflex_bpe_decode),
    spec!("reflex_bpe_save", 1, "reflex"; builtin_reflex_bpe_save, "Unit"),
    spec!("reflex_bpe_load", 1, "reflex"; builtin_reflex_bpe_load),
    // ── Vision pillar (Наряд №210, ADR-0124) ──
    // Наряд №240 (R4.2): vision_generate arity 3→2 (R4 contract, plan §3:
    // `vision_generate("decl_name", "prompt")` — model/steps/size/seed come
    // from the `vision { }` declaration; the R1 stub doc "(model_name,
    // prompt, seed)" predates the declaration language and was never the
    // contract). generate/list/export are intercepted before the generic
    // fallback (лекало reflex_train); these specs remain the last-resort
    // handlers + the arity/type contract for LSP and checks.
    spec!("vision_generate", 2, "vision"; builtin_vision_generate_stub),
    spec!("vision_edit", 2, "vision"; builtin_vision_edit_stub),
    spec!("vision_export", 2, "vision"; builtin_vision_export_stub),
    // Наряд №241 (R5, Block 2.1 — ADR-0125): explicit raw opt-out. Real
    // path intercepted like vision_export (state-carrying). Every call
    // site is audit-flagged VISION_UNSIGNED_EXPORT_RAW (Warning).
    spec!("vision_export_raw", 2, "vision"; builtin_vision_export_raw_stub),
    // Наряд №241 (R5, Block 3.2 — ADR-0125 MODEL_WEIGHTS_UNSAFE): REAL
    // stateless handler (no interception needed — no registry state):
    // SSRF-guarded (check_url_ssrf, лекало №130), allowlist default-deny
    // (MLOG_VISION_WEIGHTS_ALLOWLIST), manifest.json-class only, SHA-256
    // pinned via reused WeightsManifest. Registry 388→389.
    spec!("vision_fetch_weights", 2, "vision"; builtin_vision_fetch_weights),
    spec!("vision_list", 0, "vision"; builtin_vision_list_stub),
    // Наряд №242 (R6.1): vision_save/vision_load — real SQLite persistence
    // (crate::vision::store), intercepted like the rest of the vision
    // family (state-carrying PLUS the program's db connection). These
    // specs remain the last-resort handlers + the arity contract. Registry
    // count UNCHANGED (389): both builtins existed here since №210 — this
    // naryad replaces their stub bodies with real dispatch paths.
    spec!("vision_save", 2, "vision"; builtin_vision_save_stub),
    spec!("vision_load", 1, "vision"; builtin_vision_load_stub),
    // Наряд №244 (R6.3): LoRA adapters — SQLite BLOB persistence
    // (ADR-0124 §6) + application to the DiT attention projections.
    // Intercepted like the rest of the vision family (state-carrying: the
    // program's db connection, plus decls + registry for generate). These
    // specs remain the last-resort handlers + the arity contract. Registry
    // 389→391: the vision family reaches its ADR-0124 §3 ceiling of
    // ~10 (8→10) — the NEXT vision builtin requires an ADR-0124 edit
    // (loudly noted in CHANGELOG).
    spec!("vision_lora_load", 2, "vision"; builtin_vision_lora_load_stub),
    spec!("vision_lora_generate", 3, "vision"; builtin_vision_lora_generate_stub),
    // ── Наряд №331 (ADR-0162): unified media layer (category "media") ──
    // Four per-type store builtins (opaque handles — bytes never enter
    // Value), the sanctioned materialization SINK (media_save), the
    // refcount pair, and the metadata observer. All state-carrying:
    // interpreter and VM intercept these names BEFORE the generic
    // fallback and route through src/builtins/media.rs dispatches; these
    // specs remain the last-resort handlers + the arity/type contract.
    // Registry 421→429. media_save egress is gated by №325 (file kind)
    // + the runtime backstop (MEDIA_SEALED_EGRESS).
    spec!("media_store_image", 2, "media"; builtin_media_store_image_stub),
    spec!("media_store_audio", 2, "media"; builtin_media_store_audio_stub),
    spec!("media_store_video_frame", 2, "media"; builtin_media_store_video_frame_stub),
    spec!("media_store_video_segment", 2, "media"; builtin_media_store_video_segment_stub),
    spec!("media_save", 2, 3, "media"; builtin_media_save_stub),
    spec!("media_retain", 1, "media"; builtin_media_retain_stub),
    spec!("media_release", 1, "media"; builtin_media_release_stub),
    spec!("media_meta", 1, "media"; builtin_media_meta_stub),
    // ── Наряд №332 (ADR-0164): perception origin — HandleSource/ProvBind ──
    // Both are state-carrying (origin declarations + media store) and are
    // intercepted like the media family; the compiler lowers
    // `source <origin>` / `from <origin> <construction>` to these calls.
    // Registry 430→432.
    spec!("media_source_capture", 1, "media"; builtin_media_source_capture_stub),
    spec!("media_bind_origin", 2, "media"; builtin_media_bind_origin_stub),
    // ── Наряд №333 (ADR-0163): backend registry — read-only metadata ──
    // The SSOT table lives in src/backends.rs; this builtin is the
    // language surface (name/class/weights_id/pin/license/license_note).
    // Registry 429→430; category "registry" (41st module).
    spec!("backend_list", 0, "registry"; builtin_backend_list),
    // ── Наряд №336 (ADR-0165): BackendSelect — the backend try-chain ──
    // Priority ladder over the №333 registry SSOT; exhaustion →
    // Degraded(t), the typed degradation result (never a panic, never a
    // silent mock). Registry 439→440; category "registry".
    spec!("backend_select", 2, "registry"; builtin_backend_select),
    // ── Наряд №337 (ADR-0166): the C2PA contour of media handles ──
    // media_manifest is STATE-CARRYING (store read) — intercepted by
    // interpreter/VM before the generic fallback; media_manifest_read is
    // stateless (sandboxed sidecar read). Registry 440→442.
    spec!("media_manifest", 1, "media"; builtin_media_manifest_stub),
    spec!("media_manifest_read", 1, "media"; builtin_media_manifest_read),
    // ── Наряд №284 (P1, M1): canary-токены недоверенного текста ──
    // Runtime-детектор утечки недоверенного контента через LLM-канал
    // (паттерн rebuff/Spotlighting). Связан с taint-моделью: утечка →
    // runtime warning CANARY_LEAK + llm_usage().canary_leaks (canary.rs);
    // статическая метка «компрометированный канал» в ветке утечки →
    // audit-warning CANARY_LEAK (src/audit.rs, check_canary_leak,
    // advisory-слой audit_program). Детектор, НЕ гейт. Чистые обработчики
    // (без интерсепшена) — execution.rs не требуется. Registry 399→401;
    // новая категория "security" (37→38 модулей).
    spec!("canary_insert", 1, 2, "security"; builtin_canary_insert),
    spec!("canary_check", 2, 3, "security"; builtin_canary_check),
    // ── Наряд №286 (P2, M1): json_validate — валидатор ADR-0133 как
    // standalone builtin («shape-before-use»). ОДИН И ТОТ ЖЕ валидатор, что
    // у call_llm_schema — извлечён в src/schema/validate.rs, ни одного
    // нового правила; дифференциальный корпус (tests/naryad_286_json_validate.rs)
    // сверяет вердикты и тексты нарушений обоих путей. Schema-side — громкий
    // LLM_SCHEMA_UNSUPPORTED_FEATURE (единый код с call_llm_schema);
    // value_json не-JSON — громкая ошибка парсинга (валидатор судит
    // структуру, парсер — байты). strict (дефолт true) = strict-by-default
    // ADR-0133 D2; strict=false разрешает необъявленные поля (opt-in №286).
    // Не feature-гейт: проверяет данные НЕ от LLM (MCP tool-outputs №268,
    // HTTP-ответы, request_body) — см. minimal-build. Категория "llm" —
    // семейство ADR-0133. Registry 401→402.
    spec!("json_validate", 2, 3, "llm"; builtin_json_validate),
    // ── Наряд №280 (P2, M2): memory_forget — управляемое забывание с
    // границами (supermemory forget-matching: dry_run-превью → apply
    // строго по явным ids → batch_id в ledger). Soft-delete: физического
    // удаления нет, стёртые id — в {table}__forgotten (id, batch_id,
    // reason, forgotten_at) — журнал операции (audit-след в духе №276).
    // vec_search получает опциональный include_forgotten (дефолт false —
    // забытые не возвращаются; арность 4→4..5 на месте, индексы не
    // сдвигаются). Tier 1 поверх vec_search (№272), тот же feature-gate
    // `vec`; песочница ForRead (превью) / ForWrite (apply). Порог и
    // max_forget капируют поражённую область; apply по id вне границ
    // превью — ГРОМКАЯ ошибка (bound deletes). Автозабывание (TTL,
    // вытеснение updates-фактом) — v2, вне скоупа. Категория "memory".
    // Registry 402→403 (append-only, bytecode-индексы стабильны).
    #[cfg(feature = "vec")]
    spec!("memory_forget", 5, 7, "memory"; builtin_memory_forget, "Struct"), // №539: the store-lane dry-run/apply result (MemoryForgetResult)
    // ── Наряд №281 (P2, M2): user_profile — детерминированная выжимка
    // контейнера одним вызовом (supermemory user-profiles): static /
    // dynamic / buckets из KV-записей container:<c>:<bucket>:<key>
    // (источник — memorize/kv_set с memory{persist}); БЕЗ LLM-вызова
    // (LLM-синтез — вне скоупа Tier-1, громко); ин-процессный кэш с
    // генерационной (kv-записи) + mtime (внешние записи) инвалидацией.
    // Контейнер-префикс — жёсткая изоляция (cross-container физически
    // не виден); scope-параметр — на vec_store/vec_search. Не
    // feature-гейт: kv-контур ядровой. Registry 403→404 (append-only).
    spec!("user_profile", 2, "memory"; builtin_user_profile, "Struct"), // №539: the UserProfile (both the fresh-build and the cache-hit path)
    // ── Наряд №285 (P2, feature/memory): text_chunk — структура-осознанное
    // чанкование для RAG-пайплайна (первая стадия поверх №272 vec-контур):
    // strategies markdown|paragraph|fixed; opts{max_chars, overlap,
    // max_tokens?}; markdown-секции несут header_path ("H1 > H2 > H3") —
    // готовые метаданные для vec_store. Каскад «заголовок → абзац →
    // перенос → пробел» + жадное слияние мелких + overlap при окнировании
    // (RecursiveCharacterTextSplitter-дух, без зависимостей). Token-бюджет —
    // реюз token_count (memory.rs token_count_estimate). Registry 404→405
    // (append-only).
    spec!("text_chunk", 2, 3, "string"; builtin_text_chunk, "List"), // text, strategy | +opts{max_chars, overlap, max_tokens}
    // ── Наряд №275 (P1, feature/llm): LLM streaming — итераторный
    // стиль над SmartRouter (ADR-0137). Opaque handle Value::LlmStream,
    // registry crate::llm::LLM_STREAM_REGISTRY (bounded №263). Trace —
    // одна строка на завершённый стрим (ADR-0138 §D4). Mock / non-SSE →
    // loud STREAM_UNSUPPORTED. Registry 405→408 (append-only).
    #[cfg(feature = "llm")]
    spec!("llm_stream_open", 1, 2, "llm"; builtin_llm_stream_open), // prompt | prompt,input
    #[cfg(feature = "llm")]
    spec!("llm_stream_next", 1, "llm"; builtin_llm_stream_next, "String"), // handle
    #[cfg(feature = "llm")]
    spec!("llm_stream_close", 1, "llm"; builtin_llm_stream_close), // handle
    // ── Наряд №283 (P2, feature): path-параметры роутов mlogserver —
    // шаблонный диспетчер {name} / {*path} (axum 0.8.9 syntax). Builtin
    // stub — real body in interpreter/vm FnCall dispatch (needs access
    // to server_path_params HashMap). Static routes win over templates;
    // conflict of two templates matching the same path → loud error at
    // server start. Registry 408→409 (append-only).
    spec!("server_path_param", 1, "web"; builtin_server_path_param, "String"), // name
    // ── Наряд №302 (P2, feature/voice): Voice pillar skeleton builtins —
    // stubs. Loud errors, no silent fallbacks. Real implementation in
    // phases A2/A3/A4/A5/A6. Feature-gated under `voice` (implies candle).
    // Registry 409→415 (append-only, bytecode indices stable).
    #[cfg(feature = "voice")]
    spec!("voice_enroll", 2, 3, "voice"; builtin_voice_enroll_stub), // decl, audio | +kind
    #[cfg(feature = "voice")]
    spec!("tts_speak", 2, "voice"; builtin_tts_speak_stub), // decl, text
    #[cfg(feature = "voice")]
    spec!("audio_export", 1, "voice"; builtin_audio_export_stub), // handle
    #[cfg(feature = "voice")]
    spec!("voice_design", 2, "voice"; builtin_voice_design_stub), // text, voice
    #[cfg(feature = "voice")]
    spec!("voice_save", 2, "voice"; builtin_voice_save_stub), // handle, name
    #[cfg(feature = "voice")]
    spec!("voice_load", 1, "voice"; builtin_voice_load_stub), // name
    // ── Наряд №307/№309 (P2, feature/video): Video pillar builtins.
    // №309 (ADR-0151): real implementations on the tiny seeded pipeline —
    // I2V first/last anchors, RIFE-class interp, extension, AV sidecar mux,
    // signed-by-construction export. video_render: T2V (2) / I2V (3) /
    // two-anchor first–last (4). MODEL_WEIGHTS_UNSAFE covers
    // video_fetch_weights via suffix convention (Наряд №300).
    // Registry 420→421 (append-only).
    #[cfg(feature = "video")]
    spec!("video_render", 2, 4, "video"; builtin_video_render),
    #[cfg(feature = "video")]
    spec!("video_export", 2, "video"; builtin_video_export),
    #[cfg(feature = "video")]
    spec!("av_mux", 2, "video"; builtin_av_mux),
    #[cfg(feature = "video")]
    spec!("frame_interp", 2, "video"; builtin_frame_interp),
    #[cfg(feature = "video")]
    spec!("video_extend", 2, "video"; builtin_video_extend),
    #[cfg(feature = "video")]
    spec!("video_fetch_weights", 2, "video"; builtin_video_fetch_weights_stub),
    // №408 (wave 4.5): the comprehension side of the video pillar —
    // mock-first backend call (the understanding twin of №309's
    // generation pipeline), behind the same feature gate.
    #[cfg(feature = "video")]
    spec!("video_understand", 1, 3, "video"; builtin_video_understand),
    // ── Наряд №334 (P0, feature/backends): real STT/omni/vision-
    // understanding backends — the SHA-pin path. Mock-first call surface
    // over the №333 registry: METALOGOS_MOCK_LLM opt-in = deterministic
    // mock (the golden contract); real mode refuses LOUDLY unless the
    // SHA-verified weights are on disk (PARKED №294 — no inference is
    // promised). Handlers: voice::backend (stt/omni), vision::understand.
    // Registry 432→435 (append-only, bytecode indices stable).
    spec!("stt_transcribe", 1, 2, "voice"; builtin_stt_transcribe),
    spec!("omni_ask", 1, 3, "voice"; builtin_omni_ask),
    spec!("vision_understand", 1, 3, "vision"; builtin_vision_understand),
    // ── Наряд №335 (spec §7.2 v2): consent grant/revoke + quarantine ──
    // The consent component's surface (redact precedent: policy as
    // value, builtins not AST). grant/revoke record the ledger and pass
    // the value; the STATIC label rules live in semantic.rs label_source
    // (grant extends the consent scope, revoke = quarantine label — the
    // flat cascade via lattice absorption). quarantine_write is the ONLY
    // legal egress for poisoned values (audit event). Ledger export is
    // file egress, audited. Registry 435→439 (append-only).
    spec!("consent_grant", 2, 4, "security"; builtin_consent_grant),
    spec!("consent_revoke", 1, 2, "security"; builtin_consent_revoke),
    spec!("quarantine_write", 1, 2, "security"; builtin_quarantine_write, "String"),
    spec!("consent_ledger_export", 1, "security"; builtin_consent_ledger_export, "String"),
    // ── Naryad #390 (ADR-0155): Grant algebra — capabilities for ──
    // irreversible actions (wave 3, dispatch #491). The ungranted
    // destructive-SQL deny (IRREVERSIBLE_NO_GRANT, №325) is UNCHANGED —
    // db_execute_with_grant is the additional allowing path, gated by
    // the grant ledger (state/TTL/scope/quota) at runtime and by the
    // Once-linearity check (GRANT_REUSED) at compile time. Registry
    // 442→447 (append-only; bytecode indices must not shift).
    spec!("grant_issue", 2, 4, "action"; builtin_grant_issue),
    spec!("grant_subgrant", 3, 5, "action"; builtin_grant_subgrant),
    spec!("grant_revoke", 1, "action"; builtin_grant_revoke, "Float"),
    spec!("grant_use", 1, "action"; builtin_grant_use, "Float"),
    spec!("db_execute_with_grant", 2, 3, "action"; builtin_db_execute_with_grant),
    // ── Naryad #392: the DenyEvent surface ──────────────────────────────
    // Handler-scoped, intercepted by NAME in BOTH backends (the
    // interpreter's invoke() and the VM's call_builtin). Registry stubs
    // exist so the compiler resolves the calls; real dispatch is
    // backend-side (the event is runtime state). The analyzer blocks
    // usage outside an on_deny handler at compile time; the runtime
    // refuses when no event is live. APPENDED at the end — inserting
    // mid-array would shift existing CallBuiltin indices (.mbc contract).
    spec!("deny_event", 0, "stub"),
    spec!("deny_reason", 0, "stub"),
    // ── Naryad #393 (ADR-0167): Action Ledger v1 surface ────────────────
    // Introspection reads (count/head — not egress) and the two FILE
    // EGRESS exports (classified Sink, the consent_ledger_export
    // precedent) plus the rotation/snapshot writers. The journal writes
    // for grant/deny/irreversible/session events live in the ACTION
    // paths themselves (ADR-0167 §3.4) — this surface never becomes a
    // forgotten "also log it" API. APPENDED at the end — inserting
    // mid-array would shift existing CallBuiltin indices (.mbc contract).
    // Registry 451→457 (append-only).
    spec!("ledger_count", 0, "security"; builtin_ledger_count, "Float"),
    spec!("ledger_head", 0, "security"; builtin_ledger_head, "String"),
    spec!("ledger_export", 1, "security"; builtin_ledger_export),
    spec!("ledger_export_intoto", 1, "security"; builtin_ledger_export_intoto),
    spec!("ledger_rotate", 0, "security"; builtin_ledger_rotate, "String"),
    spec!("ledger_snapshot", 0, "security"; builtin_ledger_snapshot, "String"),
    // ── Naryad #387 (ADR-0149 D1/D6): the likeness consent ritual ──
    // The one-time challenge + the opaque LikenessToken. APPENDED at
    // the end — inserting mid-array would shift existing CallBuiltin
    // indices (.mbc contract). Registry 457→459 (append-only).
    spec!("likeness_challenge", 1, 3, "security"; builtin_likeness_challenge),
    spec!("likeness_verify", 1, 3, "security"; builtin_likeness_verify),
    // ── Naryad #407 (wave 4.5): the OCR class — text extraction from an
    // image. Donor contract = №334's vision_understand (mock-first,
    // deterministic golden path; real mode refuses loudly naming the
    // weights artifact — PARKED №294). Handler: vision::ocr. APPENDED
    // at the end — inserting mid-array would shift existing CallBuiltin
    // indices (.mbc contract). Registry 458→459 (append-only).
    spec!("ocr_extract", 1, 3, "vision"; builtin_ocr_extract),
    // ── Naryad #415 (P1, security/ledger): the runtime verify hook —
    // read-only structural verification of an exported chain (the audit
    // P2-2 residue). APPENDED at the end — inserting mid-array would
    // shift existing CallBuiltin indices (.mbc contract).
    // Registry 459→460 (append-only).
    spec!("ledger_verify", 1, "security"; builtin_ledger_verify),
    // ── Naryad #440 (P1, feature/forecast): the forecasting domain ──
    // series_make/series_pull/forecast_next/forecast_state/
    // forecast_points over the `timeseries` registry class; the taint
    // transfer + the gated export live in src/forecast.rs. APPENDED at
    // the end — inserting mid-array would shift existing CallBuiltin
    // indices (.mbc contract). Registry 460→465 (append-only).
    spec!("series_make", 1, 2, "forecast"; builtin_series_make),
    spec!("series_pull", 2, "forecast"; builtin_series_pull),
    spec!("forecast_next", 2, "forecast"; builtin_forecast_next),
    spec!("forecast_state", 1, "forecast"; builtin_forecast_state),
    spec!("forecast_points", 1, "forecast"; builtin_forecast_points),
    // ── Naryad #445 (P1, feature/memory): the canon retain(memory, ttl)
    // — the typed entry's lifetime; past the deadline the sweep
    // auto-forgets it (the №280 "v2" deferral lifted). APPENDED at the
    // end — inserting mid-array would shift existing CallBuiltin
    // indices (.mbc contract). Registry 499→500 (append-only).
    spec!("memory_retain_ttl", 3, "memory"; builtin_memory_retain_ttl, "Unit"),
    // ── №757 (P1, llm/hardening): the mlog-visible truncation probe —
    // the finish_reason/stop_reason of the last completed call/stream
    // ("" = none reported yet). The non-silent half of №757: the caller
    // checks `llm_last_finish_reason() == "length"` after call_llm.
    // Typed "String" (№467 vocabulary) — the typed-share floor rises
    // with this row. APPENDED at the end — inserting mid-array would
    // shift existing CallBuiltin indices (.mbc contract).
    // Registry 500→501 (append-only).
    #[cfg(feature = "llm")]
    spec!("llm_last_finish_reason", 0, "llm"; builtin_llm_last_finish_reason, "String"),
    // ── №507 (P2, hardening): the EXPLICIT-silence twin of read_file —
    // the `_or` suffix symmetry with env_or (№481): the MISSING file
    // yields the caller's default (announced on the audit stderr),
    // every loud branch (sandbox, deny-list, open/read) stays shared
    // with read_file. The read_file №254 contract is unchanged.
    // APPENDED at the end — inserting mid-array would shift existing
    // CallBuiltin indices (.mbc contract). Registry 501→502 (append-only).
    spec!("read_file_or", 2, "io"; builtin_read_file_or),
    // ── №514 (P1, hardening; the audit 28.09 C-10): the EXPLICIT-silence
    // twins of to_float/to_int — the `_or` suffix carries the silent-default
    // semantics IN THE NAME (the №481 env/env_or naming rule, ONE soft-
    // failure rule for the language). The fallback firing is announced on
    // the audit stderr ([TO_FLOAT_OR]/[TO_INT_OR]); the loud branches of
    // to_float/to_int stay shared. Bool→1.0/0.0 is a conversion, not a
    // soft failure — kept in both twins. APPENDED at the end — inserting
    // mid-array would shift existing CallBuiltin indices (.mbc contract).
    // Registry 503→505 (append-only).
    spec!("to_float_or", 2, "convert"; builtin_to_float_or, "Float"),
    spec!("to_int_or", 2, "string"; builtin_to_int_or, "Float"),
    // ── Наряд №526 (issue #835; audit 30.09 N-2): the GDPR Art. 17
    // erasure path for voiceprints — voice_delete(handle) is IDEMPOTENT
    // ("deleted"/"absent", a repeated erase succeeds), voice_list() is
    // the informed-deletion basis (id + model; the embedding never enters
    // the result). Not feature-gated (the erasure right cannot depend on
    // a build flag); the RAM registry is the live surface (a shipped
    // runtime persists zero voiceprints — privacy.md §2.1), the persisted
    // path is VoiceStore::{delete_voiceprint, list_voiceprints} (the same
    // secure zero-then-delete path, tested at the store level). The
    // consent ledger survives by design (the Art. 9 proof). APPENDED at
    // the end — inserting mid-array would shift existing CallBuiltin
    // indices (.mbc contract). Registry 505→507 (append-only).
    spec!("voice_delete", 1, "voice"; builtin_voice_delete), // voice|audio handle
    spec!("voice_list", 0, "voice"; builtin_voice_list),
    // ── №591 (Волна 30, Камертон Н1-05/Н1-07, инвариант 3): the spectral
    // contour — the Lomb–Scargle periodogram for unevenly sampled series
    // (gapped observations are the norm; FFT needs a uniform grid) and the
    // dominant-frequency readout with the false-alarm p-value. Typed
    // returns, pure functions, loud degradation ([SPECTRAL_INPUT] stamp +
    // the typed Degraded struct). APPENDED at the end — inserting
    // mid-array would shift existing CallBuiltin indices (.mbc contract).
    // Registry 509→511 (append-only).
    spec!("lomb_scargle", 2, "math"; builtin_lomb_scargle, "Struct"),
    spec!("spectral_peak", 1, "math"; builtin_spectral_peak, "Struct"),
    // ── №595 (Волна 30, Камертон Н1-05/Н1-07): the UTC calendar arithmetic —
    // the ISO parse/format pair, the signed fractional day difference and
    // the UTC-facing epoch read. ALL four typed PRECISE (Float/Float/
    // Float/String — the verified single-shape handler facts). now_unix
    // carries the explicit №316 Source classification (the wall-clock
    // read); the rest are the provably-pure `time` default. APPENDED at
    // the end — inserting mid-array would shift existing CallBuiltin
    // indices (.mbc contract). Registry 511→515 (append-only).
    spec!("now_unix", 0, "time"; builtin_now_unix, "Float"),
    spec!("date_parse_iso", 1, "time"; builtin_date_parse_iso, "Float"),
    spec!("date_diff_days", 2, "time"; builtin_date_diff_days, "Float"),
    spec!("date_format_iso", 1, "time"; builtin_date_format_iso, "String"),
    // ── №590 (Волна 30, Камертон Н1-05): the Box–Muller normal sampler over
    // the shared PRNG — typed return, pure function, loud domain gate
    // ([NORMAL_SAMPLE_STDDEV]). APPENDED at the end — inserting mid-array
    // would shift existing CallBuiltin indices (.mbc contract).
    // Registry 515→516 (append-only).
    spec!("normal_sample", 2, "math"; builtin_normal_sample, "Float"),
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
