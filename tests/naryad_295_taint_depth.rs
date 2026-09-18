// ── tests/naryad_295_taint_depth.rs ─────────────────────────────────
// Наряд №295 (issue #359), TAINT_DEPTH — bounded nesting depth.
//
// `expr_is_llm_tainted` was single-level (caught `respond(call_llm(...))`
// but NOT `respond(upper(call_llm(...)))`). Наряд №295 makes it bounded-
// recursive up to `TAINT_NESTING_MAX_DEPTH = 3`.
//
// Contract tests (issue #359):
//   (а) Depth 2 — `respond(upper(call_llm(...)))` → HTML_INJECTION.
//   (б) Depth 3 — `respond(upper(upper(call_llm(...))))` → HTML_INJECTION.
//   (в) Legitimate render/escape_html path at any depth → NOT flagged.
//   (г) Depth 4 — exceeds TAINT_NESTING_MAX_DEPTH=3; not flagged
//       intraprocedurally (TAINT_INTERP catches if pattern call involved).

use metalogos::audit_program;

fn has_finding(source: &str, check_id: &str) -> bool {
    match audit_program(source) {
        Ok(result) => result.findings.iter().any(|f| f.check_id == check_id),
        Err(_) => false,
    }
}

fn findings_with(source: &str, check_id: &str) -> Vec<metalogos::audit::AuditFinding> {
    metalogos::audit_program(source)
        .expect("audit")
        .findings
        .into_iter()
        .filter(|f| f.check_id == check_id)
        .collect()
}

fn has_no_finding(source: &str, check_id: &str) -> bool {
    !has_finding(source, check_id)
}

// ── Contract (а): depth 2 → HTML_INJECTION ──────────────────────────

#[test]
fn c_a_depth_2_caught() {
    // `respond(upper(call_llm(...)))` — depth 2 (upper → call_llm).
    // Single-level check missed this; bounded-recursive catches.
    let source = r#"
pattern Handler() -> String {
    let r = respond("200 OK", upper(call_llm("Tell me a joke")))
    return r
}
"#;
    assert!(
        has_finding(source, "HTML_INJECTION"),
        "respond(upper(call_llm(...))) must trigger HTML_INJECTION (depth 2)"
    );
}

// ── Contract (б): depth 3 → HTML_INJECTION ──────────────────────────

#[test]
fn c_b_depth_3_caught() {
    // `respond(upper(upper(call_llm(...))))` — depth 3.
    let source = r#"
pattern Handler() -> String {
    let r = respond("200 OK", upper(upper(call_llm("Tell me a joke"))))
    return r
}
"#;
    assert!(
        has_finding(source, "HTML_INJECTION"),
        "respond(upper(upper(call_llm(...)))) must trigger HTML_INJECTION (depth 3)"
    );
}

// ── Contract (в): legitimate render/escape_html → NOT flagged ───────

#[test]
fn c_v_render_at_any_depth_not_flagged() {
    // render(...) at depth 2 lifts the taint.
    let source = r#"
template Safe(out: String) { <div>{{out}}</div> }

pattern Handler() -> String {
    let r = respond("200 OK", upper(render("Safe", call_llm("Tell me a joke"))))
    return r
}
"#;
    assert!(
        has_no_finding(source, "HTML_INJECTION"),
        "respond(upper(render(...))) must NOT trigger HTML_INJECTION — render lifts taint at any depth"
    );
}

#[test]
fn c_v_escape_html_at_depth_2_not_flagged() {
    let source = r#"
pattern Handler() -> String {
    let r = respond("200 OK", upper(escape_html(call_llm("Tell me a joke"))))
    return r
}
"#;
    assert!(
        has_no_finding(source, "HTML_INJECTION"),
        "respond(upper(escape_html(...))) must NOT trigger HTML_INJECTION — escape_html lifts taint"
    );
}

// ── Contract (г): depth 5 — exceeds MAX_DEPTH=3, not flagged ────────

#[test]
fn c_g_depth_5_not_flagged_intraprocedurally() {
    // `respond(upper(upper(upper(upper(call_llm(...))))))` — call_llm at depth 4.
    // TAINT_NESTING_MAX_DEPTH=3 → at depth 4, check returns false before
    // reaching the call_llm source. Note: depth 4 (call_llm at depth 3)
    // IS caught — the boundary is at depth 4 (call_llm at depth 4).
    let source = r#"
pattern Handler() -> String {
    let r = respond("200 OK", upper(upper(upper(upper(call_llm("Tell me a joke"))))))
    return r
}
"#;
    // №325 note: the legacy interprocedural check stays silent at this
    // depth (the documented boundary); the lattice gate flags the same
    // site under the shared HTML_INJECTION id, so the legacy finding is
    // identified by its own message text.
    assert!(
        !findings_with(source, "HTML_INJECTION")
            .iter()
            .any(|f| f.message.contains("LLM output passed to respond()")),
        "depth 5 nesting (call_llm at depth 4) exceeds TAINT_NESTING_MAX_DEPTH=3 — the LEGACY check must stay silent (documented boundary); the №325 gate may still flag it"
    );
}

// ── Additional: depth 1 still works (no regression) ─────────────────

#[test]
fn depth_1_still_caught() {
    let source = r#"
pattern Handler() -> String {
    let r = respond("200 OK", call_llm("Tell me a joke"))
    return r
}
"#;
    assert!(
        has_finding(source, "HTML_INJECTION"),
        "depth 1 (direct call_llm) must still trigger HTML_INJECTION (no regression)"
    );
}

// ── Additional: BinaryOp with LLM-tainted operand ───────────────────

#[test]
fn binop_with_llm_tainted_operand() {
    // `respond("x" + call_llm(...))` — BinaryOp left=literal, right=LLM source.
    // Bounded-recursive check catches via BinaryOp arm.
    let source = r#"
pattern Handler() -> String {
    let r = respond("200 OK", "prefix: " + call_llm("Tell me a joke"))
    return r
}
"#;
    assert!(
        has_finding(source, "HTML_INJECTION"),
        "respond(\"prefix: \" + call_llm(...)) must trigger HTML_INJECTION (BinaryOp with LLM operand)"
    );
}

// ── Additional: reflex_generate IS an LLM-equivalent source (Наряд №201) ─

#[test]
fn reflex_generate_at_depth_2_caught() {
    // reflex_generate is an LLM-output-equivalent source (Наряд №201) —
    // `is_llm_source` returns true for "reflex_generate" regardless of
    // feature gates (the audit check is static, not runtime-gated).
    let source = r#"
pattern Handler() -> String {
    let r = respond("200 OK", upper(reflex_generate("StoryModel", "prompt")))
    return r
}
"#;
    assert!(
        has_finding(source, "HTML_INJECTION"),
        "respond(upper(reflex_generate(...))) must trigger HTML_INJECTION — reflex_generate is an LLM-equivalent source (Наряд №201, is_llm_source)"
    );
}
