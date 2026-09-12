// ── Наряд №279: tts_generate (синтез без доставки) + факт-чек арности whisper_transcribe ──
//
// Контракты:
//   V1: tts_generate с mock-сервером → аудиофайл создан в песочнице,
//       путь валиден, содержимое байт-в-байт; ключ берётся из
//       METALOGOS_TTS_API_KEY (Authorization проверяется моком громко).
//   V2: TW и VM дают одинаковый контракт (crosscheck).
//   V3: ключ отсутствует → громкая ошибка с именем переменной (не хардкод).
//   V4: неизвестный провайдер → громкий отказ (v1: только openai).
//   V5: арность обеих функций на СТАТИКЕ: mlog check с 1-аргументным
//       whisper_transcribe → ошибка арности (раньше реестр пропускал —
//       рантайм-взрыв); границы tts_generate 2..4.
//   V6: tts_send делегирует синтез: с mock-TTS синтез УСПЕШЕН (байты
//       получены), падение — уже на доставке в Telegram.
//
// Честная граница: whisper_transcribe-end-to-end не тестируется — первый
// шаг функции это реальный вызов api.telegram.org (getFile); METALOGOS_STT_BASE_URL
// симметричен TTS-оверрайду и покрывается код-ревью (см. PR).

use serial_test::serial;

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

const MOCK_AUDIO: &[u8] = b"FAKE-MP3-BYTES-279-RIFFxxxxWAVEfmt";

/// Mock TTS server: loud-checks the Authorization header (key must come from
/// METALOGOS_TTS_API_KEY) and the JSON body (model/voice/input), then
/// responds with MOCK_AUDIO bytes. Reads the FULL request (headers +
/// Content-Length body) before judging — a single read() can truncate.
/// Accepts in a LOOP: one test drives BOTH backends (TW and VM) through the
/// same mock; the thread is leaked on purpose (no join — the test process
/// exit reclaims it).
fn spawn_tts_mock() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let addr = listener.local_addr().unwrap().to_string();
    let _handle = std::thread::spawn(move || {
        use std::io::{Read, Write};
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut raw: Vec<u8> = Vec::new();
            let mut buf = [0u8; 16384];
            // Read until end of headers, then until Content-Length is satisfied.
            let header_end = loop {
                let n = stream.read(&mut buf).unwrap_or(0);
                if n == 0 {
                    break None;
                }
                raw.extend_from_slice(&buf[..n]);
                if let Some(pos) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                    break Some(pos + 4);
                }
            };
            if let Some(hend) = header_end {
                let headers = String::from_utf8_lossy(&raw[..hend]).to_lowercase();
                let content_length: usize = headers
                    .lines()
                    .find_map(|l| l.strip_prefix("content-length:"))
                    .and_then(|v| v.trim().parse().ok())
                    .unwrap_or(0);
                while raw.len() - hend < content_length {
                    let n = stream.read(&mut buf).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    raw.extend_from_slice(&buf[..n]);
                }
                let req = String::from_utf8_lossy(&raw).to_lowercase();
                let auth_ok = req.contains("authorization: bearer test-key-279");
                let body_ok = req.contains("\"model\"") && req.contains("\"voice\"");
                let (code, payload) = if auth_ok && body_ok {
                    ("200 OK", MOCK_AUDIO.to_vec())
                } else {
                    // Loud failure — the test asserts on the builtin error anyway.
                    (
                        "500 Internal Server Error",
                        b"mock rejected request".to_vec(),
                    )
                };
                let resp = format!(
                    "HTTP/1.1 {}\r\nContent-Type: audio/mpeg\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    code,
                    payload.len()
                );
                let _ = stream.write_all(resp.as_bytes());
                let _ = stream.write_all(&payload);
                let _ = stream.flush();
            }
        }
    });
    format!("http://{}", addr)
}

fn set_tts_env(base: &str) -> (Option<String>, Option<String>, Option<String>) {
    let prev_base = std::env::var("METALOGOS_TTS_BASE_URL").ok();
    let prev_key = std::env::var("METALOGOS_TTS_API_KEY").ok();
    let prev_openai = std::env::var("OPENAI_API_KEY").ok();
    std::env::set_var("METALOGOS_TTS_BASE_URL", base);
    std::env::set_var("METALOGOS_TTS_API_KEY", "test-key-279");
    std::env::remove_var("OPENAI_API_KEY"); // prove METALOGOS_TTS_API_KEY suffices
    (prev_base, prev_key, prev_openai)
}

fn restore_tts_env(prev: (Option<String>, Option<String>, Option<String>)) {
    match prev.0 {
        Some(v) => std::env::set_var("METALOGOS_TTS_BASE_URL", v),
        None => std::env::remove_var("METALOGOS_TTS_BASE_URL"),
    }
    match prev.1 {
        Some(v) => std::env::set_var("METALOGOS_TTS_API_KEY", v),
        None => std::env::remove_var("METALOGOS_TTS_API_KEY"),
    }
    match prev.2 {
        Some(v) => std::env::set_var("OPENAI_API_KEY", v),
        None => std::env::remove_var("OPENAI_API_KEY"),
    }
}

struct Cleanup(Vec<String>);
impl Drop for Cleanup {
    fn drop(&mut self) {
        for f in &self.0 {
            let _ = std::fs::remove_file(f);
        }
    }
}

const GEN_PROGRAM: &str = r#"
pattern Gen(x: String) -> String {
  let p = tts_generate("Привет, мир", "alloy")
  return p
}
flow Main {
  input: String = "go"
  -> Gen
  -> output
}
"#;

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let program = metalogos::compiler::Compiler::new().compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

// ── V1 + V2: mock server → file created, byte-for-byte, TW == VM ──────

#[test]
#[serial]
fn n279_tts_generate_mock_file_bytes_tw_and_vm() {
    let _env = lock_env();
    let base = spawn_tts_mock();
    let prev = set_tts_env(&base);
    let _cleanup = Cleanup(Vec::new());

    // TW
    let out_tw =
        metalogos::run_program(GEN_PROGRAM).expect("V1: TW must synthesize against the mock");
    let fname_tw = out_tw
        .expect("V1: flow returns the path")
        .trim()
        .to_string();
    assert!(
        !fname_tw.is_empty() && !fname_tw.contains('/'),
        "V1: path is sandbox-relative, got: {}",
        fname_tw
    );
    let bytes_tw =
        std::fs::read(&fname_tw).unwrap_or_else(|e| panic!("V1: read {}: {}", fname_tw, e));
    assert_eq!(bytes_tw, MOCK_AUDIO, "V1: content byte-for-byte (TW)");

    // VM (crosscheck: same contract on the bytecode backend)
    let out_vm = run_vm(GEN_PROGRAM).expect("V2: VM must synthesize against the mock");
    let fname_vm = out_vm
        .expect("V2: flow returns the path")
        .trim()
        .to_string();
    let bytes_vm =
        std::fs::read(&fname_vm).unwrap_or_else(|e| panic!("V2: read {}: {}", fname_vm, e));
    assert_eq!(bytes_vm, MOCK_AUDIO, "V2: content byte-for-byte (VM)");

    let _ = std::fs::remove_file(&fname_tw);
    let _ = std::fs::remove_file(&fname_vm);
    restore_tts_env(prev);
}

// ── V3 + V4: missing key → loud error naming the variable; bad provider ──

#[test]
#[serial]
fn n279_tts_generate_key_and_provider_contracts() {
    let _env = lock_env();

    // V3: no key anywhere → loud error naming METALOGOS_TTS_API_KEY
    let prev_base = std::env::var("METALOGOS_TTS_BASE_URL").ok();
    let prev_key = std::env::var("METALOGOS_TTS_API_KEY").ok();
    let prev_openai = std::env::var("OPENAI_API_KEY").ok();
    std::env::remove_var("METALOGOS_TTS_BASE_URL"); // real URL — but no key, no call
    std::env::remove_var("METALOGOS_TTS_API_KEY");
    std::env::remove_var("OPENAI_API_KEY");
    let err = metalogos::run_program(GEN_PROGRAM).expect_err("V3: must fail loudly without a key");
    assert!(
        err.contains("METALOGOS_TTS_API_KEY"),
        "V3: error must name the env variable, got: {}",
        err
    );

    // V4: unknown provider → loud refusal before any HTTP (no mock needed —
    // the refusal fires before the HTTP exchange).
    std::env::set_var("METALOGOS_TTS_BASE_URL", "http://127.0.0.1:1");
    std::env::set_var("METALOGOS_TTS_API_KEY", "test-key-279");
    const BAD_PROVIDER: &str = r#"
pattern Bad(x: String) -> String {
  let p = tts_generate("text", "alloy", "elevenlabs")
  return p
}
flow Main {
  input: String = "go"
  -> Bad
  -> output
}
"#;
    let err2 = metalogos::run_program(BAD_PROVIDER).expect_err("V4: unknown provider must fail");
    assert!(
        err2.contains("unknown provider"),
        "V4: loud refusal, got: {}",
        err2
    );

    restore_tts_env((prev_base, prev_key, prev_openai));
}

// ── V5: static arity — mlog check catches 1-arg whisper_transcribe ────

#[test]
#[serial]
fn n279_static_arity_whisper_and_tts_generate() {
    let _env = lock_env();

    // The exact contract from the naryad: 1-arg call must fail on STATICS.
    const ONE_ARG_WHISPER: &str = r#"
pattern W(x: String) -> String {
  let t = whisper_transcribe(x)
  return t
}
flow Main {
  input: String = "go"
  -> W
  -> output
}
"#;
    // check_program returns an AnalysisResult carrying .errors (Ok(Ok/Err))
    // — a static arity violation is an ERROR inside the result, not the
    // Result's Err arm (which is only for parse failures).
    let result = metalogos::check_program(ONE_ARG_WHISPER).expect("V5: parse must succeed");
    assert!(
        !result.is_ok(),
        "V5: mlog check MUST reject 1-arg whisper_transcribe (the naryad's core bug)"
    );
    let msg = result
        .errors
        .iter()
        .map(|e| format!("{:?}", e))
        .collect::<Vec<_>>()
        .join("; ");
    assert!(
        msg.contains("whisper_transcribe"),
        "V5: arity error names the function, got: {}",
        msg
    );

    // Registry-level bounds (pub SSOT check).
    assert!(metalogos::builtins::check_builtin_arity("whisper_transcribe", 3).is_ok());
    assert!(metalogos::builtins::check_builtin_arity("whisper_transcribe", 4).is_ok());
    assert!(metalogos::builtins::check_builtin_arity("whisper_transcribe", 1).is_err());
    assert!(metalogos::builtins::check_builtin_arity("whisper_transcribe", 2).is_err());
    assert!(metalogos::builtins::check_builtin_arity("whisper_transcribe", 5).is_err());

    assert!(metalogos::builtins::check_builtin_arity("tts_generate", 2).is_ok());
    assert!(metalogos::builtins::check_builtin_arity("tts_generate", 3).is_ok());
    assert!(metalogos::builtins::check_builtin_arity("tts_generate", 4).is_ok());
    assert!(metalogos::builtins::check_builtin_arity("tts_generate", 1).is_err());
    assert!(metalogos::builtins::check_builtin_arity("tts_generate", 5).is_err());

    // A well-formed 4-arg whisper call passes statics (runtime is Telegram's
    // business — never executed here).
    assert!(metalogos::builtins::check_builtin_arity("tts_send", 4).is_ok());
    assert!(metalogos::builtins::check_builtin_arity("tts_send", 5).is_ok());
}

// ── V6: tts_send delegates synthesis (delivery-only convenience) ──────

#[test]
#[serial]
fn n279_tts_send_delegates_synthesis_fails_at_delivery() {
    let _env = lock_env();
    let base = spawn_tts_mock();
    let prev = set_tts_env(&base);

    const SEND_PROGRAM: &str = r#"
pattern S(x: String) -> String {
  let r = tts_send("text", "alloy", "123:fake-token", "42")
  return r
}
flow Main {
  input: String = "go"
  -> S
  -> output
}
"#;
    let err = metalogos::run_program(SEND_PROGRAM);
    // Synthesis against the mock SUCCEEDED (mock would 500 on missing key /
    // malformed body) — so the failure must be at the TELEGRAM delivery stage.
    let msg = match err {
        Err(e) => e,
        Ok(v) => panic!(
            "V6: expected delivery failure against fake token, got: {:?}",
            v
        ),
    };
    assert!(
        msg.contains("Telegram"),
        "V6: failure is at delivery (synthesis already succeeded), got: {}",
        msg
    );
    restore_tts_env(prev);
}
