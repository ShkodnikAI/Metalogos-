//! OCR backend wiring (Naryad №407, wave 4.5; the donor pattern is
//! №334's `vision::understand`, ADR-0163 §2.1).
//!
//! The OCR class answers TEXT extraction FROM an image (the canon
//! `trocr-base-printed` wedge) — reading/understanding, NOT generation:
//! no Art. 50 synthetic marking on the output, and the №331/№332
//! handle/origin provenance discipline applies unchanged. The call
//! contract mirrors `vision_understand`: mock-first (deterministic
//! golden path), real mode refuses loudly unless the SHA-verified
//! weights are on disk (PARKED by hardware, №294 — production media is
//! not promised).

use crate::interpreter::values::Value;

/// `ocr_extract(image, lang?, model?)` — the OCR backend call (№407).
/// `image` is the image payload reference (String); `lang` is the
/// recognition language hint (e.g. "eng"); `model` defaults to the
/// registry canon `trocr-base-printed` (weights: trocr-base-printed,
/// microsoft/trocr-base-printed).
pub(crate) fn builtin_ocr_extract(args: &[Value]) -> Result<Value, String> {
    let fn_name = "ocr_extract";
    if args.is_empty() || args.len() > 3 {
        return Err(format!(
            "{}: expects 1..3 arguments (image, lang?, model?), got {}",
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
    let lang = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        None => String::new(),
        Some(other) => {
            return Err(format!(
                "{}: lang must be String, got {}",
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
    let weights_id = model.unwrap_or("trocr-base-printed").to_lowercase();
    let entry = crate::backends::find_by_weights_id(&weights_id).ok_or_else(|| {
        format!(
            "{}: model '{}' has no registry record (the №333 registry is the SSOT)",
            fn_name, weights_id
        )
    })?;
    if entry.class != crate::backends::BackendClass::Ocr {
        return Err(format!(
            "{}: backend '{}' is class '{}', not ocr",
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
        "[MOCK: ocr_extract | {} | {} | {}]",
        weights_id, image, lang
    )))
}
