// ── НАРЯД №257: url_decode_fallback — таблица ожиданий RFC 3986 ─────
//
// Контракт (issue #260): краевые случаи percent-декодера query-параметров
// покрыты тестами, поведение совпадает с задокументированным; зависимость
// НЕ добавлена (ручной декодер починен и задокументирован — см. док
// url_decode_fallback в src/server.rs; обоснование по dependency-
// дисциплине: ~20 строк, без новых поверхностей поставки).
//
// Выбранное и задокументированное поведение:
//   %XX-байты собираются и интерпретируются как UTF-8 (фикс mojibake
//   %D0%B6); `%ZZ`/обрезанный `%` — литеральный passthrough (строгий
//   RFC-рефект отклонён: парсинг query не должен падать на пользовательском
//   вводе); `+` → пробел (application/x-www-form-urlencoded — конвенция
//   HTML-форм и axum-тулинга); декодирование однопроходное; невалидный
//   UTF-8 — lossy (U+FFFD), декодер не падает.

use metalogos::server::url_decode_fallback as dec;

// ── Плоские строки и unreserved ─────────────────────────────────────

#[test]
fn rfc_plain_passthrough() {
    assert_eq!(dec("abcXYZ123"), "abcXYZ123");
    assert_eq!(dec("a-b._~"), "a-b._~"); // RFC 3986 unreserved
}

// ── Корректные escape-последовательности ────────────────────────────

#[test]
fn rfc_simple_escape() {
    assert_eq!(dec("%41"), "A");
    assert_eq!(dec("%2B"), "+"); // литеральный плюс — только через %2B
    assert_eq!(dec("q%20r"), "q r"); // %20 — пробел по RFC 3986
    assert_eq!(dec("%3A%2F%3F"), ":/?");
}

#[test]
fn rfc_multibyte_utf8_reassembles() {
    // КЛЮЧЕВОЙ фикс №257: до него %D0%B6 давало mojibake ("Ð¶") —
    // каждый байт пушался как отдельный char.
    assert_eq!(dec("%D0%B6"), "ж");
    assert_eq!(dec("%D0%B6%D0%BC"), "жм");
    assert_eq!(dec("name=%D0%98%D0%B2%D0%B0%D0%BD"), "name=Иван");
}

#[test]
fn rfc_lowercase_hex_accepted() {
    assert_eq!(dec("%d0%b6"), "ж"); // hex-регистр не важен
    assert_eq!(dec("%2b"), "+");
}

// ── Невалидные escape — литеральный passthrough (задокументировано) ──

#[test]
fn rfc_invalid_hex_passthrough() {
    assert_eq!(dec("%ZZ"), "%ZZ");
    assert_eq!(dec("%G1"), "%G1");
    assert_eq!(dec("100%"), "100%"); // обрезанный % в конце
    assert_eq!(dec("%2"), "%2"); // одна hex-цифра
    assert_eq!(dec("a%ZZb"), "a%ZZb"); // посреди строки
}

// ── `+` — пробел (form-urlencoded, выбрано и записано) ──────────────

#[test]
fn rfc_plus_is_space() {
    assert_eq!(dec("+"), " ");
    assert_eq!(dec("a+b"), "a b");
    assert_eq!(dec("hello+world+%D0%B6"), "hello world ж");
}

// ── Однопроходность (двойное кодирование требует два прохода) ───────

#[test]
fn rfc_double_encoding_single_pass() {
    // "ж" дважды закодированная = %25D0%25B6 → один проход даёт литерал "%D0%B6"
    assert_eq!(dec("%25D0%25B6"), "%D0%B6");
    // а ещё один проход уже дал бы "ж" — это работа вызывателя, не декодера
    assert_eq!(dec(dec("%25D0%25B6").as_str()), "ж");
}

// ── Lossy: невалидный UTF-8 не роняет декодер ───────────────────────

#[test]
fn rfc_invalid_utf8_is_lossy_not_panic() {
    // 0x80 — невалидный стартовый байт → U+FFFD, без паники
    assert_eq!(dec("%80"), "\u{FFFD}");
    // обрыв многобайтовой последовательности: 0xD0 без продолжения
    assert_eq!(dec("%D0!"), "\u{FFFD}!");
}

// ── Раунд-трип: encode → decode восстанавливает исходное ────────────

#[test]
fn rfc_roundtrip_cyrillic_and_specials() {
    let samples = [
        "ж",
        "Иван Петров",
        "a+b",     // литеральный плюс должен кодироваться как %2B
        "100%",    // процент кодируется как %25
        "q& r=?#", // reserved-символы
        "日本語",
        "mixed ж text 🚀",
    ];
    for s in samples {
        let encoded = encode_component(s);
        let decoded = dec(&encoded);
        assert_eq!(
            decoded, s,
            "roundtrip failed: {s:?} -> {encoded:?} -> {decoded:?}"
        );
    }
}

/// Эталонный энкодер (как encodeURIComponent, но `+` и пробел кодируются —
/// чтобы раунд-трип был честным при `+`→пробел семантике декодера).
fn encode_component(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

// ── End-to-end: query_param с кириллицей через реальный HTTP ────────

#[cfg(feature = "server")]
mod serve_e2e {
    use metalogos::server::ServeBackend;

    const SOURCE: &str = r#"
mlogserver {
  port: 8096
  route "/hi" method=GET {
    let name = query_param("name")
    respond("200", "hi " + name)
  }
}
"#;

    async fn start() -> u16 {
        let base_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let (port, _h) = metalogos::server::run_test_server_with_backend_in_dir(
            SOURCE,
            ServeBackend::Interpreter,
            base_dir,
        )
        .await
        .expect("server should start");
        port
    }

    #[tokio::test]
    async fn query_param_decodes_cyrillic_end_to_end() {
        let port = start().await;
        let url = format!("http://127.0.0.1:{}/hi?name=%D0%96%D0%B5%D0%BD%D1%8F", port);
        let resp = reqwest::get(&url).await.expect("GET should succeed");
        assert_eq!(resp.status().as_u16(), 200);
        let body = resp.text().await.expect("body");
        assert_eq!(
            body, "hi Женя",
            "мультибайт query-параметр должен декодироваться (был mojibake)"
        );
    }
}
