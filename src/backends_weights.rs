//! Backend weights loading path (Наряд №334, ADR-0163 §2.1) — the ONLY
//! way real backend weights enter the system: SHA-pinned, SSRF-guarded,
//! manifest-mandatory. Silent fallback is forbidden at every layer (the
//! `vision_fetch_weights` discipline, №241; the SSRF лекало №130/№261).
//!
//! Layers, in refusal order:
//!   1. `weights_plan` — the DRY-RUN: registry entry → per-file plan
//!      (URL → path → pinned sha256 → bytes). Loud on unknown ids,
//!      missing manifests, and manifests without valid pins (the
//!      `validate_weights_source` contract). NO network, NO writes.
//!   2. `fetch_weights` — the REAL fetch: allowlist default-deny
//!      (`MLOG_BACKEND_WEIGHTS_ALLOWLIST`), SSRF guard with pinned
//!      resolves (DNS-rebinding protection), per-file download →
//!      SHA-256 verification → atomic write. Mismatch = loud Err, the
//!      file is NEVER written.
//!   3. Real backend CALLS (`stt_transcribe` / `omni_ask` /
//!      `vision_understand` with `METALOGOS_LLM_MOCK=false`) require the
//!      weights on disk — verified against the SAME manifest — and are
//!      PARKED by hardware (№294) in this environment: the refusal is
//!      loud and names the boundary.

use crate::backends::{
    validate_weights_source, weights_source, ShaPin, WeightsFile, BACKEND_REGISTRY,
};

/// One planned file: the resolved URL, the repo-relative destination
/// path, the pinned SHA-256 and the declared byte count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeightsPlanFile {
    pub url: String,
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

/// The dry-run plan for a backend's weights (№334 DoD: the dry-run path
/// — registry + loader + wire-up — without any network activity).
/// Refuses loudly: unknown weights_id, a `PendingNo334` pin (the typed
/// №333 boundary — never upgraded by hand), a missing manifest, or a
/// manifest that fails validation (e.g. a file without a pinned SHA).
pub fn weights_plan(weights_id: &str) -> Result<Vec<WeightsPlanFile>, String> {
    let entry = BACKEND_REGISTRY
        .iter()
        .find(|e| e.weights_id == weights_id)
        .ok_or_else(|| {
            format!(
                "backend weights: unknown weights_id '{}' (no registry record — \
                 the №333 registry is the SSOT)",
                weights_id
            )
        })?;
    let expected = match entry.pin {
        ShaPin::Pinned(h) => h,
        ShaPin::PendingNo334 => {
            return Err(format!(
                "backend weights: '{}' is PendingNo334 (the weights artifact is \
                 not pinned — ADR-0163 typed boundary) — refusing to plan a \
                 load; a pin must land in the registry first",
                weights_id
            ));
        }
    };
    let source = weights_source(weights_id).ok_or_else(|| {
        format!(
            "backend weights: '{}' has a registry pin but no per-file manifest \
             in WEIGHTS_SOURCES — a manifest is mandatory, silent fallback is \
             forbidden (№334)",
            weights_id
        )
    })?;
    validate_weights_source(source)?;
    // The primary artifact pin must correspond to a manifest file (the
    // registry and the manifest are two views of the same truth).
    if !source.files.iter().any(|f| f.sha256 == expected) {
        return Err(format!(
            "backend weights: registry pin of '{}' does not match any file in \
             its manifest — the registry and the manifest disagree",
            weights_id
        ));
    }
    Ok(source
        .files
        .iter()
        .map(|f| WeightsPlanFile {
            url: format!(
                "https://huggingface.co/{}/resolve/{}/{}",
                source.repo, source.revision, f.path
            ),
            path: f.path.to_string(),
            sha256: f.sha256.to_string(),
            bytes: f.bytes,
        })
        .collect())
}

/// SHA-256 of a byte slice, lowercase hex (the verification primitive).
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Verify one downloaded buffer against its pin: hash match AND byte
/// count match. Loud refusal on any mismatch (the file is not written —
/// the caller owns that ordering).
pub fn verify_pinned_bytes(
    expected_sha: &str,
    expected_bytes: u64,
    actual: &[u8],
) -> Result<(), String> {
    if actual.len() as u64 != expected_bytes {
        return Err(format!(
            "backend weights: byte-count mismatch (expected {}, got {}) — \
             refusing the artifact (silent fallback forbidden)",
            expected_bytes,
            actual.len()
        ));
    }
    let actual_hex = sha256_hex(actual);
    if actual_hex != expected_sha.to_lowercase() {
        return Err(format!(
            "backend weights: SHA-256 mismatch (expected {}, got {}) — \
             refusing the artifact; the file was NOT written",
            expected_sha, actual_hex
        ));
    }
    Ok(())
}

/// The REAL fetch path (№334): download every manifest file through the
/// SSRF guard + host allowlist, verify each against its pin, write to
/// `dest_dir/<repo-relative path>`. Returns the written paths.
///
/// Refusal order (everything fires BEFORE any network activity):
///   1. the plan refusals (`weights_plan`) — manifest mandatory;
///   2. allowlist default-deny: `MLOG_BACKEND_WEIGHTS_ALLOWLIST` unset
///      or empty → loud refusal (no network, no writes);
///   3. per-URL SSRF guard (`check_url_ssrf`, №130 лекало) — private/
///      loopback/link-local/metadata targets refused, resolves pinned
///      against DNS rebinding.
pub fn fetch_weights(weights_id: &str, dest_dir: &str) -> Result<Vec<String>, String> {
    let plan = weights_plan(weights_id)?;

    let allowlist_raw = std::env::var("MLOG_BACKEND_WEIGHTS_ALLOWLIST").map_err(|_| {
        "MODEL_WEIGHTS_UNSAFE: MLOG_BACKEND_WEIGHTS_ALLOWLIST is not set — backend \
         weights downloading is default-deny (the №241/№261 posture); set it to a \
         comma-separated list of trusted hostnames (e.g. huggingface.co)"
            .to_string()
    })?;
    let allowlist: Vec<String> = allowlist_raw
        .split(',')
        .map(|h| h.trim().to_lowercase())
        .filter(|h| !h.is_empty())
        .collect();
    if allowlist.is_empty() {
        return Err(
            "MODEL_WEIGHTS_UNSAFE: MLOG_BACKEND_WEIGHTS_ALLOWLIST is empty — backend \
             weights downloading is default-deny"
                .to_string(),
        );
    }

    // SSRF guard + allowlist per planned URL, all BEFORE any request.
    let mut resolves = Vec::new();
    for f in &plan {
        let parsed = reqwest::Url::parse(&f.url)
            .map_err(|e| format!("backend weights: invalid URL '{}': {}", f.url, e))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| format!("backend weights: URL '{}' has no host", f.url))?
            .to_lowercase();
        if !allowlist.contains(&host) {
            return Err(format!(
                "MODEL_WEIGHTS_UNSAFE: host '{}' is not in MLOG_BACKEND_WEIGHTS_ALLOWLIST \
                 (allowed: {}) — refusing to download weights",
                host,
                allowlist.join(", ")
            ));
        }
        if parsed.scheme() != "https" {
            return Err(format!(
                "MODEL_WEIGHTS_UNSAFE: scheme '{}' is refused — backend weights are \
                 fetched over https only",
                parsed.scheme()
            ));
        }
        resolves.push(crate::builtins::http::check_url_ssrf(&f.url)?);
    }

    // Client with SSRF-pinned resolves (the http.rs builder pattern).
    let mut builder =
        reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(600));
    for pair in resolves.into_iter().flatten() {
        builder = builder.resolve(&pair.0, pair.1);
    }
    let client = builder
        .build()
        .map_err(|e| format!("backend weights: client build failed: {}", e))?;

    let dest_root = std::path::Path::new(dest_dir);
    let mut written = Vec::new();
    for f in &plan {
        let resp = client
            .get(&f.url)
            .send()
            .map_err(|e| format!("backend weights: GET {} failed: {}", f.url, e))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!(
                "backend weights: GET {} returned HTTP {} — refusing (no partial \
                 writes, no skip-and-continue)",
                f.url, status
            ));
        }
        let body = resp
            .bytes()
            .map_err(|e| format!("backend weights: reading body of {} failed: {}", f.url, e))?;
        verify_pinned_bytes(&f.sha256, f.bytes, &body)?;
        let out_path = dest_root.join(&f.path);
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                format!("backend weights: cannot create {}: {}", parent.display(), e)
            })?;
        }
        std::fs::write(&out_path, &body).map_err(|e| {
            format!(
                "backend weights: cannot write {}: {}",
                out_path.display(),
                e
            )
        })?;
        written.push(out_path.display().to_string());
    }
    Ok(written)
}

/// Whether the REAL inference path may proceed for a weights id: the
/// registry pin is real, the manifest validates, and every file exists
/// under `dir` with the pinned SHA-256. The file-verification half runs
/// on the actual bytes — a file present but hash-mismatched is a refusal.
pub fn weights_loaded(weights_id: &str, dir: &str) -> Result<bool, String> {
    let plan = weights_plan(weights_id)?;
    for f in &plan {
        let p = std::path::Path::new(dir).join(&f.path);
        if !p.exists() {
            return Ok(false);
        }
        let bytes = std::fs::read(&p)
            .map_err(|e| format!("backend weights: cannot read {}: {}", p.display(), e))?;
        verify_pinned_bytes(&f.sha256, f.bytes, &bytes).map_err(|e| {
            format!(
                "backend weights: existing file {} failed verification: {}",
                p.display(),
                e
            )
        })?;
    }
    Ok(true)
}

/// Static assertion used by tests via the public API surface: the
/// №334-scoped entries and their manifest parity.
pub fn scoped_entries() -> Vec<&'static crate::backends::BackendEntry> {
    BACKEND_REGISTRY
        .iter()
        .filter(|e| weights_source(e.weights_id).is_some())
        .collect()
}

/// Re-exported for the обвязка modules: one manifest file (used in the
/// loud real-mode refusal messages to name the missing artifact).
pub fn first_manifest_file(weights_id: &str) -> Option<&'static WeightsFile> {
    weights_source(weights_id).and_then(|s| s.files.first())
}
