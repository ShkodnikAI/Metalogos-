//! Naryad №392 (issue #486) — DenyEvent: the typed deny-event layer.
//!
//! Contract under test (the naryad's "Сделано, когда" items):
//! (а) the seven core reasons are typed (`crate::deny::DENY_REASONS`), a
//!     runtime refusal is handled by an on_deny handler and the event
//!     carries an explainable reason;
//! (б) the exhaustive check catches an incomplete Match over deny_reason()
//!     (the `.error` example is red with the unhandled-reason list);
//! (в) all existing red examples stay red — a handler never weakens a
//!     static deny (deny behavior by default is unchanged);
//! (г) deny_event()/deny_reason() outside an on_deny handler are a
//!     compile error — the event is runtime-constructed, it cannot be
//!     forged;
//! (д) TW and VM agree on the handled-deny path (the №328 agreement
//!     principle, extended to events).
//!
//! How to verify manually: `cargo test --test naryad_392_deny_event`.

use std::path::Path;

/// Execute via tree-walking interpreter (the crosscheck's run_tw).
fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, Path::new("examples").to_path_buf())
}

/// Execute via the bytecode VM (compile + run — the crosscheck's run_vm).
fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp =
        metalogos::compiler::Compiler::with_std_root(Path::new("examples").to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

fn check_errors(source: &str) -> Vec<String> {
    let declarations = metalogos::parser::parse(source).expect("parse must succeed");
    metalogos::semantic::check_program(&declarations)
        .errors
        .iter()
        .map(|e| e.message.clone())
        .collect()
}

/// The shared scenario: an N(1) grant meters ONE destructive DELETE; the
/// second granted call refuses (GRANT_EXHAUSTED — runtime state the static
/// Once-linearity cannot see) and the on_deny(db) handler degrades it.
const HANDLED_SCENARIO: &str = r#"
on_deny(db) {
  match deny_reason() {
    "VOICE_EGRESS_UNCONSENTED" then { print("deny:VOICE_EGRESS_UNCONSENTED") }
    "IRREVERSIBLE_NO_GRANT" then { print("deny:IRREVERSIBLE_NO_GRANT") }
    "UNTRUSTED_EXEC_DECISION" then { print("deny:UNTRUSTED_EXEC_DECISION") }
    "SECRET_TO_EXEC" then { print("deny:SECRET_TO_EXEC") }
    "SECRET_EGRESS_VCS" then { print("deny:SECRET_EGRESS_VCS") }
    "SECRET_EGRESS_NETWORK" then { print("deny:SECRET_EGRESS_NETWORK") }
    "PII_EGRESS_NETWORK" then { print("deny:PII_EGRESS_NETWORK") }
    "PII_EGRESS_OUTPUT" then { print("deny:PII_EGRESS_OUTPUT") }
    "UNTRUSTED_EGRESS_NETWORK" then { print("deny:UNTRUSTED_EGRESS_NETWORK") }
    "MEDIA_SEALED_EGRESS" then { print("deny:MEDIA_SEALED_EGRESS") }
    "SINK_CLEARANCE" then { print("deny:SINK_CLEARANCE") }
    "HTML_INJECTION" then { print("deny:HTML_INJECTION") }
    "TAINT_PERSISTENCE" then { print("deny:TAINT_PERSISTENCE") }
    "SECRET_LEAK" then { print("deny:SECRET_LEAK") }
  }
}

db { url: "sqlite::memory:" }

pattern GrantDenyFlow(_tick: String) -> String {
  db_execute("CREATE TABLE IF NOT EXISTS notes (id INTEGER PRIMARY KEY, user TEXT)")
  db_execute("INSERT INTO notes (user) VALUES ('alice')")
  let g = grant_issue("db:delete:notes", 60, "n", 1)
  let n1 = db_execute_with_grant(g, "DELETE FROM notes WHERE user = 'alice'")
  let n2 = db_execute_with_grant(g, "DELETE FROM notes")
  return "granted:" + n1 + "|second-call-refused:" + type_of(n2)
}
flow Main { input: String = "tick" -> GrantDenyFlow -> output }
"#;

// ── (а) the handler catches the runtime refusal, the reason is real ──

#[test]
fn n392_handled_grant_refusal_degrades_and_continues_tw() {
    let out = run_tw(HANDLED_SCENARIO).expect("handled refusal must not abort the program");
    let out = out.expect("flow output present");
    assert!(
        out.contains("granted:1"),
        "the allow path ran exactly once: {out}"
    );
    assert!(
        out.contains("second-call-refused:Unit"),
        "the refused call degraded to Unit and execution continued: {out}"
    );
}

#[test]
fn n392_grant_refusal_without_handler_stays_loud() {
    let bare = HANDLED_SCENARIO.replace(
        "on_deny(db) {",
        "on_deny(voice) {", // a handler for a DIFFERENT class does not cover db
    );
    // The voice-class handler cannot handle a db refusal — loud default.
    let err = run_tw(&bare).expect_err("uncovered refusal stays a loud error");
    assert!(
        err.contains("GRANT_EXHAUSTED"),
        "the typed grant error surfaces unchanged: {err}"
    );
}

#[test]
fn n392_event_reason_matches_the_audit_class_vocabulary() {
    // The reason the handler matched is IRREVERSIBLE_NO_GRANT — the same
    // class the №325 gate prints for ungranted destructive SQL (the grant
    // refusals fold there, the typed GRANT_* detail rides in `human`).
    let covered: Vec<String> = metalogos::deny::DENY_REASONS
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert!(covered.contains(&"IRREVERSIBLE_NO_GRANT".to_string()));
    // The seven core classes of the №392 contract.
    for core in [
        "VOICE_EGRESS_UNCONSENTED",
        "IRREVERSIBLE_NO_GRANT",
        "UNTRUSTED_EXEC_DECISION",
        "SECRET_TO_EXEC",
        "SECRET_EGRESS_VCS",
        "SECRET_EGRESS_NETWORK",
        "PII_EGRESS_NETWORK",
    ] {
        assert!(metalogos::deny::is_known_reason(core));
    }
}

// ── (б) exhaustive matching over the reason enum ─────────────────────

#[test]
fn n392_exhaustive_match_passes_when_complete() {
    let errors = check_errors(HANDLED_SCENARIO);
    let deny_errors: Vec<_> = errors.iter().filter(|e| e.contains("[DENY_")).collect();
    assert!(
        deny_errors.is_empty(),
        "a match covering every reason passes: {deny_errors:?}"
    );
}

#[test]
fn n392_incomplete_match_is_a_compile_error_listing_reasons() {
    let src = r#"
on_deny(db) {
  match deny_reason() {
    "SECRET_TO_EXEC" then { print("deny:SECRET_TO_EXEC") }
  }
}
"#;
    let errors = check_errors(src);
    let exhaustive: Vec<_> = errors
        .iter()
        .filter(|e| e.contains("[DENY_MATCH_EXHAUSTIVE]"))
        .collect();
    assert_eq!(
        exhaustive.len(),
        1,
        "exactly one exhaustive error: {errors:?}"
    );
    // The error lists unhandled reasons (spot-check three from different groups).
    for reason in [
        "VOICE_EGRESS_UNCONSENTED",
        "PII_EGRESS_OUTPUT",
        "SECRET_LEAK",
    ] {
        assert!(
            exhaustive[0].contains(reason),
            "unhandled list must contain {reason}: {}",
            exhaustive[0]
        );
    }
    // ...and NOT the covered reason.
    assert!(
        !exhaustive[0].contains("SECRET_TO_EXEC)"),
        "covered reason not listed"
    );
}

#[test]
fn n392_else_arm_satisfies_the_exhaustive_check() {
    let src = r#"
on_deny(db) {
  match deny_reason() {
    "SECRET_TO_EXEC" then { print("deny:SECRET_TO_EXEC") }
    else { print("deny:other") }
  }
}
"#;
    let errors = check_errors(src);
    assert!(
        errors
            .iter()
            .all(|e| !e.contains("[DENY_MATCH_EXHAUSTIVE]")),
        "an else arm is the wildcard: {errors:?}"
    );
}

#[test]
fn n392_unknown_reason_literal_is_rejected() {
    let src = r#"
on_deny(db) {
  match deny_reason() {
    "SECRET_TO_EXECC" then { print("typo") }
    else { print("other") }
  }
}
"#;
    let errors = check_errors(src);
    assert!(
        errors.iter().any(|e| e.contains("[DENY_MATCH_UNKNOWN]")),
        "a typo can never silently match nothing: {errors:?}"
    );
}

#[test]
fn n392_deny_class_validation_and_duplicates() {
    let src = "on_deny(everything) { print(\"x\") }";
    let errors = check_errors(src);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("[DENY_CLASS] unknown on_deny class")),
        "unknown class word is loud: {errors:?}"
    );

    let src = "on_deny(db) { print(\"a\") }\non_deny(db) { print(\"b\") }";
    let errors = check_errors(src);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("duplicate on_deny handler")),
        "duplicate class handler is loud: {errors:?}"
    );
}

// ── (г) DenyEvent cannot be forged ────────────────────────────────────

#[test]
fn n392_deny_builtins_outside_handler_are_compile_errors() {
    let src = r#"
pattern Leaky() -> String {
  let ev = deny_event()
  return "forged:" + ev
}
"#;
    let errors = check_errors(src);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("[DENY_HANDLER_SCOPE] deny_event()")),
        "deny_event outside a handler is a compile error: {errors:?}"
    );

    let src = r#"
pattern Leaky() -> String {
  return deny_reason()
}
"#;
    let errors = check_errors(src);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("[DENY_HANDLER_SCOPE] deny_reason()")),
        "deny_reason outside a handler is a compile error: {errors:?}"
    );
}

#[test]
fn n392_deny_builtins_inside_handler_are_allowed() {
    let src = r#"
on_deny(*) {
  let ev = deny_event()
  let r = deny_reason()
  print("reason:" + r)
}
pattern Ok() -> String {
  return "fine"
}
flow Main { input: String = "x" -> Ok -> output }
"#;
    let errors = check_errors(src);
    assert!(
        errors.iter().all(|e| !e.contains("[DENY_")),
        "handler-scoped usage is legal: {errors:?}"
    );
}

// ── (д) TW/VM agreement on the handled-deny path ─────────────────────

#[test]
fn n392_tw_vm_parity_on_handled_refusal() {
    let tw = run_tw(HANDLED_SCENARIO).expect("TW run");
    let vm = run_vm(HANDLED_SCENARIO).expect("VM run");
    assert_eq!(
        tw, vm,
        "both backends emit the same degraded continuation (the event itself goes to stderr observability, the return value is the contract)"
    );
    assert_eq!(tw, Some("granted:1|second-call-refused:Unit".to_string()));
}

// ── (в) a handler never weakens a static deny ────────────────────────

#[test]
fn n392_handler_does_not_weaken_the_static_gate() {
    // A private secret into respond() is a STATIC SECRET_LEAK — an
    // on_deny(*) handler must not turn it into a handled runtime event.
    let src = r#"
on_deny(*) {
  print("deny handled")
}
pattern Leak(k: String) -> String {
  let key = env("FAKE_API_KEY")
  respond("200", "key is " + key)
  return key
}
"#;
    let err = run_tw(src).expect_err("static deny stays loud even with a handler");
    assert!(
        err.contains("SECRET_LEAK") || err.contains("PII_EGRESS_OUTPUT"),
        "the static class surfaces: {err}"
    );
}

// ── Serialization: handlers survive the .mbc roundtrip ───────────────

#[test]
fn n392_deny_handlers_survive_bytecode_roundtrip() {
    let declarations = metalogos::parser::parse(HANDLED_SCENARIO).expect("parse");
    let mut comp = metalogos::compiler::Compiler::new();
    let program = comp.compile(declarations).expect("compile");
    assert_eq!(program.deny_handlers.len(), 1);
    assert_eq!(program.deny_handlers[0].class, "db");

    let bytes = program.serialize().expect("serialize");
    let restored = metalogos::bytecode::Program::deserialize(&bytes).expect("deserialize");
    assert_eq!(restored.deny_handlers.len(), 1);
    assert_eq!(restored.deny_handlers[0].class, "db");
    assert!(!restored.deny_handlers[0].code.is_empty());
}

// ── Golden examples: the .mlog/.expected/.error contracts ────────────

#[test]
fn n392_w2_deny_exhaustive_golden() {
    let src = std::fs::read_to_string("examples/w2_deny_exhaustive.mlog").expect("example");
    let expected =
        std::fs::read_to_string("examples/w2_deny_exhaustive.expected").expect("expected");
    let out = run_tw(&src).expect("handled scenario runs");
    assert_eq!(
        out.as_deref(),
        Some(expected.trim()),
        "golden contract: the deny log precedes the degraded continuation"
    );
}

#[test]
fn n392_w2_deny_incomplete_error_golden() {
    let src = std::fs::read_to_string("examples/w2_deny_incomplete.mlog").expect("example");
    let expected = std::fs::read_to_string("examples/w2_deny_incomplete.error").expect("error");
    let err = run_tw(&src).expect_err("incomplete match is red");
    assert!(err.contains(expected.trim()), "the .error contract: {err}");
}
