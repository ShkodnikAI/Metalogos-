//! Наряд №242 (Vision R6.1): SQLite-персистенция артефактов —
//! roundtrip-контракт + негативы (плановая приёмка R6 «roundtrip-тест»).
//!
//! Coverage (naryad §Block 3):
//! - главный контракт (Block 3.1): dispatch-уровень, БД `sqlite::memory:`:
//!   рег A с подписанным артефактом → `vision_save` → НОВЫЙ пустой рег B →
//!   `vision_load` → `vision_export` в tempdir → PNG байт-в-байт = исходным,
//!   sidecar `<path>.manifest.json` байт-в-байт = исходному (дословная
//!   персистенция provenance, включая `timestamp`);
//! - негативы (Block 3.2, все — громкие): unknown handle, unknown name
//!   (с loud-диагностикой списка сохранённого), пустое имя, коллизия имени,
//!   no-db (громкий Err с подсказкой `db { url: ... }`), manifest-None
//!   roundtrip (signed `vision_export` отказывается с
//!   `VISION_UNSIGNED_EXPORT`, `vision_export_raw` работает и не пишет
//!   sidecar — backstop №241 жив после персистенции);
//! - битый manifest-JSON в БД = громкий Err через dispatch (не тихий None,
//!   Block 1.3).
//!
//! NO network. NO weights. NO #[serial] (env не трогается, Block 3.3) —
//! каждый тест держит СВОЙ `sqlite::memory:` (изоляция без файлов);
//! tempdir — только для export-пути.

use metalogos::builtins::{
    vision_export_dispatch, vision_export_raw_dispatch, vision_load_dispatch, vision_save_dispatch,
};
use metalogos::interpreter::Value;
use metalogos::vision::provenance::{manifest_sidecar_json, sha256_hex, VisionManifest};
use metalogos::vision::{VisionArtifact, VisionId, VisionRegistry};

// ── Helpers ──────────────────────────────────────────────────────────

/// Synthetic PNG-ish bytes — персистенция не обязана уметь их декодировать
/// (store ходит байтами, PNG-кодек — территория generate/export).
fn png_fixture() -> Vec<u8> {
    let mut v = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    v.extend(0u32.to_be_bytes());
    v.extend((0..=255).cycle().take(1024));
    v
}

/// Signed manifest with a FIXED timestamp — verbatim persistence means
/// the original generation timestamp survives save→load byte-for-byte.
fn signed_manifest(png: &[u8]) -> VisionManifest {
    VisionManifest {
        model_id: "z-image-turbo".to_string(),
        model_sha256: "deadbeefdeadbeef".to_string(),
        seed: 42,
        prompt_sha256: "cafebabetest".to_string(),
        policy: "safe".to_string(),
        timestamp: "2026-09-09T12:34:56.789+00:00".to_string(),
        png_sha256: sha256_hex(png),
    }
}

fn signed_artifact(png: Vec<u8>) -> VisionArtifact {
    VisionArtifact {
        png_bytes: png.clone(),
        manifest: Some(signed_manifest(&png)),
    }
}

fn mem_conn() -> rusqlite::Connection {
    rusqlite::Connection::open_in_memory().expect("open in-memory sqlite")
}

fn vision_handle(v: &Value) -> metalogos::vision::VisionId {
    match v {
        Value::Vision(id) => *id,
        other => panic!("expected a Vision handle, got {:?}", other),
    }
}

// ── Block 3.1: главный roundtrip-контракт ────────────────────────────

/// Рег A (подписанный артефакт) → save → НОВЫЙ пустой рег B → load →
/// signed export в tempdir: PNG байт-в-байт, sidecar байт-в-байт,
/// манифест (включая timestamp) не перегенерируется.
#[test]
fn roundtrip_signed_artifact_survives_byte_for_byte() {
    let conn = mem_conn();
    let png = png_fixture();
    let artifact = signed_artifact(png.clone());

    let mut reg_a = VisionRegistry::new();
    let id_a = reg_a.insert(artifact.clone());

    let saved = vision_save_dispatch(
        &reg_a,
        Some(&conn),
        &[Value::Vision(id_a), Value::String("poster".to_string())],
    )
    .expect("save must succeed");
    assert!(
        matches!(&saved, Value::String(s) if s == "poster"),
        "vision_save returns the persistent key, got {:?}",
        saved
    );

    // НОВЫЙ пустой рег — тот же контур, что и новый процесс/сессия.
    let mut reg_b = VisionRegistry::new();
    let loaded = vision_load_dispatch(
        &mut reg_b,
        Some(&conn),
        &[Value::String("poster".to_string())],
    )
    .expect("load must succeed");
    let id_b = vision_handle(&loaded);
    assert_eq!(
        id_b.0, 0,
        "fresh registry assigns a fresh monotonic id — id is a session handle, \
         not the persisted key (naryad №242 prerequisites)"
    );

    let roundtripped = reg_b.get(id_b).expect("artifact present in reg B");
    assert_eq!(
        roundtripped, &artifact,
        "artifact must roundtrip verbatim — bytes AND manifest field-for-field \
         (persistence never regenerates provenance)"
    );
    assert_eq!(
        roundtripped.manifest.as_ref().expect("signed").timestamp,
        "2026-09-09T12:34:56.789+00:00",
        "the original generation timestamp survives persistence"
    );

    // Signed export из загруженного артефакта — и сверка байт-в-байт.
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join("out.png");
    vision_export_dispatch(
        &reg_b,
        &[
            Value::Vision(id_b),
            Value::String(out.to_string_lossy().into_owned()),
        ],
    )
    .expect("signed export of the loaded artifact must work");
    let written = std::fs::read(&out).expect("exported PNG");
    assert_eq!(
        written, png,
        "PNG must survive save→load→export byte-for-byte"
    );
    let sidecar_path = dir.path().join("out.png.manifest.json");
    let sidecar = std::fs::read(&sidecar_path).expect("sidecar written by export");
    let expected_sidecar =
        manifest_sidecar_json(roundtripped.manifest.as_ref().unwrap()).expect("sidecar json");
    assert_eq!(
        sidecar,
        expected_sidecar.into_bytes(),
        "sidecar must be byte-for-byte the original provenance JSON"
    );
}

// ── Block 3.2: негативы (все — громкие) ──────────────────────────────

/// Unknown handle → громкий Err с `[Vision#N]` (лекало export).
#[test]
fn save_unknown_handle_is_loud() {
    let conn = mem_conn();
    let reg = VisionRegistry::new();
    let err = vision_save_dispatch(
        &reg,
        Some(&conn),
        &[
            Value::Vision(VisionId(999)),
            Value::String("poster".to_string()),
        ],
    )
    .expect_err("unknown handle must be a loud error");
    assert!(
        err.contains("[Vision#999]"),
        "error must name the handle: {}",
        err
    );
}

/// Unknown name → громкий Err; до первого save — «nothing saved», после —
/// список сохранённых имён (loud-диагностика, Block 1.2).
#[test]
fn load_unknown_name_is_loud_with_saved_diagnostics() {
    let conn = mem_conn();
    let mut reg = VisionRegistry::new();
    let err = vision_load_dispatch(&mut reg, Some(&conn), &[Value::String("nope".to_string())])
        .expect_err("unknown name must be a loud error");
    assert!(
        err.contains("no artifact named 'nope'"),
        "error must name the requested key: {}",
        err
    );
    assert!(
        err.contains("nothing saved"),
        "empty store must be stated loudly, not implied: {}",
        err
    );

    // После save диагностика перечисляет сохранённое.
    let id = reg.insert(signed_artifact(png_fixture()));
    vision_save_dispatch(
        &reg,
        Some(&conn),
        &[Value::Vision(id), Value::String("known".to_string())],
    )
    .expect("save 'known'");
    let err = vision_load_dispatch(&mut reg, Some(&conn), &[Value::String("nope".to_string())])
        .expect_err("unknown name must stay loud");
    assert!(
        err.contains("\"known\""),
        "error must list the saved names as diagnostics: {}",
        err
    );
}

/// Пустое имя = громкий Err (Block 1.4).
#[test]
fn save_empty_name_is_loud() {
    let conn = mem_conn();
    let mut reg = VisionRegistry::new();
    let id = reg.insert(signed_artifact(png_fixture()));
    let err = vision_save_dispatch(
        &reg,
        Some(&conn),
        &[Value::Vision(id), Value::String(String::new())],
    )
    .expect_err("empty name must be a loud error");
    assert!(
        err.contains("empty"),
        "error must name the problem: {}",
        err
    );
}

/// Коллизия имени = громкий Err (второй save с тем же именем), первый
/// артефакт не тронут — тихой перезаписи нет (Block 1.4).
#[test]
fn save_name_collision_is_loud_and_first_survives() {
    let conn = mem_conn();
    let mut reg = VisionRegistry::new();
    let png_first = vec![1, 1, 1];
    let id_first = reg.insert(signed_artifact(png_first.clone()));
    vision_save_dispatch(
        &reg,
        Some(&conn),
        &[Value::Vision(id_first), Value::String("poster".to_string())],
    )
    .expect("first save");

    let id_second = reg.insert(signed_artifact(vec![2, 2, 2]));
    let err = vision_save_dispatch(
        &reg,
        Some(&conn),
        &[
            Value::Vision(id_second),
            Value::String("poster".to_string()),
        ],
    )
    .expect_err("collision must be a loud error");
    assert!(
        err.contains("already exists"),
        "error must name the collision: {}",
        err
    );
    assert!(
        err.contains("provenance"),
        "error must state WHY upsert is absent (quiet overwrite = quiet \
         provenance loss): {}",
        err
    );

    // Первый артефакт уцелел байт-в-байт.
    let mut reg_fresh = VisionRegistry::new();
    let loaded = vision_load_dispatch(
        &mut reg_fresh,
        Some(&conn),
        &[Value::String("poster".to_string())],
    )
    .expect("load after refused collision");
    let id = vision_handle(&loaded);
    assert_eq!(
        reg_fresh.get(id).expect("present").png_bytes,
        png_first,
        "the FIRST artifact must survive a refused collision"
    );
}

/// No-db (db_conn: None) → громкий Err, называющий отсутствие БД и как
/// включить (декларация `db { url: ... }`) — для save и load (Block 2.1).
#[test]
fn no_db_is_loud_with_declaration_hint() {
    let mut reg = VisionRegistry::new();
    let id = reg.insert(signed_artifact(png_fixture()));
    let err = vision_save_dispatch(
        &reg,
        None,
        &[Value::Vision(id), Value::String("poster".to_string())],
    )
    .expect_err("no-db save must be a loud error");
    assert!(
        err.contains("no database connection"),
        "error must name the missing component: {}",
        err
    );
    assert!(
        err.contains("db { url:"),
        "error must name HOW to enable (лекало env-отказов №240): {}",
        err
    );

    let err = vision_load_dispatch(
        &mut VisionRegistry::new(),
        None,
        &[Value::String("poster".to_string())],
    )
    .expect_err("no-db load must be a loud error");
    assert!(
        err.contains("no database connection") && err.contains("db { url:"),
        "load must refuse equally loudly with the hint: {}",
        err
    );
}

/// manifest-None roundtrip (Block 3.2, последний негатив): unsigned
/// артефакт персистится как есть (save/load — не подпись), после load
/// signed `vision_export` отказывается с `VISION_UNSIGNED_EXPORT`
/// (backstop №241 жив после персистенции), `vision_export_raw` работает
/// и не пишет sidecar.
#[test]
fn manifest_none_roundtrip_backstop_stays_alive() {
    let conn = mem_conn();
    let png = png_fixture();
    let unsigned = VisionArtifact {
        png_bytes: png.clone(),
        manifest: None,
    };

    let mut reg_a = VisionRegistry::new();
    let id_a = reg_a.insert(unsigned.clone());
    vision_save_dispatch(
        &reg_a,
        Some(&conn),
        &[Value::Vision(id_a), Value::String("raw_art".to_string())],
    )
    .expect("saving an unsigned artifact is persistence, not signing");

    let mut reg_b = VisionRegistry::new();
    let loaded = vision_load_dispatch(
        &mut reg_b,
        Some(&conn),
        &[Value::String("raw_art".to_string())],
    )
    .expect("load");
    let id_b = vision_handle(&loaded);
    assert!(
        reg_b.get(id_b).expect("present").manifest.is_none(),
        "NULL must load back as None — persistence must not invent provenance"
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join("out.png");
    let err = vision_export_dispatch(
        &reg_b,
        &[
            Value::Vision(id_b),
            Value::String(out.to_string_lossy().into_owned()),
        ],
    )
    .expect_err("signed export must refuse an unsigned artifact after load");
    assert!(
        err.contains("VISION_UNSIGNED_EXPORT"),
        "backstop check-id must survive persistence: {}",
        err
    );
    assert!(
        !out.exists(),
        "the refused signed export must not have written the PNG"
    );

    let out_raw = dir.path().join("raw.png");
    vision_export_raw_dispatch(
        &reg_b,
        &[
            Value::Vision(id_b),
            Value::String(out_raw.to_string_lossy().into_owned()),
        ],
    )
    .expect("raw export works on the loaded unsigned artifact");
    assert_eq!(
        std::fs::read(&out_raw).expect("raw PNG"),
        png,
        "raw export ships exactly the persisted bytes"
    );
    assert!(
        !dir.path().join("raw.png.manifest.json").exists(),
        "raw export must NOT write a sidecar"
    );
}

/// Битый manifest-JSON в БД → громкий Err через dispatch (не тихий None —
/// тихая потеря provenance запрещена, Block 1.3).
#[test]
fn corrupted_manifest_json_is_loud_through_dispatch() {
    let conn = mem_conn();
    let mut reg = VisionRegistry::new();
    let id = reg.insert(signed_artifact(png_fixture()));
    vision_save_dispatch(
        &reg,
        Some(&conn),
        &[Value::Vision(id), Value::String("broken".to_string())],
    )
    .expect("save");

    conn.execute(
        "UPDATE vision_artifacts SET manifest_json = ?1 WHERE name = ?2",
        rusqlite::params!["{ not json", "broken"],
    )
    .expect("corrupt the manifest_json column directly in the DB");

    let mut reg_b = VisionRegistry::new();
    let err = vision_load_dispatch(
        &mut reg_b,
        Some(&conn),
        &[Value::String("broken".to_string())],
    )
    .expect_err("corrupted manifest must be a loud error");
    assert!(
        err.contains("corrupted"),
        "error must name the corruption: {}",
        err
    );
}
