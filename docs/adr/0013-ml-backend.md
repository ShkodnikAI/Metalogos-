# ADR-0013: ML Backend (PyO3 Bridge for Fine-Tuning Learnable Patterns)

**Status:** Implemented (Phase 2.3 — MockMlBackend + `learn` statement)
**Date:** 2026-06-01
**Milestone:** Phase 2

---

## Context

Before this ADR, learnable patterns in Metalogos could only be called via LLM
(prompt-based inference, M3) or have few-shot examples added via `adapt` (M5).
There was no mechanism for **fine-tuning** a pattern on actual training data —
the "Stage 3" capability from the metalogos-language-semantics skill:

> **Stage 3 (phase 2):** дообучение/локальная модель через PyO3+PyTorch,
> экспорт в ONNX для рантайма.

The user requirement: `learn Classify with { data: corpus, epochs: 5 }` should
trigger fine-tuning on a test dataset. Tests must not depend on GPU/PyTorch.

Question: how should Metalogos bridge to ML training infrastructure, and how
should the `learn` statement work in the language?

## Prior Art

| Approach | Source | Trade-off |
|---|---|---|
| In-context learning | GPT-3/4 few-shot | No training needed, limited by context window |
| Prompt programming | DSPy (Khattab et al., 2023) | Declarative prompt optimization |
| Full fine-tuning | PyTorch HuggingFace Transformers | State-of-art, requires GPU + data pipeline |
| LoRA / QLoRA | Hu et al., 2021 | Parameter-efficient fine-tuning, lower GPU requirements |
| ONNX export | Microsoft ONNX Runtime | Cross-platform inference, no Python at runtime |
| PyO3 bridge | PyO3 Rust↔Python bindings | Best of both worlds, deployment complexity |

## Decision

### `learn` Statement in the Language

New declaration type: `learn PatternName with { hyperparams }`.

```mlog
learnable pattern Sentiment(text: String) -> String {
  prompt: "Classify sentiment: positive | negative | neutral"
}

learn Sentiment with { data: "sample_corpus", epochs: 5 }
```

Semantics:
1. Look up the learnable pattern by name (must exist or error).
2. Evaluate hyperparameter expressions (data, epochs, etc.).
3. Call the ML backend's `fine_tune()` method.
4. Log the result as a `[LEARN]` status message.
5. The status message is prepended to the program output (before flow output).

Supported hyperparameters:
- `data: String` — training data identifier or path
- `epochs: Int` — number of training epochs (default: 1)

Unknown hyperparameters are silently ignored (forward-compatible).

### MlBackend Trait

A trait abstracts the ML training source, mirroring the existing `LlmBackend` pattern:

```rust
pub trait MlBackend: Send + Sync {
    fn fine_tune(
        &self,
        pattern_name: &str,
        prompt: &str,
        data: &str,
        epochs: i32,
    ) -> Result<FineTuneResult, String>;
}

pub struct FineTuneResult {
    pub epochs_trained: i32,
    pub accuracy: f64,
    pub summary: String,
}
```

### Phase 2.3: MockMlBackend (No External Dependencies)

Deterministic mock that always returns accuracy=0.95. Used by golden tests.
No GPU, no PyTorch, no Python dependency.

```rust
pub struct MockMlBackend;

impl MlBackend for MockMlBackend {
    fn fine_tune(&self, pattern_name: &str, _prompt: &str, _data: &str, epochs: i32)
        -> Result<FineTuneResult, String>
    {
        Ok(FineTuneResult {
            epochs_trained: epochs,
            accuracy: 0.95,
            summary: format!("{}: fine-tuned (epochs={}, accuracy=0.95)", pattern_name, epochs),
        })
    }
}
```

### EmbeddingBackend Integration

The `EmbeddingBackend` trait from Phase 2.2 (ADR-0012) is the extension point for
real embeddings. Phase 2.3 does NOT modify the embedding backend — that will be
done when the PyO3 bridge is actually connected to sentence-transformers.

### INT Literal Support

Integer literals (`5`, `10`, `30`) are now supported in expressions (previously
only `FLOAT_LITERAL` like `5.0` was valid). Parsed as `Expr::FloatLit(N)`.
This was needed for `epochs: 5` to work naturally in the `learn` statement.

## Rationale

- **Why a separate `MlBackend` trait?** Mirrors the proven `LlmBackend` pattern
  (trait + mock + real). Keeps ML concerns out of the interpreter. Tests use
  mock, production uses real PyO3 backend.
- **Why prepend `[LEARN]` to output?** Learn is a side-effect declaration (like
  `memorize`), not a flow. The status needs to reach the user. Prepending to
  output is consistent with how warnings are already handled.
- **Why not inline the training in the learnable pattern?** Separation of
  concerns: declaration = "train this", invocation = "use this". Training is
  a separate lifecycle event from inference.
- **Why default epochs=1?** Single epoch is the safest default — fast, no
  overfitting risk. User explicitly opts into more training.
- **Why MockMlBackend always returns 0.95?** Deterministic golden test output.
  Real accuracy would vary between runs and platforms.

## Impact

- **`src/ml.rs`:** New module — `MlBackend` trait, `FineTuneResult`,
  `MockMlBackend`, `create_ml_backend()`. 3 unit tests.
- **`src/ast.rs`:** New `LearnDecl` struct, added to `Declaration` enum.
- **`src/grammar.pest`:** New `learn_decl`, `learn_param_list`, `learn_param`
  rules. `LEARN_KW` keyword. `INT` added to `primary_expr` and `literal`.
  `learn` added to `step_ident` exclusion list.
- **`src/parser.rs`:** New `parse_learn_decl()` function. `INT` handled in
  expression parsing and literal conversion.
- **`src/interpreter.rs`:** `Interpreter` gains `ml_backend` and `learn_log`
  fields. `Declaration::Learn` handler calls `ml_backend.fine_tune()`.
  `take_learn_log()` public method.
- **`src/lib.rs`:** Added `pub mod ml`. Learn log collected and prepended to output.
- **`src/semantic.rs`:** `Declaration::Learn` validation — checks pattern exists,
  validates hyperparameter expressions.
- **Backward compatible.** All 13 existing tests pass (9 golden + 4 error).
  New test: `p23_ml_learn.mlog` — learn triggers fine-tuning, status in output.

## Test Coverage

| Test | Type | Validates |
|---|---|---|
| `p23_ml_learn.mlog` | Golden | `learn` statement parses, calls ML backend, status in output |
| `ml::tests::test_mock_fine_tune_returns_result` | Unit | MockMlBackend returns correct epochs and accuracy |
| `ml::tests::test_mock_fine_tune_summary_format` | Unit | Summary string format contains pattern name, epochs, accuracy |
| `ml::tests::test_create_ml_backend_default_mock` | Unit | Factory function creates working mock backend |

**Total: 14 test pairs (10 golden + 4 error) + 3 ML unit tests = 17 tests. All green.**
No test depends on GPU, PyTorch, or Python.

## Phase 2.4 Path — PyO3 Real Backend

The `MlBackend` trait is the extension point. To add real training:

```rust
// Future: PyO3MlBackend (behind feature gate)
#[cfg(feature = "pyo3")]
pub struct PyO3MlBackend {
    model: pyo3::Py<pyo3::PyModule>,  // sentence-transformers
}

impl MlBackend for PyO3MlBackend {
    fn fine_tune(&self, pattern_name: &str, prompt: &str, data: &str, epochs: i32)
        -> Result<FineTuneResult, String>
    {
        // Call Python: trainer.fine_tune(pattern_name, prompt, data, epochs)
        // Export to ONNX for fast inference
    }
}
```

The `create_ml_backend()` factory will switch based on `METALOGOS_MOCK_ML` env var
(and feature flag for PyO3). The `EmbeddingBackend` trait from ADR-0012 gets its
real implementation in the same PyO3 integration.
