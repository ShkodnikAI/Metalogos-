// ── Naryad #352 (P1, voice/duplex): the barge-in state machine ────────
//
// The registry naryad's test contract (ADR-0174 §3.3-3.5):
//   D1  the channel binds ONE live session — unknown session refuses
//       (SESSION_UNKNOWN, fail-closed); the priority vocabulary is the
//       SESSION ladder (low|normal|high|critical), unknown words are loud.
//   D2  same-direction concurrency refuses (DUPLEX_BUSY — one flow per
//       direction).
//   D3  BARGE-IN by the session ladder: an opposite-direction start with
//       rank >= the active stream's rank PREEMPTS it (equal wins); the
//       preempted stream ends TYPED (InterruptedBy{by, priority}) —
//       observable, never a panic; a LOWER-rank start refuses
//       (DUPLEX_PREEMPT_DENIED).
//   D4  the direction is free again after a terminal outcome (the slot
//       invariant) — the typed outcome lives in the terminal-history
//       slot; stops complete typed (DUPLEX_IDLE when already idle).
//   D5  AUDIT COMPLETENESS (the acceptance invariant): every transition
//       is a ledger record; preemption emits TWO (the decision AND the
//       terminal outcome) — interruption loses no audit event; the count
//       is pinned differentially against an independent transition model.

use metalogos::builtins::BUILTIN_REGISTRY;
use metalogos::interpreter::Value;
use metalogos::ledger::all_records;

fn call_builtin(name: &str, args: &[Value]) -> Result<Value, String> {
    let spec = BUILTIN_REGISTRY
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("builtin {name} not in BUILTIN_REGISTRY"));
    let handler = spec.handler.expect("builtin has handler");
    handler(args)
}

fn s(v: &str) -> Value {
    Value::String(v.to_string())
}

fn unique(tag: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}-{}-{}", tag, std::process::id(), nanos)
}

fn fresh_session() -> Value {
    call_builtin(
        "session_login",
        &[s(&unique("dup-user")), s("pw-not-checked-by-design")],
    )
    .expect("session_login")
}

fn session_id(handle: &Value) -> String {
    match handle {
        Value::Session(map) => map.get("id").cloned().expect("session id"),
        other => panic!("expected Session, got {}", other.type_name()),
    }
}

fn open_channel(session: &Value, priority: Option<&str>) -> Result<Value, String> {
    let mut args = vec![session.clone()];
    if let Some(p) = priority {
        args.push(s(p));
    }
    call_builtin("duplex_open", &args)
}

fn channel_id(handle: &Value) -> String {
    match handle {
        Value::Duplex(map) => map.get("id").cloned().expect("channel id"),
        other => panic!("expected Duplex, got {}", other.type_name()),
    }
}

fn channel_records(ch: &str) -> Vec<String> {
    all_records()
        .expect("ledger readable")
        .into_iter()
        .filter(|r| r.action.starts_with("duplex.") && r.actor == ch)
        .map(|r| r.action)
        .collect()
}

fn str_field(v: &Value, name: &str) -> String {
    match v {
        Value::Struct { fields, .. } => match fields.iter().find(|(k, _)| k.as_str() == name) {
            Some((_, Value::String(x))) => x.clone(),
            _ => panic!("no string field {name}"),
        },
        other => panic!("expected Struct, got {}", other.type_name()),
    }
}

fn str_list_field(v: &Value, name: &str) -> Vec<String> {
    match v {
        Value::Struct { fields, .. } => match fields.iter().find(|(k, _)| k.as_str() == name) {
            Some((_, Value::List(items))) => items
                .iter()
                .map(|x| match x {
                    Value::String(t) => t.clone(),
                    other => panic!("element not a string: {}", other.type_name()),
                })
                .collect(),
            _ => panic!("no list field {name}"),
        },
        other => panic!("expected Struct, got {}", other.type_name()),
    }
}

// ── D1: the session binding and the priority vocabulary ────────────────

#[test]
fn open_requires_a_live_session_and_the_session_ladder() {
    // A well-formed Session handle naming an UNKNOWN (already-ended)
    // session — the registry refuses fail-closed.
    let ghost = Value::Session(std::collections::HashMap::from([(
        "id".to_string(),
        "sess-ghost-000000".to_string(),
    )]));
    let err = open_channel(&ghost, None).expect_err("unknown session");
    assert!(err.contains("SESSION_UNKNOWN"), "typed refusal, got: {err}");
    assert!(!err.contains("panic"));

    let session = fresh_session();
    let handle = open_channel(&session, Some("normal")).expect("open with the ladder word");
    let ch = channel_id(&handle);
    assert!(ch.starts_with("dup-"), "channel id prefix: {ch}");

    let err2 = open_channel(&session, Some("urgent")).expect_err("unknown priority word");
    assert!(err2.contains("unknown priority 'urgent'"), "got: {err2}");

    // The default fills when the word is absent (normal by default).
    let handle2 = open_channel(&session, None).expect("open with the default");
    assert_ne!(channel_id(&handle2), ch, "a fresh channel, not a reopen");
}

#[test]
fn a_handle_that_is_not_duplex_refuses_typed() {
    let session = fresh_session();
    let err = call_builtin("speak_start", &[session.clone(), s("text")])
        .expect_err("Session is not Duplex");
    assert!(err.contains("expected Duplex"), "got: {err}");
    let err2 = call_builtin("listen_start", std::slice::from_ref(&session))
        .expect_err("Session is not Duplex");
    assert!(err2.contains("expected Duplex"), "got: {err2}");
}

// ── D2: one flow per direction ─────────────────────────────────────────

#[test]
fn same_direction_concurrency_refuses_with_duplex_busy() {
    let session = fresh_session();
    let handle = open_channel(&session, None).expect("open");
    let _ch = channel_id(&handle);
    let first =
        call_builtin("speak_start", &[handle.clone(), s("hello"), s("normal")]).expect("speak 1");
    assert!(
        str_list_field(&first, "preempted").is_empty(),
        "nothing to preempt"
    );
    let err = call_builtin("speak_start", &[handle.clone(), s("second"), s("critical")])
        .expect_err("same-direction busy");
    assert!(err.contains("DUPLEX_BUSY"), "got: {err}");
    // The listen direction is unaffected — full-duplex IS the point.
    let listen =
        call_builtin("listen_start", &[handle.clone(), s("critical")]).expect("listen over speak");
    assert_eq!(
        str_list_field(&listen, "preempted").len(),
        1,
        "the speak was preempted"
    );
}

// ── D3: the barge-in ladder ────────────────────────────────────────────

#[test]
fn barge_in_equal_rank_wins_and_the_preempted_stream_ends_typed() {
    let session = fresh_session();
    let handle = open_channel(&session, None).expect("open");
    let ch = channel_id(&handle);
    let speak = call_builtin(
        "speak_start",
        &[handle.clone(), s("long answer"), s("normal")],
    )
    .expect("speak");
    let speak_stream = str_field(&speak, "stream_id");

    // listen at the SAME rank: equal wins (barge-in is the point).
    let listen = call_builtin("listen_start", &[handle.clone(), s("normal")]).expect("barge-in");
    assert_eq!(
        str_list_field(&listen, "preempted"),
        vec![speak_stream.clone()]
    );

    // The preempted stream's outcome is TYPED and observable — the
    // terminal-history slot carries InterruptedBy{by, priority}.
    let outcome =
        metalogos::duplex::stream_last_outcome(&ch, "speak").expect("the terminal outcome exists");
    match outcome {
        metalogos::duplex::StreamOutcome::InterruptedBy { by, priority } => {
            assert_eq!(by, str_field(&listen, "stream_id"));
            assert_eq!(priority, 1, "normal = rank 1");
        }
        other => panic!("expected InterruptedBy, got {other:?}"),
    }
    // The slot is free: the direction can start again (D4).
    assert!(
        metalogos::duplex::stream_id(&ch, "speak").is_none(),
        "the vacated slot"
    );
    let again = call_builtin("speak_start", &[handle.clone(), s("retry"), s("high")])
        .expect("the vacated direction restarts");
    assert_eq!(
        str_list_field(&again, "preempted").len(),
        1,
        "the listen (normal) is preempted by high"
    );
}

#[test]
fn lower_rank_cannot_barge_in_and_the_denial_consumes_nothing() {
    let session = fresh_session();
    let handle = open_channel(&session, None).expect("open");
    let ch = channel_id(&handle);
    let speak = call_builtin(
        "speak_start",
        &[handle.clone(), s("protected"), s("critical")],
    )
    .expect("speak");
    let speak_stream = str_field(&speak, "stream_id");

    let err =
        call_builtin("listen_start", &[handle.clone(), s("low")]).expect_err("low < critical");
    assert!(err.contains("DUPLEX_PREEMPT_DENIED"), "got: {err}");
    // The denial consumed NOTHING: the speak stream is still ACTIVE and
    // its typed outcome is untouched.
    assert_eq!(
        metalogos::duplex::stream_id(&ch, "speak").expect("still active"),
        speak_stream
    );
    assert!(metalogos::duplex::stream_last_outcome(&ch, "speak").is_none());
}

// ── D4: typed completion and DUPLEX_IDLE ───────────────────────────────

#[test]
fn stops_complete_typed_and_idle_direction_refuses() {
    let session = fresh_session();
    let handle = open_channel(&session, None).expect("open");
    let ch = channel_id(&handle);
    let err =
        call_builtin("speak_stop", std::slice::from_ref(&handle)).expect_err("nothing to stop");
    assert!(err.contains("DUPLEX_IDLE"), "got: {err}");

    call_builtin("speak_start", &[handle.clone(), s("text"), s("normal")]).expect("speak");
    let stopped = call_builtin("speak_stop", std::slice::from_ref(&handle)).expect("stop");
    assert_eq!(str_field(&stopped, "outcome"), "completed");
    assert_eq!(str_field(&stopped, "direction"), "speak");
    // The slot is free; the terminal outcome is observable.
    assert!(metalogos::duplex::stream_id(&ch, "speak").is_none());
    match metalogos::duplex::stream_last_outcome(&ch, "speak").expect("terminal") {
        metalogos::duplex::StreamOutcome::Completed => {}
        other => panic!("expected Completed, got {other:?}"),
    }
}

#[test]
fn duplex_state_projects_both_directions_and_terminal_outcomes() {
    let session = fresh_session();
    let handle = open_channel(&session, Some("high")).expect("open");
    let _ch = channel_id(&handle);
    call_builtin("speak_start", &[handle.clone(), s("answer"), s("high")]).expect("speak");
    let st = call_builtin("duplex_state", std::slice::from_ref(&handle)).expect("state");
    assert_eq!(str_field(&st, "session"), session_id(&session));
    // speak active, listen idle (Unit), the terminal slots Unit.
    // The speak projection carries the outcome word.
    match &st {
        Value::Struct { fields, .. } => {
            let speak_active = fields.iter().find(|(k, _)| k.as_str() == "speak").unwrap();
            match &speak_active.1 {
                Value::Struct { fields, .. } => {
                    let outcome = fields
                        .iter()
                        .find(|(k, _)| k.as_str() == "outcome")
                        .unwrap();
                    match &outcome.1 {
                        Value::String(w) => assert_eq!(w, "active"),
                        other => panic!("outcome not a string: {}", other.type_name()),
                    }
                }
                other => panic!("speak not a Struct: {}", other.type_name()),
            }
            let listen_idle = fields.iter().find(|(k, _)| k.as_str() == "listen").unwrap();
            assert!(matches!(listen_idle.1, Value::Unit));
            let speak_last = fields
                .iter()
                .find(|(k, _)| k.as_str() == "speak_last")
                .unwrap();
            assert!(matches!(speak_last.1, Value::Unit));
        }
        other => panic!("expected Struct, got {}", other.type_name()),
    }
}

// ── D5: the audit completeness (the acceptance invariant) ──────────────

#[test]
fn interruption_loses_no_audit_event_differential_count() {
    let session = fresh_session();
    let handle = open_channel(&session, None).expect("open");
    let ch = channel_id(&handle);
    // Other tests run in PARALLEL against the process-global ledger —
    // the count is scoped to THIS channel's actor (the isolation that
    // makes the differential exact).
    // A fresh channel has exactly ONE record — its own open.
    assert_eq!(channel_records(&ch), vec!["duplex.open"]);

    // The scripted interleaving (the independent transition model):
    //   0 duplex_open                              -> 1  (duplex.open)
    //   1 speak_start normal                       -> 1  (duplex.speak_start, plain)
    //   2 listen_start normal  — BARGE-IN (equal)  -> 3  (preempt + interrupted + listen_start)
    //   3 speak_start high     — BARGE-IN          -> 3  (preempt + interrupted + speak_start)
    //   4 listen_start high    — BARGE-IN (equal)  -> 3  (preempt + interrupted + listen_start)
    //   5 listen_stop                              -> 1  (duplex.stop)
    //   6 duplex_state                             -> 1  (duplex.state)
    //   7 speak_start normal                       -> 1  (duplex.speak_start, plain — speak slot free, listen idle)
    //   8 speak_stop                               -> 1  (duplex.stop)
    //   9 listen_start low                         -> 1  (duplex.listen_start, plain)
    //  10 listen_stop                              -> 1  (duplex.stop)
    // Expected TOTAL: 1+1+3+3+3+1+1+1+1+1+1 = 17
    call_builtin("speak_start", &[handle.clone(), s("a"), s("normal")]).expect("1");
    call_builtin("listen_start", &[handle.clone(), s("normal")]).expect("2");
    call_builtin("speak_start", &[handle.clone(), s("b"), s("high")]).expect("3");
    call_builtin("listen_start", &[handle.clone(), s("high")]).expect("4");
    call_builtin("listen_stop", std::slice::from_ref(&handle)).expect("5");
    call_builtin("duplex_state", std::slice::from_ref(&handle)).expect("6");
    call_builtin("speak_start", &[handle.clone(), s("c"), s("normal")]).expect("7");
    call_builtin("speak_stop", std::slice::from_ref(&handle)).expect("8");
    call_builtin("listen_start", &[handle.clone(), s("low")]).expect("9");
    call_builtin("listen_stop", std::slice::from_ref(&handle)).expect("10");

    let actions: Vec<String> = channel_records(&ch);
    assert_eq!(
        actions.len(),
        17,
        "every transition recorded, preemptions double-recorded; got {}: {:?}",
        actions.len(),
        actions
    );
    for a in &actions {
        assert!(
            a.starts_with("duplex."),
            "a non-duplex record leaked into the family: {a}"
        );
    }
    // All three preemptions produced BOTH records (the decision AND the
    // terminal outcome) — nothing lost.
    let preempts = actions.iter().filter(|a| **a == "duplex.preempt").count();
    let interrupted = actions
        .iter()
        .filter(|a| **a == "duplex.interrupted")
        .count();
    assert_eq!(preempts, 3, "three barge-ins");
    assert_eq!(interrupted, 3, "each preempt carries its terminal record");
}

#[test]
fn the_registry_has_all_six_surfaces() {
    for name in [
        "duplex_open",
        "speak_start",
        "listen_start",
        "speak_stop",
        "listen_stop",
        "duplex_state",
    ] {
        assert!(
            BUILTIN_REGISTRY.iter().any(|sp| sp.name == name),
            "{name} registered"
        );
    }
}
