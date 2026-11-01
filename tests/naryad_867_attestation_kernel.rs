#![allow(clippy::disallowed_methods)]

// ── Naryad №867 (wave 76 — W-E, P1, [core][compliance]): the
// attestation kernel integration suite (ADR-0206 §2–§6) ────────────────
//
// The negative controls (the №860 form — the refusals are part of the
// contract), the parents-referenced-never-copied semantics (ADR-0206
// §2.3), the C4 dedup guard (the R54 boundary — `src/nn/attest.rs`
// stays the Reflex distillation domain, untouched), and the frozen
// surface guards (the 14-word vocabulary, №385; the kernel adds no
// deny paths).
//
// The 1:1 migration (ADR-0206 §6) is pinned by the EXISTING suites
// (the ledger-golden and conformance tests run in this CI green with
// zero golden edits — this PR touches no existing constructor, no
// Ledger record, no DenyEvent surface; the only lib.rs line is the new
// module declaration).

use metalogos::compliance::{
    canonical_row, canonical_row_digest, subject_digest, Attestation, AttestationMap, Evidence,
    EvidenceKind, ATTESTATION_TYPE,
};
use metalogos::refusal_algebra::Verdict;

const GOOD: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const FOREIGN: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

// ── the C4 dedup guard — the name boundary is mechanical ───────────────

#[test]
fn the_c4_attest_domain_stays_untouched_and_the_names_stay_apart() {
    // (1) the C4 surface (№825, frozen by №832) keeps its domain doc —
    // the Reflex distillation certificates, never the compliance map.
    let nn_attest = std::fs::read_to_string("src/nn/attest.rs")
        .expect("src/nn/attest.rs must exist — the C4 surface is frozen");
    assert!(
        nn_attest.contains("Reflex attestation"),
        "the C4 domain header must keep the Reflex attestation identity"
    );
    assert!(
        nn_attest.contains("№825")
            || nn_attest.contains("Naryad №825")
            || nn_attest.contains("Наряд №825"),
        "the C4 domain keeps its naryad provenance (the №832 freeze)"
    );
    // (2) the compliance kernel carries the boundary note — the two
    // domains share a root word, not a contract.
    let compliance = std::fs::read_to_string("src/compliance.rs")
        .expect("src/compliance.rs must exist — the №867 kernel");
    assert!(
        compliance.contains("DEDUP GUARD") && compliance.contains("nn/attest.rs"),
        "the compliance kernel must carry the C4 name-boundary note verbatim"
    );
    // (3) the type names do not collide at the crate root: the C4
    // certificate types live under `nn::attest`, the compliance types
    // under `compliance` — the reserved type name is the boundary.
    assert_eq!(ATTESTATION_TYPE, "Attestation");
}

// ── the frozen surface guards — the kernel adds no vocabulary ──────────

#[test]
fn the_deny_vocabulary_stays_frozen_at_14_words() {
    // №385 / ADR-0169: the map aggregates traces, it never mints
    // refusal words — the kernel's deny verdict is the EXISTING
    // IRREVERSIBLE_NO_GRANT class.
    assert_eq!(metalogos::refusal_algebra::DENY_REASONS.len(), 14);
    let mut att = Attestation::issue(
        "actor",
        "the golden ledger trace",
        &["ledger", "w72_capability_trace"],
        "ledger.golden.pinned",
        evidence(EvidenceKind::Ledger, "w72:seq:1", GOOD),
        "cargo test --test ledger_golden",
        100,
        0,
        &|_| Ok(()),
    )
    .unwrap();
    att.revoke();
    assert_eq!(att.verify(200).code(), Some("IRREVERSIBLE_NO_GRANT"));
}

fn evidence(kind: EvidenceKind, reference: &str, digest: &str) -> Evidence {
    Evidence {
        kind,
        reference: reference.to_string(),
        digest: digest.to_string(),
    }
}

// ── the parents are REFERENCED, never copied (ADR-0206 §2.3) ──────────

#[test]
fn the_attestation_references_a_live_mandate_and_never_copies_it() {
    // (1) the LIVE parent: a typed mandate over the cron carrier (the
    // ADR-0205 surface, untouched — the attestation consumes it).
    let mandate = metalogos::approval::TypedApproval::issue(
        "owner",
        &metalogos::approval::target_digest(&["carrier:cron_run"]),
        100,
        600,
    );
    // the LIVE digest of the parent's own bytes — the resolver computes
    // it from the parent, the attestation only pins it
    let mandate_digest = metalogos::approval::target_digest(&[
        &mandate.actor,
        &mandate.target_digest,
        &mandate.granted_at.to_string(),
        &mandate.expires_at.to_string(),
        if mandate.revoked { "revoked" } else { "live" },
    ]);
    let att = Attestation::issue(
        "office-machine",
        "the cron_run mandate",
        &["mandate", "carrier:cron_run"],
        "mandate.live-verify",
        evidence(EvidenceKind::Mandate, "carrier:cron_run", &mandate_digest),
        "the live verify of the TypedApproval over carrier:cron_run",
        150,
        0,
        &|ev: &Evidence| {
            // the production resolver shape (№868): recompute the live
            // digest of the parent, refuse the drift
            if ev.digest == mandate_digest {
                Ok(())
            } else {
                Err("the mandate drift".into())
            }
        },
    )
    .expect("the live parent resolves");
    assert!(att.evidence_matches(&mandate_digest));
    assert!(
        !att.evidence_matches(FOREIGN),
        "a foreign digest is a loud mismatch"
    );
    assert!(matches!(att.verify(200), Verdict::Allow));
    // (2) the payload is NOT copied: the attestation carries the kind,
    // the reference and the digest — 64 hex + the id, never the
    // mandate's fields.
    assert_eq!(att.evidence.reference, "carrier:cron_run");
    assert_eq!(att.evidence.digest, mandate_digest);
    assert!(!canonical_row(&att).contains(&mandate.target_digest));
}

// ── the kernel flow: issue → map → verify → revoke → deny ─────────────

#[test]
fn the_map_lifecycle_refuses_the_reuse_of_a_closed_attestation() {
    let mut map = AttestationMap::new();
    let att = Attestation::issue(
        "actor",
        "s",
        &["s"],
        "c",
        evidence(EvidenceKind::Capability, "builtin:http_post", GOOD),
        "v",
        100,
        0,
        &|_| Ok(()),
    )
    .unwrap();
    let dg = att.row_digest.clone();
    map.issue(att).unwrap();
    // live inside the window
    assert!(matches!(map.rows()[0].verify(150), Verdict::Allow));
    // the human/actor withdraws — the closed attestation refuses reuse
    map.revoke(&dg).unwrap();
    assert!(matches!(map.rows()[0].verify(150), Verdict::Deny { .. }));
    // the row stays (append-only) but attests nothing
    assert_eq!(map.len(), 1);
    assert_eq!(map.live_count(), 0);
    // the same row digest cannot be re-added (the exact duplicate is a
    // construction error, not a row)
    let dup = Attestation::issue(
        "actor",
        "s",
        &["s"],
        "c",
        evidence(EvidenceKind::Capability, "builtin:http_post", GOOD),
        "v",
        100,
        0,
        &|_| Ok(()),
    )
    .unwrap();
    assert!(map.issue(dup).is_err());
}

// ── the subject binding — the foreign-subject control ─────────────────

#[test]
fn the_subject_digest_binds_the_parts_and_refuses_the_foreign_subject() {
    let d1 = subject_digest(&["mandate", "carrier:cron_run"]);
    let d2 = subject_digest(&["mandate", "carrier:cron_run", "extra"]);
    let d3 = subject_digest(&["mandate", "carrier:cron_run"]);
    assert_ne!(d1, d2, "any changed part changes the digest");
    assert_eq!(d1, d3, "the same parts bind the same subject");
    assert_eq!(d1.len(), 64);
}

// ── the determinism seed — the canonical row is stable (№868 feeds) ────

#[test]
fn the_canonical_row_is_deterministic_across_the_equal_rows() {
    let mk = || {
        Attestation::issue(
            "actor",
            "subject",
            &["a", "b"],
            "claim",
            evidence(EvidenceKind::Gateway, "mcp_policy:g5", GOOD),
            "verifier",
            100,
            0,
            &|_| Ok(()),
        )
        .unwrap()
    };
    let a = mk();
    let b = mk();
    assert_eq!(canonical_row(&a), canonical_row(&b));
    assert_eq!(canonical_row_digest(&a), canonical_row_digest(&b));
    assert_eq!(a.row_digest, b.row_digest);
}
