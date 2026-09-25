//! Наряд №454 (gh#673, P0 llm/security) — audit 25.09 finding 3.1 High:
//! Mock-LLM was the DEFAULT backend (`create_llm_backend` returned
//! `MockLlm` when `METALOGOS_MOCK_LLM` was unset) and `MockLlm` answered
//! with the ECHO of the effective prompt. In `mlog serve` that prompt
//! carries the instruction block, recalled memory (`context: recall/...`)
//! and conversation state — every LLM-backed route handed that material
//! back to the HTTP client as if it were a model answer.
//!
//! Red → green contract (audit repro: `mock_llm_not_default_in_serve`):
//!   * WITHOUT the variable: the route runs the REAL backend, which fails
//!     LOUDLY at the first call (no credentials) — the prompt marker never
//!     reaches the client, and the failure names the missing config
//!     instead of producing a confident wrong answer.
//!   * WITH `METALOGOS_MOCK_LLM=1`: the mock answers with the
//!     deterministic non-echo marker `[mock-llm:<8 hex>]` — useful for
//!     tests, reveals nothing about the prompt.
//!
//! The fixture plants the marker where the old echo definitely leaked it:
//! a `context: "literal"` block prepended to the pattern prompt (the same
//! prompt channel that carries recalled memory and history in real
//! deployments).
//!
//! Backend coverage (honest boundaries, documented):
//!   * Serve-path repro runs on the INTERPRETER backend — the office
//!     default (№399 precedent). On the VM serve backend a route body
//!     CANNOT invoke a learnable pattern at all today: `Vm::load_program`
//!     deliberately leaves `learnables` empty ("populated by
//!     RegisterLearnable during main_code execution", src/vm.rs) and the
//!     per-request route VM never runs main_code, so the route fails with
//!     "VM: learnable index N not found" BEFORE any LLM call. That is a
//!     PRE-EXISTING VM-serve × learnable gap, untouched by №454 (no LLM
//!     call is reachable there to leak anything); it is reported in the
//!     №454 completion report as tracker material for a follow-up wave.
//!   * VM parity for the №454 marker/fail-loud semantics is pinned on the
//!     flow path (parse → compile → Vm::run), where RegisterLearnable
//!     executes normally.
//!
//! Verify: cargo test --test mock_llm_not_default_in_serve

#![cfg(feature = "server")]

use metalogos::server::{run_test_server_with_backend, ServeBackend};

/// Serialize real-socket servers AND process-global env mutations across
/// the test binary (one port per run; the mock flag is read per call).
static N454_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// The prompt channel the old default mock echoed back verbatim: the
/// literal context stands in for the instruction/memory/history block.
/// The route passes the caller's text as the pattern input; the marker
/// lives ONLY in the prompt, so any echo of the prompt leaks it.
const N454_PROGRAM: &str = r#"
learnable pattern Tag(text: String) -> String {
  prompt: "classify the caller text"
  context: "session transcript: N454_SECRET_MARKER_ALPHA"
}
mlogserver {
  port: 18095
  route "/tag" method=GET { return respond("200", Tag(query_param("q"))) }
}
"#;

/// The same learnable fixture on the flow path (no server) — drives the
/// VM backend, where RegisterLearnable executes with main_code.
const N454_FLOW: &str = r#"
learnable pattern Tag(text: String) -> String {
  prompt: "classify the caller text"
  context: "session transcript: N454_SECRET_MARKER_ALPHA"
}
flow Main { input: String = "tell-me" -> Tag -> output }
"#;

async fn http_get(port: u16, path: &str) -> (u16, String) {
    let resp = reqwest::get(format!("http://127.0.0.1:{}{}", port, path))
        .await
        .expect("request must not fail at transport level");
    let status = resp.status().as_u16();
    let body = resp.text().await.expect("response body");
    (status, body)
}

fn assert_no_marker(body: &str, what: &str) {
    assert!(
        !body.contains("N454_SECRET_MARKER_ALPHA"),
        "{}: the prompt marker must NEVER reach the caller (old echo leak) — body: {}",
        what,
        body
    );
}

/// Compile-and-run the fixture on the bytecode VM (crosscheck harness
/// shape): parse → compile → fresh Vm → run.
fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let base_dir = std::env::current_dir().map_err(|e| format!("cwd error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir);
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

// ── (1) THE audit repro — serve, default env (no variable): fail-loud ──

#[tokio::test]
async fn n454_mock_not_default_in_serve() {
    let _guard = N454_LOCK.lock().await;
    std::env::remove_var("METALOGOS_MOCK_LLM");
    let (port, handle) = run_test_server_with_backend(N454_PROGRAM, ServeBackend::Interpreter)
        .await
        .expect("test server must start");
    let (status, body) = http_get(port, "/tag?q=tell-me").await;
    handle.abort();
    std::env::remove_var("METALOGOS_MOCK_LLM");
    assert_no_marker(&body, "serve default");
    assert_ne!(
        status, 200,
        "without credentials the LLM call must fail loudly, not answer — body: {}",
        body
    );
    assert!(
        body.contains("METALOGOS_API_KEY"),
        "the loud failure must name the missing configuration (real backend, not a silent mock) — body: {}",
        body
    );
}

// ── (2) Serve, explicit mock: deterministic non-echo marker ────────────

#[tokio::test]
async fn n454_explicit_mock_answers_non_echo_marker_in_serve() {
    let _guard = N454_LOCK.lock().await;
    std::env::set_var("METALOGOS_MOCK_LLM", "1");
    let (port, handle) = run_test_server_with_backend(N454_PROGRAM, ServeBackend::Interpreter)
        .await
        .expect("test server must start");
    let (status, body) = http_get(port, "/tag?q=tell-me").await;
    handle.abort();
    std::env::remove_var("METALOGOS_MOCK_LLM");
    assert_eq!(
        status, 200,
        "explicit mock mode must still serve the route — body: {}",
        body
    );
    assert_no_marker(&body, "serve explicit mock");
    assert!(
        body.contains("[mock-llm:"),
        "the mock answer must be the deterministic [mock-llm:<hex>] marker — body: {}",
        body
    );
}

// ── (3) Flow-path parity, default env: real backend fails loudly — VM ──

#[test]
fn n454_flow_default_fail_loud_vm() {
    let _guard = N454_LOCK.blocking_lock();
    std::env::remove_var("METALOGOS_MOCK_LLM");
    let result = run_vm(N454_FLOW);
    std::env::remove_var("METALOGOS_MOCK_LLM");
    let err = result
        .expect_err("VM flow without credentials must fail loudly (№454)");
    assert_no_marker(&err, "VM flow default");
    assert!(
        err.contains("METALOGOS_API_KEY"),
        "VM parity: the loud failure names the missing configuration — err: {}",
        err
    );
}

// ── (4) Flow-path parity, explicit mock: non-echo marker — TW and VM ───

#[test]
fn n454_flow_explicit_mock_non_echo_marker_tw() {
    let _guard = N454_LOCK.blocking_lock();
    std::env::set_var("METALOGOS_MOCK_LLM", "1");
    let result = metalogos::run_program(N454_FLOW);
    std::env::remove_var("METALOGOS_MOCK_LLM");
    let out = result
        .expect("TW flow with explicit mock must succeed")
        .unwrap_or_default();
    assert_no_marker(&out, "TW flow explicit mock");
    assert!(
        out.starts_with("[mock-llm:") && out.ends_with("]"),
        "TW flow: deterministic non-echo marker, got: {}",
        out
    );
}

#[test]
fn n454_flow_explicit_mock_non_echo_marker_vm() {
    let _guard = N454_LOCK.blocking_lock();
    std::env::set_var("METALOGOS_MOCK_LLM", "1");
    let result = run_vm(N454_FLOW);
    std::env::remove_var("METALOGOS_MOCK_LLM");
    let out = result
        .expect("VM flow with explicit mock must succeed")
        .unwrap_or_default();
    assert_no_marker(&out, "VM flow explicit mock");
    assert!(
        out.starts_with("[mock-llm:") && out.ends_with("]"),
        "VM flow: deterministic non-echo marker, got: {}",
        out
    );
}
