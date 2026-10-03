//! №521 (Wave 22, dispatch gh#845; the №509 functional criterion,
//! ADR-0179 §2.5) — the conversation accumulation arc through the
//! SERVING path. The №496 образец: a REAL HTTP server
//! (`run_test_server_with_backend_…`), BOTH serve backends, one
//! sequential test fn.
//!
//! What is exercised (the naryad's scope: the accumulation arc, NOT the
//! №508 grammar — though the declaration below deliberately uses the
//! FREE field order, so the boot itself re-pins №508):
//!
//! 1. The `conversation { compress_after, ttl, max_messages }`
//!    declaration (the fields in a NON-canonical order — the №508
//!    free-field-order guarantee) is copied into every per-request
//!    context (the clone_definitions_into CONFIG copy).
//! 2. The accumulation + rotation + compression are visible in the
//!    RESPONSE behavior of one conversation lifetime driven through a
//!    POST route: six `conv_add` calls against `max_messages: 5` /
//!    `compress_after: 2`:
//!    - the ROTATION (max_messages) is the shared core (session_ops):
//!      the oldest message (m1) is evicted — visible on BOTH backends;
//!    - the COMPRESSION tail diverges BY DESIGN (the conv_add contract:
//!      "the TW passes the ADR-0053 auto-compression callback, the VM
//!      passes a no-op"): the TW replaces everything beyond
//!      `compress_after` with ONE system summary message (3 lines left:
//!      system + m5 + m6), the VM keeps the rotated window (5 lines:
//!      m2..m6). The test pins each side's TRUE posture — no semantics
//!      changed to make the two sides match (the №466 rule).
//! 3. The per-request conversation isolation (the №72 posture: "request
//!    A's dialogue must not continue in request B" — `Vm::reset_for_reuse`
//!    clears the conversations; the TW per-request context starts with an
//!    empty map, the CONFIG-only clone): a FRESH request's
//!    `conv_history` on the same id reports the conversation NOT FOUND on
//!    both backends — the honest pin that the dialogue did not leak
//!    across requests.
//!
//! The ttl field rides the declaration (the parse) but has no runtime
//! enforcement on the access path today — the test does NOT pretend a
//! ttl behavior exists (the honesty rule); see the M2 report.
//!
//! The security/session gates (the №72 session middleware, the consent
//! lane) are NOT re-tested here — their own suites pin them.

#![cfg(feature = "server")]
// The test harness (not program-influenced code) manages its temp files
// directly — the №475 fs_gate restricts PROGRAM-INFLUENCED paths.
#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;
use std::time::Duration;

fn temp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("n521-{}-{}", tag, std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

async fn post(port: u16, path: &str) -> (reqwest::StatusCode, String) {
    let resp = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{}{}", port, path))
        .json(&serde_json::json!({}))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .expect("POST should succeed");
    let status = resp.status();
    let text = resp.text().await.expect("body");
    (status, text)
}

fn source() -> String {
    // The declaration fields are in a NON-canonical order (compress_after
    // first) — the boot re-pins the №508 free-field-order guarantee.
    r#"
conversation { compress_after: 2, ttl: 3600, max_messages: 5 }
mlogserver {
  port: 0
  route "/chat" method=POST {
    conv_start("c1")
    conv_add("c1", "user", "m1")
    conv_add("c1", "user", "m2")
    conv_add("c1", "user", "m3")
    conv_add("c1", "user", "m4")
    conv_add("c1", "user", "m5")
    conv_add("c1", "user", "m6")
    let ctx = conv_context("c1")
    return ctx
  }
  route "/peek" method=POST {
    let h = conv_history("c1")
    return h
  }
}
"#
    .to_string()
}

async fn run_scenario(backend: metalogos_server::server::ServeBackend, tag: &str) {
    let dir = temp_dir(tag);
    let src = source();

    let (port, handle, _state) =
        metalogos_server::server::run_test_server_with_backend_state_in_dir(
            &src,
            backend,
            dir.clone(),
        )
        .await
        .expect("boot must start");

    // ── the accumulation arc: six adds, the context returns ──
    let (status, body) = post(port, "/chat").await;
    assert_eq!(
        status,
        reqwest::StatusCode::OK,
        "{}: the /chat route must answer 200, got {}: {}",
        tag,
        status,
        body
    );

    let lines: Vec<&str> = body
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if matches!(backend, metalogos_server::server::ServeBackend::Interpreter) {
        // The TW compression tail (ADR-0053): everything beyond
        // compress_after collapses into ONE system summary — 3 lines.
        assert_eq!(
            lines.len(),
            3,
            "{}: the TW context must hold the summary + the last 2 messages, got: {:?}",
            tag,
            lines
        );
        assert!(
            lines[0].starts_with("system:"),
            "{}: the first line must be the system summary, got: {:?}",
            tag,
            lines
        );
        assert!(
            body.contains("user: m5") && body.contains("user: m6"),
            "{}: the rotated window must keep the two newest messages, got: {:?}",
            tag,
            lines
        );
        assert!(
            !body.contains("user: m1") && !body.contains("user: m2"),
            "{}: the compression must remove the older messages, got: {:?}",
            tag,
            lines
        );
    } else {
        // The VM no-op compression tail (the documented divergence): only
        // the rotation ran — the full rotated window of 5 messages stays.
        assert_eq!(
            lines.len(),
            5,
            "{}: the VM context must hold the rotated window m2..m6, got: {:?}",
            tag,
            lines
        );
        assert!(
            body.contains("user: m2") && body.contains("user: m6"),
            "{}: the rotation must keep m2..m6, got: {:?}",
            tag,
            lines
        );
        assert!(
            !body.contains("user: m1"),
            "{}: the rotation must evict the oldest message (m1), got: {:?}",
            tag,
            lines
        );
        assert!(
            !body.contains("system:"),
            "{}: the VM lane does not compress (the no-op tail) — no summary \
             may appear, got: {:?}",
            tag,
            lines
        );
    }

    // ── the per-request isolation (the №72 posture) ──
    // A FRESH request (a fresh per-request context / a reset pooled VM)
    // must NOT see request A's conversation: the history lookup reports
    // the conversation NOT FOUND — the dialogue did not leak.
    handle.abort();
    let _ = handle.await;

    let (port2, handle2, _state2) =
        metalogos_server::server::run_test_server_with_backend_state_in_dir(
            &src,
            backend,
            dir.clone(),
        )
        .await
        .expect("boot 2 must start");

    let (status, body) = post(port2, "/peek").await;
    assert_ne!(
        status,
        reqwest::StatusCode::OK,
        "{}: a fresh request must NOT see the previous request's conversation \
         (the №72 isolation posture), got 200: {}",
        tag,
        body
    );
    assert!(
        body.contains("not found"),
        "{}: the fresh-request lookup must report the missing conversation, \
         got {}: {}",
        tag,
        status,
        body
    );

    handle2.abort();
    let _ = handle2.await;
    let _ = std::fs::remove_dir_all(&dir);
}

/// ONE test fn, BOTH backends sequentially (the №496 pattern): the
/// METALOGOS_MOCK_LLM env is process-global (№454) — the TW compression
/// tail uses the LLM summarizer, so parallel fns would race on it.
#[tokio::test]
async fn n521_conversation_serve_e2e_both_backends() {
    std::env::set_var("METALOGOS_MOCK_LLM", "1");
    // The TW interpreter lane first, then the VM (the serve default).
    run_scenario(metalogos_server::server::ServeBackend::Interpreter, "tw").await;
    run_scenario(metalogos_server::server::ServeBackend::Vm, "vm").await;
}
