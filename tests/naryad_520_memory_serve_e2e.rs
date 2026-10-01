//! №520 (Wave 22, dispatch gh#845; the №509 functional criterion,
//! ADR-0179 §2.5) — the memory accumulation arc through the SERVING
//! path. The №509 lesson: a function accumulating state over time must
//! have an end-to-end test through the mode it is used in —
//! `run_test_server`, BOTH backends — or a serve-only defect (the №495
//! class) stays invisible to CI.
//!
//! Two arcs, one test fn, both serve backends (TW interpreter and VM —
//! the serve default), sequentially (the №496 pattern: the process-global
//! env anchors make parallel fns race):
//!
//! 1. The `memory { persist }` + `memorize`/`recall` lane (the
//!    DECLARATION-driven store):
//!    - the same-request roundtrip (memorize + recall in ONE route body)
//!      works on BOTH backends;
//!    - the CROSS-REQUEST accumulation: on the TW the per-request
//!      interpreter re-opens the SQLite-backed store at the declared
//!      path, so a `recall` in request B finds what request A memorized,
//!      and the store survives a server RESTART (the persistence leg);
//!    - on the VM the simple-memory twin is REQUEST-SCOPED BY DESIGN
//!      (the №442 posture; `Vm::reset_for_reuse` clears `self.memory`
//!      between pooled generations — the isolation class pinned by
//!      naryad_402_step_a): a cross-request `recall` honestly finds
//!      NOTHING. The test PINS that posture — the VM leg asserts the
//!      absence, so an accidental cross-request leak (a reset regression)
//!      turns RED.
//! 2. The typed-memory lane (`memory_open`/`memory_put`/`memory_read`,
//!    the №350 Memory<K> surface): the container registry is
//!    PROCESS-GLOBAL and backend-independent (the subject IS the
//!    address — the deterministic re-open), so the accumulation arc is
//!    identical on BOTH backends: a PUT in request A, a READ in request
//!    B, and a READ after the server RESTART with the persistence
//!    anchors (`METALOGOS_MEMORY_DB` + `METALOGOS_MEMORY_MASTER`,
//!    ADR-0173 §3.5) set — the decrypted value survives the restart.
//!
//! The boundaries (the naryad): the memory security gates (№335 consent,
//! №442 recall ledger) are NOT re-tested here — they are pinned by their
//! own suites; public containers only. This file is exactly the
//! accumulation arc through serve.

#![cfg(feature = "server")]
// The test harness (not program-influenced code) manages its temp files
// directly — the №475 fs_gate restricts PROGRAM-INFLUENCED paths.
#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;
use std::time::Duration;

/// The fixed 64-hex master key: without the env anchor the typed-memory
/// store uses a fresh per-process key and a restart CANNOT decrypt
/// (the №351 posture) — the persistence leg pins the anchored form.
const MASTER_HEX: &str = "ab5d1037f21c6e89b04a55c2d3e71f689c4b2a77e0d1c3f5a6b7c8d9e0f1a2b3";

fn temp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("n520-{}-{}", tag, std::process::id()));
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

fn assert_ok(status: reqwest::StatusCode, body: &str, what: &str) {
    assert_eq!(
        status,
        reqwest::StatusCode::OK,
        "{} must answer 200, got {}: {}",
        what,
        status,
        body
    );
}

// ── Arc 1: the memory { persist } + memorize/recall lane ────────────

fn kv_source(persist_path: &str) -> String {
    format!(
        r#"
memory {{ persist: "{persist_path}" }}
mlogserver {{
  port: 0
  route "/remember" method=POST {{
    let body = json_body()
    let t = body.text
    memorize(t, 1.0)
    return "ok"
  }}
  route "/roundtrip" method=POST {{
    let body = json_body()
    let t = body.text
    memorize(t, 1.0)
    let r = recall(t)
    return r
  }}
  route "/ask" method=POST {{
    let body = json_body()
    let q = body.q
    let r = recall(q)
    return r
  }}
}}
"#
    )
}

async fn kv_arc(backend: metalogos::server::ServeBackend, tag: &str) {
    let dir = temp_dir(&format!("kv-{}", tag));
    let db_path = dir.join("memory-persist.db");
    let _ = std::fs::remove_file(&db_path); // a fresh store per scenario
    let source = kv_source(db_path.to_str().expect("utf8 path"));

    // ── boot 1: accumulate through POST routes ──
    let (port, handle, _state) =
        metalogos::server::run_test_server_with_backend_state_in_dir(&source, backend, dir.clone())
            .await
            .expect("boot 1 must start");

    // The same-request roundtrip works on BOTH backends: memorize then
    // recall inside one route body (one request, one store instance).
    let (status, body) = post_json(
        port,
        "/roundtrip",
        serde_json::json!({
            "text": "n520-roundtrip-needle-alpha"
        }),
    )
    .await;
    assert_ok(status, &body, "the same-request memorize+recall roundtrip");
    assert!(
        body.contains("n520-roundtrip-needle-alpha"),
        "the same-request roundtrip must return the memorized text, got: {}",
        body
    );

    // The CROSS-REQUEST accumulation: a separate request memorizes ONE
    // distinct text…
    let (status, body) = post_json(
        port,
        "/remember",
        serde_json::json!({
            "text": "n520-cross-request-needle"
        }),
    )
    .await;
    assert_ok(status, &body, "the memorize route");
    // …and ANOTHER request recalls it. On the TW the per-request
    // interpreter re-opens the SQLite-backed store at the declared path —
    // the state lives in the FILE, not in the request. On the VM the
    // simple-memory twin is request-scoped by design (the №442 posture,
    // reset_for_reuse) — the recall finds nothing, and the test pins that
    // honest absence (an accidental leak would turn this RED).
    let (status, body) = post_json(
        port,
        "/ask",
        serde_json::json!({
            "q": "n520-cross-request-needle"
        }),
    )
    .await;
    assert_ok(status, &body, "the cross-request recall");
    if matches!(backend, metalogos::server::ServeBackend::Interpreter) {
        assert!(
            body.contains("n520-cross-request-needle"),
            "TW: the persisted store must carry request A's memorize into \
             request B's recall, got: {}",
            body
        );
    } else {
        assert!(
            !body.contains("n520-cross-request-needle"),
            "VM: the simple-memory twin is request-scoped by design \
             (reset_for_reuse clears self.memory) — a cross-request hit \
             would be an isolation leak, got: {}",
            body
        );
    }

    // ── boot 2 (RESTART): the accumulated state survives ──
    handle.abort();
    let _ = handle.await;

    let (port2, handle2, _state2) =
        metalogos::server::run_test_server_with_backend_state_in_dir(&source, backend, dir.clone())
            .await
            .expect("boot 2 must start");

    let (status, body) = post_json(
        port2,
        "/ask",
        serde_json::json!({
            "q": "n520-cross-request-needle"
        }),
    )
    .await;
    assert_ok(status, &body, "the recall after the restart");
    if matches!(backend, metalogos::server::ServeBackend::Interpreter) {
        assert!(
            body.contains("n520-cross-request-needle"),
            "TW: the memory {{ persist }} store must survive the server \
             restart (the whole point of the declaration), got: {}",
            body
        );
    } else {
        assert!(
            !body.contains("n520-cross-request-needle"),
            "VM: nothing to persist by design — the pin holds after the \
             restart too, got: {}",
            body
        );
    }

    handle2.abort();
    let _ = handle2.await;
    let _ = std::fs::remove_dir_all(&dir);
}

// ── Arc 2: the typed-memory lane (memory_open / memory_put / memory_read) ──

fn typed_source(subject: &str) -> String {
    format!(
        r#"
mlogserver {{
  port: 0
  route "/mput" method=POST {{
    let h = memory_open("{subject}", "public")
    let body = json_body()
    let v = body.value
    memory_put(h, "k1", v)
    return "ok"
  }}
  route "/mread" method=POST {{
    let h = memory_open("{subject}", "public")
    let v = memory_read(h, "k1")
    return v
  }}
}}
"#
    )
}

async fn typed_arc(backend: metalogos::server::ServeBackend, tag: &str) {
    let dir = temp_dir(&format!("typed-{}", tag));
    // The subject IS the address (the deterministic re-open) — one per
    // backend leg so the process-global registry never crosses the legs.
    let subject = format!("n520-e2e-{}", tag);
    let source = typed_source(&subject);

    // ── boot 1: put through a POST route ──
    let (port, handle, _state) =
        metalogos::server::run_test_server_with_backend_state_in_dir(&source, backend, dir.clone())
            .await
            .expect("typed boot 1 must start");
    let (status, body) = post_json(
        port,
        "/mput",
        serde_json::json!({ "value": "n520-typed-v1" }),
    )
    .await;
    assert_ok(status, &body, "the memory_put route");

    // The cross-request read: a NEW request re-opens the SAME container
    // (the subject is the address) and reads the entry — the process-
    // global registry carries the state, identically on BOTH backends.
    let (status, body) = post_json(port, "/mread", serde_json::json!({})).await;
    assert_ok(status, &body, "the cross-request memory_read");
    assert!(
        body.contains("n520-typed-v1"),
        "{}: the typed-memory registry must carry the PUT into the next \
         request's READ (the subject is the address), got: {}",
        tag,
        body
    );

    // ── boot 2 (RESTART): the anchored store re-opens and decrypts ──
    handle.abort();
    let _ = handle.await;

    let (port2, handle2, _state2) =
        metalogos::server::run_test_server_with_backend_state_in_dir(&source, backend, dir.clone())
            .await
            .expect("typed boot 2 must start");
    let (status, body) = post_json(port2, "/mread", serde_json::json!({})).await;
    assert_ok(status, &body, "the memory_read after the restart");
    assert!(
        body.contains("n520-typed-v1"),
        "{}: the anchored typed-memory store (METALOGOS_MEMORY_DB + \
         METALOGOS_MEMORY_MASTER) must survive the server restart with \
         the value decryptable, got: {}",
        tag,
        body
    );

    handle2.abort();
    let _ = handle2.await;
    let _ = std::fs::remove_dir_all(&dir);
}

/// ONE test fn, BOTH backends sequentially (the №496 pattern): the
/// process-global env anchors (the mock LLM, the typed-memory DB and
/// master key) make parallel test fns race.
#[tokio::test]
async fn n520_memory_serve_e2e_both_backends() {
    // The typed-memory persistence anchors are process-global (a
    // OnceLock): they MUST be set before the first memory_open call in
    // this process — the restart leg depends on the stable master key.
    let dir = temp_dir("anchor");
    std::env::set_var("METALOGOS_MEMORY_DB", dir.join("typed-memory.db"));
    std::env::set_var("METALOGOS_MEMORY_MASTER", MASTER_HEX);

    // Arc 1 — the declaration-driven memorize/recall lane.
    kv_arc(metalogos::server::ServeBackend::Interpreter, "tw").await;
    kv_arc(metalogos::server::ServeBackend::Vm, "vm").await;
    // Arc 2 — the typed-memory lane, identical on both backends.
    typed_arc(metalogos::server::ServeBackend::Interpreter, "tw").await;
    typed_arc(metalogos::server::ServeBackend::Vm, "vm").await;

    let _ = std::fs::remove_dir_all(&dir);
}
