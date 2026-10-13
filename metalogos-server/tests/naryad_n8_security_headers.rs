// ── tests/naryad_n8_security_headers.rs ──────────────────────────────
// Naryad N8 (gh#1218, PE-2, serve/bug): the `security_headers` middleware
// — the owner's finding: the declaration was VALIDATED (the semantic
// name check + the "no security_headers" recommendation warning) but
// NEVER APPLIED — `curl -i` on a plain GET showed no CSP, no
// X-Content-Type-Options, no HSTS.
//
// The fix: the middleware actually applies the three protective headers
// to EVERY response (the route responses AND the fall-through 404 —
// the OWASP shape is every-response, not happy-path-only):
//
//   H1: Content-Security-Policy: default-src 'self'
//   H2: X-Content-Type-Options: nosniff
//   H3: Strict-Transport-Security: max-age=31536000; includeSubDomains
//
// The opt-in contract: WITHOUT the middleware in the list the headers
// are ABSENT (the declaration is the switch — the semantic warning
// recommends it, the server honors it).
//
// The wire level is where the defect lived (the №892 precedent: the
// builtin-level contract cannot see the wire) — a REAL
// `run_test_server_with_backend` HTTP round-trip, both serve backends.

#![cfg(feature = "server")]
// The test harness (not program-influenced code) manages its HTTP
// client directly — the №475 fs_gate restricts PROGRAM-INFLUENCED paths.
#![allow(clippy::disallowed_methods)]

use std::time::Duration;

fn source_with_middleware() -> String {
    r#"
mlogserver {
  port: 0
  middleware: [security_headers]
  route "/" method=GET {
    return respond("200", "ok")
  }
}
"#
    .to_string()
}

fn source_without_middleware() -> String {
    r#"
mlogserver {
  port: 0
  route "/" method=GET {
    return respond("200", "ok")
  }
}
"#
    .to_string()
}

async fn get(port: u16, path: &str) -> reqwest::Response {
    reqwest::Client::new()
        .get(format!("http://127.0.0.1:{}/{}", port, path))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .expect("the GET round-trip should succeed")
}

async fn assert_wire_headers(
    backend: metalogos_server::server::ServeBackend,
    source: &str,
    expected: bool,
    tag: &str,
) {
    let (port, handle) = metalogos_server::server::run_test_server_with_backend(source, backend)
        .await
        .unwrap_or_else(|e| panic!("{tag}: the test server must start: {e}"));

    // The route response.
    let resp = get(port, "").await; // the root route (the harness appends the slash)
    assert_eq!(resp.status(), 200, "{tag}: the route answers 200");
    let csp = resp.headers().get("Content-Security-Policy").is_some();
    let xcto = resp.headers().get("X-Content-Type-Options").is_some();
    let hsts = resp.headers().get("Strict-Transport-Security").is_some();
    assert_eq!(
        (csp, xcto, hsts),
        (expected, expected, expected),
        "{tag}: the protective headers on the route response (expected presence: {expected})"
    );
    // The fall-through 404 — every-response shape.
    let missing = get(port, "no-such-route").await;
    assert_eq!(
        missing.status(),
        404,
        "{tag}: the unknown route answers 404"
    );
    let csp404 = missing.headers().get("Content-Security-Policy").is_some();
    let xcto404 = missing.headers().get("X-Content-Type-Options").is_some();
    let hsts404 = missing.headers().get("Strict-Transport-Security").is_some();
    assert_eq!(
        (csp404, xcto404, hsts404),
        (expected, expected, expected),
        "{tag}: the protective headers on the 404 (every-response shape, expected: {expected})"
    );

    handle.abort();
}

#[tokio::test]
async fn n8_security_headers_present_on_the_wire_tw() {
    assert_wire_headers(
        metalogos_server::server::ServeBackend::Interpreter,
        &source_with_middleware(),
        true,
        "TW+middleware",
    )
    .await;
}

#[tokio::test]
async fn n8_security_headers_present_on_the_wire_vm() {
    assert_wire_headers(
        metalogos_server::server::ServeBackend::Vm,
        &source_with_middleware(),
        true,
        "VM+middleware",
    )
    .await;
}

#[tokio::test]
async fn n8_absent_without_the_middleware_tw() {
    assert_wire_headers(
        metalogos_server::server::ServeBackend::Interpreter,
        &source_without_middleware(),
        false,
        "TW no-middleware",
    )
    .await;
}

#[tokio::test]
async fn n8_absent_without_the_middleware_vm() {
    assert_wire_headers(
        metalogos_server::server::ServeBackend::Vm,
        &source_without_middleware(),
        false,
        "VM no-middleware",
    )
    .await;
}
