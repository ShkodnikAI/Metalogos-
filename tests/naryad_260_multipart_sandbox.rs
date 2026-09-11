// ── НАРЯД №260: http_post_multipart — чтение файлов по сырым путям ──────
//
// Контракт (issue #276, docs/naryads-258-265-audit-tails.md §260):
// - файловые поля (3-й аргумент, Struct) читаются ТОЛЬКО внутри песочницы:
//   каждый путь прогоняется через sandbox_path_ex(path, ForRead) ДО
//   построения клиента и запроса;
// - нарушение песочницы (абсолютный путь, '..', symlink-побег) — ГРОМКАЯ
//   ошибка «file I/O sandbox: ...» со стабильным кодом [SANDBOX_VIOLATION]
//   (лекало write_file, №252/№254);
// - валидный относительный путь внутри песочницы отправляется как раньше
//   (принимающая сторона — локальный bind 127.0.0.1:0, без внешней сети;
//   METALOGOS_HTTP_ALLOW_PRIVATE=1 ставится на время теста под env-мьютексом
//   — лекало tests/naryad_244_vision_lora.rs:69 — и возвращается назад).
//
// Билтин-уровень через run_program (лекало
// tests/naryad_254_sandbox_violation.rs). CWD тестов = корень крейта:
// песочница = корень крейта. Негативные кейсы используют URL
// http://8.8.8.8/ (публичный IP-литерал — проходит SSRF-гейт №130 без
// DNS), при этом соединение НЕ открывается: sandbox-проверка падает
// раньше построения клиента. Регрессия фикса проявилась бы как
// «request failed» (не содержит «file I/O sandbox») — тест честно красный.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Mutex;

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

// ── Env mutex (лекало tests/naryad_244_vision_lora.rs:69) ────────────

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static ENV_LOCK: Mutex<()> = Mutex::new(());
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ── Принимающая сторона: локальный HTTP-сервер на 127.0.0.1:0 ────────

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Bind 127.0.0.1:0, принять РОВНО один запрос (headers + тело по
/// Content-Length, с fallback на chunked) и ответить 200 "ok".
/// Возвращает (url, JoinHandle → сырой текст запроса для assert'ов).
fn spawn_receiver() -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let port = listener.local_addr().expect("local_addr").port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept one request");
        let mut raw: Vec<u8> = Vec::new();
        let mut buf = [0u8; 8192];
        // 1) читаем заголовки
        let header_end = loop {
            if let Some(pos) = find(&raw, b"\r\n\r\n") {
                break pos;
            }
            let n = stream.read(&mut buf).expect("read headers");
            assert!(n > 0, "client closed before headers complete");
            raw.extend_from_slice(&buf[..n]);
        };
        let headers = String::from_utf8_lossy(&raw[..header_end]).to_ascii_lowercase();
        let content_length = headers
            .lines()
            .find_map(|l| l.strip_prefix("content-length:"))
            .and_then(|v| v.trim().parse::<usize>().ok());
        // 2) читаем тело
        loop {
            let complete = if let Some(cl) = content_length {
                raw.len() >= header_end + 4 + cl
            } else if headers.contains("transfer-encoding: chunked") {
                find(&raw, b"\r\n0\r\n\r\n").is_some()
            } else {
                false
            };
            if complete {
                break;
            }
            let n = stream.read(&mut buf).expect("read body");
            assert!(n > 0, "client closed before body complete");
            raw.extend_from_slice(&buf[..n]);
        }
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .expect("write response");
        let _ = stream.flush();
        String::from_utf8_lossy(&raw).into_owned()
    });
    (format!("http://127.0.0.1:{}/", port), handle)
}

// ── (а) traversal — громкая ошибка ───────────────────────────────────

#[test]
fn n260_multipart_traversal_is_loud() {
    let err = eval_expr(
        "http_post_multipart(\"http://8.8.8.8/\", {src: \"n260\"}, {f: \"../../../etc/passwd\"})",
    )
    .unwrap_err();
    assert!(
        err.contains("file I/O sandbox"),
        "нарушение песочницы должно быть громким («file I/O sandbox»), got: {}",
        err
    );
    assert!(
        err.contains("[SANDBOX_VIOLATION]"),
        "громкий отказ должен нести стабильный код, got: {}",
        err
    );
}

// ── (б) абсолютный путь — громкая ошибка ─────────────────────────────

#[test]
fn n260_multipart_absolute_path_is_loud() {
    let err = eval_expr(
        "http_post_multipart(\"http://8.8.8.8/\", {src: \"n260\"}, {f: \"/etc/passwd\"})",
    )
    .unwrap_err();
    assert!(
        err.contains("file I/O sandbox"),
        "абсолютный путь должен быть громким отказом, got: {}",
        err
    );
    assert!(err.contains("[SANDBOX_VIOLATION]"), "got: {}", err);
}

// ── (в) symlink внутри песочницы, указывающий наружу — громко ────────
// (тот же SSOT, что №131/№252: canonicalize + prefix-проверка)

#[test]
#[cfg(unix)]
fn n260_multipart_symlink_escape_is_loud() {
    let link = "n260_escape_link.txt";
    let _ = std::fs::remove_file(link);
    std::os::unix::fs::symlink("/etc/passwd", link).expect("create symlink");
    let result = eval_expr(&format!(
        "http_post_multipart(\"http://8.8.8.8/\", {{src: \"n260\"}}, {{f: \"{}\"}})",
        link
    ));
    let _ = std::fs::remove_file(link);
    let err = result.unwrap_err();
    assert!(
        err.contains("file I/O sandbox"),
        "symlink-побег должен быть громким отказом, got: {}",
        err
    );
    assert!(err.contains("[SANDBOX_VIOLATION]"), "got: {}", err);
}

// ── (г) валидный относительный путь внутри песочницы — отправляется ──

#[test]
fn n260_multipart_file_inside_sandbox_is_sent() {
    let _guard = env_lock();
    let prev = std::env::var("METALOGOS_HTTP_ALLOW_PRIVATE").ok();
    std::env::set_var("METALOGOS_HTTP_ALLOW_PRIVATE", "1");

    let (url, receiver) = spawn_receiver();

    // Файл внутри песочницы (CWD = корень крейта) — легитимный кейс
    // «отправить файл, созданный программой».
    let path = "n260_upload.txt";
    std::fs::write(path, "payload-260").expect("write in-sandbox file");

    let program = format!(
        "http_post_multipart(\"{}\", {{note: \"hi\"}}, {{f: \"{}\"}})",
        url, path
    );
    let result = eval_expr(&program);

    // уборка + возврат окружения ДО assert'ов
    let _ = std::fs::remove_file(path);
    match prev {
        Some(v) => std::env::set_var("METALOGOS_HTTP_ALLOW_PRIVATE", v),
        None => std::env::remove_var("METALOGOS_HTTP_ALLOW_PRIVATE"),
    }

    let out = result.expect("in-sandbox file must be sent");
    assert_eq!(out, "ok", "приёмник должен ответить 200 с телом ok");

    let received = receiver.join().expect("receiver thread");
    assert!(
        received.contains("payload-260"),
        "содержимое файла должно доехать до приёмника, got:\n{}",
        received
    );
    assert!(
        received.contains("name=\"f\""),
        "поле файла f должно присутствовать, got:\n{}",
        received
    );
    assert!(
        received.contains("filename=\"n260_upload.txt\""),
        "multipart filename должен сохраниться, got:\n{}",
        received
    );
    assert!(
        received.contains("name=\"note\"") && received.contains("hi"),
        "текстовое поле note=hi должно доехать, got:\n{}",
        received
    );
}
