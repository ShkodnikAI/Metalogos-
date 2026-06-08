# ADR-0050: Eval Harness — Automatic Evaluation of Learnable Patterns

**Status:** Implemented
**Date:** 2026-06-09

## Context

Learnable patterns in Metalogos delegate classification, translation, and other NLP tasks to an LLM. While `adapt` declarations add few-shot examples at development time, there is no built-in mechanism to systematically verify that a learnable pattern produces correct outputs against a labeled test dataset. Developers need a way to define test suites, run them, measure accuracy, and receive actionable feedback (including automatic `adapt` suggestions) when accuracy falls below a threshold. This is especially important for CI/CD pipelines where pattern quality must be verified before deployment.

## Decision

Add a new top-level `eval` construct that defines a test harness for a learnable pattern:

```mlog
eval Classify {
    dataset: [
        ("ужасный сервис", "complaint"),
        ("спасибо", "greeting"),
        ("когда?", "question")
    ]
    metric: accuracy
    threshold: 0.8
}
```

### Semantics

1. **`eval PatternName { ... }`**: Names the learnable pattern to evaluate. The pattern must be declared (as a `learnable pattern`) earlier in the same file or in an imported module.

2. **`dataset: [(input, expected), ...]`**: A list of (input_string, expected_label) pairs. Each input is passed as the sole argument to the learnable pattern. The pattern's output is compared to the expected label using exact string matching (trimmed). Supports Unicode strings including Cyrillic.

3. **`metric: accuracy`**: The evaluation metric. Currently only `accuracy` (fraction of correct predictions) is supported. Future metrics may include precision, recall, F1.

4. **`threshold: 0.8`**: Minimum acceptable accuracy (0.0 to 1.0). If computed accuracy is below this threshold, the eval block is marked as FAIL.

5. **Execution order**: `eval` blocks are collected during normal `run()` execution (they do not produce output) but are only executed when the `mlog eval` CLI command is used. This ensures that:
   - All learnable patterns are registered first
   - All `adapt` few-shot examples are applied
   - The eval harness runs against the fully configured patterns

6. **Confusion matrix**: For each eval block, a confusion matrix is computed showing expected vs. predicted label counts. This helps identify systematic misclassification patterns (e.g., always confusing "greeting" with "question").

7. **Adapt suggestions**: When accuracy is below threshold, the eval report includes auto-generated `adapt` commands for each failing example:
   ```
   adapt Classify add_example("спасибо", "greeting")
   ```
   These can be copy-pasted directly into the .mlog source to improve the pattern's few-shot examples.

### CLI Command

```bash
mlog eval app.mlog
```

- Parses the .mlog file
- Executes all declarations (registers patterns, applies adapt examples)
- Runs each `eval` block against its dataset
- Prints a report per eval block: accuracy, confusion matrix, pass/fail
- Exits with code 0 if all evals pass, code 1 if any fail

### Output Format

```
Eval: Classify
  Dataset: 3 examples
  Accuracy: 66.7% (2/3)
  Threshold: 0.8
  Result: FAIL (below threshold)

                  complaint     greeting    question
  complaint              1            0           0
  greeting               0            0           1
  question               0            0           1

  Failing examples (suggest adapt):
    - "спасибо" -> expected "greeting", got "wrong"
    - "когда?" -> expected "question", got "wrong"

  Suggested adapt commands:
    adapt Classify add_example("спасибо", "greeting")
    adapt Classify add_example("когда?", "question")
```

### Implementation

- **Grammar**: `eval_decl` rule with `eval_body` containing `eval_dataset`, `eval_metric`, `eval_threshold` sub-rules. Dataset uses specialized `eval_example` = `(STRING_LITERAL, STRING_LITERAL)` syntax (not general expressions, to keep parsing simple and unambiguous). `eval` and related keywords (`dataset`, `metric`, `threshold`) added to the `step_ident` exclusion list to prevent conflicts with flow step names.
- **AST**: `EvalDecl` struct with `pattern_name: String`, `dataset: Vec<(String, String)>`, `metric: String`, `threshold: f64`. `Declaration::Eval(EvalDecl)` variant.
- **Parser**: `parse_eval_decl()` extracts the IDENT pattern name, iterates eval_example pairs to build the dataset, extracts metric name and threshold float.
- **Interpreter**:
  - `eval_blocks: Vec<EvalDecl>` field on `Interpreter`, initialized empty.
  - `run()` stores `Declaration::Eval` blocks without executing them.
  - `run_eval_blocks(&self)` iterates stored eval blocks, invokes `run_single_eval()` for each.
  - `run_single_eval()` looks up the learnable pattern, invokes it on each dataset input via `invoke_learnable_with_env()`, compares trimmed output with expected label, builds confusion matrix, computes accuracy.
  - `EvalResult` public struct with `format_report()` method for human-readable output.
- **CLI**: New `Eval` subcommand in clap. `cmd_eval()` reads file, calls `eval_program()`, prints reports, exits 1 on any failure.
- **lib.rs**: `eval_program()` and `eval_program_with_dir()` public functions.

### Backward Compatibility

- `eval` is a new keyword added to the `step_ident` exclusion list — existing flow step names cannot be `eval`, `dataset`, `metric`, or `threshold`. These are unlikely to have been used as step names in existing code.
- `Declaration::Eval` is a new variant — no existing declarations are affected.
- During normal `run()`, eval blocks are silently collected — they produce no output and modify no state.
- The `mlog eval` command is new — existing commands (`run`, `serve`, `check`, `compile`, `repl`) are unchanged.

## Consequences

- **Positive**: Provides automated quality gates for learnable patterns. The confusion matrix gives diagnostic insight into misclassification patterns. Auto-generated adapt suggestions reduce the friction of improving pattern accuracy. The `mlog eval` exit code enables CI/CD integration.
- **Negative**: The eval harness currently only supports single-argument learnable patterns (the dataset input is passed as one String argument). Patterns with multiple parameters cannot be directly evaluated. The `accuracy` metric is the only one supported — precision/recall/F1 are deferred. The harness always runs the LLM (no dry-run mode) — for large datasets with a real LLM backend, this could be expensive.
- **Neutral**: The eval block is a top-level declaration — it cannot appear inside flows, patterns, or other blocks. The threshold default (0.8) matches common ML practice but is configurable per eval block.
