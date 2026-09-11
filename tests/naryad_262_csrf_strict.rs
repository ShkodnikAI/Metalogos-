#![cfg(feature = "server")]
// ── Наряд №262: CSRF strict — server-issued tokens + session binding ──
// Contract (HTTP level, real axum stack via run_test_server — no external
// network, both servers bind 127.0.0.1:0):
//   1. A double-submit pair never ISSUED by the server (self-made cookie +
//      matching header — the naive double-submit bypass) → 403. Pre-№262
//      the stateless fallback accepted exactly this with 200 — the probe
//      regression of this naryad.
//   2. An issued token, presented in the context it was issued in
//      (sessionless GET → sessionless POST) → 200.
//   3. A token issued by ANOTHER server process (restart simulation) → 403
//      — the store is process-local by design.
//   4. Session binding: a sessionless-issued token replayed WITH a valid
//      signed session cookie → 403 (binding mismatch); with a garbage
//      session cookie (bad HMAC) → 200 — the documented №262 boundary: a
//      request that cannot prove a session identity counts as sessionless.
// TTL (15 min) is pinned by unit tests in src/server.rs — over HTTP a token
// is always freshly issued, backdating is impossible.

use std::sync::Mutex;

// Fixed HMAC key (64 hex chars → 32 bytes) so the test can sign session
// cookies the server will accept — set as METALOGOS_HMAC_KEY before the
// test server's build_state reads it.
const HMAC_KEY_HEX: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

const SOURCE: &str = r#"
mlogserver {
  port: 0
  middleware: [session, csrf, security_headers]
  route "/" method=GET { return "hello" }
  route "/data" method=POST { return "posted" }
}
"#;

/// Env mutex (лекало naryad_244:69): METALOGOS_HMAC_KEY is process-global,
/// so the tests that touch it hold the lock for their WHOLE body (lesson of
/// naryad #261: a parallel sibling must not observe a half-applied env).
fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static ENV_LOCK: Mutex<()> = Mutex::new(());
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Extract _mlog_csrf value from Set-Cookie header (лекало naryad_125).
fn extract_csrf_from_set_cookie(headers: &reqwest::header::HeaderMap) -> Option<String> {
    headers
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .find_map(|v| {
            v.to_str().ok().and_then(|s| {
                s.split(';')
                    .next()
                    .and_then(|part| part.strip_prefix("_mlog_csrf="))
                    .map(|t| t.to_string())
            })
        })
}

async fn start_server() -> (
    u16,
    tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
) {
    metalogos::server::run_test_server(SOURCE)
        .await
        .expect("server should start")
}

/// POST /data with the given Cookie header value and X-CSRF-Token.
async fn post_csrf(base: &str, cookie: &str, header_token: &str) -> reqwest::Response {
    reqwest::Client::new()
        .post(format!("{}/data", base))
        .header("Cookie", cookie)
        .header("X-CSRF-Token", header_token)
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn test_262_forged_pair_rejected() {
    // The naryad's probe: a self-made cookie+header pair that was never
    // issued. Pre-№262: 200 (stateless fallback). Contract now: 403.
    let (port, _handle) = start_server().await;
    let base = format!("http://127.0.0.1:{}", port);

    let forged = "deadbeefdeadbeefdeadbeefdeadbeef";
    let resp = post_csrf(&base, &format!("_mlog_csrf={}", forged), forged).await;
    assert_eq!(
        resp.status(),
        403,
        "a never-issued double-submit pair must be rejected"
    );
}

#[tokio::test]
async fn test_262_issued_token_sessionless_flow_accepted() {
    // GET (no session cookie) issues a token bound to ""; the sessionless
    // POST presenting exactly that token passes.
    let (port, _handle) = start_server().await;
    let base = format!("http://127.0.0.1:{}", port);

    let resp = reqwest::get(format!("{}/", base)).await.unwrap();
    assert_eq!(resp.status(), 200);
    let token =
        extract_csrf_from_set_cookie(resp.headers()).expect("GET should set _mlog_csrf cookie");

    let resp = post_csrf(&base, &format!("_mlog_csrf={}", token), &token).await;
    assert_eq!(resp.status(), 200, "issued token in its own context passes");
    assert_eq!(resp.text().await.unwrap(), "posted");
}

#[tokio::test]
async fn test_262_token_from_other_instance_rejected() {
    // Restart simulation: the token was issued by process A, the POST goes
    // to process B whose store has never seen it → 403.
    let (port_a, _ha) = start_server().await;
    let (port_b, _hb) = start_server().await;
    let base_a = format!("http://127.0.0.1:{}", port_a);
    let base_b = format!("http://127.0.0.1:{}", port_b);

    let resp = reqwest::get(&base_a).await.unwrap();
    let token =
        extract_csrf_from_set_cookie(resp.headers()).expect("GET should set _mlog_csrf cookie");

    let resp = post_csrf(&base_b, &format!("_mlog_csrf={}", token), &token).await;
    assert_eq!(
        resp.status(),
        403,
        "a token issued by another server process must be rejected"
    );
}

#[tokio::test]
// The env guard intentionally spans the awaits: the server state (built at
// an await point inside run_test_server) reads METALOGOS_HMAC_KEY, so the
// value must stay pinned for the WHOLE body. Test-only std::Mutex, no other
// code path takes this lock — no deadlock/race risk (naryad #261 lesson).
#[allow(clippy::await_holding_lock)]
async fn test_262_sessionless_token_replayed_with_valid_session_rejected() {
    // A token issued without a session (bound to "") replayed WITH a validly
    // signed session cookie → 403 binding mismatch (the "no-session" token is
    // only valid without a session). Negative control: the same token without
    // the session cookie passes — the 403 comes from the binding, nothing else.
    let _guard = env_lock();
    std::env::set_var("METALOGOS_HMAC_KEY", HMAC_KEY_HEX);

    let (port, _handle) = start_server().await;
    let base = format!("http://127.0.0.1:{}", port);

    let resp = reqwest::get(&base).await.unwrap();
    let token = extract_csrf_from_set_cookie(resp.headers()).unwrap();

    let key = hex::decode(HMAC_KEY_HEX).unwrap();
    let signed_foreign = metalogos::server::sign_cookie("sess-Z", &key);

    let resp = post_csrf(
        &base,
        &format!("_mlog_csrf={}; _mlog_session={}", token, signed_foreign),
        &token,
    )
    .await;
    assert_eq!(
        resp.status(),
        403,
        "sessionless token + a valid foreign session = binding mismatch"
    );

    let resp = post_csrf(&base, &format!("_mlog_csrf={}", token), &token).await;
    assert_eq!(
        resp.status(),
        200,
        "negative control: without the session cookie the same token passes"
    );

    std::env::remove_var("METALOGOS_HMAC_KEY");
}

#[tokio::test]
// Same intentional guard-across-await as above (see the comment there).
#[allow(clippy::await_holding_lock)]
async fn test_262_sessionless_token_with_unverifiable_session_cookie_accepted() {
    // Documented №262 boundary: a session cookie that fails HMAC verification
    // proves nothing, so the request counts as sessionless and the
    // sessionless-issued token stays valid. (Everywhere else in the pipeline
    // a bad-HMAC session cookie is ignored identically — route_handler step 3.)
    let _guard = env_lock();
    std::env::set_var("METALOGOS_HMAC_KEY", HMAC_KEY_HEX);

    let (port, _handle) = start_server().await;
    let base = format!("http://127.0.0.1:{}", port);

    let resp = reqwest::get(&base).await.unwrap();
    let token = extract_csrf_from_set_cookie(resp.headers()).unwrap();

    let resp = post_csrf(
        &base,
        &format!("_mlog_csrf={}; _mlog_session=not-a-valid-signature", token),
        &token,
    )
    .await;
    assert_eq!(
        resp.status(),
        200,
        "unverifiable session cookie counts as sessionless (documented boundary)"
    );

    std::env::remove_var("METALOGOS_HMAC_KEY");
}
