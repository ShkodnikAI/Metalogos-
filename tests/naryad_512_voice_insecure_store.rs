// ── tests/naryad_512_voice_insecure_store.rs ────────────────────────────
// №512 (P1, security/voice; the consolidated audit 28.09 C-06, Medium):
// the voiceprint store comments claimed "encrypted at rest (AES-256-GCM)"
// while the code XORs with a key derived from the PUBLIC name — reversible
// by anyone who reads the table. A voiceprint is biometric data (GDPR
// Art. 9 special category): a fake crypto label misleads auditors, users
// and external reviewers alike. This naryad makes the surface HONEST:
//   T1: real runtime (no METALOGOS_MOCK_LLM) — a keyless save_voiceprint
//       REFUSES LOUDLY with the [VOICE_INSECURE_STORE] class (the
//       coded_error stamp, №413/ADR-0169 convention); NOTHING is
//       persisted. №517 EVOLVED the message: the store now encrypts with
//       AES-256-GCM when the secret()-gate key IS provided, so the
//       keyless refusal names the missing key, not the placeholder (the
//       placeholder is mock-only now).
//   T2: mock runtime — the placeholder path still works and roundtrips
//       (the skeleton tests are not broken; the fixtures carry the
//       insecure mark, allowed by the naryad boundary).
//   T3: source honesty pins — №517 FLIPPED the crypto pin: "AES-256-GCM"
//       is REQUIRED in store.rs (the claim is now TRUE) and must appear
//       NOWHERE ELSE in src/voice/ (a claim without the implementation is
//       the №512 dishonesty class); the old names
//       encrypt_placeholder / decrypt_placeholder stay gone tree-wide;
//       insecure_placeholder stays present (the mock-only path).
// The process-global mock flag is pinned under a static lock and always
// restored (the №251 global-state discipline).

// №475 fs_gate ratchet: the disallowed-methods lint targets PRODUCTION
// I/O paths. T3 exercises the REAL filesystem (reading src/voice/*.rs to
// pin the source honesty) by design — the allow is scoped to this file,
// same posture as tests/architecture_contract.rs.
#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};
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

/// Recursively collect the files under `root` (std-only walk).
fn walk_files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_files(&path, out);
        } else {
            out.push(path);
        }
    }
}

// ── T1: real runtime refuses loudly, nothing persisted ──────────────────

#[test]
fn n512_real_mode_save_refuses_loud_and_persists_nothing() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(false);

    let store = test_store();
    // №517: the keyless real-runtime save stays fail-closed.
    let err = store
        .save_voiceprint("alice", &[0.1, 0.2, 0.3], "chatterbox-v3", None)
        .unwrap_err();

    assert!(
        err.starts_with("[VOICE_INSECURE_STORE] "),
        "the refusal must carry the stable class stamp at position 0, got: {}",
        err
    );
    assert!(
        err.contains("no key provided"),
        "the refusal must name the missing key (the honest crypto EXISTS since №517 — the save without it is the refusal), got: {}",
        err
    );
    assert!(
        err.contains("AES-256-GCM"),
        "the refusal must name the honest crypto the store would use, got: {}",
        err
    );
    assert!(
        err.contains("secret()"),
        "the refusal must point at the key gate, got: {}",
        err
    );

    // Nothing was persisted: the load misses, loudly.
    let load = store.load_voiceprint("alice", None);
    assert!(
        load.is_err(),
        "a refused save must not leave a loadable row, got: {:?}",
        load
    );

    pin_mock(false);
}

// ── T2: mock runtime keeps the placeholder path (roundtrip) ─────────────

#[test]
fn n512_mock_mode_roundtrip_unchanged() {
    let _guard = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pin_mock(true);

    let store = test_store();
    let embedding = vec![0.25f32, -0.5, 1.0, 0.0, 3.75];
    store
        .save_voiceprint("bob", &embedding, "koko-ro-82m", None)
        .expect("mock mode keeps the insecure placeholder path (№512 boundary)");
    let (loaded, model) = store.load_voiceprint("bob", None).unwrap();
    assert_eq!(embedding, loaded, "roundtrip parity must hold in mock mode");
    assert_eq!(model, "koko-ro-82m");

    pin_mock(false);
}

// ── T3: source honesty pins ─────────────────────────────────────────────

#[test]
fn n512_voice_sources_carry_no_fake_crypto_claims() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let voice_dir = manifest.join("src/voice");

    let mut files = Vec::new();
    walk_files(&voice_dir, &mut files);
    assert!(
        files.len() >= 4,
        "the voice module must be present for the source pins"
    );

    for file in &files {
        let text = std::fs::read_to_string(file).unwrap_or_default();
        let is_store = file.file_name().map(|n| n == "store.rs").unwrap_or(false);
        if is_store {
            // №517 FLIP: store.rs MUST carry the crypto name — the claim
            // is now TRUE (the real AES-256-GCM implementation lives here).
            assert!(
                text.contains("AES-256-GCM"),
                "store.rs must carry the REAL AES-256-GCM implementation name (№517)"
            );
        } else {
            assert!(
                !text.contains("AES-256-GCM"),
                "{} must not claim AES-256-GCM outside the implementation module \
                 (a claim without the implementation is the №512 dishonesty class)",
                file.display()
            );
        }
        assert!(
            !text.contains("encrypt_placeholder"),
            "{} must use the honest name insecure_placeholder (№512)",
            file.display()
        );
    }

    let store_src = std::fs::read_to_string(manifest.join("src/voice/store.rs")).unwrap();
    assert!(
        !store_src.contains("decrypt_placeholder"),
        "store.rs must not carry the decrypt_placeholder name (the same \
         fake-crypto class; №512)"
    );
    assert!(
        store_src.contains("insecure_placeholder"),
        "store.rs must carry the honest insecure_placeholder name (№512)"
    );
    // №512's "no at-rest claim without crypto" pin EVOLVED with №517: the
    // claim is now allowed (it is true); the placeholder is marked
    // insecure IN THE SCHEMA (the algo column), never faked as crypto.
    assert!(
        store_src.contains("AES-256-GCM"),
        "store.rs must implement AES-256-GCM (№517)"
    );
    assert!(
        store_src.contains("INSECURE-XOR-MOCK"),
        "store.rs must carry the visible insecure schema mark for the mock path (№517)"
    );

    // The rename is tree-wide: no other module references the old names.
    let mut src_files = Vec::new();
    walk_files(&manifest.join("src"), &mut src_files);
    for file in &src_files {
        let text = std::fs::read_to_string(file).unwrap_or_default();
        assert!(
            !text.contains("encrypt_placeholder") && !text.contains("decrypt_placeholder"),
            "{} must not reference the old placeholder names (№512 tree-wide \
             rename)",
            file.display()
        );
    }
}

#[test]
fn n512_class_constant_is_stable_and_wired() {
    assert_eq!(
        metalogos::interpreter::values::CODE_VOICE_INSECURE_STORE,
        "VOICE_INSECURE_STORE",
        "the class name is the agent-facing contract (№413 convention)"
    );
    // The store refusal goes through coded_error — "[CODE] " + message.
    let stamped = metalogos::interpreter::values::coded_error(
        metalogos::interpreter::values::CODE_VOICE_INSECURE_STORE,
        "probe",
    );
    assert_eq!(stamped, "[VOICE_INSECURE_STORE] probe");
}
