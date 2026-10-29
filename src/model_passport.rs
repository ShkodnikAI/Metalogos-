//! №838 (ADR-0202, wave 71 — W-A, the 8D-1 subphase): ModelPassport —
//! the passport of a model as ONE typed artifact.
//!
//! ADR-0163 (№333) carries the license classes as claims with named
//! bases inside the backend registry; the vision weights manifest
//! carries SHA-256 pins with no license field. The facts about one model
//! lived in two holders, and freshness/revocation/identity did not live
//! anywhere. This module lands the passport TYPE that unifies them:
//!
//! - `ModelPassport` — identity, digest (the SHA-pin contract),
//!   the ADR-0163 license class + `license_note` verbatim, version /
//!   source, the verification date, the origin link. Every mandatory
//!   field is validated by the constructor — a passport without a
//!   digest, a class, or a verification date does NOT construct (loud
//!   `Result`, never a silent default).
//! - The digest carries the ADR-0163 SHA-pin when the registry pins an
//!   artifact (`PassportPinKind::Artifact`). Entries without an artifact
//!   pin (the `ShaPin::PendingNo334` lane) carry the **record digest** —
//!   SHA-256 over the passport's own fields (`PassportPinKind::Record`):
//!   tamper-evidence without inventing a hash (the "never a fabricated
//!   hash" rule of the manifest discipline).
//! - `verify_integrity` is the detempo-guard: recompute the record
//!   digest and compare with the anchors captured at construction. Any
//!   post-construction mutation of any field — including the digest
//!   itself — fails the check with the «passport digest mismatch» class.
//!   Fail-closed; the re-pin is №842's explicit-update flow.
//!
//! ## The migration 1:1 (no behavior growth — the №545 stop-list lever)
//!
//! The static SSOT is the passport table below, keyed 1:1 to
//! `BACKEND_REGISTRY` (the arity is tested in both directions by
//! `tests/naryad_838_model_passport.rs`). The registry entries and the
//! weights-manifest sources read the SAME passport through the same key
//! (`crate::vision::weights::WeightsManifest::passport_for_model`) —
//! one source of truth, no second copy. NOTHING in the compile/audit/
//! load paths switches to the passport in this naryad: the consumers
//! arrive with №839 (the profile rule), №841 (the fact machines), and
//! №842 (the hygiene). The registry file `src/backends.rs` is not
//! touched at all — the passport is a module-neighbor, not an inline
//! field, so the №333 contracts stay byte-identical.
//!
//! ## Place (fixed per the naryad's requirement)
//!
//! `src/model_passport.rs` — a module next to `backends.rs`, not inside
//! it: the passport reads the registry, the registry does not know the
//! passport (no dependency cycle, no №333 surface growth, the
//! module-size posture of the guard stays intact).

use std::sync::OnceLock;

use crate::backends::{BackendEntry, LicenseClass, ShaPin, BACKEND_REGISTRY};
use crate::ledger::sha256_hex;

/// Which pin the passport's digest carries (ADR-0202 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassportPinKind {
    /// The digest is the registry's artifact SHA-pin (the №334 contract;
    /// verified at fetch/load time by the №334 lane — the passport does
    /// not duplicate the loader).
    Artifact,
    /// No artifact pin exists (the `PendingNo334` lane — in-tree rungs or
    /// manifests not yet filled). The digest is the RECORD digest —
    /// SHA-256 over the passport's own fields: tamper-evidence without
    /// inventing a hash.
    Record,
}

impl PassportPinKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PassportPinKind::Artifact => "artifact-pin",
            PassportPinKind::Record => "record-digest",
        }
    }
}

/// The loud passport-construction failure classes (ADR-0202 §2.1: a
/// passport without a mandatory field does not construct).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PassportError {
    /// A mandatory field is empty.
    MissingField(&'static str),
    /// The digest is not a valid 64-char lowercase hex SHA-256 pin, or a
    /// Record-kind digest does not match the passport's own fields.
    BadDigest(String),
    /// `verified_at` is not a valid YYYY-MM-DD date.
    BadVerificationDate(String),
    /// The post-construction integrity check failed — a field (or the
    /// digest itself) was modified after the passport was built.
    /// Fail-closed, the message names the «passport digest mismatch»
    /// class explicitly.
    Tampered(String),
}

impl std::fmt::Display for PassportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PassportError::MissingField(name) => {
                write!(f, "model passport: mandatory field '{name}' is empty — the passport does not construct (ADR-0202)")
            }
            PassportError::BadDigest(why) => {
                write!(f, "model passport: invalid digest — {why}")
            }
            PassportError::BadVerificationDate(value) => {
                write!(
                    f,
                    "model passport: 'verified_at' must be a YYYY-MM-DD date, got '{value}'"
                )
            }
            PassportError::Tampered(why) => {
                write!(f, "passport digest mismatch — {why} (the integrity check is fail-closed, ADR-0202)")
            }
        }
    }
}

impl std::error::Error for PassportError {}

/// The construction-time fields of a passport (the validating
/// constructor consumes this shape — no 9-argument call sites).
#[derive(Debug, Clone)]
pub struct NewPassport {
    /// The registry identifier (the weights id — the №333 gate's key).
    pub identity: String,
    /// The SHA-256 pin the passport vouches for (64 lowercase hex).
    pub digest: String,
    pub pin_kind: PassportPinKind,
    /// The ADR-0163 class — reused verbatim, no new taxonomy.
    pub license: LicenseClass,
    /// The license and its classification basis (the ADR-0163 one-line
    /// rule).
    pub license_note: String,
    /// The model version (the manifest revision, or the in-tree mark).
    pub version: String,
    /// The origin source (the HF repo id, or the in-tree statement).
    pub source: String,
    /// The verification date (the freshness anchor), `YYYY-MM-DD`.
    pub verified_at: String,
    /// The origin-chain link (the ADR-0164 provenance posture applied to
    /// model records).
    pub origin: String,
}

/// The per-entry passport metadata of the 1:1 migration. The table is
/// hand-curated the same way the №212/№334 manifests are (provenance in
/// every row); `tests/naryad_838_model_passport.rs` cross-checks every
/// row against the live registry — the artifact-pin rows against
/// `WEIGHTS_SOURCES` (repo equality), so the table cannot drift from
/// the SSOT without a red CI.
struct PassportMeta {
    weights_id: &'static str,
    version: &'static str,
    source: &'static str,
    verified_at: &'static str,
}

/// The verification-date rule of the migration: artifact-pinned entries
/// carry the documented №334 HF-tree-API fetch date (2026-09-16 — the
/// date the pins were actually verified against the artifacts); every
/// pending/in-tree record carries the date of THIS migration's in-tree
/// verification pass (2026-10-29 — the №838 review that re-checked each
/// class, note, and pin state against the live tree).
const PIN_VERIFIED_AT: &str = "2026-09-16";
const MIGRATION_VERIFIED_AT: &str = "2026-10-29";

const PENDING_SOURCE: &str = "external-weights (manifest pending, №334 lane)";
const IN_TREE_SOURCE: &str = "in-tree (no external weights artifact exists)";

const PASSPORT_META: &[PassportMeta] = &[
    PassportMeta {
        weights_id: "chatterbox-multilingual-v3",
        version: "multilingual-v3",
        source: PENDING_SOURCE,
        verified_at: MIGRATION_VERIFIED_AT,
    },
    PassportMeta {
        weights_id: "koko-ro-82m",
        version: "82m",
        source: PENDING_SOURCE,
        verified_at: MIGRATION_VERIFIED_AT,
    },
    PassportMeta {
        weights_id: "z-image-turbo",
        version: "turbo",
        source: PENDING_SOURCE,
        verified_at: MIGRATION_VERIFIED_AT,
    },
    PassportMeta {
        weights_id: "molmoact2",
        version: "main",
        source: "allenai/MolmoAct2",
        verified_at: PIN_VERIFIED_AT,
    },
    PassportMeta {
        weights_id: "wall-oss-0.5",
        version: "0.5",
        source: PENDING_SOURCE,
        verified_at: MIGRATION_VERIFIED_AT,
    },
    PassportMeta {
        weights_id: "nemotron-3-nano-omni-30b-a3b",
        version: "main",
        source: "nvidia/Nemotron-3-Nano-Omni-30B-A3B-Reasoning-BF16",
        verified_at: PIN_VERIFIED_AT,
    },
    PassportMeta {
        weights_id: "whisper-large-v3-turbo",
        version: "main",
        source: "openai/whisper-large-v3-turbo",
        verified_at: PIN_VERIFIED_AT,
    },
    PassportMeta {
        weights_id: "trocr-base-printed",
        version: "main",
        source: "microsoft/trocr-base-printed",
        verified_at: PIN_VERIFIED_AT,
    },
    PassportMeta {
        weights_id: "qwen2.5-vl-7b-instruct",
        version: "main",
        source: "Qwen/Qwen2.5-VL-7B-Instruct",
        verified_at: PIN_VERIFIED_AT,
    },
    PassportMeta {
        weights_id: "llava-video-7b-qwen2",
        version: "main",
        source: "lmms-lab/LLaVA-Video-7B-Qwen2",
        verified_at: PIN_VERIFIED_AT,
    },
    PassportMeta {
        weights_id: "internvl3-8b",
        version: "main",
        source: "OpenGVLab/InternVL3-8B",
        verified_at: PIN_VERIFIED_AT,
    },
    PassportMeta {
        weights_id: "in-tree-kinematics-sim",
        version: "in-tree",
        source: IN_TREE_SOURCE,
        verified_at: MIGRATION_VERIFIED_AT,
    },
    PassportMeta {
        weights_id: "in-tree-mock-device",
        version: "in-tree",
        source: IN_TREE_SOURCE,
        verified_at: MIGRATION_VERIFIED_AT,
    },
    PassportMeta {
        weights_id: "google/timesfm-2.5-200m-pytorch",
        version: "2.5-200m-pytorch",
        source: PENDING_SOURCE,
        verified_at: MIGRATION_VERIFIED_AT,
    },
    PassportMeta {
        weights_id: "n/a-software-rung",
        version: "software-rung",
        source: IN_TREE_SOURCE,
        verified_at: MIGRATION_VERIFIED_AT,
    },
    PassportMeta {
        weights_id: "builtin-seasonal-naive",
        version: "in-tree",
        source: IN_TREE_SOURCE,
        verified_at: MIGRATION_VERIFIED_AT,
    },
];

/// The origin link of the migration: every passport is derived 1:1 from
/// the №333 registry row (the origin-chain anchor of the record).
const REGISTRY_ORIGIN: &str =
    "adr-0163:BACKEND_REGISTRY (the №333 registry row is the origin of this passport)";

/// The record digest: SHA-256 over the passport's identity fields —
/// EXCLUDING the digest itself (else circular). The tamper-evidence
/// anchor of both pin kinds.
fn record_digest_of(fields: &NewPassport) -> String {
    sha256_hex(
        format!(
            "model-passport-v1\x1f{}\x1f{}\x1f{}\x1f{}\x1f{}\x1f{}\x1f{}\x1f{}",
            fields.identity,
            fields.pin_kind.as_str(),
            fields.license.as_str(),
            fields.license_note,
            fields.version,
            fields.source,
            fields.verified_at,
            fields.origin,
        )
        .as_bytes(),
    )
}

/// `true` when `s` is a valid calendar date in the `YYYY-MM-DD` shape
/// (light validation: the shape, the ranges; no external date crate).
/// Byte-level first: a non-ASCII input can never slice-panic.
fn is_valid_verification_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let all_digits = b
        .iter()
        .enumerate()
        .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    if !all_digits {
        return false;
    }
    let year: i32 = s[..4].parse().unwrap_or(0);
    let month: i32 = s[5..7].parse().unwrap_or(0);
    let day: i32 = s[8..10].parse().unwrap_or(0);
    (2020..=2100).contains(&year) && (1..=12).contains(&month) && (1..=31).contains(&day)
}

/// The model passport (ADR-0202 §2): ONE typed object carrying every
/// fact the compiler/audit lanes need about a model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPassport {
    /// The registry identifier (the weights id — the №333 gate's key).
    pub identity: String,
    /// The SHA-256 pin the passport vouches for (64 lowercase hex): the
    /// artifact pin (`Artifact`) or the record digest (`Record`).
    pub digest: String,
    pub pin_kind: PassportPinKind,
    /// The ADR-0163 class — the single taxonomy, reused verbatim.
    pub license: LicenseClass,
    /// The license and its classification basis (ADR-0163).
    pub license_note: String,
    /// The model version.
    pub version: String,
    /// The origin source (the HF repo id, or the in-tree statement).
    pub source: String,
    /// The verification date (the freshness anchor), `YYYY-MM-DD`.
    pub verified_at: String,
    /// The origin-chain link (ADR-0164 posture).
    pub origin: String,
    /// The record digest captured at construction (private — the
    /// integrity anchor; re-derivable via [`ModelPassport::record_digest`]).
    record_anchor: String,
    /// The digest value captured at construction (private — the second
    /// integrity anchor; catches a direct digest tamper on the
    /// Artifact kind).
    digest_anchor: String,
}

impl ModelPassport {
    /// The validating constructor (ADR-0202 §2.1): a passport without a
    /// digest, a class note, a version, a source, a verification date,
    /// or an origin does NOT construct — loud `Result`, never a silent
    /// default. The digest must be a 64-char lowercase hex SHA-256; a
    /// Record-kind digest must equal the passport's own record digest
    /// (self-consistent construction).
    pub fn new(fields: NewPassport) -> Result<Self, PassportError> {
        if fields.identity.trim().is_empty() {
            return Err(PassportError::MissingField("identity"));
        }
        if fields.license_note.trim().is_empty() {
            return Err(PassportError::MissingField("license_note"));
        }
        if fields.version.trim().is_empty() {
            return Err(PassportError::MissingField("version"));
        }
        if fields.source.trim().is_empty() {
            return Err(PassportError::MissingField("source"));
        }
        if fields.verified_at.trim().is_empty() {
            return Err(PassportError::MissingField("verified_at"));
        }
        if fields.origin.trim().is_empty() {
            return Err(PassportError::MissingField("origin"));
        }
        if !is_valid_verification_date(&fields.verified_at) {
            return Err(PassportError::BadVerificationDate(fields.verified_at));
        }
        let digest_ok = fields.digest.len() == 64
            && fields
                .digest
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c));
        if !digest_ok {
            return Err(PassportError::BadDigest(
                "the SHA-256 pin must be 64 lowercase hex chars (the ADR-0163 contract)"
                    .to_string(),
            ));
        }
        if fields.pin_kind == PassportPinKind::Record {
            let expected = record_digest_of(&fields);
            if fields.digest != expected {
                return Err(PassportError::BadDigest(
                    "the Record-kind digest must equal the passport's own record digest"
                        .to_string(),
                ));
            }
        }
        Ok(Self {
            record_anchor: record_digest_of(&fields),
            digest_anchor: fields.digest.clone(),
            identity: fields.identity,
            digest: fields.digest,
            pin_kind: fields.pin_kind,
            license: fields.license,
            license_note: fields.license_note,
            version: fields.version,
            source: fields.source,
            verified_at: fields.verified_at,
            origin: fields.origin,
        })
    }

    /// The migration path (ADR-0202 §2.2, №838): derive the passport 1:1
    /// from the registry row — identity/weights_id, the digest from the
    /// pin contract, the class + note verbatim, the version/source/date
    /// from the cross-checked meta table.
    pub fn from_entry(entry: &BackendEntry) -> Result<Self, PassportError> {
        let meta = PASSPORT_META
            .iter()
            .find(|m| m.weights_id == entry.weights_id)
            .ok_or(PassportError::MissingField(
                "passport meta (the №838 table must cover every registry entry)",
            ))?;
        let mut fields = NewPassport {
            identity: entry.weights_id.to_string(),
            digest: String::new(),
            pin_kind: PassportPinKind::Record,
            license: entry.license,
            license_note: entry.license_note.to_string(),
            version: meta.version.to_string(),
            source: meta.source.to_string(),
            verified_at: meta.verified_at.to_string(),
            origin: REGISTRY_ORIGIN.to_string(),
        };
        match entry.pin {
            ShaPin::Pinned(h) => {
                fields.digest = h.to_string();
                fields.pin_kind = PassportPinKind::Artifact;
            }
            // The record digest IS the digest for the pending lane —
            // tamper-evidence without a fabricated hash.
            ShaPin::PendingNo334 => {
                fields.pin_kind = PassportPinKind::Record;
                fields.digest = record_digest_of(&fields);
            }
        }
        Self::new(fields)
    }

    /// Recompute the record digest from the CURRENT fields (the verify
    /// primitive — the comparison against the construction anchors is
    /// what `verify_integrity` does).
    pub fn record_digest(&self) -> String {
        sha256_hex(
            format!(
                "model-passport-v1\x1f{}\x1f{}\x1f{}\x1f{}\x1f{}\x1f{}\x1f{}\x1f{}",
                self.identity,
                self.pin_kind.as_str(),
                self.license.as_str(),
                self.license_note,
                self.version,
                self.source,
                self.verified_at,
                self.origin,
            )
            .as_bytes(),
        )
    }

    /// The detempo-guard (№838 negative (б)): fail-closed integrity.
    /// Recomputes the record digest and compares it with the anchor
    /// captured at construction, and the current digest with its anchor.
    /// ANY post-construction mutation of any field — including the
    /// digest itself — fails with the «passport digest mismatch» class.
    pub fn verify_integrity(&self) -> Result<(), PassportError> {
        let current = self.record_digest();
        if current != self.record_anchor {
            return Err(PassportError::Tampered(format!(
                "the record fields were modified after construction (record digest {} != pinned {})",
                &current[..16],
                &self.record_anchor[..16]
            )));
        }
        if self.digest != self.digest_anchor {
            return Err(PassportError::Tampered(
                "the digest was modified after construction".to_string(),
            ));
        }
        if self.pin_kind == PassportPinKind::Record && self.digest != current {
            return Err(PassportError::Tampered(
                "the Record-kind digest no longer matches the passport's own fields".to_string(),
            ));
        }
        Ok(())
    }
}

/// The static passport SSOT (ADR-0202 §2.2): keyed 1:1 to
/// `BACKEND_REGISTRY`. Built through the VALIDATING constructor on first
/// access — an entry that cannot produce a valid passport fails LOUDLY
/// (the panic names the offending row; the arity table tests pin the
/// 1:1 in both directions, so a drifted registry is red in CI before
/// any consumer reads a half-migrated table). The unwrap/expect deny
/// (№29 §6.4) is honored — the loud failure is a direct `panic!` with
/// the passport error named, not a hidden unwrap.
static PASSPORT_TABLE: OnceLock<Vec<ModelPassport>> = OnceLock::new();

/// All model passports (the 1:1 mirror of `BACKEND_REGISTRY`).
pub fn all_passports() -> &'static [ModelPassport] {
    PASSPORT_TABLE.get_or_init(|| {
        match BACKEND_REGISTRY
            .iter()
            .map(ModelPassport::from_entry)
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(table) => table,
            Err(e) => panic!(
                "model passports: a registry entry does not construct a valid passport — {e} (the №838 arity tests pin the 1:1)"
            ),
        }
    })
}

/// The passport for a weights id (the same key the №333 gate matches).
pub fn passport_for(weights_id: &str) -> Option<&'static ModelPassport> {
    all_passports().iter().find(|p| p.identity == weights_id)
}
