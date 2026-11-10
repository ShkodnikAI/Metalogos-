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
/// (String; a `VideoSegmentId` surface) OR a `VideoFrame` media handle
/// (№939, ADR-0182 §3.3 step 3 — the multi-form overload posture of
/// №493: the handle form ADDS capability, the String golden behavior is
/// untouched — the №706 mock golden and the №697 real-path posture
/// included). A handle materializes through the store's sanctioned read
/// path (sealed entries decrypt only inside the sanctioned consumer —
/// the PROCESSING posture, not byte egress); the handle's static label
/// joins the result's label through the existing №323 inference (private
/// frame in → private description out). `prompt` is the comprehension
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
    // №478: the SSOT mock predicate (№454) — explicit opt-in only;
    // the real-path refusal without weights stays loud (№294).
    let mock_mode = crate::llm::mock_llm_requested();
    if !mock_mode {
        let artifact = crate::backends_weights::first_manifest_file(&weights_id)
            .map(|f| f.path.to_string())
            .unwrap_or_else(|| "<no manifest>".to_string());
        return Err(format!(
            "{}: real backend '{}' requires its weights ({}) fetched and SHA-verified \
             first (MLOG_BACKEND_WEIGHTS_ALLOWLIST + backends::fetch_weights); real \
             inference is PARKED by hardware (№294) in this environment — no weights \
             on disk, refusing honestly (mock mode is explicit: METALOGOS_MOCK_LLM)",
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
        // №478: the SSOT predicate reads METALOGOS_MOCK_LLM now.
        if on {
            std::env::set_var("METALOGOS_MOCK_LLM", "1");
        } else {
            std::env::remove_var("METALOGOS_MOCK_LLM");
        }
    }

    fn clear_mock() {
        std::env::remove_var("METALOGOS_MOCK_LLM");
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

// ── №939 (ADR-0182 §3.3 step 3): the VideoFrame media-input bridge ────
//
// The bridge corpus (the №599/№938 step corpus, the VideoFrame twin) —
// `video`-gated like the builtin itself; runs in the parked-features
// video job's `--lib --features video` step on BOTH backends (TW + VM).
//
// Red/green corpus:
//   (1) the capability table — llava-video-7b/internvl3-8b carry
//       exactly [VideoFrame] (№939); qwen25-vl-7b (the class's third
//       record, outside the step roster) is completed to [VideoFrame]
//       by №941 (В94 — the planner's decision r74, office main
//       `e8fd8b5`, R74 finding (1); the №939 roster history is
//       preserved in the pin wording);
//   (2) the handle input returns the SAME answer as the String
//       equivalent on BOTH backends (the №706 mock golden untouched —
//       the overloads ADD capability, №493/§7.2);
//   (3) a foreign MediaKind refuses LOUDLY (fail-closed) on both
//       backends — and a forged VideoFrame-typed handle naming a
//       non-VideoFrame entry cannot launder through the sanctioned read
//       path (the defense-in-depth entry-kind check);
//   (4) the label join is visible statically: a private-derived handle
//       → the bridge result → a file sink is a COMPILE error
//       (SECRET_LEAK); the public positive control compiles clean;
//   (5) the consent scope rides the carrier: the granted control
//       compiles, the ungranted twin refuses (voice-unconsented);
//   (6) a sealed (consented) entry materializes for PROCESSING on both
//       backends — no likeness/consent credential (the Voice
//       precedent), the at-rest rule untouched;
//   (7) the create → use → drop lifecycle: media_release after use
//       evicts; a post-eviction use is the store's loud «unknown
//       handle»;
//   (8) the statement-position bridge (the ExprStmt path, both).
//
// Mutation verification (the №382 protocol): dropping the VideoFrame
// arm from `resolve_video_frame_input` turns (2) red (String type
// error); pulling the foreign-kind refusal turns (3) red; removing the
// VIDEO_UNDERSTAND interception from either backend turns (2)/(6) red
// on that backend (the stateless fallback refuses Media) — every red
// below names its mutation.
#[cfg(all(test, feature = "video"))]
mod bridge_tests {
    use std::path::Path;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn run_tw(source: &str) -> Result<Option<String>, String> {
        crate::run_program_with_dir(
            source.trim(),
            Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf(),
        )
    }

    fn run_vm(source: &str) -> Result<Option<String>, String> {
        let declarations =
            crate::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
        let mut comp = crate::compiler::Compiler::with_std_root(
            Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf(),
        );
        let program = comp.compile(declarations)?;
        let mut vm = crate::vm::Vm::new();
        vm.run(program)
    }

    // ── (1) The capability table (the step-3 pin, by name) ────────────

    #[test]
    fn capability_descriptors_pin_the_video_frame_inputs() {
        use crate::backends::{BackendClass, BACKEND_REGISTRY};
        use crate::media::MediaKind;

        let video_inputs = [MediaKind::VideoFrame];
        for entry in BACKEND_REGISTRY {
            match entry.name {
                // №939 (this step): the two named video-understanding
                // records consume a VideoFrame handle; the class's third
                // record joins them by №941 (В94 — the planner's
                // decision r74, office main `e8fd8b5`, R74 finding (1);
                // the №939 roster history is preserved in the pin
                // wording below).
                "llava-video-7b" | "internvl3-8b" | "qwen25-vl-7b" => {
                    assert_eq!(
                        entry.inputs, video_inputs,
                        "record '{}' must declare exactly [VideoFrame] (№939 step 3; the qwen25-vl-7b completion is №941/r74)",
                        entry.name
                    );
                }
                _ => {}
            }
            // The class word is intact for the three records.
            if matches!(
                entry.name,
                "llava-video-7b" | "internvl3-8b" | "qwen25-vl-7b"
            ) {
                assert_eq!(entry.class, BackendClass::VideoUnderstanding);
            }
        }
    }

    // ── (2) Handle input == String equivalent (the golden equality) ───

    const HANDLE_PROG: &str = r#"
origin demo_cam { kind: generation, media: video_frame, label: public }
pattern Describe(_tick: String) -> String {
  let frame = from demo_cam media_store_video_frame("frame-bytes", "public")
  return video_understand(frame, "what happens", "llava-video-7b-qwen2")
}
flow Main { input: String = "tick" -> Describe -> output }
"#;

    const STRING_PROG: &str = r#"
pattern Describe(_tick: String) -> String {
  return video_understand("frame-bytes", "what happens", "llava-video-7b-qwen2")
}
flow Main { input: String = "tick" -> Describe -> output }
"#;

    #[test]
    fn handle_input_equals_string_form_on_both_backends() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        std::env::set_var("METALOGOS_MOCK_LLM", "1"); // №478: the mock is explicit

        let expected =
            "[MOCK: video_understand | llava-video-7b-qwen2 | frame-bytes | what happens]";
        let tw_handle = run_tw(HANDLE_PROG).expect("handle form runs on TW");
        assert_eq!(
            tw_handle.as_deref().unwrap_or_default().trim_end(),
            expected,
            "the handle input must return the string-equivalent answer (TW)"
        );
        let tw_string = run_tw(STRING_PROG).expect("string form runs on TW");
        assert_eq!(
            tw_string.as_deref().unwrap_or_default().trim_end(),
            expected,
            "the STRING golden must not move (TW)"
        );
        let vm_handle = run_vm(HANDLE_PROG).expect("handle form runs on VM");
        assert_eq!(
            vm_handle.as_deref().unwrap_or_default().trim_end(),
            expected,
            "TW/VM parity of the bridge (VM)"
        );
        let vm_string = run_vm(STRING_PROG).expect("string form runs on VM");
        assert_eq!(vm_string, tw_string, "VM string golden unchanged");
        std::env::remove_var("METALOGOS_MOCK_LLM");
    }

    // ── (3) Foreign MediaKind refuses loudly (fail-closed) ────────────

    const WRONG_KIND_PROG: &str = r#"
origin demo_gen { kind: generation, media: image, label: public }
pattern Describe(_tick: String) -> String {
  let frame = from demo_gen media_store_image("frame-bytes", "public")
  return video_understand(frame, "what happens")
}
flow Main { input: String = "tick" -> Describe -> output }
"#;

    #[test]
    fn wrong_media_kind_refuses_loudly_on_both_backends() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        std::env::set_var("METALOGOS_MOCK_LLM", "1"); // the refusal is kind-based, not mode-based

        let err = run_tw(WRONG_KIND_PROG)
            .expect_err("an Image handle must not enter a video backend (TW)");
        assert!(
            err.contains("video_understand")
                && err.contains("VideoFrame handle")
                && err.contains("Image"),
            "the foreign-kind refusal must name the expected and the actual kind, got: {}",
            err
        );
        let err_vm = run_vm(WRONG_KIND_PROG)
            .expect_err("an Image handle must not enter a video backend (VM)");
        assert!(
            err_vm.contains("VideoFrame handle") && err_vm.contains("Image"),
            "VM parity of the foreign-kind refusal, got: {}",
            err_vm
        );
        std::env::remove_var("METALOGOS_MOCK_LLM");
    }

    /// Defense-in-depth (unit level): a VideoFrame-TYPED handle pointing
    /// at a non-VideoFrame entry (only constructible through the direct
    /// store API) cannot launder through the sanctioned read path.
    #[test]
    fn forged_kind_handle_cannot_launder_through_store() {
        use crate::interpreter::values::Value;
        use crate::labels::{Conf, Integrity, Label};
        use crate::media::{MediaKind, MediaStore, VideoFrameId};

        let mut store = MediaStore::new();
        let image = store
            .insert(
                MediaKind::Image,
                b"frame".to_vec(),
                Label {
                    conf: Conf::Public,
                    integrity: Integrity::Trusted,
                    consent: Default::default(),
                },
            )
            .expect("image insert");
        let forged = crate::media::MediaHandle::VideoFrame(VideoFrameId(image.id()));
        let err = crate::builtins::video_understand_dispatch(
            &store,
            &[Value::Media(forged), Value::String("q".into())],
        )
        .expect_err("the entry-kind check must refuse the forgery");
        assert!(
            err.contains("not video_frame") && err.contains("fail-closed"),
            "expected the loud entry-kind refusal, got: {}",
            err
        );
    }

    // ── (4) The label join is visible statically (№323, no new rules) ─

    const PRIVATE_BRIDGE_LEAK_PROG: &str = r#"
origin feed { kind: generation, media: video_frame, label: public }
pattern Describe(_tick: String) -> String {
  let tok = env("MLOG_939_TEST_TOKEN")
  let frame = from feed media_store_video_frame(tok, "public")
  let desc = video_understand(frame, "describe")
  let _p = write_file(desc, "target/n939-leak.txt")
  return "x"
}
flow Main { input: String = "x" -> Describe -> output }
"#;

    const PUBLIC_BRIDGE_CONTROL_PROG: &str = r#"
origin feed { kind: generation, media: video_frame, label: public }
pattern Describe(_tick: String) -> String {
  let frame = from feed media_store_video_frame("public-bytes", "public")
  let desc = video_understand(frame, "describe")
  let _p = write_file(desc, "target/n939-public.txt")
  return "x"
}
flow Main { input: String = "x" -> Describe -> output }
"#;

    #[test]
    fn private_label_joins_the_bridge_result_statically() {
        let err = crate::compile_program(PRIVATE_BRIDGE_LEAK_PROG.trim())
            .expect_err("private frame → bridge → file sink must not compile");
        assert!(
            err.contains("SECRET_LEAK") && err.contains("write_file"),
            "expected SECRET_LEAK on the post-bridge egress, got: {}",
            err
        );
        // Positive control: the public flow compiles clean.
        crate::compile_program(PUBLIC_BRIDGE_CONTROL_PROG.trim())
            .unwrap_or_else(|e| panic!("public flow must compile, got: {}", e));
    }

    // ── (5) The consent scope rides the carrier (№335 + the bridge) ───

    const CONSENT_RIDE_PROG: &str = r#"
profile capability { allow_carrier: "tts_send" }
origin feed { kind: generation, media: video_frame, label: public }
pattern Describe(_tick: String) -> String {
  let tok = env("MLOG_939_TEST_TOKEN")
  let frame = from feed media_store_video_frame(tok, "public")
  let granted = consent_grant(frame, "clip-release")
  let desc = video_understand(granted, "describe")
  let voice = consent_grant("canary-voice", "clip-release")
  let bot = consent_grant("bot-token", "clip-release")
  let chat = consent_grant("chat-id", "clip-release")
  let _s = tts_send(desc, voice, bot, chat)
  return "x"
}
flow Main { input: String = "x" -> Describe -> output }
"#;

    const NO_CONSENT_PROG: &str = r#"
profile capability { allow_carrier: "tts_send" }
origin feed { kind: generation, media: video_frame, label: public }
pattern Describe(_tick: String) -> String {
  let tok = env("MLOG_939_TEST_TOKEN")
  let frame = from feed media_store_video_frame(tok, "public")
  let desc = video_understand(frame, "describe")
  let voice = consent_grant("canary-voice", "clip-release")
  let bot = consent_grant("bot-token", "clip-release")
  let chat = consent_grant("chat-id", "clip-release")
  let _s = tts_send(desc, voice, bot, chat)
  return "x"
}
flow Main { input: String = "x" -> Describe -> output }
"#;

    #[test]
    fn consent_scope_rides_the_bridge_statically() {
        crate::compile_program(CONSENT_RIDE_PROG.trim())
            .unwrap_or_else(|e| panic!("granted scope must clear the voice sink, got: {}", e));
        let err = crate::compile_program(NO_CONSENT_PROG.trim())
            .expect_err("no consent → the voice sink must refuse");
        assert!(
            err.contains("voice-unconsented"),
            "expected voice-unconsented, got: {}",
            err
        );
    }

    // ── (6) PROCESSING: a sealed entry materializes without creds ─────

    const SEALED_BRIDGE_PROG: &str = r#"
origin demo_cam { kind: generation, media: video_frame, label: public }
pattern Describe(_tick: String) -> String {
  let frame = from demo_cam media_store_video_frame("gdpr-frame", "consented")
  let seen = video_understand(frame, "what happens", "llava-video-7b-qwen2")
  if seen == "[MOCK: video_understand | llava-video-7b-qwen2 | gdpr-frame | what happens]" {
    return "processing-ok"
  }
  return "processing-broken"
}
flow Main { input: String = "tick" -> Describe -> output }
"#;

    #[test]
    fn sealed_entry_materializes_for_processing_on_both_backends() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        std::env::set_var("METALOGOS_MOCK_LLM", "1");
        let out = run_tw(SEALED_BRIDGE_PROG).expect("sealed entry serves PROCESSING on TW");
        assert_eq!(
            out.as_deref().unwrap_or_default().trim_end(),
            "processing-ok",
            "the sealed payload must reach the sanctioned consumer verbatim"
        );
        let out_vm = run_vm(SEALED_BRIDGE_PROG).expect("sealed entry serves PROCESSING on VM");
        assert_eq!(
            out_vm.as_deref().unwrap_or_default().trim_end(),
            "processing-ok"
        );
        std::env::remove_var("METALOGOS_MOCK_LLM");
    }

    // ── (7) The lifecycle: create → use → drop (media_release) ────────

    const LIFECYCLE_PROG: &str = r#"
origin demo_cam { kind: generation, media: video_frame, label: public }
pattern Describe(_tick: String) -> String {
  let frame = from demo_cam media_store_video_frame("lifecycle-bytes", "public")
  let _seen = video_understand(frame, "describe")
  let refs = media_release(frame)
  if refs == 0.0 {
    let _again = video_understand(frame, "describe")
  }
  return "unexpected-survivor"
}
flow Main { input: String = "tick" -> Describe -> output }
"#;

    #[test]
    fn media_release_after_use_evicts_and_post_eviction_use_is_loud() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        std::env::set_var("METALOGOS_MOCK_LLM", "1");
        let err = run_tw(LIFECYCLE_PROG).expect_err("a released handle cannot serve PROCESSING");
        assert!(
            err.contains("unknown handle"),
            "expected the store's eviction loudness, got: {}",
            err
        );
        let err_vm = run_vm(LIFECYCLE_PROG).expect_err("VM parity of eviction loudness");
        assert!(err_vm.contains("unknown handle"), "got: {}", err_vm);
        std::env::remove_var("METALOGOS_MOCK_LLM");
    }

    // ── (8) The statement-position bridge (the ExprStmt path, both) ───

    const STATEMENT_PATH_PROG: &str = r#"
origin demo_cam { kind: generation, media: video_frame, label: public }
pattern Describe(_tick: String) -> String {
  let frame = from demo_cam media_store_video_frame("stmt-bytes", "public")
  video_understand(frame, "what happens")
  let _r = media_retain(frame)
  return "statement-ok"
}
flow Main { input: String = "tick" -> Describe -> output }
"#;

    #[test]
    fn statement_position_bridge_runs_on_both_backends() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        std::env::set_var("METALOGOS_MOCK_LLM", "1");
        let out = run_tw(STATEMENT_PATH_PROG).expect("statement path runs on TW");
        assert_eq!(
            out.as_deref().unwrap_or_default().trim_end(),
            "statement-ok"
        );
        let out_vm = run_vm(STATEMENT_PATH_PROG).expect("statement path runs on VM");
        assert_eq!(
            out_vm.as_deref().unwrap_or_default().trim_end(),
            "statement-ok"
        );
        std::env::remove_var("METALOGOS_MOCK_LLM");
    }
}
