#![allow(clippy::disallowed_methods)]

// ── Naryad №860 (wave 75 — W-D, P1, [core][consent]): the typed
// approval mandate in code (ADR-0205 §3, the dispatch gh#1699) ─────────
//
// The pure TypedApproval carrier lands (the №844 form, one subphase
// later): issue / verify / revoke over the grant contour, `verify`
// returning the W-C algebra verdict (ADR-0204), every deny cause the
// frozen `IRREVERSIBLE_NO_GRANT` class (№385 — no new codes).
//
// The 1:1 migration discipline: this naryad ADDS the carrier and touches
// NO existing constructor, event surface or Ledger record — the pins
// below hold the surfaces the wave must not perturb (the №854 parity
// suite keeps pinning the 14 words byte-for-byte; the ledger-golden and
// conformance suites run unchanged in CI).
//
// Negative controls (the «2 negative controls» standard): (а) a mandate
// on a foreign digest → Deny; (б) an expired/revoked mandate → Deny;
// both refusals answer BEFORE any action executes.

use metalogos::approval::{target_digest, TypedApproval, APPROVAL_TYPE};
use metalogos::refusal_algebra::{Verdict, DENY_REASONS};

const MANDATE_DENY_WORD: &str = "IRREVERSIBLE_NO_GRANT";

fn mandate_deny() -> Verdict {
    Verdict::deny(MANDATE_DENY_WORD).expect("the frozen vocabulary carries the class")
}

#[test]
fn the_public_carrier_verifies_inside_its_window() {
    let d = target_digest(&["goal", "archive-2025", "delete"]);
    let m = TypedApproval::issue("owner", &d, 1_000, 600);
    assert_eq!(m.verify(1_000, &d), Verdict::Allow);
    assert_eq!(m.verify(1_599, &d), Verdict::Allow);
}

#[test]
fn the_deny_verdict_is_the_algebra_value_of_the_frozen_class() {
    // verify returns the W-C algebra verdict — the SAME value the gate
    // refusals carry (№856 dogfood), never a parallel refusal shape.
    let d = target_digest(&["goal", "archive-2025", "delete"]);
    let m = TypedApproval::issue("owner", &d, 1_000, 600);
    assert_eq!(m.verify(1_600, &d), mandate_deny());
    assert_eq!(m.verify(1_100, "not-the-digest"), mandate_deny());
    match m.verify(1_100, "not-the-digest") {
        Verdict::Deny { code, cause } => {
            assert_eq!(code, MANDATE_DENY_WORD);
            assert_eq!(cause.word(), MANDATE_DENY_WORD);
        }
        Verdict::Allow => panic!("a foreign digest cannot Allow"),
    }
}

#[test]
fn negative_control_a_foreign_digest_denies_before_execution() {
    let bound = target_digest(&["action", "kv_delete", "session-cache"]);
    let foreign = target_digest(&["action", "kv_delete", "other-table"]);
    assert_ne!(bound, foreign);
    let m = TypedApproval::issue("owner", &bound, 100, 100);
    let verdict = m.verify(150, &foreign);
    assert!(
        verdict.is_deny(),
        "the foreign-digest mandate refuses BEFORE the action"
    );
}

#[test]
fn negative_control_b_expired_and_revoked_denies_before_execution() {
    let d = target_digest(&["action", "memory_prune"]);
    let m = TypedApproval::issue("owner", &d, 100, 50);
    assert!(
        m.verify(150, &d).is_deny(),
        "at expires_at the window is closed"
    );

    let mut live = TypedApproval::issue("owner", &d, 100, 50);
    assert_eq!(live.verify(120, &d), Verdict::Allow);
    live.revoke();
    assert!(
        live.verify(120, &d).is_deny(),
        "after revoke the previously live mandate refuses"
    );
    assert!(
        live.verify(u64::MAX - 1, &d).is_deny(),
        "revocation is permanent"
    );
}

#[test]
fn the_full_consent_arc_issue_verify_revoke_holds_in_one_piece() {
    // The Rust-side precondition of the №861 e2e trace
    // (request → approve → execute → revoke/expire → deny).
    let d = target_digest(&["goal", "audit-export", "run"]);
    let mut m = TypedApproval::issue("operator-2", &d, 1_000, 1_000);
    assert_eq!(m.verify(1_500, &d), Verdict::Allow, "execute passes");
    m.revoke();
    assert_eq!(m.verify(1_500, &d), mandate_deny(), "the later deny");
    assert_eq!(m.actor, "operator-2");
    assert_eq!(m.target_digest, d);
}

#[test]
fn the_carrier_is_pure_and_serializable() {
    // The portable-kernel boundary (ADR-0204 §3): no IO in the carrier —
    // serde round-trip is the whole persistence story, and a deserialized
    // (untrusted) mandate verifies only by its own fields.
    let d = target_digest(&["goal", "x"]);
    let m = TypedApproval::issue("owner", &d, 10, 0);
    let json = serde_json::to_string(&m).expect("serialize");
    let back: TypedApproval = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, m);
    assert_eq!(
        back.verify(5_000_000, &d),
        Verdict::Allow,
        "ttl 0 = no window"
    );
    assert_eq!(APPROVAL_TYPE, "TypedApproval");
}

#[test]
fn the_existing_surfaces_the_wave_must_not_perturb_stay_pinned() {
    // The 1:1 discipline: the existing surfaces keep their exact shapes —
    // the parity suite of №854 keeps pinning the 14 words byte-for-byte,
    // the ledger-golden and conformance suites run unchanged in CI.
    assert_eq!(DENY_REASONS.len(), 14);
    assert!(DENY_REASONS.contains(&MANDATE_DENY_WORD));
    assert_eq!(metalogos::capability::CAPABILITY_TYPE, "Capability");
    assert_eq!(
        metalogos::capability::CAPABILITY_STATE_TYPE,
        "CapabilityState"
    );
}
