// ── Naryad №867 (wave 76 — W-E, P1, [core][compliance]): Attestation ───
//
// ADR-0206 §2/§3: the compliance attestation as a typed carrier of a
// verifiable claim — who attests (`actor`), what the claim is about
// (`subject` + `subject_digest`), the claim itself (`claim` — a code
// from the EXISTING vocabularies), the digest of the referenced
// evidence (`Evidence`), the verifier (the named check a third party
// can re-run), the freshness window (`issued_at`/`expires_at`) and the
// revocation state.
//
// The map is APPEND-ONLY (ADR-0206 §2.2): issue adds, revoke MARKS — a
// revocation never deletes the row (the consent-ledger posture of
// №335). An attestation REFERENCES its parents — a Ledger record
// (ADR-0167), a typed mandate (ADR-0205), a model passport (ADR-0202),
// a capability token (ADR-0203), a gateway digest (ADR-0196/№828) —
// and never copies them (the untrusted-view posture of №844: the
// parent surface stays the SSOT).
//
// THE DEDUP GUARD (the R54 fact-check, ADR-0206 §5): this module is the
// COMPLIANCE domain (C1). The word *attest* is alive in a DIFFERENT
// domain — `src/nn/attest.rs`, the Reflex distillation certificates
// (the C4 line, №825, the surface frozen by №832). The two share a
// root word, not a contract: this module never renames, moves, wraps
// or re-exports the C4 surface; the name-separation test in
// tests/naryad_867_attestation_kernel.rs pins the boundary mechanically.
//
// The refusal value is the W-C algebra (src/refusal_algebra.rs,
// ADR-0204): `verify` returns `Allow | Deny { code, cause }` and every
// deny cause is the EXISTING `IRREVERSIBLE_NO_GRANT` class of the
// frozen 14-word vocabulary (№385 — no new codes). The map creates no
// new deny paths and no new DenyEvent classes: the construction
// refusals (a missing evidence digest, a dangling parent reference, a
// revoked-row reuse) are constructor errors (`Result::Err`), not
// DenyEvent vocabulary.
//
// Purity (the portable-kernel boundary of ADR-0206 §3 — the
// ADR-0204/ADR-0205 form): no IO of any kind — no clock, no network,
// no filesystem, no Ledger. The host passes the moment and the live
// evidence digests in; hashing is the only computation and it is
// deterministic (the G5 digest-pinning pattern of №828: a row without
// a digest is not a row of the map). The parent-existence check is an
// INJECTED resolver — the kernel never opens a file.
//
// Migration 1:1 (the №844/№854/№860 pattern): this module only ADDS
// the carrier — no existing constructor, event surface, Ledger record
// or golden file changes shape (the ledger-golden and conformance
// suites stay green with ZERO golden edits).

use crate::refusal_algebra::Verdict;
use sha2::{Digest, Sha256};

/// The frozen deny-reason class every attestation-freshness/refusal
/// verdict carries (№392 — the same live class the mandate gate of
/// №861 uses). `Verdict::deny` cannot fail for this word; the guard
/// below keeps the vocabulary freeze loud (№385).
const COMPLIANCE_DENY_WORD: &str = "IRREVERSIBLE_NO_GRANT";

fn compliance_deny() -> Verdict {
    Verdict::deny(COMPLIANCE_DENY_WORD)
        .unwrap_or_else(|| panic!("the frozen vocabulary must carry {COMPLIANCE_DENY_WORD}"))
}

/// The Struct type_name reserved for the future language-visible view of
/// the attestation (the №844 `Capability` pattern). The pure Rust
/// carrier is the SSOT; any wiring that surfaces it to programs is a
/// future explicit registry transaction — NOT this naryad (the №867
/// boundary: no new builtin surfaces).
pub const ATTESTATION_TYPE: &str = "Attestation";

/// The evidence parent kinds (ADR-0206 §2.3): the attestation names its
/// parent by kind plus the parent's own identifier plus the digest of
/// the referenced artifact bytes. The kinds are CLOSED — a new parent
/// surface lands as an explicit ADR/registry transaction, never by a
/// silent string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EvidenceKind {
    /// The Action Ledger record (ADR-0167) — the reference is the
    /// record's identity in the trace (seq/key).
    Ledger,
    /// The typed mandate (ADR-0205) — the reference is the mandate
    /// target (`carrier:<name>` form).
    Mandate,
    /// The model passport (ADR-0202) — the reference is the passport
    /// model id.
    Passport,
    /// The capability token (ADR-0203) — the reference is the grant
    /// scope (`builtin:<name>` form).
    Capability,
    /// The MCP policy gateway digest (ADR-0196, the G5 pinning of
    /// №828) — the reference is the gateway artifact name.
    Gateway,
    /// The checked-in toolchain artifact — a source file, a gate
    /// record, a baseline, a golden, an ADR (the reference is the
    /// repo-relative path). The kind lands WITH the evidence-package
    /// compiler (№868) — the explicit transaction the closedness rule
    /// of this enum requires (ADR-0206 §2.3: the parents are named by
    /// kind plus digest; the toolchain's own artifacts are the C1
    /// seed's evidence surface).
    Artifact,
}

impl EvidenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EvidenceKind::Ledger => "ledger",
            EvidenceKind::Mandate => "mandate",
            EvidenceKind::Passport => "passport",
            EvidenceKind::Capability => "capability",
            EvidenceKind::Gateway => "gateway",
            EvidenceKind::Artifact => "artifact",
        }
    }
}

/// The evidence reference (ADR-0206 §2.3): kind + the parent's own
/// identifier + the SHA-256 (lowercase hex) of the referenced artifact
/// bytes. The attestation carries this — it NEVER embeds the parent's
/// payload (the copy would fork the SSOT and make the map lie).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Evidence {
    pub kind: EvidenceKind,
    /// The parent's own identifier (the ledger seq, the mandate target,
    /// the passport model id, the grant scope, the gateway artifact).
    pub reference: String,
    /// SHA-256, lowercase hex, of the referenced artifact bytes.
    pub digest: String,
}

fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// The typed compliance attestation (ADR-0206 §2.1): a pure value. A
/// row is issued ONCE, is withdrawable (`revoke`), and once revoked
/// stays revoked — a closed attestation cannot revive a claim (the
/// ADR-0155 handle posture, the mandate form of №860).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Attestation {
    /// Who attests — the actor ("office-machine", "auditor-2", …).
    pub actor: String,
    /// What the claim is about (the human-readable subject name).
    pub subject: String,
    /// The digest binding the subject parts (the canonical subject
    /// digest — `subject_digest`): any changed part changes the digest,
    /// the foreign-subject control is then a plain inequality.
    pub subject_digest: String,
    /// The claim code — from the EXISTING vocabularies (the gates, the
    /// baselines, the golden suites; ADR-0206 §2.1: no new claim
    /// language is minted by this record).
    pub claim: String,
    /// The referenced evidence (kind + parent id + digest).
    pub evidence: Evidence,
    /// The named check a third party can re-run (the verifier).
    pub verifier: String,
    /// Unix epoch seconds when the attestation was issued.
    pub issued_at: u64,
    /// Unix epoch seconds when the freshness window closes (exclusive).
    /// The no-window form (`ttl_seconds == 0` at issue) stores `u64::MAX`
    /// (the consent-ledger ttl=0 precedent).
    pub expires_at: u64,
    /// Revocation state — once true, permanent.
    pub revoked: bool,
    /// The row digest (the G5 №828 form): SHA-256 over the canonical
    /// serialization of every field above. A row without a digest is
    /// not a row of the map.
    pub row_digest: String,
}

/// The resolver contract (the injected parent check): returns `Ok(())`
/// when the referenced parent exists and its live digest matches
/// `evidence.digest`, `Err(reason)` otherwise. The kernel calls it at
/// issue time (the dangling-reference negative control is a
/// CONSTRUCTION refusal, ADR-0206 §4/№867) — the production resolver
/// (№868) reads the live in-repo sources; the tests inject stubs.
pub type EvidenceResolver<'a> = &'a dyn Fn(&Evidence) -> Result<(), String>;

/// The no-op resolver: every reference resolves (the map used as a pure
/// value container — the compiler pins the live resolution itself).
pub fn resolve_any(_: &Evidence) -> Result<(), String> {
    Ok(())
}

impl Attestation {
    /// Issue an attestation row with the full negative controls:
    /// (1) the empty carrier fields refuse; (2) an attestation WITHOUT
    /// an evidence digest (not a 64-hex SHA-256) refuses at
    /// construction; (3) a dangling parent reference refuses through
    /// the resolver; (4) the row digest pins the row (the G5 form).
    #[allow(clippy::too_many_arguments)]
    pub fn issue(
        actor: &str,
        subject: &str,
        subject_parts: &[&str],
        claim: &str,
        evidence: Evidence,
        verifier: &str,
        issued_at: u64,
        ttl_seconds: u64,
        resolve: EvidenceResolver,
    ) -> Result<Self, String> {
        for (name, val) in [
            ("actor", actor),
            ("subject", subject),
            ("claim", claim),
            ("verifier", verifier),
        ] {
            if val.trim().is_empty() {
                return Err(format!("attestation issue: the {name} must be non-empty"));
            }
        }
        if subject_parts.is_empty() {
            return Err("attestation issue: the subject parts must be non-empty".to_string());
        }
        if evidence.reference.trim().is_empty() {
            return Err("attestation issue: the evidence reference must be non-empty".to_string());
        }
        // Negative control (2) — the digest-pinning form of №828: an
        // attestation without a real evidence digest is not an
        // attestation, it is a claim nobody can verify.
        if !is_sha256_hex(&evidence.digest) {
            return Err(format!(
                "attestation issue: the evidence digest must be a lowercase SHA-256 hex (64 chars), got {:?}",
                evidence.digest
            ));
        }
        // Negative control (3) — the dangling parent reference refuses
        // BEFORE the row exists (fail-closed, the №845 posture).
        resolve(&evidence)?;
        let subject_digest = subject_digest(subject_parts);
        let expires_at = if ttl_seconds == 0 {
            u64::MAX
        } else {
            issued_at.saturating_add(ttl_seconds)
        };
        let mut att = Self {
            actor: actor.to_string(),
            subject: subject.to_string(),
            subject_digest,
            claim: claim.to_string(),
            evidence,
            verifier: verifier.to_string(),
            issued_at,
            expires_at,
            revoked: false,
            row_digest: String::new(),
        };
        att.row_digest = canonical_row_digest(&att);
        Ok(att)
    }

    /// The live check (ADR-0206 §2.5, the mandate verify form of
    /// №860): the freshness window holds and the row is not revoked.
    /// Returns the W-C algebra verdict — `Allow`, or a `Deny` of the
    /// frozen `IRREVERSIBLE_NO_GRANT` class. The evidence-digest match
    /// is the CONSUMER's check (`evidence_matches`) — the kernel has
    /// no filesystem; the map never vouches for bytes it has not seen.
    pub fn verify(&self, now: u64) -> Verdict {
        if self.revoked || now < self.issued_at || now >= self.expires_at {
            return compliance_deny();
        }
        Verdict::Allow
    }

    /// The evidence-digest match: the live digest of the referenced
    /// artifact against the pinned one. A mismatch is a LOUD
    /// verification failure (ADR-0206 §2.5 — nothing degrades silently
    /// into «compliant»).
    pub fn evidence_matches(&self, live_digest: &str) -> bool {
        self.evidence.digest == live_digest
    }

    /// Revoke the attestation — the attesting actor withdraws the
    /// claim. Idempotent and permanent: a revoked row never verifies
    /// again, even inside the original window (the stale-token posture).
    pub fn revoke(&mut self) {
        self.revoked = true;
    }
}

/// The canonical subject digest: SHA-256 over the parts joined with the
/// ASCII unit separator (`\x1f`), hex-encoded (the mandate
/// `target_digest` form of src/approval.rs — the same deterministic
/// binding so the two carriers interoperate).
pub fn subject_digest(parts: &[&str]) -> String {
    let canonical = parts.join("\u{1f}");
    let digest = Sha256::digest(canonical.as_bytes());
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{:02x}", byte));
    }
    hex
}

/// The canonical row serialization (the determinism contract of
/// ADR-0206 §2.4 — the fixed field order, the `\x1f` separator). The
/// row digest pins the CLAIM (every field except `revoked` — the
/// revocation is a lifecycle mark that rides BESIDE the digest in the
/// export, №868, so a revocation never silently re-digests a row:
/// the auditor sees the original claim digest plus the status). Pure
/// and deterministic: the same fields always produce the same bytes.
pub fn canonical_row(att: &Attestation) -> String {
    [
        "v1",
        &att.actor,
        &att.subject,
        &att.subject_digest,
        &att.claim,
        att.evidence.kind.as_str(),
        &att.evidence.reference,
        &att.evidence.digest,
        &att.verifier,
        &att.issued_at.to_string(),
        &att.expires_at.to_string(),
    ]
    .join("\u{1f}")
}

/// The row digest: SHA-256 over `canonical_row`, hex-encoded (the G5
/// form of №828 — a row without a digest is not a row of the map).
pub fn canonical_row_digest(att: &Attestation) -> String {
    let digest = Sha256::digest(canonical_row(att).as_bytes());
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{:02x}", byte));
    }
    hex
}

/// The append-only attestation map (ADR-0206 §2.2): issue adds, revoke
/// MARKS — a revocation never deletes the row. The rows stay in ISSUE
/// ORDER; the deterministic SORT for the export is the compiler's
/// contract (№868), not the store's.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AttestationMap {
    rows: Vec<Attestation>,
}

impl AttestationMap {
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    /// Append one attestation (the resolver-injected form — the
    /// dangling-parent control runs at issue). An EXACT duplicate (the
    /// same row digest) refuses: the same attestation twice is a
    /// construction error, not a row (the append-only store stays
    /// honest; re-attesting the same claim later is a NEW row with a
    /// new `issued_at` and a new digest).
    pub fn issue_with(
        &mut self,
        att: Attestation,
        resolve: EvidenceResolver,
    ) -> Result<usize, String> {
        if self.rows.iter().any(|r| r.row_digest == att.row_digest) {
            return Err(format!(
                "attestation map: the exact row {} is already in the map (append-only, no duplicates)",
                att.row_digest
            ));
        }
        resolve(&att.evidence)?;
        self.rows.push(att);
        Ok(self.rows.len() - 1)
    }

    /// Append with the no-op resolver (the pure-container form).
    pub fn issue(&mut self, att: Attestation) -> Result<usize, String> {
        self.issue_with(att, &resolve_any)
    }

    /// Revoke by row digest — MARKS the row (append-only: the row
    /// stays, `revoked` flips). An unknown digest refuses loudly (the
    /// dangling-reference posture); a known digest revokes
    /// idempotently and returns the row index.
    pub fn revoke(&mut self, row_digest: &str) -> Result<usize, String> {
        let idx = self
            .rows
            .iter()
            .position(|r| r.row_digest == row_digest)
            .ok_or_else(|| {
                format!(
                    "attestation map: no row with digest {row_digest} — the dangling reference refuses"
                )
            })?;
        self.rows[idx].revoke();
        Ok(idx)
    }

    /// The read-only row view (the compiler's input, №868).
    pub fn rows(&self) -> &[Attestation] {
        &self.rows
    }

    /// The live-row count (the ATTEST_MAP fact counts LIVE rows —
    /// revoked rows stay in the map for the audit trail but attest
    /// nothing; №869 counts from the RAW rows structurally).
    pub fn live_count(&self) -> usize {
        self.rows.iter().filter(|r| !r.revoked).count()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[cfg(test)]
mod kernel_tests {
    //! The negative controls are part of the CONTRACT (ADR-0206 §4, the
    //! №860 form): the refusals run at construction/verification time,
    //! not only in the integration suite.

    use super::*;

    const GOOD_DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn evidence() -> Evidence {
        Evidence {
            kind: EvidenceKind::Ledger,
            reference: "w72_capability_trace:seq:1".to_string(),
            digest: GOOD_DIGEST.to_string(),
        }
    }

    #[test]
    fn issue_refuses_without_an_evidence_digest() {
        let long65 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        for bad in ["", "zz", "AAAA", "a", long65] {
            let ev = Evidence {
                digest: bad.to_string(),
                ..evidence()
            };
            let err = Attestation::issue(
                "actor",
                "subject",
                &["s"],
                "claim",
                ev,
                "verifier",
                100,
                0,
                &resolve_any,
            )
            .unwrap_err();
            assert!(
                err.contains("SHA-256"),
                "the bad digest {bad:?} refuses at construction, got: {err}"
            );
        }
    }

    #[test]
    fn issue_refuses_empty_carrier_fields_and_the_dangling_parent() {
        let err = Attestation::issue("", "s", &["s"], "c", evidence(), "v", 0, 0, &resolve_any)
            .unwrap_err();
        assert!(err.contains("actor"));
        let dangling = |ev: &Evidence| Err(format!("no ledger record {}", ev.reference));
        let err = Attestation::issue("a", "s", &["s"], "c", evidence(), "v", 0, 0, &dangling)
            .unwrap_err();
        assert!(
            err.contains("no ledger record"),
            "the dangling parent reference refuses at construction, got: {err}"
        );
    }

    #[test]
    fn verify_refuses_expired_and_revoked_on_the_frozen_class() {
        let mut att = Attestation::issue(
            "a",
            "s",
            &["s"],
            "c",
            evidence(),
            "v",
            100,
            50,
            &resolve_any,
        )
        .unwrap();
        assert!(att.verify(120) == Verdict::Allow);
        // the window is exclusive at expires_at
        assert!(matches!(att.verify(150), Verdict::Deny { .. }));
        assert_eq!(att.verify(150).code(), Some(COMPLIANCE_DENY_WORD));
        // revocation is permanent
        att.revoke();
        assert!(matches!(att.verify(120), Verdict::Deny { .. }));
        assert_eq!(att.row_digest.len(), 64);
        // the digest pins the CLAIM, not the lifecycle flag: a
        // revocation never silently re-digests a row (the export rides
        // the status beside the digest — №868)
        assert_eq!(canonical_row_digest(&att), att.row_digest);
    }

    #[test]
    fn the_map_is_append_only_and_revoke_marks() {
        let mut map = AttestationMap::new();
        let att = Attestation::issue("a", "s", &["s"], "c", evidence(), "v", 100, 0, &resolve_any)
            .unwrap();
        let dg = att.row_digest.clone();
        assert_eq!(map.issue(att.clone()).unwrap(), 0);
        assert!(
            map.issue(att).unwrap_err().contains("already in the map"),
            "the exact duplicate refuses"
        );
        assert_eq!(map.live_count(), 1);
        assert_eq!(map.revoke(&dg).unwrap(), 0);
        assert_eq!(
            map.live_count(),
            0,
            "the revoked row stays but attests nothing"
        );
        assert_eq!(map.len(), 1, "append-only: the revocation never deletes");
        let err = map.revoke("ffffff").unwrap_err();
        assert!(
            err.contains("dangling"),
            "the unknown digest refuses loudly, got: {err}"
        );
    }
}
