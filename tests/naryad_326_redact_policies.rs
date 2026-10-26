// Naryad #475 (issue #723): the fs_gate ratchet (clippy disallowed-methods)
// targets PRODUCTION I/O paths. This test file exercises the REAL filesystem
// for fixtures and assertions by design — the allow is scoped to this file.
#![allow(clippy::disallowed_methods)]

//! Наряд №326 (issue #420) — redact/declassify: the ONLY sanctioned
//! downward move on the conf axis (ADR-0154 §10), policy as a value.
//!
//! Covers:
//! - (а) policy-as-value: built-in registry (pii_strip, hash_only,
//!   truncate + the legacy ADR-0136 modes), target conf per policy,
//!   loud unknown policy words;
//! - (б) the downward path: with a one-way policy the label goes public
//!   and the sink gate passes; without redact — the №325 gate blocks;
//! - (в) conservatism: pii_strip/truncate keep the source label (they
//!   can miss data) — the gate keeps blocking;
//! - (г) every application is an UNCONDITIONAL audit event
//!   (REDACT_APPLIED, Severity::Info + stderr line) — what, which
//!   policy, which target;
//! - (д) runtime behavior of the new policies (hash determinism,
//!   truncation shape);
//! - (е) no-stub hygiene.
//!
//! Boundary (loud, per the naryad): consent revocation and the poisoned
//! cascade are Phase 2 (№335); consent_ledger integration Phase 2;
//! custom (user-defined) policies — the registry is open, values later.

use metalogos::builtins::string::{redact_policy, REDACT_POLICIES};
use metalogos::parser::parse;
use metalogos::semantic::check_program;
use std::sync::Mutex;

// The env-mutation lock for the №806 keyed tests (the process-global env
// is shared by the parallel tests in this binary — the mutations
// serialize on this lock, the n716 form).
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

fn compile_ok(source: &str) -> bool {
    metalogos::compile_program(source).is_ok()
}

fn compile_errs(source: &str) -> Vec<String> {
    match metalogos::compile_program(source) {
        Ok(_) => Vec::new(),
        Err(e) => e.lines().map(str::to_string).collect(),
    }
}

// ── (а) Policy as a value: the registry ──────────────────────────────

#[test]
fn n326_policy_registry_has_builtins_with_targets() {
    // The built-in policies: legacy ADR-0136 modes + the №326 additions.
    let names: Vec<&str> = REDACT_POLICIES.iter().map(|p| p.name).collect();
    for expected in [
        "secrets",
        "pii",
        "all",
        "pii_strip",
        "hash_only",
        "hash_keyed",
        "truncate",
    ] {
        assert!(
            names.contains(&expected),
            "{expected} must be in the registry"
        );
    }
    // One-way policies declare public; conservative ones keep private.
    // №806 (B1): hash_only does NOT declassify anymore (the unkeyed
    // fingerprint of a low-entropy secret is brute-forceable offline) —
    // the public fingerprint is the KEYED policy's job.
    assert_eq!(redact_policy("hash_only").unwrap().target_conf, "private");
    assert_eq!(redact_policy("hash_keyed").unwrap().target_conf, "public");
    assert_eq!(redact_policy("secrets").unwrap().target_conf, "public");
    assert_eq!(redact_policy("all").unwrap().target_conf, "public");
    assert_eq!(redact_policy("pii_strip").unwrap().target_conf, "private");
    assert_eq!(redact_policy("truncate").unwrap().target_conf, "private");
    assert_eq!(redact_policy("pii").unwrap().target_conf, "private");
    // Lookup of a non-policy is None.
    assert!(redact_policy("wide_open").is_none());
}

#[test]
fn n326_unknown_policy_word_is_loud_at_runtime() {
    let out = metalogos::builtins::string::redact_string("x", "wide_open");
    assert!(out.is_err(), "unknown policy must be a loud runtime error");
    let msg = out.err().unwrap();
    assert!(
        msg.contains("unknown policy"),
        "message names the problem: {msg}"
    );
    // And the error enumerates the available policies (discoverability).
    assert!(msg.contains("hash_only"));
}

// ── (б) The downward path: redact → public → sinks pass ──────────────

#[test]
fn n806_hash_keyed_path_down_compiles() {
    let src = r#"
        pattern Send(data: String) -> String {
            let _ = print(redact(env("APP_KEY"), "hash_keyed"))
            return "sent"
        }
    "#;
    assert!(
        compile_ok(src),
        "the keyed fingerprint is the sanctioned downward move — the gate passes (the runtime key is a runtime concern; the static label is registry-driven)"
    );
}

#[test]
fn n806_hash_only_no_longer_declassifies() {
    // №806 (B1): the unkeyed fingerprint of a low-entropy secret is
    // brute-forceable offline — responding/egressing it is a leak now.
    let src = r#"
        pattern Send(data: String) -> String {
            let _ = print(redact(env("APP_KEY"), "hash_only"))
            return "sent"
        }
    "#;
    let errs = compile_errs(src);
    assert!(
        errs.iter().any(|m| m.contains("sink clearance violated")),
        "the unkeyed digest keeps the secret's label — the gate keeps blocking: {errs:?}"
    );
}

#[test]
fn n325_gate_blocks_the_same_flow_without_redact() {
    let src = r#"
        pattern Send(data: String) -> String {
            let _ = print(env("APP_KEY"))
            return "sent"
        }
    "#;
    let errs = compile_errs(src);
    assert!(
        !errs.is_empty(),
        "without redact the №325 gate must block the private → public sink"
    );
}

#[test]
fn n326_secrets_and_all_still_lift_the_label() {
    // The legacy ADR-0136 modes remain members of the same registry.
    for policy in ["secrets", "all"] {
        let src = format!(
            r#"
            pattern Send(data: String) -> String {{
                let _ = print(redact(env("APP_KEY"), "{policy}"))
                return "sent"
            }}
            "#
        );
        assert!(
            compile_ok(&src),
            "policy {policy} must keep its sanctioned downward move"
        );
    }
}

// ── (в) Conservatism: pattern strips keep the label ──────────────────

#[test]
fn n326_pii_strip_keeps_the_label_conservative() {
    let src = r#"
        entity record: String = "Иванов Иван, снилс 123-456-789, диагноз конфиденциален"
        pattern Send(data: String) -> String {
            let stripped = redact(record, "pii_strip")
            let _ = print(stripped)
            return "sent"
        }
    "#;
    let errs = compile_errs(src);
    assert!(
        errs.iter().any(|m| m.contains("[PII_EGRESS_OUTPUT]")),
        "pattern strips are conservative — the gate keeps blocking: {errs:?}"
    );
}

#[test]
fn n326_truncate_keeps_the_label_conservative() {
    let src = r#"
        pattern Send(data: String) -> String {
            let short = redact(env("APP_KEY"), "truncate")
            let _ = print(short)
            return "sent"
        }
    "#;
    let errs = compile_errs(src);
    assert!(
        errs.iter().any(|m| m.contains("sink clearance violated")),
        "truncation does not declassify — the gate keeps blocking: {errs:?}"
    );
}

// ── (г) Unconditional audit events ───────────────────────────────────

#[test]
fn n326_every_redact_application_is_an_audit_event() {
    let src = r#"
        pattern Send(data: String) -> String {
            let h = redact(env("APP_KEY"), "hash_keyed")
            let s = redact(data, "pii_strip")
            let _ = print(h)
            return "sent"
        }
    "#;
    let report = metalogos::audit_program(src).expect("audit must parse");
    let events: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.check_id == "REDACT_APPLIED")
        .collect();
    assert_eq!(events.len(), 2, "each application is one event: {events:?}");
    for e in &events {
        assert_eq!(e.severity, metalogos::audit::Severity::Info);
    }
    // The events name the policy and the target.
    assert!(events
        .iter()
        .any(|e| e.message.contains("policy 'hash_keyed'")
            && e.message.contains("target conf 'public'")));
    assert!(events
        .iter()
        .any(|e| e.message.contains("policy 'pii_strip'")
            && e.message.contains("target conf 'private'")));
    // №806 (B1): the unkeyed policy's event names the new private target.
    let src806 = r#"
        pattern Bind(data: String) -> String {
            let h = redact(data, "hash_only")
            return h
        }
    "#;
    let report = metalogos::audit_program(src806).expect("audit");
    let events: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.check_id == "REDACT_APPLIED")
        .collect();
    assert_eq!(events.len(), 1);
    assert!(
        events[0].message.contains("policy 'hash_only'")
            && events[0].message.contains("target conf 'private'")
    );
    // Unconditional: the events are present even under the legacy
    // profile (no toggles — ADR-0154 §10).
    let src_legacy = format!("profile legacy {{ egress: permissive_with_audit }}\n{src}");
    let report = metalogos::audit_program(&src_legacy).expect("audit");
    let events = report
        .findings
        .iter()
        .filter(|f| f.check_id == "REDACT_APPLIED")
        .count();
    assert_eq!(events, 2, "events are not switchable, even under legacy");
}

#[test]
fn n326_dynamic_policy_is_recorded_as_unknown() {
    // A dynamic (non-literal) policy cannot be resolved statically —
    // the event still fires, with the conservative target.
    let src = r#"
        pattern Send(data: String, mode: String) -> String {
            let h = redact(data, mode)
            return h
        }
    "#;
    let report = metalogos::audit_program(src).expect("audit");
    let events: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.check_id == "REDACT_APPLIED")
        .collect();
    assert_eq!(events.len(), 1);
    assert!(events[0].message.contains("policy '<dynamic>'"));
}

// ── (д) Runtime behavior of the new policies ─────────────────────────

#[test]
fn n806_hash_only_full_digest_deterministic_and_data_destroying() {
    let a = metalogos::builtins::string::redact_string("секрет-данные-123", "hash_only").unwrap();
    let b = metalogos::builtins::string::redact_string("секрет-данные-123", "hash_only").unwrap();
    assert_eq!(a, b, "hash_only must be deterministic");
    assert!(!a.contains("секрет"), "the data must be destroyed");
    assert!(a.contains("[HASH:"), "shape: {a}");
    // №806 (B1): the FULL 256-bit digest — 64 hex chars, not a 64-bit prefix.
    let hex = a
        .trim_start_matches("[HASH:")
        .trim_end_matches("] hex")
        .to_string();
    assert_eq!(hex.len(), 64, "the full SHA-256 hex, got: {a}");
    let c = metalogos::builtins::string::redact_string("другие-данные", "hash_only").unwrap();
    assert_ne!(a, c, "different inputs → different hashes");
}

#[test]
fn n806_hash_keyed_is_deterministic_and_differs_from_unkeyed() {
    let _g = lock_env();
    // The keyed fingerprint: deterministic under the SAME process key,
    // distinct from the unkeyed digest, [HMAC: shape, full 64 hex.
    // The env set/remove guard is this file's own — the file-scoped allow.
    let prev = std::env::var("METALOGOS_REDACT_KEY").ok();
    std::env::set_var("METALOGOS_REDACT_KEY", "n806-process-key-0123456789abcdef");
    let a = metalogos::builtins::string::redact_string("секрет-данные-123", "hash_keyed").unwrap();
    let b = metalogos::builtins::string::redact_string("секрет-данные-123", "hash_keyed").unwrap();
    match prev {
        Some(v) => std::env::set_var("METALOGOS_REDACT_KEY", v),
        None => std::env::remove_var("METALOGOS_REDACT_KEY"),
    }
    assert_eq!(a, b, "hash_keyed must be deterministic under the same key");
    assert!(a.contains("[HMAC:"), "shape: {a}");
    let hex = a
        .trim_start_matches("[HMAC:")
        .trim_end_matches("] hex")
        .to_string();
    assert_eq!(hex.len(), 64, "the full HMAC-SHA256 hex, got: {a}");
    let unkeyed =
        metalogos::builtins::string::redact_string("секрет-данные-123", "hash_only").unwrap();
    assert_ne!(
        a, unkeyed,
        "the keyed digest must differ from the unkeyed one"
    );
}

#[test]
fn n806_hash_keyed_refuses_loud_without_the_key() {
    let _g = lock_env();
    // Fail-closed: no METALOGOS_REDACT_KEY → a loud refusal, never a
    // silent unkeyed fallback (the №481 loud-env posture).
    let prev = std::env::var("METALOGOS_REDACT_KEY").ok();
    std::env::remove_var("METALOGOS_REDACT_KEY");
    let out = metalogos::builtins::string::redact_string("x", "hash_keyed");
    match prev {
        Some(v) => std::env::set_var("METALOGOS_REDACT_KEY", v),
        None => std::env::remove_var("METALOGOS_REDACT_KEY"),
    }
    let msg = out.err().expect("the missing key must refuse");
    assert!(msg.contains("METALOGOS_REDACT_KEY"), "names the env: {msg}");
    assert!(msg.contains("fail-closed"), "the posture is loud: {msg}");
    // And an EMPTY key refuses the same way.
    let prev = std::env::var("METALOGOS_REDACT_KEY").ok();
    std::env::set_var("METALOGOS_REDACT_KEY", "   ");
    let out = metalogos::builtins::string::redact_string("x", "hash_keyed");
    match prev {
        Some(v) => std::env::set_var("METALOGOS_REDACT_KEY", v),
        None => std::env::remove_var("METALOGOS_REDACT_KEY"),
    }
    let msg = out.err().expect("the empty key must refuse");
    assert!(msg.contains("EMPTY"), "the empty-key refusal: {msg}");
}

#[test]
fn n326_truncate_shape() {
    let out =
        metalogos::builtins::string::redact_string("паспорт 4510 123456", "truncate").unwrap();
    assert!(out.starts_with("пас"), "keeps the first chars: {out}");
    assert!(out.contains("[REDACTED:truncated"), "masks the rest: {out}");
    // PII markers do not survive the truncation head (conservative but
    // the LABEL stays private — see the gate test above).
    assert!(!out.contains("4510"));
}

// ── (е) The showcase example compiles and carries the events ─────────

#[test]
fn n326_example_l1_redact_compiles_with_events() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src = std::fs::read_to_string(format!("{manifest_dir}/examples/l1_redact.mlog"))
        .expect("example");
    let result = metalogos::compile_program(&src);
    assert!(
        result.is_ok(),
        "the showcase must compile: {:?}",
        result.err()
    );
    let report = metalogos::audit_program(&src).expect("audit");
    let events = report
        .findings
        .iter()
        .filter(|f| f.check_id == "REDACT_APPLIED")
        .count();
    assert!(events >= 3, "every application is an event: {events}");
}

#[test]
fn n326_semantic_stays_clean_for_redacted_flows() {
    // The semantic pass emits no diagnostics for redacted flows.
    // №806 (B1): the keyed fingerprint keeps the sanctioned downward move;
    // the unkeyed digest keeps the label — branching on its equality is
    // still a TRUSTED decision (the integrity restore survives, the conf
    // pass-through is the B1 change).
    let decls = parse(
        r#"
        pattern Send(data: String) -> String {
            let _ = print(redact(env("APP_KEY"), "hash_keyed"))
            return "sent"
        }
        "#,
    )
    .unwrap();
    let result = check_program(&decls);
    assert!(result.is_ok(), "{:?}", result.format());
}

// ── (ж) No-stub hygiene (№16.0-D) ────────────────────────────────────

#[test]
fn n326_no_stub_markers_in_touched_files() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    for file in ["src/builtins/string.rs", "src/semantic.rs", "src/audit.rs"] {
        let src = std::fs::read_to_string(format!("{manifest_dir}/{file}"))
            .unwrap_or_else(|_| panic!("{file} must exist"));
        for marker in ["todo!", "unimplemented!", "SKELETON"] {
            assert!(
                !src.contains(marker),
                "stub marker `{marker}` found in {file}"
            );
        }
    }
}
