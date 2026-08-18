// ── Integration tests: mlog check (semantic analysis) ──────────────

#[test]
fn check_ok_program() {
    let source = r#"
        entity greeting: String = "Hello, Metalogos!"
        pattern SayHello(text: String) -> String { return text }
        flow Main { input: String = greeting -> SayHello -> output }
    "#;
    let result = metalogos::check_program(source).unwrap();
    assert!(result.is_ok());
    assert_eq!(result.error_count(), 0);
}

#[test]
fn check_undefined_type_error() {
    let source = r#"
        entity m: UnknownType = { text: "hi" }
    "#;
    let result = metalogos::check_program(source).unwrap();
    assert!(!result.is_ok());
    assert!(result.errors.iter().any(|e| e.contains("unknown type")));
}

#[test]
fn check_adapt_target_not_found() {
    let source = r#"
        adapt NonExistent add_example("in", "out")
    "#;
    let result = metalogos::check_program(source).unwrap();
    assert!(!result.is_ok());
    assert!(result.errors.iter().any(|e| e.contains("not found")));
}

#[test]
fn check_duplicate_entity_type() {
    let source = r#"
        entity Message { text: String }
        entity Message { text: String, urgency: Float }
    "#;
    let result = metalogos::check_program(source).unwrap();
    assert!(!result.is_ok());
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("duplicate entity type")));
}

#[test]
fn check_format_no_issues() {
    let source = r#"
        entity x: String = "test"
    "#;
    let result = metalogos::check_program(source).unwrap();
    let fmt = result.format();
    assert!(fmt.contains("OK"), "format should contain 'OK': {}", fmt);
}

// ── Наряд №98: Category A audit as compiler errors ────────────────
//
// These three checks are structural security invariants that have been
// promoted from `mlog audit` (advisory) to `mlog check` (compiler errors).
// A violation is never a legitimate false positive — the code IS insecure.

/// SQL_DYNAMIC: query() with non-literal SQL must be a compile-time error.
/// This is the core SQL injection prevention — only parameterized queries
/// with literal SQL strings are allowed.
#[test]
fn n98_sql_dynamic_rejected_by_check() {
    // query(variable) is SQL injection — must not pass mlog check
    let source = r#"
        pattern BadQuery(table: String) -> String {
            let result = query(table)
            return result
        }
    "#;
    let result = metalogos::check_program(source).unwrap();
    assert!(!result.is_ok(), "query(variable) should fail check");
    assert!(
        result.errors.iter().any(|e| e.contains("SQL_DYNAMIC")),
        "error should mention SQL_DYNAMIC, got: {:?}",
        result.errors
    );
}

/// SQL_DYNAMIC: query() with literal SQL passes check.
#[test]
fn n98_sql_literal_passes_check() {
    let source = r#"
        pattern GoodQuery(id: String) -> String {
            let result = query("SELECT * FROM users WHERE id = $1", [id])
            return result
        }
    "#;
    let result = metalogos::check_program(source).unwrap();
    assert!(
        result.is_ok(),
        "query(literal) should pass check, got: {:?}",
        result.errors
    );
}

/// SECRET_LEAK: env() result passed to respond() must be a compile-time error.
/// There is no legitimate reason to include a secret in an HTTP response body.
#[test]
fn n98_secret_leak_rejected_by_check() {
    let source = r#"
        mlogserver {
            port: 8080
            route "/leak" method=GET {
                let key = env("API_KEY")
                respond(key)
            }
        }
    "#;
    let result = metalogos::check_program(source).unwrap();
    assert!(!result.is_ok(), "env() to respond() should fail check");
    assert!(
        result.errors.iter().any(|e| e.contains("SECRET_LEAK")),
        "error should mention SECRET_LEAK, got: {:?}",
        result.errors
    );
}

/// SECRET_LEAK: env() used in non-sink context passes check.
#[test]
fn n98_secret_used_safely_passes_check() {
    let source = r#"
        entity api_key: Secret = env("API_KEY")
        pattern UseKey() -> String {
            let result = http_post("https://api.example.com", "", env("API_KEY"))
            return result
        }
    "#;
    let result = metalogos::check_program(source).unwrap();
    // http_post with secret in headers position (arg 3+) is allowed
    // This should not have SECRET_LEAK error
    assert!(
        !result.errors.iter().any(|e| e.contains("SECRET_LEAK")),
        "env() in http_post headers should not be SECRET_LEAK, got: {:?}",
        result.errors
    );
}

/// HTML_INJECTION: LLM output passed to respond() without sanitization
/// must be a compile-time error. This is XSS prevention.
#[test]
fn n98_html_injection_rejected_by_check() {
    let source = r#"
        mlogserver {
            port: 8080
            route "/ask" method=POST {
                let data = form_data()
                let reply = call_llm(data.question)
                respond(reply)
            }
        }
    "#;
    let result = metalogos::check_program(source).unwrap();
    assert!(!result.is_ok(), "LLM output to respond() should fail check");
    assert!(
        result.errors.iter().any(|e| e.contains("HTML_INJECTION")),
        "error should mention HTML_INJECTION, got: {:?}",
        result.errors
    );
}

/// HTML_INJECTION: LLM output via render() passes check (template auto-escaping).
#[test]
fn n98_html_injection_sanitized_passes_check() {
    let source = r#"
        template Page(content: String) -> Html {
            <div>{{ content }}</div>
        }
        mlogserver {
            port: 8080
            route "/ask" method=GET {
                let reply = call_llm("hello")
                let page = render(Page, reply)
                return page
            }
        }
    "#;
    let result = metalogos::check_program(source).unwrap();
    assert!(
        !result.errors.iter().any(|e| e.contains("HTML_INJECTION")),
        "LLM output via render() should not be HTML_INJECTION, got: {:?}",
        result.errors
    );
}

/// Verify that mlog run also blocks on Category A violations.
#[test]
fn n98_run_blocks_on_sql_dynamic() {
    let source = r#"
        pattern BadQuery(table: String) -> String {
            let result = query(table)
            return result
        }
    "#;
    let result = metalogos::run_program(source);
    assert!(result.is_err(), "mlog run should block on SQL_DYNAMIC");
    assert!(
        result.unwrap_err().contains("SQL_DYNAMIC"),
        "error should mention SQL_DYNAMIC"
    );
}
