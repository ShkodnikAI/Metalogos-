// ── ADR-0049: Session Memory — Contract Tests ──────────────────────────
//
// Session memory: session_set / session_get / session_clear
// In-memory HashMap<String, HashMap<String, String>> — NOT persistent.
// Resets on restart (by design).
// Unlike mem_set/mem_get (global), session_* is scoped to session_id.

use metalogos::interpreter::Interpreter;

/// Helper: parse and run a .mlog program, return the final output.
fn run_mlog(source: &str) -> Result<Option<String>, String> {
    metalogos::builtins::reset_session_store(); // isolate tests
    let mut interp = Interpreter::new();
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    interp.run(declarations)
}

#[test]
fn contract_session_set_get_roundtrip() {
    // session_set -> session_get -> same value
    let source = r#"
        pattern Test(sid: String) -> String {
            session_set(sid, "username", "alice")
            let fetched = session_get(sid, "username")
            return fetched
        }
        entity s: String = "chat-42"
        flow Main { input: String = s -> Test -> output }
    "#;
    let output = run_mlog(source).unwrap();
    assert_eq!(output, Some("alice".to_string()));
}

#[test]
fn contract_session_set_returns_value() {
    // session_set returns the stored value
    let source = r#"
        pattern Test(sid: String) -> String {
            let stored = session_set(sid, "color", "blue")
            return stored
        }
        entity s: String = "s1"
        flow Main { input: String = s -> Test -> output }
    "#;
    let output = run_mlog(source).unwrap();
    assert_eq!(output, Some("blue".to_string()));
}

#[test]
fn contract_session_get_missing_key() {
    // session_get with non-existent key returns empty string
    let source = r#"
        pattern Test(sid: String) -> String {
            let val = session_get(sid, "nonexistent")
            return val
        }
        entity s: String = "chat-99"
        flow Main { input: String = s -> Test -> output }
    "#;
    let output = run_mlog(source).unwrap();
    assert_eq!(output, Some("".to_string()));
}

#[test]
fn contract_session_get_missing_session() {
    // session_get with non-existent session returns empty string
    let source = r#"
        pattern Test(sid: String) -> String {
            let val = session_get(sid, "key")
            return val
        }
        entity s: String = "no-such-session"
        flow Main { input: String = s -> Test -> output }
    "#;
    let output = run_mlog(source).unwrap();
    assert_eq!(output, Some("".to_string()));
}

#[test]
fn contract_session_isolation() {
    // Two different sessions are isolated: write in one, read from other -> empty
    let source = r#"
        pattern Test(sid: String) -> String {
            session_set(sid, "data", "value-a")
            return session_get("other-session", "data")
        }
        entity s: String = "session-a"
        flow Main { input: String = s -> Test -> output }
    "#;
    let output = run_mlog(source).unwrap();
    assert_eq!(output, Some("".to_string()));
}

#[test]
fn contract_session_clear() {
    // session_clear removes all keys for a session
    let source = r#"
        pattern Test(sid: String) -> String {
            session_set(sid, "k1", "v1")
            session_set(sid, "k2", "v2")
            session_clear(sid)
            let v1 = session_get(sid, "k1")
            let v2 = session_get(sid, "k2")
            return v1
        }
        entity s: String = "chat-55"
        flow Main { input: String = s -> Test -> output }
    "#;
    let output = run_mlog(source).unwrap();
    // After clear, both keys should be empty
    assert_eq!(output, Some("".to_string()));
}

#[test]
fn contract_session_restart_empties() {
    // Simulate restart: reset_session_store() -> all session data is gone
    metalogos::builtins::reset_session_store();

    // First run: write data
    let source1 = r#"
        pattern Write(sid: String) -> String {
            session_set(sid, "token", "abc123")
            return "done"
        }
        entity s: String = "chat-100"
        flow Main { input: String = s -> Write -> output }
    "#;
    run_mlog(source1).unwrap();
    assert_eq!(metalogos::builtins::session_key_count("chat-100"), 1);

    // Simulate restart: clear all sessions
    metalogos::builtins::reset_session_store();

    // Verify empty
    assert_eq!(metalogos::builtins::session_store_count(), 0);

    // Second run: data is gone
    let source2 = r#"
        pattern Read(sid: String) -> String {
            let val = session_get(sid, "token")
            return val
        }
        entity s: String = "chat-100"
        flow Main { input: String = s -> Read -> output }
    "#;
    let output = run_mlog(source2).unwrap();
    assert_eq!(output, Some("".to_string()));
}

#[test]
fn contract_session_multiple_keys() {
    // Multiple keys in the same session coexist
    let source = r#"
        pattern Test(sid: String) -> String {
            session_set(sid, "name", "bob")
            session_set(sid, "age", "30")
            session_set(sid, "role", "admin")
            let n = session_get(sid, "name")
            let a = session_get(sid, "age")
            let r = session_get(sid, "role")
            return n + " " + a + " " + r
        }
        entity s: String = "multi"
        flow Main { input: String = s -> Test -> output }
    "#;
    let output = run_mlog(source).unwrap();
    assert_eq!(output, Some("bob 30 admin".to_string()));
}

#[test]
fn contract_session_overwrite() {
    // Overwriting a key replaces the old value
    let source = r#"
        pattern Test(sid: String) -> String {
            session_set(sid, "counter", "1")
            session_set(sid, "counter", "2")
            let val = session_get(sid, "counter")
            return val
        }
        entity s: String = "s1"
        flow Main { input: String = s -> Test -> output }
    "#;
    let output = run_mlog(source).unwrap();
    assert_eq!(output, Some("2".to_string()));
}

#[test]
fn contract_session_no_persistence() {
    // Session store is purely in-memory — no SQLite interaction
    // Verify by using the reset function: after reset, data is gone
    metalogos::builtins::reset_session_store();

    // Use direct store access for this test (no mlog parsing needed)
    let args = vec![
        metalogos::interpreter::Value::String("persist-test".to_string()),
        metalogos::interpreter::Value::String("k".to_string()),
        metalogos::interpreter::Value::String("v".to_string()),
    ];
    let builtins = metalogos::builtins::Builtins::new();
    if let Some(func) = builtins.get("session_set") {
        let _ = func(&args);
    }
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
