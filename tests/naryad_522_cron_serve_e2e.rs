//! №522 (Wave 22, dispatch gh#845; the №509 functional criterion,
//! ADR-0179 §2.5) — the cron accumulation arc through the SERVING path.
//! The №496 образец: a REAL HTTP server, BOTH serve backends, one
//! sequential test fn.
//!
//! What is exercised (the naryad's scope: the accumulation arc, NOT the
//! №457 security gate — that stays pinned by its own suite):
//!
//! 1. The job registration: a POST route calls `cron_add("* * * * *",
//!    "tick_job")` — the job lands in the KV-backed job store, which is
//!    persisted through the `memory { persist: "…" }` declaration (the
//!    KV SQLite write-through; the "cron_jobs" key).
//! 2. The ticks FIRE through the scheduler: `cron_run(id)` marks the
//!    job force_run (the №418 manual override — no waiting for a minute
//!    window); the scheduler's 5-second pass executes the target (the
//!    №426 contract: the tick runs in the PROGRAM context — a fresh
//!    per-tick context, a blocking thread) via `cron_mark_fired`-stamped
//!    dispatch, and the pattern body increments a KV counter
//!    (`tick_n`). The accumulation is visible in the /ticks route
//!    response: the count GROWS with every tick.
//! 3. The RESTART leg: the server restarts on the same persist file —
//!    the JOB survives (the /force route still reports "queued", i.e.
//!    the job was found in the reloaded store) and the COUNTER
//!    continues from the pre-restart value (the KV SQLite
//!    load-back — "SQLite is authoritative on init"), proving both the
//!    registered job and the accumulated state survive the restart.
//!
//! Backend note (honest): the tick EXECUTION path (`execute_tick_call`,
//! the №426 contract) is the shared program-context executor — it is
//! the same machinery under both serve backends, and the KV store it
//! accumulates through is process-global + persisted. The test runs the
//! full scenario under BOTH serve backends and pins the identical arc.
//!
//! The security gates (№457 tick env/exec posture) are NOT re-tested
//! here — their own suites pin them.

#![cfg(feature = "server")]
// The test harness (not program-influenced code) manages its temp files
// directly — the №475 fs_gate restricts PROGRAM-INFLUENCED paths.
#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;
use std::time::Duration;

fn temp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("n522-{}-{}", tag, std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

async fn post_json(
    port: u16,
    path: &str,
    body: serde_json::Value,
) -> (reqwest::StatusCode, String) {
    let resp = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{}{}", port, path))
        .json(&body)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .expect("POST should succeed");
    let status = resp.status();
    let text = resp.text().await.expect("body");
    (status, text)
}

fn source(persist_path: &str) -> String {
    format!(
        r#"
memory {{ persist: "{persist_path}" }}
pattern tick_job() -> Unit {{
  let n = kv_get("tick_n")
  let v = to_float_or(n, 0)
  kv_set("tick_n", to_string(v + 1))
}}
mlogserver {{
  port: 0
  route "/setup" method=POST {{
    let j = cron_add("* * * * *", "tick_job")
    return j.id
  }}
  route "/force" method=POST {{
    let body = json_body()
    let r = cron_run(body.id)
    return r.status
  }}
  route "/ticks" method=POST {{
    return kv_get("tick_n")
  }}
}}
"#
    )
}

fn count_of(body: &str) -> f64 {
    body.trim().parse::<f64>().unwrap_or(0.0)
}

/// Force one tick (`cron_run` → the scheduler's next 5s pass) and poll
/// /ticks until the counter reaches `at_least`.
async fn force_and_wait(port: u16, id: &str, at_least: f64, tag: &str) {
    let (status, body) = post_json(port, "/force", serde_json::json!({ "id": id })).await;
    assert_eq!(
        status,
        reqwest::StatusCode::OK,
        "{}: the /force route must answer 200, got {}: {}",
        tag,
        status,
        body
    );
    assert!(
        body.contains("queued"),
        "{}: cron_run must queue the job (the store holds it), got: {}",
        tag,
        body
    );
    // The scheduler's 5-second pass picks the force_run up; poll past it.
    let deadline = std::time::Instant::now() + Duration::from_secs(25);
    loop {
        let (_, body) = post_json(port, "/ticks", serde_json::json!({})).await;
        if count_of(&body) >= at_least {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{}: the tick counter must reach {} within the deadline \
             (the scheduler must fire the forced tick), last: {}",
            tag,
            at_least,
            body
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

async fn run_scenario(backend: metalogos::server::ServeBackend, tag: &str) {
    let dir = temp_dir(tag);
    let db_path = dir.join("cron-persist.db");
    let _ = std::fs::remove_file(&db_path); // a fresh store per scenario
    let src = source(db_path.to_str().expect("utf8 path"));

    // ── boot 1: register the job, force ticks, the counter accumulates ──
    let (port, handle, _state) =
        metalogos::server::run_test_server_with_backend_state_in_dir(&src, backend, dir.clone())
            .await
            .expect("boot 1 must start");

    let (status, body) = post_json(port, "/setup", serde_json::json!({})).await;
    assert_eq!(
        status,
        reqwest::StatusCode::OK,
        "{}: the /setup route must answer 200, got {}: {}",
        tag,
        status,
        body
    );
    let job_id = body.trim().to_string();
    assert!(
        !job_id.is_empty(),
        "{}: cron_add must return the job id, got: {}",
        tag,
        job_id
    );

    // Two forced ticks pre-restart: the counter accumulates.
    force_and_wait(port, &job_id, 1.0, tag).await;
    force_and_wait(port, &job_id, 2.0, tag).await;

    // ── boot 2 (RESTART): the job AND the counter survive ──
    handle.abort();
    let _ = handle.await;

    let (port2, handle2, _state2) =
        metalogos::server::run_test_server_with_backend_state_in_dir(&src, backend, dir.clone())
            .await
            .expect("boot 2 must start");

    // The forced tick on the RESTARTED server: the job is found in the
    // reloaded store ("queued", not "not_found") and the counter
    // CONTINUES from the pre-restart value (the KV load-back) instead of
    // restarting from zero.
    force_and_wait(port2, &job_id, 3.0, tag).await;

    handle2.abort();
    let _ = handle2.await;
    let _ = std::fs::remove_dir_all(&dir);
}

/// ONE test fn, BOTH backends sequentially (the №496 pattern): the KV
/// store and the cron job store are process-global — parallel fns would
/// race on the shared "cron_jobs"/"tick_n" state.
#[tokio::test]
async fn n522_cron_serve_e2e_both_backends() {
    // The TW interpreter lane first, then the VM (the serve default).
    run_scenario(metalogos::server::ServeBackend::Interpreter, "tw").await;
    run_scenario(metalogos::server::ServeBackend::Vm, "vm").await;
}
