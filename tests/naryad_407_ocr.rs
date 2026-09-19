// ── Naryad №407 (P1, perception, wave 4.5): the OCR class —
//    `ocr_extract` + `BackendClass::Ocr` + the registry canon
//    `trocr-base-printed` ──────────────────────────────────────────────
//
// Red/green corpus (the №334 donor protocol, mirrored):
//   (1) the class word "ocr" round-trips through `BackendClass::parse` /
//       `as_str` (loud on both ends — the backend_select ladder's
//       semantic companion accepts it, unknown words still refuse) and
//       the registry entry holds (class Ocr, OSI license, real pin);
//   (2) the weights-plan DRY RUN validates: manifest mandatory, the
//       registry pin matches a manifest file, the URL is HF-shaped —
//       no network, no writes (the №334 DoD path);
//   (3) the golden mock contract: `ocr_extract` is deterministic in
//       mock mode (default) on BOTH backends —
//       `[MOCK: ocr_extract | trocr-base-printed | <image> | <lang>]`;
//   (4) the class-mismatch and unknown-model refusals are loud;
//   (5) real mode refuses LOUDLY naming the missing artifact
//       (PARKED №294 — no inference is promised);
//   (6) the ladder end: `backend_select("ocr", …)` selects the canon
//       rung in mock mode with the mode visible; an unknown class word
//       stays a COMPILE error whose available-list now includes ocr;
//   (7) limitations.md carries the OCR PARKED line (loud, not silent).
//
// Mutation verification (the №382 protocol, shown in the naryad
// report): pulling the "ocr" arm out of `BackendClass::parse` turns
// (1), (6) red; removing the class-check in vision::ocr turns (4) red.

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

fn compile_err(source: &str) -> String {
    match metalogos::compile_program(source.trim()) {
        Ok(_) => panic!("expected a COMPILE error, got success for:\n{}", source),
        Err(e) => e,
    }
}

// ── (1) The class word + the registry entry ──────────────────────────

#[test]
fn ocr_class_word_round_trips_and_registry_entry_holds() {
    let cls = metalogos::backends::BackendClass::parse("ocr")
        .expect("'ocr' is a valid class word (№407)");
    assert_eq!(cls.as_str(), "ocr");
    // No cross-class drift: the pre-existing words still parse, unknown
    // words still refuse.
    for word in ["stt", "tts", "omni", "vision-understanding", "llm"] {
        assert!(
            metalogos::backends::BackendClass::parse(word).is_some(),
            "{} must keep parsing",
            word
        );
    }
    assert!(metalogos::backends::BackendClass::parse("bogus").is_none());

    // The registry entry: class, name, OSI license (TrOCR — MIT).
    let entry = metalogos::backends::find_by_weights_id("trocr-base-printed")
        .expect("the №407 canon has a registry record");
    assert_eq!(entry.class, metalogos::backends::BackendClass::Ocr);
    assert_eq!(entry.name, "trocr-printed");
    assert_eq!(entry.license, metalogos::backends::LicenseClass::Osi);
}

// ── (2) The weights-plan DRY RUN (no network, no writes) ─────────────

#[test]
fn weights_plan_dry_run_is_pinned_and_hf_shaped() {
    let plan = metalogos::backends_weights::weights_plan("trocr-base-printed")
        .expect("the dry run validates (manifest mandatory, pin matches)");
    assert_eq!(plan.len(), 1, "single-artifact manifest (the whisper pattern)");
    assert_eq!(plan[0].path, "model.safetensors");
    assert_eq!(
        plan[0].sha256,
        "1cf4a6eedab26afaaf505f1c7f73d9634944924dbd1ed049d569db98039cd596"
    );
    assert_eq!(plan[0].bytes, 1_333_384_464);
    assert!(
        plan[0]
            .url
            .starts_with("https://huggingface.co/microsoft/trocr-base-printed/resolve/main/"),
        "the URL is HF-shaped (№334 loader contract): {}",
        plan[0].url
    );
}

// ── (3) The golden mock contract, deterministic on BOTH backends ─────

const OCR_PROG: &str = r#"
pattern P(_x: String) -> String {
  return ocr_extract("page-001.png")
}
flow Main { input: String = "x" -> P -> output }
"#;

const OCR_PROG_LANG: &str = r#"
pattern P(_x: String) -> String {
  return ocr_extract("page-001.png", "eng")
}
flow Main { input: String = "x" -> P -> output }
"#;

#[test]
fn mock_contract_is_deterministic_on_tw_and_vm() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::remove_var("METALOGOS_LLM_MOCK");

    // The mock format is FIXED 4-field (the №407 contract: weights |
    // image | lang) — the omitted lang is an EMPTY field, not a shorter
    // line (mirrors vision_understand's empty-prompt shape).
    let out_tw = run_tw(OCR_PROG).expect("ocr mock runs on TW");
    assert_eq!(
        out_tw.as_deref().unwrap_or_default().trim_end(),
        "[MOCK: ocr_extract | trocr-base-printed | page-001.png | ]"
    );
    let out_lang = run_tw(OCR_PROG_LANG).expect("ocr mock with lang runs");
    assert_eq!(
        out_lang.as_deref().unwrap_or_default().trim_end(),
        "[MOCK: ocr_extract | trocr-base-printed | page-001.png | eng]"
    );
    // Determinism: a second run yields the same string (no randomness,
    // no time — the golden contract).
    let out_tw_again = run_tw(OCR_PROG).expect("second run");
    assert_eq!(out_tw, out_tw_again);

    // VM parity — the same strings on the compiled path.
    let out_vm = run_vm(OCR_PROG).expect("ocr mock runs on VM");
    assert_eq!(
        out_vm.as_deref().unwrap_or_default().trim_end(),
        "[MOCK: ocr_extract | trocr-base-printed | page-001.png | ]"
    );
    let out_vm_lang = run_vm(OCR_PROG_LANG).expect("ocr mock with lang on VM");
    assert_eq!(
        out_vm_lang.as_deref().unwrap_or_default().trim_end(),
        "[MOCK: ocr_extract | trocr-base-printed | page-001.png | eng]"
    );
    std::env::remove_var("METALOGOS_LLM_MOCK");
}

// ── (4) Class mismatch and unknown model are loud ────────────────────

const MISMATCH_PROG: &str = r#"
pattern P(_x: String) -> String {
  return ocr_extract("page-001.png", "eng", "molmoact2")
}
flow Main { input: String = "x" -> P -> output }
"#;

const UNKNOWN_MODEL_PROG: &str = r#"
pattern P(_x: String) -> String {
  return ocr_extract("page-001.png", "eng", "no-such-model")
}
flow Main { input: String = "x" -> P -> output }
"#;

#[test]
fn class_mismatch_and_unknown_model_are_loud() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::remove_var("METALOGOS_LLM_MOCK");

    let err = run_tw(MISMATCH_PROG).expect_err("a vision-understanding model is NOT an ocr model");
    assert!(
        err.contains("is class 'vision-understanding', not ocr"),
        "the class-check refusal must be loud: {}",
        err
    );
    let err_vm = run_vm(MISMATCH_PROG).expect_err("VM parity of the class check");
    assert!(err_vm.contains("not ocr"), "got: {}", err_vm);

    let err_unknown = run_tw(UNKNOWN_MODEL_PROG).expect_err("unknown model refuses");
    assert!(
        err_unknown.contains("no registry record"),
        "got: {}",
        err_unknown
    );
    std::env::remove_var("METALOGOS_LLM_MOCK");
}

// ── (5) Real mode refuses loudly (PARKED №294) ───────────────────────

const REAL_MODE_PROG: &str = r#"
pattern P(_x: String) -> String {
  return ocr_extract("page-001.png")
}
flow Main { input: String = "x" -> P -> output }
"#;

#[test]
fn real_mode_refuses_loudly_without_weights() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var("METALOGOS_LLM_MOCK", "false");
    let err = run_tw(REAL_MODE_PROG).expect_err("real mode without weights refuses");
    assert!(
        err.contains("PARKED by hardware") && err.contains("model.safetensors"),
        "expected the loud PARKED refusal naming the artifact, got: {}",
        err
    );
    let err_vm = run_vm(REAL_MODE_PROG).expect_err("real mode refuses on VM too");
    assert!(err_vm.contains("PARKED by hardware"), "got: {}", err_vm);
    std::env::remove_var("METALOGOS_LLM_MOCK");
}

// ── (6) The ladder end: the class word on both ends ──────────────────

const SELECT_OCR_PROG: &str = r#"
pattern Pick(_tick: String) -> String {
  let sel = backend_select("ocr", ["trocr-printed"])
  if sel.ok {
    return sel.backend + "/" + sel.mode + "/" + sel.weights_id
  }
  return "unexpected"
}
flow Main { input: String = "t" -> Pick -> output }
"#;

const BAD_CLASS_PROG: &str = r#"
pattern Pick(_tick: String) -> String {
  let sel = backend_select("bogus", ["trocr-printed"])
  return sel.backend
}
flow Main { input: String = "t" -> Pick -> output }
"#;

#[test]
fn backend_select_accepts_ocr_on_both_ends() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::remove_var("METALOGOS_LLM_MOCK");

    // Runtime end: the canon rung is selected, the mode is visible.
    let expected = "trocr-printed/mock/trocr-base-printed";
    let out = run_tw(SELECT_OCR_PROG).expect("tw ladder runs");
    assert_eq!(out.as_deref().unwrap_or_default().trim_end(), expected);
    let out_vm = run_vm(SELECT_OCR_PROG).expect("vm ladder runs");
    assert_eq!(out_vm.as_deref().unwrap_or_default().trim_end(), expected);

    // Static end: an unknown class word stays a compile error, and its
    // available-list now names ocr (the №336 companion message).
    let err = compile_err(BAD_CLASS_PROG);
    assert!(
        err.contains("unknown backend class 'bogus'") && err.contains("ocr"),
        "the companion refusal must be loud and list ocr: {}",
        err
    );
    std::env::remove_var("METALOGOS_LLM_MOCK");
}

// ── (7) limitations.md carries the OCR PARKED line ───────────────────

#[test]
fn limitations_names_the_ocr_parked_boundary() {
    let doc = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/limitations.md"))
        .expect("limitations.md exists");
    assert!(
        doc.contains("№407") && doc.contains("ocr_extract"),
        "limitations.md must name the №407 OCR real-weights PARKED boundary"
    );
}
