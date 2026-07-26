// ── Contract tests for Tool Abstraction (ADR-0054) ─────────────────────
//
// Tests:
// 1. tool with two methods — basic math_api double/square (contract from spec)
// 2. tool method with string operations
// 3. tool namespace isolation — same method name in different tools
// 4. tool with no methods — empty tool is valid
// 5. tool method calling a regular pattern
// 6. tool with multiple params
// 7. tool method used in flow pipeline
// 8. undefined tool — error on QualifiedCall
// 9. tool with 3 methods (telegram-like)

use metalogos::interpreter::Interpreter;
use metalogos::parser;

/// Helper: parse + run declarations, return interpreter for inspection.
fn run_source(source: &str) -> Result<Interpreter, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut interp = Interpreter::new();
    interp.run(declarations)?;
    Ok(interp)
}

/// Helper: parse source, run, and get flow output (returned from run()).
fn get_flow_output(source: &str) -> Result<String, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut interp = Interpreter::new();
    let output = interp.run(declarations)?;
    output.ok_or_else(|| "no flow output".to_string())
}

// ── Test 1: tool with two methods — double and square (contract) ───────

#[test]
fn test_tool_basic_double_square() {
    let source = r#"
        tool math_api {
            double(n: Float) -> Float { return n + n }
            square(n: Float) -> Float { return n * n }
        }
        entity x: Float = 5.0
        pattern Test(n: Float) -> String {
            let d = math_api.double(n)
            let s = math_api.square(n)
            return to_string(d) + " " + to_string(s)
        }
        flow Main { input: Float = x -> Test -> output }
    "#;

    let result = get_flow_output(source).unwrap();
    assert_eq!(result, "10 25");
}

// ── Test 2: tool method with string operations ────────────────────────

#[test]
fn test_tool_string_methods() {
    let source = r#"
        tool str_api {
            greet(name: String) -> String {
                return "Hello, " + name + "!"
            }
            repeat(text: String) -> String {
                return text + " " + text
            }
        }
        pattern Test(name: String) -> String {
            let greeting = str_api.greet(name)
            let doubled = str_api.repeat(greeting)
            return doubled
        }
        flow Main { input: String = "world" -> Test -> output }
    "#;

    let result = get_flow_output(source).unwrap();
    assert_eq!(result, "Hello, world! Hello, world!");
}

// ── Test 3: tool namespace isolation ────────────────────────────────

#[test]
fn test_tool_namespace_isolation() {
    // NOTE: tool names must not start with "tool" since "tool" is a keyword.
    // Use calc_a / calc_b instead.
    let source = r#"
        tool calc_a {
            compute(x: Float) -> Float { return x + 1.0 }
        }
        tool calc_b {
            compute(x: Float) -> Float { return x * 2.0 }
        }
        pattern Test(x: Float) -> String {
            let a = calc_a.compute(x)
            let b = calc_b.compute(x)
            return to_string(a) + " " + to_string(b)
        }
        flow Main { input: Float = 3.0 -> Test -> output }
    "#;

    let result = get_flow_output(source).unwrap();
    assert_eq!(result, "4 6");
}

// ── Test 4: tool with no methods — valid but unusable ────────────────

#[test]
fn test_tool_empty() {
    // An empty tool should parse and run without error
    let source = r#"
        tool empty_tool {
        }
    "#;

    let result = run_source(source);
    assert!(result.is_ok());
}

// ── Test 5: tool method calling a regular pattern ───────────────────

#[test]
fn test_tool_method_calls_pattern() {
    let source = r#"
        pattern helper(n: Float) -> Float {
            return n * 10.0
        }
        tool calc {
            scaled(n: Float) -> Float {
                let h = helper(n)
                return h + 1.0
            }
        }
        pattern Test(n: Float) -> String {
            let r = calc.scaled(n)
            return to_string(r)
        }
        flow Main { input: Float = 5.0 -> Test -> output }
    "#;

    let result = get_flow_output(source).unwrap();
    assert_eq!(result, "51");
}

// ── Test 6: tool with multiple params ────────────────────────────────

#[test]
fn test_tool_multi_param_flow() {
    let source = r#"
        tool math_api {
            add(a: Float, b: Float) -> Float { return a + b }
            mul(a: Float, b: Float) -> Float { return a * b }
        }
        pattern Sum(a: Float) -> String {
            let s = math_api.add(a, 7.0)
            return to_string(s)
        }
        flow Main { input: Float = 3.0 -> Sum -> output }
    "#;

    let result = get_flow_output(source).unwrap();
    assert_eq!(result, "10");
}

// ── Test 7: tool method used in flow pipeline step ────────────────────

#[test]
fn test_tool_in_flow_pipeline() {
    let source = r#"
        tool calc {
            double(n: Float) -> Float { return n * 2.0 }
        }
        pattern DoubleIt(n: Float) -> Float {
            return calc.double(n)
        }
        pattern Format(n: Float) -> String {
            return to_string(n)
        }
        flow Main { input: Float = 4.0 -> DoubleIt -> Format -> output }
    "#;

    let result = get_flow_output(source).unwrap();
    assert_eq!(result, "8");
}

// ── Test 8: undefined tool — error on QualifiedCall ──────────────────

#[test]
fn test_tool_undefined_error() {
    let source = r#"
        pattern Test(n: Float) -> String {
            let r = nonexistent.do_something(n)
            return to_string(r)
        }
        flow Main { input: Float = 5.0 -> Test -> output }
    "#;

    let result = get_flow_output(source);
    assert!(result.is_err());
    let err = result.unwrap_err();
    // QualifiedCall checks module_namespaces first
    assert!(
        err.contains("undefined module"),
        "expected 'undefined module' in error, got: {}",
        err
    );
}

// ── Test 9: tool with 3 methods (telegram-like) ────────────────────

#[test]
fn test_tool_three_methods() {
    let source = r#"
        tool api {
            base_url() -> String { return "https://example.com" }
            full_url(path: String) -> String {
                let base = api.base_url()
                return base + path
            }
            status() -> String { return "ok" }
        }
        pattern Test(path: String) -> String {
            let url = api.full_url(path)
            let st = api.status()
            return url + " " + st
        }
        flow Main { input: String = "/api" -> Test -> output }
    "#;

    let result = get_flow_output(source).unwrap();
    assert_eq!(result, "https://example.com/api ok");
}
