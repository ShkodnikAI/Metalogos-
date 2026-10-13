// ── Naryad N12 (gh#1220, platform-enabler, llm): per-call model choice ──
//
// `call_llm` grows an OPTIONAL third String argument — the model for THIS
// call (a direct provider model name, e.g. "glm-5.3", or a named profile
// alias resolved via `METALOGOS_LLM_MODEL_{ALIAS}`):
//
//   M1: `call_llm(prompt, input, model)` — the wire body carries the
//       explicit model ("glm-5.3") on the OpenAI-compatible legacy path.
//   M2: named profiles — with `METALOGOS_LLM_MODEL_flash=glm-5.3-flash`
//       the alias "flash" resolves to the env value on the wire (the
//       owner's decision: the primary model is glm-5.3-flash; both
//       glm-5.3-flash and glm-5.3 must work in ONE process).
//   M3: the default — `call_llm(prompt, input)` without the third
//       argument keeps the configured `METALOGOS_LLM_MODEL` unchanged
//       (backward compatibility).
//   M4: VM parity — the explicit-model call behaves identically on the
//       VM backend (shared registry handler; the parity is pinned so a
//       future VM-side shortcut cannot diverge).
//   M5: a non-String third argument is a LOUD argument-type error.
//
// The mock OpenAI-compatible server binds 127.0.0.1:0 (the №261 mock
// lekało) and is reached through METALOGOS_OPENAI_BASE_URL (№12). The
// env vars are read at CALL time (the №130 convention — set/remove
// under the env mutex).
//
// RED-first discipline: on main `call_llm` accepts 1..2 arguments, so
// every three-argument call fails at the registry arity gate and the
// wire never sees the requested model. M3 (the default) is the
// backward-compatibility leg — it must stay GREEN across the change.

#![allow(clippy::disallowed_methods)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Mutex;

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

/// The VM twin of the TW eval above — the parity leg (both backends
/// share the registry handler; the parity is pinned so a future
/// VM-side shortcut cannot diverge).
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
    /// `settings`: var name -> Some(value) to SET, None to REMOVE.
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

/// Serve ONE canned OpenAI chat.completions response and return the raw
/// request log (headers + body) so the test can assert on the wire body.
fn serve_openai_mock(listener: TcpListener, body: &str) -> std::thread::JoinHandle<String> {
    let canned = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
    .into_bytes();
    let canned_len = canned.len();
    std::thread::spawn(move || {
        let mut log = String::new();
        // The safety valve: far beyond any honest test's needs; the
        // EARLY exit below is the normal path.
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(8000);
        let _ = listener.set_nonblocking(true);
        let mut served = 0usize;
        while std::time::Instant::now() < deadline {
            if served >= 1 {
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
                    stream
                        .write_all(&canned[..canned_len])
                        .expect("write response");
                    let _ = stream.flush();
                }
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(10)),
            }
        }
        log
    })
}

/// A GLM-shaped OpenAI chat.completions response (N13 shape: no
/// reasoning_content here — that naryad pins its own purity legs).
fn glm_ok(text: &str) -> String {
    format!(
        "{{\"id\":\"chatcmpl-n12\",\"choices\":[{{\"finish_reason\":\"stop\",\"index\":0,\"message\":{{\"content\":\"{text}\",\"role\":\"assistant\"}}}}],\"model\":\"glm-5.3\",\"usage\":{{\"completion_tokens\":7,\"prompt_tokens\":5,\"total_tokens\":12}}}}"
    )
}

/// Extract the JSON body from the captured raw request log.
fn wire_body(log: &str) -> serde_json::Value {
    let body = log
        .split("\r\n\r\n")
        .nth(1)
        .unwrap_or_else(|| panic!("no request body in log: {log}"));
    serde_json::from_str(body).unwrap_or_else(|e| panic!("wire body not JSON ({e}): {body}"))
}

// ── The shared test setup: the real (non-mock-LLM) path to the local mock ──

struct N12Env {
    _guard: EnvGuard,
}

fn n12_env(base: &str, extra: &[(&'static str, Option<&str>)]) -> N12Env {
    let mut settings: Vec<(&'static str, Option<&str>)> = vec![
        ("METALOGOS_MOCK_LLM", None), // the real path, never the mock
        ("METALOGOS_LLM_PROVIDER", Some("openai")),
        ("METALOGOS_OPENAI_BASE_URL", Some(base)),
        ("METALOGOS_API_KEY", Some("test-key-n12")),
        ("METALOGOS_LLM_MODEL", Some("glm-5.3-flash")), // the owner's primary
        ("METALOGOS_LLM_TRACE", None),                  // keep the process quiet
    ];
    settings.extend_from_slice(extra);
    N12Env {
        _guard: EnvGuard::set(&settings),
    }
}

// ── M1: the explicit direct model reaches the wire ───────────────────

#[test]
fn n12_explicit_direct_model_reaches_the_wire() {
    let (base, listener) = bind_mock_listener();
    let handle = serve_openai_mock(listener, &glm_ok("ok-text"));
    let _env = n12_env(&base, &[]);

    let out = eval_expr("call_llm(\"Say ok\", \"input\", \"glm-5.3\")")
        .expect("call_llm with explicit model must succeed");
    assert_eq!(out, "ok-text", "the model text flows to the output");

    let log = handle.join().expect("mock join");
    let body = wire_body(&log);
    assert_eq!(
        body["model"].as_str(),
        Some("glm-5.3"),
        "the EXPLICIT per-call model must be on the wire, got: {log}"
    );
}

// ── M2: the named profile (env) resolves on the wire ─────────────────

#[test]
fn n12_named_profile_via_env_resolves() {
    let (base, listener) = bind_mock_listener();
    let handle = serve_openai_mock(listener, &glm_ok("profiled"));
    let _env = n12_env(
        &base,
        &[("METALOGOS_LLM_MODEL_flash", Some("glm-5.3-flash"))],
    );

    let out = eval_expr("call_llm(\"Say ok\", \"input\", \"flash\")")
        .expect("call_llm with a named profile must succeed");
    assert_eq!(out, "profiled");

    let log = handle.join().expect("mock join");
    let body = wire_body(&log);
    assert_eq!(
        body["model"].as_str(),
        Some("glm-5.3-flash"),
        "the alias 'flash' must resolve via METALOGOS_LLM_MODEL_flash, got: {log}"
    );
}

// ── M3: the default (no third argument) is unchanged ─────────────────

#[test]
fn n12_default_model_unchanged_without_third_arg() {
    let (base, listener) = bind_mock_listener();
    let handle = serve_openai_mock(listener, &glm_ok("defaulted"));
    let _env = n12_env(&base, &[]);

    let out = eval_expr("call_llm(\"Say ok\", \"input\")")
        .expect("two-argument call_llm must keep working");
    assert_eq!(out, "defaulted");

    let log = handle.join().expect("mock join");
    let body = wire_body(&log);
    assert_eq!(
        body["model"].as_str(),
        Some("glm-5.3-flash"),
        "no third argument -> the configured METALOGOS_LLM_MODEL, got: {log}"
    );
}

// ── M4: VM parity for the explicit-model call ────────────────────────

#[test]
fn n12_vm_parity_explicit_model() {
    let (base, listener) = bind_mock_listener();
    let handle = serve_openai_mock(listener, &glm_ok("vm-parity"));
    let _env = n12_env(&base, &[]);

    let out = eval_expr_vm("call_llm(\"Say ok\", \"input\", \"glm-5.3\")")
        .expect("VM call_llm with explicit model must succeed");
    assert_eq!(out, "vm-parity");

    let log = handle.join().expect("mock join");
    let body = wire_body(&log);
    assert_eq!(
        body["model"].as_str(),
        Some("glm-5.3"),
        "the VM path must carry the explicit model on the wire too, got: {log}"
    );
}

// ── M5: a non-String model argument is a loud type error ─────────────

#[test]
fn n12_non_string_model_arg_is_loud() {
    let (base, listener) = bind_mock_listener();
    let _handle = serve_openai_mock(listener, &glm_ok("never"));
    let _env = n12_env(&base, &[]);

    let err = eval_expr("call_llm(\"a\", \"b\", 42)")
        .expect_err("a Number model argument must be a loud error");
    assert!(
        err.contains("call_llm"),
        "the error must name the builtin, got: {err}"
    );
}
