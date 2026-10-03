// ── tests/naryad_565_respond_html_forms.rs ───────────────────────────
// №565 (Wave 25 P2 tail; the audit 02.10 §7.2 W-3; dispatch gh#925):
// the explicit respond_html forms — respond_html_status(status, body)
// and respond_html_doc(title, body) — the sense lives in the NAME, not
// in the CONTENT of the first argument. The audit's examples:
//   respond_html("200 причин выбрать нас", body) → HTTP 200, the title
//     silently lost (a number-like string read as a status);
//   respond_html("404 — страница-пасхалка", body) → HTTP 404 for a page
//     that must serve 200.
// The 2-arg respond_html stays for one release, DEPRECATED: a warn-only
// semantic pass announces every 2-arg call whose first argument is NOT
// a string literal (the №474 posture — warnings only, no rejection).
// The egress gates (HTML_INJECTION, OPEN_REDIRECT, SECRET_LEAK,
// recall-taint) treat the new names EXACTLY like respond_html — an
// explicit name is not a bypass.
#![allow(clippy::disallowed_methods)]

use metalogos::builtins::{sig_types::Type, BUILTIN_REGISTRY};
use metalogos::interpreter::{Interpreter, Value};

const HTML_CT: &str = "text/html; charset=utf-8";

fn s(v: &str) -> Value {
    Value::String(v.to_string())
}

fn call_builtin(name: &str, args: &[Value]) -> Result<Value, String> {
    let interp = Interpreter::new();
    let f = interp
        .get_builtin(name)
        .unwrap_or_else(|| panic!("{} must be registered", name));
    f(args)
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

fn run_tw(source: &str) -> Option<String> {
    metalogos::run_program(source).expect("the TW flow must run")
}

fn run_vm(source: &str) -> Option<String> {
    let declarations =
        metalogos::parser::parse(source).expect("parse must succeed for the VM flow");
    let program = metalogos::compiler::Compiler::new()
        .compile(declarations)
        .expect("compile must succeed for the VM flow");
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program).expect("the VM flow must run")
}

// ── the explicit status form ─────────────────────────────────────────

#[test]
fn n565_status_form_names_the_sense_and_serves_verbatim() {
    let (status, body, ct) = as_http_response(
        call_builtin(
            "respond_html_status",
            &[s("404 Not Found"), s("<p>gone</p>")],
        )
        .unwrap(),
    );
    assert_eq!(
        status, 404,
        "the status is explicit — not guessed from content"
    );
    assert_eq!(body, "<p>gone</p>", "the body is verbatim");
    assert_eq!(
        ct.as_deref(),
        Some(HTML_CT),
        "the №892 content-type contract"
    );
}

#[test]
fn n565_status_form_accepts_a_whole_number_float() {
    let (status, _, _) = as_http_response(
        call_builtin("respond_html_status", &[Value::Float(404.0), s("b")]).unwrap(),
    );
    assert_eq!(status, 404);
}

#[test]
fn n565_status_form_refuses_loudly_instead_of_the_legacy_silent_200() {
    // The legacy 2-arg form fell back to unwrap_or(200) — the exact
    // silence the audit flagged. The explicit form has no legacy debt:
    // a non-status first argument is a LOUD argument error.
    for bad in [
        s("banana"),
        s(""),
        Value::Float(200.5),
        Value::Float(99.0),
        Value::Float(600.0),
    ] {
        let err = call_builtin("respond_html_status", &[bad, s("b")])
            .expect_err("a non-status must refuse loudly");
        assert!(
            err.contains("respond_html_status()"),
            "the error names the form: {}",
            err
        );
    }
    let err =
        call_builtin("respond_html_status", &[s("404")]).expect_err("the arity is strict (2)");
    assert!(err.contains("exactly 2"), "{}", err);
}

// ── the explicit document form ───────────────────────────────────────

#[test]
fn n565_doc_form_title_is_never_a_status() {
    // The audit's exact example: "404 — страница-пасхалка" must serve 200
    // with the title PRESENT, not HTTP 404 with the title lost.
    let (status, body, ct) = as_http_response(
        call_builtin(
            "respond_html_doc",
            &[s("404 — страница-пасхалка"), s("<p>the real body</p>")],
        )
        .unwrap(),
    );
    assert_eq!(status, 200);
    assert!(
        body.contains("<title>404 — страница-пасхалка</title>"),
        "the title lands in <head>: {}",
        body
    );
    assert!(
        body.contains("the real body"),
        "the body is present: {}",
        body
    );
    assert_eq!(ct.as_deref(), Some(HTML_CT));

    // The other audit example: a number-like title is a TITLE here.
    let (status2, body2, _) = as_http_response(
        call_builtin(
            "respond_html_doc",
            &[s("200 причин выбрать нас"), s("<p>b</p>")],
        )
        .unwrap(),
    );
    assert_eq!(status2, 200, "no status guessing in the doc form");
    assert!(
        body2.contains("200 причин выбрать нас"),
        "the title survives: {}",
        body2
    );
}

// ── the 1-arg SSOT form is unchanged ─────────────────────────────────

#[test]
fn n565_one_arg_form_unchanged() {
    let (status, body, ct) =
        as_http_response(call_builtin("respond_html", &[s("<p>body only</p>")]).unwrap());
    assert_eq!(status, 200);
    assert_eq!(body, "<p>body only</p>");
    assert_eq!(ct.as_deref(), Some(HTML_CT));
}

// ── both backends dispatch the new names ─────────────────────────────

const STATUS_FLOW: &str = r#"
pattern Make404(input: String) -> HttpResponse {
    let r = respond_html_status("404 Not Found", "<p>gone</p>")
    return r
}
flow Main { input: String = "x" -> Make404 -> output }
"#;

const DOC_FLOW: &str = r#"
pattern MakeDoc(input: String) -> HttpResponse {
    let r = respond_html_doc("My Page", "<p>content</p>")
    return r
}
flow Main { input: String = "x" -> MakeDoc -> output }
"#;

#[test]
fn n565_tw_flow_dispatches_the_explicit_forms() {
    let out = run_tw(STATUS_FLOW).expect("the status flow produces output");
    assert!(
        out.contains("HttpResponse 404"),
        "the TW lane returns the explicit status: {}",
        out
    );
    let out = run_tw(DOC_FLOW).expect("the doc flow produces output");
    assert!(
        out.contains("HttpResponse 200"),
        "the doc form serves 200: {}",
        out
    );
}

#[test]
fn n565_vm_flow_dispatches_the_explicit_forms() {
    let out = run_vm(STATUS_FLOW).expect("the status flow produces output on the VM");
    assert!(
        out.contains("HttpResponse 404"),
        "the VM lane matches the TW lane (the parity of rejection and dispatch): {}",
        out
    );
    let out = run_vm(DOC_FLOW).expect("the doc flow produces output on the VM");
    assert!(out.contains("HttpResponse 200"), "{}", out);
}

// ── the deprecation warning (warn-only, the №474 posture) ────────────

#[test]
fn n565_deprecation_warning_fires_on_a_non_literal_first_arg() {
    let src = r#"
pattern Office(input: String) -> String {
    let title = "page title"
    let page = respond_html(title, "<p>body</p>")
    return "x"
}
"#;
    let result = metalogos::check_program(src).expect("check must run");
    let deprecated: Vec<_> = result
        .warnings
        .iter()
        .filter(|w| w.message.contains("deprecated") && w.message.contains("respond_html"))
        .collect();
    assert_eq!(
        deprecated.len(),
        1,
        "exactly one deprecation warning for the 2-arg non-literal call: {:?}",
        result.warnings
    );
    let msg = &deprecated[0].message;
    assert!(
        msg.contains("respond_html_status") && msg.contains("respond_html_doc"),
        "the warning names BOTH migration paths: {}",
        msg
    );
    // Warn-only: no error was added by the pass.
    assert!(
        result.is_ok(),
        "the deprecation pass must not reject the program: {:?}",
        result.errors
    );
}

#[test]
fn n565_no_deprecation_warning_for_the_literal_and_one_arg_forms() {
    let src = r#"
pattern Office(input: String) -> String {
    let a = respond_html("404 Not Found", "<p>body</p>")
    let b = respond_html("<p>body only</p>")
    return "x"
}
"#;
    let result = metalogos::check_program(src).expect("check must run");
    let deprecated: Vec<_> = result
        .warnings
        .iter()
        .filter(|w| w.message.contains("deprecated") && w.message.contains("respond_html"))
        .collect();
    assert!(
        deprecated.is_empty(),
        "a literal first argument keeps working silently this release (the audit's wording): {:?}",
        deprecated
    );
}

#[test]
fn n565_warning_reaches_the_route_bodies_too() {
    let src = r#"
mlogserver {
    port: 18099
    route "/page" method=GET {
        let t = title_from_db()
        respond_html(t, "<p>body</p>")
        return respond("200 ok")
    }
}
"#;
    let result = metalogos::check_program(src).expect("check must run");
    let deprecated: Vec<_> = result
        .warnings
        .iter()
        .filter(|w| w.message.contains("deprecated") && w.message.contains("respond_html"))
        .collect();
    assert_eq!(
        deprecated.len(),
        1,
        "the route body is the office's exact hazard site: {:?}",
        result.warnings
    );
}

// ── the egress gates: the explicit names are not a bypass ────────────

fn has_finding(source: &str, check_id: &str) -> bool {
    let result = metalogos::audit_program(source).expect("audit");
    result.findings.iter().any(|f| f.check_id == check_id)
}

#[test]
fn n565_html_injection_rides_the_new_names() {
    let status_form = r#"
pattern Joke(input: String) -> String {
    let j = call_llm("Tell me a joke")
    respond_html_status("200 OK", j)
    return "x"
}
flow Main { input: String = "x" -> Joke -> output }
"#;
    assert!(
        has_finding(status_form, "HTML_INJECTION"),
        "the explicit status form is the same LLM egress sink"
    );

    let doc_form = r#"
pattern Page(input: String) -> String {
    let g = call_llm("write a page")
    respond_html_doc("Generated", g)
    return "x"
}
flow Main { input: String = "x" -> Page -> output }
"#;
    assert!(
        has_finding(doc_form, "HTML_INJECTION"),
        "the explicit doc form is the same LLM egress sink"
    );
}

#[test]
fn n565_secret_leak_rides_the_new_names() {
    let src = r#"
pattern Leak(input: String) -> String {
    let token = secret("api_key")
    respond_html_doc("Config", token)
    return "x"
}
"#;
    assert!(
        has_finding(src, "SECRET_LEAK"),
        "a secret through the explicit doc form is the same leak"
    );
}

// ── the registry + classification facts ──────────────────────────────

#[test]
fn n565_registry_rows_are_strict_and_typed() {
    for (name, ret) in [
        ("respond_html_status", Type::Struct),
        ("respond_html_doc", Type::Struct),
    ] {
        let spec = BUILTIN_REGISTRY
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("{} must be registered", name));
        assert_eq!(spec.arity, 2, "{}: exactly 2", name);
        assert_eq!(
            spec.max_arity,
            Some(2),
            "{}: no range — the sense is in the name",
            name
        );
        assert_eq!(
            spec.return_type, ret,
            "{}: the HttpResponse struct shape (the html_to_pdf precedent)",
            name
        );
    }
}
