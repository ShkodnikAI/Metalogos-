// ── НАРЯД №261: SSRF-пакет исходящего HTTP ──────────────────────────────
//
// Контракт (issue #277, docs/naryads-258-265-audit-tails.md §261):
//   C1: IPv4-mapped IPv6 (::ffff:a.b.c.d) блокируется — V6-ветка
//       разворачивает mapped-адрес и прогоняет V4-ветку: ::ffff:10.0.0.5,
//       ::ffff:169.254.169.254 (metadata), ::ffff:127.0.0.1 — блок;
//       ::ffff:8.8.8.8 (публичный в V4-части) — НЕ блок.
//   C2: unspecified (0.0.0.0 и ::) блокируются.
//   C3: CGNAT 100.64.0.0/10 (100.64.0.0 — 100.127.255.255, RFC 6598) и
//       benchmark 198.18.0.0/15 (198.18.0.0 — 198.19.255.255, RFC 2544)
//       блокируются с точными границами; соседи вне диапазона
//       (100.63.255.255, 100.128.0.1, 198.17.255.255, 198.20.0.1) — НЕ блок.
//   C4: 3xx НЕ ходятся (reqwest::redirect::Policy::none на всех четырёх
//       эгресс-билтинах): http_get/http_post возвращают тело 302-ответа
//       как есть, целевой путь /final вторым запросом НЕ запрашивается
//       (обход SSRF-пина одним редиректом закрыт; принимающая сторона —
//       локальный bind, без внешней сети).
//   C5: http_download за гейтом: без флага 127.0.0.1 — ГРОМКИЙ отказ
//       «SSRF guard» (паритет с http_get: это отказ политики, а не
//       «сеть не удалась»); с METALOGOS_HTTP_ALLOW_PRIVATE=1 обычное
//       скачивание работает, soft-контракт Ok(Bool) не тронут.
//
// Юнит-таблица проверяет is_blocked_address напрямую — это SSOT для
// check_url_ssrf (все http_* билтины), vision_fetch_weights и статического
// гейта MODEL_WEIGHTS_UNSAFE (audit.rs). Сетевые тесты — локальный bind
// 127.0.0.1:0; METALOGOS_HTTP_ALLOW_PRIVATE ставится/возвращается под
// env-мьютексом (лекало tests/naryad_244_vision_lora.rs:69). ВСЕ тесты
// файла берут мьютекс на всё тело, включая негативные: иначе позитивный
// тест соседнего потока подсадил бы флаг негативному.

use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, TcpListener};
use std::sync::Mutex;

use metalogos::builtins::{check_url_ssrf, is_blocked_address};

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

/// Сохранить/восстановить METALOGOS_HTTP_ALLOW_PRIVATE (лекало n260).
/// ВАЖНО: держит env-мьютекс ВСЮ свою жизнь — env переменная процесса
/// глобальна, и без мьютекса позитивный тест соседнего потока подсадил
/// бы флаг негативному (это не теория: первый прогон ловил ровно эту
/// гонку — check_url_ssrf видел «1» от redirect-теста).
struct EnvGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    prev: Option<String>,
}

impl EnvGuard {
    fn set(flag: Option<&str>) -> Self {
        let lock = env_lock();
        let prev = std::env::var("METALOGOS_HTTP_ALLOW_PRIVATE").ok();
        match flag {
            Some(v) => std::env::set_var("METALOGOS_HTTP_ALLOW_PRIVATE", v),
            None => std::env::remove_var("METALOGOS_HTTP_ALLOW_PRIVATE"),
        }
        EnvGuard { _lock: lock, prev }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.prev {
            Some(v) => std::env::set_var("METALOGOS_HTTP_ALLOW_PRIVATE", v),
            None => std::env::remove_var("METALOGOS_HTTP_ALLOW_PRIVATE"),
        }
    }
}

// ── Вспомогательное ──────────────────────────────────────────────────

fn v4(octets: [u8; 4]) -> IpAddr {
    IpAddr::V4(Ipv4Addr::from(octets))
}

fn v6(literal: &str) -> IpAddr {
    IpAddr::V6(literal.parse::<std::net::Ipv6Addr>().expect("v6 literal"))
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

// ── Мок: редирект-сервер ─────────────────────────────────────────────
//
// На ЛЮБОЙ путь, кроме /final, отвечает 302 Found + Location: /final +
// маркерным телом «n261-3xx-body»; на /final — 200 «final». Логирует
// сырые запросы. Регрессия (reqwest ходит по редиректу) видна как
// второй запрос «/final» в логе.

const REDIRECT_302_RESPONSE: &[u8] = b"HTTP/1.1 302 Found\r\nLocation: /final\r\nContent-Length: 13\r\nConnection: close\r\n\r\nn261-3xx-body";
const FINAL_200_RESPONSE: &[u8] =
    b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nfinal";

fn spawn_redirect_server() -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let port = listener.local_addr().expect("local_addr").port();
    let handle = std::thread::spawn(move || {
        let mut log = String::new();
        let _ = listener.set_nonblocking(true);
        // 1с хватит с запасом: если регрессия — reqwest сделает второй
        // запрос немедленно; если фикса на месте — просто ждём дедлайн.
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(1000);
        while std::time::Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(500)));
                    let mut raw = Vec::new();
                    let mut buf = [0u8; 8192];
                    loop {
                        match stream.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                raw.extend_from_slice(&buf[..n]);
                                if find(&raw, b"\r\n\r\n").is_some() {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    let req = String::from_utf8_lossy(&raw).into_owned();
                    let wants_final = req.contains(" /final");
                    log.push_str(&req);
                    let resp = if wants_final {
                        FINAL_200_RESPONSE
                    } else {
                        REDIRECT_302_RESPONSE
                    };
                    stream.write_all(resp).expect("write response");
                    let _ = stream.flush();
                    if log.matches("HTTP/1.1").count() >= 2 {
                        break;
                    }
                }
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(20)),
            }
        }
        log
    });
    (format!("http://127.0.0.1:{}/", port), handle)
}

// ── Мок: статический ответ (для download) ────────────────────────────

fn spawn_static_server(response: &'static [u8]) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let port = listener.local_addr().expect("local_addr").port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept one request");
        let mut raw: Vec<u8> = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            if find(&raw, b"\r\n\r\n").is_some() {
                break;
            }
            let n = stream.read(&mut buf).expect("read request");
            assert!(n > 0, "client closed before headers complete");
            raw.extend_from_slice(&buf[..n]);
        }
        stream.write_all(response).expect("write response");
        let _ = stream.flush();
        String::from_utf8_lossy(&raw).into_owned()
    });
    (format!("http://127.0.0.1:{}/", port), handle)
}

// ── C1: IPv4-mapped IPv6 ─────────────────────────────────────────────

#[test]
fn n261_mapped_ipv6_is_blocked_via_v4_branch() {
    // Точные кейсы обхода из аудита: metadata и private через mapped-форму
    assert!(
        is_blocked_address(&v6("::ffff:169.254.169.254")),
        "C1: ::ffff:169.254.169.254 (mapped metadata) must be blocked"
    );
    assert!(
        is_blocked_address(&v6("::ffff:10.0.0.5")),
        "C1: ::ffff:10.0.0.5 (mapped private) must be blocked"
    );
    assert!(
        is_blocked_address(&v6("::ffff:127.0.0.1")),
        "C1: ::ffff:127.0.0.1 (mapped loopback) must be blocked"
    );
    assert!(
        is_blocked_address(&v6("::ffff:192.168.1.1")),
        "C1: ::ffff:192.168.1.1 (mapped private) must be blocked"
    );
    // Публичный V4 в mapped-форме НЕ блокируется — гейт не стал шире по публичным
    assert!(
        !is_blocked_address(&v6("::ffff:8.8.8.8")),
        "C1: ::ffff:8.8.8.8 (mapped public) must NOT be blocked"
    );
    // Обычные публичные V6 не задеты
    assert!(
        !is_blocked_address(&v6("2001:4860:4860::8888")),
        "C1: public v6 must NOT be blocked"
    );
}

// ── C2: unspecified ──────────────────────────────────────────────────

#[test]
fn n261_unspecified_addresses_are_blocked() {
    assert!(
        is_blocked_address(&v4([0, 0, 0, 0])),
        "C2: 0.0.0.0 (unspecified) must be blocked"
    );
    assert!(
        is_blocked_address(&v6("::")),
        "C2: :: (unspecified) must be blocked"
    );
    // Соседние публичные классы не задеты
    assert!(
        !is_blocked_address(&"8.8.8.8".parse::<IpAddr>().unwrap()),
        "C2: 8.8.8.8 must NOT be blocked (no overreach)"
    );
    assert!(
        !is_blocked_address(&"203.0.113.1".parse::<IpAddr>().unwrap()),
        "C2: 203.0.113.1 (documentation) must NOT be blocked"
    );
}

// ── C3: CGNAT 100.64/10 и benchmark 198.18/15 ────────────────────────

#[test]
fn n261_cgnat_range_blocked_with_exact_bounds() {
    assert!(
        is_blocked_address(&v4([100, 64, 0, 0])),
        "C3: 100.64.0.0 (CGNAT lower bound) must be blocked"
    );
    assert!(
        is_blocked_address(&v4([100, 64, 1, 1])),
        "C3: 100.64.1.1 (CGNAT) must be blocked"
    );
    assert!(
        is_blocked_address(&v4([100, 100, 0, 0])),
        "C3: 100.100.0.0 (CGNAT middle) must be blocked"
    );
    assert!(
        is_blocked_address(&v4([100, 127, 255, 255])),
        "C3: 100.127.255.255 (CGNAT upper bound) must be blocked"
    );
    // Соседи вне диапазона — НЕ блокируются
    assert!(
        !is_blocked_address(&v4([100, 63, 255, 255])),
        "C3: 100.63.255.255 is below CGNAT — must NOT be blocked"
    );
    assert!(
        !is_blocked_address(&v4([100, 128, 0, 1])),
        "C3: 100.128.0.1 is above CGNAT — must NOT be blocked"
    );
}

#[test]
fn n261_benchmark_range_blocked_with_exact_bounds() {
    assert!(
        is_blocked_address(&v4([198, 18, 0, 0])),
        "C3: 198.18.0.0 (benchmark lower bound) must be blocked"
    );
    assert!(
        is_blocked_address(&v4([198, 18, 0, 1])),
        "C3: 198.18.0.1 (benchmark) must be blocked"
    );
    assert!(
        is_blocked_address(&v4([198, 19, 255, 255])),
        "C3: 198.19.255.255 (benchmark upper bound) must be blocked"
    );
    assert!(
        !is_blocked_address(&v4([198, 17, 255, 255])),
        "C3: 198.17.255.255 is below benchmark — must NOT be blocked"
    );
    assert!(
        !is_blocked_address(&v4([198, 20, 0, 1])),
        "C3: 198.20.0.1 is above benchmark — must NOT be blocked"
    );
}

// ── Регрессия: классы №130/№150 не ослаблены ─────────────────────────

#[test]
fn n261_legacy_blocked_classes_stay_blocked() {
    for literal in [
        "127.0.0.1",
        "127.255.255.255",
        "10.0.0.1",
        "172.16.0.1",
        "172.31.255.255",
        "192.168.1.1",
        "169.254.1.1",
        "169.254.169.254",
        "::1",
        "fe80::1",
        "fd00::1",
        "fc00::dead:beef",
    ] {
        let ip: IpAddr = literal.parse().unwrap();
        assert!(
            is_blocked_address(&ip),
            "regression: {} must STAY blocked (n130/n150)",
            literal
        );
    }
    for literal in [
        "8.8.8.8",
        "1.1.1.1",
        "203.0.113.1",
        "2001:4860:4860::8888",
        "2606:4700:4700::1111",
        "2001:db8::1",
    ] {
        let ip: IpAddr = literal.parse().unwrap();
        assert!(
            !is_blocked_address(&ip),
            "regression: {} must NOT be blocked (public)",
            literal
        );
    }
}

// ── C3+: URL-уровень гейта (check_url_ssrf) на новых классах ─────────

#[test]
fn n261_check_url_ssrf_blocks_cgnat_literal_and_passes_neighbor() {
    let _env = EnvGuard::set(None); // флаг НЕ установлен — гейт активен

    let blocked_result = check_url_ssrf("http://100.64.1.1/");
    assert!(
        blocked_result.is_err(),
        "C3: CGNAT literal URL must be rejected by check_url_ssrf"
    );
    let msg = blocked_result.unwrap_err();
    assert!(
        msg.contains("SSRF guard"),
        "C3: error should mention SSRF guard, got: {}",
        msg
    );
    assert!(
        msg.contains("100.64.1.1"),
        "C3: error should contain the blocked IP, got: {}",
        msg
    );

    // Сосед выше CGNAT — проходит (IP-литерал, DNS не нужен — offline-safe,
    // лекало n130 test_ssrf_public_ip_literal_passes)
    let ok_result = check_url_ssrf("http://100.128.0.1/");
    assert!(
        ok_result.is_ok(),
        "C3: 100.128.0.1 is outside CGNAT — check_url_ssrf must pass"
    );
}

// ── C4: 3xx не ходятся ───────────────────────────────────────────────

#[test]
fn n261_http_get_does_not_follow_302() {
    let _env = EnvGuard::set(Some("1"));
    let (url, server) = spawn_redirect_server();

    let result = eval_expr(&format!("http_get(\"{}r\")", url));
    let log = server.join().expect("redirect server thread");

    let out = result.expect("302-ответ должен вернуться как есть (не ошибкой)");
    assert_eq!(
        out, "n261-3xx-body",
        "C4: http_get должен вернуть ТЕЛО 302-ответа (redirect none), got: {}",
        out
    );
    assert!(
        log.contains("GET /r"),
        "C4: первый запрос должен быть на /r, got:\n{}",
        log
    );
    assert!(
        !log.contains("/final"),
        "C4: reqwest НЕ должен ходить по редиректу — второго запроса на /final быть не должно (обход SSRF-пина), got:\n{}",
        log
    );
}

#[test]
fn n261_http_post_does_not_follow_302() {
    let _env = EnvGuard::set(Some("1"));
    let (url, server) = spawn_redirect_server();

    let result = eval_expr(&format!(
        "http_post(\"{}r\", \"body-261\", \"text/plain\")",
        url
    ));
    let log = server.join().expect("redirect server thread");

    let out = result.expect("302-ответ должен вернуться как есть (не ошибкой)");
    assert_eq!(
        out, "n261-3xx-body",
        "C4: http_post должен вернуть ТЕЛО 302-ответа (redirect none), got: {}",
        out
    );
    assert!(
        log.contains("POST /r") && log.contains("body-261"),
        "C4: первый запрос должен быть POST /r с телом, got:\n{}",
        log
    );
    assert!(
        !log.contains("/final"),
        "C4: reqwest НЕ должен ходить по редиректу, got:\n{}",
        log
    );
}

// ── C5: http_download за гейтом ──────────────────────────────────────

#[test]
fn n261_http_download_private_without_flag_is_loud() {
    let _env = EnvGuard::set(None); // флаг НЕ установлен

    let dest = "n261_gate.bin";
    let _ = std::fs::remove_file(dest);

    let result = eval_expr(&format!(
        "http_download(\"http://127.0.0.1:22/\", \"{}\")",
        dest
    ));

    let err = match result {
        Err(e) => e,
        Ok(out) => panic!(
            "C5 РЕГРЕССИЯ: http_download прошёл мимо SSRF-гейта (soft-ответ {:?}) — \
             отказ гейта должен быть ГРОМКИМ (паритет с http_get)",
            out
        ),
    };
    assert!(
        err.contains("SSRF guard"),
        "C5: отказ гейта должен нести текст «SSRF guard», got: {}",
        err
    );
    assert!(
        err.contains("127.0.0.1"),
        "C5: ошибка должна называть заблокированный адрес, got: {}",
        err
    );
    assert!(
        !std::path::Path::new(dest).exists(),
        "C5: файл назначения не должен создаваться при отказе гейта"
    );
    let _ = std::fs::remove_file(dest);
}

#[test]
fn n261_http_download_with_flag_works_soft_contract_intact() {
    let _env = EnvGuard::set(Some("1"));

    let response: &'static [u8] =
        b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\nConnection: close\r\n\r\nn261-bytes!";
    let (url, server) = spawn_static_server(response);

    let dest = "n261_dl.bin";
    let _ = std::fs::remove_file(dest);

    let result = eval_expr(&format!("http_download(\"{}f\", \"{}\")", url, dest));
    let log = server.join().expect("static server thread");

    let content = std::fs::read_to_string(dest);
    let _ = std::fs::remove_file(dest);

    let out = result.expect("с флагом и 200-ответом скачивание должно удаться");
    assert_eq!(
        out, "true",
        "C5: http_download должен вернуть Bool(true) — soft-контракт Ok(Bool) не тронут, got: {:?}",
        out
    );
    assert!(
        log.contains("GET /f"),
        "C5: запрос должен уйти на мок-сервер, got:\n{}",
        log
    );
    assert_eq!(
        content.expect("файл должен быть записан"),
        "n261-bytes!",
        "C5: содержимое должно скачаться дословно"
    );
}

#[test]
fn n261_http_download_does_not_follow_302() {
    let _env = EnvGuard::set(Some("1"));
    let (url, server) = spawn_redirect_server();

    let dest = "n261_dl_302.bin";
    let _ = std::fs::remove_file(dest);

    let result = eval_expr(&format!("http_download(\"{}r\", \"{}\")", url, dest));
    let log = server.join().expect("redirect server thread");

    let content = std::fs::read_to_string(dest);
    let _ = std::fs::remove_file(dest);

    let out = result.expect("302-ответ записывается как есть, download успешен");
    assert_eq!(
        out, "true",
        "C4: 302 < 400 — download возвращает true (тело 3xx — это содержимое ответа), got: {:?}",
        out
    );
    assert_eq!(
        content.expect("файл должен быть записан"),
        "n261-3xx-body",
        "C4: в файл должно попасть ТЕЛО 302-ответа (redirect none), а не содержимое /final"
    );
    assert!(
        !log.contains("/final"),
        "C4: редирект НЕ должен быть отработан вторым запросом, got:\n{}",
        log
    );
}
