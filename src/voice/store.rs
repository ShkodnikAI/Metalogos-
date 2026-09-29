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

use rusqlite::Connection;
use std::sync::Mutex;

/// The schema version label stamped on every №517-encrypted row.
pub(crate) const VOICEPRINT_ALGO_AES_GCM: &str = "AES-256-GCM-v1";
/// The visible-in-schema insecure mark of the mock-runtime placeholder rows.
pub(crate) const VOICEPRINT_ALGO_INSECURE_MOCK: &str = "INSECURE-XOR-MOCK";

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
        self.conn.lock().expect("store lock")
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
    pub fn load_voiceprint(
        &self,
        name: &str,
        key_hex: Option<&str>,
    ) -> Result<(Vec<f32>, String), String> {
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
        let bytes = match algo.as_deref() {
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
                self.insecure_restore(&stored, name)
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
                decrypt_voiceprint(&stored, key_hex, name)?
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
        Ok((embedding, model_id))
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
#[doc(hidden)]
pub fn encrypt_voiceprint_for_tests(data: &[u8], key_hex: &str) -> Result<Vec<u8>, String> {
    encrypt_voiceprint(data, key_hex, "test")
}

#[doc(hidden)]
pub fn decrypt_voiceprint_for_tests(blob: &[u8], key_hex: &str) -> Result<Vec<u8>, String> {
    decrypt_voiceprint(blob, key_hex, "test")
}

fn encrypt_voiceprint(data: &[u8], key_hex: &str, name: &str) -> Result<Vec<u8>, String> {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Key, Nonce};

    let key_bytes = hex::decode(key_hex)
        .map_err(|e| format!("voiceprint '{}': the key must be hex: {}", name, e))?;
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
    let ciphertext = cipher.encrypt(&nonce, data).map_err(|e| {
        format!(
            "voiceprint '{}': AES-256-GCM encryption failed: {}",
            name, e
        )
    })?;
    let mut blob = nonce.to_vec();
    blob.extend_from_slice(&ciphertext);
    Ok(blob)
}

/// AES-256-GCM decrypt a stored voiceprint blob (№517). A wrong key or a
/// corrupted blob refuses LOUDLY ([VOICEPRINT_DECRYPT] — the GCM auth tag
/// does not lie).
fn decrypt_voiceprint(blob: &[u8], key_hex: &str, name: &str) -> Result<Vec<u8>, String> {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Key, Nonce};

    if blob.len() < 13 {
        return Err(crate::interpreter::values::coded_error(
            crate::interpreter::values::CODE_VOICEPRINT_DECRYPT,
            format!(
                "voiceprint '{}': the stored blob is too short to carry nonce‖ciphertext — corrupted (№517)",
                name
            ),
        ));
    }
    let key_bytes = hex::decode(key_hex)
        .map_err(|e| format!("voiceprint '{}': the key must be hex: {}", name, e))?;
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
    cipher.decrypt(&nonce, ciphertext).map_err(|_| {
        crate::interpreter::values::coded_error(
            crate::interpreter::values::CODE_VOICEPRINT_DECRYPT,
            format!(
                "voiceprint '{}': AES-256-GCM authentication FAILED — wrong key or corrupted data; nothing is returned (№517)",
                name
            ),
        )
    })
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
}
