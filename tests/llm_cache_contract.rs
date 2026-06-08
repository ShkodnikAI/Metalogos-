// ── Наряд №3 / ADR-0047: LLM Response Caching — Contract Tests ──────
//
// Contracts:
// 1. Two identical calls to a cached learnable pattern → LLM invoked only once
// 2. Different args → cache miss → LLM invoked again
// 3. Uncached learnable pattern → every call hits LLM
// 4. cache_ttl: 0.seconds → immediate expiry → cache miss on second call
// 5. Few-shot match bypasses LLM entirely (pre-cache optimization)
// 6. MockLlm call counter is correctly reset between tests

use metalogos::llm::MockLlm;

/// Helper: reset MockLlm counter and set env for mock mode.
fn setup_mock() {
    MockLlm::reset_call_count();
    std::env::set_var("METALOGOS_MOCK_LLM", "true");
}

/// Helper: run a sequence of .mlog lines on a fresh interpreter.
/// Returns the final output (if any).
fn run_lines(lines: &[&str]) -> Result<Option<String>, String> {
    let mut interp = metalogos::interpreter::Interpreter::new();
    let mut last_output = None;
    for line in lines {
        last_output = metalogos::feed_line(&mut interp, line)?;
    }
    Ok(last_output)
}

// ── Contract 1: Two identical calls → LLM called once (cache hit) ─────

#[test]
fn test_cache_hit_two_identical_calls() {
    setup_mock();

    run_lines(&[
        // Declare a cached learnable pattern
        r#"learnable pattern Translate(text: String) -> String {
  prompt: "Translate to French."
  cache: true
  cache_ttl: 60.minutes
}"#,
        // First call — LLM invoked, response cached
        r#"let r1 = Translate("hello")"#,
        // Second call with SAME input — cache hit, no LLM call
        r#"let r2 = Translate("hello")"#,
    ]).unwrap();

    // LLM should have been called exactly ONCE (second call was cached)
    assert_eq!(
        MockLlm::call_count(), 1,
        "Two identical calls with cache:true should invoke LLM only once"
    );
}

// ── Contract 2: Different args → cache miss → LLM called again ───────

#[test]
fn test_cache_miss_different_args() {
    setup_mock();

    run_lines(&[
        r#"learnable pattern Translate(text: String) -> String {
  prompt: "Translate to French."
  cache: true
  cache_ttl: 60.minutes
}"#,
        r#"let r1 = Translate("hello")"#,
        r#"let r2 = Translate("hello")"#,  // cache hit
        r#"let r3 = Translate("goodbye")"#, // cache miss — different arg
    ]).unwrap();

    // LLM called twice: once for "hello", once for "goodbye"
    assert_eq!(
        MockLlm::call_count(), 2,
        "Different args should cause cache miss and new LLM call"
    );
}

// ── Contract 3: Uncached pattern → every call hits LLM ───────────────

#[test]
fn test_uncached_always_calls_llm() {
    setup_mock();

    run_lines(&[
        // cache defaults to false — NOT cached
        r#"learnable pattern Summarize(text: String) -> String {
  prompt: "Summarize this."
}"#,
        r#"let s1 = Summarize("same text")"#,
        r#"let s2 = Summarize("same text")"#,
    ]).unwrap();

    // Both calls hit LLM (no caching)
    assert_eq!(
        MockLlm::call_count(), 2,
        "Uncached pattern should invoke LLM on every call"
    );
}

// ── Contract 4: TTL expiry → cache miss ──────────────────────────────

#[test]
fn test_cache_ttl_expiry() {
    setup_mock();

    run_lines(&[
        // TTL = 0 seconds → entry expires immediately
        r#"learnable pattern QuickClassify(text: String) -> String {
  prompt: "Classify as: positive | negative."
  cache: true
  cache_ttl: 0.seconds
}"#,
        r#"let r1 = QuickClassify("hello")"#,
        // Even with same input, TTL=0 means entry is expired
        r#"let r2 = QuickClassify("hello")"#,
    ]).unwrap();

    // Both calls hit LLM because first entry expired immediately
    assert_eq!(
        MockLlm::call_count(), 2,
        "cache_ttl: 0.seconds should cause immediate expiry"
    );
}

// ── Contract 5: Few-shot match bypasses LLM (even with cache) ───────

#[test]
fn test_few_shot_bypasses_llm() {
    setup_mock();

    run_lines(&[
        r#"learnable pattern Greet(text: String) -> String {
  prompt: "Greet the user."
  cache: true
}"#,
        // Add a few-shot example: "Alice" → "Hello, Alice!"
        r#"adapt Greet add_example("Alice", "Hello, Alice!")"#,
        // This matches few-shot → no LLM call at all
        r#"let r1 = Greet("Alice")"#,
        // Second call — still few-shot match
        r#"let r2 = Greet("Alice")"#,
        // Different input — no few-shot match → LLM called
        r#"let r3 = Greet("Bob")"#,
    ]).unwrap();

    // Only "Bob" triggered an LLM call; "Alice" matched few-shot both times
    assert_eq!(
        MockLlm::call_count(), 1,
        "Few-shot match should bypass LLM entirely"
    );
}

// ── Contract 6: Counter reset works correctly ─────────────────────────

#[test]
fn test_counter_reset() {
    setup_mock();

    // First call increments counter
    let _ = MockLlm.call("test", "input");
    assert_eq!(MockLlm::call_count(), 1);

    // Reset
    MockLlm::reset_call_count();
    assert_eq!(MockLlm::call_count(), 0);

    // Increment again after reset
    let _ = MockLlm.call("test2", "input");
    assert_eq!(MockLlm::call_count(), 1);
}

// ── Contract 7: Cache key includes prompt (different prompt = miss) ─

#[test]
fn test_cache_key_includes_prompt() {
    setup_mock();

    run_lines(&[
        r#"learnable pattern ClassifyA(text: String) -> String {
  prompt: "Classify as A."
  cache: true
  cache_ttl: 60.minutes
}"#,
        r#"learnable pattern ClassifyB(text: String) -> String {
  prompt: "Classify as B."
  cache: true
  cache_ttl: 60.minutes
}"#,
        // Same input but different patterns (different prompts)
        r#"let r1 = ClassifyA("test")"#,
        r#"let r2 = ClassifyB("test")"#,
    ]).unwrap();

    // Two different prompts → two different cache keys → two LLM calls
    assert_eq!(
        MockLlm::call_count(), 2,
        "Different prompts should produce different cache keys"
    );
}
