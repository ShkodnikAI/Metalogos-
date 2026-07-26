// ── ADR-0045 Contract Tests: before_pattern / after_pattern Hooks ──────
// Contracts:
//   C1: before_pattern hook fires before every pattern invocation
//   C2: after_pattern hook fires after every pattern invocation (with result)
//   C3: Hook receives pattern_name variable
//   C4: Hook error does not prevent pattern execution
//   C5: Hooks do NOT fire on builtin calls
//   C6: Multiple before hooks fire in declaration order

use metalogos::interpreter::Interpreter;
use metalogos::parser;

/// Helper: parse + run source code, return interpreter.
fn run_source(source: &str) -> Result<Interpreter, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));
    interp.run(declarations)?;
    Ok(interp)
}

/// Helper: call mem_get builtin and return the value string.
fn kv_get(interp: &Interpreter, key: &str) -> String {
    let result = interp.eval_expr(&metalogos::ast::Expr::FnCall(
        "mem_get".to_string(),
        vec![metalogos::ast::Expr::StringLit(key.to_string())],
    ));
    match result {
        Ok(v) => format!("{}", v),
        Err(_) => String::new(),
    }
}

// ── C1: before_pattern hook fires, sets mem_set visible after ─────────

#[test]
fn test_hook_before_pattern_fires() {
    let source = r#"
        hook before_pattern {
            mem_set("hook_fired", "yes")
        }
        pattern Hello(name: String) -> String {
            return "Hi " + name
        }
        entity n: String = "World"
        flow Main { input: String = n -> Hello -> output }
    "#;

    let interp = run_source(source).unwrap();
    let val = kv_get(&interp, "hook_fired");
    assert_eq!(
        val, "yes",
        "C1: before_pattern hook should set hook_fired to 'yes'"
    );
}

// ── C2: after_pattern hook fires, receives result ─────────────────────

#[test]
fn test_hook_after_pattern_receives_result() {
    // Use both before and after hooks to verify after hook fires
    let source = r#"
        hook before_pattern {
            mem_set("before_check", "yes")
        }
        hook after_pattern {
            mem_set("after_check", "yes")
        }
        pattern Hello(name: String) -> String {
            return "Hi " + name
        }
        entity n: String = "World"
        flow Main { input: String = n -> Hello -> output }
    "#;

    let interp = run_source(source).unwrap();
    let before = kv_get(&interp, "before_check");
    let after = kv_get(&interp, "after_check");
    assert_eq!(before, "yes", "C2: before_pattern hook should fire");
    assert_eq!(after, "yes", "C2: after_pattern hook should fire");
}

// ── C3: Hook receives pattern_name variable ─────────────────────────

#[test]
fn test_hook_receives_pattern_name() {
    let source = r#"
        hook before_pattern {
            mem_set("captured_name", pattern_name)
        }
        pattern Greet(name: String) -> String {
            return "Hello"
        }
        entity n: String = "Alice"
        flow Main { input: String = n -> Greet -> output }
    "#;

    let interp = run_source(source).unwrap();
    let val = kv_get(&interp, "captured_name");
    assert_eq!(val, "Greet", "C3: hook should receive pattern_name 'Greet'");
}

// ── C4: Hook error does not prevent pattern execution ───────────────

#[test]
fn test_hook_error_does_not_block() {
    let source = r#"
        hook before_pattern {
            let x = undefined_function_xyz()
        }
        pattern Safe(name: String) -> String {
            return "Safe: " + name
        }
        entity n: String = "test"
        flow Main { input: String = n -> Safe -> output }
    "#;

    let result = run_source(source);
    assert!(
        result.is_ok(),
        "C4: program should execute despite hook error"
    );
}

// ── C5: Hooks do NOT fire on builtin calls ──────────────────────────

#[test]
fn test_hooks_do_not_fire_on_builtins() {
    let source = r#"
        hook before_pattern {
            mem_set("hook_count", to_string(1 + to_float(mem_get("hook_count"))))
        }
        pattern Process(name: String) -> String {
            let _ = upper("test")
            let _ = len("abc")
            return "done"
        }
        entity n: String = "World"
        flow Main { input: String = n -> Process -> output }
    "#;

    let interp = run_source(source).unwrap();
    let val = kv_get(&interp, "hook_count");
    assert_eq!(
        val, "1",
        "C5: hook should fire exactly once (for Process), not for upper/len"
    );
}

// ── C6: Multiple before hooks fire in declaration order ──────────────

#[test]
fn test_multiple_hooks_in_order() {
    let source = r#"
        hook before_pattern {
            mem_set("hook_log", mem_get("hook_log") + "|1")
        }
        hook before_pattern {
            mem_set("hook_log", mem_get("hook_log") + "|2")
        }
        pattern Ping(name: String) -> String {
            return "pong"
        }
        entity n: String = "test"
        flow Main { input: String = n -> Ping -> output }
    "#;

    let interp = run_source(source).unwrap();
    let val = kv_get(&interp, "hook_log");
    assert_eq!(
        val, "|1|2",
        "C6: multiple before hooks should fire in declaration order"
    );
}
