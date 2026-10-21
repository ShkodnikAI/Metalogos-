// ── №765 (gh#1418, Wave 59): the S-1b XSS closure — the let-form of
// render() is no longer a silent sanitizer bypass ─────────────────────
//
// The external audit (METALOGOS_AUDIT_9fa9821.md, 21.10.2026, §3 H-1,
// the S-1b finding carried from the 16.10 audit): `let page =
// render("Comment", j.author, j.text)` → `respond_html(page)` produced
// NO finding and the `<script>` in `j.text` reached the page raw — the
// binding walk answered a clean `Type::String` for ANY render result,
// the label engines listed render among the sanitizers, and the
// runtime never checked the raw slots. The fix (the audit's minimal
// repair, пп.1–3):
//
//   1. builtin_render: the raw_params computed from the template body
//      (the SAME template_scan tokenizer — ONE scan, four consumers);
//      a raw-slot value must be `Value::Html` — the same fail-closed
//      backstop the safe slots got in №695 (RAW_SLOT_NOT_HTML);
//   2. html_label::walk: the BINDING walk resolves the raw positions
//      (the shared resolver) — a labeled argument at a raw position
//      answers `Labeled(String, Untrusted)`;
//   3. render is OUT of the blanket sanitizer lists (the binary
//      predicates) and POSITION-AWARE in the label engines (the raw
//      map decides per slot — the №685-D/№685-E/n98 interlocks hold:
//      the all-escaped template keeps the sanitizer semantics, the
//      dynamic name stays the conservative conductor).
//
// The audit's §6 п.1 criteria: the let-test red BEFORE and green AFTER;
// the negative render(render()) composition without findings; CI
// blocking GREEN; the floors only-up.
#![allow(clippy::disallowed_methods)]

use metalogos::audit_program;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn audit_ids(source: &str) -> Vec<String> {
    match audit_program(source) {
        Ok(r) => r.findings.iter().map(|f| f.check_id.to_string()).collect(),
        Err(_) => vec!["<audit-error>".to_string()],
    }
}

fn check_ids(source: &str) -> Vec<String> {
    match metalogos::check_program(source) {
        Ok(r) => r
            .errors
            .iter()
            .filter_map(|e| {
                // The check-path errors carry the check id inside the
                // message (the n98/n201 probe form).
                ["HTML_INJECTION", "TAINT_INTERP", "TAINT_PERSISTENCE"]
                    .iter()
                    .find(|id| e.message.contains(*id))
                    .map(|s| s.to_string())
            })
            .collect(),
        Err(_) => vec!["<check-error>".to_string()],
    }
}

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source.trim(), repo_root())
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(repo_root());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

const AUDIT_SCENARIO: &str = r#"
template Comment(author: String, body: Html) -> Html {
  <b>{{ author }}</b><div>{{{ body }}}</div>
}
pattern P(x: String) -> String {
  let j = json_body()
  let page = render("Comment", j.author, j.text)
  let _ = respond_html(page)
  return "ok"
}
flow Main { input: String = "z" -> P -> output }
"#;

// ── The audit's §3 H-1 scenario: the let-form is CAUGHT now ──────────

#[test]
fn n765_let_form_audit_scenario_is_flagged() {
    // The exact audit scenario: the raw slot carries j.text (json_body —
    // the untrusted request lane) through a let-bound render result into
    // respond_html. BEFORE the fix: zero findings (the audit's r30
    // verification; the binding walk answered a clean String). AFTER:
    // the binding walk resolves the raw position, the variable holds
    // Labeled(String, Untrusted), the sink refuses the silence.
    let audit = audit_ids(AUDIT_SCENARIO);
    assert!(
        audit.iter().any(|id| id == "HTML_INJECTION"),
        "the audit scenario must flag HTML_INJECTION (the S-1b closure); got: {audit:?}"
    );
    // The check path carries the same verdict (the c1/n201 shape — both
    // surfaces must agree; the audit's report is the serving surface).
    let check = check_ids(AUDIT_SCENARIO);
    assert!(
        check.iter().any(|id| id == "HTML_INJECTION"),
        "the check path must flag the same let-form; got: {check:?}"
    );
}

#[test]
fn n765_query_param_into_raw_slot_is_flagged() {
    // The query_param lane through the same let-form — the user-input
    // kind conducts at the raw position too (the audit's §6 п.4 list).
    let src = r#"
template Echo(q: Html) -> Html {
  <div>{{{ q }}}</div>
}
pattern P(x: String) -> String {
  let v = query_param("q")
  let page = render("Echo", v)
  let _ = respond_html(page)
  return "ok"
}
flow Main { input: String = "z" -> P -> output }
"#;
    let audit = audit_ids(src);
    assert!(
        audit.iter().any(|id| id == "HTML_INJECTION"),
        "query_param into a raw slot must flag; got: {audit:?}"
    );
}

#[test]
fn n765_render_memory_respondhtml_is_flagged() {
    // render → memory → respond_html: the memory-taint path (the audit's
    // §6 п.4 list). The blanket sanitizer removal (is_memory_taint_
    // sanitizer) means a rendered-then-memorized LLM output keeps the
    // persist-fact — recall at the HTML sink fires.
    let src = r#"
pattern P(x: String) -> String {
  memorize call_llm("draft the page") with priority=0.9
  let page = recall("draft-the-page")
  let _ = respond_html(page)
  return "ok"
}
flow Main { input: String = "z" -> P -> output }
"#;
    let audit = audit_ids(src);
    assert!(
        audit.iter().any(|id| id == "TAINT_INTERP" || id == "TAINT_PERSISTENCE" || id == "HTML_INJECTION"),
        "render→memory→respond_html must keep the taint fact (the interp contour arms the persist-fact — the c1 shape); got: {audit:?}"
    );
}

// ── The runtime backstop: a plain String in a raw slot refuses ───────

#[test]
fn n765_raw_slot_runtime_backstop_refuses_string() {
    // The checked TW path: the static implicit-coercion gate refuses a
    // plain String in the Html-typed param at the call site. The VM
    // compile carries no coercion gate — the value lands on the RUNTIME
    // backstop (RAW_SLOT_NOT_HTML) — the REAL pipeline proof that the
    // ungated VM lane refuses too (the inline units pin the same at the
    // pub(crate) boundary).
    let src = r#"
template Card(body: Html) -> Html {
  <div>{{{ body }}}</div>
}
pattern P(x: String) -> String {
  return str(render("Card", "plain text"))
}
flow Main { input: String = "z" -> P -> output }
"#;
    let tw_err = run_tw(src)
        .err()
        .unwrap_or_else(|| panic!("TW: the raw slot must refuse a plain String"));
    assert!(
        tw_err.contains("implicit String→Html coercion"),
        "TW: the static gate must refuse the coercion; got: {tw_err}"
    );
    let vm_err = run_vm(src)
        .err()
        .unwrap_or_else(|| panic!("VM: the raw slot must refuse a plain String"));
    assert!(
        vm_err.contains("RAW_SLOT_NOT_HTML") && vm_err.contains("raw insert"),
        "VM: the runtime backstop must refuse; got: {vm_err}"
    );
}

#[test]
fn n765_html_value_into_raw_slot_runs() {
    // The honest composition: an explicit Html producer (render() — the
    // language's Html producer per the coercion-gate remedy) in the raw
    // slot runs clean on BOTH backends (the backstop is about the opaque
    // type, not about banning raw slots).
    let src = r#"
template Leaf(t: String) -> Html {
  <b>{{ t }}</b>
}
template Card(body: Html) -> Html {
  <div>{{{ body }}}</div>
}
pattern P(x: String) -> String {
  return str(render("Card", render("Leaf", "bold")))
}
flow Main { input: String = "z" -> P -> output }
"#;
    for (name, run) in [("TW", run_tw(src)), ("VM", run_vm(src))] {
        let out = run.unwrap_or_else(|e| panic!("{name}: the Html value must run: {e}"));
        assert_eq!(
            out.as_deref(),
            Some("<div><b>bold</b></div>"),
            "{name}: the raw insert substitutes verbatim"
        );
    }
}

// ── The audit's negative: the render composition stays clean ─────────

#[test]
fn n765_render_composition_negative_no_findings() {
    // render("Outer", render("Inner", render("Leaf", "x"))): the Leaf
    // render returns the opaque Html — the Inner raw slot receives Html
    // (legal), the Outer raw slot receives Inner's Html — the value
    // arguments are literals throughout (no untrusted source), the
    // static layers agree and BOTH backends run clean.
    let src = r#"
template Leaf(t: String) -> Html {
  <b>{{ t }}</b>
}
template Inner(frag: Html) -> Html {
  <em>{{{ frag }}}</em>
}
template Outer(frag: Html) -> Html {
  <section>{{{ frag }}}</section>
}
pattern P(x: String) -> String {
  return str(render("Outer", render("Inner", render("Leaf", "x"))))
}
flow Main { input: String = "z" -> P -> output }
"#;
    let audit = audit_ids(src);
    assert!(
        !audit.iter().any(|id| id == "HTML_INJECTION"),
        "the honest composition must stay silent; got: {audit:?}"
    );
    for (name, run) in [("TW", run_tw(src)), ("VM", run_vm(src))] {
        let out = run.unwrap_or_else(|e| panic!("{name}: the composition must run: {e}"));
        assert_eq!(
            out.as_deref(),
            Some("<section><em><b>x</b></em></section>"),
            "{name}: the nested raw insert composes"
        );
    }
}

// ── The interlocks hold: the escaped template stays silent ───────────

#[test]
fn n765_all_escaped_template_binding_stays_clean() {
    // The №685-D mirror on the BINDING path: an all-escaped template
    // through a let-bound render result — the escaped slots escape at
    // runtime, the binding answers the pinned clean String, no finding.
    let src = r#"
template Page(content: String) -> Html {
  <div>{{ content }}</div>
}
pattern P(x: String) -> String {
  let reply = call_llm("hello")
  let page = render("Page", reply)
  respond_html(page)
  return "ok"
}
flow Main { input: String = "z" -> P -> output }
"#;
    let audit = audit_ids(src);
    assert!(
        !audit.iter().any(|id| id == "HTML_INJECTION"),
        "the all-escaped template keeps the sanitizer semantics (the №685-D twin on the binding path); got: {audit:?}"
    );
    let check = check_ids(src);
    assert!(
        !check.iter().any(|id| id == "HTML_INJECTION"),
        "the check path agrees (the n98 twin through the let-form); got: {check:?}"
    );
}
