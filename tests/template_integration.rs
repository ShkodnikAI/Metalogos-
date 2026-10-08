// ── Integration tests: Phase 6.2 templates + render (Наряд №115) ────

/// XSS prevention — template auto-escapes dangerous input via render().
#[test]
fn xss_prevention_template_render() {
    let source = r#"
template Card(name: String) -> Html {
<div>{{ name }}</div>
}
pattern Main(x: String) -> Html {
  return render("Card", "<script>alert(1)</script>")
}
flow F { input: String = "x" -> Main -> output }
"#;
    let out = metalogos::run_program(source).unwrap().unwrap_or_default();
    assert!(
        out.contains("&lt;script&gt;") || out.contains("&lt;script"),
        "script tags must be escaped, got: {}",
        out
    );
    assert!(
        !out.contains("<script>alert"),
        "raw script must not appear, got: {}",
        out
    );
}

/// Unknown template name → runtime error.
#[test]
fn render_unknown_template_errors() {
    let source = r#"
pattern Main(x: String) -> Html {
  return render("DoesNotExist", "x")
}
flow F { input: String = "x" -> Main -> output }
"#;
    let err = metalogos::run_program(source).unwrap_err();
    assert!(
        err.contains("unknown template") || err.contains("DoesNotExist"),
        "expected unknown template error, got: {}",
        err
    );
}

/// Basic substitution.
#[test]
fn render_basic_substitution() {
    let source = r#"
template Greeting(who: String) -> Html {
<p>Hello {{ who }}</p>
}
pattern Main(x: String) -> Html {
  return render("Greeting", "World")
}
flow F { input: String = "x" -> Main -> output }
"#;
    let out = metalogos::run_program(source).unwrap().unwrap_or_default();
    assert!(
        out.contains("Hello World") || (out.contains("Hello") && out.contains("World")),
        "got: {}",
        out
    );
}

/// Pipe syntax `{{ x | safe }}` not implemented — keep ignored with honest reason.
#[test]
#[ignore = "Known issue gh#967 §7: the pipe syntax {{ var | safe }} is not implemented (№115); composition needs an ADR"]
fn template_safe_pipe_not_implemented() {
    let source = r#"
template Layout(content: Html) -> Html {
<body>{{ content | safe }}</body>
}
"#;
    let _ = metalogos::run_program(source);
}

/// Compile-time opaque Html check not implemented — runtime only (naryad 114/115).
#[test]
/// №662 (ADR-0188 option (б), the owner's verdict gh#1123): the IMPLICIT
/// String→Html coercion refuses at CHECK time now — the assignment-position
/// rule (a String-shaped initializer into an opaque-typed entity); the
/// gh#967 §7 ignore is resolved. The runtime keeps the backstop.
fn check_html_from_string_error() {
    let source = r#"entity page: Html = "<div>" + "hello" + "</div>""#;
    let result = metalogos::check_program(source).unwrap();
    assert!(!result.is_ok());
}

/// template + render combo is valid under semantic check.
#[test]
fn check_template_render_valid() {
    let source = r#"
        template Card(name: String) -> Html { <div>{{ name }}</div> }
        server { port: 8080  route "/" method=GET { render("Card", "hello") } }
    "#;
    let result = metalogos::check_program(source).unwrap();
    assert!(result.is_ok());
}

/// Unknown template in server route — semantic checker still does not catch it.
/// №662 (ADR-0188 option (б)): the unknown-template refusal is a CHECK-time
/// fact now — render("Unknown") against the collected template declarations
/// refuses in check_program; the runtime stays the backstop for dynamic names.
#[test]
fn check_server_render_unknown_template() {
    let source = r#"server { port: 8080  route "/" method=GET { render("Unknown", "x") } }"#;
    let result = metalogos::check_program(source).unwrap();
    assert!(!result.is_ok());
}

/// Nested composition without |safe still escapes (safe default).
#[test]
fn render_escapes_nested_html_value() {
    let source = r#"
template Inner(t: String) -> Html {
<span>{{ t }}</span>
}
template Outer(c: String) -> Html {
<div>{{ c }}</div>
}
pattern Main(x: String) -> Html {
  let inner = render("Inner", "<b>x</b>")
  return render("Outer", inner)
}
flow F { input: String = "x" -> Main -> output }
"#;
    let out = metalogos::run_program(source).unwrap().unwrap_or_default();
    assert!(
        !out.contains("<b>x</b>") || out.contains("&lt;b&gt;"),
        "nested raw HTML should be escaped by default, got: {}",
        out
    );
}
