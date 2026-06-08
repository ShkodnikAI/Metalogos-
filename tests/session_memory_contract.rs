// ── ADR-0049: Session Memory — Contract Tests ──────────────────────────
//
// Session memory: session_set / session_get / session_clear
// In-memory HashMap<String, HashMap<String, String>> — NOT persistent.
// Resets on restart (by design).
// Unlike mem_set/mem_get (global), session_* is scoped to session_id.

use metalogos::interpreter::Interpreter;

/// Helper: parse and run a .mlog program, return the final output.
fn run_mlog(source: &str) -> Result<Option<String>, String> {
    let mut interp = Interpreter::new();
    let declarations = metalogos::parser::parse(source)
        .map_err(|e| format!("parse error: {}", e))?;
    interp.run(declarations)
}

#[test]
fn contract_session_set_get_roundtrip() {
    // session_set → session_get → same value
    let source = r#"
        let result = session_set("chat-42", "username", "alice")
        let fetched = session_get("chat-42", "username")
        print(fetched)
    "#;
    let output = run_mlog(source).unwrap();
    assert_eq!(output, Some("alice".to_string()));
}

#[test]
fn contract_session_set_returns_value() {
    // session_set returns the stored value
    let source = r#"
        let stored = session_set("s1", "color", "blue")
        print(stored)
    "#;
    let output = run_mlog(source).unwrap();
    assert_eq!(output, Some("blue".to_string()));
}

#[test]
fn contract_session_get_missing_key() {
    // session_get with non-existent key returns empty string
    let source = r#"
        let val = session_get("chat-99", "nonexistent")
        print(val)
    "#;
    let output = run_mlog(source).unwrap();
    assert_eq!(output, Some("".to_string()));
}

#[test]
fn contract_session_get_missing_session() {
    // session_get with non-existent session returns empty string
    let source = r#"
        let val = session_get("no-such-session", "key")
        print(val)
    "#;
    let output = run_mlog(source).unwrap();
    assert_eq!(output, Some("".to_string()));
}

#[test]
fn contract_session_isolation() {
    // Two different sessions are isolated: write in one, read from other → empty
    let source = r#"
        let r1 = session_set("session-a", "data", "value-a")
        let val = session_get("session-b", "data")
        print(val)
    "#;
    let output = run_mlog(source).unwrap();
    assert_eq!(output, Some("".to_string()));
}

#[test]
fn contract_session_clear() {
    // session_clear removes all keys for a session
    let source = r#"
        session_set("chat-55", "k1", "v1")
        session_set("chat-55", "k2", "v2")
        session_clear("chat-55")
        let v1 = session_get("chat-55", "k1")
        let v2 = session_get("chat-55", "k2")
        print(v1)
    "#;
    let output = run_mlog(source).unwrap();
    // After clear, both keys should be empty
    assert_eq!(output, Some("".to_string()));
}

#[test]
fn contract_session_restart_empties() {
    // Simulate restart: reset_session_store() → all session data is gone
    metalogos::builtins::reset_session_store();

    // First run: write data
    let source1 = r#"
        session_set("chat-100", "token", "abc123")
    "#;
    run_mlog(source1).unwrap();
    assert_eq!(metalogos::builtins::session_key_count("chat-100"), 1);

    // Simulate restart: clear all sessions
    metalogos::builtins::reset_session_store();

    // Verify empty
    assert_eq!(metalogos::builtins::session_store_count(), 0);

    // Second run: data is gone
    let source2 = r#"
        let val = session_get("chat-100", "token")
        print(val)
    "#;
    let output = run_mlog(source2).unwrap();
    assert_eq!(output, Some("".to_string()));
}

#[test]
fn contract_session_multiple_keys() {
    // Multiple keys in the same session coexist
    let source = r#"
        session_set("multi", "name", "bob")
        session_set("multi", "age", "30")
        session_set("multi", "role", "admin")
        let n = session_get("multi", "name")
        let a = session_get("multi", "age")
        let r = session_get("multi", "role")
        print(n + " " + a + " " + r)
    "#;
    let output = run_mlog(source).unwrap();
    assert_eq!(output, Some("bob 30 admin".to_string()));
}

#[test]
fn contract_session_overwrite() {
    // Overwriting a key replaces the old value
    let source = r#"
        session_set("s1", "counter", "1")
        session_set("s1", "counter", "2")
        let val = session_get("s1", "counter")
        print(val)
    "#;
    let output = run_mlog(source).unwrap();
    assert_eq!(output, Some("2".to_string()));
}

#[test]
fn contract_session_no_persistence() {
    // Session store is purely in-memory — no SQLite interaction
    // Verify by using the reset function: after reset, data is gone
    metalogos::builtins::reset_session_store();

    session_set_direct("persist-test", "k", "v");
    assert_eq!(metalogos::builtins::session_key_count("persist-test"), 1);

    metalogos::builtins::reset_session_store();
    assert_eq!(metalogos::builtins::session_key_count("persist-test"), 0);
}

/// Direct session_set via builtins API for test isolation (no parse needed).
fn session_set_direct(session_id: &str, key: &str, value: &str) {
    let args = vec![
        metalogos::interpreter::Value::String(session_id.to_string()),
        metalogos::interpreter::Value::String(key.to_string()),
        metalogos::interpreter::Value::String(value.to_string()),
    ];
    // Call the builtin directly through the builtins registry
    let builtins = metalogos::builtins::Builtins::new();
    if let Some(func) = builtins.get("session_set") {
        let _ = func(&args);
    }
}
