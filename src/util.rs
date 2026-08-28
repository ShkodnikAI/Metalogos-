// ── Shared utility functions ──────────────────────────────────────────

/// Truncate a string to approximately `max_bytes` bytes without splitting
/// a UTF-8 character. The last character whose start offset is < max_bytes
/// is included in full, so the returned slice may be up to 3 bytes longer
/// than `max_bytes` (max UTF-8 char size).
///
/// # Safety
///
/// Raw `&s[..N]` slicing panics at runtime if byte N falls in the middle
/// of a multi-byte UTF-8 character (e.g. Cyrillic letters are 2 bytes,
/// emoji are 3-4 bytes). This helper uses `char_indices()` to find the
/// last character boundary that fits within the limit.
///
/// Наряд №132: replaces six unsafe byte-slice sites that panicked on
/// Cyrillic/emoji user input.
pub(crate) fn safe_byte_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let end = s
        .char_indices()
        .take_while(|(i, _)| *i < max_bytes)
        .last()
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn n132_short_string_unchanged() {
        assert_eq!(safe_byte_truncate("hello", 10), "hello");
    }

    #[test]
    fn n132_exact_length_unchanged() {
        assert_eq!(safe_byte_truncate("hello", 5), "hello");
    }

    #[test]
    fn n132_ascii_truncate() {
        assert_eq!(safe_byte_truncate("hello world", 5), "hello");
    }

    // ── Cyrillic: each letter is 2 bytes ──

    #[test]
    fn n132_cyrillic_boundary_at_char_edge() {
        // "Привет" = 12 bytes (6 chars × 2 bytes)
        assert_eq!(safe_byte_truncate("Привет", 12), "Привет");
    }

    #[test]
    fn n132_cyrillic_boundary_in_middle_of_char() {
        // 15 кириллических букв = 30 байт.
        // trunc(29): символ 'П' начинается на байте 28 (< 29), включается
        // полностью → 30 байт. Граница символа не нарушена.
        let s = "АБВГДЕЖЗИКЛМНОП"; // 15 chars = 30 bytes
        let result = safe_byte_truncate(s, 29);
        assert_eq!(result, "АБВГДЕЖЗИКЛМНОП"); // 15 chars, 30 bytes
        assert!(result.is_char_boundary(result.len()));
        // Raw slice would panic: &s[..29] splits 'П'
        // (bytes 28-29 of a 2-byte char)
    }

    #[test]
    fn n132_cyrillic_30byte_boundary() {
        // Паттерн из execution.rs / modules.rs: len > 30 → truncate to 30
        let s = "АБВГДЕЖЗИКЛМНОПРСТУФ"; // 20 chars = 40 bytes
        let result = safe_byte_truncate(s, 30);
        // 'П' starts at byte 28 (< 30), included fully → 30 bytes
        assert_eq!(result, "АБВГДЕЖЗИКЛМНОП"); // 15 chars = 30 bytes
        assert!(result.is_char_boundary(result.len()));
    }

    #[test]
    fn n132_cyrillic_20byte_boundary() {
        // Паттерн из audit.rs: len > 20 → truncate to 20
        let s = "АБВГДЕЖЗИКЛМН"; // 13 chars = 26 bytes
        let result = safe_byte_truncate(s, 20);
        // 'К' starts at byte 18 (< 20), included fully → 20 bytes
        assert_eq!(result, "АБВГДЕЖЗИК"); // 10 chars = 20 bytes
        assert!(result.is_char_boundary(result.len()));
    }

    #[test]
    fn n132_empty_string() {
        assert_eq!(safe_byte_truncate("", 10), "");
    }

    #[test]
    fn n132_zero_max_bytes() {
        assert_eq!(safe_byte_truncate("hello", 0), "");
    }

    #[test]
    fn n132_emoji_4byte() {
        // "✅" = 3 bytes, "🚀" = 4 bytes
        let s = "✅🚀test"; // 3+4+4 = 11 bytes
                            // trunc(5): '🚀' starts at byte 3 (< 5), included fully → 7 bytes
        let result = safe_byte_truncate(s, 5);
        assert_eq!(result, "✅🚀"); // 7 bytes — no panic
        assert!(result.is_char_boundary(result.len()));
        // Raw &s[..5] would panic splitting '🚀' (bytes 3-6)
    }
}
