//! Наряд №455 (gh#674, P0 security) — audit 25.09 finding 3.2 High:
//! `read_file` did not check the serve-route context, so a route body
//! could read `.env` (and the SQLite files, the grammar, the git
//! metadata) straight out of the application directory — a live bypass of
//! the №259 env-gate through the file channel.
//!
//! Red → green contract (the audit repro: `serve_read_file_env_denied`):
//!   * a program whose route serves `read_file(".env")` content must not
//!     even START: the №455 layer-3 label (a sensitive-named literal
//!     yields the SECRET label) makes the respond() a SINK_CLEARANCE
//!     category-A error at serve startup;
//!   * a route serving `read_file(query_param(...))` must not START:
//!     the №455 layer-4 check (UNTRUSTED_FILE_PATH — the file-channel
//!     twin of UNTRUSTED_EXEC_DECISION) is a category-A error;
//!   * a RUNNING server refuses the ingest at RUNTIME (defense in depth
//!     against static-walker blind spots — the audit exists because of
//!     them): a pattern body that touches a deny-listed path or reads
//!     outside the data directory fails loudly with the stable
//!     `SANDBOX_SENSITIVE_PATH` code, and the file content never reaches
//!     the client.
//!
//! The `.env` fixture carries a unique marker; every assertion pins that
//! the marker NEVER reaches any HTTP response.
//!
//! Verify: cargo test --test serve_read_file_env_denied

#![cfg(feature = "server")]

use metalogos::server::{run_test_server_with_backend, ServeBackend};

/// The CLI startup path (`mlog serve`): run_server enforces the
/// category-A gate BEFORE binding — the test harness
/// (`run_test_server_with_backend`) skips that gate by design (it builds
/// the router directly), so the startup-refusal pins go through the real
/// entry point. A refused program returns Err; an accepted one would run
/// forever, which these tests never reach (both fixtures are refusals).
async fn startup_verdict(
    source: &'static str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    metalogos::server::run_server(source).await
}

static N455_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// The audit's exploit shape #1: a literal sensitive path in a route.
const LEAK_ENV_ROUTE: &str = r#"
mlogserver {
  port: 18096
  route "/env" method=GET { return respond("200", read_file(".env")) }
}
"#;

/// The audit's exploit shape #2: the path comes from the request.
const LEAK_TAINT_ROUTE: &str = r#"
mlogserver {
  port: 18097
  route "/f" method=GET { return respond("200", read_file(query_param("p"))) }
}
"#;

/// A running server (no category-A violation: the read result is only
/// used for its existence) that TOUCHES the deny-listed path at runtime.
const PEEK_ENV_RUNTIME: &str = r#"
pattern PeekEnv() -> String {
  let c = read_file(".env")
  return "peeked"
}
mlogserver {
  port: 18098
  route "/peek" method=GET { return respond("200", PeekEnv()) }
}
"#;

/// A running server that touches a NON-sensitive path OUTSIDE the data
/// directory at runtime (the layer-2 containment).
const PEEK_OUTSIDE_ROOT_RUNTIME: &str = r#"
pattern PeekRoot() -> String {
  let c = read_file("n455_outside_marker.txt")
  return "peeked"
}
mlogserver {
  port: 18099
  route "/peek" method=GET { return respond("200", PeekRoot()) }
}
"#;

/// A running server reading from INSIDE the data directory — the
/// sanctioned serve shape (the file does not need to exist: a missing
/// path degrades to the soft empty read, the contract №254 keeps).
const PEEK_INSIDE_ROOT_RUNTIME: &str = r#"
pattern PeekData() -> String {
  let c = read_file("data/n455_inside.txt")
  return "peeked"
}
mlogserver {
  port: 18100
  route "/peek" method=GET { return respond("200", PeekData()) }
}
"#;

async fn http_get(port: u16, path: &str) -> (u16, String) {
    let resp = reqwest::get(format!("http://127.0.0.1:{}{}", port, path))
        .await
        .expect("request must not fail at transport level");
    let status = resp.status().as_u16();
    let body = resp.text().await.expect("response body");
    (status, body)
}

fn assert_no_marker(body: &str, what: &str) {
    assert!(
        !body.contains("N455_ENV_MARKER_SECRET"),
        "{}: the .env marker must NEVER reach the client — body: {}",
        what,
        body
    );
}

// ── (1) The audit's exploit #1 must not even start ─────────────────────

#[tokio::test]
async fn n455_serve_read_file_env_route_refused_at_startup() {
    let result = startup_verdict(LEAK_ENV_ROUTE).await;
    let err = result.expect_err(
        "a route serving read_file(\".env\") content must refuse to start \
         (№455 layer 3: the sensitive literal yields the SECRET label → \
         SINK_CLEARANCE category A)",
    );
    let msg = format!("{}", err);
    assert_no_marker(&msg, "startup refusal (.env route)");
    assert!(
        msg.contains("Category A"),
        "the refusal must be the category-A gate — err: {}",
        msg
    );
}

// ── (2) The audit's exploit #2 must not even start ─────────────────────

#[tokio::test]
async fn n455_serve_read_file_taint_path_refused_at_startup() {
    let result = startup_verdict(LEAK_TAINT_ROUTE).await;
    let err = result.expect_err(
        "a route serving read_file(query_param(...)) content must refuse to \
         start (№455 layer 4: UNTRUSTED_FILE_PATH category A)",
    );
    let msg = format!("{}", err);
    assert_no_marker(&msg, "startup refusal (taint path route)");
    assert!(
        msg.contains("UNTRUSTED_FILE_PATH"),
        "the refusal must name the file-path decision class — err: {}",
        msg
    );
}

// ── (3) Runtime deny-list: a RUNNING server refuses the ingest ─────────

#[tokio::test]
async fn n455_serve_read_file_env_denied_at_runtime() {
    let _guard = N455_LOCK.lock().await;
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    let (port, handle) = run_test_server_with_backend(PEEK_ENV_RUNTIME, ServeBackend::Interpreter)
        .await
        .expect("the peek server itself must start (the read result is discarded)");
    let (status, body) = http_get(port, "/peek").await;
    handle.abort();
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    assert_ne!(
        status, 200,
        "the sensitive ingest must fail — body: {}",
        body
    );
    assert_no_marker(&body, "runtime deny (.env)");
    assert!(
        body.contains("SANDBOX_SENSITIVE_PATH"),
        "the refusal must carry the stable №455 code — body: {}",
        body
    );
}

// ── (4) Runtime containment: outside the data root is refused ──────────

#[tokio::test]
async fn n455_serve_read_file_outside_data_dir_denied() {
    let _guard = N455_LOCK.lock().await;
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    // The bait file sits in the process cwd (the sandbox root), outside
    // the data directory — exactly the layout the audit exploited. The
    // data directory itself must EXIST so the test exercises the
    // containment refusal, not the fail-closed root-resolution refusal.
    std::fs::create_dir_all("data").expect("create the data dir");
    std::fs::write("n455_outside_marker.txt", "N455_ENV_MARKER_SECRET")
        .expect("write the bait file");
    let (port, handle) =
        run_test_server_with_backend(PEEK_OUTSIDE_ROOT_RUNTIME, ServeBackend::Interpreter)
            .await
            .expect("the peek server must start");
    let (status, body) = http_get(port, "/peek").await;
    handle.abort();
    let _ = std::fs::remove_file("n455_outside_marker.txt");
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    assert_ne!(
        status, 200,
        "the outside-the-data-root ingest must fail — body: {}",
        body
    );
    assert_no_marker(&body, "runtime containment (outside root)");
    assert!(
        body.contains("SANDBOX_SENSITIVE_PATH"),
        "the containment refusal must carry the stable №455 code — body: {}",
        body
    );
}

// ── (5) The sanctioned shape: reads inside the data dir still work ─────

#[tokio::test]
async fn n455_serve_read_file_inside_data_dir_allowed() {
    let _guard = N455_LOCK.lock().await;
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    // The sanctioned shape with the file ACTUALLY present: the read goes
    // through the gate, resolves inside the root, and is allowed.
    std::fs::create_dir_all("data").expect("create the data dir");
    std::fs::write("data/n455_inside.txt", "inside").expect("write the data file");
    let (port, handle) =
        run_test_server_with_backend(PEEK_INSIDE_ROOT_RUNTIME, ServeBackend::Interpreter)
            .await
            .expect("the peek server must start");
    let (status, body) = http_get(port, "/peek").await;
    handle.abort();
    let _ = std::fs::remove_file("data/n455_inside.txt");
    assert_eq!(
        status, 200,
        "a data-directory read (default ./data) must keep working — body: {}",
        body
    );
    assert_eq!(
        body, "peeked",
        "the route answers normally (read discarded)"
    );
}
