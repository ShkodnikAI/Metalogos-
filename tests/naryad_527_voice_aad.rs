// ── tests/naryad_527_voice_aad.rs ───────────────────────────────────────
// №527 (P1, security/voice; the consolidated audit 30.09 N-3): the
// AES-256-GCM ciphertext becomes BOUND TO ITS SUBJECT — the GCM AAD
// carries (subject_id, registry, schema version), so a voiceprint blob
// transplanted onto another subject's row fails authentication (the
// swap attack №517 left open: the tag validated for ANY permutation).
//   T1: THE SWAP REFUSAL — A's blob under B's row is a loud
//       [VOICEPRINT_DECRYPT] refusal, nothing is returned.
//   T2: THE LEGACY REFUSAL (the v0.28.0 deadline executed) — a legacy
//       №517 row (empty AAD, the same algo mark — the schema is
//       untouched) REFUSES with the single coded [VOICEPRINT_DECRYPT]
//       refusal through BOTH entry points; the transitional read and
//       the LegacyNoAad flag are removed (the №524 rule — the
//       limitations row closes in the same PR as the removal).
//   T3: THE MIGRATION STORY — a legacy row refuses first, a re-save
//       goes through the AAD-bound write path (every write is bound),
//       and the rebound row loads and refuses the swap again.
//   T4: THE WRONG KEY still refuses loudly — the single-attempt decrypt
//       leaks nothing: the answer is the same coded refusal.
//   T5: FRESH WRITES ARE AAD-BOUND — the status flag of every freshly
//       saved row is AadBound (no new legacy rows can be produced).
// Boundaries held: the storage schema and the secret()-gate semantics
// are untouched (the algo mark stays 'AES-256-GCM-v1' — №517's T2 pins
// it); the discriminator is the GCM authentication itself. №517's tests
// stay green unchanged.
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

// ── T2: the legacy refusal (the v0.28.0 deadline executed) ─────────────

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

    // The deadline is executed: the transitional read is GONE — the
    // legacy row refuses with the single coded refusal through BOTH
    // entry points (the compat load and the status-returning load).
    // Nothing is returned, nothing leaks — the same loud failure as a
    // wrong key; the only path back is re-enroll/re-save (T3).
    let err = store.load_voiceprint("legacy517", Some(KEY_A)).unwrap_err();
    assert!(
        err.starts_with("[VOICEPRINT_DECRYPT] "),
        "the legacy row refuses loudly after the deadline, got: {}",
        err
    );
    assert!(store
        .load_voiceprint_with_status("legacy517", Some(KEY_A))
        .is_err());

    pin_mock(false);
}

// ── T3: the migration story — a re-save rebinds the row ─────────────────

#[test]
fn n527_resave_of_legacy_row_rebinds_and_refuses_swap() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(false);

    let store = test_store();
    let embedding = [0.125f32, 3.5];

    // A legacy row again (the T2 fixture shape).
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
    // The deadline first: the legacy row refuses (T2's property).
    assert!(store
        .load_voiceprint_with_status("carol", Some(KEY_A))
        .is_err());

    // The re-enroll path: the write path is ALWAYS AAD-bound (№527 task 1:
    // "запись — только с AAD"). The row is now bound to its subject.
    store
        .save_voiceprint("carol", &embedding, "chatterbox-v3", Some(KEY_A))
        .unwrap();
    let (_, _, status2) = store
        .load_voiceprint_with_status("carol", Some(KEY_A))
        .unwrap();
    assert_eq!(
        status2,
        metalogos::voice::store::VoiceprintCryptoStatus::AadBound,
        "the re-saved row is bound — the transition window shrank by one row"
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
fn n527_wrong_key_refusal_survives_the_two_step_decrypt() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(false);

    let store = test_store();
    store
        .save_voiceprint("eve", &[0.1f32, 0.2], "koko-ro-82m", Some(KEY_A))
        .unwrap();

    // The wrong key fails BOTH decrypt attempts (bound and transitional):
    // the answer stays the single loud [VOICEPRINT_DECRYPT] refusal.
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
