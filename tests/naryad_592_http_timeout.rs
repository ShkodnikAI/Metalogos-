//! Наряд №592 (Волна 30, Камертон Н1-06): the per-call HTTP timeout and
//! the outcome taxonomy for http_get/http_post.
//!
//! Contracts under test:
//! 1. THE DEADLINE (the DoD row «таймаут срабатывает в заданный срок
//!    ±10%»): a local slow endpoint with an explicit per-call timeout of
//!    2.0 s — the call fails with the typed [HTTP_TIMEOUT] stamp and the
//!    elapsed time lands in [1.9, 2.2] s (the ±10% band of the deadline,
//!    plus the client-build measurement overhead documented in the
//!    assert).
//! 2. THE MID-RESPONSE BREAK (the DoD row «разрыв соединения на середине
//!    ответа даёт типизированную ошибку»): a server that declares
//!    Content-Length: 100 and closes after 10 bytes — the previous code
//!    LAUNDERED this into a silent empty body (unwrap_or_default); now it
//!    is a typed [HTTP_CONNECT] error.
//! 3. THREE DISTINCT OUTCOMES: [HTTP_TIMEOUT] / [HTTP_CONNECT] /
//!    [HTTP_STATUS] — branchable via try{} through the language on BOTH
//!    backends (the №385/ADR-0169 position-0 stamp). The retry stays the
//!    caller's (№71's retry_config — untouched; the language hides no
//!    retry inside).
//! 4. THE LOUD RANGE GATE: the per-call deadline outside 1..=300 s (or
//!    NaN) refuses with [HTTP_TIMEOUT_RANGE] BEFORE any network activity —
//!    the loud replacement of №261's silent clamp.
//! 5. The effects classification is UNCHANGED (№316/№449): no
//!    classification row is touched in this PR — the network axis rides
//!    on the existing http_get/http_post entries.
//!
//! Uses the Python test server at tests/p592_http_timeout_server.py (the
//! №71 house pattern: a child process, the documented
//! METALOGOS_HTTP_ALLOW_PRIVATE=1 kill-switch — these tests target
//! 127.0.0.1 BY DESIGN, all #[serial] to sequence the process-global env
//! and the fixed port).
//!
//! Boundary: ZERO edits to the LLM path and SmartRouter (the dispatch's
//! «не делать» row) — the LLM deadline machinery (№156/№248) is untouched.

use metalogos::builtins::Builtins;
use metalogos::interpreter::Value;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const SERVER_PORT: u16 = 18792;
const BASE_URL: &str = "http://127.0.0.1:18792";

fn allow_loopback_egress() {
    std::env::set_var("METALOGOS_HTTP_ALLOW_PRIVATE", "1");
}

/// RAII guard: kills the child server process on drop (the №71 pattern).
struct ServerGuard(Child);

impl ServerGuard {
    fn spawn() -> Self {
        let child = Command::new("python3")
            .arg("tests/p592_http_timeout_server.py")
            .arg("--port")
            .arg(SERVER_PORT.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to start the №592 test server");
        thread::sleep(Duration::from_millis(500));
        Self(child)
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn builtins() -> Builtins {
    Builtins::new()
}

// ── Backend harness (the language-level classification) ─────────────

fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.to_path_buf())
}

fn run_vm(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp
        .compile(declarations)
        .map_err(|e| format!("compile error: {}", e))?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program).map_err(|e| e.to_string())
}

fn program(body: &str) -> String {
    format!(
        "pattern T(_input: String) -> String {{\n{}\n}}\nflow Main {{ input: String = \"s\" -> T -> output }}",
        body
    )
}

fn try_code_program(call: &str) -> String {
    program(&format!("  let r = try {}\n  return r.error.code", call))
}

/// Run the try-code program on BOTH backends and assert the typed code.
fn assert_try_code(src: &str, want: &str, name: &str) {
    let base_dir = PathBuf::from("examples");
    for (backend, res) in [
        ("TW", run_tw(src, &base_dir)),
        ("VM", run_vm(src, &base_dir)),
    ] {
        let out = res
            .unwrap_or_else(|e| panic!("{} {}: try must capture the refusal: {}", name, backend, e))
            .unwrap_or_else(|| panic!("{} {}: the pattern must return", name, backend));
        assert_eq!(
            out.trim(),
            want,
            "{} {}: typed code mismatch",
            name,
            backend
        );
    }
}

// ── 1: the deadline fires within the ±10% band ──────────────────────

#[test]
#[serial_test::serial]
fn n592_timeout_fires_within_the_deadline_band() {
    allow_loopback_egress();
    let _server = ServerGuard::spawn();
    let b = builtins();
    let http_get = b.get("http_get").expect("http_get exists");

    let deadline = 2.0f64;
    let started = Instant::now();
    let result = http_get(&[
        Value::String(format!("{}/slow?sec=6", BASE_URL)), // the server would answer at 6 s
        Value::Float(deadline),
    ]);
    let elapsed = started.elapsed().as_secs_f64();

    let err = result.expect_err("a 6 s server behind a 2 s deadline must fail");
    assert!(
        err.starts_with("[HTTP_TIMEOUT] "),
        "the refusal must carry the typed stamp, got: {}",
        err
    );
    // The DoD band: ±10% of the deadline = [1.8, 2.2]; the lower bound is
    // padded to 1.9 (a preemptive failure would be a shorter, WRONG
    // deadline) and the upper keeps the client-build overhead (~ms).
    assert!(
        (1.9..=2.2).contains(&elapsed),
        "the deadline must fire within ±10% of {} s, got {:.3} s",
        deadline,
        elapsed
    );
}

// ── 2: the mid-response break is a typed failure, not a silent body ──

#[test]
#[serial_test::serial]
fn n592_mid_response_break_is_typed_not_silent() {
    allow_loopback_egress();
    let _server = ServerGuard::spawn();
    let b = builtins();
    let http_get = b.get("http_get").expect("http_get exists");

    let result = http_get(&[Value::String(format!("{}/break", BASE_URL))]);
    let err = result.expect_err("a mid-response break must NOT launder into an empty body");
    assert!(
        err.starts_with("[HTTP_CONNECT] "),
        "the break must carry the typed stamp, got: {}",
        err
    );
}

// ── 3: the three outcomes are distinct stamps ───────────────────────

#[test]
#[serial_test::serial]
fn n592_status_outcome_is_typed_and_distinct() {
    allow_loopback_egress();
    let _server = ServerGuard::spawn();
    let b = builtins();
    let http_get = b.get("http_get").expect("http_get exists");

    let result = http_get(&[Value::String(format!("{}/status?code=404", BASE_URL))]);
    let err = result.expect_err("a 404 must fail with the typed status outcome");
    assert!(err.starts_with("[HTTP_STATUS] "), "got: {}", err);
    assert!(
        err.contains("404"),
        "the status number stays in the message: {}",
        err
    );

    // the success path is untouched
    let ok = http_get(&[Value::String(format!("{}/ok", BASE_URL))]).expect("the ok path");
    match ok {
        Value::String(body) => assert_eq!(body, "fine"),
        other => panic!("expected String, got {:?}", other.type_name()),
    }

    // http_post mirrors the taxonomy
    let bp = builtins();
    let http_post = bp.get("http_post").expect("http_post exists");
    let post_err = http_post(&[
        Value::String(format!("{}/status?code=500", BASE_URL)),
        Value::String("x".to_string()),
    ])
    .expect_err("a 500 POST must fail with the typed status outcome");
    assert!(post_err.starts_with("[HTTP_STATUS] "), "got: {}", post_err);
}

// ── 4: the loud range gate (before any network activity) ────────────

#[test]
#[serial_test::serial]
fn n592_out_of_range_deadline_is_loud() {
    allow_loopback_egress();
    let b = builtins();
    let http_get = b.get("http_get").expect("http_get exists");
    for bad in [500.0, 0.5, f64::NAN, f64::INFINITY] {
        let err = http_get(&[
            Value::String("http://example.invalid/x".to_string()),
            Value::Float(bad),
        ])
        .expect_err("an out-of-range deadline must refuse loudly");
        assert!(
            err.starts_with("[HTTP_TIMEOUT_RANGE] "),
            "bad deadline {}: got: {}",
            bad,
            err
        );
    }
    let bp = builtins();
    let http_post = bp.get("http_post").expect("http_post exists");
    let err = http_post(&[
        Value::String("http://example.invalid/x".to_string()),
        Value::String("b".to_string()),
        Value::Float(0.0),
    ])
    .expect_err("a zero deadline must refuse loudly");
    assert!(err.starts_with("[HTTP_TIMEOUT_RANGE] "), "got: {}", err);
}

// ── 5: try{} classification through the language, BOTH backends ─────

#[test]
#[serial_test::serial]
fn n592_try_classification_http_status_both_backends() {
    allow_loopback_egress();
    let _server = ServerGuard::spawn();
    let src = try_code_program(&format!("http_get(\"{}/status?code=404\")", BASE_URL));
    assert_try_code(&src, "HTTP_STATUS", "n592_status");
}

#[test]
#[serial_test::serial]
fn n592_try_classification_http_connect_both_backends() {
    allow_loopback_egress();
    let _server = ServerGuard::spawn();
    let src = try_code_program(&format!("http_get(\"{}/break\")", BASE_URL));
    assert_try_code(&src, "HTTP_CONNECT", "n592_break");
}

#[test]
#[serial_test::serial]
fn n592_try_classification_http_timeout_both_backends() {
    allow_loopback_egress();
    let _server = ServerGuard::spawn();
    // the deadline 1.0 s against a 6 s server — the typed timeout, twice
    // (TW + VM), each within its deadline band
    let src = try_code_program(&format!("http_get(\"{}/slow?sec=6\", 1.0)", BASE_URL));
    let base_dir = PathBuf::from("examples");
    for (backend, res) in [
        ("TW", run_tw(&src, &base_dir)),
        ("VM", run_vm(&src, &base_dir)),
    ] {
        let out = res
            .unwrap_or_else(|e| panic!("{}: try must capture the refusal: {}", backend, e))
            .unwrap_or_else(|| panic!("{}: the pattern must return", backend));
        assert_eq!(
            out.trim(),
            "HTTP_TIMEOUT",
            "{}: typed code mismatch",
            backend
        );
    }
}
