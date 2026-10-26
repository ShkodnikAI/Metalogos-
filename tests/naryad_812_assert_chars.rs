// ── Naryad №812 (wave 66, P3 — the audit-23.10 B4, S-6) ───────────────
//
// The contract (issue #1466): `assert_contains` never PANICS — the
// failure preview slices by CHARS (the byte slice
// `&haystack[..min(len, 80)]` panicked with `byte index is not a char
// boundary` on a Cyrillic haystack past 80 bytes — the assert refusal
// became a crash), and the ARGUMENTS in the error text go through the
// redaction (a secret must not land in the error text — the needle and
// the haystack are data-controlled).
//
// RED-before (форма №772): the Cyrillic pin was run against the
// pre-№812 revision and PANICKED (the byte-boundary crash) instead of
// returning the refusal — the fix turns the crash into the pinned
// refusal.

use metalogos::builtins::spec_by_name;
use metalogos::interpreter::Value;

fn call_assert(haystack: &str, needle: &str) -> Result<Value, String> {
    let f = spec_by_name("assert_contains")
        .expect("registered")
        .handler
        .expect("the real handler");
    f(&[
        Value::String(haystack.to_string()),
        Value::String(needle.to_string()),
    ])
}

#[test]
fn n812_cyrillic_haystack_refuses_without_panicking() {
    // 25 Cyrillic letters = 50 bytes; past 80 bytes the old byte-slice
    // panicked. The refusal must come back as a loud Err, never a panic.
    let haystack = "привет".repeat(20); // 120 bytes, 120 chars
    let result = call_assert(&haystack, "нет-такого-слова");
    let err = match result {
        Ok(v) => panic!("the assert must refuse, got {v}"),
        Err(e) => e,
    };
    assert!(err.contains("assert_contains failed"), "{err}");
    // The preview is char-safe: the refusal carries Cyrillic text and
    // the ellipsis, no panic, no replacement garbage mid-char.
    assert!(
        err.contains('…'),
        "the truncated preview marks the cut: {err}"
    );
}

#[test]
fn n812_ascii_haystack_refusal_unchanged_shape() {
    // The legacy ASCII shape stays the same (the pin is the contract):
    // the refusal names the needle and the preview.
    let err = match call_assert("hello world", "missing") {
        Ok(v) => panic!("the assert must refuse, got {v}"),
        Err(e) => e,
    };
    assert!(err.contains("assert_contains failed"), "{err}");
    assert!(err.contains("'missing'"), "the needle is named: {err}");
}

#[test]
fn n812_secret_needle_is_redacted_in_the_error_text() {
    // The needle goes through the masking pipeline: a token-like secret
    // in the needle lands in the error text MASKED (the explicit
    // provider-token shape — the ADR-0136 redact("secrets") shape; the
    // ghp_ family is re_apikey's first-class citizen).
    //
    // The literal is the SANCTIONED synthetic shape (.gitleaks.toml, the
    // №459 allowlist doctrine: a synthetic fixture carries the EXAMPLE
    // marker). The first push carried a 72-char base64 blob instead —
    // the gitleaks default rule generic-api-key caught it and the head
    // went red (the tripwire worked as designed; a leak lands as a red
    // build, not as an allowlist entry).
    let secret = "ghp_EXAMPLE00000000000000000000000000000";
    let err = match call_assert("haystack", secret) {
        Ok(v) => panic!("the assert must refuse, got {v}"),
        Err(e) => e,
    };
    assert!(
        !err.contains(secret),
        "the secret must not ride the error text: {err}"
    );
    assert!(
        err.contains("[REDACTED:"),
        "the masked form is visible instead: {err}"
    );
}

#[test]
fn n812_happy_path_unchanged() {
    // The found case returns the haystack (the №514-ish passthrough
    // contract — the assert is a guard, not a transform).
    let out = call_assert("hello world", "world").expect("must pass");
    assert_eq!(format!("{out}"), "hello world");
}
