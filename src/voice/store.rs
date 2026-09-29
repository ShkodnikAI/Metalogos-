// ── Voice store: SQLite persistence for voiceprints + audio artifacts ──
// Наряд №303 (issue #371, P2/feature/voice) — VOICE_A3_SPEAKER_ENCODER.
//
// Mirrors src/vision/store.rs (Наряд №242) — SQLite BLOB persistence.
// №512 (audit 28.09 C-06): HONESTY FIX. The earlier header claimed
// at-rest encryption — that was FALSE. What actually happens: the bytes
// go through an INSECURE XOR placeholder keyed by the PUBLIC name
// (reversible by anyone who reads the table) — NOT encryption. In the
// real runtime the store refuses to persist voiceprints at all
// ([VOICE_INSECURE_STORE]); the placeholder path survives in the mock
// runtime only, for the skeleton tests. Voiceprints are biometric data
// (GDPR Art. 9 special category). The honest crypto that lifts the
// refusal is naryad №517; the privacy policy lands in docs/privacy.md
// (№519).
// Ledger records consent (hash(voiceprint, nonce, date, model)).

use rusqlite::Connection;
use std::sync::Mutex;

/// Voice store — SQLite-backed persistence for voiceprints and audio artifacts.
/// Voiceprints stored as BLOBs through an INSECURE XOR placeholder (NOT
/// encryption — see the module header); audio as raw BLOBs.
pub struct VoiceStore {
    conn: Mutex<Connection>,
}

/// Schema:
/// ```sql
/// CREATE TABLE voice_artifacts (
///     name TEXT PRIMARY KEY,
///     audio_bytes BLOB NOT NULL,
///     manifest_json TEXT,  -- NULL if no manifest
///     saved_at TEXT NOT NULL  -- RFC 3339
/// );
/// CREATE TABLE voiceprints (
///     name TEXT PRIMARY KEY,
///     embedding_encrypted BLOB NOT NULL,  -- insecure XOR placeholder,
///                                         -- NOT encryption (№512); the
///                                         -- column name predates the
///                                         -- honesty fix and is kept for
///                                         -- schema stability
///     model_id TEXT NOT NULL,
///     saved_at TEXT NOT NULL
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
        Ok(())
    }

    /// Save a voiceprint.
    ///
    /// №512 (audit 28.09 C-06): in the REAL runtime (no `METALOGOS_MOCK_LLM`)
    /// this REFUSES LOUDLY with `[VOICE_INSECURE_STORE]`: the storage under
    /// this row is an INSECURE XOR placeholder — NOT encryption (the XOR key
    /// derives from the public name, so anyone who reads the table can
    /// reverse it) — and a voiceprint is biometric data (GDPR Art. 9).
    /// Persisting biometric data behind a fake crypto label is exactly the
    /// dishonesty this naryad removes; the honest crypto that lifts the
    /// refusal is naryad №517.
    ///
    /// In the MOCK runtime (the SSOT predicate `llm::mock_llm_requested()`)
    /// the placeholder path stays available for the skeleton tests — the
    /// bytes are still NOT protected (the test fixtures carry the insecure
    /// mark). The physical column name `embedding_encrypted` predates the
    /// honesty fix and is kept for schema stability (see the schema comment).
    pub fn save_voiceprint(
        &self,
        name: &str,
        embedding: &[f32],
        model_id: &str,
    ) -> Result<(), String> {
        if !crate::llm::mock_llm_requested() {
            return Err(crate::interpreter::values::coded_error(
                crate::interpreter::values::CODE_VOICE_INSECURE_STORE,
                format!(
                    "voiceprint '{}' not saved: the store is an INSECURE XOR placeholder, NOT encryption (the key derives from the public name and is reversible by anyone who reads the table); voiceprints are biometric data (GDPR Art. 9) — real-runtime persistence is refused until honest crypto lands (naryad №517)",
                    name
                ),
            ));
        }
        let conn = self.conn.lock().map_err(|e| format!("lock: {}", e))?;
        // Serialize embedding to bytes
        let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
        // INSECURE placeholder: XOR with a key derived from the PUBLIC name —
        // NOT encryption (№512 honesty fix; the fake crypto claim is gone).
        let stored = self.insecure_placeholder(&bytes, name);
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR REPLACE INTO voiceprints (name, embedding_encrypted, model_id, saved_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![name, stored, model_id, now],
        ).map_err(|e| format!("voice store save: {}", e))?;
        Ok(())
    }

    /// Load a voiceprint by name. Returns the embedding de-XORed from the
    /// insecure placeholder store (NOT a decrypted value — there is no
    /// crypto here; №512).
    pub fn load_voiceprint(&self, name: &str) -> Result<(Vec<f32>, String), String> {
        let conn = self.conn.lock().map_err(|e| format!("lock: {}", e))?;
        let row = conn
            .query_row(
                "SELECT embedding_encrypted, model_id FROM voiceprints WHERE name = ?1",
                rusqlite::params![name],
                |row| {
                    let encrypted: Vec<u8> = row.get(0)?;
                    let model_id: String = row.get(1)?;
                    Ok((encrypted, model_id))
                },
            )
            .map_err(|e| format!("voice store load '{}': {}", name, e))?;

        let (stored, model_id) = row;
        let bytes = self.insecure_restore(&stored, name);
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
    // (reversible by anyone who sees the table; №512 honesty fix).
    // The honest crypto that replaces it: naryad №517.
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
            .save_voiceprint("alice", &embedding, "chatterbox-v3")
            .unwrap();
        let (loaded, model) = store.load_voiceprint("alice").unwrap();
        assert_eq!(embedding, loaded);
        assert_eq!(model, "chatterbox-v3");
        pin_mock(false);
    }

    #[test]
    fn voiceprint_not_found() {
        let store = test_store();
        let result = store.load_voiceprint("nonexistent");
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
            .save_voiceprint("bob", &embedding, "koko-ro-82m")
            .unwrap();
        // Verify the stored bytes are NOT the raw embedding — the insecure
        // XOR placeholder at least does not leave plaintext f32 values on
        // disk (that is ALL it does; it is NOT encryption — №512).
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
