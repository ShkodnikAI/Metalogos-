// ── Naryad N3 (gh#1212, PE-1): the send_message opts pack ──────────────
//
// The Telegram delivery family grows the optional last `opts: Struct`
// argument and the new `send_photo` builtin:
//
//   O1: `send_message(chat_id, text, reply_markup?, opts?)` — the opts
//       fields: parse_mode ("HTML" | "MarkdownV2"), bot (the token comes
//       from TELEGRAM_BOT_TOKEN_<NAME>, the name uppercased),
//       disable_preview, chunk (default true), retry {max_retries,
//       base_delay}. The wire body carries `parse_mode` /
//       `disable_web_page_preview`; a 3-argument Struct WITHOUT any
//       known opts key stays the legacy reply_markup (backward compat).
//   O2: the chunking — chunk=true (the default) routes the text through
//       text_chunk (the "paragraph" strategy, budget 4000, overlap 0):
//       long texts leave as SEVERAL sendMessage calls at paragraph
//       boundaries, `reply_markup` rides ONLY the last chunk,
//       parse_mode rides every chunk; chunk=false keeps one call.
//   O3: the retries — a 429 honors the Telegram retry_after (the delay
//       capped at 30 s), a 5xx backs off base_delay * 2^attempt (the
//       same 30 s ceiling); up to max_retries (default 3) retries after
//       the initial call; other 4xx are loud immediately.
//   O4: the secret discipline — the bot token NEVER appears in an
//       error string (the reqwest URL Display and the response bodies
//       are redacted), and an explicitly named bot with no token env is
//       a loud error naming the ENV VAR (never a value).
//   O5: the new `send_photo(chat_id, photo, caption?, opts?)` — the
//       caption is refused loud beyond 1024 characters BEFORE any wire
//       traffic; parse_mode/retry/bot apply; chunk/disable_preview are
//       documented no-ops for a photo.
//   O6: the compile gate — untrusted text into send_message/send_photo
//       with the literal opts parse_mode "HTML" is the typed
//       [TELEGRAM_HTML_INJECTION] compile error (the HTML_INJECTION
//       analogy the naryad pins; escape_html lifts it). The existing
//       №325 lattice stays the general untrusted→network gate.
//
// The mock Telegram server binds 127.0.0.1:0 (the №261 mock lekało) and
// is reached through METALOGOS_TELEGRAM_API_BASE — the test/proxy seam
// mirroring METALOGOS_OPENAI_BASE_URL (№12). The env vars are read at
// CALL time (the №130 convention — set/remove under the env mutex).
//
// RED-first discipline: on main the opts argument does not exist (a
// 3-argument Struct is reply_markup), send_photo is an unknown builtin,
// the retries/chunking are absent, the token leaks into the reqwest URL
// error Display, and the compile gate does not exist. Every test below
// was RED on main before the implementation landed.

#![allow(clippy::disallowed_methods)]

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

// ── eval_expr (the №261 lekało) ──────────────────────────────────────

fn eval_expr(src: &str) -> Result<String, String> {
    // №845 (ADR-0203 §2.4): the harness permit — these tests pin the
    // DELIVERY-MECHANICS contracts (opts mapping, the wire format) against
    // a local mock; the capability layer requires the explicit carrier
    // permits for the delivery builtins (ADR-0203 §2.4).
    let full = format!(
        "profile capability {{ allow_carrier: \"send_message\" allow_carrier: \"send_photo\" }}\npattern __eval(input: String) -> String {{ return {} }}\nflow Main {{ input: String = \"x\" -> __eval -> output }}",
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
        "profile capability {{ allow_carrier: \"send_message\" allow_carrier: \"send_photo\" }}\npattern __eval(input: String) -> String {{ return {} }}\nflow Main {{ input: String = \"x\" -> __eval -> output }}",
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

// ── The Telegram mock (the №261 lekało + a canned-response queue) ────

/// Bind the mock listener FIRST (the port is stable), acquire the env
/// guard SECOND, and only then start the serving thread (serve_mock) —
/// under parallel test load the env-lock wait can exceed any fixed mock
/// lifetime, so the serving clock must start AFTER the lock is held.
fn bind_mock_listener() -> (String, TcpListener) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let port = listener.local_addr().expect("local_addr").port();
    (format!("http://127.0.0.1:{port}"), listener)
}

fn serve_mock(
    listener: TcpListener,
    responses: Vec<Vec<u8>>,
    expected_requests: usize,
) -> std::thread::JoinHandle<String> {
    let queue = Arc::new(Mutex::new(VecDeque::from(responses)));
    std::thread::spawn(move || {
        let mut log = String::new();
        let mut served = 0usize;
        // The safety valve: far beyond any honest test's needs; the
        // EARLY exit below is the normal path (the join returns the
        // moment the expected request count is served).
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(8000);
        let _ = listener.set_nonblocking(true);
        while std::time::Instant::now() < deadline {
            if served >= expected_requests {
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
                    log.push('\n');
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

fn tg_ok(body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
    .into_bytes()
}

fn tg_429(retry_after_secs: u64) -> Vec<u8> {
    let body = format!(
        "{{\"ok\":false,\"error_code\":429,\"description\":\"Too Many Requests: retry after {}\",\"parameters\":{{\"retry_after\":{}}}}}",
        retry_after_secs, retry_after_secs
    );
    format!(
        "HTTP/1.1 429 Too Many Requests\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
    .into_bytes()
}

fn tg_500() -> Vec<u8> {
    let body = "{\"ok\":false,\"error_code\":500,\"description\":\"Internal Server Error\"}";
    format!(
        "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
    .into_bytes()
}

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

const TOKEN: &str = "TESTTOKEN123456";

/// Standard env for the wire tests: the token + the API base seam.
fn wire_env(base: &str) -> EnvGuard {
    EnvGuard::set(&[
        ("TELEGRAM_BOT_TOKEN", Some(TOKEN)),
        ("METALOGOS_TELEGRAM_API_BASE", Some(base)),
    ])
}

// ── O1: the opts reach the wire ──────────────────────────────────────

#[test]
fn n3_opts_parse_mode_and_preview_reach_wire() {
    let (base, listener) = bind_mock_listener();
    let _env = wire_env(&base);
    let handle = serve_mock(listener, vec![tg_ok("{\"ok\":true,\"result\":{}}")], 1);
    let out = eval_expr(
        "type_of(send_message(\"123\", \"hello\", {parse_mode: \"HTML\", disable_preview: true}))",
    )
    .expect("send_message with opts must deliver");
    assert_eq!(
        out.trim(),
        "String",
        "delivered message returns the API response (String)"
    );
    let log = handle.join().expect("mock join");
    assert_eq!(
        count_occurrences(&log, &format!("POST /bot{TOKEN}/sendMessage")),
        1,
        "exactly one sendMessage call"
    );
    assert!(
        log.contains("\"parse_mode\":\"HTML\""),
        "the wire body carries parse_mode (on main the struct lands in reply_markup): {log}"
    );
    assert!(
        log.contains("\"disable_web_page_preview\":true"),
        "disable_preview maps to the Telegram disable_web_page_preview field: {log}"
    );
    assert!(
        !log.contains("\"reply_markup\""),
        "an opts-only struct is NOT a reply_markup: {log}"
    );
}

#[test]
fn n3_three_arg_struct_without_opts_keys_stays_reply_markup() {
    let (base, listener) = bind_mock_listener();
    let _env = wire_env(&base);
    let handle = serve_mock(listener, vec![tg_ok("{\"ok\":true,\"result\":{}}")], 1);
    eval_expr("send_message(\"123\", \"hello\", {inline_keyboard: [[{text: \"a\", callback_data: \"b\"}]]})")
    .expect("legacy 3-arg markup must keep working");
    let log = handle.join().expect("mock join");
    assert!(
        log.contains("\"reply_markup\""),
        "a struct with no known opts keys stays the legacy reply_markup: {log}"
    );
    assert!(
        !log.contains("\"parse_mode\""),
        "no opts -> no parse_mode on the wire: {log}"
    );
}

#[test]
fn n3_markup_plus_opts_four_args() {
    let (base, listener) = bind_mock_listener();
    let _env = wire_env(&base);
    let handle = serve_mock(listener, vec![tg_ok("{\"ok\":true,\"result\":{}}")], 1);
    eval_expr("send_message(\"123\", \"hello\", {inline_keyboard: [[{text: \"a\", callback_data: \"b\"}]]}, {parse_mode: \"HTML\"})")
    .expect("4-arg call: markup + opts");
    let log = handle.join().expect("mock join");
    assert!(
        log.contains("\"reply_markup\"") && log.contains("\"parse_mode\":\"HTML\""),
        "the 4th argument is opts, the 3rd stays reply_markup: {log}"
    );
}

#[test]
fn n3_opts_unknown_field_is_loud() {
    let _env = EnvGuard::set(&[("TELEGRAM_BOT_TOKEN", None)]);
    // A struct carrying a KNOWN opts key enters the strict opts parse —
    // unknown fields there are loud. (A 3-argument struct with ONLY
    // unknown keys is the legacy reply_markup by contract — inline_keyboard
    // and friends are unknown to the opts vocabulary by design.)
    let err =
        eval_expr("send_message(\"123\", \"hello\", {parse_mode: \"HTML\", parse_modee: 1.0})")
            .expect_err("unknown opts field must be loud");
    assert!(
        err.contains("parse_modee") && err.contains("unknown"),
        "the error names the unknown field: {err}"
    );
}

#[test]
fn n3_opts_parse_mode_value_validated() {
    let _env = EnvGuard::set(&[("TELEGRAM_BOT_TOKEN", None)]);
    let err = eval_expr("send_message(\"123\", \"hello\", {parse_mode: \"Markdown\"})")
        .expect_err("an unsupported parse_mode must be loud");
    assert!(
        err.contains("parse_mode") && (err.contains("HTML") && err.contains("MarkdownV2")),
        "the error names the allowed values: {err}"
    );
}

// ── O2: the chunking ─────────────────────────────────────────────────

fn long_paragraph_text_expr() -> String {
    // 220 paragraphs of ~40 chars -> ~9200 chars: several 4000-budget chunks.
    "repeat(\"the quick brown fox jumps over the lazy dog paragraph\\n\\n\", 220.0)".to_string()
}

#[test]
fn n3_chunking_delivers_several_messages_markup_on_last() {
    let (base, listener) = bind_mock_listener();
    let _env = wire_env(&base);

    let n_chunks: usize = eval_expr(&format!(
        "str(len(text_chunk({}, \"paragraph\", {{max_chars: 4000.0, overlap: 0.0}})))",
        long_paragraph_text_expr()
    ))
    .expect("text_chunk count")
    .trim()
    .parse()
    .expect("chunk count is a number");
    assert!(n_chunks > 1, "the fixture must span several chunks");

    // The serving thread starts only AFTER the chunk count is known —
    // the early-exit contract needs the expected request count.
    let handle = serve_mock(listener, vec![tg_ok("{\"ok\":true}"); 8], n_chunks);

    let markup = "{inline_keyboard: [[{text: \"go\", callback_data: \"go\"}]]}";
    eval_expr(&format!(
        "send_message(\"123\", {}, {}, {{parse_mode: \"HTML\"}})",
        long_paragraph_text_expr(),
        markup
    ))
    .expect("chunked delivery must succeed");

    let log = handle.join().expect("mock join");
    let calls = count_occurrences(&log, &format!("POST /bot{TOKEN}/sendMessage"));
    assert_eq!(
        calls, n_chunks,
        "one sendMessage per chunk (expected {n_chunks}, saw {calls})"
    );
    // parse_mode rides EVERY chunk.
    assert_eq!(
        count_occurrences(&log, "\"parse_mode\":\"HTML\""),
        n_chunks,
        "parse_mode on every chunk"
    );
    // reply_markup rides ONLY the last chunk.
    assert_eq!(
        count_occurrences(&log, "\"reply_markup\""),
        1,
        "reply_markup exactly once"
    );
    let last_call_start = log
        .rmatch_indices(&format!("POST /bot{TOKEN}/sendMessage"))
        .next()
        .map(|(i, _)| i)
        .expect("last call");
    assert!(
        log[last_call_start..].contains("\"reply_markup\""),
        "the LAST call carries the markup"
    );
}

#[test]
fn n3_chunk_false_keeps_one_call() {
    let (base, listener) = bind_mock_listener();
    let _env = wire_env(&base);
    let handle = serve_mock(listener, vec![tg_ok("{\"ok\":true}")], 1);
    eval_expr(&format!(
        "send_message(\"123\", {}, {{chunk: false}})",
        long_paragraph_text_expr()
    ))
    .expect("chunk=false delivers as one message");
    let log = handle.join().expect("mock join");
    assert_eq!(
        count_occurrences(&log, &format!("POST /bot{TOKEN}/sendMessage")),
        1,
        "chunk=false is exactly one wire call"
    );
}

#[test]
fn n3_short_text_is_one_call_by_default() {
    let (base, listener) = bind_mock_listener();
    let _env = wire_env(&base);
    let handle = serve_mock(listener, vec![tg_ok("{\"ok\":true}")], 1);
    eval_expr("send_message(\"123\", \"short\")").expect("short text delivers");
    let log = handle.join().expect("mock join");
    assert_eq!(
        count_occurrences(&log, &format!("POST /bot{TOKEN}/sendMessage")),
        1,
        "the default chunk=true must not split a short text"
    );
}

// ── O3: the retries ──────────────────────────────────────────────────

#[test]
fn n3_retry_429_honors_retry_after_then_succeeds() {
    let (base, listener) = bind_mock_listener();
    let _env = wire_env(&base);
    let handle = serve_mock(
        listener,
        vec![tg_429(0), tg_ok("{\"ok\":true,\"result\":{}}")],
        2,
    );
    let out =
        eval_expr("send_message(\"123\", \"hello\", {retry: {max_retries: 3.0, base_delay: 1.0}})")
            .expect("a 429 with a retry budget must be retried to success");
    assert!(
        out.contains("\"ok\":true"),
        "the retried call returns the 200 body: {out}"
    );
    let log = handle.join().expect("mock join");
    assert_eq!(
        count_occurrences(&log, &format!("POST /bot{TOKEN}/sendMessage")),
        2,
        "the initial call plus one retry"
    );
}

#[test]
fn n3_retry_429_default_budget_without_opts() {
    // The retries are part of the DEFAULT behavior (max 3 retries).
    let (base, listener) = bind_mock_listener();
    let _env = wire_env(&base);
    let handle = serve_mock(listener, vec![tg_429(0), tg_ok("{\"ok\":true}")], 2);
    eval_expr("send_message(\"123\", \"hello\")")
        .expect("the default retry budget covers a single 429");
    let log = handle.join().expect("mock join");
    assert_eq!(
        count_occurrences(&log, &format!("POST /bot{TOKEN}/sendMessage")),
        2,
        "the default policy retries a 429 once here"
    );
}

#[test]
fn n3_retry_429_exhaustion_is_loud() {
    // Always-429 with the default budget: 1 initial + 3 retries = 4 calls,
    // then the loud error.
    let (base, listener) = bind_mock_listener();
    let _env = wire_env(&base);
    let handle = serve_mock(listener, vec![tg_429(0); 6], 4);
    let err =
        eval_expr("send_message(\"123\", \"hello\")").expect_err("exhausted retries must be loud");
    let log = handle.join().expect("mock join");
    assert_eq!(
        count_occurrences(&log, &format!("POST /bot{TOKEN}/sendMessage")),
        4,
        "the default budget: 1 initial call + 3 retries (saw {})",
        count_occurrences(&log, &format!("POST /bot{TOKEN}/sendMessage"))
    );
    assert!(
        err.contains("429"),
        "the error names the last status: {err}"
    );
    assert!(
        !err.contains(TOKEN),
        "the error NEVER carries the token: {err}"
    );
}

#[test]
fn n3_retry_5xx_backs_off_then_succeeds() {
    let (base, listener) = bind_mock_listener();
    let _env = wire_env(&base);
    let handle = serve_mock(
        listener,
        vec![tg_500(), tg_500(), tg_ok("{\"ok\":true}")],
        3,
    );
    let out =
        eval_expr("send_message(\"123\", \"hello\", {retry: {max_retries: 3.0, base_delay: 1.0}})")
            .expect("5xx responses are retried within the budget");
    assert!(
        out.contains("\"ok\":true"),
        "the third attempt succeeds: {out}"
    );
    let log = handle.join().expect("mock join");
    assert_eq!(
        count_occurrences(&log, &format!("POST /bot{TOKEN}/sendMessage")),
        3,
        "two 5xx retried, the third attempt delivers"
    );
}

#[test]
fn n3_other_4xx_never_retried() {
    let (base, listener) = bind_mock_listener();
    let _env = wire_env(&base);
    let handle = serve_mock(
        listener,
        vec![tg_ok_full(
            "HTTP/1.1 400 Bad Request",
            "{\"ok\":false,\"error_code\":400,\"description\":\"Bad Request: chat not found\"}",
        )],
        1,
    );
    let err =
        eval_expr("send_message(\"123\", \"hello\")").expect_err("a plain 4xx is loud immediately");
    let log = handle.join().expect("mock join");
    assert_eq!(
        count_occurrences(&log, &format!("POST /bot{TOKEN}/sendMessage")),
        1,
        "a non-429/5xx error is never retried"
    );
    assert!(err.contains("400"), "the error names the status: {err}");
}

fn tg_ok_full(status_line: &str, body: &str) -> Vec<u8> {
    format!(
        "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
    .into_bytes()
}

#[test]
fn n3_retry_delay_cap_is_30_seconds() {
    // The pure delay helper: the 30 s ceiling is pinned WITHOUT sleeping.
    use metalogos::builtins::{telegram_retry_delay_ms, TgRetryCause};
    // A huge retry_after is capped at 30_000 ms.
    assert_eq!(
        telegram_retry_delay_ms(
            &TgRetryCause::RateLimit {
                retry_after_secs: Some(120.0)
            },
            0,
            1_000
        ),
        30_000,
        "the retry_after delay is capped at 30 s"
    );
    // Backoff growth is capped at 30_000 ms.
    assert_eq!(
        telegram_retry_delay_ms(&TgRetryCause::ServerError, 10, 25_000),
        30_000,
        "the backoff delay is capped at 30 s"
    );
    // A small retry_after passes through in ms.
    assert_eq!(
        telegram_retry_delay_ms(
            &TgRetryCause::RateLimit {
                retry_after_secs: Some(0.5)
            },
            0,
            1_000
        ),
        500,
        "retry_after 0.5 s = 500 ms"
    );
    // The attempt exponent: 1000 * 2^2 = 4000.
    assert_eq!(
        telegram_retry_delay_ms(&TgRetryCause::ServerError, 2, 1_000),
        4_000,
        "base_delay * 2^attempt"
    );
}

// ── O4: the secret discipline ────────────────────────────────────────

#[test]
fn n3_token_never_leaks_into_connect_error() {
    // A closed port: reqwest fails at connect; the Display of that error
    // embeds the FULL URL (with the token) — the builtin must redact it.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("port").port();
    drop(listener); // nothing listens there anymore
    let _env = EnvGuard::set(&[
        ("TELEGRAM_BOT_TOKEN", Some(TOKEN)),
        (
            "METALOGOS_TELEGRAM_API_BASE",
            Some(&format!("http://127.0.0.1:{port}")),
        ),
    ]);
    let err =
        eval_expr("send_message(\"123\", \"hello\")").expect_err("a dead endpoint is a loud error");
    assert!(
        !err.contains(TOKEN),
        "the error MUST NOT carry the token (on main the reqwest URL Display leaks it): {err}"
    );
    assert!(
        err.contains("send_message"),
        "the error still names the builtin (the redaction hits the token, not the diagnostics): {err}"
    );
}

#[test]
fn n3_token_never_leaks_into_error_body() {
    // A hostile/buggy endpoint ECHOES the token in the 401 body — the
    // redaction must catch the body path too.
    let echo_body = format!(
        "{{\"ok\":false,\"error_code\":401,\"description\":\"Unauthorized: bad {TOKEN}\"}}"
    );
    let (base, listener) = bind_mock_listener();
    let _env = wire_env(&base);
    let handle = serve_mock(
        listener,
        vec![tg_ok_full("HTTP/1.1 401 Unauthorized", &echo_body)],
        1,
    );
    let err = eval_expr("send_message(\"123\", \"hello\")").expect_err("a 401 is loud");
    let _ = handle.join();
    assert!(
        !err.contains(TOKEN),
        "the token echoed by the endpoint must be redacted: {err}"
    );
}

#[test]
fn n3_bot_name_resolves_named_env() {
    let (base, listener) = bind_mock_listener();
    let _env = EnvGuard::set(&[
        ("TELEGRAM_BOT_TOKEN", None),
        ("TELEGRAM_BOT_TOKEN_NOTIFY", Some("TOKNOTIFY789")),
        ("METALOGOS_TELEGRAM_API_BASE", Some(&base)),
    ]);
    let handle = serve_mock(listener, vec![tg_ok("{\"ok\":true}")], 1);
    eval_expr("send_message(\"123\", \"hello\", {bot: \"notify\"})")
        .expect("the named bot's token comes from TELEGRAM_BOT_TOKEN_NOTIFY");
    let log = handle.join().expect("mock join");
    assert!(
        log.contains("POST /botTOKNOTIFY789/sendMessage"),
        "the named bot's token hit the wire: {log}"
    );
    assert!(
        !log.contains(&format!("bot{TOKEN}")),
        "the default token must not be used when bot is named"
    );
}

#[test]
fn n3_bot_name_missing_env_is_loud_and_names_the_env_var() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("port").port();
    drop(listener); // a dead local endpoint — the RED run never leaves localhost
    let _env = EnvGuard::set(&[
        ("TELEGRAM_BOT_TOKEN", Some(TOKEN)),
        ("TELEGRAM_BOT_TOKEN_NOTIFY", None),
        (
            "METALOGOS_TELEGRAM_API_BASE",
            Some(&format!("http://127.0.0.1:{port}")),
        ),
    ]);
    let err = eval_expr("send_message(\"123\", \"hello\", {bot: \"notify\"})")
        .expect_err("an explicit bot with no token env is loud, not a silent stub");
    assert!(
        err.contains("TELEGRAM_BOT_TOKEN_NOTIFY"),
        "the error names the env var to set: {err}"
    );
    assert!(
        !err.contains(TOKEN),
        "the error must not carry the (unrelated) default token: {err}"
    );
}

// ── O5: send_photo ───────────────────────────────────────────────────

#[test]
fn n3_send_photo_delivers_caption_and_parse_mode() {
    let (base, listener) = bind_mock_listener();
    let _env = wire_env(&base);
    let handle = serve_mock(listener, vec![tg_ok("{\"ok\":true,\"result\":{}}")], 1);
    let out = eval_expr("type_of(send_photo(\"123\", \"https://example.org/x.jpg\", \"a caption\", {parse_mode: \"HTML\"}))")
    .expect("send_photo must deliver");
    assert_eq!(
        out.trim(),
        "String",
        "delivered photo returns the API response"
    );
    let log = handle.join().expect("mock join");
    assert!(
        log.contains(&format!("POST /bot{TOKEN}/sendPhoto")),
        "send_photo hits the sendPhoto API method: {log}"
    );
    assert!(
        log.contains("\"photo\":\"https://example.org/x.jpg\""),
        "{log}"
    );
    assert!(log.contains("\"caption\":\"a caption\""), "{log}");
    assert!(log.contains("\"parse_mode\":\"HTML\""), "{log}");
}

#[test]
fn n3_send_photo_without_caption() {
    let (base, listener) = bind_mock_listener();
    let _env = wire_env(&base);
    let handle = serve_mock(listener, vec![tg_ok("{\"ok\":true}")], 1);
    eval_expr("send_photo(\"123\", \"https://example.org/x.jpg\")").expect("caption is optional");
    let log = handle.join().expect("mock join");
    assert!(
        log.contains("sendPhoto") && !log.contains("\"caption\""),
        "{log}"
    );
}

#[test]
fn n3_send_photo_caption_over_1024_refused_before_wire() {
    let (base, listener) = bind_mock_listener();
    let _env = wire_env(&base);
    let handle = serve_mock(listener, vec![], 0);
    let err =
        eval_expr("send_photo(\"123\", \"https://example.org/x.jpg\", repeat(\"x\", 1025.0))")
            .expect_err("a caption beyond 1024 chars is refused loud");
    assert!(err.contains("1024"), "the error names the limit: {err}");
    let log = handle.join().expect("mock join");
    assert!(
        !log.contains("sendPhoto"),
        "the refusal happens BEFORE any wire traffic: {log}"
    );
}

#[test]
fn n3_send_photo_caption_exactly_1024_passes() {
    let (base, listener) = bind_mock_listener();
    let _env = wire_env(&base);
    let handle = serve_mock(listener, vec![tg_ok("{\"ok\":true}")], 1);
    eval_expr("send_photo(\"123\", \"https://example.org/x.jpg\", repeat(\"x\", 1024.0))")
        .expect("a 1024-char caption is inside the limit");
    let log = handle.join().expect("mock join");
    assert!(log.contains("sendPhoto"), "{log}");
}

#[test]
fn n3_send_photo_vm_parity() {
    let (base, listener) = bind_mock_listener();
    let _env = wire_env(&base);
    let handle = serve_mock(listener, vec![tg_ok("{\"ok\":true}")], 1);
    let out = eval_expr_vm("type_of(send_photo(\"123\", \"https://example.org/x.jpg\"))")
        .expect("the VM shares the registry handler");
    assert_eq!(out.trim(), "String");
    let log = handle.join().expect("mock join");
    assert!(log.contains("sendPhoto"), "{log}");
}

// ── O6: the compile gate ─────────────────────────────────────────────

fn compile(src: &str) -> Result<(), String> {
    match metalogos::compile_program(src) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

const GATE_UNTRUSTED_HTML: &str = "\
pattern Send(input: String) -> String {
  let _ = send_message(\"c\", query_param(\"q\"), {parse_mode: \"HTML\"})
  return \"ok\"
}
flow Main { input: String = \"x\" -> Send -> output }";

#[test]
fn n3_compile_gate_untrusted_text_with_html_parse_mode() {
    let err = compile(GATE_UNTRUSTED_HTML)
        .expect_err("untrusted text into parse_mode HTML must not compile");
    assert!(
        err.contains("[TELEGRAM_HTML_INJECTION]"),
        "the typed class fires (on main the error is the generic UNTRUSTED_EGRESS_NETWORK): {err}"
    );
    assert!(
        err.contains("escape_html"),
        "the message names the sanctioned fix: {err}"
    );
}

#[test]
fn n3_compile_gate_escaped_text_compiles() {
    let src = "\
pattern Send(input: String) -> String {
  let _ = send_message(\"c\", escape_html(query_param(\"q\")), {parse_mode: \"HTML\"})
  return \"ok\"
}
flow Main { input: String = \"x\" -> Send -> output }";
    compile(src).expect("escape_html lifts the gate");
}

#[test]
fn n3_compile_gate_trusted_literal_compiles() {
    let src = "\
pattern Send(input: String) -> String {
  let _ = send_message(\"c\", \"<b>static greeting</b>\", {parse_mode: \"HTML\"})
  return \"ok\"
}
flow Main { input: String = \"x\" -> Send -> output }";
    compile(src).expect("trusted literal text is the intended use");
}

#[test]
fn n3_compile_gate_markdown_v2_keeps_lattice_class() {
    // parse_mode MarkdownV2 is NOT the HTML gate's business; the general
    // №325 lattice still refuses untrusted text into the network sink.
    let src = "\
pattern Send(input: String) -> String {
  let _ = send_message(\"c\", query_param(\"q\"), {parse_mode: \"MarkdownV2\"})
  return \"ok\"
}
flow Main { input: String = \"x\" -> Send -> output }";
    let err = compile(src).expect_err("the lattice still fires");
    assert!(
        err.contains("[UNTRUSTED_EGRESS_NETWORK]") && !err.contains("[TELEGRAM_HTML_INJECTION]"),
        "MarkdownV2 keeps the lattice class, not the HTML gate: {err}"
    );
}

#[test]
fn n3_compile_gate_send_photo_caption_html() {
    let src = "\
pattern Send(input: String) -> String {
  let _ = send_photo(\"c\", \"https://example.org/x.jpg\", query_param(\"cap\"), {parse_mode: \"HTML\"})
  return \"ok\"
}
flow Main { input: String = \"x\" -> Send -> output }";
    let err = compile(src).expect_err("an untrusted caption with parse_mode HTML must not compile");
    assert!(
        err.contains("[TELEGRAM_HTML_INJECTION]"),
        "the gate covers the send_photo caption position: {err}"
    );
}

#[test]
fn n3_compile_gate_no_opts_keeps_legacy_behavior() {
    // Without parse_mode the existing surface is unchanged: the №325
    // lattice speaks (UNTRUSTED_EGRESS_NETWORK), the new gate is silent.
    let src = "\
pattern Send(input: String) -> String {
  let _ = send_message(\"c\", query_param(\"q\"))
  return \"ok\"
}
flow Main { input: String = \"x\" -> Send -> output }";
    let err = compile(src).expect_err("the lattice fires as on main");
    assert!(
        err.contains("[UNTRUSTED_EGRESS_NETWORK]") && !err.contains("[TELEGRAM_HTML_INJECTION]"),
        "no opts -> no new class: {err}"
    );
}

// ── O1 compat: the no-token stub ─────────────────────────────────────

#[test]
fn n3_no_token_stub_still_unit_with_opts() {
    let _env = EnvGuard::set(&[
        ("TELEGRAM_BOT_TOKEN", None),
        ("METALOGOS_TELEGRAM_API_BASE", None),
    ]);
    let out = eval_expr("type_of(send_message(\"123\", \"hello\", {parse_mode: \"HTML\"}))")
        .expect("the stub path works with opts too");
    assert_eq!(out.trim(), "Unit", "no token -> the audit-stub Unit (№543)");
    let out2 = eval_expr("type_of(send_photo(\"123\", \"https://example.org/x.jpg\"))")
        .expect("send_photo stubs the same way");
    assert_eq!(out2.trim(), "Unit", "no token -> the audit-stub Unit");
}
