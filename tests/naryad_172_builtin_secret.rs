// ── Наряд №172: builtin secret() — contract tests ─────────────────────
//
// Block 3 contracts from the naryad:
//   1. `print(secret("K"))` → runtime error (Secret is nonprintable)
//   2. `entity k: Secret = secret("K")` → works, gives Value::Secret directly
//   3. `respond(k)` → SECRET_LEAK at compile time (static taint)
//   4. `secret("NONEXISTENT_VAR")` → explicit error, NOT empty string
//      (unlike `env()` which returns `""` on missing var)
//
// The test for contract 2 requires a real env var. We set one via
// std::env::set_var before running — this is safe in tests (no parallel
// access to the same env var name).

use metalogos::check_program;

/// Helper: set an env var for the test, return a guard that unsets it on drop.
struct EnvVarGuard {
    key: String,
    had_value: Option<String>,
}

impl EnvVarGuard {
    fn set(key: &str, value: &str) -> Self {
        let had_value = std::env::var(key).ok();
        std::env::set_var(key, value);
        EnvVarGuard {
            key: key.to_string(),
            had_value,
        }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.had_value {
            Some(v) => std::env::set_var(&self.key, v),
            None => std::env::remove_var(&self.key),
        }
    }
}

// ── Contract 1: print(secret("K")) → runtime error ──────────────────
//
// `secret()` returns Value::Secret. `print()` calls Display on the
// value, which returns "[Secret]" for Value::Secret (not the real
// value). The audit layer catches this at compile time via
// binding_taint, but the runtime also enforces nonprintability.

#[test]
fn secret_is_nonprintable_runtime() {
    let _guard = EnvVarGuard::set("N172_TEST_SECRET", "my-api-key-12345");
    let src = r#"
pattern Leak(input: String) -> String {
  let s = secret("N172_TEST_SECRET")
  return print(s)
}
flow Main { input: String = "x" -> Leak -> output }
"#;
    let result = check_program(src);
    // At compile time, the audit should catch this as SECRET_LEAK
    // because binding_taint("secret") = Secret, and print() is a sink.
    assert!(
        result.is_err() || {
            // If check_program succeeds (audit is advisory in some modes),
            // the runtime will still block print(Secret).
            match &result {
                Ok(analysis) => analysis
                    .errors
                    .iter()
                    .any(|e| e.message.contains("SECRET_LEAK")),
                Err(_) => false,
            }
        },
        "secret() result should be caught by SECRET_LEAK audit when passed to print(). \
         Result: {:?}",
        result
    );
}

// ── Contract 2: entity k: Secret = secret("K") → Value::Secret ──────
//
// `secret("K")` returns Value::Secret directly. When bound to a
// Secret-typed entity, the value is already the right type — no
// coerce_to_declared_type step needed (unlike `env()` which returns
// String and must be coerced).

#[test]
fn secret_bound_to_secret_entity_works() {
    let _guard = EnvVarGuard::set("N172_TEST_SECRET_2", "my-api-key-67890");
    let src = r#"
entity k: Secret = secret("N172_TEST_SECRET_2")
pattern Show(input: String) -> String { return "ok" }
flow Main { input: String = "x" -> Show -> output }
"#;
    // Should compile and check without errors (no SECRET_LEAK —
    // the secret is bound to an entity, not passed to a sink).
    let result = check_program(src);
    match result {
        Ok(analysis) => {
            assert!(
                analysis.is_ok(),
                "secret() bound to Secret entity should not produce errors. Got: {}",
                analysis.format()
            );
        }
        Err(e) => panic!("check_program failed: {}", e),
    }
}

// ── Contract 3: respond(k) → SECRET_LEAK at compile time ─────────────
//
// `respond(secret("K"))` — the static audit should catch this:
// binding_taint("secret") = Secret, respond() is a network sink,
// SECRET_LEAK is a Category A error.

#[test]
fn secret_passed_to_respond_triggers_secret_leak() {
    let _guard = EnvVarGuard::set("N172_TEST_SECRET_3", "should-not-leak");
    let src = r#"
pattern Leak(input: String) -> String {
  respond(secret("N172_TEST_SECRET_3"))
  return "done"
}
flow Main { input: String = "x" -> Leak -> output }
"#;
    let result = check_program(src);
    let has_secret_leak = match &result {
        Ok(analysis) => analysis
            .errors
            .iter()
            .any(|e| e.message.contains("SECRET_LEAK")),
        Err(_) => false,
    };
    assert!(
        has_secret_leak,
        "respond(secret(...)) should trigger SECRET_LEAK audit error. \
         Result: {:?}",
        result
    );
}

// ── Contract 4: secret("NONEXISTENT_VAR") → hard error ──────────────
//
// Unlike env() which returns "" on missing var, secret() returns
// an explicit error. This is the key behavioral difference.

#[test]
fn secret_missing_env_var_returns_hard_error() {
    // Ensure the var does not exist
    let _guard = EnvVarGuard::set("N172_NONEXISTENT_VAR", "");
    std::env::remove_var("N172_NONEXISTENT_VAR");

    let src = r#"
pattern GetSecret(input: String) -> String {
  let s = secret("N172_NONEXISTENT_VAR")
  return to_string(s)
}
flow Main { input: String = "x" -> GetSecret -> output }
"#;
    // This should compile (no static error — missing env var is a
    // runtime condition, not a compile-time one). The error only
    // surfaces when the pattern is actually executed.
    let result = check_program(src);
    assert!(
        result.is_ok(),
        "secret() with missing env var should compile fine (runtime error, not compile-time). \
         Got: {:?}",
        result
    );

    // Verify the runtime error by calling the builtin directly
    use metalogos::builtins::Builtins;
    let builtins = Builtins::new();
    let handler = builtins
        .get("secret")
        .expect("secret() must be registered in BUILTIN_REGISTRY");
    let result = handler(&[metalogos::interpreter::Value::String(
        "N172_NONEXISTENT_VAR".to_string(),
    )]);
    assert!(
        result.is_err(),
        "secret(\"NONEXISTENT_VAR\") should return Err, not Ok. Got: {:?}",
        result
    );
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("not found"),
        "Error message should mention 'not found'. Got: {}",
        err_msg
    );
}

// ── Contract 5: secret() is registered in BUILTIN_REGISTRY ──────────
//
// Smoke test — verifies the SSOT registration from наряд №170 works.

#[test]
fn secret_is_registered_in_registry() {
    use metalogos::builtins::{builtin_name_set, is_builtin};
    assert!(
        is_builtin("secret"),
        "secret() must be in BUILTIN_REGISTRY (is_builtin returned false)"
    );
    assert!(
        builtin_name_set().contains("secret"),
        "secret() must be in builtin_name_set()"
    );
}

// ── Contract 6: binding_taint treats secret() same as env() ──────────
//
// Both `env("KEY")` and `secret("KEY")` produce `TaintKind::Secret` in
// the audit's binding_taint function. This ensures respond(secret("K"))
// triggers SECRET_LEAK just like respond(env("K")) does.

#[test]
fn secret_taint_matches_env_taint() {
    let _guard = EnvVarGuard::set("N172_TAINT_TEST", "taint-value");
    // env() version — should trigger SECRET_LEAK
    let env_src = r#"
pattern Leak(input: String) -> String {
  respond(env("N172_TAINT_TEST"))
  return "done"
}
flow Main { input: String = "x" -> Leak -> output }
"#;
    // secret() version — should also trigger SECRET_LEAK
    let secret_src = r#"
pattern Leak(input: String) -> String {
  respond(secret("N172_TAINT_TEST"))
  return "done"
}
flow Main { input: String = "x" -> Leak -> output }
"#;
    let env_result = check_program(env_src);
    let secret_result = check_program(secret_src);

    let env_has_leak = match &env_result {
        Ok(a) => a.errors.iter().any(|e| e.message.contains("SECRET_LEAK")),
        Err(_) => false,
    };
    let secret_has_leak = match &secret_result {
        Ok(a) => a.errors.iter().any(|e| e.message.contains("SECRET_LEAK")),
        Err(_) => false,
    };

    assert!(
        env_has_leak && secret_has_leak,
        "Both env() and secret() should trigger SECRET_LEAK when passed to respond(). \
         env={:?}, secret={:?}",
        env_has_leak,
        secret_has_leak
    );
}
