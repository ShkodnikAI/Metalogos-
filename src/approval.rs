// ── Naryad №860 (wave 75 — W-D, P1, [core][consent]): TypedApproval ────
//
// ADR-0205 §2/§3: the mandate as a typed carrier of the human
// consent/oversight decision — who approved (`actor`), what exactly was
// approved (`target_digest`), when (`granted_at`), how long it lives
// (`expires_at`), and the revocation state.
//
// The mandate is a CONDITION OF ACTION for a grant of the human-oversight
// class — NOT a new grant mechanism and NOT a replacement: the grant
// contour (src/grants.rs, ADR-0203) stays the single rights machinery,
// `ask_approval`/`awaiting_approval` keep their semantics untouched, and
// the enforcement gate is the next naryad of the wave. Migration 1:1
// (the №844 pattern): this module only ADDS the carrier — no existing
// constructor, event surface or Ledger record changes shape.
//
// The refusal value is the W-C algebra (src/refusal_algebra.rs, ADR-0204):
// `verify` returns `Allow | Deny { code, cause }` and every deny cause is
// the EXISTING `IRREVERSIBLE_NO_GRANT` class of the frozen 14-word
// vocabulary (№385 — no new codes; the typed detail of the refusal —
// expired vs revoked vs foreign digest — stays inspectable on the
// carrier's own fields, the algebra never re-derives it).
//
// Purity (the portable-kernel boundary of ADR-0204 §3): no IO of any
// kind — no clock, no network, no filesystem, no Ledger. The host passes
// the moment in (`now`); hashing (`target_digest`) is the only
// computation and it is deterministic.

use crate::refusal_algebra::Verdict;
use sha2::{Digest, Sha256};

/// The Struct type_name reserved for the future language-visible view of
/// the mandate (the №844 `Capability` pattern). The pure Rust carrier is
/// the SSOT; the wiring that surfaces it to programs is the next
/// naryad's explicit transaction.
pub const APPROVAL_TYPE: &str = "TypedApproval";

/// The frozen deny-reason class every mandate refusal carries (№392 —
/// the live dogfood class of №856). `Verdict::deny` cannot fail for this
/// word, so the constructors below unwrap with a pinned message — the
/// vocabulary freeze (№385) is itself pinned by the №854 parity test.
const MANDATE_DENY_WORD: &str = "IRREVERSIBLE_NO_GRANT";

fn mandate_deny() -> Verdict {
    Verdict::deny(MANDATE_DENY_WORD)
        .unwrap_or_else(|| panic!("the frozen vocabulary must carry {MANDATE_DENY_WORD}"))
}

/// The typed mandate (ADR-0205 §2.1): a pure value carrying the human
/// consent/oversight decision. A mandate is issued ONCE, is withdrawable
/// by the human (`revoke`), and once revoked stays revoked — a stale
/// mandate cannot revive consent (the ADR-0155 handle posture).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TypedApproval {
    /// Who approved — the human actor ("owner", "operator-2", …).
    pub actor: String,
    /// The digest of the goal/action the approval binds. Verify refuses
    /// any other digest — a mandate is never transferable across targets
    /// (ADR-0205 §3, the foreign-digest negative control).
    pub target_digest: String,
    /// Unix epoch seconds when the human granted the mandate.
    pub granted_at: u64,
    /// Unix epoch seconds when the window closes (exclusive). The
    /// no-window form (`ttl_seconds == 0` at issue) stores `u64::MAX`
    /// (the consent-ledger ttl=0 precedent, src/consent.rs).
    pub expires_at: u64,
    /// Revocation state — once true, permanent (the stale-token posture).
    pub revoked: bool,
}

impl TypedApproval {
    /// Issue a mandate for one target digest. `ttl_seconds == 0` means
    /// no expiry window (the consent-ledger convention); any other value
    /// closes the window at `granted_at + ttl` (overflow saturates to
    /// the no-window form — a mandate never wraps into the past).
    pub fn issue(actor: &str, target_digest: &str, granted_at: u64, ttl_seconds: u64) -> Self {
        let expires_at = if ttl_seconds == 0 {
            u64::MAX
        } else {
            granted_at.saturating_add(ttl_seconds)
        };
        Self {
            actor: actor.to_string(),
            target_digest: target_digest.to_string(),
            granted_at,
            expires_at,
            revoked: false,
        }
    }

    /// The live check (ADR-0205 §3): the window holds, the target digest
    /// matches, the mandate is not revoked. Returns the W-C algebra
    /// verdict — `Allow`, or a `Deny` of the frozen
    /// `IRREVERSIBLE_NO_GRANT` class. A refusal answers BEFORE the
    /// action runs (the fail-closed posture of №845).
    pub fn verify(&self, now: u64, target_digest: &str) -> Verdict {
        if self.revoked
            || self.target_digest != target_digest
            || now < self.granted_at
            || now >= self.expires_at
        {
            return mandate_deny();
        }
        Verdict::Allow
    }

    /// Revoke the mandate — the human consent is withdrawable by the
    /// human. Idempotent and permanent: a revoked mandate never verifies
    /// again, even with a matching digest inside the original window.
    pub fn revoke(&mut self) {
        self.revoked = true;
    }
}

/// The canonical target digest: SHA-256 over the parts joined with the
/// ASCII unit separator (`\x1f`), hex-encoded. Pure and deterministic —
/// the same goal/action text always binds the same mandate target, and
/// any changed part changes the digest (the foreign-digest negative
/// control is then a plain string inequality).
pub fn target_digest(parts: &[&str]) -> String {
    let canonical = parts.join("\u{1f}");
    let digest = Sha256::digest(canonical.as_bytes());
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{:02x}", byte));
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_issued_mandate_verifies_allow_inside_its_window() {
        let d = target_digest(&["goal", "archive-2025", "delete"]);
        let m = TypedApproval::issue("owner", &d, 1_000, 600);
        assert!(!m.revoked);
        assert_eq!(m.actor, "owner");
        assert_eq!(m.expires_at, 1_600);
        assert_eq!(m.verify(1_000, &d), Verdict::Allow);
        assert_eq!(m.verify(1_599, &d), Verdict::Allow);
    }

    #[test]
    fn a_foreign_digest_refuses_before_any_action() {
        // The negative control (а): a mandate on a foreign digest → Deny.
        let d = target_digest(&["goal", "archive-2025", "delete"]);
        let other = target_digest(&["goal", "archive-2026", "delete"]);
        assert_ne!(d, other, "distinct parts must bind distinct targets");
        let m = TypedApproval::issue("owner", &d, 1_000, 600);
        let v = m.verify(1_100, &other);
        assert!(v.is_deny());
        assert_eq!(v.code(), Some("IRREVERSIBLE_NO_GRANT"));
    }

    #[test]
    fn an_expired_or_revoked_mandate_refuses_before_any_action() {
        // The negative control (б): an expired/revoked mandate → Deny.
        let d = target_digest(&["action", "kv_delete", "session-cache"]);
        let m = TypedApproval::issue("owner", &d, 1_000, 600);
        let expired = m.verify(1_600, &d); // the window is exclusive
        assert!(expired.is_deny());
        assert_eq!(expired.code(), Some("IRREVERSIBLE_NO_GRANT"));

        let mut live = TypedApproval::issue("owner", &d, 1_000, 600);
        assert_eq!(live.verify(1_500, &d), Verdict::Allow);
        live.revoke();
        let revoked = live.verify(1_500, &d);
        assert!(revoked.is_deny());
        assert_eq!(revoked.code(), Some("IRREVERSIBLE_NO_GRANT"));
    }

    #[test]
    fn revocation_is_permanent_and_the_window_edge_is_honest() {
        let d = target_digest(&["action", "memory_prune"]);
        let mut m = TypedApproval::issue("owner", &d, 1_000, 100);
        m.revoke();
        m.revoke(); // idempotent
        assert!(m.verify(1_050, &d).is_deny(), "revoked never revives");
        assert!(m.verify(2_000, &d).is_deny());

        let w = TypedApproval::issue("owner", &d, 1_000, 100);
        assert!(w.verify(999, &d).is_deny(), "before granted_at is not live");
        assert!(w.verify(1_100, &d).is_deny(), "at expires_at is not live");
    }

    #[test]
    fn the_no_window_form_stays_live_and_the_deny_word_is_frozen() {
        let d = target_digest(&["goal", "audit-export"]);
        let m = TypedApproval::issue("owner", &d, 1_000, 0);
        assert_eq!(m.expires_at, u64::MAX, "ttl 0 = no window (consent form)");
        assert_eq!(m.verify(u64::MAX - 1, &d), Verdict::Allow);

        // The vocabulary freeze (№385): the mandate deny word is one of
        // the frozen 14 — the algebra is consumed, never extended.
        assert!(crate::refusal_algebra::DENY_REASONS.contains(&MANDATE_DENY_WORD));
        assert_eq!(MANDATE_DENY_WORD, "IRREVERSIBLE_NO_GRANT");
    }

    #[test]
    fn the_carrier_is_a_plain_serializable_value_without_hidden_authority() {
        // Purity (ADR-0204 §3): serde round-trip carries the fields and
        // nothing else — a deserialized mandate has no hidden authority
        // and verifies only by its own (untrusted) contents.
        let d = target_digest(&["action", "ledger_rotate"]);
        let m = TypedApproval::issue("owner", &d, 10, 5);
        let json = serde_json::to_string(&m).expect("serialize");
        let back: TypedApproval = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, m);
        assert_eq!(back.verify(12, &d), Verdict::Allow);
        assert!(back.verify(12, &target_digest(&["forged"])).is_deny());
    }

    #[test]
    fn the_target_digest_is_deterministic_and_order_sensitive() {
        let a = target_digest(&["goal", "x", "delete"]);
        let b = target_digest(&["goal", "x", "delete"]);
        assert_eq!(a, b, "the same parts bind the same target");
        assert_ne!(a, target_digest(&["goal", "xdelete"]));
        assert_ne!(a, target_digest(&["delete", "x", "goal"]), "order matters");
        assert_eq!(a.len(), 64, "sha-256 hex");
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
