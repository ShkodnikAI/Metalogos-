// ── Contract tests for inspect() builtin (ADR-0051) ─────────────────────
//
// Tests:
// 1. Call regular pattern 3 times via flow -> inspect().calls == 3.0
// 2. inspect() on non-invoked learnable pattern -> all zeros, is_learnable==1
// 3. inspect() returns correct field names (8 fields) and types
// 4. adapt increments examples_count, last_adapt > 0
// 5. Few-shot match counts as cache hit, cache_misses computed
// 6. Multiple patterns tracked independently
// 7. inspect("nonexistent") -> Value::Unit (soft-failure)
// 8. Regular pattern: is_learnable == 0.0

use metalogos::interpreter::Interpreter;
use metalogos::parser;

/// Helper: parse + run declarations, return interpreter.
fn run_source(source: &str) -> Result<Interpreter, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut interp = Interpreter::new();
    interp.run(declarations)?;
    Ok(interp)
}

// ── Test 1: Call regular pattern 3 times via flow -> inspect().calls == 3 ──

#[test]
fn test_inspect_calls_count() {
    let source = r#"
        pattern Hello(name: String) -> String {
            return "Hi " + name
        }
        entity a: String = "World"
        flow Setup { input: String = a -> Hello -> Hello -> Hello -> output }
    "#;

    let interp = run_source(source).unwrap();
    let stats = interp.inspect_pattern("Hello");
    assert_eq!(stats.get_field("calls").unwrap().as_float().unwrap(), 3.0);
    assert_eq!(
        stats.get_field("is_learnable").unwrap().as_float().unwrap(),
        0.0
    );
}

// ── Test 2: inspect() on non-invoked learnable pattern -> all zeros ───────

#[test]
fn test_inspect_non_invoked() {
    let source = r#"
        learnable pattern Unused(text: String) -> String {
            prompt: "test"
        }
    "#;

    let interp = run_source(source).unwrap();
    let stats = interp.inspect_pattern("Unused");
    assert_eq!(stats.get_field("calls").unwrap().as_float().unwrap(), 0.0);
    assert_eq!(
        stats
            .get_field("avg_confidence")
            .unwrap()
            .as_float()
            .unwrap(),
        0.0
    );
    assert_eq!(
        stats.get_field("cache_hits").unwrap().as_float().unwrap(),
        0.0
    );
    assert_eq!(
        stats.get_field("cache_misses").unwrap().as_float().unwrap(),
        0.0
    );
    assert_eq!(
        stats.get_field("last_adapt").unwrap().as_float().unwrap(),
        0.0
    );
    assert_eq!(
        stats.get_field("last_call").unwrap().as_float().unwrap(),
        0.0
    );
    assert_eq!(
        stats
            .get_field("examples_count")
            .unwrap()
            .as_float()
            .unwrap(),
        0.0
    );
    assert_eq!(
        stats.get_field("is_learnable").unwrap().as_float().unwrap(),
        1.0
    );
}

// ── Test 3: inspect() returns correct field names and type ────────────

#[test]
fn test_inspect_field_names() {
    let source = r#"
        learnable pattern X(text: String) -> String {
            prompt: "p"
        }
    "#;

    let interp = run_source(source).unwrap();
    let stats = interp.inspect_pattern("X");
    // Should be a Struct
    assert_eq!(stats.type_name(), "Struct");
    // Should have all 8 fields
    let fields = [
        "calls",
        "avg_confidence",
        "cache_hits",
        "cache_misses",
        "last_adapt",
        "last_call",
        "examples_count",
        "is_learnable",
    ];
    for field in &fields {
        assert!(stats.get_field(field).is_ok(), "missing field: {}", field);
        // All fields should be Float
        assert_eq!(stats.get_field(field).unwrap().type_name(), "Float");
    }
}

// ── Test 4: adapt increments examples_count ────────────────────────────

#[test]
fn test_inspect_adapt_examples_count() {
    let source = r#"
        learnable pattern Sentiment(text: String) -> String {
            prompt: "positive"
        }
        adapt Sentiment add_example("good", "positive")
        adapt Sentiment add_example("bad", "negative")
    "#;

    let interp = run_source(source).unwrap();
    let stats = interp.inspect_pattern("Sentiment");
    assert_eq!(
        stats
            .get_field("examples_count")
            .unwrap()
            .as_float()
            .unwrap(),
        2.0
    );
    // last_adapt should be non-zero (recent timestamp)
    let last_adapt = stats.get_field("last_adapt").unwrap().as_float().unwrap();
    assert!(
        last_adapt > 0.0,
        "last_adapt should be > 0, got {}",
        last_adapt
    );
}

// ── Test 5: Few-shot match counts as cache hit ──────────────────────────

#[test]
fn test_inspect_few_shot_cache_hit() {
    let source = r#"
        learnable pattern Route(text: String) -> String {
            prompt: "Classify as: support | sales | billing"
        }
        adapt Route add_example("reset password", "support")
        adapt Route add_example("pricing", "sales")
        entity e1: String = "reset password"
        entity e2: String = "pricing"
        entity e3: String = "unknown query"
        flow F1 { input: String = e1 -> Route -> output }
        flow F2 { input: String = e2 -> Route -> output }
        flow F3 { input: String = e3 -> Route -> output }
    "#;

    let interp = run_source(source).unwrap();
    let stats = interp.inspect_pattern("Route");
    assert_eq!(stats.get_field("calls").unwrap().as_float().unwrap(), 3.0);
    assert_eq!(
        stats.get_field("cache_hits").unwrap().as_float().unwrap(),
        2.0
    );
    assert_eq!(
        stats.get_field("cache_misses").unwrap().as_float().unwrap(),
        1.0
    );
}

// ── Test 6: Multiple patterns tracked independently ─────────────────────

#[test]
fn test_inspect_multiple_patterns() {
    let source = r#"
        learnable pattern A(text: String) -> String { prompt: "a" }
        learnable pattern B(text: String) -> String { prompt: "b" }
        entity x: String = "input"
        flow FA { input: String = x -> A -> output }
        flow FA2 { input: String = x -> A -> output }
        flow FB { input: String = x -> B -> output }
    "#;

    let interp = run_source(source).unwrap();
    let stats_a = interp.inspect_pattern("A");
    let stats_b = interp.inspect_pattern("B");

    assert_eq!(stats_a.get_field("calls").unwrap().as_float().unwrap(), 2.0);
    assert_eq!(stats_b.get_field("calls").unwrap().as_float().unwrap(), 1.0);
}

// ── Test 7: inspect("nonexistent") -> Value::Unit (soft-failure) ───────

#[test]
fn test_inspect_nonexistent_returns_unit() {
    let source = r#"
        learnable pattern X(text: String) -> String { prompt: "p" }
    "#;

    let interp = run_source(source).unwrap();
    let result = interp.inspect_pattern("nonexistent_pattern");
    assert_eq!(result.type_name(), "Unit");
}

// ── Test 8: Regular pattern has is_learnable == 0.0 ────────────────────

#[test]
fn test_inspect_regular_pattern_is_learnable() {
    let source = r#"
        pattern Greet(name: String) -> String {
            return "Hello " + name
        }
        entity w: String = "World"
        flow F { input: String = w -> Greet -> output }
    "#;

    let interp = run_source(source).unwrap();
    let stats = interp.inspect_pattern("Greet");
    assert_eq!(
        stats.get_field("is_learnable").unwrap().as_float().unwrap(),
        0.0
    );
    assert_eq!(stats.get_field("calls").unwrap().as_float().unwrap(), 1.0);
}
