// ── Naryad #352 (P1, voice/duplex): the duplex-channel builtins ──────
//
// The Language surface of the barge-in state machine (state:
// src/duplex.rs). Arity/typing contract (registry SSOT — keep in sync
// with spec! rows):
//
//   duplex_open(session, priority?)   -> Duplex   binds ONE live №348
//     session; unknown session — SESSION_UNKNOWN; the priority word is
//     the session ladder (low|normal|high|critical; default normal)
//   speak_start(duplex, text, priority?) -> Struct   the SPEAK flow:
//     {stream_id, priority, preempted: List<String>}; same-direction
//     busy — DUPLEX_BUSY; lower-rank barge-in — DUPLEX_PREEMPT_DENIED;
//     a successful barge-in ends the listen stream TYPED
//   listen_start(duplex, priority?)   -> Struct   the LISTEN flow
//     (symmetric to speak_start)
//   speak_stop(duplex)                -> Struct   ends the speak stream
//     ({stream_id, outcome: "completed"}); idle — DUPLEX_IDLE
//   listen_stop(duplex)               -> Struct   ends the listen stream
//   duplex_state(duplex)              -> Struct   {id, session, speak,
//     listen} — the stream projections or Unit per direction

use super::core::expect_string_arg;
use crate::duplex::{self, channel_id_arg};
use crate::interpreter::values::Value;

fn expect_optional_priority(
    fn_name: &str,
    args: &[Value],
    idx: usize,
) -> Result<Option<String>, String> {
    match args.get(idx) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(other) => Err(format!(
            "{}: priority must be String ({}), got {}",
            fn_name,
            duplex::DUPLEX_PRIORITIES.join("/"),
            other.type_name()
        )),
    }
}

fn start_result_value(stream: &duplex::Stream, preempted: &[duplex::Stream]) -> Value {
    super::core::make_struct(
        "DuplexFlow",
        vec![
            ("stream_id", Value::String(stream.id.clone())),
            ("direction", Value::String(stream.direction.to_string())),
            ("priority", Value::String(stream.priority_word.clone())),
            (
                "preempted",
                Value::List(
                    preempted
                        .iter()
                        .map(|s| Value::String(s.id.clone()))
                        .collect(),
                ),
            ),
        ],
    )
}

fn stop_result_value(stream: &duplex::Stream) -> Value {
    super::core::make_struct(
        "DuplexStopped",
        vec![
            ("stream_id", Value::String(stream.id.clone())),
            ("direction", Value::String(stream.direction.to_string())),
            (
                "outcome",
                Value::String(stream.outcome.as_str().to_string()),
            ),
        ],
    )
}

/// `duplex_open(session, priority?) -> Duplex`
pub(crate) fn builtin_duplex_open(args: &[Value]) -> Result<Value, String> {
    let fn_name = "duplex_open";
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "{}: expects 1 or 2 arguments (session, priority?), got {}",
            fn_name,
            args.len()
        ));
    }
    let session_id = crate::session::session_id_arg(fn_name, args, 0)?;
    let priority = expect_optional_priority(fn_name, args, 1)?;
    duplex::open(&session_id, priority.as_deref().unwrap_or("normal"))
}

/// `speak_start(duplex, text, priority?) -> Struct`
pub(crate) fn builtin_speak_start(args: &[Value]) -> Result<Value, String> {
    let fn_name = "speak_start";
    if args.len() < 2 || args.len() > 3 {
        return Err(format!(
            "{}: expects 2 or 3 arguments (duplex, text, priority?), got {}",
            fn_name,
            args.len()
        ));
    }
    let channel_id = channel_id_arg(fn_name, args, 0)?;
    let _text = expect_string_arg(fn_name, args, 1)?;
    let priority = expect_optional_priority(fn_name, args, 2)?;
    let (stream, preempted) = duplex::speak_start(&channel_id, &_text, priority.as_deref())?;
    Ok(start_result_value(&stream, &preempted))
}

/// `listen_start(duplex, priority?) -> Struct`
pub(crate) fn builtin_listen_start(args: &[Value]) -> Result<Value, String> {
    let fn_name = "listen_start";
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "{}: expects 1 or 2 arguments (duplex, priority?), got {}",
            fn_name,
            args.len()
        ));
    }
    let channel_id = channel_id_arg(fn_name, args, 0)?;
    let priority = expect_optional_priority(fn_name, args, 1)?;
    let (stream, preempted) = duplex::listen_start(&channel_id, priority.as_deref())?;
    Ok(start_result_value(&stream, &preempted))
}

/// `speak_stop(duplex) -> Struct`
pub(crate) fn builtin_speak_stop(args: &[Value]) -> Result<Value, String> {
    let fn_name = "speak_stop";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (duplex), got {}",
            fn_name,
            args.len()
        ));
    }
    let channel_id = channel_id_arg(fn_name, args, 0)?;
    let stream = duplex::speak_stop(&channel_id)?;
    Ok(stop_result_value(&stream))
}

/// `listen_stop(duplex) -> Struct`
pub(crate) fn builtin_listen_stop(args: &[Value]) -> Result<Value, String> {
    let fn_name = "listen_stop";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (duplex), got {}",
            fn_name,
            args.len()
        ));
    }
    let channel_id = channel_id_arg(fn_name, args, 0)?;
    let stream = duplex::listen_stop(&channel_id)?;
    Ok(stop_result_value(&stream))
}

/// `duplex_state(duplex) -> Struct`
pub(crate) fn builtin_duplex_state(args: &[Value]) -> Result<Value, String> {
    let fn_name = "duplex_state";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (duplex), got {}",
            fn_name,
            args.len()
        ));
    }
    let channel_id = channel_id_arg(fn_name, args, 0)?;
    duplex::state(&channel_id)
}
