// ── Voice store: SQLite persistence for voiceprints + audio artifacts ──
// Наряд №303 (issue #371, P2/feature/voice) — VOICE_A3_SPEAKER_ENCODER.
//
// Mirrors src/vision/store.rs (Наряд №242) — SQLite BLOB persistence.
// №512 (audit 28.09 C-06): HONESTY FIX — the earlier header claimed
// at-rest encryption while the bytes went through an INSECURE XOR
// placeholder keyed by the PUBLIC name; the real runtime refused to
// persist voiceprints at all ([VOICE_INSECURE_STORE], fail-closed).
// №517 (issue #801; audit C-06 step 2): the honest crypto LANDS — the
// real runtime persists voiceprints encrypted with AES-256-GCM (the SAME
// primitive as the encrypt()/decrypt() builtins, Phase 7.3 — no new
// dependencies), the key arrives through the secret() gate semantics
// (env-sourced, hex-256, NEVER derived from the name), and every write
// carries a fresh random 96-bit nonce. Legacy pre-№517 XOR rows (algo
// NULL) are NOT silently read as decrypted — the load refuses loudly
// with [VOICEPRINT_STALE] ("re-enroll", the №504 migration posture).
// The mock runtime keeps the insecure placeholder path for the skeleton
// tests, with the insecure mark VISIBLE IN THE SCHEMA (algo =
// 'INSECURE-XOR-MOCK'). Voiceprints are biometric data (GDPR Art. 9
// special category); the privacy policy lands in docs/privacy.md (№519).
// Ledger records consent (hash(voiceprint, nonce, date, model)).
// №527 (issue #836; audit 30.09 N-3): the ciphertext becomes BOUND TO
// ITS SUBJECT — the GCM AAD carries the (subject_id, registry, schema
// version) triplet, so a blob transplanted onto another subject's row
// fails authentication (the swap attack the bare tag accepted). The
// key material is wiped under Zeroizing (the decoded 32-byte buffer's
// lifetime is the single operation). The storage schema and the algo
// mark are UNTOUCHED (the №527 boundary): legacy №517 rows (empty AAD)
// stay readable in the announced transition window — every such read
// announces [VOICEPRINT_NO_AAD_LEGACY] on stderr and the honest crypto
// status (LegacyNoAad) is surfaced through load_voiceprint_with_status;
// every WRITE is AAD-bound, so the legacy population only shrinks.
// The deadline row lives in docs/limitations.md (the №524 rule).

use rusqlite::Connection;
use std::sync::Mutex;

/// The schema version label stamped on every №517-encrypted row.
pub(crate) const VOICEPRINT_ALGO_AES_GCM: &str = "AES-256-GCM-v1";
/// The visible-in-schema insecure mark of the mock-runtime placeholder rows.
pub(crate) const VOICEPRINT_ALGO_INSECURE_MOCK: &str = "INSECURE-XOR-MOCK";

/// №527: the schema-version component of the AAD triplet. NOT the schema
/// `algo` mark (the №527 boundaries keep the storage schema untouched):
/// rows carrying the same algo mark can be AAD-bound (№527-era writes) or
/// legacy no-AAD (№517-era writes) — the discriminator is the GCM
/// authentication itself (try-bound first, then the loud transitional
/// fallback), because a blob that authenticates only under the empty AAD
/// IS a legacy row by construction.
pub(crate) const VOICEPRINT_AAD_SCHEMA: &str = "voiceprints-aad-v1";

/// №527: the transitional crypto status of a loaded voiceprint — the
/// "переходный флаг" of the AAD migration, observable by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceprintCryptoStatus {
    /// The row is AAD-bound to its subject (№527): a swapped or misplaced
    /// ciphertext fails GCM authentication.
    AadBound,
    /// A legacy №517 row (encrypted with an empty AAD) — readable only in
    /// the announced transition window, with a loud warning on every read.
    LegacyNoAad,
}

/// №527: the AAD = (subject_id, registry, schema version) — the ordered
/// triplet joined with the unit separator (0x1F, absent from ordinary
/// subject names), the registry being the store's own `voiceprints` table.
/// The composition is deterministic: the same subject always yields the
/// same AAD, so a ciphertext moved to another name (or another registry)
/// no longer authenticates — the swap attack the bare GCM tag accepted
/// (№517) is closed.
fn voiceprint_aad(name: &str) -> Vec<u8> {
    format!("voiceprints\u{1f}{name}\u{1f}{VOICEPRINT_AAD_SCHEMA}").into_bytes()
}

/// The №526 listing record — the persisted voiceprint's metadata WITHOUT
/// the biometric bytes (docs/privacy.md §2.1 composition; the listing
/// never decrypts). `algo` mirrors the schema mark (№517): Some("AES-256-GCM-v1")
/// / Some("INSECURE-XOR-MOCK") / None = a legacy pre-№517 row.
#[derive(Debug, Clone, PartialEq)]
pub struct VoiceprintRecord {
    pub name: String,
    pub model_id: String,
    pub saved_at: String,
    pub algo: Option<String>,
    pub blob_len: i64,
}

/// Voice store — SQLite-backed persistence for voiceprints and audio artifacts.
/// Voiceprints stored as BLOBs: AES-256-GCM (nonce ‖ ciphertext+tag) in the
/// real runtime with a secret()-gate key; the INSECURE XOR placeholder in the
/// mock runtime only (marked in the `algo` column); audio as raw BLOBs.
pub struct VoiceStore {
    conn: Mutex<Connection>,
}

/// Schema (№517 adds the additive `algo` column — old databases are
/// extended, never rejected; the №504 migration posture):
/// ```sql
/// CREATE TABLE voice_artifacts (
///     name TEXT PRIMARY KEY,
///     audio_bytes BLOB NOT NULL,
///     manifest_json TEXT,  -- NULL if no manifest
///     saved_at TEXT NOT NULL  -- RFC 3339
/// );
/// CREATE TABLE voiceprints (
///     name TEXT PRIMARY KEY,
///     embedding_encrypted BLOB NOT NULL,  -- AES-256-GCM nonce‖ct with the
///                                         -- №517 key (real runtime) or the
///                                         -- insecure XOR placeholder (mock
///                                         -- runtime only); the column name
///                                         -- predates the honesty fix and is
///                                         -- kept for schema stability
///     model_id TEXT NOT NULL,
///     saved_at TEXT NOT NULL,
///     algo TEXT                           -- №517: 'AES-256-GCM-v1' or
///                                         -- 'INSECURE-XOR-MOCK'; NULL on a
///                                         -- legacy pre-№517 row = the load
///                                         -- refuses loudly ([VOICEPRINT_STALE])
/// );
/// CREATE TABLE consent_ledger (
///     id INTEGER PRIMARY KEY AUTOINCREMENT,
///     voiceprint_hash TEXT NOT NULL,
///     nonce TEXT NOT NULL,
///     model TEXT NOT NULL,
///     timestamp TEXT NOT NULL  -- RFC 3339
/// );
/// ```
impl VoiceStore {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    /// №517: test-support accessor — the raw SQLite connection for the
    /// honesty/migration fixtures (the schema-level assertions read the raw
    /// row). #[doc(hidden)]: not part of the public API surface.
    #[doc(hidden)]
    pub fn raw_connection_for_tests(&self) -> std::sync::MutexGuard<'_, Connection> {
        // The poison-recovery pattern (the house lock posture): a panic in
        // a prior holder leaves the guard poisoned but the data usable for
        // the test fixtures — no expect/unwrap in the lib build (the
        // lib.rs deny).
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn init_tables(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("lock: {}", e))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS voice_artifacts (
                name TEXT PRIMARY KEY,
                audio_bytes BLOB NOT NULL,
                manifest_json TEXT,
                saved_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS voiceprints (
                name TEXT PRIMARY KEY,
                embedding_encrypted BLOB NOT NULL,
                model_id TEXT NOT NULL,
                saved_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS consent_ledger (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                voiceprint_hash TEXT NOT NULL,
                nonce TEXT NOT NULL,
                model TEXT NOT NULL,
                timestamp TEXT NOT NULL
            );",
        )
        .map_err(|e| format!("voice store init: {}", e))?;
        // №517: the additive migration — extend pre-№517 databases with the
        // algo column. Existing rows keep NULL (the loud-legacy contract).
        // A fresh database gets the column inside the CREATE above only in
        // future schemas; here the ALTER is idempotent-by-tolerance.
        if let Err(e) = conn.execute_batch("ALTER TABLE voiceprints ADD COLUMN algo TEXT;") {
            let msg = format!("{}", e);
            if !msg.contains("duplicate column name") {
                return Err(format!("voice store migration (algo): {}", msg));
            }
        }
        Ok(())
    }

    /// Save a voiceprint.
    ///
    /// №517 (audit 28.09 C-06 step 2): in the REAL runtime the embedding is
    /// AES-256-GCM encrypted BEFORE it touches the disk — the key arrives
    /// through the secret() gate semantics (env-sourced hex-256, NEVER
    /// derived from the name) and every write carries a fresh random 96-bit
    /// nonce. With NO key the store still refuses loudly
    /// ([VOICE_INSECURE_STORE]): biometric data is never persisted
    /// unencrypted, fail-closed.
    ///
    /// In the MOCK runtime (the SSOT predicate `llm::mock_llm_requested()`)
    /// the insecure placeholder path stays available for the skeleton tests
    /// — the row carries the visible insecure mark (algo =
    /// 'INSECURE-XOR-MOCK'), so a mock row can never masquerade as
    /// encrypted.
    pub fn save_voiceprint(
        &self,
        name: &str,
        embedding: &[f32],
        model_id: &str,
        key_hex: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("lock: {}", e))?;
        // Serialize embedding to bytes
        let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();

        let (stored, algo): (Vec<u8>, &'static str) = if crate::llm::mock_llm_requested() {
            // INSECURE placeholder — the mock-only skeleton path (№512);
            // the mark is visible in the schema (№517), never faked as crypto.
            (
                self.insecure_placeholder(&bytes, name),
                VOICEPRINT_ALGO_INSECURE_MOCK,
            )
        } else {
            let key_hex = key_hex.ok_or_else(|| {
                crate::interpreter::values::coded_error(
                    crate::interpreter::values::CODE_VOICE_INSECURE_STORE,
                    format!(
                        "voiceprint '{}' not saved: no key provided — biometric data (GDPR Art. 9) is persisted ONLY under AES-256-GCM with a secret()-gate key (METALOGOS_VOICEPRINT_KEY, 64 hex chars); unencrypted persistence stays refused (fail-closed, №512→№517)",
                        name
                    ),
                )
            })?;
            (
                encrypt_voiceprint(&bytes, key_hex, name)?,
                VOICEPRINT_ALGO_AES_GCM,
            )
        };

        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR REPLACE INTO voiceprints (name, embedding_encrypted, model_id, saved_at, algo) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![name, stored, model_id, now, algo],
        ).map_err(|e| format!("voice store save: {}", e))?;
        Ok(())
    }

    /// Load a voiceprint by name. Returns the decrypted embedding.
    ///
    /// №517 migration contract: a legacy pre-№517 row (algo NULL — the
    /// insecure XOR placeholder era) is NEVER silently read as decrypted —
    /// the load refuses loudly with [VOICEPRINT_STALE] ("re-enroll", the
    /// №504 posture). An AES-256-GCM row needs the key; a wrong key or a
    /// corrupted blob refuses loudly ([VOICEPRINT_DECRYPT] — GCM auth).
    ///
    /// №527: the load is the status-dropping wrapper over
    /// [`Self::load_voiceprint_with_status`] — the transitional legacy
    /// warning is announced inside regardless of the entry point.
    pub fn load_voiceprint(
        &self,
        name: &str,
        key_hex: Option<&str>,
    ) -> Result<(Vec<f32>, String), String> {
        self.load_voiceprint_with_status(name, key_hex)
            .map(|(embedding, model, _status)| (embedding, model))
    }

    /// №527: the status-returning load — the third element is the honest
    /// crypto status of the row ([`VoiceprintCryptoStatus`]): `AadBound`
    /// for №527-era rows (the subject-bound ciphertext), `LegacyNoAad`
    /// for №517-era rows read in the transition window (every such read
    /// also announces [VOICEPRINT_NO_AAD_LEGACY] on stderr — the flag and
    /// the warning are the observable pair, the caller decides on the
    /// re-enroll). The write path is always AAD-bound, so `LegacyNoAad`
    /// can only shrink to zero — the deadline lives in limitations.md.
    pub fn load_voiceprint_with_status(
        &self,
        name: &str,
        key_hex: Option<&str>,
    ) -> Result<(Vec<f32>, String, VoiceprintCryptoStatus), String> {
        let conn = self.conn.lock().map_err(|e| format!("lock: {}", e))?;
        let row = conn
            .query_row(
                "SELECT embedding_encrypted, model_id, algo FROM voiceprints WHERE name = ?1",
                rusqlite::params![name],
                |row| {
                    let stored: Vec<u8> = row.get(0)?;
                    let model_id: String = row.get(1)?;
                    let algo: Option<String> = row.get(2)?;
                    Ok((stored, model_id, algo))
                },
            )
            .map_err(|e| format!("voice store load '{}': {}", name, e))?;

        let (stored, model_id, algo) = row;
        let (bytes, crypto_status) = match algo.as_deref() {
            None => {
                return Err(crate::interpreter::values::coded_error(
                    crate::interpreter::values::CODE_VOICEPRINT_STALE,
                    format!(
                        "voiceprint '{}': a legacy pre-№517 row (insecure XOR placeholder) — it is NOT read as decrypted; re-enroll the speaker to migrate (№517)",
                        name
                    ),
                ))
            }
            Some(VOICEPRINT_ALGO_INSECURE_MOCK) => {
                // The mock skeleton path — XOR restore (NOT decryption).
                // The mock row is out of the AAD migration entirely (it
                // never held real biometric persistence — №512), so the
                // crypto status is meaningless there; the load keeps the
                // pre-№527 shape and never reports LegacyNoAad for it.
                (self.insecure_restore(&stored, name), VoiceprintCryptoStatus::AadBound)
            }
            Some(VOICEPRINT_ALGO_AES_GCM) => {
                let key_hex = key_hex.ok_or_else(|| {
                    crate::interpreter::values::coded_error(
                        crate::interpreter::values::CODE_VOICEPRINT_DECRYPT,
                        format!(
                            "voiceprint '{}': the row is AES-256-GCM encrypted (№517) — the key is required to load it",
                            name
                        ),
                    )
                })?;
                decrypt_voiceprint_with_status(&stored, key_hex, name)?
            }
            Some(other) => {
                return Err(crate::interpreter::values::coded_error(
                    crate::interpreter::values::CODE_VOICEPRINT_DECRYPT,
                    format!(
                        "voiceprint '{}': unknown storage algo '{}' — refusing (№517)",
                        name, other
                    ),
                ))
            }
        };
        // Deserialize embedding from bytes
        let embedding: Vec<f32> = bytes
            .chunks(4)
            .map(|chunk| {
                let arr: [u8; 4] = chunk.try_into().unwrap_or([0; 4]);
                f32::from_le_bytes(arr)
            })
            .collect();
        Ok((embedding, model_id, crypto_status))
    }

    /// Record a consent ledger entry.
    pub fn record_consent(
        &self,
        voiceprint_hash: &str,
        nonce: &str,
        model: &str,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("lock: {}", e))?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO consent_ledger (voiceprint_hash, nonce, model, timestamp) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![voiceprint_hash, nonce, model, now],
        ).map_err(|e| format!("consent ledger: {}", e))?;
        Ok(())
    }

    /// Check if a consent record exists for a given voiceprint hash.
    pub fn has_consent_record(&self, voiceprint_hash: &str) -> bool {
        let Ok(conn) = self.conn.lock() else {
            return false;
        };
        conn.query_row(
            "SELECT COUNT(*) FROM consent_ledger WHERE voiceprint_hash = ?1",
            rusqlite::params![voiceprint_hash],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
            > 0
    }

    /// Count consent records (for tests).
    pub fn consent_count(&self) -> usize {
        let Ok(conn) = self.conn.lock() else { return 0 };
        conn.query_row("SELECT COUNT(*) FROM consent_ledger", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap_or(0) as usize
    }

    /// №526 (issue #835; audit 30.09 N-2): list the persisted voiceprints —
    /// the informed-deletion basis (the data composition mirrors
    /// docs/privacy.md §2.1: name, model, timestamp, the storage algo mark,
    /// the ciphertext byte length). The listing NEVER decrypts: no key is
    /// required, no plaintext embedding leaves the store, the biometric
    /// bytes never enter the result.
    pub fn list_voiceprints(&self) -> Result<Vec<VoiceprintRecord>, String> {
        let conn = self.conn.lock().map_err(|e| format!("lock: {}", e))?;
        let mut stmt = conn
            .prepare(
                "SELECT name, model_id, saved_at, algo, LENGTH(embedding_encrypted) \
                 FROM voiceprints ORDER BY name",
            )
            .map_err(|e| format!("voice store list: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(VoiceprintRecord {
                    name: row.get(0)?,
                    model_id: row.get(1)?,
                    saved_at: row.get(2)?,
                    algo: row.get(3)?,
                    blob_len: row.get(4)?,
                })
            })
            .map_err(|e| format!("voice store list: {}", e))?;
        let mut records = Vec::new();
        for r in rows {
            records.push(r.map_err(|e| format!("voice store list: {}", e))?);
        }
        Ok(records)
    }

    /// №526: delete a persisted voiceprint by name — the GDPR Art. 17
    /// erasure path (docs/privacy.md §2.1). THE SECURE-DELETE PATH: the
    /// ciphertext blob is overwritten with zeros IN PLACE before the row
    /// is removed (the store-level analogue of overwrite-before-unlink —
    /// the biometric bytes do not ride on into the file's free pages via
    /// this row's last live copy), then the same-name voice_artifacts row
    /// (the audio artifact bytes — "файл артефакта + запись реестра") gets
    /// the same zero-then-delete treatment. IDEMPOTENT: a missing name is
    /// Ok(false) (already erased / never enrolled), never an error — a
    /// repeated erasure request succeeds. HONEST BOUNDARY (loud, not
    /// silent): SQLite cannot guarantee per-row block-level erasure (the
    /// freelist/WAL may hold older page images until SQLite reuses them);
    /// the row-level overwrite erases THIS row's live copy. File-level
    /// guarantees belong to the DB file's owner deleting the whole file
    /// (docs/privacy.md §2.1 retention). The consent_ledger rows SURVIVE
    /// by design — they are the Art. 9 consent PROOF (pseudonymous hash),
    /// the retention basis privacy.md documents.
    /// Returns Ok(true) when a voiceprint row was erased, Ok(false) when
    /// the name was absent.
    pub fn delete_voiceprint(&self, name: &str) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| format!("lock: {}", e))?;
        // (1) the zero-overwrite of the live ciphertext copy — the secure
        // path proper (перезапись перед удалением, №526).
        conn.execute(
            "UPDATE voiceprints SET embedding_encrypted = zeroblob(LENGTH(embedding_encrypted)) \
             WHERE name = ?1",
            rusqlite::params![name],
        )
        .map_err(|e| format!("voice store secure overwrite '{}': {}", name, e))?;
        // (2) the registry record removal.
        let deleted = conn
            .execute(
                "DELETE FROM voiceprints WHERE name = ?1",
                rusqlite::params![name],
            )
            .map_err(|e| format!("voice store delete '{}': {}", name, e))?;
        // (3) the same-name audio artifact row — the same zero-then-delete
        // treatment (best-effort purge; a name may carry no artifact).
        conn.execute(
            "UPDATE voice_artifacts SET audio_bytes = zeroblob(LENGTH(audio_bytes)) \
             WHERE name = ?1",
            rusqlite::params![name],
        )
        .map_err(|e| format!("voice store artifact overwrite '{}': {}", name, e))?;
        conn.execute(
            "DELETE FROM voice_artifacts WHERE name = ?1",
            rusqlite::params![name],
        )
        .map_err(|e| format!("voice store artifact delete '{}': {}", name, e))?;
        Ok(deleted > 0)
    }

    // INSECURE placeholder — XOR with the name-derived key; NOT encryption
    // (reversible by anyone who sees the table). MOCK RUNTIME ONLY since
    // №517: real-runtime rows are always AES-256-GCM (see the schema algo
    // mark). The name is kept honest (№512).
    fn insecure_placeholder(&self, data: &[u8], key: &str) -> Vec<u8> {
        let key_bytes = key.as_bytes();
        data.iter()
            .enumerate()
            .map(|(i, &b)| b ^ key_bytes[i % key_bytes.len()])
            .collect()
    }

    fn insecure_restore(&self, data: &[u8], key: &str) -> Vec<u8> {
        // XOR is symmetric — the same transform inverts the placeholder
        // (NOT a decrypted value; there is no crypto here)
        self.insecure_placeholder(data, key)
    }
}

/// AES-256-GCM encrypt the raw embedding bytes (№517) — the SAME primitive
/// as the encrypt() builtin (Phase 7.3, the `aes-gcm` crate): the key is
/// 32 bytes (64 hex chars — the secret()-gate value, NEVER derived from
/// the name), the nonce is a fresh random 96 bits per write, the stored
/// blob is self-contained `nonce ‖ ciphertext+tag`.
/// №527: the ciphertext is bound to its subject — the GCM AAD carries the
/// (subject_id, registry, schema version) triplet; the decoded key buffer
/// lives under Zeroizing and is wiped at the scope exit.
#[doc(hidden)]
pub fn encrypt_voiceprint_for_tests(data: &[u8], key_hex: &str) -> Result<Vec<u8>, String> {
    encrypt_voiceprint(data, key_hex, "test")
}

#[doc(hidden)]
pub fn decrypt_voiceprint_for_tests(blob: &[u8], key_hex: &str) -> Result<Vec<u8>, String> {
    Ok(decrypt_voiceprint_with_status(blob, key_hex, "test")?.0)
}

/// №527: the №517-era blob factory for the migration fixtures — the SAME
/// AES-256-GCM primitive with an EMPTY AAD (exactly what the pre-№527
/// contour produced). NOT a production path: the migration fixtures pin
/// the transitional read behaviour against the true legacy shape.
#[doc(hidden)]
pub fn encrypt_voiceprint_legacy_no_aad_for_tests(
    data: &[u8],
    key_hex: &str,
) -> Result<Vec<u8>, String> {
    use aes_gcm::aead::{Aead, KeyInit, Payload};
    use aes_gcm::{Aes256Gcm, Key, Nonce};
    use zeroize::Zeroizing;

    let key_bytes = Zeroizing::new(
        hex::decode(key_hex)
            .map_err(|e| format!("voiceprint 'legacy': the key must be hex: {}", e))?,
    );
    if key_bytes.len() != 32 {
        return Err("voiceprint 'legacy': the key must be 256-bit (64 hex chars)".into());
    }
    let key = Key::<Aes256Gcm>::try_from(key_bytes.as_slice())
        .map_err(|_| "voiceprint 'legacy': key conversion failed".to_string())?;
    let cipher = Aes256Gcm::new(&key);
    let mut nonce_bytes = [0u8; 12];
    use rand::Rng;
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::try_from(nonce_bytes.as_slice())
        .map_err(|_| "voiceprint 'legacy': nonce conversion failed".to_string())?;
    // The EMPTY AAD — the exact №517-era contour (the pre-№527 writes).
    let ciphertext = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: data,
                aad: b"",
            },
        )
        .map_err(|e| format!("voiceprint 'legacy': AES-256-GCM encryption failed: {}", e))?;
    let mut blob = nonce.to_vec();
    blob.extend_from_slice(&ciphertext);
    Ok(blob)
}

fn encrypt_voiceprint(data: &[u8], key_hex: &str, name: &str) -> Result<Vec<u8>, String> {
    use aes_gcm::aead::{Aead, KeyInit, Payload};
    use aes_gcm::{Aes256Gcm, Key, Nonce};
    use zeroize::Zeroizing;

    // №527: the decoded key material lives under Zeroizing — the buffer is
    // wiped when the scope exits, the key's plaintext lifetime is the
    // single operation (the hex string itself comes from the secret() gate,
    // whose lifetime is the caller's — unchanged by №527).
    let key_bytes = Zeroizing::new(
        hex::decode(key_hex)
            .map_err(|e| format!("voiceprint '{}': the key must be hex: {}", name, e))?,
    );
    if key_bytes.len() != 32 {
        return Err(crate::interpreter::values::coded_error(
            crate::interpreter::values::CODE_VOICE_INSECURE_STORE,
            format!(
                "voiceprint '{}': the key must be 256-bit (64 hex chars), got {} bytes — fail-closed (№517)",
                name,
                key_bytes.len()
            ),
        ));
    }
    let key = Key::<Aes256Gcm>::try_from(key_bytes.as_slice())
        .map_err(|_| format!("voiceprint '{}': key conversion failed", name))?;
    let cipher = Aes256Gcm::new(&key);
    // Fresh random 96-bit nonce per write — uniqueness by construction.
    let mut nonce_bytes = [0u8; 12];
    use rand::Rng;
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::try_from(nonce_bytes.as_slice())
        .map_err(|_| format!("voiceprint '{}': nonce conversion failed", name))?;
    // №527: the AAD binds the ciphertext to (subject_id, registry, schema
    // version) — a blob transplanted onto another subject's row fails the
    // authentication (the swap attack the bare tag accepted is closed).
    let ciphertext = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: data,
                aad: &voiceprint_aad(name),
            },
        )
        .map_err(|e| {
            format!(
                "voiceprint '{}': AES-256-GCM encryption failed: {}",
                name, e
            )
        })?;
    let mut blob = nonce.to_vec();
    blob.extend_from_slice(&ciphertext);
    Ok(blob)
}

/// AES-256-GCM decrypt a stored voiceprint blob (№517 → №527). A wrong key
/// or a corrupted blob refuses LOUDLY ([VOICEPRINT_DECRYPT] — the GCM auth
/// tag does not lie). №527: the status-returning core of the transitional
/// decrypt — the bound attempt (the №527 AAD) first; if it fails, the
/// transitional empty-AAD attempt (the №517-era rows) succeeds ONLY for a
/// genuinely legacy blob, announcing [VOICEPRINT_NO_AAD_LEGACY] on stderr
/// and reporting [`VoiceprintCryptoStatus::LegacyNoAad`]. A wrong key fails
/// BOTH attempts — the single coded refusal, nothing leaks.
fn decrypt_voiceprint_with_status(
    blob: &[u8],
    key_hex: &str,
    name: &str,
) -> Result<(Vec<u8>, VoiceprintCryptoStatus), String> {
    use aes_gcm::aead::{Aead, KeyInit, Payload};
    use aes_gcm::{Aes256Gcm, Key, Nonce};
    use zeroize::Zeroizing;

    if blob.len() < 13 {
        return Err(crate::interpreter::values::coded_error(
            crate::interpreter::values::CODE_VOICEPRINT_DECRYPT,
            format!(
                "voiceprint '{}': the stored blob is too short to carry nonce‖ciphertext — corrupted (№517)",
                name
            ),
        ));
    }
    // №527: the decoded key material lives under Zeroizing (the wipe at the
    // scope exit — the same minimal-lifetime posture as the encrypt side).
    let key_bytes = Zeroizing::new(
        hex::decode(key_hex)
            .map_err(|e| format!("voiceprint '{}': the key must be hex: {}", name, e))?,
    );
    if key_bytes.len() != 32 {
        return Err(crate::interpreter::values::coded_error(
            crate::interpreter::values::CODE_VOICEPRINT_DECRYPT,
            format!(
                "voiceprint '{}': the key must be 256-bit (64 hex chars), got {} bytes (№517)",
                name,
                key_bytes.len()
            ),
        ));
    }
    let key = Key::<Aes256Gcm>::try_from(key_bytes.as_slice())
        .map_err(|_| format!("voiceprint '{}': key conversion failed", name))?;
    let cipher = Aes256Gcm::new(&key);
    let (nonce_bytes, ciphertext) = blob.split_at(12);
    let nonce = Nonce::try_from(nonce_bytes)
        .map_err(|_| format!("voiceprint '{}': nonce conversion failed", name))?;
    // Attempt 1: the №527 subject-bound composition.
    if let Ok(plaintext) = cipher.decrypt(
        &nonce,
        Payload {
            msg: ciphertext,
            aad: &voiceprint_aad(name),
        },
    ) {
        return Ok((plaintext, VoiceprintCryptoStatus::AadBound));
    }
    // Attempt 2: the transitional legacy read (the №517-era empty AAD).
    // Success here means the row IS a legacy row by construction (a bound
    // blob never authenticates without its AAD); the swap of a legacy blob
    // onto another name still authenticates — that residue is exactly what
    // the transition window is for, and it announces itself loudly on
    // EVERY read until the deadline removes the fallback.
    match cipher.decrypt(&nonce, ciphertext) {
        Ok(plaintext) => {
            eprintln!(
                "[VOICEPRINT_NO_AAD_LEGACY] voiceprint '{}': the row is a legacy №517 blob (no AAD subject binding) — readable in the transition window only; re-enroll or re-save to bind it (№527; the deadline: docs/limitations.md)",
                name
            );
            Ok((plaintext, VoiceprintCryptoStatus::LegacyNoAad))
        }
        Err(_) => Err(crate::interpreter::values::coded_error(
            crate::interpreter::values::CODE_VOICEPRINT_DECRYPT,
            format!(
                "voiceprint '{}': AES-256-GCM authentication FAILED — wrong key or corrupted data; nothing is returned (№517)",
                name
            ),
        )),
    }
}

/// Compute a hash of a voiceprint embedding for the ledger.
/// Uses a simple deterministic hash (not cryptographic — for ledger
/// uniqueness, not for security). Real implementation should use SHA-256.
pub fn voiceprint_hash(embedding: &[f32]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    for f in embedding {
        f.to_bits().hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> VoiceStore {
        let conn = Connection::open_in_memory().unwrap();
        let store = VoiceStore::new(conn);
        store.init_tables().unwrap();
        store
    }

    /// The process-global mock flag (№251 family discipline): serialize the
    /// tests that pin it, restore the unset state after each use.
    static MOCK_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn pin_mock(on: bool) {
        if on {
            std::env::set_var("METALOGOS_MOCK_LLM", "1");
        } else {
            std::env::remove_var("METALOGOS_MOCK_LLM");
        }
    }

    #[test]
    fn voiceprint_roundtrip() {
        let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        pin_mock(true);
        let store = test_store();
        let embedding = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        store
            .save_voiceprint("alice", &embedding, "chatterbox-v3", None)
            .unwrap();
        let (loaded, model) = store.load_voiceprint("alice", None).unwrap();
        assert_eq!(embedding, loaded);
        assert_eq!(model, "chatterbox-v3");
        pin_mock(false);
    }

    #[test]
    fn voiceprint_not_found() {
        let store = test_store();
        let result = store.load_voiceprint("nonexistent", None);
        assert!(result.is_err());
    }

    #[test]
    fn consent_ledger() {
        let store = test_store();
        let hash = voiceprint_hash(&[0.1, 0.2, 0.3]);
        assert_eq!(store.consent_count(), 0);
        assert!(!store.has_consent_record(&hash));
        store
            .record_consent(&hash, "nonce-123", "chatterbox-v3")
            .unwrap();
        assert_eq!(store.consent_count(), 1);
        assert!(store.has_consent_record(&hash));
    }

    #[test]
    fn voiceprint_bytes_are_not_stored_raw() {
        let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        pin_mock(true);
        let store = test_store();
        let embedding = vec![1.0, 2.0, 3.0];
        store
            .save_voiceprint("bob", &embedding, "koko-ro-82m", None)
            .unwrap();
        // Verify the stored bytes are NOT the raw embedding — the insecure
        // XOR placeholder at least does not leave plaintext f32 values on
        // disk (that is ALL it does; it is NOT encryption — №512; mock-only
        // since №517).
        let conn = store.conn.lock().unwrap();
        let stored: Vec<u8> = conn
            .query_row(
                "SELECT embedding_encrypted FROM voiceprints WHERE name = 'bob'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let raw_bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
        assert_ne!(
            stored, raw_bytes,
            "voiceprint must not land as raw bytes (insecure XOR placeholder, not encryption — №512)"
        );
        pin_mock(false);
    }

    #[test]
    fn voiceprint_hash_deterministic() {
        let h1 = voiceprint_hash(&[0.1, 0.2, 0.3]);
        let h2 = voiceprint_hash(&[0.1, 0.2, 0.3]);
        assert_eq!(h1, h2);
        let h3 = voiceprint_hash(&[0.1, 0.2, 0.4]);
        assert_ne!(h1, h3);
    }

    // ── №526 (issue #835; N-2): the delete path — list / secure delete ──

    #[test]
    fn n526_list_then_delete_cycle() {
        let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        pin_mock(true);
        let store = test_store();
        store
            .save_voiceprint("alice", &[0.1, 0.2, 0.3], "chatterbox-v3", None)
            .unwrap();
        store
            .save_voiceprint("bob", &[0.4, 0.5], "koko-ro-82m", None)
            .unwrap();
        // list: both present, the composition without the biometric bytes
        let records = store.list_voiceprints().unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].name, "alice");
        assert_eq!(records[0].model_id, "chatterbox-v3");
        assert_eq!(
            records[0].algo.as_deref(),
            Some(VOICEPRINT_ALGO_INSECURE_MOCK)
        );
        assert!(records[0].blob_len > 0);
        // delete → list empty for that name
        assert!(store.delete_voiceprint("alice").unwrap());
        let records = store.list_voiceprints().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].name, "bob");
        // idempotent: a repeated erase is a success, not an error
        assert!(!store.delete_voiceprint("alice").unwrap());
        assert!(!store.delete_voiceprint("never-enrolled").unwrap());
        pin_mock(false);
    }

    #[test]
    fn n526_secure_delete_overwrites_blob_with_zeros_before_removal() {
        let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        pin_mock(true);
        let store = test_store();
        store
            .save_voiceprint("carol", &[1.0; 64], "koko-ro-82m", None)
            .unwrap();
        // sanity: the live blob is NOT zeros before the delete
        {
            let conn = store.raw_connection_for_tests();
            let blob: Vec<u8> = conn
                .query_row(
                    "SELECT embedding_encrypted FROM voiceprints WHERE name = 'carol'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(!blob.is_empty());
            assert!(blob.iter().any(|&b| b != 0), "the live copy holds bytes");
        }
        // capture the blob length, delete, and verify the row is gone
        let len = {
            let conn = store.raw_connection_for_tests();
            conn.query_row(
                "SELECT LENGTH(embedding_encrypted) FROM voiceprints WHERE name = 'carol'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap()
        };
        assert!(store.delete_voiceprint("carol").unwrap());
        {
            let conn = store.raw_connection_for_tests();
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM voiceprints WHERE name = 'carol'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 0, "the registry record is removed");
            let _ = len; // the length pin: the overwrite ran at the same size
        }
        pin_mock(false);
    }

    #[test]
    // №526: the reopen fixture constructs its own throwaway SQLite file —
    // the raw Connection::open/std::fs calls here are the TEST SANDBOX
    // itself (the same posture as the №475 post-gate allowance), not
    // program IO: there is no gate to bypass in a fixture that BUILDS
    // the database the gate would later guard.
    #[allow(clippy::disallowed_methods)]
    fn n526_deleted_print_does_not_resurrect_after_reopen() {
        // the restart test: delete → a FRESH store over the same DB file →
        // the deleted print is gone (no resurrection), the survivor stays
        let dir = std::env::temp_dir().join(format!(
            "n526_reopen_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("voice.db");
        let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        pin_mock(true);
        {
            let conn = Connection::open(&db_path).unwrap();
            let store = VoiceStore::new(conn);
            store.init_tables().unwrap();
            store
                .save_voiceprint("dave", &[0.9, 0.8], "chatterbox-v3", None)
                .unwrap();
            store
                .save_voiceprint("erin", &[0.7, 0.6], "koko-ro-82m", None)
                .unwrap();
            assert!(store.delete_voiceprint("dave").unwrap());
        }
        // "server restart": a brand-new store over the same file
        {
            let conn = Connection::open(&db_path).unwrap();
            let store = VoiceStore::new(conn);
            store.init_tables().unwrap();
            let names: Vec<String> = store
                .list_voiceprints()
                .unwrap()
                .into_iter()
                .map(|r| r.name)
                .collect();
            assert_eq!(names, vec!["erin".to_string()], "no resurrection");
            assert!(!store.delete_voiceprint("dave").unwrap());
        }
        pin_mock(false);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn n526_artifact_row_is_purged_with_the_print() {
        let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        pin_mock(true);
        let store = test_store();
        store
            .save_voiceprint("frank", &[0.5], "koko-ro-82m", None)
            .unwrap();
        // a same-name audio artifact row — "файл артефакта + запись реестра"
        {
            let conn = store.raw_connection_for_tests();
            conn.execute(
                "INSERT INTO voice_artifacts (name, audio_bytes, manifest_json, saved_at) \
                 VALUES ('frank', ?1, NULL, '2026-09-30T00:00:00Z')",
                rusqlite::params![vec![0xABu8; 128]],
            )
            .unwrap();
        }
        assert!(store.delete_voiceprint("frank").unwrap());
        {
            let conn = store.raw_connection_for_tests();
            let artifacts: i64 = conn
                .query_row("SELECT COUNT(*) FROM voice_artifacts", [], |row| row.get(0))
                .unwrap();
            assert_eq!(artifacts, 0, "the artifact row is purged with the print");
            let prints: i64 = conn
                .query_row("SELECT COUNT(*) FROM voiceprints", [], |row| row.get(0))
                .unwrap();
            assert_eq!(prints, 0);
        }
        pin_mock(false);
    }
}
