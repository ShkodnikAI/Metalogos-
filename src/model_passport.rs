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
    /// №842: the revocation status — consumption refuses while it
    /// stands; cleared ONLY by the explicit re-pin transaction.
    pub revoked: bool,
    /// №842: the revocation note (the why, carried in the refusal).
    pub revocation_note: String,
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
            revoked: false,
            revocation_note: String::new(),
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

// ── №842 (wave 71 — W-A, the R38(4) decision): the passport hygiene ──
//
// The G5 pattern (№828, merged `694e0a2`) brought to the passport:
// fail-closed on the digest mismatch, re-pin ONLY by an explicit update,
// revocation as a consumption refusal. The hygiene lives at the passport
// level — BEFORE the runtime weight gates (the SSRF-guard / SHA-pin lane
// of №334 decides WHERE weights may load from; the passport decides
// whether the RECORD is trusted at all). No auto-refresh, no automatic
// re-validation of external sources — every state change is an explicit
// transaction.
//
// The wave-71 mode constant (the choice fixed in the PR report): the
// staleness mode is `Flag` — a stale passport raises the loud flag but
// does not refuse, because the in-tree verification dates would
// otherwise hard-refuse every program on day one (the honest reading of
// "по умолчанию флаг + отказ на публичных путях": the REFUSAL class —
// revoked and digest-mismatch — is fail-closed on EVERY path and every
// mode; the staleness escalation to Refuse is the owner's explicit act,
// a future naryad flips the constant).

/// The freshness window (days) — the wave-71 constant. A passport whose
/// `verified_at` is older than the window (against the consumption
/// date) is STALE. The №841 PASSPORT_FRESH counter mirrors this exact
/// rule (the same constant, the Python twin).
pub const PASSPORT_FRESHNESS_DAYS: i64 = 180;

/// The staleness mode — the wave-71 constant (see the module note):
/// `Flag` by default (the loud flag, no refusal); `Refuse` is the
/// owner's explicit escalation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StalenessMode {
    /// The stale passport raises the loud flag (the refusal carries the
    /// flag text; the compile-path wiring reports it, the consumption
    /// verdict names it) — but does NOT refuse.
    Flag,
    /// The stale passport refuses consumption (the owner's escalation).
    Refuse,
}

pub const PASSPORT_STALENESS_MODE: StalenessMode = StalenessMode::Flag;

/// The consumption refusal classes (fail-closed — the «отказ потребления»
/// family of the naryad). The Display text names the class explicitly:
/// «passport digest mismatch» for the digest class (the G5 wording).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PassportRefusal {
    /// The passport is revoked — consumption refuses; the un-revocation
    /// is only an explicit update (the re-pin transaction clears it).
    Revoked(String),
    /// The passport's digest does not match the pinned context's digest.
    /// Fail-closed, never a warning.
    DigestMismatch { expected: String, actual: String },
    /// The passport is stale (only fires when the mode is `Refuse`; the
    /// `Flag` mode carries the same data as the loud flag text).
    Stale {
        verified_at: String,
        window_days: i64,
    },
    /// The passport failed its own integrity check (the detempo-guard —
    /// a post-construction mutation).
    Tampered(String),
}

impl std::fmt::Display for PassportRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PassportRefusal::Revoked(note) => {
                write!(f, "passport revoked — consumption refused ({note}); the un-revocation is only an explicit update (the re-pin transaction, ADR-0202 §2.4)")
            }
            PassportRefusal::DigestMismatch { expected, actual } => {
                write!(
                    f,
                    "passport digest mismatch — the pinned context expects {}, the passport carries {} (fail-closed, №842; the re-pin is only an explicit update)",
                    &expected[..16.min(expected.len())],
                    &actual[..16.min(actual.len())]
                )
            }
            PassportRefusal::Stale {
                verified_at,
                window_days,
            } => {
                write!(
                    f,
                    "passport stale — verified_at {verified_at} is older than the freshness window ({window_days} days); re-pin by the explicit update (new digest + new verification date in one transaction)"
                )
            }
            PassportRefusal::Tampered(why) => {
                write!(
                    f,
                    "passport digest mismatch — {why} (the integrity check is fail-closed)"
                )
            }
        }
    }
}

impl std::error::Error for PassportRefusal {}

/// Parse `YYYY-MM-DD` to the (year, month, day) triple — the mirror of
/// the constructor's date validation (the light, std-only shape).
fn parse_verification_date(s: &str) -> Option<(i32, i32, i32)> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let all_digits = b
        .iter()
        .enumerate()
        .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    if !all_digits {
        return None;
    }
    Some((
        s[..4].parse().ok()?,
        s[5..7].parse().ok()?,
        s[8..10].parse().ok()?,
    ))
}

/// The days-since-epoch arithmetic (the civil-date algorithm, std-only —
/// no external date crate; the same convention both twins use).
fn days_from_civil(y: i32, m: i32, d: i32) -> i64 {
    let y = i64::from(y);
    let m = i64::from(m);
    let d = i64::from(d);
    let (y, m) = if m <= 2 { (y - 1, m + 12) } else { (y, m) };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (m - 3) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe
}

impl ModelPassport {
    /// The days between the passport's `verified_at` and `today` (both
    /// `YYYY-MM-DD`). `None` = an unparseable date (the constructor
    /// forbids it — `None` here is the fail-closed reading).
    pub fn age_days(&self, today: &str) -> Option<i64> {
        let (y1, m1, d1) = parse_verification_date(&self.verified_at)?;
        let (y2, m2, d2) = parse_verification_date(today)?;
        Some(days_from_civil(y2, m2, d2) - days_from_civil(y1, m1, d1))
    }

    /// Is the passport fresh against `today` (the №842 rule: the age
    /// within [`PASSPORT_FRESHNESS_DAYS`])? An unparseable date is NOT
    /// fresh (fail-closed).
    pub fn is_fresh(&self, today: &str) -> bool {
        match self.age_days(today) {
            Some(age) => (0..=PASSPORT_FRESHNESS_DAYS).contains(&age),
            None => false,
        }
    }

    /// The explicit RE-PIN transaction (№842 §3): the new digest AND the
    /// new verification date land in ONE call — never one without the
    /// other, never automatic. Validates both (the same constructor
    /// rules), re-captures the integrity anchors (the re-pin IS the
    /// explicit act that re-anchors), and clears the revocation (the
    /// un-revocation rides the same explicit transaction — the naryad's
    /// "снятие ревокации — только явным обновлением").
    pub fn re_pin(
        &mut self,
        new_digest: String,
        new_verified_at: String,
    ) -> Result<(), PassportError> {
        // Validate BEFORE mutating (the transaction is all-or-nothing).
        let digest_ok = new_digest.len() == 64
            && new_digest
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c));
        if !digest_ok {
            return Err(PassportError::BadDigest(
                "the re-pin digest must be 64 lowercase hex chars (the ADR-0163 contract)"
                    .to_string(),
            ));
        }
        if !is_valid_verification_date(&new_verified_at) {
            return Err(PassportError::BadVerificationDate(new_verified_at));
        }
        self.digest = new_digest;
        self.verified_at = new_verified_at;
        // The record fields changed → the record anchor re-captures; the
        // digest anchor follows the explicit new pin.
        self.record_anchor = self.record_digest();
        self.digest_anchor = self.digest.clone();
        self.revoked = false;
        Ok(())
    }

    /// The explicit REVOCATION (№842 §4): consumption refuses while the
    /// status stands. The un-revocation is ONLY the explicit re-pin
    /// transaction above — no automatic path exists.
    pub fn revoke(&mut self, note: String) {
        self.revoked = true;
        self.revocation_note = note;
    }

    /// The consumption verdict (№842 §1–§4): the fail-closed gate a
    /// consumer passes BEFORE trusting the passport. `pinned_digest` =
    /// the digest the pin context recorded (the static table's digest
    /// for the in-tree passports); `today` = the consumption date
    /// (`YYYY-MM-DD`). Order: revoked → mismatch → integrity → staleness
    /// (per the mode constant). Fresh + matching + not-revoked = Ok.
    pub fn consume(&self, pinned_digest: &str, today: &str) -> Result<(), PassportRefusal> {
        if self.revoked {
            return Err(PassportRefusal::Revoked(self.revocation_note.clone()));
        }
        if self.digest != pinned_digest {
            return Err(PassportRefusal::DigestMismatch {
                expected: pinned_digest.to_string(),
                actual: self.digest.clone(),
            });
        }
        if let Err(e) = self.verify_integrity() {
            return Err(PassportRefusal::Tampered(e.to_string()));
        }
        if !self.is_fresh(today) {
            match PASSPORT_STALENESS_MODE {
                StalenessMode::Refuse => {
                    return Err(PassportRefusal::Stale {
                        verified_at: self.verified_at.clone(),
                        window_days: PASSPORT_FRESHNESS_DAYS,
                    });
                }
                StalenessMode::Flag => {
                    // The loud flag rides the refusal TEXT (the caller
                    // renders it), but the verdict is Ok — the flag-only
                    // posture of the wave-71 constant.
                }
            }
        }
        Ok(())
    }
}

/// The inverse civil-date algorithm (the twin of `days_from_civil`).
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (y + i64::from(m <= 2), m, d)
}

/// The consumption date of the compile path (`YYYY-MM-DD`, UTC) — std
/// only, no external date crate.
pub fn today_utc() -> String {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64 / 86_400)
        .unwrap_or(0);
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}
