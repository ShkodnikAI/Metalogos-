//! №466 (gh#687) — group 5 (audit-ledger) of the TW/VM duplicate-name
//! transfer: the shared LIVE home of the deny-event pair and the event
//! stream readers (gate gh#680, decision 4-A, step 3; threshold 35 → 30).
//!
//! The five names in this group — `deny_event`, `deny_reason`,
//! `event_count`, `event_sum`, `events_since` — were handled separately
//! by both backends (the №462 counter's duplicates). Their name
//! constants now live HERE (the single spelling outside
//! `BUILTIN_REGISTRY`); the backends keep only argument marshaling and
//! their per-backend deny-event accessor (TW: `Mutex<Option<Value>>`
//! via `take_deny_event`; VM: the plain `current_deny_event` field —
//! the same "event live exactly while the handler body runs" contract,
//! spelled once in `DENY_HANDLER_ERR`).
//!
//! The event-stream readers share ONE state contract without a trait:
//! both backends hold the identical `event_log: std::sync::Mutex<Vec<Event>>`
//! over the same `crate::interpreter::Event` record, so the bodies move
//! here verbatim over `&Mutex<Vec<Event>>` — the strongest live contract
//! available (a shared type, not a substitute). The poison-fallback
//! semantics (a poisoned log reads as 0 / 0.0 / an empty list) are part
//! of the shared body and are pinned by the unit tests below.
//!
//! Per-backend divergences are NONE on this group: the dispatch-site
//! bodies were byte-identical modulo the accessor spelling (verified
//! byte-for-byte at the transfer), and the №465 diff-fuzzer runs
//! before/after the transfer prove the semantics unchanged.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::interpreter::{Event, Value};

/// The single spelling of the audit-ledger group outside the registry.
pub const NAME_DENY_EVENT: &str = "deny_event";
pub const NAME_DENY_REASON: &str = "deny_reason";
pub const NAME_EVENT_COUNT: &str = "event_count";
pub const NAME_EVENT_SUM: &str = "event_sum";
pub const NAME_EVENTS_SINCE: &str = "events_since";

/// The deny-event availability error — spelled once (Наряд №392: the
/// event is runtime-constructed and cannot be forged or stale-read;
/// outside an on_deny handler this is a loud error on BOTH backends).
pub const DENY_HANDLER_ERR: &str = "deny_event() is only available inside an on_deny handler";

/// Whether `name` belongs to the audit-ledger group (the dispatch hook
/// the backends call before falling through to the registry front door).
pub fn handles(name: &str) -> bool {
    matches!(
        name,
        NAME_DENY_EVENT | NAME_DENY_REASON | NAME_EVENT_COUNT | NAME_EVENT_SUM | NAME_EVENTS_SINCE
    )
}

/// The shared body of the deny pair: extract `reason` from the live
/// deny event and return the event itself (`deny_event`) or the reason
/// (`deny_reason`). The caller has already obtained the event through
/// its per-backend accessor (and errored loudly when outside a handler).
pub fn deny_event_or_reason(name: &str, event: Value) -> Value {
    let reason = match &event {
        Value::Struct { fields, .. } => fields.get("reason").cloned().unwrap_or(Value::Unit),
        other => other.clone(),
    };
    if name == NAME_DENY_EVENT {
        event
    } else {
        reason
    }
}

/// The core of `event_count`: the count over the shared log, filtered
/// by type when given; a poisoned log reads as 0.
pub fn event_count_in(event_log: &std::sync::Mutex<Vec<Event>>, event_type: Option<&str>) -> usize {
    if let Ok(log) = event_log.lock() {
        match event_type {
            Some(t) => log.iter().filter(|e| e.event_type == t).count(),
            None => log.len(),
        }
    } else {
        0
    }
}

/// The dispatch shape of `event_count` (the typed-arg marshaling the
/// backends kept duplicated): the type argument stringifies, the count
/// is a `Float`.
pub fn event_count(event_log: &std::sync::Mutex<Vec<Event>>, args: &[Value]) -> Value {
    let etype = args.first().map(|a| format!("{}", a));
    let count = event_count_in(event_log, etype.as_deref());
    Value::Float(count as f64)
}

/// The core of `event_sum`: the numeric sum of `field` across events of
/// `event_type`; a poisoned log reads as 0.0.
pub fn event_sum_in(
    event_log: &std::sync::Mutex<Vec<Event>>,
    event_type: &str,
    field: &str,
) -> f64 {
    if let Ok(log) = event_log.lock() {
        log.iter()
            .filter(|e| e.event_type == event_type)
            .filter_map(|e| e.data.get(field))
            .filter_map(|v| v.parse::<f64>().ok())
            .sum()
    } else {
        0.0
    }
}

/// The dispatch shape of `event_sum(type, field)`: exactly two
/// arguments, both stringify, the sum is a `Float`.
pub fn event_sum(
    event_log: &std::sync::Mutex<Vec<Event>>,
    args: &[Value],
) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("event_sum() requires 2 arguments (type, field)".to_string());
    }
    let etype = format!("{}", args[0]);
    let field = format!("{}", args[1]);
    Ok(Value::Float(event_sum_in(event_log, &etype, &field)))
}

/// The core of `events_since`: the log entries with `timestamp >= since_ms`
/// (cloned, in log order); a poisoned log reads as an empty list.
pub fn events_since_in(event_log: &std::sync::Mutex<Vec<Event>>, since_ms: u64) -> Vec<Event> {
    if let Ok(log) = event_log.lock() {
        log.iter()
            .filter(|e| e.timestamp >= since_ms)
            .cloned()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    }
}

/// The dispatch shape of `events_since(seconds)`: the Float validation
/// with the exact error texts, the now-anchored window, and the Event
/// struct mapping (id / timestamp / event_type / source / data_json /
/// duration_ms, `type_name: "Event"`) — byte-identical on both backends
/// at the transfer.
pub fn events_since(
    event_log: &std::sync::Mutex<Vec<Event>>,
    args: &[Value],
) -> Result<Value, String> {
    let seconds = match args.first() {
        Some(Value::Float(s)) => *s,
        Some(other) => {
            return Err(format!(
                "events_since() expected Float, got {}",
                other.type_name()
            ))
        }
        None => return Err("events_since() requires 1 argument (seconds)".to_string()),
    };
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let since_ms = now_ms.saturating_sub((seconds * 1000.0) as u64);
    let events = events_since_in(event_log, since_ms);
    let mut list = Vec::new();
    for ev in events {
        let mut fields = HashMap::new();
        fields.insert("id".to_string(), Value::Float(ev.id as f64));
        fields.insert("timestamp".to_string(), Value::Float(ev.timestamp as f64));
        fields.insert("event_type".to_string(), Value::String(ev.event_type));
        fields.insert("source".to_string(), Value::String(ev.source));
        fields.insert(
            "data_json".to_string(),
            Value::String(format!("{:?}", ev.data)),
        );
        if let Some(dur) = ev.duration_ms {
            fields.insert("duration_ms".to_string(), Value::Float(dur as f64));
        }
        list.push(Value::Struct {
            type_name: "Event".to_string(),
            fields,
        });
    }
    Ok(Value::List(list))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(id: u64, ts: u64, event_type: &str, data: &[(&str, &str)]) -> Event {
        Event {
            id,
            timestamp: ts,
            event_type: event_type.to_string(),
            source: "test".to_string(),
            data: data
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            duration_ms: None,
        }
    }

    // Value has no PartialEq — the tests assert by shape.
    fn is_float(v: &Value, x: f64) -> bool {
        matches!(v, Value::Float(f) if *f == x)
    }

    fn is_string(v: &Value, s: &str) -> bool {
        matches!(v, Value::String(t) if t == s)
    }

    fn log(events: Vec<Event>) -> std::sync::Mutex<Vec<Event>> {
        std::sync::Mutex::new(events)
    }

    /// Genuinely poison the log mutex: panic while a guard is held (the
    /// same fallback path the backends hit when a poisoned log races).
    fn poison(l: &std::sync::Mutex<Vec<Event>>) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = l.lock().unwrap();
            panic!("poison the log for the fallback-path test");
        }));
    }

    // ── deny_event_or_reason ────────────────────────────────────────

    fn deny_event_value(reason: &str) -> Value {
        let mut fields = HashMap::new();
        fields.insert("reason".to_string(), Value::String(reason.to_string()));
        Value::Struct {
            type_name: "DenyEvent".to_string(),
            fields,
        }
    }

    #[test]
    fn deny_pair_returns_event_and_reason() {
        let event = deny_event_value("policy X");
        let ev_out = deny_event_or_reason(NAME_DENY_EVENT, event.clone());
        match ev_out {
            Value::Struct { type_name, .. } => assert_eq!(type_name, "DenyEvent"),
            other => panic!("expected the event struct, got {:?}", other),
        }
        let reason_out = deny_event_or_reason(NAME_DENY_REASON, event);
        assert!(is_string(&reason_out, "policy X"), "got {:?}", reason_out);
    }

    #[test]
    fn deny_reason_missing_field_is_unit_and_non_struct_clones() {
        let mut fields = HashMap::new();
        fields.insert("other".to_string(), Value::Float(1.0));
        let no_reason = Value::Struct {
            type_name: "DenyEvent".to_string(),
            fields,
        };
        assert!(matches!(
            deny_event_or_reason(NAME_DENY_REASON, no_reason),
            Value::Unit
        ));
        // A non-struct event clones through as the reason.
        let scalar = Value::String("bare".to_string());
        assert!(is_string(
            &deny_event_or_reason(NAME_DENY_REASON, scalar.clone()),
            "bare"
        ));
    }

    #[test]
    fn deny_handler_error_text_is_shared_and_exact() {
        // The single spelling — TW (hooks.rs) and VM (vm.rs) both surface it.
        assert_eq!(
            DENY_HANDLER_ERR,
            "deny_event() is only available inside an on_deny handler"
        );
    }

    #[test]
    fn handles_covers_exactly_the_five_names() {
        for name in [
            NAME_DENY_EVENT,
            NAME_DENY_REASON,
            NAME_EVENT_COUNT,
            NAME_EVENT_SUM,
            NAME_EVENTS_SINCE,
        ] {
            assert!(handles(name), "{name} must be handled");
        }
        assert!(!handles("print"));
        assert!(!handles("deny"));
        assert!(!handles("event"));
    }

    // ── event_count ─────────────────────────────────────────────────

    #[test]
    fn event_count_filters_by_type_and_counts_all() {
        let l = log(vec![
            ev(1, 10, "memory_store", &[]),
            ev(2, 20, "memory_store", &[]),
            ev(3, 30, "adapt", &[]),
        ]);
        assert_eq!(event_count_in(&l, Some("memory_store")), 2);
        assert_eq!(event_count_in(&l, Some("adapt")), 1);
        assert_eq!(event_count_in(&l, None), 3);
        assert_eq!(event_count_in(&l, Some("absent")), 0);
    }

    #[test]
    fn event_count_dispatch_stringifies_the_type_arg() {
        let l = log(vec![ev(1, 10, "memory_store", &[])]);
        // The dispatch shape: `format!("{}", Value)` — a String Value
        // stringifies to its contents.
        let out = event_count(&l, &[Value::String("memory_store".to_string())]);
        assert!(is_float(&out, 1.0), "got {:?}", out);
        // No argument — the unfiltered total.
        let total = event_count(&l, &[]);
        assert!(is_float(&total, 1.0), "got {:?}", total);
    }

    #[test]
    fn event_count_poisoned_log_reads_zero() {
        let l = log(vec![ev(1, 10, "t", &[])]);
        poison(&l);
        assert_eq!(event_count_in(&l, None), 0);
        let out = event_count(&l, &[]);
        assert!(is_float(&out, 0.0), "got {:?}", out);
    }

    // ── event_sum ───────────────────────────────────────────────────

    #[test]
    fn event_sum_sums_numeric_fields_of_the_type() {
        let l = log(vec![
            ev(1, 10, "llm_call", &[("tokens", "12")]),
            ev(2, 20, "llm_call", &[("tokens", "30.5")]),
            ev(3, 30, "llm_call", &[("tokens", "not-a-number")]),
            ev(4, 40, "other", &[("tokens", "100")]),
        ]);
        assert_eq!(event_sum_in(&l, "llm_call", "tokens"), 42.5);
        assert_eq!(event_sum_in(&l, "other", "tokens"), 100.0);
        assert_eq!(event_sum_in(&l, "llm_call", "absent"), 0.0);
    }

    #[test]
    fn event_sum_dispatch_validates_arity_and_stringifies_args() {
        let l = log(vec![ev(1, 10, "llm_call", &[("tokens", "12")])]);
        let ok = event_sum(
            &l,
            &[
                Value::String("llm_call".to_string()),
                Value::String("tokens".to_string()),
            ],
        )
        .unwrap();
        assert!(is_float(&ok, 12.0), "got {:?}", ok);
        let err = event_sum(&l, &[Value::String("llm_call".to_string())]).unwrap_err();
        assert_eq!(err, "event_sum() requires 2 arguments (type, field)");
        let err0 = event_sum(&l, &[]).unwrap_err();
        assert_eq!(err0, "event_sum() requires 2 arguments (type, field)");
    }

    #[test]
    fn event_sum_poisoned_log_reads_zero() {
        let l = log(vec![ev(1, 10, "t", &[("v", "1")])]);
        poison(&l);
        let out = event_sum(
            &l,
            &[
                Value::String("t".to_string()),
                Value::String("v".to_string()),
            ],
        )
        .unwrap();
        assert!(is_float(&out, 0.0), "got {:?}", out);
    }

    // ── events_since ────────────────────────────────────────────────

    #[test]
    fn events_since_filters_by_timestamp_and_maps_the_struct() {
        let now: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let l = log(vec![
            ev(1, now - 60_000, "old", &[("k", "v")]),
            ev(2, now - 1_000, "new", &[("k", "v2")]),
        ]);
        let out = events_since(&l, &[Value::Float(30.0)]).unwrap(); // 30s window
        match out {
            Value::List(items) => {
                assert_eq!(items.len(), 1, "the 60s-old entry must be filtered out");
                match &items[0] {
                    Value::Struct { type_name, fields } => {
                        assert_eq!(type_name, "Event");
                        assert!(matches!(fields.get("id"), Some(v) if is_float(v, 2.0)));
                        assert!(matches!(fields.get("event_type"), Some(v) if is_string(v, "new")));
                        assert!(matches!(fields.get("data_json"), Some(Value::String(_))));
                        assert!(fields.get("duration_ms").is_none());
                    }
                    other => panic!("expected an Event struct, got {:?}", other),
                }
            }
            other => panic!("expected a List, got {:?}", other),
        }
    }

    #[test]
    fn events_since_validates_the_seconds_argument() {
        let l = log(vec![]);
        let err_non_float = events_since(&l, &[Value::String("soon".to_string())]).unwrap_err();
        assert_eq!(
            err_non_float,
            "events_since() expected Float, got String".to_string()
        );
        let err_missing = events_since(&l, &[]).unwrap_err();
        assert_eq!(
            err_missing,
            "events_since() requires 1 argument (seconds)".to_string()
        );
    }

    #[test]
    fn events_since_duration_ms_is_mapped_when_present() {
        let now: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let mut e = ev(1, now, "timed", &[]);
        e.duration_ms = Some(250);
        let l = log(vec![e]);
        let out = events_since(&l, &[Value::Float(60.0)]).unwrap();
        match out {
            Value::List(items) => match &items[0] {
                Value::Struct { fields, .. } => {
                    assert!(matches!(fields.get("duration_ms"), Some(v) if is_float(v, 250.0)));
                }
                other => panic!("expected a struct, got {:?}", other),
            },
            other => panic!("expected a list, got {:?}", other),
        }
    }

    #[test]
    fn events_since_poisoned_log_reads_empty_list() {
        let l = log(vec![ev(1, 10, "t", &[])]);
        poison(&l);
        let out = events_since(&l, &[Value::Float(0.0)]).unwrap();
        match out {
            Value::List(items) => assert!(items.is_empty()),
            other => panic!("expected a list, got {:?}", other),
        }
    }
}
