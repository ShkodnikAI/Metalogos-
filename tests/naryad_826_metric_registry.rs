//! Наряд №826 (issue #1609, wave 68, P1, класс [domain]): W2′ — the metric
//! registry (`macro_f1`, `min_class_recall`), the `rollback_if` metric NAME
//! (the old literal-accuracy form stays valid), and the OBLIGATORY majority
//! baseline on the gating rule (ADR-0216).
//!
//! RED-first (форма №772): every test below compiles on the UNPATCHED main
//! and fails there behaviorally — the registry exposes only `accuracy`, the
//! grammar pins the `accuracy` literal (a named metric fails at parse), and
//! the mutate gate carries no baseline. After the patch all six contract
//! groups go green without touching the №375 mock byte contract.
//!
//! The metric math lives in `metalogos-reflex/src/nn/metric.rs` (the SSOT
//! №545) and reaches these tests ONLY through the public registry surface
//! (`metalogos::nn::metric`) — the registration and the math are pinned
//! together.
//!
//! Env hygiene: mock- and real-mode tests share one process — every
//! env-sensitive test is `#[serial]` so `METALOGOS_MOCK_LLM` never races
//! (the №375 two-file split is not available here: the наряд mandates one
//! file).

// Naryad #475 (issue #723): the fs_gate ratchet (clippy disallowed-methods)
// targets PRODUCTION I/O paths. This test file reads the golden example
// fixture by design — the allow is scoped to this file (the №375 precedent).
#![allow(clippy::disallowed_methods)]

use serial_test::serial;
use std::path::{Path, PathBuf};

fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.to_path_buf())
}

fn run_vm(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp
        .compile(declarations)
        .map_err(|e| format!("compile error: {}", e))?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program).map_err(|e| e.to_string())
}

fn force_env(value: &str) {
    std::env::set_var("METALOGOS_MOCK_LLM", value);
}

fn one_hot(class: usize, n_classes: usize) -> Vec<f64> {
    let mut v = vec![0.0; n_classes];
    v[class] = 1.0;
    v
}

// ── Contract 1: the registry (macro_f1, min_class_recall) ───────────────

/// The registry carries all three metrics; the named ones resolve.
#[test]
fn n826_contract1_registry_contains_named_metrics() {
    let names = metalogos::nn::metric_names();
    assert!(
        names.contains(&"accuracy"),
        "accuracy must stay registered: {:?}",
        names
    );
    assert!(
        names.contains(&"macro_f1"),
        "macro_f1 must be registered (№826): {:?}",
        names
    );
    assert!(
        names.contains(&"min_class_recall"),
        "min_class_recall must be registered (№826): {:?}",
        names
    );
    assert_eq!(
        names.len(),
        3,
        "no metrics beyond №826's three: {:?}",
        names
    );
    assert!(metalogos::nn::find_metric("macro_f1").is_some());
    assert!(metalogos::nn::find_metric("min_class_recall").is_some());
    assert!(metalogos::nn::find_metric("f1_score").is_none());
}

/// macro_f1 through the registry compute surface — hand vectors, including
/// the edge cases the наряд names (a class without predictions, a class
/// without truths).
#[test]
fn n826_contract1_macro_f1_hand_vectors() {
    let f1 = metalogos::nn::find_metric("macro_f1")
        .expect("macro_f1 registered")
        .compute;
    // Perfect 3-class: argmax == truth everywhere → 1.0.
    let preds = vec![one_hot(0, 3), one_hot(1, 3), one_hot(2, 3)];
    assert!((f1(&preds, &[0, 1, 2]) - 1.0).abs() < 1e-9);
    // Mixed: truth [0,0,1,1,2], pred [0,0,1,1,0] → per-class F1:
    // class0 TP=2 FP=1 FN=0 → 0.8; class1 TP=2 → 1.0; class2 FN=1 → 0.0
    // → macro 1.8/3 = 0.6.
    let preds = vec![
        one_hot(0, 3),
        one_hot(0, 3),
        one_hot(1, 3),
        one_hot(1, 3),
        one_hot(0, 3),
    ];
    assert!((f1(&preds, &[0, 0, 1, 1, 2]) - 0.6).abs() < 1e-9,);
    // A vector with FP and FN on the same class: truth [0,0,1,1,2],
    // pred [0,1,1,1,0] → class0 0.5, class1 0.8, class2 0.0 → 13/30.
    let preds = vec![
        one_hot(0, 3),
        one_hot(1, 3),
        one_hot(1, 3),
        one_hot(1, 3),
        one_hot(0, 3),
    ];
    assert!((f1(&preds, &[0, 0, 1, 1, 2]) - 13.0 / 30.0).abs() < 1e-9);
    // A class predicted but never true (class 1): precision 0 → F1 0; the
    // class counts in the average (it IS present in the confusion).
    // truth [0,0], pred [0,1] → class0 F1 = 2/3, class1 F1 = 0 → macro 1/3.
    let preds = vec![one_hot(0, 2), one_hot(1, 2)];
    assert!((f1(&preds, &[0, 0]) - 1.0 / 3.0).abs() < 1e-9);
    // Empty evidence → 0.0 (no invention).
    assert_eq!(f1(&[], &[]), 0.0);
}

/// min_class_recall through the registry — hand vectors + the no-truth
/// skip (a class without truths cannot be recalled; it does not drag the
/// minimum to a fabricated 0).
#[test]
fn n826_contract1_min_class_recall_hand_vectors() {
    let mcr = metalogos::nn::find_metric("min_class_recall")
        .expect("min_class_recall registered")
        .compute;
    let preds = vec![one_hot(0, 3), one_hot(1, 3), one_hot(2, 3)];
    assert!((mcr(&preds, &[0, 1, 2]) - 1.0).abs() < 1e-9);
    // Mixed: recalls 1.0, 1.0, 0.0 → min 0.0 (class2 in truth, never predicted).
    let preds = vec![
        one_hot(0, 3),
        one_hot(0, 3),
        one_hot(1, 3),
        one_hot(1, 3),
        one_hot(0, 3),
    ];
    assert!((mcr(&preds, &[0, 0, 1, 1, 2]) - 0.0).abs() < 1e-9);
    // A recall below the top: truth [0,0,1,1,2], pred [0,1,1,1,1] →
    // recalls 0.5, 1.0, 0.0 → min 0.0; and truth [0,1,1], pred [0,1,1] →
    // recalls 1.0, 1.0 → min 1.0.
    let preds = vec![one_hot(0, 2), one_hot(1, 2), one_hot(1, 2)];
    assert!((mcr(&preds, &[0, 1, 1]) - 1.0).abs() < 1e-9);
    // A class without truths is SKIPPED (undefined, not zero):
    // truth [0,0], pred [0,1] → recall(class0) = 0.5; class1 has no truths.
    let preds = vec![one_hot(0, 2), one_hot(1, 2)];
    assert!((mcr(&preds, &[0, 0]) - 0.5).abs() < 1e-9);
    assert_eq!(mcr(&[], &[]), 0.0);
}

/// The majority-baseline math (the №485 pattern brought to the mutate gate):
/// the baseline value of a metric is the metric of the always-majority
/// predictor over the same truth. The majority class here comes from a
/// LOCAL oracle (an independent count), the metric from the registry —
/// cross-checked. (Tie-breaking is pinned at the gate level by the E2E
/// baseline values in contract 3/5.)
#[test]
fn n826_contract1_majority_baseline_math() {
    // Local oracle: majority of truth [0,0,1] over 2 classes = 0.
    let truth = [0usize, 0, 1];
    let mut counts = [0usize; 2];
    for &t in &truth {
        counts[t] += 1;
    }
    let majority = counts
        .iter()
        .enumerate()
        .max_by_key(|(i, c)| (**c, std::cmp::Reverse(*i)))
        .map(|(i, _)| i)
        .expect("non-empty");
    assert_eq!(majority, 0);
    // Baseline of macro_f1: the always-majority predictor → class0 F1 = 0.8,
    // class1 F1 = 0 → 0.4.
    let baseline_preds = vec![one_hot(majority, 2); 3];
    let f1 = metalogos::nn::find_metric("macro_f1")
        .expect("registered")
        .compute;
    assert!((f1(&baseline_preds, &truth) - 0.4).abs() < 1e-9);
}

// ── Contract 2: the old form stays valid ────────────────────────────────

/// `rollback_if: accuracy < 0.9` parses and runs EXACTLY as before — the
/// №375 mock byte contract holds (the message has no metric-name or
/// baseline additions in mock mode).
#[test]
#[serial]
fn n826_contract2_old_form_stays_valid() {
    force_env("1");
    let base = PathBuf::from("examples");
    let src = "learnable pattern Sentiment(text: String) -> String {\n  prompt: \"Classify sentiment\"\n}\n\nmutate Sentiment { add_example(\"x\", \"y\") rollback_if: accuracy < 0.9 }\n\nflow Main { input: String = \"s\" -> Sentiment -> output }";
    let tw = run_tw(src, &base).expect("TW runs");
    let vm = run_vm(src, &base).expect("VM runs");
    let tw_out = tw.as_deref().map(str::trim_end).unwrap_or_default();
    let vm_out = vm.as_deref().map(str::trim_end).unwrap_or_default();
    assert_eq!(tw_out, vm_out, "old-form TW/VM divergence");
    assert!(
        tw_out.contains("[MUTATE] Sentiment: accuracy=0.95, kept (>= 0.9)"),
        "the №375 mock byte contract must hold verbatim: {}",
        tw_out
    );
}

/// The p2_full_adapt golden contract is untouched by the grammar change.
#[test]
#[serial]
fn n826_contract2_p2_golden_unchanged() {
    force_env("1");
    let base = PathBuf::from("examples");
    let path = base.join("p2_full_adapt.mlog");
    let source = std::fs::read_to_string(&path).expect("p2 example exists");
    let expected =
        std::fs::read_to_string(path.with_extension("expected")).expect("p2 expected exists");
    let tw = run_tw(&source, &base).expect("p2 runs on TW");
    let vm = run_vm(&source, &base).expect("p2 runs on VM");
    let exp = expected.trim_end();
    assert_eq!(tw.as_deref().map(str::trim_end).unwrap_or_default(), exp);
    assert_eq!(vm.as_deref().map(str::trim_end).unwrap_or_default(), exp);
}

// ── Contract 3: the new form gates on the metric's own value ────────────

/// Mock mode: the named metric flows through the SAME decision path — the
/// message carries the metric's own name and the stub value (the №375 stub
/// is metric-agnostic; ADR-0216 records the posture).
#[test]
#[serial]
fn n826_contract3_new_form_mock_gates() {
    force_env("1");
    let base = PathBuf::from("examples");
    // macro_f1 > 0.8: stub 0.95 > 0.8 → the rollback condition is TRUE →
    // rolled back (the decision reads the METRIC's value, not accuracy's).
    let rollback_src = "learnable pattern Sentiment(text: String) -> String {\n  prompt: \"Classify sentiment\"\n}\n\nmutate Sentiment { add_example(\"x\", \"y\") rollback_if: macro_f1 > 0.8 }\n\nflow Main { input: String = \"s\" -> Sentiment -> output }";
    let out = run_tw(rollback_src, &base).expect("TW runs");
    let out = out.as_deref().map(str::trim_end).unwrap_or_default();
    assert!(
        out.contains("[MUTATE] Sentiment: macro_f1=0.95, rolled back (below 0.8)"),
        "the gate must read the named metric's own value: {}",
        out
    );
    // min_class_recall < 0.9: stub 0.95 < 0.9 is false → kept.
    let keep_src = "learnable pattern Sentiment(text: String) -> String {\n  prompt: \"Classify sentiment\"\n}\n\nmutate Sentiment { add_example(\"x\", \"y\") rollback_if: min_class_recall < 0.9 }\n\nflow Main { input: String = \"s\" -> Sentiment -> output }";
    let out = run_tw(keep_src, &base).expect("TW runs");
    let out = out.as_deref().map(str::trim_end).unwrap_or_default();
    assert!(
        out.contains("min_class_recall=0.95, kept (>= 0.9)"),
        "keep edge on the named metric: {}",
        out
    );
}

/// Real mode (no provider → every held-out answer errors): the named
/// metric's REAL value gates the mutation, and the decision is accompanied
/// by the majority baseline segment (№826 requirement 3).
#[test]
#[serial]
fn n826_contract3_new_form_real_value_with_baseline() {
    force_env("false");
    let base = PathBuf::from("examples");
    let src = "learnable pattern Sentiment(text: String) -> String {\n  prompt: \"Classify sentiment\"\n}\n\nadapt Sentiment add_example(\"great service\", \"positive\")\n\nmutate Sentiment { add_example(\"terrible experience\", \"negative\") rollback_if: macro_f1 < 0.8 }\n\npattern Sink(_x: String) -> String {\n  return \"done\"\n}\n\nflow Main { input: String = \"s\" -> Sink -> output }";
    let out = run_tw(src, &base).expect("TW runs");
    let out = out.as_deref().map(str::trim_end).unwrap_or_default();
    assert!(
        out.contains("macro_f1=") && out.contains("rolled back"),
        "real mode must report the named metric's real value: {}",
        out
    );
    assert!(
        !out.contains("0.95"),
        "real mode must NOT report the stub: {}",
        out
    );
    assert!(
        out.contains("majority baseline"),
        "the gate decision must be accompanied by the majority baseline: {}",
        out
    );
    // The exact baseline pin: the battery is 1 task ("positive"), every
    // answer errors → the majority predictor (always "positive") is perfect
    // on the same surface → the baseline of macro_f1 is exactly 1.000.
    assert!(
        out.contains("majority baseline macro_f1=1.000"),
        "the majority baseline must carry the metric name and the exact value: {}",
        out
    );
    assert!(
        out.contains("(battery: 1 tasks, held-out 1, correct 0"),
        "the battery note stays: {}",
        out
    );
}

// ── Contract 4: unknown metric name — loud refusal at the check ─────────

/// The grammar accepts the name (IDENT), the semantic check refuses an
/// unregistered one loudly — never a silent fallback (the reflex_train
/// convention). The refusal is at the CHECK phase: parse alone succeeds.
#[test]
fn n826_contract4_unknown_metric_loud_refusal() {
    let src = "learnable pattern Sentiment(text: String) -> String {\n  prompt: \"Classify sentiment\"\n}\n\nmutate Sentiment { add_example(\"x\", \"y\") rollback_if: f1_score > 0.8 }\n\nflow Main { input: String = \"s\" -> Sentiment -> output }";
    // Parse alone SUCCEEDS (the name is a legal IDENT).
    let parsed = metalogos::parser::parse(src);
    assert!(
        parsed.is_ok(),
        "the grammar accepts a metric NAME — refusal belongs to the check: {:?}",
        parsed.err()
    );
    // The check refuses loudly, naming the metric and the registered set.
    let analysis = metalogos::check_program(src).expect("check runs");
    assert!(
        analysis
            .errors
            .iter()
            .any(|e| e.message.contains("mutate: unknown metric 'f1_score'")
                && e.message.contains("Registered metrics:")),
        "the semantic check must refuse the unknown metric loudly: {:?}",
        analysis.errors
    );
    // The run path blocks on the finding (fail-closed, №523).
    let base = PathBuf::from("examples");
    let run = run_tw(src, &base);
    assert!(
        run.is_err()
            && run
                .as_ref()
                .err()
                .unwrap()
                .contains("unknown metric 'f1_score'"),
        "the run path must block on the unknown metric: {:?}",
        run
    );
}

// ── Contract 5: a gating metric without a baseline — loud refusal ───────

/// Real mode: the battery is fully covered by the mutation's own inputs →
/// no held-out evidence → no majority baseline → the gate refuses loudly
/// (a self-modifying system does not gate on nothing).
#[test]
#[serial]
fn n826_contract5_gate_without_baseline_refuses() {
    force_env("false");
    let base = PathBuf::from("examples");
    // The adapt example IS the build input → the battery (the pre-mutation
    // few-shot) is fully covered → held-out 0.
    let src = "learnable pattern Sentiment(text: String) -> String {\n  prompt: \"Classify sentiment\"\n}\n\nadapt Sentiment add_example(\"great service\", \"positive\")\n\nmutate Sentiment { add_example(\"great service\", \"negative\") rollback_if: accuracy < 0.5 }\n\npattern Sink(_x: String) -> String {\n  return \"done\"\n}\n\nflow Main { input: String = \"s\" -> Sink -> output }";
    let tw = run_tw(src, &base);
    assert!(
        tw.is_err() && tw.as_ref().err().unwrap().contains("majority baseline"),
        "TW must refuse the baseline-less gate loudly: {:?}",
        tw
    );
    let vm = run_vm(src, &base);
    assert!(
        vm.is_err() && vm.as_ref().err().unwrap().contains("majority baseline"),
        "VM must refuse the baseline-less gate loudly: {:?}",
        vm
    );
}

// ── Contract 6: VM ↔ TW crosscheck on the new form ──────────────────────

/// Byte parity of the full new-form surface (both metrics, both verdicts,
/// the baseline segment in real mode) across the lanes.
#[test]
#[serial]
fn n826_contract6_vm_tw_crosscheck_new_form() {
    force_env("1");
    let base = PathBuf::from("examples");
    let cases = [
        "mutate Sentiment { add_example(\"x\", \"y\") rollback_if: macro_f1 > 0.8 }",
        "mutate Sentiment { add_example(\"x\", \"y\") rollback_if: macro_f1 < 0.9 }",
        "mutate Sentiment { add_example(\"x\", \"y\") rollback_if: min_class_recall > 0.8 }",
        "mutate Sentiment { add_example(\"x\", \"y\") rollback_if: min_class_recall < 0.9 }",
        "mutate Sentiment { add_example(\"x\", \"y\") rollback_if: accuracy > 0.8 }",
    ];
    for clause in cases {
        let src = format!("learnable pattern Sentiment(text: String) -> String {{\n  prompt: \"Classify sentiment\"\n}}\n\n{}\n\nflow Main {{ input: String = \"s\" -> Sentiment -> output }}", clause);
        let tw = run_tw(&src, &base).expect("TW runs");
        let vm = run_vm(&src, &base).expect("VM runs");
        assert_eq!(
            tw.as_deref().map(str::trim_end).unwrap_or_default(),
            vm.as_deref().map(str::trim_end).unwrap_or_default(),
            "TW/VM divergence on: {}",
            clause
        );
    }
}
