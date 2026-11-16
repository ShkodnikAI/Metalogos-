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
    /// №957 (ADR-0211 §3): whether the mandate was issued by a path
    /// OUTSIDE the program's reach (the host CLI `mlog approve`) — the
    /// counted class of the human-oversight coverage. The
    /// `approval_issue` builtin always records `false`: the actor name a
    /// program passes is a STRING, not a human (ADR-0211 §1 — the trust
    /// anchor moves outside the machine). `serde(default)` keeps the
    /// fail-closed direction for any store row that predates the field:
    /// a missing mark reads as `false` — never-counted, never forged.
    #[serde(default)]
    pub actor_verified: bool,
}

impl TypedApproval {
    /// Issue a mandate for one target digest. `ttl_seconds == 0` means
    /// no expiry window (the consent-ledger convention); any other value
    /// closes the window at `granted_at + ttl` (overflow saturates to
    /// the no-window form — a mandate never wraps into the past).
    ///
    /// The PROGRAM-reachable constructor: the mandate it records is
    /// NEVER verified (`actor_verified: false`, ADR-0211 §3 — the
    /// marking mechanic of variant (в) inside variant (б)); the verified
    /// class is born only through `issue_verified`, whose call sites are
    /// host tooling outside the `.mlog` reach.
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
            actor_verified: false,
        }
    }

    /// №957 (ADR-0211 §3): the HOST-issued constructor — the verified
    /// class. The call sites live in the host CLI (`mlog approve`) and
    /// the tests that pin the seam; no `.mlog` builtin reaches it, so a
    /// counted mandate is born outside the program's reach by
    /// construction (the fail-closed direction is structural, §3.2 of
    /// the recommendation).
    pub fn issue_verified(
        actor: &str,
        target_digest: &str,
        granted_at: u64,
        ttl_seconds: u64,
    ) -> Self {
        let mut m = Self::issue(actor, target_digest, granted_at, ttl_seconds);
        m.actor_verified = true;
        m
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

// ── №957 (issue #1924, ADR-0211 §3): the host-issued verified store ───
//
// The counted class of the human-oversight coverage is born OUTSIDE the
// program's reach: the host CLI (`mlog approve`) appends verified
// mandates to a host-owned file, and the gate reads that file AT CALL
// TIME. The file is the persistence form (the in-memory limitation of
// the №861 store is closed for the verified class — a mandate survives
// the restart by construction, because it never lived in the process).
//
// The trust boundary (ADR-0211 §2.2, the ADR-0186 §4.5 posture): the
// store path is HOST territory — `METALOGOS_MANDATE_STORE` or
// `$HOME/.metalogos/mandates.json` — and the path is on the №455
// sensitive-path deny vocabulary (`.metalogos/**`, `mandates.json`,
// src/builtins/io.rs), so a program cannot read OR write it through any
// gated file surface: the fail-closed direction is structural, not a
// check the program could talk its way past.
//
// The file I/O here is raw `std::fs` BY DESIGN (the №475 ratchet's
// explicitly-justified service module): this path is the HOST'S approval
// authority, not a program-influenced surface — routing it through
// `crate::fs_gate` would apply the program-facing sandbox to the very
// file the program must never touch. Every call below runs in the host
// seam (`mlog approve`) or the runtime gate's read; no `.mlog` path
// reaches them.

/// The verified-store path: `METALOGOS_MANDATE_STORE` wins (the host
/// contour's override — tests and multi-store hosts), the default is the
/// host-scoped `~/.metalogos/mandates.json`. No HOME and no override →
/// the loud error (fail-closed: the CLI refuses, the gate reports the
/// store unavailable and refuses the verified class).
pub fn mandate_store_path() -> Result<std::path::PathBuf, String> {
    if let Ok(p) = std::env::var("METALOGOS_MANDATE_STORE") {
        if p.trim().is_empty() {
            return Err("METALOGOS_MANDATE_STORE is set but empty".to_string());
        }
        return Ok(std::path::PathBuf::from(p));
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| {
            "no mandate store: set METALOGOS_MANDATE_STORE, or run with HOME/USERPROFILE set"
                .to_string()
        })?;
    Ok(std::path::PathBuf::from(home)
        .join(".metalogos")
        .join("mandates.json"))
}

/// Load the persisted verified mandates. A MISSING file is the normal
/// empty state (no host approval has ever been recorded) — NOT an error.
/// An UNREADABLE or CORRUPT store is the loud error: the gate turns it
/// into the fail-closed refusal, never a silent pass (the same posture
/// as the in-memory lock error).
#[allow(clippy::disallowed_methods)] // №957: the host-owned approval store — see the section note
fn verified_store_load() -> Result<Vec<TypedApproval>, String> {
    let path = mandate_store_path()?;
    let Ok(bytes) = std::fs::read(&path) else {
        // Missing = empty. Any other read error (permissions, is-a-dir)
        // is loud: the host must see why its authority is unreadable.
        if std::fs::metadata(&path).is_err() {
            return Ok(Vec::new());
        }
        return Err(format!(
            "the verified mandate store {} is unreadable",
            path.display()
        ));
    };
    #[derive(serde::Deserialize)]
    struct StoreFile {
        mandates: Vec<TypedApproval>,
    }
    let parsed: StoreFile = serde_json::from_slice(&bytes).map_err(|e| {
        format!(
            "the verified mandate store {} is corrupt: {e}",
            path.display()
        )
    })?;
    Ok(parsed.mandates)
}

/// Append one verified mandate to the persisted store. The write is
/// atomic (temp file + rename — a torn write would be a corrupt store,
/// the fail-closed refusal of every later carrier call) and the file is
/// host-private (0700 dir / 0600 file, unix). Append-only like the
/// in-memory store: a revocation MARKS, it never deletes.
#[allow(clippy::disallowed_methods)] // №957: the host-owned approval store — see the section note
fn verified_store_append(mandate: &TypedApproval) -> Result<std::path::PathBuf, String> {
    use std::io::Write;
    let path = mandate_store_path()?;
    let mut existing = verified_store_load()?;
    existing.push(mandate.clone());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("the mandate store dir {}: {e}", dir.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
        }
    }
    #[derive(serde::Serialize)]
    struct StoreFile<'a> {
        mandates: &'a [TypedApproval],
    }
    let body = serde_json::to_vec_pretty(&StoreFile {
        mandates: &existing,
    })
    .map_err(|e| format!("the mandate store serialize: {e}"))?;
    let tmp = path.with_extension("json.tmp");
    {
        let mut f = std::fs::File::create(&tmp)
            .map_err(|e| format!("the mandate store {}: {e}", tmp.display()))?;
        f.write_all(&body)
            .map_err(|e| format!("the mandate store {}: {e}", tmp.display()))?;
        f.sync_all()
            .map_err(|e| format!("the mandate store sync {}: {e}", tmp.display()))?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, &path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("the mandate store rename {}: {e}", path.display())
    })?;
    Ok(path)
}

/// №957 (ADR-0211 §3): the HOST seam — record the human's approval as a
/// VERIFIED mandate over `target` and persist it (the mandate survives
/// the restart; the in-memory limitation of the №861 store is closed for
/// the verified class). The ONLY path that mints `actor_verified: true`;
/// its call sites are the host CLI (`mlog approve`) and the tests pinning
/// the seam — no `.mlog` builtin reaches it.
pub fn mandate_issue_verified(
    actor: &str,
    target: &str,
    ttl_seconds: u64,
) -> Result<TypedApproval, String> {
    if actor.trim().is_empty() {
        return Err(
            "approve: the actor must name the human who approved (non-empty String)".to_string(),
        );
    }
    if target.trim().is_empty() {
        return Err("approve: the target must name what was approved (non-empty String, e.g. \"carrier:cron_run\")".to_string());
    }
    let mandate =
        TypedApproval::issue_verified(actor, &target_digest(&[target]), now_epoch(), ttl_seconds);
    let path = verified_store_append(&mandate)?;
    let _ = path;
    Ok(mandate)
}

/// The refusal text (the typed deny surface): the SAME reason class as
/// the capability gate — the vocabulary does not grow (№385) — with the
/// human remedy verbatim (the honest refusal form of №845). №957: the
/// remedy is the HOST CLI — a program-issued mandate is never counted
/// (ADR-0211 §3), so the text names `mlog approve` and, for the
/// migration window, the §12.5-form key.
fn approval_refusal(name: &str) -> String {
    format!(
        "IRREVERSIBLE_NO_GRANT: approval gate: '{name}' is a human-oversight carrier — no live \
         VERIFIED approval mandate (issue one OUTSIDE the program — the host CLI `mlog approve \
         carrier:{name} --actor <who> [--ttl S]`; a mandate recorded by approval_issue from \
         .mlog is actor_verified=false and never counts — ADR-0205 §2.6, №861, №957/ADR-0211 \
         §3: fail-closed, a silent pass is forbidden; the migration window key is `profile \
         legacy {{ approval: warn }}` in the entry program)"
    )
}

/// THE approval decision (№861/ADR-0205 §2.6; №957/ADR-0211 §3 — the
/// verified class): a human-oversight carrier without a LIVE VERIFIED
/// mandate refuses BEFORE the action. Composed at the tail of
/// `capability::authorize_call` — every dispatch site that authorizes
/// the capability layer approval-gates too, both backends, one surface.
/// A carrier outside the class resolves trivially (`Ok`).
///
/// The counted class is the VERIFIED one: the mandates minted by the
/// host CLI (`mlog approve` — `mandate_issue_verified`, the persisted
/// host-owned store) and nothing else. A PROGRAM-issued mandate
/// (`approval_issue` from .mlog, `actor_verified: false`) is never
/// counted in the default mode — the marking mechanic of ADR-0211 §3
/// (variant (в) inside variant (б)). The TRANSITION WINDOW (the №949
/// п.5 rule, the §12.5 form): while the ENTRY program declares
/// `profile legacy { approval: warn }`, a live program-issued mandate
/// still opens the carrier, with a LOUD stderr warning naming the host
/// remedy — one release window, then the key is removed and the program
/// carrying it is a check error (the window flag rides the
/// CapabilityPermits wire, so TW and VM share it by construction).
///
/// Fail-closed directions: an unavailable in-memory store refuses; an
/// unreadable or corrupt VERIFIED store refuses (the host's authority
/// must never degrade into a silent pass); a missing store file is the
/// normal empty state.
pub fn authorize_overseen_call(name: &str, legacy_approval_warn: bool) -> Result<(), String> {
    if !HUMAN_OVERSIGHT.contains(&name) {
        return Ok(());
    }
    let digest = target_digest(&[&carrier_target(name)]);
    let now = now_epoch();
    // 1. The verified class — the persisted host-issued store. Missing
    //    file = empty (no host approval yet); corrupt/unreadable = the
    //    loud fail-closed refusal.
    match verified_store_load() {
        Ok(store) => {
            if store
                .iter()
                .any(|m| m.actor_verified && m.verify(now, &digest) == Verdict::Allow)
            {
                return Ok(());
            }
        }
        Err(e) => {
            return Err(format!(
                "{} (the verified mandate store is unavailable: {e})",
                approval_refusal(name)
            ))
        }
    }
    // 2. The transition window — a live PROGRAM-issued mandate opens the
    //    carrier only while the entry declares the legacy key, loudly
    //    (the §12.5 form: the refusals fire as warnings, one window).
    if legacy_approval_warn {
        match lock_store() {
            Ok(store) => {
                if store
                    .iter()
                    .any(|m| m.verify(now, &digest) == Verdict::Allow)
                {
                    eprintln!(
                        "⚠  [APPROVAL_ACTOR_UNVERIFIED] (the transition window): the live \
                         mandate over carrier:{name} was issued by the PROGRAM (approval_issue \
                         — actor_verified=false) and stops counting after the window; the \
                         human-oversight carrier is opened THIS ONCE with the loud mark. \
                         Issue the verified mandate OUTSIDE the program: `mlog approve \
                         carrier:{name} --actor <who> [--ttl S]` (№957, ADR-0211 §3; the key \
                         `profile legacy {{ approval: warn }}` is removed after the window)."
                    );
                    return Ok(());
                }
            }
            // The window cannot verify the live state → fail-closed even
            // under the warn flag (never a silent pass).
            Err(e) => {
                return Err(format!(
                    "{} (the mandate store is unavailable: {e})",
                    approval_refusal(name)
                ))
            }
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
        // №957 (ADR-0211 §3): the program-reachable constructor NEVER
        // mints the verified class — the actor a program passes is a
        // string, not a human.
        assert!(
            !m.actor_verified,
            "TypedApproval::issue must record actor_verified=false (№957)"
        );
        assert_eq!(m.verify(1_000, &d), Verdict::Allow);
        assert_eq!(m.verify(1_599, &d), Verdict::Allow);
    }

    #[test]
    fn the_verified_constructor_is_the_only_verified_birth() {
        // №957 (ADR-0211 §3): the host seam mints the counted class; the
        // marking travels the serde round-trip (the persisted store
        // reads back what the host wrote).
        let d = target_digest(&["carrier:cron_run"]);
        let m = TypedApproval::issue_verified("owner", &d, 10, 60);
        assert!(m.actor_verified, "issue_verified mints the verified class");
        let json = serde_json::to_string(&m).expect("serialize");
        let back: TypedApproval = serde_json::from_str(&json).expect("deserialize");
        assert!(back.actor_verified);
        assert_eq!(back.verify(20, &d), Verdict::Allow);

        // The fail-closed default: a store row without the field (the
        // pre-№957 form) reads as NEVER-COUNTED, not as verified.
        let legacy_row = format!(
            r#"{{"actor":"owner","target_digest":"{d}","granted_at":10,"expires_at":70,"revoked":false}}"#
        );
        let old: TypedApproval = serde_json::from_str(&legacy_row).expect("deserialize");
        assert!(
            !old.actor_verified,
            "a missing actor_verified mark reads fail-closed (never-counted)"
        );
    }

    #[test]
    #[serial_test::serial]
    fn the_verified_store_persists_and_survives_the_restart_shape() {
        // №957 (ADR-0211 §3): the host-issued mandate lives in the
        // persisted store — the "survives the restart" half of the
        // naryad is the file being the SSOT (the gate reads it AT CALL
        // TIME, a fresh load every time; the in-memory store never sees
        // the verified class).
        let dir = tempfile::tempdir().expect("tempdir");
        let store = dir.path().join("mandates.json");
        // SAFETY (serial): the env var is process-global; the test is
        // serialized and restores the caller's value.
        let saved = std::env::var("METALOGOS_MANDATE_STORE").ok();
        std::env::set_var("METALOGOS_MANDATE_STORE", &store);
        let issued = mandate_issue_verified("owner", "carrier:cron_run", 600)
            .expect("the host seam issues and persists");
        assert!(issued.actor_verified);
        // The file exists, holds the mandate, and a FRESH load (the
        // restart shape: nothing in memory carries it) verifies it live.
        let digest = target_digest(&["carrier:cron_run"]);
        let reloaded = verified_store_load().expect("the persisted store reads back");
        assert_eq!(reloaded.len(), 1);
        assert!(reloaded[0].actor_verified);
        assert_eq!(reloaded[0].verify(now_epoch(), &digest), Verdict::Allow);
        assert!(store.exists(), "the store file is written");
        let _ = issued;
        match saved {
            Some(v) => std::env::set_var("METALOGOS_MANDATE_STORE", v),
            None => std::env::remove_var("METALOGOS_MANDATE_STORE"),
        }
    }

    #[test]
    #[serial_test::serial]
    #[allow(clippy::disallowed_methods)] // the №475 ratchet: the test harness class — the fixture is the corrupt store itself
    fn a_corrupt_verified_store_is_the_loud_error_never_a_pass() {
        // The fail-closed direction: a corrupt store is the loud error
        // (the gate refuses the verified class), never an empty pass.
        let dir = tempfile::tempdir().expect("tempdir");
        let store = dir.path().join("mandates.json");
        let saved = std::env::var("METALOGOS_MANDATE_STORE").ok();
        std::env::set_var("METALOGOS_MANDATE_STORE", &store);
        std::fs::write(&store, b"{ not json").expect("write the corrupt store");
        let err = verified_store_load().expect_err("a corrupt store refuses loudly");
        assert!(
            err.contains("corrupt"),
            "the error names the corruption: {err}"
        );
        match saved {
            Some(v) => std::env::set_var("METALOGOS_MANDATE_STORE", v),
            None => std::env::remove_var("METALOGOS_MANDATE_STORE"),
        }
    }

    #[test]
    fn the_gate_refuses_without_a_verified_mandate_and_names_the_host_remedy() {
        // №957 (ADR-0211 §3): the default-mode gate text names the HOST
        // remedy (`mlog approve`) — the refusal stays explainable after
        // the program path stops counting. The frozen class is
        // unchanged (№385).
        let text = approval_refusal("cron_run");
        assert!(text.contains("IRREVERSIBLE_NO_GRANT"));
        assert!(text.contains("mlog approve"), "the remedy is the host CLI");
        assert!(text.contains("actor_verified=false"));
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
