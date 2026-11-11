// Naryad #475 (issue #723): the fs_gate ratchet (clippy disallowed-methods)
// targets PRODUCTION I/O paths. This test file exercises the REAL filesystem
// for fixtures and assertions by design — the allow is scoped to this file.
#![allow(clippy::disallowed_methods)]

// ── Наряд №938 (P2, media/voice/process; issue #1889; ADR-0182 §3.3.2) ─
//
// The Audio media-input bridge (the Image bridge step 2 of 3):
// `stt_transcribe` and `omni_ask`'s optional media argument accept
// `Value::Media(MediaHandle::Audio(_))` — materialized through the
// store's sanctioned read path; the capability descriptors land in the
// backend registry (whisper-turbo / wall-oss / nemotron-omni); the
// label-join rides the existing №323 inference; TW/VM parity holds (one
// shared dispatch per builtin).
//
// Red/green corpus (the №599 step-1 corpus, the Audio twin):
//   (1) the capability table — Stt/Omni records carry exactly [Audio]
//       (№938); the step-1 records keep [Image]; the video-understanding
//       records carry [VideoFrame, VideoSegment] (№939; the
//       qwen25-vl-7b completion is №941/r74; the consumer-path step is
//       №945/ADR-0182 §7.3); every other record stays text-only (drift
//       = failure);
//   (2) the handle input returns the SAME answer as the String
//       equivalent on BOTH backends (the golden string surface is
//       untouched — the overloads ADD capability, №493/§7.2);
//   (3) a foreign MediaKind refuses LOUDLY (fail-closed) on both
//       backends — and a forged Audio-typed handle naming a non-Audio
//       entry cannot launder through the sanctioned read path (the
//       defense-in-depth entry-kind check);
//   (4) the label join is visible statically: a private-derived handle
//       → the bridge result → a file sink is a COMPILE error
//       (private-egress/SECRET_LEAK); the public positive control
//       compiles clean;
//   (5) the consent scope rides the carrier: consent_grant before the
//       bridge makes the result voice-egress-eligible (compiles);
//       without the grant the voice sink refuses (voice-unconsented);
//   (6) a sealed (consented) entry materializes for PROCESSING on both
//       backends — no likeness/consent credential is needed for the
//       backend call (the Voice precedent), the at-rest rule untouched;
//   (7) the create → use → drop lifecycle: media_release after use
//       evicts the entry; a post-eviction use is the store's loud
//       "unknown handle";
//   (8) the statement-position bridge (the ExprStmt path, both);
//   (9) the omni model argument still selects (wall-oss) — the optional
//       media handle form composes with the explicit model form.
//
// Mutation verification (the №382 protocol): dropping the Audio arm
// from `resolve_stt_audio_input`/`resolve_omni_media_input` turns (2)
// red (String type error); pulling the foreign-kind refusal turns (3)
// red; removing the STT_TRANSCRIBE/OMNI_ASK interception from either
// backend turns (2)/(6) red on that backend (the stateless fallback
// refuses Media) — every red below names its mutation.

use std::path::Path;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(
        source.trim(),
        Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf(),
    )
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(
        Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf(),
    );
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

// ── (1) The capability table (ADR-0182 §3.2 — the schema pin) ─────────

#[test]
fn capability_descriptors_pin_the_audio_inputs() {
    use metalogos::backends::{BackendClass, BACKEND_REGISTRY};
    use metalogos::media::MediaKind;

    let audio_inputs = [MediaKind::Audio];
    let image_inputs = [MediaKind::Image];
    let video_inputs = [MediaKind::VideoFrame, MediaKind::VideoSegment];
    for entry in BACKEND_REGISTRY {
        // №939 (ADR-0182 §3.3 step 3): the two named video-understanding
        // records consume a VideoFrame handle; the class's third record,
        // qwen25-vl-7b, is completed by №941 (В94 — the planner's
        // decision r74, office main `e8fd8b5`, R74 finding (1): the В93
        // honest tail — the naryads' fact-bases called it an [Image]
        // holder since №599, the live tree kept it text-only per the
        // №939 roster, the completion landed as №941; the №939 roster
        // history is preserved in that wording).
        if matches!(
            entry.name,
            "llava-video-7b" | "internvl3-8b" | "qwen25-vl-7b"
        ) {
            assert_eq!(
                entry.inputs, video_inputs,
                "record '{}' must declare exactly [VideoFrame, VideoSegment] (№939 step 3 + №941/r74; the consumer-path step is №945/ADR-0182 §7.3)",
                entry.name
            );
            continue;
        }
        match entry.class {
            // №938 (this step): the STT/Omni records consume Audio.
            BackendClass::Stt | BackendClass::Omni => {
                assert_eq!(
                    entry.inputs,
                    audio_inputs,
                    "record '{}' ({}) must declare exactly [Audio] (№938 step 2)",
                    entry.name,
                    entry.class.as_str()
                );
            }
            // №599 step 1: the image-understanding records keep [Image].
            BackendClass::VisionUnderstanding | BackendClass::Ocr => {
                assert_eq!(
                    entry.inputs,
                    image_inputs,
                    "record '{}' ({}) must declare exactly [Image] (№599 step 1)",
                    entry.name,
                    entry.class.as_str()
                );
            }
            // The №939 step (VideoFrame) and the №945 consumer-path
            // step (VideoSegment) carry the named-roster arm above;
            // every other record stays text-only.
            _ => {
                assert!(
                    entry.inputs.is_empty(),
                    "record '{}' ({}) declares media inputs {:?} — the ADR-0182 line fills them per-step, not in bulk",
                    entry.name,
                    entry.class.as_str(),
                    entry.inputs
                );
            }
        }
    }
    // The three canon records of step 2 are named (the SSOT witnesses).
    for weights_id in [
        "whisper-large-v3-turbo",
        "wall-oss-0.5",
        "nemotron-3-nano-omni-30b-a3b",
    ] {
        let entry = metalogos::backends::find_by_weights_id(weights_id)
            .unwrap_or_else(|| panic!("record {weights_id} exists"));
        assert_eq!(entry.inputs, audio_inputs, "{weights_id} consumes Audio");
    }
}

// ── (2) Handle input == String equivalent (the golden equality) ───────

const STT_HANDLE_PROG: &str = r#"
origin demo_feed { kind: generation, media: audio, label: public }
pattern Transcribe(_tick: String) -> String {
  let clip = from demo_feed media_store_audio("clip-bytes", "public")
  return stt_transcribe(clip)
}
flow Main { input: String = "tick" -> Transcribe -> output }
"#;

const STT_STRING_PROG: &str = r#"
pattern Transcribe(_tick: String) -> String {
  return stt_transcribe("clip-bytes")
}
flow Main { input: String = "tick" -> Transcribe -> output }
"#;

const OMNI_HANDLE_PROG: &str = r#"
origin demo_feed { kind: generation, media: audio, label: public }
pattern Ask(_tick: String) -> String {
  let clip = from demo_feed media_store_audio("clip-bytes", "public")
  return omni_ask("what is said", clip)
}
flow Main { input: String = "tick" -> Ask -> output }
"#;

const OMNI_STRING_PROG: &str = r#"
pattern Ask(_tick: String) -> String {
  return omni_ask("what is said", "clip-bytes")
}
flow Main { input: String = "tick" -> Ask -> output }
"#;

#[test]
fn handle_input_equals_string_form_on_both_backends() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var("METALOGOS_MOCK_LLM", "1"); // №478: the mock is explicit

    // stt_transcribe: the handle answer IS the string answer.
    let stt_expected = "[MOCK: stt_transcribe | whisper-large-v3-turbo | clip-bytes]";
    let tw_handle = run_tw(STT_HANDLE_PROG).expect("handle form runs on TW");
    assert_eq!(
        tw_handle.as_deref().unwrap_or_default().trim_end(),
        stt_expected,
        "the handle input must return the string-equivalent answer (TW)"
    );
    let tw_string = run_tw(STT_STRING_PROG).expect("string form runs on TW");
    assert_eq!(
        tw_string.as_deref().unwrap_or_default().trim_end(),
        stt_expected,
        "the STRING golden must not move (TW)"
    );
    let vm_handle = run_vm(STT_HANDLE_PROG).expect("handle form runs on VM");
    assert_eq!(
        vm_handle.as_deref().unwrap_or_default().trim_end(),
        stt_expected,
        "TW/VM parity of the bridge (VM)"
    );
    let vm_string = run_vm(STT_STRING_PROG).expect("string form runs on VM");
    assert_eq!(vm_string, tw_string, "VM string golden unchanged");

    // omni_ask: the same bridge contract on the omni twin (the optional
    // media argument).
    let omni_expected =
        "[MOCK: omni_ask | nemotron-3-nano-omni-30b-a3b | what is said | clip-bytes]";
    let tw_omni = run_tw(OMNI_HANDLE_PROG).expect("omni handle form runs on TW");
    assert_eq!(
        tw_omni.as_deref().unwrap_or_default().trim_end(),
        omni_expected
    );
    let tw_omni_string = run_tw(OMNI_STRING_PROG).expect("omni string form runs");
    assert_eq!(
        tw_omni_string.as_deref().unwrap_or_default().trim_end(),
        omni_expected
    );
    let vm_omni = run_vm(OMNI_HANDLE_PROG).expect("omni handle form runs on VM");
    assert_eq!(
        vm_omni.as_deref().unwrap_or_default().trim_end(),
        omni_expected
    );
    std::env::remove_var("METALOGOS_MOCK_LLM");
}

// ── (3) Foreign MediaKind refuses loudly (fail-closed) ────────────────

const WRONG_KIND_STT_PROG: &str = r#"
origin demo_gen { kind: generation, media: image, label: public }
pattern Transcribe(_tick: String) -> String {
  let frame = from demo_gen media_store_image("frame-bytes", "public")
  return stt_transcribe(frame)
}
flow Main { input: String = "tick" -> Transcribe -> output }
"#;

const WRONG_KIND_OMNI_PROG: &str = r#"
origin demo_gen { kind: generation, media: image, label: public }
pattern Ask(_tick: String) -> String {
  let frame = from demo_gen media_store_image("frame-bytes", "public")
  return omni_ask("what is this", frame)
}
flow Main { input: String = "tick" -> Ask -> output }
"#;

#[test]
fn wrong_media_kind_refuses_loudly_on_both_backends() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var("METALOGOS_MOCK_LLM", "1"); // the refusal is kind-based, not mode-based

    for (src, fn_name) in [
        (WRONG_KIND_STT_PROG, "stt_transcribe"),
        (WRONG_KIND_OMNI_PROG, "omni_ask"),
    ] {
        let err = run_tw(src).expect_err("an Image handle must not enter an audio backend (TW)");
        assert!(
            err.contains(fn_name) && err.contains("Audio handle") && err.contains("Image"),
            "{fn_name}: the foreign-kind refusal must name the expected and the actual kind, got: {}",
            err
        );
        let err_vm = run_vm(src).expect_err("an Image handle must not enter an audio backend (VM)");
        assert!(
            err_vm.contains("Audio handle") && err_vm.contains("Image"),
            "VM parity of the foreign-kind refusal, got: {}",
            err_vm
        );
    }
    std::env::remove_var("METALOGOS_MOCK_LLM");
}

/// Defense-in-depth (unit level): an Audio-TYPED handle pointing at a
/// non-Audio entry (only constructible through the direct store API —
/// no sanctioned language path forges it) cannot launder through the
/// sanctioned read path — on BOTH bridge dispatches.
#[test]
fn forged_kind_handle_cannot_launder_through_store() {
    use metalogos::interpreter::values::Value;
    use metalogos::labels::{Conf, Integrity, Label};
    use metalogos::media::{AudioId, MediaKind, MediaStore};

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
    let forged = metalogos::media::MediaHandle::Audio(AudioId(image.id()));
    let err = metalogos::builtins::stt_transcribe_dispatch(&store, &[Value::Media(forged)])
        .expect_err("the entry-kind check must refuse the forgery (stt)");
    assert!(
        err.contains("not audio") && err.contains("fail-closed"),
        "expected the loud entry-kind refusal, got: {}",
        err
    );
    let err = metalogos::builtins::omni_ask_dispatch(
        &store,
        &[Value::String("q".into()), Value::Media(forged)],
    )
    .expect_err("the entry-kind check must refuse the forgery (omni)");
    assert!(
        err.contains("not audio") && err.contains("fail-closed"),
        "expected the loud entry-kind refusal, got: {}",
        err
    );
}

// ── (4) The label join is visible statically (№323, no new rules) ─────

const PRIVATE_BRIDGE_LEAK_PROG: &str = r#"
origin feed { kind: generation, media: audio, label: public }
pattern Transcribe(_tick: String) -> String {
  let tok = env("MLOG_938_TEST_TOKEN")
  let clip = from feed media_store_audio(tok, "public")
  let text = stt_transcribe(clip)
  let _p = write_file(text, "target/n938-leak.txt")
  return "x"
}
flow Main { input: String = "x" -> Transcribe -> output }
"#;

const PRIVATE_OMNI_LEAK_PROG: &str = r#"
origin feed { kind: generation, media: audio, label: public }
pattern Ask(_tick: String) -> String {
  let tok = env("MLOG_938_TEST_TOKEN")
  let clip = from feed media_store_audio(tok, "public")
  let ans = omni_ask("summarize", clip)
  let _p = write_file(ans, "target/n938-omni-leak.txt")
  return "x"
}
flow Main { input: String = "x" -> Ask -> output }
"#;

const PUBLIC_BRIDGE_CONTROL_PROG: &str = r#"
origin feed { kind: generation, media: audio, label: public }
pattern Transcribe(_tick: String) -> String {
  let clip = from feed media_store_audio("public-bytes", "public")
  let text = stt_transcribe(clip)
  let _p = write_file(text, "target/n938-public.txt")
  return "x"
}
flow Main { input: String = "x" -> Transcribe -> output }
"#;

#[test]
fn private_label_joins_the_bridge_result_statically() {
    // The mutation: removing the bridge's label plumbing is IMPOSSIBLE —
    // the join is the №938 label_source rule (the №599 form); this test
    // pins that the bridge did not ADD a sanitizing hole (a Source role
    // never lifts conf).
    for src in [PRIVATE_BRIDGE_LEAK_PROG, PRIVATE_OMNI_LEAK_PROG] {
        let err = metalogos::compile_program(src.trim())
            .expect_err("private audio → bridge → file sink must not compile");
        assert!(
            err.contains("SECRET_LEAK") && err.contains("write_file"),
            "expected SECRET_LEAK on the post-bridge egress, got: {}",
            err
        );
    }
    // Positive control: the public flow compiles clean.
    metalogos::compile_program(PUBLIC_BRIDGE_CONTROL_PROG.trim())
        .unwrap_or_else(|e| panic!("public flow must compile, got: {}", e));
}

// ── (5) The consent scope rides the carrier (№335 + the bridge) ───────

const CONSENT_RIDE_PROG: &str = r#"
profile capability { allow_carrier: "tts_send" }
origin feed { kind: generation, media: audio, label: public }
pattern Transcribe(_tick: String) -> String {
  let tok = env("MLOG_938_TEST_TOKEN")
  let clip = from feed media_store_audio(tok, "public")
  let granted = consent_grant(clip, "audio-release")
  let text = stt_transcribe(granted)
  let voice = consent_grant("canary-voice", "audio-release")
  let bot = consent_grant("bot-token", "audio-release")
  let chat = consent_grant("chat-id", "audio-release")
  let _s = tts_send(text, voice, bot, chat)
  return "x"
}
flow Main { input: String = "x" -> Transcribe -> output }
"#;

const NO_CONSENT_PROG: &str = r#"
profile capability { allow_carrier: "tts_send" }
origin feed { kind: generation, media: audio, label: public }
pattern Transcribe(_tick: String) -> String {
  let tok = env("MLOG_938_TEST_TOKEN")
  let clip = from feed media_store_audio(tok, "public")
  let text = stt_transcribe(clip)
  let voice = consent_grant("canary-voice", "audio-release")
  let bot = consent_grant("bot-token", "audio-release")
  let chat = consent_grant("chat-id", "audio-release")
  let _s = tts_send(text, voice, bot, chat)
  return "x"
}
flow Main { input: String = "x" -> Transcribe -> output }
"#;

#[test]
fn consent_scope_rides_the_bridge_statically() {
    // With the grant the voice sink is cleared: the scope flows through
    // the bridge into the transcript's label (the №323 consent join on
    // the SOURCE side — the bridge result inherits the handle's
    // consent, and the voice gate reads it). The grant-wrapped literals
    // are the voice gate's own every-argument posture (№335 Phase-2),
    // not part of the bridge surface.
    metalogos::compile_program(CONSENT_RIDE_PROG.trim())
        .unwrap_or_else(|e| panic!("granted scope must clear the voice sink, got: {}", e));
    // Without the grant on the HANDLE the same program refuses — the
    // only difference is the bridge input's consent scope.
    let err = metalogos::compile_program(NO_CONSENT_PROG.trim())
        .expect_err("no consent → the voice sink must refuse");
    assert!(
        err.contains("voice-unconsented"),
        "expected voice-unconsented, got: {}",
        err
    );
}

// ── (6) PROCESSING: a sealed entry materializes without credentials ───

const SEALED_BRIDGE_PROG: &str = r#"
origin demo_feed { kind: generation, media: audio, label: public }
pattern Transcribe(_tick: String) -> String {
  let clip = from demo_feed media_store_audio("gdpr-clip", "consented")
  let text = stt_transcribe(clip)
  let ans = omni_ask("what is said", clip)
  if text == "[MOCK: stt_transcribe | whisper-large-v3-turbo | gdpr-clip]" {
    if ans == "[MOCK: omni_ask | nemotron-3-nano-omni-30b-a3b | what is said | gdpr-clip]" {
      return "processing-ok"
    }
  }
  return "processing-broken"
}
flow Main { input: String = "tick" -> Transcribe -> output }
"#;

#[test]
fn sealed_entry_materializes_for_processing_on_both_backends() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var("METALOGOS_MOCK_LLM", "1");
    // The consented entry is AES-256-GCM sealed at rest; the backend
    // call is PROCESSING (the Voice precedent) — no likeness/consent
    // credential is requested, the decryption happens only inside the
    // sanctioned consumer, and the mock answer equals the string form's.
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

// ── (7) The lifecycle: create → use → drop (media_release) ────────────

const LIFECYCLE_PROG: &str = r#"
origin demo_feed { kind: generation, media: audio, label: public }
pattern Transcribe(_tick: String) -> String {
  let clip = from demo_feed media_store_audio("lifecycle-bytes", "public")
  let _text = stt_transcribe(clip)
  let refs = media_release(clip)
  if refs == 0.0 {
    let _again = stt_transcribe(clip)
  }
  return "unexpected-survivor"
}
flow Main { input: String = "tick" -> Transcribe -> output }
"#;

#[test]
fn media_release_after_use_evicts_and_post_eviction_use_is_loud() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var("METALOGOS_MOCK_LLM", "1");
    // The use works; the release drops the refcount to 0 and evicts;
    // the SECOND use hits the store's loud "unknown handle" — the
    // create → use → drop lifecycle (ADR-0182 §3.1) behaves exactly as
    // the store contract promises.
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

// ── (8) The statement-position bridge (the ExprStmt path, both) ───────

const STATEMENT_PATH_PROG: &str = r#"
origin demo_feed { kind: generation, media: audio, label: public }
pattern Transcribe(_tick: String) -> String {
  let clip = from demo_feed media_store_audio("stmt-bytes", "public")
  stt_transcribe(clip)
  let _r = media_retain(clip)
  return "statement-ok"
}
flow Main { input: String = "tick" -> Transcribe -> output }
"#;

#[test]
fn statement_position_bridge_runs_on_both_backends() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var("METALOGOS_MOCK_LLM", "1");
    // The bare expression-statement call exercises the statement-path
    // interception (the let-bound calls above exercise the expression
    // path); media_retain after it proves the entry SURVIVED the call
    // (PROCESSING does not consume the caller's refcount).
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

// ── (9) The optional media handle composes with the explicit model ────

const OMNI_MODEL_PROG: &str = r#"
// The explicit wall-oss selection is a RESTRICTIVE-license record — the
// №839 named-class permit unlocks it (the licensing profile is the
// gate's own unlock form; the bridge composes with it unchanged).
profile licensing { allow_license_class: "non-osi" allow_license_class: "restrictive" }
origin demo_feed { kind: generation, media: audio, label: public }
pattern Ask(_tick: String) -> String {
  let clip = from demo_feed media_store_audio("model-bytes", "public")
  return omni_ask("what is said", clip, "wall-oss-0.5")
}
flow Main { input: String = "tick" -> Ask -> output }
"#;

#[test]
fn omni_media_handle_composes_with_explicit_model() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var("METALOGOS_MOCK_LLM", "1");
    // The 3-argument form: the media handle in slot 2, the explicit
    // model in slot 3 — the selection lands on the second Omni record
    // (wall-oss, №938 descriptor), the payload still materializes.
    let out = run_tw(OMNI_MODEL_PROG).expect("3-arg omni form runs on TW");
    assert_eq!(
        out.as_deref().unwrap_or_default().trim_end(),
        "[MOCK: omni_ask | wall-oss-0.5 | what is said | model-bytes]"
    );
    let out_vm = run_vm(OMNI_MODEL_PROG).expect("3-arg omni form runs on VM");
    assert_eq!(
        out_vm.as_deref().unwrap_or_default().trim_end(),
        "[MOCK: omni_ask | wall-oss-0.5 | what is said | model-bytes]"
    );
    std::env::remove_var("METALOGOS_MOCK_LLM");
}
