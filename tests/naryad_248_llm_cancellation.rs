// ── tests/naryad_248_llm_cancellation.rs ────────────────────────────────
// Наряд №248: LLM request cancellation — deadline-based cancellation on
// every call path. Closes the README:113 / REFERENCE §5.15 promise
// ("Full request cancellation … a separate naryad") recorded by the
// external audit 2026-09-10 ("LLM timeout exists, abort does not").
//
// Proven here:
//   C1: RealLlm against a local accept-and-hang server —
//       call_with_deadline(~2s) fails LOUDLY at the deadline (NOT the
//       120s client default), and the SERVER side observes the TCP close:
//       observable evidence that the request itself was cancelled, not
//       the wait (the former №126 abandoned-thread behavior).
//   C2: MockLlm — deadline tighter than the artificial delay → fast loud
//       Err; delay well below the deadline → Ok.
//   C3: default trait impl delegates to call_with_model (trait
//       compatibility proven by test).
//
// The hang server is a plain std::net::TcpListener bound to
// 127.0.0.1:0 (random port) — in-process, no external network, no
// python, no fixed-port races, NO #[ignore] (invariant delta = 0).
// MockLlm tests share the process-global MOCK_LLM_DELAY_MS — they hold
// one static mutex for the whole body (naryad #251 discipline, the
// global-store-race family lesson).

use std::io::Read;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use metalogos::llm::{LlmBackend, MockLlm, Provider, RealLlm};

/// Poison-tolerant static mutex serializing tests that touch the
/// process-global MockLlm state (MOCK_LLM_DELAY_MS) — naryad #251 lesson.
fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Accept-and-hang server: accepts connections, consumes request bytes,
/// NEVER responds. Counts how many connections were observed closed
/// (read returns EOF or error) — the observable proof that the CLIENT
/// dropped the TCP connection at the deadline (request cancelled,
/// not left in flight).
struct HangServer {
    addr: SocketAddr,
    closed_count: Arc<AtomicUsize>,
}

impl HangServer {
    fn spawn() -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
        let addr = listener.local_addr().expect("local_addr");
        let closed_count = Arc::new(AtomicUsize::new(0));
        let closed = closed_count.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = match stream {
                    Ok(s) => s,
                    Err(_) => break,
                };
                let closed = closed.clone();
                std::thread::spawn(move || {
                    // Consume request bytes, never answer. When the client
                    // gives up (deadline), the socket closes: read yields
                    // Ok(0) (EOF) or an error — count one observed close.
                    let mut buf = [0u8; 8192];
                    loop {
                        match stream.read(&mut buf) {
                            Ok(0) | Err(_) => break,
                            Ok(_) => continue,
                        }
                    }
                    closed.fetch_add(1, Ordering::SeqCst);
                });
            }
        });
        HangServer { addr, closed_count }
    }

    fn url(&self) -> String {
        format!("http://{}", self.addr)
    }
}

// ── C1: RealLlm — the request itself is cancelled at the deadline ──────

#[test]
fn contract_llm_real_deadline_cancels_request_via_hang_server() {
    let server = HangServer::spawn();
    let mut backend = RealLlm::with_config(
        Provider::OpenAI,
        "test-model".to_string(),
        Some("test-key".to_string()),
    );
    // Route the OpenAI path to the local hang server (resolve_endpoint
    // honors base_url; the server accepts anything).
    backend.base_url = Some(server.url());

    let start = Instant::now();
    let result = backend.call_with_deadline("p", "in", None, Duration::from_secs(2));
    let elapsed = start.elapsed();

    // Loud failure at the deadline — NOT the 120s client default.
    let err = result.expect_err("deadline must fail loudly against a hanging server");
    assert!(
        err.contains("timed out"),
        "error must mention the deadline, got: {}",
        err
    );
    assert!(
        elapsed < Duration::from_secs(30),
        "request must be cancelled at the ~2s deadline (got {:?}) — \
         it must not outlive the 120s client default",
        elapsed
    );

    // SERVER-side evidence: the client's TCP connection was observed
    // closing after the deadline — the request was cancelled, not merely
    // the wait (the pre-248 abandoned-thread behavior would leave the
    // server still hanging on an open socket).
    let wait_until = Instant::now() + Duration::from_secs(5);
    while Instant::now() < wait_until && server.closed_count.load(Ordering::SeqCst) == 0 {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        server.closed_count.load(Ordering::SeqCst) >= 1,
        "hang server must observe the client-side connection close (TCP drop)"
    );
}

// ── C2: MockLlm — min(delay, deadline) semantics ────────────────────────

#[test]
fn contract_llm_mock_deadline_tighter_than_delay_fails_fast() {
    let _g = test_lock();
    MockLlm::reset_delay();
    MockLlm::set_delay_ms(5000);

    let start = Instant::now();
    let result = MockLlm.call_with_deadline("p", "in", None, Duration::from_secs(1));
    let elapsed = start.elapsed();

    MockLlm::reset_delay();

    let err = result.expect_err("deadline tighter than the delay must fail loudly");
    assert!(
        err.contains("timed out"),
        "loud timeout wording expected, got: {}",
        err
    );
    assert!(
        elapsed < Duration::from_secs(4),
        "must fail fast at the deadline (got {:?}), not sleep the full 5s delay",
        elapsed
    );
}

#[test]
fn contract_llm_mock_delay_below_deadline_succeeds() {
    let _g = test_lock();
    MockLlm::reset_delay();
    MockLlm::set_delay_ms(100);

    let start = Instant::now();
    let result = MockLlm.call_with_deadline("p", "in", None, Duration::from_secs(5));
    let elapsed = start.elapsed();

    MockLlm::reset_delay();

    assert_eq!(result.expect("delay below deadline must succeed"), "p");
    assert!(
        elapsed < Duration::from_secs(2),
        "100ms delay must complete well under the 5s deadline (got {:?})",
        elapsed
    );
}

// ── C3: default trait impl delegates to call_with_model ────────────────

/// A backend that does NOT override call_with_deadline — the default
/// must delegate to call_with_model (existing impls stay compatible).
struct DelegatingBackend;

impl LlmBackend for DelegatingBackend {
    fn call(&self, prompt: &str, _input: &str) -> Result<String, String> {
        Ok(format!("CALL:{}", prompt))
    }

    fn call_with_model(
        &self,
        prompt: &str,
        _input: &str,
        model: Option<&str>,
    ) -> Result<String, String> {
        Ok(format!("WITH_MODEL:{:?}:{}", model, prompt))
    }
}

#[test]
fn contract_llm_default_deadline_impl_delegates_to_call_with_model() {
    let backend = DelegatingBackend;
    let result = backend
        .call_with_deadline("p", "in", Some("m"), Duration::from_secs(5))
        .expect("default impl must delegate, not fail");
    assert!(
        result.contains("WITH_MODEL:"),
        "default call_with_deadline must delegate to call_with_model, got: {}",
        result
    );
}
