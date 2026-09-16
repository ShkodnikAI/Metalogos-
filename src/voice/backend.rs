//! STT / omni backend wiring (Наряд №334, ADR-0163 §2.1 + §2.4).
//!
//! The call contract is mock-first and NEVER silent:
//!   - `METALOGOS_LLM_MOCK` unset/true (the default, №4 posture):
//!     a DETERMINISTIC mock result — the golden-test path;
//!   - `METALOGOS_LLM_MOCK=false/0` (the REAL path): refuses LOUDLY
//!     unless the backend's weights are on disk and hash-verified
//!     against the №334 manifest (`backends_weights::weights_loaded`).
//!     Real INFERENCE is PARKED by hardware (№294) in this environment:
//!     the wired path is the turnkey piece — loader + registry + pins —
//!     and the refusal names exactly what is missing. Production media
//!     is not promised (the loud boundary).

use crate::interpreter::values::Value;

/// The mock-mode flag (the №4 contour: default ON).
pub fn mock_mode() -> bool {
    std::env::var("METALOGOS_LLM_MOCK")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(true)
}

fn model_weights_id(model: Option<&str>, default: &str) -> String {
    model.unwrap_or(default).to_lowercase()
}

/// The loud real-mode refusal: names the backend, the weights id, the
/// missing artifact and the PARKED boundary (№294). Never a silent
/// mock-substitution in real mode — that would be a lie.
fn real_mode_refusal(fn_name: &str, weights_id: &str) -> String {
    let artifact = crate::backends_weights::first_manifest_file(weights_id)
        .map(|f| f.path.to_string())
        .unwrap_or_else(|| "<no manifest>".to_string());
    format!(
        "{}: real backend '{}' requires its weights ({}) fetched and SHA-verified \
         first (MLOG_BACKEND_WEIGHTS_ALLOWLIST + backends::fetch_weights); real \
         inference is PARKED by hardware (№294) in this environment — no weights \
         on disk, refusing honestly (mock mode is explicit: METALOGOS_LLM_MOCK)",
        fn_name, weights_id, artifact
    )
}

/// `stt_transcribe(audio, model?)` — the STT-class backend call (№334).
/// `audio` is the audio payload reference (String); `model` defaults to
/// the registry canon `whisper-turbo` (weights: whisper-large-v3-turbo).
pub(crate) fn builtin_stt_transcribe(args: &[Value]) -> Result<Value, String> {
    let fn_name = "stt_transcribe";
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "{}: expects 1 or 2 arguments (audio, model?), got {}",
            fn_name,
            args.len()
        ));
    }
    let audio = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "{}: audio must be String, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    let model = match args.get(1) {
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
    let weights_id = model_weights_id(model, "whisper-large-v3-turbo");
    let entry = crate::backends::find_by_weights_id(&weights_id).ok_or_else(|| {
        format!(
            "{}: model '{}' has no registry record (the №333 registry is the SSOT)",
            fn_name, weights_id
        )
    })?;
    if entry.class != crate::backends::BackendClass::Stt {
        return Err(format!(
            "{}: backend '{}' is class '{}', not stt",
            fn_name,
            entry.name,
            entry.class.as_str()
        ));
    }
    if !mock_mode() {
        return Err(real_mode_refusal(fn_name, &weights_id));
    }
    // Deterministic mock: the golden contract (no randomness, no time).
    Ok(Value::String(format!(
        "[MOCK: stt_transcribe | {} | {}]",
        weights_id, audio
    )))
}

/// `omni_ask(prompt, media?, model?)` — the omni-class backend call
/// (№334): a prompt with an OPTIONAL media payload reference (String).
/// Defaults to the registry canon `nemotron-omni`.
pub(crate) fn builtin_omni_ask(args: &[Value]) -> Result<Value, String> {
    let fn_name = "omni_ask";
    if args.is_empty() || args.len() > 3 {
        return Err(format!(
            "{}: expects 1..3 arguments (prompt, media?, model?), got {}",
            fn_name,
            args.len()
        ));
    }
    let prompt = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "{}: prompt must be String, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    let media = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        None => String::new(),
        Some(other) => {
            return Err(format!(
                "{}: media must be String, got {}",
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
    let weights_id = model_weights_id(model, "nemotron-3-nano-omni-30b-a3b");
    let entry = crate::backends::find_by_weights_id(&weights_id).ok_or_else(|| {
        format!(
            "{}: model '{}' has no registry record (the №333 registry is the SSOT)",
            fn_name, weights_id
        )
    })?;
    if entry.class != crate::backends::BackendClass::Omni {
        return Err(format!(
            "{}: backend '{}' is class '{}', not omni",
            fn_name,
            entry.name,
            entry.class.as_str()
        ));
    }
    if !mock_mode() {
        return Err(real_mode_refusal(fn_name, &weights_id));
    }
    // Deterministic mock: the golden contract (no randomness, no time).
    // The optional media payload is part of the call contract and is
    // echoed into the deterministic result (empty when absent).
    Ok(Value::String(format!(
        "[MOCK: omni_ask | {} | {} | {}]",
        weights_id, prompt, media
    )))
}
