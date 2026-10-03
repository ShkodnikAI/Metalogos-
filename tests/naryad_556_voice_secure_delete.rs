// ── tests/naryad_556_voice_secure_delete.rs ──────────────────────────
// №556 (Wave 25 P1; the audit 02.10 M-6; dispatch gh#925): the strict
// erasure of the biometric/audio residuals and the honest at-rest
// crypto for the audio artifacts.
//
// The contract pinned here:
//   - the store's setup turns PRAGMA secure_delete = ON (the deleted
//     content is overwritten before the pages are freed — the freelist
//     cannot carry the erased bytes either) and the init REFUSES loudly
//     if the posture is not active;
//   - every delete_voiceprint ends with PRAGMA wal_checkpoint(TRUNCATE):
//     a WAL-mode database's write-ahead log is reset to ZERO LENGTH by
//     the call — the deleted page does not live on in the journal (the
//     exact residual the audit found: the n526 zero-overwrite left the
//     page image in the WAL);
//   - the audio artifacts are encrypted AT REST with the SAME scheme as
//     the voiceprints (№517 AES-256-GCM + the №527 subject binding, the
//     `voice_artifacts` AAD registry): a wrong key refuses, a blob
//     transplanted onto another name refuses, a keyless real-runtime
//     write fails closed ([VOICE_INSECURE_STORE]);
//   - the migration: a pre-№556 database's legacy plaintext
//     `audio_bytes` table is secured (zero-overwrite) and rebuilt in the
//     encrypted shape — the plaintext rows are NOT carried (no released
//     version ever wrote one; the key is unavailable at init).
#![allow(clippy::disallowed_methods)]

use std::sync::Mutex;

static MOCK_LOCK: Mutex<()> = Mutex::new(());

fn pin_mock(on: bool) {
    if on {
        std::env::set_var("METALOGOS_MOCK_LLM", "1");
    } else {
        std::env::remove_var("METALOGOS_MOCK_LLM");
    }
}

fn test_store() -> metalogos::voice::store::VoiceStore {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let store = metalogos::voice::store::VoiceStore::new(conn);
    store.init_tables().unwrap();
    store
}

const KEY_A: &str = "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90";
const KEY_B: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

// ── the PRAGMA posture ────────────────────────────────────────────────

#[test]
fn n556_secure_delete_pragma_is_on_after_init() {
    let store = test_store();
    let conn = store.raw_connection_for_tests();
    let on: i64 = conn
        .query_row("PRAGMA secure_delete;", [], |row| row.get(0))
        .unwrap();
    assert_eq!(on, 1, "secure_delete must be ON for the store's connection");
}

#[test]
fn n556_wal_checkpoint_truncates_the_log_after_delete() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(false);
    let dir = std::env::temp_dir().join(format!(
        "n556_wal_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = dir.join("voice.db");
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        // The WAL-mode deployment — the shape the checkpoint matters for.
        let mode: String = conn
            .query_row("PRAGMA journal_mode=WAL;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(mode, "wal", "the fixture runs the WAL journal mode");
        let store = metalogos::voice::store::VoiceStore::new(conn);
        store.init_tables().unwrap();
        store
            .save_voiceprint("wal-subject", &[0.5f32; 128], "koko-ro-82m", Some(KEY_A))
            .unwrap();
        let wal_path = dir.join("voice.db-wal");
        let wal_len_after_write = std::fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0);
        // sanity: the WAL actually carries page images after the writes
        // (otherwise this fixture proves nothing about the truncate)
        assert!(
            wal_len_after_write > 0,
            "the WAL must hold pages after the writes (got {} bytes)",
            wal_len_after_write
        );
        assert!(store.delete_voiceprint("wal-subject").unwrap());
        // THE CONTRACT: after the erasure returns, the WAL is truncated to
        // zero length — the deleted page is not in the journal anymore.
        let wal_len_after_delete = std::fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0);
        assert_eq!(
            wal_len_after_delete, 0,
            "wal_checkpoint(TRUNCATE) must reset the WAL (got {} bytes)",
            wal_len_after_delete
        );
    }
    pin_mock(false);
    let _ = std::fs::remove_dir_all(&dir);
}

// ── the audio artifacts at rest (the encrypted shape) ────────────────

#[test]
fn n556_artifact_roundtrip_is_encrypted_at_rest() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(false);
    let store = test_store();
    let audio = vec![0xABu8; 512];
    store
        .save_audio_artifact("clip", &audio, Some("{\"rate\":16000}"), Some(KEY_A))
        .unwrap();
    // at rest: the stored blob is NOT the plaintext, the algo mark is the
    // honest AES-256-GCM mark (the №517 posture mirrored for artifacts)
    {
        let conn = store.raw_connection_for_tests();
        let (stored, algo): (Vec<u8>, String) = conn
            .query_row(
                "SELECT audio_encrypted, algo FROM voice_artifacts WHERE name = 'clip'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_ne!(
            stored, audio,
            "the artifact must not land as raw bytes (№556)"
        );
        assert_eq!(algo, "AES-256-GCM-v1");
        assert!(stored.len() > 12 + 16, "nonce + tag must be present");
    }
    // the roundtrip: the same key returns the exact bytes + the manifest
    let (loaded, manifest) = store.load_audio_artifact("clip", Some(KEY_A)).unwrap();
    assert_eq!(loaded, audio);
    assert_eq!(manifest.as_deref(), Some("{\"rate\":16000}"));
    pin_mock(false);
}

#[test]
fn n556_artifact_wrong_key_refuses_loudly() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(false);
    let store = test_store();
    store
        .save_audio_artifact("clip", &[1u8; 64], None, Some(KEY_A))
        .unwrap();
    let err = store.load_audio_artifact("clip", Some(KEY_B)).unwrap_err();
    assert!(
        err.starts_with("[VOICE_ARTIFACT_DECRYPT] "),
        "the wrong-key refusal is the loud GCM auth failure, got: {}",
        err
    );
    assert!(err.contains("authentication FAILED"));
    pin_mock(false);
}

#[test]
fn n556_artifact_swap_between_names_fails_authentication() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(false);
    let store = test_store();
    store
        .save_audio_artifact("alice-clip", &[1u8; 128], None, Some(KEY_A))
        .unwrap();
    store
        .save_audio_artifact("bob-clip", &[2u8; 128], None, Some(KEY_A))
        .unwrap();
    // The swap: A's ciphertext is transplanted onto B's row — the №527
    // subject binding closes it for the artifacts exactly as for the
    // voiceprints (the AAD registry is `voice_artifacts`).
    {
        let conn = store.raw_connection_for_tests();
        let changed = conn
            .execute(
                "UPDATE voice_artifacts SET audio_encrypted = \
                 (SELECT audio_encrypted FROM voice_artifacts WHERE name = 'alice-clip') \
                 WHERE name = 'bob-clip'",
                [],
            )
            .unwrap();
        assert_eq!(changed, 1, "the swap fixture replaced exactly B's row");
    }
    let err = store
        .load_audio_artifact("bob-clip", Some(KEY_A))
        .unwrap_err();
    assert!(
        err.starts_with("[VOICE_ARTIFACT_DECRYPT] "),
        "the transplanted blob must refuse under B's subject, got: {}",
        err
    );
    // A is intact.
    let (loaded, _) = store
        .load_audio_artifact("alice-clip", Some(KEY_A))
        .unwrap();
    assert_eq!(loaded, vec![1u8; 128]);
    pin_mock(false);
}

#[test]
fn n556_artifact_keyless_real_write_fails_closed() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(false);
    let store = test_store();
    let err = store
        .save_audio_artifact("clip", &[7u8; 32], None, None)
        .unwrap_err();
    assert!(
        err.starts_with("[VOICE_INSECURE_STORE] "),
        "a keyless real-runtime write must fail closed (№517 posture), got: {}",
        err
    );
    // the failed write left NO row behind
    let conn = store.raw_connection_for_tests();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM voice_artifacts", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0, "the refused write persists nothing");
    pin_mock(false);
}

#[test]
fn n556_artifact_mock_row_keeps_the_visible_mark() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(true);
    let store = test_store();
    let audio = vec![0xCDu8; 96];
    store
        .save_audio_artifact("mock-clip", &audio, None, None)
        .unwrap();
    {
        let conn = store.raw_connection_for_tests();
        let (stored, algo): (Vec<u8>, String) = conn
            .query_row(
                "SELECT audio_encrypted, algo FROM voice_artifacts WHERE name = 'mock-clip'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_ne!(stored, audio, "not raw even in the mock path (№512)");
        assert_eq!(
            algo, "INSECURE-XOR-MOCK",
            "the mark is visible in the schema"
        );
    }
    // the mock roundtrip (no key — the placeholder is name-keyed)
    let (loaded, _) = store.load_audio_artifact("mock-clip", None).unwrap();
    assert_eq!(loaded, audio);
    pin_mock(false);
}

#[test]
fn n556_artifact_unknown_algo_refused() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(true);
    let store = test_store();
    store
        .save_audio_artifact("clip", &[1u8; 16], None, None)
        .unwrap();
    {
        let conn = store.raw_connection_for_tests();
        conn.execute(
            "UPDATE voice_artifacts SET algo = 'FAKE-ALGO' WHERE name = 'clip'",
            [],
        )
        .unwrap();
    }
    let err = store.load_audio_artifact("clip", None).unwrap_err();
    assert!(
        err.contains("unknown storage algo"),
        "a foreign algo mark refuses (№517 posture), got: {}",
        err
    );
    pin_mock(false);
}

// ── the migration: the legacy plaintext shape is secured and rebuilt ──

#[test]
fn n556_migration_secures_and_rebuilds_the_legacy_table() {
    let dir = std::env::temp_dir().join(format!(
        "n556_migration_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = dir.join("legacy.db");
    // A pre-№556 database: the legacy plaintext schema with a row.
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE voice_artifacts (
                name TEXT PRIMARY KEY,
                audio_bytes BLOB NOT NULL,
                manifest_json TEXT,
                saved_at TEXT NOT NULL
            );
            INSERT INTO voice_artifacts (name, audio_bytes, manifest_json, saved_at)
            VALUES ('legacy-clip', x'0102030405', NULL, '2026-09-30T00:00:00Z');",
        )
        .unwrap();
    }
    // The №556 init migrates.
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let store = metalogos::voice::store::VoiceStore::new(conn);
        store.init_tables().unwrap();
        {
            // the raw-guard block ENDS before any store method call — the
            // store's methods take the same connection mutex (a guard held
            // across save_voiceprint would self-deadlock the fixture)
            let conn = store.raw_connection_for_tests();
            // the new shape: audio_encrypted + algo, NO audio_bytes column
            let has_audio_bytes: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('voice_artifacts') WHERE name = 'audio_bytes'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(has_audio_bytes, 0, "the legacy column is gone");
            let has_encrypted: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('voice_artifacts') WHERE name = 'audio_encrypted'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(has_encrypted, 1, "the encrypted column is the new shape");
            let has_algo: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('voice_artifacts') WHERE name = 'algo'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(has_algo, 1, "the algo mark column is present");
            // the legacy plaintext row is NOT carried (no released version ever
            // wrote one; the key is unavailable at init — the №517 posture)
            let rows: i64 = conn
                .query_row("SELECT COUNT(*) FROM voice_artifacts", [], |row| row.get(0))
                .unwrap();
            assert_eq!(rows, 0, "the dev-era plaintext rows are not carried");
        }
        // the voiceprints table is untouched by the artifact migration
        store
            .save_voiceprint("post-migration", &[0.1f32], "koko-ro-82m", Some(KEY_A))
            .unwrap();
    }
    let _ = std::fs::remove_dir_all(&dir);
}
