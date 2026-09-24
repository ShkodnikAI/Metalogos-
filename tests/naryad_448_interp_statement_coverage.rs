// ── tests/naryad_448_interp_statement_coverage.rs ───────────────────
// Наряд №448 (Волна 15, issue #656): TAINT_INTERP — full statement-kind
// coverage (REALITY §2.2 was "9 of 15").
//
// Before the naryad the interprocedural engine's walkers fell into the
// `_ => {}` arm for `Match`, `Break`, `Continue` and the Memory variants
// (`Memorize`/`Forget`/`Relate`): sinks inside a match arm, taint carried
// out of a loop through break/continue, and memory persist-facts were
// invisible to the interp contour.
//
// Contract groups pinned here:
//   (а) Match: sinks/chains inside arm bodies are scanned; binding
//       states are merged across arms through the label lattice join
//       (ADR-0154 §2, src/labels.rs — no new lattice); `return` inside
//       an arm marks params in the pattern summary (may-union).
//   (б) Break/Continue: the state at the break/continue point is merged
//       into the loop-exit state (conservative may-analysis); statements
//       after a break/continue are dead and cannot clean the state.
//   (в) Memory variants: statement-form `memorize` and the keyed call
//       form contribute persist-facts to the state; `recall()` of a
//       tainted key (or after a key-less LLM write) is untrusted;
//       TAINT_PERSISTENCE contracts are untouched (parity, double
//       reporting is a signal, not a duplicate).
//   (г) Zero false positives: sanitized and clean flows through the NEW
//       kinds stay clean.

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

const WRAP: &str = "pattern Wrap(x: String) -> String {\n  return upper(x)\n}\n";

// ── (а) Match ────────────────────────────────────────────────────────

#[test]
fn a1_sink_chain_inside_match_arm_is_taint_interp() {
    // The whole interprocedural chain lives inside one arm body. The same
    // chain outside the match was caught before the naryad; inside the arm
    // it compiled clean (the flat collector skipped Match).
    let source = format!(
        "{WRAP}pattern Send(data: String) -> String {{\n  match data {{\n    \"a\" then {{\n      let _ = write_file(\"/tmp/report448.txt\", Wrap(call_llm(\"summarize the document\")))\n    }}\n    else {{\n      let _ = write_file(\"/tmp/report448.txt\", \"nothing to report\")\n    }}\n  }}\n  return \"sent\"\n}}\nflow Main {{ input: String = \"a\" -> Send -> output }}\n"
    );
    assert!(
        has_finding(&source, "TAINT_INTERP"),
        "write_file(Wrap(call_llm(...))) inside a match arm must trigger TAINT_INTERP"
    );
}

#[test]
fn a2_binding_tainted_in_one_arm_merges_after_match() {
    // The binding is tainted in ONE arm (assign), the sink sits after the
    // match — the post-match state is the lattice JOIN of the arm states.
    let source = format!(
        "{WRAP}pattern Send(data: String) -> String {{\n  let mut v = \"clean\"\n  match data {{\n    \"a\" then {{\n      v = Wrap(call_llm(\"summarize the document\"))\n    }}\n    else {{\n      v = \"safe\"\n    }}\n  }}\n  let _ = respond(\"200\", v)\n  return \"sent\"\n}}\nflow Main {{ input: String = \"a\" -> Send -> output }}\n"
    );
    assert!(
        has_finding(&source, "TAINT_INTERP"),
        "taint entering one match arm must survive the arm-state join and fire TAINT_INTERP at the sink"
    );
}

#[test]
fn a3_let_binding_in_arm_merges_after_match() {
    // Same join contract, let-binding form inside the arm.
    let source = format!(
        "{WRAP}pattern Send(data: String) -> String {{\n  let mut v = \"clean\"\n  match data {{\n    \"a\" then {{\n      let w = Wrap(call_llm(\"summarize the document\"))\n      v = w\n    }}\n    else {{\n      v = \"safe\"\n    }}\n  }}\n  let _ = respond(\"200\", v)\n  return \"sent\"\n}}\nflow Main {{ input: String = \"a\" -> Send -> output }}\n"
    );
    assert!(
        has_finding(&source, "TAINT_INTERP"),
        "let-bound taint inside one arm must survive the arm-state join"
    );
}

#[test]
fn a4_return_inside_match_arm_marks_summary_params() {
    // `return` inside an arm: the pattern summary must mark the param as
    // tainting the return (may-union over arms) — the interprocedural
    // sink check then fires at the call site.
    let source = "pattern MatchWrap(x: String) -> String {\n  match x {\n    \"a\" then {\n      return upper(x)\n    }\n    else {\n      return x\n    }\n  }\n}\npattern Send(data: String) -> String {\n  let _ = respond(\"200\", MatchWrap(call_llm(\"summarize the document\")))\n  return \"sent\"\n}\nflow Main { input: String = \"a\" -> Send -> output }\n";
    assert!(
        has_finding(source, "TAINT_INTERP"),
        "a return inside a match arm must mark summary params — respond(MatchWrap(call_llm(...))) fires TAINT_INTERP"
    );
}

#[test]
fn a5_sanitized_arm_flow_is_not_flagged() {
    // Zero false positives: render() inside the arm lifts the taint.
    let source = format!(
        "{WRAP}pattern Send(data: String) -> String {{\n  let mut v = \"clean\"\n  match data {{\n    \"a\" then {{\n      v = render(call_llm(\"summarize the document\"))\n    }}\n    else {{\n      v = \"safe\"\n    }}\n  }}\n  let _ = respond(\"200\", v)\n  return \"sent\"\n}}\nflow Main {{ input: String = \"a\" -> Send -> output }}\n"
    );
    assert!(
        has_no_findings_with_id(&source, "TAINT_INTERP"),
        "render() in the arm must lift the taint through the join — no TAINT_INTERP"
    );
}

#[test]
fn a6_clean_match_flow_is_not_flagged() {
    let source = "pattern Send(data: String) -> String {\n  match data {\n    \"a\" then {\n      let _ = respond(\"200\", \"alpha\")\n    }\n    else {\n      let _ = respond(\"200\", \"beta\")\n    }\n  }\n  return \"sent\"\n}\nflow Main { input: String = \"a\" -> Send -> output }\n";
    assert!(
        has_no_findings_with_id(source, "TAINT_INTERP"),
        "literal-only match arms must stay clean"
    );
}

// ── (б) Break / Continue ─────────────────────────────────────────────

#[test]
fn b1_break_carries_taint_out_of_each_loop() {
    // The binding is tainted inside the body, break exits, the sink sits
    // after the loop — the break-point state must join the loop-exit state.
    let source = format!(
        "{WRAP}entity items: List = [1.0, 2.0]\npattern Send(data: String) -> String {{\n  let mut carried = \"clean\"\n  each i in items {{\n    let w = Wrap(call_llm(\"summarize the document\"))\n    carried = w\n    break\n  }}\n  let _ = respond(\"200\", carried)\n  return \"sent\"\n}}\nflow Main {{ input: String = \"x\" -> Send -> output }}\n"
    );
    assert!(
        has_finding(&source, "TAINT_INTERP"),
        "taint carried out of a loop through break must fire TAINT_INTERP at the after-loop sink"
    );
}

#[test]
fn b2_break_point_state_survives_dead_code() {
    // Statements after break are dead: their (clean) assignments must NOT
    // overwrite the break-point state collected into the loop-exit merge.
    let source = format!(
        "{WRAP}entity items: List = [1.0, 2.0]\npattern Send(data: String) -> String {{\n  let mut carried = \"clean\"\n  each i in items {{\n    carried = Wrap(call_llm(\"summarize the document\"))\n    break\n    carried = \"safe\"\n  }}\n  let _ = respond(\"200\", carried)\n  return \"sent\"\n}}\nflow Main {{ input: String = \"x\" -> Send -> output }}\n"
    );
    assert!(
        has_finding(&source, "TAINT_INTERP"),
        "the state at the break point must be merged into the loop exit even when dead code follows the break"
    );
}

#[test]
fn b3_continue_point_state_survives_dead_code() {
    // Symmetric contract for continue: the continue-point state joins the
    // loop-continuation merge; dead code after continue cannot clean it.
    let source = format!(
        "{WRAP}pattern Send(data: String) -> String {{\n  let mut carried = \"clean\"\n  while data == \"go\" {{\n    carried = Wrap(call_llm(\"summarize the document\"))\n    continue\n    carried = \"safe\"\n  }}\n  let _ = respond(\"200\", carried)\n  return \"sent\"\n}}\nflow Main {{ input: String = \"go\" -> Send -> output }}\n"
    );
    assert!(
        has_finding(&source, "TAINT_INTERP"),
        "the state at the continue point must be merged even when dead code follows the continue"
    );
}

#[test]
fn b4_natural_continue_carry_is_taint_interp() {
    // Natural shape: assignment then continue in a while body, sink after.
    let source = format!(
        "{WRAP}pattern Send(data: String) -> String {{\n  let mut carried = \"clean\"\n  while data == \"go\" {{\n    let w = Wrap(call_llm(\"summarize the document\"))\n    carried = w\n    continue\n  }}\n  let _ = respond(\"200\", carried)\n  return \"sent\"\n}}\nflow Main {{ input: String = \"go\" -> Send -> output }}\n"
    );
    assert!(
        has_finding(&source, "TAINT_INTERP"),
        "taint accumulated in a while body with continue must reach the after-loop sink check"
    );
}

#[test]
fn b5_clean_loop_carry_is_not_flagged() {
    // The loop body renders the LLM output before storing; break exits
    // with a sanitized binding — the loop-exit merge must stay clean.
    let source = "entity items: List = [1.0, 2.0]\npattern Send(data: String) -> String {\n  let mut carried = \"clean\"\n  each i in items {\n    let raw = call_llm(\"summarize the document\")\n    let shown = render(raw)\n    carried = shown\n    if i > 1.0 then {\n      break\n    }\n  }\n  let _ = respond(\"200\", carried)\n  return \"sent\"\n}\nflow Main { input: String = \"x\" -> Send -> output }\n";
    assert!(
        has_no_findings_with_id(source, "TAINT_INTERP"),
        "sanitized loop-carry must stay clean through the loop-exit merge"
    );
}

// ── (в) Memory variants in the interp contour ────────────────────────

#[test]
fn c1_statement_form_memorize_then_recall_in_arm() {
    // The key-less statement form `memorize <llm> with priority` arms the
    // persist-fact; recall in a (match-arm) branch is then may-tainted.
    let source = "pattern Send(data: String) -> String {\n  memorize call_llm(\"draft a reply\") with priority=0.9\n  match data {\n    \"send\" then {\n      let _ = respond(\"200\", recall(\"draft-reply\"))\n    }\n    else {\n      let _ = respond(\"200\", \"nothing stored\")\n    }\n  }\n  return \"sent\"\n}\nflow Main { input: String = \"send\" -> Send -> output }\n";
    assert!(
        has_finding(source, "TAINT_INTERP"),
        "statement-form memorize(LLM) must arm the persist-fact — recall→respond inside a match arm fires TAINT_INTERP"
    );
}

#[test]
fn c2_keyed_memorize_in_branch_then_recall_in_other_branch() {
    // Keyed call-form write inside an if-branch; the recall sits in a
    // match arm. The persist-fact must survive the branch joins.
    let source = "pattern Send(data: String) -> String {\n  if data == \"w\" then {\n    let _ = memorize(\"k448b\", call_llm(\"draft a reply\"))\n  }\n  match data {\n    \"send\" then {\n      let _ = respond(\"200\", recall(\"k448b\"))\n    }\n    else {\n      let _ = respond(\"200\", \"nothing stored\")\n    }\n  }\n  return \"sent\"\n}\nflow Main { input: String = \"send\" -> Send -> output }\n";
    assert!(
        has_finding(source, "TAINT_INTERP"),
        "keyed memorize(LLM) inside a branch must arm the key — recall→respond in a match arm fires TAINT_INTERP"
    );
}

#[test]
fn c3_keyed_clean_memorize_then_recall_is_not_flagged() {
    // Zero false positives: the stored VALUE is a literal — no persist-fact.
    let source = "pattern Send(data: String) -> String {\n  let _ = memorize(\"k448clean\", \"plain stored note\")\n  match data {\n    \"send\" then {\n      let got = recall(\"k448clean\")\n      let _ = respond(\"200\", got)\n    }\n    else {\n      let _ = respond(\"200\", \"nothing stored\")\n    }\n  }\n  return \"sent\"\n}\nflow Main { input: String = \"send\" -> Send -> output }\n";
    assert!(
        has_no_findings_with_id(source, "TAINT_INTERP"),
        "memorize of a clean literal must not arm the persist-fact"
    );
}

#[test]
fn c4_sanitized_recall_is_not_flagged() {
    // recall of a tainted key wrapped in render() before the sink — clean.
    let source = "pattern Send(data: String) -> String {\n  memorize call_llm(\"draft a reply\") with priority=0.9\n  match data {\n    \"send\" then {\n      let _ = respond(\"200\", render(recall(\"draft-reply\")))\n    }\n    else {\n      let _ = respond(\"200\", \"nothing stored\")\n    }\n  }\n  return \"sent\"\n}\nflow Main { input: String = \"send\" -> Send -> output }\n";
    assert!(
        has_no_findings_with_id(source, "TAINT_INTERP"),
        "render() over the recall must lift the memory taint — no TAINT_INTERP"
    );
}

#[test]
fn c5_relate_expression_is_scanned_for_sinks() {
    // Relate participates: its expressions are scanned — a sink hidden in
    // the relation's from-expression is caught.
    let source = format!(
        "{WRAP}pattern Send(data: String) -> String {{\n  relate write_file(\"/tmp/relate448.txt\", Wrap(call_llm(\"summarize the document\"))) to \"report\"\n  return \"sent\"\n}}\nflow Main {{ input: String = \"x\" -> Send -> output }}\n"
    );
    assert!(
        has_finding(&source, "TAINT_INTERP"),
        "a sink inside a relate expression must be scanned by the interp contour"
    );
}

// ── (г) Regression guards ────────────────────────────────────────────

#[test]
fn d1_straight_line_chain_still_caught() {
    // №292 contract (а) re-pinned: the refactor to the state walker must
    // not lose the original interprocedural catch.
    let source = format!(
        "{WRAP}pattern Send(data: String) -> String {{\n  let _ = respond(\"200\", Wrap(call_llm(\"summarize the document\")))\n  return \"sent\"\n}}\nflow Main {{ input: String = \"x\" -> Send -> output }}\n"
    );
    assert!(has_finding(&source, "TAINT_INTERP"));
}

#[test]
fn d2_sanitizer_still_lifts_straight_line() {
    // №292 contract (в) re-pinned.
    let source = format!(
        "{WRAP}pattern Send(data: String) -> String {{\n  let _ = respond(\"200\", render(call_llm(\"summarize the document\")))\n  return \"sent\"\n}}\nflow Main {{ input: String = \"x\" -> Send -> output }}\n"
    );
    assert!(has_no_findings_with_id(&source, "TAINT_INTERP"));
}

#[test]
fn d3_literal_respond_still_clean() {
    let source = "pattern Send(data: String) -> String {\n  let _ = respond(\"200\", \"all good\")\n  return \"sent\"\n}\nflow Main { input: String = \"x\" -> Send -> output }\n";
    assert!(has_no_findings_with_id(source, "TAINT_INTERP"));
}

#[test]
fn b6_taint_and_break_inside_if_branch_inside_loop() {
    // The taint and the break both live inside an if-branch: the branch is
    // terminated, so its end-state reaches the loop exit ONLY through the
    // break/continue merge — this pins the exit-collection machinery
    // itself (the M2 mutation contract).
    let source = format!(
        "{WRAP}entity items: List = [1.0, 2.0]\npattern Send(data: String) -> String {{\n  let mut carried = \"clean\"\n  each i in items {{\n    if i > 0.0 then {{\n      carried = Wrap(call_llm(\"summarize the document\"))\n      break\n    }}\n  }}\n  let _ = respond(\"200\", carried)\n  return \"sent\"\n}}\nflow Main {{ input: String = \"x\" -> Send -> output }}\n"
    );
    assert!(
        has_finding(&source, "TAINT_INTERP"),
        "taint entering a binding right before a break inside an if-branch must reach the loop-exit merge and fire TAINT_INTERP"
    );
}
