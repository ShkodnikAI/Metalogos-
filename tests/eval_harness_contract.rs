// ── Contract tests for Eval Harness (ADR-0050) ─────────────────────────
//
// Tests:
// 1. eval block with 3 examples — accuracy computed correctly
// 2. eval with all correct — PASS
// 3. eval below threshold — FAIL with adapt suggestions
// 4. eval with empty dataset — PASS by convention
// 5. eval with non-existent pattern — error
// 6. eval with few-shot examples boosting accuracy
// 7. confusion matrix correctness

use metalogos::interpreter::{EvalResult, Interpreter};
use metalogos::parser;

/// Helper: parse + run declarations, then run eval blocks.
fn run_eval(source: &str) -> Result<Vec<EvalResult>, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut interp = Interpreter::new();
    interp.run(declarations)?;
    interp.run_eval_blocks()
}

// ── Test 1: eval block with 3 examples, accuracy computed correctly ─────

#[test]
fn test_eval_accuracy_computed() {
    // MockLlm returns the prompt verbatim. Set prompt to the expected label
    // so that all examples match.
    let source = r#"
        learnable pattern Classify(text: String) -> String {
            prompt: "complaint"
        }
        eval Classify {
            dataset: [
                ("ужасный сервис", "complaint"),
                ("спасибо", "complaint"),
                ("когда?", "complaint")
            ],
            metric: accuracy,
            threshold: 0.8
        }
    "#;

    let results = run_eval(source).unwrap();
    assert_eq!(results.len(), 1);
    let r = &results[0];
    assert_eq!(r.pattern_name, "Classify");
    assert_eq!(r.total, 3);
    assert_eq!(r.correct, 3);
    assert!((r.accuracy - 1.0).abs() < 0.001);
    assert!(r.passed);
}

// ── Test 2: eval with all correct — PASS ────────────────────────────────

#[test]
fn test_eval_all_correct_pass() {
    // MockLlm returns the prompt as-is. If prompt says "Return: greeting",
    // the response is "Return: greeting". So to match expected labels,
    // we set the prompt to the expected label for a subset of examples.
    let source = r#"
        learnable pattern Sentiment(text: String) -> String {
            prompt: "positive"
        }
        eval Sentiment {
            dataset: [
                ("great product", "positive"),
                ("love it", "positive"),
                ("awesome", "positive")
            ],
            metric: accuracy,
            threshold: 0.5
        }
    "#;

    let results = run_eval(source).unwrap();
    assert_eq!(results[0].correct, 3);
    assert!(results[0].passed);
}

// ── Test 3: eval below threshold — FAIL ────────────────────────────────

#[test]
fn test_eval_below_threshold_fail() {
    let source = r#"
        learnable pattern Classify(text: String) -> String {
            prompt: "wrong_label"
        }
        eval Classify {
            dataset: [
                ("ужасный сервис", "complaint"),
                ("спасибо", "greeting"),
                ("когда?", "question")
            ],
            metric: accuracy,
            threshold: 0.8
        }
    "#;

    let results = run_eval(source).unwrap();
    assert_eq!(results.len(), 1);
    let r = &results[0];
    assert_eq!(r.correct, 0);
    assert!(!r.passed);
    // Should have 3 failures
    assert_eq!(r.failures.len(), 3);
    // Should have adapt suggestions for each failure
    for (input, expected, actual) in &r.failures {
        assert_eq!(actual, "wrong_label");
        assert_ne!(expected, "wrong_label");
        assert!(!input.is_empty());
    }
}

// ── Test 4: eval with empty dataset — PASS by convention ──────────────

#[test]
fn test_eval_empty_dataset_pass() {
    let source = r#"
        learnable pattern Classify(text: String) -> String {
            prompt: "any"
        }
        eval Classify {
            dataset: [],
            metric: accuracy,
            threshold: 0.8
        }
    "#;

    let results = run_eval(source).unwrap();
    assert_eq!(results[0].total, 0);
    assert_eq!(results[0].correct, 0);
    assert!((results[0].accuracy - 1.0).abs() < 0.001);
    assert!(results[0].passed);
}

// ── Test 5: eval with non-existent pattern — error ─────────────────────

#[test]
fn test_eval_nonexistent_pattern_error() {
    let source = r#"
        eval NonExistent {
            dataset: [("hello", "greeting")],
            metric: accuracy,
            threshold: 0.8
        }
    "#;

    let result = run_eval(source);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

// ── Test 6: eval with few-shot examples boosting accuracy ───────────────

#[test]
fn test_eval_with_few_shot_boost() {
    // Few-shot adapt examples provide exact-match shortcuts.
    // MockLlm returns the prompt, but few-shot matching happens first
    // in invoke_learnable_with_env. If input matches a few-shot example input,
    // the few-shot output is returned directly (bypassing LLM).
    let source = r#"
        learnable pattern Classify(text: String) -> String {
            prompt: "Classify as: complaint | greeting | question"
        }
        adapt Classify add_example("ужасный сервис", "complaint")
        adapt Classify add_example("спасибо", "greeting")
        eval Classify {
            dataset: [
                ("ужасный сервис", "complaint"),
                ("спасибо", "greeting")
            ],
            metric: accuracy,
            threshold: 0.8
        }
    "#;

    let results = run_eval(source).unwrap();
    assert_eq!(results[0].total, 2);
    assert_eq!(results[0].correct, 2);
    assert!(results[0].passed);
}

// ── Test 7: confusion matrix correctness ────────────────────────────────

#[test]
fn test_eval_confusion_matrix() {
    let source = r#"
        learnable pattern RouteMsg(text: String) -> String {
            prompt: "wrong"
        }
        eval RouteMsg {
            dataset: [
                ("bug report", "engineering"),
                ("pay issue", "billing"),
                ("feature request", "product")
            ],
            metric: accuracy,
            threshold: 0.5
        }
    "#;

    let results = run_eval(source).unwrap();
    let r = &results[0];

    // All predictions are "wrong", all expected are different labels
    // Confusion matrix should have 3 entries: each expected -> wrong -> 1
    assert_eq!(r.confusion.len(), 3);
    assert_eq!(
        *r.confusion
            .get("engineering")
            .unwrap()
            .get("wrong")
            .unwrap(),
        1
    );
    assert_eq!(
        *r.confusion.get("billing").unwrap().get("wrong").unwrap(),
        1
    );
    assert_eq!(
        *r.confusion.get("product").unwrap().get("wrong").unwrap(),
        1
    );

    // All wrong -> accuracy = 0.0
    assert!((r.accuracy).abs() < 0.001);
    assert!(!r.passed);
}

// ── Test 8: format_report output ───────────────────────────────────────

#[test]
fn test_eval_format_report() {
    let source = r#"
        learnable pattern X(text: String) -> String {
            prompt: "a"
        }
        eval X {
            dataset: [
                ("input1", "a"),
                ("input2", "b")
            ],
            metric: accuracy,
            threshold: 0.8
        }
    "#;

    let results = run_eval(source).unwrap();
    let report = results[0].format_report();

    assert!(report.contains("Eval: X"));
    assert!(report.contains("Dataset: 2 examples"));
    assert!(report.contains("Accuracy: 50.0%"));
    assert!(report.contains("Threshold: 0.8"));
    assert!(report.contains("FAIL"));
    assert!(report.contains("Failing examples"));
    assert!(report.contains("suggest adapt"));
    assert!(report.contains("adapt X add_example"));
}

// ── Test 9: multiple eval blocks ──────────────────────────────────────

#[test]
fn test_eval_multiple_blocks() {
    let source = r#"
        learnable pattern A(text: String) -> String {
            prompt: "correct"
        }
        learnable pattern B(text: String) -> String {
            prompt: "correct"
        }
        eval A {
            dataset: [("x", "correct")],
            metric: accuracy,
            threshold: 0.5
        }
        eval B {
            dataset: [("y", "correct")],
            metric: accuracy,
            threshold: 0.5
        }
    "#;

    let results = run_eval(source).unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].pattern_name, "A");
    assert_eq!(results[1].pattern_name, "B");
    assert!(results[0].passed);
    assert!(results[1].passed);
}
