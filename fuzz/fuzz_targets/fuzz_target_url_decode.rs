#![no_main]
use libfuzzer_sys::fuzz_target;

// ── Наряд №256: фаззинг ручного percent-декодера query-параметров ───
//
// `url_decode_fallback(&str)` (src/server.rs) принимает внешние данные
// от пользователей сервера.
//
// Инварианты:
//   1. не паникует и не читает за границей (на ЛЮБОМ входе, включая
//      невалидный UTF-8 — он отсекается здесь, в цели);
//   2. ASCII-раунд-трип: корректно percent-закодированная (uppercase hex,
//      всё кроме unreserved) ASCII-строка восстанавливается декодером 1:1.
//
// Граница (§8.4 наряда): раунд-трип для НЕ-ASCII входов не ассертится —
// декодер собирает байты как char'ы (мультибайт UTF-8 даёт mojibake), это
// корректностное расхождение с RFC 3986 — предмет наряда №257, который
// чинит декодер или документирует отклонение. Паник здесь нет и после
// №257 быть не должно: цель продолжает пинить панико-свободу.
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Инвариант 1: панико-свобода на любом корректном UTF-8 входе.
        let _ = metalogos::server::url_decode_fallback(s);

        // Инвариант 2: ASCII-раунд-трип.
        if s.is_ascii() {
            let encoded = percent_encode_upper(s.as_bytes());
            let reparsed = std::str::from_utf8(&encoded).expect("encoder is ASCII-only");
            let decoded = metalogos::server::url_decode_fallback(reparsed);
            assert_eq!(
                decoded, s,
                "ASCII roundtrip failed: {:?} -> {:?} -> {:?}",
                s, reparsed, decoded
            );
        }
    }
});

/// Минимальный энкодер в духе encodeURIComponent: оставляет unreserved
/// (ALPHA / DIGIT / `-` / `.` / `_` / `~`), всё остальное — %XX uppercase.
fn percent_encode_upper(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() * 3);
    for &b in bytes {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b),
            _ => {
                out.push(b'%');
                let hex = format!("{:02X}", b);
                out.push(hex.as_bytes()[0]);
                out.push(hex.as_bytes()[1]);
            }
        }
    }
    out
}
