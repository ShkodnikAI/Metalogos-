#![cfg(feature = "server")]
// INTENTIONAL guard-across-await (allow precedent naryad_262): every test holds
// the process-global env mutex across the HTTP awaits — run_test_server's
// build_state reads METALOGOS_TRUSTED_PROXIES at server startup, so a sibling
// test must not observe a half-applied env (лекало naryad_244:69, lesson n261).
#![allow(clippy::await_holding_lock)]
// ── Наряд №263: rate-limit keyed by the connection peer + bounded state maps ──
// Contract (HTTP level, real axum stack via run_test_server — no external
// network, every server binds 127.0.0.1:0):
//   1. The 101st request from ONE peer (default limit 100, unchanged by #263)
//      → 429.
//   2. Spoofed X-Forwarded-For WITHOUT METALOGOS_TRUSTED_PROXIES does not
//      rotate the key: requests with different XFF values land in ONE bucket
//      (the pre-#263 bypass — a fresh spoofed XFF per request — returned 200
//      forever; it now 429s).
//   3. WITH METALOGOS_TRUSTED_PROXIES and the direct peer listed, the key is
//      the leftmost X-Forwarded-For entry: a full XFF bucket blocks that XFF
//      while another XFF still passes — the key demonstrably came from the
//      header, not from the peer.
//   4. A direct peer NOT in METALOGOS_TRUSTED_PROXIES is never allowed to key
//      through headers, even when the env var is set for other proxies.
//   5. The mlogserver `rate_limit: N` field drives the limit end-to-end.
// Map caps and the sweep are pinned by unit tests in src/server.rs
// (the caps are constants — filling them over HTTP would need 10k–65k requests).
//
// Env mutex (лекало naryad_244:69 / naryad_262): METALOGOS_TRUSTED_PROXIES is
// process-global and build_state reads it at server startup, so EVERY test in
// this file holds the lock for its WHOLE body — including the ones that never
// touch the var (lesson of naryad #261: a parallel sibling must not observe a
// half-applied env while its server state is being built).

use std::sync::Mutex;

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static ENV_LOCK: Mutex<()> = Mutex::new(());
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

const SOURCE_DEFAULT_LIMIT: &str = r#"
mlogserver {
  port: 0
  middleware: [rate_limit]
  route "/ping" method=GET { return "pong" }
}
"#;

const SOURCE_LIMIT_3: &str = r#"
mlogserver {
  port: 0
  middleware: [rate_limit]
  rate_limit: 3
  route "/ping" method=GET { return "pong" }
}
"#;

const SOURCE_LIMIT_2: &str = r#"
mlogserver {
  port: 0
  middleware: [rate_limit]
  rate_limit: 2
  route "/ping" method=GET { return "pong" }
}
"#;

async fn start_server(source: &str) -> String {
    let (port, _handle) = metalogos::server::run_test_server(source)
        .await
        .expect("server should start");
    format!("http://127.0.0.1:{}", port)
}

/// One GET /ping with an optional X-Forwarded-For value.
async fn get_ping(base: &str, xff: Option<&str>) -> reqwest::Response {
    let mut req = reqwest::Client::new().get(format!("{}/ping", base));
    if let Some(xff) = xff {
        req = req.header("X-Forwarded-For", xff);
    }
    req.send().await.unwrap()
}

#[tokio::test]
async fn test_263_101st_request_from_same_peer_is_429() {
    let _env = env_lock();
    std::env::remove_var("METALOGOS_TRUSTED_PROXIES");
    let base = start_server(SOURCE_DEFAULT_LIMIT).await;

    // Default limit: 100 req/min per peer (pre-#263 hard-wired value, unchanged).
    for i in 0..100 {
        let resp = get_ping(&base, None).await;
        assert_eq!(
            resp.status(),
            200,
            "request #{} below the default limit must pass",
            i + 1
        );
    }
    let resp = get_ping(&base, None).await;
    assert_eq!(
        resp.status(),
        429,
        "the 101st request from one peer must 429"
    );
    assert!(resp.text().await.unwrap().contains("rate limit exceeded"));
}

#[tokio::test]
async fn test_263_spoofed_xff_without_trusted_proxies_shares_one_bucket() {
    let _env = env_lock();
    std::env::remove_var("METALOGOS_TRUSTED_PROXIES");
    let base = start_server(SOURCE_LIMIT_3).await;

    // Three requests, each with a DIFFERENT spoofed XFF: pre-#263 each got a
    // fresh bucket (the bypass). Post-#263 they share the peer bucket.
    for xff in ["1.2.3.4", "5.6.7.8", "9.9.9.9"] {
        let resp = get_ping(&base, Some(xff)).await;
        assert_eq!(
            resp.status(),
            200,
            "spoofed XFF {} must not pre-exhaust the bucket",
            xff
        );
    }
    // A 4th request with yet another spoofed XFF → 429: one bucket = the peer.
    let resp = get_ping(&base, Some("7.7.7.7")).await;
    assert_eq!(
        resp.status(),
        429,
        "spoofed XFF must not rotate the rate-limit key without trusted proxies"
    );
}

#[tokio::test]
async fn test_263_trusted_proxies_key_from_xff() {
    let _env = env_lock();
    std::env::set_var("METALOGOS_TRUSTED_PROXIES", "127.0.0.1");
    let base = start_server(SOURCE_LIMIT_3).await;

    // The direct peer (127.0.0.1) is trusted: the key is the leftmost XFF entry.
    for _ in 0..3 {
        let resp = get_ping(&base, Some("203.0.113.7")).await;
        assert_eq!(
            resp.status(),
            200,
            "requests of client .7 fill its own bucket"
        );
    }
    let resp = get_ping(&base, Some("203.0.113.7")).await;
    assert_eq!(resp.status(), 429, "the .7 bucket is full at rate_limit: 3");

    // A DIFFERENT XFF still passes — proof the key came from the header, not
    // from the peer (under peer-keying this 5th request would 429 as well).
    let resp = get_ping(&base, Some("203.0.113.8")).await;
    assert_eq!(resp.status(), 200, "another XFF entry = another bucket");

    std::env::remove_var("METALOGOS_TRUSTED_PROXIES");
}

#[tokio::test]
async fn test_263_trusted_proxies_peer_not_listed_headers_ignored() {
    let _env = env_lock();
    // The env var is set — but for a DIFFERENT proxy; the direct peer 127.0.0.1
    // is not in the list, so headers must never be honored.
    std::env::set_var("METALOGOS_TRUSTED_PROXIES", "10.99.99.99");
    let base = start_server(SOURCE_LIMIT_2).await;

    let r1 = get_ping(&base, Some("203.0.113.7")).await;
    let r2 = get_ping(&base, Some("203.0.113.8")).await;
    assert_eq!(r1.status(), 200);
    assert_eq!(r2.status(), 200);
    let r3 = get_ping(&base, Some("203.0.113.9")).await;
    assert_eq!(
        r3.status(),
        429,
        "an unlisted direct peer must never key through headers, even with the env var set"
    );

    std::env::remove_var("METALOGOS_TRUSTED_PROXIES");
}

#[tokio::test]
async fn test_263_rate_limit_field_drives_the_limit() {
    let _env = env_lock();
    std::env::remove_var("METALOGOS_TRUSTED_PROXIES");
    let base = start_server(SOURCE_LIMIT_2).await;

    let r1 = get_ping(&base, None).await;
    let r2 = get_ping(&base, None).await;
    assert_eq!(r1.status(), 200);
    assert_eq!(r2.status(), 200);
    let r3 = get_ping(&base, None).await;
    assert_eq!(
        r3.status(),
        429,
        "mlogserver rate_limit: 2 must override the default end-to-end"
    );
}
