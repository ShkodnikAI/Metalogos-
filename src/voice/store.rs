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
// mark are UNTOUCHED (the №527 boundary). THE DEADLINE IS HONORED: the
// transitional empty-AAD read (the №517-era legacy rows) existed ONLY
// between №527 and v0.28.0 — the fallback is REMOVED in the 0.28.0
// release prep (№550), so a legacy row now refuses with
// [VOICEPRINT_DECRYPT] and the only path back is re-enroll/re-save.
// No released version ever wrote a legacy row (the GCM store itself
// ships in 0.28.0) — the transitional population was main-only.
// №556 (issue #917; audit 02.10 M-6; the GDPR line №526→№527→№556): the
// strict-erasure posture closes the two residuals the audit found.
// (1) The DELETED page no longer survives inside the database file's
// journal: the store's setup turns PRAGMA secure_delete ON (deleted
// content is overwritten before the pages are freed) and every
// delete_voiceprint ends with PRAGMA wal_checkpoint(TRUNCATE) — the WAL
// cannot carry the erased page after the call (a non-WAL connection
// takes the harmless no-op checkpoint; a WAL-mode deployment gets the
// truncate, and a busy WAL is a LOUD error, never silence).
// (2) The audio artifacts stop lying about at-rest secrecy (the №512
// lesson): the legacy plaintext `audio_bytes` column becomes
// `audio_encrypted` — the SAME AES-256-GCM scheme as the voiceprints
// (№517) with the №527 subject binding (the AAD registry is
// `voice_artifacts`: a blob transplanted onto another name fails
// authentication), the mock-runtime placeholder keeps its visible
// INSECURE-XOR-MOCK mark, and a keyless real-runtime write fails closed
// ([VOICE_INSECURE_STORE]). The migration: a pre-№556 database's
// plaintext payload cannot honestly become ciphertext (the key is
// unavailable at init time; the №517 posture refuses keyless
// persistence) and no released version ever wrote an audio row (the
// write path never shipped) — the legacy bytes are zero-overwritten and
// the table is rebuilt in the new shape. The store-level
// save/load_audio_artifact pair is the AT-REST CONTOUR ONLY — wiring a
// program-facing audio write surface is NOT in №556 (the boundary).

use rusqlite::Connection;
use std::sync::Mutex;

/// The schema version label stamped on every №517-encrypted row.
pub(crate) const VOICEPRINT_ALGO_AES_GCM: &str = "AES-256-GCM-v1";
/// The visible-in-schema insecure mark of the mock-runtime placeholder rows.
pub(crate) const VOICEPRINT_ALGO_INSECURE_MOCK: &str = "INSECURE-XOR-MOCK";

/// №527: the schema-version component of the AAD triplet. NOT the schema
/// `algo` mark (the №527 boundaries keep the storage schema untouched):
/// rows carrying the same algo mark can be AAD-bound (№527-era writes) or
/// legacy no-AAD (№517-era writes) — the discriminator WAS the GCM
/// authentication itself (try-bound first, then the transitional
/// fallback). THE DEADLINE (limitations.md, the №524 rule): v0.28.0
/// removed the fallback — a blob that does not authenticate under its
/// subject AAD refuses with [VOICEPRINT_DECRYPT]; there is no second
/// attempt anymore.
pub(crate) const VOICEPRINT_AAD_SCHEMA: &str = "voiceprints-aad-v1";

/// №556: the scheme marks of the audio-artifact rows — the SAME scheme as
/// the voiceprints (the AES-256-GCM-v1 contour, the INSECURE-XOR-MOCK
/// placeholder); only the AAD REGISTRY differs (`voice_artifacts`). The
/// aliases keep the schema marks grep-able per table.
pub(crate) const ARTIFACT_ALGO_AES_GCM: &str = VOICEPRINT_ALGO_AES_GCM;
pub(crate) const ARTIFACT_ALGO_INSECURE_MOCK: &str = VOICEPRINT_ALGO_INSECURE_MOCK;

/// №556: the schema-version component of the audio-artifact AAD triplet —
/// the mirror of VOICEPRINT_AAD_SCHEMA for the `voice_artifacts` registry.
pub(crate) const VOICE_ARTIFACT_AAD_SCHEMA: &str = "voice-artifacts-aad-v1";

/// №527: the crypto status of a loaded voiceprint — observable by the
/// caller. Since the v0.28.0 deadline (the limitations.md TRANSITION row
/// honored in №550) the only value is `AadBound`: the empty-AAD
/// transitional read is gone, so a row either authenticates under its
/// subject AAD or refuses with [VOICEPRINT_DECRYPT].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceprintCryptoStatus {
    /// The row is AAD-bound to its subject (№527): a swapped or misplaced
    /// ciphertext fails GCM authentication.
    AadBound,
}

/// №527 (generalized by №556): the AAD = (registry, subject, schema
/// version) — the ordered triplet joined with the unit separator (0x1F,
/// absent from ordinary subject names). The composition is deterministic:
/// the same subject always yields the same AAD, so a ciphertext moved to
/// another name (or another registry) no longer authenticates — the swap
/// attack the bare GCM tag accepted (№517) is closed.
fn voice_aad(registry: &str, subject: &str, schema: &str) -> Vec<u8> {
    format!("{registry}\u{1f}{subject}\u{1f}{schema}").into_bytes()
}

fn voiceprint_aad(name: &str) -> Vec<u8> {
    voice_aad("voiceprints", name, VOICEPRINT_AAD_SCHEMA)
}

/// №556: the audio-artifact subject binding — the same triplet shape, the
/// `voice_artifacts` registry. A blob saved under name A refuses to load
/// under name B (the swap attack the bare GCM tag accepted is closed for
/// the artifacts exactly as it is closed for the voiceprints).
fn voice_artifact_aad(name: &str) -> Vec<u8> {
    voice_aad("voice_artifacts", name, VOICE_ARTIFACT_AAD_SCHEMA)
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
/// mock runtime only (marked in the `algo` column). №556: audio artifacts are
/// encrypted AT REST with the SAME scheme (the `audio_encrypted` column,
/// the `voice_artifacts` AAD registry) — the pre-№556 plaintext `audio_bytes`
/// column is migrated away at init. The setup turns PRAGMA secure_delete ON
/// and every delete ends with a WAL checkpoint (TRUNCATE) — the strict-erasure
/// posture.
pub struct VoiceStore {
    conn: Mutex<Connection>,
}

/// Schema (№517 adds the additive `algo` column — old databases are
/// extended, never rejected; the №504 migration posture. №556 migrates the
/// audio-artifacts table to the encrypted shape — see the init_tables
/// migration note: the pre-№556 plaintext column cannot honestly become
/// ciphertext at init time):
/// ```sql
/// CREATE TABLE voice_artifacts (
///     name TEXT PRIMARY KEY,
///     audio_encrypted BLOB NOT NULL,  -- №556: AES-256-GCM nonce‖ct with
///                                     -- the voice_artifacts AAD registry
///                                     -- (№527 subject binding); the mock
///                                     -- runtime keeps the visible
///                                     -- INSECURE-XOR-MOCK mark
///     algo TEXT,           -- №556: 'AES-256-GCM-v1' or 'INSECURE-XOR-MOCK'
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
        // №556: the strict-erasure setup — PRAGMA secure_delete = ON is a
        // PER-CONNECTION setting: deleted content is overwritten before the
        // pages are freed, so the freelist cannot carry the erased bytes
        // either. Set FIRST (before any table work), verified loudly: a
        // connection that refuses the posture is an init error, never a
        // silent degradation.
        let secure_delete: i64 = conn
            .query_row("PRAGMA secure_delete = ON;", [], |row| row.get(0))
            .map_err(|e| format!("voice store secure_delete setup: {}", e))?;
        if secure_delete != 1 {
            return Err(format!(
                "voice store secure_delete setup: the PRAGMA returned {secure_delete} — the strict-erasure posture is not active (fail-closed, №556)"
            ));
        }
        // №556 migration: a pre-№556 database carries the legacy plaintext
        // `audio_bytes` column. The plaintext payload CANNOT honestly become
        // ciphertext at init time (the key is not available here; the №517
        // posture refuses keyless persistence) and no released version ever
        // wrote an audio row (the write path never shipped — the honest
        // store was born in №517 with voiceprints only). The migration
        // SECURES the legacy bytes (the zero-overwrite first — the №526
        // posture; secure_delete is already ON so the DROP purges the freed
        // pages) and lets the CREATE below rebuild the table in the new
        // shape. The dev-era rows are NOT carried.
        let artifacts_table_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'voice_artifacts'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("voice store migration probe: {}", e))?;
        if artifacts_table_exists > 0 {
            let legacy_column: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('voice_artifacts') WHERE name = 'audio_bytes'",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| format!("voice store migration probe: {}", e))?;
            if legacy_column > 0 {
                conn.execute(
                    "UPDATE voice_artifacts SET audio_bytes = zeroblob(LENGTH(audio_bytes))",
                    [],
                )
                .map_err(|e| format!("voice store migration (secure the legacy bytes): {}", e))?;
                conn.execute_batch("DROP TABLE voice_artifacts;")
                    .map_err(|e| format!("voice store migration (drop the legacy table): {}", e))?;
            }
        }
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS voice_artifacts (
                name TEXT PRIMARY KEY,
                audio_encrypted BLOB NOT NULL,
                algo TEXT,
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
    /// for every readable row (the subject-bound ciphertext). Since the
    /// v0.28.0 deadline (№550 — the limitations.md TRANSITION row
    /// honored) a №517-era legacy row (empty AAD) refuses with
    /// [VOICEPRINT_DECRYPT] instead of reading — the only path back is
    /// re-enroll/re-save. The write path is always AAD-bound.
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
                // AadBound shape without a GCM round-trip.
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

    /// Save an audio artifact (№556) — the AT-REST contour for the
    /// `voice_artifacts` table. The SAME dual-path as `save_voiceprint`:
    /// the real runtime persists AES-256-GCM ciphertext (the №527 subject
    /// binding, the `voice_artifacts` AAD registry; a keyless write fails
    /// closed with [VOICE_INSECURE_STORE]); the mock runtime keeps the
    /// insecure XOR placeholder with the visible INSECURE-XOR-MOCK mark.
    /// BOUNDARY: this is a store-level method — wiring a program-facing
    /// audio write surface is NOT in №556.
    pub fn save_audio_artifact(
        &self,
        name: &str,
        audio: &[u8],
        manifest_json: Option<&str>,
        key_hex: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("lock: {}", e))?;
        let (stored, algo): (Vec<u8>, &'static str) = if crate::llm::mock_llm_requested() {
            // INSECURE placeholder — the mock-only skeleton path (№512);
            // the mark is visible in the schema (№517 posture), never
            // faked as crypto.
            (
                self.insecure_placeholder(audio, name),
                ARTIFACT_ALGO_INSECURE_MOCK,
            )
        } else {
            let key_hex = key_hex.ok_or_else(|| {
                crate::interpreter::values::coded_error(
                    crate::interpreter::values::CODE_VOICE_INSECURE_STORE,
                    format!(
                        "audio artifact '{}' not saved: no key provided — audio is persisted ONLY under AES-256-GCM with a secret()-gate key (64 hex chars); unencrypted persistence stays refused (fail-closed, №556)",
                        name
                    ),
                )
            })?;
            (
                encrypt_audio_artifact(audio, key_hex, name)?,
                ARTIFACT_ALGO_AES_GCM,
            )
        };
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR REPLACE INTO voice_artifacts (name, audio_encrypted, algo, manifest_json, saved_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![name, stored, algo, manifest_json, now],
        )
        .map_err(|e| format!("voice store artifact save '{}': {}", name, e))?;
        Ok(())
    }

    /// Load an audio artifact by name (№556) — returns the decrypted audio
    /// bytes and the manifest (None if the row carries none). The SAME
    /// loud-legacy posture as the voiceprints: a NULL-algo row refuses
    /// (it cannot exist after the №556 migration — a NULL-algo row means
    /// tampering or a foreign writer), an unknown algo refuses, the
    /// AES-256-GCM path requires the key and the №527 subject binding
    /// (a transplanted blob refuses with [VOICE_ARTIFACT_DECRYPT]).
    pub fn load_audio_artifact(
        &self,
        name: &str,
        key_hex: Option<&str>,
    ) -> Result<(Vec<u8>, Option<String>), String> {
        let conn = self.conn.lock().map_err(|e| format!("lock: {}", e))?;
        let row = conn
            .query_row(
                "SELECT audio_encrypted, algo, manifest_json FROM voice_artifacts WHERE name = ?1",
                rusqlite::params![name],
                |row| {
                    let stored: Vec<u8> = row.get(0)?;
                    let algo: Option<String> = row.get(1)?;
                    let manifest: Option<String> = row.get(2)?;
                    Ok((stored, algo, manifest))
                },
            )
            .map_err(|e| format!("voice store artifact load '{}': {}", name, e))?;
        let (stored, algo, manifest) = row;
        let bytes = match algo.as_deref() {
            None => {
                return Err(crate::interpreter::values::coded_error(
                    crate::interpreter::values::CODE_VOICE_ARTIFACT_DECRYPT,
                    format!(
                        "audio artifact '{}': a row without the algo mark (the pre-№556 plaintext era is migrated away; NULL means a foreign writer or tampering) — refusing (№556)",
                        name
                    ),
                ))
            }
            Some(ARTIFACT_ALGO_INSECURE_MOCK) => {
                // The mock skeleton path — XOR restore (NOT decryption);
                // the key is keyed by the name, exactly like the
                // voiceprint placeholder path.
                self.insecure_restore(&stored, name)
            }
            Some(ARTIFACT_ALGO_AES_GCM) => {
                let key_hex = key_hex.ok_or_else(|| {
                    crate::interpreter::values::coded_error(
                        crate::interpreter::values::CODE_VOICE_ARTIFACT_DECRYPT,
                        format!(
                            "audio artifact '{}': the row is AES-256-GCM encrypted (№556) — the key is required to load it",
                            name
                        ),
                    )
                })?;
                decrypt_audio_artifact(&stored, key_hex, name)?
            }
            Some(other) => {
                return Err(crate::interpreter::values::coded_error(
                    crate::interpreter::values::CODE_VOICE_ARTIFACT_DECRYPT,
                    format!(
                        "audio artifact '{}': unknown storage algo '{}' — refusing (№556)",
                        name, other
                    ),
                ))
            }
        };
        Ok((bytes, manifest))
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
        // №556: the column is the encrypted shape now.
        conn.execute(
            "UPDATE voice_artifacts SET audio_encrypted = zeroblob(LENGTH(audio_encrypted)) \
             WHERE name = ?1",
            rusqlite::params![name],
        )
        .map_err(|e| format!("voice store artifact overwrite '{}': {}", name, e))?;
        conn.execute(
            "DELETE FROM voice_artifacts WHERE name = ?1",
            rusqlite::params![name],
        )
        .map_err(|e| format!("voice store artifact delete '{}': {}", name, e))?;
        // (4) №556: the WAL checkpoint — the deleted page must not live on
        // in the write-ahead log after the erasure returns. TRUNCATE resets
        // the WAL file to zero length (a non-WAL connection takes the
        // harmless no-op checkpoint: busy = 0). A busy WAL is a LOUD error —
        // silence about a page that may still hold the erased bytes is the
        // exact lie №556 closes.
        let (busy, _log_pages, _checkpointed): (i64, i64, i64) = conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE);", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(|e| format!("voice store wal checkpoint: {}", e))?;
        if busy != 0 {
            return Err(
                "voice store wal checkpoint: the WAL was BUSY — the deleted page may still live in the log; retry the erasure (fail-closed, №556)".into(),
            );
        }
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
    aes_gcm_encrypt(data, key_hex, &voiceprint_aad(name))
        .map_err(|e| format!("voiceprint '{}': {}", name, e))
}

/// №556: the audio-artifact encrypt wrapper — the SAME shared AES-256-GCM
/// core, the `voice_artifacts` AAD registry.
fn encrypt_audio_artifact(data: &[u8], key_hex: &str, name: &str) -> Result<Vec<u8>, String> {
    aes_gcm_encrypt(data, key_hex, &voice_artifact_aad(name))
        .map_err(|e| format!("audio artifact '{}': {}", name, e))
}

/// The shared AES-256-GCM encrypt core (№517 → №556): the key is 32 bytes
/// (64 hex chars — the secret()-gate value, NEVER derived from the name),
/// the nonce is a fresh random 96 bits per write, the stored blob is
/// self-contained `nonce ‖ ciphertext+tag`, the AAD binds the ciphertext
/// to its subject (№527 — the caller composes the registry-specific AAD),
/// the decoded key buffer lives under Zeroizing.
fn aes_gcm_encrypt(data: &[u8], key_hex: &str, aad: &[u8]) -> Result<Vec<u8>, String> {
    use aes_gcm::aead::{Aead, KeyInit, Payload};
    use aes_gcm::{Aes256Gcm, Key, Nonce};
    use zeroize::Zeroizing;

    // №527: the decoded key material lives under Zeroizing — the buffer is
    // wiped when the scope exits, the key's plaintext lifetime is the
    // single operation (the hex string itself comes from the secret() gate,
    // whose lifetime is the caller's — unchanged by №527).
    let key_bytes =
        Zeroizing::new(hex::decode(key_hex).map_err(|e| format!("the key must be hex: {}", e))?);
    if key_bytes.len() != 32 {
        return Err(crate::interpreter::values::coded_error(
            crate::interpreter::values::CODE_VOICE_INSECURE_STORE,
            format!(
                "the key must be 256-bit (64 hex chars), got {} bytes — fail-closed (№517)",
                key_bytes.len()
            ),
        ));
    }
    let key = Key::<Aes256Gcm>::try_from(key_bytes.as_slice())
        .map_err(|_| "key conversion failed".to_string())?;
    let cipher = Aes256Gcm::new(&key);
    // Fresh random 96-bit nonce per write — uniqueness by construction.
    let mut nonce_bytes = [0u8; 12];
    use rand::Rng;
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::try_from(nonce_bytes.as_slice())
        .map_err(|_| "nonce conversion failed".to_string())?;
    // №527: the AAD binds the ciphertext to (subject_id, registry, schema
    // version) — a blob transplanted onto another subject's row fails the
    // authentication (the swap attack the bare tag accepted is closed).
    let ciphertext = cipher
        .encrypt(&nonce, Payload { msg: data, aad })
        .map_err(|e| format!("AES-256-GCM encryption failed: {}", e))?;
    let mut blob = nonce.to_vec();
    blob.extend_from_slice(&ciphertext);
    Ok(blob)
}

/// AES-256-GCM decrypt a stored voiceprint blob (№517 → №527). A wrong key
/// or a corrupted blob refuses LOUDLY ([VOICEPRINT_DECRYPT] — the GCM auth
/// tag does not lie). №527 + the v0.28.0 deadline (№550): the ONLY
/// attempt is the subject-bound decrypt (the №527 AAD). The transitional
/// empty-AAD attempt for the №517-era legacy rows is REMOVED (the
/// limitations.md TRANSITION row honored) — a legacy row now refuses
/// with the same single coded refusal, and the only path back is
/// re-enroll/re-save. A wrong key fails the attempt — nothing leaks.
fn decrypt_voiceprint_with_status(
    blob: &[u8],
    key_hex: &str,
    name: &str,
) -> Result<(Vec<u8>, VoiceprintCryptoStatus), String> {
    let plaintext = aes_gcm_decrypt(blob, key_hex, &voiceprint_aad(name)).map_err(|reason| {
        crate::interpreter::values::coded_error(
            crate::interpreter::values::CODE_VOICEPRINT_DECRYPT,
            format!("voiceprint '{}': {}", name, reason.describe()),
        )
    })?;
    Ok((plaintext, VoiceprintCryptoStatus::AadBound))
}

/// №556: the audio-artifact decrypt wrapper — the SAME shared core, the
/// `voice_artifacts` AAD registry, the [VOICE_ARTIFACT_DECRYPT] code.
fn decrypt_audio_artifact(blob: &[u8], key_hex: &str, name: &str) -> Result<Vec<u8>, String> {
    aes_gcm_decrypt(blob, key_hex, &voice_artifact_aad(name)).map_err(|reason| {
        crate::interpreter::values::coded_error(
            crate::interpreter::values::CODE_VOICE_ARTIFACT_DECRYPT,
            format!("audio artifact '{}': {}", name, reason.describe()),
        )
    })
}

/// The shared decrypt-failure taxonomy (№556): the core names WHAT failed,
/// the wrappers own the coded message and the subject label — the voiceprint
/// refusals keep their exact №517/№527-era wording (the tests pin the
/// substrings), the artifacts get the mirror wording under their own code.
enum DecryptFailure {
    TooShort,
    BadHex(String),
    KeyLength(usize),
    KeyConversion,
    NonceConversion,
    AuthFailed,
}

impl DecryptFailure {
    fn describe(&self) -> String {
        match self {
            Self::TooShort => {
                "the stored blob is too short to carry nonce‖ciphertext — corrupted (№517)"
                    .to_string()
            }
            Self::BadHex(e) => format!("the key must be hex: {}", e),
            Self::KeyLength(n) => format!(
                "the key must be 256-bit (64 hex chars), got {} bytes (№517)",
                n
            ),
            Self::KeyConversion => "key conversion failed".to_string(),
            Self::NonceConversion => "nonce conversion failed".to_string(),
            Self::AuthFailed =>
                "AES-256-GCM authentication FAILED — wrong key, corrupted data, or a legacy №517-era row (the empty-AAD transitional read was removed at v0.28.0): re-enroll or re-save to bind the subject (№527)"
                    .to_string(),
        }
    }
}

/// The shared AES-256-GCM decrypt core (№517 → №527 → №556): the ONLY
/// attempt is the subject-bound decrypt — the caller composes the
/// registry-specific AAD; the transitional empty-AAD legacy read stays
/// REMOVED (the v0.28.0 deadline, №550). A wrong key fails the attempt —
/// nothing leaks.
fn aes_gcm_decrypt(blob: &[u8], key_hex: &str, aad: &[u8]) -> Result<Vec<u8>, DecryptFailure> {
    use aes_gcm::aead::{Aead, KeyInit, Payload};
    use aes_gcm::{Aes256Gcm, Key, Nonce};
    use zeroize::Zeroizing;

    if blob.len() < 13 {
        return Err(DecryptFailure::TooShort);
    }
    // №527: the decoded key material lives under Zeroizing (the wipe at the
    // scope exit — the same minimal-lifetime posture as the encrypt side).
    let key_bytes =
        Zeroizing::new(hex::decode(key_hex).map_err(|e| DecryptFailure::BadHex(e.to_string()))?);
    if key_bytes.len() != 32 {
        return Err(DecryptFailure::KeyLength(key_bytes.len()));
    }
    let key = Key::<Aes256Gcm>::try_from(key_bytes.as_slice())
        .map_err(|_| DecryptFailure::KeyConversion)?;
    let cipher = Aes256Gcm::new(&key);
    let (nonce_bytes, ciphertext) = blob.split_at(12);
    let nonce = Nonce::try_from(nonce_bytes).map_err(|_| DecryptFailure::NonceConversion)?;
    // The ONLY attempt: the №527 subject-bound composition. The
    // transitional legacy read (the №517-era empty AAD) is REMOVED at the
    // v0.28.0 deadline (№550 — the limitations.md TRANSITION row): a blob
    // that does not authenticate under its subject AAD is a loud refusal,
    // never a silent fallback — the swap residue the window existed for
    // is gone with it.
    cipher
        .decrypt(
            &nonce,
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| DecryptFailure::AuthFailed)
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
        // (№556: the store-level at-rest contour; the mock runtime keeps
        // the visible-mark placeholder path)
        store
            .save_audio_artifact("frank", &[0xABu8; 128], None, None)
            .unwrap();
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
