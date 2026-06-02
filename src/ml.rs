// ── ML Backend for METALOGOS Phase 2.3 ──────────────────────────────────
// Provides fine-tuning and real embeddings for learnable patterns.
// Phase 2.3: MockMlBackend — deterministic stub for golden tests.
// Future:     PyO3MlBackend — real PyTorch/sentence-transformers via PyO3.
//
// The MlBackend trait abstracts away the ML training/inference so that
// `learn` statements work with any backend without changing interpreter code.
//
// Prior art: DSPy (prompt-programming + training), LMQL/Guidance (structured output),
// FastAPI model serving, PyO3 Rust↔Python bridge.

/// Result of a fine-tuning run.
#[derive(Debug, Clone)]
pub struct FineTuneResult {
    /// Number of epochs actually trained.
    pub epochs_trained: i32,
    /// Final accuracy on validation set (0.0..1.0).
    pub accuracy: f64,
    /// Human-readable summary message.
    pub summary: String,
}

/// Trait for ML backends — allows swapping between mock and real.
/// Implementations can use PyO3+PyTorch, ONNX runtime, or local training.
pub trait MlBackend: Send + Sync {
    /// Fine-tune a learnable pattern on the given data for N epochs.
    /// `pattern_name`: name of the learnable pattern to train.
    /// `prompt`: the prompt template from the learnable pattern.
    /// `data`: training data identifier or path.
    /// `epochs`: number of training epochs.
    /// Returns the training result (accuracy, epochs, summary).
    fn fine_tune(
        &self,
        pattern_name: &str,
        prompt: &str,
        data: &str,
        epochs: i32,
    ) -> Result<FineTuneResult, String>;
}

/// Mock ML backend for testing. Returns deterministic results.
/// This is what golden tests use — no GPU, no PyTorch, no Python dependency.
pub struct MockMlBackend;

impl MockMlBackend {
    pub fn new() -> Self {
        MockMlBackend
    }
}

impl MlBackend for MockMlBackend {
    fn fine_tune(
        &self,
        pattern_name: &str,
        _prompt: &str,
        _data: &str,
        epochs: i32,
    ) -> Result<FineTuneResult, String> {
        // Deterministic mock: always succeeds with fixed accuracy.
        // In real backend, this would call PyTorch training via PyO3.
        let accuracy = 0.95;
        let summary = format!(
            "{}: fine-tuned (epochs={}, accuracy={:.2})",
            pattern_name, epochs, accuracy
        );
        Ok(FineTuneResult {
            epochs_trained: epochs,
            accuracy,
            summary,
        })
    }
}

/// Create an ML backend based on environment.
/// If METALOGOS_MOCK_ML is set (default: true), returns MockMlBackend.
/// Future: if false and PyO3 available, returns PyO3MlBackend.
pub fn create_ml_backend() -> Box<dyn MlBackend> {
    let use_mock = std::env::var("METALOGOS_MOCK_ML")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(true); // Default to mock for safety

    if use_mock {
        Box::new(MockMlBackend::new())
    } else {
        // Future: PyO3MlBackend::new()
        // For now, fall back to mock with a note
        Box::new(MockMlBackend::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_fine_tune_returns_result() {
        let backend = MockMlBackend::new();
        let result = backend.fine_tune("TestPattern", "test prompt", "test_data", 5);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.epochs_trained, 5);
        assert!((r.accuracy - 0.95).abs() < 1e-9);
    }

    #[test]
    fn test_mock_fine_tune_summary_format() {
        let backend = MockMlBackend::new();
        let result = backend.fine_tune("Sentiment", "prompt", "corpus", 3);
        let r = result.unwrap();
        assert!(r.summary.contains("Sentiment"));
        assert!(r.summary.contains("epochs=3"));
        assert!(r.summary.contains("0.95"));
    }

    #[test]
    fn test_create_ml_backend_default_mock() {
        let backend = create_ml_backend();
        let result = backend.fine_tune("X", "p", "d", 1);
        assert!(result.is_ok());
    }
}
