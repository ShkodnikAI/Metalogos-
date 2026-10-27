// ── tests/naryad_818_route_handler_stderr.rs ─────────────────────────
// Наряд №818 (issue #1586, wave 67, P1): the serve route-handler errors
// were swallowed — a 500 with a body, NOTHING in stderr, no access log,
// no tracing (the office Telegram webhook failed on every department
// request for 17 days — found only by local reproduction, #1573).
//
// The fix: on every Err(e) path that builds the 500 response, ONE
// unconditional stderr line:
//   [ROUTE_HANDLER_ERROR] <method> <path> -> 500: <error>
//
// The pin (the №772 RED-before form): spawn the REAL `mlog serve`
// process (the production dispatch path, not the in-process probe),
// hit a route whose handler errors at runtime, assert BOTH the 500 in
// the response AND the [ROUTE_HANDLER_ERROR] line on the server's
// stderr — on BOTH backend lanes (the VM default and the transitional
// METALOGOS_SERVE_BACKEND=interpreter, ADR-0197 §4).
//
// RED on unpatched main: the response is 500 but the stderr line is
// absent — the silence the naryad closes.

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::Duration;

static MLOG_BIN: OnceLock<String> = OnceLock::new();

fn mlog_bin() -> &'static str {
    MLOG_BIN.get_or_init(|| {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        std::path::Path::new(&manifest_dir)
            .join("target")
            .join("debug")
            .join("mlog")
            .to_string_lossy()
            .into_owned()
    })
}

const FIXTURE: &str = "tests/fixtures/naryad_818_route_error.mlog";
const PORT: u16 = 18471;

fn get_via_tcp() -> String {
    let mut stream =
        std::net::TcpStream::connect(("127.0.0.1", PORT)).expect("connect to the test server");
    stream
        .write_all(b"GET /boom HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .expect("write request");
    let mut resp = String::new();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("read timeout");
    stream.read_to_string(&mut resp).expect("read response");
    resp
}

fn spawn_serve(backend: Option<&str>) -> std::process::Child {
    let mut cmd = Command::new(mlog_bin());
    cmd.arg("serve")
        .arg(FIXTURE)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(b) = backend {
        cmd.env("METALOGOS_SERVE_BACKEND", b);
    }
    cmd.spawn().expect("spawn mlog serve")
}

fn wait_listening(child: &mut std::process::Child) {
    let mut stdout = String::new();
    if let Some(out) = child.stdout.as_mut() {
        // Rust's Stdout is a LineWriter (flushed per newline) even into a
        // pipe — the "listening on" line arrives as its own chunk; a
        // blocking read returns it promptly (EOF if the process dies).
        let mut buf = [0u8; 4096];
        loop {
            match out.read(&mut buf) {
                Ok(0) => break, // EOF — the process is gone
                Ok(n) => {
                    stdout.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if stdout.contains("listening on") {
                        // The bind happens right after the line; give the
                        // accept loop a beat.
                        std::thread::sleep(Duration::from_millis(500));
                        return;
                    }
                }
                Err(_) => break,
            }
        }
    }
    panic!(
        "mlog serve never reported 'listening on' (stdout: {:?}, exited: {:?})",
        stdout,
        child.try_wait()
    );
}

fn assert_lane(backend: Option<&str>, lane: &str) {
    let mut child = spawn_serve(backend);
    wait_listening(&mut child);

    let resp = get_via_tcp();
    assert!(
        resp.contains("500"),
        "[{lane}] the erroring route must answer 500, got: {}",
        resp.lines().next().unwrap_or("")
    );
    assert!(
        resp.contains("Handler error"),
        "[{lane}] the 500 body must carry the handler error text"
    );

    // Give the server a beat to flush the stderr line.
    std::thread::sleep(Duration::from_millis(300));
    child.kill().expect("kill serve");
    let mut stderr_buf = Vec::new();
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_end(&mut stderr_buf);
    }
    let _ = child.wait();
    let stderr = String::from_utf8_lossy(&stderr_buf).into_owned();
    assert!(
        stderr.contains("[ROUTE_HANDLER_ERROR]"),
        "[{lane}] the [ROUTE_HANDLER_ERROR] stderr line is missing — the №818 silence class:\n{}",
        stderr
    );
    assert!(
        stderr.contains("GET /boom -> 500:"),
        "[{lane}] the stderr line must carry the method, path and the 500 verdict:\n{}",
        stderr
    );
    assert!(
        stderr.contains("get() requires List"),
        "[{lane}] the stderr line must carry the coded error text:\n{}",
        stderr
    );
}

#[test]
fn naryad_818_route_handler_error_prints_stderr_line_both_lanes() {
    // The VM lane (the default executor, ADR-0197).
    assert_lane(None, "vm");
    // The transitional interpreter lane (the --reference twin).
    assert_lane(Some("interpreter"), "interpreter");
}
