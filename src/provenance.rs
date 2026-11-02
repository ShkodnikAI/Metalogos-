// ── Naryad №873 (wave 77 — W-F, P1, [core][provenance]): ProvenanceManifest
//
// ADR-0207 §2: provenance is a typed fact about an I/O boundary
// crossing — the origin digest (the SHA-256 of the value bytes that
// crossed, the G5 digest-pinning form of №828), the channel (file /
// network / respond / print), the direction (ingress / egress), the
// revision (a monotone per-process record sequence — later records of
// the same bytes are revisions, never replacements) and the fact of
// record (the record digest that pins the row itself).
//
// The map is APPEND-ONLY (ADR-0207 §2.3): a crossing adds a row,
// nothing edits or deletes a row (the loud-bookkeeping posture of
// №335, the attestation-map form of ADR-0206 §2.2). Two records of
// the same bytes never contradict — they are two attestations of the
// same origin digest (the media-consistency guard: the C2PA contour
// of №337 hashes the same bytes with the same SHA-256; the guard test
// pins the agreement).
//
// THE DEDUP GUARD (the R55 fact-check, ADR-0207 §5): this module is
// the SHARED I/O layer (8C/C3). The word *provenance* is alive in
// OTHER domains — `src/vision/provenance.rs` (the vision/C2PA origin
// hashes, ADR-0152/ADR-0166) and `media_manifest` (the №337 sidecar
// surface, src/media_ops.rs) — this module builds OVER the shared
// boundary and never renames, moves, wraps or re-exports those
// surfaces; the name-separation test in
// tests/naryad_873_provenance_kernel.rs pins the boundary mechanically.
// The `Attestation` contour of ADR-0206 (src/compliance.rs) is a THIRD
// domain (the compliance evidence map) — the two share the
// digest-pinning pattern, not the contract.
//
// Purity (the portable-kernel boundary of ADR-0207 §3 — the
// ADR-0204/ADR-0205/ADR-0206 form): the carrier layer below the seam
// is pure — no clock, no network, no filesystem, no Ledger; the host
// passes the moment and the bytes in. Hashing is the only computation
// and it is deterministic.
//
// The refusal vocabulary does not grow (№385/ADR-0207 §3): the
// «provenance missing» refusal is NOT a new deny word — it rides the
// EXISTING `SINK_CLEARANCE` class (the generic sink-clearance word)
// with the typed detail in the refusal text (№874 composes the gate);
// the construction refusals (an unregistered carrier, a malformed
// digest) are constructor errors (`Result::Err`), not DenyEvent
// vocabulary.
//
// Migration 1:1 (the №844/№860/№867 pattern): this module only ADDS
// the carrier and the boundary record — no existing constructor,
// builtin signature, Ledger record or golden file changes shape (the
// ledger-golden and conformance suites stay green with ZERO golden
// edits; the happy path adds NO Ledger records).

use sha2::{Digest, Sha256};

/// The Struct type_name reserved for the future language-visible view
/// of the manifest (the №844 `Capability` pattern). The pure Rust
/// carrier is the SSOT; any wiring that surfaces it to programs is a
/// future explicit registry transaction — NOT this naryad (no new
/// builtin surfaces).
pub const PROVENANCE_TYPE: &str = "ProvenanceManifest";

/// The channels of the shared I/O layer (ADR-0207 §2.1). CLOSED — a
/// new channel lands as an explicit ADR/registry transaction, never
/// by a silent string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Channel {
    /// The filesystem boundary (read_file / write_file and their
    /// shared-path twins — the fs_gate facade).
    File,
    /// The outbound network boundary (http_get / http_post — the
    /// SSRF-guarded client surface).
    Network,
    /// The HTTP-response boundary (respond — the value the server
    /// materializes).
    Respond,
    /// The stdout boundary (print — the human-observable channel).
    Print,
}

impl Channel {
    pub fn as_str(self) -> &'static str {
        match self {
            Channel::File => "file",
            Channel::Network => "network",
            Channel::Respond => "respond",
            Channel::Print => "print",
        }
    }
}

/// The crossing direction (ADR-0207 §2.1): a value entering the
/// program (ingress) or leaving it (egress).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Direction {
    Ingress,
    Egress,
}

impl Direction {
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::Ingress => "ingress",
            Direction::Egress => "egress",
        }
    }
}

/// The typed provenance manifest (ADR-0207 §2.1): a pure value pinning
/// one boundary crossing. The row is content-addressed (the origin
/// digest) and self-pinned (the record digest — the G5 form: a record
/// without a digest is not a row of the map).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProvenanceManifest {
    /// The carrier that performed the crossing (a
    /// `PROVENANCE_CARRIERS` name — the registry is the closed
    /// vocabulary of the shared layer).
    pub carrier: String,
    /// The channel of the crossing.
    pub channel: Channel,
    /// The crossing direction.
    pub direction: Direction,
    /// SHA-256, lowercase hex, of the value bytes that crossed.
    pub origin_digest: String,
    /// The monotone per-process record sequence (revisions never
    /// replace — they append).
    pub revision: u64,
    /// Unix epoch seconds when the crossing was recorded (the G5
    /// «дата сверки» — the verification date the use-site check
    /// compares against, ADR-0207 §2.4).
    pub recorded_at: u64,
    /// The row digest (the G5 form of №828): SHA-256 over the
    /// canonical serialization of every field above. The fact of
    /// record — a row without it is not a row of the map.
    pub record_digest: String,
}

/// The canonical origin digest: SHA-256 over the exact bytes that
/// crossed, hex-encoded. The SAME hash form the media contour uses
/// for `bytes_sha256` (src/media/mod.rs — the №337 insert-time fact;
/// the consistency guard pins the agreement) and the passports/ledger
/// use for their pins: one SHA-256 vocabulary across the domains, no
/// re-derivation.
pub fn origin_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{:02x}", byte));
    }
    hex
}

fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// The canonical row serialization (the determinism contract of
/// ADR-0207 §3 — the fixed field order, the `\x1f` separator, the
/// house form of approval.rs/compliance.rs). Pure and deterministic:
/// the same fields always produce the same bytes.
pub fn canonical_row(m: &ProvenanceManifest) -> String {
    [
        "v1",
        &m.carrier,
        m.channel.as_str(),
        m.direction.as_str(),
        &m.origin_digest,
        &m.revision.to_string(),
        &m.recorded_at.to_string(),
    ]
    .join("\u{1f}")
}

/// The record digest: SHA-256 over `canonical_row`, hex-encoded (the
/// G5 form of №828 — the fact of record).
pub fn record_digest_of(m: &ProvenanceManifest) -> String {
    let digest = Sha256::digest(canonical_row(m).as_bytes());
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{:02x}", byte));
    }
    hex
}

impl ProvenanceManifest {
    /// Build a manifest row with the full negative controls (the №860
    /// form — the refusals are part of the contract): (1) the carrier
    /// must be a REGISTERED shared-layer carrier (the registry is the
    /// closed vocabulary — an unregistered name is not a crossing,
    /// it is a forgery); (2) the origin digest must be a well-formed
    /// lowercase SHA-256 hex; (3) the row digest pins the row.
    pub fn build(
        carrier: &str,
        origin_digest_hex: &str,
        revision: u64,
        recorded_at: u64,
    ) -> Result<Self, String> {
        let spec = carrier_spec(carrier).ok_or_else(|| {
            format!(
                "provenance record: '{carrier}' is not a registered I/O carrier — \
                 the registry is the closed vocabulary (ADR-0207 §4, №873)"
            )
        })?;
        if !is_sha256_hex(origin_digest_hex) {
            return Err(format!(
                "provenance record: the origin digest must be a lowercase SHA-256 hex (64 chars), got {:?}",
                origin_digest_hex
            ));
        }
        let mut m = Self {
            carrier: carrier.to_string(),
            channel: spec.channel,
            direction: spec.direction,
            origin_digest: origin_digest_hex.to_string(),
            revision,
            recorded_at,
            record_digest: String::new(),
        };
        m.record_digest = record_digest_of(&m);
        Ok(m)
    }

    /// The freshness/use rule (ADR-0207 §2.4, the G5 digest-pinning
    /// form of №828): the observed bytes re-hash to the pinned origin
    /// digest, or the record does not vouch for them. A substituted
    /// digest — the bytes changed between the crossing and the use —
    /// refuses LOUDLY (the tamper control (б) of №873).
    pub fn verify_bytes(&self, observed: &[u8]) -> Result<(), String> {
        let observed_digest = origin_digest(observed);
        if observed_digest == self.origin_digest {
            return Ok(());
        }
        Err(format!(
            "provenance digest mismatch: the record {} pins {}, the observed bytes hash {} — \
             the bytes changed between the crossing and the use (the G5 digest-pinning rule, \
             ADR-0207 §2.4)",
            self.record_digest, self.origin_digest, observed_digest
        ))
    }
}

/// The append-only provenance map (ADR-0207 §2.3): a crossing adds a
/// row; nothing edits or deletes a row. The rows stay in REVISION
/// order (the record sequence is the map order).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProvenanceMap {
    rows: Vec<ProvenanceManifest>,
}

impl ProvenanceMap {
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    /// Append one manifest (the kernel form — the seam assigns the
    /// revision BEFORE the append). An EXACT duplicate (the same
    /// record digest) refuses: the same row twice is a construction
    /// error, not a record (the attestation-map posture of
    /// ADR-0206 §2.2 — re-crossing the same bytes later is a NEW row
    /// with a new revision).
    pub fn append(&mut self, m: ProvenanceManifest) -> Result<usize, String> {
        if self.rows.iter().any(|r| r.record_digest == m.record_digest) {
            return Err(format!(
                "provenance map: the exact record {} is already in the map (append-only, no duplicates)",
                m.record_digest
            ));
        }
        self.rows.push(m);
        Ok(self.rows.len() - 1)
    }

    /// The read-only row view.
    pub fn rows(&self) -> &[ProvenanceManifest] {
        &self.rows
    }

    /// Every live record vouching for the origin digest (the use-site
    /// lookup: two crossings of the same bytes are two rows — both
    /// vouch, neither contradicts, ADR-0207 §2.3).
    pub fn for_origin(&self, origin_digest: &str) -> Vec<&ProvenanceManifest> {
        self.rows
            .iter()
            .filter(|r| r.origin_digest == origin_digest)
            .collect()
    }

    /// The live-record check the use-site rule reads (ADR-0207 §2.5):
    /// at least one recorded row vouches for the digest.
    pub fn has_origin(&self, origin_digest: &str) -> bool {
        self.rows.iter().any(|r| r.origin_digest == origin_digest)
    }

    /// The next revision the seam hands out (the map length — the
    /// monotone record sequence).
    pub fn next_revision(&self) -> u64 {
        self.rows.len() as u64
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

// ── The executor seam (the approval.rs form): the explicit registry ──
// transaction, the process store, and the boundary record functions.
// This section is NOT the portable kernel of ADR-0207 §3 (it keeps
// process state and reads the wall clock; the pure carrier above
// stays pure).

/// The carrier spec: the channel and direction the registry pins for
/// a shared-layer carrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarrierSpec {
    pub name: &'static str,
    pub channel: Channel,
    pub direction: Direction,
}

/// The wave-77 carrier registry (the EXPLICIT registry transaction,
/// ADR-0207 §4 — no silent shelves, the №845/№861 roster form): the
/// six shared-boundary carriers of the dispatch fact line (read_file
/// 12 src-files, write_file 17, respond 20, http_get 9, http_post 10,
/// print 9 — R55 over main `89d06850`). Every name here is a
/// classified Source/Sink builtin (the totality test pins the
/// reachability against the classification SSOT — a non-carrier name
/// reddens loudly). The shared-path twin (`read_file_or` — the
/// read_file_impl body) rides the SAME boundary record through the
/// shared body; `read_file_tokens` has its OWN read body
/// (builtins/office/text.rs) and joins the registry via an explicit
/// future transaction (the honest-limits note of the wave-77 report).
/// Adding the media carriers (media_save, vision_export, …) to the
/// registry is the ADR-0207 §5 boundary — the media domain is
/// consumed, never rewritten.
pub const PROVENANCE_CARRIERS: &[CarrierSpec] = &[
    CarrierSpec {
        name: "read_file",
        channel: Channel::File,
        direction: Direction::Ingress,
    },
    CarrierSpec {
        name: "write_file",
        channel: Channel::File,
        direction: Direction::Egress,
    },
    CarrierSpec {
        name: "http_get",
        channel: Channel::Network,
        direction: Direction::Ingress,
    },
    CarrierSpec {
        name: "http_post",
        channel: Channel::Network,
        direction: Direction::Egress,
    },
    CarrierSpec {
        name: "respond",
        channel: Channel::Respond,
        direction: Direction::Egress,
    },
    CarrierSpec {
        name: "print",
        channel: Channel::Print,
        direction: Direction::Egress,
    },
];

/// The registry lookup (the closed-vocabulary check the record
/// constructor and the wiring both read).
pub fn carrier_spec(name: &str) -> Option<&'static CarrierSpec> {
    PROVENANCE_CARRIERS.iter().find(|s| s.name == name)
}

fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// The in-process provenance store (ADR-0207 §2.3 — append-only; the
/// approval-store seam form). The lock error propagates (the house
/// `with_conn` form) — the wiring turns it into a loud stderr note
/// (№873: the record failure never flips the outcome; №874 flips the
/// EGRESS outcome fail-closed — the ADR-0207 §2.5 rule).
fn provenance_store() -> &'static std::sync::Mutex<ProvenanceMap> {
    static MAP: std::sync::OnceLock<std::sync::Mutex<ProvenanceMap>> = std::sync::OnceLock::new();
    MAP.get_or_init(|| std::sync::Mutex::new(ProvenanceMap::new()))
}

fn lock_store() -> Result<std::sync::MutexGuard<'static, ProvenanceMap>, String> {
    provenance_store()
        .lock()
        .map_err(|e| format!("the provenance store lock: {}", e))
}

/// Record one INGRESS crossing at the boundary (read_file /
/// http_get and the shared-path twins). The origin digest is computed
/// over the exact bytes the program receives. The revision is the
/// store's monotone sequence; the moment is the wall clock NOW (the
/// executor seam — the kernel never reads a clock).
pub fn ingest(carrier: &str, bytes: &[u8]) -> Result<ProvenanceManifest, String> {
    let digest = origin_digest(bytes);
    let mut store = lock_store()?;
    let revision = store.next_revision();
    let m = ProvenanceManifest::build(carrier, &digest, revision, now_epoch())?;
    store.append(m.clone())?;
    Ok(m)
}

/// Record one EGRESS crossing at the boundary (write_file / http_post
/// / respond / print) — BEFORE the side effect completes (ADR-0207
/// §2.2; №874 makes the record a CONDITION of the side effect, №873
/// records it beside).
pub fn emit(carrier: &str, bytes: &[u8]) -> Result<ProvenanceManifest, String> {
    ingest(carrier, bytes)
}

// ── №874 (wave 77 — W-F, P1, [provenance][core]): the coverage rule ──
//
// The fail-closed egress gate (ADR-0207 §2.5): an egress carrier whose
// boundary record did not land does NOT perform the side effect — the
// refusal is loud, names «provenance missing», and journals itself
// through the EXISTING Ledger ACTION path (`ledger::record`, the
// fire_on_deny form of ADR-0167 §3.4 — no new record kind, the
// action/detail ride the existing fields). The deny CLASS stays the
// frozen SINK_CLEARANCE (№385 — the 14-word vocabulary does not grow).
// The ingress side of the rule is the USE-site check (`require` /
// `verify_origin`): a value whose crossing is not recorded refuses ON
// USE — «молча пропустить» is forbidden (the №584/№601 posture).

/// The canonical egress refusal text (the frozen class word + the
/// typed detail — the honest refusal form of №845/№861).
fn provenance_missing_refusal(carrier: &str, why: &str) -> String {
    format!(
        "SINK_CLEARANCE: provenance missing: '{carrier}' performed no live boundary record \
         ({why}) — the egress side effect is refused (ADR-0207 §2.5, №874: fail-closed, \
         a silent pass is forbidden)"
    )
}

/// THE coverage gate for the EGRESS carriers (№874/ADR-0207 §2.5):
/// the boundary record MUST land before the side effect completes.
/// `Ok(manifest)` — the record is live, the side effect may proceed;
/// `Err` — the fail-closed refusal: the caller MUST NOT perform the
/// side effect, the refusal already journaled itself through the
/// existing Ledger ACTION path (action `deny.SINK_CLEARANCE`, scope
/// `provenance`, the typed detail hashed into args — the №393 form).
pub fn gate_emit(carrier: &str, bytes: &[u8]) -> Result<ProvenanceManifest, String> {
    match emit(carrier, bytes) {
        Ok(m) => Ok(m),
        Err(e) => {
            crate::ledger::record(
                "deny.SINK_CLEARANCE",
                "runtime",
                "provenance",
                &format!("{carrier}|provenance missing|{e}"),
            );
            eprintln!(
                "[PROVENANCE] {carrier}: the egress record failed — the side effect is refused: {e}"
            );
            Err(provenance_missing_refusal(carrier, &e))
        }
    }
}

/// The use-site lookup (ADR-0207 §2.4): at least one recorded row
/// vouches for the digest, or the use refuses «provenance missing».
/// The refusal TEXT names the remedy; the deny CLASS stays the frozen
/// SINK_CLEARANCE (№385 — no new words; №874 composes the egress
/// gate over this check).
pub fn require(origin_digest_hex: &str) -> Result<(), String> {
    let store = lock_store()?;
    if store.has_origin(origin_digest_hex) {
        return Ok(());
    }
    Err(format!(
        "SINK_CLEARANCE: provenance missing: no recorded I/O crossing vouches for the \
         origin digest {} — the shared-layer boundary record is the condition of the use \
         (ADR-0207 §2.5, №874: fail-closed, a silent pass is forbidden)",
        origin_digest_hex
    ))
}

/// The substitution check (the G5 form, ADR-0207 §2.4): the record
/// for the pinned digest must exist AND the observed bytes must
/// re-hash to it. The use of tampered bytes refuses even when a row
/// with the pinned digest exists (the digest-pinning negative
/// control of №828).
pub fn verify_origin(origin_digest_hex: &str, observed: &[u8]) -> Result<(), String> {
    require(origin_digest_hex)?;
    let observed_digest = origin_digest(observed);
    if observed_digest == origin_digest_hex {
        return Ok(());
    }
    Err(format!(
        "SINK_CLEARANCE: provenance digest mismatch: the pinned origin {} does not match the \
         observed bytes ({}) — the bytes changed between the crossing and the use \
         (the G5 digest-pinning rule, ADR-0207 §2.4)",
        origin_digest_hex, observed_digest
    ))
}

/// The process-map read-only view (the diagnostic read — the
/// capability_state posture: a read consumes and mutates nothing). A
/// lock error reads an empty map (a diagnostic never lies loudly —
/// the honest count is the store's).
pub fn snapshot() -> ProvenanceMap {
    lock_store().map(|s| s.clone()).unwrap_or_default()
}

/// The test-harness reset seam: the store is process-global, the
/// serial e2e harness resets it between scenarios (the approval-store
/// seam posture — a serve process never calls this, documented).
pub fn store_reset() {
    if let Ok(mut store) = lock_store() {
        *store = ProvenanceMap::new();
    }
}

#[cfg(test)]
mod kernel_tests {
    //! The negative controls are part of the CONTRACT (ADR-0207 §4,
    //! the №860 form): the refusals run at construction/verification
    //! time, not only in the integration suite.

    use super::*;

    const GOOD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn build_refuses_an_unregistered_carrier_and_a_malformed_digest() {
        // (1) the registry is the closed vocabulary — a forged carrier
        // name is not a crossing (the media/vision surfaces stay out
        // of the shared registry, ADR-0207 §5).
        let err = ProvenanceManifest::build("media_save", GOOD, 0, 100).unwrap_err();
        assert!(
            err.contains("not a registered I/O carrier"),
            "the unregistered carrier refuses at construction, got: {err}"
        );
        // (2) the digest is the G5 pin — a malformed one refuses.
        for bad in ["", "zz", "AAAA", "a", &"a".repeat(65)] {
            let err = ProvenanceManifest::build("read_file", bad, 0, 100).unwrap_err();
            assert!(
                err.contains("SHA-256"),
                "the bad digest {bad:?} refuses at construction, got: {err}"
            );
        }
    }

    #[test]
    fn verify_bytes_refuses_a_substituted_digest() {
        // The tamper control (б): the bytes changed between the
        // crossing and the use — the record does not vouch for them.
        let m = ProvenanceManifest::build("read_file", &origin_digest(b"payload"), 0, 100).unwrap();
        assert_eq!(m.channel, Channel::File);
        assert_eq!(m.direction, Direction::Ingress);
        assert!(m.verify_bytes(b"payload").is_ok());
        let err = m.verify_bytes(b"tampered").unwrap_err();
        assert!(
            err.contains("digest mismatch"),
            "the substituted bytes refuse the verify, got: {err}"
        );
    }

    #[test]
    fn the_map_is_append_only_and_two_records_of_the_same_bytes_agree() {
        let mut map = ProvenanceMap::new();
        let a = ProvenanceManifest::build("read_file", &origin_digest(b"x"), 0, 100).unwrap();
        let b = ProvenanceManifest::build("write_file", &origin_digest(b"x"), 1, 101).unwrap();
        assert_eq!(map.append(a).unwrap(), 0);
        assert_eq!(map.append(b).unwrap(), 1);
        assert_eq!(map.len(), 2);
        // two crossings of the same bytes: both rows vouch, neither
        // contradicts (ADR-0207 §2.3 — the consistency posture).
        assert_eq!(map.for_origin(&origin_digest(b"x")).len(), 2);
        assert!(map.has_origin(&origin_digest(b"x")));
        assert!(!map.has_origin(&origin_digest(b"y")));
        // the exact duplicate refuses (append-only)
        let dup = ProvenanceManifest::build("read_file", &origin_digest(b"x"), 0, 100).unwrap();
        assert!(
            map.append(dup).unwrap_err().contains("already in the map"),
            "the exact duplicate refuses"
        );
    }

    #[test]
    fn the_record_digest_pins_the_row_and_is_deterministic() {
        let m = ProvenanceManifest::build("respond", &origin_digest(b"body"), 7, 123).unwrap();
        assert_eq!(m.channel, Channel::Respond);
        assert_eq!(m.direction, Direction::Egress);
        assert_eq!(record_digest_of(&m), m.record_digest);
        let m2 = ProvenanceManifest::build("respond", &origin_digest(b"body"), 7, 123).unwrap();
        assert_eq!(m.record_digest, m2.record_digest, "deterministic");
        // any changed part changes the digest
        let m3 = ProvenanceManifest::build("respond", &origin_digest(b"body"), 8, 123).unwrap();
        assert_ne!(m.record_digest, m3.record_digest, "the revision is pinned");
        let m4 = ProvenanceManifest::build("print", &origin_digest(b"body"), 7, 123).unwrap();
        assert_ne!(m.record_digest, m4.record_digest, "the channel is pinned");
    }

    #[test]
    fn the_registry_is_the_six_shared_carriers_with_pinned_directions() {
        let names: Vec<&str> = PROVENANCE_CARRIERS.iter().map(|s| s.name).collect();
        assert_eq!(
            names,
            [
                "read_file",
                "write_file",
                "http_get",
                "http_post",
                "respond",
                "print"
            ],
            "the wave-77 registry transaction is explicit (ADR-0207 §4)"
        );
        for s in PROVENANCE_CARRIERS {
            match s.name {
                "read_file" | "http_get" => assert_eq!(s.direction, Direction::Ingress),
                _ => assert_eq!(s.direction, Direction::Egress),
            }
        }
        // the channel classes are pinned (the classification SSOT
        // agrees — the totality test in the integration suite pins
        // the roles)
        assert_eq!(carrier_spec("read_file").unwrap().channel, Channel::File);
        assert_eq!(carrier_spec("http_post").unwrap().channel, Channel::Network);
        assert_eq!(carrier_spec("respond").unwrap().channel, Channel::Respond);
        assert_eq!(carrier_spec("print").unwrap().channel, Channel::Print);
        assert!(
            carrier_spec("media_save").is_none(),
            "the media domain stays out (ADR-0207 §5)"
        );
    }

    #[test]
    fn the_missing_refusal_names_the_frozen_class() {
        // The «provenance missing» refusal rides the EXISTING
        // SINK_CLEARANCE class — the vocabulary does not grow (№385).
        let digest = origin_digest(b"never-crossed");
        let err = require(&digest).unwrap_err();
        assert!(
            err.starts_with("SINK_CLEARANCE"),
            "the refusal carries the frozen class word, got: {err}"
        );
        assert!(err.contains("provenance missing"));
        // the substitution check refuses on a foreign digest too
        let err = verify_origin(&digest, b"other-bytes").unwrap_err();
        assert!(err.starts_with("SINK_CLEARANCE"), "got: {err}");
    }

    #[test]
    fn the_ingest_emit_seam_records_and_the_lookup_finds() {
        store_reset();
        let payload = b"the seam records the crossing";
        let m = ingest("read_file", payload).unwrap();
        assert_eq!(m.origin_digest, origin_digest(payload));
        assert_eq!(m.direction, Direction::Ingress);
        let e = emit("write_file", payload).unwrap();
        assert_eq!(e.direction, Direction::Egress);
        assert_eq!(e.channel, Channel::File);
        // revisions are monotone
        assert!(e.revision > m.revision);
        // the lookups find both
        let snap = snapshot();
        assert_eq!(snap.for_origin(&origin_digest(payload)).len(), 2);
        assert!(require(&origin_digest(payload)).is_ok());
        assert!(verify_origin(&origin_digest(payload), payload).is_ok());
        // the use-site rule refuses a never-crossed digest
        assert!(require(&origin_digest(b"never")).is_err());
        store_reset();
        assert!(snapshot().is_empty(), "the reset seam clears the store");
    }
}
