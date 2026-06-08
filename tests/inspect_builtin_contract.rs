// ── Contract tests for inspect() builtin (ADR-0051) ─────────────────────
//
// Tests:
// 1. Call learnable pattern 3 times → inspect().calls == 3.0
// 2. inspect() on non-invoked pattern → all zeros
// 3. inspect() returns correct field names and types
// 4. adapt increments examples_count
// 5. Few-shot match counts as cache hit
// 6. Multiple patterns tracked independently

use metalogos::interpreter::Interpreter;
use metalogos::parser;

/// Helper: parse + run declarations, then eval an expression.
fn run_source(source: &str) -> Result<metalogos::interpreter::Interpreter, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut interp = Interpreter::new();
    interp.run(declarations)?;
    Ok(interp)
}

/// Helper: parse + run + eval single expression.
fn eval_expr(source: &str, expr: &str) -> Result<metalogos::interpreter::Value, String> {
    let mut interp = run_source(source)?;
    let declarations = parser::parse(expr).map_err(|e| format!("parse error: {}", e))?;
    interp.run(declarations)?;
    // The expression should produce output via let binding → return
    // Use feed_line approach
    let decls = parser::parse(
        &format!("let __result = {}", expr)
    ).map_err(|e| format!("parse error: {}", e))?;
    interp.run(decls)?;
    interp.variables.get("__result").cloned()
        .ok_or_else(|| "no result".to_string())
}

/// Helper: eval an expression by wrapping in a pattern that returns it.
fn eval_inspect(source: &str) -> Result<metalogos::interpreter::Value, String> {
    let mut interp = run_source(source)?;
    // Call inspect via a flow that invokes it
    // Simpler: use feed_line approach
    let full = format!(
        r#"
            let stats = inspect("Classify")
            return stats
        "#
    );
    let decls = parser::parse(&full).map_err(|e| format!("parse error: {}", e))?;
    let result = interp.run(decls)?;
    // Result should be the returned struct
    match result {
        Some(s) => {
            // Parse the string representation back — or get from variables
            let return_decls = parser::parse("let __r = inspect(\"Classify\")").unwrap();
            interp.run(return_decls).unwrap();
            Ok(interp.variables.get("__r").cloned().unwrap())
        }
        None => {
            let return_decls = parser::parse("let __r = inspect(\"Classify\")").unwrap();
            interp.run(return_decls).unwrap();
            interp.variables.get("__r").cloned().ok_or_else(|| "no __r".to_string())
        }
    }
}

// ── Test 1: Call Classify 3 times → inspect("Classify").calls == 3.0 ─────

#[test]
fn test_inspect_calls_count() {
    let source = r#"
        learnable pattern Classify(text: String) -> String {
            prompt: "complaint"
        }
        let _ = Classify("first")
        let _ = Classify("second")
        let _ = Classify("third")
    "#;

    let mut interp = run_source(source).unwrap();
    // Now call inspect
    let decls = parser::parse(r#"let stats = inspect("Classify")"#).unwrap();
    interp.run(decls).unwrap();

    let stats = interp.variables.get("stats").unwrap();
    let calls = stats.get_field("calls").unwrap();
    assert_eq!(calls.as_float().unwrap(), 3.0);
}

// ── Test 2: inspect() on non-invoked pattern → all zeros ───────────────

#[test]
fn test_inspect_non_invoked() {
    let source = r#"
        learnable pattern Unused(text: String) -> String {
            prompt: "test"
        }
    "#;

    let mut interp = run_source(source).unwrap();
    let decls = parser::parse(r#"let stats = inspect("Unused")"#).unwrap();
    interp.run(decls).unwrap();

    let stats = interp.variables.get("stats").unwrap();
    assert_eq!(stats.get_field("calls").unwrap().as_float().unwrap(), 0.0);
    assert_eq!(stats.get_field("avg_confidence").unwrap().as_float().unwrap(), 0.0);
    assert_eq!(stats.get_field("cache_hits").unwrap().as_float().unwrap(), 0.0);
    assert_eq!(stats.get_field("examples_count").unwrap().as_float().unwrap(), 0.0);
}

// ── Test 3: inspect() returns correct field names and type ────────────

#[test]
fn test_inspect_field_names() {
    let source = r#"
        learnable pattern X(text: String) -> String {
            prompt: "p"
        }
    "#;

    let mut interp = run_source(source).unwrap();
    let decls = parser::parse(r#"let stats = inspect("X")"#).unwrap();
    interp.run(decls).unwrap();

    let stats = interp.variables.get("stats").unwrap();
    // Should be a Struct
    assert_eq!(stats.type_name(), "Struct");
    // Should have all 5 fields
    let fields = ["calls", "avg_confidence", "cache_hits", "last_adapt", "examples_count"];
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

    let mut interp = run_source(source).unwrap();
    let decls = parser::parse(r#"let stats = inspect("Sentiment")"#).unwrap();
    interp.run(decls).unwrap();

    let stats = interp.variables.get("stats").unwrap();
    assert_eq!(stats.get_field("examples_count").unwrap().as_float().unwrap(), 2.0);
    // last_adapt should be non-zero
    let last_adapt = stats.get_field("last_adapt").unwrap().as_float().unwrap();
    assert!(last_adapt > 0.0);
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
        // These calls match few-shot examples → cache hits
        let _ = Route("reset password")
        let _ = Route("pricing")
        // This call doesn't match → goes to LLM → not cache hit
        let _ = Route("unknown query")
    "#;

    let mut interp = run_source(source).unwrap();
    let decls = parser::parse(r#"let stats = inspect("Route")"#).unwrap();
    interp.run(decls).unwrap();

    let stats = interp.variables.get("stats").unwrap();
    assert_eq!(stats.get_field("calls").unwrap().as_float().unwrap(), 3.0);
    assert_eq!(stats.get_field("cache_hits").unwrap().as_float().unwrap(), 2.0);
}

// ── Test 6: Multiple patterns tracked independently ─────────────────────

#[test]
fn test_inspect_multiple_patterns() {
    let source = r#"
        learnable pattern A(text: String) -> String {
            prompt: "a"
        }
        learnable pattern B(text: String) -> String {
            prompt: "b"
        }
        let _ = A("input")
        let _ = A("input2")
        let _ = B("input")
    "#;

    let mut interp = run_source(source).unwrap();
    let decls = parser::parse(
        r#"
            let stats_a = inspect("A")
            let stats_b = inspect("B")
        "#
    ).unwrap();
    interp.run(decls).unwrap();

    let stats_a = interp.variables.get("stats_a").unwrap();
    let stats_b = interp.variables.get("stats_b").unwrap();

    assert_eq!(stats_a.get_field("calls").unwrap().as_float().unwrap(), 2.0);
    assert_eq!(stats_b.get_field("calls").unwrap().as_float().unwrap(), 1.0);
}
