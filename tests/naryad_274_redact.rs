// ── Наряд №274 (ADR-0136): redact(text, mode) — PII/секреты как taint-санитайзер ──
//
// Контракт (стоп-гейт СГ-2, решение владельца 2026-09-12):
//   1. redact("secrets"|"all") снимает Secret-taint → Sanitized
//      («mask before sink» — легальный путь);
//   2. redact("pii") Secret НЕ снимает — инвариант
//      secret → redact("pii") → http_post ОТКЛОНЯЕТСЯ;
//   3. LlmOutput не снимается redact'ом вообще (санитайзер вывода
//      модели один — render; маскирование ≠ HTML-escape);
//   4. динамический mode — fail-closed (taint наследуется).
//
// Пара тестов DoD «рядом»: n274_secret_to_sink_rejected (без redact —
// отклоняется) и n274_secret_redact_secrets_to_sink_passes (с redact —
// проходит). Каждый паттерн-класс покрыт позитивом и негативом.

use metalogos::builtins::redact_string;

// ── Helpers ─────────────────────────────────────────────────────────────

fn redact_ok(text: &str, mode: &str) -> String {
    redact_string(text, mode).unwrap_or_else(|e| panic!("redact({:?}) failed: {}", mode, e))
}

fn check_has(source: &str, needle: &str) -> bool {
    let result = metalogos::check_program(source).unwrap();
    result.errors.iter().any(|e| e.message.contains(needle))
}

// ── Secret pattern classes: positive + negative per class ──────────────

#[test]
fn n274_sk_key_masked_positive() {
    let out = redact_ok("my key is sk-proj-abcdefghij1234567890abcd end", "secrets");
    assert!(
        !out.contains("sk-proj-abcdefghij1234567890abcd"),
        "raw key leaked: {}",
        out
    );
    assert!(
        out.contains("[REDACTED:sk-\u{2026}abcd]"),
        "typed mask expected: {}",
        out
    );
}

#[test]
fn n274_sk_prefix_word_negative() {
    // «skating» — не sk-ключ: после sk- нужно ≥16 символов класса.
    let out = redact_ok("we went skating rink side", "secrets");
    assert_eq!(
        out, "we went skating rink side",
        "false positive on prose: {}",
        out
    );
}

#[test]
fn n274_aws_akia_masked() {
    let out = redact_ok("AKIAIOSFODNN7EXAMPLE", "secrets");
    assert_eq!(out, "[REDACTED:AKIA\u{2026}MPLE]");
}

#[test]
fn n274_akia_lowercase_negative() {
    // AWS access key id — только uppercase+цифры после AKIA.
    let out = redact_ok("akiaiosfodnn7example", "secrets");
    assert_eq!(out, "akiaiosfodnn7example");
}

#[test]
fn n274_github_token_masked() {
    let out = redact_ok("token ghp_0123456789abcdefghijklmnopqrstuvwxyz", "secrets");
    assert!(!out.contains("ghp_0123456789"), "raw token leaked: {}", out);
    assert!(out.contains("[REDACTED:ghp_\u{2026}wxyz]"), "{}", out);
}

#[test]
fn n274_bearer_masked() {
    let out = redact_ok("Authorization: Bearer abcdefgh12345678.done", "secrets");
    assert!(!out.contains("abcdefgh12345678"), "{}", out);
    assert!(out.contains("[REDACTED:bearer\u{2026}done]"), "{}", out);
}

#[test]
fn n274_bearer_prose_negative() {
    // «bearer» в прозе без токена за ним не маскируется.
    let out = redact_ok("a bearer of gifts arrived", "secrets");
    assert_eq!(out, "a bearer of gifts arrived", "{}", out);
}

#[test]
fn n274_jwt_masked() {
    let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJVadQssw5c";
    let out = redact_ok(jwt, "secrets");
    assert!(!out.contains("eyJhbGciOiJIUzI1NiIs"), "{}", out);
    assert!(out.contains("[REDACTED:jwt\u{2026}sw5c]"), "{}", out);
}

#[test]
fn n274_pem_block_masked() {
    let pem = "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQ\nrandomstuff123\n-----END PRIVATE KEY-----";
    let out = redact_ok(pem, "secrets");
    assert!(!out.contains("MIIEvQIBADANBgkqhkiG9w0BAQ"), "{}", out);
    assert!(out.contains("[REDACTED:pem-block]"), "{}", out);
}

// ── Entropy net: safety net, honest negatives ───────────────────────────

#[test]
fn n274_entropy_base64_and_hex_masked() {
    let out = redact_ok(
        "blob aGVsbG8gd29ybGQgYmFzZTY0IHRva2Vu hash 0123456789abcdef0123456789abcdef01234567",
        "secrets",
    );
    assert!(
        out.contains("[REDACTED:entropy"),
        "base64/hex runs masked: {}",
        out
    );
    assert!(!out.contains("0123456789abcdef0123456789abcdef"), "{}", out);
}

#[test]
fn n274_entropy_word_and_digits_negative() {
    // Длинное слово без цифр и длинная цифровая строка без hex-букв —
    // НЕ маскируются (документированный фильтр сети: цифра + hex-буква).
    let out = redact_ok(
        "pneumonoultramicroscopicsilicovolcanoconiosis 123456789012345678901234567890",
        "secrets",
    );
    assert!(
        out.contains("pneumonoultramicroscopicsilicovolcanoconiosis"),
        "{}",
        out
    );
    assert!(out.contains("123456789012345678901234567890"), "{}", out);
}

// ── PII pattern classes: positive + negative per class ─────────────────

#[test]
fn n274_email_masked_preserves_tld() {
    let out = redact_ok("write to john.doe@acme.io today", "pii");
    assert_eq!(out, "write to ***@***.io today", "{}", out);
}

#[test]
fn n274_email_negative() {
    let out = redact_ok("not an email: a@b, also @example.com and foo@", "pii");
    assert_eq!(
        out, "not an email: a@b, also @example.com and foo@",
        "{}",
        out
    );
}

#[test]
fn n274_phone_intl_and_ru_masked() {
    let out = redact_ok("+7 926 123-45-67 or 8 (912) 555-66-77", "pii");
    assert_eq!(out, "[REDACTED:phone] or [REDACTED:phone]", "{}", out);
}

#[test]
fn n274_phone_negative() {
    // Короткие числа и годы не трогаются; голые 7+ цифр без +/8-формы — тоже.
    let out = redact_ok("year 2026, room 1234, order 1234567", "pii");
    assert_eq!(out, "year 2026, room 1234, order 1234567", "{}", out);
}

#[test]
fn n274_card_visa_masked_with_luhn() {
    let out = redact_ok("card 4111111111111111 on file", "pii");
    assert_eq!(
        out, "card [REDACTED:visa-card\u{2026}1111] on file",
        "{}",
        out
    );
}

#[test]
fn n274_card_spaced_amex_masked() {
    let out = redact_ok("pay 3782 822463 10005", "pii");
    assert!(out.contains("[REDACTED:amex-card"), "{}", out);
}

#[test]
fn n274_card_luhn_fail_negative() {
    // 1234567812345678 не проходит Luhn — не маскируется.
    let out = redact_ok("ref 1234567812345678", "pii");
    assert_eq!(out, "ref 1234567812345678", "{}", out);
}

#[test]
fn n274_iban_masked() {
    let out = redact_ok("wire DE44 5001 0517 5407 3249 31 now", "pii");
    assert_eq!(out, "wire [REDACTED:iban\u{2026}4931] now", "{}", out);
}

#[test]
fn n274_iban_negative() {
    let out = redact_ok("codes ABCD1234 and METALOGOS RULES", "pii");
    assert_eq!(out, "codes ABCD1234 and METALOGOS RULES", "{}", out);
}

// ── Mask determinism / idempotence / loud errors ────────────────────────

#[test]
fn n274_masks_deterministic() {
    let a = redact_ok("k=sk-abcdefghij1234567890, e=mail@site.org", "all");
    let b = redact_ok("k=sk-abcdefghij1234567890, e=mail@site.org", "all");
    assert_eq!(
        a, b,
        "same input must give the same mask (naryad requirement)"
    );
}

#[test]
fn n274_masks_idempotent() {
    let corpus = [
        "sk-proj-abcdefghij1234567890abcd",
        "AKIAIOSFODNN7EXAMPLE",
        "ghp_0123456789abcdefghijklmnopqrstuvwxyz",
        "Bearer abcdefgh12345678.done",
        "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJSMeKKF2QT4fwpQ",
        "john.doe@acme.io +7 926 123-45-67 4111111111111111",
        "DE44 5001 0517 5407 3249 31",
        "aGVsbG8gd29ybGQgYmFzZTY0IHRva2Vu",
        "-----BEGIN PRIVATE KEY-----\nMIIEvQ\n-----END PRIVATE KEY-----",
    ];
    for s in corpus {
        let once = redact_ok(s, "all");
        let twice = redact_ok(&once, "all");
        assert_eq!(once, twice, "mask must not re-trigger itself on {:?}", s);
    }
}

#[test]
fn n274_invalid_mode_loud() {
    let err = redact_string("text", "everything").unwrap_err();
    assert!(err.contains("unknown mode"), "{}", err);
    assert!(
        err.contains("pii") && err.contains("secrets") && err.contains("all"),
        "{}",
        err
    );
}

// ── Language-level: builtin end-to-end (TW runtime) ─────────────────────

#[test]
fn n274_language_level_redact() {
    let src = r#"
        pattern Main(_input: String) -> String {
            return redact("key sk-proj-abcdefghij1234567890abcd here", "secrets")
        }
        flow F { input: String = "x" -> Main -> output }
    "#;
    let out = metalogos::run_program(src)
        .expect("run")
        .expect("flow output");
    assert!(out.contains("[REDACTED:sk-\u{2026}abcd]"), "{}", out);
    assert!(
        !out.contains("abcdefghij1234567890"),
        "raw key in flow output: {}",
        out
    );
}

#[test]
fn n274_accepts_secret_value_runtime() {
    // Value::Secret (secret()) принимается — маскирование секрета на месте
    // и есть назначение билтина (результат — Value::String с маской).
    std::env::set_var(
        "METALOGOS_N274_TEST_KEY",
        "sk-proj-abcdefghij1234567890abcd",
    );
    let src = r#"
        pattern Main(_input: String) -> String {
            return redact(secret("METALOGOS_N274_TEST_KEY"), "secrets")
        }
        flow F { input: String = "x" -> Main -> output }
    "#;
    let out = metalogos::run_program(src)
        .expect("run")
        .expect("flow output");
    assert!(out.contains("[REDACTED:sk-"), "{}", out);
    assert!(!out.contains("abcdefghij1234567890"), "{}", out);
}

// ── Static taint invariants (check_program) — the ADR-0136 core ─────────

/// DoD-пара, член 1 (контроль): secret → sink отклоняется.
#[test]
fn n274_secret_to_sink_rejected() {
    let source = r#"
        mlogserver {
            port: 8080
            route "/leak" method=GET {
                let key = secret("API_KEY")
                respond(key)
            }
        }
    "#;
    assert!(
        check_has(source, "SECRET_LEAK"),
        "secret → respond must stay rejected (control)"
    );
}

/// DoD-пара, член 2: secret → redact("secrets") → sink проходит.
#[test]
fn n274_secret_redact_secrets_to_sink_passes() {
    let source = r#"
        mlogserver {
            port: 8080
            route "/ok" method=GET {
                let key = secret("API_KEY")
                let masked = redact(key, "secrets")
                respond(masked)
            }
        }
    "#;
    let result = metalogos::check_program(source).unwrap();
    let leaks: Vec<_> = result
        .errors
        .iter()
        .filter(|e| e.message.contains("SECRET_LEAK"))
        .collect();
    assert!(
        leaks.is_empty(),
        "masked secret must pass the audit: {:?}",
        leaks
    );
}

/// Третий инвариант СГ-2: secret → redact("pii") → http_post ОТКЛОНЯЕТСЯ.
#[test]
fn n274_secret_redact_pii_to_sink_rejected() {
    let source = r#"
        mlogserver {
            port: 8080
            route "/leak" method=POST {
                let key = secret("API_KEY")
                let masked = redact(key, "pii")
                let r = http_post("https://api.example.com", masked)
                return r
            }
        }
    "#;
    assert!(
        check_has(source, "SECRET_LEAK"),
        "pii-mode must NOT remove Secret-taint (owner decision D2)"
    );
}

/// mode "all" снимает Secret так же, как "secrets".
#[test]
fn n274_secret_redact_all_passes() {
    let source = r#"
        mlogserver {
            port: 8080
            route "/ok" method=GET {
                let masked = redact(env("API_KEY"), "all")
                respond(masked)
            }
        }
    "#;
    let result = metalogos::check_program(source).unwrap();
    let leaks: Vec<_> = result
        .errors
        .iter()
        .filter(|e| e.message.contains("SECRET_LEAK"))
        .collect();
    assert!(leaks.is_empty(), "{:?}", leaks);
}

/// http_post positional: body с redact("secrets") проходит, без — нет.
#[test]
fn n274_http_post_body_pair() {
    let rejected = r#"
        mlogserver {
            port: 8080
            route "/p1" method=POST {
                let r = http_post("https://api.example.com", env("K"))
                return r
            }
        }
    "#;
    assert!(
        check_has(rejected, "SECRET_LEAK"),
        "raw env in body must be rejected"
    );

    let accepted = r#"
        mlogserver {
            port: 8080
            route "/p2" method=POST {
                let r = http_post("https://api.example.com", redact(env("K"), "secrets"))
                return r
            }
        }
    "#;
    let result = metalogos::check_program(accepted).unwrap();
    let leaks: Vec<_> = result
        .errors
        .iter()
        .filter(|e| e.message.contains("SECRET_LEAK"))
        .collect();
    assert!(
        leaks.is_empty(),
        "inline redact in body must pass: {:?}",
        leaks
    );
}

/// Маскирование ≠ HTML-escape: LlmOutput не снимается redact'ом вообще.
#[test]
fn n274_llm_redact_still_html_injection() {
    let source = r#"
        mlogserver {
            port: 8080
            route "/ask" method=POST {
                let data = form_data()
                let reply = call_llm(data.question)
                let masked = redact(reply, "all")
                respond(masked)
            }
        }
    "#;
    assert!(
        check_has(source, "HTML_INJECTION"),
        "redact must NOT wash LlmOutput — only render does (ADR-0136 D2)"
    );
}

/// Динамический mode — fail-closed: taint наследуется без снятия.
#[test]
fn n274_dynamic_mode_fail_closed() {
    let source = r#"
        mlogserver {
            port: 8080
            route "/dyn" method=POST {
                let data = form_data()
                let masked = redact(env("K"), data.mode)
                respond(masked)
            }
        }
    "#;
    assert!(
        check_has(source, "SECRET_LEAK"),
        "non-literal mode must be fail-closed (ADR-0136)"
    );
}

// ── Arity pin ───────────────────────────────────────────────────────────

#[test]
fn n274_arity_pin() {
    let registered = metalogos::builtins::builtin_names();
    let idx = registered
        .iter()
        .position(|n| n == "redact")
        .expect("redact registered");
    assert!(idx > 0, "redact must be in the registry");
    // Arity 2: один аргумент — громкая ошибка времени выполнения.
    let err = metalogos::run_program(
        "pattern M(_input: String) -> String { return redact(\"only-text\") } flow F { input: String = \"x\" -> M -> output }",
    )
    .unwrap_err();
    assert!(err.contains("redact()"), "{}", err);
}

// ── Local fuzz smoke (cargo-fuzz в контейнере нет — см. PR-девиацию) ────
/// Детерминированный прогон 20k псевдослучайных входов через все три
/// инварианта fuzz-цели (панико-свобода / детерминизм / идемпотентность).
/// Полная версия — fuzz/fuzz_targets/fuzz_target_redact.rs в fuzz-smoke CI.
#[test]
fn n274_fuzz_smoke_local() {
    // LCG: воспроизводимый поток байт без внешних зависимостей.
    let mut state: u64 = 0x4D4554414C4F474F; // "METALOGO"
    let mut next_byte = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 33) as u8
    };
    for iteration in 0..20_000usize {
        let len = (next_byte() as usize) % 160;
        let bytes: Vec<u8> = (0..len).map(|_| next_byte()).collect();
        let Ok(s) = std::str::from_utf8(&bytes) else {
            continue;
        };
        for mode in ["pii", "secrets", "all"] {
            let once = redact_string(s, mode)
                .unwrap_or_else(|e| panic!("iter {iteration}: redact errored: {e}"));
            let twice = redact_string(&once, mode).expect("re-redact");
            assert_eq!(once, twice, "iter {iteration}: mask re-triggered on {s:?}");
        }
    }
}
