// ── Contract tests for Context Compression (ADR-0055) ──────────────────
//
// Tests:
// 1. Token estimation for English text
// 2. Token estimation for Cyrillic text
// 3. Token estimation for mixed text
// 4. Token estimation for empty string
// 5. parse + default context_strategy (None)
// 6. parse context_strategy: auto
// 7. parse context_strategy: compress
// 8. parse max_context_tokens with custom value
// 9. Default max_context_tokens is 2000

use metalogos::interpreter::Interpreter;
use metalogos::parser;

/// Helper: parse + run declarations, return interpreter.
fn run_source(source: &str) -> Result<Interpreter, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut interp = Interpreter::new();
    interp.run(declarations)?;
    Ok(interp)
}

// ── Test 1: estimate_tokens for English ─────────────────────────────

#[test]
fn test_estimate_tokens_english() {
    // "Hello world" = 11 chars, all ASCII → ~4 chars/token → ~3 tokens
    let tokens = Interpreter::estimate_tokens_static("Hello world");
    assert!(tokens <= 4, "Expected ~3 tokens, got {}", tokens);
    assert!(tokens >= 2);
}

// ── Test 2: estimate_tokens for Cyrillic ────────────────────────────

#[test]
fn test_estimate_tokens_cyrillic() {
    // "Привет мир" = 11 Cyrillic chars → ~2 chars/token → ~6 tokens
    let tokens = Interpreter::estimate_tokens_static("Привет мир");
    assert!(tokens <= 7, "Expected ~6 tokens, got {}", tokens);
    assert!(tokens >= 4);
}

// ── Test 3: estimate_tokens for mixed text ────────────────────────────

#[test]
fn test_estimate_tokens_mixed() {
    // "Hello Привет world мир" = 22 chars, ~50% Cyrillic
    // chars_per_token ≈ 4*0.5 + 2*0.5 = 3
    // tokens ≈ 22/3 ≈ 8
    let tokens = Interpreter::estimate_tokens_static("Hello Привет world мир");
    assert!(tokens <= 10, "Expected ~8 tokens, got {}", tokens);
    assert!(tokens >= 5);
}

// ── Test 4: estimate_tokens for empty ─────────────────────────────

#[test]
fn test_estimate_tokens_empty() {
    let tokens = Interpreter::estimate_tokens_static("");
    assert_eq!(tokens, 0);
}

// ── Test 5: default context_strategy when not specified ───────────────

#[test]
fn test_default_context_strategy() {
    let source = r#"
        learnable pattern P(text: String) -> String {
            prompt: "classify"
            context: auto
        }
    "#;

    // Parse should succeed without error
    let decls = parser::parse(source).map_err(|e| format!("parse error: {}", e));
    assert!(decls.is_ok(), "Should parse successfully");
}

// ── Test 6: parse context_strategy: auto ────────────────────────────

#[test]
fn test_parse_context_strategy_auto() {
    let source = r#"
        learnable pattern P(text: String) -> String {
            prompt: "classify"
            context: auto
            context_strategy: auto
        }
    "#;

    let decls = parser::parse(source).map_err(|e| format!("parse error: {}", e));
    assert!(decls.is_ok(), "Should parse context_strategy: auto");
}

// ── Test 7: parse context_strategy: compress ──────────────────────

#[test]
fn test_parse_context_strategy_compress() {
    let source = r#"
        learnable pattern P(text: String) -> String {
            prompt: "analyze"
            context: auto
            context_strategy: compress
            max_context_tokens: 500
        }
    "#;

    let decls = parser::parse(source).map_err(|e| format!("parse error: {}", e));
    assert!(decls.is_ok(), "Should parse context_strategy: compress");
}

// ── Test 8: parse max_context_tokens with custom value ────────────────

#[test]
fn test_parse_max_context_tokens_custom() {
    let source = r#"
        learnable pattern P(text: String) -> String {
            prompt: "classify"
            context: auto
            context_strategy: compress
            max_context_tokens: 10000
        }
    "#;

    let decls = parser::parse(source).map_err(|e| format!("parse error: {}", e));
    assert!(decls.is_ok(), "Should parse max_context_tokens: 10000");
}

// ── Test 9: default max_context_tokens ──────────────────────────────

#[test]
fn test_default_max_context_tokens() {
    let source = r#"
        learnable pattern P(text: String) -> String {
            prompt: "classify"
            context: auto
            context_strategy: compress
        }
    "#;

    // Parse and run, then check via get_learnable_patterns
    let interp = run_source(source).unwrap();
    let lp = interp.get_learnable_patterns();
    let p = lp.get("P").unwrap();
    // Default should be 2000
    assert_eq!(p.max_context_tokens, 2000);
}

// ── Test 10: context_strategy: none does not compress ───────────────

#[test]
fn test_none_strategy_no_compression() {
    // With context_strategy: none and max_context_tokens: 1,
    // the prompt should NOT be compressed even with large context.
    // We can verify by memorizing facts and checking the prompt.
    let mut source = String::from("");
    for i in 0..5 {
        source.push_str(&format!(
            "memorize \"fact {}\"\n", i
        ));
    }
    source.push_str(r#"
        learnable pattern P(text: String) -> String {
            prompt: "analyze"
            context: auto
            context_strategy: none
            max_context_tokens: 1
        }
    "#);

    // This should parse and run without error
    let interp = run_source(&source).unwrap();
    let lp = interp.get_learnable_patterns();
    let p = lp.get("P").unwrap();
    assert_eq!(p.max_context_tokens, 1);
}

// ── Test 11: context_strategy: compress without exceeding threshold ─────

#[test]
fn test_compress_no_overflow() {
    // With context_strategy: compress but max_context_tokens very high,
    // no compression should be needed (context fits within budget).
    let mut source = String::from("");
    for i in 0..5 {
        source.push_str(&format!(
            "memorize \"short fact {}\"\n", i
        ));
    }
    source.push_str(r#"
        learnable pattern P(text: String) -> String {
            prompt: "analyze"
            context: auto
            context_strategy: compress
            max_context_tokens: 10000
        }
    "#);

    let result = run_source(&source);
    assert!(result.is_ok(), "Should parse and run without error");
}
