//! Vision artifact SQLite persistence (Наряд №242, R6.1).
//!
//! The first third of R6 "Edit + LoRA" (plan §7.1): `vision_save` /
//! `vision_load` — program-database persistence of [`VisionArtifact`]s,
//! replacing the R1 loud stubs (`src/builtins/vision.rs:723/733`).
//!
//! ## Contract (naryad №242, Blocks 1.1–1.5)
//!
//! - Table `vision_artifacts`: `name TEXT PRIMARY KEY, png_bytes BLOB
//!   NOT NULL, manifest_json TEXT, saved_at TEXT NOT NULL` (RFC 3339
//!   UTC). Table creation follows the `init_kv_persist` лекало
//!   (`src/builtins/memory.rs:43` — `CREATE TABLE IF NOT EXISTS`); WAL
//!   is NOT touched here — the db layer owns journal mode
//!   (`src/interpreter/db.rs:131`).
//! - **Verbatim manifest roundtrip**: `Some(m)` → `manifest_sidecar_json`
//!   (the exact sidecar JSON the signed `vision_export` ships) → read →
//!   `Some(m')` with `m' == m` byte-for-byte across all fields —
//!   persistence NEVER regenerates provenance (the `timestamp` of the
//!   original generation survives; regenerating it would forge the
//!   provenance chain). `None` → `NULL` → `None`.
//! - **Broken manifest JSON = loud Err** — silently degrading to `None`
//!   would be a quiet loss of provenance (the artifact would still pass
//!   `vision_export_raw` unsigned — exactly what the loud discipline
//!   forbids).
//! - **Empty name = loud Err; name collision = loud Err.** A silent
//!   overwrite (upsert) would quietly destroy the provenance chain of
//!   the stored artifact; upsert/delete semantics are NOT part of №242 —
//!   if they turn out to be needed, that is a loud deviation with its
//!   own justification, not a silent replacement of the `Err`.
//! - PNG bytes travel ONLY as a BLOB inside the program's database —
//!   nothing is written to disk here (disk is export territory:
//!   `vision_export` / `vision_export_raw`).
//!
//! Artifacts live in the program's own database (`db { url: ... }` →
//! `db_conn`/`db_url`), NOT in the global KV store (the `KV_SQLITE`
//! singleton is the KV-pillar's persistence, not the vision pillar's).

use crate::vision::provenance::VisionManifest;
use crate::vision::VisionArtifact;

/// Schema of the vision-artifact store (naryad №242 Block 1.1).
/// `manifest_json` is the serialized provenance manifest — `NULL` exactly
/// when the artifact carries `manifest: None` (verbatim roundtrip).
const VISION_ARTIFACTS_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS vision_artifacts (\
name TEXT PRIMARY KEY, png_bytes BLOB NOT NULL, manifest_json TEXT, saved_at TEXT NOT NULL);";

/// Idempotently ensure the `vision_artifacts` table exists (лекало
/// `init_kv_persist`, `memory.rs:43`). Called by every entry point so a
/// fresh database behaves identically on save and on load — a missing
/// table can never masquerade as a SQL error (or, worse, as a silent
/// "not found" of a different kind).
fn ensure_table(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute_batch(VISION_ARTIFACTS_TABLE_SQL)
        .map_err(|e| format!("vision store: cannot create vision_artifacts table: {}", e))
}

/// Persist a vision artifact under `name` (Наряд №242 Block 1.2/1.4).
///
/// - Empty `name` → loud Err (there is no honest default name).
/// - Name collision → loud Err: `INSERT` (plain, no upsert) violates the
///   PRIMARY KEY and the violation is surfaced with the offending name —
///   a silent overwrite would destroy the stored artifact's provenance
///   chain (Block 1.4).
/// - `manifest: Some(m)` → `manifest_json` = `manifest_sidecar_json(m)`
///   (the same pretty JSON the signed export writes, Block 1.3);
///   `manifest: None` → `NULL`.
/// - PNG bytes go in as a single BLOB — no disk writes (Block 1.5).
pub fn save(
    conn: &rusqlite::Connection,
    name: &str,
    artifact: &VisionArtifact,
) -> Result<(), String> {
    if name.is_empty() {
        return Err(
            "vision_save: artifact name is empty — a non-empty name is required \
             (an unnamed artifact cannot be addressed by vision_load)"
                .to_string(),
        );
    }
    ensure_table(conn)?;
    let manifest_json = match &artifact.manifest {
        Some(m) => Some(crate::vision::provenance::manifest_sidecar_json(m)?),
        None => None,
    };
    // RFC 3339 UTC wall-clock (same form as the manifest timestamp,
    // provenance.rs — provenance records its generation time, this row
    // records its persistence time; neither is regenerated on load).
    let saved_at = chrono::Utc::now().to_rfc3339();
    let res = conn.execute(
        "INSERT INTO vision_artifacts (name, png_bytes, manifest_json, saved_at) \
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![name, artifact.png_bytes, manifest_json, saved_at],
    );
    if let Err(e) = res {
        // Name collision → loud, naming the name and the reason upsert is
        // absent (Block 1.4: quiet overwrite = quiet provenance loss).
        if let rusqlite::Error::SqliteFailure(ffi, _) = &e {
            if ffi.code == rusqlite::ErrorCode::ConstraintViolation {
                return Err(format!(
                    "vision_save: name '{}' already exists in vision_artifacts — \
                     refusing to overwrite (a silent upsert would destroy the \
                     stored artifact's provenance chain; upsert/delete semantics \
                     are not part of naryad №242 and must be added loudly, not \
                     silently)",
                    name
                ));
            }
        }
        return Err(format!(
            "vision_save: sqlite insert for '{}' failed: {}",
            name, e
        ));
    }
    Ok(())
}

/// Load a vision artifact by `name` (Наряд №242 Block 1.2/1.3).
///
/// - `Ok(None)` — no artifact under this name (the caller — the
///   `vision_load` dispatch — turns this into a loud user-facing error
///   with the list of saved names).
/// - `Ok(Some(artifact))` — EXACTLY the bytes/manifest that were saved:
///   the manifest JSON is deserialized back, never regenerated (the
///   original `timestamp` survives persistence verbatim).
/// - Broken manifest JSON → loud Err (Block 1.3: silently returning
///   `None`-manifest here would quietly strip provenance while the bytes
///   keep flowing — the forbidden degradation).
pub fn load(conn: &rusqlite::Connection, name: &str) -> Result<Option<VisionArtifact>, String> {
    if name.is_empty() {
        return Err(
            "vision_load: artifact name is empty — a non-empty name is required".to_string(),
        );
    }
    ensure_table(conn)?;
    let row = conn.query_row(
        "SELECT png_bytes, manifest_json FROM vision_artifacts WHERE name = ?1",
        rusqlite::params![name],
        |row| {
            let png_bytes: Vec<u8> = row.get("png_bytes")?;
            let manifest_json: Option<String> = row.get("manifest_json")?;
            Ok((png_bytes, manifest_json))
        },
    );
    let (png_bytes, manifest_json) = match row {
        Ok(pair) => pair,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
        Err(e) => {
            return Err(format!(
                "vision_load: sqlite query for '{}' failed: {}",
                name, e
            ))
        }
    };
    let manifest = match manifest_json {
        // NULL ⇔ None — the unsigned artifact stays unsigned, verbatim.
        None => None,
        Some(json) => Some(serde_json::from_str::<VisionManifest>(&json).map_err(|e| {
            format!(
                "vision_load: manifest JSON for '{}' is corrupted ({}) — \
                     refusing to degrade it to an unsigned artifact; provenance \
                     must not be quietly lost (naryad №242 Block 1.3)",
                name, e
            )
        })?),
    };
    Ok(Some(VisionArtifact {
        png_bytes,
        manifest,
    }))
}

/// List saved artifact names, sorted (Наряд №242 Block 1.2 — for tests
/// and loud diagnostics; there is deliberately NO `vision_list`-style
/// builtin over the DB: the DB persists beyond the session, and a
/// session-scoped builtin listing a cross-session store would blur the
/// boundary the registry/DB split exists to keep).
pub fn list(conn: &rusqlite::Connection) -> Result<Vec<String>, String> {
    ensure_table(conn)?;
    let mut stmt = conn
        .prepare("SELECT name FROM vision_artifacts ORDER BY name")
        .map_err(|e| format!("vision store: sqlite list failed: {}", e))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("vision store: sqlite list failed: {}", e))?;
    let mut names = Vec::new();
    for r in rows {
        names.push(r.map_err(|e| format!("vision store: sqlite list failed: {}", e))?);
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_conn() -> rusqlite::Connection {
        rusqlite::Connection::open_in_memory().expect("open in-memory sqlite")
    }

    fn signed_artifact(png: Vec<u8>) -> VisionArtifact {
        VisionArtifact {
            png_bytes: png,
            manifest: Some(VisionManifest {
                model_id: "z-image-turbo".to_string(),
                model_sha256: "deadbeef".to_string(),
                seed: 42,
                prompt_sha256: "cafebabe".to_string(),
                policy: "safe".to_string(),
                timestamp: "2026-09-09T00:00:00+00:00".to_string(),
                png_sha256: crate::vision::provenance::sha256_hex(&[1, 2, 3, 4]),
            }),
        }
    }

    /// Verbatim roundtrip: `Some(m)` → JSON → `Some(m')`, `m' == m`
    /// field-for-field including `timestamp` (Block 1.3), PNG BLOB exact.
    #[test]
    fn save_load_roundtrip_signed_verbatim() {
        let conn = mem_conn();
        let png = vec![0x89, b'P', b'N', b'G', 1, 2, 3, 4, 250];
        let artifact = signed_artifact(png.clone());
        save(&conn, "poster", &artifact).expect("save");
        let loaded = load(&conn, "poster")
            .expect("load")
            .expect("row must exist");
        assert_eq!(loaded.png_bytes, png, "PNG BLOB must survive verbatim");
        assert_eq!(
            loaded.manifest.as_ref(),
            artifact.manifest.as_ref(),
            "manifest must roundtrip byte-for-byte (incl. timestamp)"
        );
    }

    /// `None` → `NULL` → `None` (verbatim in the unsigned direction too).
    #[test]
    fn manifest_none_roundtrips_via_null() {
        let conn = mem_conn();
        let artifact = VisionArtifact {
            png_bytes: vec![9, 9, 9],
            manifest: None,
        };
        save(&conn, "raw", &artifact).expect("save");
        let loaded = load(&conn, "raw").expect("load").expect("row");
        assert!(loaded.manifest.is_none(), "NULL must load back as None");
        assert_eq!(loaded.png_bytes, vec![9, 9, 9]);
    }

    /// Empty name = loud Err (Block 1.4).
    #[test]
    fn empty_name_is_loud_error() {
        let conn = mem_conn();
        let err = save(&conn, "", &signed_artifact(vec![1]))
            .expect_err("empty name must be a loud error");
        assert!(err.contains("empty"), "err must name the problem: {}", err);
        let err = load(&conn, "").expect_err("empty name must be a loud error");
        assert!(err.contains("empty"), "err must name the problem: {}", err);
    }

    /// Name collision = loud Err; the first artifact is untouched
    /// (no silent upsert, Block 1.4).
    #[test]
    fn name_collision_is_loud_error_and_first_survives() {
        let conn = mem_conn();
        save(&conn, "poster", &signed_artifact(vec![1, 1])).expect("first save");
        let err = save(&conn, "poster", &signed_artifact(vec![2, 2]))
            .expect_err("collision must be a loud error");
        assert!(
            err.contains("already exists"),
            "err must name the collision: {}",
            err
        );
        let loaded = load(&conn, "poster").expect("load").expect("row");
        assert_eq!(
            loaded.png_bytes,
            vec![1, 1],
            "first artifact must survive untouched"
        );
    }

    /// Broken manifest JSON = loud Err, NOT a silent `None` (Block 1.3:
    /// quiet degradation would strip provenance while bytes keep flowing).
    #[test]
    fn corrupted_manifest_json_is_loud_error() {
        let conn = mem_conn();
        save(&conn, "broken", &signed_artifact(vec![5, 5])).expect("save");
        conn.execute(
            "UPDATE vision_artifacts SET manifest_json = ?1 WHERE name = ?2",
            rusqlite::params!["{ not json", "broken"],
        )
        .expect("corrupt the manifest_json column");
        let err = load(&conn, "broken").expect_err("corrupted manifest must be a loud error");
        assert!(
            err.contains("corrupted"),
            "err must name the corruption: {}",
            err
        );
    }

    /// `list` returns sorted names for loud diagnostics (Block 1.2).
    #[test]
    fn list_is_sorted_and_complete() {
        let conn = mem_conn();
        assert!(list(&conn).expect("list empty").is_empty());
        save(&conn, "zeta", &signed_artifact(vec![1])).expect("save zeta");
        save(&conn, "alpha", &signed_artifact(vec![2])).expect("save alpha");
        save(&conn, "mid", &signed_artifact(vec![3])).expect("save mid");
        assert_eq!(
            list(&conn).expect("list"),
            vec!["alpha".to_string(), "mid".to_string(), "zeta".to_string()]
        );
    }

    /// PNG bytes live only as a BLOB in the program DB — no files appear
    /// on disk anywhere (Block 1.5; the store API takes a connection and
    /// nothing else, so there is no path to write to).
    #[test]
    fn png_is_stored_as_blob_only() {
        let conn = mem_conn();
        let png = vec![7u8; 4096];
        save(&conn, "big", &signed_artifact(png.clone())).expect("save");
        let stored: Vec<u8> = conn
            .query_row(
                "SELECT png_bytes FROM vision_artifacts WHERE name = 'big'",
                [],
                |row| row.get(0),
            )
            .expect("query blob");
        assert_eq!(stored, png, "BLOB must carry the exact bytes");
    }

    /// `saved_at` is present and RFC 3339-parseable (Block 1.1).
    #[test]
    fn saved_at_is_rfc3339() {
        let conn = mem_conn();
        save(&conn, "stamped", &signed_artifact(vec![1])).expect("save");
        let saved_at: String = conn
            .query_row(
                "SELECT saved_at FROM vision_artifacts WHERE name = 'stamped'",
                [],
                |row| row.get(0),
            )
            .expect("query saved_at");
        assert!(
            chrono::DateTime::parse_from_rfc3339(&saved_at).is_ok(),
            "saved_at must be RFC 3339, got: {}",
            saved_at
        );
    }
}
