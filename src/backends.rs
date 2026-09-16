//! Backend registry (Наряд №333, ADR-0163) — classes, SHA-pin contract,
//! license classes and the distribution gate data.
//!
//! ## ADR-0163 — the SSOT for what a backend IS
//!
//! One static table (`BACKEND_REGISTRY`), spec!-style: every entry is a
//! plain struct literal statically visible to the audit (the №316
//! classification precedent). Programs reference backends by identifier;
//! the license gate matches those identifiers at string-literal positions
//! (the `MODEL_WEIGHTS_UNSAFE` literal-URL precedent) plus the
//! `vision { model: … }` declaration field.
//!
//! ## The SHA-pin boundary is a TYPE, not a placeholder
//!
//! `ShaPin::Pinned(hash)` = the expected SHA-256 of the weights artifact,
//! verified at fetch/load time (the №334 contract). `ShaPin::PendingNo334`
//! = the weights artifact is NOT vendored in-tree (real-weights runs are
//! PARKED by hardware, №294) — there is NO hash to state, and fabricating
//! one would be a lie. №334 replaces every `PendingNo334` with a real pin
//! and refuses to LOAD `PendingNo334` entries.
//!
//! ## License classes are claims with named bases
//!
//! `Osi` / `NonOsi` / `Restrictive`, each entry carrying `license_note`
//! naming the license and its basis. Legal fine-reading of specific
//! licenses is OUT of scope (the issue's loud boundary): classes only.
//! Unverified licenses are **restrictive by default-deny** — the
//! `MODEL_WEIGHTS_UNSAFE` allowlist posture.

/// The §7.6 MDL model classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendClass {
    Stt,
    Tts,
    Omni,
    VisionUnderstanding,
    Llm,
}

impl BackendClass {
    pub fn as_str(self) -> &'static str {
        match self {
            BackendClass::Stt => "stt",
            BackendClass::Tts => "tts",
            BackendClass::Omni => "omni",
            BackendClass::VisionUnderstanding => "vision-understanding",
            BackendClass::Llm => "llm",
        }
    }
}

/// License class for distribution governance (ADR-0163 §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseClass {
    /// OSI-approved license (MIT, Apache-2.0, …) — distributable by default.
    Osi,
    /// Source-available but NOT OSI-approved (e.g. NVIDIA Open Model
    /// License) — forbidden in the distribution profile.
    NonOsi,
    /// Restrictive / unverified — forbidden by default-deny until a
    /// license record lands.
    Restrictive,
}

impl LicenseClass {
    pub fn as_str(self) -> &'static str {
        match self {
            LicenseClass::Osi => "osi",
            LicenseClass::NonOsi => "non-osi",
            LicenseClass::Restrictive => "restrictive",
        }
    }
}

/// The SHA-pin contract (ADR-0163 §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaPin {
    /// Expected SHA-256 of the weights artifact (hex, lowercase),
    /// verified at fetch/load time — the №334 contract.
    Pinned(&'static str),
    /// Weights not vendored in-tree (PARKED №294) — no hash to state.
    /// №334 replaces this variant with a real pin and REFUSES to load
    /// it. Loud, typed, grep-visible: never a fabricated hash.
    PendingNo334,
}

impl ShaPin {
    /// Report form (`backend_list` / audit messages).
    pub fn as_str(self) -> &'static str {
        match self {
            ShaPin::Pinned(h) => h,
            ShaPin::PendingNo334 => "pending (naryad №334 sha-pin path)",
        }
    }
}

/// One backend registry entry (ADR-0163 §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendEntry {
    /// Program-visible backend identifier.
    pub name: &'static str,
    pub class: BackendClass,
    /// Weights artifact identifier — the gate's matching key.
    pub weights_id: &'static str,
    pub pin: ShaPin,
    pub license: LicenseClass,
    /// The license and its classification basis (one line).
    pub license_note: &'static str,
}

/// The backend SSOT (ADR-0163 §2.1). Seed entries — the models the tree
/// and plan §15 already name; the list is reported loudly in the naryad
/// report. New backends (№334+) join HERE, never bypass the registry.
pub const BACKEND_REGISTRY: &[BackendEntry] = &[
    BackendEntry {
        name: "chatterbox",
        class: BackendClass::Tts,
        weights_id: "chatterbox-multilingual-v3",
        pin: ShaPin::PendingNo334,
        license: LicenseClass::Osi,
        license_note: "Chatterbox (Resemble AI) — MIT (osi)",
    },
    BackendEntry {
        name: "kokoro",
        class: BackendClass::Tts,
        weights_id: "koko-ro-82m",
        pin: ShaPin::PendingNo334,
        license: LicenseClass::Osi,
        license_note: "Kokoro-82M — Apache-2.0 (osi)",
    },
    BackendEntry {
        name: "z-image-turbo",
        class: BackendClass::VisionUnderstanding,
        weights_id: "z-image-turbo",
        pin: ShaPin::PendingNo334,
        license: LicenseClass::Osi,
        license_note: "Z-Image Turbo (Tongyi) — Apache-2.0 (osi)",
    },
    BackendEntry {
        name: "molmoact2",
        class: BackendClass::VisionUnderstanding,
        weights_id: "molmoact2",
        pin: ShaPin::PendingNo334,
        license: LicenseClass::Osi,
        license_note: "MolmoAct2 (AllenAI lineage) — Apache-2.0 (osi)",
    },
    BackendEntry {
        name: "wall-oss",
        class: BackendClass::Omni,
        weights_id: "wall-oss-0.5",
        pin: ShaPin::PendingNo334,
        license: LicenseClass::Restrictive,
        license_note: "license NOT verified in-tree — restrictive by default-deny (allowlist posture, ADR-0163 §2.1) until a license record lands",
    },
    BackendEntry {
        name: "nemotron-omni",
        class: BackendClass::Omni,
        weights_id: "nemotron-3-nano-omni-30b-a3b",
        pin: ShaPin::PendingNo334,
        license: LicenseClass::NonOsi,
        license_note: "Nemotron — NVIDIA Open Model License (non-osi; the MDL-3 test case)",
    },
];

/// Find a registry entry by weights identifier (exact, case-sensitive —
/// the identifiers are lowercase canon; gate matches are case-insensitive
/// and go through `find_by_weights_id_ci`).
pub fn find_by_weights_id(id: &str) -> Option<&'static BackendEntry> {
    BACKEND_REGISTRY.iter().find(|e| e.weights_id == id)
}

/// Case-insensitive weights-id match (the gate's key: a literal in ANY
/// casing names the same weights).
pub fn find_by_weights_id_ci(id: &str) -> Option<&'static BackendEntry> {
    let lower = id.to_lowercase();
    BACKEND_REGISTRY.iter().find(|e| e.weights_id == lower)
}

/// Find by the program-visible backend name.
pub fn find_by_name(name: &str) -> Option<&'static BackendEntry> {
    BACKEND_REGISTRY.iter().find(|e| e.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_is_complete() {
        for e in BACKEND_REGISTRY {
            assert!(!e.name.is_empty(), "{}: name", e.weights_id);
            assert!(!e.weights_id.is_empty(), "{}: weights_id", e.name);
            assert!(
                e.weights_id == e.weights_id.to_lowercase(),
                "{}: weights_id must be lowercase canon",
                e.name
            );
            assert!(
                !e.license_note.trim().is_empty(),
                "{}: license note is the classification basis",
                e.name
            );
            // Pins: either a real hex pin (64 lowercase hex chars) or the
            // typed PendingNo334 boundary — never an invented hash.
            match e.pin {
                ShaPin::PendingNo334 => {}
                ShaPin::Pinned(h) => {
                    assert_eq!(h.len(), 64, "{}: pin is sha256 hex", e.name);
                    assert!(
                        h.chars().all(|c| c.is_ascii_hexdigit()),
                        "{}: pin is hex",
                        e.name
                    );
                }
            }
        }
    }

    #[test]
    fn seed_license_classes_match_the_reported_basis() {
        // The MDL-3 test case (issue #460): Nemotron is not OSI-approved.
        let n = find_by_weights_id_ci("Nemotron-3-Nano-Omni-30B-A3B").expect("nemotron entry");
        assert_eq!(n.license, LicenseClass::NonOsi);
        // The default-deny posture: an unverified license is restrictive.
        let w = find_by_weights_id("wall-oss-0.5").expect("wall entry");
        assert_eq!(w.license, LicenseClass::Restrictive);
        // Known osi seeds.
        for id in [
            "chatterbox-multilingual-v3",
            "koko-ro-82m",
            "z-image-turbo",
            "molmoact2",
        ] {
            let e = find_by_weights_id(id).expect(id);
            assert_eq!(e.license, LicenseClass::Osi, "{}", id);
        }
    }

    #[test]
    fn classes_cover_the_mdl_vocabulary() {
        // At least one entry per §7.6 class that the tree already names.
        let has = |c: BackendClass| BACKEND_REGISTRY.iter().any(|e| e.class == c);
        assert!(has(BackendClass::Tts));
        assert!(has(BackendClass::VisionUnderstanding));
        assert!(has(BackendClass::Omni));
        // STT/LLM entries land with №334 (the SHA-pin path) — recorded
        // loudly in the naryad report, not stubbed here.
    }

    #[test]
    fn no_duplicates() {
        for (i, a) in BACKEND_REGISTRY.iter().enumerate() {
            for b in &BACKEND_REGISTRY[i + 1..] {
                assert_ne!(a.weights_id, b.weights_id, "weights_id uniqueness");
                assert_ne!(a.name, b.name, "name uniqueness");
            }
        }
    }
}
