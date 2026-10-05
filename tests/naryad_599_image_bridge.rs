// Naryad #475 (issue #723): the fs_gate ratchet (clippy disallowed-methods)
// targets PRODUCTION I/O paths. This test file exercises the REAL filesystem
// for fixtures and assertions by design — the allow is scoped to this file.
#![allow(clippy::disallowed_methods)]

// ── Наряд №599 (P1, core/media; issue #1029; ADR-0182 §3.3 step 1) ───
//
// The Image media-input bridge: `vision_understand` and `ocr_extract`
// accept `Value::Media(MediaHandle::Image(_))` — materialized through
// the store's sanctioned read path; the capability descriptors land in
// the backend registry; the label-join rides the existing №323
// inference; TW/VM parity holds (one shared dispatch per builtin).
//
// Red/green corpus:
//   (1) the capability table — VisionUnderstanding/Ocr records carry
//       exactly [Image]; every other record stays text-only (drift =
//       failure; the №600+ steps evolve this pin);
//   (2) the handle input returns the SAME answer as the String
//       equivalent on BOTH backends (the golden string surface is
//       untouched — the overloads ADD capability, №493/§7.2);
//   (3) a foreign MediaKind refuses LOUDLY (fail-closed) on both
//       backends — and a forged Image-typed handle naming a non-Image
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
//       "unknown handle".
//
// Mutation verification (the №382 protocol): dropping the Image arm
// from `resolve_image_input` turns (2) red (String type error); pulling
// the foreign-kind refusal turns (3) red; removing the VISION_UNDERSTAND
// interception from either backend turns (2)/(6) red on that backend
// (the stateless fallback refuses Media) — every red below names its
// mutation.

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

// ── (1) The capability table (ADR-0182 §3.2 — the schema pin) ────────

#[test]
fn capability_descriptors_pin_the_image_inputs() {
    use metalogos::backends::{BackendClass, BACKEND_REGISTRY};
    use metalogos::media::MediaKind;

    let image_inputs = [MediaKind::Image];
    for entry in BACKEND_REGISTRY {
        match entry.class {
            BackendClass::VisionUnderstanding | BackendClass::Ocr => {
                assert_eq!(
                    entry.inputs,
                    image_inputs,
                    "record '{}' ({}) must declare exactly [Image] (№599 step 1)",
                    entry.name,
                    entry.class.as_str()
                );
            }
            // The №600+ steps (Audio, VideoFrame) evolve this pin when
            // they land; today every other record is text-only.
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
    // The three canon records of step 1 are named (the SSOT witnesses).
    for weights_id in ["molmoact2", "trocr-base-printed", "z-image-turbo"] {
        let entry = metalogos::backends::find_by_weights_id(weights_id)
            .unwrap_or_else(|| panic!("record {weights_id} exists"));
        assert_eq!(entry.inputs, image_inputs, "{weights_id} consumes Image");
    }
}

// ── (2) Handle input == String equivalent (the golden equality) ──────

const HANDLE_PROG: &str = r#"
origin demo_gen { kind: generation, media: image, label: public }
pattern Describe(_tick: String) -> String {
  let frame = from demo_gen media_store_image("frame-bytes", "public")
  return vision_understand(frame, "what is this")
}
flow Main { input: String = "tick" -> Describe -> output }
"#;

const STRING_PROG: &str = r#"
pattern Describe(_tick: String) -> String {
  return vision_understand("frame-bytes", "what is this")
}
flow Main { input: String = "tick" -> Describe -> output }
"#;

const OCR_HANDLE_PROG: &str = r#"
origin demo_gen { kind: generation, media: image, label: public }
pattern Describe(_tick: String) -> String {
  let frame = from demo_gen media_store_image("page-001.png", "public")
  return ocr_extract(frame, "eng")
}
flow Main { input: String = "tick" -> Describe -> output }
"#;

const OCR_STRING_PROG: &str = r#"
pattern Describe(_tick: String) -> String {
  return ocr_extract("page-001.png", "eng")
}
flow Main { input: String = "tick" -> Describe -> output }
"#;

#[test]
fn handle_input_equals_string_form_on_both_backends() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var("METALOGOS_MOCK_LLM", "1"); // №478: the mock is explicit

    // vision_understand: the handle answer IS the string answer.
    let expected = "[MOCK: vision_understand | molmoact2 | frame-bytes | what is this]";
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

    // ocr_extract: the same bridge contract on the OCR twin.
    let ocr_expected = "[MOCK: ocr_extract | trocr-base-printed | page-001.png | eng]";
    let tw_ocr = run_tw(OCR_HANDLE_PROG).expect("ocr handle form runs on TW");
    assert_eq!(
        tw_ocr.as_deref().unwrap_or_default().trim_end(),
        ocr_expected
    );
    let tw_ocr_string = run_tw(OCR_STRING_PROG).expect("ocr string form runs");
    assert_eq!(
        tw_ocr_string.as_deref().unwrap_or_default().trim_end(),
        ocr_expected
    );
    let vm_ocr = run_vm(OCR_HANDLE_PROG).expect("ocr handle form runs on VM");
    assert_eq!(
        vm_ocr.as_deref().unwrap_or_default().trim_end(),
        ocr_expected
    );
    std::env::remove_var("METALOGOS_MOCK_LLM");
}

// ── (3) Foreign MediaKind refuses loudly (fail-closed) ───────────────

const WRONG_KIND_PROG: &str = r#"
origin audio_feed { kind: generation, media: audio, label: public }
pattern Describe(_tick: String) -> String {
  let clip = from audio_feed media_store_audio("beep", "public")
  return vision_understand(clip, "what is this")
}
flow Main { input: String = "tick" -> Describe -> output }
"#;

const WRONG_KIND_OCR_PROG: &str = r#"
origin audio_feed { kind: generation, media: audio, label: public }
pattern Describe(_tick: String) -> String {
  let clip = from audio_feed media_store_audio("beep", "public")
  return ocr_extract(clip)
}
flow Main { input: String = "tick" -> Describe -> output }
"#;

#[test]
fn wrong_media_kind_refuses_loudly_on_both_backends() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var("METALOGOS_MOCK_LLM", "1"); // the refusal is kind-based, not mode-based

    for (src, fn_name) in [
        (WRONG_KIND_PROG, "vision_understand"),
        (WRONG_KIND_OCR_PROG, "ocr_extract"),
    ] {
        let err = run_tw(src).expect_err("an Audio handle must not enter an image backend (TW)");
        assert!(
            err.contains(fn_name) && err.contains("Image handle") && err.contains("Audio"),
            "{fn_name}: the foreign-kind refusal must name the expected and the actual kind, got: {}",
            err
        );
        let err_vm = run_vm(src).expect_err("an Audio handle must not enter an image backend (VM)");
        assert!(
            err_vm.contains("Image handle") && err_vm.contains("Audio"),
            "VM parity of the foreign-kind refusal, got: {}",
            err_vm
        );
    }
    std::env::remove_var("METALOGOS_MOCK_LLM");
}

/// Defense-in-depth (unit level): an Image-TYPED handle pointing at a
/// non-Image entry (only constructible through the direct store API —
/// no sanctioned language path forges it) cannot launder through the
/// sanctioned read path.
#[test]
fn forged_kind_handle_cannot_launder_through_store() {
    use metalogos::interpreter::values::Value;
    use metalogos::labels::{Conf, Integrity, Label};
    use metalogos::media::{ImageId, MediaKind, MediaStore};

    let mut store = MediaStore::new();
    let audio = store
        .insert(
            MediaKind::Audio,
            b"beep".to_vec(),
            Label {
                conf: Conf::Public,
                integrity: Integrity::Trusted,
                consent: Default::default(),
            },
        )
        .expect("audio insert");
    let forged = metalogos::media::MediaHandle::Image(ImageId(audio.id()));
    let err = metalogos::builtins::vision_understand_dispatch(
        &store,
        &[Value::Media(forged), Value::String("q".into())],
    )
    .expect_err("the entry-kind check must refuse the forgery");
    assert!(
        err.contains("not image") && err.contains("fail-closed"),
        "expected the loud entry-kind refusal, got: {}",
        err
    );
}

// ── (4) The label join is visible statically (№323, no new rules) ────

const PRIVATE_BRIDGE_LEAK_PROG: &str = r#"
origin feed { kind: generation, media: image, label: public }
pattern Describe(_tick: String) -> String {
  let tok = env("MLOG_599_TEST_TOKEN")
  let img = from feed media_store_image(tok, "public")
  let desc = vision_understand(img, "describe")
  let _p = write_file(desc, "target/n599-leak.txt")
  return "x"
}
flow Main { input: String = "x" -> Describe -> output }
"#;

const PRIVATE_OCR_LEAK_PROG: &str = r#"
origin feed { kind: generation, media: image, label: public }
pattern Describe(_tick: String) -> String {
  let tok = env("MLOG_599_TEST_TOKEN")
  let img = from feed media_store_image(tok, "public")
  let text = ocr_extract(img)
  let _p = write_file(text, "target/n599-ocr-leak.txt")
  return "x"
}
flow Main { input: String = "x" -> Describe -> output }
"#;

const PUBLIC_BRIDGE_CONTROL_PROG: &str = r#"
origin feed { kind: generation, media: image, label: public }
pattern Describe(_tick: String) -> String {
  let img = from feed media_store_image("public-bytes", "public")
  let desc = vision_understand(img, "describe")
  let _p = write_file(desc, "target/n599-public.txt")
  return "x"
}
flow Main { input: String = "x" -> Describe -> output }
"#;

#[test]
fn private_label_joins_the_bridge_result_statically() {
    // The mutation: removing the bridge's label plumbing is IMPOSSIBLE —
    // the join is the generic №323 rule; this test pins that the bridge
    // did not ADD a sanitizing hole (a Source role never lifts conf).
    for src in [PRIVATE_BRIDGE_LEAK_PROG, PRIVATE_OCR_LEAK_PROG] {
        let err = metalogos::compile_program(src.trim())
            .expect_err("private image → bridge → file sink must not compile");
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

// ── (5) The consent scope rides the carrier (№335 + the bridge) ──────

const CONSENT_RIDE_PROG: &str = r#"
origin feed { kind: generation, media: image, label: public }
pattern Describe(_tick: String) -> String {
  let tok = env("MLOG_599_TEST_TOKEN")
  let img = from feed media_store_image(tok, "public")
  let granted = consent_grant(img, "photo-release")
  let desc = vision_understand(granted, "describe")
  let voice = consent_grant("canary-voice", "photo-release")
  let bot = consent_grant("bot-token", "photo-release")
  let chat = consent_grant("chat-id", "photo-release")
  let _s = tts_send(desc, voice, bot, chat)
  return "x"
}
flow Main { input: String = "x" -> Describe -> output }
"#;

const NO_CONSENT_PROG: &str = r#"
origin feed { kind: generation, media: image, label: public }
pattern Describe(_tick: String) -> String {
  let tok = env("MLOG_599_TEST_TOKEN")
  let img = from feed media_store_image(tok, "public")
  let desc = vision_understand(img, "describe")
  let voice = consent_grant("canary-voice", "photo-release")
  let bot = consent_grant("bot-token", "photo-release")
  let chat = consent_grant("chat-id", "photo-release")
  let _s = tts_send(desc, voice, bot, chat)
  return "x"
}
flow Main { input: String = "x" -> Describe -> output }
"#;

#[test]
fn consent_scope_rides_the_bridge_statically() {
    // With the grant the voice sink is cleared: the scope flows through
    // the bridge into the description's label (the №323 consent join on
    // the SOURCE side — the bridge result inherits the handle's consent,
    // and the voice gate reads it). The grant-wrapped literals are the
    // voice gate's own every-argument posture (№335 Phase-2), not part
    // of the bridge surface.
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

// ── (6) PROCESSING: a sealed entry materializes without credentials ──

const SEALED_BRIDGE_PROG: &str = r#"
origin demo_gen { kind: generation, media: image, label: public }
pattern Describe(_tick: String) -> String {
  let frame = from demo_gen media_store_image("gdpr-frame", "consented")
  let seen = vision_understand(frame, "what is this")
  let text = ocr_extract(frame)
  if seen == "[MOCK: vision_understand | molmoact2 | gdpr-frame | what is this]" {
    if text == "[MOCK: ocr_extract | trocr-base-printed | gdpr-frame | ]" {
      return "processing-ok"
    }
  }
  return "processing-broken"
}
flow Main { input: String = "tick" -> Describe -> output }
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

// ── (7) The lifecycle: create → use → drop (media_release) ───────────

const LIFECYCLE_PROG: &str = r#"
origin demo_gen { kind: generation, media: image, label: public }
pattern Describe(_tick: String) -> String {
  let frame = from demo_gen media_store_image("lifecycle-bytes", "public")
  let _seen = vision_understand(frame, "describe")
  let refs = media_release(frame)
  if refs == 0.0 {
    let _again = vision_understand(frame, "describe")
  }
  return "unexpected-survivor"
}
flow Main { input: String = "tick" -> Describe -> output }
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

// ── (8) The statement-position bridge (the ExprStmt path, both) ──────

const STATEMENT_PATH_PROG: &str = r#"
origin demo_gen { kind: generation, media: image, label: public }
pattern Describe(_tick: String) -> String {
  let frame = from demo_gen media_store_image("stmt-bytes", "public")
  vision_understand(frame, "what is this")
  let _r = media_retain(frame)
  return "statement-ok"
}
flow Main { input: String = "tick" -> Describe -> output }
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
