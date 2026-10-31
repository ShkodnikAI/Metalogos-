// ── Naryad №861 (wave 75 — W-D, P1, [consent][core]): the typed-approval
// consent surface ────────────────────────────────────────────────────────
//
// ADR-0205 §3: the human's consent/oversight decision recorded as a typed
// mandate — `approval_issue` (the decision becomes the carrier) and
// `approval_revoke` (the human withdraws it). The verify side is NOT a
// builtin: it is the approval GATE composed inside
// `capability::authorize_call` (the fail-closed layer on the
// human-oversight carriers — src/approval.rs; the refusal is BEFORE the
// action, both backends).
//
// The pure carrier (crate::approval::TypedApproval) does the semantics;
// these builtins are the IO seam — the wall clock is read HERE, the store
// is process-local (the №335 consent-ledger posture). The Ledger surface
// is unchanged: the gate refusals journal through the existing deny path
// (deny.IRREVERSIBLE_NO_GRANT), no new record kinds.

use crate::approval::{self, TypedApproval, APPROVAL_TYPE};
use crate::interpreter::Value;

use super::core::*;

/// The typed Struct view of a mandate (the №844 `Capability` view
/// posture): the fields ARE the carrier's — actor, target_digest,
/// granted_at, expires_at, revoked. A program can read it; a mandate's
/// AUTHORITY is never carried by the struct — the gate reads the store
/// AT CALL TIME (an untrusted cache cannot revive consent).
pub(crate) fn mandate_view(m: &TypedApproval) -> Value {
    let mut fields = std::collections::HashMap::new();
    fields.insert("actor".to_string(), Value::String(m.actor.clone()));
    fields.insert(
        "target_digest".to_string(),
        Value::String(m.target_digest.clone()),
    );
    fields.insert("granted_at".to_string(), Value::Float(m.granted_at as f64));
    fields.insert("expires_at".to_string(), Value::Float(m.expires_at as f64));
    fields.insert("revoked".to_string(), Value::Bool(m.revoked));
    Value::Struct {
        type_name: APPROVAL_TYPE.to_string(),
        fields,
    }
}

/// `approval_issue(actor, target, ttl_seconds) -> Struct<TypedApproval>` —
/// record the human's approval decision over `target` as a typed mandate.
/// `ttl_seconds` 0 = no expiry window (the consent-ledger convention).
/// The mandate covers the human-oversight carriers whose gate target
/// matches (`carrier:<name>` — the wave-75 registry transaction); the
/// gate verifies the window, the digest and the revocation AT CALL TIME.
pub(crate) fn builtin_approval_issue(args: &[Value]) -> Result<Value, String> {
    let fn_name = "approval_issue";
    if args.len() != 3 {
        return Err(format!(
            "{}: expects 3 arguments (actor, target, ttl_seconds), got {}",
            fn_name,
            args.len()
        ));
    }
    let actor = expect_string_arg(fn_name, args, 0)?;
    let target = expect_string_arg(fn_name, args, 1)?;
    let ttl = expect_ttl_arg(fn_name, args, 2)?;
    if actor.trim().is_empty() {
        return Err(format!(
            "{}: the actor must name the human who approved (non-empty String)",
            fn_name
        ));
    }
    if target.trim().is_empty() {
        return Err(format!(
            "{}: the target must name what was approved (non-empty String, e.g. \"carrier:cron_run\")",
            fn_name
        ));
    }
    let mandate = approval::mandate_issue(&actor, &target, ttl)?;
    Ok(mandate_view(&mandate))
}

/// `approval_revoke(target) -> Float` — the human withdraws the consent:
/// every LIVE mandate over `target` is marked revoked (permanent — a
/// revoked mandate never revives). Idempotent; returns the revoked count.
pub(crate) fn builtin_approval_revoke(args: &[Value]) -> Result<Value, String> {
    let fn_name = "approval_revoke";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (target), got {}",
            fn_name,
            args.len()
        ));
    }
    let target = expect_string_arg(fn_name, args, 0)?;
    let n = approval::mandate_revoke(&target)?;
    Ok(Value::Float(n as f64))
}

/// The ttl parse — the `grant_issue` form (a non-negative whole Number).
fn expect_ttl_arg(fn_name: &str, args: &[Value], idx: usize) -> Result<u64, String> {
    match args.get(idx) {
        Some(Value::Float(n)) if *n >= 0.0 && n.fract() == 0.0 => Ok(*n as u64),
        Some(other) => Err(format!(
            "{}: argument {} (ttl_seconds) must be a non-negative whole Number, got {}",
            fn_name,
            idx + 1,
            other.type_name()
        )),
        None => Err(format!(
            "{}: missing argument {} (ttl_seconds)",
            fn_name,
            idx + 1
        )),
    }
}
