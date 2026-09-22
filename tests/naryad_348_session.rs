// ── Naryad #348 (P1, feature/session, ADR-0172): session contract ─────
//
// The real session surface replaces the §16.0-6(б) pillar stubs. The
// contracts pinned here:
//   S1  lifecycle: login mints a live registered session; duty enter/exit
//       flip the flag; logout ends it — every later op refuses with the
//       typed SESSION_UNKNOWN (fail-closed, no implicit recreate);
//   S2  ledger 100% coverage: EVERY transition of a session lands in the
//       Action Ledger — asserted per-session via the actor filter (the
//       session id is unique per login, so parallel tests cannot
//       cross-contaminate the counts);
//   S3  wake: closed source vocabulary (keyword|event|schedule), FIFO
//       delivery, Unit on empty (no panic);
//   S4  interrupts: closed priority ladder (low<normal<high<critical),
//       highest-first take with FIFO within a rank (the №352 lever);
//   S5  the duty profile carrier: `profile duty { ... }` validates,
//       resolves into the ResolvedProfiles flags, and rejects unknown
//       words loudly (closed vocabulary — the ADR-0161 discipline);
//   S6  the value surface: the Session handle carries id/user/duty.

use metalogos::builtins::BUILTIN_REGISTRY;
use metalogos::interpreter::{SecretString, Value};
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

/// session_login for a fresh user — the username doubles as the
/// per-test marker (ids are sha-derived and unique anyway).
fn login(user: &str) -> Value {
    call_builtin("session_login", &[s(user), s("pw-not-verified")])
        .expect("session_login must succeed")
}

/// Extract the opaque handle's id field (the registry key).
fn id_of(handle: &Value) -> String {
    match handle {
        Value::Session(map) => map
            .get("id")
            .cloned()
            .unwrap_or_else(|| panic!("session handle has no id")),
        other => panic!("expected Session handle, got {}", other.type_name()),
    }
}

fn session_records(id: &str) -> Vec<(String, String)> {
    all_records()
        .expect("ledger readable")
        .into_iter()
        .filter(|r| r.actor == id)
        .map(|r| (r.action, r.args_hash))
        .collect()
}

fn actions_of(id: &str) -> Vec<String> {
    session_records(id).into_iter().map(|(a, _)| a).collect()
}

// ── S1: lifecycle + fail-closed end ───────────────────────────────────

#[test]
fn lifecycle_login_duty_logout_and_session_unknown() {
    let handle = login("alice-lifecycle");
    let id = id_of(&handle);
    assert!(
        metalogos::session::is_live(&id),
        "session must be registered after login"
    );

    // Duty enter → true (flipped), again → false (idempotent flip report).
    let d1 = call_builtin("session_duty_enter", std::slice::from_ref(&handle)).unwrap();
    assert_eq!(format!("{:?}", d1), "Bool(true)");
    let d2 = call_builtin("session_duty_enter", std::slice::from_ref(&handle)).unwrap();
    assert_eq!(format!("{:?}", d2), "Bool(false)");
    let d3 = call_builtin("session_duty_exit", std::slice::from_ref(&handle)).unwrap();
    assert_eq!(format!("{:?}", d3), "Bool(true)");

    // The handle projection carries duty=0 again after exit.
    match &handle {
        Value::Session(map) => assert_eq!(map.get("duty").unwrap(), "0"),
        other => panic!("expected Session, got {}", other.type_name()),
    }

    // Logout ends the session; every later op refuses with SESSION_UNKNOWN.
    call_builtin("session_logout", std::slice::from_ref(&handle)).expect("first logout ok");
    assert!(!metalogos::session::is_live(&id));
    let err = call_builtin("session_logout", std::slice::from_ref(&handle)).unwrap_err();
    assert!(err.contains("SESSION_UNKNOWN"), "double logout: {}", err);
    let err = call_builtin("session_wake", &[handle.clone(), s("event")]).unwrap_err();
    assert!(err.contains("SESSION_UNKNOWN"), "wake after end: {}", err);
    let err = call_builtin("session_duty_enter", std::slice::from_ref(&handle)).unwrap_err();
    assert!(err.contains("SESSION_UNKNOWN"), "duty after end: {}", err);
}

// ── S2: ledger 100% coverage of the transition sequence ──────────────

#[test]
fn every_transition_lands_in_the_action_ledger() {
    let handle = login("bob-ledger");
    let id = id_of(&handle);

    call_builtin("session_duty_enter", std::slice::from_ref(&handle)).unwrap();
    call_builtin("session_wake", &[handle.clone(), s("keyword"), s("k1")]).unwrap();
    call_builtin("session_wake", &[handle.clone(), s("schedule"), s("cron")]).unwrap();
    call_builtin("session_poll_wake", std::slice::from_ref(&handle)).unwrap();
    call_builtin(
        "session_interrupt",
        &[handle.clone(), s("high"), s("barge")],
    )
    .unwrap();
    call_builtin("session_take_interrupt", std::slice::from_ref(&handle)).unwrap();
    call_builtin("session_duty_exit", std::slice::from_ref(&handle)).unwrap();
    call_builtin("session_logout", std::slice::from_ref(&handle)).unwrap();

    let actions = actions_of(&id);
    // The exact transition sequence, in order — 100% coverage, no gaps:
    // create, duty_enter, wake, wake, wake_delivered, interrupt,
    // interrupt_taken, duty_exit, end. The empty poll (below) records
    // nothing because it delivers nothing.
    assert_eq!(
        actions,
        vec![
            "session.create",
            "session.duty_enter",
            "session.wake",
            "session.wake",
            "session.wake_delivered",
            "session.interrupt",
            "session.interrupt_taken",
            "session.duty_exit",
            "session.end",
        ],
        "every session transition must be a ledger record (in order)"
    );

    // The wake record carries the args preimage ONLY as its SHA-256
    // (the ADR-0167 §2 driver-6 confidentiality rule — payloads never
    // enter the journal as text).
    let (_, wake_args_hash) = session_records(&id)
        .into_iter()
        .find(|(a, _)| a == "session.wake")
        .expect("wake record exists");
    assert_eq!(
        wake_args_hash,
        metalogos::ledger::args_hash_of("source=keyword|k1"),
        "wake args_hash must be the SHA-256 of the record detail preimage"
    );
}

// ── S3: wake FIFO + closed vocabulary ─────────────────────────────────

#[test]
fn wake_fifo_delivery_and_closed_sources() {
    let handle = login("carol-wake");
    let id = id_of(&handle);

    // Empty poll → Unit (typed), no record (nothing delivered).
    let empty = call_builtin("session_poll_wake", std::slice::from_ref(&handle)).unwrap();
    assert_eq!(format!("{:?}", empty), "Unit");

    call_builtin("session_wake", &[handle.clone(), s("event"), s("second")]).unwrap();
    call_builtin("session_wake", &[handle.clone(), s("keyword"), s("first")]).unwrap();

    let w1 = call_builtin("session_poll_wake", std::slice::from_ref(&handle)).unwrap();
    match w1 {
        Value::Struct { type_name, fields } => {
            assert_eq!(type_name, "Wake");
            assert_eq!(
                format!("{:?}", fields.get("source").unwrap()),
                r#"String("event")"#
            );
            assert_eq!(
                format!("{:?}", fields.get("payload").unwrap()),
                r#"String("second")"#
            );
        }
        other => panic!("expected Wake struct, got {}", other.type_name()),
    }
    // FIFO: the second delivery is the keyword wake.
    let w2 = call_builtin("session_poll_wake", std::slice::from_ref(&handle)).unwrap();
    match w2 {
        Value::Struct { fields, .. } => {
            assert_eq!(
                format!("{:?}", fields.get("source").unwrap()),
                r#"String("keyword")"#
            );
        }
        other => panic!("expected Wake struct, got {}", other.type_name()),
    }
    let w3 = call_builtin("session_poll_wake", std::slice::from_ref(&handle)).unwrap();
    assert_eq!(format!("{:?}", w3), "Unit");

    // Closed vocabulary: unknown sources are loud errors.
    let err = call_builtin("session_wake", &[handle.clone(), s("tcp"), s("x")]).unwrap_err();
    assert!(err.contains("unknown wake source"), "bad source: {}", err);
    let _ = call_builtin("session_logout", &[handle]);
    assert_eq!(actions_of(&id).last().unwrap(), "session.end");
}

// ── S4: typed interrupt priorities — highest first, FIFO within ──────

#[test]
fn interrupt_priority_preemption_order() {
    let handle = login("dave-interrupt");
    let id = id_of(&handle);

    call_builtin(
        "session_interrupt",
        &[handle.clone(), s("low"), s("ambient")],
    )
    .unwrap();
    call_builtin(
        "session_interrupt",
        &[handle.clone(), s("critical"), s("shutdown")],
    )
    .unwrap();
    call_builtin(
        "session_interrupt",
        &[handle.clone(), s("high"), s("barge-in")],
    )
    .unwrap();
    call_builtin(
        "session_interrupt",
        &[handle.clone(), s("high"), s("second-barge")],
    )
    .unwrap();

    // Highest rank first; within one rank FIFO (second-barge after
    // barge-in).
    let order: Vec<String> = ["t1", "t2", "t3", "t4"]
        .iter()
        .map(|_| call_builtin("session_take_interrupt", std::slice::from_ref(&handle)).unwrap())
        .map(|v| match v {
            Value::Struct { fields, .. } => match fields.get("priority") {
                Some(Value::String(p)) => p.clone(),
                other => panic!("expected priority string, got {:?}", other),
            },
            Value::Unit => "Unit".to_string(),
            other => panic!("expected Struct|Unit, got {}", other.type_name()),
        })
        .collect();
    assert_eq!(
        order,
        vec!["critical", "high", "high", "low"],
        "preemption must take the highest priority first, FIFO within"
    );
    // Fifth take → Unit (queue drained, no panic).
    let t5 = call_builtin("session_take_interrupt", std::slice::from_ref(&handle)).unwrap();
    assert_eq!(format!("{:?}", t5), "Unit");

    // Closed ladder: unknown priorities refuse loudly.
    let err =
        call_builtin("session_interrupt", &[handle.clone(), s("urgent"), s("x")]).unwrap_err();
    assert!(err.contains("unknown priority"), "bad priority: {}", err);
    let _ = call_builtin("session_logout", &[handle]);
    let interrupt_records: Vec<String> = actions_of(&id)
        .into_iter()
        .filter(|a| a == "session.interrupt" || a == "session.interrupt_taken")
        .collect();
    assert_eq!(
        interrupt_records.len(),
        8,
        "4 enqueued + 4 taken = 8 records"
    );
}

// ── S5: the duty profile carrier (static half) ────────────────────────

#[test]
fn duty_profile_resolves_and_validates_loudly() {
    use metalogos::parser;

    let src = r#"
profile duty { materialization: denied surfaces: local_only }
pattern P() -> String { return "x" }
"#;
    let decls = parser::parse(src).expect("duty profile parses");
    let resolved = metalogos::profile::resolve(&decls);
    assert!(
        resolved.duty_materialization_denied && resolved.duty_surfaces_local_only,
        "both duty knobs must resolve"
    );
    // Semantic validation accepts the closed vocabulary.
    let analysis = metalogos::semantic::check_program(&decls);
    assert!(
        analysis.is_ok(),
        "duty profile must pass semantic validation: {:?}",
        analysis.errors.iter().take(3).collect::<Vec<_>>()
    );

    // One knob only — resolves independently.
    let src = r#"
profile duty { materialization: denied }
pattern P() -> String { return "x" }
"#;
    let decls = parser::parse(src).expect("single-knob duty parses");
    let resolved = metalogos::profile::resolve(&decls);
    assert!(resolved.duty_materialization_denied);
    assert!(!resolved.duty_surfaces_local_only);

    // Unknown word — loud error (closed vocabulary, ADR-0161 discipline).
    let src = r#"
profile duty { materialization: allowed }
pattern P() -> String { return "x" }
"#;
    let decls = parser::parse(src).expect("parses");
    let analysis = metalogos::semantic::check_program(&decls);
    assert!(
        !analysis.is_ok(),
        "unknown duty mode must be a loud semantic error"
    );

    // Unknown option key — loud error.
    let src = r#"
profile duty { network: open }
pattern P() -> String { return "x" }
"#;
    let decls = parser::parse(src).expect("parses");
    let analysis = metalogos::semantic::check_program(&decls);
    assert!(
        !analysis.is_ok(),
        "unknown duty option must be a loud semantic error"
    );

    // Empty options — loud error (a duty declaration without knobs
    // would be a silent no-op).
    let src = r#"
profile duty { }
pattern P() -> String { return "x" }
"#;
    let decls = parser::parse(src).expect("parses");
    let analysis = metalogos::semantic::check_program(&decls);
    assert!(
        !analysis.is_ok(),
        "empty duty profile must be a loud semantic error"
    );
}

// ── S6: the value surface + typed refusals on bad arguments ──────────

#[test]
fn session_value_surface_and_argument_typing() {
    let handle = login("erin-surface");
    match &handle {
        Value::Session(map) => {
            assert_eq!(map.get("user").unwrap(), "erin-surface");
            assert_eq!(map.get("duty").unwrap(), "0");
            let id = map.get("id").unwrap();
            assert!(id.starts_with("s-"), "session id shape: {}", id);
            assert_eq!(id.len(), 18, "s- + 16 hex chars");
        }
        other => panic!("expected Session, got {}", other.type_name()),
    }

    // Bad handle type → loud typing error (no panics).
    let err = call_builtin("session_duty_enter", &[s("not-a-session")]).unwrap_err();
    assert!(err.contains("expected Session"), "bad handle: {}", err);

    // Password accepts Secret as well as String (№274 surface).
    let secret_login = call_builtin(
        "session_login",
        &[
            s("fred-secret"),
            Value::Secret(SecretString::new("hunter2".to_string())),
        ],
    )
    .expect("Secret password accepted");
    let _ = call_builtin("session_logout", &[secret_login]).unwrap();

    let _ = call_builtin("session_logout", &[handle]).unwrap();
}

// ── Registry hygiene: append-only rows, category, arity ──────────────

#[test]
fn registry_rows_are_real_session_surface() {
    let names: Vec<&str> = BUILTIN_REGISTRY.iter().map(|s| s.name).collect();
    for n in [
        "session_login",
        "session_logout",
        "session_duty_enter",
        "session_duty_exit",
        "session_wake",
        "session_poll_wake",
        "session_interrupt",
        "session_take_interrupt",
    ] {
        assert!(names.contains(&n), "{} must be registered", n);
    }
    // The rows moved OUT of the stub category — the session surface is
    // real (the §16.0-6(б) stub ledger shrinks by two rows).
    for n in ["session_login", "session_logout"] {
        let spec = BUILTIN_REGISTRY.iter().find(|s| s.name == n).unwrap();
        assert_eq!(
            spec.category, "session",
            "{} must leave the stub category",
            n
        );
    }
}
