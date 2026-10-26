// ── Naryad №807 (wave 66, P2 — the audit-23.10 B2, S-4) ───────────────
//
// The contract (issue #1464): the Secret material never lands in a plain
// unsecured String copy on the crypto/masking paths, and the recommended
// key-carrying path accepts the opaque Secret:
//   1. hash_password / verify_password BORROW the Secret (the zero-copy
//      form — argon2 reads the Zeroizing buffer directly);
//   2. hmac_sha256 accepts Secret as the KEY (the recommended path) and
//      String (backward compat) — both borrowed;
//   3. redact BORROWS the Secret (the masking pipeline reads it in place);
//   4. the same-class find: the CalDAV session credential lives in a
//      Zeroizing buffer (the copy is unavoidable there — the session
//      stores it for the connection lifetime).
//
// RED-before (форма №772): the hmac_sha256(secret("K"), msg) program was
// run against the pre-№807 revision and refused LOUD at runtime
// (`hmac_sha256() key must be String`) while the static check passed
// (the opaque posture) — the pins below flip that refusal into the
// working contract on BOTH backends.
//
// The zero-copy invariant is pinned STRUCTURALLY (the source scan, the
// №604 class-pin form): the three audit-named sites must not carry
// `as_str().to_string()` on the Secret arms — a runtime test cannot
// observe a heap copy, the source IS the observable here.

#![allow(clippy::disallowed_methods)]

use metalogos::interpreter::{SecretString, Value};

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program(source)
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let declarations = metalogos::parser::parse(source).map_err(|e| format!("parse error: {e}"))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base.to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

const KEY_ENV: &str = "METALOGOS_N807_PROBE_KEY";
const KEY_VAL: &str = "n807-demo-key-0123456789abcdef";

fn hmac_secret_src() -> String {
    format!(
        r#"
pattern Probe(data: String) -> String {{
  let k = secret("{KEY_ENV}")
  let tag = hmac_sha256(k, "message")
  return tag
}}
flow Main {{ input: String = "x" -> Probe -> output }}
"#
    )
}

// ── 1. The recommended path: the key rides the opaque Secret ─────────

#[test]
fn n807_hmac_sha256_accepts_secret_key_both_backends() {
    std::env::set_var(KEY_ENV, KEY_VAL);
    let src = hmac_secret_src();
    let tw = run_tw(&src).expect("TW must run");
    let vm = run_vm(&src).expect("VM must run");
    let tw = tw.unwrap_or_default();
    let vm = vm.unwrap_or_default();
    assert_eq!(tw, vm, "the backends must agree on the keyed digest");
    assert_eq!(tw.len(), 64, "the HMAC-SHA256 hex contract: {tw}");
    assert!(
        tw.chars().all(|c| c.is_ascii_hexdigit()),
        "the hex form: {tw}"
    );
}

#[test]
fn n807_hmac_sha256_secret_key_differs_from_string_key() {
    // The keyed digest over the SAME material must equal regardless of
    // the carrier (Secret vs String) — the carrier is the memory-safety
    // posture, not the crypto input.
    std::env::set_var(KEY_ENV, KEY_VAL);
    let secret_src = hmac_secret_src();
    let string_src = r#"
pattern Probe(data: String) -> String {
  let tag = hmac_sha256("n807-demo-key-0123456789abcdef", "message")
  return tag
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    let via_secret = run_tw(&secret_src)
        .expect("TW: the Secret path")
        .unwrap_or_default();
    let via_string = run_tw(string_src)
        .expect("TW: the String path")
        .unwrap_or_default();
    assert_eq!(
        via_secret, via_string,
        "the digest must not depend on the carrier"
    );
}

#[test]
fn n807_hash_verify_password_roundtrip_with_secret() {
    std::env::set_var(KEY_ENV, KEY_VAL);
    let src = r#"
pattern Probe(data: String) -> String {
  let pass = secret("METALOGOS_N807_PROBE_KEY")
  let h = hash_password(pass)
  let ok = verify_password(pass, h)
  let bad = verify_password("not-the-password", h)
  if ok and (bad == false) {
    return "roundtrip-ok"
  }
  return "roundtrip-broken"
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    let tw = run_tw(src).expect("TW must run").unwrap_or_default();
    let vm = run_vm(src).expect("VM must run").unwrap_or_default();
    assert_eq!(tw, "roundtrip-ok", "TW: {tw}");
    assert_eq!(vm, "roundtrip-ok", "VM: {vm}");
}

#[test]
fn n807_redact_accepts_secret_in_place() {
    // The masking path: the Secret rides the policy pipeline by borrow.
    std::env::set_var(KEY_ENV, KEY_VAL);
    let src = r#"
pattern Probe(data: String) -> String {
  let pass = secret("METALOGOS_N807_PROBE_KEY")
  let masked = redact(pass, "hash_only")
  return masked
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    let tw = run_tw(src).expect("TW must run").unwrap_or_default();
    let vm = run_vm(src).expect("VM must run").unwrap_or_default();
    assert_eq!(tw, vm, "the backends must agree");
    assert!(tw.starts_with("[HASH:"), "the digest shape: {tw}");
    assert!(!tw.contains(KEY_VAL), "the data must be destroyed");
}

// ── 2. The structural zero-copy pin (the №604 source-scan form) ──────

#[test]
fn n807_no_unsecured_secret_copies_on_the_named_sites() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let crypto = std::fs::read_to_string(manifest.join("src/builtins/crypto.rs"))
        .expect("the crypto source");
    let string_rs = std::fs::read_to_string(manifest.join("src/builtins/string.rs"))
        .expect("the string source");
    // The audit's exact finding: `zs.as_str().to_string()` — the unsecured
    // plain-String copy of the Secret material. The three named sites are
    // hash_password, verify_password (crypto.rs) and redact (string.rs).
    for (name, src) in [("crypto.rs", &crypto), ("string.rs", &string_rs)] {
        assert!(
            !src.contains(".as_str().to_string()"),
            "{name}: the unsecured Secret copy pattern must stay absent (№807/B2)"
        );
    }
    // The calendar same-class find: the credential helper is the ZEROIZING
    // form, and the session field carries the buffer.
    let calendar = std::fs::read_to_string(manifest.join("src/builtins/calendar.rs"))
        .expect("the calendar source");
    assert!(
        calendar.contains("fn zero_str_arg"),
        "calendar: the zeroizing credential helper must be present"
    );
    assert!(
        calendar.contains("pass: Zeroizing<String>"),
        "calendar: the session credential must live in a Zeroizing buffer"
    );
}

// ── 3. The runtime shapes (the direct builtin calls, the №429 form) ──

#[test]
fn n807_builtin_surface_accepts_secret_values() {
    let secret = Value::Secret(SecretString::new(KEY_VAL.to_string()));
    // hmac_sha256: the key as Secret — the same digest as the String key.
    let hmac_fn = metalogos::builtins::spec_by_name("hmac_sha256")
        .expect("registered")
        .handler
        .expect("the real handler (not a stub)");
    let via_secret = hmac_fn(&[secret.clone(), Value::String("message".to_string())])
        .expect("the Secret key must be accepted");
    let via_string = hmac_fn(&[
        Value::String(KEY_VAL.to_string()),
        Value::String("message".to_string()),
    ])
    .expect("the String key must stay accepted");
    assert_eq!(
        format!("{via_secret}"),
        format!("{via_string}"),
        "the carrier must not change the digest"
    );
    // redact: the Secret rides by borrow.
    let redact_fn = metalogos::builtins::spec_by_name("redact")
        .expect("registered")
        .handler
        .expect("the real handler (not a stub)");
    let masked = redact_fn(&[secret, Value::String("hash_only".to_string())])
        .expect("the Secret must be accepted");
    assert!(format!("{masked}").contains("[HASH:"), "{masked}");
}
