//! №1735 (issue #1735, P0 — OUT of the release train, the owner's
//! decision 03.11): the entry's `profile capability { allow_carrier: … }`
//! roster must reach BOTH serve context builders — the boot merge
//! (`merge_interpreter` → `clone_definitions_into`) and the fresh program
//! context (`fresh_program_context` → the same clone). The native cron
//! tick executes on the tree-walking interpreter whatever the serve
//! backend is (№426), and the TW route bodies share that constructor —
//! on 0.38.0 neither leg carried the roster, so a correctly declared
//! office refused every sharp carrier from a native cron tick with
//! IRREVERSIBLE_NO_GRANT (the FOSVED office night cycle, morning brief,
//! admin reply and LZ daily were silently down), and an interpreter-
//! backend route refused the same way (the original issue repro).
//!
//! The pins run the REAL `mlog serve` process (CARGO_BIN_EXE_mlog), BOTH
//! backends, per the owner's decision:
//!   1. a native cron tick calling `http_post` under a valid
//!      `allow_carrier` → NOT refused (the outcome KV records the
//!      round-trip; the local receiver sees the ping);
//!   2. the same program WITHOUT the declaration → `IRREVERSIBLE_NO_GRANT`
//!      (the gate stays fail-closed by default; the receiver sees NO ping
//!      — the refusal is BEFORE the action, and the TW route reproduces
//!      the issue's original 500 table row).
//!
//! The gate itself is untouched: no new grant source (the roster
//! originates only in `permits_from_declarations` / the VM wire field),
//! PROFILE_PERMIT_SHARP / GRANT_ALWAYS / the refusal vocabulary do not
//! move (ADR-0203 §2.4).

#![cfg(feature = "server")]
// The test harness (not program-influenced code) manages its temp files
// and child processes directly — the №475 fs_gate restricts
// PROGRAM-INFLUENCED paths.
#![allow(clippy::disallowed_methods)]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// One recorded request (path + body) seen by the local receiver.
type Hits = Arc<Mutex<Vec<String>>>;

fn temp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "n1735-{}-{}-{}",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|t| t.subsec_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::create_dir_all(&d);
    d
}

/// The serve program. The tick target is a PATTERN (the office shape:
/// `[cron] firing: 0 5 * * * — NightTickSlot`); the receiver URL is a
/// LITERAL baked into the generated file (the static taint lane of the
/// REAL `mlog serve` boot refuses `kv_set`/`http_post` over
/// `json_body()`-tainted arguments — the Category A invariant — so the
/// harness trusts no program input at all). The tick verdict lands in
/// the process-global KV (literal prefixes; the network response is
/// NEVER persisted — only `/probe` returns it to its own caller), and
/// the cron job id rides KV so `/force` passes the same lane without a
/// request body.
fn serve_source(serve_port: u16, receiver_port: u16, with_declaration: bool) -> String {
    let capability_line = if with_declaration {
        "profile capability { allow_carrier: \"http_post\" }\n"
    } else {
        ""
    };
    format!(
        r#"{capability_line}pattern tick_probe() -> Unit {{
  let r = try http_post("http://127.0.0.1:{receiver_port}/ping", "{{}}", "application/json")
  if r.ok {{
    kv_set("outcome", "TICK-OK")
  }} else {{
    kv_set("outcome", "TICK-ERR " + r.error.code)
  }}
}}
mlogserver {{
  port: {serve_port}
  host: "127.0.0.1"
  route "/setup" method=POST {{
    let j = cron_add("* * * * *", "tick_probe")
    kv_set("job_id", j.id)
    return j.id
  }}
  route "/force" method=POST {{
    let id = kv_get("job_id")
    let _m = approval_issue("serve-operator", "carrier:cron_run", 600)
    let r = cron_run(id)
    return r.status
  }}
  route "/outcome" method=POST {{
    return kv_get("outcome")
  }}
  route "/probe" method=GET {{
    return to_string(http_post("http://127.0.0.1:{receiver_port}/ping", "{{}}", "application/json"))
  }}
}}
"#
    )
}

/// The local loopback sink (the issue's `{"reply":"pong"}` receiver):
/// a minimal HTTP/1.1 responder on 127.0.0.1:<ephemeral>. Every request
/// body is recorded; the answer is byte-identical to the issue's.
async fn spawn_receiver(hits: Hits) -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("receiver bind");
    let port = listener.local_addr().expect("receiver addr").port();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                continue;
            };
            // Read until the full request (headers + Content-Length body)
            // is buffered; record the body; answer the fixed pong.
            let mut buf: Vec<u8> = Vec::new();
            let mut chunk = [0u8; 4096];
            let header_end = loop {
                match sock.read(&mut chunk).await {
                    Ok(0) => break None,
                    Ok(n) => {
                        buf.extend_from_slice(&chunk[..n]);
                        if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                            break Some(p);
                        }
                        if buf.len() > 64 * 1024 {
                            break None;
                        }
                    }
                    Err(_) => break None,
                }
            };
            let body = header_end.and_then(|p| {
                let headers = String::from_utf8_lossy(&buf[..p]).to_string();
                let len: usize = headers
                    .lines()
                    .find_map(|l| {
                        let (k, v) = l.split_once(':')?;
                        k.eq_ignore_ascii_case("content-length")
                            .then(|| v.trim().parse().ok())?
                    })
                    .unwrap_or(0);
                let start = p + 4;
                (buf.len() >= start + len)
                    .then(|| String::from_utf8_lossy(&buf[start..start + len]).to_string())
            });
            if let Some(body) = body {
                let path = header_end
                    .and_then(|p| {
                        String::from_utf8_lossy(&buf[..p])
                            .lines()
                            .next()
                            .map(|s| s.to_string())
                    })
                    .unwrap_or_default();
                hits.lock()
                    .expect("hits lock")
                    .push(format!("{path} :: {body}"));
            }
            let resp = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                        Content-Length: 16\r\nConnection: close\r\n\r\n{\"reply\":\"pong\"}";
            let _ = sock.write_all(resp.as_bytes()).await;
            let _ = sock.flush().await;
            let _ = sock.shutdown().await;
        }
    });
    port
}

/// A free 127.0.0.1 port for the serve process (the `mlog serve` print
/// happens BEFORE the bind, so `port: 0` would not reveal the address).
fn pick_serve_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .expect("probe bind")
        .local_addr()
        .expect("probe addr")
        .port()
}

struct ChildGuard(std::process::Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Boot the REAL `mlog serve` process on the written file and drain its
/// stderr into a shared string (the negative leg asserts the refusal
/// class on the log surface too).
fn spawn_serve(file: &Path, backend: &str) -> (ChildGuard, Arc<Mutex<String>>) {
    use std::process::{Command, Stdio};
    let mut child = Command::new(env!("CARGO_BIN_EXE_mlog"))
        .args(["serve", file.to_str().expect("utf8 path")])
        .env("METALOGOS_HTTP_ALLOW_PRIVATE", "1")
        .env("METALOGOS_SERVE_BACKEND", backend)
        .current_dir(file.parent().expect("temp dir"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("mlog serve must spawn");
    let stderr = child.stderr.take().expect("stderr pipe");
    let log = Arc::new(Mutex::new(String::new()));
    let log2 = Arc::clone(&log);
    std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(stderr);
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if let Ok(mut l) = log2.lock() {
                        l.push_str(&String::from_utf8_lossy(&buf[..n]));
                    }
                }
            }
        }
    });
    (ChildGuard(child), log)
}

async fn post_json(
    port: u16,
    path: &str,
    body: serde_json::Value,
) -> Result<(reqwest::StatusCode, String), reqwest::Error> {
    let resp = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{}{}", port, path))
        .json(&body)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    let status = resp.status();
    let text = resp.text().await.expect("body");
    Ok((status, text))
}

/// Poll /setup until the serve process answers (boot readiness), then
/// return the registered cron job id (no request body — the static taint
/// lane of the real boot refuses json_body-tainted sinks).
async fn setup(port: u16, tag: &str) -> String {
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    loop {
        // Connection errors are RETRYABLE (the child is still booting).
        let attempt = post_json(port, "/setup", serde_json::json!({})).await;
        match attempt {
            Ok((reqwest::StatusCode::OK, body)) => {
                let id = body.trim().to_string();
                assert!(
                    !id.is_empty(),
                    "{tag}: /setup must return the cron job id, got: {body}"
                );
                return id;
            }
            other => {
                assert!(
                    std::time::Instant::now() < deadline,
                    "{tag}: /setup must answer 200 within the deadline (boot), \
                     last: {other:?}"
                );
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
        }
    }
}

/// Force one tick (`cron_run` — the №418 manual override) and poll
/// /outcome until the tick's KV verdict lands (the scheduler's 5s pass).
async fn force_and_poll_outcome(port: u16, id: &str, tag: &str) -> String {
    let (status, body) = post_json(port, "/force", serde_json::json!({ "id": id }))
        .await
        .expect("/force must connect (the server is up)");
    assert_eq!(
        status,
        reqwest::StatusCode::OK,
        "{tag}: /force must answer 200, got {status}: {body}"
    );
    assert!(
        body.contains("queued"),
        "{tag}: cron_run must queue the job, got: {body}"
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(40);
    loop {
        let body = post_json(port, "/outcome", serde_json::json!({}))
            .await
            .expect("/outcome must connect (the server is up)")
            .1;
        if body.contains("TICK-") {
            return body;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{tag}: the tick verdict must land within the deadline (the \
             scheduler's 5s pass), last: {body}"
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

async fn run_scenario(backend: &str, tag: &str) {
    let dir = temp_dir(tag);
    let serve_port = pick_serve_port();
    let hits: Hits = Arc::new(Mutex::new(Vec::new()));
    let receiver_port = spawn_receiver(Arc::clone(&hits)).await;

    // ── leg 1: the VALID declaration — the tick and the route both pass ──
    let ok_file = dir.join("n1735_declared.mlog");
    std::fs::write(&ok_file, serve_source(serve_port, receiver_port, true)).expect("write ok file");
    let (child, _log) = spawn_serve(&ok_file, backend);
    let id = setup(serve_port, tag).await;

    let outcome = force_and_poll_outcome(serve_port, &id, tag).await;
    assert!(
        outcome.contains("TICK-OK"),
        "{tag}: the native cron tick must NOT refuse under a valid \
         allow_carrier, got: {outcome}"
    );

    // The issue's ORIGINAL repro surface: the route-body http_post.
    let resp = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{serve_port}/probe"))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .expect("probe must connect");
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::OK,
        "{tag}: the route-body http_post must answer 200 under the \
         declaration (the issue's vm table row, now on {backend} too)"
    );
    let probe_body = resp.text().await.expect("probe body");
    assert!(
        probe_body.contains("pong"),
        "{tag}: the route probe must carry the receiver's body, got: {probe_body}"
    );

    let ok_hits = hits.lock().expect("hits lock").len();
    assert!(
        ok_hits >= 2,
        "{tag}: the receiver must see the tick ping AND the probe ping, \
         got: {:?}",
        hits.lock().expect("hits lock")
    );
    drop(child);
    hits.lock().expect("hits lock").clear();

    // ── leg 2: the SAME program WITHOUT the declaration — fail-closed ──
    let serve_port2 = pick_serve_port();
    let no_file = dir.join("n1735_undeclared.mlog");
    std::fs::write(&no_file, serve_source(serve_port2, receiver_port, false))
        .expect("write no file");
    let (child, log) = spawn_serve(&no_file, backend);
    let id2 = setup(serve_port2, tag).await;

    let outcome2 = force_and_poll_outcome(serve_port2, &id2, tag).await;
    assert!(
        outcome2.contains("TICK-ERR"),
        "{tag}: the undeclared tick must refuse (the try verdict lands in \
         KV), got: {outcome2}"
    );
    assert!(
        !outcome2.contains("TICK-OK"),
        "{tag}: the undeclared tick must NOT succeed, got: {outcome2}"
    );
    let hits2 = hits.lock().expect("hits lock").len();
    assert_eq!(
        hits2,
        0,
        "{tag}: the refusal is BEFORE the action — the receiver must see \
         NO ping from the undeclared program, got: {:?}",
        hits.lock().expect("hits lock")
    );
    // The issue's original 500 row: the route-body refusal on the wire.
    let resp2 = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{serve_port2}/probe"))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .expect("probe2 must connect");
    let status2 = resp2.status();
    let body2 = resp2.text().await.expect("probe2 body");
    assert_eq!(
        status2,
        reqwest::StatusCode::INTERNAL_SERVER_ERROR,
        "{tag}: the undeclared route must refuse 500, got {status2}: {body2}"
    );
    // №955 (the audit 12.11 H-3): the body is the GENERIC id-form — the
    // frozen class never reaches the client; the class rides the serve
    // stderr (the next assert — the office evidence surface, unchanged).
    assert!(
        body2.contains("Internal error (id: err-"),
        "{tag}: the undeclared route refusal must be the generic №955 id-form, \
         got: {body2}"
    );
    assert!(
        !body2.contains("IRREVERSIBLE_NO_GRANT"),
        "{tag}: the frozen class must NOT reach the client (№955), got: {body2}"
    );
    assert!(
        log.lock()
            .expect("log lock")
            .contains("IRREVERSIBLE_NO_GRANT"),
        "{tag}: the serve stderr must carry the refusal class (the office \
         evidence surface), got: {}",
        log.lock().expect("log lock")
    );
    drop(child);
    let _ = std::fs::remove_dir_all(&dir);
}

/// BOTH backends, sequential (the №496 house posture): each scenario
/// boots its OWN child process (per-process KV globals, OS-assigned
/// ports), but the deterministic order keeps the diagnostics linear —
/// the TW interpreter lane first (the leg the defect was filed on),
/// then the VM serve default.
#[tokio::test]
async fn n1735_tick_capability_context_both_backends() {
    run_scenario("interpreter", "tw").await;
    run_scenario("vm", "vm").await;
}
