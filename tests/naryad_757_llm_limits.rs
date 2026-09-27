// ── tests/naryad_757_llm_limits.rs ──────────────────────────────────────
// №757 (P1, llm/hardening): `call_llm` carried a hard-coded
// `"max_tokens": 1024` in every request body and never read
// `finish_reason` — a length-truncated answer came back as a normal
// success (the real office-report failure this naryad fixes: long
// structured reports cut mid-sentence with no signal).
//
// Proven here, over a REAL local socket (127.0.0.1:0, in-process, no
// external network — the naryad_248 hang-server discipline):
//   T1: llm { max_tokens: 8000 } reaches the provider BODY (the stub
//       captures the raw request) and a `finish_reason: "length"`
//       response is visible from mlog via `llm_last_finish_reason()`
//       (the program RETURNS the probe value — full end-to-end).
//   T2: no max_tokens field → the body carries the NEW default 4096
//       (not the former 1024) — the CHANGELOG behavior change, proven
//       on the wire.
//   T3: llm { temperature: 0.7 } reaches the body as 0.7.
//   T4: the probe + the new builtin exist on BOTH backends (VM parity:
//       the program compiles and runs on the VM, no "undefined
//       function").
//   T5: streaming — an SSE stub whose final chunk carries
//       `finish_reason: "length"` leaves "length" in the probe after
//       `stream_close` (the stream path of the same contract).
//
// Every test that touches the process-global probe/limits holds the
// static test mutex (the №251 global-store-race family lesson).

use std::io::{Read, Write};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

const TRUNCATED_JSON: &str = r#"{"choices":[{"message":{"role":"assistant","content":"partial answer"},"finish_reason":"length"}]}"#;
const STOPPED_JSON: &str = r#"{"choices":[{"message":{"role":"assistant","content":"complete answer"},"finish_reason":"stop"}]}"#;

/// Serialize tests that touch the process-global LLM state (probe,
/// BLOCK_* limits, global router) — №251 discipline.
fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// A one-shot-able local HTTP stub: records every request body it
/// receives, answers `response` with `content_type`. Plain
/// std::net::TcpListener on an ephemeral port — no external network,
/// no fixed-port races.
struct StubServer {
    addr: SocketAddr,
    captured: Arc<Mutex<Vec<String>>>,
    _keep: Arc<()>, // dropped when the test drops the handle → thread dies lazily
}

fn read_full_request(stream: &mut std::net::TcpStream) -> String {
    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        let n = stream.read(&mut tmp).unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        let s = String::from_utf8_lossy(&buf).to_string();
        if let Some(i) = s.find("\r\n\r\n") {
            let headers = s[..i].to_lowercase();
            let body_len = headers
                .split("\r\n")
                .find_map(|h| h.strip_prefix("content-length:"))
                .and_then(|v| v.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if buf.len() - (i + 4) >= body_len {
                break;
            }
        }
    }
    String::from_utf8_lossy(&buf).to_string()
}

fn respond(stream: &mut std::net::TcpStream, content_type: &str, body: &str) {
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        content_type,
        body.len(),
        body
    );
    let _ = stream.write_all(resp.as_bytes());
    let _ = stream.flush();
}

fn spawn_stub(content_type: &'static str, response: &'static str) -> StubServer {
    spawn_stub_dyn(content_type, response.to_string())
}

fn spawn_stub_dyn(content_type: &'static str, response: String) -> StubServer {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let addr = listener.local_addr().expect("local_addr");
    let captured = Arc::new(Mutex::new(Vec::<String>::new()));
    let cap = captured.clone();
    let keep = Arc::new(());
    let keep2 = keep.clone();
    std::thread::spawn(move || {
        let _keep = keep2; // hold the arc so the borrow lives as long as needed
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(_) => break,
            };
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .ok();
            let req = read_full_request(&mut stream);
            cap.lock().unwrap_or_else(|e| e.into_inner()).push(req);
            respond(&mut stream, content_type, &response);
            // Connection: close — the client sees EOF after the response.
        }
    });
    StubServer {
        addr,
        captured,
        _keep: keep,
    }
}

impl StubServer {
    fn url(&self) -> String {
        format!("http://{}", self.addr)
    }
    fn last_body(&self) -> String {
        self.captured
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .last()
            .cloned()
            .unwrap_or_default()
    }
}

/// TW run of a full mlog program (the bug_530 harness shape).
fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source.trim(), std::path::PathBuf::from("."))
}

/// VM run of the same source (parse → compile → Vm::run).
fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(std::path::PathBuf::from("."));
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

// ── T1: llm { max_tokens: 8000 } reaches the body; "length" is visible ──

#[test]
fn n757_block_max_tokens_reaches_body_and_truncation_is_visible() {
    let _g = test_lock();
    let stub = spawn_stub("application/json", TRUNCATED_JSON);

    let prog = format!(
        r#"
llm {{ providers: [{{alias: p, provider: openai, url: "{}"}}], max_tokens: 8000 }}
pattern P(t: String) -> String {{
  let _r = call_llm("Report:", t)
  return llm_last_finish_reason()
}}
flow Main {{ input: String = "x" -> P -> output }}
"#,
        stub.url()
    );

    let out = run_tw(&prog).expect("program must run");
    let out = out.unwrap_or_default();
    // The mlog-level probe sees the provider's finish_reason — the
    // truncation is NOT a silent success anymore.
    assert_eq!(out, "length", "probe must surface the truncation reason");

    // The configured ceiling reached the provider BODY (not 1024).
    let body = stub.last_body();
    assert!(
        body.contains("\"max_tokens\":8000"),
        "request body must carry max_tokens 8000, got: {}",
        body
    );
}

// ── T2: the new default 4096 (was 1024) — proven on the wire ────────────

#[test]
fn n757_default_ceiling_on_the_wire_is_4096() {
    let _g = test_lock();
    let stub = spawn_stub("application/json", STOPPED_JSON);

    let prog = format!(
        r#"
llm {{ providers: [{{alias: p, provider: openai, url: "{}"}}] }}
pattern P(t: String) -> String {{
  let _r = call_llm("Report:", t)
  return llm_last_finish_reason()
}}
flow Main {{ input: String = "x" -> P -> output }}
"#,
        stub.url()
    );

    let out = run_tw(&prog).expect("program must run");
    assert_eq!(out.unwrap_or_default(), "stop");

    let body = stub.last_body();
    assert!(
        body.contains("\"max_tokens\":4096"),
        "default body must carry the new default 4096, got: {}",
        body
    );
    assert!(
        !body.contains("\"max_tokens\":1024"),
        "the former hard-coded 1024 must be gone, got: {}",
        body
    );
}

// ── T3: llm { temperature } reaches the body ────────────────────────────

#[test]
fn n757_block_temperature_reaches_body() {
    let _g = test_lock();
    let stub = spawn_stub("application/json", STOPPED_JSON);

    let prog = format!(
        r#"
llm {{ providers: [{{alias: p, provider: openai, url: "{}"}}], max_tokens: 777, temperature: 0.7 }}
pattern P(t: String) -> String {{
  let _r = call_llm("Report:", t)
  return llm_last_finish_reason()
}}
flow Main {{ input: String = "x" -> P -> output }}
"#,
        stub.url()
    );

    let _ = run_tw(&prog).expect("program must run");

    let body = stub.last_body();
    assert!(
        body.contains("\"temperature\":0.7"),
        "body must carry temperature 0.7, got: {}",
        body
    );
    assert!(
        body.contains("\"max_tokens\":777"),
        "body must carry max_tokens 777, got: {}",
        body
    );
}

// ── T4: the probe builtin exists on both backends (VM parity) ───────────

#[test]
fn n757_probe_builtin_runs_on_vm() {
    let prog = r#"
pattern P(t: String) -> String {
  return llm_last_finish_reason()
}
flow Main { input: String = "x" -> P -> output }
"#;
    // The compile is the parity gate: an unregistered name fails the VM
    // compiler with "undefined function" (the №530 class).
    let out = run_vm(prog).expect("VM must compile and run the probe builtin");
    // No call happened in this process before the probe read → the
    // honest "" (or a reason left by an earlier serialized test) — the
    // point is a clean run, not a specific value.
    let _ = out.unwrap_or_default();
}

// ── T5: streaming — the SSE final chunk drives the probe ────────────────

#[test]
fn n757_stream_truncation_is_visible_after_close() {
    let _g = test_lock();
    // OpenAI-compatible SSE: two content chunks, the second carries
    // finish_reason "length", then [DONE].
    let sse = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"partial \"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"answer\"},\"finish_reason\":\"length\"}]}\n\n",
        "data: [DONE]\n\n"
    );
    let stub = spawn_stub_dyn("text/event-stream", sse.to_string());

    // The config is PARSED from mlog — the same llm {} surface a real
    // program writes (exercises the №757 grammar tail on the way).
    let source = format!(
        "llm {{ providers: [{{alias: p, provider: openai, url: \"{}\"}}], max_tokens: 8000 }}",
        stub.url()
    );
    let decls = metalogos::parser::parse(&source).expect("llm decl must parse");
    let config = decls
        .iter()
        .find_map(|d| match d {
            metalogos::ast::Declaration::LlmConfig(c) => Some(c.clone()),
            _ => None,
        })
        .expect("llm decl present");
    assert_eq!(config.max_tokens, Some(8000));

    metalogos::llm::set_global_smart_router(metalogos::llm::SmartRouter::from_config(&config));
    let handle = metalogos::llm::stream_via_smart_router("Report:", "long text", None, None)
        .expect("stream must open via global router");
    let _ = metalogos::llm::stream_next(handle).expect("chunk 1");
    let _ = metalogos::llm::stream_next(handle).expect("chunk 2");
    let end = metalogos::llm::stream_next(handle).expect("[DONE] tick");
    // The [DONE] event returns "" once; the next tick is the end marker.
    if end != metalogos::llm::LLM_STREAM_END_MARKER {
        let marker = metalogos::llm::stream_next(handle).expect("end marker");
        assert_eq!(marker, metalogos::llm::LLM_STREAM_END_MARKER);
    }
    let fin = metalogos::llm::stream_close(handle).expect("close");
    assert_eq!(fin.aggregated_text, "partial answer");
    // The stream's finish_reason reached the probe at close time.
    assert_eq!(
        metalogos::llm::llm_last_finish_reason(),
        "length",
        "the truncated stream must be visible after stream_close"
    );
}
