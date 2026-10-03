// ── tests/naryad_892_respond_html_content_type.rs ────────────────────
// №552 (Wave 25, the до-релизный block 0.28, P0; the audit 02.10 M-2
// step 3 / the §7.3 tail): the Content-Type contract of `respond_html`
// is pinned at the HTTP WIRE level — the level where the original
// defect actually lived.
//
// HISTORY: №523's spec hardening (the arity 1→2) made every office HTML
// route refuse at `mlog check`, and after #899 restored the 1-arg form
// the SERVER still served the String body as `text/plain` (the axum
// default for a String body) — the office FORGE pages 500/404'd and
// then rendered as plain text, caught by the OFFICE, not by CI. The
// builtin-level contract (tests/n892_respond_html_contract.rs) pins the
// Value::HttpResponse shapes — it cannot see the wire. THIS file pins
// the wire: a REAL `run_test_server_with_backend` HTTP round-trip for
// all three forms × BOTH serve backends, asserting `content-type`
// STARTS WITH `text/html` on every response (the exact regression
// shape: a String body leaking through as text/plain would fail here).
//
//   W1: respond_html(html)            → 200, text/html, body verbatim;
//   W2: respond_html(status, html)    → the status lands (404 here),
//                                       text/html;
//   W3: respond_html(title, body)     → 200, text/html, the full
//                                       document carries the title.
//
// Both backends (TW interpreter and VM — the serve default) run the
// same three routes sequentially in ONE test fn (the №496/№522
// precedent: one fn, no process-global races), each boot on its own
// OS-assigned port.

#![cfg(feature = "server")]
// The test harness (not program-influenced code) manages its HTTP
// client directly — the №475 fs_gate restricts PROGRAM-INFLUENCED paths.
#![allow(clippy::disallowed_methods)]

use std::time::Duration;

fn wire_source() -> String {
    r#"
mlogserver {
  port: 0
  route "/one" method=GET {
    return respond_html("<h1>one</h1>")
  }
  route "/status" method=GET {
    return respond_html("404", "<h1>not found</h1>")
  }
  route "/titled" method=GET {
    return respond_html("The Page", "<p>titled body</p>")
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

fn wire_content_type(resp: &reqwest::Response) -> String {
    resp.headers()
        .get(reqwest::header::CONTENT_TYPE)
        .unwrap_or_else(|| {
            panic!(
                "content-type header must be present on the wire (status {})",
                resp.status()
            )
        })
        .to_str()
        .expect("the content-type header is ASCII")
        .to_string()
}

async fn assert_wire_content_type(backend: metalogos_server::server::ServeBackend, tag: &str) {
    let source = wire_source();
    let (port, handle) = metalogos_server::server::run_test_server_with_backend(&source, backend)
        .await
        .unwrap_or_else(|e| panic!("{tag}: the test server must start: {e}"));

    // W1: the 1-arg office corpus form — 200, text/html, body verbatim.
    let resp = get(port, "one").await;
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::OK,
        "{tag}: W1 must answer 200"
    );
    let ct = wire_content_type(&resp);
    assert!(
        ct.starts_with("text/html"),
        "{tag}: W1 the wire content-type must be text/html, got '{ct}' — \
         the original defect was exactly this leak (String body → text/plain)"
    );
    let body = resp.text().await.expect("{tag}: W1 body");
    assert!(
        body.contains("<h1>one</h1>"),
        "{tag}: W1 the body must carry the HTML verbatim, got: {body}"
    );

    // W2: the (status, html) form — the status lands on the wire.
    let resp = get(port, "status").await;
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::NOT_FOUND,
        "{tag}: W2 the first-arg status token must become the HTTP status"
    );
    let ct = wire_content_type(&resp);
    assert!(
        ct.starts_with("text/html"),
        "{tag}: W2 the wire content-type must be text/html, got '{ct}'"
    );
    let body = resp.text().await.expect("{tag}: W2 body");
    assert!(
        body.contains("<h1>not found</h1>"),
        "{tag}: W2 the body must stay VERBATIM (no document wrapping on \
         the status form), got: {body}"
    );

    // W3: the (title, body) office form — a full document, text/html.
    let resp = get(port, "titled").await;
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::OK,
        "{tag}: W3 must answer 200"
    );
    let ct = wire_content_type(&resp);
    assert!(
        ct.starts_with("text/html"),
        "{tag}: W3 the wire content-type must be text/html, got '{ct}'"
    );
    let body = resp.text().await.expect("{tag}: W3 body");
    assert!(
        body.contains("The Page"),
        "{tag}: W3 the full document must carry the title, got: {body}"
    );

    handle.abort();
}

#[tokio::test]
async fn n552_respond_html_content_type_wire_both_backends() {
    // The TW interpreter backend (the explicit opt-out).
    assert_wire_content_type(metalogos_server::server::ServeBackend::Interpreter, "TW").await;
    // The VM backend (the serve default since ADR-0171) — the same
    // three routes through the bytecode compiler and the pool path.
    assert_wire_content_type(metalogos_server::server::ServeBackend::Vm, "VM").await;
}
