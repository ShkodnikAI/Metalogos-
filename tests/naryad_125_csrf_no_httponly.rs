#![cfg(feature = "server")]
// ── Наряд №125: CSRF double-submit — HttpOnly removed from _mlog_csrf cookie ──
// Contract: full double-submit cycle:
//   1. GET request receives _mlog_csrf cookie WITHOUT HttpOnly flag
//   2. JS can read it (cookie is present in Set-Cookie without HttpOnly)
//   3. POST with matching X-CSRF-Token header → 200 accepted
//   4. POST without/with wrong token → 403 rejected

const SOURCE: &str = r#"
mlogserver {
  port: 0
  middleware: [session, csrf, security_headers]
  route "/" method=GET { return "hello" }
  route "/data" method=POST { return "posted" }
}
"#;

/// Extract _mlog_csrf value from Set-Cookie header.
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

/// Verify that the _mlog_csrf Set-Cookie does NOT contain HttpOnly.
fn csrf_cookie_lacks_httponly(headers: &reqwest::header::HeaderMap) -> bool {
    headers
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .any(|v| {
            v.to_str()
                .map(|s| s.contains("_mlog_csrf=") && !s.contains("HttpOnly"))
                .unwrap_or(false)
        })
}

#[tokio::test]
async fn test_csrf_cookie_no_httponly() {
    let (port, _handle) = metalogos::server::run_test_server(SOURCE)
        .await
        .expect("server should start");
    let url = format!("http://127.0.0.1:{}/", port);
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 200);

    // The CSRF cookie must be set without HttpOnly
    assert!(
        csrf_cookie_lacks_httponly(resp.headers()),
        "_mlog_csrf cookie must NOT have HttpOnly flag"
    );
}

#[tokio::test]
async fn test_csrf_double_submit_accept() {
    let (port, _handle) = metalogos::server::run_test_server(SOURCE)
        .await
        .expect("server should start");
    let base = format!("http://127.0.0.1:{}", port);

    // Step 1: GET — obtain CSRF token
    let resp = reqwest::get(&format!("{}/", base)).await.unwrap();
    assert_eq!(resp.status(), 200);
    let token = extract_csrf_from_set_cookie(resp.headers())
        .expect("GET should set _mlog_csrf cookie");

    // Step 2: POST with matching token → accepted (200)
    let client = reqwest::Client::new();
    let resp = client
        .post(&format!("{}/data", base))
        .header("X-CSRF-Token", &token)
        .header("Cookie", format!("_mlog_csrf={}", token))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "POST with valid double-submit should be accepted");
    assert_eq!(resp.text().await.unwrap(), "posted");
}

#[tokio::test]
async fn test_csrf_double_submit_reject_missing() {
    let (port, _handle) = metalogos::server::run_test_server(SOURCE)
        .await
        .expect("server should start");
    let base = format!("http://127.0.0.1:{}", port);

    // GET to get a session
    let _ = reqwest::get(&format!("{}/", base)).await.unwrap();

    // POST without any CSRF token → 403
    let client = reqwest::Client::new();
    let resp = client
        .post(&format!("{}/data", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "POST without CSRF token should be rejected");
}

#[tokio::test]
async fn test_csrf_double_submit_reject_wrong() {
    let (port, _handle) = metalogos::server::run_test_server(SOURCE)
        .await
        .expect("server should start");
    let base = format!("http://127.0.0.1:{}", port);

    // GET — obtain real token
    let resp = reqwest::get(&format!("{}/", base)).await.unwrap();
    let real_token = extract_csrf_from_set_cookie(resp.headers())
        .expect("GET should set _mlog_csrf cookie");

    // POST with wrong token → 403
    let client = reqwest::Client::new();
    let resp = client
        .post(&format!("{}/data", base))
        .header("X-CSRF-Token", "wrong_token_value")
        .header("Cookie", format!("_mlog_csrf={}", real_token))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "POST with wrong CSRF token should be rejected");
}
