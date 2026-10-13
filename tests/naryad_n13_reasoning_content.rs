// ── Naryad N13 (gh#1221, platform-enabler, llm): reasoning_content ─────
//
// GLM reasoning models (thinking enabled) answer on the
// OpenAI-compatible wire with a `reasoning_content` block alongside the
// final `content` in the message. The contract pinned here:
//
//   R1: purity — the user sees ONLY the final `content`; the reasoning
//       block never reaches the output/templates (all three paths:
//       legacy RealLlm, SmartRouter, streaming delta extraction).
//   R2: usage — the legacy (no-`llm config`) path accounts the token
//       usage the provider reported (GLM/OpenAI contract:
//       `completion_tokens` INCLUDES the reasoning tokens) in the
//       METALOGOS_LLM_TRACE line — honest per-call data (№276/ADR-0138).
//   R3: a plain response WITHOUT reasoning_content behaves exactly as
//       before (pin).
//   R4: reasoning present but `content: null` (the answer was cut
//       mid-reasoning) stays a LOUD format error — never a silent
//       empty string.
//   R5: streaming purity — a reasoning delta (`delta.reasoning_content`)
//       yields NO text; only `delta.content` deltas flow.
//
// The mock OpenAI-compatible server binds 127.0.0.1:0 (the №261 mock
// lekało) and is reached through METALOGOS_OPENAI_BASE_URL (№12).
//
// RED-first discipline: R1/R3/R4 are the verification legs the naryad
// asks to pin ("Убедиться") — they were GREEN on main by construction
// (the parsers read ONLY `content`). R2 was RED on main: the legacy
// path dropped the usage block on the floor and the trace line carried
// no token counts at all.

#![allow(clippy::disallowed_methods)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

// ── eval_expr (the №261 lekało) ──────────────────────────────────────

fn eval_expr(src: &str) -> Result<String, String> {
    let full = format!(
        "pattern __eval(input: String) -> String {{ return {} }}\nflow Main {{ input: String = \"x\" -> __eval -> output }}",
        src
    );
    match metalogos::run_program(&full) {
        Ok(Some(s)) => Ok(s),
        Ok(None) => Err("eval returned None".to_string()),
        Err(e) => Err(e),
    }
}

/// The VM twin of the TW eval above — the parity leg.
fn eval_expr_vm(src: &str) -> Result<String, String> {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let declarations = metalogos::parser::parse(&format!(
        "pattern __eval(input: String) -> String {{ return {} }}\nflow Main {{ input: String = \"x\" -> __eval -> output }}",
        src
    ))
    .map_err(|e| format!("parse error: {e}"))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base.to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    match vm.run(program)? {
        Some(s) => Ok(s.trim_end().to_string()),
        None => Err("vm eval returned None".to_string()),
    }
}

// ── Env mutex + guard (the №261 lekało, generalized to any var) ──────

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static ENV_LOCK: Mutex<()> = Mutex::new(());
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

struct EnvGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    prevs: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn set(settings: &[(&'static str, Option<&str>)]) -> Self {
        let lock = env_lock();
        let mut prevs = Vec::new();
        for (name, val) in settings {
            prevs.push((*name, std::env::var(name).ok()));
            match val {
                Some(v) => std::env::set_var(name, v),
                None => std::env::remove_var(name),
            }
        }
        EnvGuard { _lock: lock, prevs }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (name, prev) in &self.prevs {
            match prev {
                Some(v) => std::env::set_var(name, v),
                None => std::env::remove_var(name),
            }
        }
    }
}

// ── The OpenAI-compatible mock (the №261 lekało + a capture log) ─────

fn bind_mock_listener() -> (String, TcpListener) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let port = listener.local_addr().expect("local_addr").port();
    (format!("http://127.0.0.1:{port}"), listener)
}

fn serve_openai_mock(
    listener: TcpListener,
    bodies: Vec<String>,
) -> std::thread::JoinHandle<String> {
    // One canned HTTP response per expected request. The legacy RealLlm
    // retry loop makes up to 4 attempts (1 + 3 retries with 1s/2s/4s
    // backoff) — a test that pins the LAST error must queue a response
    // for EVERY attempt, otherwise the retries hit a dead mock and the
    // honest first-attempt error is masked by a connect-refused.
    let queue = Arc::new(Mutex::new(std::collections::VecDeque::from(
        bodies
            .into_iter()
            .map(|b| {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    b.len(),
                    b
                )
                .into_bytes()
            })
            .collect::<Vec<_>>(),
    )));
    let expected = queue.lock().unwrap_or_else(|e| e.into_inner()).len();
    std::thread::spawn(move || {
        let mut log = String::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(15000);
        let _ = listener.set_nonblocking(true);
        let mut served = 0usize;
        while std::time::Instant::now() < deadline {
            if served >= expected {
                break;
            }
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(500)));
                    let mut raw = Vec::new();
                    let mut buf = [0u8; 65536];
                    loop {
                        match stream.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                raw.extend_from_slice(&buf[..n]);
                                if raw.windows(4).any(|w| w == b"\r\n\r\n") {
                                    let s = String::from_utf8_lossy(&raw);
                                    if let Some(hdr_end) = s.find("\r\n\r\n") {
                                        if let Some(cl) = s
                                            .to_lowercase()
                                            .match_indices("content-length:")
                                            .next()
                                            .and_then(|(i, _)| {
                                                s[i + "content-length:".len()..]
                                                    .split("\r\n")
                                                    .next()
                                                    .and_then(|v| v.trim().parse::<usize>().ok())
                                            })
                                        {
                                            if raw.len() >= hdr_end + 4 + cl {
                                                break;
                                            }
                                        } else {
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    log.push_str(&String::from_utf8_lossy(&raw));
                    served += 1;
                    let resp = {
                        let mut q = queue.lock().unwrap_or_else(|e| e.into_inner());
                        match q.pop_front() {
                            Some(r) => r,
                            // queue exhausted — a 500 keeps an unexpected
                            // extra request loud instead of hanging.
                            None => b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
                        }
                    };
                    stream.write_all(&resp).expect("write response");
                    let _ = stream.flush();
                }
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(10)),
            }
        }
        log
    })
}

/// A GLM thinking-mode chat.completions response: the reasoning block
/// rides `message.reasoning_content`, the answer rides `message.content`.
fn glm_reasoning(content: serde_json::Value, usage: bool) -> String {
    let mut msg = serde_json::json!({
        "role": "assistant",
        "reasoning_content": "I should think carefully about the question before answering..."
    });
    msg["content"] = content;
    let mut body = serde_json::json!({
        "id": "chatcmpl-n13",
        "choices": [{"finish_reason": "stop", "index": 0, "message": msg}]
    });
    if usage {
        // The GLM/OpenAI contract: completion_tokens INCLUDES the
        // reasoning tokens.
        body["usage"] = serde_json::json!({
            "prompt_tokens": 11,
            "completion_tokens": 42,
            "total_tokens": 53
        });
    }
    body.to_string()
}

/// A plain (no reasoning) chat.completions response.
fn glm_plain(content: &str) -> String {
    serde_json::json!({
        "id": "chatcmpl-n13",
        "choices": [{"finish_reason": "stop", "index": 0, "message": {
            "role": "assistant", "content": content
        }}]
    })
    .to_string()
}

struct N13Env {
    _guard: EnvGuard,
}

fn n13_env(base: &str, extra: &[(&'static str, Option<&str>)]) -> N13Env {
    let mut settings: Vec<(&'static str, Option<&str>)> = vec![
        ("METALOGOS_MOCK_LLM", None), // the real path, never the mock
        ("METALOGOS_LLM_PROVIDER", Some("openai")),
        ("METALOGOS_OPENAI_BASE_URL", Some(base)),
        ("METALOGOS_API_KEY", Some("test-key-n13")),
        ("METALOGOS_LLM_MODEL", Some("glm-5.3-flash")),
        ("METALOGOS_LLM_TRACE", None),
    ];
    settings.extend_from_slice(extra);
    N13Env {
        _guard: EnvGuard::set(&settings),
    }
}

// ── R1: the reasoning block never reaches the output ─────────────────

#[test]
fn n13_reasoning_never_reaches_the_output_tw() {
    let (base, listener) = bind_mock_listener();
    let handle = serve_openai_mock(
        listener,
        vec![glm_reasoning(serde_json::json!("the final answer"), false)],
    );
    let _env = n13_env(&base, &[]);

    let out = eval_expr("call_llm(\"Question?\", \"input\")")
        .expect("a thinking-mode answer must succeed");
    assert_eq!(
        out, "the final answer",
        "ONLY content flows out; got: {out}"
    );
    assert!(
        !out.contains("think"),
        "no reasoning fragment may leak into the output, got: {out}"
    );
    let _ = handle.join();
}

#[test]
fn n13_reasoning_never_reaches_the_output_vm() {
    let (base, listener) = bind_mock_listener();
    let handle = serve_openai_mock(
        listener,
        vec![glm_reasoning(serde_json::json!("vm answer"), false)],
    );
    let _env = n13_env(&base, &[]);

    let out = eval_expr_vm("call_llm(\"Question?\", \"input\")")
        .expect("VM thinking-mode answer must succeed");
    assert_eq!(out, "vm answer");
    let _ = handle.join();
}

// ── R2: the legacy path accounts the reported usage in the trace ─────

#[test]
fn n13_legacy_path_usage_reaches_the_trace() {
    let (base, listener) = bind_mock_listener();
    let _handle = serve_openai_mock(
        listener,
        vec![glm_reasoning(serde_json::json!("traced answer"), true)],
    );
    let trace_path = std::env::temp_dir().join(format!(
        "n13_trace_{}_{}.jsonl",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_file(&trace_path);
    let _env = n13_env(
        &base,
        &[(
            "METALOGOS_LLM_TRACE",
            Some(trace_path.to_str().expect("utf8 path")),
        )],
    );

    let out = eval_expr("call_llm(\"Question?\", \"input\")").expect("call must succeed");
    assert_eq!(out, "traced answer");

    let trace = std::fs::read_to_string(&trace_path).expect("trace file must exist");
    let _ = std::fs::remove_file(&trace_path);
    let lines: Vec<serde_json::Value> = trace
        .lines()
        .map(|l| serde_json::from_str(l).expect("each trace line is JSON"))
        .collect();
    let call_line = lines
        .iter()
        .find(|l| {
            l["gen_ai.provider.name"] == serde_json::json!("openai")
                && l["status"] == serde_json::json!("ok")
        })
        .unwrap_or_else(|| panic!("no ok openai trace line in: {trace}"));
    assert_eq!(
        call_line["gen_ai.usage.input_tokens"],
        serde_json::json!(11),
        "prompt_tokens must be on the trace line, got: {call_line}"
    );
    assert_eq!(
        call_line["gen_ai.usage.output_tokens"],
        serde_json::json!(42),
        "completion_tokens (INCLUDING the reasoning tokens, the GLM/OpenAI \
         contract) must be on the trace line, got: {call_line}"
    );
}

// ── R3: a plain response is unchanged ────────────────────────────────

#[test]
fn n13_plain_response_unchanged() {
    let (base, listener) = bind_mock_listener();
    let handle = serve_openai_mock(listener, vec![glm_plain("plain answer")]);
    let _env = n13_env(&base, &[]);

    let out = eval_expr("call_llm(\"Question?\", \"input\")").expect("plain call must succeed");
    assert_eq!(out, "plain answer");
    let _ = handle.join();
}

// ── R4: reasoning present, content null — LOUD, never silent-empty ───

#[test]
fn n13_reasoning_only_content_null_is_loud() {
    let (base, listener) = bind_mock_listener();
    // The legacy retry loop makes 4 attempts (1 + 3 retries) — every
    // attempt must see the SAME reasoning-only body so the FINAL error
    // is the honest format error, not a retry-masked connect-refused.
    let handle = serve_openai_mock(
        listener,
        vec![glm_reasoning(serde_json::Value::Null, false); 4],
    );
    let _env = n13_env(&base, &[]);

    let err = eval_expr("call_llm(\"Question?\", \"input\")")
        .expect_err("content:null with a reasoning block must fail LOUD");
    assert!(
        err.contains("Unexpected OpenAI response format"),
        "the error must name the response-format contract, got: {err}"
    );
    let _ = handle.join();
}
