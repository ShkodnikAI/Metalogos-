//! Наряд №454 (gh#673, P0 llm/security) — audit 25.09 finding 3.1 High:
//! Mock-LLM was the DEFAULT backend (`create_llm_backend` returned
//! `MockLlm` when `METALOGOS_MOCK_LLM` was unset) and `MockLlm` answered
//! with the ECHO of the effective prompt. In `mlog serve` that prompt
//! carries the instruction block, recalled memory (`context: recall/...`)
//! and conversation state — every LLM-backed route handed that material
//! back to the HTTP client as if it were a model answer.
//!
//! Red → green contract (audit repro: `mock_llm_not_default_in_serve`),
//! re-pinned by №879 (wave 78 — W-G, ADR-0208 §2.2/§5 — the ONE
//! intentional posture change):
//!   * WITHOUT any declaration (no variable, no provider/key/base-url
//!     env): the route answers on the LOCAL provider — the
//!     deterministic, non-echo, offline `[mock-llm:<8 hex>]` marker. The
//!     №454 SECURITY property is preserved and STRONGER: the prompt
//!     marker NEVER reaches the client in ANY mode, and the local
//!     answer is VISIBLE as such (never a silent stand-in for an
//!     external voice).
//!   * WITH a DECLARED external provider (`METALOGOS_LLM_PROVIDER` set)
//!     and no credentials: the route fails LOUDLY at the first call —
//!     the prompt marker never reaches the client, and the failure
//!     names the missing configuration (the №454 fail-loud posture,
//!     preserved on the declared lane).
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

use metalogos_server::server::{run_test_server_with_backend, ServeBackend};

/// Serialize real-socket servers AND process-global env mutations across
/// the test binary (one port per run; the mock flag is read per call).
static N454_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// №879/№401 hygiene: clear EVERY declaration surface after each case —
/// a leftover provider/key env would silently flip the next test's
/// composition to the external lane.
fn clear_serve_env() {
    std::env::remove_var("METALOGOS_MOCK_LLM");
    std::env::remove_var("METALOGOS_LLM_PROVIDER");
    std::env::remove_var("METALOGOS_API_KEY");
    std::env::remove_var("METALOGOS_OPENAI_BASE_URL");
}

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

// ── (1) THE audit repro — serve, nothing declared: the local-first ──
// answer (№879/ADR-0208 §2.2: the visible marker, never the old echo)

#[tokio::test]
async fn n454_nothing_declared_refuses_in_serve() {
    let _guard = N454_LOCK.lock().await;
    std::env::remove_var("METALOGOS_MOCK_LLM");
    std::env::remove_var("METALOGOS_LLM_PROVIDER");
    std::env::remove_var("METALOGOS_API_KEY");
    std::env::remove_var("METALOGOS_OPENAI_BASE_URL");
    let (port, handle) = run_test_server_with_backend(N454_PROGRAM, ServeBackend::Interpreter)
        .await
        .expect("test server must start");
    let (status, body) = http_get(port, "/tag?q=tell-me").await;
    handle.abort();
    clear_serve_env();
    assert_no_marker(&body, "serve nothing-declared");
    // №898: the nothing-declared serve lane answers the route with the
    // LOUD LLM refusal (500-class, the unconfigured posture) — the body
    // names the configuration surfaces; a mock marker never materializes
    // from an unconfigured system (the №879 default is rolled back).
    assert_ne!(
        status, 200,
        "the unconfigured serve lane must NOT answer 200 with a marker — body: {}",
        body
    );
    assert!(
        body.contains("METALOGOS_LLM_PROVIDER") && body.contains("METALOGOS_MOCK_LLM"),
        "the unconfigured serve body must name the configuration surfaces — body: {}",
        body
    );
}

// ── (1b) Serve, DECLARED external provider without a key: fail-loud ──

#[tokio::test]
async fn n454_declared_external_fails_loud_in_serve() {
    let _guard = N454_LOCK.lock().await;
    std::env::remove_var("METALOGOS_MOCK_LLM");
    std::env::remove_var("METALOGOS_API_KEY");
    std::env::remove_var("METALOGOS_OPENAI_BASE_URL");
    std::env::set_var("METALOGOS_LLM_PROVIDER", "anthropic");
    let (port, handle) = run_test_server_with_backend(N454_PROGRAM, ServeBackend::Interpreter)
        .await
        .expect("test server must start");
    let (status, body) = http_get(port, "/tag?q=tell-me").await;
    handle.abort();
    clear_serve_env();
    assert_no_marker(&body, "serve declared-external");
    assert_ne!(
        status, 200,
        "a DECLARED external provider without credentials must fail loudly, not answer — body: {}",
        body
    );
    assert!(
        body.contains("METALOGOS_API_KEY"),
        "the loud failure must name the missing configuration (the №454 posture on the declared lane) — body: {}",
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

// ── (3) Flow-path parity, nothing declared: the local provider answers ──

#[test]
fn n454_flow_nothing_declared_refuses_vm() {
    let _guard = N454_LOCK.blocking_lock();
    std::env::remove_var("METALOGOS_MOCK_LLM");
    std::env::remove_var("METALOGOS_LLM_PROVIDER");
    std::env::remove_var("METALOGOS_API_KEY");
    std::env::remove_var("METALOGOS_OPENAI_BASE_URL");
    let result = run_vm(N454_FLOW);
    clear_serve_env();
    let err = result.expect_err(
        "VM flow with nothing declared must REFUSE (№898) — a marker \
                     here would be the rolled-back №879 posture",
    );
    assert_no_marker(&err, "VM flow nothing-declared");
    assert!(
        err.contains("METALOGOS_LLM_PROVIDER") && err.contains("METALOGOS_MOCK_LLM"),
        "the VM parity refusal names the configuration surfaces — err: {}",
        err
    );
}

// ── (3b) Flow-path parity, DECLARED external: the loud real path ──

#[test]
fn n454_flow_declared_external_fails_loud_vm() {
    let _guard = N454_LOCK.blocking_lock();
    std::env::remove_var("METALOGOS_MOCK_LLM");
    std::env::remove_var("METALOGOS_API_KEY");
    std::env::remove_var("METALOGOS_OPENAI_BASE_URL");
    std::env::set_var("METALOGOS_LLM_PROVIDER", "anthropic");
    let result = run_vm(N454_FLOW);
    clear_serve_env();
    let err = result.expect_err("VM flow on the declared-external lane must fail loudly (№454)");
    assert_no_marker(&err, "VM flow declared-external");
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
