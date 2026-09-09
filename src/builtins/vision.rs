//! Vision builtins (Наряд №210 skeleton → №240 R4.2 real paths).
//!
//! Наряд №240 (R4.2) replaces the R1 loud stubs for `vision_generate`,
//! `vision_list`, `vision_export` with real dispatch functions, following
//! the reflex лекало (`src/builtins/reflex.rs`): the interpreter and the VM
//! each hold their own registry/declaration state and route through these
//! shared dispatch functions — the inference logic is NOT reimplemented
//! per backend.
//!
//! **Arity contract (plan §3, R4):** `vision_generate("decl_name", "prompt")`
//! — 2 arguments. The declaration carries model/steps/width/height/seed;
//! the registry spec was truth-up'd 3→2 in №240 (the R1 stub doc
//! "(model_name, prompt, seed)" predates the declaration language).
//!
//! **Loud refusal discipline (§3.5):** `vision_generate` performs a real
//! pipeline; missing `MLOG_VISION_WEIGHTS_DIR` or missing weights components
//! is an honest environment refusal (loud `Err` naming the env var and the
//! missing component) — NOT a silent stub. `vision_edit` remains a loud
//! stub (R6 edit — №243). `vision_save`/`vision_load` became real
//! SQLite-persistence dispatches in №242 (R6.1, `src/vision/store.rs`) —
//! the same state-carrying interception pattern plus the program's
//! database connection (`db { url: "sqlite:..." }`).
//!
//! Every handler either returns `Ok` with a real result or `Err` with a
//! message naming the failure. It does NOT return placeholder values,
//! does NOT silently succeed, and does NOT `panic!`.

use crate::interpreter::Value;
#[cfg(feature = "vision")]
use crate::vision::VisionArtifact;
use crate::vision::VisionRegistry;
use std::collections::HashMap;
use std::path::PathBuf;

/// Directory layout of the Z-Image-Turbo weights tree (manifest №212).
/// Each entry: (subdirectory, human name for loud error messages).
const WEIGHTS_COMPONENTS: [(&str, &str); 4] = [
    ("tokenizer", "tokenizer (BPE)"),
    ("text_encoder", "text encoder (Qwen3-4B)"),
    ("transformer", "transformer (Z-Image DiT)"),
    ("vae", "VAE decoder"),
];

/// `vision_generate(decl_name, prompt) -> Vision`
///
/// Real path (Наряд №240, R4.2):
/// 1. Resolve `decl_name` against the registered `vision { }` declarations
///    (unknown name → loud `Err` listing the declared names).
/// 2. Runtime re-check `model ∈ KNOWN_VISION_MODELS` (defense-in-depth;
///    semantic validates at compile time — this re-check guards hand-built
///    `Program`s and deserialized bytecode).
/// 3. Resolve `MLOG_VISION_WEIGHTS_DIR`; missing env / missing component →
///    loud `Err` naming the env var and the missing component.
/// 4. Full clip: tokenizer → text encoder (Qwen3-4B) → DiT (Z-Image) +
///    `flow_match_euler_sample` (steps and seed from the declaration) →
///    VAE decode → PNG encode → artifact into the `VisionRegistry` →
///    `Value::Vision(id)`.
pub fn vision_generate_dispatch(
    decls: &HashMap<String, crate::bytecode::CompiledVisionDecl>,
    registry: &mut VisionRegistry,
    args: &[Value],
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "vision_generate: expects 2 arguments (decl_name, prompt), got {} — \
             seed/steps/width/height come from the `vision {{ }}` declaration (R4 contract, plan §3)",
            args.len()
        ));
    }
    let decl_name = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "vision_generate: first argument must be a declaration name (String), got {}",
                value_type_name(other)
            ))
        }
    };
    let prompt = match &args[1] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "vision_generate: second argument must be a prompt (String), got {}",
                value_type_name(other)
            ))
        }
    };
    if prompt.is_empty() {
        return Err("vision_generate: prompt must not be empty".to_string());
    }

    // 1. Resolve the declaration (loud, with the list of declared names).
    let decl = match decls.get(&decl_name) {
        Some(d) => d,
        None => {
            let mut names: Vec<&String> = decls.keys().collect();
            names.sort();
            let listed = if names.is_empty() {
                "no `vision { }` declarations in the program".to_string()
            } else {
                names
                    .into_iter()
                    .map(|n| format!("\"{}\"", n))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            return Err(format!(
                "vision_generate: vision declaration '{}' not declared (declared: {})",
                decl_name, listed
            ));
        }
    };

    // 2. Runtime re-check of the model (defense-in-depth).
    if !crate::vision::KNOWN_VISION_MODELS.contains(&decl.model.as_str()) {
        return Err(format!(
            "vision_generate: declaration '{}' names unknown model '{}' (known models: {})",
            decl.name,
            decl.model,
            crate::vision::KNOWN_VISION_MODELS.join(", ")
        ));
    }

    // 3. Weights environment (loud environment refusal — NOT a stub).
    let weights_dir = match std::env::var_os("MLOG_VISION_WEIGHTS_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => {
            return Err(
                "vision_generate: MLOG_VISION_WEIGHTS_DIR is not set — real generation \
                 requires the Z-Image-Turbo weights directory (see \
                 docs/research/naryad-237-real-weights-runbook.md)"
                    .to_string(),
            )
        }
    };
    for (sub, human) in WEIGHTS_COMPONENTS {
        let path = weights_dir.join(sub);
        if !path.is_dir() {
            return Err(format!(
                "vision_generate: MLOG_VISION_WEIGHTS_DIR component missing: '{}' ({}) not found at {}",
                sub,
                human,
                path.display()
            ));
        }
    }

    // 4. Full clip. Steps/seed from the declaration. The R4.2 z-image-turbo
    //    pipeline generates a fixed 1024×1024 image (the sampler derives the
    //    128×128 latent from the DiT config); a declaration asking for a
    //    different size is a loud error — silently emitting a different
    //    resolution than declared would be a fake answer. Size
    //    parameterization is R5 manifest territory.
    if decl.width != 1024 || decl.height != 1024 {
        return Err(format!(
            "vision_generate: declaration '{}' asks for {}x{} — the R4.2 z-image-turbo \
             pipeline generates a fixed 1024x1024 (sampler derives the latent from the \
             DiT config); other resolutions require size parameterization (R5 manifest)",
            decl.name, decl.width, decl.height
        ));
    }

    #[cfg(feature = "vision")]
    {
        let vision_id = generate_real(decl, registry, &prompt)?;
        Ok(Value::Vision(vision_id))
    }
    #[cfg(not(feature = "vision"))]
    {
        let _ = registry; // registry is only written by the gated real path
        Err(format!(
            "vision_generate: this build was compiled WITHOUT the `vision` feature — \
             real generation requires `--features vision` (declaration '{}' resolved \
             cleanly; model '{}', steps {}, seed {} — loud refusal, not a placeholder)",
            decl.name, decl.model, decl.steps, decl.seed
        ))
    }
}

/// Real inference clip (Наряд №240, R4.2). Feature-gated: the vision
/// inference stack (tokenizer/text_encoder/dit/sampler/vae) lives behind
/// `--features vision` (ADR-0122 — vision stays off-by-default).
#[cfg(feature = "vision")]
fn generate_real(
    decl: &crate::bytecode::CompiledVisionDecl,
    registry: &mut VisionRegistry,
    prompt: &str,
) -> Result<crate::vision::VisionId, String> {
    use candle_core::{DType, Device};

    let weights_dir =
        PathBuf::from(std::env::var_os("MLOG_VISION_WEIGHTS_DIR").ok_or_else(|| {
            "vision_generate: MLOG_VISION_WEIGHTS_DIR is not set (re-checked at clip entry)"
                .to_string()
        })?);

    // Stage A: tokenize.
    let tokenizer = crate::vision::tokenizer::Tokenizer::from_dir(&weights_dir.join("tokenizer"))?;
    let tokens = tokenizer.encode(prompt)?;

    // Stage B: text encoder (Qwen3-4B).
    let te_tensors = crate::vision::weights::load_safetensors_sharded(
        &weights_dir.join("text_encoder"),
        "model",
        &Device::Cpu,
    )?;
    let encoder = crate::vision::text_encoder::TextEncoder::from_weights(
        &crate::vision::text_encoder::QWEN3_4B_CONFIG,
        &te_tensors,
    )?;
    drop(te_tensors);
    let cap = encoder.forward(&tokens)?;

    // Stage C: DiT + flow-match Euler sampler.
    // decl.steps counts Euler updates (ADR-0124 distilled NFE=8); the
    // sampler takes the sigma count = steps + 1 (matches the №212 clip:
    // 9 sigmas → 8 forwards).
    let dit_tensors = crate::vision::weights::load_safetensors_sharded(
        &weights_dir.join("transformer"),
        "diffusion_pytorch_model",
        &Device::Cpu,
    )?;
    let dit = crate::vision::dit::ZImageTransformer::from_weights(&dit_tensors)?;
    drop(dit_tensors);
    let latent = crate::vision::sampler::flow_match_euler_sample(
        &dit,
        &cap,
        decl.seed,
        decl.steps as usize + 1,
        0.0,
    )?;

    // Stage D: VAE decode → PNG bytes.
    let vae_tensors = crate::vision::weights::load_safetensors_single(
        &weights_dir.join("vae"),
        "diffusion_pytorch_model",
        &Device::Cpu,
    )?;
    let decoder = crate::vision::vae::VaeDecoder::from_weights(&vae_tensors)?;
    drop(vae_tensors);
    let img = decoder.decode(&latent)?;
    let img = img.to_dtype(DType::F32).map_err(|e| e.to_string())?;
    let png_bytes = crate::vision::vae::encode_png(&img)?;

    // ── Наряд №241 (R5, Block 1.3): sign ALWAYS — no unsigned artifact
    // can ever reach the registry. The PNG gets the LSB watermark; the
    // artifact carries the provenance manifest (final-PNG SHA is
    // computed AFTER the watermark, so it describes exactly the bytes
    // the default export ships). Every signing failure is a loud `Err`
    // BEFORE insertion — a silent "unmarked but registered" outcome is
    // forbidden (ADR-0125: security is a type, not a procedure).
    let png_bytes = crate::vision::provenance::embed_lsb_watermark(&png_bytes, &decl.model)?;
    let manifest = crate::vision::provenance::VisionManifest {
        model_id: decl.model.clone(),
        model_sha256: crate::vision::provenance::weights_tree_sha256(&weights_dir)?,
        seed: decl.seed,
        prompt_sha256: crate::vision::provenance::prompt_hash(prompt),
        policy: match decl.policy {
            Some(crate::bytecode::CompiledVisionPolicy::Safe) => "safe".to_string(),
            // Наряд №241 (Block 3.1): omitted policy → the honest marker
            // "unspecified" in the manifest (ADR-0125 policy-relax).
            None => "unspecified".to_string(),
        },
        timestamp: chrono::Utc::now().to_rfc3339(),
        png_sha256: crate::vision::provenance::sha256_hex(&png_bytes),
    };

    let id = registry.insert(VisionArtifact {
        png_bytes,
        manifest: Some(manifest),
    });
    Ok(id)
}

/// `vision_list() -> List<String>`
///
/// Real handles from the `VisionRegistry`, sorted by id (determinism).
/// Returns `[Vision#N]` display handles — the same rendering `Value::Vision`
/// prints, so list output round-trips visually with generate results.
pub fn vision_list_dispatch(registry: &VisionRegistry, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!(
            "vision_list: expects 0 arguments, got {}",
            args.len()
        ));
    }
    let handles = registry
        .list_ids()
        .into_iter()
        .map(|id| Value::String(format!("[Vision#{}]", id.0)))
        .collect();
    Ok(Value::List(handles))
}

/// `vision_export(handle, path) -> String`
///
/// **Signed export** (Наряд №241, Block 1.4 — ADR-0125): writes the
/// artifact's watermarked PNG bytes to `path` AND the provenance manifest
/// to the sidecar `<path>.manifest.json`. The R4.2 unsigned-WARN is GONE
/// — every default export is signed by construction, because
/// `vision_generate` signs always (Block 1.3).
///
/// A registry artifact WITHOUT a manifest (hand-built/deserialized —
/// `VisionArtifact.manifest: None`) cannot be exported here: loud `Err`
/// naming the `VISION_UNSIGNED_EXPORT` check-id — the runtime backstop of
/// the Category-A gate (Block 2.2: an unsigned artifact can only exist
/// outside the real generation path, i.e. hand-built or deserialized;
/// exporting it must be as loud as compiling it). The explicit opt-out is
/// `vision_export_raw` (Block 2.1).
pub fn vision_export_dispatch(registry: &VisionRegistry, args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "vision_export: expects 2 arguments (handle, path), got {}",
            args.len()
        ));
    }
    let id = match &args[0] {
        Value::Vision(id) => *id,
        other => {
            return Err(format!(
                "vision_export: first argument must be a Vision handle, got {}",
                value_type_name(other)
            ))
        }
    };
    let path = match &args[1] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "vision_export: second argument must be a path (String), got {}",
                value_type_name(other)
            ))
        }
    };
    let artifact = registry.get(id).ok_or_else(|| {
        format!(
            "vision_export: vision handle [Vision#{}] not found in the registry \
             (was it generated in this session? artifacts do not persist across runs)",
            id.0
        )
    })?;
    let manifest = artifact.manifest.as_ref().ok_or_else(|| {
        format!(
            "VISION_UNSIGNED_EXPORT: [Vision#{}] carries no provenance manifest — signed \
             export is impossible (hand-built or deserialized artifact; ADR-0125). \
             The explicit opt-out is vision_export_raw",
            id.0
        )
    })?;
    std::fs::write(&path, &artifact.png_bytes)
        .map_err(|e| format!("vision_export: write to {}: {}", path, e))?;
    let sidecar = format!("{}.manifest.json", path);
    let sidecar_json = crate::vision::provenance::manifest_sidecar_json(manifest)?;
    std::fs::write(&sidecar, sidecar_json)
        .map_err(|e| format!("vision_export: write sidecar {}: {}", sidecar, e))?;
    Ok(Value::String(path))
}

/// `vision_export_raw(handle, path) -> String`
///
/// **Explicit opt-out** (Наряд №241, Block 2.1 — ADR-0125: "Opt-out is a
/// separate explicit form `export_raw` with a loud audit warning").
/// Writes the artifact's PNG bytes AS-IS: no watermark embedding, no
/// manifest sidecar, no signature requirement. The loud layer is the
/// audit: every `vision_export_raw` call site is flagged as a
/// `VISION_UNSIGNED_EXPORT_RAW` audit-WARNING (Block 2.3; the check-id is
/// fixed here — ADR-0125 does not name the raw-warning itself).
///
/// Unlike `vision_export`, raw export works on ANY registry artifact —
/// including hand-built ones without a manifest: the point of the opt-out
/// is that the operator CHOSE unsigned, loudly, in source.
pub fn vision_export_raw_dispatch(
    registry: &VisionRegistry,
    args: &[Value],
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "vision_export_raw: expects 2 arguments (handle, path), got {}",
            args.len()
        ));
    }
    let id = match &args[0] {
        Value::Vision(id) => *id,
        other => {
            return Err(format!(
                "vision_export_raw: first argument must be a Vision handle, got {}",
                value_type_name(other)
            ))
        }
    };
    let path = match &args[1] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "vision_export_raw: second argument must be a path (String), got {}",
                value_type_name(other)
            ))
        }
    };
    let artifact = registry.get(id).ok_or_else(|| {
        format!(
            "vision_export_raw: vision handle [Vision#{}] not found in the registry \
             (was it generated in this session? artifacts do not persist across runs)",
            id.0
        )
    })?;
    std::fs::write(&path, &artifact.png_bytes)
        .map_err(|e| format!("vision_export_raw: write to {}: {}", path, e))?;
    Ok(Value::String(path))
}

/// `vision_save(handle, name) -> String`
///
/// **SQLite persistence of vision artifacts** (Наряд №242, R6.1 — the
/// first third of R6 "Edit + LoRA", plan §7.1). Writes the artifact
/// (PNG bytes as a BLOB + provenance manifest JSON) into the program's
/// own database — the connection declared via `db { url: "sqlite:..." }`
/// — through `crate::vision::store`. The name is the persistent key; the
/// registry id is a session handle and is NOT persisted (prerequisites
/// №242: id — сессионный хэндл, персистентный ключ = name).
///
/// Loud refusals (§3.5): no database configured (names the `db`-decl),
/// empty name, name collision (plain INSERT — a silent upsert would
/// quietly destroy the stored artifact's provenance chain), unknown
/// handle. PNG bytes never touch the disk here — the DB BLOB is the
/// whole store (disk is export territory).
pub fn vision_save_dispatch(
    registry: &VisionRegistry,
    db_conn: Option<&rusqlite::Connection>,
    args: &[Value],
) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "vision_save: expects 2 arguments (handle, name), got {}",
            args.len()
        ));
    }
    let id = match &args[0] {
        Value::Vision(id) => *id,
        other => {
            return Err(format!(
                "vision_save: first argument must be a Vision handle, got {}",
                value_type_name(other)
            ))
        }
    };
    let name = match &args[1] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "vision_save: second argument must be the artifact name (String), got {}",
                value_type_name(other)
            ))
        }
    };
    let artifact = registry.get(id).ok_or_else(|| {
        format!(
            "vision_save: vision handle [Vision#{}] not found in the registry \
             (was it generated in this session? registry handles do not persist \
             across runs — persistence goes through vision_save)",
            id.0
        )
    })?;
    let conn = db_conn.ok_or_else(|| {
        "vision_save: no database connection — vision persistence requires the \
         program's SQLite database; declare db { url: \"sqlite:vision.db\" } (or \
         db { url: \"sqlite::memory:\" }) in the program first"
            .to_string()
    })?;
    crate::vision::store::save(conn, &name, artifact)?;
    Ok(Value::String(name))
}

/// `vision_load(name) -> Vision`
///
/// **SQLite persistence of vision artifacts** (Наряд №242, R6.1). Reads
/// the artifact saved under `name` back from the program's database and
/// inserts it into THIS session's registry, returning a fresh handle.
///
/// Verbatim contract (Block 1.3): the bytes and the manifest come out of
/// the DB exactly as they went in — persistence NEVER regenerates or
/// supplements provenance (the original generation `timestamp`
/// survives). A fresh registry id is assigned (`registry.insert` is
/// monotonic from zero) — ids are session handles, the name is the
/// persistent key.
///
/// Loud refusals: no database configured, unknown name (with the list of
/// saved names as loud diagnostics), corrupted manifest JSON in the DB
/// (degrading it to an unsigned artifact would quietly strip provenance
/// — forbidden, Block 1.3).
pub fn vision_load_dispatch(
    registry: &mut VisionRegistry,
    db_conn: Option<&rusqlite::Connection>,
    args: &[Value],
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "vision_load: expects 1 argument (name), got {}",
            args.len()
        ));
    }
    let name = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "vision_load: argument must be the artifact name (String), got {}",
                value_type_name(other)
            ))
        }
    };
    let conn = db_conn.ok_or_else(|| {
        "vision_load: no database connection — vision persistence requires the \
         program's SQLite database; declare db { url: \"sqlite:vision.db\" } (or \
         db { url: \"sqlite::memory:\" }) in the program first"
            .to_string()
    })?;
    let artifact = match crate::vision::store::load(conn, &name)? {
        Some(a) => a,
        None => {
            let listed = match crate::vision::store::list(conn) {
                Ok(names) if names.is_empty() => "nothing saved in this database yet".to_string(),
                Ok(names) => names
                    .iter()
                    .map(|n| format!("\"{}\"", n))
                    .collect::<Vec<_>>()
                    .join(", "),
                Err(e) => format!("could not list saved names: {}", e),
            };
            return Err(format!(
                "vision_load: no artifact named '{}' in vision_artifacts (saved: {})",
                name, listed
            ));
        }
    };
    let id = registry.insert(artifact);
    Ok(Value::Vision(id))
}

/// `vision_fetch_weights(manifest_url, dest_dir) -> String`
///
/// **SSRF-guarded, allowlist-gated, SHA-pinned weights fetching**
/// (Наряд №241, Block 3.2 — ADR-0125 `MODEL_WEIGHTS_UNSAFE`; the
/// compiler-side SSOT gate shared with the Voice pillar lives in
/// `src/audit.rs`, this is the real enforcement path).
///
/// Contract:
/// 1. **Allowlist default-deny** (loud): env `MLOG_VISION_WEIGHTS_ALLOWLIST`
///    = comma-separated hostnames. Unset/empty → loud refusal — downloading
///    is FORBIDDEN until the operator names the hosts (Block 3.2в).
/// 2. **SSRF-guard** (лекало №130): `check_url_ssrf` resolves DNS, refuses
///    private/loopback/link-local/metadata targets, returns pinned
///    addresses; the kill-switch `METALOGOS_HTTP_ALLOW_PRIVATE` keeps its
///    exact pre-existing semantics (not weakened).
/// 3. **manifest.json-class only**: the URL points at the weights manifest
///    (path ending `manifest.json`) or at the package base (we append
///    `/manifest.json`). A bare `.safetensors` URL is refused — no manifest,
///    no pin source. Pickle-RCE-class extensions are refused by extension,
///    loudly.
/// 4. **SHA-256 pinning** (reuses `WeightsManifest`, `src/vision/weights.rs`
///    is NOT modified): the fetched manifest lists `filename`+`sha256`;
///    every file is downloaded into memory, hashed, compared against the
///    pin — mismatch is a loud Err naming expected/computed SHA and the
///    file is NOT written. Entries must be bare `.safetensors` filenames
///    (no path separators, no traversal).
///
/// The downloaded tree (`manifest.json` + shards) is directly consumable
/// by `vision_generate` via `MLOG_VISION_WEIGHTS_DIR` (weights.rs verifies
/// SHAs again at load time — defense-in-depth, same pins).
pub(crate) fn builtin_vision_fetch_weights(args: &[Value]) -> Result<Value, String> {
    let url = match args.first() {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "vision_fetch_weights: first argument must be the manifest URL (String), got {}",
                value_type_name(other)
            ))
        }
        None => {
            return Err(
                "vision_fetch_weights: expects 2 arguments (manifest_url, dest_dir)".to_string(),
            )
        }
    };
    let dest_dir = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "vision_fetch_weights: second argument must be the destination directory (String), got {}",
                value_type_name(other)
            ))
        }
        None => return Err("vision_fetch_weights: expects 2 arguments (manifest_url, dest_dir)".to_string()),
    };
    if args.len() != 2 {
        return Err(format!(
            "vision_fetch_weights: expects 2 arguments (manifest_url, dest_dir), got {}",
            args.len()
        ));
    }

    // (в) Allowlist — DEFAULT-DENY. Empty/unset env = loud refusal before
    // any network activity (contract test asserts no side effects).
    let allowlist_raw = std::env::var("MLOG_VISION_WEIGHTS_ALLOWLIST").map_err(|_| {
        "MODEL_WEIGHTS_UNSAFE: MLOG_VISION_WEIGHTS_ALLOWLIST is not set — weights \
         downloading is default-deny (ADR-0125); set it to a comma-separated \
         list of trusted hostnames to enable vision_fetch_weights"
            .to_string()
    })?;
    let allowlist: Vec<String> = allowlist_raw
        .split(',')
        .map(|h| h.trim().to_lowercase())
        .filter(|h| !h.is_empty())
        .collect();
    if allowlist.is_empty() {
        return Err(
            "MODEL_WEIGHTS_UNSAFE: MLOG_VISION_WEIGHTS_ALLOWLIST is empty — weights \
             downloading is default-deny (ADR-0125)"
                .to_string(),
        );
    }

    // URL parsing + host allowlist match (case-insensitive).
    let parsed = reqwest::Url::parse(&url)
        .map_err(|e| format!("vision_fetch_weights: invalid URL '{}': {}", url, e))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| format!("vision_fetch_weights: URL '{}' has no host", url))?
        .to_lowercase();
    if !allowlist.contains(&host) {
        return Err(format!(
            "MODEL_WEIGHTS_UNSAFE: host '{}' is not in MLOG_VISION_WEIGHTS_ALLOWLIST \
             (allowed: {}) — refusing to download weights (ADR-0125)",
            host,
            allowlist.join(", ")
        ));
    }

    // (а) SSRF-guard — лекало №130. Refuses private/loopback/link-local/
    // metadata targets; returns pinned addresses (DNS-rebinding protection).
    // Kill-switch semantics unchanged.
    let resolves = crate::builtins::http::check_url_ssrf(&url)?;

    // (б) manifest.json-class only: manifest URL or package base.
    let path = parsed.path().to_lowercase();
    if path.ends_with(".safetensors") {
        return Err(format!(
            "MODEL_WEIGHTS_UNSAFE: '{}' points at a bare weights file — no manifest, \
             no pinned SHA-256 source; fetch the manifest.json of the package instead \
             (ADR-0125)",
            url
        ));
    }
    const PICKLE_CLASS: &[&str] = &[
        ".pkl", ".pickle", ".pt", ".pth", ".ckpt", ".bin", ".py", ".so", ".dll", ".exe", ".zip",
        ".tar", ".gz", ".7z",
    ];
    if PICKLE_CLASS.iter().any(|ext| path.ends_with(ext)) {
        return Err(format!(
            "MODEL_WEIGHTS_UNSAFE: '{}' is not a manifest.json-class URL — \
             pickle-RCE-class weight formats are refused by extension (ADR-0125)",
            url
        ));
    }
    let manifest_url = if path.ends_with("manifest.json") {
        url.clone()
    } else {
        format!("{}/manifest.json", url.trim_end_matches('/'))
    };

    // (г) Fetch + pin + write. The weights-verification stack
    // (`WeightsManifest`, `src/vision/weights.rs`) is vision-gated — in a
    // non-vision build the security layers above (allowlist default-deny,
    // SSRF guard, URL-class refusals) still fire loudly, and the actual
    // download is an honest environment refusal (same discipline as
    // vision_generate's non-gated refusal).
    #[cfg(feature = "vision")]
    {
        // Client with SSRF-pinned resolves (same builder pattern as http.rs).
        let mut builder =
            reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(600));
        for (domain, addr) in resolves {
            builder = builder.resolve(&domain, addr);
        }
        let client = builder
            .build()
            .map_err(|e| format!("vision_fetch_weights: client build failed: {}", e))?;
        fetch_and_pin_weights(&client, &manifest_url, &dest_dir)?;
        Ok(Value::String(dest_dir))
    }
    #[cfg(not(feature = "vision"))]
    {
        let _ = (manifest_url, dest_dir, resolves);
        Err(format!(
            "vision_fetch_weights: this build was compiled WITHOUT the `vision` feature — \
             weights verification (pinned SHA-256 via WeightsManifest) requires \
             `--features vision` (security layers already enforced: allowlist '{}', \
             SSRF guard, manifest.json-class check — loud refusal, not a placeholder)",
            host
        ))
    }
}

/// The gated download-and-pin body (Наряд №241 Block 3.2г): fetch the
/// manifest, parse it via REUSED `WeightsManifest` (src/vision/weights.rs
/// NOT modified), then per-entry fetch → SHA-256 pin verification →
/// write. Mismatch = loud Err, file NEVER written.
#[cfg(feature = "vision")]
fn fetch_and_pin_weights(
    client: &reqwest::blocking::Client,
    manifest_url: &str,
    dest_dir: &str,
) -> Result<(), String> {
    let fetch = |url: &str| -> Result<bytes::Bytes, String> {
        let resp = client
            .get(url)
            .send()
            .map_err(|e| format!("vision_fetch_weights: GET {} failed: {}", url, e))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!(
                "vision_fetch_weights: GET {} returned HTTP {}",
                url, status
            ));
        }
        resp.bytes().map_err(|e| {
            format!(
                "vision_fetch_weights: reading body of {} failed: {}",
                url, e
            )
        })
    };

    let manifest_bytes = fetch(manifest_url)?;
    let dest = std::path::PathBuf::from(dest_dir);
    std::fs::create_dir_all(&dest).map_err(|e| {
        format!(
            "vision_fetch_weights: cannot create {}: {}",
            dest.display(),
            e
        )
    })?;
    let manifest_path = dest.join("manifest.json");
    std::fs::write(&manifest_path, &manifest_bytes).map_err(|e| {
        format!(
            "vision_fetch_weights: cannot write {}: {}",
            manifest_path.display(),
            e
        )
    })?;
    let manifest =
        crate::vision::weights::WeightsManifest::load_from_dir(&dest)?.ok_or_else(|| {
            "vision_fetch_weights: manifest.json vanished between write and load (filesystem race)"
                .to_string()
        })?;
    if manifest.entries.is_empty() {
        return Err(format!(
            "vision_fetch_weights: manifest at {} has no entries — nothing pinned to fetch",
            manifest_url
        ));
    }

    let base = manifest_url
        .trim_end_matches("manifest.json")
        .trim_end_matches('/');
    let mut total_bytes: u64 = 0;
    for entry in &manifest.entries {
        // Filename hygiene: bare .safetensors names only — no paths, no
        // traversal, no pickle-class surprises smuggled via the manifest.
        let name = entry.filename.as_str();
        if name.contains('/') || name.contains('\\') || name.contains("..") {
            return Err(format!(
                "MODEL_WEIGHTS_UNSAFE: manifest entry '{}' is not a bare filename — \
                 path separators/traversal are refused (ADR-0125)",
                name
            ));
        }
        if !name.ends_with(".safetensors") {
            return Err(format!(
                "MODEL_WEIGHTS_UNSAFE: manifest entry '{}' is not a .safetensors file — \
                 pickle-RCE-class weight formats are refused (ADR-0125)",
                name
            ));
        }
        let file_url = format!("{}/{}", base, name);
        let file_bytes = fetch(&file_url)?;
        crate::vision::provenance::verify_sha_pin(name, &entry.sha256, &file_bytes)?;
        if let Some(want) = entry.bytes {
            if file_bytes.len() as u64 != want {
                return Err(format!(
                    "MODEL_WEIGHTS_UNSAFE: byte-count mismatch for '{}' — manifest says {}, \
                     got {} (file NOT written)",
                    name,
                    want,
                    file_bytes.len()
                ));
            }
        }
        let file_path = dest.join(name);
        std::fs::write(&file_path, &file_bytes).map_err(|e| {
            format!(
                "vision_fetch_weights: cannot write {}: {}",
                file_path.display(),
                e
            )
        })?;
        total_bytes += file_bytes.len() as u64;
        eprintln!(
            "vision_fetch_weights: {} SHA-256 OK ({}) — {} bytes",
            name,
            crate::vision::provenance::sha256_hex(&file_bytes),
            file_bytes.len()
        );
    }

    eprintln!(
        "vision_fetch_weights: {} file(s), {} bytes total, written to {} (SSRF-guarded, SHA-pinned)",
        manifest.entries.len(),
        total_bytes,
        dest.display()
    );
    Ok(())
}

/// Human-readable type name for loud argument errors.
fn value_type_name(v: &Value) -> &'static str {
    match v {
        Value::String(_) => "String",
        Value::Float(_) => "Float",
        Value::Bool(_) => "Bool",
        Value::List(_) => "List",
        Value::Unit => "Unit",
        Value::Vision(_) => "Vision",
        Value::Reflex(_) => "Reflex",
        _ => "non-string value",
    }
}

/// `vision_edit(handle, prompt) -> Vision`
///
/// **Not implemented** — R6 (naryad 214/215 territory: image editing).
pub(crate) fn builtin_vision_edit_stub(_args: &[Value]) -> Result<Value, String> {
    Err(
        "vision_edit: Vision editing is not implemented in this build \
         (naryad 210 skeleton; real implementation lands in R6, naryad 214/215, \
         per ADR-0122). This is a loud refusal, not a placeholder result."
            .to_string(),
    )
}

// ── Last-resort stubs for the registry (Наряд №240) ──────────────────
//
// `vision_generate` / `vision_list` / `vision_export` are intercepted by
// the interpreter (`invoke` + expression evaluation) and the VM
// (`call_builtin`) BEFORE the generic builtin fallback — the real dispatch
// functions above need backend-owned registry/declaration state that the
// stateless `fn(&[Value])` registry signature cannot carry (same structure
// as reflex_train/reflex_predict, `src/builtins/reflex.rs`). These stubs
// only fire if some third code path reaches the registry directly.

/// Last-resort stub — real path is the intercepted `vision_generate_dispatch`.
pub(crate) fn builtin_vision_generate_stub(_args: &[Value]) -> Result<Value, String> {
    Err(
        "vision_generate: reached the generic builtin registry — this builtin is \
         intercepted by the interpreter/VM dispatch (Наряд №240) which owns the \
         vision registry; direct registry calls are not supported (loud refusal)"
            .to_string(),
    )
}

/// Last-resort stub — real path is the intercepted `vision_list_dispatch`.
pub(crate) fn builtin_vision_list_stub(_args: &[Value]) -> Result<Value, String> {
    Err(
        "vision_list: reached the generic builtin registry — this builtin is \
         intercepted by the interpreter/VM dispatch (Наряд №240) which owns the \
         vision registry; direct registry calls are not supported (loud refusal)"
            .to_string(),
    )
}

/// Last-resort stub — real path is the intercepted `vision_export_dispatch`.
pub(crate) fn builtin_vision_export_stub(_args: &[Value]) -> Result<Value, String> {
    Err(
        "vision_export: reached the generic builtin registry — this builtin is \
         intercepted by the interpreter/VM dispatch (Наряд №240) which owns the \
         vision registry; direct registry calls are not supported (loud refusal)"
            .to_string(),
    )
}

/// Last-resort stub — real path is the intercepted `vision_export_raw_dispatch`
/// (Наряд №241 Block 2.1; same state-carrying pattern as `vision_export`).
pub(crate) fn builtin_vision_export_raw_stub(_args: &[Value]) -> Result<Value, String> {
    Err(
        "vision_export_raw: reached the generic builtin registry — this builtin is \
         intercepted by the interpreter/VM dispatch (Наряд №241) which owns the \
         vision registry; direct registry calls are not supported (loud refusal)"
            .to_string(),
    )
}

/// Last-resort stub — real path is the intercepted `vision_save_dispatch`
/// (Наряд №242 R6.1: state-carrying like `vision_export` PLUS the
/// program's SQLite connection, which the stateless `fn(&[Value])`
/// registry signature cannot carry). Supersedes the R1-era loud stub
/// that referenced the obsolete "naryad 214/215" numbering (R0-epoch).
pub(crate) fn builtin_vision_save_stub(_args: &[Value]) -> Result<Value, String> {
    Err(
        "vision_save: reached the generic builtin registry — this builtin is \
         intercepted by the interpreter/VM dispatch (Наряд №242) which owns the \
         vision registry and the database connection; direct registry calls are \
         not supported (loud refusal)"
            .to_string(),
    )
}

/// Last-resort stub — real path is the intercepted `vision_load_dispatch`
/// (Наряд №242 R6.1; same state-carrying pattern as `vision_save`).
/// Supersedes the R1-era loud stub that referenced the obsolete
/// "naryad 214/215" numbering (R0-epoch).
pub(crate) fn builtin_vision_load_stub(_args: &[Value]) -> Result<Value, String> {
    Err(
        "vision_load: reached the generic builtin registry — this builtin is \
         intercepted by the interpreter/VM dispatch (Наряд №242) which owns the \
         vision registry and the database connection; direct registry calls are \
         not supported (loud refusal)"
            .to_string(),
    )
}
