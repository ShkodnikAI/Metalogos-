// ── Наряд №180: Contract 2 — REFLEX_VERSION mismatch → explicit error ──
//
// ADR-0116: a stale format is a loud failure, never a silent
// misinterpretation of bytes.
//
// We save a model, then manually corrupt the REFLEX_VERSION field in the
// stored bytes (write 999 instead of REFLEX_VERSION), then verify that
// reflex_load returns an explicit "unsupported model version" error
// instead of silently misinterpreting the data.

use metalogos::nn::{
    persist,
    serde_weights::{deserialize_model, REFLEX_MAGIC, REFLEX_VERSION},
    ActivationKind, Dense, ReflexModel, ReflexRegistry,
};
use std::collections::HashMap;
use tempfile::TempDir;

fn make_model(seed: u64) -> ReflexModel {
    ReflexModel {
        name: "version_test".to_string(),
        layers: vec![
            Box::new(Dense::new(2, 4, ActivationKind::Relu, seed)),
            Box::new(Dense::new(4, 2, ActivationKind::Softmax, seed + 1)),
        ],
        seed,
        last_metric: None,
        input_size: 2,
        labels: vec!["a".to_string(), "b".to_string()],
    }
}

#[test]
fn version_mismatch_is_explicit_error() {
    let temp = TempDir::new().expect("temp dir");
    let db_path = temp.path().join("reflex_version.db");

    let mut registry = ReflexRegistry::new();
    let id = registry.register(make_model(42));
    let mut name_to_id = HashMap::new();
    name_to_id.insert("version_test".to_string(), id);

    // Save the model.
    {
        let model = registry.get(id).expect("model");
        persist::save_model_to_db(model, "version_test", &db_path).expect("save");
    }

    // Manually corrupt the REFLEX_VERSION field in the stored blob.
    // Read the blob back from SQLite, modify bytes [4..8] (the version
    // u32 LE), write it back.
    let conn = rusqlite::Connection::open(&db_path).expect("open db");
    let blob: Vec<u8> = conn
        .query_row(
            "SELECT weights FROM reflex_models WHERE name = ?1",
            rusqlite::params!["version_test"],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .expect("query");

    assert_eq!(
        &blob[0..4],
        REFLEX_MAGIC,
        "magic bytes intact before corruption"
    );
    assert_eq!(
        u32::from_le_bytes(blob[4..8].try_into().unwrap()),
        REFLEX_VERSION,
        "version matches current before corruption"
    );

    // Write a fake future version (999).
    let mut corrupted = blob.clone();
    corrupted[4..8].copy_from_slice(&999u32.to_le_bytes());
    conn.execute(
        "UPDATE reflex_models SET weights = ?1 WHERE name = ?2",
        rusqlite::params![&corrupted, "version_test"],
    )
    .expect("update");

    // Try to load — should fail with explicit version error.
    let result = persist::load_model_from_db(&mut registry, id, "version_test", &db_path);

    assert!(
        result.is_err(),
        "load should fail on version mismatch, got: {:?}",
        result
    );

    let err_msg = result.unwrap_err().to_lowercase();
    assert!(
        err_msg.contains("version"),
        "error should mention version, got: {}",
        err_msg
    );

    println!("Version mismatch error: {}", err_msg);
}

#[test]
fn corrupted_magic_bytes_is_explicit_error() {
    // Also verify that corrupted magic bytes produce a loud error
    // (not a silent attempt to parse the wrong format).
    let temp = TempDir::new().expect("temp dir");
    let db_path = temp.path().join("reflex_magic.db");

    let mut registry = ReflexRegistry::new();
    let id = registry.register(make_model(7));
    {
        let model = registry.get(id).expect("model");
        persist::save_model_to_db(model, "version_test", &db_path).expect("save");
    }

    // Corrupt the magic bytes.
    let conn = rusqlite::Connection::open(&db_path).expect("open db");
    let blob: Vec<u8> = conn
        .query_row(
            "SELECT weights FROM reflex_models WHERE name = ?1",
            rusqlite::params!["version_test"],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .expect("query");

    let mut corrupted = blob.clone();
    corrupted[0..4].copy_from_slice(b"GARB");
    conn.execute(
        "UPDATE reflex_models SET weights = ?1 WHERE name = ?2",
        rusqlite::params![&corrupted, "version_test"],
    )
    .expect("update");

    let result = persist::load_model_from_db(&mut registry, id, "version_test", &db_path);
    assert!(result.is_err(), "load should fail on bad magic");
    let err_msg = result.unwrap_err().to_lowercase();
    assert!(
        err_msg.contains("magic") || err_msg.contains("invalid"),
        "error should mention invalid magic, got: {}",
        err_msg
    );

    println!("Magic mismatch error: {}", err_msg);
}

#[test]
fn deserialize_model_directly_rejects_version() {
    // Unit test the underlying serde_weights::deserialize_model.
    // Construct a valid blob, then flip the version byte.
    let layer_blobs: Vec<Vec<u8>> = vec![vec![0u8; 9 + 16 + 2]]; // dummy 2→2 layer
    let valid = metalogos::nn::serde_weights::serialize_model(&layer_blobs);
    assert!(deserialize_model(&valid).is_ok(), "valid blob should parse");

    let mut bad_version = valid.clone();
    bad_version[4..8].copy_from_slice(&999u32.to_le_bytes());
    let result = deserialize_model(&bad_version);
    assert!(result.is_err(), "version mismatch should error");
    assert!(
        result.unwrap_err().to_lowercase().contains("version"),
        "error should mention version"
    );
}
