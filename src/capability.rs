// ── Naryad №844 (wave 72 — W-B, P1, [core][security]): Capability-Token ──
//
//! The typed capability carrier (ADR-0203 §2.1): the language-visible,
//! read-only VIEW over a grant — the single consumption contract for an
//! irreversible action. The grant ledger (src/grants.rs) stays the SSOT
//! behind every field; the token is an UNTRUSTED CACHE of the ledger
//! record (the ADR-0155 handle posture kept verbatim), so a stale token
//! cannot revive power: every authority check reads the ledger at use
//! time, and consume happens ONLY through the single points
//! (`grant_use` — the explicit consume, and the carrier gates' own
//! post-success consumption, e.g. `db_execute_with_grant`).
//!
//! The language surface is a `Value::Struct` with `type_name
//! "Capability"` (the №838 ModelPassport typed-artifact pattern; the
//! DenyEvent/ForecastState runtime-Struct precedent):
//!
//!   Capability { scope, class, ttl, quota, revoked, grant }
//!     scope   String  — the grant scope (`db:delete:sessions`, …)
//!     class   String  — the canonical ledger form "once"|"n:<k>"|"unlimited"
//!     ttl     Float   — the expiry instant (unix seconds)
//!     quota   Float   — the remaining uses at MINT (-1 = unlimited); the
//!                       LIVE state reads through `capability_state`
//!     revoked Bool    — mint-time state (always false at mint; the live
//!                       view is `capability_state`)
//!     grant   Value::Grant — the opaque authority handle (non-printable,
//!                       serde dead marker; a program cannot forge one —
//!                       only `grant_issue`/`grant_subgrant` mint it)
//!
//! Migration 1:1 (the №838 pattern): `grant_issue`/`grant_subgrant` now
//! return the typed Capability, and EVERY grant-argument extractor in the
//! tree accepts BOTH forms (the opaque `Value::Grant` and the Capability
//! view) through `grant_from_value` — existing programs keep working
//! byte-for-byte.

use crate::grants::{GrantClass, GrantHandle};
use crate::interpreter::values::Value;

/// The Struct type_name of the typed capability carrier.
pub const CAPABILITY_TYPE: &str = "Capability";

/// The Struct type_name of the LIVE capability-state view.
pub const CAPABILITY_STATE_TYPE: &str = "CapabilityState";

/// Build the mint-time Capability view over a freshly minted handle.
/// `remaining` is the ledger row's remaining uses at mint (-1 = unlimited).
pub fn mint_view(handle: &GrantHandle, remaining: i64) -> Value {
    let mut fields = std::collections::HashMap::new();
    fields.insert("scope".to_string(), Value::String(handle.scope.clone()));
    fields.insert(
        "class".to_string(),
        Value::String(handle.class.as_ledger_str()),
    );
    fields.insert("ttl".to_string(), Value::Float(handle.expires_at as f64));
    fields.insert("quota".to_string(), Value::Float(remaining as f64));
    fields.insert("revoked".to_string(), Value::Bool(false));
    fields.insert("grant".to_string(), handle.to_value());
    Value::Struct {
        type_name: CAPABILITY_TYPE.to_string(),
        fields,
    }
}

/// True when the value is the typed Capability carrier.
pub fn is_capability(v: &Value) -> bool {
    matches!(v, Value::Struct { type_name, .. } if type_name == CAPABILITY_TYPE)
}

/// The shared grant-argument extractor (the migration 1:1 seam): accepts
/// the opaque `Value::Grant` (the pre-№844 form) AND the typed Capability
/// view (its `grant` field). A forged Struct without a real Grant handle
/// inside refuses — programs cannot construct `Value::Grant` (only the
/// mint builtins produce it), and a deserialized Capability carries the
/// dead `"[GRANT]"` String marker, which is not a handle (fail-closed).
pub fn grant_from_value(v: &Value) -> Option<GrantHandle> {
    match v {
        Value::Grant(h) => Some(h.clone()),
        Value::Struct { type_name, fields } if type_name == CAPABILITY_TYPE => {
            match fields.get("grant") {
                Some(g @ Value::Grant(_)) => grant_from_value(g),
                _ => None,
            }
        }
        _ => None,
    }
}

/// The LIVE typed view over the ledger record (`capability_state(c)`):
/// every field is read from the grant ledger AT CALL TIME — the SSOT
/// posture of ADR-0203 §2.1. Refuses `GRANT_MISSING` for an unknown id
/// (fail-closed honesty: a dead token does not fabricate a state).
pub fn state_view(handle: &GrantHandle) -> Result<Value, String> {
    let rec = crate::grants::record_of(&handle.grant_id)?;
    let mut fields = std::collections::HashMap::new();
    fields.insert("scope".to_string(), Value::String(rec.scope.clone()));
    fields.insert(
        "class".to_string(),
        Value::String(rec.class.as_ledger_str()),
    );
    fields.insert("ttl".to_string(), Value::Float(rec.expires_at as f64));
    fields.insert("quota".to_string(), Value::Float(rec.remaining as f64));
    fields.insert("revoked".to_string(), Value::Bool(rec.state == "revoked"));
    fields.insert("state".to_string(), Value::String(rec.state.clone()));
    Ok(Value::Struct {
        type_name: CAPABILITY_STATE_TYPE.to_string(),
        fields,
    })
}

/// The mint-time remaining uses for a class (the ledger row's initial
/// `remaining`; -1 = unlimited). Mirrors `GrantClass::initial_remaining`
/// (kept in sync by the grants.rs unit test).
pub fn initial_remaining(class: &GrantClass) -> i64 {
    match class {
        GrantClass::Once => 1,
        GrantClass::N(k) => *k as i64,
        GrantClass::Unlimited => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grants;

    #[test]
    fn mint_view_carries_the_adr_fields() {
        let h = grants::issue("db:delete:users", 3600, &GrantClass::N(3), "t").unwrap();
        let v = mint_view(&h, 3);
        match &v {
            Value::Struct { type_name, fields } => {
                assert_eq!(type_name, "Capability");
                for key in ["scope", "class", "ttl", "quota", "revoked", "grant"] {
                    assert!(fields.contains_key(key), "field {key} missing");
                }
                assert_eq!(fields.get("scope").unwrap().type_name(), "String");
                assert_eq!(fields.get("class").unwrap().type_name(), "String");
                assert_eq!(fields.get("quota").unwrap().type_name(), "Float");
                assert_eq!(fields.get("revoked").unwrap().type_name(), "Bool");
                assert_eq!(fields.get("grant").unwrap().type_name(), "Grant");
            }
            other => panic!("expected a Capability struct, got {other:?}"),
        }
    }

    #[test]
    fn grant_from_value_accepts_both_forms_and_refuses_forgery() {
        let h = grants::issue("db:delete:users", 60, &GrantClass::Once, "t").unwrap();
        // The opaque form (pre-№844) still extracts.
        assert!(grant_from_value(&h.to_value()).is_some());
        // The typed Capability view extracts the same handle.
        let cap = mint_view(&h, 1);
        let extracted = grant_from_value(&cap).unwrap();
        assert_eq!(extracted.grant_id, h.grant_id);
        // A forged Struct without a real handle refuses.
        let mut fields = std::collections::HashMap::new();
        fields.insert("grant".to_string(), Value::String("[GRANT]".to_string()));
        let forged = Value::Struct {
            type_name: CAPABILITY_TYPE.to_string(),
            fields,
        };
        assert!(grant_from_value(&forged).is_none());
        // A deserialized Capability carries the tombstone handle (the
        // serde dead marker reconstructs the expired-on-arrival Grant —
        // the ADR-0155 posture): the SEAM extracts it, but every USE
        // refuses at the ledger — power never revives.
        let json = serde_json::to_string(&cap).unwrap();
        assert!(json.contains("\"[GRANT]\""), "serde dead marker: {json}");
        let back: Value = serde_json::from_str(&json).unwrap();
        let tomb =
            grant_from_value(&back).expect("the tombstone extracts — the ADR-0155 serde posture");
        let err = grants::grant_use(&tomb, "revived").unwrap_err();
        assert!(
            err.starts_with("GRANT_"),
            "a deserialized capability must refuse every use, got: {err}"
        );
    }

    #[test]
    fn state_view_is_live_and_fail_closed() {
        let h = grants::issue("db:delete:users", 60, &GrantClass::N(2), "t").unwrap();
        let v = state_view(&h).unwrap();
        match &v {
            Value::Struct { type_name, fields } => {
                assert_eq!(type_name, "CapabilityState");
                assert_eq!(
                    fields.get("quota").unwrap().type_name(),
                    "Float",
                    "quota is a live ledger read"
                );
                assert_eq!(fields.get("state").unwrap().type_name(), "String");
            }
            other => panic!("expected CapabilityState, got {other:?}"),
        }
        // Consume through the SINGLE point → the view reflects the ledger.
        grants::grant_use(&h, "t").unwrap();
        let v2 = state_view(&h).unwrap();
        match &v2 {
            Value::Struct { fields, .. } => match fields.get("quota") {
                Some(Value::Float(q)) => assert_eq!(*q, 1.0, "live quota decremented"),
                other => panic!("quota must be Float, got {other:?}"),
            },
            _ => unreachable!(),
        }
        // Revoke → the live view flips.
        grants::revoke(&h, "t").unwrap();
        let v3 = state_view(&h).unwrap();
        match &v3 {
            Value::Struct { fields, .. } => {
                assert!(matches!(fields.get("revoked"), Some(Value::Bool(true))));
                assert!(matches!(
                    fields.get("state"),
                    Some(Value::String(s)) if s == "revoked"
                ));
            }
            _ => unreachable!(),
        }
        // An unknown id refuses (fail-closed, no fabricated state).
        let ghost = GrantHandle {
            grant_id: "g-does-not-exist".to_string(),
            scope: "db:*:*".to_string(),
            class: GrantClass::Once,
            expires_at: grants::now_secs() + 60,
            issuer: "t".to_string(),
        };
        let err = state_view(&ghost).unwrap_err();
        assert!(err.starts_with("GRANT_MISSING"), "got: {err}");
    }

    #[test]
    fn initial_remaining_matches_the_ledger_rule() {
        assert_eq!(initial_remaining(&GrantClass::Once), 1);
        assert_eq!(initial_remaining(&GrantClass::N(7)), 7);
        assert_eq!(initial_remaining(&GrantClass::Unlimited), -1);
    }
}
