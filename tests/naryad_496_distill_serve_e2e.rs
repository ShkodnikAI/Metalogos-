//! №496 (Wave 19, dispatch gh#793) — the audit 28.09 §3.1 e2e
//! reproduction, the report's own headline recommendation: "every
//! function accumulating state must have an end-to-end test through the
//! mode it will be used in." The 11 pre-existing distill test files never
//! went through `run_test_server`/serve — that is exactly why the №495
//! defect (serve distillation loses its examples with every request; the
//! training threshold `max(distill_after, 10)` unreachable by
//! construction, no error, no audit event) was invisible to CI.
//!
//! The test drives a REAL HTTP server (`run_test_server_with_backend_…`)
//! with `METALOGOS_MOCK_LLM=1` and a `learnable pattern` wired through
//! `distill_to`:
//!   1. `distill_after + 10` POST requests accumulate examples through
//!      the hub (№495) — the mock counter grows while TEACHING;
//!   2. the audit events `distill.training-started` /
//!      `distill.training-finished … switched=true` arrive (the hub's
//!      ONE background trainer; both backends);
//!   3. the NEXT requests stop calling the LLM — the mock call counter
//!      freezes (DISTILLED answers, no LLM);
//!   4. persistence: the server RESTARTS on the same memory-persist file
//!      and the accumulated examples survive (`distill_samples`,
//!      №495) — the reloaded hub re-trains and freezes the counter
//!      again, without fresh LLM traffic.
//!
//! Red-before/green-after: on the pre-№495 merge-base this test FAILS at
//! step 3 (the counter never freezes — behavior indistinguishable from
//! "not enough data yet", the audit's exact wording); the red log is
//! attached to the №496 report. Steps 1–2 compile only on the post-№495
//! surface (the hub + the state-returning test helper are the №495
//! deliverables); the red-run shim swaps this one helper call and is
//! documented in the report.
//!
//! Both serve backends (TW interpreter and VM — the serve default) run
//! the same scenario sequentially in ONE test fn: the mock call counter
//! and the `METALOGOS_MOCK_LLM` env are process-global (№454), so two
//! parallel test fns would race on them.

#![cfg(feature = "server")]
// The test harness (not program-influenced code) manages its temp files
// directly — the №475 fs_gate restricts PROGRAM-INFLUENCED paths.
#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;
use std::time::Duration;

/// The mock answers `mock_response(prompt)` — the pattern's effective
/// prompt is the constant `"answer"` (no context), so the LLM "label" is
/// a CONSTANT the test computes and declares. Every recorded example
/// then carries the SAME closed label (the №485 single-class carve-out:
/// the majority baseline is trivial, the raw `distill_min_accuracy`
/// gate applies, and the holdout accuracy of an all-one-label split is
/// 1.0) — the honest deterministic way to reach the DISTILLED switch
/// under the network-free mock.
fn distill_source(persist_path: &str) -> String {
    let mock_label = metalogos::llm::mock_response("answer");
    format!(
        r#"
memory {{ persist: "{persist_path}" }}
reflex Head {{
  input: embedding(4)
  layers: [dense(4, "relu"), dense(2, "softmax")]
  labels: ["{mock_label}", "other"]
  seed: 42
}}
learnable pattern Ask(q: String) -> String {{
  prompt: "answer"
  distill_to: Head
  distill_after: 20
}}
mlogserver {{
  port: 0
  route "/ask" method=POST {{
    let q = json_body("q")
    let r = Ask(q)
    return r
  }}
}}
"#
    )
}

fn temp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("n496-{}-{}", tag, std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

async fn post_ask(port: u16, q: &str) -> String {
    let resp = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{}/ask", port))
        .json(&serde_json::json!({ "q": q }))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .expect("POST /ask should succeed");
    let status = resp.status();
    let body = resp.text().await.expect("body");
    assert_eq!(
        status,
        reqwest::StatusCode::OK,
        "the distill route must answer 200, got {}: {}",
        status,
        body
    );
    body
}

fn persisted_rows(db_path: &PathBuf) -> usize {
    let conn = rusqlite::Connection::open(db_path).expect("open distill_samples db");
    conn.query_row("SELECT COUNT(*) FROM distill_samples", [], |row| {
        row.get::<_, i64>(0)
    })
    .map(|n| n as usize)
    .expect("count distill_samples rows")
}

/// Wait until the audit log holds the `switched=true` training verdict
/// (the hub's background trainer lands it in the SHARED audit log — the
/// №495 fix; the pre-№495 mailbox died with the request and no event
/// ever arrived).
async fn await_switched(
    state: &std::sync::Arc<metalogos::server::ServerState>,
    timeout: Duration,
) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        {
            let log = state.audit_log.read().await;
            if log
                .iter()
                .any(|l| l.contains("distill.training-finished") && l.contains("switched=true"))
            {
                return true;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

/// Step 3: the NEXT requests must not call the LLM. Polls because the
/// switch lands asynchronously (the №489/№495 background posture): each
/// round fires 3 probes and compares the mock counter around them.
async fn await_counter_frozen(port: u16, rounds: usize) -> bool {
    for _ in 0..rounds {
        let before = metalogos::llm::MockLlm::call_count();
        for i in 0..3 {
            let _ = post_ask(port, &format!("probe-{}", i)).await;
        }
        let after = metalogos::llm::MockLlm::call_count();
        if after == before {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    false
}

async fn run_scenario(backend: metalogos::server::ServeBackend, tag: &str) {
    metalogos::llm::MockLlm::reset_call_count();
    metalogos::llm::MockLlm::reset_delay();

    let dir = temp_dir(tag);
    let db_path = dir.join("distill-e2e.db");
    let _ = std::fs::remove_file(&db_path); // a fresh ledger per scenario

    // ── boot 1: TEACHING accumulates, the trainer switches, LLM stops ──
    let source = distill_source(db_path.to_str().expect("utf8 path"));
    let (port, handle, state) =
        metalogos::server::run_test_server_with_backend_state_in_dir(&source, backend, dir.clone())
            .await
            .expect("boot 1 must start");

    // distill_after: 20 → the training threshold max(20, 10) = 20; the
    // holdout gate (MIN_HOLDOUT = 4) needs 20% of the examples ≥ 4 —
    // exactly 20 examples clear both gates on the first attempt; +5
    // margin — 25 requests of TEACHING traffic.
    for i in 0..25 {
        let _ = post_ask(port, &format!("teaching-{}", i)).await;
    }

    // The persisted ledger holds the examples (the №495 distill_samples
    // table) — every request's example survived the request.
    let rows = persisted_rows(&db_path);
    // ≥ 20 (the teaching threshold) — NOT exactly 25: once the switch
    // lands mid-traffic, the DISTILLED path stops calling the LLM and
    // stops recording (the honest semantics) — the tail requests race
    // the trainer. Pre-№495 this count was 0–1 (the per-request context
    // destroyed every example with its request).
    assert!(
        rows >= 20,
        "distill_samples must hold every TEACHING example (got {} < 20) \
         — the per-request context destroyed them before №495",
        rows
    );

    // The audit events arrive: started + finished switched=true.
    if !await_switched(&state, Duration::from_secs(30)).await {
        let log = state.audit_log.read().await;
        let tail: Vec<String> = log
            .iter()
            .filter(|l| l.contains("distill"))
            .cloned()
            .collect();
        panic!(
            "the distill.training-started/finished audit events must arrive \
             (the hub's background trainer) — pre-№495 serve never reached \
             the training threshold. The distill audit lines were: {:?}",
            tail
        );
    }

    // The NEXT requests take the DISTILLED path: no LLM calls.
    assert!(
        await_counter_frozen(port, 10).await,
        "after the DISTILLED switch the mock counter must freeze — the \
         distilled model answers without the LLM. Pre-№495 this never \
         happens (the audit 28.09 §3.1 red reproduction)."
    );

    // ── boot 2 (RESTART): the accumulated examples survive ──
    handle.abort();
    let _ = handle.await;
    let rows_before_restart = persisted_rows(&db_path);

    let (port2, handle2, state2) =
        metalogos::server::run_test_server_with_backend_state_in_dir(&source, backend, dir.clone())
            .await
            .expect("boot 2 must start");

    // The reloaded hub restores the ledger (honest migration note: the
    // first 0.27.1 boot starts from zero; every later restart reloads).
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let mut reloaded = 0usize;
    while std::time::Instant::now() < deadline {
        reloaded = persisted_rows(&db_path);
        if reloaded >= rows_before_restart {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(
        reloaded, rows_before_restart,
        "the distill_samples ledger must survive the server restart"
    );

    // The reloaded hub re-trains from the PERSISTED examples (no fresh
    // LLM traffic needed to reach the threshold) and freezes again.
    // The training attempt fires on the FIRST call after the restart
    // (the threshold check happens on the call path) — one warm-up POST.
    let _ = post_ask(port2, "restart-warmup").await;
    assert!(
        await_switched(&state2, Duration::from_secs(30)).await,
        "the restarted server must re-train from the persisted examples"
    );
    assert!(
        await_counter_frozen(port2, 10).await,
        "the restarted server must answer distilled (no LLM) — the \
         accumulated state survived the restart"
    );

    handle2.abort();
    let _ = handle2.await;
    let _ = std::fs::remove_dir_all(&dir);
}

/// ONE test fn, BOTH backends sequentially: the mock call counter and
/// the `METALOGOS_MOCK_LLM` env are process-global (№454) — two parallel
/// test fns would race on them.
#[tokio::test]
async fn n496_distill_serve_e2e_both_backends() {
    std::env::set_var("METALOGOS_MOCK_LLM", "1");
    // The TW interpreter lane first, then the VM (the serve default).
    run_scenario(metalogos::server::ServeBackend::Interpreter, "tw").await;
    run_scenario(metalogos::server::ServeBackend::Vm, "vm").await;
}
