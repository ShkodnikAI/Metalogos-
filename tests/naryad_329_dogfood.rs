//! Наряд №329 (issue #423) — dogfooding: the Fosved Office message contour
//! under the Wave-1 gate + the ergonomics measurement (plan v2 §13.3).
//!
//! Covers:
//! - (а) the office contour (examples/l1_dogfood.mlog) compiles AND runs
//!   end-to-end under the gate — the run path enforces audit_category_a,
//!   which promotes the №325 sink gate to compile/run errors;
//! - (б) red/green pairs: the gate-required annotations are exactly what
//!   keeps the contour green — the un-sanitized LLM draft is
//!   UNTRUSTED_EGRESS_NETWORK, the raw webhook token is
//!   SECRET_EGRESS_NETWORK;
//! - (в) the full office sink list from the №316 fact (send_message,
//!   http_post, exec, git_push, write_file + the output/memory
//!   vocabulary) is gated with the expected classes;
//! - (г) the ergonomics measurement: the share of annotated lines is
//!   under 50% (the Go threshold of Фаза 1), pinned to the exact
//!   annotation inventory (2 lines: escape_html + redact/hash_only);
//! - (д) no-stub hygiene (№16.0-D).
//!
//! Boundary (loud): the office-side integration (the real Fosved Office
//! codebase) is outside this repository — the equivalent contour is the
//! deliverable per the naryad's §3 files; the Go/No-Go decision itself is
//! №330 (the owner's call) — this naryad produces the measurement only.

use std::path::Path;

fn dogfood_source() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/l1_dogfood.mlog");
    std::fs::read_to_string(p).expect("examples/l1_dogfood.mlog must exist")
}

fn compile_errors(source: &str) -> Vec<String> {
    match metalogos::compile_program(source) {
        Ok(_) => Vec::new(),
        Err(e) => e.lines().map(str::to_string).collect(),
    }
}

// ── (а) The contour runs end-to-end under the gate ───────────────────

#[test]
fn n329_office_contour_compiles_under_the_gate() {
    let src = dogfood_source();
    let errs = compile_errors(&src);
    assert!(
        errs.is_empty(),
        "the office contour must compile clean under №325/№327: {errs:?}"
    );
}

#[test]
fn n329_office_contour_runs_end_to_end() {
    // run_program enforces audit_category_a (the №325 promotion) — a
    // successful run IS a gated run, not a gate bypass.
    let src = dogfood_source();
    let out = metalogos::run_program(&src).expect("the contour must run end-to-end");
    let out = out.unwrap_or_default();
    assert!(
        out.contains("dispatched"),
        "the contour must produce the flow output: {out:?}"
    );
}

#[test]
fn n329_golden_pair_pins_the_end_to_end_run() {
    let expected = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/l1_dogfood.expected"),
    )
    .expect("the golden pair file must exist");
    let out = metalogos::run_program(&dogfood_source()).expect("the contour must run");
    assert_eq!(
        out.unwrap_or_default().trim_end(),
        expected.trim_end(),
        "the golden pair must match the actual run"
    );
}

// ── (б) Red/green: the annotations are exactly what keeps it green ───

#[test]
fn n329_unsanitized_llm_draft_is_untrusted_egress() {
    // Remove the escape_html sanitizer → the raw LLM draft (public conf,
    // UNTRUSTED integrity — №316) hits send_message → the network gate
    // refuses untrusted data.
    let src = dogfood_source().replace("escape_html(draft)", "draft");
    assert!(src.contains("= draft"), "the replacement must take effect");
    let errs = compile_errors(&src);
    assert!(
        errs.iter()
            .any(|m| m.contains("[UNTRUSTED_EGRESS_NETWORK]")),
        "unsanitized LLM output into a network sink must be UNTRUSTED_EGRESS_NETWORK: {errs:?}"
    );
}

#[test]
fn n329_raw_webhook_token_is_secret_egress() {
    // Remove the redact/hash_only downward move → the raw env Secret hits
    // http_post. In the BODY position the private label is
    // PII_EGRESS_NETWORK (the sink_check_id vocabulary); in the ADDRESS
    // position it is SECRET_EGRESS_NETWORK — both blocked.
    let body_src = dogfood_source().replace(
        "redact(env(\"FOSVED_WEBHOOK_TOKEN\"), \"hash_only\")",
        "env(\"FOSVED_WEBHOOK_TOKEN\")",
    );
    let errs = compile_errors(&body_src);
    assert!(
        errs.iter().any(|m| m.contains("[PII_EGRESS_NETWORK]")),
        "the raw webhook token in the body must be PII_EGRESS_NETWORK: {errs:?}"
    );
    let addr_src = r#"
        pattern Dispatch(task: String) -> String {
          let draft = call_llm(task)
          let safe = escape_html(draft)
          let _ = http_post(env("FOSVED_WEBHOOK_URL"), safe)
          return "dispatched"
        }
    "#;
    let errs = compile_errors(addr_src);
    assert!(
        errs.iter().any(|m| m.contains("[SECRET_EGRESS_NETWORK]")),
        "the private address into http_post must be SECRET_EGRESS_NETWORK: {errs:?}"
    );
}

#[test]
fn n329_pii_strip_is_conservative_on_the_draft() {
    // The office considered pii_strip for the draft hygiene — the gate
    // keeps the result untrusted (a strip can miss data): the label
    // passes through UNCHANGED, the sink still refuses. The contour uses
    // escape_html (trust-restoring) instead; this test pins WHY.
    let src = r#"
        pattern Dispatch(task: String) -> String {
          let draft = call_llm(task)
          let safe = redact(draft, "pii_strip")
          let _ = send_message("fosved-owner-chat", safe)
          return "dispatched"
        }
    "#;
    let errs = compile_errors(src);
    assert!(
        errs.iter()
            .any(|m| m.contains("[UNTRUSTED_EGRESS_NETWORK]")),
        "pii_strip must NOT restore trust (№326 conservatism): {errs:?}"
    );
}

// ── (в) The office sink list from the fact is gated ──────────────────

#[test]
fn n329_office_sink_list_from_the_fact_is_gated() {
    // send_message + http_post are pinned by the contour's red/green
    // pairs above; here: the rest of the №316 office sink fact list.
    let cases: &[(&str, &str, &str)] = &[
        ("exec", "exec(env(\"FOSVED_DEPLOY_KEY\"))", "SECRET_TO_EXEC"),
        (
            "exec",
            "exec(call_llm(\"next command?\"))",
            "UNTRUSTED_EXEC_DECISION",
        ),
        (
            "git_push",
            "git_push(env(\"FOSVED_DEPLOY_KEY\"))",
            "SECRET_EGRESS_VCS",
        ),
        (
            "write_file",
            "write_file(\"/tmp/status.md\", env(\"FOSVED_DEPLOY_KEY\"))",
            "SECRET_LEAK",
        ),
    ];
    for (sink, call, class) in cases {
        let src =
            format!("pattern Office(data: String) -> String {{ let _ = {call} return \"done\" }}");
        let errs = compile_errors(&src);
        assert!(
            errs.iter().any(|m| m.contains(&format!("[{class}]"))),
            "{sink} must be gated as {class}: {errs:?}"
        );
    }
}

#[test]
fn n329_output_and_memory_vocabulary_on_the_same_contour() {
    let print_src = r#"
        pattern Office(data: String) -> String {
          let _ = print(call_llm("draft the reply"))
          return "done"
        }
    "#;
    let errs = compile_errors(print_src);
    assert!(
        errs.iter().any(|m| m.contains("[HTML_INJECTION]")),
        "untrusted LLM output into stdout must be HTML_INJECTION: {errs:?}"
    );
    let mem_src = r#"
        pattern Office(data: String) -> String {
          let _ = memorize("last_draft", call_llm(data))
          return "done"
        }
    "#;
    let errs = compile_errors(mem_src);
    assert!(
        errs.iter().any(|m| m.contains("[TAINT_PERSISTENCE]")),
        "untrusted LLM output into persistent memory must be TAINT_PERSISTENCE: {errs:?}"
    );
}

// ── (г) The ergonomics measurement (the naryad's task 2) ─────────────

/// A line counts as an annotation when it carries explicit security
/// ceremony: a redact policy (№326), a trust-restoring sanitizer call
/// (the gate-required escape), an explicit label annotation
/// (ADR-0154), an effect trail (№324), or a compatibility profile
/// (ADR-0161). Comments and blanks are excluded on both sides.
fn measure_annotation_share(source: &str) -> (usize, usize, Vec<String>) {
    const MARKERS: &[&str] = &[
        "redact(",
        "escape_html(",
        "render(",
        "String<",
        "⟨",
        "profile ",
    ];
    let mut code_lines = 0usize;
    let mut annotated = Vec::new();
    for line in source.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        code_lines += 1;
        if MARKERS.iter().any(|m| line.contains(m)) {
            annotated.push(t.to_string());
        }
    }
    (code_lines, annotated.len(), annotated)
}

#[test]
fn n329_annotation_share_is_under_the_go_threshold() {
    let src = dogfood_source();
    let (total, annotated, lines) = measure_annotation_share(&src);
    assert_eq!(
        annotated, 2,
        "the minimal annotation inventory is pinned: {lines:?}"
    );
    assert!(total >= 10, "the contour must not shrink below a real body");
    let share = (annotated as f64) / (total as f64) * 100.0;
    assert!(
        share < 50.0,
        "the Go threshold of Фаза 1 is < 50% annotated lines; measured {share:.1}% ({annotated}/{total})"
    );
    eprintln!(
        "n329 ergonomics measurement: {annotated}/{total} annotated lines = {share:.1}% (threshold < 50%)"
    );
    for line in &lines {
        eprintln!("  annotation: {line}");
    }
}

#[test]
fn n329_annotation_inventory_by_place() {
    // The full list of annotations the contour required (the naryad's
    // task 2, "по местам") — pinned line-level:
    let src = dogfood_source();
    let (_, _, lines) = measure_annotation_share(&src);
    assert!(
        lines.iter().any(|l| l.contains("escape_html(draft)")),
        "annotation 1: the trust-restoring sanitizer on the LLM draft: {lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("redact(env(\"FOSVED_WEBHOOK_TOKEN\"), \"hash_only\")")),
        "annotation 2: the one-way redact on the webhook secret: {lines:?}"
    );
    assert_eq!(lines.len(), 2, "no other ceremony is required: {lines:?}");
}

#[test]
fn n329_plain_office_contour_stays_annotation_free() {
    // The zero-delta contract on the office shape: without sources, the
    // contour needs NO ceremony at all.
    let src = r#"
        pattern Greet(name: String) -> String {
          let _ = print("Здравствуйте, " + name)
          return "ok"
        }
        flow Main { input: String = "Иван" -> Greet -> output }
    "#;
    let errs = compile_errors(src);
    assert!(errs.is_empty(), "bottom labels clear every sink: {errs:?}");
}

// ── (д) No-stub hygiene (№16.0-D) ────────────────────────────────────

#[test]
fn n329_no_stub_markers_in_touched_files() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    for file in [
        "examples/l1_dogfood.mlog",
        "examples/l1_dogfood.expected",
        "tests/naryad_329_dogfood.rs",
    ] {
        let src = std::fs::read_to_string(format!("{manifest_dir}/{file}"))
            .unwrap_or_else(|_| panic!("{file} must exist"));
        for marker in [
            concat!("todo", "!"),
            concat!("unimplemented", "!"),
            concat!("SKELE", "TON"),
        ] {
            assert!(
                !src.contains(marker),
                "stub marker `{marker}` found in {file}"
            );
        }
    }
}
