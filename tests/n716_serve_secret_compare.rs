// ── №716 (gh#1323, Wave 49 — the gh#1317 resolution) ──────────────────
//
// The owner's bug report gh#1317 (production source: FOSVED-office-v2
// #536): under `mlog serve` some inbound header/query values "arrive
// altered" — `ct_eq` / `==` / `hash_only` comparisons fail silently on
// correct secrets, and so does `length(...)`.
//
// The honest probe (release binary v0.32.0, sha256 dd8164b7…, 2026-10-17)
// REFUTED the inbound-path hypothesis: raw header/query bytes reach the
// route UNCHANGED (the `==` on raw values is blocked by the Category-A
// UNTRUSTED_DECISION invariant — data integrity, not alteration). The
// alteration is the program's own `redact(x, "all")`: its entropy net
// (ADR-0136) masks token-like runs — ≥24 base64-class chars containing
// a digit AND a hex letter — into `[REDACTED:entropy…last4]`. The
// gh#1317 table is explained 7/7 by that filter:
//
//   V1 "abcdefgh1369abcdefghi"                     (21) → identity (run < 24)
//   V2 "abcdefgh1369abcdefghi13ab"                 (25) → [REDACTED:entropy…13ab]
//   V3 "abcdefgh1369abcdefghi13abcde69ab"          (32) → [REDACTED:entropy…69ab]
//   V4 "abcdefgh1369abcdefghi13abcde69abcdefghijk" (41) → [REDACTED:entropy…hijk]
//   V5 "abcdefghijklmnopqrstuvwxyzabcdefghijklmno" (41) → identity (no digit)
//   V6 "abcdefghijklmnop1234567"                   (23) → identity (run < 24)
//   V7 "a1b2c3d4e5f6g7h8i9j0k1"                    (22) → identity (run < 24)
//
// The secret-comparison recipe evolution. The ORIGINAL №716 recipe was
// `redact(x, "hash_only")` on BOTH sides — the SHA-256 fingerprint
// destroys the data, keeps comparability, declassifies to public.
// №806 (the audit-23.10 B1) RETIRED the declassification: an unkeyed
// digest of a LOW-ENTROPY secret is brute-forced offline in seconds, so
// the public label on it was a leak channel dressed as hygiene. The
// recipe as of №806: `ct_eq(provided, expected)` — constant-time, no
// declassification — or the KEYED fingerprint `redact(x, "hash_keyed")`
// under the process-scoped `METALOGOS_REDACT_KEY` on BOTH sides (loud
// refusal without the key). The unkeyed `hash_only` keeps determinism
// and comparability (the digest is still `[HASH:<64hex>] hex`, the full
// 256 bits now) but the label stays with the source.
//
// This file pins the contract: the golden table 7/7, the recipe's
// comparability, the trap shape (mask vs raw literal), and the
// UNTRUSTED_DECISION refusal that now NAMES the recipe. No runtime
// semantics changed — the masking is intended, the guidance gets loud.

use metalogos::builtins::redact_string;
use std::sync::Mutex;

// The env-mutation lock: the №806 keyed tests set/remove
// METALOGOS_REDACT_KEY — the process-global env is shared by the
// parallel tests in this binary, the mutations serialize on this lock.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

fn set_key(v: &str) {
    std::env::set_var("METALOGOS_REDACT_KEY", v);
}

fn restore_key(prev: Option<String>) {
    match prev {
        Some(v) => std::env::set_var("METALOGOS_REDACT_KEY", v),
        None => std::env::remove_var("METALOGOS_REDACT_KEY"),
    }
}

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

fn trim(s: Option<String>) -> String {
    s.as_deref()
        .map(|v| v.trim_end().to_string())
        .unwrap_or_default()
}

fn tw_out(src: &str) -> String {
    trim(run_tw(src).unwrap_or_else(|e| panic!("TW run failed: {e}")))
}

fn vm_out(src: &str) -> String {
    trim(run_vm(src).unwrap_or_else(|e| panic!("VM run failed: {e}")))
}

fn compile_errs(source: &str) -> Vec<String> {
    match metalogos::compile_program(source) {
        Ok(_) => Vec::new(),
        Err(e) => e.lines().map(str::to_string).collect(),
    }
}

fn redact_ok(text: &str, mode: &str) -> String {
    redact_string(text, mode).unwrap_or_else(|e| panic!("redact({:?}) failed: {}", mode, e))
}

/// The gh#1317 table verbatim: (sent value, the exact redact("all") outcome).
const GH1317_TABLE: &[(&str, &str)] = &[
    ("abcdefgh1369abcdefghi", "abcdefgh1369abcdefghi"),
    (
        "abcdefgh1369abcdefghi13ab",
        "[REDACTED:entropy\u{2026}13ab]",
    ),
    (
        "abcdefgh1369abcdefghi13abcde69ab",
        "[REDACTED:entropy\u{2026}69ab]",
    ),
    (
        "abcdefgh1369abcdefghi13abcde69abcdefghijk",
        "[REDACTED:entropy\u{2026}hijk]",
    ),
    (
        "abcdefghijklmnopqrstuvwxyzabcdefghijklmno",
        "abcdefghijklmnopqrstuvwxyzabcdefghijklmno",
    ),
    ("abcdefghijklmnop1234567", "abcdefghijklmnop1234567"),
    ("a1b2c3d4e5f6g7h8i9j0k1", "a1b2c3d4e5f6g7h8i9j0k1"),
];

// ── The golden table: redact(x, "all") outcomes are EXACTLY as probed ──

#[test]
fn n716_gh1317_table_redact_all_golden_7_of_7() {
    for (input, expected) in GH1317_TABLE {
        let out = redact_ok(input, "all");
        assert_eq!(
            &out,
            expected,
            "gh#1317 table row drifted for len={}",
            input.len()
        );
    }
}

#[test]
fn n716_gh1317_failing_rows_are_masked_not_lost() {
    // The three FAIL rows must carry the typed entropy mask (diagnosable,
    // per the mask_typed contract) — not an empty string, not a crash.
    for (input, expected) in GH1317_TABLE {
        if input.len() != expected.len() {
            assert!(
                expected.starts_with("[REDACTED:entropy\u{2026}"),
                "the failing row must be the typed mask: {expected}"
            );
            assert_eq!(&redact_ok(input, "all"), expected);
        }
    }
}

// ── The sanctioned recipe: hash_only on BOTH sides keeps comparability ──

#[test]
fn n716_hash_only_digest_deterministic_and_comparable() {
    for (input, _) in GH1317_TABLE {
        let a = redact_ok(input, "hash_only");
        let b = redact_ok(input, "hash_only");
        assert_eq!(
            a,
            b,
            "hash_only must be deterministic for len={}",
            input.len()
        );
        assert_ne!(a, *input, "hash_only must destroy the data");
        assert!(
            a.starts_with("[HASH:") && a.ends_with("] hex"),
            "the digest shape is the [HASH:<64hex>] hex contract (the full 256 bits, №806): {a}"
        );
    }
}

#[test]
fn n716_hash_only_distinguishes_different_secrets() {
    let base = redact_ok("abcdefgh1369abcdefghi13ab", "hash_only");
    for (other, _) in GH1317_TABLE {
        if *other == "abcdefgh1369abcdefghi13ab" {
            continue;
        }
        assert_ne!(
            base,
            redact_ok(other, "hash_only"),
            "distinct secrets must not collide under hash_only"
        );
    }
}

// ── Program level (TW + VM parity): the recipe decides, the trap fails ──

fn compare_src(mode: &str, a: &str, b: &str) -> String {
    format!(
        "pattern C(_input: String) -> String {{\n  if redact(\"{a}\", \"{mode}\") == redact(\"{b}\", \"{mode}\") {{ return \"true\" }}\n  return \"false\"\n}}\nflow Main {{ input: String = \"x\" -> C -> output }}"
    )
}

fn trap_src(a: &str) -> String {
    // The production trap shape (office #536): the INBOUND side is
    // redacted, the expected literal is raw — the masked copy never
    // equals the original for token-like values.
    format!(
        "pattern C(_input: String) -> String {{\n  if redact(\"{a}\", \"all\") == \"{a}\" {{ return \"true\" }}\n  return \"false\"\n}}\nflow Main {{ input: String = \"x\" -> C -> output }}"
    )
}

#[test]
fn n716_recipe_hash_only_equal_secrets_true_both_backends() {
    let v = "abcdefgh1369abcdefghi13ab";
    let src = compare_src("hash_only", v, v);
    assert_eq!(tw_out(&src), "true", "TW: hash_only recipe matches itself");
    assert_eq!(vm_out(&src), "true", "VM: hash_only recipe matches itself");
}

#[test]
fn n716_recipe_hash_only_different_secrets_false_both_backends() {
    let src = compare_src(
        "hash_only",
        "abcdefgh1369abcdefghi13ab",
        "abcdefgh1369abcdefghi13abcde69ab",
    );
    assert_eq!(tw_out(&src), "false", "TW: distinct secrets stay distinct");
    assert_eq!(vm_out(&src), "false", "VM: distinct secrets stay distinct");
}

#[test]
fn n716_trap_mask_vs_raw_literal_false_both_backends() {
    // The gh#1317 FAIL row: redact("all") vs the raw literal — the silent
    // failure the production auth hit. Pinned as the documented trap.
    let src = trap_src("abcdefgh1369abcdefghi13ab");
    assert_eq!(
        tw_out(&src),
        "false",
        "TW: the mask never equals the raw token"
    );
    assert_eq!(
        vm_out(&src),
        "false",
        "VM: the mask never equals the raw token"
    );
}

#[test]
fn n716_identity_rows_still_compare_true_under_all() {
    // The PASS rows of the table (V1/V5/V6/V7): redact("all") is the
    // identity there, so the trap comparison still answers true. This is
    // WHY the bug looked value-dependent and content-dependent at once.
    for (input, expected) in GH1317_TABLE {
        if input == expected {
            let src = trap_src(input);
            assert_eq!(
                tw_out(&src),
                "true",
                "TW: identity row must survive: {input}"
            );
            assert_eq!(
                vm_out(&src),
                "true",
                "VM: identity row must survive: {input}"
            );
        }
    }
}

// ── The refusal now names the recipe (the loud part) ──────────────────

#[test]
fn n716_untrusted_decision_message_names_hash_only_recipe() {
    let src = r#"
        pattern Decide() -> String {
            let token = request_header("X-Secret")
            if token == "whsec-expected" {
                return "ok"
            }
            return "deny"
        }
    "#;
    let errs = compile_errs(src);
    assert!(
        errs.iter().any(|m| m.contains("[UNTRUSTED_DECISION]")
            && m.contains("source 'request_header'")
            && m.contains("ct_eq")
            && m.contains("hash_keyed")),
        "the refusal must name the №806 recipe (ct_eq / the keyed fingerprint): {errs:?}"
    );
}

// ── №806 (B1): the keyed fingerprint — the sanctioned public recipe ───

#[test]
fn n806_hash_keyed_deterministic_comparable_keyed_shape() {
    let _g = lock_env();
    let prev = std::env::var("METALOGOS_REDACT_KEY").ok();
    set_key("n716-n806-process-key-0123456789abcdef");
    let a = redact_ok("abcdefgh1369abcdefghi13ab", "hash_keyed");
    let b = redact_ok("abcdefgh1369abcdefghi13ab", "hash_keyed");
    match prev {
        Some(v) => std::env::set_var("METALOGOS_REDACT_KEY", v),
        None => std::env::remove_var("METALOGOS_REDACT_KEY"),
    }
    assert_eq!(
        a, b,
        "the keyed fingerprint must be deterministic under the same key"
    );
    assert_ne!(a, "abcdefgh1369abcdefghi13ab", "the data must be destroyed");
    assert!(
        a.starts_with("[HMAC:") && a.ends_with("] hex"),
        "the keyed digest shape is the [HMAC:<64hex>] hex contract: {a}"
    );
    let hex = a
        .trim_start_matches("[HMAC:")
        .trim_end_matches("] hex")
        .to_string();
    assert_eq!(hex.len(), 64, "the full HMAC-SHA256 hex, got: {a}");
}

#[test]
fn n806_hash_keyed_recipe_decides_on_both_backends() {
    // The №806 recipe at the program level: the keyed fingerprint on BOTH
    // sides decides (the static label is bottom — the №327 gate passes).
    let _g = lock_env();
    let prev = std::env::var("METALOGOS_REDACT_KEY").ok();
    set_key("n716-n806-process-key-0123456789abcdef");
    let v = "abcdefgh1369abcdefghi13ab";
    let src = compare_src("hash_keyed", v, v);
    let tw = tw_out(&src);
    let vm = vm_out(&src);
    let ne = compare_src("hash_keyed", v, "abcdefgh1369abcdefghi13abcde69ab");
    let tw_ne = tw_out(&ne);
    let vm_ne = vm_out(&ne);
    match prev {
        Some(v) => std::env::set_var("METALOGOS_REDACT_KEY", v),
        None => std::env::remove_var("METALOGOS_REDACT_KEY"),
    }
    assert_eq!(tw, "true", "TW: the keyed recipe matches itself");
    assert_eq!(vm, "true", "VM: the keyed recipe matches itself");
    assert_eq!(tw_ne, "false", "TW: distinct secrets stay distinct (keyed)");
    assert_eq!(vm_ne, "false", "VM: distinct secrets stay distinct (keyed)");
}

#[test]
fn n806_hash_keyed_refuses_loud_without_the_key_both_backends() {
    // Fail-closed: the missing key is a LOUD runtime error, never a
    // silent unkeyed fallback — pinned on both backends.
    let _g = lock_env();
    let prev = std::env::var("METALOGOS_REDACT_KEY").ok();
    std::env::remove_var("METALOGOS_REDACT_KEY");
    let src = compare_src("hash_keyed", "a", "a");
    let tw = run_tw(&src).err();
    let vm = run_vm(&src).err();
    match prev {
        Some(v) => std::env::set_var("METALOGOS_REDACT_KEY", v),
        None => std::env::remove_var("METALOGOS_REDACT_KEY"),
    }
    let tw = tw.expect("TW must refuse without the key");
    let vm = vm.expect("VM must refuse without the key");
    assert!(
        tw.contains("METALOGOS_REDACT_KEY") && tw.contains("fail-closed"),
        "TW: {tw}"
    );
    assert!(
        vm.contains("METALOGOS_REDACT_KEY") && vm.contains("fail-closed"),
        "VM: {vm}"
    );
}

#[test]
fn n806_hash_only_still_comparable_but_not_public() {
    // The unkeyed digest keeps its contract half: deterministic
    // comparability. The declassification half is retired — the static
    // gate refuses the digest at a public sink (the n326 pin covers the
    // compile side; here the full digest shape is the honest marker).
    let a = redact_ok("abcdefgh1369abcdefghi13ab", "hash_only");
    let b = redact_ok("abcdefgh1369abcdefghi13ab", "hash_only");
    assert_eq!(a, b, "comparability is kept");
    let hex = a
        .trim_start_matches("[HASH:")
        .trim_end_matches("] hex")
        .to_string();
    assert_eq!(hex.len(), 64, "the full 256-bit digest, got: {a}");
}
