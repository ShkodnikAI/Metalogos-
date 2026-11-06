// ── src/vertical.rs ──────────────────────────────────────────────────
// №921 (issue #1840, wave 84 — W-L2, ADR-0220 §2): the vertical-profiles
// kernel (the 8H-2/C13а kernel). A vertical profile is a TYPED
// CONFIGURATION OF LIMITS AND MANDATORY CONTOURS — never a
// certification, never legal advice, never a regulator's acceptance
// (the C10 frame, ADR-0209 §5): the profile carries the SIX mandatory
// Phase-8 contours (jurisdiction coverage, human oversight/approval,
// decision-trace verify, the taint floor, the license class, the
// frozen refusal vocabulary as the version-pin) and a carrier digest
// pin (the G5/№828 form — SHA-256 over the canonical serialization of
// the profile's own fields), and the registry is append-only (the
// JurisdictionMatrix discipline, №888).
//
// The consumption is fail-closed at the verdict site (ADR-0220 §2, the
// №874/№845/№911 form): a package declaring a vertical (mlog.toml
// `[vertical]`: id + digest, the №910 [registry] shape) with an
// unknown id, a digest mismatch or an unverifiable contour refuses
// BEFORE any effect with the divergence NAMED; no default attach —
// the absence of the declaration NEVER attaches a profile (the
// negative control: a manifest without `[vertical]` verifies with
// ZERO behavioral mutations).
//
// The dedup guard (№921): the type names are `VerticalProfile` /
// `VerticalId` / `vertical_verdict` — apart from every existing
// surface (`src/profile.rs` is the session duty profile, №349/№281 —
// untouched; «vertical» in the tree was only the SVG layout
// vocabulary). The parents are consumed by reference (ADR-0220 §3):
// the profile REFERENCES the six contours, never re-derives, never
// widens a right (ADR-0203 stays the rights SSOT), never adds a
// second egress gate (ADR-0207 stays the egress SSOT), never touches
// the frozen refusal vocabulary (№385 — the version-pin references
// it). The refusal rides the named-divergence string form of the
// package-integrity gate (№911) — the packaged path composes it
// AFTER the integrity pass (a tampered package refuses as INTEGRITY
// first, ADR-0218 §4).
//
// The roster literals (the gate_045 machine source): each seed
// function (seed_payments / seed_public_sector) carries the
// PROFILE-SCOPED contour anchor literals (the roster needles) next to
// the typed contour variants (the consumption sub-anchors) — the
// gate_040 form: the check pins the quoted literals INSIDE the seed
// region, not a call shape; a removed seeding un-checks its six rows
// (the tamper traps of gen_metrics prove each class red).

use sha2::{Digest, Sha256};

/// The canonical separator (the compliance/canonical-row form of
/// №828/№868 — the same `\x1f` byte, the same determinism contract).
const SEP: char = '\u{1f}';

/// The typed vertical identifier (ADR-0220 §2): the two verticals of
/// R38(5) — `payments` and `public-sector`. Medicine is BACKLOG until
/// the PHI expertise exists (the owner decision, R38(5)) — it is NOT
/// a variant here, and the refusal for an unknown id names the two
/// live ids (the dispatch gh#1842 roster; the fail-closed form: the
/// lowercase ids are the contract, a typo refuses).
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum VerticalId {
    Payments,
    PublicSector,
}

impl VerticalId {
    /// The registry id string (the mlog.toml `[vertical] id` form).
    pub fn as_str(&self) -> &'static str {
        match self {
            VerticalId::Payments => "payments",
            VerticalId::PublicSector => "public-sector",
        }
    }

    /// Parse the declared id (the fail-closed consumption entry: an
    /// unknown id refuses HERE — «unknown vertical id» — the named
    /// divergence of ADR-0220 §2; no fuzzy match, no default).
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "payments" => Ok(VerticalId::Payments),
            "public-sector" => Ok(VerticalId::PublicSector),
            _ => Err(format!(
                "vertical: unknown vertical id '{s}' — the registry carries payments, public-sector (ADR-0220 §2; medicine is backlog until the PHI expertise)"
            )),
        }
    }
}

/// The six mandatory Phase-8 contour anchors (ADR-0220 §2): a profile
/// without the FULL six does not construct (the fail-closed
/// constructor form №879/№888). Each contour is a REFERENCE to the
/// live parent module + the attestation requirement the consumer must
/// satisfy — never a re-derivation (ADR-0220 §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VerticalContour {
    /// (1) jurisdiction coverage — the JurisdictionMatrix package
    /// (ADR-0209, strictest-wins).
    JurisdictionCoverage,
    /// (2) human oversight / approval for irreversible — the typed
    /// approval mandate (ADR-0205, the APPROVAL_COVERAGE contour).
    HumanOversightApproval,
    /// (3) decision-trace verify — the replay/forensic ledger
    /// (ADR-0210, the REPLAY_MATCH contour).
    DecisionTraceVerify,
    /// (4) the taint floor — the taint saturation lane (ADR-0217, the
    /// fuzz_taint_triples baseline).
    TaintFloor,
    /// (5) the license class of the backends — the license-typed model
    /// layer (ADR-0202, the №841 LICENSED_FLOOR contour).
    LicenseClass,
    /// (6) the refusal vocabulary — the frozen №385 dictionary, the
    /// VERSION-PIN (the profile names the vocabulary generation it
    /// was attested against).
    RefusalVocabularyPin,
}

impl VerticalContour {
    /// The canonical contour name (the anchor's middle form).
    pub fn name(&self) -> &'static str {
        match self {
            VerticalContour::JurisdictionCoverage => "jurisdiction-coverage",
            VerticalContour::HumanOversightApproval => "human-oversight-approval",
            VerticalContour::DecisionTraceVerify => "decision-trace-verify",
            VerticalContour::TaintFloor => "taint-floor",
            VerticalContour::LicenseClass => "license-class",
            VerticalContour::RefusalVocabularyPin => "refusal-vocabulary-pin",
        }
    }

    /// The parents the contour consumes BY REFERENCE (the ADR-0220
    /// §3 discipline — the anchor's tail form).
    pub fn parents(&self) -> &'static str {
        match self {
            VerticalContour::JurisdictionCoverage => {
                "the JurisdictionMatrix package, strictest-wins (ADR-0209)"
            }
            VerticalContour::HumanOversightApproval => {
                "the typed approval mandate (ADR-0205)"
            }
            VerticalContour::DecisionTraceVerify => "the replay/forensic ledger (ADR-0210)",
            VerticalContour::TaintFloor => "the taint saturation lane (ADR-0217)",
            VerticalContour::LicenseClass => "the license-typed model layer (ADR-0202)",
            VerticalContour::RefusalVocabularyPin => {
                "the frozen №385 dictionary, the version-pin"
            }
        }
    }
}

/// The canonical PROFILE-SCOPED anchor (the gate_045 roster's needle
/// form): `<id> contour (<position>) <name> — <parents>`. The seed
/// literals must equal the canonical derivation — the constructor
/// verifies equality, so the roster needle can never drift from the
/// contour semantics (the fail-closed literal contract).
pub fn canonical_anchor(id: &VerticalId, position: usize, contour: &VerticalContour) -> String {
    format!(
        "{id} contour ({position}) {name} — {parents}",
        id = id.as_str(),
        position = position + 1,
        name = contour.name(),
        parents = contour.parents()
    )
}

/// The typed vertical profile (ADR-0220 §2): id + the six mandatory
/// contours + the profile-scoped anchor literals (the roster needles,
/// verified against the canonical derivation) + the carrier digest
/// pin (the G5/№828 form — SHA-256 over the canonical serialization
/// of the profile's own fields; the pin is SELF-DESCRIBING: the
/// consumer re-derives and compares byte-for-byte, the №868
/// re-attestation discipline).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VerticalProfile {
    pub id: VerticalId,
    /// The six mandatory contours — ALWAYS the full set (the
    /// constructor refuses anything less; ADR-0220 §2).
    pub mandatory_contours: [VerticalContour; 6],
    /// The profile-scoped contour anchor literals (the gate_045
    /// roster needles) — verified against the canonical derivation at
    /// construction (a drifted literal refuses HERE).
    pub contour_anchor_literals: [String; 6],
    /// The refusal-vocabulary generation the profile is attested
    /// against (the version-pin of contour (6); the frozen №385 set
    /// is referenced, never restated).
    pub refusal_vocabulary_version: String,
    /// The carrier digest pin (the G5/№828 form): hex(SHA-256) over
    /// the canonical serialization of (id, the six anchors, the
    /// vocabulary version). A profile without a verifiable pin is not
    /// a profile.
    pub carrier_digest: String,
}

impl VerticalProfile {
    /// Construct + digest-pin one profile (the fail-closed
    /// constructor, the №879/№888 form): the contour set must be the
    /// FULL six — a partial set refuses HERE, at the construction
    /// site, not at the audit; each anchor literal must equal the
    /// canonical derivation for its (id, position, contour); the
    /// vocabulary version must be non-empty (the version-pin of
    /// contour (6) is mandatory).
    pub fn new(
        id: VerticalId,
        contours: [VerticalContour; 6],
        contour_anchor_literals: [&str; 6],
        refusal_vocabulary_version: &str,
    ) -> Result<Self, String> {
        // The full-six contract: each of the six contours must appear
        // exactly once (a duplicate or a missing contour refuses —
        // a profile without the FULL set is not a vertical profile).
        let mut seen = [false; 6];
        for c in &contours {
            seen[*c as usize] = true;
        }
        if !seen.iter().all(|&s| s) {
            return Err(format!(
                "vertical: profile '{}' does not carry the FULL six mandatory contours (ADR-0220 §2) — a partial set refuses at construction",
                id.as_str()
            ));
        }
        // The anchor-literal contract: the roster needle equals the
        // canonical derivation (the fail-closed literal contract —
        // the needle can never drift from the contour semantics).
        for (i, c) in contours.iter().enumerate() {
            let canonical = canonical_anchor(&id, i, c);
            if contour_anchor_literals[i] != canonical {
                return Err(format!(
                    "vertical: profile '{}' contour anchor literal '{}' does not match the canonical derivation '{}' (ADR-0220 §2)",
                    id.as_str(),
                    contour_anchor_literals[i],
                    canonical
                ));
            }
        }
        if refusal_vocabulary_version.trim().is_empty() {
            return Err(format!(
                "vertical: profile '{}' carries an empty refusal-vocabulary version-pin (contour (6) is mandatory — the frozen №385 dictionary is referenced, never restated)",
                id.as_str()
            ));
        }
        let mut anchors = [String::new(); 6];
        for (i, s) in contour_anchor_literals.iter().enumerate() {
            anchors[i] = s.to_string();
        }
        let mut profile = Self {
            id,
            mandatory_contours: contours,
            contour_anchor_literals: anchors,
            refusal_vocabulary_version: refusal_vocabulary_version.to_string(),
            carrier_digest: String::new(),
        };
        profile.carrier_digest = profile.derive_digest();
        Ok(profile)
    }

    /// The canonical serialization (the compliance/canonical-row form
    /// of №828/№868): id \x1f the six profile-scoped anchors in roster
    /// order \x1f the vocabulary version. Deterministic — the same
    /// profile re-derives the same digest byte-for-byte.
    pub fn canonical_serialization(&self) -> String {
        let mut parts: Vec<String> = vec![self.id.as_str().to_string()];
        for a in &self.contour_anchor_literals {
            parts.push(a.clone());
        }
        parts.push(self.refusal_vocabulary_version.clone());
        parts.join(&SEP.to_string())
    }

    /// The carrier digest (the G5/№828 form): hex(SHA-256) over the
    /// canonical serialization.
    pub fn derive_digest(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.canonical_serialization().as_bytes());
        hex::encode(hasher.finalize())
    }
}

// ── The seed region (the gate_045 machine source) ────────────────────
// The two seed functions of the R38(5) roster — the PROFILE-SCOPED
// anchor literals (the needles) stand NEXT to the typed contour
// variants (the consumption sub-anchors) inside each seed body; the
// gate_045 machine pins both INSIDE the named seed region (the
// gate_040 form — the seed calls are rustfmt-normalized, so the check
// pins the quoted literals inside the region, not a call shape).

/// The payments profile seed (the roster rows 1–6, ADR-0220 §2).
fn seed_payments() -> VerticalProfile {
    VerticalProfile::new(
        VerticalId::Payments,
        [
            VerticalContour::JurisdictionCoverage,
            VerticalContour::HumanOversightApproval,
            VerticalContour::DecisionTraceVerify,
            VerticalContour::TaintFloor,
            VerticalContour::LicenseClass,
            VerticalContour::RefusalVocabularyPin,
        ],
        [
            "payments contour (1) jurisdiction-coverage — the JurisdictionMatrix package, strictest-wins (ADR-0209)",
            "payments contour (2) human-oversight-approval — the typed approval mandate (ADR-0205)",
            "payments contour (3) decision-trace-verify — the replay/forensic ledger (ADR-0210)",
            "payments contour (4) taint-floor — the taint saturation lane (ADR-0217)",
            "payments contour (5) license-class — the license-typed model layer (ADR-0202)",
            "payments contour (6) refusal-vocabulary-pin — the frozen №385 dictionary, the version-pin",
        ],
        "385-frozen-2026-11",
    )
    .expect("the payments seed profile carries the full six contours")
}

/// The public-sector profile seed (the roster rows 7–12, ADR-0220 §2).
/// The C7-связка: the profile REFERENCES the repro-line trigger
/// (the first gov/bank pilot — R38(8), the №913 contract); the
/// profile does NOT ignite the trigger (the trigger is the owner's
/// decision, never a configuration act).
fn seed_public_sector() -> VerticalProfile {
    VerticalProfile::new(
        VerticalId::PublicSector,
        [
            VerticalContour::JurisdictionCoverage,
            VerticalContour::HumanOversightApproval,
            VerticalContour::DecisionTraceVerify,
            VerticalContour::TaintFloor,
            VerticalContour::LicenseClass,
            VerticalContour::RefusalVocabularyPin,
        ],
        [
            "public-sector contour (1) jurisdiction-coverage — the JurisdictionMatrix package, strictest-wins (ADR-0209)",
            "public-sector contour (2) human-oversight-approval — the typed approval mandate (ADR-0205)",
            "public-sector contour (3) decision-trace-verify — the replay/forensic ledger (ADR-0210)",
            "public-sector contour (4) taint-floor — the taint saturation lane (ADR-0217)",
            "public-sector contour (5) license-class — the license-typed model layer (ADR-0202)",
            "public-sector contour (6) refusal-vocabulary-pin — the frozen №385 dictionary, the version-pin",
        ],
        "385-frozen-2026-11",
    )
    .expect("the public-sector seed profile carries the full six contours")
}

/// The append-only vertical registry (the JurisdictionMatrix form,
/// №888): the two verticals of R38(5) with the full six contours
/// each — the 12-element roster of ADR-0220 §2. The registry is a
/// CONSTANT: profiles are added by code review (an append), never by
/// configuration; a removal is a tamper the gate_045 roster catches
/// (the seed region disappears — the count drops, the floor breaches).
pub fn vertical_registry() -> &'static [VerticalProfile] {
    // The roster order is FIXED (payments first, public-sector second —
    // the ADR-0220 §2 table order; the deterministic contract).
    static REGISTRY: std::sync::OnceLock<Vec<VerticalProfile>> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(|| vec![seed_payments(), seed_public_sector()])
}

/// The declared vertical (the mlog.toml `[vertical]` table, the №910
/// [registry] shape): the pin lives in the declaring package's
/// manifest, never in a side channel (the G5 form — the trust anchor
/// is an explicit pin).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VerticalDeclaration {
    pub id: String,
    pub digest: String,
}

/// The named divergences of the fail-closed consumption (ADR-0220 §2).
/// Each refusal NAMES itself — a silent skip is impossible: there is
/// no code path where a declared vertical «passes loosely».
pub mod refusals {
    pub const UNKNOWN_ID: &str = "vertical: unknown vertical id";
    pub const DIGEST_MISMATCH: &str = "vertical: vertical digest pin mismatch";
    pub const CONTOUR_UNVERIFIED: &str = "vertical: mandatory contour not verifiable";
}

/// The verdict of the fail-closed consumption (ADR-0220 §2, the
/// №874/№845/№911 form). `None` = no `[vertical]` declaration → NO
/// default attach, `Ok(None)` — the undeclared manifest proceeds with
/// ZERO behavioral mutations (the negative control). `Some(decl)` →
/// all-or-refuse:
/// - an unknown id refuses («unknown vertical id» + the id);
/// - a digest mismatch refuses («vertical digest pin mismatch» + both
///   digests — the package was attested against a different profile
///   generation);
/// - a profile whose full-six contour set is not verifiable refuses
///   («mandatory contour not verifiable»).
/// The PACKAGE PATH composes this verdict AFTER the №911 integrity
/// pass (a tampered package refuses as INTEGRITY first — the ordering
/// discipline of ADR-0218 §4).
pub fn vertical_verdict(
    declared: Option<&VerticalDeclaration>,
) -> Result<Option<&'static VerticalProfile>, String> {
    let Some(decl) = declared else {
        return Ok(None); // no default attach — the negative control
    };
    let id = VerticalId::parse(&decl.id).map_err(|e| {
        // The named divergence: unknown id (refusals::UNKNOWN_ID is
        // the stable prefix the tests and the gates pin).
        format!(
            "{unknown} '{decl_id}' — {e}",
            unknown = refusals::UNKNOWN_ID,
            decl_id = decl.id,
            e = e
        )
    })?;
    let profile = vertical_registry()
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| {
            format!(
                "{unknown} '{decl_id}' — not in the registry (ADR-0220 §2)",
                unknown = refusals::UNKNOWN_ID,
                decl_id = decl.id
            )
        })?;
    if profile.carrier_digest != decl.digest {
        return Err(format!(
            "{mismatch}: declared {declared}, the live profile '{name}' pins {live} — the package was attested against a different profile generation (ADR-0220 §2; re-pin or re-attest)",
            mismatch = refusals::DIGEST_MISMATCH,
            declared = decl.digest,
            name = profile.id.as_str(),
            live = profile.carrier_digest
        ));
    }
    // The full-six verification: the constructor already refuses a
    // partial set, and the registry seeds construct — but the verdict
    // re-checks the LIVE profile (a tamper that swapped the contour
    // set in code would fail here at the consumption site too; the
    // gate_045 roster anchors are the machine backstop).
    let full_six = [
        VerticalContour::JurisdictionCoverage,
        VerticalContour::HumanOversightApproval,
        VerticalContour::DecisionTraceVerify,
        VerticalContour::TaintFloor,
        VerticalContour::LicenseClass,
        VerticalContour::RefusalVocabularyPin,
    ];
    let full = profile.mandatory_contours.len() == 6
        && full_six.iter().all(|c| profile.mandatory_contours.contains(c));
    if !full {
        return Err(format!(
            "{unverified} — profile '{name}' lost part of the six mandatory contours (ADR-0220 §2)",
            unverified = refusals::CONTOUR_UNVERIFIED,
            name = profile.id.as_str()
        ));
    }
    Ok(Some(profile))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_carries_two_profiles_with_full_six_contours() {
        let reg = vertical_registry();
        assert_eq!(reg.len(), 2, "the R38(5) roster: payments + public-sector");
        assert_eq!(reg[0].id.as_str(), "payments");
        assert_eq!(reg[1].id.as_str(), "public-sector");
        for p in reg {
            assert_eq!(p.mandatory_contours.len(), 6, "the full six (ADR-0220 §2)");
        }
    }

    #[test]
    fn carrier_digest_is_deterministic_and_discriminating() {
        // The registry pins match the derivation byte-for-byte (the
        // №868 re-attestation discipline).
        for p in vertical_registry() {
            assert_eq!(p.carrier_digest, p.derive_digest(), "self-describing pin");
            assert_eq!(p.carrier_digest.len(), 64, "hex(SHA-256)");
        }
        assert_ne!(
            vertical_registry()[0].carrier_digest,
            vertical_registry()[1].carrier_digest,
            "the id discriminates the pins"
        );
    }

    #[test]
    fn constructor_refuses_partial_contour_set_and_drifted_anchor() {
        let partial = VerticalProfile::new(
            VerticalId::Payments,
            [
                VerticalContour::JurisdictionCoverage,
                VerticalContour::HumanOversightApproval,
                VerticalContour::DecisionTraceVerify,
                VerticalContour::TaintFloor,
                VerticalContour::LicenseClass,
                VerticalContour::JurisdictionCoverage, // duplicate — the set is partial
            ],
            [
                canonical_anchor(&VerticalId::Payments, 0, &VerticalContour::JurisdictionCoverage),
                canonical_anchor(&VerticalId::Payments, 1, &VerticalContour::HumanOversightApproval),
                canonical_anchor(&VerticalId::Payments, 2, &VerticalContour::DecisionTraceVerify),
                canonical_anchor(&VerticalId::Payments, 3, &VerticalContour::TaintFloor),
                canonical_anchor(&VerticalId::Payments, 4, &VerticalContour::LicenseClass),
                canonical_anchor(&VerticalId::Payments, 5, &VerticalContour::JurisdictionCoverage),
            ],
            "385-frozen-2026-11",
        );
        assert!(partial.is_err(), "a partial set refuses at construction (№879 form)");
        // A drifted anchor literal refuses (the fail-closed literal
        // contract — the roster needle can never lie).
        let drifted = VerticalProfile::new(
            VerticalId::Payments,
            [
                VerticalContour::JurisdictionCoverage,
                VerticalContour::HumanOversightApproval,
                VerticalContour::DecisionTraceVerify,
                VerticalContour::TaintFloor,
                VerticalContour::LicenseClass,
                VerticalContour::RefusalVocabularyPin,
            ],
            [
                "payments contour (1) A COMPLETELY DIFFERENT CLAIM",
                canonical_anchor(&VerticalId::Payments, 1, &VerticalContour::HumanOversightApproval),
                canonical_anchor(&VerticalId::Payments, 2, &VerticalContour::DecisionTraceVerify),
                canonical_anchor(&VerticalId::Payments, 3, &VerticalContour::TaintFloor),
                canonical_anchor(&VerticalId::Payments, 4, &VerticalContour::LicenseClass),
                canonical_anchor(&VerticalId::Payments, 5, &VerticalContour::RefusalVocabularyPin),
            ],
            "385-frozen-2026-11",
        );
        assert!(drifted.is_err(), "a drifted needle refuses at construction");
    }

    #[test]
    fn verdict_refuses_unknown_id_and_digest_mismatch_and_passes_live_pin() {
        // Unknown id → named refusal.
        let bad_id = VerticalDeclaration {
            id: "medicine".to_string(),
            digest: "00".repeat(32),
        };
        let err = vertical_verdict(Some(&bad_id)).unwrap_err();
        assert!(err.starts_with(refusals::UNKNOWN_ID), "{err}");
        // Digest mismatch → named refusal.
        let bad_digest = VerticalDeclaration {
            id: "payments".to_string(),
            digest: "ab".repeat(32),
        };
        let err = vertical_verdict(Some(&bad_digest)).unwrap_err();
        assert!(err.starts_with(refusals::DIGEST_MISMATCH), "{err}");
        // The live pin → Ok(Some(profile)).
        let live = VerticalDeclaration {
            id: "payments".to_string(),
            digest: vertical_registry()[0].carrier_digest.clone(),
        };
        let v = vertical_verdict(Some(&live)).expect("the live pin passes");
        assert_eq!(v.unwrap().id.as_str(), "payments");
    }

    #[test]
    fn no_declaration_attaches_nothing() {
        // The negative control (ADR-0220 §2): no `[vertical]` →
        // Ok(None) — zero behavioral mutations on the undeclared path.
        assert!(matches!(vertical_verdict(None), Ok(None)));
    }
}
