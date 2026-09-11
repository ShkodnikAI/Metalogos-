// ── НАРЯД №255: явный лимит тела запроса вместо неявного дефолта axum ──
//
// Контракт (issue #258): сервер принимает тела до 2 МиБ
// (REQUEST_BODY_LIMIT_BYTES = 2 097 152 байта, src/server.rs) —
// осознанная константа, а не молчаливый дефолт чужого крейта.
// Тело N−1 байт → проходит (роут отвечает), тело N+1 байт → HTTP 413.
//
// Реальный HTTP-стек (TcpListener + reqwest), как в naryad_160.

#![cfg(feature = "server")]

use metalogos::server::ServeBackend;

const ECHO_SOURCE: &str = r#"
mlogserver {
  port: 8095
  route "/accept" method=POST {
    respond("200", "accepted")
  }
}
"#;

/// REQUEST_BODY_LIMIT_BYTES: 2 MiB (пин числа из src/server.rs).
const BODY_LIMIT: usize = 2 * 1024 * 1024;

async fn start_server() -> u16 {
    let base_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let (port, _handle) = metalogos::server::run_test_server_with_backend_in_dir(
        ECHO_SOURCE,
        ServeBackend::Interpreter,
        base_dir,
    )
    .await
    .expect("test server should start");
    port
}

async fn post_bytes(port: u16, path: &str, len: usize) -> u16 {
    let url = format!("http://127.0.0.1:{}{}", port, path);
    // Тело — не JSON (роут его не читает): важен только размер.
    let body = vec![b'a'; len];
    let resp = reqwest::Client::new()
        .post(&url)
        .header("Content-Type", "application/octet-stream")
        .body(body)
        .send()
        .await
        .expect("POST should succeed");
    resp.status().as_u16()
}

#[tokio::test]
async fn n255_body_under_limit_passes() {
    let port = start_server().await;
    // 1 КиБ << N
    assert_eq!(post_bytes(port, "/accept", 1024).await, 200);
}

#[tokio::test]
async fn n255_body_limit_minus_one_passes() {
    let port = start_server().await;
    // N−1 байт: проходит
    let status = post_bytes(port, "/accept", BODY_LIMIT - 1).await;
    assert_eq!(
        status, 200,
        "тело N−1 байт должно проходить, got {}",
        status
    );
}

#[tokio::test]
async fn n255_body_limit_plus_one_is_413() {
    let port = start_server().await;
    // N+1 байт: 413 Payload Too Large
    let status = post_bytes(port, "/accept", BODY_LIMIT + 1).await;
    assert_eq!(
        status, 413,
        "тело N+1 байт должно давать 413, got {}",
        status
    );
}
