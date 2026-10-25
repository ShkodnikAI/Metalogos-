//! Naryad №802 (issue #1549, wave 65, P2 — the C6 R-b 2 static check,
//! the ADR-0198 queue): the try-result and the `T|Unit` union are
//! verified BEFORE use — `[UNCHECKED_RESULT]` / `[UNCHECKED_OPTION]`.
//!
//! The ADR-0198 model (№792): `Result<T>` is the type of the `try`
//! result (`Struct{ok, value, error}`, ADR-0142), `Option<T>` is the
//! spelling of `T|Unit` — the ALIASES of the existing shapes. Nothing
//! stopped a program from reading `r.value` without ever checking
//! `r.ok`, or from using a `T|Unit` value in a `T` position — the
//! failure surfaced only at RUNTIME. This naryad shifts BOTH refusals
//! left to `mlog check` (the shared semantic pass — backend-independent
//! by construction), with the flow-sensitive guard recognition (the
//! label-walk shape) and the explicit suppression comment
//! `// allow(unchecked_result): <reason>` (the reason is part of the
//! spelling — a bare allow is NOT a suppression).
//!
//! RED-before (the №772 form): the negative pins 1 (UNCHECKED_RESULT)
//! and 6 (UNCHECKED_OPTION) produced ZERO errors on main @ 0623eba —
//! the class was silent; the control pins existed to keep the
//! implementation honest (they must stay green BEFORE and AFTER).

use metalogos::semantic::AnalysisResult;

fn check(src: &str) -> AnalysisResult {
    // The public source-level entry (lib.rs) — parse + semantic::check_program
    // + the suppression filter (the same surface `mlog check` uses).
    metalogos::check_program(src).expect("test programs must parse")
}

fn messages(result: &AnalysisResult) -> Vec<String> {
    result.errors.iter().map(|e| e.message.clone()).collect()
}

fn has_code(result: &AnalysisResult, code: &str) -> bool {
    messages(result).iter().any(|m| m.contains(code))
}

// ── The result side: [UNCHECKED_RESULT] ─────────────────────────────

/// Pin 1 (RED-before): `.value` on a try-bound variable with NO guard —
/// the check-time refusal exists.
#[test]
fn n802_unguarded_value_is_unchecked_result() {
    let src = r#"
pattern P() -> String {
  let r = try trim("hello")
  return r.value
}
"#;
    let result = check(src);
    assert!(
        has_code(&result, "UNCHECKED_RESULT"),
        "the unguarded .value must refuse with [UNCHECKED_RESULT], got: {:?}",
        messages(&result)
    );
}

/// Pin 2 (control): the guard `if r.ok { ... }` — the access inside the
/// then-branch is checked, no error.
#[test]
fn n802_if_ok_then_branch_is_checked() {
    let src = r#"
pattern P() -> String {
  let r = try trim("hello")
  if r.ok {
    return r.value
  }
  return "none"
}
"#;
    let result = check(src);
    assert!(
        !has_code(&result, "UNCHECKED_RESULT"),
        "the guarded .value must NOT refuse, got: {:?}",
        messages(&result)
    );
}

/// Pin 3 (control): the early-exit guard `if r.ok == false { return ... }`
/// — the code AFTER the if is checked (the dominance form).
#[test]
fn n802_early_exit_guard_checks_the_rest() {
    let src = r#"
pattern P() -> String {
  let r = try trim("hello")
  if r.ok == false {
    return "err"
  }
  return r.value
}
"#;
    let result = check(src);
    assert!(
        !has_code(&result, "UNCHECKED_RESULT"),
        "the post-early-exit .value must NOT refuse, got: {:?}",
        messages(&result)
    );
}

/// Pin 4 (control): the and-join guard — `r.ok and E` short-circuits,
/// the right side is provably guarded (the p91 shape).
#[test]
fn n802_and_join_right_side_is_guarded() {
    let src = r#"
pattern P() -> Float {
  let r = try trim("hello")
  return to_float(r.ok and type_of(r.value) == "String")
}
"#;
    let result = check(src);
    assert!(
        !has_code(&result, "UNCHECKED_RESULT"),
        "the and-guarded .value must NOT refuse, got: {:?}",
        messages(&result)
    );
}

/// Pin 5: the explicit suppression comment — the sanctioned escape with
/// a NON-EMPTY reason (the w385 redact-obfuscated guard shape).
#[test]
fn n802_suppression_with_reason_is_accepted() {
    let src = r#"
pattern P() -> String {
  let r = try trim("hello")
  if redact(to_string(r.ok), "hash_only") == redact("true", "hash_only") {
    return r.value // allow(unchecked_result): the guard is the redact-obfuscated r.ok == true check (ADR-0169 (д))
  }
  return "none"
}
"#;
    let result = check(src);
    assert!(
        !has_code(&result, "UNCHECKED_RESULT"),
        "the explicit suppression must be honored, got: {:?}",
        messages(&result)
    );
}

/// Pin 6: a BARE `allow(unchecked_result)` (no reason after the colon)
/// is NOT a suppression — the error stays (the honest-spelling rule).
#[test]
fn n802_bare_allow_is_not_a_suppression() {
    let src = r#"
pattern P() -> String {
  let r = try trim("hello")
  return r.value // allow(unchecked_result):
}
"#;
    let result = check(src);
    assert!(
        has_code(&result, "UNCHECKED_RESULT"),
        "a bare allow without the reason must NOT suppress, got: {:?}",
        messages(&result)
    );
}

/// Pin 7 (negative control, the p7-compat shape): `.value` on a row
/// struct (NOT a try result) must never fire — the Result-ness comes
/// ONLY from the `try` binding.
#[test]
fn n802_row_value_is_not_a_result() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern P() -> String {
  db_execute("CREATE TABLE kv (key TEXT, value TEXT)", [])
  db_execute("INSERT INTO kv VALUES ('host', 'localhost')", [])
  let rows = query("SELECT key, value FROM kv", [])
  let second = get(rows, 1)
  return second.value
}
"#;
    let result = check(src);
    assert!(
        !has_code(&result, "UNCHECKED_RESULT"),
        "a row-struct .value must NOT refuse, got: {:?}",
        messages(&result)
    );
}

// ── The option side: [UNCHECKED_OPTION] ─────────────────────────────

/// Pin 8 (RED-before): a `T|Unit` union value (query_scalar →
/// `Float|String|Unit`) concatenated RAW — the Unit arm is a runtime
/// refusal; the check-time refusal exists.
#[test]
fn n802_raw_union_in_concat_is_unchecked_option() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern P() -> String {
  let first = query_scalar("SELECT name FROM t LIMIT 1", [])
  return "first=" + first
}
"#;
    let result = check(src);
    assert!(
        has_code(&result, "UNCHECKED_OPTION"),
        "the raw union concat must refuse with [UNCHECKED_OPTION], got: {:?}",
        messages(&result)
    );
}

/// Pin 9 (control): the sanctioned string projection — `to_string` is
/// the total projection, NOT a T position.
#[test]
fn n802_to_string_projection_is_not_a_t_position() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern P() -> String {
  let n = query_scalar("SELECT COUNT(*) FROM t", [])
  return "count=" + to_string(n)
}
"#;
    let result = check(src);
    assert!(
        !has_code(&result, "UNCHECKED_OPTION"),
        "to_string of a union must NOT refuse, got: {:?}",
        messages(&result)
    );
}

/// Pin 10 (control): the Unit-handling guard — `if type_of(v) == "Unit"
/// { return ... }` makes the code after the if a handled context.
#[test]
fn n802_type_of_unit_guard_handles_the_union() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern P() -> String {
  let first = query_scalar("SELECT name FROM t LIMIT 1", [])
  if type_of(first) == "Unit" {
    return "none"
  }
  return "first=" + first
}
"#;
    let result = check(src);
    assert!(
        !has_code(&result, "UNCHECKED_OPTION"),
        "the type_of-guarded union must NOT refuse, got: {:?}",
        messages(&result)
    );
}

/// Pin 11 (negative control): a precise-typed builtin result (len →
/// Float) is NOT a union — no UNCHECKED_OPTION on ordinary code.
#[test]
fn n802_precise_typed_vars_never_fire_option() {
    let src = r#"
pattern P() -> String {
  let n = len("hello")
  return "len=" + to_string(n)
}
"#;
    let result = check(src);
    assert!(
        !has_code(&result, "UNCHECKED_OPTION") && !has_code(&result, "UNCHECKED_RESULT"),
        "ordinary precise-typed code must NOT refuse, got: {:?}",
        messages(&result)
    );
}

/// Pin 12: the run-path blocking shape — the №523 gate blocks the run
/// on EVERY non-exempt check_program error, so the run refusal carries
/// the stable code at position 0 (the fail-closed default; the kinds
/// are NOT added to the exemption list).
#[test]
fn n802_run_gate_blocks_on_the_class() {
    let src = "pattern P() -> String {\n  let r = try trim(\"hello\")\n  return r.value\n}\n";
    let run = metalogos::run_program(src);
    match run {
        Err(msg) => assert!(
            msg.contains("UNCHECKED_RESULT"),
            "the run gate must refuse with the stable code, got: {}",
            msg
        ),
        Ok(_) => panic!("the run gate must block the unchecked use (RED-before: it ran silently)"),
    }
}
