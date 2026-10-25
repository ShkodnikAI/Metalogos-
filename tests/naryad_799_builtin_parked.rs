// ── Naryad №799 (issue #1546, wave 65, P1 — the C3 (a) T2 decision) ────
//
// The parked rows refuse at CHECK time: `[BUILTIN_PARKED] <name> requires
// --features <f> (parked: <basis>)` — the static refusal, backend-
// independent — instead of the today's split (the ungated parked rows pass
// check and refuse only at the call; the gated parked rows read
// UNDEFINED_FUNCTION).
//
// RED-before (форма №772): this file ran against the pre-№799 main
// (fe64e39). The recorded RED: pins 1–3 fail behaviorally (the check
// passes for the ungated rows / reads UNDEFINED_FUNCTION for the gated
// row), pin 4 fails (no warning), pin 7 fails (the TW run reads
// UNDEFINED_FUNCTION for the gated parked name), pin 8 fails (the VM
// compile accepts the ungated parked row / reads UNDEFINED_FUNCTION for
// the gated one). Pins 5–6 are the guard rails. The pins assert the
// MESSAGE TEXT only — no new-API references — so the file compiles and
// runs on the pre-№799 tree too (the honest RED).
//
// The parity lock (the parked-tier rows ↔ the parked table) and the
// enum-kind pins live in naryad_799_parked_parity.rs — they compile only
// with the fix (the table IS the fix).
//
// The TW-run pin rides voice_enroll (a NON-intercepted parked name): the
// intercepted parked names (vision_*/reflex_generate/video_*) refuse at
// their dispatches BEFORE the registry lookup on the run lane — the
// №736 intercept-walk ordering — so their run-lane refusals keep the
// dispatch shape; the CHECK lane (backend-independent) is the №799
// surface, and the VM compile lane names [BUILTIN_PARKED] for both.

use metalogos::{check_program, compile_program, run_program};

const VISION_CALL: &str = r#"
pattern Gen(p: String) -> String {
  let _ = vision_generate(p, "a cat")
  return "done"
}
"#;

const VOICE_CALL: &str = r#"
pattern Enroll(d: String) -> String {
  let _ = voice_enroll(d, "audio.wav")
  return "done"
}
"#;

const REFLEX_CALL: &str = r#"
pattern Gen(m: String) -> String {
  let _ = reflex_generate(m, "p", 8, 1)
  return "done"
}
"#;

fn check_errors(src: &str) -> Vec<String> {
    let result = check_program(src).expect("parse ok");
    result.errors.iter().map(|e| e.message.clone()).collect()
}

fn check_warnings(src: &str) -> Vec<String> {
    let result = check_program(src).expect("parse ok");
    result.warnings.iter().map(|w| w.message.clone()).collect()
}

#[test]
fn n799_ungated_parked_row_refuses_check_naming_the_feature() {
    // vision_generate: parked, feature `vision` (№210, ADR-0124). Pre-№799:
    // the row is registered in the default build — check PASSES (the RED).
    let errors = check_errors(VISION_CALL);
    assert!(
        errors.iter().any(|m| m.contains("[BUILTIN_PARKED]")
            && m.contains("vision_generate")
            && m.contains("--features vision")
            && m.contains("parked:")),
        "№799: the parked vision_generate must refuse CHECK with [BUILTIN_PARKED] \
         naming --features vision and the basis; got: {errors:?}"
    );
}

#[test]
fn n799_gated_parked_row_reads_builtin_parked_not_undefined() {
    // voice_enroll: the row is `#[cfg(feature = "voice")]`-gated today —
    // pre-№799 the default-build check reads UNDEFINED_FUNCTION (the RED:
    // the name is known to the registry source but not this build; the
    // refusal must name the feature, not lie "undefined").
    let errors = check_errors(VOICE_CALL);
    assert!(
        errors.iter().any(|m| m.contains("[BUILTIN_PARKED]")
            && m.contains("voice_enroll")
            && m.contains("--features voice")),
        "№799: the gated parked voice_enroll must refuse CHECK with [BUILTIN_PARKED] \
         naming --features voice (not UNDEFINED_FUNCTION); got: {errors:?}"
    );
}

#[test]
fn n799_reflex_generate_names_candle_feature() {
    // reflex_generate: parked, feature `candle` (№748 — the live Ok arm is
    // candle-gated). Pre-№799: the row is registered in the default build
    // — check PASSES (the RED).
    let errors = check_errors(REFLEX_CALL);
    assert!(
        errors.iter().any(|m| m.contains("[BUILTIN_PARKED]")
            && m.contains("reflex_generate")
            && m.contains("--features candle")),
        "№799: the parked reflex_generate must refuse CHECK naming --features candle; \
         got: {errors:?}"
    );
}

#[test]
fn n799_experimental_call_is_a_warning_not_an_error() {
    // newline(): tier `experimental`, handler-less stub row. The check
    // stays GREEN (exit-0 semantics — is_ok()) and warns. Pre-№799: the
    // warnings channel carries nothing for it (the RED).
    let src = r#"
pattern P() -> String {
  let _ = newline()
  return "done"
}
"#;
    let errors = check_errors(src);
    let warnings = check_warnings(src);
    assert!(
        errors.is_empty(),
        "№799: the experimental call must NOT be a check error; got: {errors:?}"
    );
    assert!(
        warnings
            .iter()
            .any(|m| m.contains("[BUILTIN_EXPERIMENTAL]") && m.contains("newline")),
        "№799: the experimental call must carry the [BUILTIN_EXPERIMENTAL] warning; \
         got: {warnings:?}"
    );
}

#[test]
fn n799_stable_calls_stay_clean() {
    // The guard rail: the stable surface is untouched by the parked gate.
    let errors = check_errors(
        r#"
pattern P(s: String) -> String {
  let n = len(s)
  return n
}
"#,
    );
    assert!(
        errors.is_empty(),
        "№799: the stable call must stay clean; got: {errors:?}"
    );
}

#[test]
fn n799_truly_unknown_name_stays_undefined_function() {
    // The guard rail: the parked consult must NOT swallow the honest
    // UNDEFINED_FUNCTION for names the registry never knew.
    let errors = check_errors(
        r#"
pattern P() -> String {
  let _ = totally_unknown_fn_xyz(1)
  return "done"
}
"#,
    );
    assert!(
        errors
            .iter()
            .any(|m| m.contains("undefined") && !m.contains("BUILTIN_PARKED")),
        "№799: a truly unknown name must stay UNDEFINED_FUNCTION; got: {errors:?}"
    );
}

#[test]
fn n799_tw_runtime_refusal_names_the_parked_contract() {
    // The TW run path (the №523 contract: an unknown function is an ERROR
    // at the origin): after №799 a parked name that is NOT intercepted
    // (voice_enroll) refuses with the stable [BUILTIN_PARKED] code naming
    // the feature. Pre-№799: the read is UNDEFINED_FUNCTION (the RED).
    let err = run_program(VOICE_CALL).expect_err("the parked call must refuse at run too");
    assert!(
        err.contains("[BUILTIN_PARKED]") && err.contains("--features voice"),
        "№799: the TW run refusal must carry [BUILTIN_PARKED] naming --features voice; got: {err}"
    );
}

#[test]
fn n799_vm_compile_refusal_names_the_parked_contract() {
    // The .mbc/VM compile path (the №479 coded error at the origin):
    // after №799 the unknown-name refusal for a PARKED name carries
    // [BUILTIN_PARKED]. Pre-№799: the registered row compiles fine (the
    // RED on vision_generate; the gated voice row read UNDEFINED_FUNCTION).
    let err = compile_program(VISION_CALL).expect_err("the parked call must refuse at VM compile");
    assert!(
        err.contains("[BUILTIN_PARKED]") && err.contains("--features vision"),
        "№799: the VM compile refusal must carry [BUILTIN_PARKED] naming --features vision; got: {err}"
    );
    let err =
        compile_program(VOICE_CALL).expect_err("the gated parked call must refuse at VM compile");
    assert!(
        err.contains("[BUILTIN_PARKED]") && err.contains("--features voice"),
        "№799: the VM compile refusal for the gated parked row must carry \
         [BUILTIN_PARKED] naming --features voice; got: {err}"
    );
}
