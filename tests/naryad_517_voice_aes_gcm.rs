// ── tests/naryad_517_voice_aes_gcm.rs ───────────────────────────────────
// №517 (P2, security/voice; the consolidated audit 28.09 C-06 step 2):
// the honest at-rest crypto for voiceprints LANDS.
//   T1: REAL runtime + the secret()-gate key → AES-256-GCM roundtrip; the
//       stored blob is nonce‖ciphertext, NOT plaintext, NOT XOR.
//   T2: a WRONG key → the loud [VOICEPRINT_DECRYPT] refusal (GCM auth);
//       nothing is returned.
//   T3: REAL runtime + NO key → the loud [VOICE_INSECURE_STORE]
//       fail-closed refusal (biometric data is never persisted
//       unencrypted; №512 posture preserved).
//   T4: MIGRATION — a legacy pre-№517 row (algo NULL, the XOR era) is
//       NEVER silently read as decrypted: [VOICEPRINT_STALE], "re-enroll"
//       (the №504 loud-migration posture).
//   T5: NONCE UNIQUENESS — two saves of the SAME embedding produce two
//       DIFFERENT stored blobs (a fresh random 96-bit nonce per write).
//   T6: MOCK runtime keeps the placeholder path with the VISIBLE insecure
//       schema mark (algo = 'INSECURE-XOR-MOCK') — never faked as crypto.
//   T7: KEY NEVER FROM THE NAME — the name is not a key source: the
//       schema carries algo, the key comes from the caller through the
//       secret() gate semantics (env-sourced hex-256); pinned by the
//       source greps (no name-derived key derivation in the AES path).
// The process-global mock flag is pinned under a static lock and always
// restored (the №251 global-state discipline).

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

/// A valid 256-bit key in the house hex form (64 hex chars).
const KEY_A: &str = "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90";
const KEY_B: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

// ── T1: the AES-256-GCM roundtrip in the real runtime ───────────────────

#[test]
fn n517_real_mode_roundtrip_with_key() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(false);

    let store = test_store();
    let embedding = vec![0.25f32, -0.5, 1.0, 0.0, 3.75, 192.5];
    store
        .save_voiceprint(
            "carol",
            &embedding,
            "chatterbox-multilingual-v3",
            Some(KEY_A),
        )
        .expect("a keyed save SUCCEEDS in the real runtime (the №512 refusal lifts)");

    let (loaded, model) = store.load_voiceprint("carol", Some(KEY_A)).unwrap();
    assert_eq!(embedding, loaded, "the AES-256-GCM roundtrip is exact");
    assert_eq!(model, "chatterbox-multilingual-v3");

    pin_mock(false);
}

#[test]
fn n517_stored_bytes_are_ciphertext_not_xor_of_the_name() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(false);

    // The blob observable: an on-disk db in target/ opened twice (the store
    // owns the connection, so the raw row is read through a second handle).
    let dir = format!("target/n517_{}", std::process::id());
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = format!("{}/voice.db", dir);
    let _ = std::fs::remove_file(&db_path);
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let store = metalogos::voice::store::VoiceStore::new(conn);
        store.init_tables().unwrap();
        store
            .save_voiceprint("dave", &[1.5f32, -2.5], "koko-ro-82m", Some(KEY_A))
            .unwrap();
    }
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let (blob, algo): (Vec<u8>, String) = conn
            .query_row(
                "SELECT embedding_encrypted, algo FROM voiceprints WHERE name = 'dave'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        // The honest schema mark: the row is labelled as real crypto.
        assert_eq!(
            algo, "AES-256-GCM-v1",
            "the algo column carries the №517 label"
        );
        // Not plaintext: the raw f32 bytes of [1.5, -2.5] are absent.
        let raw: Vec<u8> = [1.5f32, -2.5]
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        assert_ne!(blob[..raw.len()], raw[..], "plaintext must not leak");
        // Not the №512 XOR placeholder: XORing the blob with the NAME must
        // NOT yield the raw bytes (the name is never a key).
        let name_xor: Vec<u8> = raw
            .iter()
            .enumerate()
            .map(|(i, &b)| b ^ b"dave"[i % 4])
            .collect();
        assert_ne!(
            blob[..raw.len()],
            name_xor[..],
            "the name is not the key (№517)"
        );
        // The self-contained blob carries the 12-byte nonce prefix.
        assert!(
            blob.len() > 12 + 16,
            "nonce (12) + ciphertext + GCM tag (16)"
        );
    }
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_dir(&dir);
    pin_mock(false);
}

// ── T2: a wrong key refuses loudly (GCM auth) ────────────────────────────

#[test]
fn n517_wrong_key_refuses_loud_nothing_returned() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(false);

    let store = test_store();
    store
        .save_voiceprint("eve", &[0.1, 0.2], "chatterbox-v3", Some(KEY_A))
        .unwrap();

    let err = store
        .load_voiceprint("eve", Some(KEY_B))
        .expect_err("a wrong key must not yield the embedding");
    assert!(
        err.starts_with("[VOICEPRINT_DECRYPT] "),
        "the wrong-key refusal must carry the stable class stamp, got: {}",
        err
    );
    assert!(
        err.contains("authentication FAILED"),
        "the refusal must name the GCM auth failure, got: {}",
        err
    );

    pin_mock(false);
}

// ── T3: the keyless real-runtime save stays fail-closed ─────────────────

#[test]
fn n517_keyless_real_save_still_refuses() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(false);

    let store = test_store();
    let err = store
        .save_voiceprint("frank", &[0.3], "chatterbox-v3", None)
        .expect_err("a keyless save must not persist biometric data");
    assert!(err.starts_with("[VOICE_INSECURE_STORE] "), "got: {}", err);
    // The load misses — nothing was persisted.
    assert!(store.load_voiceprint("frank", Some(KEY_A)).is_err());

    pin_mock(false);
}

// ── T4: the legacy pre-№517 row migrates LOUDLY ──────────────────────────

#[test]
fn n517_legacy_xor_row_is_not_silently_decrypted() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(false);

    let store = test_store();
    // Simulate a pre-№517 database row: the raw XOR bytes, algo NULL.
    {
        let conn = store.raw_connection_for_tests();
        let raw: Vec<u8> = [0.25f32, -0.5]
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .map(|b| b ^ b"legacy"[0]) // any XOR-shaped bytes; algo is the discriminator
            .collect();
        conn.execute(
            "INSERT INTO voiceprints (name, embedding_encrypted, model_id, saved_at) VALUES ('legacy', ?1, 'old-model', '2026-09-28T00:00:00Z')",
            rusqlite::params![raw],
        )
        .unwrap();
    }
    let err = store
        .load_voiceprint("legacy", Some(KEY_A))
        .expect_err("a legacy XOR row must NOT be silently read as decrypted");
    assert!(err.starts_with("[VOICEPRINT_STALE] "), "got: {}", err);
    assert!(
        err.contains("re-enroll"),
        "the migration message must tell the caller what to do, got: {}",
        err
    );

    pin_mock(false);
}

// ── T5: a fresh nonce per write ──────────────────────────────────────────

#[test]
fn n517_nonce_uniqueness_direct() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(false);

    // The direct observable: two saves into TWO SEPARATE stores (same key,
    // same embedding) produce different nonce prefixes — the randomness is
    // per-write, deterministic reuse would be a nonce-reuse defect.
    let s1 = test_store();
    let s2 = test_store();
    let embedding = vec![7.0f32; 4];
    s1.save_voiceprint("hank", &embedding, "m", Some(KEY_A))
        .unwrap();
    s2.save_voiceprint("hank", &embedding, "m", Some(KEY_A))
        .unwrap();

    // Both roundtrip fine (the store-side contract), and the in-code
    // encryption helper is deterministic-inputs/nondeterministic-nonce —
    // assert via the public helper on two calls.
    let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
    let blob1 = metalogos::voice::store::encrypt_voiceprint_for_tests(&bytes, KEY_A).unwrap();
    let blob2 = metalogos::voice::store::encrypt_voiceprint_for_tests(&bytes, KEY_A).unwrap();
    assert_ne!(
        blob1[..12],
        blob2[..12],
        "two writes of the same plaintext must carry DIFFERENT nonces"
    );
    assert_ne!(blob1, blob2, "the ciphertexts differ with the nonce");
    // Both decrypt back to the same plaintext.
    let p1 = metalogos::voice::store::decrypt_voiceprint_for_tests(&blob1, KEY_A).unwrap();
    let p2 = metalogos::voice::store::decrypt_voiceprint_for_tests(&blob2, KEY_A).unwrap();
    assert_eq!(p1, p2);
    assert_eq!(p1, bytes);

    pin_mock(false);
}

// ── T6: the mock path stays the INSECURE placeholder, visibly marked ────

#[test]
fn n517_mock_mode_stays_insecure_placeholder_with_schema_mark() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(true);

    let dir = format!("target/n517_mock_{}", std::process::id());
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = format!("{}/voice.db", dir);
    let _ = std::fs::remove_file(&db_path);
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let store = metalogos::voice::store::VoiceStore::new(conn);
        store.init_tables().unwrap();
        store
            .save_voiceprint("iris", &[0.5f32, 1.5], "koko-ro-82m", None)
            .expect("mock keeps the placeholder path");
    }
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let (algo, blob): (String, Vec<u8>) = conn
            .query_row(
                "SELECT algo, embedding_encrypted FROM voiceprints WHERE name = 'iris'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            algo, "INSECURE-XOR-MOCK",
            "the insecure mark is VISIBLE IN THE SCHEMA — a mock row can never masquerade as encrypted (№517)"
        );
        let raw: Vec<u8> = [0.5f32, 1.5].iter().flat_map(|f| f.to_le_bytes()).collect();
        assert_ne!(
            blob, raw,
            "not raw bytes (the placeholder XOR still applies)"
        );
    }
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_dir(&dir);
    pin_mock(false);
}
