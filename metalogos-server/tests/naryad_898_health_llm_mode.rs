// ── tests/naryad_898_health_llm_mode.rs ──────────────────────────────
// №898 (issue #1792, P1, [core][llm][process]): the built-in /health
// names the LLM posture in the `x-llm-mode` header — mock (the explicit
// METALOGOS_MOCK_LLM=1), declared (an external provider is configured),
// unconfigured (every LLM call will refuse at the first call — the №898
// loud posture). The BODY stays exactly "ok" (the №511 docker/uptime
// pin is untouched — the header is the operator-visible addition).

#![cfg(feature = "server")]

use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::Mutex;

// The async-aware env serialization (the №278 posture): the guard is
// held across the awaits legally (tokio Mutex is Send + async-safe —
// the std guard across await is the clippy await_holding_lock class).
static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn env_lock() -> &'static Mutex<()> {
    ENV_LOCK.get_or_init(|| Mutex::new(()))
}

async fn get_response(port: u16, path: &str) -> reqwest::Response {
    let resp = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}{path}"))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .expect("the health request must answer");
    resp
}

fn strip_all_llm_env() {
    std::env::remove_var("METALOGOS_MOCK_LLM");
    std::env::remove_var("METALOGOS_LLM_PROVIDER");
    std::env::remove_var("METALOGOS_API_KEY");
    std::env::remove_var("METALOGOS_OPENAI_BASE_URL");
}

// ── mock mode: the header names the explicit mock ─────────────────────
#[tokio::test]
async fn n898_health_x_llm_mode_mock() {
    let _env = env_lock().lock().await;
    strip_all_llm_env();
    std::env::set_var("METALOGOS_MOCK_LLM", "1");

    let src = r#"
mlogserver {
  port: 0
  route "/ping" method=GET {
    return respond("200 pong")
  }
}
"#;
    let (port, _handle) = metalogos_server::server::run_test_server(src)
        .await
        .expect("test server must start");

    let resp = get_response(port, "/health").await;
    assert_eq!(resp.status().as_u16(), 200);
    let mode = resp
        .headers()
        .get("x-llm-mode")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert_eq!(
        resp.text().await.unwrap(),
        "ok",
        "the /health body must stay exactly 'ok' (the №511 pin)"
    );
    assert_eq!(
        mode, "mock",
        "with METALOGOS_MOCK_LLM=1 the /health must carry x-llm-mode: mock"
    );
    strip_all_llm_env();
}

// ── unconfigured: the header warns before the first LLM call ──────────
#[tokio::test]
async fn n898_health_x_llm_mode_unconfigured() {
    let _env = env_lock().lock().await;
    strip_all_llm_env();

    let src = r#"
mlogserver {
  port: 0
  route "/ping" method=GET {
    return respond("200 pong")
  }
}
"#;
    let (port, _handle) = metalogos_server::server::run_test_server(src)
        .await
        .expect("test server must start");

    let resp = get_response(port, "/health").await;
    assert_eq!(resp.status().as_u16(), 200);
    let mode = resp
        .headers()
        .get("x-llm-mode")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert_eq!(
        mode, "unconfigured",
        "with nothing declared the /health must carry x-llm-mode: unconfigured \
         (the operator sees the №898 refusal posture before any request)"
    );
    strip_all_llm_env();
}

// ── declared: the header names the configured external lane ───────────
#[tokio::test]
async fn n898_health_x_llm_mode_declared() {
    let _env = env_lock().lock().await;
    strip_all_llm_env();
    std::env::set_var("METALOGOS_LLM_PROVIDER", "anthropic");

    let src = r#"
mlogserver {
  port: 0
  route "/ping" method=GET {
    return respond("200 pong")
  }
}
"#;
    let (port, _handle) = metalogos_server::server::run_test_server(src)
        .await
        .expect("test server must start");

    let resp = get_response(port, "/health").await;
    assert_eq!(resp.status().as_u16(), 200);
    let mode = resp
        .headers()
        .get("x-llm-mode")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert_eq!(
        mode, "declared",
        "with an external declaration the /health must carry x-llm-mode: declared"
    );
    strip_all_llm_env();
}
