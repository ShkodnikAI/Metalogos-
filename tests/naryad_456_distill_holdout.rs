//! №456 (gh#675, P1 reflex; audit 25.09, finding 3.3) — the distillation
//! switch is GATED: holdout validation before the TEACHING→DISTILLED
//! switch, fail-closed on non-finite confidence, and a default fallback
//! barrier of `confidence < 0.7` for patterns without `fallback_if`.
//!
//! Contract pinned here (the behavioral gates are unit-tested in
//! `src/interpreter/learnable.rs` (TW) and `src/vm.rs` (VM) — under
//! MockLlm the recorded labels are stubs, so the switch is unreachable
//! through programs; the unit seam is the honest way):
//!   * the grammar accepts `distill_min_accuracy: <float>` as a distill
//!     field and the AST carries it (the parser plumbing, TW + VM both
//!     compile it through);
//!   * TW/VM parity: a distill-configured program behaves identically on
//!     both backends (byte-equal outputs).
//!
//! Verify: cargo test --test naryad_456_distill_holdout

use metalogos::ast::{Declaration, LearnablePatternDecl};

fn parse_learnable(source: &str) -> LearnablePatternDecl {
    let declarations = metalogos::parser::parse(source).expect("must parse");
    match &declarations[0] {
        Declaration::LearnablePattern(lp) => lp.clone(),
        other => panic!("expected a learnable pattern, got {:?}", other),
    }
}

#[test]
fn grammar_accepts_distill_min_accuracy() {
    let lp = parse_learnable(
        r#"
learnable pattern Decide(question: String) -> String {
  prompt: "answer"
  distill_to: DecideHead
  distill_after: 5
  distill_min_accuracy: 0.9
}
"#,
    );
    assert_eq!(
        lp.distill_min_accuracy,
        Some(0.9),
        "the explicit gate must land in the AST"
    );
}

#[test]
fn grammar_distill_min_accuracy_is_optional() {
    let lp = parse_learnable(
        r#"
learnable pattern Decide(question: String) -> String {
  prompt: "answer"
  distill_to: DecideHead
}
"#,
    );
    assert!(
        lp.distill_min_accuracy.is_none(),
        "absent field → None → the 0.85 runtime default applies"
    );
}

#[test]
fn grammar_distill_min_accuracy_is_a_distill_field() {
    // Field order inside the learnable body must not matter
    // (distill_field* — the same freedom the other distill fields have).
    let lp = parse_learnable(
        r#"
learnable pattern Decide(question: String) -> String {
  prompt: "answer"
  distill_min_accuracy: 1.01
  distill_to: DecideHead
}
"#,
    );
    assert_eq!(lp.distill_min_accuracy, Some(1.01));
}

/// TW/VM parity: the distill-configured pattern answers identically on
/// both backends (TEACHING-mode stubs under the explicit mock opt-in).
#[test]
fn tw_vm_parity_distill_program() {
    // №454: explicit mock opt-in — the silent mock default is removed.
    std::env::set_var("METALOGOS_MOCK_LLM", "1");

    let source = r#"
reflex ParityHead {
  input: embedding(4)
  layers: [dense(4, "relu"), dense(2, "softmax")]
  labels: ["yes", "no"]
  seed: 42
}

learnable pattern Decide(question: String) -> String {
  prompt: "answer"
  distill_to: ParityHead
  distill_after: 5
  fallback_if: confidence < 0.85
}

pattern Wrap(q: String) -> String {
  let r = Decide(q)
  return r
}

flow Main { input: String = "test" -> Wrap -> output }
"#;

    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let tw = metalogos::run_program_with_dir(source, base.to_path_buf())
        .expect("TW must run")
        .expect("TW must produce output");
    let vm = run_vm(source, base)
        .expect("VM must run")
        .expect("VM output");
    assert_eq!(tw.trim(), vm.trim(), "TW/VM distill parity broken");
}

fn run_vm(source: &str, base_dir: &std::path::Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}
