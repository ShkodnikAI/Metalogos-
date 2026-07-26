// ── Contract tests for Event Stream (ADR-0052) ──────────────────────────
//
// Tests:
// 1. memorize emits memory_store event
// 2. pattern call emits pattern_call event
// 3. adapt emits adapt event
// 4. event_count() returns total events
// 5. event_count(type) returns filtered count
// 6. events_since() returns events in time window
// 7. event_sum() sums numeric fields
// 8. Event IDs are auto-incrementing
// 9. Multiple event types in single run

use metalogos::interpreter::{Event, Interpreter};
use metalogos::parser;
use std::time::SystemTime;

/// Helper: parse + run declarations, return interpreter.
fn run_source(source: &str) -> Result<Interpreter, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut interp = Interpreter::new();
    interp.run(declarations)?;
    Ok(interp)
}

// ── Test 1: memorize emits memory_store event ─────────────────────────────

#[test]
fn test_event_memorize() {
    let source = r#"
        memorize "test fact" with priority=0.8
        memorize "another fact" with priority=0.5
    "#;

    let interp = run_source(source).unwrap();
    assert_eq!(interp.event_count(Some("memory_store")), 2);

    let events = interp.get_events();
    let store_events: Vec<&Event> = events
        .iter()
        .filter(|e| e.event_type == "memory_store")
        .collect();
    assert_eq!(
        store_events[0].data.get("key_preview").unwrap(),
        "test fact"
    );
    assert_eq!(store_events[0].data.get("priority").unwrap(), "0.8");
    assert_eq!(
        store_events[1].data.get("key_preview").unwrap(),
        "another fact"
    );
}

// ── Test 2: pattern call emits pattern_call event ────────────────────────

#[test]
fn test_event_pattern_call() {
    let source = r#"
        pattern Hello(name: String) -> String {
            return "Hi " + name
        }
        entity w: String = "World"
        flow F { input: String = w -> Hello -> output }
    "#;

    let interp = run_source(source).unwrap();
    assert_eq!(interp.event_count(Some("pattern_call")), 1);

    let events = interp.get_events();
    let pc: &Event = events
        .iter()
        .find(|e| e.event_type == "pattern_call")
        .unwrap();
    assert_eq!(pc.source, "Hello");
    assert_eq!(pc.data.get("name").unwrap(), "Hello");
    assert_eq!(pc.data.get("cache_hit").unwrap(), "false");
}

// ── Test 3: adapt emits adapt event ─────────────────────────────────────

#[test]
fn test_event_adapt() {
    let source = r#"
        learnable pattern Sentiment(text: String) -> String {
            prompt: "positive"
        }
        adapt Sentiment add_example("good", "positive")
        adapt Sentiment add_example("bad", "negative")
    "#;

    let interp = run_source(source).unwrap();
    assert_eq!(interp.event_count(Some("adapt")), 2);

    let events = interp.get_events();
    let adapt_events: Vec<&Event> = events.iter().filter(|e| e.event_type == "adapt").collect();
    assert_eq!(adapt_events[0].source, "Sentiment");
    assert_eq!(adapt_events[0].data.get("action").unwrap(), "add_example");
    assert_eq!(adapt_events[1].data.get("examples_count").unwrap(), "2");
}

// ── Test 4: event_count() returns total events ────────────────────────────

#[test]
fn test_event_count_total() {
    let source = r#"
        memorize "fact1" with priority=0.5
        memorize "fact2" with priority=0.5
        pattern P(name: String) -> String { return name }
        entity x: String = "hi"
        flow F { input: String = x -> P -> output }
    "#;

    let interp = run_source(source).unwrap();
    // 2 memory_store + 1 pattern_call = 3
    let total = interp.event_count(None);
    assert!(total >= 3, "expected at least 3 events, got {}", total);
}

// ── Test 5: event_count(type) returns filtered count ─────────────────────

#[test]
fn test_event_count_filtered() {
    let source = r#"
        memorize "fact1" with priority=0.5
        memorize "fact2" with priority=0.5
        memorize "fact3" with priority=0.5
    "#;

    let interp = run_source(source).unwrap();
    assert_eq!(interp.event_count(Some("memory_store")), 3);
    assert_eq!(interp.event_count(Some("pattern_call")), 0);
    assert_eq!(interp.event_count(Some("adapt")), 0);
    assert_eq!(interp.event_count(Some("nonexistent")), 0);
}

// ── Test 6: events_since() returns events in time window ─────────────────

#[test]
fn test_events_since() {
    let source = r#"
        memorize "fact1" with priority=0.5
    "#;

    let interp = run_source(source).unwrap();
    // All events within last 60 seconds should include our events
    let events = interp.events_since_ms(
        (SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64)
            .saturating_sub(60_000),
    );
    assert!(events.len() >= 1, "expected at least 1 recent event");

    // Events since 10 years ago should include all events
    let ancient = interp.events_since_ms(0);
    assert!(ancient.len() >= 1);
}

// ── Test 7: event_sum() sums numeric fields ───────────────────────────────

#[test]
fn test_event_sum() {
    let source = r#"
        memorize "fact1" with priority=0.8
        memorize "fact2" with priority=0.5
        memorize "fact3" with priority=0.3
    "#;

    let interp = run_source(source).unwrap();
    // Sum of priorities: 0.8 + 0.5 + 0.3 = 1.6
    let sum = interp.event_sum("memory_store", "priority");
    assert!(
        (sum - 1.6).abs() < 0.001,
        "expected priority sum 1.6, got {}",
        sum
    );

    // Sum of nonexistent field = 0.0
    let zero = interp.event_sum("memory_store", "nonexistent");
    assert_eq!(zero, 0.0);
}

// ── Test 8: Event IDs are auto-incrementing ─────────────────────────────

#[test]
fn test_event_auto_increment_ids() {
    let source = r#"
        memorize "a" with priority=0.5
        memorize "b" with priority=0.5
        memorize "c" with priority=0.5
    "#;

    let interp = run_source(source).unwrap();
    let events = interp.get_events();
    assert!(events.len() >= 3);

    // IDs should be strictly increasing
    for i in 1..events.len() {
        assert!(
            events[i].id > events[i - 1].id,
            "event {} id ({}) should be > event {} id ({})",
            i,
            events[i].id,
            i - 1,
            events[i - 1].id
        );
    }
    // First ID should be >= 1 (AtomicU64 starts at 1)
    assert!(events[0].id >= 1);
}

// ── Test 9: Multiple event types in single run ───────────────────────────

#[test]
fn test_event_mixed_types() {
    let source = r#"
        learnable pattern C(text: String) -> String { prompt: "label" }
        adapt C add_example("x", "label")
        memorize "fact" with priority=0.9
        pattern P(name: String) -> String { return name }
        entity n: String = "test"
        flow F { input: String = n -> P -> output }
    "#;

    let interp = run_source(source).unwrap();
    assert!(interp.event_count(Some("adapt")) >= 1);
    assert!(interp.event_count(Some("memory_store")) >= 1);
    assert!(interp.event_count(Some("pattern_call")) >= 1);

    // Total should be sum of individual types
    let total = interp.event_count(None) as i64;
    let adapt = interp.event_count(Some("adapt")) as i64;
    let mem = interp.event_count(Some("memory_store")) as i64;
    let pc = interp.event_count(Some("pattern_call")) as i64;
    assert!(total >= adapt + mem + pc);
}
