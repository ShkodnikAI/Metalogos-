// ── tests/naryad_527_voice_aad.rs ───────────────────────────────────────
// №527 (P1, security/voice; the consolidated audit 30.09 N-3): the
// AES-256-GCM ciphertext becomes BOUND TO ITS SUBJECT — the GCM AAD
// carries (subject_id, registry, schema version), so a voiceprint blob
// transplanted onto another subject's row fails authentication (the
// swap attack №517 left open: the tag validated for ANY permutation).
//   T1: THE SWAP REFUSAL — A's blob under B's row is a loud
//       [VOICEPRINT_DECRYPT] refusal, nothing is returned.
//   T2: THE DEADLINE HONORED — a legacy №517 row (empty AAD, the same
//       algo mark — the schema is untouched) REFUSES with the loud
//       [VOICEPRINT_DECRYPT]: the transitional empty-AAD read existed
//       only between №527 and v0.28.0; the v0.28.0 release prep (№550)
//       removed it. No released version ever wrote a legacy row.
//   T3: THE MIGRATION STORY — with the transitional read gone, the ONLY
//       path back for a legacy row is re-enroll/re-save from the source
//       (the write path is always AAD-bound), and the rebound row
//       refuses the swap again.
//   T4: THE WRONG KEY still refuses loudly — the single subject-bound
//       decrypt leaks nothing: a wrong key is the same coded refusal.
//   T5: FRESH WRITES ARE AAD-BOUND — the status flag of every freshly
//       saved row is AadBound (no new legacy rows can be produced).
// Boundaries held: the storage schema and the secret()-gate semantics
// are untouched (the algo mark stays 'AES-256-GCM-v1' — №517's T2 pins
// it); the discriminator between AAD-bound and legacy rows is the GCM
// authentication itself. №517's tests stay green unchanged.
// The key material is wiped under Zeroizing (the №527 task 2) — the
// decoded 32-byte key buffer's lifetime is the single operation.

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

// ── T1: the swap attack is refused ──────────────────────────────────────

#[test]
fn n527_swap_between_subjects_fails_authentication() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(false);

    let store = test_store();
    let alice = vec![0.25f32, -0.5, 1.0];
    let bob = vec![9.75f32, -3.25, 0.125];
    store
        .save_voiceprint("alice", &alice, "koko-ro-82m", Some(KEY_A))
        .unwrap();
    store
        .save_voiceprint("bob", &bob, "koko-ro-82m", Some(KEY_A))
        .unwrap();

    // The swap: A's ciphertext blob is transplanted onto B's row (the
    // attacker controls the DB rows — the tag alone did not bind the
    // subject, №517 left this open).
    {
        let conn = store.raw_connection_for_tests();
        let changed = conn
            .execute(
                "UPDATE voiceprints SET embedding_encrypted = \
                 (SELECT embedding_encrypted FROM voiceprints WHERE name = 'alice') \
                 WHERE name = 'bob'",
                [],
            )
            .unwrap();
        assert_eq!(changed, 1, "the swap fixture replaced exactly B's row");
    }

    // The transplanted blob must NOT decrypt under B's subject: the AAD
    // (subject, registry, schema) does not match B's row composition.
    let err = store.load_voiceprint("bob", Some(KEY_A)).unwrap_err();
    assert!(
        err.starts_with("[VOICEPRINT_DECRYPT] "),
        "the swap refusal is the loud GCM auth failure, got: {}",
        err
    );
    // The same refusal through the status-returning entry point.
    assert!(store
        .load_voiceprint_with_status("bob", Some(KEY_A))
        .is_err());
    // A is intact: her own row still loads exactly.
    let (loaded, _, _) = store
        .load_voiceprint_with_status("alice", Some(KEY_A))
        .unwrap();
    assert_eq!(loaded, alice);
    assert_eq!(
        store
            .load_voiceprint_with_status("alice", Some(KEY_A))
            .unwrap()
            .2,
        metalogos::voice::store::VoiceprintCryptoStatus::AadBound
    );

    pin_mock(false);
}

// ── T2: the deadline honored — the legacy row REFUSES (no transitional read)

#[test]
fn n527_legacy_no_aad_row_refuses_after_the_v0280_deadline() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(false);

    let store = test_store();
    let embedding = [1.5f32, -2.5, 0.0, 42.0];

    // A №517-era row: encrypted with an EMPTY AAD (the old contour), the
    // same algo mark — the №527 schema boundary keeps the mark.
    let blob = metalogos::voice::store::encrypt_voiceprint_legacy_no_aad_for_tests(
        &embedding
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect::<Vec<u8>>(),
        KEY_A,
    )
    .unwrap();
    {
        let conn = store.raw_connection_for_tests();
        conn.execute(
            "INSERT INTO voiceprints (name, embedding_encrypted, model_id, saved_at, algo) \
             VALUES ('legacy517', ?1, 'chatterbox-v3', '2026-09-30T00:00:00Z', 'AES-256-GCM-v1')",
            rusqlite::params![blob],
        )
        .unwrap();
    }

    // The deadline (limitations.md TRANSITION row, №550): the row does
    // NOT decrypt anymore — neither through the compat entry point nor
    // through the status-returning one. The refusal is the single loud
    // [VOICEPRINT_DECRYPT], and it names the legacy possibility honestly.
    let err = store.load_voiceprint("legacy517", Some(KEY_A)).unwrap_err();
    assert!(
        err.starts_with("[VOICEPRINT_DECRYPT] "),
        "the legacy row refuses after the deadline, got: {}",
        err
    );
    assert!(
        err.contains("legacy №517-era row"),
        "the refusal names the removed transitional read, got: {}",
        err
    );
    assert!(store
        .load_voiceprint_with_status("legacy517", Some(KEY_A))
        .is_err());

    pin_mock(false);
}

// ── T3: the migration story — re-enroll/re-save is the ONLY path back ──

#[test]
fn n527_reenroll_is_the_only_path_back_and_rebinds() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(false);

    let store = test_store();
    let embedding = [0.125f32, 3.5];

    // A legacy row again (the T2 fixture shape) — unreadable now.
    let blob = metalogos::voice::store::encrypt_voiceprint_legacy_no_aad_for_tests(
        &embedding
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect::<Vec<u8>>(),
        KEY_A,
    )
    .unwrap();
    {
        let conn = store.raw_connection_for_tests();
        conn.execute(
            "INSERT INTO voiceprints (name, embedding_encrypted, model_id, saved_at, algo) \
             VALUES ('carol', ?1, 'chatterbox-v3', '2026-09-30T00:00:00Z', 'AES-256-GCM-v1')",
            rusqlite::params![blob],
        )
        .unwrap();
    }
    assert!(store
        .load_voiceprint_with_status("carol", Some(KEY_A))
        .is_err());

    // The re-enroll path — the ONLY one left: the write path is ALWAYS
    // AAD-bound (№527 task 1: "запись — только с AAD"). The row is bound
    // to its subject again.
    store
        .save_voiceprint("carol", &embedding, "chatterbox-v3", Some(KEY_A))
        .unwrap();
    let (_, _, status2) = store
        .load_voiceprint_with_status("carol", Some(KEY_A))
        .unwrap();
    assert_eq!(
        status2,
        metalogos::voice::store::VoiceprintCryptoStatus::AadBound,
        "the re-saved row is bound — the legacy population is gone"
    );

    // The rebound row refuses the swap again (the T1 property).
    store
        .save_voiceprint("dave", &[7.0f32], "chatterbox-v3", Some(KEY_A))
        .unwrap();
    {
        let conn = store.raw_connection_for_tests();
        conn.execute(
            "UPDATE voiceprints SET embedding_encrypted = \
             (SELECT embedding_encrypted FROM voiceprints WHERE name = 'carol') \
             WHERE name = 'dave'",
            [],
        )
        .unwrap();
    }
    let err = store.load_voiceprint("dave", Some(KEY_A)).unwrap_err();
    assert!(err.starts_with("[VOICEPRINT_DECRYPT] "), "got: {}", err);

    pin_mock(false);
}

// ── T4: the wrong key still refuses loudly, nothing leaks ───────────────

#[test]
fn n527_wrong_key_still_refuses_loudly() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(false);

    let store = test_store();
    store
        .save_voiceprint("eve", &[0.1f32, 0.2], "koko-ro-82m", Some(KEY_A))
        .unwrap();

    // The single subject-bound decrypt: a wrong key is the one loud
    // [VOICEPRINT_DECRYPT] refusal — nothing leaks, nothing falls back.
    let err = store.load_voiceprint("eve", Some(KEY_B)).unwrap_err();
    assert!(err.starts_with("[VOICEPRINT_DECRYPT] "), "got: {}", err);
    assert!(store
        .load_voiceprint_with_status("eve", Some(KEY_B))
        .is_err());

    pin_mock(false);
}

// ── T5: fresh writes are always AAD-bound ───────────────────────────────

#[test]
fn n527_fresh_writes_carry_the_aad_binding() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(false);

    let store = test_store();
    let embedding = vec![2.25f32, -1.125];
    store
        .save_voiceprint("frank", &embedding, "koko-ro-82m", Some(KEY_A))
        .unwrap();
    let (loaded, model, status) = store
        .load_voiceprint_with_status("frank", Some(KEY_A))
        .unwrap();
    assert_eq!(loaded, embedding);
    assert_eq!(model, "koko-ro-82m");
    assert_eq!(
        status,
        metalogos::voice::store::VoiceprintCryptoStatus::AadBound,
        "every №527-era write is subject-bound — no new legacy rows"
    );

    pin_mock(false);
}
