// ── Integration tests for Наряд MLG-4: Email (SMTP + IMAP) ────────────
// These tests verify function registration and argument handling.
// Live SMTP/IMAP tests require real credentials (env vars), tested manually.

use metalogos::builtins::{builtin_count, builtin_name_set, Builtins, BUILTIN_REGISTRY};
use metalogos::interpreter::Value;

/// Verify all MLG-4 email functions are registered with correct arity.
#[test]
fn test_registry_mlg4_entries_exist() {
    let names = builtin_name_set();

    // SMTP
    assert!(
        names.contains("smtp_send"),
        "smtp_send missing from registry"
    );
    assert!(
        names.contains("smtp_send_html"),
        "smtp_send_html missing from registry"
    );

    // IMAP
    assert!(
        names.contains("imap_list"),
        "imap_list missing from registry"
    );
    assert!(
        names.contains("imap_read"),
        "imap_read missing from registry"
    );
    assert!(
        names.contains("imap_search"),
        "imap_search missing from registry"
    );
    assert!(
        names.contains("imap_mark_read"),
        "imap_mark_read missing from registry"
    );
    assert!(
        names.contains("imap_move"),
        "imap_move missing from registry"
    );

    // Verify arities
    for spec in BUILTIN_REGISTRY.iter() {
        match spec.name {
            "smtp_send" => {
                assert_eq!(spec.arity, 3, "smtp_send min arity should be 3");
                assert_eq!(spec.max_arity, Some(6), "smtp_send max arity should be 6");
            }
            "smtp_send_html" => {
                assert_eq!(spec.arity, 3, "smtp_send_html min arity should be 3");
                assert_eq!(
                    spec.max_arity,
                    Some(4),
                    "smtp_send_html max arity should be 4"
                );
            }
            "imap_list" => {
                assert_eq!(spec.arity, 2, "imap_list min arity should be 2");
                assert_eq!(spec.max_arity, Some(3), "imap_list max arity should be 3");
            }
            "imap_read" => {
                assert_eq!(spec.arity, 1, "imap_read arity should be 1");
                assert_eq!(spec.max_arity, None, "imap_read max_arity should be None");
            }
            "imap_search" => {
                assert_eq!(spec.arity, 2, "imap_search arity should be 2");
                assert_eq!(spec.max_arity, None, "imap_search max_arity should be None");
            }
            "imap_mark_read" => {
                assert_eq!(spec.arity, 1, "imap_mark_read arity should be 1");
                assert_eq!(
                    spec.max_arity, None,
                    "imap_mark_read max_arity should be None"
                );
            }
            "imap_move" => {
                assert_eq!(spec.arity, 2, "imap_move arity should be 2");
                assert_eq!(spec.max_arity, None, "imap_move max_arity should be None");
            }
            _ => {}
        }
    }
}

/// Verify email functions are in the "email" category.
#[test]
fn test_mlg4_category_email() {
    for spec in BUILTIN_REGISTRY.iter() {
        if matches!(
            spec.name,
            "smtp_send"
                | "smtp_send_html"
                | "imap_list"
                | "imap_read"
                | "imap_search"
                | "imap_mark_read"
                | "imap_move"
        ) {
            assert_eq!(
                spec.category, "email",
                "function '{}' should be in 'email' category, got '{}'",
                spec.name, spec.category
            );
        }
    }
}

/// Verify total builtin count increased after MLG-4.
#[test]
fn test_mlg4_builtin_count() {
    let count = builtin_count();
    assert!(
        count >= 282,
        "expected at least 282 builtins after MLG-4, got {}",
        count
    );
}

/// Verify smtp_send returns error when SMTP env vars are not set.
#[test]
fn test_smtp_send_no_env() {
    std::env::remove_var("SMTP_HOST");
    let builtins = metalogos::builtins::Builtins::new();
    let func = builtins.get("smtp_send").expect("smtp_send not registered");

    let result = func(&[
        Value::String("test@example.com".to_string()),
        Value::String("Test Subject".to_string()),
        Value::String("Test body".to_string()),
    ]);
    assert!(result.is_err(), "smtp_send should fail without SMTP_HOST");
    assert!(
        result.unwrap_err().contains("SMTP_HOST"),
        "error should mention SMTP_HOST"
    );
}

/// Verify smtp_send_html returns error when SMTP env vars are not set.
#[test]
fn test_smtp_send_html_no_env() {
    std::env::remove_var("SMTP_HOST");
    let builtins = metalogos::builtins::Builtins::new();
    let func = builtins
        .get("smtp_send_html")
        .expect("smtp_send_html not registered");

    let result = func(&[
        Value::String("test@example.com".to_string()),
        Value::String("HTML Subject".to_string()),
        Value::String("<h1>Hello</h1>".to_string()),
    ]);
    assert!(
        result.is_err(),
        "smtp_send_html should fail without SMTP_HOST"
    );
}

/// Verify imap_list returns error when IMAP env vars are not set.
#[test]
fn test_imap_list_no_env() {
    std::env::remove_var("IMAP_HOST");
    let builtins = metalogos::builtins::Builtins::new();
    let func = builtins.get("imap_list").expect("imap_list not registered");

    let result = func(&[Value::String("INBOX".to_string()), Value::Float(10.0)]);
    assert!(result.is_err(), "imap_list should fail without IMAP_HOST");
    assert!(
        result.unwrap_err().contains("IMAP_HOST"),
        "error should mention IMAP_HOST"
    );
}

/// Verify imap_read returns error when IMAP env vars are not set.
#[test]
fn test_imap_read_no_env() {
    std::env::remove_var("IMAP_HOST");
    let builtins = metalogos::builtins::Builtins::new();
    let func = builtins.get("imap_read").expect("imap_read not registered");

    let result = func(&[Value::Float(42.0)]);
    assert!(result.is_err(), "imap_read should fail without IMAP_HOST");
}

/// Verify imap_search returns error when IMAP env vars are not set.
#[test]
fn test_imap_search_no_env() {
    std::env::remove_var("IMAP_HOST");
    let builtins = metalogos::builtins::Builtins::new();
    let func = builtins
        .get("imap_search")
        .expect("imap_search not registered");

    let result = func(&[
        Value::String("invoice".to_string()),
        Value::String("INBOX".to_string()),
    ]);
    assert!(result.is_err(), "imap_search should fail without IMAP_HOST");
}

/// Verify imap_mark_read returns error when IMAP env vars are not set.
#[test]
fn test_imap_mark_read_no_env() {
    std::env::remove_var("IMAP_HOST");
    let builtins = metalogos::builtins::Builtins::new();
    let func = builtins
        .get("imap_mark_read")
        .expect("imap_mark_read not registered");

    let result = func(&[Value::Float(1.0)]);
    assert!(
        result.is_err(),
        "imap_mark_read should fail without IMAP_HOST"
    );
}

/// Verify imap_move returns error when IMAP env vars are not set.
#[test]
fn test_imap_move_no_env() {
    std::env::remove_var("IMAP_HOST");
    let builtins = metalogos::builtins::Builtins::new();
    let func = builtins.get("imap_move").expect("imap_move not registered");

    let result = func(&[Value::Float(1.0), Value::String("Archive".to_string())]);
    assert!(result.is_err(), "imap_move should fail without IMAP_HOST");
}
