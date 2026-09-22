//! Naryad №428 (P1, security/voice): the audio consent gate + ledger on
//! the duplex path.
//!
//! speak/listen are DIRECTED audio effects: speak is egress (the agent's
//! voice out), listen is ingress (the caller's voice in). The media
//! principles (ADR-0145/№335 — no silent egress, consent-gated surfaces)
//! apply: both `speak_start` and `listen_start` require an ACTIVE consent
//! grant for their direction (`audio.speak` / `audio.listen`) recorded
//! through the №335 consent contour (`consent_grant`), fail-closed on any
//! store error. A refusal is TYPED (`AUDIO_CONSENT_REQUIRED`, the №413
//! origin-stamp convention — `try` can branch the ask-for-consent policy)
//! and is itself a ledger record (`duplex.speak_denied` / `duplex.listen_denied`)
//! — no silent egress AND no silent refusal. Consented flows keep the №352
//! audit completeness: open → start → stop all land in the Action Ledger.
//!
//! The barge-in state machine (№352/ADR-0174 §3) is untouched — its
//! contract runs in tests/naryad_352_duplex.rs with the consent harness.
//!
//! ISOLATION: the consent store is process-global (the SQLite consent
//! ledger), so every gate-dependent test holds the shared GATE_LOCK — the
//! grants/revocations of one test never race another's refusals.

use metalogos::builtins::BUILTIN_REGISTRY;
use metalogos::interpreter::Value;
use metalogos::ledger::all_records;
use std::sync::{Mutex, MutexGuard, OnceLock};

fn gate_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

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

fn fresh_channel() -> Value {
    let sess = call_builtin(
        "session_login",
        &[s(&unique("consent-user")), s("pw-not-checked-by-design")],
    )
    .expect("session_login");
    call_builtin("duplex_open", &[sess, s("normal")]).expect("duplex_open")
}

fn grant(scope: &str) {
    call_builtin(
        "consent_grant",
        &[s("n428"), s(scope), s("consent-harness")],
    )
    .unwrap_or_else(|e| panic!("consent_grant({scope}) must succeed: {e}"));
}

fn revoke(scope: &str) {
    call_builtin("consent_revoke", &[s("n428"), s(scope)])
        .unwrap_or_else(|e| panic!("consent_revoke({scope}) must succeed: {e}"));
}

fn count_kind(kinds: &[String], kind: &str) -> usize {
    kinds.iter().filter(|k| k.as_str() == kind).count()
}

fn ledger_kinds() -> Vec<String> {
    all_records()
        .expect("action ledger must be readable")
        .into_iter()
        .map(|r| r.action)
        .collect()
}

/// T1 — the NEGATIVE: speak without consent is a typed, origin-stamped
/// refusal; the refusal itself is in the ledger; NO speak_start record
/// may appear for the refused call (no silent egress happened).
/// The gate is re-armed explicitly — the consent ledger is process-global.
#[test]
fn n428_speak_without_consent_is_typed_and_audited() {
    let _g = gate_lock();
    revoke("audio.speak");
    let ch = fresh_channel();
    let before = ledger_kinds();
    let err = call_builtin("speak_start", &[ch.clone(), s("no consent"), s("normal")])
        .expect_err("speak without consent must refuse");
    assert!(
        err.starts_with("[AUDIO_CONSENT_REQUIRED] "),
        "the refusal must carry the stable origin stamp; got: {err}"
    );
    let after = ledger_kinds();
    assert_eq!(
        count_kind(&after, "duplex.speak_denied"),
        count_kind(&before, "duplex.speak_denied") + 1,
        "the refusal must land in the ledger exactly once (duplex.speak_denied)"
    );
    assert_eq!(
        count_kind(&after, "duplex.speak_start"),
        count_kind(&before, "duplex.speak_start"),
        "no speak_start record may exist for the refused call"
    );
}

/// T2 — the POSITIVE: after `consent_grant` the same speak_start succeeds
/// and the egress is audited (a duplex.speak_start record appears).
#[test]
fn n428_consented_speak_flows_and_is_audited() {
    let _g = gate_lock();
    grant("audio.speak");
    let ch = fresh_channel();
    let before = ledger_kinds();
    call_builtin("speak_start", &[ch, s("consented"), s("normal")])
        .expect("speak with consent must succeed");
    let after = ledger_kinds();
    assert_eq!(
        count_kind(&after, "duplex.speak_start"),
        count_kind(&before, "duplex.speak_start") + 1,
        "the consented egress must be audited"
    );
}

/// T3 — the LISTEN direction mirrors the contract: without its own grant
/// the listen_start refuses typed even when the speak direction IS granted;
/// after the listen grant the flow opens.
#[test]
fn n428_listen_is_gated_independently() {
    let _g = gate_lock();
    revoke("audio.listen");
    let ch = fresh_channel();
    grant("audio.speak"); // the OTHER direction only
    let err =
        call_builtin("listen_start", &[ch]).expect_err("listen without audio.listen must refuse");
    assert!(err.starts_with("[AUDIO_CONSENT_REQUIRED] "), "got: {err}");
    grant("audio.listen");
    let ch2 = fresh_channel();
    call_builtin("listen_start", &[ch2]).expect("listen with consent must succeed");
}

/// T4 — revocation re-arms the gate: after `consent_revoke` the same
/// direction refuses again (the grant is no longer active).
#[test]
fn n428_revoke_re_arms_the_gate() {
    let _g = gate_lock();
    grant("audio.speak");
    let ch = fresh_channel();
    call_builtin("speak_start", &[ch.clone(), s("ok"), s("normal")])
        .expect("consented speak must succeed");
    call_builtin("speak_stop", std::slice::from_ref(&ch)).expect("speak_stop");
    revoke("audio.speak");
    let err = call_builtin("speak_start", &[ch, s("after revoke"), s("normal")])
        .expect_err("speak after revocation must refuse");
    assert!(err.starts_with("[AUDIO_CONSENT_REQUIRED] "), "got: {err}");
}

/// T5 — the registry surface is unchanged: the duplex builtins keep their
/// №352 arities (the gate adds no new surface).
#[test]
fn n428_registry_arity_unchanged() {
    let expect = [
        ("duplex_open", 1, Some(2)),
        ("speak_start", 2, Some(3)),
        ("listen_start", 1, Some(2)),
        ("speak_stop", 1, None),
        ("listen_stop", 1, None),
        ("duplex_state", 1, None),
    ];
    for (name, arity, max) in expect {
        let spec = BUILTIN_REGISTRY
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("{name} must be registered"));
        assert_eq!(spec.arity, arity, "{name} min arity");
        assert_eq!(spec.max_arity, max, "{name} max arity");
    }
}

/// T6 — the consent refusal is catchable by `try` on BOTH backends and
/// branches on the stable code (the office ask-for-consent policy shape).
/// Serial: the language-level run needs NO grant to exist — the whole
/// process must not have one from a concurrent test, hence the lock.
#[test]
fn n428_try_branches_on_the_stable_code() {
    let _g = gate_lock();
    revoke("audio.speak"); // ensure the gate is armed for this process
    let src = r#"
pattern T(_input: String) -> String {
  let sess = session_login("try-consent-user", "pw")
  let ch = duplex_open(sess, "normal")
  let r = try speak_start(ch, "no consent here", "normal")
  if r.ok == false { return "code:" + r.error.code }
  return "unreachable"
}
flow Main { input: String = "s" -> T -> output }
"#;
    let base = std::path::PathBuf::from("examples");
    let tw = metalogos::run_program_with_dir(src, base.to_path_buf())
        .expect("TW must succeed")
        .unwrap_or_default();
    let decls = metalogos::parser::parse(src).expect("parse");
    let mut comp = metalogos::compiler::Compiler::with_std_root(base.to_path_buf());
    let prog = comp.compile(decls).expect("compile");
    let mut vm = metalogos::vm::Vm::new();
    let vm_out = vm.run(prog).expect("VM must succeed").unwrap_or_default();
    assert_eq!(tw.trim_end(), "code:AUDIO_CONSENT_REQUIRED");
    assert_eq!(vm_out.trim_end(), "code:AUDIO_CONSENT_REQUIRED");
}

/// T7 — the clock-granularity regression (№428 surfaced it through the
/// golden runner): a grant recorded THE SAME SECOND as the revoke it
/// supersedes must be ACTIVE (row order is the ledger truth, not the
/// second-resolution wall clock). This test revokes and re-grants
/// immediately — within one second — and the speak flow must open.
#[test]
fn n428_same_second_grant_after_revoke_is_active() {
    let _g = gate_lock();
    revoke("audio.speak"); // arm the gate
    grant("audio.speak"); // re-grant IMMEDIATELY (same wall-clock second)
    let ch = fresh_channel();
    call_builtin("speak_start", &[ch, s("same-second"), s("normal")])
        .expect("a grant newer than the revoke must be active even within the same second");
}

/// №16.0-D: no stubs in this test file (markers assembled from parts).
#[test]
fn n428_no_stubs() {
    let src = std::fs::read_to_string(file!()).unwrap_or_default();
    let bang = String::from("!");
    for m in ["todo", "unimplemented"] {
        let marker = format!("{}{}", m, bang);
        assert!(!src.contains(&marker), "stub marker {} found", marker);
    }
    let skeleton = ["SKELE", "TON"].concat();
    assert!(!src.contains(&skeleton), "stub marker (assembled) found");
}
