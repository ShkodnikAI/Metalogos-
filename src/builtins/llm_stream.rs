// ── LLM streaming builtins: llm_stream_open / llm_stream_next / llm_stream_close ──
//
// Наряд №275 (issue #311, ADR-0137, dispatch #316) — streaming LLM over
// `reqwest::blocking` (`impl Read` + incremental SSE-парсер, ADR-0137 §D3).
//
// API (ADR-0137 §D1):
//   let s      = llm_stream_open(prompt, input?)   -> Struct{handle, model, provider}
//   let delta  = llm_stream_next(s.handle)         -> String  | "" keep-alive | "__end__"
//   let final  = llm_stream_close(s.handle)         -> Struct{tokens, latency_ms, status, provider, model}
//
// Implementation lives in `crate::llm` (LlmStreamRegistry, stream_via_smart_router,
// stream_next, stream_close) — this module is the BUILTIN surface that
// translates Value args to/from those calls. Body is the SAME for both
// backends (TW / VM) — parity by construction (ADR-0137 §D9).

use crate::interpreter::Value;
use crate::llm::{self, LlmStreamId, LLM_STREAM_END_MARKER};

use super::core::expect_string_arg;

/// `llm_stream_open(prompt, input?) -> Struct { handle, model, provider }`
///
/// Opens a streaming LLM call through the global SmartRouter. The SmartRouter
/// picks the best available provider (health-sorted, circuit-breaker-aware),
/// issues the POST with `stream: true`, and stores the active response +
/// incremental SSE-парсер in `LLM_STREAM_REGISTRY`. Returns an opaque
/// `Value::LlmStream` handle the caller passes to `llm_stream_next` /
/// `llm_stream_close`.
///
/// Mock / non-SSE backends (METALOGOS_MOCK_LLM=true, future non-streaming
/// providers) → loud `STREAM_UNSUPPORTED` error (issue #311 contract: not a
/// silent full-answer).
///
/// Bounded by `LLM_STREAM_REGISTRY` upper limit (default 64,
/// `METALOGOS_LLM_STREAM_MAX` env override). Exceeding → loud
/// `STREAM_LIMIT_REACHED` (lesson №263).
pub(crate) fn builtin_llm_stream_open(args: &[Value]) -> Result<Value, String> {
    let prompt = expect_string_arg("llm_stream_open", args, 0)?;
    let input = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Unit) | None => String::new(),
        Some(other) => format!("{}", other),
    };

    let handle: LlmStreamId = llm::stream_via_smart_router(&prompt, &input, None, None)?;

    // Read back the provider/model from the registry for the Struct return.
    // This is the only place the open builtin looks inside — `next` / `close`
    // use the handle directly. Failure to fetch provenance here is non-fatal:
    // the stream was opened, we just couldn't peek at metadata for the
    // response Struct.
    let (provider, model) = llm::peek_stream_provenance(handle)
        .unwrap_or(("unknown".to_string(), "unknown".to_string()));

    let mut fields = std::collections::HashMap::new();
    fields.insert("handle".to_string(), Value::LlmStream(handle));
    fields.insert("model".to_string(), Value::String(model));
    fields.insert("provider".to_string(), Value::String(provider));
    Ok(Value::Struct {
        type_name: "LlmStream".to_string(),
        fields,
    })
}

/// `llm_stream_next(handle) -> String`
///
/// One blocking `Read::read` + parse one SSE delta. Returns:
/// - the delta text chunk (may be empty if the provider sent a keep-alive
///   comment / SSE-meta line),
/// - `"__end__"` when the stream has been fully consumed (final SSE event
///   seen — `data: [DONE]` for OpenAI, `message_stop` for Anthropic,
///   `done: true` for Ollama). The caller should call `llm_stream_close`
///   upon receiving this marker.
///
/// Errors are loud — a broken connection, a malformed SSE event, or an
/// unknown handle surfaces as a `String` error.
pub(crate) fn builtin_llm_stream_next(args: &[Value]) -> Result<Value, String> {
    let handle = expect_stream_handle("llm_stream_next", args, 0)?;
    let delta = llm::stream_next(handle)?;
    Ok(Value::String(delta))
}

/// `llm_stream_close(handle) -> Struct { tokens, latency_ms, status, provider, model }`
///
/// Drops the active response (closes the underlying TCP connection — visible
/// to the provider as a client-side close), aggregates the final usage from
/// the last SSE event, writes ONE trace line (ADR-0138 §D4 — one line per
/// completed stream, never per chunk), and returns the final metadata.
///
/// `tokens` = sum of `input_tokens + output_tokens` (as `Float`); `None` if
/// the provider did not report usage. `latency_ms` = open→close wall-clock.
/// `status` = "ok" | "error".
pub(crate) fn builtin_llm_stream_close(args: &[Value]) -> Result<Value, String> {
    let handle = expect_stream_handle("llm_stream_close", args, 0)?;
    let final_meta = llm::stream_close(handle)?;

    let tokens = match (final_meta.input_tokens, final_meta.output_tokens) {
        (Some(i), Some(o)) => Value::Float((i + o) as f64),
        (Some(i), None) => Value::Float(i as f64),
        (None, Some(o)) => Value::Float(o as f64),
        (None, None) => Value::Float(0.0),
    };
    let mut fields = std::collections::HashMap::new();
    fields.insert("tokens".to_string(), tokens);
    fields.insert(
        "latency_ms".to_string(),
        Value::Float(final_meta.latency_ms as f64),
    );
    fields.insert(
        "status".to_string(),
        Value::String(final_meta.status.to_string()),
    );
    fields.insert("provider".to_string(), Value::String(final_meta.provider));
    fields.insert("model".to_string(), Value::String(final_meta.model));
    fields.insert(
        "input_tokens".to_string(),
        match final_meta.input_tokens {
            Some(t) => Value::Float(t as f64),
            None => Value::Unit,
        },
    );
    fields.insert(
        "output_tokens".to_string(),
        match final_meta.output_tokens {
            Some(t) => Value::Float(t as f64),
            None => Value::Unit,
        },
    );
    // Aggregated delta text — equivalent to the single-shot call_llm
    // response for the same prompt+input. Useful for the equivalence test
    // (issue #311: "Итоговый текст идентичен не-стримовому вызову той же
    // фразы") without re-running the LLM.
    fields.insert(
        "aggregated_text".to_string(),
        Value::String(final_meta.aggregated_text),
    );
    Ok(Value::Struct {
        type_name: "LlmStreamFinal".to_string(),
        fields,
    })
}

/// `llm_stream_end_marker()` — return the end-of-stream sentinel constant.
/// Not strictly necessary (callers can compare to `"__end__"`), but this
/// gives a stable source of truth if the marker changes shape later.
#[allow(dead_code)]
pub(crate) fn builtin_llm_stream_end_marker(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::String(LLM_STREAM_END_MARKER.to_string()))
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Extract a `LlmStreamId` from `args[idx]` — loud error on wrong type.
fn expect_stream_handle(builtin: &str, args: &[Value], idx: usize) -> Result<LlmStreamId, String> {
    match args.get(idx) {
        Some(Value::LlmStream(id)) => Ok(*id),
        Some(other) => Err(format!(
            "{}(): expected LlmStream handle as arg {}, got {}",
            builtin,
            idx,
            other.type_name()
        )),
        None => Err(format!(
            "{}(): requires at least {} argument(s) — missing LlmStream handle",
            builtin,
            idx + 1
        )),
    }
}
