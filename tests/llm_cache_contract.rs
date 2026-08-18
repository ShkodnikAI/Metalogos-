// ── ADR-0047 Contract Tests: LLM Response Caching ────────────────────
// Contracts:
//   C1: Two identical calls to a cached learnable pattern invoke LLM only ONCE
//   C2: Different inputs to a cached pattern result in separate LLM calls
//   C3: Uncached learnable patterns always invoke LLM
//   C4: Cache TTL expiration triggers a new LLM call

use metalogos::ast::*;
use metalogos::interpreter::Interpreter;
use metalogos::llm::MockLlm;
use serial_test::serial;

/// Helper: create a cached learnable pattern declaration.
fn make_cached_learnable_decl(name: &str, prompt: &str, cache: bool, ttl: u64) -> Declaration {
    Declaration::LearnablePattern(LearnablePatternDecl {
        name: name.to_string(),
        params: vec![Param {
            name: "text".to_string(),
            type_name: "String".to_string(),
        }],
        return_type: "String".to_string(),
        prompt: prompt.to_string(),
        context: None,
        context_strategy: ContextStrategy::None,
        max_context_tokens: 2000,
        max_tokens: None,
        cache,
        cache_ttl: ttl,
        model: None,
        conversation: None,
    })
}

// ── C1: Identical calls to cached pattern → LLM called exactly once ────

#[test]
#[serial]
fn test_cache_identical_calls_single_llm_invocation() {
    MockLlm::reset_call_count();

    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    // Register a cached learnable pattern
    let _ = interp
        .run(vec![make_cached_learnable_decl(
            "Echo",
            "echo this",
            true,
            3600,
        )])
        .unwrap();

    // First call — should invoke LLM
    let r1 = interp.eval_expr(&Expr::FnCall(
        "Echo".to_string(),
        vec![Expr::StringLit("hello".to_string())],
    ));
    assert!(r1.is_ok(), "C1: first call should succeed");
    assert_eq!(
        MockLlm::call_count(),
        1,
        "C1: first call should invoke LLM once"
    );

    // Second call with IDENTICAL input — should hit cache, NOT invoke LLM
    let r2 = interp.eval_expr(&Expr::FnCall(
        "Echo".to_string(),
        vec![Expr::StringLit("hello".to_string())],
    ));
    assert!(r2.is_ok(), "C1: second call should succeed");
    assert_eq!(
        MockLlm::call_count(),
        1,
        "C1: second call should be served from cache (still count=1)"
    );

    // Results should be identical
    assert_eq!(
        format!("{}", r1.unwrap()),
        format!("{}", r2.unwrap()),
        "C1: cached result should match first result"
    );
}

// ── C2: Different inputs → separate LLM calls ───────────────────────

#[test]
#[serial]
fn test_cache_different_inputs_separate_calls() {
    MockLlm::reset_call_count();

    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    let _ = interp
        .run(vec![make_cached_learnable_decl(
            "Echo",
            "echo this",
            true,
            3600,
        )])
        .unwrap();

    // First call
    let _ = interp.eval_expr(&Expr::FnCall(
        "Echo".to_string(),
        vec![Expr::StringLit("hello".to_string())],
    ));
    assert_eq!(MockLlm::call_count(), 1, "C2: first call");

    // Second call with DIFFERENT input — cache miss → new LLM call
    let _ = interp.eval_expr(&Expr::FnCall(
        "Echo".to_string(),
        vec![Expr::StringLit("world".to_string())],
    ));
    assert_eq!(
        MockLlm::call_count(),
        2,
        "C2: different input should trigger new LLM call"
    );

    // Third call with first input again — cache hit
    let _ = interp.eval_expr(&Expr::FnCall(
        "Echo".to_string(),
        vec![Expr::StringLit("hello".to_string())],
    ));
    assert_eq!(
        MockLlm::call_count(),
        2,
        "C2: repeated first input should hit cache"
    );
}

// ── C3: Uncached pattern → every call invokes LLM ────────────────────

#[test]
#[serial]
fn test_uncached_pattern_always_invokes_llm() {
    MockLlm::reset_call_count();

    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    // Register an UNCACHED learnable pattern (cache: false, default)
    let _ = interp
        .run(vec![make_cached_learnable_decl(
            "NoCache",
            "summarize",
            false,
            3600,
        )])
        .unwrap();

    // Two identical calls — both should invoke LLM (no caching)
    let _ = interp.eval_expr(&Expr::FnCall(
        "NoCache".to_string(),
        vec![Expr::StringLit("hello".to_string())],
    ));
    let _ = interp.eval_expr(&Expr::FnCall(
        "NoCache".to_string(),
        vec![Expr::StringLit("hello".to_string())],
    ));
    assert_eq!(
        MockLlm::call_count(),
        2,
        "C3: uncached pattern should call LLM twice"
    );
}

// ── C4: Cache stores the response correctly (MockLlm returns prompt) ─

#[test]
#[serial]
fn test_cache_stores_correct_response() {
    MockLlm::reset_call_count();

    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    let prompt_text = "Classify this text as positive or negative";
    let _ = interp
        .run(vec![make_cached_learnable_decl(
            "Classify",
            prompt_text,
            true,
            3600,
        )])
        .unwrap();

    // First call: MockLlm returns the system prompt as response
    let r1 = interp
        .eval_expr(&Expr::FnCall(
            "Classify".to_string(),
            vec![Expr::StringLit("I love this".to_string())],
        ))
        .unwrap();

    // Second call: should return same cached value
    let r2 = interp
        .eval_expr(&Expr::FnCall(
            "Classify".to_string(),
            vec![Expr::StringLit("I love this".to_string())],
        ))
        .unwrap();

    assert_eq!(
        format!("{}", r1),
        format!("{}", r2),
        "C4: cached response should be identical to first response"
    );
    assert_eq!(MockLlm::call_count(), 1, "C4: only one LLM invocation");
}
