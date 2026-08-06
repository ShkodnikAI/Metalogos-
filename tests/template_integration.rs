// ── Integration tests: Phase 6.2 templates + render ────────────────

/// Contract test: XSS prevention — template auto-escapes dangerous input.
/// Category: Unimplemented Feature (template render not yet in interpreter)
#[test]
#[ignore] // TODO: template render not yet implemented — Interpreter::render_template unavailable
/// TODO: re-enable when Interpreter::render_template is implemented
fn xss_prevention_template_render() {
    // TODO: re-enable when Interpreter::render_template is implemented
    // let source = r#"template Card(name: String) -> Html { <div>{{ name }}</div> }"#;
    // let decls = metalogos::parser::parse(source).unwrap();
    // let mut interp = metalogos::interpreter::Interpreter::new();
    // interp.run(decls).unwrap();
    // let args = vec![
    //     metalogos::interpreter::Value::String("<script>alert(1)</script>".to_string()),
    // ];
    // let html = interp.render_template("Card", &args).unwrap();
    // assert_eq!(html, "<div>&lt;script&gt;alert(1)&lt;/script&gt;</div>");
}

/// Contract test: `{{ content | safe }}` passes Html through without escaping.
/// Category: Unimplemented Feature (template render not yet in interpreter)
#[test]
#[ignore] // TODO: template render not yet implemented — Interpreter::render_template unavailable
/// TODO: re-enable when Interpreter::render_template is implemented
fn safe_pipe_passthrough() {
    // TODO: re-enable when Interpreter::render_template is implemented
    // let source = r#"template Layout(content: Html) -> Html { <main>{{ content | safe }}</main> }"#;
    // let decls = metalogos::parser::parse(source).unwrap();
    // let mut interp = metalogos::interpreter::Interpreter::new();
    // interp.run(decls).unwrap();
    // let args = vec![
    //     metalogos::interpreter::Value::Html("<b>bold</b>".to_string()),
    // ];
    // let html = interp.render_template("Layout", &args).unwrap();
    // assert_eq!(html, "<main><b>bold</b></main>");
}

/// Contract test: multi-param template with full entity escaping.
/// Category: Unimplemented Feature (template render not yet in interpreter)
#[test]
#[ignore] // TODO: template render not yet implemented — Interpreter::render_template unavailable
/// TODO: re-enable when Interpreter::render_template is implemented
fn multi_param_escape() {
    // TODO: re-enable when Interpreter::render_template is implemented
    // let source = r#"template Page(title: String, body: String) -> Html { <h1>{{ title }}</h1><p>{{ body }}</p> }"#;
    // let decls = metalogos::parser::parse(source).unwrap();
    // let mut interp = metalogos::interpreter::Interpreter::new();
    // interp.run(decls).unwrap();
    // let args = vec![
    //     metalogos::interpreter::Value::String("Hello & <World>".to_string()),
    //     metalogos::interpreter::Value::String("Some \"quoted\" text".to_string()),
    // ];
    // let html = interp.render_template("Page", &args).unwrap();
    // assert!(html.contains("<h1>Hello &amp; &lt;World&gt;</h1>"));
    // assert!(html.contains("<p>Some &quot;quoted&quot; text</p>"));
}

/// Contract test: to_string(Html) is blocked (Html is opaque).
/// Category: Unimplemented Feature (template render not yet in interpreter)
#[test]
#[ignore] // TODO: template render not yet implemented — Interpreter::render_template unavailable
/// TODO: re-enable when Interpreter::render_template is implemented
fn html_opaque_blocks_to_string() {
    // TODO: re-enable when Interpreter::render_template is implemented
    // let source = r#"template Card(name: String) -> Html { <div>{{ name }}</div> }"#;
    // let decls = metalogos::parser::parse(source).unwrap();
    // let mut interp = metalogos::interpreter::Interpreter::new();
    // interp.run(decls).unwrap();
    // let html_val = metalogos::interpreter::Value::Html("<div>test</div>".to_string());
    // let result = interp.render_template("Card", &[html_val]).unwrap();
    // assert_eq!(result, "<div>&lt;div&gt;test&lt;/div&gt;</div>");
}

/// Contract test: semantic analysis catches Html from String.
#[test]
#[ignore] // TODO: Opaque Html type constraint not yet implemented in semantic checker
fn check_html_from_string_error() {
    let source = r#"entity page: Html = "<div>" + "hello" + "</div>""#;
    let result = metalogos::check_program(source).unwrap();
    assert!(!result.is_ok());
    assert!(result.errors.iter().any(|e| e.contains("opaque type Html")));
}

/// Contract test: template + render combo is valid.
#[test]
fn check_template_render_valid() {
    let source = r#"
        template Card(name: String) -> Html { <div>{{ name }}</div> }
        server { port: 8080  route "/" method=GET { render(Card, "hello") } }
    "#;
    let result = metalogos::check_program(source).unwrap();
    assert!(result.is_ok());
}

/// Contract test: server render with unknown template → error.
#[test]
#[ignore] // TODO: Unknown template detection not yet implemented in semantic checker
fn check_server_render_unknown_template() {
    let source = r#"server { port: 8080  route "/" method=GET { render(Unknown, "x") } }"#;
    let result = metalogos::check_program(source).unwrap();
    assert!(!result.is_ok());
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("undefined template")));
}
