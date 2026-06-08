// ── Наряд №4 / ADR-0048: Cost-Aware Model Routing — Contract Tests ──────
//
// Contracts:
// 1. Learnable pattern with model: "haiku" → MockLlm records "haiku"
// 2. Two patterns with different model overrides → different models recorded
// 3. Pattern without model override → empty last_model (global default)
// 4. Model override does not affect cache behavior (cache still works)
// 5. Model is included in cache key (same prompt, different model = cache miss)

use metalogos::llm::MockLlm;

/// Helper: reset MockLlm counters and set env for mock mode.
fn setup_mock() {
    MockLlm::reset_call_count();
    MockLlm::reset_last_model();
    std::env::set_var("METALOGOS_MOCK_LLM", "true");
}

/// Helper: run a sequence of .mlog lines on a fresh interpreter.
fn run_lines(lines: &[&str]) -> Result<Option<String>, String> {
    let mut interp = metalogos::interpreter::Interpreter::new();
    let mut last_output = None;
    for line in lines {
        last_output = metalogos::feed_line(&mut interp, line)?;
    }
    Ok(last_output)
}

// ── Contract 1: Pattern with model: "haiku" → mock records "haiku" ────

#[test]
fn test_model_override_recorded() {
    setup_mock();

    run_lines(&[
        r#"learnable pattern QuickClassify(text: String) -> String {
  prompt: "Classify as positive or negative."
  model: "haiku"
}"#,
        r#"let r = QuickClassify("great day today")"#,
    ]).unwrap();

    assert_eq!(
        MockLlm::last_model(), "haiku",
        "MockLlm should record 'haiku' as the model override"
    );
    assert_eq!(MockLlm::call_count(), 1);
}

// ── Contract 2: Two patterns with different models ────────────────────

#[test]
fn test_two_patterns_different_models() {
    setup_mock();

    run_lines(&[
        r#"learnable pattern CheapTask(text: String) -> String {
  prompt: "Simple classification."
  model: "haiku"
}"#,
        r#"learnable pattern ExpensiveTask(text: String) -> String {
  prompt: "Complex analysis."
  model: "opus"
}"#,
        // Call CheapTask → last_model should be "haiku"
        r#"let r1 = CheapTask("hello")"#,
        // Call ExpensiveTask → last_model should be "opus"
        r#"let r2 = ExpensiveTask("hello")"#,
    ]).unwrap();

    // After both calls, last_model reflects the most recent call
    assert_eq!(
        MockLlm::last_model(), "opus",
        "Last model should be 'opus' from ExpensiveTask"
    );
    // Total 2 calls (no cache — cache defaults to false)
    assert_eq!(MockLlm::call_count(), 2);
}

// ── Contract 3: Pattern without model override → empty last_model ──────

#[test]
fn test_no_model_override() {
    setup_mock();

    run_lines(&[
        r#"learnable pattern DefaultModel(text: String) -> String {
  prompt: "Summarize."
}"#,
        r#"let r = DefaultModel("some text")"#,
    ]).unwrap();

    // No model override → call_with_model receives None → last_model empty
    assert_eq!(
        MockLlm::last_model(), "",
        "No model override should leave last_model empty"
    );
}

// ── Contract 4: Model override + cache still works ────────────────────

#[test]
fn test_model_override_with_cache() {
    setup_mock();

    run_lines(&[
        r#"learnable pattern CachedTranslate(text: String) -> String {
  prompt: "Translate to French."
  model: "sonnet"
  cache: true
  cache_ttl: 60.minutes
}"#,
        // First call — LLM invoked, model recorded
        r#"let r1 = CachedTranslate("hello")"#,
        // Record model after first call
        // Second call — cache hit, no LLM call, model unchanged
        r#"let r2 = CachedTranslate("hello")"#,
    ]).unwrap();

    // LLM called only once (second hit cache)
    assert_eq!(
        MockLlm::call_count(), 1,
        "Cache should prevent second LLM call"
    );
    // Model was recorded during the first (only) LLM call
    assert_eq!(
        MockLlm::last_model(), "sonnet",
        "Model override should be 'sonnet'"
    );
}

// ── Contract 5: Model included in cache key ───────────────────────────
// Two patterns with same prompt but different models → different cache keys
// → each calls LLM separately (no cross-contamination)

#[test]
fn test_model_affects_cache_key() {
    setup_mock();

    run_lines(&[
        r#"learnable pattern VersionA(text: String) -> String {
  prompt: "Classify."
  model: "haiku"
  cache: true
  cache_ttl: 60.minutes
}"#,
        r#"learnable pattern VersionB(text: String) -> String {
  prompt: "Classify."
  model: "opus"
  cache: true
  cache_ttl: 60.minutes
}"#,
        // Same input, same prompt, but different models → different cache keys
        r#"let r1 = VersionA("test")"#,
        r#"let r2 = VersionB("test")"#,
    ]).unwrap();

    // Both calls should hit LLM (different cache keys due to model)
    assert_eq!(
        MockLlm::call_count(), 2,
        "Different models should produce different cache keys"
    );
    assert_eq!(MockLlm::last_model(), "opus");
}

// ── Contract 6: Verify model sequence across multiple calls ───────────

#[test]
fn test_model_sequence_tracking() {
    setup_mock();

    run_lines(&[
        r#"learnable pattern Fast(text: String) -> String {
  prompt: "Quick."
  model: "haiku"
}"#,
        r#"learnable pattern Slow(text: String) -> String {
  prompt: "Deep."
  model: "opus"
}"#,
        r#"learnable pattern Normal(text: String) -> String {
  prompt: "Standard."
}"#,
    ]).unwrap();

    // Call Fast → haiku
    let mut interp = metalogos::interpreter::Interpreter::new();
    metalogos::feed_line(&mut interp, r#"learnable pattern Fast(text: String) -> String {
  prompt: "Quick."
  model: "haiku"
}"#).unwrap();
    metalogos::feed_line(&mut interp, r#"let r = Fast("x")"#).unwrap();
    assert_eq!(MockLlm::last_model(), "haiku");

    // Call Slow → opus
    MockLlm::reset_last_model();
    metalogos::feed_line(&mut interp, r#"learnable pattern Slow(text: String) -> String {
  prompt: "Deep."
  model: "opus"
}"#).unwrap();
    metalogos::feed_line(&mut interp, r#"let r = Slow("x")"#).unwrap();
    assert_eq!(MockLlm::last_model(), "opus");

    // Call Normal → empty (no override)
    MockLlm::reset_last_model();
    metalogos::feed_line(&mut interp, r#"learnable pattern Normal(text: String) -> String {
  prompt: "Standard."
}"#).unwrap();
    metalogos::feed_line(&mut interp, r#"let r = Normal("x")"#).unwrap();
    assert_eq!(MockLlm::last_model(), "");
}
