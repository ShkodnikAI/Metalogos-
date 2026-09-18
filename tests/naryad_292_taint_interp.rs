// ── tests/naryad_292_taint_interp.rs ────────────────────────────────
// Наряд №292 (issue #355), TAINT_INTERP — interprocedural taint MVP.
//
// Summary-based analysis catches non-trivial passthrough chains that
// the trivial TAINT_PASSTHROUGH (Наряд #141/#157) misses.
//
// Contract tests (issue #355):
//   (а) `wrap(x) { return upper(x) }` + `respond(Wrap(call_llm(...)))` →
//       TAINT_INTERP finding (today TAINT_PASSTHROUGH misses this — non-trivial body).
//   (б) Chain of 2 user-pattern calls: `respond(Outer(Inner(call_llm(...))))` →
//       TAINT_INTERP finding.
//   (в) Legitimate path through `render(...)` / `escape_html(...)` → NOT flagged
//       (zero false positives on legitimate code).
//   (г) Recursive pattern → analysis terminates with INTERP_DEPTH_LIMIT
//       warning (not error, not hang).

use metalogos::audit_program;

fn has_finding(source: &str, check_id: &str) -> bool {
    match audit_program(source) {
        Ok(result) => result.findings.iter().any(|f| f.check_id == check_id),
        Err(_) => false,
    }
}

fn has_no_findings_with_id(source: &str, check_id: &str) -> bool {
    !has_finding(source, check_id)
}

// ── Contract (а): non-trivial wrap → TAINT_INTERP ─────────────────────

#[test]
fn c_a_nontrivial_wrap_llm_to_respond_is_taint_interp() {
    // `pattern Wrap(x) { return upper(x) }` — non-trivial body (wraps x in upper()).
    // `respond(Wrap(call_llm(...)))` — LLM output flows through Wrap to respond.
    // TAINT_PASSTHROUGH (trivial 1-param passthrough) does NOT catch this;
    // TAINT_INTERP (summary-based) DOES — Wrap's summary says param 0 taints
    // return.
    let source = r#"
pattern Wrap(x: String) -> String {
    return upper(x)
}

pattern Handler() -> String {
    let r = respond("200 OK", Wrap(call_llm("Tell me a joke")))
    return r
}
"#;
    assert!(
        has_finding(source, "TAINT_INTERP"),
        "respond(Wrap(call_llm(...))) must trigger TAINT_INTERP (non-trivial wrap)"
    );
}

// ── Contract (б): 2-level chain → TAINT_INTERP ───────────────────────

#[test]
fn c_b_two_level_chain_is_taint_interp() {
    // Outer wraps Inner's output. Inner's summary says param 0 taints return.
    // Outer's summary says: param 0 taints return (via Inner call).
    // respond(Outer(call_llm(...))) — LLM flows through Inner+Outer to respond.
    let source = r#"
pattern Inner(x: String) -> String {
    return x
}

pattern Outer(x: String) -> String {
    return Inner(x)
}

pattern Handler() -> String {
    let r = respond("200 OK", Outer(call_llm("Tell me a joke")))
    return r
}
"#;
    assert!(
        has_finding(source, "TAINT_INTERP"),
        "respond(Outer(Inner(call_llm(...)))) must trigger TAINT_INTERP (2-level chain)"
    );
}

// ── Contract (в): legitimate render/escape_html path → NOT flagged ───

#[test]
fn c_v_render_wraps_llm_is_not_flagged() {
    // render(...) is a sanitizer — taint is lifted. respond(render(...)) is safe.
    let source = r#"
template Safe(out: String) { <div>{{out}}</div> }

pattern Handler() -> String {
    let r = respond("200 OK", render("Safe", call_llm("Tell me a joke")))
    return r
}
"#;
    assert!(
        has_no_findings_with_id(source, "TAINT_INTERP"),
        "respond(render(...)) must NOT trigger TAINT_INTERP — render is a sanitizer"
    );
    assert!(
        has_no_findings_with_id(source, "TAINT_PASSTHROUGH"),
        "respond(render(...)) must NOT trigger TAINT_PASSTHROUGH either"
    );
}

#[test]
fn c_v_escape_html_wraps_llm_is_not_flagged() {
    // escape_html(...) is a sanitizer — taint is lifted.
    let source = r#"
pattern Handler() -> String {
    let r = respond("200 OK", escape_html(call_llm("Tell me a joke")))
    return r
}
"#;
    assert!(
        has_no_findings_with_id(source, "TAINT_INTERP"),
        "respond(escape_html(...)) must NOT trigger TAINT_INTERP — escape_html is a sanitizer"
    );
}

#[test]
fn c_v_render_wraps_pattern_call_llm_is_not_flagged() {
    // Wrap wraps LLM, but Wrap's output is rendered through render(...) — the
    // sanitizer wraps the entire pattern call, lifting the taint. No finding.
    let source = r#"
pattern Wrap(x: String) -> String {
    return upper(x)
}

template Safe(out: String) { <div>{{out}}</div> }

pattern Handler() -> String {
    let r = respond("200 OK", render("Safe", Wrap(call_llm("Tell me a joke"))))
    return r
}
"#;
    assert!(
        has_no_findings_with_id(source, "TAINT_INTERP"),
        "respond(render(\"Safe\", Wrap(call_llm(...)))) must NOT trigger TAINT_INTERP — render wraps the pattern call"
    );
}

// ── Contract (г): recursive pattern → INTERP_DEPTH_LIMIT warning ────

#[test]
fn c_g_recursive_pattern_emits_interp_depth_limit_warning() {
    // Self-referential pattern — the call graph has a cycle (Recurse calls
    // itself). Analysis terminates with INTERP_DEPTH_LIMIT warning — not
    // error, not hang.
    let source = r#"
pattern Recurse(x: String) -> String {
    return Recurse(x)
}

pattern Handler() -> String {
    let r = respond("200 OK", Recurse(call_llm("Tell me a joke")))
    return r
}
"#;
    assert!(
        has_finding(source, "INTERP_DEPTH_LIMIT"),
        "recursive pattern must emit INTERP_DEPTH_LIMIT warning (analysis boundary documented)"
    );
}

// ── Additional: trivial passthrough (return param) → still caught by TAINT_PASSTHROUGH ──

#[test]
fn trivial_passthrough_still_caught_by_taint_passthrough() {
    // The existing trivial-passthrough check (Наряд #141/#157) still fires.
    // TAINT_INTERP does NOT need to duplicate this — it focuses on non-trivial.
    let source = r#"
pattern Wrap(x: String) -> String {
    return x
}

pattern Handler() -> String {
    let r = respond("200 OK", Wrap(call_llm("Tell me a joke")))
    return r
}
"#;
    assert!(
        has_finding(source, "TAINT_PASSTHROUGH"),
        "trivial 1-param passthrough (return x) must still trigger TAINT_PASSTHROUGH"
    );
}

// ── Additional: pattern that does NOT return its param → no taint flow ──

#[test]
fn pattern_not_returning_param_no_taint_flow() {
    // `pattern Const(x: String) -> String { return "constant" }` — param x
    // does NOT flow into return. So respond(Const(call_llm(...))) is safe —
    // the LLM output is discarded inside Const.
    let source = r#"
pattern Const(x: String) -> String {
    return "constant"
}

pattern Handler() -> String {
    let r = respond("200 OK", Const(call_llm("Tell me a joke")))
    return r
}
"#;
    assert!(
        has_no_findings_with_id(source, "TAINT_INTERP"),
        "respond(Const(call_llm(...))) must NOT trigger TAINT_INTERP — Const discards the LLM arg"
    );
}

// ── Additional: respond_html with non-trivial wrap → TAINT_INTERP ────

#[test]
fn respond_html_nontrivial_wrap_llm_is_taint_interp() {
    let source = r#"
pattern Wrap(x: String) -> String {
    return upper(x)
}

pattern Handler() -> String {
    let r = respond_html(Wrap(call_llm("Tell me a joke")))
    return r
}
"#;
    assert!(
        has_finding(source, "TAINT_INTERP"),
        "respond_html(Wrap(call_llm(...))) must trigger TAINT_INTERP (respond_html is also a sink)"
    );
}

// ── Additional: write_file sink with non-trivial wrap → TAINT_INTERP ─

#[test]
fn write_file_nontrivial_wrap_llm_is_taint_interp() {
    let source = r#"
pattern Wrap(x: String) -> String {
    return upper(x)
}

pattern Handler() -> String {
    let _ = write_file("out.txt", Wrap(call_llm("Tell me a joke")))
    return "ok"
}
"#;
    assert!(
        has_finding(source, "TAINT_INTERP"),
        "write_file(Wrap(call_llm(...))) must trigger TAINT_INTERP (write_file is a sink)"
    );
}
