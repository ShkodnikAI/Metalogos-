#![cfg(feature = "video")]
//! Video-understanding backend wiring (Naryad №408, wave 4.5; the donor
//! pattern is №407's `vision::ocr`, which mirrors №334's
//! `vision::understand`, ADR-0163 §2.1).
//!
//! The video-understanding class answers questions ABOUT a video segment
//! (comprehension) — reading/understanding, NOT generation: no Art. 50
//! synthetic marking on the output, and the №331/№332 handle/origin
//! provenance discipline applies unchanged. The call contract mirrors
//! `vision_understand`: mock-first (deterministic golden path), real mode
//! refuses loudly unless the SHA-verified weights are on disk (PARKED by
//! hardware, №294 — production media is not promised).
//!
//! REAL-MODE FRAME-SAMPLING POLICY (fixed in advance so the contract is
//! reproducible, №408 task 3): frames are sampled deterministically — a
//! fixed stride over the segment's frame table plus the first and last
//! frames as anchors; NO randomness and NO wall-clock time participate in
//! the selection, so the same segment always yields the same frame set
//! and therefore the same answer for the same weights and prompt.

use crate::interpreter::values::Value;

/// `video_understand(segment, prompt?, model?)` — the video-understanding
/// backend call (№408). `segment` is the segment payload reference
/// (String; a `VideoSegmentId` surface); `prompt` is the comprehension
/// question (e.g. "what happens in this clip"); `model` defaults to the
/// registry canon `qwen2.5-vl-7b-instruct` (weights: qwen2.5-vl-7b-instruct,
/// Qwen/Qwen2.5-VL-7B-Instruct).
pub(crate) fn builtin_video_understand(args: &[Value]) -> Result<Value, String> {
    let fn_name = "video_understand";
    if args.is_empty() || args.len() > 3 {
        return Err(format!(
            "{}: expects 1..3 arguments (segment, prompt?, model?), got {}",
            fn_name,
            args.len()
        ));
    }
    let segment = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "{}: segment must be String, got {}",
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
    let weights_id = model.unwrap_or("qwen2.5-vl-7b-instruct").to_lowercase();
    let entry = crate::backends::find_by_weights_id(&weights_id).ok_or_else(|| {
        format!(
            "{}: model '{}' has no registry record (the №333 registry is the SSOT)",
            fn_name, weights_id
        )
    })?;
    if entry.class != crate::backends::BackendClass::VideoUnderstanding {
        return Err(format!(
            "{}: backend '{}' is class '{}', not video-understanding",
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
        "[MOCK: video_understand | {} | {} | {}]",
        weights_id, segment, prompt
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mock flag is process-global: serialize and pin it explicitly.
    static MOCK_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn pin_mock(on: bool) {
        std::env::set_var("METALOGOS_LLM_MOCK", if on { "1" } else { "0" });
    }

    fn clear_mock() {
        std::env::remove_var("METALOGOS_LLM_MOCK");
    }

    const SEGMENT: &str = "video:segment#1";

    #[test]
    fn mock_golden_is_deterministic_full_arity() {
        let _g = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        pin_mock(true);
        let out = builtin_video_understand(&[
            Value::String(SEGMENT.into()),
            Value::String("what happens here".into()),
            Value::String("qwen2.5-vl-7b-instruct".into()),
        ])
        .expect("mock call succeeds");
        match out {
            Value::String(s) => {
                assert_eq!(
                    s,
                    "[MOCK: video_understand | qwen2.5-vl-7b-instruct | video:segment#1 | what happens here]"
                );
            }
            other => panic!("expected String, got {}", other.type_name()),
        }
        // Determinism: the same call twice, byte-identical.
        let again = builtin_video_understand(&[
            Value::String(SEGMENT.into()),
            Value::String("what happens here".into()),
        ])
        .expect("second call succeeds");
        let default_model = builtin_video_understand(&[Value::String(SEGMENT.into())])
            .expect("default-model call succeeds");
        // The default model is the registry canon.
        match (again, default_model) {
            (Value::String(a), Value::String(d)) => {
                assert!(a.contains("| qwen2.5-vl-7b-instruct |"));
                assert!(d.contains("| qwen2.5-vl-7b-instruct |"));
            }
            _ => panic!("String expected"),
        }
        clear_mock();
    }

    #[test]
    fn arity_and_argument_types_are_loud() {
        let _g = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        pin_mock(true);
        let err = builtin_video_understand(&[]).unwrap_err();
        assert!(err.contains("expects 1..3 arguments"), "{err}");
        let err = builtin_video_understand(&[Value::Unit]).unwrap_err();
        assert!(err.contains("segment must be String"), "{err}");
        let err = builtin_video_understand(&[Value::String(SEGMENT.into()), Value::Bool(true)])
            .unwrap_err();
        assert!(err.contains("prompt must be String"), "{err}");
        clear_mock();
    }

    #[test]
    fn unknown_model_refuses_with_registry_pointer() {
        let _g = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        pin_mock(true);
        let err = builtin_video_understand(&[
            Value::String(SEGMENT.into()),
            Value::String("q".into()),
            Value::String("no-such-model-408".into()),
        ])
        .unwrap_err();
        assert!(err.contains("no registry record"), "{err}");
        clear_mock();
    }

    #[test]
    fn class_mismatch_refuses_loudly() {
        let _g = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        pin_mock(true);
        // trocr-base-printed is class ocr — a foreign class for a
        // video-understanding call.
        let err = builtin_video_understand(&[
            Value::String(SEGMENT.into()),
            Value::String("q".into()),
            Value::String("trocr-base-printed".into()),
        ])
        .unwrap_err();
        assert!(
            err.contains("is class 'ocr', not video-understanding"),
            "{err}"
        );
        clear_mock();
    }

    #[test]
    fn real_mode_refuses_naming_the_expected_weights_artifact() {
        let _g = MOCK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        pin_mock(false);
        let err = builtin_video_understand(&[Value::String(SEGMENT.into())]).unwrap_err();
        assert!(
            err.contains("PARKED by hardware (№294)")
                && err.contains("model-00001-of-00005.safetensors"),
            "the refusal names the weights artifact: {err}"
        );
        clear_mock();
    }

    #[test]
    fn registry_class_and_pins_are_canon() {
        // The three canon donors: class video-understanding, Osi license,
        // REAL pins (not PendingNo334), manifests present.
        for id in [
            "qwen2.5-vl-7b-instruct",
            "llava-video-7b-qwen2",
            "internvl3-8b",
        ] {
            let e = crate::backends::find_by_weights_id(id).unwrap_or_else(|| panic!("{id}"));
            assert_eq!(e.class.as_str(), "video-understanding", "{id}");
            assert!(
                matches!(e.pin, crate::backends::ShaPin::Pinned(_)),
                "{id}: the pin is real"
            );
            assert_eq!(e.license, crate::backends::LicenseClass::Osi, "{id}");
            let src = crate::backends::WEIGHTS_SOURCES
                .iter()
                .find(|(k, _)| *k == id)
                .map(|(_, v)| v)
                .unwrap_or_else(|| panic!("{id} manifest"));
            assert!(!src.files.is_empty(), "{id}: per-shard manifest present");
            // The registry pin IS the primary (first) shard's hash.
            match e.pin {
                crate::backends::ShaPin::Pinned(sha) => {
                    assert_eq!(sha, src.files[0].sha256, "{id}: pin == primary shard");
                }
                _ => unreachable!(),
            }
        }
        // The class word round-trips through the ladder's parser.
        let c = crate::backends::BackendClass::parse("video-understanding")
            .expect("video-understanding parses");
        assert_eq!(c.as_str(), "video-understanding");
    }
}
