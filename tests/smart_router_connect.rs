// ── Наряд №4 fix: SmartRouter is connected to call_llm() ────────────
//
// Contract: when a program declares `llm { providers: [...] }`, the
// builtin `call_llm()` MUST route through SmartRouter — not silently
// fall back to mock or legacy create_llm_backend().
//
// These tests use `run_program()` (same path as `mlog run file.mlog`)
// and verify the router is active by checking:
//   - The call reaches SmartRouter (not mocked when providers exist)
//   - The global router slot is populated after parsing llm {}
//   - Mock-mode logic is revised (no silent mock when providers declared)
//
// NOTE: We do NOT test actual circuit breaker failover here because that
// requires a real HTTP server. Circuit breaker unit tests already exist in
// src/llm.rs. The goal of THIS test is to prove the bridge is connected.

use serial_test::serial;

// ── T1: llm {} declaration populates the global router ──────────────

#[test]
#[serial]
fn test_llm_config_sets_global_router() {
    metalogos::llm::reset_global_smart_router();
    // Verify no router before
    assert_eq!(
        metalogos::llm::global_router_provider_count(),
        0,
        "T1: no router before llm declaration"
    );

    let source = r#"
llm {
  providers: [
    { alias: test_provider, provider: openai, key: env("TEST_KEY"), url: "http://127.0.0.1:99999/v1/chat" }
  ],
  failover: auto,
  circuit_breaker: 3,
  timeout: 5
}

pattern T(d: String) -> String {
  return call_llm("test prompt", d)
}

flow Main { input: String = "x" -> T -> output }
"#;

    // run_program will fail because TEST_KEY is not set and the server
    // doesn't exist — but the router should be created before the call.
    let _ = metalogos::run_program(source);

    // The key assertion: global router IS populated
    assert_eq!(
        metalogos::llm::global_router_provider_count(),
        1,
        "T1: global router should have 1 provider after llm declaration"
    );

    metalogos::llm::reset_global_smart_router();
}

// ── T2: call_llm with no llm {} falls back to mock in empty env ──────

#[test]
#[serial]
fn test_call_llm_no_config_returns_mock() {
    metalogos::llm::reset_global_smart_router();
    std::env::remove_var("METALOGOS_LLM_MOCK");
    std::env::remove_var("METALOGOS_API_KEY");
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("ANTHROPIC_API_KEY");

    let source = r#"
pattern T(d: String) -> String {
  return call_llm("prompt", d)
}

flow Main { input: String = "hello" -> T -> output }
"#;

    let result = metalogos::run_program(source);
    assert!(
        result.is_ok(),
        "T2: should succeed (mock mode in empty env)"
    );
    let output = result.unwrap().unwrap_or_default();
    assert!(
        output.contains("[MOCK:"),
        "T2: output should contain [MOCK: ...], got: {}",
        output
    );

    metalogos::llm::reset_global_smart_router();
}

// ── T3: call_llm with llm {} does NOT silently mock ────────────────

#[test]
#[serial]
fn test_call_llm_with_config_not_silently_mocked() {
    metalogos::llm::reset_global_smart_router();
    std::env::remove_var("METALOGOS_LLM_MOCK");
    std::env::remove_var("METALOGOS_API_KEY");
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("ANTHROPIC_API_KEY");
    // Set a dummy key so the provider config resolves
    std::env::set_var("TEST_KEY", "sk-dummy");

    let source = r#"
llm {
  providers: [
    { alias: primary, provider: openai, key: env("TEST_KEY"), url: "http://127.0.0.1:19999/v1/chat" }
  ],
  failover: auto,
  circuit_breaker: 3,
  timeout: 2
}

pattern T(d: String) -> String {
  return call_llm("test prompt", d)
}

flow Main { input: String = "x" -> T -> output }
"#;

    let result = metalogos::run_program(source);

    // With a provider configured, call_llm should go through SmartRouter.
    // The server at 127.0.0.1:19999 doesn't exist, so SmartRouter should fail
    // with a connection error — NOT return [MOCK: ...].
    //
    // This proves the router is actually connected.
    match result {
        Err(e) => {
            assert!(
                !e.contains("[MOCK"),
                "T3: should NOT return mock when providers are declared, got: {}",
                e
            );
            assert!(
                e.contains("All LLM providers failed") || e.contains("request failed") || e.contains("API error"),
                "T3: should fail with provider error, got: {}",
                e
            );
        }
        Ok(output) => {
            let out = output.unwrap_or_default();
            panic!(
                "T3: expected provider failure, but got success: {}",
                out
            );
        }
    }

    std::env::remove_var("TEST_KEY");
    metalogos::llm::reset_global_smart_router();
}

// ── T4: METALOGOS_LLM_MOCK=true overrides router (explicit mock) ──────

#[test]
#[serial]
fn test_explicit_mock_overrides_router() {
    metalogos::llm::reset_global_smart_router();
    std::env::set_var("METALOGOS_LLM_MOCK", "true");
    std::env::set_var("TEST_KEY", "sk-dummy");

    let source = r#"
llm {
  providers: [
    { alias: p, provider: openai, key: env("TEST_KEY"), url: "http://127.0.0.1:19999/v1/chat" }
  ],
  failover: auto,
  circuit_breaker: 3,
  timeout: 2
}

pattern T(d: String) -> String {
  return call_llm("prompt", d)
}

flow Main { input: String = "hi" -> T -> output }
"#;

    let result = metalogos::run_program(source);
    assert!(
        result.is_ok(),
        "T4: METALOGOS_LLM_MOCK=true should force mock even with providers: {:?}",
        result
    );
    let output = result.unwrap().unwrap_or_default();
    assert!(
        output.contains("[MOCK:"),
        "T4: explicit mock should return [MOCK: ...], got: {}",
        output
    );

    std::env::remove_var("METALOGOS_LLM_MOCK");
    std::env::remove_var("TEST_KEY");
    metalogos::llm::reset_global_smart_router();
}

// ── T5: call_llm 3rd arg (model override) is accepted ────────────────

#[test]
#[serial]
fn test_call_llm_model_override_arg_accepted() {
    metalogos::llm::reset_global_smart_router();
    std::env::remove_var("METALOGOS_LLM_MOCK");
    std::env::remove_var("METALOGOS_API_KEY");
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("ANTHROPIC_API_KEY");

    // No llm {} → mock → 3rd arg should be accepted without error
    let source = r#"
pattern T(d: String) -> String {
  return call_llm("prompt", d, "gpt-4o-mini")
}

flow Main { input: String = "x" -> T -> output }
"#;

    let result = metalogos::run_program(source);
    assert!(
        result.is_ok(),
        "T5: call_llm with 3 args (model override) should parse and run: {:?}",
        result
    );
    let output = result.unwrap().unwrap_or_default();
    assert!(
        output.contains("[MOCK:"),
        "T5: should return mock, got: {}",
        output
    );

    metalogos::llm::reset_global_smart_router();
}
