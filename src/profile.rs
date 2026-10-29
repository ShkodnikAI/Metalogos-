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
//!   №839 (ADR-0202 §2.3, wave 71 — W-A) adds the NAMED class permit:
//!   `profile licensing { allow_license_class: "non-osi" }` (repeatable;
//!   the ADR-0163 vocabulary `osi` / `non-osi` / `restrictive`). The
//!   program-level selection rule (`LICENSE_CLASS_NOT_ALLOWED`, the
//!   semantic lane) refuses a backend whose license class is NOT
//!   permitted: absence of the key = only `Osi` is allowed. The named
//!   permit is the MINIMAL unlock (per class); the advisory bridge stays
//!   the blanket unlock it always was (implied by `license_allows`).
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
use crate::backends::LicenseClass;

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
    /// `profile duty { materialization: denied }` declared (№348 /
    /// ADR-0172) — the static carrier for the №349 rule that makes
    /// private-handle materialization a compile error in duty mode.
    /// №348 lands the carrier; enforcement is №349.
    pub duty_materialization_denied: bool,
    /// `profile duty { surfaces: local_only }` declared (№348 /
    /// ADR-0172) — the static carrier for the №349 rule that makes
    /// network sinks compile errors in duty mode.
    pub duty_surfaces_local_only: bool,
    /// №839 (ADR-0202 §2.3): the NAMED license-class permits of the
    /// `licensing` profile — `allow_license_class: "non-osi"` and
    /// `allow_license_class: "restrictive"` (each repeatable; `osi` is
    /// always allowed and needs no key). Absence = only `Osi` for the
    /// program's backend selections (the default-deny posture, named).
    pub license_allow_nonosi: bool,
    pub license_allow_restrictive: bool,
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

    /// №839 (ADR-0202 §2.3): the license-class permit predicate — the
    /// SINGLE decision point shared by the selection rule (the semantic
    /// lane, `LICENSE_CLASS_NOT_ALLOWED`) and the №333 audit gate (the
    /// distribution posture). `Osi` is unconditionally allowed; the
    /// named permits unlock their class; the advisory bridge
    /// (`backends: permissive_with_audit`) implies the blanket unlock it
    /// always carried (the ADR-0163 surface, unchanged — backward
    /// compatibility). The default posture (no licensing profile)
    /// allows `Osi` only.
    pub fn license_allows(&self, class: LicenseClass) -> bool {
        match class {
            LicenseClass::Osi => true,
            LicenseClass::NonOsi => {
                self.license_allow_nonosi || self.backend_license_permissive_with_audit
            }
            LicenseClass::Restrictive => {
                self.license_allow_restrictive || self.backend_license_permissive_with_audit
            }
        }
    }
}

/// Validate one `profile` declaration. Closed shapes: name `legacy` with
/// option `egress: permissive_with_audit` (№325); name `licensing` with
/// option `backends: permissive_with_audit` (№333); name `device` with
/// `mode: production|development` (№336); name `duty` (№348/ADR-0172)
/// with `materialization: denied` and/or `surfaces: local_only` — the
/// static carrier of the session duty profile; the №349 compiler rule
/// keys on the resolved flags. Anything else is a loud error (unknown
/// words are compat-profile mistakes, not silent no-ops).
pub fn validate(p: &crate::ast::ProfileDecl) -> Result<(), String> {
    match p.name.as_str() {
        "legacy" => {
            validate_options(p, "egress", &["permissive_with_audit"])?;
            Ok(())
        }
        "licensing" => {
            // №333 (the bridge) + №839 (the named class permits): two
            // option families on one profile. At least one option is
            // required — a licensing declaration without either is a
            // no-op mistake, and no-ops must be loud (the §16.0 honesty
            // rule). The class words are the ADR-0163 vocabulary
            // (`as_str`); the IDENT spelling `non_osi` is the accepted
            // compat form of the hyphenated canonical.
            if p.options.is_empty() {
                return Err(
                    "profile 'licensing' requires options (backends: permissive_with_audit; allow_license_class: osi|non-osi|restrictive)"
                        .to_string(),
                );
            }
            for (k, v) in &p.options {
                match (k.as_str(), v.as_str()) {
                    ("backends", "permissive_with_audit") => {}
                    ("backends", other) => {
                        return Err(format!(
                            "unknown licensing-profile mode '{other}' for 'backends' (available: permissive_with_audit)",
                        ));
                    }
                    ("allow_license_class", word) => {
                        if normalize_license_class(word).is_none() {
                            return Err(format!(
                                "unknown license class '{word}' for 'allow_license_class' (available: osi, non-osi, restrictive — the ADR-0163 vocabulary)",
                            ));
                        }
                    }
                    (other, _) => {
                        return Err(format!(
                            "unknown profile option '{other}' for 'licensing' (available: backends, allow_license_class)",
                        ));
                    }
                }
            }
            Ok(())
        }
        "device" => {
            validate_options(p, "mode", &["production", "development"])?;
            Ok(())
        }
        "duty" => {
            // Two independent knobs (ADR-0172 §5): materialization of
            // private handles and network surfaces. At least one is
            // required — a duty declaration without either is a no-op
            // mistake, and no-ops must be loud (the §16.0 honesty rule).
            if p.options.is_empty() {
                return Err(
                    "profile 'duty' requires options (materialization: denied, surfaces: local_only)"
                        .to_string(),
                );
            }
            for (k, v) in &p.options {
                match (k.as_str(), v.as_str()) {
                    ("materialization", "denied") => {}
                    ("surfaces", "local_only") => {}
                    ("materialization", other) | ("surfaces", other) => {
                        return Err(format!(
                            "unknown duty-profile mode '{}' for option '{}' (available: materialization: denied; surfaces: local_only)",
                            other, k
                        ));
                    }
                    (other, _) => {
                        return Err(format!(
                            "unknown profile option '{}' for 'duty' (available: materialization, surfaces)",
                            other
                        ));
                    }
                }
            }
            Ok(())
        }
        other => Err(format!(
            "unknown compatibility profile '{}' (available: legacy, licensing, device, duty)",
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
                    // №333: the audited blanket bridge (unchanged).
                    if p.options
                        .iter()
                        .any(|(k, v)| k == "backends" && v == "permissive_with_audit")
                    {
                        resolved.backend_license_permissive_with_audit = true;
                    }
                    // №839 (ADR-0202 §2.3): the named class permits — each
                    // `allow_license_class` option names ONE class; the
                    // repeatable spelling is the multi-class permit. `osi`
                    // is explicitly legal and redundant (always allowed).
                    for (k, v) in p.options.iter().filter(|(k, _)| k == "allow_license_class") {
                        match normalize_license_class(v) {
                            Some(LicenseClass::NonOsi) => resolved.license_allow_nonosi = true,
                            Some(LicenseClass::Restrictive) => {
                                resolved.license_allow_restrictive = true;
                            }
                            _ => {}
                        }
                    }
                }
                "device" => {
                    resolved.device_mode_production = p
                        .options
                        .iter()
                        .any(|(k, v)| k == "mode" && v == "production");
                }
                "duty" => {
                    // №348/ADR-0172: the duty-profile carrier. Each knob
                    // resolves independently; the №349 compile rule keys
                    // on these flags (materialization of private
                    // handles / network sinks).
                    if p.options
                        .iter()
                        .any(|(k, v)| k == "materialization" && v == "denied")
                    {
                        resolved.duty_materialization_denied = true;
                    }
                    if p.options
                        .iter()
                        .any(|(k, v)| k == "surfaces" && v == "local_only")
                    {
                        resolved.duty_surfaces_local_only = true;
                    }
                }
                _ => {}
            }
        }
    }
    resolved
}

/// №839: the canonical license-class word — the ADR-0163 vocabulary
/// (`LicenseClass::as_str`); the IDENT spelling `non_osi` is the
/// accepted compat form of the hyphenated canonical (the STRING form
/// `"non-osi"` is the primary spelling, the parser carries both).
fn normalize_license_class(word: &str) -> Option<LicenseClass> {
    match word {
        "osi" => Some(LicenseClass::Osi),
        "non-osi" | "non_osi" => Some(LicenseClass::NonOsi),
        "restrictive" => Some(LicenseClass::Restrictive),
        _ => None,
    }
}
