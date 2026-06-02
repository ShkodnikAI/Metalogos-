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
    assert!(result.errors.iter().any(|e| e.contains("duplicate entity type")));
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
