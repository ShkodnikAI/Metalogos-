// ── Naryad #348 (P1, feature/session, ADR-0172): session builtins ────
//
// The real session surface (replaces the "stub"-category mock handlers
// that lived in crypto.rs: `session_login` returned an empty Session
// map, `session_logout` was a no-op). All state transitions are
// Action-Ledger-recorded by the registry layer (src/session.rs).
//
// Arity/typing contract (registry SSOT — keep in sync with spec! rows):
//   session_login(user, password)              -> Session   (password NOT
//     verified — no server user-store exists; loud boundary, ADR-0172 §6)
//   session_logout(session)                    -> Unit      (SESSION_UNKNOWN
//     on unknown/ended — fail-closed, no implicit recreate)
//   session_duty_enter(session)                -> Bool      (true when the
//     flag flipped)
//   session_duty_exit(session)                 -> Bool
//   session_wake(session, source, payload?)    -> Number    (queue length)
//     sources: "keyword" | "event" | "schedule" (closed set; schedule
//     wakes arrive via the №418 cron payload-dispatch — SSOT untouched)
//   session_poll_wake(session)                 -> Struct{source,payload} | Unit
//   session_interrupt(session, priority, reason?) -> Number
//     priorities: "low" < "normal" < "high" < "critical" (typed enum)
//   session_take_interrupt(session)            -> Struct{priority,reason} | Unit
//     (highest priority first, FIFO within — the №352 preemption lever)

use super::core::expect_string_arg;
use crate::interpreter::values::Value;
use crate::session;
use std::collections::HashMap;

/// `session_login(user, password) -> Session`
pub(crate) fn builtin_session_login(args: &[Value]) -> Result<Value, String> {
    let fn_name = "session_login";
    if args.len() != 2 {
        return Err(format!(
            "{}: expects 2 arguments (user, password), got {}",
            fn_name,
            args.len()
        ));
    }
    let user = expect_string_arg(fn_name, args, 0)?;
    // The password is accepted as String or Secret (the №274 crypto
    // surface); it is NOT verified — there is no server user-store in
    // the interpreter. Documented boundary, not a stub (ADR-0172 §6).
    match args.get(1) {
        Some(Value::String(_)) | Some(Value::Secret(_)) => {}
        Some(other) => {
            return Err(format!(
                "{}: expected String or Secret as password, got {}",
                fn_name,
                other.type_name()
            ))
        }
        None => {
            return Err(format!(
                "{}: requires 2 arguments (user, password)",
                fn_name
            ))
        }
    }
    Ok(session::create(&user))
}

/// `session_logout(session) -> Unit`
pub(crate) fn builtin_session_logout(args: &[Value]) -> Result<Value, String> {
    let fn_name = "session_logout";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (session), got {}",
            fn_name,
            args.len()
        ));
    }
    let id = session::session_id_arg(fn_name, args, 0)?;
    session::end(fn_name, &id)?;
    Ok(Value::Unit)
}

/// `session_duty_enter(session) -> Bool`
pub(crate) fn builtin_session_duty_enter(args: &[Value]) -> Result<Value, String> {
    let fn_name = "session_duty_enter";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (session), got {}",
            fn_name,
            args.len()
        ));
    }
    let id = session::session_id_arg(fn_name, args, 0)?;
    let changed = session::duty_enter(&id)?;
    Ok(Value::Bool(changed))
}

/// `session_duty_exit(session) -> Bool`
pub(crate) fn builtin_session_duty_exit(args: &[Value]) -> Result<Value, String> {
    let fn_name = "session_duty_exit";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (session), got {}",
            fn_name,
            args.len()
        ));
    }
    let id = session::session_id_arg(fn_name, args, 0)?;
    let changed = session::duty_exit(&id)?;
    Ok(Value::Bool(changed))
}

/// `session_wake(session, source, payload?) -> Number`
pub(crate) fn builtin_session_wake(args: &[Value]) -> Result<Value, String> {
    let fn_name = "session_wake";
    if args.len() < 2 || args.len() > 3 {
        return Err(format!(
            "{}: expects 2..3 arguments (session, source, payload?), got {}",
            fn_name,
            args.len()
        ));
    }
    let id = session::session_id_arg(fn_name, args, 0)?;
    let source = expect_string_arg(fn_name, args, 1)?;
    let payload = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Unit) | None => String::new(),
        Some(other) => {
            return Err(format!(
                "{}: expected String or Unit as payload, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    session::wake(&id, &source, &payload)?;
    // The queue is internal state; the surface returns the delivered
    // source so programs can branch on what woke them without polling.
    Ok(Value::String(source))
}

/// `session_poll_wake(session) -> Struct | Unit`
pub(crate) fn builtin_session_poll_wake(args: &[Value]) -> Result<Value, String> {
    let fn_name = "session_poll_wake";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (session), got {}",
            fn_name,
            args.len()
        ));
    }
    let id = session::session_id_arg(fn_name, args, 0)?;
    match session::poll_wake(&id)? {
        Some((source, payload)) => Ok(Value::Struct {
            type_name: "Wake".to_string(),
            fields: HashMap::from([
                ("source".to_string(), Value::String(source)),
                ("payload".to_string(), Value::String(payload)),
            ]),
        }),
        None => Ok(Value::Unit),
    }
}

/// `session_interrupt(session, priority, reason?) -> String`
pub(crate) fn builtin_session_interrupt(args: &[Value]) -> Result<Value, String> {
    let fn_name = "session_interrupt";
    if args.len() < 2 || args.len() > 3 {
        return Err(format!(
            "{}: expects 2..3 arguments (session, priority, reason?), got {}",
            fn_name,
            args.len()
        ));
    }
    let id = session::session_id_arg(fn_name, args, 0)?;
    let priority = expect_string_arg(fn_name, args, 1)?;
    let reason = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Unit) | None => String::new(),
        Some(other) => {
            return Err(format!(
                "{}: expected String or Unit as reason, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    session::interrupt(&id, &priority, &reason)?;
    Ok(Value::String(priority))
}

/// `session_take_interrupt(session) -> Struct | Unit`
pub(crate) fn builtin_session_take_interrupt(args: &[Value]) -> Result<Value, String> {
    let fn_name = "session_take_interrupt";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (session), got {}",
            fn_name,
            args.len()
        ));
    }
    let id = session::session_id_arg(fn_name, args, 0)?;
    match session::take_interrupt(&id)? {
        Some((priority, reason)) => Ok(Value::Struct {
            type_name: "Interrupt".to_string(),
            fields: HashMap::from([
                ("priority".to_string(), Value::String(priority)),
                ("reason".to_string(), Value::String(reason)),
            ]),
        }),
        None => Ok(Value::Unit),
    }
}
