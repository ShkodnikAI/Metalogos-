// ── Compatibility profile (Наряд №325, ADR-0161) ─────────────────────
//
//! Program-level compatibility profiles: `profile legacy { egress:
//! permissive_with_audit }`.
//!
//! A profile is a program declaration that switches statically-enforced
//! security gates into their compatibility mode. The only profile in
//! this slice is `legacy` with `egress: permissive_with_audit`: the
//! №325 `SINK_CLEARANCE` gate runs ADVISORY — every violation becomes
//! an audit event (Severity::Info in the audit report, an `[SINK_
//! CLEARANCE][audit-event]` line on the compile/run stderr) instead of
//! a compile error.
//!
//! Lifecycle (ADR-0161): `legacy` is a MIGRATION bridge, not a residence.
//! Its exit criterion is per-program: the profile declaration is removed
//! when the program's flows are either annotated/redacted to pass the
//! strict gate or the flows are dead. The audit report counts the events
//! (see `audit_events`), so the burn-down is measurable.

use crate::ast::Declaration;

/// What mode the №325 sink-clearance gate runs in for a program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileMode {
    /// No profile declared — all gates STRICT (default).
    Strict,
    /// `profile legacy { egress: permissive_with_audit }` — the gate
    /// reports audit events instead of compile errors.
    LegacyPermissiveWithAudit,
}

impl ProfileMode {
    /// `true` when the sink-clearance gate must NOT block compilation.
    pub fn permissive(&self) -> bool {
        matches!(self, ProfileMode::LegacyPermissiveWithAudit)
    }
}

/// Validate one `profile` declaration. The closed shape in this slice:
/// name `legacy`, options `egress: permissive_with_audit`. Anything
/// else is a loud error (unknown words are compat-profile mistakes,
/// not silent no-ops).
pub fn validate(p: &crate::ast::ProfileDecl) -> Result<(), String> {
    if p.name != "legacy" {
        return Err(format!(
            "unknown compatibility profile '{}' (available: legacy)",
            p.name
        ));
    }
    for (k, v) in &p.options {
        if k != "egress" {
            return Err(format!("unknown profile option '{k}' (available: egress)"));
        }
        if v != "permissive_with_audit" {
            return Err(format!(
                "unknown egress mode '{v}' (available: permissive_with_audit)"
            ));
        }
    }
    Ok(())
}

/// Resolve the program's profile mode from its declarations.
/// The LAST `profile` declaration wins (a program may declare one;
/// re-declaration is a documented override). Unknown profile names or
/// option values are loud semantic errors (validated in
/// `semantic::check_program`) — here an unknown shape simply does not
/// switch the mode away from `Strict`.
pub fn resolve(declarations: &[Declaration]) -> ProfileMode {
    let mut mode = ProfileMode::Strict;
    for decl in declarations {
        if let Declaration::Profile(p) = decl {
            if p.name == "legacy"
                && p.options
                    .iter()
                    .any(|(k, v)| k == "egress" && v == "permissive_with_audit")
            {
                mode = ProfileMode::LegacyPermissiveWithAudit;
            }
        }
    }
    mode
}
