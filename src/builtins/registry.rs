use super::reflex::{
    builtin_reflex_bpe_decode, builtin_reflex_bpe_encode, builtin_reflex_bpe_load,
    builtin_reflex_bpe_save, builtin_reflex_bpe_train, builtin_reflex_detokenize,
    builtin_reflex_list_stub, builtin_reflex_load_stub, builtin_reflex_metrics_stub,
    builtin_reflex_predict_stub, builtin_reflex_save_stub, builtin_reflex_tokenize,
    builtin_reflex_train_stub,
};
// №799 (T2, gh#1488): the parked row gates with its feature — the stub
// import rides the same gate (the default build neither registers nor
// compiles the parked surface; `builtins::parked` names it).
#[cfg(feature = "candle")]
use super::reflex::builtin_reflex_generate_stub;
// Наряд №210: Vision pillar stub handlers (ADR-0124).
// №799 (T2, gh#1488): the parked stubs gate with the `vision` feature —
// the stable vision rows (vision_list/save/load) keep their stub imports
// ungated (the rows compile in the default build).
#[cfg(feature = "vision")]
use super::vision::{
    builtin_vision_edit_stub, builtin_vision_export_raw_stub, builtin_vision_export_stub,
    builtin_vision_fetch_weights, builtin_vision_generate_stub, builtin_vision_lora_generate_stub,
    builtin_vision_lora_load_stub,
};
use super::vision::{builtin_vision_list_stub, builtin_vision_load_stub, builtin_vision_save_stub};
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
// + the granted destructive-SQL action surface. №844 (ADR-0203): the
// capability_state LIVE typed view joins the surface.
use super::grants::{
    builtin_capability_state, builtin_db_execute_with_grant, builtin_grant_issue,
    builtin_grant_revoke, builtin_grant_subgrant, builtin_grant_use,
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
// never feature-gated. The fns stay compiled in EVERY build (voice/mod.rs
// owns the erasure contract); №799 gates only the USER-PROGRAM rows —
// so this registry-side import rides the same gate as the rows.
#[cfg(feature = "voice")]
use crate::voice::{builtin_voice_delete, builtin_voice_list};

/// Master registry of ALL builtin functions.
/// Order determines bytecode indices — DO NOT reorder existing entries.
/// To add a new builtin: append a `spec!` row here,
/// add the handler in Builtins::new(), and you're done.
pub const BUILTIN_REGISTRY: &[BuiltinSpec] = &[
    // ── String builtins ──
    spec!("upper", 1, "string"; builtin_upper, "String", "stable"),
    spec!("lower", 1, "string"; builtin_lower, "String", "stable"),
    spec!("len", 1, "string"; builtin_len, "Float", "stable"),
    spec!("str", 1, "string"; builtin_str, "String", "stable"),
    spec!("contains", 2, "string"; builtin_contains, "Bool", "stable"),
    spec!("index_of", 2, "string"; builtin_index_of, "Float", "stable"),
    spec!("substring", 3, "string"; builtin_substring, "String", "stable"),
    spec!("char_at", 2, "string"; builtin_char_at, "String", "stable"),
    spec!("starts_with", 2, "string"; builtin_starts_with, "Bool", "stable"),
    spec!("ends_with", 2, "string"; builtin_ends_with, "Bool", "stable"),
    spec!("trim", 1, "string"; builtin_trim, "String", "stable"),
    spec!("replace", 3, "string"; builtin_replace, "String", "stable"),
    spec!("split", 2, "string"; builtin_split, "List<String>", "stable"), // №631: the String parts (string.rs: Value::String elements)
    spec!("join", 2, "string"; builtin_join, "String", "stable"),
    spec!("length", 1, "string"; builtin_length, "Float", "stable"),
    spec!("reverse", 1, "string"; builtin_reverse, "stable"), // №536: Unknown honest — polymorphic (String → String, List → List); the flat vocabulary has no union
    spec!("escape_html", 1, "string"; builtin_escape_html, "String", "stable"),
    spec!("escape_json", 1, "string"; builtin_escape_json, "String", "stable"),
    // Наряд №274 (ADR-0136): redact(text, mode) — PII/секреты как
    // taint-санитайзер («mask before sink»). Единственный легальный путь
    // снять Secret-taint; семантика снятия — в src/audit.rs + ADR-0136.
    spec!("redact", 2, "string"; builtin_redact, "String", "stable"),
    spec!("escape_js", 1, "string"; builtin_escape_js, "String", "stable"),
    spec!("fuzzy_match", 2, "string"; builtin_fuzzy_match, "Float", "stable"),
    spec!("strip", 2, "string"; builtin_strip, "String", "stable"),
    spec!("chomp", 1, "string"; builtin_chomp, "String", "stable"),
    spec!("repeat", 2, "string"; builtin_repeat, "String", "stable"),
    spec!("pad_left", 3, "string"; builtin_pad_left, "String", "stable"),
    spec!("pad_right", 3, "string"; builtin_pad_right, "String", "stable"),
    spec!("lines", 1, "string"; builtin_lines, "List<String>", "stable"), // №631: the String lines (string.rs: Value::String elements)
    spec!("words", 1, "string"; builtin_words, "List<String>", "stable"), // №631: the String words (string.rs: Value::String elements)
    spec!("token_count", 1, "string"; builtin_token_count, "Float", "stable"),
    spec!("type_of", 1, "string"; builtin_type_of, "String", "stable"),
    spec!("format", 0, "string"; builtin_format, "String", "stable"), // variadic: 1 template + N fill args
    // НАРЯД №117: missing string utilities
    spec!("trim_start", 1, "string"; builtin_trim_start, "String", "stable"),
    spec!("trim_end", 1, "string"; builtin_trim_end, "String", "stable"),
    spec!("truncate", 2, "string"; builtin_truncate, "String", "stable"),
    spec!("slugify", 1, "string"; builtin_slugify, "String", "stable"),
    spec!("word_wrap", 2, "string"; builtin_word_wrap, "String", "stable"),
    spec!("capitalize", 1, "string"; builtin_capitalize, "String", "stable"),
    spec!("title_case", 1, "string"; builtin_title_case, "String", "stable"),
    // ── Stdlib backing (double-underscore prefix) ──
    spec!("__trim", 1, "std"; builtin_trim, "String", "stable"),
    spec!("__replace", 3, "std"; builtin_replace, "String", "stable"),
    spec!("__split", 2, "std"; builtin_split, "stable"),
    spec!("__join", 2, "std"; builtin_join, "String", "stable"),
    spec!("__abs", 1, "std"; builtin_abs, "Float", "stable"),
    spec!("__min", 2, "std"; builtin_min, "Float", "stable"),
    spec!("__max", 2, "std"; builtin_max, "Float", "stable"),
    spec!("__clamp", 3, "std"; builtin_clamp, "Float", "stable"),
    spec!("__round", 1, "std"; builtin_round, "Float", "stable"),
    spec!("__first", 1, "std"; builtin_first, "stable"),
    spec!("__last", 1, "std"; builtin_last, "stable"),
    // ── Math builtins (public aliases for __abs/__min/__max/__clamp/__round) ──
    spec!("abs", 1, "math"; builtin_abs, "Float", "stable"),
    spec!("min", 2, "math"; builtin_min, "Float", "stable"),
    spec!("max", 2, "math"; builtin_max, "Float", "stable"),
    spec!("clamp", 3, "math"; builtin_clamp, "Float", "stable"),
    spec!("round", 1, "math"; builtin_round, "Float", "stable"),
    // Наряд №177: Math foundation for Reflex (stage 1/6)
    spec!("exp", 1, "math"; builtin_exp, "Float", "stable"),
    spec!("ln", 1, "math"; builtin_ln, "Float", "stable"),
    spec!("sqrt", 1, "math"; builtin_sqrt, "Float", "stable"),
    spec!("pow", 2, "math"; builtin_pow, "Float", "stable"),
    spec!("tanh", 1, "math"; builtin_tanh, "Float", "stable"),
    spec!("sigmoid", 1, "math"; builtin_sigmoid, "Float", "stable"),
    spec!("softmax", 1, "math"; builtin_softmax, "stable"),
    spec!("random_seed", 1, "math"; builtin_random_seed, "Unit", "stable"),
    spec!("random", 0, "math"; builtin_random, "Float", "stable"), // ── Phase 4.4 self-hosting — historical placeholders, never implemented ──
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
    spec!("newline", 0, "stub", "experimental"),
    spec!("stdin", 0, "stub", "experimental"), // №712 re-refusal: the true stub — no handler on any lane (registry.rs:180); the closure would either invent the no-stdin fallback contract or type a non-existent shape — both are semantics decisions outside the types movement
    spec!("split_tokens", 0, "stub", "experimental"),
    spec!("if_eq", 3, "stub", "experimental"),
    spec!("is_string_token", 1, "stub", "experimental"),
    // №820 (gh#1588, wave 67 — the #1528 stub-audit follow-up): the row
    // stops lying. The #1528 anchor ("no handler on any lane") was
    // factually STALE — the surface landed with №409 (the lazy-open VM
    // lane) and carries the №783 params SSOT: `db_insert(table, struct)`
    // is a real parameterized INSERT over src/db_ops.rs returning
    // last_insert_rowid, special-cased on BOTH lanes (TW
    // execution.rs:2541-2546, VM vm.rs:2821-2827) BEFORE the registry
    // dispatch — the №736 query_scalar stale-anchor correction class.
    // The registered fallback (builtin_db_insert_fallback) is the
    // context-less lane contract (the db_execute form): the loud
    // "no database connection" refusal where no db {} was declared —
    // never a silent mock. The shape is FIXED (String, Struct) — arity 2
    // exact; the tier stays experimental (the №809 only-up floor, not
    // this naryad's call). The two-lane pin: tests/naryad_820_db_insert_surface.rs.
    spec!("db_insert", 2, "db"; builtin_db_insert_fallback, "Float", "experimental"), // ── Convert builtins ──
    spec!("float", 1, "convert"; builtin_float, "Float", "stable"),
    spec!("to_string", 1, "convert"; builtin_to_string, "String", "stable"),
    spec!("to_float", 1, "convert"; builtin_to_float, "Float", "stable"), // ── IO builtins ──
    spec!("print", 1, "io"; builtin_print, "String", "stable"), // №613: returns the echoed string (the handler fact), not Unit
    spec!("read_file", 1, "io"; builtin_read_file, "String", "stable"),
    spec!("write_file", 2, "io"; builtin_write_file, "String", "stable"),
    spec!("append_file", 2, "io"; builtin_append_file, "String", "stable"),
    spec!("delete_file", 1, "io"; builtin_delete_file, "String", "stable"),
    spec!("file_exists", 1, "io"; builtin_file_exists, "Bool", "stable"),
    spec!("list_dir", 1, "io"; builtin_list_dir, "List<String>", "stable"),
    spec!("exec", 1, "io"; builtin_exec, "String", "stable"),
    spec!("exec_argv", 1, 2, "io"; builtin_exec_argv, "String", "stable"), // binary required, args list optional
    spec!("git_push", 1, "io"; builtin_git_push, "String", "stable"),
    // Наряд №268 (ADR-0132): MCP stdio-клиент — stateless, exec-гейт +
    // METALOGOS_MCP_ALLOWLIST + taint UserInput на выводе mcp_call.
    spec!("mcp_call", 4, "io"; builtin_mcp_call, "String", "stable"),
    spec!("mcp_list_tools", 2, "io"; builtin_mcp_list_tools, "List<Tool>", "stable"),
    // ── List builtins ──
    spec!("get", 2, "list"; builtin_get, "stable"), // №537: Unknown honest — returns the ELEMENT; a heterogeneous list has no fixed element type
    spec!("push", 2, "list"; builtin_push, "List", "stable"),
    spec!("slice", 3, "list"; builtin_slice, "List", "stable"),
    spec!("zip", 2, "list"; builtin_zip, "List", "stable"),
    spec!("sort_by", 2, 3, "list"; builtin_sort_by, "List", "stable"),
    spec!("filter", 3, "list"; builtin_filter, "List", "stable"),
    spec!("reduce", 3, "list"; builtin_reduce, "Float", "stable"), // №537: Unknown honest — returns the ACCUMULATOR; its type is the caller's choice
    spec!("dedup", 1, "list"; builtin_dedup, "List", "stable"),
    spec!("condense", 1, "list"; builtin_condense, "List", "stable"),
    // НАРЯД №118: collection utilities (unique, chunk, sort)
    spec!("unique", 1, "list"; builtin_unique, "List", "stable"),
    spec!("chunk", 2, "list"; builtin_chunk, "List", "stable"),
    spec!("sort", 1, "list"; builtin_sort, "List", "stable"),
    spec!("first", 1, "list"; builtin_first, "stable"), // №537: Unknown honest — the element + the soft-failure "" path (ADR-0180 posture)
    spec!("last", 1, "list"; builtin_last, "stable"), // №537: Unknown honest — the element + the soft-failure "" path (ADR-0180 posture)
    spec!("make_list", 0, "list"; builtin_make_list, "List", "stable"),
    spec!("matches_any", 2, "list"; builtin_matches_any, "Float", "stable"), // 1.0/0.0 — the numeric verdict form // ── JSON builtins ──
    spec!("parse_json", 1, 2, "json"; builtin_parse_json, "stable"),
    spec!("json_encode", 1, "json"; builtin_json_encode, "String", "stable"),
    spec!("json_get", 2, 3, "json"; builtin_json_get, "stable"),
    spec!("has_field", 2, "json"; builtin_has_field, "Float", "stable"),
    spec!("dict_get", 3, "json"; builtin_json_get, "stable"),
    spec!("dict_set", 3, "json"; builtin_dict_set, "stable"),
    spec!("dict_has", 2, "json"; builtin_dict_has, "Bool", "stable"),
    spec!("dict_keys", 1, "json"; builtin_dict_keys, "stable"),
    spec!("dict_values", 1, "json"; builtin_dict_values, "stable"), // ── Web builtins ──
    spec!("respond", 1, 2, "web"; builtin_respond, "stable"),
    // №523: the spec followed the then-implementation (status, html) — and
    // broke the 1-arg office corpus form (issue #892: 500 on every HTML route).
    // #892 restores the full contract: 1 arg (office form) / 2 args (documented
    // (status, html) OR office (title, body) — the builtin disambiguates).
    spec!("respond_html", 1, 2, "web"; builtin_respond_html, "stable"), // №565: the 2-arg form is DEPRECATED (the sense-by-content hazard) — migrate to respond_html_status/respond_html_doc; the 1-arg form is the gh#899 SSOT
    spec!("respond_html_status", 2, 2, "web"; builtin_respond_html_status, "Struct", "stable"), // №565: the explicit (status, body) form — the HttpResponse struct shape; the HTML egress gates treat it like respond_html
    spec!("respond_html_doc", 2, 2, "web"; builtin_respond_html_doc, "Struct", "stable"), // №565: the explicit (title, body) form — the title is never a status
    spec!("set_header", 3, "web"; builtin_set_header, "Struct<HttpResponse>{status:internal,body:internal,content_type:internal,headers:internal}", "stable"), // N7 (gh#1217): the application response headers — the allowlist (Cache-Control/ETag/Content-Type/Location/Content-Disposition) + the CR/LF refusal + the static taint lane; `set` semantics replace the same name; the field-meta: every field is program-computed at construction (internal) — the egress clearance is the respond/audit lane's duty
    spec!("form_data", 1, "web"; builtin_form_data, "Struct<FormData>", "stable"),
    spec!("json_body", 0, "web"; builtin_json_body, "Struct<JsonBody>", "stable"),
    spec!("query_param", 1, "web"; builtin_query_param, "String", "stable"), // №565 top-up (the №543 line): both paths verified — the handler stub and the db_ops::query_param dispatch return Value::String
    spec!("request_header", 1, "web"; builtin_request_header, "String", "stable"), // N2 (gh#1211): the per-request header lookup — the handler stub; the real dispatch lives in the TW FnCall intercepts and the VM (the injected per-request HeaderMap)
    spec!("html_select", 2, "web"; builtin_html_select, "List<HtmlMatch>", "stable"), // N5 (gh#1215, ADR-0192): the DOM extraction — every element matching the comma-separated selector group
    spec!("html_text", 1, 2, "web"; builtin_html_text, "String", "stable"), // N5 (gh#1215, ADR-0192): the normalized text — script/style never contribute; the optional selector narrows the scope
    spec!("html_links", 2, "web"; builtin_html_links, "List<HtmlLink>", "stable"), // N5 (gh#1215, ADR-0192): the absolute http/https links — javascript: and every other scheme is DROPPED by contract
    spec!("feed_parse", 1, "web"; builtin_feed_parse, "List<FeedItem>", "stable"), // N6 (gh#1216, ADR-0192): RSS 2.0 / Atom 1.0 — the missing field is the EMPTY string (fail-closed items), DTD/entities never expanded
    spec!("sitemap_parse", 1, "web"; builtin_sitemap_parse, "List<SitemapEntry>", "stable"), // N6 (gh#1216, ADR-0192): the urlset/sitemapindex shapes uniformly
    spec!("xml_parse", 1, "web"; builtin_xml_parse, "Struct<XmlNode>{tag:untrusted,attrs:untrusted,text:untrusted,children:untrusted}", "stable"), // N6 (gh#1216, ADR-0192): the general XML tree — every field derives from the untrusted document; depth ≤ 256, nodes ≤ 10 000, both loud
    // №523: the 2..3 range is NOMINAL — render's real contract is dynamic
    // (the template's parameter list is data, №115; the 1-arg form is the
    // №448 taint-lift surface). The builtin validates loudly at runtime;
    // the semantic arity check exempts the name (see check_expr_calls).
    spec!("render", 2, 3, "web"; builtin_render, "stable"),
    spec!("http_get", 1, 4, "web"; builtin_http_get, "String", "stable"), // url | url,headers | url,headers,timeout | ...,{max_retries:N,base_delay:N}
    spec!("http_post", 2, 6, "web"; builtin_http_post, "String", "stable"), // up to +retry_config Struct
    spec!("http_post_multipart", 2, 4, "web"; builtin_http_post_multipart, "String", "stable"),
    spec!("http_download", 2, 3, "web"; builtin_http_download, "Bool", "stable"), // Наряд №76: url,dest_path | url,dest_path,headers
    spec!("require", 1, 2, "web"; builtin_require, "Unit", "stable"),
    spec!("request_body", 0, "web"; builtin_json_body, "stable"), // №712 re-refusal: the dynamic-shape Struct<JsonBody> (http.rs:469-476 — the registered fallback fields={}, the per-request keys parsed dynamically in the TW/VM dispatch); the №661 field-meta armor refuses the static labeling — the dynamic-fields meta form is the dictionary lane
    spec!("web_search", 1, 2, "web"; builtin_web_search, "String", "stable"), // query | query,num
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
    spec!("geo_ip", 0, 1, "web"; builtin_geo_ip, "Struct<GeoLocation>{ip:untrusted,city:untrusted,region:untrusted,country:untrusted,country_code:untrusted,lat:untrusted,lon:untrusted,isp:untrusted,timezone:untrusted}", "stable"), // ip? (omit = caller IP; builtin_geo_ip)
    spec!("weather", 2, "web"; builtin_weather, "Struct<Weather>{temp:untrusted,feels_like:untrusted,temp_min:untrusted,temp_max:untrusted,humidity:untrusted,description:internal,wind_speed:untrusted,wind_direction:untrusted,pressure:untrusted,cloud_cover:untrusted,is_day:untrusted,city:untrusted,country:internal}", "stable"),
    spec!("geo_distance", 2, 5, "web"; builtin_geo_distance, "Float", "stable"),
    spec!("weather_forecast", 1, 3, "web"; builtin_weather_forecast, "List<DayForecast>", "stable"), // city | lat,lon | lat,lon,days
    // ── Crypto builtins ──
    spec!("hash_password", 1, "crypto"; builtin_hash_password, "stable"),
    spec!("verify_password", 2, "crypto"; builtin_verify_password, "Bool", "stable"),
    spec!("ct_eq", 2, "crypto"; builtin_ct_eq, "Bool", "stable"), // N2 (gh#1211): constant-time string equality — the secret-comparison builtin (no `==` timing leak)
    spec!("encrypt", 2, "crypto"; builtin_encrypt, "stable"),
    spec!("decrypt", 2, "crypto"; builtin_decrypt, "String", "stable"), // №757 precise movement (N6, gh#1216): the compensation package — the verified single-shape handler fact (the ONLY Ok arm is Value::String(s) — the UTF-8 plaintext)
    spec!("generate_key", 0, "crypto"; builtin_generate_key, "stable"), // №712 re-refusal: the REAL 256-bit random key returns Value::Secret (src/builtins/crypto.rs:206-216; the №730 re-verified anchor) — the opaque-Secret ground (the secret() form; no Secret spelling in the vocabulary)
    spec!("base64_encode", 1, "encoding"; builtin_base64_encode, "String", "stable"),
    spec!("base64_decode", 1, "encoding"; builtin_base64_decode, "String", "stable"),
    // ── Auth stubs (interpreter-mode mocks; real auth requires server mode; builtin_base64_decode) ──
    // authenticate: №784 (audit-23.10 A5) — the handler refuses LOUDLY
    // with [NOT_IMPLEMENTED] naming verify_password (the Ok(Unit) mock
    // answered while REFERENCE promised credential checking — the
    // promise had no code behind it). The "stub" class names the
    // not-implemented seam; the declared Unit is the vacuous success
    // type (every call errs — the error channel is the contract).
    spec!("authenticate", 2, "stub"; builtin_authenticate, "Unit", "experimental"),
    // ── Session (№348, ADR-0172): real session surface over the process-global
    // registry (src/session.rs) — the pre-№348 mock handlers lived above in
    // crypto.rs. Every transition is an Action-Ledger record (ADR-0167 §3.4).
    spec!("session_login", 2, "session"; builtin_session_login, "stable"),
    spec!("session_logout", 1, "session"; builtin_session_logout, "Unit", "stable"),
    spec!("session_duty_enter", 1, "session"; builtin_session_duty_enter, "Bool", "stable"),
    spec!("session_duty_exit", 1, "session"; builtin_session_duty_exit, "Bool", "stable"),
    spec!("session_wake", 2, 3, "session"; builtin_session_wake, "String", "stable"), // №613: returns the delivered source (String) so programs can branch
    spec!("session_poll_wake", 1, "session"; builtin_session_poll_wake, "stable"), // №712: Unknown honest — Struct "Wake"{source,payload} when a wake is pending | Unit when none (session.rs:148-160 — the №748 re-verified anchor: the №730 range 276-291 was factually STALE, the handler is at session.rs:138) — the state-dependent union (the №543 send_message form: the flat vocabulary has no state-dependent union — the (β) lane, the owner gate 7, NOT taken)
    spec!("session_interrupt", 2, 3, "session"; builtin_session_interrupt, "String", "stable"), // №613: returns the accepted priority (String)
    spec!("session_take_interrupt", 1, "session"; builtin_session_take_interrupt, "stable"), // №712: Unknown honest — Struct "Interrupt"{priority,reason} when an interrupt is queued | Unit when none (session.rs:198-207 — the №748 re-verified anchor: the №730 range 326-345 was factually STALE, the handler is at session.rs:188) — the same state-dependent union (the №543 form — the (β) lane, the owner gate 7, NOT taken)
    // №539: String — the literal "ok" return (verified handler; not Unit).
    spec!("session_clear", 1, "memory"; builtin_session_clear, "String", "stable"), // ── Bot — Telegram messaging ──
    spec!("send_message", 2, 4, "bot"; builtin_send_message, "stable"), // №543: Unknown honest — String (delivered) | Unit (no TELEGRAM_BOT_TOKEN fallback); the flat vocabulary has no env-dependent union. N3: the 4th argument is opts (parse_mode/bot/disable_preview/chunk/retry)
    spec!("send_photo", 2, 4, "bot"; builtin_send_photo, "stable"), // N3: chat_id, photo [, caption ≤1024] [, opts]; Unknown honest — String | Unit (no-token fallback), same env-dependent split as send_message
    spec!("answer_callback_query", 1, 3, "bot"; builtin_answer_callback_query, "stable"), // №543: Unknown honest — String | Unit (no-token fallback), same env-dependent split as send_message
    spec!("edit_message_text", 3, 4, "bot"; builtin_edit_message_text, "stable"), // №543: Unknown honest — String | Unit (no-token fallback), same env-dependent split as send_message
    // ── Voice / transcription ──
    // Naryad #279 fact-check: registry said min=1, implementation has always
    // required 3 strings (file_id, bot_token, whisper_key) + optional provider.
    // A 1-arg call passed mlog check and exploded at runtime — fixed to 3..4.
    spec!("whisper_transcribe", 3, 4, "voice"; builtin_whisper_transcribe, "String", "stable"), // file_id,bot_token,whisper_key | +provider
    spec!("tts_send", 4, 5, "voice"; builtin_tts_send, "String", "stable"), // text,voice,bot_token,chat_id | +mode — delivery convenience (delegates synthesis to tts_synth, Naryad #279)
    spec!("tts_generate", 2, 4, "voice"; builtin_tts_generate, "String", "stable"), // Naryad #279: text,voice | +provider | +model — synthesis to sandbox file, no delivery (APPENDED: bytecode indices must not shift)
    // ── System builtins ──
    spec!("env", 1, "system"; builtin_env, "String", "stable"),
    // №481: the EXPLICIT-silence twin of env — the `_or` suffix carries the
    // silent-default semantics in the name (audit 25.09 §3.9 naming rule).
    spec!("env_or", 2, "system"; builtin_env_or, "String", "stable"), // ── DB builtins ──
    spec!("query", 1, 2, "db"; builtin_query, "List<Struct<Row>>|String", "stable"), // №740 (gh#1378, Wave 53, lane α): the SQL-class UNION rows|count — the FIRST movement along the growth path №736/§6.37 named: SELECT/PRAGMA → the List of Row structs (the fields dynamic by construction — the type_name "Row" is the db_ops.rs:209/787 fact, no field-meta claimed; the rows-path NOT extended, the ADR-0194 Запреты hold), mutations → the affected count as a String on BOTH lanes (the №474 ONE contract — ADR-0194 ACCEPTED, the landing №734; TW db_ops.rs:212-218 / VM db_ops.rs:744-755; the SELECT List TW db_ops.rs:199-212 / VM db_ops.rs:757-790; the parity battery tests/n194_query_parity.rs). The union spelling names EVERY reachable arm — the №539 false-warn guard (the memory_read form: typing String alone would false-warn the PRIMARY SELECT arm) is satisfied by construction. The context-less call still refuses LOUDLY on both lanes (the №7 intercepts precede the registry dispatch — execution.rs:2440-2442 / vm.rs:2447 — the db-not-open named remedy, the №758 form; the registry handler's Value::Query wrapper is the shadowed static fallback arm; the t6 pin of tests/naryad_736_query_refusals.rs). The vocabulary: the stage-0 union grammar `arm|arm|...` (≥2 stage-0-clean arms, strictly ascending — sig_types.rs from_path/union_well_formed; the (β) fixed-result and (γ) Row/Query opaque-kind lanes NOT taken — separate owner gates, the В53 dispatch gh#1381 gate 7)
    spec!("db_execute", 1, 2, "db"; builtin_db_execute, "Unit", "stable"), // ADR-0068: optional params list — the interpreter path returns Unit; the executed surface is the Query pipeline (№484 DbAccess)
    // ── LLM builtins ──
    #[cfg(feature = "llm")]
    spec!("call_llm", 1, 3, "llm"; builtin_call_llm, "String", "stable"), // prompt | prompt,input | prompt,input,model (N12 gh#1220)
    #[cfg(feature = "llm")]
    spec!("call_claude", 4, "llm"; builtin_call_claude, "String", "stable"), // api_key,model,system,user
    #[cfg(feature = "llm")]
    spec!("llm_usage", 0, "llm"; builtin_llm_usage, "Struct<LlmUsage>{total_calls:internal,total_tokens:internal,total_errors:internal,cache_hits_semantic:internal,canary_leaks:internal,providers:internal}", "stable"),
    #[cfg(feature = "llm")]
    spec!("call_llm_schema", 2, 3, "llm"; builtin_call_llm_schema, "stable"), // prompt,schema | prompt,input,schema (Наряд №269, ADR-0133) — №717 re-refusal: the DATA-DEPENDENT return — Ok(json_to_mlog(&parsed)) (llm_schema.rs:162); json_to_mlog maps the LLM payload by shape (llm_schema.rs:287-303: object→Struct, array→List, string→String, bool→Bool, number→Float, null→Unit) — the flat vocabulary has no schema-dependent union; typing one arm would lie about the others (the query_row/query_scalar data-dependent form)
    // ── Memory builtins ──
    spec!("kv_set", 2, "memory"; builtin_kv_set, "Unit", "stable"),
    spec!("kv_get", 1, "memory"; builtin_kv_get, "String", "stable"),
    spec!("kv_delete", 1, "memory"; builtin_kv_delete, "Unit", "stable"),
    spec!("kv_exists", 1, "memory"; builtin_kv_exists, "Bool", "stable"),
    spec!("kv_list", 0, "memory"; builtin_kv_list, "List<String>", "stable"), // №631: the String keys (memory.rs: Value::String map)
    // №539: the mem_* twins RETURN the value (String) — the honest
    // asymmetry vs kv_set/kv_delete (Unit), verified in the handlers.
    spec!("mem_set", 2, "memory"; builtin_mem_set, "String", "stable"),
    spec!("mem_get", 1, "memory"; builtin_mem_get, "String", "stable"),
    spec!("mem_delete", 1, "memory"; builtin_mem_delete, "String", "stable"),
    spec!("memorize", 2, 3, "memory"; builtin_kv_set, "Unit", "stable"), // №539: the kv_set handler — Unit
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
    // both backends intercept by name BEFORE the generic fallback (the
    // media_store_* stub pattern); the registered fallback fires only on
    // the no-memory lanes.
    // №706 (gh#1306, Wave 48): the row is TYPED — the both-backend
    // intercept contracts are the serialized JSON hit array AS A String
    // (memory_ops recall_top_k_tw / recall_top_k_vm both end in
    // Value::String(serde_json::to_string(...))). The registered fallback
    // IS the no-memory lane contract (the №702 fail-closed form): a lane
    // without a memory store holds no hits, so the fallback answers the
    // declared type's empty value — the zero-hit serialization "[]",
    // exactly what the real lanes answer on an empty store — never a
    // wrong-type value.
    spec!("recall_top_k", 1, 3, "memory"; builtin_recall_top_k, "String", "stable"),
    // Наряд №272 (ADR-0134): векторный контур поверх sqlite-vec — KNN
    // (distance_metric=cosine), песочница через sandbox_path_ex, dim-гейт.
    #[cfg(feature = "vec")]
    spec!("embed", 1, "memory"; builtin_embed, "List<Float>", "stable"), // №539: the Float embedding vector; №631: the Float element read off the handler (Value::Float map)
    #[cfg(feature = "vec")]
    spec!("vec_store", 4, 5, "memory"; builtin_vec_store, "Struct<VecStoreResult>{stored:internal,table:untrusted,id:untrusted,dim:internal,rowid:internal}", "stable"), // db_path,table,id,embedding | +text|opts{text,scope} (№281) — №539: the VecStoreResult; №631: the name read off make_struct; №638: the fields read off make_struct (vector.rs:360-368) — table/id echo the caller's arguments → untrusted, stored/dim/rowid computed in-tree → internal
    #[cfg(feature = "vec")]
    spec!("vec_search", 4, 5, "memory"; builtin_vec_search, "List<VecSearchHit>", "stable"), // db_path,table,query,k | +include_forgotten (№280, дефолт false) — №539: the hit structs (the post-filter may shrink below k); №631: the VecSearchHit element read off make_struct (all three arms)
    // Наряд №442: the registry-level recall row (see the №442 comment at
    // the memory block head) — a REAL handler over the typed lane; the
    // stub-spec row is gone. forget/find/inspect remain the planned
    // high-level memory API rows (use kv_*/mem_* instead).
    spec!("recall", 1, 2, "memory"; builtin_recall, "String", "stable"), // №539: the best hit's text + the [MEM] provenance suffix; the miss is ""
    // ── Typed Memory<K> (№350): label-typed containers over the
    // process-global registry (src/memory_typed.rs) — private is
    // consent-gated + encrypted at rest, reads/exports are audited
    // sinks, redact is the only private egress. Category "memory".
    spec!("memory_open", 2, "memory"; builtin_memory_open, "stable"), // №539: Unknown honest — the opaque Value::Memory handle; the flat vocabulary has no Memory spelling (the №538 Query posture; the №730 fresh anchor: the wrapper src/builtins/memory_typed.rs:28-44, the handle src/memory_typed.rs:300)
    spec!("memory_put", 3, 5, "memory"; builtin_memory_put, "Unit", "stable"),
    spec!("memory_read", 2, "memory"; builtin_memory_read, "stable"), // №539: Unknown honest — String|Secret by the container label (public → String, private → the gated Secret); typing String would false-warn the private lane (the №730 fresh anchor: the wrapper src/builtins/memory_typed.rs:137-152, the union arms src/memory_typed.rs:550-556 — Stored::Plain → String | Stored::Enc → Value::Secret)
    spec!("memory_keys", 1, "memory"; builtin_memory_keys, "List<String>", "stable"), // №631: the String keys (memory_typed.rs)
    spec!("memory_provenance", 2, "memory"; builtin_memory_provenance, "List<String>", "stable"), // №631: the String parents (memory_typed.rs)
    spec!("memory_export", 3, "memory"; builtin_memory_export, "String", "stable"), // №539: the exported path
    // №351 (ADR-0173): the derived-graph surfaces — the cascade preview
    // (the №280 dry-run discipline), the retain pins, and the grant-gated
    // cascading forget (the ADR-0155 linear action, `irreversible.
    // memory_forget` ledger record; scope `memory:forget:<container>`).
    spec!("memory_cascade_preview", 2, "memory"; builtin_memory_cascade_preview, "Struct<MemoryCascadePlan>{closure:internal,blocked_by:internal}", "stable"), // №539: the MemoryCascadePlan; №631: the name read off make_struct; №638: both fields are the in-tree graph computation (memory_typed.rs:286-294) → internal
    spec!("memory_retain", 2, "memory"; builtin_memory_retain, "Unit", "stable"),
    spec!("memory_release", 2, "memory"; builtin_memory_release, "Unit", "stable"),
    spec!("memory_retained", 1, "memory"; builtin_memory_retained, "List<String>", "stable"), // №631: the String keys (memory_typed.rs)
    spec!("memory_forget_cascade", 3, "memory"; builtin_memory_forget_cascade, "Struct<MemoryForgetResult>{root:untrusted,deleted:internal,batch_id:internal}", "stable"), // №539: the MemoryForgetResult; №631: the name read off forget_outcome_value; №638: the fields read off forget_outcome_value (memory_typed.rs:249-268) — root echoes the caller's key → untrusted, deleted/batch_id computed → internal (NOTE: the SAME type_name carries a DIFFERENT field set on the forget row — the two-shapes observation recorded in the PR)
    // №352 (ADR-0174): the duplex channel — barge-in over the №348
    // session priority ladder. NOT feature-gated: the CI contour
    // exercises the same state machine as the serve contour. The
    // direction is STATIC (separate *_start/*_stop surfaces) so the
    // №324 effect trail can type it (speak_start → ⟨speak⟩ etc.).
    spec!("duplex_open", 1, 2, "voice"; builtin_duplex_open, "stable"), // №717 re-refusal (the №748 anchor refresh): the OPAQUE Value::Duplex handle (src/duplex.rs:218 via channel_value src/duplex.rs:172-177 — the №748 re-verified anchor: the cited 213 drifted to 218) — no Duplex spelling in the stage-0 vocabulary (the №538 posture); the introspection duplex_state and the flow projections speak/listen ARE typed (№712/№717 listen_start)
    spec!("speak_start", 2, 3, "voice"; builtin_speak_start, "stable"),
    spec!("listen_start", 1, 2, "voice"; builtin_listen_start, "Struct<DuplexFlow>{stream_id:internal,direction:internal,priority:untrusted,preempted:internal}", "stable"), // №717 (gh#1329, Wave 50): the FIXED 4-field flow projection (duplex.rs start_result_value "DuplexFlow", the SINGLE Ok arm — duplex.rs:122); stream_id is the FRESH registry-generated stream id → internal (the cron_list generated-count form); direction is the fixed speak|listen word → internal; priority echoes the caller's ladder word (or the session default) → untrusted (the №706 device_state echo form); preempted is the registry-generated List of preempted stream ids → internal
    spec!("speak_stop", 1, "voice"; builtin_speak_stop, "stable"),
    spec!("listen_stop", 1, "voice"; builtin_listen_stop, "stable"),
    spec!("duplex_state", 1, "voice"; builtin_duplex_state, "Struct<DuplexState>{id:untrusted,session:internal,speak:internal,listen:internal,speak_last:internal,listen_last:internal}", "stable"), // №712 (gh#1319, Wave 49): the FIXED 6-field introspection projection (duplex.rs state → struct_of "DuplexState", the SINGLE Ok arm — unlike the heterogeneous backend_select); id echoes the caller's channel handle → untrusted (the №706 device_state form); session and the four stream projections read off the in-tree duplex registry → internal (the goal_get store-read form); speak/listen/speak_last/listen_last are the OPTIONAL Struct<DuplexStream>|Unit slots (the memory_revise precedent — the inner DuplexStream carries stream_id/direction/priority/outcome [+interrupted_by/interrupted_priority] metadata, no content payload)
    // №355 (registry В5 — Phase 5 «Embodied, sim-only», ADR-0159): the
    // embodied surface over the sim contour. NOT feature-gated: the
    // contour is sim-only (in-tree deterministic records — no hardware,
    // no GPU, no weights); the monitor/state evolution lands with №356
    // BEHIND the GPU-budget gate, the TYPES are live now.
    spec!("device_open", 1, 2, "embodied"; builtin_device_open, "experimental"), // №717 re-refusal: the OPAQUE Value::Device handle (embodied.rs:351 via device_value embodied.rs:412-417) — the stage-0 vocabulary has no Device spelling (the №538 Query posture; the №669 forecast_next form); the introspection projection device_state IS typed (№706)
    spec!("bounds_attach", 2, "embodied"; builtin_bounds_attach, "experimental"),
    spec!("device_state", 1, "embodied"; builtin_device_state, "Struct<EmbodiedDeviceState>{id:untrusted,backend:internal,bounds_present:internal,bounds_digest:internal}", "experimental"), // №706 (gh#1306, Wave 48): the FIXED introspection shape (embodied.rs device_state → make_struct "EmbodiedDeviceState", the SINGLE Ok arm); id echoes the caller's handle → untrusted (the boost/todo form); backend/bounds_present/bounds_digest read off the in-tree device registry → internal; bounds_digest is the OPTIONAL field (Unit when bounds are absent — the memory_revise optional-field precedent)
    spec!("world_state", 1, "embodied"; builtin_world_state, "Embodied", "experimental"), // №668: the OPAQUE WorldState handle (embodied.rs:489-497 world_value → Value::WorldState; the embodied contour №355/ADR-0159; the audit's own opaque grouping). The honest cause why the SENSITIVE row does NOT close on this package: the handle is neither scalar nor a parameterized List/Struct (typed-coarse stays uncovered) and every materialization surface refuses it (WORLD_STATE_PRIVATE + audit, private-by-default №349/№355) — closing it would need a dictionary/vocabulary decision that is OUTSIDE this naryad (the №604-dictionary lane is the separate owner decision)
    spec!("pose_make", 4, "embodied"; builtin_pose_make, "experimental"), // №717 re-refusal: the OPAQUE Value::Pose handle (embodied.rs:576) — no Pose spelling in the stage-0 vocabulary (the №538 posture; the №669 forecast_next form)
    spec!("trajectory_make", 1, "embodied"; builtin_trajectory_make, "experimental"), // №717 re-refusal: the OPAQUE Value::Trajectory handle (embodied.rs:627) — no Trajectory spelling in the stage-0 vocabulary (the №538 posture; the №669 forecast_next form)
    spec!("goal_make", 1, "embodied"; builtin_goal_make, "experimental"), // №717 re-refusal: the OPAQUE Value::GoalPredicate handle (embodied.rs:676) — no GoalPredicate spelling in the stage-0 vocabulary (the №538 posture; the №669 forecast_next form); the goal_get store-read projection IS typed (№706)
    spec!("chunk_make", 2, 3, "embodied"; builtin_chunk_make, "experimental"),
    spec!("proof_seal", 2, 3, "embodied"; builtin_proof_seal, "experimental"),
    spec!("proof_verify", 1, "embodied"; builtin_proof_verify, "Struct<EmbodiedProofReport>{valid:internal,proof_id:untrusted,verdict:internal,world_hash:internal,chunk_id:internal,bounds_id:internal,backend:internal,reasons:internal}", "experimental"), // №706 (gh#1306, Wave 48): the FIXED report shape (embodied.rs proof_verify → make_struct "EmbodiedProofReport", the SINGLE Ok arm); proof_id echoes the caller's handle → untrusted (the boost/todo form); valid/verdict/world_hash/chunk_id/bounds_id/backend/reasons read off the in-tree proof record → internal
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
    spec!("forget", 3, 4, "memory"; builtin_forget, "Struct<MemoryForgetResult>{candidates:internal,applied:internal,batch_id:internal}", "stable"), // №539: the 3..4-argument typed front door (MemoryForgetResult); №631: the name read off make_struct — the legacy 1..2-argument forget(query, days?) intercepts first and returns Unit — a different arity surface; №638: the fields read off struct_value BOTH arms (memory_forget.rs:464, 538) — candidates/applied/batch_id computed in-tree → internal (NOTE: the type_name collides with the cascade row's shape — see the PR)
    spec!("find", 4, "stub", "stable"), // №748 re-refusal (gh#1392, Wave 55): the №717 "planned wrapper, no handler" anchor was factually STALE — the handler IS LIVE on both lanes (the ADR-0051 intercepts — TW invoke_find interpreter/memory.rs:196-206, VM vm.rs:2415-2425): the FIRST variable/globals value matching the (entity_type, field, op, threshold) predicate, or the soft-failure Unit when nothing matches — the matched value is an ARBITRARY Value (any type in scope) — the data-dependent ground the flat vocabulary cannot spell (the call_llm_schema №717 form: the bare arms are not covered-grade); the Unknown ground stays
    spec!("inspect", 1, "stub", "stable"), // №748 re-refusal (gh#1392, Wave 55): the №717 "planned wrapper, no handler" anchor was factually STALE — the handler IS LIVE on both lanes (the ADR-0051 intercepts — TW invoke_inspect learnable.rs:1264, VM vm.rs:2716-2760): the pattern exists → the FIXED 8-field stats Struct "PatternStats" (calls/avg_confidence/cache_hits/cache_misses/last_adapt/last_call/examples_count/is_learnable — all Floats, both lanes), the pattern does not exist → Unit — the STATE-DEPENDENT Struct|Unit union (the №543 send_message form — the (β) lane, the OWNER gate 7, NOT taken — the session_* №744 wall form); the Unknown ground stays until the owner's (β) verdict
    // conv_start/add/history/context/end: conversation lifecycle management.
    // №702 (gh#1296, Wave 47): conv_history/conv_context are TYPED — the
    // live dispatch lives in session_ops (the №72 TW FnCall intercepts and
    // the VM parity lanes), and the probe-verified contracts are
    // List<Message> (the fixed Message{role,text,timestamp} shape,
    // session_ops.rs:198-209) and String (the "{role}: {text}" injection
    // format, session_ops.rs:233-235). The registered fallback IS the
    // context-less lane contract (the authenticate mock-handler form and
    // the query_param №565 fail-closed value): an off-server lane holds no
    // conversations, so the fallback answers the declared type's empty
    // value — never a wrong-type value. conv_start/conv_add/conv_end stay
    // untyped stubs (the mod.rs:180 doctrine — no documented path yet).
    spec!("conv_start", 1, "stub", "stable"),
    spec!("conv_add", 3, "stub", "stable"),
    spec!("conv_history", 1, "stub"; builtin_conv_history, "List<Message>", "stable"),
    spec!("conv_context", 1, "stub"; builtin_conv_context, "String", "stable"),
    spec!("conv_end", 1, "stub", "stable"),
    // №523: the spec is 3 — the implementation (builtins/memory.rs
    // builtin_session_set) hard-requires (session_id, key, value) and
    // refuses 2 arguments loudly at runtime; the 2 here was a stale
    // pre-session-id spec the №523 run gate surfaced (examples/p8 calls
    // with 3 on every line).
    spec!("session_set", 3, "memory"; builtin_session_set, "String", "stable"), // №539: the stored value
    // №523: the specs are the implementation's truth (builtins/memory.rs):
    // session_get hard-requires (session_id, key); session_clear requires
    // exactly (session_id). The stale 1/0 specs predated the session-id
    // parameter; the №523 run gate surfaced the drift (examples/p8).
    spec!("session_get", 2, "memory"; builtin_session_get, "String", "stable"),
    spec!("ref", 1, "memory"; builtin_content_ref, "String", "stable"), // №539: the SHA-256 hex hash
    spec!("deref", 1, "memory"; builtin_content_deref, "String", "stable"), // ── Time builtins ──
    spec!("now", 0, "time"; builtin_now, "Float", "stable"),
    spec!("sleep", 1, "time"; builtin_sleep, "Unit", "stable"),
    spec!("time", 0, "time"; builtin_now, "Float", "stable"),
    spec!("add_days", 2, "time"; builtin_add_days, "Float", "stable"),
    spec!("add_hours", 2, "time"; builtin_add_hours, "Float", "stable"),
    spec!("date_parts", 1, "time"; builtin_date_parts, "stable"),
    spec!("format_date", 2, "time"; builtin_format_date, "String", "stable"),
    spec!("days_between", 2, "time"; builtin_days_between, "Float", "stable"),
    spec!("days_in_month", 2, "time"; builtin_days_in_month, "Float", "stable"),
    spec!("is_leap_year", 1, "time"; builtin_is_leap_year, "Bool", "stable"),
    // №591: typed PRECISE ("String" — the verified single-path handler
    // fact: the only Ok arm is Value::String(WEEKDAY_NAMES_MON[...]); the
    // №565 procedure) — compensates the typed-but-coarse "Struct" of the
    // spectral rows so the precise share keeps moving only up (№560).
    spec!("weekday_name", 1, "time"; builtin_weekday_name, "String", "stable"), // ── Graph builtins ──
    spec!("graph_query", 1, 3, "graph"; builtin_graph_query, "List<GraphEntry>", "stable"), // query | query,limit | query,limit,level — №669: the element read off builtin_graph_query (memory.rs:759-769, make_date_struct "GraphEntry", the SINGLE arm); id/text/level are the memory-graph node payload (untrusted), score/relevance are the in-tree graph_search scoring (internal); the element's field-labels — stage 2 (the Tool/DayForecast precedent)
    spec!("graph_path", 2, "graph"; builtin_graph_path, "List<Struct<PathError>|Struct<PathNode>>", "stable"), // from_id,to_id — №744 (gh#1385, Wave 54, lane α): the result is ALWAYS a List (the №170 SSOT handler builtin_graph_path — memory.rs:789 — serves both lanes): the found arm carries the PathNode{id,text,level} elements (memory.rs:800), the no-path arm the SINGLE PathError{status,from,to} element (memory.rs:812) — the №669 "a fictitious List<PathNode> would lie about the no-path arm" concern dissolves through the union INNER (the №740 query_row form): the element spelling names BOTH reachable shapes, `Struct<PathError>|Struct<PathNode>` strictly ascending (the canonical form, the №740 grammar). Field-labels stage 2 (the sibling graph_query №669 precedent); no field-meta claimed — the type_name facts are the make_date_struct calls. The honest shape move (the typed error channel) stays OUTSIDE the types movement (the ADR-0194 Запреты — no semantics change)
    spec!("graph_neighbors", 0, "graph"; builtin_graph_neighbors, "List<Neighbor>", "stable"), // №669: the element read off builtin_graph_neighbors (memory.rs:837-845, make_date_struct "Neighbor", the SINGLE arm); id/text/level are the memory-graph node payload (untrusted), distance is the in-tree BFS computation (internal); the element's field-labels — stage 2 (the Tool/DayForecast precedent)
    spec!("memory_decay", 0, "memory"; builtin_memory_decay, "Struct<DecayResult>{decayed:internal,nodes:internal,edges:internal,components:internal}", "stable"), // №539: the DecayResult; №631: the name read off make_date_struct; №638: all four fields are the in-tree graph computation (memory.rs:865-871) → internal
    spec!("memory_boost", 0, "memory"; builtin_memory_boost, "Struct<BoostResult>{id:untrusted,new_score:internal,access_count:internal}", "stable"), // №539: the BoostResult; №631: the name read off make_date_struct; №638: the fields read off make_date_struct (memory.rs:893-902) — id echoes the caller's argument → untrusted, the metrics computed → internal
    spec!("memory_prune", 0, "memory"; builtin_memory_prune, "Struct<PruneResult>{pruned:internal,remaining:internal}", "stable"), // №539: the PruneResult; №631: the name read off make_date_struct; №638: both fields computed in-tree (memory.rs:924-929) → internal
    spec!("memory_revise", 0, "memory"; builtin_memory_revise, "Struct<ReviseResult>{action:internal,winner_id:internal,superseded_id:internal}", "stable"), // №539: the ReviseResult; №631: the name read off make_date_struct; №638: the fields read off make_date_struct (memory.rs:957-964) — all computed in-tree; superseded_id is OPTIONAL (present only when a contradiction resolved) → internal
    spec!("subgraph_extract", 0, "graph"; builtin_subgraph_extract, "stable"), // №717 re-refusal: the OPAQUE Value::Subgraph snapshot handle (memory.rs:1010) — no Subgraph spelling in the stage-0 vocabulary (the №538 posture); the projections subgraph_nodes/subgraph_json carry their own contracts
    spec!("subgraph_nodes", 0, "graph"; builtin_subgraph_nodes, "stable"),
    spec!("subgraph_json", 0, "graph"; builtin_subgraph_json, "stable"),
    spec!("trace_start", 0, "graph"; builtin_trace_start, "stable"),
    spec!("trace_end", 0, "graph"; builtin_trace_end, "stable"),
    spec!("memory_score", 1, "bot" => "ext"; builtin_memory_score, "Struct<MemoryScore>{score:internal,admitted:internal,token_count:internal,unique_words:internal,entity_density:internal}", "stable"), // №543: the MemoryScore make_date_struct (verified office/text.rs); №631: the name read off the handler; №638: all five fields are the in-tree weighted computation (office/text.rs:424-441) → internal
    spec!("mtree_summarize", 0, "mtree"; builtin_mtree_summarize, "Struct<MTreeSummarize>{l0_promoted:internal,l1_count:internal,l2_created:internal,status:internal,graph_nodes:internal,graph_edges:internal,components:internal}", "stable"), // №668: the shape read off builtin_mtree_summarize (memory.rs:690-701, make_date_struct "MTreeSummarize", the SINGLE arm); all 7 fields are the in-tree graph computation (the promotion counters, the recount, graph.stats()) → internal
    spec!("mtree_retrieve", 1, 2, "mtree"; builtin_mtree_retrieve, "List<MTreeEntry>", "stable"), // query | query,limit — №668: the element read off builtin_mtree_retrieve (memory.rs:535-545, make_date_struct "MTreeEntry", the SINGLE arm); id/text/level are the memory-graph node payload (untrusted), score/relevance are the in-tree graph_search scoring (internal); the element's field-labels stay OFF the spelling at stage 0 (the Tool/DayForecast precedent)
    spec!("mtree_store", 2, "mtree"; builtin_mtree_store, "stable"),
    spec!("mtree_stats", 0, "mtree"; builtin_mtree_stats, "Struct<MTreeStats>{l0:internal,l1:internal,l2:internal,total:internal,total_chars:internal,graph_edges:internal,components:internal}", "stable"), // №668: the shape read off builtin_mtree_stats (memory.rs:716-727, make_date_struct "MTreeStats", the SINGLE arm); all 7 fields are the in-tree graph diagnostics (count_by_level/total_chars/stats) → internal
    spec!("mtree_forget", 1, "mtree"; builtin_mtree_forget, "stable"), // ── Cron builtins ──
    spec!("cron_mark_fired", 1, "cron"; builtin_cron_mark_fired_stamped, "stable"),
    // №418: 2..5 — the optional tz / catch_up / payload extensions
    // (additive arity widening; the 2-arg 0.20.x call shape is unchanged).
    spec!("cron_add", 2, 5, "cron"; builtin_cron_add_stamped, "stable"), // cron_expr, prompt, tz?, catch_up?, payload?
    spec!("cron_list", 0, "cron"; builtin_cron_list_stamped, "Struct<CronJob>{id:internal,cron_expr:internal,prompt:private,enabled:internal,run_count:internal,force_run:internal,created_at:internal,last_run:internal,last_run_tz:internal,last_window:internal,tz:internal,catch_up:internal,payload:private,next_run:internal,next_run_tz:internal}", "stable"), // №706 (gh#1306, Wave 48): the FIXED job shape (cron.rs builtin_cron_list → make_date_struct "CronJob", 15 fields, the SINGLE Ok arm); prompt/payload are the user-authored job content → private (the №689 mail form — the registry private-flow bridge seeds the fail-closed egress posture); the scheduling metadata reads off the in-tree store → internal (the goal_get store-read form); last_run/last_window/next_run are the OPTIONAL Float|Unit fields (the memory_revise precedent)
    spec!("cron_remove", 1, "cron"; builtin_cron_remove_stamped, "stable"),
    spec!("cron_run", 1, "cron"; builtin_cron_run_stamped, "stable"), // ── Event / query analytics stubs ──
    // event_count/events_since/event_sum: the ADR-0052 intercepts are the
    // LIVE handlers on both lanes (№748) — the "stub" class names only the
    // context-less registry fallback (the query_scalar №740 form).
    spec!("event_count", 0, "stub"; intercept, "Float", "stable"), // №748 (gh#1392, Wave 55, lane α): the handler IS LIVE on both lanes (the ADR-0052 intercepts — TW execution.rs:2337-2339, VM vm.rs:2824-2827; the №717 "planned wrapper, no handler" anchor was factually STALE — the №736 intercept-walk lesson) — the shared audit_ops::event_count answers the LOG COUNT as Value::Float (audit_ops.rs:86-91, `Value::Float(count as f64)` — the empty log reads 0.0, the optional etype filter does not change the return shape); the verified single-shape fact (the №757 decrypt form); the registry "stub" class names only the context-less fallback (the intercept marker variant — the query_scalar №740 form)
    spec!("events_since", 1, "stub"; intercept, "List<Struct<Event>>", "stable"), // №748 (gh#1392, Wave 55, lane α): the handler IS LIVE on both lanes (the ADR-0052 intercepts — TW execution.rs:2342-2344, VM vm.rs:2834-2837) — the shared audit_ops::events_since answers the List of the ledger Event structs (audit_ops.rs:142-179: every element is Struct{type_name: "Event"} — the type_name fact; the FIELDS are the audit_ops.rs:165-177 mapping — id/timestamp/duration_ms Floats, event_type/source/data_json Strings, duration_ms present only when the record carries a duration — the fields dynamic by construction, NO field-meta claimed — the query rows №740 form: the type_name carries the shape); the Float validation and the wrong-arity refusals stay LOUD (the error channel, not a shape)
    spec!("event_sum", 2, "stub"; intercept, "Float", "stable"), // №748 (gh#1392, Wave 55, lane α): the handler IS LIVE on both lanes (the ADR-0052 intercepts — TW execution.rs:2351-2353, VM vm.rs:2829-2832; the №717 "planned wrapper, no handler" anchor was factually STALE) — the shared audit_ops::event_sum requires exactly 2 arguments (the wrong-arity refusal stays LOUD — audit_ops.rs:115-117) and answers the numeric field SUM as Value::Float (audit_ops.rs:123; a poisoned log reads 0.0 — the single-shape fact, the №757 decrypt form)
    spec!("query_scalar", 0, "stub"; intercept, "Float|String|Unit", "stable"), // №740 (gh#1378, Wave 53, lane α): the handler IS LIVE on both db lanes (the №26 P1-7 intercepts — TW execution.rs:2474-2482, VM vm.rs:2439-2442; the №730 "planned wrapper, no handler" anchor was factually STALE, corrected by №736) — the class "stub" names only the registry's context-less fallback (the loud Err, handler: None — the intercept marker variant); the value type is data-dependent by column type: Null→Unit, Integer/Real→Float, Text/Blob→String (db_ops.rs:478-489 — the №734 re-verified mapping rows) → the CELL UNION `Float|String|Unit`: the stage-0 union grammar (№740, lane α) spells every reachable arm — typing String alone would false-warn the Float/Unit arms (the memory_read №539 form); the arms strictly ascending (the canonical form). The t2–t4 pins of tests/naryad_736_query_refusals.rs carry the live shapes; the (β)/(γ) lanes NOT taken (the В53 dispatch gh#1381 gate 7)
    spec!("query_row", 0, "stub"; intercept, "List<Float|String|Unit>", "stable"), // №740 (gh#1378, Wave 53, lane α): the handler IS LIVE on both db lanes (the №26 P1-7 intercepts — TW execution.rs:2483-2490, VM vm.rs:2706-2712; the №730 anchor was factually STALE, corrected by №736) — the class "stub" names only the registry's context-less fallback (the loud Err, handler: None — the intercept marker variant); the row returns the FIRST row's cells as a List (db_ops.rs:570-576), the element type data-dependent by column type: Null→Unit, Integer/Real→Float, Text/Blob→String (db_ops.rs:554-564) → the HETEROGENEOUS List spelled honestly through the union INNER: `List<Float|String|Unit>` — the №669 "a List<X> spelling would be fictitious" concern dissolves because the inner is no longer a single fictitious X but the EXACT cell union (the №740 vocabulary); the parameterized form (the erasure keeps the List head). The t1 pin of tests/naryad_736_query_refusals.rs carries the live shape; the (β)/(γ) lanes NOT taken (the В53 dispatch gh#1381 gate 7) // ── Test builtins ──
    spec!("assert_eq", 2, "test"; builtin_assert_eq, "stable"),
    spec!("assert_contains", 2, "test"; builtin_assert_contains, "stable"), // ── Fluid builtins ──
    spec!("confidence", 1, "fluid"; builtin_confidence, "Float", "stable"), // ── Encoding builtins ──
    spec!("toon_encode", 1, "encoding"; builtin_toon_encode, "String", "stable"),
    spec!("toon_decode", 1, "encoding"; builtin_toon_decode, "stable"), // ── Recipe / DAG / Orchestration builtins ──
    spec!("recipe_save", 0, "recipe" => "ext"; builtin_recipe_save, "stable"),
    spec!("recipe_search", 1, 2, "recipe" => "ext"; builtin_recipe_search, "stable"), // semantic search via recall_top_k + kv_get — №717 re-refusal: the always-empty List handler (src/builtins/office/recipes.rs:63-71 — the same placeholder ground as recipe_list src/builtins/office/recipes.rs:80-87; the №730 re-verified anchors) — a List<X> spelling would be fictitious (the №712 recipe_list form; the №669 graph_path form)
    spec!("recipe_list", 0, "recipe" => "ext"; builtin_recipe_list, "stable"), // №712: Unknown honest — the handler answers the always-empty List (src/builtins/office/recipes.rs:80-87 — the №730 re-verified anchor; no live element instance exists on any lane) — spelling List<X> would be fictitious (the graph_path №669 form)
    spec!("dag_phases", 1, "orchestration" => "ext"; builtin_dag_phases, "stable"),
    spec!("topo_sort", 1, "orchestration" => "ext"; builtin_topo_sort, "stable"),
    spec!("resolve_skill_index", 1, "stub", "stable"), // №756 (gh#1408, Wave 57): the lane divergence from the №751 (gh#1397) walk is RESOLVED — the lanes answer ONE shape now: type_name = format!("SkillIndex_{dept}") on BOTH (the TW canon — TW execution.rs:2600; VM vm.rs:2524-2529 was the fixed "SkillIndex") and the truncation field carries on BOTH (the TW spelling "whole_skill_only"/"truncate_at_boundary"; the VM rides the compiled field — bytecode.rs CompiledSkillIndex.truncation, the compiler carries si.truncation, compiler.rs:414-425; the VM arm vm.rs:2518-2523 was budget-only). The absent-field READ is loud-fail on BOTH lanes too (the ADR-0194 package A = (c) contract: the TW FieldAccess propagated the get_field Err already — execution.rs:1743; the VM GetField silent Unit fallback REMOVED on both VM loops — vm.rs:1230-1241 run() / vm.rs:1965-1977 execute_code_inner(), was: unwrap_or(Value::Unit), the №604 class-4 silently-wrong shape). Handlers live on both lanes (the №466 group 7 shared validation runtime_ops.rs:261-290; the №717 "true stub" anchor stayed dead). Pins: tests/naryad_756_lane_divergences.rs (the type_name/truncation/absent-field parity on both lanes); the ledger: docs/adr/0075-tw-vm-divergence.md (the №756 section). The №751 divergence pin (tests/naryad_751_endgame_walk.rs) checks the shared SHAPE (type_of = Struct) — it holds unchanged. The row category ("stub") is the registry taxonomy, NOT a coverage claim; the spelling stays the pre-union honest form — the union grammar has no dynamic-arm shape (the №669 form), so the Struct<SkillIndex> spelling is not invented here.
    spec!("fit_to_budget", 0, "stub", "stable"), // №751 (gh#1397, Wave 56): the "planned budget planner; no handler" anchor was STALE — the handler IS LIVE on both lanes (TW execution.rs:964/2605-2609, VM vm.rs:2839 — the shared №466 group 7 body runtime_ops.rs:78, the identity-on-lists MVP per its own unit test runtime_ops.rs:304); NOT in the №688 fixed list — outside the movement scope (the passthrough element typing is its own future walk)
    spec!("map", 0, "stub", "stable"), // planned list mapper; no handler (use filter+reduce instead)
    // ── OpenPlanter-inspired: fuzzy / safe editing / agent utilities ──
    spec!("fuzzy_find_best", 2, "string"; builtin_fuzzy_find_best, "stable"), // №536: Unknown honest — Unit (no candidates) | struct{index, candidate, score}
    spec!("hashline_read", 1, "string"; builtin_hashline_read, "String", "stable"),
    spec!("hashline_edit", 2, "string"; builtin_hashline_edit, "String", "stable"),
    spec!("compact_list", 3, "list"; builtin_compact_list, "stable"), // №537: Unknown honest — List | the struct report{compacted, removed_count}
    spec!("budget_check", 2, "fluid"; builtin_budget_check, "stable"),
    spec!("replay_snapshot", 1, "system"; builtin_replay_snapshot, "Struct<ReplaySnapshot>{seq:internal,count:internal,snapshot:untrusted}", "stable"), // №706 (gh#1306, Wave 48): the FIXED snapshot shape (server.rs → make_struct "ReplaySnapshot", the SINGLE Ok arm); seq is the constant 0.0 and count the argument length → internal; snapshot serializes the caller's argument — the echo form → untrusted (the boost/todo echo form)
    spec!("policy_check", 1, "system"; builtin_policy_check, "Struct<PolicyResult>{allowed:internal,reason:internal}", "stable"), // ── obsidian-mind: Vault / semantic search ──
    spec!("semantic_search", 3, "vault" => "ext"; builtin_semantic_search, "List<SearchResult>", "stable"), // №668: the element read off builtin_semantic_search (office/text.rs:738-745, make_struct "SearchResult", the SINGLE arm — the empty-documents early return IS the empty List of the same shape); text echoes the caller's documents (untrusted), index/score are the in-tree embedding/similarity computation (internal); the element's field-labels — stage 2 (the Tool/DayForecast precedent)
    spec!("config_load", 1, "vault" => "ext"; builtin_config_load, "stable"), // №717 re-refusal: the DATA-DEPENDENT return — the shape follows the vault document (config.rs:130 → json_value_to_mlog_value_with_type, json.rs:321-341: the mapping mirrors the JSON shape: object→Struct, array→List, scalar→the scalar); the flat vocabulary has no document-dependent union (the №717 call_llm_schema form)
    spec!("vault_validate", 2, "vault" => "ext"; builtin_vault_validate, "stable"),
    // ── Bot — Telegram ──
    spec!("todo_add", 2, "bot" => "ext"; builtin_todo_add, "Struct<Todo>{id:internal,title:untrusted,status:untrusted}", "stable"), // №543: the Todo make_date_struct (verified goals.rs); №631: the name read off the handler; №638: the fields read off make_date_struct (goals.rs:265-270) — id computed → internal; title echoes the argument, status echoes the optional argument (default const) → untrusted
    spec!("todo_list", 0, "bot" => "ext"; builtin_todo_list, "List<Todo>", "stable"), // №543: the todo rows vector (verified goals.rs); №631: the Todo element read off the handler
    spec!("todo_update", 2, "bot" => "ext"; builtin_todo_update, "Struct<TodoUpdate>{id:untrusted,old_status:internal,new_status:untrusted,updated:internal}", "stable"), // №543: the TodoUpdate make_date_struct (verified goals.rs); №631: the name read off the handler; №638: the fields read off make_date_struct (goals.rs:308-315) — id/new_status echo the arguments → untrusted; old_status read from the in-tree store, updated computed → internal
    spec!("goal_get", 0, "bot" => "ext"; builtin_goal_get, "Struct<ThreadGoal>{objective:internal,status:internal,budget:internal,spent:internal}", "stable"), // №543: the ThreadGoal make_date_struct (verified goals.rs); №631: the name read off the handler (both paths); №638: the fields read off make_date_struct ALL 3 arms (goals.rs:38-43, 56-62, 76-81) — reads from the in-tree kv store, no call-argument echo (0-arg row) → internal
    spec!("goal_set", 2, "bot" => "ext"; builtin_goal_set, "Struct<ThreadGoal>{objective:untrusted,status:internal,budget:untrusted,spent:internal}", "stable"), // №543: the ThreadGoal make_date_struct (verified goals.rs); №631: the name read off the handler; №638: the fields read off make_date_struct (goals.rs:38-43) — objective/budget echo the arguments → untrusted (the geo_ip ip-field precedent); status const, spent const → internal
    spec!("goals_add", 1, "bot" => "ext"; builtin_goals_add, "Struct<Goal>{id:internal,text:untrusted,status:internal}", "stable"), // №543: the Goal make_date_struct (verified goals.rs); №631: the name read off the handler; №638: the fields read off make_date_struct (goals.rs:172-180) — id computed (g{n}) → internal; text echoes the argument → untrusted; status const → internal
    spec!("goals_list", 0, "bot" => "ext"; builtin_goals_list, "List<Goal>", "stable"), // №543: the goal rows vector (verified goals.rs); №631: the Goal element read off the handler
    spec!("remind", 3, "bot"; builtin_remind_stamped, "String", "stable"), // №543: the reminder id (verified cron.rs builtin_remind; the stamped wrapper is pass-through)
    spec!("get_profile", 0, "bot" => "ext"; builtin_get_profile, "List<Preference>", "stable"), // №543: the profile rows vector (verified); №631: the Preference element read off the handler
    spec!("human_mood", 3, "bot"; builtin_human_mood, "Struct<Mood>{persona:untrusted,mood:internal,intensity:internal,updated_at:internal}", "stable"), // №543: the Mood make_date_struct (verified server.rs); №631: the name read off the handler; №638: the fields read off make_date_struct (server.rs:654-663) — persona echoes the caller's argument → untrusted; mood/intensity/updated_at read from the in-tree persona store → internal
    spec!("ask_approval", 1, "bot" => "ext"; builtin_ask_approval, "Struct", "stable"), // №543: the Approval make_date_struct (verified config.rs)
    spec!("goal_complete", 0, "bot" => "ext"; builtin_goal_complete, "Struct", "stable"), // №543: the GoalComplete make_date_struct (verified goals.rs)
    spec!("goals_reflect", 0, "bot" => "ext"; builtin_goals_reflect, "Struct", "stable"), // №543: the GoalsReflection make_date_struct (verified goals.rs)
    spec!("cancel_remind", 1, "bot"; builtin_cancel_remind_stamped, "String", "stable"), // №543: the literal "ok"/"not_found" (verified cron.rs)
    spec!("check_reminders", 0, "bot"; builtin_check_reminders_stamped, "List<DueReminder>", "stable"), // №543: the DueReminder rows vector (verified cron.rs); №668: the element read off builtin_check_reminders (cron.rs:284-297, make_date_struct "DueReminder", the SINGLE arm); id is the in-tree uuid (internal), message/data are the caller-set reminder payload (untrusted), type/next_fire/overdue_seconds are the in-tree scheduler computation (internal); the element's field-labels — stage 2 (the Tool/DayForecast precedent)
    spec!("list_reminders", 0, "bot"; builtin_list_reminders_stamped, "List<Reminder>", "stable"), // №543: the Reminder rows vector (verified cron.rs); №668: the element read off builtin_list_reminders (cron.rs:242-256, make_date_struct "Reminder", the SINGLE arm); id is the in-tree uuid (internal), message/data/fire_at/interval/created_at echo the caller's reminder payload and schedule (untrusted), next_fire/type are the in-tree scheduler computation (internal); the element's field-labels — stage 2 (the Tool/DayForecast precedent)
    spec!("remind_recurring", 2, "bot"; builtin_remind_recurring_stamped, "String", "stable"), // №543: the reminder id (verified cron.rs)
    spec!("human_create", 2, "bot"; builtin_human_create, "Struct", "stable"), // №543: the Persona make_date_struct (verified server.rs)
    spec!("human_delete", 1, "bot" => "ext"; builtin_human_delete, "Struct", "stable"), // №543: the DeleteResult make_date_struct (verified office/human.rs)
    spec!("human_forget", 2, "bot"; builtin_human_forget, "stable"), // №543: Unknown honest — Float (deleted count, 1-arg all-persona path) | String ("ok"/"not_found", 2-arg path); arity-dependent union
    spec!("human_personas", 0, "bot" => "ext"; builtin_human_personas, "List<PersonaSummary>", "stable"), // №543: the persona rows vector (verified); №668: the element read off builtin_human_personas (office/human.rs:210-220, make_date_struct "PersonaSummary", the SINGLE arm); name/traits/mood/created_at are the persisted persona data (untrusted), memory_count is the in-tree KV count (internal); the element's field-labels — stage 2 (the Tool/DayForecast precedent)
    spec!("human_recall", 3, "bot"; builtin_human_recall, "List<Memory>", "stable"), // №543: the matched memory rows vector (verified server.rs); №668: the element read off builtin_human_recall (server.rs:825-840, make_date_struct "Memory", the SINGLE arm); key/content/importance/created_at/access_count are the persisted memory rows (untrusted), relevance/score are the in-tree scoring (internal); the element's field-labels — stage 2 (the Tool/DayForecast precedent)
    spec!("human_remember", 4, "bot"; builtin_human_remember, "String", "stable"), // №543: the literal "ok" (verified server.rs)
    spec!("human_respond", 2, "bot" => "ext"; builtin_human_respond, "String", "stable"), // №543: the reply text (delegates to human_recall; verified)
    spec!("compress_html", 1, "bot" => "ext"; builtin_compress_html, "String", "stable"), // №543: the compressed HTML (verified)
    spec!("estimate_tokens", 1, "bot" => "ext"; builtin_estimate_tokens, "Float", "stable"), // №543: the token estimate (verified)
    spec!("extract_entities", 1, "bot" => "ext"; builtin_extract_entities, "List", "stable"), // №543: the entity rows vector (verified)
    spec!("extract_param", 2, "bot" => "ext"; builtin_extract_param, "String", "stable"), // text,index — №543: the extracted substring (verified)
    spec!("learn_preference", 3, "bot" => "ext"; builtin_learn_preference, "Struct", "stable"), // №543: the Preference make_date_struct (verified office/human.rs)
    spec!("read_file_tokens", 1, "bot" => "ext"; builtin_read_file_tokens, "Struct<FileInfo>{content:untrusted,chars:untrusted,tokens:untrusted}", "stable"), // №712 (gh#1319, Wave 49): the FIXED 3-field token-accounting shape (text.rs builtin_read_file_tokens → Struct "FileInfo", the SINGLE Ok arm — the bare №543 "Struct" spelling upgraded to the full meta); content is the file body read through the sandbox gate → untrusted (the read_file family ground); chars/tokens are the numeric DERIVATIONS of the untrusted content → untrusted (the №623 label-flow: derivation conducts; the length-oracle conservatism — a length of secret content is itself a side channel, gh#1317 class)
    // ── sqz-inspired: string/list utilities ──
    spec!("squeeze", 2, "string"; builtin_squeeze, "String", "stable"),
    spec!("to_int", 1, "string"; builtin_to_int, "Float", "stable"), // parse string/float to integer
    // ── PDF processing (Наряд №48) ──
    spec!("pdf_classify", 1, "pdf"; builtin_pdf_classify, "List", "stable"), // №543: the pdf.rs make_dict — a List of String keys/values (verified pdf.rs:363-371); №637: stays bare — the flat [String,Float,List,Float] is heterogeneous, no honest element type (№616 honest Unknown)
    spec!("pdf_to_markdown", 1, "pdf"; builtin_pdf_to_markdown, "List", "stable"), // №543: the same make_dict shape (verified); №637: stays bare — the flat mixed-type pairs, no honest element type (№616)
    spec!("pdf_extract_regions", 2, "pdf"; builtin_pdf_extract_regions, "List", "stable"), // №543: the region structs vector (verified); №637: stays bare — a List of flat pseudo-dict lists (heterogeneous key/value pairs), no honest element type (№616)
    spec!("pdf_ocr", 1, "pdf"; builtin_pdf_ocr, "List", "stable"), // №543: the make_dict shape (verified); №637: stays bare — the flat mixed-type pairs, no honest element type (№616) // ── PDF creation & manipulation (Наряд MLG-1; builtin_pdf_ocr) ──
    spec!("pdf_create", 0, "pdf"; builtin_pdf_create, "Struct<PdfDocId>{id:internal}", "stable"), // → { id } — №543: the PdfDocId make_struct (verified); №637: PdfDocId{ id } (pdf.rs:591) — the id computed in-tree (uuid v4) → internal
    spec!("pdf_add_page", 3, "pdf"; builtin_pdf_add_page, "Struct<PdfPage>{page:internal}", "stable"), // id, width, height — №543: the PdfPage make_struct (verified); №637: PdfPage{ page } (pdf.rs:629-632) — the 1-based page count computed in-tree → internal
    spec!("pdf_write_text", 4, 6, "pdf"; builtin_pdf_write_text, "Struct<PdfResult>{ok:internal}", "stable"), // id, x, y, text [,font, size] — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:687) — the constant true → internal
    spec!("pdf_draw_line", 5, 6, "pdf"; builtin_pdf_draw_line, "Struct<PdfResult>{ok:internal}", "stable"), // id, x1, y1, x2, y2 [,width] — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:730)
    spec!("pdf_draw_rect", 5, 7, "pdf"; builtin_pdf_draw_rect, "Struct<PdfResult>{ok:internal}", "stable"), // id, x, y, w, h [,stroke, fill] — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:780)
    spec!("pdf_save", 2, "pdf"; builtin_pdf_save, "Struct<PdfFile>{path:untrusted,size:internal}", "stable"), // id, path — №543: the PdfFile make_struct (verified); №637: PdfFile{ path, size } (pdf.rs:818-821) — path echoes the user argument → untrusted, size computed in-tree → internal
    spec!("pdf_merge", 2, "pdf"; builtin_pdf_merge, "Struct<PdfMerge>{path:untrusted,pages:internal,size:internal}", "stable"), // paths_json, output — №543: the PdfMerge make_struct (verified); №637: PdfMerge{ path, pages, size } both arms (pdf.rs:1989-1994, 2164-2170) — path echoes the output argument, pages/size computed
    spec!("pdf_split", 3, "pdf"; builtin_pdf_split, "Struct<PdfSplit>{files:untrusted,pages:internal}", "stable"), // path, ranges_json, output_dir — №543: the PdfSplit make_struct (verified); №637: PdfSplit{ files, pages } (pdf.rs:2261-2264) — the file paths embed the user-supplied output_dir → untrusted, pages computed
    spec!("pdf_metadata", 1, "pdf"; builtin_pdf_metadata, "Struct<PdfMetadata>{title:untrusted,author:untrusted,subject:untrusted,creator:untrusted,producer:untrusted,pages:untrusted,created:untrusted,modified:untrusted}", "stable"), // path — №543: the PdfMetadata make_struct (verified); №637: PdfMetadata{ 8 fields } (pdf.rs:2345-2356) — every field read off the EXTERNAL PDF body → untrusted
    spec!("pdf_set_metadata", 3, "pdf"; builtin_pdf_set_metadata, "Struct<PdfResult>{ok:internal}", "stable"), // path, key, value — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:2431)
    spec!("html_to_pdf", 2, "pdf"; builtin_html_to_pdf, "Struct<PdfFile>{path:untrusted,size:internal}", "stable"), // html, path — №543: the PdfFile make_struct (verified); №637: PdfFile{ path, size } both arms (pdf.rs:2458-2468) — path echoes the user argument, size computed
    spec!("send_document", 2, 3, "bot" => "ext"; builtin_send_document, "stable"), // chat_id, file_path [,caption] — №543: Unknown honest — String | Unit (no-token fallback), same env-dependent split as send_message
    // ── Crypto: SHA-256 / HMAC (Наряд №50 Block 3) ──
    spec!("sha256", 1, "crypto"; builtin_sha256, "String", "stable"),
    spec!("hmac_sha256", 2, "crypto"; builtin_hmac_sha256, "String", "stable"),
    spec!("hex_encode", 1, "crypto"; builtin_hex_encode, "String", "stable"),
    spec!("hex_decode", 1, "crypto"; builtin_hex_decode, "stable"),
    // Наряд №172: secret() — reads env var as Value::Secret directly
    // (hard-failure if missing, unlike env() which returns empty string).
    spec!("secret", 1, "crypto"; builtin_secret, "stable"), // №712 re-refusal: the opaque Value::Secret (src/builtins/crypto.rs:322-328 — SecretString wrap; the №730 re-verified anchor); the flat vocabulary has no Secret spelling (the №538 Query posture) and typing String would breach the SECRET_LEAK Category-A contract — the Secret spelling is the dictionary lane (the owner decision) // ── Regex (Наряд №54; builtin_hex_decode) ──
    spec!("regex_match", 2, "string"; builtin_regex_match, "Bool", "stable"),
    spec!("regex_captures", 2, "string"; builtin_regex_captures, "List", "stable"),
    spec!("regex_replace", 3, "string"; builtin_regex_replace, "String", "stable"), // ── PDF office automation (Наряд MLG-3; builtin_regex_replace) ──
    spec!("pdf_draw_table", 5, 6, "pdf"; builtin_pdf_draw_table, "Struct<PdfResult>{ok:internal}", "stable"), // id, x, y, col_widths_json, rows_json [,style_json] — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:1385)
    spec!("pdf_add_image", 4, 6, "pdf"; builtin_pdf_add_image, "Struct<PdfResult>{ok:internal}", "stable"), // id, x, y, image_path [,width, height] — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:1442)
    spec!("pdf_set_page_header", 2, 4, "pdf"; builtin_pdf_set_page_header, "Struct<PdfResult>{ok:internal}", "stable"), // id, text [,font, size] — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:1516)
    spec!("pdf_set_page_footer", 2, 4, "pdf"; builtin_pdf_set_page_footer, "Struct<PdfResult>{ok:internal}", "stable"), // id, text [,font, size] — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:1558)
    spec!("pdf_page_numbers", 1, 4, "pdf"; builtin_pdf_page_numbers, "Struct<PdfResult>{ok:internal}", "stable"), // id [,format, x, y] — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:1595)
    spec!("pdf_watermark", 2, 5, "pdf"; builtin_pdf_watermark, "Struct<PdfResult>{ok:internal}", "stable"), // id, text [,font, size, opacity] — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:1645)
    spec!("pdf_fill_form", 3, "pdf"; builtin_pdf_fill_form, "Struct<PdfFillForm>{path:untrusted,fields_filled:internal}", "stable"), // path, fields_json, output_path — №543: the PdfFillForm make_struct (verified); №637: PdfFillForm{ path, fields_filled } (pdf.rs:1737-1744) — path echoes output_path → untrusted, the count computed
    spec!("pdf_rotate_page", 4, "pdf"; builtin_pdf_rotate_page, "Struct<PdfResult>{ok:internal}", "stable"), // path, page_number, degrees, output_path — №543: the PdfResult make_struct (verified); №637: PdfResult{ ok } (pdf.rs:1799)
    spec!("pdf_delete_pages", 3, "pdf"; builtin_pdf_delete_pages, "Struct<PdfDeletePages>{ok:internal,pages_remaining:internal}", "stable"), // path, pages_json, output_path — №543: the PdfDeletePages make_struct (verified); №637: PdfDeletePages{ ok, pages_remaining } (pdf.rs:1852-1855) — both computed in-tree
    spec!("pdf_extract_images", 1, 2, "pdf"; builtin_pdf_extract_images, "List<String>", "stable"), // path [,output_dir] — №543: the extracted paths vector (verified); №637: the String path element read off the handler (extracted_paths.push(Value::String(out_path)), pdf.rs:1938)
    // ── Email: SMTP + IMAP (Наряд MLG-4) ──
    spec!("smtp_send", 3, 6, "email"; builtin_smtp_send, "stable"), // to, subject, body [,attachments_json, from, reply_to]
    spec!("smtp_send_html", 3, 4, "email"; builtin_smtp_send_html, "stable"), // to, subject, html [,attachments_json]
    spec!("imap_list", 2, 3, "email"; builtin_imap_list, "List<ImapMessage>", "stable"), // folder, limit [,since_date] — №650: the element read off imap_list_impl (make_struct "ImapMessage"); every field arrives from the IMAP server (Network ingress) or echoes the caller uid → untrusted. The element's field-labels stay OFF the spelling at stage 0 (the Tool/DayForecast precedent: the {field:label} section is a Struct form — stage 2), recorded here honestly: uid/from/subject/date/seen/size — all untrusted
    // №650 (the shape read off imap_read_impl, make_struct "ImapEmail") +
    // №689 (gh#1256, R-1): the ПДн fields of the mail container are
    // `private` — from/to/subject are the correspondence participants and
    // topic, body_text/body_html are the letter bodies, attachments are
    // the enclosed files; the server metadata (uid = the caller's echo,
    // date/seen = the envelope flags) stays `untrusted`. The marks are
    // the FIRST flow-path input: the №689 bridge (semantic.rs) reads them
    // so imap_read fields into smtp_send/http_post/respond speak the
    // PII_EGRESS_* classes (the RED-first proof:
    // tests/naryad_689_private_field_flow.rs).
    spec!("imap_read", 1, "email"; builtin_imap_read, "Struct<ImapEmail>{uid:untrusted,from:private,to:private,subject:private,date:untrusted,body_text:private,body_html:private,attachments:private,seen:untrusted}", "stable"),
    spec!("imap_search", 2, "email"; builtin_imap_search, "List<ImapMessage>", "stable"), // query, folder — №650: the element read off imap_search_impl (make_struct "ImapMessage", the 4-field arm); all fields from the IMAP server → untrusted (the SAME type_name carries a DIFFERENT field set on the imap_list row — the №638 two-shapes precedent; the element labels — stage 2, the Tool/DayForecast precedent)
    spec!("imap_mark_read", 1, "email"; builtin_imap_mark_read, "stable"),                // uid
    spec!("imap_move", 2, "email"; builtin_imap_move, "stable"), // uid, dest_folder
    // ── Наряд MLG-5: Calendar (CalDAV + iCal) ──
    spec!("cal_connect", 3, "calendar"; builtin_cal_connect, "String", "stable"), // url, user, pass
    spec!("cal_list", 1, "calendar"; builtin_cal_list, "String", "stable"),       // session_id
    // №689: the live return is the JSON array of event structs
    // (calendar.rs, Value::String) — the honest unstructured-string
    // answer with the reason: the CONTENT is ПДн (schedules, attendees,
    // locations), but the stage-0 scalar vocabulary carries no label
    // (a `String<private>` row erases to Unknown — the typed floor
    // regresses), and the structural typing is the stage-2 contour.
    // Nothing here changed silently; see the card_contacts row twin.
    spec!("cal_events", 3, "calendar"; builtin_cal_events, "String", "stable"), // calendar_id, start, end — №650: the live return is Value::String (the JSON array of event structs, calendar.rs) — a scalar stage-0 type
    spec!("cal_read", 1, "calendar"; builtin_cal_read, "String", "stable"), // event_uid — №650: Value::String (the JSON struct, calendar.rs)
    spec!("cal_create", 4, 7, "calendar"; builtin_cal_create, "stable"), // cal_id, summary, start, end [,desc, location, attendees_json]
    spec!("cal_update", 2, "calendar"; builtin_cal_update, "String", "stable"), // event_uid, fields_json
    spec!("cal_delete", 1, "calendar"; builtin_cal_delete, "String", "stable"), // event_uid
    spec!("cal_freebusy", 3, "calendar"; builtin_cal_freebusy, "String", "stable"), // calendar_id, start, end
    spec!("ical_parse", 1, "calendar"; builtin_ical_parse, "stable"),               // text
    spec!("ical_generate", 1, "calendar"; builtin_ical_generate, "String", "stable"), // event_json
    // ── Наряд MLG-6: Contacts (CardDAV + vCard) ──
    spec!("card_connect", 3, "contacts"; builtin_card_connect, "String", "stable"), // url, user, pass
    spec!("card_list", 1, "contacts"; builtin_card_list, "String", "stable"),       // session_id
    // №689: the live return is the JSON array of contact structs
    // (contacts.rs, Value::String) — the honest unstructured-string
    // answer with the reason: the CONTENT is ПДн (names/emails/phones),
    // but the stage-0 scalar vocabulary carries no label (from_path
    // lifts the flat spellings only — a `String<private>` row erases to
    // Unknown and REGRESSES the typed floor), and the structural typing
    // means changing the impl to a Value::Struct return — the stage-2
    // contour, not the first tranche. The ПДн flow check for this row
    // lands with the structural typing (the №688 list, next tranches);
    // nothing here changed silently.
    spec!("card_contacts", 2, "contacts"; builtin_card_contacts, "String", "stable"), // addressbook_id, query — №650: Value::String (the JSON array, contacts.rs)
    spec!("card_read", 1, "contacts"; builtin_card_read, "String", "stable"),         // contact_uid
    spec!("card_create", 3, 7, "contacts"; builtin_card_create, "String", "stable"), // addressbook_id, fn, email [,tel, org, title, note]
    spec!("card_update", 2, "contacts"; builtin_card_update, "String", "stable"), // contact_uid, fields_json
    spec!("card_delete", 1, "contacts"; builtin_card_delete, "String", "stable"), // contact_uid
    spec!("card_search", 2, "contacts"; builtin_card_search, "String", "stable"), // session_id, query — №650: Value::String (the JSON array, contacts.rs)
    spec!("vcard_parse", 1, "contacts"; builtin_vcard_parse, "String", "stable"), // text
    spec!("vcard_generate", 1, "contacts"; builtin_vcard_generate, "String", "stable"), // contact_json
    // ── Наряд №74: Native SVG Graphics & Diagrams (ADR-0102) ──
    // Level 1: SVG primitives — return XML fragments
    #[cfg(feature = "svg")]
    spec!("svg_rect", 5, 6, "svg"; builtin_svg_rect, "String", "stable"), // x, y, w, h, fill [, stroke]
    #[cfg(feature = "svg")]
    spec!("svg_circle", 4, "svg"; builtin_svg_circle, "String", "stable"), // cx, cy, r, fill
    #[cfg(feature = "svg")]
    spec!("svg_line", 5, 6, "svg"; builtin_svg_line, "String", "stable"), // x1, y1, x2, y2, stroke [, width]
    #[cfg(feature = "svg")]
    spec!("svg_text", 5, 6, "svg"; builtin_svg_text, "String", "stable"), // x, y, content, font_size, fill [, anchor]
    #[cfg(feature = "svg")]
    spec!("svg_path", 2, 3, "svg"; builtin_svg_path, "String", "stable"), // d, fill [, stroke]
    #[cfg(feature = "svg")]
    spec!("svg_group", 1, 2, "svg"; builtin_svg_group, "String", "stable"), // children [, transform]
    #[cfg(feature = "svg")]
    spec!("svg_canvas", 4, "svg"; builtin_svg_canvas, "String", "stable"), // width, height, viewbox, children
    // Level 2: design tokens
    #[cfg(feature = "svg")]
    spec!("diagram_style", 1, "tokens"; builtin_diagram_style, "Struct", "stable"), // {paper, ink, accent, muted, rule}
    // Level 2.5: wow-effects
    #[cfg(feature = "svg")]
    spec!("svg_sketchy_filter", 1, 5, "svg"; builtin_svg_sketchy_filter, "String", "stable"), // id [, base_freq, octaves, scale, seed]
    #[cfg(feature = "svg")]
    spec!("svg_icon", 5, "svg"; builtin_svg_icon, "String", "stable"), // name, x, y, size, color
    #[cfg(feature = "svg")]
    spec!("svg_callout", 5, 6, "svg"; builtin_svg_callout, "String", "stable"), // text, from_x, from_y, to_x, to_y [, intent]
    // Level 3: high-level chart types
    #[cfg(feature = "chart")]
    spec!("chart_bar", 2, "chart"; builtin_chart_bar, "String", "stable"), // data, style
    #[cfg(feature = "chart")]
    spec!("chart_donut", 2, "chart"; builtin_chart_donut, "String", "stable"), // data, style — Наряд №77 Block 2
    #[cfg(feature = "chart")]
    spec!("chart_line", 2, "chart"; builtin_chart_line, "String", "stable"), // data, style — Наряд №78 Block 1
    #[cfg(feature = "chart")]
    spec!("chart_scatter", 2, "chart"; builtin_chart_scatter, "String", "stable"), // data, style — Наряд №78 Block 2
    #[cfg(feature = "chart")]
    spec!("chart_area", 2, "chart"; builtin_chart_area, "String", "stable"), // data, style — Наряд №78 Block 3
    #[cfg(feature = "chart")]
    spec!("chart_radar", 2, "chart"; builtin_chart_radar, "String", "stable"), // data, style — Наряд №79 Block 1
    #[cfg(feature = "chart")]
    spec!("chart_heatmap", 2, "chart"; builtin_chart_heatmap, "String", "stable"), // data, style — Наряд №79 Block 2
    #[cfg(feature = "chart")]
    spec!("chart_boxplot", 2, "chart"; builtin_chart_boxplot, "String", "stable"), // data, style — Наряд №79 Block 3
    // Level 2.6: derived palette (Наряд №77 Block 1)
    #[cfg(feature = "svg")]
    spec!("color_palette", 2, "svg"; builtin_color_palette, "stable"), // intent, mode → DiagramStyle
    // Level 2.6/2.7: procedural backgrounds + canvas presets (Наряд №80)
    #[cfg(feature = "svg")]
    spec!("svg_generate", 4, "svg"; builtin_svg_generate, "String", "stable"), // kind, intent, w, h → SVG fragment
    #[cfg(feature = "svg")]
    spec!("svg_canvas_preset", 3, "svg"; builtin_svg_canvas_preset, "stable"), // preset_name, viewbox, children
    // Level 3.1: diagrams (Наряд №81) — hierarchies & flows
    #[cfg(feature = "diagram")]
    spec!("diagram_tree", 2, "diagram"; builtin_diagram_tree, "String", "stable"), // data, style — recursive tree
    #[cfg(feature = "diagram")]
    spec!("diagram_org_chart", 2, "diagram"; builtin_diagram_org_chart, "String", "stable"), // data, style — tree with title field
    #[cfg(feature = "diagram")]
    spec!("diagram_flowchart", 2, "diagram"; builtin_diagram_flowchart, "String", "stable"), // data, style — layered DAG
    #[cfg(feature = "diagram")]
    spec!("diagram_layers", 2, "diagram"; builtin_diagram_layers, "String", "stable"), // data, style — horizontal stripes
    // Level 3.2: diagrams (Наряд №82) — temporal & process
    #[cfg(feature = "diagram")]
    spec!("diagram_sequence", 2, "diagram"; builtin_diagram_sequence, "String", "stable"), // data, style — UML sequence (lifelines + messages; builtin_diagram_sequence)
    #[cfg(feature = "diagram")]
    spec!("diagram_timeline", 2, "diagram"; builtin_diagram_timeline, "String", "stable"), // data, style — horizontal axis with event dots
    #[cfg(feature = "diagram")]
    spec!("diagram_gantt", 2, "diagram"; builtin_diagram_gantt, "String", "stable"), // data, style — horizontal bars per task
    #[cfg(feature = "diagram")]
    spec!("diagram_process", 2, "diagram"; builtin_diagram_process, "String", "stable"), // data, style — linear numbered step chain
    #[cfg(feature = "diagram")]
    spec!("diagram_loop", 2, "diagram"; builtin_diagram_loop, "String", "stable"), // data, style — closed-loop circular steps
    // Level 3.3: diagrams (Наряд №83) — sets & comparisons
    #[cfg(feature = "diagram")]
    spec!("diagram_venn", 2, "diagram"; builtin_diagram_venn, "String", "stable"), // data, style — 2 or 3 overlapping circles
    #[cfg(feature = "diagram")]
    spec!("diagram_quadrant", 2, "diagram"; builtin_diagram_quadrant, "String", "stable"), // data, style — 2x2 strategic quadrant
    #[cfg(feature = "diagram")]
    spec!("diagram_pyramid", 2, "diagram"; builtin_diagram_pyramid, "String", "stable"), // data, style — stacked trapezoids (top=apex; builtin_diagram_pyramid)
    #[cfg(feature = "diagram")]
    spec!("diagram_nested", 2, "diagram"; builtin_diagram_nested, "String", "stable"), // data, style — concentric circles
    #[cfg(feature = "diagram")]
    spec!("diagram_medallion", 2, "diagram"; builtin_diagram_medallion, "String", "stable"), // data, style — row of round badges w/ icons
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
    spec!("diagram_er", 2, "diagram"; builtin_diagram_er, "String", "stable"), // data, style — entity boxes on a grid w/ relations
    #[cfg(feature = "diagram")]
    spec!("diagram_state", 2, "diagram"; builtin_diagram_state, "String", "stable"), // data, style — state machine (cycles OK; builtin_diagram_state)
    #[cfg(feature = "diagram")]
    spec!("diagram_swimlane", 2, "diagram"; builtin_diagram_swimlane, "String", "stable"), // data, style — lanes × steps positioned by order
    #[cfg(feature = "diagram")]
    spec!("diagram_data_flow", 2, "diagram"; builtin_diagram_data_flow, "String", "stable"), // data, style — graph w/ cycles OK
    #[cfg(feature = "diagram")]
    spec!("diagram_high_level", 2, "diagram"; builtin_diagram_high_level, "String", "stable"), // data, style — large bolder blocks, no cycles
    #[cfg(feature = "diagram")]
    spec!("diagram_architecture", 2, "diagram"; builtin_diagram_architecture, "String", "stable"), // data, style — high_level + svg_icon per node
    // ── Наряд №86: Mini template engine ──
    //   template_render(template, data) -> Html
    //   Parses Mustache/Handlebars-like subset: {{ var }} (auto-escaped),
    //   {{{ var }}} (raw), {{#if cond}}...{{else}}...{{/if}},
    //   {{#each items}}...{{/each}}. Returns opaque Value::Html.
    //   INTENTIONALLY NOT in SVG_AUTO_ESCAPE_BUILTINS — the template is
    //   trusted code (written by the .mlog programmer, not user input);
    //   data substitution is escaped at runtime via escape_html_chars.
    #[cfg(feature = "template")]
    spec!("template_render", 2, "template"; builtin_template_render, "stable"), // ── Наряд №88: HTML rendering via headless browser ──
    //   html_render(html, width, height) -> String (path to PNG)
    //   Renders self-contained HTML to a PNG screenshot using
    //   Chromium/Chrome (configured via METALOGOS_BROWSER_BIN env var).
    //   NO shell interpretation — uses exec_restricted internally.
    //   Network isolation: caller's responsibility (self-contained HTML
    //   with data: URIs; external resources NOT blocked at OS level).
    spec!("html_render", 3, "web"; builtin_html_render, "stable"), // ── Наряд №89: Infographic quality assurance ──
    //   infographic_qa(svg_string) -> Struct { passed, warnings, checks_run }
    //   Three mechanical checks: contrast (WCAG), saturation discipline,
    //   element density. Advisory — passed:false means "review", not "broken".
    #[cfg(feature = "diagram")]
    spec!("infographic_qa", 1, "diagram"; builtin_infographic_qa, "stable"),
    // ── Наряд №179b: Reflex training/prediction builtins ──
    // Stub handlers — the real dispatch lives in interpreter::execution::invoke()
    // because reflex_train/reflex_predict need access to ReflexRegistry (which
    // lives on the Interpreter struct). The stubs produce a clean "VM not yet
    // supported" error if the VM backend somehow reaches them directly.
    // When VM gains Reflex support (future naryad), the same dispatch logic
    // in src/builtins/reflex.rs will be reused — see reflex_train_dispatch /
    // reflex_predict_dispatch.
    spec!("reflex_train", 5, "reflex"; builtin_reflex_train_stub, "stable"),
    spec!("reflex_predict", 2, "reflex"; builtin_reflex_predict_stub, "stable"),
    // ── Наряд №180: Reflex persistence (ADR-0116) ──
    // Same pattern as reflex_train/reflex_predict: stub handlers — the
    // real dispatch lives in interpreter::execution::invoke() (and
    // interpreter::reflex_builtin.rs for FnCall expressions) because
    // reflex_save/reflex_load need access to both the ReflexRegistry
    // and the SQLite persist path (set by `memory { persist: "..." }`).
    spec!("reflex_save", 1, "reflex"; builtin_reflex_save_stub, "stable"),
    spec!("reflex_load", 1, "reflex"; builtin_reflex_load_stub, "stable"), // №748 re-refusal (gh#1392, Wave 55): the №717 "loud Err stub" ground was STALE — the shared reflex_load_dispatch precedes the registry row on BOTH lanes (TW execution.rs:556/2031, VM vm.rs:2909-2913) and its Ok arm carries the Reflex handle Value::Reflex(id) (reflex.rs:1113); the Reflex opaque KIND exists in the vocabulary (the №538 family) but a bare opaque is not covered-grade (not scalar, not parameterized — the №740 union rule), so the spelling would be typed-but-coarse WITHOUT closing the row — the movement keeps the Unknown ground (the №744 device_open coarse-refusal form); the stub stays byte-for-byte as the shadowed fallback
    // ── Наряд №187: Reflex introspection (read-only, ADR-0114) ──
    // Same stub pattern. reflex_metrics returns metadata (no weights);
    // reflex_list returns names of all declared reflex/reflex_seq models.
    spec!("reflex_metrics", 1, "reflex"; builtin_reflex_metrics_stub, "stable"),
    spec!("reflex_list", 0, "reflex"; builtin_reflex_list_stub, "List<String>", "stable"), // №748 (gh#1392, Wave 55, lane α): the LIVE path is the shared reflex_list_dispatch — it precedes the registry row on BOTH lanes (TW execution.rs:578/2038, VM vm.rs:2918-2923; the №717 "loud Err stub" anchor reflex.rs:112-117 named only the shadowed fallback arm — the №736 intercept-walk lesson) and answers the List of the declared reflex/reflex_seq model names as Strings (reflex.rs:1220-1241: the names sorted by ReflexId.0 — the registration order, deterministic; the empty registry answers the honest empty List — the vision_list №744 form); the spelling names the ONLY reachable Ok arm (the №539 guard by construction: no other Ok arm exists on either lane); the registry stub stays byte-for-byte as the shadowed fail-closed fallback (the query №740 posture — the type is the INTERCEPT's shape, not the fallback's)
    // ── Наряд №825 (ADR-0200): the attest_verify builtin — поправка (г):
    // tier `experimental` (the №809 C3-a T1 gate), Source × Internal × Pure
    // in builtins_classification.rs. Same intercept shape as the reflex
    // group: the live dispatch (src/builtins/attest.rs) precedes the row
    // on BOTH lanes (TW execution.rs, VM call_reflex_builtin); the stub
    // stays as the shadowed fail-closed fallback (the №748 posture).
    spec!("attest_verify", 1, "reflex"; builtin_attest_verify_stub, "Struct<AttestVerdict>{ok:internal,level:internal,spec_hash:internal,reason:internal}", "experimental"),
    // ── Наряд №193: text generation (ADR-0120) ──
    // №799 (T2, gh#1488): the parked row gates with `candle` (the live Ok
    // arm is candle-gated, №748) — the default build refuses at CHECK via
    // `builtins::parked` naming --features candle.
    #[cfg(feature = "candle")]
    spec!("reflex_generate", 4, "reflex"; builtin_reflex_generate_stub, "parked"), // №748 re-refusal (gh#1392, Wave 55): the №717 stub-only ground was STALE — the shared reflex_generate_dispatch precedes the registry row on BOTH lanes (TW execution.rs:588/2042, VM vm.rs:2924-2928); the Ok arm is the List<Float> of token ids (reflex.rs:1353) but it is CANDLE-GATED (#[cfg(feature = "candle")]) — the default build refuses loudly (reflex.rs:1359 "candle feature not enabled"), so the reachable-Ok ground is FEATURE-DEPENDENT (the reflex_bpe_load never-Ok posture applies to the default binary; the candle build is a different contract) — typing the feature-conditional arm has NO house precedent (the №744 vision intercepts were UNCONDITIONAL) and the row CANNOT close in the default build — the Unknown ground stays; the feature-lane typing verdict is the owner lane; the stub stays byte-for-byte as the shadowed fallback
    // ── Наряд №194: tokenization (ADR-0120 follow-up) ──
    // These are pure functions (no registry access) — real handlers, not stubs.
    spec!("reflex_tokenize", 1, "reflex"; builtin_reflex_tokenize, "stable"),
    spec!("reflex_detokenize", 1, "reflex"; builtin_reflex_detokenize, "String", "stable"),
    // ── Наряд №195: BPE tokenization ──
    spec!("reflex_bpe_train", 2, "reflex"; builtin_reflex_bpe_train, "stable"),
    spec!("reflex_bpe_encode", 2, "reflex"; builtin_reflex_bpe_encode, "stable"),
    spec!("reflex_bpe_decode", 2, "reflex"; builtin_reflex_bpe_decode, "String", "stable"), // №757 precise movement (N6, gh#1216): the compensation package — the verified single-shape handler fact (the ONLY Ok arm is Value::String(text))
    spec!("reflex_bpe_save", 1, "reflex"; builtin_reflex_bpe_save, "Unit", "stable"),
    spec!("reflex_bpe_load", 1, "reflex"; builtin_reflex_bpe_load, "parked"), // №717: the handler NEVER returns Ok — the loud Err on every lane (reflex.rs:1529-1531, the №195 contract 4 note); typing an unreachable Ok shape would be fictitious (the stdin true-stub posture, the louder form)
    // ── Vision pillar (Наряд №210, ADR-0124) ──
    // Наряд №240 (R4.2): vision_generate arity 3→2 (R4 contract, plan §3:
    // `vision_generate("decl_name", "prompt")` — model/steps/size/seed come
    // from the `vision { }` declaration; the R1 stub doc "(model_name,
    // prompt, seed)" predates the declaration language and was never the
    // contract). generate/list/export are intercepted before the generic
    // fallback (лекало reflex_train); these specs remain the last-resort
    // handlers + the arity/type contract for LSP and checks.
    // №799 (T2, gh#1488): the parked vision rows gate with `vision` — the
    // default build refuses at CHECK via `builtins::parked` naming
    // --features vision (the lie "undefined" is gone; №796 tiers).
    #[cfg(feature = "vision")]
    spec!("vision_generate", 2, "vision"; builtin_vision_generate_stub, "parked"),
    #[cfg(feature = "vision")]
    spec!("vision_edit", 2, "vision"; builtin_vision_edit_stub, "parked"),
    #[cfg(feature = "vision")]
    spec!("vision_export", 2, "vision"; builtin_vision_export_stub, "parked"),
    // Наряд №241 (R5, Block 2.1 — ADR-0125): explicit raw opt-out. Real
    // path intercepted like vision_export (state-carrying). Every call
    // site is audit-flagged VISION_UNSIGNED_EXPORT_RAW (Warning).
    #[cfg(feature = "vision")]
    spec!("vision_export_raw", 2, "vision"; builtin_vision_export_raw_stub, "parked"),
    // Наряд №241 (R5, Block 3.2 — ADR-0125 MODEL_WEIGHTS_UNSAFE): REAL
    // stateless handler (no interception needed — no registry state):
    // SSRF-guarded (check_url_ssrf, лекало №130), allowlist default-deny
    // (MLOG_VISION_WEIGHTS_ALLOWLIST), manifest.json-class only, SHA-256
    // pinned via reused WeightsManifest. Registry 388→389.
    #[cfg(feature = "vision")]
    spec!("vision_fetch_weights", 2, "vision"; builtin_vision_fetch_weights, "String", "parked"), // №717 (gh#1329, Wave 50): the SINGLE Ok arm returns the caller's dest_dir echo (vision.rs:1524, the vision-gated arm; the non-vision arm is the loud refusal) — the verified single-shape String fact (the №757 decrypt form); the allowlist default-deny + SSRF guard + pickle-extension refusals all stay in the error channel (loud, unchanged)
    spec!("vision_list", 0, "vision"; builtin_vision_list_stub, "List<String>", "stable"), // №744 (gh#1385, Wave 54, lane α): the LIVE path is the №240 intercept — it precedes the registry dispatch on BOTH lanes (TW execution.rs:599-604/2061-2066, VM vm.rs:2955-2956) and answers the List of `[Vision#N]` String display handles (vision.rs:362, one per registry id, sorted — determinism; the doc contract vision.rs:351 `vision_list() -> List<String>`); the spelling names the live both-lane contract (the query №740 posture: the row KEEPS its handler — the unreachable loud-Err fallback vision.rs:1706-1712 stays byte-for-byte, the №706 fail-closed form; the type is the INTERCEPT's shape, not the fallback's); the empty registry answers the honest empty List (no element instance — nothing fictitious named: the inner is the plain String the dispatch constructs)
    // Наряд №242 (R6.1): vision_save/vision_load — real SQLite persistence
    // (crate::vision::store), intercepted like the rest of the vision
    // family (state-carrying PLUS the program's db connection). These
    // specs remain the last-resort handlers + the arity contract. Registry
    // count UNCHANGED (389): both builtins existed here since №210 — this
    // naryad replaces their stub bodies with real dispatch paths.
    spec!("vision_save", 2, "vision"; builtin_vision_save_stub, "stable"),
    spec!("vision_load", 1, "vision"; builtin_vision_load_stub, "stable"), // №717: the state-carrying vision builtin — the registry fallback is the LOUD Err (vision.rs:1756-1763, the №242 dispatch note); typing the unreachable fallback would be fictitious (the №706 fail-closed fallback form)
    // Наряд №244 (R6.3): LoRA adapters — SQLite BLOB persistence
    // (ADR-0124 §6) + application to the DiT attention projections.
    // Intercepted like the rest of the vision family (state-carrying: the
    // program's db connection, plus decls + registry for generate). These
    // specs remain the last-resort handlers + the arity contract. Registry
    // 389→391: the vision family reaches its ADR-0124 §3 ceiling of
    // ~10 (8→10) — the NEXT vision builtin requires an ADR-0124 edit
    // (loudly noted in CHANGELOG).
    #[cfg(feature = "vision")]
    spec!("vision_lora_load", 2, "vision"; builtin_vision_lora_load_stub, "String", "parked"), // №744 (gh#1385, Wave 54, lane α): the LIVE path is the №244 intercept — both lanes (TW execution.rs:813-818/2275-2280, VM vm.rs:3008-3009), the dispatch receives the program db connection (the adapter's only home — ADR-0124 §6); the SINGLE Ok arm returns the adapter name as String (vision.rs:1097 `Ok(Value::String(name))`) — the verified single-shape fact (the №757 decrypt form); the missing-adapter/wrong-handle/db refusals stay LOUD in the error channel; the registry stub stays the unreachable loud-Err fallback (the query №740 posture — the type is the INTERCEPT's shape, not the fallback's)
    #[cfg(feature = "vision")]
    spec!("vision_lora_generate", 3, "vision"; builtin_vision_lora_generate_stub, "parked"),
    // ── Наряд №331 (ADR-0162): unified media layer (category "media") ──
    // Four per-type store builtins (opaque handles — bytes never enter
    // Value), the sanctioned materialization SINK (media_save), the
    // refcount pair, and the metadata observer. All state-carrying:
    // interpreter and VM intercept these names BEFORE the generic
    // fallback and route through src/builtins/media.rs dispatches; these
    // specs remain the last-resort handlers + the arity/type contract.
    // Registry 421→429. media_save egress is gated by №325 (file kind)
    // + the runtime backstop (MEDIA_SEALED_EGRESS).
    spec!("media_store_image", 2, "media"; builtin_media_store_image_stub, "stable"),
    spec!("media_store_audio", 2, "media"; builtin_media_store_audio_stub, "stable"),
    spec!("media_store_video_frame", 2, "media"; builtin_media_store_video_frame_stub, "stable"),
    spec!("media_store_video_segment", 2, "media"; builtin_media_store_video_segment_stub, "stable"),
    spec!("media_save", 2, 3, "media"; builtin_media_save_stub, "stable"),
    spec!("media_retain", 1, "media"; builtin_media_retain_stub, "stable"),
    spec!("media_release", 1, "media"; builtin_media_release_stub, "stable"),
    spec!("media_meta", 1, "media"; builtin_media_meta_stub, "stable"),
    // ── Наряд №332 (ADR-0164): perception origin — HandleSource/ProvBind ──
    // Both are state-carrying (origin declarations + media store) and are
    // intercepted like the media family; the compiler lowers
    // `source <origin>` / `from <origin> <construction>` to these calls.
    // Registry 430→432.
    spec!("media_source_capture", 1, "media"; builtin_media_source_capture_stub, "stable"), // №717: the state-carrying media builtin — the registry fallback is the LOUD Err (media.rs:635-637, ADR-0164); the real path is the interpreter/VM dispatch contour — typing the unreachable fallback shape would be fictitious (the №706 fail-closed fallback form)
    spec!("media_bind_origin", 2, "media"; builtin_media_bind_origin_stub, "stable"),
    // ── Наряд №333 (ADR-0163): backend registry — read-only metadata ──
    // The SSOT table lives in src/backends.rs; this builtin is the
    // language surface (name/class/weights_id/pin/license/license_note).
    // Registry 429→430; category "registry" (41st module).
    spec!("backend_list", 0, "registry"; builtin_backend_list, "stable"),
    // ── Наряд №336 (ADR-0165): BackendSelect — the backend try-chain ──
    // Priority ladder over the №333 registry SSOT; exhaustion →
    // Degraded(t), the typed degradation result (never a panic, never a
    // silent mock). Registry 439→440; category "registry".
    spec!("backend_select", 2, "registry"; builtin_backend_select, "stable"), // №717 re-refusal: the HETEROGENEOUS Ok union — the selected arm Struct "BackendSelected" (backends.rs:225) vs the exhaustion arm Struct "Degraded" (backends.rs:262); no single return spelling exists (the №543 send_message form); the typed Degraded error record is the semantics lane, OUTSIDE the types movement
    // ── Наряд №337 (ADR-0166): the C2PA contour of media handles ──
    // media_manifest is STATE-CARRYING (store read) — intercepted by
    // interpreter/VM before the generic fallback; media_manifest_read is
    // stateless (sandboxed sidecar read). Registry 440→442.
    spec!("media_manifest", 1, "media"; builtin_media_manifest_stub, "stable"),
    spec!("media_manifest_read", 1, "media"; builtin_media_manifest_read, "Struct<MediaManifest>{kind:untrusted,origin:untrusted,conf:untrusted,synthetic:untrusted,bytes_sha256:untrusted,timestamp:untrusted}", "stable"), // №706 (gh#1306, Wave 48): the FIXED sidecar shape (media.rs → Value::Struct type_name "MediaManifest", the 6 provenance fields, the SINGLE Ok arm); every field reads verbatim off the user-sidecar document → untrusted (the xml_parse document-derivation form); missing/corrupt sidecars stay LOUD errors (№320), never defaults
    // ── Наряд №284 (P1, M1): canary-токены недоверенного текста ──
    // Runtime-детектор утечки недоверенного контента через LLM-канал
    // (паттерн rebuff/Spotlighting). Связан с taint-моделью: утечка →
    // runtime warning CANARY_LEAK + llm_usage().canary_leaks (canary.rs);
    // статическая метка «компрометированный канал» в ветке утечки →
    // audit-warning CANARY_LEAK (src/audit.rs, check_canary_leak,
    // advisory-слой audit_program). Детектор, НЕ гейт. Чистые обработчики
    // (без интерсепшена) — execution.rs не требуется. Registry 399→401;
    // новая категория "security" (37→38 модулей).
    spec!("canary_insert", 1, 2, "security"; builtin_canary_insert, "stable"),
    // №661 (Волна 39, the К-Б stage-2 verdict gh#1144): the FIRST private
    // field label — the canary surface IS the credential-trap surface; the
    // signature mirrors the real handler form (the CanaryCheck struct:
    // leaked Bool / id String / position Float — the canary.rs tail).
    // `id` = the canary-credential reference — private (the gated egress
    // class); leaked/position = the check outcome — internal.
    spec!("canary_check", 2, 3, "security"; builtin_canary_check, "Struct<CanaryCheck>{leaked:internal,id:private,position:internal}", "stable"),
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
    spec!("json_validate", 2, 3, "llm"; builtin_json_validate, "stable"),
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
    spec!("memory_forget", 5, 7, "memory"; builtin_memory_forget, "Struct", "stable"), // №539: the store-lane dry-run/apply result (MemoryForgetResult)
    // ── Наряд №281 (P2, M2): user_profile — детерминированная выжимка
    // контейнера одним вызовом (supermemory user-profiles): static /
    // dynamic / buckets из KV-записей container:<c>:<bucket>:<key>
    // (источник — memorize/kv_set с memory{persist}); БЕЗ LLM-вызова
    // (LLM-синтез — вне скоупа Tier-1, громко); ин-процессный кэш с
    // генерационной (kv-записи) + mtime (внешние записи) инвалидацией.
    // Контейнер-префикс — жёсткая изоляция (cross-container физически
    // не виден); scope-параметр — на vec_store/vec_search. Не
    // feature-гейт: kv-контур ядровой. Registry 403→404 (append-only).
    spec!("user_profile", 2, "memory"; builtin_user_profile, "Struct<UserProfile>{container:untrusted,count:internal,static:private,dynamic:private,buckets:private}", "stable"), // №712 (gh#1319, Wave 49): the FIXED 5-field profile shape (profile.rs user_profile_core → struct_value "UserProfile", the SINGLE Ok arm — the fresh-build and cache-hit paths return the SAME constructed value; the bare №539 "Struct" spelling upgraded to the full meta); container echoes the caller's container name → untrusted (the №706 echo form); count is the derived record count → internal (the cron_list run_count form); static/dynamic/buckets carry the user-authored KV profile content → private (the №689 mail form — the fail-closed egress posture; the nested ProfileRecord/ProfileBuckets shapes are the comment-level detail, the slots are what the meta labels)
    // ── Наряд №285 (P2, feature/memory): text_chunk — структура-осознанное
    // чанкование для RAG-пайплайна (первая стадия поверх №272 vec-контур):
    // strategies markdown|paragraph|fixed; opts{max_chars, overlap,
    // max_tokens?}; markdown-секции несут header_path ("H1 > H2 > H3") —
    // готовые метаданные для vec_store. Каскад «заголовок → абзац →
    // перенос → пробел» + жадное слияние мелких + overlap при окнировании
    // (RecursiveCharacterTextSplitter-дух, без зависимостей). Token-бюджет —
    // реюз token_count (memory.rs token_count_estimate). Registry 404→405
    // (append-only).
    spec!("text_chunk", 2, 3, "string"; builtin_text_chunk, "List", "stable"), // text, strategy | +opts{max_chars, overlap, max_tokens}
    // ── Наряд №275 (P1, feature/llm): LLM streaming — итераторный
    // стиль над SmartRouter (ADR-0137). Opaque handle Value::LlmStream,
    // registry crate::llm::LLM_STREAM_REGISTRY (bounded №263). Trace —
    // одна строка на завершённый стрим (ADR-0138 §D4). Mock / non-SSE →
    // loud STREAM_UNSUPPORTED. Registry 405→408 (append-only).
    #[cfg(feature = "llm")]
    spec!("llm_stream_open", 1, 2, "llm"; builtin_llm_stream_open, "Struct<LlmStream>{handle:internal,model:internal,provider:internal}", "stable"), // prompt | prompt,input — №717 (gh#1329, Wave 50): the FIXED 3-field open-projection (llm_stream.rs:59, the SINGLE Ok arm); handle carries the in-tree LlmStream id → internal; model/provider peek the CONFIGURED provider registry (llm_stream.rs:52-53, the "unknown" fallback) → internal; the stream CONTENT is not in the projection — next/close read the handle
    #[cfg(feature = "llm")]
    spec!("llm_stream_next", 1, "llm"; builtin_llm_stream_next, "String", "stable"), // handle
    #[cfg(feature = "llm")]
    spec!("llm_stream_close", 1, "llm"; builtin_llm_stream_close, "stable"), // handle
    // ── Наряд №283 (P2, feature): path-параметры роутов mlogserver —
    // шаблонный диспетчер {name} / {*path} (axum 0.8.9 syntax). Builtin
    // stub — real body in interpreter/vm FnCall dispatch (needs access
    // to server_path_params HashMap). Static routes win over templates;
    // conflict of two templates matching the same path → loud error at
    // server start. Registry 408→409 (append-only).
    spec!("server_path_param", 1, "web"; builtin_server_path_param, "String", "stable"), // name
    // ── Наряд №302 (P2, feature/voice): Voice pillar skeleton builtins —
    // stubs. Loud errors, no silent fallbacks. Real implementation in
    // phases A2/A3/A4/A5/A6. Feature-gated under `voice` (implies candle).
    // Registry 409→415 (append-only, bytecode indices stable).
    #[cfg(feature = "voice")]
    spec!("voice_enroll", 2, 3, "voice"; builtin_voice_enroll_stub, "parked"), // decl, audio | +kind
    #[cfg(feature = "voice")]
    spec!("tts_speak", 2, "voice"; builtin_tts_speak_stub, "parked"), // decl, text
    #[cfg(feature = "voice")]
    spec!("audio_export", 1, "voice"; builtin_audio_export_stub, "parked"), // handle
    #[cfg(feature = "voice")]
    spec!("voice_design", 2, "voice"; builtin_voice_design_stub, "parked"), // text, voice
    #[cfg(feature = "voice")]
    spec!("voice_save", 2, "voice"; builtin_voice_save_stub, "parked"), // handle, name
    #[cfg(feature = "voice")]
    spec!("voice_load", 1, "voice"; builtin_voice_load_stub, "parked"), // name — №717: not implemented — the loud Err (voice/mod.rs:409-410, phase A5 / ADR-0143); typing a refused shape would be fictitious (the №706 posture)
    // ── Наряд №307/№309 (P2, feature/video): Video pillar builtins.
    // №309 (ADR-0151): real implementations on the tiny seeded pipeline —
    // I2V first/last anchors, RIFE-class interp, extension, AV sidecar mux,
    // signed-by-construction export. video_render: T2V (2) / I2V (3) /
    // two-anchor first–last (4). MODEL_WEIGHTS_UNSAFE covers
    // video_fetch_weights via suffix convention (Наряд №300).
    // Registry 420→421 (append-only).
    #[cfg(feature = "video")]
    spec!("video_render", 2, 4, "video"; builtin_video_render, "parked"),
    #[cfg(feature = "video")]
    spec!("video_export", 2, "video"; builtin_video_export, "stable"),
    #[cfg(feature = "video")]
    spec!("av_mux", 2, "video"; builtin_av_mux, "stable"),
    #[cfg(feature = "video")]
    spec!("frame_interp", 2, "video"; builtin_frame_interp, "parked"),
    #[cfg(feature = "video")]
    spec!("video_extend", 2, "video"; builtin_video_extend, "parked"),
    #[cfg(feature = "video")]
    spec!("video_fetch_weights", 2, "video"; builtin_video_fetch_weights_stub, "parked"), // №717: the formal No-Go loud Err (video/mod.rs:692-697, the №294 class / ADR-0151 D7 — production video weights parked in this environment); typing a refused shape would be fictitious (the №706 posture)
    // №408 (wave 4.5): the comprehension side of the video pillar —
    // mock-first backend call (the understanding twin of №309's
    // generation pipeline), behind the same feature gate.
    #[cfg(feature = "video")]
    spec!("video_understand", 1, 3, "video"; builtin_video_understand, "String", "stable"), // №706 (gh#1306, Wave 48): the success-path answer is a String (understand.rs — the deterministic mock golden "[MOCK: video_understand | ...]"; the omni_ask №697 posture: the real path without weights is a LOUD Err, never a wrong type — PARKED №294)
    // ── Наряд №334 (P0, feature/backends): real STT/omni/vision-
    // understanding backends — the SHA-pin path. Mock-first call surface
    // over the №333 registry: METALOGOS_MOCK_LLM opt-in = deterministic
    // mock (the golden contract); real mode refuses LOUDLY unless the
    // SHA-verified weights are on disk (PARKED №294 — no inference is
    // promised). Handlers: voice::backend (stt/omni), vision::understand.
    // Registry 432→435 (append-only, bytecode indices stable).
    spec!("stt_transcribe", 1, 2, "voice"; builtin_stt_transcribe, "String", "stable"),
    spec!("omni_ask", 1, 3, "voice"; builtin_omni_ask, "String", "stable"),
    spec!("vision_understand", 1, 3, "vision"; builtin_vision_understand, "String", "stable"),
    // ── Наряд №335 (spec §7.2 v2): consent grant/revoke + quarantine ──
    // The consent component's surface (redact precedent: policy as
    // value, builtins not AST). grant/revoke record the ledger and pass
    // the value; the STATIC label rules live in semantic.rs label_source
    // (grant extends the consent scope, revoke = quarantine label — the
    // flat cascade via lattice absorption). quarantine_write is the ONLY
    // legal egress for poisoned values (audit event). Ledger export is
    // file egress, audited. Registry 435→439 (append-only).
    spec!("consent_grant", 2, 4, "security"; builtin_consent_grant, "stable"),
    spec!("consent_revoke", 1, 2, "security"; builtin_consent_revoke, "stable"),
    spec!("quarantine_write", 1, 2, "security"; builtin_quarantine_write, "String", "stable"),
    spec!("consent_ledger_export", 1, "security"; builtin_consent_ledger_export, "String", "stable"),
    // ── Naryad #390 (ADR-0155): Grant algebra — capabilities for ──
    // irreversible actions (wave 3, dispatch #491). The ungranted
    // destructive-SQL deny (IRREVERSIBLE_NO_GRANT, №325) is UNCHANGED —
    // db_execute_with_grant is the additional allowing path, gated by
    // the grant ledger (state/TTL/scope/quota) at runtime and by the
    // Once-linearity check (GRANT_REUSED) at compile time. Registry
    // 442→447 (append-only; bytecode indices must not shift).
    spec!("grant_issue", 2, 4, "action"; builtin_grant_issue, "stable"),
    spec!("grant_subgrant", 3, 5, "action"; builtin_grant_subgrant, "stable"),
    spec!("grant_revoke", 1, "action"; builtin_grant_revoke, "Float", "stable"),
    spec!("grant_use", 1, "action"; builtin_grant_use, "Float", "stable"),
    spec!("db_execute_with_grant", 2, 3, "action"; builtin_db_execute_with_grant, "stable"),
    // ── Naryad #392: the DenyEvent surface ──────────────────────────────
    // Handler-scoped, intercepted by NAME in BOTH backends (the
    // interpreter's invoke() and the VM's call_builtin). Registry stubs
    // exist so the compiler resolves the calls; real dispatch is
    // backend-side (the event is runtime state). The analyzer blocks
    // usage outside an on_deny handler at compile time; the runtime
    // refuses when no event is live. APPENDED at the end — inserting
    // mid-array would shift existing CallBuiltin indices (.mbc contract).
    spec!("deny_event", 0, "stub"; intercept, "Struct<DenyEvent>{reason:internal,sink:internal,class:internal,argument:untrusted,label:internal,line:internal,human:untrusted}", "stable"), // №748 (gh#1392, Wave 55, lane α): the handler IS LIVE on both lanes (the №392 on_deny intercepts — TW execution.rs:2318-2328, VM vm.rs:2395-2412; the №717 "planned wrapper, no handler on any lane" anchor was factually STALE) — the shared deny_event_or_reason returns the LIVE deny event (deny.rs make_event — hooks.rs:77/interpreter, the VM twin current_deny_event): the FIXED 7-field Struct "DenyEvent" (deny.rs:131-142 — reason/sink/class/argument/label/line/human; the fields labeled — the listen_start №717 fixed-projection form; the argument echoes the refused call's argument and the human message embeds it → untrusted (the №706 device_state echo form), the rest are the runtime's own words/numbers → internal); OUTSIDE an on_deny body the call refuses LOUDLY on BOTH lanes (DENY_HANDLER_ERR — audit_ops.rs:44-45, the №758 form) — the error channel, not a shape (the query №740 posture: the context-less refusal does not make a union)
    spec!("deny_reason", 0, "stub"; intercept, "String", "stable"), // №748 (gh#1392, Wave 55, lane α): the handler IS LIVE on both lanes (the №392 on_deny intercepts — TW execution.rs:2318-2328, VM vm.rs:2395-2412; the №717 anchor was factually STALE) — deny_event_or_reason reads the reason field off the live DenyEvent (audit_ops.rs:58-68: fields.get("reason") — make_event ALWAYS inserts it, deny.rs:132, so the defensive unwrap_or(Unit) arm is unreachable-by-construction — the shadowed-fallback form); the single reachable Ok arm is the reason String (the №757 decrypt form); outside an on_deny body the refusal stays LOUD on both lanes (DENY_HANDLER_ERR)
    // ── Naryad #393 (ADR-0167): Action Ledger v1 surface ────────────────
    // Introspection reads (count/head — not egress) and the two FILE
    // EGRESS exports (classified Sink, the consent_ledger_export
    // precedent) plus the rotation/snapshot writers. The journal writes
    // for grant/deny/irreversible/session events live in the ACTION
    // paths themselves (ADR-0167 §3.4) — this surface never becomes a
    // forgotten "also log it" API. APPENDED at the end — inserting
    // mid-array would shift existing CallBuiltin indices (.mbc contract).
    // Registry 451→457 (append-only).
    spec!("ledger_count", 0, "security"; builtin_ledger_count, "Float", "stable"),
    spec!("ledger_head", 0, "security"; builtin_ledger_head, "String", "stable"),
    spec!("ledger_export", 1, "security"; builtin_ledger_export, "stable"),
    spec!("ledger_export_intoto", 1, "security"; builtin_ledger_export_intoto, "stable"),
    spec!("ledger_rotate", 0, "security"; builtin_ledger_rotate, "String", "stable"),
    spec!("ledger_snapshot", 0, "security"; builtin_ledger_snapshot, "String", "stable"),
    // ── Naryad #387 (ADR-0149 D1/D6): the likeness consent ritual ──
    // The one-time challenge + the opaque LikenessToken. APPENDED at
    // the end — inserting mid-array would shift existing CallBuiltin
    // indices (.mbc contract). Registry 457→459 (append-only).
    spec!("likeness_challenge", 1, 3, "security"; builtin_likeness_challenge, "stable"),
    spec!("likeness_verify", 1, 3, "security"; builtin_likeness_verify, "stable"),
    // ── Naryad #407 (wave 4.5): the OCR class — text extraction from an
    // image. Donor contract = №334's vision_understand (mock-first,
    // deterministic golden path; real mode refuses loudly naming the
    // weights artifact — PARKED №294). Handler: vision::ocr. APPENDED
    // at the end — inserting mid-array would shift existing CallBuiltin
    // indices (.mbc contract). Registry 458→459 (append-only).
    spec!("ocr_extract", 1, 3, "vision"; builtin_ocr_extract, "String", "stable"),
    // ── Naryad #415 (P1, security/ledger): the runtime verify hook —
    // read-only structural verification of an exported chain (the audit
    // P2-2 residue). APPENDED at the end — inserting mid-array would
    // shift existing CallBuiltin indices (.mbc contract).
    // Registry 459→460 (append-only).
    spec!("ledger_verify", 1, "security"; builtin_ledger_verify, "Struct<LedgerVerdict>{ok:internal,records:internal,head_hash:internal,distinct_keys:internal,anchored_start:internal,error_record:internal,error_reason:internal}", "stable"), // №706 (gh#1306, Wave 48): the FIXED verdict shape (ledger.rs verdict_to_value — BOTH arms, the soft-missing read and the file verify, carry the same 7 fields); every field is the in-tree verdict computation → internal; error_record is the OPTIONAL Float|Unit field (the memory_revise precedent)
    // ── Naryad #440 (P1, feature/forecast): the forecasting domain ──
    // series_make/series_pull/forecast_next/forecast_state/
    // forecast_points over the `timeseries` registry class; the taint
    // transfer + the gated export live in src/forecast.rs. APPENDED at
    // the end — inserting mid-array would shift existing CallBuiltin
    // indices (.mbc contract). Registry 460→465 (append-only).
    spec!("series_make", 1, 2, "forecast"; builtin_series_make, "experimental"), // №717 re-refusal: the OPAQUE Value::SeriesHandle (forecast.rs:402) — no Series spelling in the stage-0 vocabulary (the №669 forecast_next form: the OpaqueKind set has no Series KIND in the signature vocabulary either)
    spec!("series_pull", 2, "forecast"; builtin_series_pull, "experimental"), // №717: the handler NEVER returns Ok — the grant verifies, then the LOUD Err (forecast.rs:451-457, the documented dispatcher gap: no external series backend in-tree, Устав §11 Шаг 3); typing an unreachable Ok shape would be fictitious (the №717 reflex_bpe_load form)
    spec!("forecast_next", 2, "forecast"; builtin_forecast_next, "experimental"), // handle,horizon — №669: the HONEST refusal (the №650 form): the value IS the opaque ForecastHandle (src/forecast.rs:736-743, Ok(Value::ForecastHandle(forecast))); the stage-0 OpaqueKind set {Secret, Reflex, Grant, Series, Media, Embodied} has NO Forecast kind, and spelling it Series/Embodied would be a fictitious type (the sig_types discipline: Unknown is the honest answer) — the row stays untyped; a Forecast opaque kind is the dictionary/vocabulary lane (the separate owner decision)
    spec!("forecast_state", 1, "forecast"; builtin_forecast_state, "Struct<ForecastState>{id:internal,series_id:internal,horizon:untrusted,rung:internal,pin:internal,degraded:internal,window_hash:internal,label:untrusted,rung_note:internal,skipped:internal}", "experimental"), // handle — №669: the shape read off forecast_state (src/forecast.rs:777-802, the SINGLE arm, Value::Struct "ForecastState"); the prov block is metadata (the device_state precedent — digests and words, no data): id/series_id are the in-tree fresh_id (internal), rung/pin/degraded/window_hash/rung_note/skipped are the in-tree run audit (internal), horizon and label echo the caller's forecast/series arguments (untrusted)
    spec!("forecast_points", 1, "forecast"; builtin_forecast_points, "Struct<ForecastPoints>{points:internal,p10:internal,p50:internal,p90:internal}", "experimental"), // handle — №669: the shape read off forecast_points (src/forecast.rs:869-880, the SINGLE arm, Value::Struct "ForecastPoints"); all four fields are the forecast record's quantile vectors (the List<Float> lists) — the in-tree ladder computation → internal; the taint gate (FORECAST_TAINTED, the export check) is UNCHANGED — the labels describe the field origin, not the egress posture
    // ── Naryad #445 (P1, feature/memory): the canon retain(memory, ttl)
    // — the typed entry's lifetime; past the deadline the sweep
    // auto-forgets it (the №280 "v2" deferral lifted). APPENDED at the
    // end — inserting mid-array would shift existing CallBuiltin
    // indices (.mbc contract). Registry 499→500 (append-only).
    spec!("memory_retain_ttl", 3, "memory"; builtin_memory_retain_ttl, "Unit", "stable"),
    // ── №757 (P1, llm/hardening): the mlog-visible truncation probe —
    // the finish_reason/stop_reason of the last completed call/stream
    // ("" = none reported yet). The non-silent half of №757: the caller
    // checks `llm_last_finish_reason() == "length"` after call_llm.
    // Typed "String" (№467 vocabulary) — the typed-share floor rises
    // with this row. APPENDED at the end — inserting mid-array would
    // shift existing CallBuiltin indices (.mbc contract).
    // Registry 500→501 (append-only).
    #[cfg(feature = "llm")]
    spec!("llm_last_finish_reason", 0, "llm"; builtin_llm_last_finish_reason, "String", "stable"),
    // ── №507 (P2, hardening): the EXPLICIT-silence twin of read_file —
    // the `_or` suffix symmetry with env_or (№481): the MISSING file
    // yields the caller's default (announced on the audit stderr),
    // every loud branch (sandbox, deny-list, open/read) stays shared
    // with read_file. The read_file №254 contract is unchanged.
    // APPENDED at the end — inserting mid-array would shift existing
    // CallBuiltin indices (.mbc contract). Registry 501→502 (append-only).
    spec!("read_file_or", 2, "io"; builtin_read_file_or, "String", "stable"),
    // ── №514 (P1, hardening; the audit 28.09 C-10): the EXPLICIT-silence
    // twins of to_float/to_int — the `_or` suffix carries the silent-default
    // semantics IN THE NAME (the №481 env/env_or naming rule, ONE soft-
    // failure rule for the language). The fallback firing is announced on
    // the audit stderr ([TO_FLOAT_OR]/[TO_INT_OR]); the loud branches of
    // to_float/to_int stay shared. Bool→1.0/0.0 is a conversion, not a
    // soft failure — kept in both twins. APPENDED at the end — inserting
    // mid-array would shift existing CallBuiltin indices (.mbc contract).
    // Registry 503→505 (append-only).
    spec!("to_float_or", 2, "convert"; builtin_to_float_or, "Float", "stable"),
    spec!("to_int_or", 2, "string"; builtin_to_int_or, "Float", "stable"),
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
    // №799 (T2, gh#1488): the parked rows gate with `voice` together
    // with the №526 erasure pair (voice/mod.rs) — a build without
    // `voice` collects no prints (enroll is the same parked family), so
    // the erasure right holds by construction; the voice build carries
    // the full collect+erase pair. The check refusal names --features
    // voice via `builtins::parked`.
    #[cfg(feature = "voice")]
    spec!("voice_delete", 1, "voice"; builtin_voice_delete, "parked"), // voice|audio handle
    #[cfg(feature = "voice")]
    spec!("voice_list", 0, "voice"; builtin_voice_list, "String", "parked"),
    // ── №591 (Волна 30, Камертон Н1-05/Н1-07, инвариант 3): the spectral
    // contour — the Lomb–Scargle periodogram for unevenly sampled series
    // (gapped observations are the norm; FFT needs a uniform grid) and the
    // dominant-frequency readout with the false-alarm p-value. Typed
    // returns, pure functions, loud degradation ([SPECTRAL_INPUT] stamp +
    // the typed Degraded struct). APPENDED at the end — inserting
    // mid-array would shift existing CallBuiltin indices (.mbc contract).
    // Registry 509→511 (append-only).
    spec!("lomb_scargle", 2, "math"; builtin_lomb_scargle, "Struct", "stable"),
    spec!("spectral_peak", 1, "math"; builtin_spectral_peak, "Struct", "stable"),
    // ── №595 (Волна 30, Камертон Н1-05/Н1-07): the UTC calendar arithmetic —
    // the ISO parse/format pair, the signed fractional day difference and
    // the UTC-facing epoch read. ALL four typed PRECISE (Float/Float/
    // Float/String — the verified single-shape handler facts). now_unix
    // carries the explicit №316 Source classification (the wall-clock
    // read); the rest are the provably-pure `time` default. APPENDED at
    // the end — inserting mid-array would shift existing CallBuiltin
    // indices (.mbc contract). Registry 511→515 (append-only).
    spec!("now_unix", 0, "time"; builtin_now_unix, "Float", "stable"),
    spec!("date_parse_iso", 1, "time"; builtin_date_parse_iso, "Float", "stable"),
    spec!("date_diff_days", 2, "time"; builtin_date_diff_days, "Float", "stable"),
    spec!("date_format_iso", 1, "time"; builtin_date_format_iso, "String", "stable"),
    // ── №590 (Волна 30, Камертон Н1-05): the Box–Muller normal sampler over
    // the shared PRNG — typed return, pure function, loud domain gate
    // ([NORMAL_SAMPLE_STDDEV]). APPENDED at the end — inserting mid-array
    // would shift existing CallBuiltin indices (.mbc contract).
    // Registry 515→516 (append-only).
    spec!("normal_sample", 2, "math"; builtin_normal_sample, "Float", "stable"),
    // ── Naryad №844 (wave 72 — W-B, ADR-0203 §2.1): the capability_state
    // LIVE typed view — scope/class/ttl/quota/revoked/state read from the
    // grant ledger AT CALL TIME (the ledger stays the SSOT; the view is
    // read-only and consumes NOTHING — the single consume points stay
    // grant_use and the carrier gates' own post-success consumption).
    // Every field is the in-tree ledger read → internal (the ledger_verify
    // verdict posture). APPENDED at the end — inserting mid-array would
    // shift existing CallBuiltin indices (.mbc contract).
    // Registry 516→517 (append-only).
    spec!("capability_state", 1, "security"; builtin_capability_state, "Struct<CapabilityState>{scope:internal,class:internal,ttl:internal,quota:internal,revoked:internal,state:internal}", "stable"),
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

/// №799 (T2, gh#1488): name → the compiled spec, if the row compiles
/// into THIS build. The tier reader for the check lane's experimental
/// warning (the parked names are NOT here in the default build — they
/// refuse through `builtins::parked`, not through their tier).
pub fn spec_by_name(name: &str) -> Option<&'static BuiltinSpec> {
    BUILTIN_REGISTRY.iter().find(|s| s.name == name)
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
