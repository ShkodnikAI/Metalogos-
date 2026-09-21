// ── Naryad #393 (P0, security/ledger, ADR-0167): Action Ledger v1 ─────
//
// The signed, append-only journal of actions. Every record is chained by
// prev-hash and signed with Ed25519 (`ed25519-dalek`) — the INTEGRITY
// upgrade the grant ledger comment promised verbatim:
//   "Signing (prev-hash + Ed25519, in-toto/PROV profile) is naryad #393:
//    it upgrades the INTEGRITY of this ledger, not its semantics."
//
// Structure (ADR-0167 §3.1-3.2):
//   record = { seq, ts, kind, actor, action, scope, args_hash, prev_hash,
//              new_pubkey?, key_id, pubkey, hash, sig }
//   hash   = SHA-256 over the canonical body (all fields except hash+sig)
//   sig    = Ed25519 over the hash bytes — EVERY record is signed
//   signer continuity: rotation records are signed by the currently active
//   key and name the next key; every later record must use it.
//
// Integration is a SIDE EFFECT of the actions themselves (ADR-0167 §3.4):
// grant lifecycle (src/grants.rs::record_event), runtime deny events
// (fire_on_deny TW + vm_fire_on_deny VM), successful irreversible actions
// (invoke_db_execute_with_grant post-success), session lifecycle (server).
//
// Honest posture (ADR-0167 §6-7): tamper-EVIDENT, not tamper-proof —
// integrity holds given an externally anchored head/key; post-host-
// compromise write integrity is out of scope. Ledger write failures inside
// action paths are loud stderr but never flip the action's outcome; the
// explicit builtins (ledger_export) refuse loudly.
//
// The in-toto/PROV export profile is ADR-0157.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::sync::{Mutex, OnceLock};

/// The genesis prev_hash: 64 hex zeros (ADR-0167 §3.2).
pub const GENESIS_PREV_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// Schema version marker exported into the verifier report.
pub const LEDGER_SCHEMA_VERSION: u32 = 1;

/// Environment variable carrying a hex signing seed (test/CI affordance,
/// ADR-0167 §3.3). 64 hex chars = 32 bytes.
pub const LEDGER_KEY_ENV: &str = "METALOGOS_LEDGER_KEY";

// ── Record ─────────────────────────────────────────────────────────────

/// One ledger record (ADR-0167 §3.1). Field order IS the canonical
/// serialization order — do not reorder without a schema-version bump.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct LedgerRecord {
    pub seq: u64,
    pub ts: u64,
    /// "action" | "key_rotation" | "snapshot"
    pub kind: String,
    #[serde(default)]
    pub actor: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub args_hash: String,
    pub prev_hash: String,
    /// key_rotation only: the public key taking over (hex).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub new_pubkey: String,
    /// 16 hex chars of SHA-256 over the signer's public key.
    pub key_id: String,
    /// Signer's Ed25519 public key (hex).
    pub pubkey: String,
    pub hash: String,
    pub sig: String,
}

/// The canonical body: every field except `hash` and `sig` participates
/// in the record hash (ADR-0167 §3.2). Declared field order matches
/// `LedgerRecord` so serde_json produces the same canonical bytes.
#[derive(serde::Serialize)]
struct LedgerBody<'a> {
    seq: u64,
    ts: u64,
    kind: &'a str,
    actor: &'a str,
    action: &'a str,
    scope: &'a str,
    args_hash: &'a str,
    prev_hash: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_pubkey: Option<&'a str>,
    key_id: &'a str,
    pubkey: &'a str,
}

/// Canonical JSON of the record body (the hash preimage).
pub fn canonical_body(r: &LedgerRecord) -> String {
    let body = LedgerBody {
        seq: r.seq,
        ts: r.ts,
        kind: &r.kind,
        actor: &r.actor,
        action: &r.action,
        scope: &r.scope,
        args_hash: &r.args_hash,
        prev_hash: &r.prev_hash,
        new_pubkey: if r.new_pubkey.is_empty() {
            None
        } else {
            Some(&r.new_pubkey)
        },
        key_id: &r.key_id,
        pubkey: &r.pubkey,
    };
    serde_json::to_string(&body).unwrap_or_default()
}

/// SHA-256 hex of a byte slice — the one hash primitive both the writer
/// and the verifier use.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    hex::encode(h.finalize())
}

/// Recompute the record hash from the body (hex).
pub fn record_hash(r: &LedgerRecord) -> String {
    sha256_hex(canonical_body(r).as_bytes())
}

/// The key id: first 16 hex chars of SHA-256 over the public key bytes.
pub fn key_id_of(pubkey_hex: &str) -> Option<String> {
    let bytes = decode_hex_32_plus(pubkey_hex)?;
    Some(sha256_hex(&bytes)[..16].to_string())
}

/// Decode a hex string into bytes (any even length).
fn decode_hex_32_plus(s: &str) -> Option<Vec<u8>> {
    if s.is_empty() || !s.len().is_multiple_of(2) {
        return None;
    }
    hex::decode(s).ok()
}

/// Deterministic detail hash for an action's metadata tuple — the
/// preimage never enters the journal (ADR-0167 §2 driver 6).
pub fn args_hash_of(detail: &str) -> String {
    sha256_hex(detail.as_bytes())
}

// ── Runtime store (the consent_ledger template) ────────────────────────

struct LedgerState {
    conn: Option<rusqlite::Connection>,
    signing: SigningKey,
    /// Currently active public key hex (changes on rotation).
    active_pubkey: String,
    /// Head cache: (last seq, last hash). `None` = the empty chain sits at
    /// the genesis position — the FIRST record takes seq 0 (ADR-0167 §3:
    /// "Genesis. seq = 0"; the 1-based slip made fresh runtime exports
    /// unverifiable by the external verifier, caught by the wave-3
    /// kitchen-camera acceptance e2e).
    head_seq: Option<u64>,
    head_hash: String,
    /// Count of loud ledger-write failures on action paths (observability).
    write_failures: u64,
}

fn ledger_state() -> &'static Mutex<LedgerState> {
    static LEDGER: OnceLock<Mutex<LedgerState>> = OnceLock::new();
    LEDGER.get_or_init(|| {
        let conn = rusqlite::Connection::open_in_memory()
            .ok()
            .and_then(|c| {
                c.execute_batch(
                    "CREATE TABLE IF NOT EXISTS action_ledger (
                        seq INTEGER PRIMARY KEY,
                        ts INTEGER NOT NULL,
                        kind TEXT NOT NULL,
                        actor TEXT NOT NULL DEFAULT '',
                        action TEXT NOT NULL DEFAULT '',
                        scope TEXT NOT NULL DEFAULT '',
                        args_hash TEXT NOT NULL DEFAULT '',
                        prev_hash TEXT NOT NULL,
                        new_pubkey TEXT NOT NULL DEFAULT '',
                        key_id TEXT NOT NULL,
                        pubkey TEXT NOT NULL,
                        hash TEXT NOT NULL,
                        sig TEXT NOT NULL
                    );",
                )
                .map(|_| c)
                .ok()
            });
        let signing = load_or_generate_signing_key();
        let active_pubkey = hex::encode(signing.verifying_key().as_bytes());
        // Recover the head from an existing store (rotation across an
        // in-process re-init keeps the chain consistent).
        let (head_seq, head_hash, stored_active) = conn
            .as_ref()
            .and_then(|c| {
                c.query_row(
                    "SELECT seq, hash, pubkey FROM action_ledger ORDER BY seq DESC LIMIT 1",
                    [],
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?.max(0) as u64,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )
                .ok()
            })
            .map(|(s, h, p)| (Some(s), h, Some(p)))
            .unwrap_or((None, GENESIS_PREV_HASH.to_string(), None));
        // The active key on recovery: the last rotation target if any.
        let active = conn
            .as_ref()
            .and_then(|c| {
                c.query_row(
                    "SELECT new_pubkey FROM action_ledger WHERE kind='key_rotation' ORDER BY seq DESC LIMIT 1",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .ok()
            })
            .or(stored_active)
            .unwrap_or(active_pubkey.clone());
        Mutex::new(LedgerState {
            conn,
            signing,
            active_pubkey: active,
            head_seq,
            head_hash,
            write_failures: 0,
        })
    })
}

fn load_or_generate_signing_key() -> SigningKey {
    if let Ok(seed_hex) = std::env::var(LEDGER_KEY_ENV) {
        if let Ok(seed) = hex::decode(seed_hex.trim()) {
            if seed.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&seed);
                return SigningKey::from_bytes(&arr);
            }
        }
        eprintln!(
            "[LEDGER] WARNING: {} is set but is not 64 hex chars — generating a fresh key",
            LEDGER_KEY_ENV
        );
    }
    // OS randomness via the already-present `rand` crate (ADR-0167 §2
    // driver 4: no new RNG dependency).
    let seed: [u8; 32] = rand::random();
    SigningKey::from_bytes(&seed)
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ── Append (the writer) ────────────────────────────────────────────────

/// Append one record of the given kind, signed by the active key.
/// `new_pubkey` is set only for key_rotation records. Returns the stored
/// record. Errors are loud and typed — the explicit builtins surface them.
pub fn append_record(
    kind: &str,
    actor: &str,
    action: &str,
    scope: &str,
    args_hash: &str,
    new_pubkey: Option<&str>,
) -> Result<LedgerRecord, String> {
    let mut guard = ledger_state()
        .lock()
        .map_err(|e| format!("action ledger lock: {}", e))?;
    let conn = guard.conn.as_ref().ok_or_else(|| {
        "action ledger unavailable (in-memory sqlite failed to initialize)".to_string()
    })?;
    let pubkey = hex::encode(guard.signing.verifying_key().as_bytes());
    let record = LedgerRecord {
        seq: match guard.head_seq {
            None => 0,
            Some(s) => s + 1,
        },
        ts: now_secs(),
        kind: kind.to_string(),
        actor: actor.to_string(),
        action: action.to_string(),
        scope: scope.to_string(),
        args_hash: args_hash.to_string(),
        prev_hash: guard.head_hash.clone(),
        new_pubkey: new_pubkey.unwrap_or("").to_string(),
        key_id: key_id_of(&pubkey).unwrap_or_default(),
        pubkey: pubkey.clone(),
        hash: String::new(),
        sig: String::new(),
    };
    let hash = record_hash(&record);
    let sig = guard.signing.sign(hash.as_bytes());
    let record = LedgerRecord {
        hash,
        sig: hex::encode(sig.to_bytes()),
        ..record
    };
    conn.execute(
        "INSERT INTO action_ledger (seq, ts, kind, actor, action, scope, args_hash, prev_hash, new_pubkey, key_id, pubkey, hash, sig)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        rusqlite::params![
            record.seq as i64,
            record.ts as i64,
            record.kind,
            record.actor,
            record.action,
            record.scope,
            record.args_hash,
            record.prev_hash,
            record.new_pubkey,
            record.key_id,
            record.pubkey,
            record.hash,
            record.sig,
        ],
    )
    .map_err(|e| format!("action ledger insert: {}", e))?;
    guard.head_seq = Some(record.seq);
    guard.head_hash = record.hash.clone();
    Ok(record)
}

/// Best-effort action-path recording (ADR-0167 §2 driver 5): a failure is
/// loud on stderr and counted, never flips the action's outcome.
pub fn record(action: &str, actor: &str, scope: &str, detail: &str) {
    let args_hash = args_hash_of(detail);
    let res = append_record("action", actor, action, scope, &args_hash, None);
    if let Err(e) = res {
        eprintln!("[LEDGER] WARNING: action-path ledger write failed: {}", e);
        if let Ok(mut guard) = ledger_state().lock() {
            guard.write_failures += 1;
        }
    }
}

/// Current record count (in-process read — not egress).
pub fn count() -> Result<u64, String> {
    let guard = ledger_state()
        .lock()
        .map_err(|e| format!("action ledger lock: {}", e))?;
    let conn = guard
        .conn
        .as_ref()
        .ok_or_else(|| "action ledger unavailable".to_string())?;
    conn.query_row("SELECT COUNT(*) FROM action_ledger", [], |r| {
        r.get::<_, i64>(0)
    })
    .map(|n| n.max(0) as u64)
    .map_err(|e| format!("action ledger count: {}", e))
}

/// Current head hash ("" for an empty chain) — designed to be published
/// out-of-band (the external anchor, ADR-0167 §7).
pub fn head_hash() -> Result<String, String> {
    let guard = ledger_state()
        .lock()
        .map_err(|e| format!("action ledger lock: {}", e))?;
    Ok(if guard.head_seq.is_none() {
        String::new()
    } else {
        guard.head_hash.clone()
    })
}

/// Number of loud action-path write failures (observability surface).
pub fn write_failure_count() -> u64 {
    ledger_state().lock().map(|g| g.write_failures).unwrap_or(0)
}

/// Key rotation (ADR-0167 §3.3): generate a fresh key, append a
/// key_rotation record signed by the CURRENT key, switch the active key.
/// Returns the new key id.
pub fn rotate() -> Result<String, String> {
    let seed_bytes: [u8; 32] = rand::random();
    let new_key = SigningKey::from_bytes(&seed_bytes);
    let new_pubkey = hex::encode(new_key.verifying_key().as_bytes());
    let new_key_id = key_id_of(&new_pubkey).unwrap_or_default();
    // The rotation record is signed by the STILL-ACTIVE (old) key —
    // append_record signs before the swap below, so signer continuity in
    // the chain is: ...old... old(sigs the rotation) new new new ...
    append_record(
        "key_rotation",
        "runtime",
        "ledger.rotate",
        "",
        &args_hash_of(&new_pubkey),
        Some(&new_pubkey),
    )?;
    {
        let mut guard = ledger_state()
            .lock()
            .map_err(|e| format!("action ledger lock: {}", e))?;
        guard.signing = new_key;
        guard.active_pubkey = new_pubkey.clone();
    }
    Ok(new_key_id)
}

/// Snapshot (ADR-0167 §3.2): append a snapshot record pinning the head.
/// Returns the snapshot record's hash (an anchor for `mlog ledger archive`).
pub fn snapshot() -> Result<String, String> {
    let rec = append_record(
        "snapshot",
        "runtime",
        "ledger.snapshot",
        "",
        &args_hash_of("snapshot"),
        None,
    )?;
    Ok(rec.hash)
}

// ── Export (the builtins read this; writing the file is FILE EGRESS) ──

/// All records in seq order.
pub fn all_records() -> Result<Vec<LedgerRecord>, String> {
    let guard = ledger_state()
        .lock()
        .map_err(|e| format!("action ledger lock: {}", e))?;
    let conn = guard
        .conn
        .as_ref()
        .ok_or_else(|| "action ledger unavailable".to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT seq, ts, kind, actor, action, scope, args_hash, prev_hash, new_pubkey, key_id, pubkey, hash, sig
             FROM action_ledger ORDER BY seq",
        )
        .map_err(|e| format!("action ledger export: {}", e))?;
    let rows = stmt
        .query_map([], map_row)
        .map_err(|e| format!("action ledger export: {}", e))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| format!("action ledger export: {}", e))?);
    }
    Ok(out)
}

fn map_row(row: &rusqlite::Row) -> rusqlite::Result<LedgerRecord> {
    Ok(LedgerRecord {
        seq: row.get::<_, i64>(0)?.max(0) as u64,
        ts: row.get::<_, i64>(1)?.max(0) as u64,
        kind: row.get(2)?,
        actor: row.get(3)?,
        action: row.get(4)?,
        scope: row.get(5)?,
        args_hash: row.get(6)?,
        prev_hash: row.get(7)?,
        new_pubkey: row.get(8)?,
        key_id: row.get(9)?,
        pubkey: row.get(10)?,
        hash: row.get(11)?,
        sig: row.get(12)?,
    })
}

/// The verifiable JSONL chain export (ADR-0167 §3.5) — one record per
/// line, canonical field order. The CALLER performs the sandboxed file
/// write (the FILE EGRESS half, `ledger_export`).
pub fn export_jsonl() -> Result<String, String> {
    let records = all_records()?;
    Ok(records_to_jsonl(&records))
}

/// Serialize records to JSONL (used by the exporter and the tests).
pub fn records_to_jsonl(records: &[LedgerRecord]) -> String {
    let mut out = String::new();
    for r in records {
        out.push_str(
            &serde_json::to_string(r)
                .unwrap_or_else(|_| "{\"error\":\"unserializable record\"}".to_string()),
        );
        out.push('\n');
    }
    out
}

/// Parse JSONL back into records (the verifier's front door).
pub fn records_from_jsonl(content: &str) -> Result<Vec<LedgerRecord>, String> {
    let mut out = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let r: LedgerRecord = serde_json::from_str(line)
            .map_err(|e| format!("line {}: not a ledger record: {}", i + 1, e))?;
        out.push(r);
    }
    Ok(out)
}

/// The in-toto Statement profile export (ADR-0157 §2) — one Statement per
/// line. The CALLER performs the sandboxed file write.
pub fn export_intoto() -> Result<String, String> {
    let records = all_records()?;
    Ok(records_to_intoto(&records))
}

/// Serialize records to the in-toto Statement stream (ADR-0157 §2).
pub fn records_to_intoto(records: &[LedgerRecord]) -> String {
    let mut out = String::new();
    for r in records {
        let mut predicate = serde_json::json!({
            "seq": r.seq,
            "ts": r.ts,
            "kind": r.kind,
            "actor": r.actor,
            "action": r.action,
            "scope": r.scope,
            "prevHash": r.prev_hash,
            "recordHash": r.hash,
            "keyId": r.key_id,
            "signature": r.sig,
        });
        if !r.new_pubkey.is_empty() {
            predicate["newPubkey"] = serde_json::json!(r.new_pubkey);
        }
        let statement = serde_json::json!({
            "_type": "https://in-toto.io/Statement/v0.1",
            "subject": [{
                "name": format!("metalogos:action:{}@{}", r.action, r.seq),
                "digest": { "sha256": r.args_hash },
            }],
            "predicateType": "https://metalogos.dev/attestations/action-ledger/v1",
            "predicate": predicate,
        });
        out.push_str(&statement.to_string());
        out.push('\n');
    }
    out
}

// ── Verification (the external contract — pure file reading) ──────────

/// The verifier report (what a successful verification asserts).
#[derive(Debug, Clone, serde::Serialize)]
pub struct VerifyReport {
    pub schema_version: u32,
    pub records: u64,
    pub head_hash: String,
    pub distinct_keys: u64,
    pub anchored_start: bool,
}

/// Verify a record sequence (ADR-0167 §3.6). `expect_head` / `expect_key`
/// pin the out-of-band anchors (§7): a self-consistent full rewrite under
/// a fresh key is detected ONLY against them.
///
/// String-error facade over [`verify_records_structural`] (the №415 runtime
/// hook needs the STRUCTURAL verdict; the CLI/legacy callers keep theirs).
pub fn verify_records(
    records: &[LedgerRecord],
    expect_head: Option<&str>,
    expect_key: Option<&str>,
) -> Result<VerifyReport, String> {
    verify_records_structural(records, expect_head, expect_key).map_err(|f| f.to_string())
}

// ── Naryad #415 (P1, security/ledger): the structural runtime hook ─────
//
// The audit P2-2 residue of №393: verification existed only as a
// library call with STRING errors (`Err(String)` — the position of the
// first fault had to be guessed out of the message text). The №415
// contract: a deterministic STRUCTURAL verdict (ok/fail + the position
// of the first fault + the reason), aggregated over the SAME crypto
// checks — nothing about the record format, hashing or signatures is
// re-implemented here (the single implementation stays in
// `verify_records_structural` below).

/// A structural verification fault (№415): the 1-based position of the
/// first offending record in the supplied sequence (`None` = the failure
/// is chain-level: parse, start rule, or the head anchor) plus the reason
/// WITHOUT the positional prefix.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct VerifyFault {
    /// 1-based record position in the supplied sequence; `None` when the
    /// failure is chain-level (parse / start rule / head anchor).
    pub record: Option<u64>,
    /// What exactly failed (no "record N: " prefix — the position is the
    /// `record` field's job).
    pub reason: String,
}

impl std::fmt::Display for VerifyFault {
    /// The display keeps the historical message format of the string
    /// errors byte-for-byte (the CLI and the №393 tests print these).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.record {
            Some(n) => write!(f, "record {}: {}", n, self.reason),
            None => write!(f, "{}", self.reason),
        }
    }
}

/// The structural verdict of the runtime hook (№415): ok/fail plus the
/// same stats a successful `verify_records` reports, plus the first fault
/// on failure. No string guessing — the position is a field, not prose.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct LedgerVerdict {
    /// `true` iff the chain (or the documented empty-ledger case) verifies.
    pub ok: bool,
    pub schema_version: u32,
    /// Record count: the verified chain length, or 0 for the empty case.
    pub records: u64,
    /// The verified head hash ("" for an empty chain or a parse failure).
    pub head_hash: String,
    pub distinct_keys: u64,
    pub anchored_start: bool,
    /// The FIRST fault (verification stops at the first failure).
    pub fault: Option<VerifyFault>,
}

/// Where the hook reads the chain from (№415: путь/снапшот → вердикт).
pub enum LedgerVerifySource<'a> {
    /// Raw JSONL content (an exported chain snapshot held in memory).
    Jsonl(&'a str),
    /// Path to an exported JSONL ledger file (READ-ONLY — the hook never
    /// writes; the caller's sandbox discipline governs how the path got here).
    File(&'a std::path::Path),
}

/// The structural core of chain verification (ADR-0167 §3.6 checks):
/// start rule, seq continuity, prev-hash linkage, record-hash recompute,
/// key_id↔pubkey match, signer continuity across rotations, Ed25519
/// signatures, kind whitelist, head/key anchors. The ONLY implementation —
/// `verify_records` (string facade) and `ledger_verify` (structural
/// verdict) both delegate here.
fn verify_records_structural(
    records: &[LedgerRecord],
    expect_head: Option<&str>,
    expect_key: Option<&str>,
) -> Result<VerifyReport, VerifyFault> {
    if records.is_empty() {
        return Err(VerifyFault {
            record: None,
            reason: "ledger is empty: no records to verify".to_string(),
        });
    }
    // Start rule: genesis (seq 0) or an anchored snapshot start.
    let first = &records[0];
    let anchored = first.kind == "snapshot";
    if anchored {
        if first.seq == 0 {
            return Err(VerifyFault {
                record: Some(1),
                reason: "snapshot anchor at seq 0 is not a valid anchor (snapshots pin a non-empty head)".to_string(),
            });
        }
        if first.new_pubkey.is_empty() && first.pubkey.is_empty() {
            return Err(VerifyFault {
                record: Some(1),
                reason: "anchored start carries no signer key".to_string(),
            });
        }
    } else {
        if first.seq != 0 {
            return Err(VerifyFault {
                record: Some(1),
                reason: format!(
                    "chain must start at seq 0 or at a snapshot anchor, got seq {}",
                    first.seq
                ),
            });
        }
        if first.prev_hash != GENESIS_PREV_HASH {
            return Err(VerifyFault {
                record: Some(1),
                reason: format!(
                    "genesis prev_hash must be {} zeros, got '{}'",
                    GENESIS_PREV_HASH.len(),
                    first.prev_hash
                ),
            });
        }
    }
    if let Some(expect_key) = expect_key {
        if first.pubkey != expect_key {
            return Err(VerifyFault {
                record: Some(1),
                reason: format!(
                    "anchored signer key mismatch: expected {}, got {}",
                    expect_key, first.pubkey
                ),
            });
        }
    }
    let mut active_pubkey = first.pubkey.clone();
    let mut distinct_keys: std::collections::HashSet<String> = std::collections::HashSet::new();
    distinct_keys.insert(first.key_id.clone());
    let mut prev_hash = first.prev_hash.clone();
    let mut expected_seq = first.seq;
    for (i, r) in records.iter().enumerate() {
        let n = i + 1;
        if r.seq != expected_seq {
            return Err(VerifyFault {
                record: Some(n as u64),
                reason: format!("seq gap (expected {}, got {})", expected_seq, r.seq),
            });
        }
        if r.prev_hash != prev_hash {
            return Err(VerifyFault {
                record: Some(n as u64),
                reason: format!(
                    "prev_hash chain break (expected {}, got {})",
                    prev_hash, r.prev_hash
                ),
            });
        }
        let recomputed = record_hash(r);
        if recomputed != r.hash {
            return Err(VerifyFault {
                record: Some(n as u64),
                reason: "hash mismatch (body modified?)".to_string(),
            });
        }
        let recomputed_key_id = key_id_of(&r.pubkey).unwrap_or_default();
        if recomputed_key_id != r.key_id {
            return Err(VerifyFault {
                record: Some(n as u64),
                reason: "key_id does not match pubkey".to_string(),
            });
        }
        if r.pubkey != active_pubkey {
            return Err(VerifyFault {
                record: Some(n as u64),
                reason: format!(
                    "signer continuity break (signed by {} while active key is {})",
                    r.key_id,
                    key_id_of(&active_pubkey).unwrap_or_default()
                ),
            });
        }
        // Signature over the hash bytes, under the record's declared key.
        let sig_arr: [u8; 64] = decode_hex_32_plus(&r.sig)
            .and_then(|v| <[u8; 64]>::try_from(v).ok())
            .ok_or_else(|| VerifyFault {
                record: Some(n as u64),
                reason: "signature is not 64 bytes of hex".to_string(),
            })?;
        let sig = Signature::from_bytes(&sig_arr);
        let vk_bytes: [u8; 32] = decode_hex_32_plus(&r.pubkey)
            .and_then(|v| <[u8; 32]>::try_from(v).ok())
            .ok_or_else(|| VerifyFault {
                record: Some(n as u64),
                reason: "pubkey is not 32 bytes".to_string(),
            })?;
        let vk = VerifyingKey::from_bytes(&vk_bytes).map_err(|e| VerifyFault {
            record: Some(n as u64),
            reason: format!("malformed pubkey: {}", e),
        })?;
        vk.verify(r.hash.as_bytes(), &sig)
            .map_err(|_| VerifyFault {
                record: Some(n as u64),
                reason: "signature verification FAILED".to_string(),
            })?;
        // Rotation transition (AFTER the record itself verified).
        if r.kind == "key_rotation" {
            if r.new_pubkey.is_empty() {
                return Err(VerifyFault {
                    record: Some(n as u64),
                    reason: "key_rotation without new_pubkey".to_string(),
                });
            }
            active_pubkey = r.new_pubkey.clone();
            distinct_keys.insert(key_id_of(&active_pubkey).unwrap_or_default());
        } else if !r.new_pubkey.is_empty() {
            return Err(VerifyFault {
                record: Some(n as u64),
                reason: format!("new_pubkey set on a {} record", r.kind),
            });
        }
        if r.kind != "action" && r.kind != "key_rotation" && r.kind != "snapshot" {
            return Err(VerifyFault {
                record: Some(n as u64),
                reason: format!("unknown kind '{}'", r.kind),
            });
        }
        distinct_keys.insert(r.key_id.clone());
        prev_hash = r.hash.clone();
        expected_seq = r.seq + 1;
    }
    if let Some(expect_head) = expect_head {
        let head = records.last().map(|r| r.hash.clone()).unwrap_or_default();
        if head != expect_head {
            return Err(VerifyFault {
                record: None,
                reason: format!(
                    "head anchor mismatch: expected {}, got {}",
                    expect_head, head
                ),
            });
        }
    }
    Ok(VerifyReport {
        schema_version: LEDGER_SCHEMA_VERSION,
        records: records.len() as u64,
        head_hash: records.last().map(|r| r.hash.clone()).unwrap_or_default(),
        distinct_keys: distinct_keys.len() as u64,
        anchored_start: anchored,
    })
}

/// The runtime verify hook (№415): a deterministic structural verdict over
/// a signed ledger chain from a read-only source. A wrapper/aggregator —
/// every crypto check lives in `verify_records_structural`; nothing about
/// the wire format or the primitives is duplicated.
///
/// The documented EMPTY-ledger case (a debatable call, pinned by the №415
/// test): a chain with zero records verifies VACUOUSLY as `ok: true` —
/// nothing in it contradicts. The honest caveat: the verifier CANNOT
/// distinguish "no actions were taken" from "all records were deleted"
/// without the out-of-band head anchor (ADR-0167 §7); when `expect_head`
/// is pinned, an empty ledger fails loudly instead (wholesale deletion is
/// exactly what the anchor exists to catch).
pub fn ledger_verify(
    source: LedgerVerifySource<'_>,
    expect_head: Option<&str>,
    expect_key: Option<&str>,
) -> LedgerVerdict {
    let parsed = match &source {
        LedgerVerifySource::Jsonl(content) => {
            records_from_jsonl(content).map_err(|e| VerifyFault {
                record: None,
                reason: e,
            })
        }
        LedgerVerifySource::File(path) => std::fs::read_to_string(path)
            .map_err(|e| VerifyFault {
                record: None,
                reason: format!("cannot read ledger file {}: {}", path.display(), e),
            })
            .and_then(|content| {
                records_from_jsonl(&content).map_err(|e| VerifyFault {
                    record: None,
                    reason: e,
                })
            }),
    };
    let records = match parsed {
        Err(fault) => {
            return LedgerVerdict {
                ok: false,
                schema_version: LEDGER_SCHEMA_VERSION,
                records: 0,
                head_hash: String::new(),
                distinct_keys: 0,
                anchored_start: false,
                fault: Some(fault),
            };
        }
        Ok(records) => records,
    };
    if records.is_empty() {
        // The documented vacuous case; the head anchor turns it loud.
        let fault = if let Some(h) = expect_head {
            Some(VerifyFault {
                record: None,
                reason: format!(
                    "head anchor mismatch on an empty ledger: expected {}, got none (nothing to verify)",
                    h
                ),
            })
        } else if expect_key.is_some() {
            Some(VerifyFault {
                record: None,
                reason:
                    "key anchor mismatch on an empty ledger: no records to carry the signer key"
                        .to_string(),
            })
        } else {
            None
        };
        return LedgerVerdict {
            ok: fault.is_none(),
            schema_version: LEDGER_SCHEMA_VERSION,
            records: 0,
            head_hash: String::new(),
            distinct_keys: 0,
            anchored_start: false,
            fault,
        };
    }
    match verify_records_structural(&records, expect_head, expect_key) {
        Ok(report) => LedgerVerdict {
            ok: true,
            schema_version: report.schema_version,
            records: report.records,
            head_hash: report.head_hash,
            distinct_keys: report.distinct_keys,
            anchored_start: report.anchored_start,
            fault: None,
        },
        Err(fault) => LedgerVerdict {
            ok: false,
            schema_version: LEDGER_SCHEMA_VERSION,
            records: records.len() as u64,
            head_hash: String::new(),
            distinct_keys: 0,
            anchored_start: false,
            fault: Some(fault),
        },
    }
}

/// Verify a JSONL ledger file — the external verifier's entry point
/// (the CLI wraps this; no Metalogos runtime involved).
pub fn verify_file(
    path: &std::path::Path,
    expect_head: Option<&str>,
    expect_key: Option<&str>,
) -> Result<VerifyReport, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read ledger file {}: {}", path.display(), e))?;
    let records = records_from_jsonl(&content)?;
    verify_records(&records, expect_head, expect_key)
}

/// Archive (ADR-0167 §3.6): truncate `<file>` at the snapshot record with
/// `at_seq` (inclusive) into `<out>`; the output starts at the snapshot
/// anchor and is verified before being reported.
pub fn archive_file(
    input: &std::path::Path,
    output: &std::path::Path,
    at_seq: u64,
) -> Result<VerifyReport, String> {
    let content = std::fs::read_to_string(input)
        .map_err(|e| format!("cannot read ledger file {}: {}", input.display(), e))?;
    let records = records_from_jsonl(&content)?;
    let pos = records
        .iter()
        .position(|r| r.seq == at_seq && r.kind == "snapshot")
        .ok_or_else(|| {
            format!(
                "no snapshot record with seq {} in {} (archive anchors only at snapshot records)",
                at_seq,
                input.display()
            )
        })?;
    let slice = &records[pos..];
    let report = verify_records(slice, None, None)
        .map_err(|e| format!("archived slice does not verify: {}", e))?;
    let jsonl = records_to_jsonl(slice);
    std::fs::write(output, jsonl.as_bytes())
        .map_err(|e| format!("cannot write {}: {}", output.display(), e))?;
    Ok(report)
}

// ── Test-chain builder (public: integration tests + fuzz harness) ────

/// Build a fully valid signed chain from `(action, actor, scope)` specs,
/// optionally with `kind` overrides: use `insert_rotations_at` positions
/// to interleave key_rotation records. The signing key is fresh per call
/// (or seeded via `METALOGOS_LEDGER_KEY` when set).
pub fn build_chain(specs: &[(&str, &str, &str)]) -> Vec<LedgerRecord> {
    let signing = load_or_generate_signing_key();
    let pubkey = hex::encode(signing.verifying_key().as_bytes());
    let key_id = key_id_of(&pubkey).unwrap_or_default();
    let mut records = Vec::with_capacity(specs.len());
    let mut prev_hash = GENESIS_PREV_HASH.to_string();
    for (i, (action, actor, scope)) in specs.iter().enumerate() {
        let mut r = LedgerRecord {
            seq: i as u64,
            ts: 1_700_000_000 + i as u64,
            kind: "action".to_string(),
            actor: actor.to_string(),
            action: action.to_string(),
            scope: scope.to_string(),
            args_hash: args_hash_of(&format!("{}|{}|{}", action, actor, scope)),
            prev_hash: prev_hash.clone(),
            new_pubkey: String::new(),
            key_id: key_id.clone(),
            pubkey: pubkey.clone(),
            hash: String::new(),
            sig: String::new(),
        };
        let hash = record_hash(&r);
        let sig = signing.sign(hash.as_bytes());
        r.hash = hash;
        r.sig = hex::encode(sig.to_bytes());
        prev_hash = r.hash.clone();
        records.push(r);
    }
    records
}
