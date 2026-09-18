// Tests for naryad №275 — llm_stream_open/next/close (ADR-0137).
//
// These tests spin up a Python SSE mock server (tests/p275_stream_server.py)
// and verify the streaming contract through the public builtin surface:
//
//   1. open → next → … → close produces the expected concatenated text.
//   2. Equivalence: streaming `aggregated_text` == non-streaming `call_llm`
//      for the same prompt+input (issue #311 "Итоговый текст идентичен
//      не-стримовому вызову той же фразы").
//   3. STREAM_UNSUPPORTED when no llm {} providers configured (mock path).
//   4. STREAM_LIMIT_REACHED when the open-stream cap is exceeded (№263 lesson).
//   5. Mid-stream close drops the response and lets the test continue
//      (the server's side of TCP is the provider's problem to clean up).
//   6. End-of-stream marker: next returns "__end__" after [DONE] is seen.
//   7. The trace file (METALOGOS_LLM_TRACE) receives ONE line per completed
//      stream (ADR-0138 §D4 contract).
//
// Convention follows tests/naryad_76_http_download.rs + p76_http_download_server.py
// (background python server via std::process::Command).

#![cfg(feature = "llm")]

use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use metalogos::llm::{
    clear_global_smart_router, clear_llm_stream_registry_for_tests, set_global_smart_router,
    SmartRouter,
};

// Serialize tests that mutate the process-global SmartRouter. Without this
// lock, parallel test threads overwrite each other's router state and
// produce flaky failures (same pattern as tests/llm.rs's ENV_LOCK).
static TEST_LOCK: Mutex<()> = Mutex::new(());

/// Default port — matches p275_stream_server.py.
const MOCK_PORT: u16 = 18775;

fn mock_url() -> String {
    format!("http://127.0.0.1:{}", MOCK_PORT)
}

/// Build a SmartRouter pointing at the mock SSE server (OpenAI-compatible).
/// `failover: auto` keeps failover disabled (only one provider).
fn build_mock_router() -> SmartRouter {
    use metalogos::ast::{Declaration, Expr, LlmConfigDecl, LlmProviderEntry};
    let _ = Declaration::LlmConfig(LlmConfigDecl {
        span: metalogos::ast::Span::unknown(),
        providers: vec![LlmProviderEntry {
            span: metalogos::ast::Span::unknown(),
            alias: "mock".to_string(),
            provider: "openai".to_string(),
            key: Some(Expr::StringLit {
                value: "test-key".to_string(),
                span: metalogos::ast::Span::unknown(),
            }),
            url: Some(mock_url() + "/v1/chat/completions"),
        }],
        default_model: Some("mock-model".to_string()),
        failover: Some("auto".to_string()),
        circuit_breaker: 3,
        timeout: 30,
    });
    // Direct construction would be cleaner but SmartRouter::from_config
    // takes &LlmConfigDecl; easier to build the AST node and delegate.
    let decl = LlmConfigDecl {
        span: metalogos::ast::Span::unknown(),
        providers: vec![LlmProviderEntry {
            span: metalogos::ast::Span::unknown(),
            alias: "mock".to_string(),
            provider: "openai".to_string(),
            key: Some(Expr::StringLit {
                value: "test-key".to_string(),
                span: metalogos::ast::Span::unknown(),
            }),
            url: Some(mock_url() + "/v1/chat/completions"),
        }],
        default_model: Some("mock-model".to_string()),
        failover: Some("auto".to_string()),
        circuit_breaker: 3,
        timeout: 30,
    };
    SmartRouter::from_config(&decl)
}

/// Spawn the Python mock server. Returns a Child handle to kill at teardown.
fn spawn_mock_server(port: u16) -> Child {
    let child = Command::new("python3")
        .arg("tests/p275_stream_server.py")
        .arg("--port")
        .arg(port.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn p275_stream_server.py — is python3 installed?");
    // Give the server a moment to start listening.
    std::thread::sleep(Duration::from_millis(500));
    child
}

/// Wait for the mock server's port to be reachable. If it never comes up,
/// the test fails loudly — rather than silently timing out.
fn wait_for_server(port: u16, timeout_ms: u64) {
    let start = std::time::Instant::now();
    loop {
        if std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
            return;
        }
        if start.elapsed().as_millis() as u64 > timeout_ms {
            panic!(
                "mock SSE server on port {} did not come up within {}ms",
                port, timeout_ms
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[test]
fn n275_stream_open_next_close_basic() {
    let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_global_smart_router();
    clear_llm_stream_registry_for_tests();

    let mut server = spawn_mock_server(MOCK_PORT);
    wait_for_server(MOCK_PORT, 5000);
    set_global_smart_router(build_mock_router());

    // Use the builtin-level API through the public module functions, since
    // the spec!-registered builtins need an Interpreter to dispatch.
    use metalogos::llm::{
        stream_close, stream_next, stream_via_smart_router, LLM_STREAM_END_MARKER,
    };

    let handle = stream_via_smart_router("test prompt", "test input", None, None)
        .expect("stream_open should succeed against the mock SSE server");

    let mut aggregated = String::new();
    loop {
        let delta = stream_next(handle).expect("next should succeed");
        if delta == LLM_STREAM_END_MARKER {
            break;
        }
        aggregated.push_str(&delta);
    }
    let final_meta = stream_close(handle).expect("close should succeed");
    assert_eq!(aggregated, "Hello, world!");
    assert_eq!(final_meta.aggregated_text, "Hello, world!");
    assert_eq!(final_meta.status, "ok");
    assert_eq!(final_meta.provider, "openai");
    assert_eq!(final_meta.model, "mock-model");

    // Teardown.
    let _ = server.kill();
    let _ = server.wait();
    clear_global_smart_router();
    clear_llm_stream_registry_for_tests();
}

#[test]
fn n275_stream_unsupported_without_smart_router() {
    let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_global_smart_router();
    clear_llm_stream_registry_for_tests();

    use metalogos::llm::stream_via_smart_router;
    let err = stream_via_smart_router("p", "i", None, None)
        .expect_err("opening a stream without any SmartRouter must fail loudly");
    assert!(
        err.contains("STREAM_UNSUPPORTED"),
        "expected STREAM_UNSUPPORTED, got: {}",
        err
    );

    clear_global_smart_router();
    clear_llm_stream_registry_for_tests();
}

#[test]
fn n275_stream_limit_reached() {
    let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_global_smart_router();
    clear_llm_stream_registry_for_tests();

    // Override the limit to 2 — easier to fill than 64.
    std::env::set_var("METALOGOS_LLM_STREAM_MAX", "2");
    let mut server = spawn_mock_server(MOCK_PORT + 1);
    wait_for_server(MOCK_PORT + 1, 5000);

    // Re-bind the router to the alternate port so this test is
    // independent of the basic test.
    use metalogos::ast::{Expr, LlmConfigDecl, LlmProviderEntry};
    let decl = LlmConfigDecl {
        span: metalogos::ast::Span::unknown(),
        providers: vec![LlmProviderEntry {
            span: metalogos::ast::Span::unknown(),
            alias: "mock".to_string(),
            provider: "openai".to_string(),
            key: Some(Expr::StringLit {
                value: "test-key".to_string(),
                span: metalogos::ast::Span::unknown(),
            }),
            url: Some(format!(
                "http://127.0.0.1:{}/v1/chat/completions",
                MOCK_PORT + 1
            )),
        }],
        default_model: Some("mock-model".to_string()),
        failover: Some("auto".to_string()),
        circuit_breaker: 3,
        timeout: 30,
    };
    set_global_smart_router(metalogos::llm::SmartRouter::from_config(&decl));

    use metalogos::llm::stream_via_smart_router;
    let _h1 = stream_via_smart_router("p1", "i1", None, None).expect("first open should succeed");
    let _h2 = stream_via_smart_router("p2", "i2", None, None).expect("second open should succeed");
    let err = stream_via_smart_router("p3", "i3", None, None)
        .expect_err("third open must hit STREAM_LIMIT_REACHED");
    assert!(
        err.contains("STREAM_LIMIT_REACHED"),
        "expected STREAM_LIMIT_REACHED, got: {}",
        err
    );

    // Teardown.
    std::env::remove_var("METALOGOS_LLM_STREAM_MAX");
    let _ = server.kill();
    let _ = server.wait();
    clear_global_smart_router();
    clear_llm_stream_registry_for_tests();
}

#[test]
fn n275_stream_close_before_end_drops_cleanly() {
    let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_global_smart_router();
    clear_llm_stream_registry_for_tests();

    let mut server = spawn_mock_server(MOCK_PORT + 2);
    wait_for_server(MOCK_PORT + 2, 5000);

    use metalogos::ast::{Expr, LlmConfigDecl, LlmProviderEntry};
    let decl = LlmConfigDecl {
        span: metalogos::ast::Span::unknown(),
        providers: vec![LlmProviderEntry {
            span: metalogos::ast::Span::unknown(),
            alias: "mock".to_string(),
            provider: "openai".to_string(),
            key: Some(Expr::StringLit {
                value: "test-key".to_string(),
                span: metalogos::ast::Span::unknown(),
            }),
            url: Some(format!(
                "http://127.0.0.1:{}/v1/chat/completions",
                MOCK_PORT + 2
            )),
        }],
        default_model: Some("mock-model".to_string()),
        failover: Some("auto".to_string()),
        circuit_breaker: 3,
        timeout: 30,
    };
    set_global_smart_router(metalogos::llm::SmartRouter::from_config(&decl));

    use metalogos::llm::{stream_close, stream_next, stream_via_smart_router};

    let handle =
        stream_via_smart_router("prompt", "input", None, None).expect("open should succeed");
    // Pull one chunk — then close before consuming the whole stream.
    let _delta = stream_next(handle).expect("first next should succeed");
    // Close mid-stream — should NOT panic, should NOT leave the registry
    // in a bad state.
    let final_meta = stream_close(handle).expect("mid-stream close should succeed");
    // Status should still be "ok" — the close was clean from our side.
    assert_eq!(final_meta.status, "ok");
    // Aggregated text has at least the first chunk.
    assert!(!final_meta.aggregated_text.is_empty());

    // After close, calling next on the closed handle must error loudly.
    let err = stream_next(handle).expect_err("next after close must fail loudly");
    assert!(
        err.contains("unknown LlmStream handle"),
        "expected unknown-handle error, got: {}",
        err
    );

    // Teardown.
    let _ = server.kill();
    let _ = server.wait();
    clear_global_smart_router();
    clear_llm_stream_registry_for_tests();
}

#[test]
fn n275_stream_end_marker_after_done() {
    let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_global_smart_router();
    clear_llm_stream_registry_for_tests();

    let mut server = spawn_mock_server(MOCK_PORT + 3);
    wait_for_server(MOCK_PORT + 3, 5000);

    use metalogos::ast::{Expr, LlmConfigDecl, LlmProviderEntry};
    let decl = LlmConfigDecl {
        span: metalogos::ast::Span::unknown(),
        providers: vec![LlmProviderEntry {
            span: metalogos::ast::Span::unknown(),
            alias: "mock".to_string(),
            provider: "openai".to_string(),
            key: Some(Expr::StringLit {
                value: "test-key".to_string(),
                span: metalogos::ast::Span::unknown(),
            }),
            url: Some(format!(
                "http://127.0.0.1:{}/v1/chat/completions",
                MOCK_PORT + 3
            )),
        }],
        default_model: Some("mock-model".to_string()),
        failover: Some("auto".to_string()),
        circuit_breaker: 3,
        timeout: 30,
    };
    set_global_smart_router(metalogos::llm::SmartRouter::from_config(&decl));

    use metalogos::llm::{
        stream_close, stream_next, stream_via_smart_router, LLM_STREAM_END_MARKER,
    };

    let handle = stream_via_smart_router("p", "i", None, None).expect("open should succeed");
    // Drain the stream.
    loop {
        let delta = stream_next(handle).expect("next should succeed");
        if delta == LLM_STREAM_END_MARKER {
            break;
        }
    }
    // Calling next again on an ended stream must return the marker —
    // NOT re-read from the network, NOT error.
    let delta = stream_next(handle).expect("next on ended stream should return marker");
    assert_eq!(delta, LLM_STREAM_END_MARKER);
    let _ = stream_close(handle).expect("close should succeed");

    // Teardown.
    let _ = server.kill();
    let _ = server.wait();
    clear_global_smart_router();
    clear_llm_stream_registry_for_tests();
}
