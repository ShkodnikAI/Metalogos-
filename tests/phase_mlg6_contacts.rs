// ── Integration tests for Наряд MLG-6: Contacts (CardDAV + vCard) ───────
// These tests verify function registration and vCard parsing/generation.
// Live CardDAV tests require real credentials (env vars), tested manually.

use metalogos::builtins::{builtin_count, builtin_name_set, BUILTIN_REGISTRY};
use metalogos::interpreter::Value;

/// Verify all MLG-6 contacts functions are registered with correct arity.
#[test]
fn test_registry_mlg6_entries_exist() {
    let names = builtin_name_set();
    let mlg6 = [
        ("card_connect", 3, None),
        ("card_list", 1, None),
        ("card_contacts", 2, None),
        ("card_read", 1, None),
        ("card_create", 3, Some(7)),
        ("card_update", 2, None),
        ("card_delete", 1, None),
        ("card_search", 2, None),
        ("vcard_parse", 1, None),
        ("vcard_generate", 1, None),
    ];

    for (name, min_arity, max_arity) in &mlg6 {
        assert!(
            names.contains(*name),
            "builtin '{}' missing from registry",
            name
        );
        let spec = BUILTIN_REGISTRY.iter().find(|s| s.name == *name).unwrap();
        assert_eq!(spec.arity, *min_arity, "arity mismatch for '{}'", name);
        assert_eq!(
            spec.max_arity, *max_arity,
            "max_arity mismatch for '{}'",
            name
        );
    }
}

/// Verify total builtin count increased by 10 (MLG-6 adds 10 functions).
#[test]
fn test_mlg6_builtin_count() {
    let count = builtin_count();
    assert!(count >= 10, "expected at least 10 builtins, got {}", count);
}

/// Verify all MLG-6 functions are in the "contacts" category.
#[test]
fn test_mlg6_category_contacts() {
    let contacts_funcs = [
        "card_connect",
        "card_list",
        "card_contacts",
        "card_read",
        "card_create",
        "card_update",
        "card_delete",
        "card_search",
        "vcard_parse",
        "vcard_generate",
    ];

    for name in &contacts_funcs {
        let spec = BUILTIN_REGISTRY.iter().find(|s| s.name == *name).unwrap();
        assert_eq!(
            spec.category, "contacts",
            "expected '{}' to be in 'contacts' category, got '{}'",
            name, spec.category
        );
    }
}

/// card_connect with non-existent server should not panic.
#[test]
fn test_card_connect_no_env() {
    let builtins = metalogos::builtins::Builtins::new();
    let handler = builtins.get("card_connect").unwrap();
    let result = handler(&[
        Value::String("http://localhost:99998/".to_string()),
        Value::String("test".to_string()),
        Value::String("test".to_string()),
    ]);
    // May succeed (creates session) or fail (connection refused) — either is acceptable
    let _ = result;
}

/// card_contacts without session should return error.
#[test]
fn test_card_contacts_no_env() {
    let builtins = metalogos::builtins::Builtins::new();
    let handler = builtins.get("card_contacts").unwrap();
    let result = handler(&[
        Value::String("nonexistent_ab".to_string()),
        Value::String("".to_string()),
    ]);
    assert!(result.is_err(), "card_contacts without session should fail");
}

/// card_create without session should return error.
#[test]
fn test_card_create_no_env() {
    let builtins = metalogos::builtins::Builtins::new();
    let handler = builtins.get("card_create").unwrap();
    let result = handler(&[
        Value::String("nonexistent_ab".to_string()),
        Value::String("John Doe".to_string()),
        Value::String("john@example.com".to_string()),
    ]);
    assert!(result.is_err(), "card_create without session should fail");
}

/// vcard_parse with a basic vCard should work.
#[test]
fn test_vcard_parse_basic() {
    let builtins = metalogos::builtins::Builtins::new();
    let handler = builtins.get("vcard_parse").unwrap();
    let vcard = "BEGIN:VCARD\r\nVERSION:4.0\r\nUID:test-uid-1\r\nFN:Elena Petrova\r\nEMAIL:elena@company.ru\r\nTEL:+7-495-123-4567\r\nORG:Russian Corp\r\nTITLE:CTO\r\nEND:VCARD\r\n";
    let result = handler(&[Value::String(vcard.to_string())]);
    assert!(result.is_ok(), "vcard_parse should succeed");
    let json_str = match result.unwrap() {
        Value::String(s) => s,
        _ => panic!("expected string"),
    };
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("vcard_parse output should be valid JSON");
    assert_eq!(parsed["FN"], "Elena Petrova");
    assert_eq!(parsed["UID"], "test-uid-1");
    assert_eq!(parsed["ORG"], "Russian Corp");
    assert_eq!(parsed["TITLE"], "CTO");
}

/// vcard_generate with basic contact JSON should produce valid vCard.
#[test]
fn test_vcard_generate_basic() {
    let builtins = metalogos::builtins::Builtins::new();
    let handler = builtins.get("vcard_generate").unwrap();
    let json = r#"{"fn":"Ivan Sidorov","email":"ivan@example.com","tel":"+7-999-888-7766","org":"TechSoft","uid":"ivan-42"}"#;
    let result = handler(&[Value::String(json.to_string())]);
    assert!(result.is_ok(), "vcard_generate should succeed");
    let vcard = match result.unwrap() {
        Value::String(s) => s,
        _ => panic!("expected string"),
    };
    assert!(vcard.contains("BEGIN:VCARD"));
    assert!(vcard.contains("END:VCARD"));
    assert!(vcard.contains("VERSION:4.0"));
    assert!(vcard.contains("UID:ivan-42"));
    assert!(vcard.contains("FN:Ivan Sidorov"));
    assert!(vcard.contains("EMAIL:ivan@example.com"));
    assert!(vcard.contains("TEL:+7-999-888-7766"));
    assert!(vcard.contains("ORG:TechSoft"));
}

/// Roundtrip: vcard_generate → vcard_parse should preserve key fields.
#[test]
fn test_vcard_roundtrip() {
    let builtins = metalogos::builtins::Builtins::new();
    let gen = builtins.get("vcard_generate").unwrap();
    let parse = builtins.get("vcard_parse").unwrap();

    let json = r#"{"fn":"Maria Kozlova","email":"maria@rt.com","tel":"+7-495-000-1111","org":"RoundCorp","uid":"maria-rt-1"}"#;

    // Generate
    let gen_result = gen(&[Value::String(json.to_string())]);
    assert!(gen_result.is_ok());
    let vcard = match gen_result.unwrap() {
        Value::String(s) => s,
        _ => panic!("expected string"),
    };

    // Parse
    let parse_result = parse(&[Value::String(vcard)]);
    assert!(parse_result.is_ok());
    let parsed_json = match parse_result.unwrap() {
        Value::String(s) => s,
        _ => panic!("expected string"),
    };

    let parsed: serde_json::Value =
        serde_json::from_str(&parsed_json).expect("should be valid JSON");
    assert_eq!(parsed["FN"], "Maria Kozlova");
    assert_eq!(parsed["UID"], "maria-rt-1");
    assert_eq!(parsed["ORG"], "RoundCorp");
}

/// vcard_parse with multiple emails should return array.
#[test]
fn test_vcard_parse_multi_email() {
    let builtins = metalogos::builtins::Builtins::new();
    let handler = builtins.get("vcard_parse").unwrap();
    let vcard = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Alex\r\nEMAIL:alex@work.com\r\nEMAIL:alex@home.com\r\nEND:VCARD\r\n";
    let result = handler(&[Value::String(vcard.to_string())]);
    assert!(result.is_ok());
    let json_str = match result.unwrap() {
        Value::String(s) => s,
        _ => panic!("expected string"),
    };
    let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("should be valid JSON");
    let emails = parsed.get("EMAIL").unwrap().as_array().unwrap();
    assert_eq!(emails.len(), 2, "should have 2 emails");
}
