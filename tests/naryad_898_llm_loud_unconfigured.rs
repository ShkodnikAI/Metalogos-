// Naryad #475 (issue #723): the fs_gate ratchet — this file touches no
// production files; the allow is scoped here for the env manipulation
// helpers and the test-server fixtures.
#![allow(clippy::disallowed_methods)]

// ── №898 (issue #1792, P1, [core][llm][process]): the loud unconfigured
//    LLM posture — the №879 silent local-first default is rolled back ──
//
// The owner decision 04.11: without an EXPLICIT LLM declaration the
// first `call_llm` REFUSES, naming the configuration surfaces — the
// value a program builds on is never a mock marker mistaken for a
// model answer (the №454 lesson). The deterministic offline mock is a
// FIRST-CLASS explicit choice: `METALOGOS_MOCK_LLM=1` or the explicit
// in-program local rung (`llm { providers: [{ alias: "mock",
// provider: "local" }] }`). A DECLARED provider without its key keeps
// the №454 loud failure (naming METALOGOS_API_KEY).
//
// The mutation probe (the «Сделано, когда» criterion): restoring
// `Box::new(MockLlm)` into the default branch of create_llm_backend
// turns (a) RED — the assert below demands the loud refusal on the
// unconfigured lane; the №865 breaking-change gate extension catches
// the same mutation at the diff level (the posture-default marker).

use metalogos::llm::{
    create_llm_backend, external_llm_declared, mock_llm_requested, unconfigured_llm_error,
};
use serial_test::serial;

/// Remove every LLM configuration surface the predicate reads (the
/// unconfigured fixture — the №898 subject state).
fn strip_all_llm_env() {
    std::env::remove_var("METALOGOS_MOCK_LLM");
    std::env::remove_var("METALOGOS_LLM_PROVIDER");
    std::env::remove_var("METALOGOS_API_KEY");
    std::env::remove_var("METALOGOS_OPENAI_BASE_URL");
}

// ── (a): nothing declared → the LOUD refusal naming the surfaces ─────
#[test]
#[serial]
fn n898_unconfigured_llm_refuses_loudly() {
    strip_all_llm_env();
    assert!(!mock_llm_requested());
    assert!(!external_llm_declared());

    let backend = create_llm_backend();
    let err = backend.call("summarize this", "input").expect_err(
        "the unconfigured lane must REFUSE at the first call (№898) — \
                     a mock marker here is the rolled-back №879 default, the \
                     mutation probe: restore MockLlm into the default branch and \
                     this test goes RED",
    );
    // The refusal names the configuration surfaces a user can act on.
    assert!(
        err.contains("METALOGOS_LLM_PROVIDER"),
        "the refusal must name METALOGOS_LLM_PROVIDER, got: {err}"
    );
    assert!(
        err.contains("METALOGOS_API_KEY"),
        "the refusal must name METALOGOS_API_KEY, got: {err}"
    );
    assert!(
        err.contains("METALOGOS_MOCK_LLM"),
        "the refusal must name the explicit mock opt-in, got: {err}"
    );
    assert!(
        err.contains("llm {"),
        "the refusal must name the in-program llm block surface, got: {err}"
    );
    // The helper agrees with the backend's own text (one SSOT message).
    assert_eq!(err, unconfigured_llm_error());

    // The same posture rides the model-override and deadline lanes —
    // every entry point of the backend refuses (no silent side door).
    let err2 = backend
        .call_with_model("p", "i", Some("claude-sonnet"))
        .expect_err("call_with_model must refuse on the unconfigured lane too");
    assert_eq!(err2, unconfigured_llm_error());
    let err3 = backend
        .call_with_deadline("p", "i", None, std::time::Duration::from_secs(5))
        .expect_err("call_with_deadline must refuse on the unconfigured lane too");
    assert_eq!(err3, unconfigured_llm_error());

    strip_all_llm_env();
}

// ── (b): the EXPLICIT mock flag → the marker + the visible posture ───
#[test]
#[serial]
fn n898_explicit_mock_flag_serves_the_marker() {
    strip_all_llm_env();
    std::env::set_var("METALOGOS_MOCK_LLM", "1");
    assert!(mock_llm_requested());

    let backend = create_llm_backend();
    let out = backend
        .call("summarize this", "input")
        .expect("the EXPLICIT mock flag is the first-class opt-in — the backend must serve");
    // The frozen non-echo marker (№454/№487): deterministic, names the
    // local provider — never a model answer masquerade.
    assert!(
        out.starts_with("[mock-llm:") && out.ends_with("]"),
        "the explicit mock answer must carry the frozen marker, got: {out}"
    );

    strip_all_llm_env();
}

// ── (c): a DECLARED provider without the key → the №454 loud failure ──
#[test]
#[serial]
fn n898_declared_provider_without_key_still_loud() {
    strip_all_llm_env();
    std::env::set_var("METALOGOS_LLM_PROVIDER", "anthropic");
    assert!(external_llm_declared());

    let backend = create_llm_backend();
    let err = backend
        .call("p", "i")
        .expect_err("the №454 posture is preserved ON THE DECLARED LANE: anthropic without METALOGOS_API_KEY refuses");
    assert!(
        err.contains("METALOGOS_API_KEY"),
        "the declared-lane refusal must name METALOGOS_API_KEY, got: {err}"
    );

    strip_all_llm_env();
}

// ── (d): the in-program explicit local rung → the covered fallback ────
// The `llm { providers: [{alias: mock, provider: local}] }`
// declaration is the FIRST-CLASS explicit mock (ADR-0208 §4): the
// SmartRouter's local rung is the covered fallback — no env flag, the
// declaration itself is the opt-in (the №880 coverage predicate).
#[test]
#[serial]
fn n898_explicit_local_rung_is_covered_fallback() {
    strip_all_llm_env();
    let src = r#"
llm { providers: [{alias: mock, provider: local}], failover: auto }

flow Main { input: String = data -> output }
"#;
    let decls = metalogos::parser::parse(src).expect("the explicit local-rung program must parse");
    let llm_decl = decls
        .iter()
        .find_map(|d| match d {
            metalogos::ast::Declaration::LlmConfig(c) => Some(c),
            _ => None,
        })
        .expect("the llm {} block must appear in the declarations");
    let router = metalogos::llm::SmartRouter::from_config(llm_decl);
    assert!(
        router.has_covered_local_fallback(),
        "the explicit local rung IS the covered fallback (ADR-0208 §4) — the \
         №898 explicit-mock surface needs no env flag"
    );

    // The contrast: an EXTERNAL-only roster is not covered by a local
    // rung — the №880 uncovered half stands (the honest exhaustion).
    let src2 = r#"
llm { providers: [{alias: main, provider: anthropic, key: "k"}], failover: auto }

flow Main { input: String = data -> output }
"#;
    let decls2 = metalogos::parser::parse(src2).expect("the external-roster program must parse");
    let llm_decl2 = decls2
        .iter()
        .find_map(|d| match d {
            metalogos::ast::Declaration::LlmConfig(c) => Some(c),
            _ => None,
        })
        .expect("the second llm {} block must appear");
    let router2 = metalogos::llm::SmartRouter::from_config(llm_decl2);
    assert!(
        !router2.has_covered_local_fallback(),
        "an external-only roster is NOT covered by a local rung — the №880 \
         uncovered half stands"
    );
    strip_all_llm_env();
}
