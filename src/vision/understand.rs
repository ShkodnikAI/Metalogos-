//! Vision-understanding backend wiring (Наряд №334, ADR-0163 §2.1).
//!
//! The understanding class answers questions ABOUT an image (molmoact2
//! canon) — distinct from generation (`vision_generate`, №213) and from
//! the handle/origin layer (№331/№332). The call contract mirrors
//! `voice::backend`: mock-first (deterministic golden path), real mode
//! refuses loudly unless the SHA-verified weights are on disk (PARKED by
//! hardware, №294 — production media is not promised).

use crate::interpreter::values::Value;

/// `vision_understand(image, prompt?, model?)` — the vision-understanding
/// backend call (№334). `image` is the image payload reference (String);
/// `prompt` is the question about the image; `model` defaults to the
/// registry canon `molmoact2` (weights: molmoact2, allenai/MolmoAct2).
pub(crate) fn builtin_vision_understand(args: &[Value]) -> Result<Value, String> {
    let fn_name = "vision_understand";
    if args.is_empty() || args.len() > 3 {
        return Err(format!(
            "{}: expects 1..3 arguments (image, prompt?, model?), got {}",
            fn_name,
            args.len()
        ));
    }
    let image = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "{}: image must be String, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    let prompt = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        None => String::new(),
        Some(other) => {
            return Err(format!(
                "{}: prompt must be String, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    let model = match args.get(2) {
        Some(Value::String(s)) => Some(s.as_str()),
        None => None,
        Some(other) => {
            return Err(format!(
                "{}: model must be String, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    let weights_id = model.unwrap_or("molmoact2").to_lowercase();
    let entry = crate::backends::find_by_weights_id(&weights_id).ok_or_else(|| {
        format!(
            "{}: model '{}' has no registry record (the №333 registry is the SSOT)",
            fn_name, weights_id
        )
    })?;
    if entry.class != crate::backends::BackendClass::VisionUnderstanding {
        return Err(format!(
            "{}: backend '{}' is class '{}', not vision-understanding",
            fn_name,
            entry.name,
            entry.class.as_str()
        ));
    }
    let mock_mode = std::env::var("METALOGOS_LLM_MOCK")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(true);
    if !mock_mode {
        let artifact = crate::backends_weights::first_manifest_file(&weights_id)
            .map(|f| f.path.to_string())
            .unwrap_or_else(|| "<no manifest>".to_string());
        return Err(format!(
            "{}: real backend '{}' requires its weights ({}) fetched and SHA-verified \
             first (MLOG_BACKEND_WEIGHTS_ALLOWLIST + backends::fetch_weights); real \
             inference is PARKED by hardware (№294) in this environment — no weights \
             on disk, refusing honestly (mock mode is explicit: METALOGOS_LLM_MOCK)",
            fn_name, weights_id, artifact
        ));
    }
    // Deterministic mock: the golden contract (no randomness, no time).
    Ok(Value::String(format!(
        "[MOCK: vision_understand | {} | {} | {}]",
        weights_id, image, prompt
    )))
}
