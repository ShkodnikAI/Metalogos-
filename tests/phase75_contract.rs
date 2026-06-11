// ── Phase 7.5 Contract Tests: Real Sandbox Enforcement & Audit Logging ──
// Contracts:
//   C1: Sandbox with forbidden:[network] blocks LLM calls
//   C2: Sandbox iteration limit (10,000) for while loops
//   C3: Audit log captures adapt operations
//   C4: Without sandbox, while loop uses existing 100,000 safety limit
//   C5: Audit log captures mutate operations
//   C6: Audit log captures unsafe_html render operations

use metalogos::ast::*;
use metalogos::interpreter::Interpreter;

/// Helper: create a SandboxDecl for testing.
fn make_sandbox(name: &str, allowed: Vec<&str>, forbidden: Vec<&str>, timeout: i64) -> SandboxDecl {
    SandboxDecl {
        name: name.to_string(),
        allowed: allowed.into_iter().map(String::from).collect(),
        forbidden: forbidden.into_iter().map(String::from).collect(),
        timeout,
    }
}

/// Helper: create a minimal learnable pattern declaration.
fn make_learnable_decl(name: &str, prompt: &str) -> Declaration {
    Declaration::LearnablePattern(LearnablePatternDecl {
        name: name.to_string(),
        params: vec![Param {
            name: "input".to_string(),
            type_name: "String".to_string(),
        }],
        return_type: "String".to_string(),
        prompt: prompt.to_string(),
        context: None,
        max_tokens: None,
        cache: false,
        cache_ttl: 3600,
        model: None,
        conversation: None,
    })
}

/// Helper: create an adapt declaration.
fn make_adapt_decl(pattern_name: &str, input: &str, output: &str) -> Declaration {
    Declaration::Adapt(AdaptDecl {
        pattern_name: pattern_name.to_string(),
        input_example: Expr::StringLit(input.to_string()),
        output_example: Expr::StringLit(output.to_string()),
    })
}

/// Helper: create a pattern with a while(true) loop body.
fn make_while_true_pattern() -> Declaration {
    Declaration::Pattern(PatternDecl {
        name: "infinite_loop".to_string(),
        params: vec![],
        return_type: "Unit".to_string(),
        body: vec![
            Statement::While {
                condition: Expr::BoolLit(true),
                body: vec![Statement::LetBinding {
                    name: "_x".to_string(),
                    value: Expr::FloatLit(1.0),
                }],
            },
        ],
    })
}

/// Helper: create a pattern with a while(true) loop body that increments a counter.
fn make_counting_while_pattern() -> Declaration {
    Declaration::Pattern(PatternDecl {
        name: "counting_loop".to_string(),
        params: vec![],
        return_type: "Unit".to_string(),
        body: vec![
            Statement::LetBinding {
                name: "counter".to_string(),
                value: Expr::FloatLit(0.0),
            },
            Statement::While {
                condition: Expr::BoolLit(true),
                body: vec![Statement::Assign {
                    name: "counter".to_string(),
                    value: Expr::BinaryOp(
                        Box::new(Expr::Ident("counter".to_string())),
                        BinOp::Add,
                        Box::new(Expr::FloatLit(1.0)),
                    ),
                }],
            },
        ],
    })
}

// ── C1: Sandbox with forbidden:[network] blocks LLM calls ─────────────

#[test]
fn test_75_sandbox_network_forbidden() {
    let mut interp = Interpreter::new();

    // Register a learnable pattern
    let _ = interp.run(vec![make_learnable_decl("Classify", "Classify this text")]).unwrap();

    // Activate sandbox with network forbidden
    interp.set_active_sandbox(make_sandbox("strict", vec![], vec!["network"], 30));

    // Try to invoke the learnable pattern (which needs LLM call)
    let result = interp.eval_expr(&Expr::FnCall(
        "Classify".to_string(),
        vec![Expr::StringLit("hello world".to_string())],
    ));

    // Should fail with network access forbidden error
    assert!(result.is_err(), "C1: LLM call should be blocked in sandbox with network forbidden");
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("network access forbidden"),
        "C1: error should mention 'network access forbidden', got: {}",
        err_msg
    );
    assert!(
        err_msg.contains("strict"),
        "C1: error should mention sandbox name 'strict', got: {}",
        err_msg
    );
}

// ── C2: Sandbox iteration limit (10,000) for while loops ─────────────

#[test]
fn test_75_sandbox_iteration_limit() {
    let mut interp = Interpreter::new();

    // Register a pattern with while(true) loop
    let _ = interp.run(vec![make_counting_while_pattern()]).unwrap();

    // Activate sandbox (no specific forbidden/timeout, just iteration limit)
    interp.set_active_sandbox(make_sandbox("limited", vec![], vec![], 30));

    // Try to invoke the pattern — should hit 10,000 iteration limit
    let result = interp.eval_expr(&Expr::FnCall(
        "counting_loop".to_string(),
        vec![],
    ));

    assert!(result.is_err(), "C2: while loop should fail in sandbox after 10,000 iterations");
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("iteration limit exceeded in sandbox"),
        "C2: error should mention 'iteration limit exceeded in sandbox', got: {}",
        err_msg
    );
}

// ── C3: Audit log captures adapt operations ──────────────────────────

#[test]
fn test_75_audit_log_adapt() {
    let mut interp = Interpreter::new();

    // Register a learnable pattern, then adapt it
    let decls = vec![
        make_learnable_decl("Translate", "Translate this text"),
        make_adapt_decl("Translate", "hello", "привет"),
    ];
    let _ = interp.run(decls).unwrap();

    // Drain audit log
    let audit = interp.take_audit_log();

    assert!(
        audit.iter().any(|e| e.contains("[AUDIT] adapt")),
        "C3: audit log should contain adapt entry, got: {:?}",
        audit
    );

    let adapt_entry = audit.iter().find(|e| e.contains("[AUDIT] adapt")).unwrap();
    assert!(
        adapt_entry.contains("Translate"),
        "C3: adapt entry should mention pattern name 'Translate', got: {}",
        adapt_entry
    );
    assert!(
        adapt_entry.contains("hello"),
        "C3: adapt entry should mention input 'hello', got: {}",
        adapt_entry
    );
    assert!(
        adapt_entry.contains("привет"),
        "C3: adapt entry should mention output 'привет', got: {}",
        adapt_entry
    );
}

// ── C4: Without sandbox, while loop uses existing 100,000 safety limit ─

#[test]
fn test_75_no_sandbox_unlimited() {
    let mut interp = Interpreter::new();

    // Register a pattern with while(true) loop
    let _ = interp.run(vec![make_counting_while_pattern()]).unwrap();

    // NO sandbox active — should use the 100,000 safety limit
    let result = interp.eval_expr(&Expr::FnCall(
        "counting_loop".to_string(),
        vec![],
    ));

    assert!(result.is_err(), "C4: while(true) should eventually fail");
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("while loop exceeded safety limit of 100000"),
        "C4: error should mention the 100,000 safety limit, got: {}",
        err_msg
    );
    assert!(
        !err_msg.contains("iteration limit exceeded in sandbox"),
        "C4: error should NOT mention sandbox (no sandbox is active), got: {}",
        err_msg
    );
}

// ── C5: Audit log captures mutate operations ────────────────────────

#[test]
fn test_75_audit_log_mutate() {
    let source = r#"
learnable pattern Classify(text: String) -> String {
  prompt: "Classify this text"
}
adapt Classify add_example("hello", "greeting")
mutate Classify {
  add_example("hi", "greeting")
}
"#;
    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));
    let _ = metalogos::run_program_with_dir(source, std::path::PathBuf::from("."));

    let mut interp2 = Interpreter::new();
    interp2.set_base_dir(std::path::PathBuf::from("."));
    let _ = interp2.run(metalogos::parser::parse(source).unwrap()).unwrap();

    let audit = interp2.take_audit_log();

    assert!(
        audit.iter().any(|e| e.contains("[AUDIT] mutate")),
        "C5: audit log should contain mutate entry, got: {:?}",
        audit
    );
    let mutate_entry = audit.iter().find(|e| e.contains("[AUDIT] mutate")).unwrap();
    assert!(
        mutate_entry.contains("Classify"),
        "C5: mutate entry should mention pattern 'Classify', got: {}",
        mutate_entry
    );
    assert!(
        mutate_entry.contains("accuracy="),
        "C5: mutate entry should mention accuracy, got: {}",
        mutate_entry
    );
}

// ── C6: Audit log captures unsafe_html render operations ─────────────

#[test]
fn test_75_audit_log_unsafe_html() {
    // Create interpreter with a template registered
    let source = r#"
template Page(title: String) -> Html {
  <h1>{{ title }}</h1>
}
"#;
    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));
    let _ = interp.run(metalogos::parser::parse(source).unwrap()).unwrap();

    // Call render() with template name + key/value pairs (3 args minimum)
    let result = interp.eval_expr(&Expr::FnCall(
        "render".to_string(),
        vec![
            Expr::StringLit("Page".to_string()),
            Expr::StringLit("title".to_string()),
            Expr::StringLit("Test".to_string()),
        ],
    ));

    // render() should succeed and return Html
    assert!(result.is_ok(), "C6: render() should succeed, got: {:?}", result);

    // Check audit log for unsafe_html entry
    let audit = interp.take_audit_log();
    assert!(
        audit.iter().any(|e| e.contains("[AUDIT] unsafe_html")),
        "C6: audit log should contain unsafe_html entry after render(), got: {:?}",
        audit
    );
    let html_entry = audit.iter().find(|e| e.contains("[AUDIT] unsafe_html")).unwrap();
    assert!(
        html_entry.contains("Page"),
        "C6: unsafe_html entry should mention template name 'Page', got: {}",
        html_entry
    );
}

// ── C7: Sandbox deactivation restores normal limits ─────────────────

#[test]
fn test_75_sandbox_deactivate_restores_limits() {
    let mut interp = Interpreter::new();

    // Register a pattern with while(true) loop
    let _ = interp.run(vec![make_counting_while_pattern()]).unwrap();

    // Activate sandbox
    interp.set_active_sandbox(make_sandbox("temp", vec![], vec![], 30));

    // Deactivate sandbox
    interp.clear_active_sandbox();

    // Should use 100,000 limit now
    let result = interp.eval_expr(&Expr::FnCall(
        "counting_loop".to_string(),
        vec![],
    ));

    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("100000"),
        "C7: after clearing sandbox, should use 100,000 limit, got: {}",
        err_msg
    );
    assert!(
        !err_msg.contains("iteration limit exceeded in sandbox"),
        "C7: after clearing sandbox, error should not mention sandbox, got: {}",
        err_msg
    );
}
