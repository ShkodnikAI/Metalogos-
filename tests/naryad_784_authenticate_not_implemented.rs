// ── tests/naryad_784_authenticate_not_implemented.rs ─────────────────
// №784 (gh#1495, Wave 62 P1 — audit-23.10 A5): `authenticate` was a
// mock — the handler answered `Ok(Unit)` while the REFERENCE row
// promised "In a server context, checks the credentials" (no server
// user-store exists in either backend — the promise had no code behind
// it). THE CONTRACT (the loud-failure line, ADR-0198): every call
// refuses with the coded `[NOT_IMPLEMENTED]` refusal naming the real
// credential path — `verify_password(password, hash)` against the
// stored hash.
//
// The honest argument-validation refusals keep their pre-№784 shape
// (the coded `[TYPE_MISMATCH]` email check, the plain
// "expected Secret or String as password" password check) — the naryad
// boundary: hash_password/verify_password untouched.
//
// Branchability: `[NOT_IMPLEMENTED]` joins ORIGIN_STAMPED_CODES (the
// №413 mechanics) — the try pin reads `r.error.code ==
// "NOT_IMPLEMENTED"` on BOTH backends (the one shared classifier,
// parity by construction).
//
// RED-before (форма №772): on the pre-№784 revision the loud-refusal
// pins FAIL — the mock returned Ok(Unit) (a printable Unit instead of
// the refusal; the try pin read no NOT_IMPLEMENTED code).

fn tw_out(source: &str) -> Result<String, String> {
    metalogos::run_program(source).map(|o| o.unwrap_or_default().trim_end().to_string())
}

fn vm_out(source: &str) -> Result<String, String> {
    let program = metalogos::compile_program(source)?;
    metalogos::run_bytecode(program).map(|o| o.unwrap_or_default().trim_end().to_string())
}

fn assert_parity_error(source: &str, fragments: &[&str], what: &str) {
    for (backend, res) in [("TW", tw_out(source)), ("VM", vm_out(source))] {
        let err = res.expect_err(&format!(
            "{what}: {backend} must refuse loudly (the №784 contract)"
        ));
        for f in fragments {
            assert!(
                err.contains(f),
                "{what}: {backend} refusal must carry '{f}', got: {err}"
            );
        }
    }
}

fn assert_parity_output(source: &str, expected: &str, what: &str) {
    for (backend, res) in [("TW", tw_out(source)), ("VM", vm_out(source))] {
        let out = res.unwrap_or_else(|e| panic!("{what}: {backend} must succeed, got: {e}"));
        assert_eq!(out, expected, "{what}: {backend} output mismatch");
    }
}

/// The core pin: `authenticate(email, password)` with a String password
/// — the loud coded refusal naming the real credential path (both
/// backends, форма №762).
#[test]
fn n784_authenticate_refuses_loudly_string_password() {
    assert_parity_error(
        r#"
pattern Probe(x: String) -> Unit {
  return authenticate(x, "pw")
}
flow Main { input: String = "e@b.c" -> Probe -> output }
"#,
        &["[NOT_IMPLEMENTED]", "verify_password(password, hash)"],
        "authenticate(String, String)",
    );
}

/// The Secret-password arm reaches the same refusal (the password
/// validation passes a `Value::Secret` through — the refusal is
/// arm-independent). The env-backed `secret()` builtin builds the
/// Secret value; the var name is test-unique (the №253 process-global
/// env discipline — serialized mutation, no reader contention).
#[test]
fn n784_authenticate_refuses_loudly_secret_password() {
    // The №413 ENV_LOCK form: the env mutation is serialized per binary.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var("METALOGOS_N784_TEST_SECRET", "pw");
    assert_parity_error(
        r#"
pattern Probe(x: String) -> Unit {
  return authenticate(x, secret("METALOGOS_N784_TEST_SECRET"))
}
flow Main { input: String = "e@b.c" -> Probe -> output }
"#,
        &["[NOT_IMPLEMENTED]", "verify_password(password, hash)"],
        "authenticate(String, Secret)",
    );
}

/// Regression pin: the honest argument validation keeps its pre-№784
/// shape — a non-Secret/non-String password refuses with the plain
/// password-check message (the naryad boundary: the validation refusals
/// are untouched, only the mock answer is replaced).
#[test]
fn n784_password_type_refusal_unchanged() {
    assert_parity_error(
        r#"
pattern Probe(x: String) -> Unit {
  return authenticate(x, 1.0)
}
flow Main { input: String = "e@b.c" -> Probe -> output }
"#,
        &["authenticate() expected Secret or String as password, got Float"],
        "password type refusal",
    );
}

/// Regression pin: the coded email validation — `[TYPE_MISMATCH]` at
/// position 0 (the expect_string_arg form, №787-adjacent convention).
#[test]
fn n784_email_type_refusal_unchanged() {
    assert_parity_error(
        r#"
pattern Probe(x: String) -> Unit {
  return authenticate(1.0, "pw")
}
flow Main { input: String = "e@b.c" -> Probe -> output }
"#,
        &[
            "[TYPE_MISMATCH]",
            "authenticate() expected String argument, got Float",
        ],
        "email type refusal",
    );
}

/// The branchability pin: `try authenticate(...)` reads the stamped
/// code — `NOT_IMPLEMENTED` classifies identically on both backends
/// (the №413 mechanics: ORIGIN_STAMPED_CODES is the whitelist).
#[test]
fn n784_try_reads_not_implemented_code() {
    assert_parity_output(
        r#"
pattern Probe(x: String) -> String {
  let r = try authenticate(x, "pw")
  return r.error.code
}
flow Main { input: String = "e@b.c" -> Probe -> output }
"#,
        "NOT_IMPLEMENTED",
        "try authenticate code",
    );
}
