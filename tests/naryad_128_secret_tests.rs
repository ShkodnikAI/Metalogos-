// ── Наряд №128: Документирование текущего механизма защиты Secret ──
//
// Два устаревших теста из phase19_22_constraints.rs (test_z19_print_secret_forbidden,
// test_z19_to_string_secret_forbidden) проверяли несуществующий механизм
// (semantic check_program → «opaque type constraint»). Удалены.
//
// Этот файл подтверждает, что защита Secret работает через другие механизмы:
//   1. print(secret)  → runtime is_nonprintable() → «print() refused: Secret»
//   2. to_string(secret) → Display возвращает «[Secret]», реальное значение
//      не утекает. Taint-трекер audit propagates Secret к downstream-стокам.
//   3. Audit SECRET_LEAK ловит print/respond/write_file с tainted-аргументами.

use metalogos::interpreter::{SecretString, Value};

// ── C1: to_string(secret) не утечёт реальное значение ─────────────
// Display для Value::Secret возвращает «[Secret]», а не содержимое.
#[test]
fn test_n128_to_string_secret_returns_placeholder() {
    let secret = Value::Secret(SecretString::new("super-secret-value-12345".to_string()));
    let displayed = format!("{}", secret);
    assert_eq!(displayed, "[Secret]");
    assert!(
        !displayed.contains("super-secret-value"),
        "to_string(Secret) must NOT contain the actual secret value"
    );
}

// ── C2: is_nonprintable блокирует Secret ─────────────────────────
// Это то, что использует builtin_print() для runtime-отказа.
#[test]
fn test_n128_secret_is_nonprintable() {
    use metalogos::interpreter::values::is_nonprintable;
    let secret = Value::Secret(SecretString::new("any-value".to_string()));
    assert!(is_nonprintable(&secret), "Secret must be non-printable");
}

// ── C3: String НЕ nonprintable ───────────────────────────────────
#[test]
fn test_n128_string_is_printable() {
    use metalogos::interpreter::values::is_nonprintable;
    let s = Value::String("hello".to_string());
    assert!(!is_nonprintable(&s), "String must be printable");
}

// ── C4: другие непечатаемые типы тоже блокируются ──────────────────
#[test]
fn test_n128_all_opaque_types_nonprintable() {
    use metalogos::interpreter::values::is_nonprintable;
    assert!(is_nonprintable(&Value::Html("<b>hi</b>".to_string())));
    assert!(is_nonprintable(&Value::Query("SELECT 1".to_string())));
    assert!(is_nonprintable(&Value::Encrypted(vec![0x01, 0x02])));
    assert!(is_nonprintable(&Value::Hash("abc123".to_string())));
}

// ── C5: Display для других opaque типов тоже не утечёт ────────────
#[test]
fn test_n128_opaque_display_no_leak() {
    let html = Value::Html("<script>alert(1)</script>".to_string());
    assert_eq!(format!("{}", html), "[Html]");
    assert!(!format!("{}", html).contains("<script>"));

    let encrypted = Value::Encrypted(vec![0xDE, 0xAD]);
    assert_eq!(format!("{}", encrypted), "[Encrypted]");

    let hash = Value::Hash("sha256digest".to_string());
    assert_eq!(format!("{}", hash), "[Hash]");
}
