//! Naryad №430 (P1, security/session): the duty/session origin-stamp
//! audit — every session-surface refusal carries its stable origin stamp
//! (the №413 convention: position-0 `[CODE]` marker, never double-stamped),
//! so the `try` classifier branches the office's session policies on the
//! subsystem instead of the message text.
//!
//! The audit table (error → stamp → test):
//! | error                                             | stamp                 | test |
//! |---------------------------------------------------|-----------------------|------|
//! | unknown/ended session on any session-surface call | `[SESSION_UNKNOWN]`   | T1, T2 (login/duty/wake/interrupt/end paths) |
//! | unknown wake source (closed vocabulary, §4.1)     | `[SESSION_CONTRACT]`  | T3 |
//! | unknown interrupt priority (closed ladder, §4.2)  | `[SESSION_CONTRACT]`  | T4 |
//! | duty-profile compile violations (№349)            | compile-time diagnostic (never enters the `try` contour) | the leak suite n30–n33 (BLOCKING) |
//! | malformed Session handle args (type/shape)        | unstamped by design — arg-shape refusals are the honest `RUNTIME_ERROR` class everywhere in the registry | T5 (documents the line) |
//!
//! ADR-0172: Accepted → Implemented (the Evidence section pins the
//! runtime + static halves and the example).

use metalogos::builtins::BUILTIN_REGISTRY;
use metalogos::interpreter::Value;
use std::path::PathBuf;

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

/// T1 — an ENDED session (the stale handle) refused by every lifecycle
/// surface carries the position-0 `[SESSION_UNKNOWN]` stamp (the №413
/// convention). The surfaces take the Session HANDLE; after logout the
/// registry entry is gone — the fail-closed refusal is stamped.
#[test]
fn n430_unknown_session_is_origin_stamped() {
    let handle = call_builtin(
        "session_login",
        &[s("stamp-user-430c"), s("pw-not-checked-by-design")],
    )
    .expect("session_login");
    call_builtin("session_logout", std::slice::from_ref(&handle)).expect("logout ends the session");
    for (name, extra) in [
        ("session_poll_wake", vec![]),
        ("session_wake", vec![s("event"), s("p")]),
        ("session_interrupt", vec![s("normal"), s("r")]),
        ("session_duty_enter", vec![]),
        ("session_duty_exit", vec![]),
        ("session_logout", vec![]),
    ] {
        let mut args = vec![handle.clone()];
        args.extend(extra);
        let err = call_builtin(name, &args)
            .expect_err(&format!("{name} on an ended session must refuse"));
        assert!(
            err.starts_with("[SESSION_UNKNOWN] "),
            "{name}: the refusal must carry the position-0 stamp; got: {err}"
        );
    }
}

/// T2 — the same stamp survives the `try` classifier on BOTH backends:
/// the office branches on `error.code == "SESSION_UNKNOWN"`.
#[test]
fn n430_try_branches_on_session_unknown_parity() {
    let src = r#"
pattern T(_input: String) -> String {
  let sess = session_login("parity-430", "pw")
  let _ = session_logout(sess)
  let r = try session_poll_wake(sess)
  if r.ok == false { return "code:" + r.error.code }
  return "unreachable"
}
flow Main { input: String = "s" -> T -> output }
"#;
    let base = PathBuf::from("examples");
    let tw = metalogos::run_program_with_dir(src, base.to_path_buf())
        .expect("TW must succeed")
        .unwrap_or_default();
    let decls = metalogos::parser::parse(src).expect("parse");
    let mut comp = metalogos::compiler::Compiler::with_std_root(base.to_path_buf());
    let prog = comp.compile(decls).expect("compile");
    let mut vm = metalogos::vm::Vm::new();
    let vm_out = vm.run(prog).expect("VM must succeed").unwrap_or_default();
    assert_eq!(tw.trim_end(), "code:SESSION_UNKNOWN");
    assert_eq!(vm_out.trim_end(), "code:SESSION_UNKNOWN");
}

/// T3 — the wake vocabulary is CLOSED (ADR-0172 §4.1): an unknown source
/// is a typed `[SESSION_CONTRACT]` refusal.
#[test]
fn n430_unknown_wake_source_is_a_contract_refusal() {
    let live = call_builtin(
        "session_login",
        &[s("stamp-user-430"), s("pw-not-checked-by-design")],
    )
    .expect("session_login");
    let err = call_builtin("session_wake", &[live.clone(), s("telepathy"), s("p")])
        .expect_err("unknown wake source must refuse");
    assert!(err.starts_with("[SESSION_CONTRACT] "), "got: {err}");
    // the vocabulary itself still accepts a legal source
    call_builtin("session_wake", &[live, s("event"), s("hello")])
        .expect("a legal wake source succeeds");
}

/// T4 — the interrupt ladder is CLOSED (ADR-0172 §4.2): an unknown
/// priority is a typed `[SESSION_CONTRACT]` refusal.
#[test]
fn n430_unknown_interrupt_priority_is_a_contract_refusal() {
    let live = call_builtin(
        "session_login",
        &[s("stamp-user-430b"), s("pw-not-checked-by-design")],
    )
    .expect("session_login");
    let err = call_builtin("session_interrupt", &[live, s("ultra"), s("reason")])
        .expect_err("unknown priority must refuse");
    assert!(err.starts_with("[SESSION_CONTRACT] "), "got: {err}");
}

/// T5 — the documented line: malformed Session HANDLE args (type/shape)
/// stay unstamped (the honest RUNTIME_ERROR class — the same contract as
/// every registry builtin's arg refusals). This test PINS the line so the
/// taxonomy cannot silently drift.
#[test]
fn n430_malformed_handle_args_stay_unstamped() {
    let err = call_builtin("session_duty_enter", &[Value::Float(1.0)])
        .expect_err("a Float instead of a Session handle must refuse");
    assert!(
        !err.starts_with('['),
        "arg-shape refusals must stay unstamped (honest RUNTIME_ERROR); got: {err}"
    );
}

/// №16.0-D: no stubs in this test file (markers assembled from parts).
#[test]
fn n430_no_stubs() {
    let src = std::fs::read_to_string(file!()).unwrap_or_default();
    let bang = String::from("!");
    for m in ["todo", "unimplemented"] {
        let marker = format!("{}{}", m, bang);
        assert!(!src.contains(&marker), "stub marker {} found", marker);
    }
    let skeleton = ["SKELE", "TON"].concat();
    assert!(!src.contains(&skeleton), "stub marker (assembled) found");
}
