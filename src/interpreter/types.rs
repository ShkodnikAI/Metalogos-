// ── Interpreter auxiliary types ───────────────────────────────────────
// Standalone types used by the interpreter that don't depend on
// the Interpreter struct itself.

use std::collections::HashMap;

use super::values::Value;
use crate::ast::{CompareOp, ContextMode, ContextStrategy, Param};

/// A learnable pattern that calls an LLM.
#[derive(Clone)]
pub struct CompiledLearnable {
    pub params: Vec<Param>,
    pub prompt: String,
    /// Few-shot examples added by `adapt` declarations.
    /// Each entry: (input_string, output_string).
    pub few_shot: Vec<(String, String)>,
    /// Optional context auto-loading mode (ADR-0046).
    /// - None: no context (default, backward compatible)
    /// - Auto: recall(first_param, limit=5)
    /// - Recall(query_expr, limit): explicit recall
    /// - Literal(string): static text prepended to prompt
    pub context: Option<ContextMode>,
    /// Optional context compression strategy (ADR-0055).
    /// - None: no compression (default)
    /// - Auto: inject as-is
    /// - Compress: compress via LLM if exceeds max_context_tokens
    pub context_strategy: ContextStrategy,
    /// Max estimated tokens for context before compression (ADR-0055).
    /// Default: 2000.
    pub max_context_tokens: usize,
    /// Optional max_tokens for LLM backend.
    pub max_tokens: Option<u32>,
    /// Enable LLM response caching (ADR-0047).
    pub cache: bool,
    /// Cache time-to-live in seconds. Default 3600 (1 hour).
    pub cache_ttl: u64,
    /// Optional per-pattern model override (ADR-0048).
    /// When set, passed to the LLM backend instead of the global model.
    pub model: Option<String>,
    /// Optional conversation binding (ADR-0053).
    /// When set (e.g., "current"), the learnable pattern injects conversation history.
    pub conversation: Option<String>,
    /// Наряд №181 (ADR-0117): distillation config — None = no distillation
    /// (LLM-only path, byte-identical to pre-Наряд №181 behavior).
    /// Some(...) = enable TEACHING→DISTILLED cycle.
    pub distill: Option<DistillConfig>,
}

/// Наряд №181 (ADR-0117): distillation configuration for a learnable pattern.
/// Built from LearnablePatternDecl's distill_to / distill_after / fallback_if.
#[derive(Clone, Debug)]
pub struct DistillConfig {
    /// Name of the `reflex X { ... }` declaration to distill into.
    pub reflex_name: String,
    /// Minimum accumulated examples before reflex_train is called once.
    pub distill_after: usize,
    /// Confidence threshold for falling back to LLM in DISTILLED mode.
    /// None = always return local prediction (no fallback).
    /// Some((op, threshold)) = if !op.compare(predict_confidence, threshold)
    /// → call LLM as fallback.
    pub fallback_if: Option<(CompareOp, f64)>,
    /// Current mode — TEACHING (still accumulating examples) or DISTILLED
    /// (reflex_train already succeeded, use reflex_predict).
    /// Starts as TEACHING; switches to DISTILLED after reflex_train returns
    /// accuracy ≥ 0.0 (i.e., any successful training).
    /// Mutable runtime state — wrapped in Mutex at the call site, not here.
    pub mode: DistillMode,
}

/// Наряд №181: execution mode for a distilling learnable pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DistillMode {
    /// Still accumulating examples. Each pattern call → LLM + record example.
    Teaching,
    /// reflex_train already succeeded. Each pattern call → reflex_predict.
    /// May fall back to LLM if confidence is below `fallback_if` threshold.
    Distilled,
}

/// Наряд №181: per-pattern runtime state for distillation.
/// Stored in `Interpreter::distill_states` (Mutex<HashMap<name, DistillRuntimeState>>)
/// so it survives across calls within a single `mlog run` even though
/// `CompiledLearnable` is cloned fresh on each invocation.
///
/// Accumulated examples use the (input_string, output_string) format
/// already established by `learnable.few_shot` — reuses the same example
/// shape rather than inventing a parallel one.
#[derive(Clone, Debug)]
pub struct DistillRuntimeState {
    /// Current mode — TEACHING (LLM-only, accumulate) or DISTILLED (predict).
    pub mode: DistillMode,
    /// Accumulated (input, target_label) examples.
    /// Input = the join of pattern args (same string used for few-shot match).
    /// Target = the LLM's response (a label string, since ADR-0117
    /// restricts distillation to closed-label patterns).
    pub examples: Vec<(String, String)>,
    /// Total example count at the last training attempt. Used to avoid
    /// retrying training on every call after `distill_after` is crossed
    /// but training failed (e.g., below ADR-0115 minimum of 10, or
    /// training accuracy was too low). Next retry: when `examples.len()`
    /// grows by 5 more past `last_train_attempt`.
    /// 0 = no training has been attempted yet.
    pub last_train_attempt: usize,
}

/// Result of running a single test block (Наряд №120).
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Test name from the `test "..."` declaration.
    pub name: String,
    /// Whether the test passed (no assertion errors).
    pub passed: bool,
    /// Error message if the test failed, None if passed.
    pub error: Option<String>,
}

impl TestResult {
    /// Human-readable one-line summary for `mlog test` output.
    pub fn format_line(&self) -> String {
        match &self.error {
            None => format!("\u{2705} {}", self.name),
            Some(e) => format!("\u{274c} {}: {}", self.name, e),
        }
    }
}

/// Result of running an eval block (ADR-0050).
/// Contains accuracy, confusion matrix, and failure details.
#[derive(Debug, Clone)]
pub struct EvalResult {
    /// Name of the evaluated learnable pattern.
    pub pattern_name: String,
    /// Metric used (currently only "accuracy").
    pub metric: String,
    /// Total number of test examples.
    pub total: usize,
    /// Number of correctly predicted examples.
    pub correct: usize,
    /// Fraction of correct predictions (correct / total).
    pub accuracy: f64,
    /// Minimum acceptable accuracy threshold.
    pub threshold: f64,
    /// Whether accuracy >= threshold (eval passes).
    pub passed: bool,
    /// Confusion matrix: expected_label -> predicted_label -> count.
    pub confusion: std::collections::HashMap<String, std::collections::HashMap<String, usize>>,
    /// Failing examples: (input, expected, actual).
    pub failures: Vec<(String, String, String)>,
}

impl EvalResult {
    /// Format the eval result as a human-readable report.
    pub fn format_report(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("Eval: {}", self.pattern_name));
        lines.push(format!("  Dataset: {} examples", self.total));
        lines.push(format!(
            "  Accuracy: {:.1}% ({}/{})",
            self.accuracy * 100.0,
            self.correct,
            self.total
        ));
        lines.push(format!("  Threshold: {}", self.threshold));
        lines.push(format!(
            "  Result: {}",
            if self.passed {
                "PASS"
            } else {
                "FAIL (below threshold)"
            }
        ));

        // Confusion matrix
        if !self.confusion.is_empty() {
            // Collect all unique labels
            let mut all_labels: std::collections::BTreeSet<String> =
                std::collections::BTreeSet::new();
            for (expected, predictions) in &self.confusion {
                all_labels.insert(expected.clone());
                for pred in predictions.keys() {
                    all_labels.insert(pred.clone());
                }
            }

            let labels: Vec<&String> = all_labels.iter().collect();

            // Header row
            let header = format!(
                "  {:12} {}",
                "",
                labels
                    .iter()
                    .map(|l| format!("{:>12}", l))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            lines.push(header);

            // Data rows
            for expected in &labels {
                let row = format!(
                    "  {:12} {}",
                    expected,
                    labels
                        .iter()
                        .map(|pred| {
                            let count = self
                                .confusion
                                .get(*expected)
                                .and_then(|m| m.get(*pred))
                                .copied()
                                .unwrap_or(0);
                            format!("{:>12}", count)
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                );
                lines.push(row);
            }
        }

        // Failing examples with adapt suggestions
        if !self.failures.is_empty() {
            lines.push(String::new());
            lines.push("  Failing examples (suggest adapt):".to_string());
            for (input, expected, actual) in &self.failures {
                lines.push(format!(
                    "    - {:?} -> expected {:?}, got {:?}",
                    input, expected, actual
                ));
            }
            // Generate adapt suggestions
            lines.push(String::new());
            lines.push("  Suggested adapt commands:".to_string());
            for (input, expected, _actual) in &self.failures {
                lines.push(format!(
                    "    adapt {} add_example({:?}, {:?})",
                    self.pattern_name, input, expected
                ));
            }
        }

        lines.join("\n")
    }
}

/// Per-pattern runtime statistics (ADR-0051).
/// Tracked automatically during pattern invocation and adapt operations.
/// Returned by the `inspect()` builtin.
#[derive(Debug, Clone)]
pub struct PatternStats {
    /// Total number of invocations of this learnable pattern.
    pub calls: u64,
    /// Sum of confidence values from each invocation (for computing average).
    pub confidence_sum: f64,
    /// Number of cache hits (responses served from few-shot or LLM cache).
    pub cache_hits: u64,
    /// Timestamp of the last adapt operation (Unix seconds), or 0 if never adapted.
    pub last_adapt: i64,
    /// Timestamp of the last invocation (Unix seconds), or 0 if never called.
    pub last_call: i64,
    /// Current count of few-shot examples added via adapt.
    pub examples_count: u64,
}

impl Default for PatternStats {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternStats {
    pub fn new() -> Self {
        PatternStats {
            calls: 0,
            confidence_sum: 0.0,
            cache_hits: 0,
            last_adapt: 0,
            last_call: 0,
            examples_count: 0,
        }
    }

    /// Average confidence across all invocations.
    pub fn avg_confidence(&self) -> f64 {
        if self.calls == 0 {
            0.0
        } else {
            self.confidence_sum / self.calls as f64
        }
    }
}

/// A single message within a conversation (ADR-0053).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConvMessage {
    /// Message role: "user", "assistant", or "system".
    pub role: String,
    /// Message text content.
    pub text: String,
    /// Unix timestamp when the message was added.
    pub timestamp: i64,
}

/// A conversation with its messages and metadata (ADR-0053).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Conversation {
    /// Conversation identifier.
    pub id: String,
    /// Ordered list of messages in this conversation.
    pub messages: Vec<ConvMessage>,
    /// Unix timestamp when the conversation was created.
    pub created_at: i64,
    /// Unix timestamp of last activity (message added/removed).
    pub last_active: i64,
    /// Additional metadata (key-value pairs).
    pub metadata: HashMap<String, String>,
}

/// Conversation configuration (ADR-0053).
/// Set by `conversation { ttl: N max_messages: N compress_after: N }`.
#[derive(Debug, Clone)]
pub struct ConversationConfig {
    /// Time-to-live in seconds. Default: 1800 (30 minutes).
    pub ttl: u64,
    /// Maximum messages per conversation. Default: 50.
    pub max_messages: usize,
    /// Compress older messages via LLM summarization after this count. Default: 20.
    pub compress_after: usize,
}

impl Default for ConversationConfig {
    fn default() -> Self {
        ConversationConfig {
            ttl: 1800,
            max_messages: 50,
            compress_after: 20,
        }
    }
}

/// A single event in the event stream (ADR-0052).
/// Represents a discrete operation that occurred during interpretation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Event {
    /// Auto-incrementing event ID.
    pub id: u64,
    /// Unix timestamp in milliseconds.
    pub timestamp: u64,
    /// Event type: "pattern_call", "llm_call", "memory_store", "memory_recall",
    /// "rule_fire", "adapt", "error", etc.
    pub event_type: String,
    /// Source: pattern name, "system", or "builtin".
    pub source: String,
    /// Arbitrary key-value data attached to the event.
    pub data: HashMap<String, String>,
    /// Duration of the operation in milliseconds, if measurable.
    pub duration_ms: Option<u64>,
}

/// Control flow signal for loop constructs (Наряд №17).
/// Break/Continue propagate through eval_statements without being confused
/// with Return values. The interpreter uses Result<ControlFlow, String>
/// internally for loop bodies, then converts back to Result<Value, String>
/// at the public eval_statements boundary.
#[derive(Debug, Clone)]
pub(crate) enum ControlFlow {
    /// Normal execution, optionally carrying a value (like implicit return).
    ContinueNormal(Value),
    /// `break` — exit the innermost loop.
    Break,
    /// `continue` — skip to next iteration of the innermost loop.
    ContinueLoop,
    /// `return expr` — early return from a pattern/function.
    Return(Value),
}

impl ControlFlow {
    pub(crate) fn is_break(&self) -> bool {
        matches!(self, ControlFlow::Break)
    }
    pub(crate) fn is_continue(&self) -> bool {
        matches!(self, ControlFlow::ContinueLoop)
    }
    pub(crate) fn is_return(&self) -> bool {
        matches!(self, ControlFlow::Return(_))
    }
    /// Extract the inner value if this is ContinueNormal or Return.
    pub(crate) fn into_value(self) -> Value {
        match self {
            ControlFlow::ContinueNormal(v) | ControlFlow::Return(v) => v,
            ControlFlow::Break | ControlFlow::ContinueLoop => Value::Unit,
        }
    }
}
