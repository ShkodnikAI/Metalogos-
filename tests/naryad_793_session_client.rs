// ── Naryad №793 (issue #1531, Wave 64 P1 — the gateway G1) ───────────
// The CLIENT side of the protocol upgrade: the session client declares
// the CURRENT revision (`2025-11-25`) in `initialize`; an unknown
// server revision is a LOUD refusal naming BOTH versions (the modern
// -only servers are served; the legacy fallback stays — the
// negotiation matrix ADR-0132 D1, updated by №793).
//
// RED-before (форма №772): on the pre-№793 revision the client
// declared `2025-03-26` — the current-revision pin fails there (the
// echo-form fixture answers the requested version, so the declaration
// is observable through the fixture's echo: the pre-№793 client asked
// for 2025-03-26 and the handshake against a MODERN-ONLY peer would
// refuse; the pins here force the observable declaration and the
// loud mismatch).

use metalogos::mcp_session::McpSession;
use std::path::PathBuf;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn fixture_path(name: &str) -> String {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .join("tests")
        .join("fixtures")
        .join(name)
        .to_string_lossy()
        .to_string()
}

fn session(name: &str) -> McpSession {
    McpSession::new(
        name,
        "python3",
        vec![fixture_path("mcp_stateful_server.py")],
    )
    .expect("session declaration")
}

/// The current-revision pin: the handshake against the echo-form
/// fixture succeeds — the fixture answers the REQUESTED version, so a
/// success here proves the client asked `2025-11-25` (the pre-№793
/// client asked `2025-03-26`; the pin's contract is the declaration).
#[test]
fn n793_client_declares_current_revision() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::remove_var("MCP_FIXTURE_VERSION");
    let mut s = session("plain");
    s.ensure_alive()
        .expect("the handshake must succeed against a peer speaking the requested revision");
}

/// The mismatch pin: the fixture forced to answer `1999-12-31` — the
/// client refuses LOUDLY naming both versions (the client's declared
/// revision and the server's answer).
#[test]
fn n793_client_version_mismatch_loud_naming_both() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var("MCP_FIXTURE_VERSION", "1999-12-31");
    let mut s = session("mismatch");
    let err = match s.ensure_alive() {
        Err(e) => e,
        Ok(_) => panic!("an unknown server revision must refuse the handshake"),
    };
    std::env::remove_var("MCP_FIXTURE_VERSION");
    assert!(
        err.contains("[MCP_PROTOCOL_ERROR]") && err.contains("handshake"),
        "the refusal is the typed protocol error at the handshake phase: {}",
        err
    );
    assert!(
        err.contains("1999-12-31") && err.contains("2025-11-25"),
        "the refusal names BOTH versions (the server's answer and the client's declared): {}",
        err
    );
}

/// The stateful proof on the session layer itself: `tools_call_text`
/// rides ONE connection — the counter answers 0, 1 across two calls.
#[test]
fn n793_session_tools_call_state() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::remove_var("MCP_FIXTURE_VERSION");
    let mut s = session("stateful");
    let a = s
        .tools_call_text("count", serde_json::json!({}))
        .expect("the first call");
    let b = s
        .tools_call_text("count", serde_json::json!({}))
        .expect("the second call");
    assert_eq!(
        (a.trim(), b.trim()),
        ("0", "1"),
        "one session, one state: the counter advanced across the calls"
    );
}

/// The METALOGOS_MCP_UPSTREAMS grammar: `name=argv0,arg1,...;...` (the №259
/// format convention); the malformed forms refuse loudly at the declaration.
#[test]
fn n793_upstreams_spec_grammar() {
    let ok =
        metalogos::mcp_session::parse_upstreams_spec("a=python3,x.py; b_2 = /usr/bin/srv , --flag")
            .expect("valid spec");
    assert_eq!(ok.len(), 2);
    assert_eq!(ok[0].name(), "a");
    assert_eq!(ok[0].command(), "python3");
    assert_eq!(ok[1].argv(), &["--flag".to_string()]);

    for bad in ["novar", "BAD NAME=python3", "dup=python3;dup=python3", "x="] {
        let res = metalogos::mcp_session::parse_upstreams_spec(bad);
        assert!(
            res.is_err(),
            "the malformed spec `{}` must refuse, got {:?}",
            bad,
            res.ok().map(|v| v.len())
        );
    }
}
