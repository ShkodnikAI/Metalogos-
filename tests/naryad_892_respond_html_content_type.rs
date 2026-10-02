//! №552 (Wave 25, dispatch gh#925; P0, the audit 02.10 M-2 step 3 and
//! the §7.3 tail) — the respond_html Content-Type contract ON THE WIRE:
//! a real HTTP server (`run_test_server_with_backend_state_in_dir`), BOTH
//! serve backends, all three contract forms (gh#899 — the SSOT), the
//! `content-type` header of the served HTTP response asserted to start
//! with `text/html`.
//!
//! WHY: the original №523 regression (gh#892) was a SERVER defect — the
//! builtin returned the String body and axum served it as `text/plain`
//! (the office's FORGE pages died with 500/404 and the wrong media
//! type). `tests/n892_respond_html_contract.rs` pins the BUILTIN value
//! (Value::HttpResponse + content_type) — this file pins the SERVED
//! response, so the regression class ("the server served the String
//! body as text/plain") can never pass silently again: the value must
//! survive `http_response_into_response` through BOTH backend paths to
//! the actual socket.
//!
//! The three contract forms over GET routes:
//!   1. `respond_html(html)` — the 1-arg office corpus form: 200, the
//!      body verbatim;
//!   2. `respond_html("404 Not Found", html)` — the (status, html)
//!      form: the FIRST arg opens with a valid HTTP status token → the
//!      requested status on the wire (404), the body verbatim;
//!   3. `respond_html("Office Page", html)` — the (title, html) office
//!      document form: 200, the title carried into the document.
//!
//! Boundaries: the respond_html contract is NOT changed here (gh#899
//! is the SSOT; the explicit-forms evolution is №565); this is test
//! protection only — the blocking integration lane picks the file up
//! automatically (the tests/ convention).

#![cfg(feature = "server")]
// The test harness (not program-influenced code) manages its temp files
// directly — the №475 fs_gate restricts PROGRAM-INFLUENCED paths.
#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;
use std::time::Duration;

fn temp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("n552-{}-{}", tag, std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

/// GET and read the FULL wire answer: the status, the content-type
/// header, the body.
async fn get(port: u16, path: &str) -> (reqwest::StatusCode, Option<String>, String) {
    let resp = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{}{}", port, path))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .expect("GET should succeed");
    let status = resp.status();
    let ct = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .map(|v| v.to_str().expect("content-type is ascii").to_string());
    let body = resp.text().await.expect("body");
    (status, ct, body)
}

fn source() -> String {
    r#"
mlogserver {
  port: 0
  route "/one" method=GET {
    return respond_html("<h1>one-arg office form</h1>")
  }
  route "/status" method=GET {
    return respond_html("404 Not Found", "<p>the status form body</p>")
  }
  route "/doc" method=GET {
    return respond_html("Office Page", "<p>the title form body</p>")
  }
}
"#
    .to_string()
}

async fn run_scenario(backend: metalogos::server::ServeBackend, tag: &str) {
    let dir = temp_dir(tag);
    let src = source();

    let (port, handle, _state) =
        metalogos::server::run_test_server_with_backend_state_in_dir(&src, backend, dir.clone())
            .await
            .expect("boot must start");

    // ── Form 1: the 1-arg office corpus form — 200 + text/html on the wire ──
    let (status, ct, body) = get(port, "/one").await;
    assert_eq!(
        status,
        reqwest::StatusCode::OK,
        "{}: the 1-arg form must serve 200, got {}: {}",
        tag,
        status,
        body
    );
    let ct = ct.unwrap_or_else(|| panic!("{}: the 1-arg form must carry content-type", tag));
    assert!(
        ct.starts_with("text/html"),
        "{}: THE REGRESSION CLASS — the 1-arg form served content-type {:?} \
         (the №523 defect served text/plain); the wire must be text/html",
        tag,
        ct
    );
    assert!(
        body.contains("one-arg office form"),
        "{}: the 1-arg body must survive verbatim, got: {}",
        tag,
        body
    );

    // ── Form 2: the (status, html) form — the requested status + text/html ──
    let (status, ct, body) = get(port, "/status").await;
    assert_eq!(
        status,
        reqwest::StatusCode::NOT_FOUND,
        "{}: the (status, html) form must serve the requested 404, got {}: {}",
        tag,
        status,
        body
    );
    let ct = ct.unwrap_or_else(|| panic!("{}: the status form must carry content-type", tag));
    assert!(
        ct.starts_with("text/html"),
        "{}: THE REGRESSION CLASS — the (status, html) form served content-type {:?}; \
         the wire must be text/html",
        tag,
        ct
    );
    assert!(
        body.contains("the status form body"),
        "{}: the (status, html) body must survive verbatim, got: {}",
        tag,
        body
    );

    // ── Form 3: the (title, html) office document form — 200 + text/html ──
    let (status, ct, body) = get(port, "/doc").await;
    assert_eq!(
        status,
        reqwest::StatusCode::OK,
        "{}: the (title, html) form must serve 200, got {}: {}",
        tag,
        status,
        body
    );
    let ct = ct.unwrap_or_else(|| panic!("{}: the title form must carry content-type", tag));
    assert!(
        ct.starts_with("text/html"),
        "{}: THE REGRESSION CLASS — the (title, html) form served content-type {:?}; \
         the wire must be text/html",
        tag,
        ct
    );
    assert!(
        body.contains("Office Page") && body.contains("the title form body"),
        "{}: the title must be carried into the served document, got: {}",
        tag,
        body
    );

    handle.abort();
    let _ = handle.await;
    let _ = std::fs::remove_dir_all(&dir);
}

/// ONE test fn, BOTH backends sequentially (the №496 pattern): the serve
/// default (VM) and the guaranteed full-language opt-out (TW) must agree
/// on the wire — a program's HTTP answer does not depend on the backend
/// that ran it.
#[tokio::test]
async fn n892_respond_html_content_type_on_the_wire_both_backends() {
    // The TW interpreter lane first, then the VM (the serve default).
    run_scenario(metalogos::server::ServeBackend::Interpreter, "tw").await;
    run_scenario(metalogos::server::ServeBackend::Vm, "vm").await;
}
