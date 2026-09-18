// ── Naryad #387 (P2, security/media): the likeness ritual surface ─────
//
// Language surface of the ADR-0149 D1/D6 LikenessToken (the consent
// ledger + registry mechanics live in `src/likeness.rs`):
//
//   - `likeness_challenge(subject, scope?, ttl?)` — issues the one-time
//     opaque challenge (`Value::LikenessChallenge`).
//   - `likeness_verify(challenge, subject?, scope?)` — consumes the
//     challenge and returns the opaque token (`Value::Likeness`).
//     Fail-closed on unknown/consumed challenges (LIKENESS_VERIFY_FAILED);
//     a String in the challenge position never verifies (the P1-7
//     unforgeability contract). A successful verify records the grant
//     in the consent ledger (the №335 trace).
//
// Both builtins are process-local bookkeeping (Role::Lift, no egress).

use crate::interpreter::values::Value;

/// `likeness_challenge(subject, scope?, ttl_seconds?)` — issue a
/// one-time likeness challenge (ADR-0149 D1).
pub(crate) fn builtin_likeness_challenge(args: &[Value]) -> Result<Value, String> {
    let fn_name = "likeness_challenge";
    if args.is_empty() || args.len() > 3 {
        return Err(format!(
            "{}: expects 1..3 arguments (subject, scope?, ttl_seconds?), got {}",
            fn_name,
            args.len()
        ));
    }
    let subject = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "{}: subject must be String, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    let scope = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        None => "likeness".to_string(),
        Some(other) => {
            return Err(format!(
                "{}: scope must be String, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    let ttl = match args.get(2) {
        Some(Value::Float(f)) if *f >= 0.0 => *f as u64,
        None => 0,
        Some(other) => {
            return Err(format!(
                "{}: ttl_seconds must be a non-negative Float, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    crate::likeness::challenge_issue(&subject, &scope, ttl)
}

/// `likeness_verify(challenge, subject?, scope?)` — consume the
/// challenge, record the consent-ledger grant, return the opaque token.
pub(crate) fn builtin_likeness_verify(args: &[Value]) -> Result<Value, String> {
    let fn_name = "likeness_verify";
    if args.is_empty() || args.len() > 3 {
        return Err(format!(
            "{}: expects 1..3 arguments (challenge, subject?, scope?), got {}",
            fn_name,
            args.len()
        ));
    }
    let subject = match args.get(1) {
        Some(Value::String(s)) => Some(s.as_str()),
        None => None,
        Some(other) => {
            return Err(format!(
                "{}: subject must be String, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    let scope = match args.get(2) {
        Some(Value::String(s)) => Some(s.as_str()),
        None => None,
        Some(other) => {
            return Err(format!(
                "{}: scope must be String, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    crate::likeness::challenge_verify(&args[0], subject, scope)
}
