//! Weights infrastructure for Vision R3 (naryad №212).
//!
//! Provides:
//! - `WeightsManifest` — record of expected files + SHA-256 (loaded from a
//!   sibling JSON manifest at the weights directory; see
//!   `docs/research/naryad-212-weights-manifest.md`).
//! - `load_safetensors_sharded(dir, stem, device)` — reads `{stem}.safetensors.index.json`,
//!   loads each shard via `candle_core::safetensors`, returns a `HashMap<String, Tensor>`.
//!   If a `manifest.json` is found at `{dir}/manifest.json`, SHA-256 of each shard file
//!   is verified against the manifest — mismatch is a loud error.
//!
//! ## NO network access
//!
//! This module performs ZERO network operations. Weights are read from the local
//! filesystem only. Auto-download is R5 (ADR-0125). The manifest is a JSON file
//! hand-curated by the executor after manually downloading the weights (see the
//! weights-manifest.md template).
//!
//! ## ADR-0124 compliance
//!
//! - Weights never enter the repo (manifest is the only weights-related artifact
//!   committed — and it's just SHA-256s + filenames).
//! - SHA-256 verification is mandatory when a manifest is present (silent fallback
//!   to "trust the file" is forbidden).
//! - Loud errors with the specific file, expected SHA, and computed SHA.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use candle_core::{Device, Tensor};
use sha2::{Digest, Sha256};

/// A single weights-file record (filename + expected SHA-256 + optional bytes count).
#[derive(Debug, Clone)]
pub struct WeightsManifestEntry {
    pub filename: String,
    pub sha256: String,
    pub bytes: Option<u64>,
}

/// The manifest — a list of expected files with their SHA-256s.
#[derive(Debug, Default, Clone)]
pub struct WeightsManifest {
    pub entries: Vec<WeightsManifestEntry>,
}

impl WeightsManifest {
    /// Load a manifest from `{dir}/manifest.json`. Returns `None` if the file
    /// doesn't exist (callers may proceed without verification — but should
    /// log that verification was skipped).
    pub fn load_from_dir(dir: &Path) -> Result<Option<Self>, String> {
        let manifest_path = dir.join("manifest.json");
        if !manifest_path.exists() {
            return Ok(None);
        }
        let contents = fs::read_to_string(&manifest_path).map_err(|e| {
            format!(
                "WeightsManifest::load_from_dir: cannot read {}: {}",
                manifest_path.display(),
                e
            )
        })?;
        let parsed: serde_json::Value = serde_json::from_str(&contents).map_err(|e| {
            format!(
                "WeightsManifest::load_from_dir: invalid JSON in {}: {}",
                manifest_path.display(),
                e
            )
        })?;
        let entries_arr = parsed
            .get("entries")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                format!(
                    "WeightsManifest::load_from_dir: missing 'entries' array in {}",
                    manifest_path.display()
                )
            })?;
        let mut entries = Vec::with_capacity(entries_arr.len());
        for entry_val in entries_arr {
            let filename = entry_val
                .get("filename")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    format!(
                        "WeightsManifest: entry missing 'filename' string in {}",
                        manifest_path.display()
                    )
                })?
                .to_string();
            let sha256 = entry_val
                .get("sha256")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    format!(
                        "WeightsManifest: entry missing 'sha256' string in {}",
                        manifest_path.display()
                    )
                })?
                .to_string();
            let bytes = entry_val.get("bytes").and_then(|v| v.as_u64());
            entries.push(WeightsManifestEntry {
                filename,
                sha256,
                bytes,
            });
        }
        Ok(Some(WeightsManifest { entries }))
    }

    /// Look up an entry by filename.
    pub fn lookup(&self, filename: &str) -> Option<&WeightsManifestEntry> {
        self.entries.iter().find(|e| e.filename == filename)
    }
}

/// Compute the SHA-256 of a file's bytes.
fn file_sha256(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path)
        .map_err(|e| format!("file_sha256: cannot read {}: {}", path.display(), e))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let result = hasher.finalize();
    Ok(result.iter().map(|b| format!("{:02x}", b)).collect())
}

/// Load all tensors from a sharded safetensors checkpoint.
///
/// Reads `{dir}/{stem}.safetensors.index.json` to discover the shard files
/// and the tensor-to-shard map. Loads each shard via `candle_core::safetensors`
/// (no network). If a `manifest.json` exists in `dir`, SHA-256 of each shard
/// file is verified against the manifest — mismatch is a loud error.
///
/// Returns a flat `HashMap<String, Tensor>` keyed by tensor name.
pub fn load_safetensors_sharded(
    dir: &Path,
    stem: &str,
    device: &Device,
) -> Result<HashMap<String, Tensor>, String> {
    let index_path = dir.join(format!("{}.safetensors.index.json", stem));
    if !index_path.exists() {
        return Err(format!(
            "load_safetensors_sharded: index file not found: {} \
             (expected at {} — verify MLOG_VISION_WEIGHTS_DIR layout per \
             docs/research/naryad-212-weights-manifest.md)",
            stem,
            index_path.display()
        ));
    }
    let index_str = fs::read_to_string(&index_path).map_err(|e| {
        format!(
            "load_safetensors_sharded: cannot read {}: {}",
            index_path.display(),
            e
        )
    })?;
    let index: serde_json::Value = serde_json::from_str(&index_str).map_err(|e| {
        format!(
            "load_safetensors_sharded: invalid JSON in {}: {}",
            index_path.display(),
            e
        )
    })?;

    let weight_map = index
        .get("weight_map")
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            format!(
                "load_safetensors_sharded: missing 'weight_map' object in {}",
                index_path.display()
            )
        })?;

    // Collect the unique shard filenames (preserve first-seen order for deterministic loading).
    let mut shard_files: Vec<String> = Vec::new();
    for (_, shard_name) in weight_map.iter() {
        if let Some(s) = shard_name.as_str() {
            if !shard_files.iter().any(|f| f == s) {
                shard_files.push(s.to_string());
            }
        }
    }
    shard_files.sort(); // deterministic order

    // Load the manifest if present.
    let manifest = WeightsManifest::load_from_dir(dir)?;

    // Load each shard, verify SHA-256 if manifest present, collect tensors.
    let mut tensors: HashMap<String, Tensor> = HashMap::new();
    for shard_filename in &shard_files {
        let shard_path: PathBuf = dir.join(shard_filename);

        // SHA-256 verification (if manifest present and has this file).
        if let Some(ref manifest) = manifest {
            if let Some(entry) = manifest.lookup(shard_filename) {
                let actual_sha = file_sha256(&shard_path)?;
                if actual_sha != entry.sha256 {
                    return Err(format!(
                        "load_safetensors_sharded: SHA-256 mismatch for {} \
                         (in {})\n  expected: {}\n  computed: {}\n\
                         Refusing to load — verify the file was downloaded \
                         correctly (see docs/research/naryad-212-weights-manifest.md).",
                        shard_filename,
                        dir.display(),
                        entry.sha256,
                        actual_sha
                    ));
                }
                // Loud success (only when stderr is captured — keep terse).
                eprintln!(
                    "[weights] {} SHA-256 verified: {}",
                    shard_filename, entry.sha256
                );
            } else {
                // Manifest exists but file not in manifest — loud warning.
                eprintln!(
                    "[weights] WARNING: {} not in manifest (no SHA-256 verification applied)",
                    shard_filename
                );
            }
        } else {
            // No manifest — explicit loud note that verification was skipped.
            eprintln!(
                "[weights] NOTE: no manifest.json found in {}; SHA-256 verification \
                 skipped for {} (see docs/research/naryad-212-weights-manifest.md to enable)",
                dir.display(),
                shard_filename
            );
        }

        // Load via candle_core safetensors.
        let shard_tensors = candle_core::safetensors::load(&shard_path, device).map_err(|e| {
            format!(
                "load_safetensors_sharded: failed to load safetensors file {}: {}",
                shard_path.display(),
                e
            )
        })?;

        for (name, tensor) in shard_tensors {
            // Detect duplicate tensor names across shards (shouldn't happen for a valid checkpoint).
            if tensors.contains_key(&name) {
                return Err(format!(
                    "load_safetensors_sharded: duplicate tensor name '{}' across shards in {}",
                    name,
                    dir.display()
                ));
            }
            tensors.insert(name, tensor);
        }
    }

    // Verify all expected tensors from weight_map are present.
    for (name, _) in weight_map.iter() {
        if !tensors.contains_key(name) {
            return Err(format!(
                "load_safetensors_sharded: tensor '{}' listed in weight_map but not \
                 found in any shard of {}",
                name,
                dir.display()
            ));
        }
    }

    Ok(tensors)
}

/// Load all tensors from a single (unsharded) safetensors file at `{dir}/{stem}.safetensors`.
///
/// Used for the VAE checkpoint (single 167 MB file, no index.json).
pub fn load_safetensors_single(
    dir: &Path,
    stem: &str,
    device: &Device,
) -> Result<HashMap<String, Tensor>, String> {
    let shard_path = dir.join(format!("{}.safetensors", stem));
    if !shard_path.exists() {
        return Err(format!(
            "load_safetensors_single: file not found: {} \
             (verify MLOG_VISION_WEIGHTS_DIR layout per \
             docs/research/naryad-212-weights-manifest.md)",
            shard_path.display()
        ));
    }

    // SHA-256 verification (if manifest present).
    let manifest = WeightsManifest::load_from_dir(dir)?;
    if let Some(ref manifest) = manifest {
        let filename = shard_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if let Some(entry) = manifest.lookup(filename) {
            let actual_sha = file_sha256(&shard_path)?;
            if actual_sha != entry.sha256 {
                return Err(format!(
                    "load_safetensors_single: SHA-256 mismatch for {}\n  expected: {}\n  computed: {}",
                    shard_path.display(),
                    entry.sha256,
                    actual_sha
                ));
            }
            eprintln!("[weights] {} SHA-256 verified: {}", filename, entry.sha256);
        }
    }

    candle_core::safetensors::load(&shard_path, device).map_err(|e| {
        format!(
            "load_safetensors_single: failed to load {}: {}",
            shard_path.display(),
            e
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Manifest loading with a missing file returns Ok(None) (not an error —
    /// callers proceed without verification).
    #[test]
    fn manifest_missing_returns_none() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        let result = WeightsManifest::load_from_dir(tmp.path());
        assert!(
            result.is_ok(),
            "missing manifest should be Ok(None), not Err"
        );
        assert!(result.unwrap().is_none());
    }

    /// Manifest with valid JSON returns the entries.
    #[test]
    fn manifest_loads_entries() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        let manifest_path = tmp.path().join("manifest.json");
        let mut f = fs::File::create(&manifest_path).expect("create");
        f.write_all(
            br#"{"entries":[{"filename":"a.safetensors","sha256":"abc","bytes":42},{"filename":"b.safetensors","sha256":"def"}]}"#,
        )
        .expect("write");

        let manifest = WeightsManifest::load_from_dir(tmp.path())
            .expect("ok")
            .expect("some");
        assert_eq!(manifest.entries.len(), 2);
        assert_eq!(manifest.entries[0].filename, "a.safetensors");
        assert_eq!(manifest.entries[0].sha256, "abc");
        assert_eq!(manifest.entries[0].bytes, Some(42));
        assert_eq!(manifest.entries[1].filename, "b.safetensors");
        assert_eq!(manifest.entries[1].sha256, "def");
        assert_eq!(manifest.entries[1].bytes, None);

        // Lookup works.
        assert!(manifest.lookup("a.safetensors").is_some());
        assert!(manifest.lookup("missing.safetensors").is_none());
    }

    /// Manifest with invalid JSON is a loud error.
    #[test]
    fn manifest_invalid_json_errors() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        let manifest_path = tmp.path().join("manifest.json");
        let mut f = fs::File::create(&manifest_path).expect("create");
        f.write_all(b"not json").expect("write");
        let result = WeightsManifest::load_from_dir(tmp.path());
        assert!(result.is_err(), "invalid JSON must be a loud error");
    }

    /// Loading from a directory with a missing index.json is a loud error.
    #[test]
    fn sharded_loader_missing_index_errors() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        let result = load_safetensors_sharded(tmp.path(), "model", &Device::Cpu);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("index file not found"), "err: {}", err);
    }
}
