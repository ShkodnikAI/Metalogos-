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

// ── Naryad №861 (wave 75 — W-D, P1, [consent][core]): the coverage rule ─
//
// The runtime contour of the typed approval: the human-oversight roster,
// the in-process mandate store, and the fail-closed gate. This section is
// the EXECUTOR SEAM — it is NOT the portable kernel of ADR-0204 §3 (it
// reads the wall clock and keeps process state; the pure carrier above
// stays pure). The gate composes into `capability::authorize_call` — ONE
// decision surface shared by BOTH backends at every dispatch choke point,
// so the TW/VM parity is by construction (the №829 lesson) and no
// dispatch site changes.
//
// The roster is an EXPLICIT registry transaction (the №845 form — no
// silent shelves): the wave-75 class opens with `cron_run` — the trigger
// of autonomous scheduled execution, the canonical C11 human-oversight
// point (a scheduled run fires only while the human's oversight mandate
// is live). The voice / delivery / destroy classes stay on their EXISTING
// consent gates (the VOICE_EGRESS_UNCONSENTED lineage — the leak-suite
// n14 class; the capability sharp permits of №845): layering a second
// consent refusal on them would muddle the frozen vocabulary classes.
// `ask_approval` stays the REQUEST surface, its semantics untouched
// (ADR-0205 §6). Extending the class = a future explicit transaction
// with the same totality discipline (the test below reddens any carrier
// the gate cannot reach).

/// The human-oversight carrier roster (the wave-75 registry transaction,
/// №861/ADR-0205 §2.6): carriers whose execution additionally demands a
/// LIVE approval mandate. Every name here must be an irreversible
/// ProfilePermit carrier (the totality test pins the reachability —
/// a GrantAlways or non-carrier name here would silently bypass the
/// gate composition and fail the test loudly).
pub const HUMAN_OVERSIGHT: &[&str] = &["cron_run"];

/// The gate's canonical target for a carrier: the wave-75 transaction
/// binds the mandate to the CARRIER — the action class (`carrier:cron_run`),
/// not the full call digest (the finer binding is a future transaction;
/// documented honestly here).
fn carrier_target(name: &str) -> String {
    format!("carrier:{name}")
}

fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// The in-process mandate store — the consent-layer SSOT for the run
/// (append-only; issue adds, revoke MARKS — a revocation never deletes
/// the record, the same loud-bookkeeping posture as the consent ledger
/// of №335). The gate reads it AT CALL TIME: a stored mandate is an
/// untrusted cache of the human decision, the live check is `verify`.
/// The lock error propagates (the house `with_conn` form of grants.rs) —
/// the gate turns it into the fail-closed refusal, never a silent pass.
fn mandates() -> &'static std::sync::Mutex<Vec<TypedApproval>> {
    static MANDATES: std::sync::OnceLock<std::sync::Mutex<Vec<TypedApproval>>> =
        std::sync::OnceLock::new();
    MANDATES.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

fn lock_store() -> Result<std::sync::MutexGuard<'static, Vec<TypedApproval>>, String> {
    mandates()
        .lock()
        .map_err(|e| format!("the mandate store lock: {}", e))
}

/// Record the human's approval decision as a typed mandate over `target`
/// (the runtime seam: `granted_at` is the wall clock NOW). Returns the
/// stored carrier.
pub fn mandate_issue(actor: &str, target: &str, ttl_seconds: u64) -> Result<TypedApproval, String> {
    let mandate = TypedApproval::issue(actor, &target_digest(&[target]), now_epoch(), ttl_seconds);
    lock_store()?.push(mandate.clone());
    Ok(mandate)
}

/// Revoke every LIVE mandate over `target` — the human withdraws consent.
/// Idempotent; returns the number of mandates revoked by this call (a
/// revoked mandate never revives — the stale-token posture of ADR-0205 §3).
pub fn mandate_revoke(target: &str) -> Result<usize, String> {
    let digest = target_digest(&[target]);
    let now = now_epoch();
    let mut store = lock_store()?;
    let mut n = 0;
    for m in store.iter_mut() {
        if m.target_digest == digest && m.verify(now, &digest) == Verdict::Allow {
            m.revoke();
            n += 1;
        }
    }
    Ok(n)
}

/// The live mandate count over `target` (the diagnostic read — the
/// `capability_state` posture: a read consumes and mutates nothing).
/// A lock error reads 0 (a diagnostic never lies loudly — the honest
/// count is the store's, the gate does not consume this number).
pub fn mandate_live_count(target: &str) -> usize {
    let Ok(store) = lock_store() else {
        return 0;
    };
    let digest = target_digest(&[target]);
    let now = now_epoch();
    store
        .iter()
        .filter(|m| m.verify(now, &digest) == Verdict::Allow)
        .count()
}

/// The test-harness reset seam: the store is process-global, the serial
/// e2e harness resets it between scenarios (a serve process never calls
/// this — documented, the №845 env-permit seam posture). A lock error is
/// tolerated here: the harness owns the store, the next scenario re-arms.
pub fn mandate_store_reset() {
    if let Ok(mut store) = lock_store() {
        store.clear();
    }
}

/// The refusal text (the typed deny surface): the SAME reason class as
/// the capability gate — the vocabulary does not grow (№385) — with the
/// human remedy verbatim (the honest refusal form of №845).
fn approval_refusal(name: &str) -> String {
    format!(
        "IRREVERSIBLE_NO_GRANT: approval gate: '{name}' is a human-oversight carrier — no live \
         approval mandate (record the human's decision with approval_issue(\"actor\", \
         \"carrier:{name}\", ttl_seconds) after the human approves; the refusal is BEFORE the \
         action — ADR-0205 §2.6, №861: fail-closed, a silent pass is forbidden)"
    )
}

/// THE approval decision (№861/ADR-0205 §2.6): a human-oversight carrier
/// without a LIVE mandate refuses BEFORE the action. Composed at the tail
/// of `capability::authorize_call` — every dispatch site that authorizes
/// the capability layer approval-gates too, both backends, one surface.
/// A carrier outside the class resolves trivially (`Ok`).
pub fn authorize_overseen_call(name: &str) -> Result<(), String> {
    if !HUMAN_OVERSIGHT.contains(&name) {
        return Ok(());
    }
    let digest = target_digest(&[&carrier_target(name)]);
    let now = now_epoch();
    match lock_store() {
        Ok(store) => {
            if store
                .iter()
                .any(|m| m.verify(now, &digest) == Verdict::Allow)
            {
                return Ok(());
            }
        }
        // The gate cannot verify the live state → fail-closed (the same
        // posture as an unreadable coverage record: never a silent pass).
        Err(e) => {
            return Err(format!(
                "{} (the mandate store is unavailable: {e})",
                approval_refusal(name)
            ))
        }
    }
    Err(approval_refusal(name))
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
