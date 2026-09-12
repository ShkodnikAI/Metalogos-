#![no_main]
use libfuzzer_sys::fuzz_target;

// ── Наряд №274: фаззинг redact(text, mode) (ADR-0136) ───────────────────
//
// `redact_string` принимает недоверенные строки (логи, письма, документы)
// и обязана быть безопасной на ЛЮБОМ входе. Инварианты:
//   1. панико-свобода (regex-движок линейный — наряд №54, ReDoS исключён
//      конструктивно; здесь пинится отсутствие паник/индексных паник);
//   2. детерминизм: одинаковый вход → одинаковая маска (требование наряда);
//   3. идемпотентность: маска не перетриггерит сама себя
//      redact(redact(s)) == redact(s).
//
// Граница: полнота маскирования (отсутствие ложных отрицаний энтропийной
// сети) НЕ ассертится — остаточный риск честно зафиксирован в ADR-0136.
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        for mode in ["pii", "secrets", "all"] {
            // Инвариант 1: панико-свобода.
            let once = match metalogos::builtins::redact_string(s, mode) {
                Ok(v) => v,
                Err(e) => panic!("redact({mode}) errored on valid UTF-8: {e}"),
            };

            // Инвариант 2: детерминизм.
            let again = metalogos::builtins::redact_string(s, mode)
                .expect("second call must not fail if the first succeeded");
            assert_eq!(once, again, "non-deterministic mask for mode {mode}");

            // Инвариант 3: идемпотентность.
            let twice = metalogos::builtins::redact_string(&once, mode)
                .expect("re-redaction of a mask must not fail");
            assert_eq!(once, twice, "mask re-triggered itself for mode {mode}");
        }
    }
});
