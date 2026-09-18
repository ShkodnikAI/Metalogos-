// ── Наряд №335 (spec §7.2 v2): consent grant/revoke + quarantine sink ──
//
// The consent component of the label (ADR-0154) gets its language
// surface. Design decisions (loud, per the naryad):
//   - Builtins, NOT new AST nodes — the redact precedent ("policy as
//     value", №326): the flow layer special-cases the call NAMES in
//     `label_source`, the runtime handlers record the ledger.
//   - `consent_grant(value, scope, subject?, ttl_seconds?)` — records
//     (subject, scope, TTL) in the ledger and passes the value through;
//     statically the value's consent-scope set is EXTENDED by the scope
//     (semantic.rs label_source; non-literal scope = conservative
//     no-extension, the redact dynamic-policy posture).
//   - `consent_revoke(value, scope?)` — records the revocation and
//     returns the value under the QUARANTINE label (conf: poisoned,
//     integrity: untrusted, consent: ∅). Poison is ABSORBING in the
//     lattice (ADR-0154 §2.1): every value derived from the revoked one
//     is quarantined by the lattice's own join — the flat cascade is
//     the lattice, not a separate analysis.
//   - `quarantine_write(value, reason?)` — THE only legal egress for a
//     poisoned value, legal with an unconditional audit event
//     (QUARANTINE_EGRESS, Info; the №326 unconditional-event posture).
//     Every other sink refuses poisoned (the №325 clearance).
//   - `consent_ledger_export(path)` — the ledger is exported as JSON to
//     a sandboxed path: FILE EGRESS, classified Sink, audit event.

use crate::interpreter::values::Value;

/// `consent_grant(value, scope, subject?, ttl_seconds?)` — record the
/// grant in the ledger; the value passes through with its consent scope
/// extended (static: semantic.rs label_source).
pub(crate) fn builtin_consent_grant(args: &[Value]) -> Result<Value, String> {
    let fn_name = "consent_grant";
    if args.len() < 2 || args.len() > 4 {
        return Err(format!(
            "{}: expects 2..4 arguments (value, scope, subject?, ttl_seconds?), got {}",
            fn_name,
            args.len()
        ));
    }
    let scope = match &args[1] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "{}: scope must be String, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    let subject = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        None => "unspecified".to_string(),
        Some(other) => {
            return Err(format!(
                "{}: subject must be String, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    let ttl_seconds = match args.get(3) {
        None => 0,
        Some(Value::Float(f)) if *f >= 0.0 => *f as u64,
        Some(other) => {
            return Err(format!(
                "{}: ttl_seconds must be a non-negative Float, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    crate::consent::record_grant(&subject, &scope, ttl_seconds)?;
    Ok(args[0].clone())
}

/// `consent_revoke(value, scope?)` — record the revocation (scope or
/// ALL); the value is returned under the quarantine label — the flat
/// cascade poisons every derivative through lattice absorption.
pub(crate) fn builtin_consent_revoke(args: &[Value]) -> Result<Value, String> {
    let fn_name = "consent_revoke";
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "{}: expects 1..2 arguments (value, scope?), got {}",
            fn_name,
            args.len()
        ));
    }
    let scope = match args.get(1) {
        None => None,
        Some(Value::String(s)) => Some(s.as_str()),
        Some(other) => {
            return Err(format!(
                "{}: scope must be String, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    crate::consent::record_revoke(scope)?;
    Ok(args[0].clone())
}

/// `quarantine_write(value, reason?)` — the quarantine sink: the ONLY
/// legal egress for a poisoned value. Returns the audit-event text (the
/// program-visible half of the event; the static half is the
/// QUARANTINE_EGRESS audit finding + stderr line, №326 posture).
pub(crate) fn builtin_quarantine_write(args: &[Value]) -> Result<Value, String> {
    let fn_name = "quarantine_write";
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "{}: expects 1..2 arguments (value, reason?), got {}",
            fn_name,
            args.len()
        ));
    }
    let reason = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        None => "unspecified".to_string(),
        Some(other) => {
            return Err(format!(
                "{}: reason must be String, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    Ok(Value::String(format!(
        "[QUARANTINE_EGRESS] poisoned value reached the quarantine sink (legal egress with audit event, №335; reason: {})",
        reason
    )))
}

/// `consent_ledger_export(path)` — dump the ledger as JSON to a
/// sandboxed path (FILE EGRESS — classified Sink, audited). Returns the
/// written path.
pub(crate) fn builtin_consent_ledger_export(args: &[Value]) -> Result<Value, String> {
    let fn_name = "consent_ledger_export";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (path), got {}",
            fn_name,
            args.len()
        ));
    }
    let path = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "{}: path must be String, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    let json = crate::consent::export_json()?;
    let safe_path =
        crate::builtins::io::sandbox_path_ex(&path, crate::builtins::io::SandboxMode::ForWrite)
            .map_err(crate::builtins::io::sandbox_violation)?;
    std::fs::write(&safe_path, json.as_bytes())
        .map_err(|e| format!("{}: cannot write {}: {}", fn_name, safe_path.display(), e))?;
    Ok(Value::String(safe_path.display().to_string()))
}
