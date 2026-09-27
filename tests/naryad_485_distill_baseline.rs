//! Naryad #485 (gh#733) — the distill gate hardening: the majority
//! baseline + margin, the stratified split, the NaN-safe refusal form.
//!
//! Grammar pins (the naryad_456 pattern):
//!   * the grammar accepts `distill_margin: <float>` as a distill field;
//!   * the field is optional (None → the 0.05 runtime default);
//!   * the value rides the AST (LearnablePatternDecl.distill_margin).
//!
//! The behavioral gates (the degenerate-majority reproduction and the
//! separable positive control, TW + VM) are unit-tested in
//! `src/interpreter/learnable.rs` and `src/vm.rs` — under MockLlm the
//! recorded labels are stubs, so the switch is unreachable through
//! programs; the unit seam is the honest way.

use metalogos::ast::{Declaration, LearnablePatternDecl};

fn parse_learnable(source: &str) -> LearnablePatternDecl {
    let declarations = metalogos::parser::parse(source).expect("must parse");
    match &declarations[0] {
        Declaration::LearnablePattern(lp) => lp.clone(),
        other => panic!("expected a learnable pattern, got {:?}", other),
    }
}

#[test]
fn grammar_accepts_distill_margin() {
    let lp = parse_learnable(
        r#"
learnable pattern Decide(question: String) -> String {
  prompt: "route the intent"
  distill_to: DecideHead
  distill_min_accuracy: 0.9
  distill_margin: 0.08
}
"#,
    );
    assert_eq!(
        lp.distill_margin,
        Some(0.08),
        "the margin must land in the AST"
    );
    assert_eq!(lp.distill_min_accuracy, Some(0.9));
}

#[test]
fn grammar_distill_margin_is_optional() {
    let lp = parse_learnable(
        r#"
learnable pattern Decide(question: String) -> String {
  prompt: "route the intent"
  distill_to: DecideHead
}
"#,
    );
    assert!(lp.distill_margin.is_none(), "no margin line → None");
}

#[test]
fn grammar_distill_margin_composes_with_the_other_fields() {
    let lp = parse_learnable(
        r#"
learnable pattern Decide(question: String) -> String {
  prompt: "route the intent"
  distill_to: DecideHead
  distill_after: 5
  distill_margin: 0.12
  fallback_if: confidence < 0.7
}
"#,
    );
    assert_eq!(lp.distill_margin, Some(0.12));
    assert_eq!(lp.distill_after, 5);
    assert!(lp.fallback_if.is_some());
}
