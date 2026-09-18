// ── Naryad #387 (P2, security/media): LikenessToken — ADR-0149 D1/D6 ──
//
// The opaque likeness-consent token: the third legal credential for
// private/camera-origin media egress beside the public label and the
// consent scope (№335). Design mirrors the freshest opaque pattern in
// the tree — `Value::Grant` (№390, ADR-0155) over the №335 consent
// ledger:
//
//   - `likeness_challenge(subject, scope?, ttl?)` — issues a ONE-TIME
//     challenge handle (opaque `Value::LikenessChallenge`); registry
//     state only, no egress.
//   - `likeness_verify(challenge, subject?, scope?)` — consumes the
//     challenge (linear: a challenge verifies exactly once; reuse is a
//     typed fail-closed error) and returns the opaque
//     `Value::Likeness` token. The verify records the grant in the
//     consent ledger (`src/consent.rs::record_grant`) so the ritual
//     leaves the same trace every other consent grant leaves.
//
// The token is NOT a String (the P1-7 guideline requirement): a String
// can never occupy a token position because the token exists only as
// the opaque `Value::Likeness` variant — serde emits a dead marker
// (serialization cannot revive the credential), Display is a bracketed
// opaque marker, and the value is non-printable like every other
// opaque security handle (ADR-0114 pattern).
//
// Honest boundary (ADR-0149 D2): the MVP ritual is presence-based —
// the STATIC gate checks that the ritual (a bound `likeness_verify`
// result) precedes the gated call site; it cannot verify face
// identity. "MVP detector, not adversarial guarantee."

use crate::interpreter::values::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Opaque one-time challenge handle carried in `Value::LikenessChallenge`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeHandle {
    pub id: u64,
}

/// Opaque likeness token carried in `Value::Likeness` (ADR-0149 D6 —
/// the cross-pillar consent credential).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenHandle {
    pub id: u64,
}

impl TokenHandle {
    /// Runtime credential check: is THIS token a registry-issued one?
    pub fn is_issued(&self) -> Result<bool, String> {
        token_is_issued(self.id)
    }
}

// serde mirrors the GrantHandle posture: the credential never crosses a
// serialization boundary — only a safe dead marker. A deserialized
// handle is a tombstone that no gate ever accepts.
impl serde::Serialize for ChallengeHandle {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str("[LIKENESS_CHALLENGE]")
    }
}

impl<'de> serde::Deserialize<'de> for ChallengeHandle {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?;
        Ok(ChallengeHandle { id: 0 })
    }
}

impl serde::Serialize for TokenHandle {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str("[LIKENESS_TOKEN]")
    }
}

impl<'de> serde::Deserialize<'de> for TokenHandle {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?;
        Ok(TokenHandle { id: 0 })
    }
}

impl std::fmt::Display for ChallengeHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[LikenessChallenge]")
    }
}

impl std::fmt::Display for TokenHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[LikenessToken]")
    }
}

impl ChallengeHandle {
    /// Build the `Value::LikenessChallenge` variant.
    pub fn to_value(self) -> Value {
        Value::LikenessChallenge(self)
    }
}

impl TokenHandle {
    /// Build the `Value::Likeness` variant.
    pub fn to_value(self) -> Value {
        Value::Likeness(self)
    }
}

/// One pending challenge: the ritual parameters captured at issue time.
#[derive(Debug, Clone)]
struct PendingChallenge {
    subject: String,
    scope: String,
}

/// Registry state: pending challenges + issued tokens.
/// The SSOT for the in-process ritual state (the consent ledger holds
/// the durable trace; this registry only decides verify success).
#[derive(Default)]
struct LikenessRegistry {
    next_id: u64,
    pending: HashMap<u64, PendingChallenge>,
    issued: HashMap<u64, PendingChallenge>,
}

static LIKENESS_REGISTRY: OnceLock<Mutex<LikenessRegistry>> = OnceLock::new();

fn registry() -> &'static Mutex<LikenessRegistry> {
    LIKENESS_REGISTRY.get_or_init(|| Mutex::new(LikenessRegistry::default()))
}

/// `likeness_challenge(subject, scope?, ttl?)` — issue a one-time
/// challenge. Registry-only; no ledger write (nothing is granted yet).
pub fn challenge_issue(subject: &str, scope: &str, _ttl_seconds: u64) -> Result<Value, String> {
    let id = {
        let mut reg = registry()
            .lock()
            .map_err(|_| "likeness registry poisoned (challenge_issue)".to_string())?;
        reg.next_id += 1;
        let id = reg.next_id;
        reg.pending.insert(
            id,
            PendingChallenge {
                subject: subject.to_string(),
                scope: scope.to_string(),
            },
        );
        id
    };
    Ok(ChallengeHandle { id }.to_value())
}

/// `likeness_verify(challenge, subject?, scope?)` — consume the
/// challenge (exactly once) and issue the token. Fail-closed: an
/// unknown, consumed, or non-challenge value refuses with a typed
/// error. A successful verify records the consent-ledger grant (the
/// same trace consent_grant leaves — №335).
pub fn challenge_verify(
    challenge: &Value,
    subject: Option<&str>,
    scope: Option<&str>,
) -> Result<Value, String> {
    let fn_name = "likeness_verify";
    let id = match challenge {
        Value::LikenessChallenge(h) => h.id,
        other => {
            return Err(format!(
                "{}: challenge must be a LikenessChallenge (a String can never occupy a token position), got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    let pending = {
        let mut reg = registry()
            .lock()
            .map_err(|_| "likeness registry poisoned (challenge_verify)".to_string())?;
        // Linear consumption: remove first — a replayed challenge is
        // consumed by the FIRST verify (the GRANT_REUSED posture).
        reg.pending.remove(&id).ok_or_else(|| {
            format!(
                "{}: unknown or already-consumed challenge (LIKENESS_VERIFY_FAILED); the challenge is one-time",
                fn_name
            )
        })?
    };
    let subject = subject.unwrap_or(&pending.subject).to_string();
    let scope = scope.unwrap_or(&pending.scope).to_string();
    {
        let mut reg = registry()
            .lock()
            .map_err(|_| "likeness registry poisoned (challenge_verify)".to_string())?;
        reg.issued.insert(
            id,
            PendingChallenge {
                subject: subject.clone(),
                scope: scope.clone(),
            },
        );
    }
    // Durable trace: the consent ledger records the ritual grant (the
    // ADR-0145 four-layer ledger reuse; TTL 0 = the ledger default).
    let _ = crate::consent::record_grant(&subject, &scope, 0);
    Ok(TokenHandle { id }.to_value())
}

/// Runtime credential check (№387): the token id must be a
/// registry-issued Likeness (forged strings deserialize to id 0, which
/// is never issued — the serialization boundary cannot revive power).
pub fn token_is_issued(id: u64) -> Result<bool, String> {
    let reg = registry()
        .lock()
        .map_err(|_| "likeness registry poisoned (token_is_issued)".to_string())?;
    Ok(reg.issued.contains_key(&id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_verify_roundtrip_is_opaque() {
        let ch = challenge_issue("subject-1", "likeness", 60).expect("issue");
        assert!(matches!(ch, Value::LikenessChallenge(_)));
        let tok = challenge_verify(&ch, None, None).expect("verify succeeds");
        assert!(matches!(tok, Value::Likeness(_)));
        // Display: opaque bracketed markers, no identity leaks.
        let ch_display = format!("{}", Value::LikenessChallenge(ChallengeHandle { id: 7 }));
        assert_eq!(ch_display, "[LikenessChallenge]");
        let tok_display = format!("{}", tok);
        assert_eq!(tok_display, "[LikenessToken]");
    }

    #[test]
    fn challenge_is_one_time_fail_closed() {
        let ch = challenge_issue("subject-2", "likeness", 60).expect("issue");
        let _ = challenge_verify(&ch, None, None).expect("first verify succeeds");
        let second = challenge_verify(&ch, None, None);
        assert!(
            second.is_err(),
            "a consumed challenge must refuse (linear consumption)"
        );
        let err = second.unwrap_err();
        assert!(err.contains("LIKENESS_VERIFY_FAILED"), "typed error: {err}");
    }

    #[test]
    fn string_never_verifies_as_challenge() {
        // A forged String in the challenge position never verifies —
        // the token is unforgeable from text (the P1-7 requirement).
        let forged = Value::String("LCH#1".to_string());
        let res = challenge_verify(&forged, None, None);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("must be a LikenessChallenge"));
    }

    #[test]
    fn unknown_challenge_id_refuses() {
        let ghost = Value::LikenessChallenge(ChallengeHandle { id: 424242 });
        assert!(challenge_verify(&ghost, None, None).is_err());
    }

    #[test]
    fn serde_emits_dead_marker() {
        // The externally-tagged enum wraps the marker; the credential
        // itself carries no id/state — only the dead string.
        let tok = serde_json::to_string(&Value::Likeness(TokenHandle { id: 1 }))
            .expect("serializes as marker");
        assert!(
            tok.contains("\"[LIKENESS_TOKEN]\""),
            "dead marker only: {tok}"
        );
    }
}
