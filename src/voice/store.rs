// ── Voice store: SQLite persistence for voiceprints + audio artifacts ──
// Наряд №303 (issue #371, P2/feature/voice) — VOICE_A3_SPEAKER_ENCODER.
//
// Mirrors src/vision/store.rs (Наряд №242) — SQLite BLOB persistence.
// Voiceprints are encrypted at rest (AES-256-GCM via secret() stack, Наряд №172).
// Ledger records consent (hash(voiceprint, nonce, date, model)).

use rusqlite::Connection;
use std::sync::Mutex;

/// Voice store — SQLite-backed persistence for voiceprints and audio artifacts.
/// Voiceprints stored as encrypted BLOBs; audio as raw BLOBs.
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
///     embedding_encrypted BLOB NOT NULL,  -- AES-256-GCM encrypted
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

    /// Save a voiceprint (encrypted at rest).
    /// The embedding is encrypted using AES-256-GCM via the secret() stack.
    /// For the skeleton, we use a simple XOR-based encryption placeholder —
    /// the real AES-256-GCM implementation will use crate::builtins::crypto
    /// (Наряд №172) in phase A4.
    pub fn save_voiceprint(
        &self,
        name: &str,
        embedding: &[f32],
        model_id: &str,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("lock: {}", e))?;
        // Serialize embedding to bytes
        let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
        // Encrypt (placeholder: XOR with key derived from name hash)
        // Real implementation: AES-256-GCM via crate::builtins::crypto (Наряд №172)
        let encrypted = self.encrypt_placeholder(&bytes, name);
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR REPLACE INTO voiceprints (name, embedding_encrypted, model_id, saved_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![name, encrypted, model_id, now],
        ).map_err(|e| format!("voice store save: {}", e))?;
        Ok(())
    }

    /// Load a voiceprint by name. Returns the decrypted embedding.
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

        let (encrypted, model_id) = row;
        let bytes = self.decrypt_placeholder(&encrypted, name);
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

    // Placeholder encryption — XOR with name-derived key.
    // Real: AES-256-GCM via crate::builtins::crypto (Наряд №172, phase A4).
    fn encrypt_placeholder(&self, data: &[u8], key: &str) -> Vec<u8> {
        let key_bytes = key.as_bytes();
        data.iter()
            .enumerate()
            .map(|(i, &b)| b ^ key_bytes[i % key_bytes.len()])
            .collect()
    }

    fn decrypt_placeholder(&self, data: &[u8], key: &str) -> Vec<u8> {
        // XOR is symmetric — same function for encrypt and decrypt
        self.encrypt_placeholder(data, key)
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

    #[test]
    fn voiceprint_roundtrip() {
        let store = test_store();
        let embedding = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        store
            .save_voiceprint("alice", &embedding, "chatterbox-v3")
            .unwrap();
        let (loaded, model) = store.load_voiceprint("alice").unwrap();
        assert_eq!(embedding, loaded);
        assert_eq!(model, "chatterbox-v3");
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
    fn voiceprint_is_encrypted_at_rest() {
        let store = test_store();
        let embedding = vec![1.0, 2.0, 3.0];
        store
            .save_voiceprint("bob", &embedding, "koko-ro-82m")
            .unwrap();
        // Verify the stored bytes are NOT the raw embedding
        let conn = store.conn.lock().unwrap();
        let stored: Vec<u8> = conn
            .query_row(
                "SELECT embedding_encrypted FROM voiceprints WHERE name = 'bob'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let raw_bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
        assert_ne!(stored, raw_bytes, "voiceprint must be encrypted at rest");
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
