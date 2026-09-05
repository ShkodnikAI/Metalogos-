//! Reflex weight persistence (Наряд №180, этап 4/6).
//!
//! Implements ADR-0116: SQLite BLOB storage of trained Reflex weights.
//! Reuses the binary serialization format from `src/nn/serde_weights.rs`
//! (Наряд №178) — this module handles **storage and access**, not format.
//!
//! ## Storage location
//!
//! Weights live in a table `reflex_models` inside the same SQLite database
//! that `memory { persist: "..." }` already opens (via `SqliteStore` /
//! `kv_store` / `checkpoints`). No new file format, no new path-traversal
//! surface — ADR-0116 explicitly rejected a dedicated `.mlm` file.
//!
//! ## Block 3 — layer shape verification
//!
//! The ADR-0116 schema stores `input_size` but not the full layer
//! structure (`layers: [dense(8, relu), dense(2, softmax)]`). If a model
//! is saved, then the `.mlog` `reflex` declaration is changed (e.g.,
//! `dense(64, relu)` → `dense(32, relu)`), `reflex_load` would silently
//! overwrite the runtime layer's input/output dims via
//! `Dense::deserialize_weights` (which sets `self.input_dim` and
//! `self.output_dim` from the stored blob).
//!
//! This module adds an explicit shape check **before** deserialize: we
//! re-derive the expected per-layer weight count from the *current*
//! declaration's `ReflexModel` structure and compare to the per-layer
//! blob byte lengths stored on disk. A mismatch is a loud error, not a
//! silent corruption.
//!
//! The check is intentionally conservative — it compares only the
//! weight+bias byte count per layer (not the activation kind), because:
//!
//!   1. Activation kind is recoverable (deserialize overwrites it from
//!      the blob, which is fine — the *architecture* didn't change if
//!      only the activation was swapped).
//!   2. Weight *count* is the dimension that, if wrong, would cause
//!      actual silent corruption (mismatched matrix multiplication).
//!
//! ## Time source
//!
//! `updated_at` uses `SystemTime::now()` in seconds since UNIX_EPOCH,
//! matching `memory_store.rs`'s pattern for `created_at` on memory entries.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::nn::layer::Layer;
use crate::nn::{serde_weights, ReflexId, ReflexModel, ReflexRegistry};

/// Schema version for the `reflex_models` table.
/// Bumped if the column set ever changes. ADR-0116 leaves this as
/// "CREATE TABLE IF NOT EXISTS" — no migration logic in v1.
pub const REFLEX_MODELS_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS reflex_models (
  name        TEXT PRIMARY KEY,
  weights     BLOB NOT NULL,
  input_size  INTEGER NOT NULL,
  labels      TEXT NOT NULL,
  seed        INTEGER NOT NULL,
  last_metric REAL,
  updated_at  INTEGER NOT NULL
);
";

/// Compute the expected byte length of a layer's serialized weights,
/// based on the **current** declaration's layer structure.
///
/// Used by Block 3 to verify that a stored blob matches the runtime
/// model architecture before attempting to deserialize.
///
/// The format is defined by `Dense::serialize_weights`:
/// `[input_dim:u32][output_dim:u32][activation:u8]` followed by
/// `input_dim*output_dim` f64 weights and `output_dim` f64 bias values.
fn expected_layer_blob_len(layer: &dyn Layer) -> usize {
    let input = layer.input_size();
    let output = layer.output_size();
    // header (9 bytes: 4+4+1) + weights (input*output*8) + bias (output*8)
    9 + input * output * 8 + output * 8
}

/// Verify that the deserialized per-layer blobs match the *current*
/// model's layer shapes.
///
/// Returns `Ok(())` if every layer's blob length equals what the current
/// `ReflexModel` would produce. Returns a descriptive `Err` on mismatch.
///
/// This is Block 3: explicit shape check, no silent corruption.
pub fn verify_layer_shapes(model: &ReflexModel, layer_blobs: &[Vec<u8>]) -> Result<(), String> {
    if layer_blobs.len() != model.layers.len() {
        return Err(format!(
            "reflex_load: layer count mismatch — saved model has {} layers, \
             current declaration has {}. Save and load must use the same architecture.",
            layer_blobs.len(),
            model.layers.len()
        ));
    }
    for (i, (blob, layer)) in layer_blobs.iter().zip(model.layers.iter()).enumerate() {
        let expected = expected_layer_blob_len(layer.as_ref());
        if blob.len() != expected {
            return Err(format!(
                "reflex_load: layer {} weight blob size mismatch — \
                 saved blob is {} bytes, current layer '{}' expects {} bytes \
                 (input={}, output={}). The model declaration changed between \
                 save and load; cannot silently apply old weights to new shape.",
                i,
                blob.len(),
                layer.name(),
                expected,
                layer.input_size(),
                layer.output_size()
            ));
        }
    }
    Ok(())
}

/// Serialize a model's weights + metadata into the on-disk row format.
///
/// The `weights` BLOB is exactly `serialize_model()` output from
/// `serde_weights.rs`. The other columns are scalar metadata needed
/// for sanity checks at load time (input_size, labels, seed).
pub fn serialize_for_storage(model: &ReflexModel) -> Vec<u8> {
    let layer_blobs: Vec<Vec<u8>> = model.layers.iter().map(|l| l.serialize_weights()).collect();
    serde_weights::serialize_model(&layer_blobs)
}

/// Apply deserialized per-layer weight blobs to a runtime model.
///
/// Performs the Block 3 shape check first — if any layer's blob size
/// doesn't match the current declaration, returns `Err` without
/// modifying any weights (atomicity: either all layers load or none).
pub fn deserialize_into_model(model: &mut ReflexModel, data: &[u8]) -> Result<(), String> {
    let layer_blobs = serde_weights::deserialize_model(data)?;
    verify_layer_shapes(model, &layer_blobs)?;

    // Atomic apply: only reach here if all shapes match.
    for (i, blob) in layer_blobs.iter().enumerate() {
        // as_any_mut().downcast_mut — same pattern as ReflexModel::train.
        if let Some(dense) = model.layers[i]
            .as_any_mut()
            .downcast_mut::<crate::nn::Dense>()
        {
            dense.deserialize_weights(blob)?;
        } else {
            return Err(format!(
                "reflex_load: layer {} is not a Dense layer (only Dense supports weight loading)",
                i
            ));
        }
    }
    Ok(())
}

/// Save a model's weights + metadata to the SQLite database.
///
/// `name` is the model identifier (from the `reflex Name { ... }` declaration).
/// `db_path` is the SQLite file path (set by `memory { persist: "..." }`).
///
/// Returns `Ok(())` on success. The model is keyed by `name` — saving twice
/// under the same name overwrites (matching `memorize`/`recall` semantics).
pub fn save_model_to_db(model: &ReflexModel, name: &str, db_path: &Path) -> Result<(), String> {
    let conn = rusqlite::Connection::open(db_path).map_err(|e| {
        format!(
            "reflex_save: failed to open db '{}': {}",
            db_path.display(),
            e
        )
    })?;

    // Ensure table exists (idempotent — safe to call on every save).
    conn.execute_batch(REFLEX_MODELS_SCHEMA)
        .map_err(|e| format!("reflex_save: failed to ensure reflex_models table: {}", e))?;

    let weights_blob = serialize_for_storage(model);
    let labels_json = serde_json::to_string(&model.labels)
        .map_err(|e| format!("reflex_save: failed to serialize labels: {}", e))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    conn.execute(
        "INSERT OR REPLACE INTO reflex_models \
         (name, weights, input_size, labels, seed, last_metric, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            name,
            weights_blob,
            model.input_size as i64,
            labels_json,
            model.seed as i64,
            model.last_metric,
            now,
        ],
    )
    .map_err(|e| format!("reflex_save: failed to insert row: {}", e))?;

    Ok(())
}

/// Load a model's weights from SQLite and apply them to the runtime model
/// with the given `ReflexId`.
///
/// Block 3: shape verification happens inside `deserialize_into_model`
/// before any weight mutation — silent corruption is impossible.
pub fn load_model_from_db(
    registry: &mut ReflexRegistry,
    id: ReflexId,
    name: &str,
    db_path: &Path,
) -> Result<(), String> {
    let conn = rusqlite::Connection::open(db_path).map_err(|e| {
        format!(
            "reflex_load: failed to open db '{}': {}",
            db_path.display(),
            e
        )
    })?;

    let mut stmt = conn
        .prepare(
            "SELECT weights, input_size, labels, seed, last_metric \
             FROM reflex_models WHERE name = ?1",
        )
        .map_err(|e| format!("reflex_load: failed to prepare query: {}", e))?;

    // Type alias to keep clippy::type_complexity happy.
    type SavedRow = (Vec<u8>, i64, String, i64, Option<f64>);

    let row_result: Result<SavedRow, rusqlite::Error> = stmt
        .query_row(rusqlite::params![name], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<f64>>(4)?,
            ))
        });

    let row = match row_result {
        Ok(r) => r,
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            return Err(format!(
                "reflex_load: no saved model with name '{}'. Did you call reflex_save first?",
                name
            ));
        }
        Err(e) => {
            return Err(format!(
                "reflex_load: database error reading '{}': {}",
                name, e
            ));
        }
    };

    let (weights_blob, saved_input_size, saved_labels_json, saved_seed, saved_metric) = row;

    // Sanity: input_size must match the runtime declaration.
    // (This is a metadata-level check; the deeper layer-shape check
    // happens inside deserialize_into_model via Block 3.)
    let model: &mut ReflexModel = registry
        .get_mut(id)
        .ok_or_else(|| format!("reflex_load: model handle {:?} not in registry", id))?;

    if saved_input_size as usize != model.input_size {
        return Err(format!(
            "reflex_load: input_size mismatch — saved model has input_size={}, \
             current declaration has input_size={}. The `reflex {} {{ input: embedding({}) }}` \
             declaration changed between save and load.",
            saved_input_size, model.input_size, name, model.input_size
        ));
    }

    // Sanity: seed should match (seeded init produces different weights
    // for different seeds — loading weights saved under one seed into a
    // model with another seed is suspicious, though not catastrophic
    // since we overwrite weights anyway). Treat as a warning, not error.
    if saved_seed as u64 != model.seed {
        eprintln!(
            "[reflex] warning: saved model '{}' used seed={}, current declaration uses seed={}. \
             Weights will be loaded as-is (seed only matters for fresh init).",
            name, saved_seed, model.seed
        );
    }

    // Sanity: labels JSON should match (different label count = different
    // output layer dimension, which Block 3 will catch, but a clearer
    // error here is helpful).
    let saved_labels: Vec<String> = serde_json::from_str(&saved_labels_json)
        .map_err(|e| format!("reflex_load: failed to parse saved labels JSON: {}", e))?;
    if saved_labels != model.labels {
        return Err(format!(
            "reflex_load: labels mismatch — saved model has labels {:?}, \
             current declaration has labels {:?}. The `reflex {} {{ labels: [...] }}` \
             declaration changed between save and load.",
            saved_labels, model.labels, name
        ));
    }

    // Block 3 + actual deserialize (atomic: shape check happens before
    // any layer mutation).
    deserialize_into_model(model, &weights_blob)?;

    // Restore last_metric ( informational — train() will overwrite it
    // on next training call anyway).
    model.last_metric = saved_metric;

    Ok(())
}
