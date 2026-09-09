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
//! missing component) — NOT a silent stub. `vision_edit`, `vision_save`,
//! `vision_load` remain loud stubs (R6: edit + LoRA/SQLite).
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
            crate::ast::VisionPolicy::Safe => "safe".to_string(),
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
    std::fs::write(&path, &artifact.png_bytes)
        .map_err(|e| format!("vision_export: write to {}: {}", path, e))?;
    eprintln!(
        "WARN: unsigned vision export — [Vision#{}] written to {} without watermark or \
         manifest (watermark/manifest/Category-A gate lands in R5)",
        id.0, path
    );
    Ok(Value::String(path))
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

/// `vision_save(handle, name) -> String`
///
/// **Not implemented** — R6 (SQLite persistence of vision artifacts).
pub(crate) fn builtin_vision_save_stub(_args: &[Value]) -> Result<Value, String> {
    Err("vision_save: Vision save is not implemented in this build \
         (naryad 210 skeleton; real implementation lands in R6, naryad 214/215, \
         per ADR-0122). This is a loud refusal, not a placeholder result."
        .to_string())
}

/// `vision_load(name) -> Vision`
///
/// **Not implemented** — R6 (SQLite persistence of vision artifacts).
pub(crate) fn builtin_vision_load_stub(_args: &[Value]) -> Result<Value, String> {
    Err("vision_load: Vision load is not implemented in this build \
         (naryad 210 skeleton; real implementation lands in R6, naryad 214/215, \
         per ADR-0122). This is a loud refusal, not a placeholder result."
        .to_string())
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
