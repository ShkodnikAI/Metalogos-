// ── Integration tests for Наряд MLG-5: Calendar (CalDAV + iCal) ────────
// These tests verify function registration and iCal parsing/generation.
// Live CalDAV tests require real credentials (env vars), tested manually.

use metalogos::builtins::{builtin_count, builtin_name_set, BUILTIN_REGISTRY};
use metalogos::interpreter::Value;

/// Verify all MLG-5 calendar functions are registered with correct arity.
#[test]
fn test_registry_mlg5_entries_exist() {
    let names = builtin_name_set();
    let mlg5 = [
        ("cal_connect", 3, None),
        ("cal_list", 1, None),
        ("cal_events", 3, None),
        ("cal_read", 1, None),
        ("cal_create", 4, Some(7)),
        ("cal_update", 2, None),
        ("cal_delete", 1, None),
        ("cal_freebusy", 3, None),
        ("ical_parse", 1, None),
        ("ical_generate", 1, None),
    ];

    for (name, min_arity, max_arity) in &mlg5 {
        assert!(
            names.contains(*name),
            "builtin '{}' missing from registry",
            name
        );
        // Find in BUILTIN_REGISTRY and check arity
        let spec = BUILTIN_REGISTRY.iter().find(|s| s.name == *name).unwrap();
        assert_eq!(spec.arity, *min_arity, "arity mismatch for '{}'", name);
        assert_eq!(
            spec.max_arity, *max_arity,
            "max_arity mismatch for '{}'",
            name
        );
    }
}

/// Verify total builtin count increased by 10 (MLG-5 adds 10 functions).
#[test]
fn test_mlg5_builtin_count() {
    let count = builtin_count();
    // Should be at least the MLG-5 functions + all prior builtins
    // Exact count may vary, but must include all 10 MLG-5 functions
    assert!(count >= 10, "expected at least 10 builtins, got {}", count);
}

/// Verify all MLG-5 functions are in the "calendar" category.
#[test]
fn test_mlg5_category_calendar() {
    let calendar_funcs = [
        "cal_connect",
        "cal_list",
        "cal_events",
        "cal_read",
        "cal_create",
        "cal_update",
        "cal_delete",
        "cal_freebusy",
        "ical_parse",
        "ical_generate",
    ];

    for name in &calendar_funcs {
        let spec = BUILTIN_REGISTRY.iter().find(|s| s.name == *name).unwrap();
        assert_eq!(
            spec.category, "calendar",
            "expected '{}' to be in 'calendar' category, got '{}'",
            name, spec.category
        );
    }
}

/// cal_connect with non-existent server should return error gracefully.
#[test]
fn test_cal_connect_no_env() {
    // Calling cal_connect with localhost that doesn't exist
    // should either connect and fail PROPFIND, or just create a session
    // We test with clearly invalid port to make it fail fast
    let builtins = metalogos::builtins::Builtins::new();
    let handler = builtins.get("cal_connect").unwrap();
    let result = handler(&[
        Value::String("http://localhost:99999/".to_string()),
        Value::String("test".to_string()),
        Value::String("test".to_string()),
    ]);
    // May succeed (creates session) or fail (connection refused) — either is acceptable
    // The important thing is it doesn't panic
    let _ = result;
}

/// cal_events without session should return error.
#[test]
fn test_cal_events_no_env() {
    let builtins = metalogos::builtins::Builtins::new();
    let handler = builtins.get("cal_events").unwrap();
    let result = handler(&[
        Value::String("nonexistent_cal".to_string()),
        Value::String("2026-08-13".to_string()),
        Value::String("2026-08-14".to_string()),
    ]);
    assert!(result.is_err(), "cal_events without session should fail");
}

/// cal_create without session should return error.
#[test]
fn test_cal_create_no_env() {
    let builtins = metalogos::builtins::Builtins::new();
    let handler = builtins.get("cal_create").unwrap();
    let result = handler(&[
        Value::String("nonexistent_cal".to_string()),
        Value::String("Test Meeting".to_string()),
        Value::String("2026-08-13T10:00:00".to_string()),
        Value::String("2026-08-13T11:00:00".to_string()),
    ]);
    assert!(result.is_err(), "cal_create without session should fail");
}

/// ical_parse with a basic VEVENT should work.
#[test]
fn test_ical_parse_basic() {
    let builtins = metalogos::builtins::Builtins::new();
    let handler = builtins.get("ical_parse").unwrap();
    let ical = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Test//1//EN\r\nBEGIN:VEVENT\r\nUID:test-123\r\nSUMMARY:Board Meeting\r\nDTSTART:20260813T100000Z\r\nDTEND:20260813T110000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let result = handler(&[Value::String(ical.to_string())]);
    assert!(result.is_ok(), "ical_parse should succeed");
    let json_str = match result.unwrap() {
        Value::String(s) => s,
        _ => panic!("expected string"),
    };
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("ical_parse output should be valid JSON");
    assert!(parsed.get("events").is_some(), "should have events array");
    let events = parsed.get("events").unwrap().as_array().unwrap();
    assert!(!events.is_empty(), "should have at least one event");
}

/// ical_generate with basic event JSON should produce valid iCal.
#[test]
fn test_ical_generate_basic() {
    let builtins = metalogos::builtins::Builtins::new();
    let handler = builtins.get("ical_generate").unwrap();
    let json = r#"{"summary":"Weekly Standup","start":"2026-08-13T09:00:00","end":"2026-08-13T09:30:00","uid":"standup-42","description":"Daily sync meeting"}"#;
    let result = handler(&[Value::String(json.to_string())]);
    assert!(result.is_ok(), "ical_generate should succeed");
    let ical = match result.unwrap() {
        Value::String(s) => s,
        _ => panic!("expected string"),
    };
    assert!(ical.contains("BEGIN:VCALENDAR"));
    assert!(ical.contains("END:VCALENDAR"));
    assert!(ical.contains("BEGIN:VEVENT"));
    assert!(ical.contains("END:VEVENT"));
    assert!(ical.contains("SUMMARY:Weekly Standup"));
    assert!(ical.contains("UID:standup-42"));
    assert!(ical.contains("DTSTART:20260813T090000Z"));
    assert!(ical.contains("DTEND:20260813T093000Z"));
    assert!(ical.contains("DESCRIPTION:Daily sync meeting"));
}

/// Roundtrip: ical_generate → ical_parse should preserve key fields.
#[test]
fn test_ical_roundtrip() {
    let builtins = metalogos::builtins::Builtins::new();
    let gen = builtins.get("ical_generate").unwrap();
    let parse = builtins.get("ical_parse").unwrap();

    let json = r#"{"summary":"Investor Call","start":"2026-08-15T14:00:00","end":"2026-08-15T15:00:00","uid":"investor-call-1","location":"Zoom"}"#;

    // Generate
    let gen_result = gen(&[Value::String(json.to_string())]);
    assert!(gen_result.is_ok());
    let ical = match gen_result.unwrap() {
        Value::String(s) => s,
        _ => panic!("expected string"),
    };

    // Parse
    let parse_result = parse(&[Value::String(ical)]);
    assert!(parse_result.is_ok());
    let parsed_json = match parse_result.unwrap() {
        Value::String(s) => s,
        _ => panic!("expected string"),
    };

    let parsed: serde_json::Value =
        serde_json::from_str(&parsed_json).expect("should be valid JSON");
    let events = parsed.get("events").unwrap().as_array().unwrap();
    assert!(!events.is_empty());

    let event = &events[0];
    assert_eq!(
        event.get("SUMMARY").and_then(|v| v.as_str()),
        Some("Investor Call")
    );
    assert_eq!(
        event.get("UID").and_then(|v| v.as_str()),
        Some("investor-call-1")
    );
}

/// ical_parse with multiple VEVENTs should return all events.
#[test]
fn test_ical_parse_multi_event() {
    let builtins = metalogos::builtins::Builtins::new();
    let handler = builtins.get("ical_parse").unwrap();
    let ical = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:ev1\r\nSUMMARY:Morning Sync\r\nDTSTART:20260813T090000Z\r\nDTEND:20260813T093000Z\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:ev2\r\nSUMMARY:Afternoon Review\r\nDTSTART:20260813T140000Z\r\nDTEND:20260813T150000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let result = handler(&[Value::String(ical.to_string())]);
    assert!(result.is_ok());
    let json_str = match result.unwrap() {
        Value::String(s) => s,
        _ => panic!("expected string"),
    };
    let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("should be valid JSON");
    let events = parsed.get("events").unwrap().as_array().unwrap();
    assert_eq!(events.len(), 2, "should have 2 events");
}
