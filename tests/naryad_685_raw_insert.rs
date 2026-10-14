// ── Naryad №685 (gh#1252, В44, compiler/security): the S-1 fail-closed
//    raw insert ────────────────────────────────────────────────────────
//
// The audit's S-1 (High): N9 (gh#1219) added the raw triple-brace form
// `{{{ var }}}` to render() while EVERY static layer answered
// «sanitized» at the render boundary — the XSS scenario
// (json_body() → render() → respond_html) passed check/run/serve
// silently. The repair is three-fold and this file pins the AUDIT-LAYER
// half (the declaration gate + the call-site coercion live in the
// semantic suites; the conformance pairs pin both backends):
//
//   №685-A: the raw position CONDUCTS the taint — a json_body() value
//           landing in `{{{ }}}` of a statically-known template keeps
//           its UserInput taint downstream (the memory-persist lane
//           fires TAINT_PERSISTENCE where the pre-fix code was silent);
//   №685-B: the ESCAPED positions keep sanitizing (the negative
//           control — today's green paths stay green);
//   №685-C: LLM output into a RAW position of a statically-known
//           template fires HTML_INJECTION at respond_html (the
//           html_label sink walk reads the seeded raw positions);
//   №685-D: LLM output into an ESCAPED position does NOT fire
//           (the sanitizer semantics for the escaped form — unchanged);
//   №685-E: a DYNAMIC template name is a conservative conductor — the
//           sink walk cannot prove the template, so the raw position
//           question fails closed (HTML_INJECTION fires);
//   №685-F: clean render composition — zero findings (the honest green
//           path the naryad must NOT break).
#![allow(clippy::disallowed_methods)]

fn audit_ids(source: &str) -> Vec<&'static str> {
    metalogos::audit_program(source)
        .expect("audit parses")
        .findings
        .iter()
        .map(|f| f.check_id)
        .collect()
}

// ── №685-A: the raw position conducts UserInput to the egress gates ──

#[test]
fn n685_raw_position_conducts_taint() {
    // The audit's scenario family: a query_param value in the RAW
    // position reaches respond_html. Before the fix render() lifted the
    // taint and every gate was silent; after the fix the raw position
    // CONDUCTS — the UserInput egress gate (OPEN_REDIRECT, the
    // №140 user-controlled-respond gate) fires.
    let src = "template Page(t: String, q: String) -> Html {\n  <h>{{ t }}</h><p>{{{ q }}}</p>\n}\nserver {\n  port: 8080\n  route \"/preview\" method=POST {\n    return respond_html(render(\"Page\", \"t\", query_param(\"q\")))\n  }\n}";
    let ids = audit_ids(src);
    assert!(
        ids.contains(&"OPEN_REDIRECT"),
        "the raw position must CONDUCT the query_param taint to the egress gate; findings: {ids:?}"
    );
    // NOTE: the UserInput egress refusal (OPEN_REDIRECT) lives on the
    // advisory audit lane (check_open_redirect is not in
    // audit_category_a); the Category A HTML lane stays silent for the
    // UserInput kind by the pinned №268 policy. The Compile-BLOCKING
    // refusal for the raw class is the DECLARATION GATE itself
    // (RAW_INSERT_NOT_HTML — the semantic suites pin it).
}

// ── №685-B: the ESCAPED positions keep sanitizing ─────────────────────

#[test]
fn n685_escaped_position_still_sanitizes() {
    // The SAME program, but the tainted value lands in the ESCAPED
    // position `{{ q }}` — the sanitizer semantics are unchanged and the
    // egress gate stays silent.
    let src = "template Page(t: String, q: String) -> Html {\n  <h>{{ t }}</h><p>{{ q }}</p>\n}\nserver {\n  port: 8080\n  route \"/preview\" method=POST {\n    return respond_html(render(\"Page\", \"t\", query_param(\"q\")))\n  }\n}";
    let ids = audit_ids(src);
    assert!(
        !ids.contains(&"OPEN_REDIRECT"),
        "the escaped position must keep sanitizing; findings: {ids:?}"
    );
}

// ── №685-C: LLM output into a RAW position fires HTML_INJECTION ───────

#[test]
fn n685_llm_into_raw_position_fires_html_injection() {
    // The INLINE shape (the audit's scenario form): the sink walk meets
    // the render call at the respond argument — the seeded raw positions
    // decide, and the raw position conducts the LLM untrusted label.
    let src = "template Page(t: String, frag: String) -> Html {\n  <h>{{ t }}</h><section>{{{ frag }}}</section>\n}\nserver {\n  port: 8080\n  route \"/p\" method=GET {\n    return respond_html(render(\"Page\", \"t\", call_llm(\"p\")))\n  }\n}";
    let ids = audit_ids(src);
    assert!(
        ids.contains(&"HTML_INJECTION"),
        "LLM output into a raw position must fire HTML_INJECTION; findings: {ids:?}"
    );
}

// ── №685-D: LLM output into an ESCAPED position does NOT fire ─────────

#[test]
fn n685_llm_into_escaped_position_stays_silent() {
    let src = "template Page(t: String, frag: String) -> Html {\n  <h>{{ t }}</h><section>{{ frag }}</section>\n}\nserver {\n  port: 8080\n  route \"/p\" method=GET {\n    return respond_html(render(\"Page\", \"t\", call_llm(\"p\")))\n  }\n}";
    let ids = audit_ids(src);
    assert!(
        !ids.contains(&"HTML_INJECTION"),
        "the escaped positions keep the sanitizer semantics; findings: {ids:?}"
    );
}

// ── №685-E: a DYNAMIC template name is a conservative conductor ───────

#[test]
fn n685_dynamic_name_is_conservative_conductor() {
    // The name rides a variable — the static layers cannot prove the
    // template, so the sink walk fails closed (every argument walks).
    let src = "template Page(t: String, frag: String) -> Html {\n  <h>{{ t }}</h><section>{{ frag }}</section>\n}\nserver {\n  port: 8080\n  route \"/p\" method=GET {\n    let name = \"Page\"\n    return respond_html(render(name, \"t\", call_llm(\"p\")))\n  }\n}";
    let ids = audit_ids(src);
    assert!(
        ids.contains(&"HTML_INJECTION"),
        "a dynamic name must fail closed at the HTML sink; findings: {ids:?}"
    );
}

// ── №685-F: the honest composition produces zero findings ─────────────

#[test]
fn n685_honest_composition_zero_findings() {
    let src = "template Inner(x: String) -> Html {\n  <em>{{ x }}</em>\n}\ntemplate Outer(frag: Html) -> Html {\n  <div>{{{ frag }}}</div>\n}\nserver {\n  port: 8080\n  route \"/p\" method=GET {\n    return respond_html(render(\"Outer\", render(\"Inner\", \"hello\")))\n  }\n}";
    let ids = audit_ids(src);
    assert!(
        !ids.contains(&"HTML_INJECTION"),
        "the honest Html-producer composition must stay green; findings: {ids:?}"
    );
    // check_program must be clean too (the declaration gate + the
    // coercion rule both silent on the honest shape).
    let declarations = metalogos::parser::parse(src).expect("parse");
    let analysis = metalogos::semantic::check_program(&declarations);
    assert!(
        analysis.errors.is_empty(),
        "no semantic errors on the honest shape: {:?}",
        analysis
            .errors
            .iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
    );
}

// ── the declaration gate + the coercion rule (the semantic half) ──────

#[test]
fn n685_declaration_gate_names_parameter() {
    let src = "template T(a: String, b: String) -> Html {\n  <p>{{{ b }}}</p>\n}\npattern P(i: String) -> String {\n  return \"x\"\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let declarations = metalogos::parser::parse(src).expect("parse");
    let analysis = metalogos::semantic::check_program(&declarations);
    assert!(
        !analysis.errors.is_empty(),
        "a String raw parameter must refuse at check time"
    );
    let msg = analysis.errors[0].message.clone();
    assert!(
        analysis.errors[0].kind.stable_code() == Some("RAW_INSERT_NOT_HTML"),
        "the stable code rides the kind: {msg}"
    );
    assert!(
        msg.contains("'b'"),
        "the refusal names the parameter: {msg}"
    );
}

#[test]
fn n685_alias_resolved_html_param_passes() {
    // `type Frag = Html` — the №119 alias resolution makes the alias an
    // honest Html (the gate reads the RESOLVED type).
    let src = "type Frag = Html\ntemplate Inner(x: String) -> Html {\n  <em>{{ x }}</em>\n}\ntemplate Outer(f: Frag) -> Html {\n  <div>{{{ f }}}</div>\n}\npattern P(i: String) -> String {\n  return \"ok\"\n}\nflow Main { input: String = \"x\" -> P -> output }";
    let declarations = metalogos::parser::parse(src).expect("parse");
    let analysis = metalogos::semantic::check_program(&declarations);
    let raw_errors: Vec<_> = analysis
        .errors
        .iter()
        .filter(|e| e.message.contains("RAW_INSERT") || e.message.contains("raw insert"))
        .collect();
    assert!(
        raw_errors.is_empty(),
        "the alias-resolved Html param must pass the gate: {:?}",
        analysis
            .errors
            .iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
    );
}
