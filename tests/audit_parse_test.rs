#[test]
fn test_parse_mlogserver_from_integration() {
    let source = r#"
mlogserver {
  port: 8080
  middleware: [session, csrf, security_headers]
  route "/" method=GET { return "Hello" }
}
"#;
    let decls = metalogos::parser::parse(source).unwrap();
    eprintln!("Integration parse: {} declarations", decls.len());
    assert!(!decls.is_empty());
}
