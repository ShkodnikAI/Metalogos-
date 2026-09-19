// ── gh#544 (bug, found by the №402 pin test): TW-serve route bodies
//    could not use `from <origin> media_store_*` ───────────────────────
//
// The per-request route interpreter is built via
// `Interpreter::clone_definitions_into`, which never copied
// `origin_decls` — a route body's media contour
// (`Expr::ProvBind` → `media_bind_origin_dispatch(&store,
// &self.origin_decls, …)`) failed loud with
// `media_bind_origin: unknown origin '…' (no origin declaration in this
// program)` (HTTP 500), while the SAME program worked:
//   * on the VM backend — `Vm::load_program` registers
//     `program.origin_decls` per request;
//   * at the TW top level (flows, `mlog run`) — the declaration pass
//     populates `origin_decls` directly (which is why the wave-3
//     kitchen-camera e2e never saw this).
//
// Fix: `clone_definitions_into` propagates `origin_decls` (union,
// first-wins — the same merge discipline as every other definition
// class; the №399 idempotency lesson). Per-request store isolation is
// untouched: `media_store` stays a per-interpreter `Mutex<MediaStore>`
// and a fresh per-request interpreter still starts with its own empty
// store — pinned here by the fresh-store counter (both requests see
// `[Image#1]`, not `[Image#2]`).
//
// The compile-time layer was never broken: `mlog check` sees the
// declared origin (the №332 static gate), and the red state was a
// TW-serve RUNTIME gap — the layer-vs-layer disagreement the wave
// protocol files as a bug.

#![cfg(feature = "server")]

use metalogos::server::ServeBackend;
use std::path::PathBuf;

const ORIGIN_ROUTE_SOURCE: &str = r#"
origin archive_cam { kind: likeness, media: image, label: private }
mlogserver {
  port: 8150
  route "/archive" method=GET {
    let frame = from archive_cam media_store_image("frame-bytes", "private")
    return respond("200", "bound:" + to_string(frame))
  }
}
"#;

async fn http_get(port: u16, path: &str) -> (u16, String) {
    let url = format!("http://127.0.0.1:{}{}", port, path);
    let resp = reqwest::get(&url).await.expect("GET should succeed");
    let status = resp.status().as_u16();
    let body = resp.text().await.expect("body should be readable");
    (status, body)
}

async fn start_server(
    backend: ServeBackend,
) -> (
    u16,
    tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
) {
    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    metalogos::server::run_test_server_with_backend_in_dir(ORIGIN_ROUTE_SOURCE, backend, base_dir)
        .await
        .expect("test server should start")
}

/// The gh#544 repro, now green: the route body's `from <origin>` chain
/// binds against the propagated origin declarations (was: 500
/// `media_bind_origin: unknown origin 'archive_cam'`).
#[tokio::test]
async fn tw_serve_route_binds_origin_and_returns_handle() {
    let (port, handle) = start_server(ServeBackend::Interpreter).await;
    let (status, body) = http_get(port, "/archive").await;

    assert_eq!(
        status, 200,
        "the origin chain must work in a TW-serve route body: {}",
        body
    );
    assert!(
        body.contains("bound:"),
        "the response must carry the bound marker: {}",
        body
    );
    assert!(
        !body.contains("unknown origin"),
        "no origin-missing refusal may leak: {}",
        body
    );
    handle.abort();
}

/// VM-backend parity on the SAME program (this path always worked —
/// `Vm::load_program` registers `program.origin_decls`; pinned so the
/// fix is provably a parity fix, not a VM regression).
#[tokio::test]
async fn vm_serve_route_parity_on_the_same_program() {
    let (port, handle) = start_server(ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/archive").await;

    assert_eq!(status, 200, "VM parity: {}", body);
    assert!(body.contains("bound:"), "{}", body);
    handle.abort();
}

/// Per-request store isolation is PRESERVED (the fix must not turn the
/// media store into shared state): the handle counter restarts on every
/// request — both requests see `[Image#0]` (the store ids are 0-based).
/// A shared store would make the second request `[Image#1]`.
#[tokio::test]
async fn tw_serve_each_request_gets_a_fresh_media_store() {
    let (port, handle) = start_server(ServeBackend::Interpreter).await;

    let (status_a, body_a) = http_get(port, "/archive").await;
    let (status_b, body_b) = http_get(port, "/archive").await;

    assert_eq!(status_a, 200, "{}", body_a);
    assert_eq!(status_b, 200, "{}", body_b);
    assert!(
        body_a.contains("[Image#0]"),
        "request A binds into its own fresh store: {}",
        body_a
    );
    assert!(
        body_b.contains("[Image#0]"),
        "request B must NOT see request A's store entries (fresh store \
         per request, the pre-fix isolation contract): {}",
        body_b
    );
    assert_eq!(
        body_a, body_b,
        "identical bind results across requests — each request binds \
         into its own fresh store; a shared store would shift the \
         second request's counter to [Image#1]"
    );
    handle.abort();
}
