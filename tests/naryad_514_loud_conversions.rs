// ── Naryad #514 (P1, hardening): loud to_float/to_int + the *_or twins ──
//
// Contract (issue #798, the consolidated audit 28.09 C-10 — the ONE
// soft-failure rule "silence is visible in the name", ADR-0180):
// 1. THE HOLE: `to_float("abc")` returned `0.0` silently — the audit's
//    vector (`to_float(json_body().amount)` with `"12,50"` → 0.0 → a
//    zero-value charge with no error). Both conversion builtins now
//    refuse LOUDLY with the stable TYPE_MISMATCH code, naming the input
//    and pointing at the `_or` twin.
// 2. THE EXPLICIT-SILENCE TWINS: `to_float_or(value, default)` /
//    `to_int_or(value, default)` — the `_or` suffix carries the
//    silent-default semantics IN THE NAME (the №481 env/env_or rule);
//    the fallback firing is announced on the audit stderr
//    ([TO_FLOAT_OR]/[TO_INT_OR] — the value never logged).
// 3. A CONVERSION IS NOT A SOFT FAILURE: Bool → 1.0/0.0 and float
//    truncation are total mappings — unchanged in both twins.
// 4. EXPLICIT SILENCE COVERS DATA, NOT TYPE ERRORS: a non-scalar input
//    (a list) and a non-Float default stay LOUD in the `_or` twin too.
// 5. The stale crypto.rs comment claiming `env()` is soft (pre-№481) is
//    fixed — pinned by the source-honesty grep below.

// Naryad #475's fs_gate ratchet targets PRODUCTION I/O paths; this test
// exercises the REAL filesystem for its fixtures (the probe .mlog written
// to target/ for the real-binary stderr pins, and the crypto.rs source
// read for the honesty pin) by design — the scoped allow mirrors the
// naryad_481/naryad_465 posture.
#![allow(clippy::disallowed_methods)]

fn program(body: &str, ret: &str) -> String {
    format!(
        "pattern Read(_x: String) -> {} {{\n  {}\n}}\nflow Main {{\n  input: String = \"a\" -> Read -> output\n}}\n",
        ret, body
    )
}

// ── 1. The loud refusals (the audit vector first) ──

#[test]
fn n514_to_float_comma_string_is_loud_the_audit_vector() {
    // "12,50" is NOT a number (the comma) — the audit's zero-charge vector.
    let src = program(r#"  return to_float("12,50")"#, "Float");
    let err = metalogos::run_program(&src).expect_err("the C-10 vector must refuse loudly");
    assert!(
        err.contains("[TYPE_MISMATCH]"),
        "the refusal must carry the stable code: {}",
        err
    );
    assert!(
        err.contains("to_float_or"),
        "the refusal must name the explicit fallback: {}",
        err
    );
}

#[test]
fn n514_to_float_non_numeric_is_loud() {
    let src = program(r#"  return to_float("abc")"#, "Float");
    let err = metalogos::run_program(&src).expect_err("a non-numeric string must refuse loudly");
    assert!(err.contains("[TYPE_MISMATCH]"), "{}", err);
    assert!(err.contains("abc"), "the input must be named: {}", err);
}

#[test]
fn n514_to_int_non_numeric_is_loud() {
    let src = program(r#"  return to_int("42abc")"#, "Float");
    let err = metalogos::run_program(&src).expect_err("a non-numeric string must refuse loudly");
    assert!(
        err.contains("[TYPE_MISMATCH]"),
        "the refusal must carry the stable code: {}",
        err
    );
    assert!(
        err.contains("to_int_or"),
        "the refusal must name the explicit fallback: {}",
        err
    );
}

#[test]
fn n514_to_float_list_input_is_loud() {
    let src = program(r#"  return to_float([1.0, 2.0])"#, "Float");
    let err = metalogos::run_program(&src)
        .expect_err("a non-scalar input must refuse loudly (both twins)");
    assert!(err.contains("[TYPE_MISMATCH]"), "{}", err);
}

// ── 2. The explicit-silence twins ──

#[test]
fn n514_to_float_or_falls_back_explicitly() {
    let src = program(r#"  return to_float_or("12,50", 0.0)"#, "Float");
    let out = metalogos::run_program(&src).expect("to_float_or falls back, not refuses");
    assert_eq!(out.as_deref(), Some("0"), "Float Display omits the .0");
}

#[test]
fn n514_to_float_or_parses_when_parseable() {
    let src = program(r#"  return to_float_or("12.50", 0.0)"#, "Float");
    let out = metalogos::run_program(&src).expect("a parseable string must parse");
    assert_eq!(out.as_deref(), Some("12.5"));
}

#[test]
fn n514_to_int_or_falls_back_explicitly() {
    let src = program(r#"  return to_int_or("42abc", 7.0)"#, "Float");
    let out = metalogos::run_program(&src).expect("to_int_or falls back, not refuses");
    assert_eq!(out.as_deref(), Some("7"));
}

#[test]
fn n514_to_int_or_truncates_when_parseable() {
    let src = program(r#"  return to_int_or("42.9", 7.0)"#, "Float");
    let out = metalogos::run_program(&src).expect("a parseable string must parse and truncate");
    assert_eq!(out.as_deref(), Some("42"));
}

#[test]
fn n514_or_twin_with_a_non_float_default_is_loud() {
    let src = program(r#"  return to_float_or("abc", "zero")"#, "Float");
    let err =
        metalogos::run_program(&src).expect_err("a non-Float default is a type error, not data");
    assert!(err.contains("[TYPE_MISMATCH]"), "{}", err);
}

// ── 3. The unchanged conversions (a conversion is not a soft failure) ──

#[test]
fn n514_numeric_strings_still_convert() {
    let src = program(r#"  return to_float("3.14")"#, "Float");
    let out = metalogos::run_program(&src).expect("a numeric string converts");
    assert_eq!(out.as_deref(), Some("3.14"));

    let src = program(r#"  return to_int("42")"#, "Float");
    let out = metalogos::run_program(&src).expect("an integer string converts");
    assert_eq!(out.as_deref(), Some("42"));
}

#[test]
fn n514_float_and_bool_conversions_unchanged() {
    let src = program(r#"  return to_int(3.9)"#, "Float");
    let out = metalogos::run_program(&src).expect("float truncation unchanged");
    assert_eq!(out.as_deref(), Some("3"));

    let src = program(r#"  return to_float(true)"#, "Float");
    let out = metalogos::run_program(&src).expect("Bool → 1.0 is a conversion, not silence");
    assert_eq!(out.as_deref(), Some("1"));
}

#[test]
fn n514_roundtrip_through_to_string() {
    let src = program(r#"  return to_float(to_string(2.5))"#, "Float");
    let out = metalogos::run_program(&src).expect("to_string → to_float must roundtrip");
    assert_eq!(out.as_deref(), Some("2.5"));
}

// ── 4. The source-honesty pin: the crypto.rs comment is no longer stale ──

#[test]
fn n514_crypto_env_comment_matches_the_n481_contract() {
    let src = std::fs::read_to_string("src/builtins/crypto.rs")
        .expect("src/builtins/crypto.rs must exist");
    assert!(
        !src.contains("soft-failure: returns `Value::String(\"\")`"),
        "the stale pre-№481 claim that env() is soft must be gone (№514)"
    );
    assert!(
        src.contains("LOUD on a missing variable (№481)"),
        "the corrected comment must state the loud №481 contract"
    );
}

// ── 5. The fallback firing is ANNOUNCED on the audit stderr (the №481
//      pattern: run the real binary, capture stderr, the value never
//      logged — the №326 posture) ──

fn run_mlog(label: &str, body: &str) -> (String, String, bool) {
    let src = program(body, "Float");
    let file = std::path::Path::new("target").join(format!(
        "n514_or_pin_{}_{}.mlog",
        std::process::id(),
        label
    ));
    std::fs::write(&file, &src).expect("write the probe program");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_mlog"))
        .arg("run")
        .arg(&file)
        .output()
        .expect("the mlog binary runs");
    let _ = std::fs::remove_file(&file);
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.success(),
    )
}

#[test]
fn n514_to_float_or_fallback_is_announced_without_the_value() {
    let (stdout, stderr, ok) = run_mlog("float_fallback", r#"  return to_float_or("12,50", 42.5)"#);
    assert!(
        ok,
        "to_float_or must NOT refuse: {} | {}",
        stdout.trim(),
        stderr.trim()
    );
    assert!(
        stderr.contains("[TO_FLOAT_OR]"),
        "the fallback firing must be announced: {}",
        stderr.trim()
    );
    assert!(
        !stderr.contains("12,50") && !stderr.contains("42.5"),
        "the VALUE must never reach the log (the №326 posture): {}",
        stderr.trim()
    );
    assert!(
        stdout.contains("42.5"),
        "the default still flows to the PROGRAM result: {}",
        stdout.trim()
    );
}

#[test]
fn n514_to_int_or_fallback_is_announced_without_the_value() {
    let (stdout, stderr, ok) = run_mlog("int_fallback", r#"  return to_int_or("42abc", 7.25)"#);
    assert!(
        ok,
        "to_int_or must NOT refuse: {} | {}",
        stdout.trim(),
        stderr.trim()
    );
    assert!(
        stderr.contains("[TO_INT_OR]"),
        "the fallback firing must be announced: {}",
        stderr.trim()
    );
    assert!(
        !stderr.contains("42abc") && !stderr.contains("7.25"),
        "the VALUE must never reach the log (the №326 posture): {}",
        stderr.trim()
    );
    assert!(
        stdout.contains("7.25"),
        "the default still flows to the PROGRAM result: {}",
        stdout.trim()
    );
}

#[test]
fn n514_successful_parse_is_silent_no_announcement() {
    let (_, stderr, ok) = run_mlog("silent_parse", r#"  return to_float_or("12.50", 0.0)"#);
    assert!(ok, "a parseable string must succeed: {}", stderr.trim());
    assert!(
        !stderr.contains("[TO_FLOAT_OR]"),
        "a successful parse is NOT a fallback firing — no announcement: {}",
        stderr.trim()
    );
}
