// ── tests/n892_respond_html_contract.rs ─────────────────────────────
// Issue #892 — the respond_html contract restored.
//
// The 0.27.x regression (prod: FOSVED-office-v2 FORGE pages 500/404):
//   (а) the 1-arg office corpus form `respond_html(html)` failed the
//       registry/semantic arity check (spec said 2) → 500
//       "requires an argument at position 1" on every HTML route;
//   (б) the 2-arg form treated arg 0 as an HTTP status — the office's
//       (title, body) call had its title silently dropped, and the body
//       was served text/plain (the axum default for a bare String body —
//       the server never set Content-Type despite the old doc comment).
//
// The restored contract (all forms carry text/html; charset=utf-8):
//   1. respond_html(html) — body verbatim, status 200;
//   2. respond_html(status, html) — first arg opens with a valid HTTP
//      status token → body verbatim, that status;
//   3. respond_html(title, body) — otherwise → full HTML document,
//      title in <head><title> (tag-stripped) and at the top of <body>
//      (verbatim) — not dropped.
//
// The issue's minimal reproduction (0.27.1 bin/mlog from the CI artifact):
//   GET /two-args → 200 text/plain, body = 2nd arg, title dropped  (broken)
//   GET /one-arg  → 500 "requires an argument at position 1"        (broken)
// Both shapes are pinned here at the builtin/semantic level; the
// server-level Content-Type is pinned in server.rs tests (value_to_response).

use metalogos::interpreter::{Interpreter, Value};
use metalogos::parser;

const HTML_CT: &str = "text/html; charset=utf-8";

fn call_respond_html(args: &[Value]) -> Result<Value, String> {
    let interp = Interpreter::new();
    let f = interp
        .get_builtin("respond_html")
        .expect("respond_html must be registered");
    f(args)
}

fn s(v: &str) -> Value {
    Value::String(v.to_string())
}

fn as_http_response(v: Value) -> (u16, String, Option<String>) {
    match v {
        Value::HttpResponse {
            status,
            body,
            content_type,
        } => (status, body, content_type),
        other => panic!("expected HttpResponse, got {}", other.type_name()),
    }
}

// ── Form 1: the office corpus 1-arg form ────────────────────────────

#[test]
fn one_arg_form_body_verbatim_status_200() {
    let (status, body, ct) =
        as_http_response(call_respond_html(&[s("<p>Только тело</p>")]).unwrap());
    assert_eq!(status, 200);
    assert_eq!(
        body, "<p>Только тело</p>",
        "1-arg body must be verbatim (no wrapping)"
    );
    assert_eq!(ct.as_deref(), Some(HTML_CT));
}

#[test]
fn one_arg_form_multiline_html_document() {
    let html = "<!DOCTYPE html>\n<html><body><h1>Fosved</h1></body></html>";
    let (status, body, ct) = as_http_response(call_respond_html(&[s(html)]).unwrap());
    assert_eq!(status, 200);
    assert_eq!(body, html);
    assert_eq!(ct.as_deref(), Some(HTML_CT));
}

// ── Form 2: the legacy documented (status, html) ────────────────────

#[test]
fn two_arg_status_form_bare_status() {
    let (status, body, ct) =
        as_http_response(call_respond_html(&[s("200"), s("<h1>x</h1>")]).unwrap());
    assert_eq!(status, 200);
    assert_eq!(body, "<h1>x</h1>", "(status, html) body must be verbatim");
    assert_eq!(ct.as_deref(), Some(HTML_CT));
}

#[test]
fn two_arg_status_form_status_line_with_reason() {
    let (status, body, _) =
        as_http_response(call_respond_html(&[s("404 Not Found"), s("<p>missing</p>")]).unwrap());
    assert_eq!(status, 404);
    assert_eq!(body, "<p>missing</p>");
}

#[test]
fn two_arg_status_form_is_not_document_wrapped() {
    // The documented form must NOT be wrapped into a document — verbatim.
    let (_, body, _) = as_http_response(call_respond_html(&[s("200"), s("<b>raw</b>")]).unwrap());
    assert!(!body.contains("<!DOCTYPE"));
    assert_eq!(body, "<b>raw</b>");
}

// ── Form 3: the office (title, body) — title NOT dropped ────────────

#[test]
fn two_arg_title_body_form_full_document() {
    let (status, body, ct) =
        as_http_response(call_respond_html(&[s("<h1>Заголовок</h1>"), s("<p>Тело</p>")]).unwrap());
    assert_eq!(status, 200);
    assert_eq!(ct.as_deref(), Some(HTML_CT));
    assert!(
        body.starts_with("<!DOCTYPE html>"),
        "must be a full document"
    );
    assert!(
        body.contains("<title>Заголовок</title>"),
        "tag-stripped title lands in <head><title>"
    );
    assert!(
        body.contains("<h1>Заголовок</h1>"),
        "the title fragment is rendered in the body — not dropped"
    );
    assert!(
        body.contains("<p>Тело</p>"),
        "the body lands in the document"
    );
}

#[test]
fn two_arg_title_body_form_plain_text_title() {
    // A plain-text title gets escaped into <head><title> and rendered as-is.
    let (status, body, _) =
        as_http_response(call_respond_html(&[s("My page"), s("<p>content</p>")]).unwrap());
    assert_eq!(status, 200);
    assert!(body.contains("<title>My page</title>"));
    assert!(body.contains("My page"));
    assert!(body.contains("<p>content</p>"));
}

#[test]
fn two_arg_out_of_range_number_is_a_title_not_a_status() {
    // "2001" parses as u16 but is outside 100..=599 — title form, not status.
    let (status, body, _) =
        as_http_response(call_respond_html(&[s("2001"), s("<p>x</p>")]).unwrap());
    assert_eq!(status, 200);
    assert!(body.starts_with("<!DOCTYPE html>"), "document form");
    assert!(body.contains("<title>2001</title>"));
}

// ── Semantic: the 1-arg office call passes static checks ────────────

#[test]
fn semantic_one_arg_call_passes_arity_check() {
    let source = r#"
mlogserver {
  port: 8080
  route "/one-arg" method=GET {
    return respond_html("<p>Только тело</p>")
  }
}
"#;
    let declarations = parser::parse(source).expect("parse");
    let result = metalogos::semantic::check_program(&declarations);
    assert!(
        result.is_ok(),
        "1-arg respond_html must pass the semantic arity check (#892), errors: {:?}",
        result.errors.iter().take(3).collect::<Vec<_>>()
    );
}

#[test]
fn semantic_three_args_still_fails() {
    let source = r#"
mlogserver {
  port: 8080
  route "/three-args" method=GET {
    return respond_html("200", "<p>x</p>", "extra")
  }
}
"#;
    let declarations = parser::parse(source).expect("parse");
    let result = metalogos::semantic::check_program(&declarations);
    assert!(
        !result.is_ok(),
        "3-arg respond_html must still fail the semantic arity check"
    );
}

// ── Registry: the spec mirrors the restored contract ────────────────

#[test]
fn registry_spec_allows_1_and_2_args() {
    use metalogos::builtins::check_builtin_arity;
    assert!(
        check_builtin_arity("respond_html", 1).is_ok(),
        "#892: 1-arg form"
    );
    assert!(check_builtin_arity("respond_html", 2).is_ok(), "2-arg form");
    assert!(check_builtin_arity("respond_html", 0).is_err());
    assert!(check_builtin_arity("respond_html", 3).is_err());
}
