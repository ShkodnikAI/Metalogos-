// ── №955 (issue #1922, the audit 12.11): serve returns a GENERIC 500 ───
//
// The audit probe (6_serve/e.mlog): `curl :18401/err` returned
// `Handler error: … to_int("TOPSECRET123") …` — the secret in the HTTP
// body. The fixed form: the client receives `Internal error (id: err-N)`;
// the full text (masked by the redact policy) is the server log's, keyed
// by the same id; `METALOGOS_SERVE_VERBOSE_ERRORS=1` returns the old body
// with the LOUD startup banner.
//
// These tests spawn the REAL binary and speak raw HTTP (no curl dependency).
#![allow(clippy::disallowed_methods)]

use serial_test::serial;
use std::io::Read;
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const PORT: u16 = 18901;

const PROBE: &str = r#"
mlogserver { port: 18901
  route "/err" method=GET { let n = to_int(env("SP")) return respond("200 ok") } }
"#;

fn spawn_serve(dir: &std::path::Path, verbose: bool) -> Child {
    let probe_path = dir.join("e.mlog");
    std::fs::write(&probe_path, PROBE).expect("probe");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mlog"));
    cmd.args(["serve", probe_path.to_str().expect("path")])
        .current_dir(dir)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    cmd.env("SP", "TOPSECRET123");
    cmd.env("METALOGOS_ENV_ALLOWLIST", "SP");
    if verbose {
        cmd.env("METALOGOS_SERVE_VERBOSE_ERRORS", "1");
    }
    cmd.spawn().expect("spawn mlog serve")
}

fn wait_port(port: u16) -> bool {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    false
}

fn get(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("timeout");
    use std::io::Write;
    write!(
        stream,
        "GET {} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        path
    )
    .expect("write");
    let mut buf = String::new();
    let _ = stream.read_to_string(&mut buf);
    buf
}

/// THE probe: the 500 body is GENERIC — the id form, no handler text,
/// no secret; the log line carries the MASKED full text (and the secret
/// appears NOWHERE — not in the body, not in the log).
#[test]
#[serial]
fn n955_serve_500_body_is_generic_and_the_log_is_masked() {
    let dir = std::env::temp_dir().join(format!("n955srv_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let mut child = spawn_serve(&dir, false);
    assert!(wait_port(PORT), "the serve did not start");

    let resp = get(PORT, "/err");
    assert!(
        resp.contains("Internal error (id: err-"),
        "the generic id-body expected, got: {}",
        resp
    );
    assert!(
        !resp.contains("Handler error:") && !resp.contains("TYPE_MISMATCH"),
        "the handler text must not reach the client, got: {}",
        resp
    );
    assert!(
        !resp.contains("TOPSECRET123"),
        "THE SECRET REACHED THE CLIENT"
    );

    // Stop the server, then read the LOG (stderr) — the masked full text
    // rides there under the same id.
    let _ = child.kill();
    let mut stderr = String::new();
    if let Some(mut handle) = child.stderr.take() {
        let _ = handle.read_to_string(&mut stderr);
    }
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        stderr.contains("[ROUTE_HANDLER_ERROR]") && stderr.contains("id=err-"),
        "the observability log line must carry the id, got: {}",
        stderr
    );
    assert!(
        stderr.contains("string(len=12)"),
        "the logged error text must carry the masked form, got: {}",
        stderr
    );
    assert!(
        !stderr.contains("TOPSECRET123"),
        "THE SECRET REACHED THE LOG: {}",
        stderr
    );
}

/// The opt-out is LOUD: the startup banner fires and the full body
/// returns (the development form — documented, never silent).
#[test]
#[serial]
fn n955_serve_verbose_mode_is_loud_and_full() {
    let dir = std::env::temp_dir().join(format!("n955srvv_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let mut child = spawn_serve(&dir, true);
    assert!(wait_port(PORT), "the serve did not start");
    let resp = get(PORT, "/err");
    let _ = child.kill();
    let mut stderr = String::new();
    if let Some(mut handle) = child.stderr.take() {
        let _ = handle.read_to_string(&mut stderr);
    }
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        resp.contains("Handler error:"),
        "the verbose mode returns the full body, got: {}",
        resp
    );
    assert!(
        stderr.contains("SERVE VERBOSE ERRORS"),
        "the loud startup banner must fire in the verbose mode, got: {}",
        stderr
    );
}
