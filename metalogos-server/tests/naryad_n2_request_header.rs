// N2 (gh#1211, PE-1): `request_header()` — the ROUTE-context surface.
//
// The off-server shape (empty string, loud arg errors, the ct_eq
// correctness table) lives in the root-crate suite
// tests/n2_request_header_ct_eq.rs. This file pins the part only a
// real server can answer: the per-request HeaderMap injection on BOTH
// backends, the case-insensitive name lookup against the LIVE request
// headers, and the strict TW↔VM cross-check (the wave discipline:
// golden outputs identical on TW and VM).
//
// The canonical scenario is the Telegram webhook secret check:
// the route reads `X-Telegram-Bot-Api-Secret-Token`, compares it
// against the configured token with `ct_eq` (constant time), and
// answers authorized/denied. No raw header value is ever echoed into
// a response — the comparison result is a Bool, so no taint lands in
// the egress (the SINK_CLEARANCE surface is not exercised here).
#![cfg(feature = "server")]
#![allow(clippy::disallowed_methods)]

use metalogos_server::server::ServeBackend;

async fn start(source: &str, backend: ServeBackend) -> u16 {
    let (port, _handle) = metalogos_server::server::run_test_server_with_backend(source, backend)
        .await
        .expect("test server should start");
    port
}

async fn http_get(port: u16, path: &str) -> (u16, String) {
    let url = format!("http://127.0.0.1:{port}{path}");
    let resp = reqwest::get(&url)
        .await
        .expect("GET request should succeed");
    let status = resp.status().as_u16();
    let body = resp.text().await.expect("response body should be readable");
    (status, body)
}

async fn http_get_with_header(port: u16, path: &str, name: &str, value: &str) -> (u16, String) {
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}{path}");
    let resp = client
        .get(&url)
        .header(name, value)
        .send()
        .await
        .expect("GET request should succeed");
    let status = resp.status().as_u16();
    let body = resp.text().await.expect("response body should be readable");
    (status, body)
}

const WEBHOOK: &str = r#"
mlogserver {
  port: 0
  route "/webhook" method=GET {
    let provided = request_header("X-Telegram-Bot-Api-Secret-Token")
    if ct_eq(provided, "whsec-test-123") {
      respond("authorized")
    }
    respond("denied")
  }
  route "/case" method=GET {
    let provided = request_header("x-TELEGRAM-bot-API-secret-TOKEN")
    if ct_eq(provided, "whsec-test-123") {
      respond("ci-ok")
    }
    respond("ci-miss")
  }
  route "/absent" method=GET {
    let provided = request_header("X-Not-Sent-By-Anyone")
    if ct_eq(provided, "") {
      respond("empty-as-contract")
    }
    respond("non-empty")
  }
}"#;

// ── The authorized path: the correct token arrives ──────────────────

#[tokio::test]
async fn n2_request_header_correct_token_authorized_both_backends() {
    for backend in [ServeBackend::Interpreter, ServeBackend::Vm] {
        let port = start(WEBHOOK, backend).await;
        let (status, body) = http_get_with_header(
            port,
            "/webhook",
            "X-Telegram-Bot-Api-Secret-Token",
            "whsec-test-123",
        )
        .await;
        assert_eq!(status, 200, "{backend:?}: the authorized answer is 200");
        assert_eq!(
            body, "authorized",
            "{backend:?}: the correct token authorizes"
        );
    }
}

// ── The denied paths: wrong / missing token ─────────────────────────

#[tokio::test]
async fn n2_request_header_wrong_token_denied_both_backends() {
    for backend in [ServeBackend::Interpreter, ServeBackend::Vm] {
        let port = start(WEBHOOK, backend).await;
        let (status, body) = http_get_with_header(
            port,
            "/webhook",
            "X-Telegram-Bot-Api-Secret-Token",
            "whsec-WRONG",
        )
        .await;
        assert_eq!(status, 200, "{backend:?}: the denied answer is still 200");
        assert_eq!(
            body, "denied",
            "{backend:?}: a wrong token never authorizes"
        );
    }
}

#[tokio::test]
async fn n2_request_header_missing_header_denied_both_backends() {
    for backend in [ServeBackend::Interpreter, ServeBackend::Vm] {
        let port = start(WEBHOOK, backend).await;
        let (status, body) = http_get(port, "/webhook").await;
        assert_eq!(status, 200, "{backend:?}: the denied answer is still 200");
        assert_eq!(
            body, "denied",
            "{backend:?}: no header → the empty string → denied"
        );
    }
}

// ── The case-insensitive NAME lookup against the live headers ───────

#[tokio::test]
async fn n2_request_header_name_lookup_case_insensitive_both_backends() {
    // The ROUTE spells the name x-TELEGRAM-bot-API-secret-TOKEN; the
    // client sends the canonical spelling. HTTP header names are
    // case-insensitive — the lookup must agree.
    for backend in [ServeBackend::Interpreter, ServeBackend::Vm] {
        let port = start(WEBHOOK, backend).await;
        let (status, body) = http_get_with_header(
            port,
            "/case",
            "X-Telegram-Bot-Api-Secret-Token",
            "whsec-test-123",
        )
        .await;
        assert_eq!(status, 200, "{backend:?}: the case-route answer is 200");
        assert_eq!(
            body, "ci-ok",
            "{backend:?}: the mixed-case lookup finds the canonical header"
        );
    }
}

// ── The absent-header contract: the empty string is observable ──────

#[tokio::test]
async fn n2_request_header_absent_header_reads_empty_both_backends() {
    for backend in [ServeBackend::Interpreter, ServeBackend::Vm] {
        let port = start(WEBHOOK, backend).await;
        let (status, body) = http_get(port, "/absent").await;
        assert_eq!(status, 200, "{backend:?}: the absent-route answer is 200");
        assert_eq!(
            body, "empty-as-contract",
            "{backend:?}: an absent header reads as the empty string"
        );
    }
}

// ── The strict cross-check: TW and VM answer IDENTICALLY ────────────

#[tokio::test]
async fn n2_request_header_vm_tw_crosscheck_identical() {
    for path in ["/webhook", "/case", "/absent"] {
        let vm_port = start(WEBHOOK, ServeBackend::Vm).await;
        let tw_port = start(WEBHOOK, ServeBackend::Interpreter).await;
        let vm = http_get_with_header(
            vm_port,
            path,
            "X-Telegram-Bot-Api-Secret-Token",
            "whsec-test-123",
        )
        .await;
        let tw = http_get_with_header(
            tw_port,
            path,
            "X-Telegram-Bot-Api-Secret-Token",
            "whsec-test-123",
        )
        .await;
        assert_eq!(vm, tw, "TW and VM must answer identically on {path}");
    }
}
