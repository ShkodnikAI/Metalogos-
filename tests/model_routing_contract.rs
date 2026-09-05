// ── ADR-0048 Contract Tests: Per-pattern Model Routing ───────────────
// Contracts:
//   C1: model:"fast" with METALOGOS_LLM_MODEL_fast=model-a → LLM receives "model-a"
//   C2: model:"strong" with METALOGOS_LLM_MODEL_strong=model-b → LLM receives "model-b"
//   C3: model:"direct" without env → LLM receives "direct" as-is
//   C4: No model field → LLM receives empty (no override)

use metalogos::ast::*;
use metalogos::interpreter::Interpreter;
use metalogos::llm::MockLlm;

/// Helper: create a learnable pattern with optional model override.
fn make_model_learnable_decl(name: &str, prompt: &str, model: Option<&str>) -> Declaration {
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
        max_tokens: None,
        cache: false,
        cache_ttl: 3600,
        model: model.map(String::from),
        conversation: None,
        context_strategy: metalogos::ast::ContextStrategy::None,
        max_context_tokens: 2000,
        // Наряд №181: distillation fields default to None (no distill).
        distill_to: None,
        distill_after: 0,
        fallback_if: None,
    })
}

// ── C1: model:"fast" resolves via env METALOGOS_LLM_MODEL_fast ────────

#[serial_test::serial]
#[test]
fn test_model_routing_resolves_alias_via_env() {
    std::env::set_var("METALOGOS_LLM_MODEL_fast", "claude-haiku-4-5-20251001");
    MockLlm::reset_call_count();
    MockLlm::reset_last_model();

    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    let _ = interp
        .run(vec![make_model_learnable_decl(
            "FastClassify",
            "Classify text",
            Some("fast"),
        )])
        .unwrap();

    let _ = interp
        .eval_expr(&Expr::FnCall {
            name: "FastClassify".to_string(),
            args: vec![Expr::StringLit {
                value: "hello".to_string(),
                span: metalogos::ast::Span::unknown(),
            }],
            span: metalogos::ast::Span::unknown(),
        })
        .unwrap();

    let last_model = MockLlm::last_model();
    assert_eq!(
        last_model, "claude-haiku-4-5-20251001",
        "C1: model alias 'fast' should resolve to 'claude-haiku-4-5-20251001' via env, got: {}",
        last_model
    );

    std::env::remove_var("METALOGOS_LLM_MODEL_fast");
}

// ── C2: model:"strong" resolves via env METALOGOS_LLM_MODEL_strong ──

#[serial_test::serial]
#[test]
fn test_model_routing_different_aliases_resolve_independently() {
    std::env::set_var("METALOGOS_LLM_MODEL_fast", "model-a");
    std::env::set_var("METALOGOS_LLM_MODEL_strong", "model-b");
    MockLlm::reset_call_count();
    MockLlm::reset_last_model();

    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    let _ = interp
        .run(vec![
            make_model_learnable_decl("Fast", "Quick classify", Some("fast")),
            make_model_learnable_decl("Strong", "Deep analysis", Some("strong")),
        ])
        .unwrap();

    // Call Fast pattern → should resolve "fast" to "model-a"
    let _ = interp
        .eval_expr(&Expr::FnCall {
            name: "Fast".to_string(),
            args: vec![Expr::StringLit {
                value: "input".to_string(),
                span: metalogos::ast::Span::unknown(),
            }],
            span: metalogos::ast::Span::unknown(),
        })
        .unwrap();
    assert_eq!(
        MockLlm::last_model(),
        "model-a",
        "C2a: 'fast' alias should resolve to 'model-a'"
    );

    MockLlm::reset_last_model();

    // Call Strong pattern → should resolve "strong" to "model-b"
    let _ = interp
        .eval_expr(&Expr::FnCall {
            name: "Strong".to_string(),
            args: vec![Expr::StringLit {
                value: "input".to_string(),
                span: metalogos::ast::Span::unknown(),
            }],
            span: metalogos::ast::Span::unknown(),
        })
        .unwrap();
    assert_eq!(
        MockLlm::last_model(),
        "model-b",
        "C2b: 'strong' alias should resolve to 'model-b'"
    );

    std::env::remove_var("METALOGOS_LLM_MODEL_fast");
    std::env::remove_var("METALOGOS_LLM_MODEL_strong");
}

// ── C3: model:"unknown" without env → passed as-is to backend ───────

#[serial_test::serial]
#[test]
fn test_model_routing_passthrough_without_env() {
    // Ensure no env variable for "unknown"
    std::env::remove_var("METALOGOS_LLM_MODEL_unknown");
    MockLlm::reset_call_count();
    MockLlm::reset_last_model();

    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    let _ = interp
        .run(vec![make_model_learnable_decl(
            "Direct",
            "Direct model",
            Some("unknown"),
        )])
        .unwrap();

    let _ = interp
        .eval_expr(&Expr::FnCall {
            name: "Direct".to_string(),
            args: vec![Expr::StringLit {
                value: "test".to_string(),
                span: metalogos::ast::Span::unknown(),
            }],
            span: metalogos::ast::Span::unknown(),
        })
        .unwrap();

    let last_model = MockLlm::last_model();
    assert_eq!(
        last_model, "unknown",
        "C3: 'unknown' without env should be passed as-is, got: {}",
        last_model
    );
}

// ── C4: No model field → no override passed to LLM ────────────────────

#[serial_test::serial]
#[test]
fn test_model_routing_no_field_no_override() {
    MockLlm::reset_call_count();
    MockLlm::reset_last_model();

    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    let _ = interp
        .run(vec![make_model_learnable_decl(
            "Default",
            "Default model",
            None,
        )])
        .unwrap();

    let _ = interp
        .eval_expr(&Expr::FnCall {
            name: "Default".to_string(),
            args: vec![Expr::StringLit {
                value: "test".to_string(),
                span: metalogos::ast::Span::unknown(),
            }],
            span: metalogos::ast::Span::unknown(),
        })
        .unwrap();

    let last_model = MockLlm::last_model();
    assert_eq!(
        last_model, "",
        "C4: no model field → no override, last_model should be empty, got: '{}'",
        last_model
    );
}

// ── C5: User-defined alias with custom name ──────────────────────────

#[serial_test::serial]
#[test]
fn test_model_routing_user_defined_alias() {
    std::env::set_var("METALOGOS_LLM_MODEL_cheap", "gpt-4o-mini");
    MockLlm::reset_call_count();
    MockLlm::reset_last_model();

    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    let _ = interp
        .run(vec![make_model_learnable_decl(
            "Cheap",
            "Budget classify",
            Some("cheap"),
        )])
        .unwrap();

    let _ = interp
        .eval_expr(&Expr::FnCall {
            name: "Cheap".to_string(),
            args: vec![Expr::StringLit {
                value: "input".to_string(),
                span: metalogos::ast::Span::unknown(),
            }],
            span: metalogos::ast::Span::unknown(),
        })
        .unwrap();

    assert_eq!(
        MockLlm::last_model(),
        "gpt-4o-mini",
        "C5: 'cheap' alias should resolve to 'gpt-4o-mini'"
    );

    std::env::remove_var("METALOGOS_LLM_MODEL_cheap");
}

// ── C6: Direct model name (e.g. "gpt-4o") without env → passthrough ─

#[serial_test::serial]
#[test]
fn test_model_routing_direct_model_name() {
    // "gpt-4o" is a real model name, not an alias
    std::env::remove_var("METALOGOS_LLM_MODEL_gpt-4o");
    MockLlm::reset_call_count();
    MockLlm::reset_last_model();

    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));

    let _ = interp
        .run(vec![make_model_learnable_decl(
            "Gpt4o",
            "GPT-4o task",
            Some("gpt-4o"),
        )])
        .unwrap();

    let _ = interp
        .eval_expr(&Expr::FnCall {
            name: "Gpt4o".to_string(),
            args: vec![Expr::StringLit {
                value: "test".to_string(),
                span: metalogos::ast::Span::unknown(),
            }],
            span: metalogos::ast::Span::unknown(),
        })
        .unwrap();

    assert_eq!(
        MockLlm::last_model(),
        "gpt-4o",
        "C6: 'gpt-4o' without env should be passed as-is"
    );
}
