//! Наряд №375 (ADR-0112 addendum): REAL golden-task battery for the mutate
//! keep/rollback decision — REAL-MODE side.
//!
//! These tests run with `METALOGOS_MOCK_LLM=false` (REAL mode): the accuracy
//! is measured on the golden-task battery, held-out split, deterministic
//! seeded order. Without a configured LLM provider every held-out answer
//! errors → counts as incorrect → accuracy 0.0 → a positive-threshold
//! rollback_if rolls the mutation back. That is the honest contract: a
//! mutation that cannot be evaluated is not kept.
//!
//! This file is a SEPARATE test binary from `naryad_375_mutate_metric.rs`
//! because the env switch is process-global and Rust runs each tests/*.rs
//! in its own process.

use std::path::{Path, PathBuf};
use std::sync::Once;

static REAL_MODE: Once = Once::new();

fn force_real_mode() {
    REAL_MODE.call_once(|| {
        std::env::set_var("METALOGOS_MOCK_LLM", "false");
    });
}

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

fn program(mutate_clause: &str, eval_block: Option<&str>) -> String {
    let eval = eval_block.unwrap_or("");
    format!(
        "learnable pattern Sentiment(text: String) -> String {{\n  prompt: \"Classify sentiment\"\n}}\n\nadapt Sentiment add_example(\"great service\", \"positive\")\n\n{}\n\n{}\n\npattern Sink(_x: String) -> String {{\n  return \"done\"\n}}\n\nflow Main {{ input: String = \"s\" -> Sink -> output }}",
        eval, mutate_clause
    )
}

/// Real mode: the accuracy comes from the battery measurement, not the stub.
/// Without a provider every held-out answer errors → accuracy 0.0 →
/// `rollback_if: accuracy < 0.5` rolls the mutation back, and the mutate log
/// carries the loud battery note (tasks / held-out / correct / below-minimum).
#[test]
fn naryad_375_real_mode_measures_battery() {
    force_real_mode();
    let base = PathBuf::from("examples");
    let src = program(
        "mutate Sentiment { add_example(\"terrible experience\", \"negative\") rollback_if: accuracy < 0.5 }",
        None,
    );
    let out = run_tw(&src, &base).expect("TW runs");
    let out = out.as_deref().map(str::trim_end).unwrap_or_default();
    assert!(
        out.contains("accuracy=0"),
        "real mode must NOT report the 0.95 stub: {}",
        out
    );
    assert!(
        out.contains("rolled back (below 0.5)"),
        "accuracy 0.0 must roll back under `< 0.5`: {}",
        out
    );
    // Battery = the pre-mutation few-shot (the adapt example) = 1 task; the
    // build input differs → held-out 1; no provider → correct 0.
    assert!(
        out.contains("(battery: 1 tasks, held-out 1, correct 0, BELOW MINIMUM 20)"),
        "battery note must report the real measurement surface: {}",
        out
    );
}

/// The eval-block dataset (ADR-0050) feeds the battery: an eval block with
/// tasks grows the measured surface and the note reflects it.
#[test]
fn naryad_375_real_mode_eval_dataset_feeds_battery() {
    force_real_mode();
    let base = PathBuf::from("examples");
    let eval_block = "eval Sentiment {\n  dataset: [\n    (\"great service\", \"positive\"),\n    (\"awful service\", \"negative\"),\n    (\"ok service\", \"neutral\")\n  ],\n  metric: accuracy,\n  threshold: 0.0\n}";
    let src = program(
        "mutate Sentiment { add_example(\"terrible experience\", \"negative\") rollback_if: accuracy < 0.5 }",
        Some(eval_block),
    );
    let out = run_tw(&src, &base).expect("TW runs");
    let out = out.as_deref().map(str::trim_end).unwrap_or_default();
    // Battery = 3 eval tasks + 1 pre-mutation few-shot task, deduped by
    // input to 3 ("great service" appears in both — eval dataset wins);
    // the build input "terrible experience" is not among them → 3 held out.
    assert!(
        out.contains("(battery: 3 tasks, held-out 3, correct 0, BELOW MINIMUM 20)"),
        "eval dataset must feed the battery: {}",
        out
    );
}

/// A mutation with NO rollback condition is still MEASURED (real accuracy in
/// the log) but stays kept — the decision mapping is unchanged (№375).
#[test]
fn naryad_375_real_mode_no_condition_keeps_but_reports() {
    force_real_mode();
    let base = PathBuf::from("examples");
    let src = program(
        "mutate Sentiment { add_example(\"terrible experience\", \"negative\") }",
        None,
    );
    let out = run_tw(&src, &base).expect("TW runs");
    let out = out.as_deref().map(str::trim_end).unwrap_or_default();
    assert!(
        out.contains("accuracy=0, kept") && out.contains("(battery:"),
        "no-condition mutate must keep but still report the real battery: {}",
        out
    );
}

/// DETERMINISM: the same seed (fixed constant) and the same battery produce
/// the identical measurement on repeated runs.
#[test]
fn naryad_375_real_mode_determinism() {
    force_real_mode();
    let base = PathBuf::from("examples");
    let src = program(
        "mutate Sentiment { add_example(\"terrible experience\", \"negative\") rollback_if: accuracy < 0.5 }",
        None,
    );
    let first = run_tw(&src, &base).expect("run 1");
    let second = run_tw(&src, &base).expect("run 2");
    assert_eq!(
        first.as_deref().map(str::trim_end).unwrap_or_default(),
        second.as_deref().map(str::trim_end).unwrap_or_default(),
        "same battery + same mutation must measure identically (deterministic seeds)"
    );
}

/// VM parity in real mode: the VM path measures the same pre-mutation
/// few-shot battery and reports the same note shape.
#[test]
fn naryad_375_real_mode_vm_parity() {
    force_real_mode();
    let base = PathBuf::from("examples");
    let src = program(
        "mutate Sentiment { add_example(\"terrible experience\", \"negative\") rollback_if: accuracy < 0.5 }",
        None,
    );
    let tw = run_tw(&src, &base).expect("TW runs");
    let vm = run_vm(&src, &base).expect("VM runs");
    let tw_out = tw.as_deref().map(str::trim_end).unwrap_or_default();
    let vm_out = vm.as_deref().map(str::trim_end).unwrap_or_default();
    assert_eq!(tw_out, vm_out, "real-mode TW/VM mutate divergence");
    assert!(vm_out.contains("(battery:"), "VM must report the battery");
}
