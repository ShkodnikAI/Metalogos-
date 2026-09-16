// ── Compatibility profiles (Наряд №325, ADR-0161; №333, ADR-0163) ────
//
//! Program-level compatibility profiles:
//! `profile legacy { egress: permissive_with_audit }` and
//! `profile licensing { backends: permissive_with_audit }`.
//!
//! A profile is a program declaration that switches a statically-enforced
//! gate into its compatibility mode. Two profiles exist:
//!
//! - `legacy` (№325/ADR-0161): the №325 `SINK_CLEARANCE` gate runs
//!   ADVISORY — every violation becomes an audit event (Severity::Info
//!   in the audit report) instead of a compile error. Lifecycle: a
//!   MIGRATION bridge, not a residence; the audit report counts the
//!   events, so the burn-down is measurable.
//! - `licensing` (№333/ADR-0163): the backend-license gate
//!   (`BACKEND_LICENSE_DISTRIBUTION`) runs ADVISORY for non-osi /
//!   restrictive weights references — allowed, but audited as events
//!   (never silent). Distribution (the default) refuses them.
//! - `device` (№336/ADR-0165): `profile device { mode: production }` —
//!   every statically-visible `backend_select` ladder rung must be
//!   SHA-pinnable; `ShaPin::PendingNo334` backends are UNVERIFIABLE for
//!   a production profile and fail compilation (the §11.2 build-time
//!   rule, ADR-0165 §2.4). Default (absent) = development: no static
//!   ladder constraints.
//!
//! The profiles are INDEPENDENT flags: `licensing` does not weaken
//! the №325 gate, `legacy` does not unlock non-OSI backends, and
//! `device` does not touch either gate. A program may declare all three.

use crate::ast::Declaration;

/// What modes the static gates run in for one program (№325 + №333).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ResolvedProfiles {
    /// `profile legacy { egress: permissive_with_audit }` declared —
    /// the №325 sink-clearance gate reports audit events instead of
    /// compile errors.
    pub legacy_permissive_with_audit: bool,
    /// `profile licensing { backends: permissive_with_audit }` declared —
    /// the №333 backend-license gate reports audit events instead of
    /// compile errors for non-osi/restrictive weights references.
    pub backend_license_permissive_with_audit: bool,
    /// `profile device { mode: production }` declared (№336/ADR-0165) —
    /// statically-visible `backend_select` ladders may only contain
    /// SHA-pinnable rungs (`PendingNo334` = unverifiable = build error).
    pub device_mode_production: bool,
}

impl ResolvedProfiles {
    /// `true` when the №325 sink-clearance gate must NOT block compilation.
    pub fn permissive(&self) -> bool {
        self.legacy_permissive_with_audit
    }

    /// `true` when the №333 backend-license gate must NOT block
    /// compilation (audit events instead).
    pub fn backend_license_permissive(&self) -> bool {
        self.backend_license_permissive_with_audit
    }
}

/// Validate one `profile` declaration. Closed shapes: name `legacy` with
/// option `egress: permissive_with_audit` (№325); name `licensing` with
/// option `backends: permissive_with_audit` (№333). Anything else is a
/// loud error (unknown words are compat-profile mistakes, not silent
/// no-ops).
pub fn validate(p: &crate::ast::ProfileDecl) -> Result<(), String> {
    match p.name.as_str() {
        "legacy" => {
            validate_options(p, "egress", &["permissive_with_audit"])?;
            Ok(())
        }
        "licensing" => {
            validate_options(p, "backends", &["permissive_with_audit"])?;
            Ok(())
        }
        "device" => {
            validate_options(p, "mode", &["production", "development"])?;
            Ok(())
        }
        other => Err(format!(
            "unknown compatibility profile '{}' (available: legacy, licensing, device)",
            other
        )),
    }
}

fn validate_options(
    p: &crate::ast::ProfileDecl,
    key: &str,
    allowed: &[&str],
) -> Result<(), String> {
    if p.options.is_empty() {
        return Err(format!(
            "profile '{}' requires options ({}: {})",
            p.name, key, allowed[0]
        ));
    }
    for (k, v) in &p.options {
        if k != key {
            return Err(format!(
                "unknown profile option '{k}' for '{}' (available: {key})",
                p.name
            ));
        }
        if !allowed.contains(&v.as_str()) {
            return Err(format!(
                "unknown {} mode '{v}' (available: {})",
                key,
                allowed.join(", ")
            ));
        }
    }
    Ok(())
}

/// Resolve the program's profiles from its declarations. Each profile
/// flag is set when its declaration is present (the LAST matching
/// declaration wins — re-declaration is a documented override). Unknown
/// profile names or option values are loud semantic errors (validated in
/// `semantic::check_program`) — here an unknown shape simply does not
/// switch the mode away from the default.
pub fn resolve(declarations: &[Declaration]) -> ResolvedProfiles {
    let mut resolved = ResolvedProfiles::default();
    for decl in declarations {
        if let Declaration::Profile(p) = decl {
            match p.name.as_str() {
                "legacy" => {
                    resolved.legacy_permissive_with_audit = p
                        .options
                        .iter()
                        .any(|(k, v)| k == "egress" && v == "permissive_with_audit");
                }
                "licensing" => {
                    resolved.backend_license_permissive_with_audit = p
                        .options
                        .iter()
                        .any(|(k, v)| k == "backends" && v == "permissive_with_audit");
                }
                "device" => {
                    resolved.device_mode_production = p
                        .options
                        .iter()
                        .any(|(k, v)| k == "mode" && v == "production");
                }
                _ => {}
            }
        }
    }
    resolved
}
