// ── tests/naryad_511_docker_health.rs ───────────────────────────────────
// №511 (P1, deploy/ci; the audit 28.09 C-05, Medium): the deploy path is
// verified. The old Dockerfile died at clap (`serve` needs a file — the
// `file` arg is NOT optional), shipped no program, carried a dead
// METALOGOS_PORT, and was checked by nothing. The container-side fixes
// (Dockerfile, .dockerignore, the example program, the docker.yml CI
// job) are exercised LIVE in .github/workflows/docker.yml (build → run
// → /health → `mlog health` → the HEALTHCHECK state). THIS file pins the
// server-side behavior the deploy story stands on:
//   T1 the built-in GET /health answers 200 "ok" with NO side effects
//      (no program route involved — the answer exists even when every
//      program route is broken; the probe target the HEALTHCHECK uses);
//   T2 a program declaring its OWN /health path keeps it (the builtin
//      is skipped — no duplicate-route panic, the declaration wins);
//   T3 the bind-host ladder: the `host:` declaration wins, then
//      METALOGOS_HOST (the deploy fallback — the image sets
//      METALOGOS_HOST=0.0.0.0, the program needs no edit), then the
//      №164 loopback default.
// The mlog health exit-code semantics (0 on 200 / 1 otherwise) run live
// in the docker.yml steps — a probe test spawning the real binary would
// duplicate the CI exactly.

#![cfg(feature = "server")]

use std::time::Duration;

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

async fn get_body(port: u16, path: &str) -> (u16, String) {
    let resp = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{}{}", port, path))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .expect("GET should succeed");
    let status = resp.status().as_u16();
    let body = resp.text().await.expect("body");
    (status, body)
}

// ── T1: the built-in /health — 200 "ok", no side effects ────────────────

#[tokio::test]
async fn n511_builtin_health_route_answers_ok() {
    let source = r#"
mlogserver {
  port: 0
  route "/hello" method=GET {
    return respond("200 hello")
  }
}
"#;
    let (port, _handle) = metalogos_server::server::run_test_server(source)
        .await
        .expect("test server must start");

    let (status, body) = get_body(port, "/health").await;
    assert_eq!(status, 200, "the built-in /health must answer 200");
    assert_eq!(body, "ok", "the built-in /health body must be exactly 'ok'");

    // A program route still works through the same router.
    let (status, body) = get_body(port, "/hello").await;
    assert_eq!(status, 200);
    assert_eq!(
        body, "hello",
        "respond(\"200 x\") serves the body without the status prefix"
    );
}

// ── T2: a program-declared /health wins (no duplicate-route panic) ──────

#[tokio::test]
async fn n511_program_declared_health_route_wins() {
    let source = r#"
mlogserver {
  port: 0
  route "/health" method=GET {
    return respond("200 program-health")
  }
}
"#;
    // The assertion here is ALSO that the server starts at all: a naive
    // built-in registration would panic on the duplicate /health path.
    let (port, _handle) = metalogos_server::server::run_test_server(source)
        .await
        .expect("a program declaring /health must not panic the router");

    let (status, body) = get_body(port, "/health").await;
    assert_eq!(status, 200);
    assert_eq!(
        body, "program-health",
        "the PROGRAM's /health must answer, not the built-in one"
    );
}

// ── T3: the bind-host ladder ────────────────────────────────────────────

#[test]
fn n511_bind_host_ladder_declaration_env_default() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // (1) The declaration ALWAYS wins — even when the env is set.
    std::env::set_var("METALOGOS_HOST", "0.0.0.0");
    assert_eq!(
        metalogos_server::server::resolve_bind_host(Some("127.0.0.1".to_string())),
        "127.0.0.1",
        "the program's host: declaration must beat the deploy env (№164 boundary)"
    );

    // (2) No declaration + the env → the deploy fallback.
    assert_eq!(
        metalogos_server::server::resolve_bind_host(None),
        "0.0.0.0",
        "METALOGOS_HOST must be the fallback when the program declares no host:"
    );

    // (3) No declaration + no env → the №164 loopback default.
    std::env::remove_var("METALOGOS_HOST");
    assert_eq!(
        metalogos_server::server::resolve_bind_host(None),
        "127.0.0.1",
        "without the declaration and the env the №164 loopback default holds"
    );
}
