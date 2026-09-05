// ── НАРЯД №126 Contract Tests: Preemptive Sandbox Timeout ────────────
// Contracts:
//   C1: When MockLlm delay > sandbox timeout, the error arrives within
//       the timeout duration (preemptive), not after the mock delay.
//   C2: When MockLlm delay < sandbox timeout, the call succeeds normally.
//   C3: Without sandbox, no timeout is applied even with slow MockLlm.
//
// Наряд #156 evolution:
//   SmartRouter path: reqwest::blocking::Client::timeout() performs
//   real HTTP-level cancellation (drops TCP connection). No thread.
//   Legacy path (tests use MockLlm): thread wrapper retained because
//   LlmBackend trait has no timeout. Error message simplified — no
//   more "may still be running" (harmless for MockLlm, real cancel
//   for SmartRouter).

use metalogos::ast::*;
use metalogos::interpreter::Interpreter;
use metalogos::llm::MockLlm;
use serial_test::serial;
use std::time::Instant;

/// Helper: create a learnable pattern declaration.
fn make_learnable_decl(name: &str, prompt: &str) -> Declaration {
    Declaration::LearnablePattern(LearnablePatternDecl {
        span: metalogos::ast::Span::unknown(),
        name: name.to_string(),
        params: vec![Param {
            span: metalogos::ast::Span::unknown(),
            name: "text".to_string(),
            type_name: "String".to_string(),
        }],
        return_type: "String".to_string(),
        prompt: prompt.to_string(),
        context: None,
        context_strategy: ContextStrategy::None,
        max_context_tokens: 2000,
        max_tokens: None,
        cache: false,
        cache_ttl: 3600,
        model: None,
        conversation: None,
        // Наряд №181: distillation fields default to None (no distill).
        distill_to: None,
        distill_after: 0,
        fallback_if: None,
    })
}

/// Helper: create a sandbox declaration with given timeout.
fn make_sandbox_decl(name: &str, timeout: i64, forbidden: Vec<&str>) -> Declaration {
    Declaration::Sandbox(SandboxDecl {
        span: metalogos::ast::Span::unknown(),
        name: name.to_string(),
        allowed: vec![],
        forbidden: forbidden.into_iter().map(String::from).collect(),
        timeout,
    })
}

/// Helper: call a learnable pattern via eval_expr.
fn call_learnable(
    interp: &mut Interpreter,
    pattern_name: &str,
    arg: &str,
) -> Result<metalogos::interpreter::Value, String> {
    interp.eval_expr(&Expr::FnCall {
        name: pattern_name.to_string(),
        args: vec![Expr::StringLit {
            value: arg.to_string(),
            span: metalogos::ast::Span::unknown(),
        }],
        span: metalogos::ast::Span::unknown(),
    })
}

// ── C1: MockLlm delay (3s) > sandbox timeout (1s) → preemptive error ─────

#[test]
#[serial]
fn test_preemptive_timeout_fires_within_budget() {
    MockLlm::reset_call_count();
    MockLlm::reset_delay();

    // Simulate a slow/hung LLM: 3 second delay
    MockLlm::set_delay_ms(3000);

    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    // Register sandbox with 1-second timeout
    let _ = interp
        .run(vec![
            make_sandbox_decl("tight", 1, vec![]),
            make_learnable_decl("SlowEcho", "echo this"),
        ])
        .unwrap();

    interp.set_active_sandbox(SandboxDecl {
        span: metalogos::ast::Span::unknown(),
        name: "tight".to_string(),
        allowed: vec![],
        forbidden: vec![],
        timeout: 1,
    });

    let start = Instant::now();
    let result = call_learnable(&mut interp, "SlowEcho", "hello");
    let elapsed = start.elapsed();

    // Must fail with timeout error
    let err_msg = result
        .expect_err("C1: should time out, not succeed")
        .to_string();
    assert!(
        err_msg.contains("timed out"),
        "C1: error should mention 'timed out', got: {}",
        err_msg
    );

    // The error must arrive within ~1.5s (1s timeout + margin), NOT after 3s mock delay
    assert!(
        elapsed.as_secs() < 2,
        "C1: timeout should fire within budget (~1s), took {:?}",
        elapsed
    );

    // Наряд #156: no more "may still be running" — SmartRouter uses
    // real reqwest cancellation; legacy MockLlm background is harmless.
    MockLlm::reset_delay();
}

// ── C2: MockLlm delay (50ms) < sandbox timeout (2s) → success ─────────

#[test]
#[serial]
fn test_timeout_not_triggered_when_call_completes_in_time() {
    MockLlm::reset_call_count();
    MockLlm::reset_delay();

    // Fast mock: 50ms, well within 2s timeout
    MockLlm::set_delay_ms(50);

    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    let _ = interp
        .run(vec![
            make_sandbox_decl("relaxed", 2, vec![]),
            make_learnable_decl("FastEcho", "echo this"),
        ])
        .unwrap();

    interp.set_active_sandbox(SandboxDecl {
        span: metalogos::ast::Span::unknown(),
        name: "relaxed".to_string(),
        allowed: vec![],
        forbidden: vec![],
        timeout: 2,
    });

    let start = Instant::now();
    let result = call_learnable(&mut interp, "FastEcho", "hello");
    let elapsed = start.elapsed();

    assert!(
        result.is_ok(),
        "C2: should succeed within timeout, got err: {:?}",
        result
    );

    // Should complete in ~50ms (mock delay) + overhead, well under 2s
    assert!(
        elapsed.as_millis() < 1000,
        "C2: should complete quickly, took {:?}",
        elapsed
    );

    MockLlm::reset_delay();
}

// ── C3: No sandbox → no timeout even with slow MockLlm ────────────────

#[test]
#[serial]
fn test_no_sandbox_no_timeout_even_with_slow_call() {
    MockLlm::reset_call_count();
    MockLlm::reset_delay();

    // Slow mock: 200ms, but no sandbox → should complete normally
    MockLlm::set_delay_ms(200);

    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    let _ = interp
        .run(vec![make_learnable_decl("NoSandboxEcho", "echo this")])
        .unwrap();

    // Deliberately NOT setting any active sandbox
    let result = call_learnable(&mut interp, "NoSandboxEcho", "hello");

    assert!(
        result.is_ok(),
        "C3: without sandbox, slow call should still succeed, got err: {:?}",
        result
    );

    MockLlm::reset_delay();
}

// ── C4: Sandbox with timeout=0 → no timeout (backward compat) ────────

#[test]
#[serial]
fn test_sandbox_timeout_zero_no_timeout_applied() {
    MockLlm::reset_call_count();
    MockLlm::reset_delay();

    // Slow mock: 200ms, but sandbox timeout=0 → no timeout
    MockLlm::set_delay_ms(200);

    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    let _ = interp
        .run(vec![
            make_sandbox_decl("no_timeout", 0, vec![]),
            make_learnable_decl("ZeroEcho", "echo this"),
        ])
        .unwrap();

    interp.set_active_sandbox(SandboxDecl {
        span: metalogos::ast::Span::unknown(),
        name: "no_timeout".to_string(),
        allowed: vec![],
        forbidden: vec![],
        timeout: 0,
    });

    let result = call_learnable(&mut interp, "ZeroEcho", "hello");

    assert!(
        result.is_ok(),
        "C4: timeout=0 should mean no timeout, got err: {:?}",
        result
    );

    MockLlm::reset_delay();
}
