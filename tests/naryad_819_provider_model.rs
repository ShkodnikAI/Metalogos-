// ── tests/naryad_819_provider_model.rs ────────────────────────────────
// Наряд №819 (issue #1587, wave 67, P1): the llm {} provider entry had
// NO `model` field (`src/grammar.pest` llm_provider_entry = alias,
// provider, key?, url?) — a program with `model: "deepseek-chat"` died
// at parse time (`expected RBRACE`, #1302). Worse: the call model was
// resolved ONCE (`model_override.or(default_model)`) and handed to
// EVERY provider in the failover loop — a mixed provider set made the
// 2nd/3rd rungs ask for a foreign model name and fail (the office OSP
// refusal class, FOSVED-office-v2 #505; the fosved-acc ТЗ §4.1/§17 п.15
// requirement: "у каждого провайдера своя модель").
//
// The fix: the optional per-provider `model:` field (grammar → AST →
// parser → SmartRouter's 5th tuple slot) and PER-PROVIDER resolution in
// BOTH failover loops (call + stream_open):
//   call model (call_llm 3rd arg) → provider's own model → default_model.
// Plus the honest-found gap: the VM user lanes ignored `llm {}` entirely
// (the compiler marks it "handled elsewhere", nothing handled it) — the
// router install rides the run/eval/REPL lanes now (the №521
// conversation_config class).
//
// Pins (the №772 RED-before form), over a REAL local socket (the
// naryad_248/naryad_757 discipline — no external network):
//   T1: failover across two providers, each with its own `model:` — the
//       FIRST (failing) rung's body carries model-a, the SECOND's body
//       carries model-b (RED on unpatched main: the program does not
//       even parse). Both lanes: TW and the VM (the real run path).
//   T2: the call model wins over the provider model (the priority head).
//   T3: the provider model wins over default_model.
//   T4: no `model:` field → default_model flows as before (the
//       back-compat pin — GREEN on unpatched main too).
//   T5: the streaming path (stream_open) resolves the provider's own
//       model per rung — not the loop-wide string.

use std::io::{Read, Write};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

const OK_JSON: &str = r#"{"choices":[{"message":{"role":"assistant","content":"ok answer"},"finish_reason":"stop"}]}"#;

/// Serialize tests that touch the process-global LLM state (router,
/// probe) — the №251 discipline.
fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// A local HTTP stub: records every request body, answers `status` with
/// `response`. Plain std::net::TcpListener on an ephemeral port.
struct StubServer {
    addr: SocketAddr,
    captured: Arc<Mutex<Vec<String>>>,
    _keep: Arc<()>,
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

fn spawn_stub(status_line: &'static str, response: &'static str) -> StubServer {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let captured: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let cap = captured.clone();
    let keep = Arc::new(());
    let keep2 = keep.clone();
    std::thread::spawn(move || {
        let _keep = keep2;
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
            let resp = format!(
                "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                status_line,
                response.len(),
                response
            );
            let _ = stream.write_all(resp.as_bytes());
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

/// TW run of a full mlog program (the naryad_757 harness shape).
fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source.trim(), std::path::PathBuf::from("."))
}

/// VM run of the same source — the REAL `mlog run` executor path
/// (run_program_on_vm: the №800/ADR-0197 user lane), not a hand-rolled
/// parse→compile→run that would skip the lane's own preparation.
fn run_vm(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_on_vm(source.trim())
}

// ── T1: failover — each rung asks for its OWN model (both lanes) ──────

#[test]
fn n819_failover_each_rung_uses_its_own_model_tw_lane() {
    let _g = test_lock();
    let dead = spawn_stub("500 Internal Server Error", r#"{"error":"boom"}"#);
    let alive = spawn_stub("200 OK", OK_JSON);

    let prog = format!(
        r#"
llm {{
  providers: [
    {{alias: first, provider: openai, url: "{dead}", model: "model-a"}},
    {{alias: second, provider: openai, url: "{alive}", model: "model-b"}}
  ],
  failover: auto,
  circuit_breaker: 5,
  default_model: "loop-wide-m"
}}
pattern P(t: String) -> String {{
  let _r = call_llm("Report:", t)
  return "done"
}}
flow Main {{ input: String = "x" -> P -> output }}
"#,
        dead = dead.url(),
        alive = alive.url()
    );

    let out =
        run_tw(&prog).expect("the program must parse and run (the #1302 parse refusal is the RED)");
    assert_eq!(out.as_deref(), Some("done"));

    // The failing FIRST rung asked for ITS OWN model — not the loop-wide
    // default, not the second provider's model.
    let first_body = dead.last_body();
    assert!(
        first_body.contains(r#""model":"model-a""#),
        "the first rung must carry its own model-a, body: {}",
        first_body
    );
    // The SECOND rung (the one that answered) asked for model-b.
    let second_body = alive.last_body();
    assert!(
        second_body.contains(r#""model":"model-b""#),
        "the second rung must carry its own model-b, body: {}",
        second_body
    );
    assert!(
        !second_body.contains("loop-wide-m"),
        "no rung may receive the loop-wide default when the provider names its own model"
    );
}

#[test]
fn n819_failover_each_rung_uses_its_own_model_vm_lane() {
    let _g = test_lock();
    let dead = spawn_stub("500 Internal Server Error", r#"{"error":"boom"}"#);
    let alive = spawn_stub("200 OK", OK_JSON);

    let prog = format!(
        r#"
llm {{
  providers: [
    {{alias: first, provider: openai, url: "{dead}", model: "model-a"}},
    {{alias: second, provider: openai, url: "{alive}", model: "model-b"}}
  ],
  failover: auto,
  circuit_breaker: 5,
  default_model: "loop-wide-m"
}}
pattern P(t: String) -> String {{
  let _r = call_llm("Report:", t)
  return "done"
}}
flow Main {{ input: String = "x" -> P -> output }}
"#,
        dead = dead.url(),
        alive = alive.url()
    );

    // The VM lane: the same program through the real `mlog run` path
    // (the №762 lane parity — the parse fix, the per-provider resolution
    // AND the router install are shared, the pin guards the VM path
    // end-to-end).
    let out = run_vm(&prog).expect("the VM must compile and run the llm program");
    assert_eq!(out.as_deref(), Some("done"));

    let first_body = dead.last_body();
    assert!(
        first_body.contains(r#""model":"model-a""#),
        "VM: the first rung must carry its own model-a, body: {}",
        first_body
    );
    let second_body = alive.last_body();
    assert!(
        second_body.contains(r#""model":"model-b""#),
        "VM: the second rung must carry its own model-b, body: {}",
        second_body
    );
}

// ── T2: the call model wins over the provider model ───────────────────

#[test]
fn n819_call_model_beats_provider_model() {
    let _g = test_lock();
    let stub = spawn_stub("200 OK", OK_JSON);

    let prog = format!(
        r#"
llm {{ providers: [{{alias: p, provider: openai, url: "{}", model: "provider-m"}}] }}
pattern P(t: String) -> String {{
  let _r = call_llm("Report:", t, "call-m")
  return "done"
}}
flow Main {{ input: String = "x" -> P -> output }}
"#,
        stub.url()
    );

    let out = run_tw(&prog).expect("the program must run");
    assert_eq!(out.as_deref(), Some("done"));
    let body = stub.last_body();
    assert!(
        body.contains(r#""model":"call-m""#),
        "the call model must win over the provider field, body: {}",
        body
    );
}

// ── T3: the provider model wins over default_model ────────────────────

#[test]
fn n819_provider_model_beats_default_model() {
    let _g = test_lock();
    let stub = spawn_stub("200 OK", OK_JSON);

    let prog = format!(
        r#"
llm {{ providers: [{{alias: p, provider: openai, url: "{}", model: "provider-m"}}], default_model: "default-m" }}
pattern P(t: String) -> String {{
  let _r = call_llm("Report:", t)
  return "done"
}}
flow Main {{ input: String = "x" -> P -> output }}
"#,
        stub.url()
    );

    let out = run_tw(&prog).expect("the program must run");
    assert_eq!(out.as_deref(), Some("done"));
    let body = stub.last_body();
    assert!(
        body.contains(r#""model":"provider-m""#),
        "the provider model must win over default_model, body: {}",
        body
    );
    assert!(
        !body.contains("default-m"),
        "default_model must not leak past the provider's own field"
    );
}

// ── T4: no model field → default_model (the back-compat pin) ──────────

#[test]
fn n819_no_provider_field_default_model_flows_as_before() {
    let _g = test_lock();
    let stub = spawn_stub("200 OK", OK_JSON);

    let prog = format!(
        r#"
llm {{ providers: [{{alias: p, provider: openai, url: "{}"}}], default_model: "default-m" }}
pattern P(t: String) -> String {{
  let _r = call_llm("Report:", t)
  return "done"
}}
flow Main {{ input: String = "x" -> P -> output }}
"#,
        stub.url()
    );

    let out = run_tw(&prog).expect("the program must run");
    assert_eq!(out.as_deref(), Some("done"));
    let body = stub.last_body();
    assert!(
        body.contains(r#""model":"default-m""#),
        "the back-compat shape must keep the default_model flow, body: {}",
        body
    );
}

// ── T5: the streaming path — per-provider model ───────────────────────

#[test]
fn n819_stream_open_uses_provider_model() {
    let _g = test_lock();
    // OpenAI-compatible SSE stub — the naryad_757 T5 shape.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let captured: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let cap = captured.clone();
    let sse = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"partial \"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"answer\"},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n"
    );
    let keep = Arc::new(());
    let keep2 = keep.clone();
    std::thread::spawn(move || {
        let _keep = keep2;
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(_) => break,
            };
            let req = read_full_request(&mut stream);
            cap.lock().unwrap_or_else(|e| e.into_inner()).push(req);
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                sse.len(),
                sse
            );
            let _ = stream.write_all(resp.as_bytes());
        }
    });

    // The config is PARSED from mlog — the same llm {} surface a real
    // program writes (exercises the №819 grammar tail on the way).
    let source = format!(
        "llm {{ providers: [{{alias: p, provider: openai, url: \"{}\", model: \"stream-m\"}}], default_model: \"default-m\" }}",
        format!("http://{}", addr)
    );
    let decls = metalogos::parser::parse(&source).expect("llm decl must parse");
    let config = decls
        .iter()
        .find_map(|d| match d {
            metalogos::ast::Declaration::LlmConfig(c) => Some(c.clone()),
            _ => None,
        })
        .expect("llm decl present");
    assert_eq!(
        config.providers[0].model.as_deref(),
        Some("stream-m"),
        "the AST must carry the provider's own model"
    );

    let router = metalogos::llm::SmartRouter::from_config(&config);
    let state = router
        .stream_open(
            "Report:",
            "x",
            None,
            Some(std::time::Duration::from_secs(5)),
        )
        .expect("stream must open");
    assert_eq!(
        state.resolved_model(),
        "stream-m",
        "the streaming state must name the provider's own model, not default_model"
    );
    let body = captured
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .last()
        .cloned()
        .unwrap_or_default();
    assert!(
        body.contains(r#""model":"stream-m""#),
        "the streaming body must carry the provider's own model, body: {}",
        body
    );
}
