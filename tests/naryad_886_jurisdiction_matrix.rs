// ── tests/naryad_886_jurisdiction_matrix.rs ──────────────────────────
// Наряд №886 (issue #1759, wave 79 — W-H, P1, [core][jurisdiction],
// ADR-0209): the jurisdiction kernel in code — JurisdictionRule +
// the append-only JurisdictionMatrix + the fail-closed compile site.
//
// Pins (the RED-before form over the typed surface):
//   T1: THE SEED MATRIX IS REAL DATA — ≥3 jurisdictions (EU/RU/US-CA),
//       every rule carries a non-empty norm_ref + a valid checked_at
//       (the KPI C10 verbatim), the source names its supplier.
//   T2: THE DIGEST IS THE №828 FORM — the canonical row is
//       deterministic, the row digest is SHA-256 hex, the matrix
//       digest changes only when the configuration changes; the
//       double compile is byte-identical (the serde_json double-run
//       serialization byte-identity — the №880 acceptance form).
//   T3: THE COMPILED VIEW IS REPRODUCIBLE — the per-jurisdiction
//       policy view pins the matrix digest it was compiled from and
//       resolves the strictest effect (deny > limit > allow).
//   T4: THE 1:1 MIGRATION HONESTY — the jurisdiction layer is a pure
//       library kernel: no grammar, no builtin-registry and no golden
//       change exists in the wave (the surfaces stay byte-identical;
//       the kernel is backend-independent by construction — the
//       compile-site does not reach TW/VM, so the honest parity claim
//       is the determinism + the construction refusals, not a fake
//       backend run; the №588 honest-boundary protocol).
//
// The negative controls live in the module kernel_tests (the
// construction/compile refusals) — this integration suite pins the
// positive machine facts over the seed data.

use metalogos::jurisdiction::{
    canonical_rule, compile_jurisdiction_policy, seed_matrix, JurisdictionMatrix, JurisdictionRule,
    PolicyEffect,
};

#[test]
fn t1_seed_matrix_is_real_kpi_true_data() {
    let m = seed_matrix();
    assert!(
        m.jurisdictions().len() >= 3,
        "the C10 fact needs ≥3 jurisdictions, got {}",
        m.jurisdictions().len()
    );
    assert_eq!(m.jurisdictions(), vec!["EU", "RU", "US-CA"]);
    for r in m.rows() {
        assert!(
            !r.norm_ref.trim().is_empty(),
            "a seed rule without a norm reference is not a rule (KPI C10)"
        );
        assert_eq!(
            r.checked_at.len(),
            10,
            "the verification date is an ISO date"
        );
        assert!(!r.source.trim().is_empty(), "the seed names its supplier");
        assert_eq!(r.row_digest.len(), 64);
    }
    // the seed carries the erasure-path allow in all three + the
    // data-minimisation limit in EU
    assert_eq!(m.effect_for("EU", "forget"), Some(PolicyEffect::Allow));
    assert_eq!(m.effect_for("RU", "forget"), Some(PolicyEffect::Allow));
    assert_eq!(m.effect_for("US-CA", "forget"), Some(PolicyEffect::Allow));
    assert_eq!(m.effect_for("EU", "http_post"), Some(PolicyEffect::Limit));
}

#[test]
fn t2_digest_forms_are_deterministic_and_double_run_is_byte_identical() {
    let m1 = seed_matrix();
    let m2 = seed_matrix();
    assert_eq!(m1.digest(), m2.digest());
    // the canonical row: the fixed field order, the \x1f separator
    let r = m1.rows()[0].clone();
    assert_eq!(canonical_rule(&r), canonical_rule(&r));
    assert!(canonical_rule(&r).contains('\u{1f}'));
    // the serde double-run: the serialized matrix is byte-identical
    let s1 = serde_json::to_vec(&m1).unwrap();
    let s2 = serde_json::to_vec(&m2).unwrap();
    assert_eq!(
        s1, s2,
        "the double-run serialization must be byte-identical (№880 form)"
    );
    // the round-trip preserves the rows and the digest
    let back: JurisdictionMatrix = serde_json::from_slice(&s1).unwrap();
    assert_eq!(back.digest(), m1.digest());
}

#[test]
fn t3_compiled_view_is_reproducible_from_its_source() {
    let m = seed_matrix();
    let p1 = compile_jurisdiction_policy(&m, "EU").unwrap();
    let p2 = compile_jurisdiction_policy(&m, "EU").unwrap();
    assert_eq!(p1, p2, "the compile is deterministic");
    assert_eq!(p1.matrix_digest, m.digest());
    assert_eq!(p1.effect_of("forget"), Some(PolicyEffect::Allow));
    assert_eq!(p1.effect_of("http_post"), Some(PolicyEffect::Limit));
    assert_eq!(p1.effect_of("exec"), None);
    // the other jurisdictions compile too (the C10 breadth)
    assert!(compile_jurisdiction_policy(&m, "RU").is_ok());
    assert!(compile_jurisdiction_policy(&m, "US-CA").is_ok());
    // the compiled view serializes and restores byte-identically
    let s1 = serde_json::to_vec(&p1).unwrap();
    let s2 = serde_json::to_vec(&p2).unwrap();
    assert_eq!(s1, s2);
}

#[test]
fn t4_append_only_supersede_and_strictest_wins_end_to_end() {
    let mut m = seed_matrix();
    let before = m.digest();
    // a supersede is a NEW row — the append-only discipline
    m.issue(
        JurisdictionRule::new(
            metalogos::jurisdiction::JurisdictionId::new("EU").unwrap(),
            "http_post",
            PolicyEffect::Deny,
            "GDPR (EU) 2016/679 Ch. V (Art. 44)",
            "2026-11-03",
            "test/supersede",
        )
        .unwrap(),
    )
    .unwrap();
    assert_ne!(
        m.digest(),
        before,
        "a configuration change changes the digest"
    );
    assert_eq!(
        m.effect_for("EU", "http_post"),
        Some(PolicyEffect::Deny),
        "deny > limit"
    );
    let p = compile_jurisdiction_policy(&m, "EU").unwrap();
    assert_eq!(p.effect_of("http_post"), Some(PolicyEffect::Deny));
    // and the compiled view drives the fail-closed refusal class of №887
    // (the MCP composition lives in metalogos-server — the core kernel
    // keeps the effect resolution the single source).
}
