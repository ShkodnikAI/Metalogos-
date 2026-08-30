// ── НАРЯД #148 — Fix Gt/Ge rollback operators ────────────────────────
//
// Bug: CompareOp::Gt and ::Ge were hardcoded to `false` (always rollback).
// Fix: Gt => accuracy <= threshold, Ge => accuracy < threshold.
//
// Mock accuracy is always 0.95.
// Contracts:
//   C1: Gt with high threshold → kept  (0.95 <= 0.99)
//   C2: Gt with low threshold  → rolled back (0.95 > 0.90)
//   C3: Ge with high threshold → kept  (0.95 < 0.99)
//   C4: Ge with low threshold  → rolled back (0.95 >= 0.90)
//   C5: Lt/Le/Eq/Ne still work (regression guard)

use metalogos::interpreter::Interpreter;

/// Helper: run a Metalogos program and return the mutate log output.
fn run_mutate_program(source: &str) -> Vec<String> {
    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));
    let _ = interp
        .run(metalogos::parser::parse(source).unwrap())
        .unwrap();
    // The mutate messages are returned via handle_mutate, not a separate log.
    // We check audit log instead — it captures both kept and rolled back cases.
    interp.take_audit_log()
}

// ── C1: Gt with threshold 0.99 → kept (0.95 is NOT > 0.99) ─────────

#[test]
fn test_148_gt_kept_when_accuracy_below_threshold() {
    let source = r#"
learnable pattern Classify(text: String) -> String {
  prompt: "Classify this text"
}
adapt Classify add_example("hello", "greeting")
mutate Classify {
  add_example("hi", "greeting")
  rollback_if: accuracy > 0.99
}
"#;
    let audit = run_mutate_program(source);
    let mutate_msg = audit
        .iter()
        .find(|e| e.contains("[AUDIT] mutate"))
        .expect("C1: should have audit mutate entry");
    assert!(
        !mutate_msg.contains("rolled back"),
        "C1: Gt with threshold 0.99 should KEEP (0.95 not > 0.99), got: {}",
        mutate_msg
    );
}

// ── C2: Gt with threshold 0.90 → rolled back (0.95 IS > 0.90) ──────

#[test]
fn test_148_gt_rolled_back_when_accuracy_above_threshold() {
    let source = r#"
learnable pattern Classify(text: String) -> String {
  prompt: "Classify this text"
}
adapt Classify add_example("hello", "greeting")
mutate Classify {
  add_example("hi", "greeting")
  rollback_if: accuracy > 0.90
}
"#;
    let audit = run_mutate_program(source);
    let mutate_msg = audit
        .iter()
        .find(|e| e.contains("[AUDIT] mutate"))
        .expect("C2: should have audit mutate entry");
    assert!(
        mutate_msg.contains("rolled back"),
        "C2: Gt with threshold 0.90 should ROLL BACK (0.95 > 0.90), got: {}",
        mutate_msg
    );
}

// ── C3: Ge with threshold 0.99 → kept (0.95 is NOT >= 0.99) ───────

#[test]
fn test_148_ge_kept_when_accuracy_below_threshold() {
    let source = r#"
learnable pattern Classify(text: String) -> String {
  prompt: "Classify this text"
}
adapt Classify add_example("hello", "greeting")
mutate Classify {
  add_example("hi", "greeting")
  rollback_if: accuracy >= 0.99
}
"#;
    let audit = run_mutate_program(source);
    let mutate_msg = audit
        .iter()
        .find(|e| e.contains("[AUDIT] mutate"))
        .expect("C3: should have audit mutate entry");
    assert!(
        !mutate_msg.contains("rolled back"),
        "C3: Ge with threshold 0.99 should KEEP (0.95 not >= 0.99), got: {}",
        mutate_msg
    );
}

// ── C4: Ge with threshold 0.90 → rolled back (0.95 IS >= 0.90) ───

#[test]
fn test_148_ge_rolled_back_when_accuracy_at_or_above_threshold() {
    let source = r#"
learnable pattern Classify(text: String) -> String {
  prompt: "Classify this text"
}
adapt Classify add_example("hello", "greeting")
mutate Classify {
  add_example("hi", "greeting")
  rollback_if: accuracy >= 0.90
}
"#;
    let audit = run_mutate_program(source);
    let mutate_msg = audit
        .iter()
        .find(|e| e.contains("[AUDIT] mutate"))
        .expect("C4: should have audit mutate entry");
    assert!(
        mutate_msg.contains("rolled back"),
        "C4: Ge with threshold 0.90 should ROLL BACK (0.95 >= 0.90), got: {}",
        mutate_msg
    );
}

// ── C5: Regression — Lt/Le/Eq/Ne still correct ───────────────────

#[test]
fn test_148_lt_kept_when_accuracy_meets_threshold() {
    // Lt: rollback if accuracy < 0.90 → 0.95 is NOT < 0.90 → kept
    let source = r#"
learnable pattern P(text: String) -> String {
  prompt: "test"
}
adapt P add_example("a", "b")
mutate P {
  add_example("c", "d")
  rollback_if: accuracy < 0.90
}
"#;
    let audit = run_mutate_program(source);
    let mutate_msg = audit
        .iter()
        .find(|e| e.contains("[AUDIT] mutate"))
        .expect("C5-Lt: should have audit mutate entry");
    assert!(
        !mutate_msg.contains("rolled back"),
        "C5-Lt: accuracy 0.95 not < 0.90 → should KEEP, got: {}",
        mutate_msg
    );
}

#[test]
fn test_148_le_rolled_back_when_accuracy_below_threshold() {
    // Le: rollback if accuracy <= 0.99 → 0.95 IS <= 0.99 → rolled back
    let source = r#"
learnable pattern P(text: String) -> String {
  prompt: "test"
}
adapt P add_example("a", "b")
mutate P {
  add_example("c", "d")
  rollback_if: accuracy <= 0.99
}
"#;
    let audit = run_mutate_program(source);
    let mutate_msg = audit
        .iter()
        .find(|e| e.contains("[AUDIT] mutate"))
        .expect("C5-Le: should have audit mutate entry");
    assert!(
        mutate_msg.contains("rolled back"),
        "C5-Le: accuracy 0.95 <= 0.99 → should ROLL BACK, got: {}",
        mutate_msg
    );
}

#[test]
fn test_148_eq_kept_when_accuracy_matches() {
    // Eq: rollback if accuracy == 0.95 → 0.95 IS == 0.95 → rolled back
    let source = r#"
learnable pattern P(text: String) -> String {
  prompt: "test"
}
adapt P add_example("a", "b")
mutate P {
  add_example("c", "d")
  rollback_if: accuracy == 0.95
}
"#;
    let audit = run_mutate_program(source);
    let mutate_msg = audit
        .iter()
        .find(|e| e.contains("[AUDIT] mutate"))
        .expect("C5-Eq: should have audit mutate entry");
    assert!(
        mutate_msg.contains("rolled back"),
        "C5-Eq: accuracy 0.95 == 0.95 → should ROLL BACK, got: {}",
        mutate_msg
    );
}

#[test]
fn test_148_ne_kept_when_accuracy_differs() {
    // Ne: rollback if accuracy != 0.95 → 0.95 IS == 0.95, so NOT != → kept
    let source = r#"
learnable pattern P(text: String) -> String {
  prompt: "test"
}
adapt P add_example("a", "b")
mutate P {
  add_example("c", "d")
  rollback_if: accuracy != 0.95
}
"#;
    let audit = run_mutate_program(source);
    let mutate_msg = audit
        .iter()
        .find(|e| e.contains("[AUDIT] mutate"))
        .expect("C5-Ne: should have audit mutate entry");
    assert!(
        !mutate_msg.contains("rolled back"),
        "C5-Ne: accuracy 0.95 not != 0.95 → should KEEP, got: {}",
        mutate_msg
    );
}
