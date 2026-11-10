// Naryad #475 (issue #723): the fs_gate ratchet (clippy disallowed-methods)
// targets PRODUCTION I/O paths. This test file exercises the REAL filesystem
// for fixtures and assertions by design — the allow is scoped to this file.
#![allow(clippy::disallowed_methods)]

// ── Наряд №940 (P2, wave 94 — W-Z, №Z1; issue #1895; ADR-0182 §7.5) ──
//
// The production LLM backend registration — the §7.5 class-fact closure.
// The R74 fact: `BackendClass::Llm` existed with ZERO entries while the
// `llm` builtin surface was ALREADY built (call_llm/call_claude, the
// Provider enum Anthropic/OpenAI/Ollama, the env routing
// METALOGOS_LLM_PROVIDER, the retry ladder №879). The registration
// closes the fact with THREE registry rows (claude-sonnet-4-20250514 /
// gpt-4o / llama3) behind that built surface — NOT a new serving line.
//
// Red/green corpus:
//   (1) the registry table — exactly three Llm entries, each with the
//       honest pin (PendingNo334 — the API weights are never vendored,
//       never a fabricated hash), the honest license class (llama3 →
//       NonOsi per the Llama Community License; claude/gpt-4o →
//       Restrictive, the closed commercial API models — never a
//       fabricated Osi), a filled license_note and the text-only
//       capability descriptor (§3.2 «empty = text-only»);
//   (2) the fail-closed selection §3.2 — a statically-verified
//       `backend_select("llm", …)` ladder names a REGISTERED entry and
//       selects it in mock mode on BOTH backends (the class word is
//       machine-verified against the registry); in real mode the same
//       rung exhausts honestly (Degraded, never a mock substitution —
//       the PendingNo334 rung cannot be verified, №334/№294);
//   (3) the loud refusal — a runtime-built ladder with no registered
//       llm rung degrades LOUDLY (BACKEND_DEGRADED, the ADR-0165
//       ladder — not a silent fallback, the №263 contract); a literal
//       rung naming another class's entry is a COMPILE error (the
//       static companion, ADR-0165 §2.4);
//   (4) the №493 surface discipline — the `llm` builtin specs, the
//       provider defaults, the endpoints and the retry ladder are
//       UNCHANGED (drift = failure): the registration adds accounting,
//       not behavior;
//   (5) the bounds honesty (№515) — the registry is a static const
//       table: the naryad adds ZERO new runtime state (no Lazy<Mutex>
//       map, no bounds row in the №529 inventory — the honest record).
//
// Mutation verification (the №382 protocol): renaming an llm entry or
// flipping a license class turns (1) red; removing a registry row turns
// (2) red (the static companion refuses the unknown rung); fabricating
// an Osi class turns (1) red; touching the builtin specs turns (4) red.

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

// ── (1) The registry table (the §7.5 class-fact closure) ─────────────

#[test]
fn three_llm_entries_close_the_75_class_fact() {
    use metalogos::backends::{BackendClass, LicenseClass, ShaPin, BACKEND_REGISTRY};

    let llm: Vec<_> = BACKEND_REGISTRY
        .iter()
        .filter(|e| e.class == BackendClass::Llm)
        .collect();
    assert!(
        llm.len() >= 3,
        "the §7.5 class fact is closed by ≥ 3 Llm entries, got {}",
        llm.len()
    );

    let by_name = |name: &str| {
        llm.iter()
            .copied()
            .find(|e| e.name == name)
            .unwrap_or_else(|| panic!("the Llm entry '{name}' exists (the §7.5 roster)"))
    };
    let claude = by_name("claude-sonnet-4-20250514");
    let gpt4o = by_name("gpt-4o");
    let llama = by_name("llama3");

    // The Provider-enum defaults (src/llm.rs) are the registered ids —
    // the §7.5 closure rides the EXISTING surface, no new model names.
    for entry in [claude, gpt4o, llama] {
        assert_eq!(
            entry.name, entry.weights_id,
            "{}: the provider-default ids register under their own name",
            entry.name
        );
        // Honest pins: the API weights are never vendored in-tree —
        // never a fabricated hash (the №334 contract).
        assert!(
            matches!(entry.pin, ShaPin::PendingNo334),
            "{}: the pin must be PendingNo334 (API weights are not vendored), got {:?}",
            entry.name,
            entry.pin
        );
        // Text-only (§3.2 «empty = text-only») — the class consumes no
        // media input (the ADR-0182 §3.3 bridge filled the other classes).
        assert!(
            entry.inputs.is_empty(),
            "{}: an Llm record stays text-only, got {:?}",
            entry.name,
            entry.inputs
        );
        // The license basis is NAMED (the №333 schema contract).
        assert!(
            !entry.license_note.is_empty() && entry.license_note.len() > 24,
            "{}: the license_note must name the license and its basis",
            entry.name
        );
    }

    // The honest license classes per the ACTUAL declaration — never a
    // fabricated Osi (ADR-0163 §2.1).
    assert_eq!(
        llama.license,
        LicenseClass::NonOsi,
        "llama3: the Llama Community License is NOT OSI-approved (the nemotron-omni precedent)"
    );
    assert_eq!(
        claude.license,
        LicenseClass::Restrictive,
        "claude-sonnet-4: a closed commercial API model — restrictive default-deny"
    );
    assert_eq!(
        gpt4o.license,
        LicenseClass::Restrictive,
        "gpt-4o: a closed commercial API model — restrictive default-deny"
    );
    for note in [claude.license_note, gpt4o.license_note] {
        assert!(
            note.contains("restrictive"),
            "the restrictive basis must be named in the note: {note}"
        );
    }
    assert!(
        llama.license_note.contains("non-osi"),
        "the non-osi basis must be named in the note: {}",
        llama.license_note
    );
}

#[test]
fn llm_passports_mirror_the_registration_1to1() {
    // The №838 1:1 posture holds for the new class too: every Llm
    // registry row has a passport, and the class/note ride unchanged.
    use metalogos::backends::{BackendClass, BACKEND_REGISTRY};
    use metalogos::model_passport::{all_passports, passport_for};

    assert_eq!(
        all_passports().len(),
        BACKEND_REGISTRY.len(),
        "the passport table must mirror the registry 1:1 (the Llm rows included)"
    );
    for entry in BACKEND_REGISTRY
        .iter()
        .filter(|e| e.class == BackendClass::Llm)
    {
        let p = passport_for(entry.weights_id)
            .unwrap_or_else(|| panic!("no passport for the Llm row {}", entry.weights_id));
        assert_eq!(p.license, entry.license);
        assert_eq!(p.license_note, entry.license_note);
        assert!(
            p.verify_integrity().is_ok(),
            "{}: the registered passport must pass its own integrity check",
            entry.weights_id
        );
        // The honest source shape: no downloadable weights for an
        // API-served model / the Ollama-managed rung.
        assert!(
            p.source.contains("provider API") || p.source.contains("ollama"),
            "{}: the source must name the honest non-weights shape, got: {}",
            entry.weights_id,
            p.source
        );
    }
}

// ── (2) The fail-closed selection §3.2 (registered rung selects) ─────

const SELECT_LLM_PROG: &str = r#"
// №839 (ADR-0202 §2.3): the rungs are Restrictive/NonOsi — the named
// class permits unlock the compile (the ladder mechanics are the test's
// subject, not the license posture).
profile licensing { allow_license_class: "non-osi" allow_license_class: "restrictive" }

pattern Pick(_tick: String) -> String {
  let sel = backend_select("llm", ["claude-sonnet-4-20250514", "llama3"])
  if sel.ok {
    return sel.backend + "/" + sel.mode + "/" + sel.weights_id
  }
  return "unexpected"
}
flow Main { input: String = "t" -> Pick -> output }
"#;

#[test]
fn llm_ladder_selects_the_registered_rung_mock_mode() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var("METALOGOS_MOCK_LLM", "1"); // №478: the mock is explicit (№454 SSOT)
    let expected = "claude-sonnet-4-20250514/mock/claude-sonnet-4-20250514";
    let out = run_tw(SELECT_LLM_PROG).expect("tw runs");
    assert_eq!(
        out.as_deref().unwrap_or_default().trim_end(),
        expected,
        "the registered llm rung is machine-verified against the registry (TW)"
    );
    let out_vm = run_vm(SELECT_LLM_PROG).expect("vm runs");
    assert_eq!(
        out_vm.as_deref().unwrap_or_default().trim_end(),
        expected,
        "TW/VM parity of the llm selection (VM)"
    );
    std::env::remove_var("METALOGOS_MOCK_LLM");
}

#[test]
fn llm_ladder_real_mode_exhausts_honestly_never_a_mock() {
    // Real mode: the PendingNo334 rung cannot be verified (no manifest —
    // the API weights are not vendored) → honest exhaustion, Degraded,
    // NEVER a mock substitution (№334/№294/ADR-0165 §2.3).
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::remove_var("METALOGOS_MOCK_LLM");
    let src = r#"
profile licensing { allow_license_class: "non-osi" allow_license_class: "restrictive" }

pattern R(_t: String) -> String {
  let sel = backend_select("llm", ["claude-sonnet-4-20250514"])
  if sel.ok {
    return "MOCK-LEAK:" + sel.backend
  }
  if sel.class == "llm" {
    if sel.error.code == "BACKEND_DEGRADED" {
      let first = sel.attempts[0]
      return "degraded/llm/" + to_string(len(sel.attempts)) + "/" + first.status
    }
  }
  return "degraded-shape-broken"
}
flow Main { input: String = "t" -> R -> output }
"#;
    let expected = "degraded/llm/1/unavailable";
    let out = run_tw(src).expect("the degraded program KEEPS RUNNING (no panic)");
    assert_eq!(out.as_deref().unwrap_or_default().trim_end(), expected);
    let out_vm = run_vm(src).expect("vm parity for the llm Degraded");
    assert_eq!(out_vm.as_deref().unwrap_or_default().trim_end(), expected);
    std::env::remove_var("METALOGOS_MOCK_LLM");
}

// ── (3) The loud refusal (the ADR-0165 ladder, the №263 contract) ────

const DEGRADE_LLM_PROG: &str = r#"
pattern Ghost(_x: String) -> String {
  return "no-such-llm-backend"
}
pattern Degrade(_tick: String) -> String {
  // A runtime-built ladder: statically unverifiable, so the runtime
  // checks own it — the rung has NO registry record, and the class has
  // no other registered entry to fall to. Exhaustion → Degraded(llm),
  // the loud typed result (never a panic, never a silent fallback).
  let words = [Ghost("a")]
  let sel = backend_select("llm", words)
  if sel.ok {
    return "unexpected: an empty llm ladder selected a rung"
  }
  if sel.class == "llm" {
    if sel.error.code == "BACKEND_DEGRADED" {
      let n = len(sel.attempts)
      let first = sel.attempts[0]
      return "degraded/llm/" + to_string(n) + "/" + first.backend + "/" + first.status
    }
  }
  return "degraded-shape-broken"
}
flow Main { input: String = "t" -> Degrade -> output }
"#;

#[test]
fn empty_llm_selection_refuses_loudly_on_both_backends() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::remove_var("METALOGOS_MOCK_LLM");
    let expected = "degraded/llm/1/no-such-llm-backend/unavailable";
    let out = run_tw(DEGRADE_LLM_PROG).expect("degraded program KEEPS RUNNING (no panic)");
    assert_eq!(
        out.as_deref().unwrap_or_default().trim_end(),
        expected,
        "the loud §3.2 refusal: an unsatisfiable llm selection degrades, never silently falls back"
    );
    let out_vm = run_vm(DEGRADE_LLM_PROG).expect("vm parity for the loud refusal");
    assert_eq!(out_vm.as_deref().unwrap_or_default().trim_end(), expected);
    std::env::remove_var("METALOGOS_MOCK_LLM");
}

#[test]
fn literal_rung_of_another_class_is_a_compile_error() {
    // The static companion (ADR-0165 §2.4): a literal rung is verified
    // against the registry — 'chatterbox' is tts, the ladder serves llm.
    let src = r#"
pattern Wrong(_t: String) -> String {
  let sel = backend_select("llm", ["chatterbox"])
  return sel.backend
}
flow Main { input: String = "t" -> Wrong -> output }
"#;
    let err = compile_err(src);
    assert!(
        err.contains("class mismatch") || err.contains("is class 'tts', ladder serves 'llm'"),
        "expected the static class-mismatch refusal, got: {err}"
    );
}

// ── (4) The №493 surface discipline (the builtin surface unchanged) ──

#[test]
fn the_llm_builtin_surface_is_unchanged() {
    // №493: the registration adds ACCOUNTING, not behavior — the specs,
    // the provider defaults and the endpoints ride as they were
    // (drift = failure; the golden specs are not touched).
    let registry = include_str!("../src/builtins/registry.rs");
    for spec in [
        "spec!(\"call_llm\", 1, 3, \"llm\";",
        "spec!(\"call_claude\", 4, \"llm\";",
        "spec!(\"llm_usage\", 0, \"llm\";",
        "spec!(\"call_llm_schema\", 2, 3, \"llm\";",
    ] {
        assert!(
            registry.contains(spec),
            "the llm builtin surface must not move: {spec} missing"
        );
    }
    let llm = include_str!("../src/llm.rs");
    for (needle, why) in [
        ("claude-sonnet-4-20250514", "the Anthropic default"),
        ("gpt-4o", "the OpenAI default"),
        ("llama3", "the Ollama default"),
        ("METALOGOS_LLM_PROVIDER", "the env routing"),
        (
            "https://api.anthropic.com/v1/messages",
            "the Anthropic endpoint",
        ),
        (
            "https://api.openai.com/v1/chat/completions",
            "the OpenAI endpoint",
        ),
        ("http://localhost:11434/api/generate", "the Ollama endpoint"),
    ] {
        assert!(
            llm.contains(needle),
            "the llm.rs surface must not move — {why}: {needle}"
        );
    }
}

// ── (5) The bounds honesty (№515: 0 new runtime state) ───────────────

#[test]
fn the_registration_adds_zero_new_runtime_state() {
    // №515: the bounds inventory (№529) lists the process-global
    // Lazy<Mutex<HashMap>> registries. The backend registry is a static
    // const table — the registration grows the TABLE, not the runtime
    // state; the honest record is the absence of a new inventory row
    // PLUS this source-level pin: no new static map landed in the
    // registry module (drift = failure — a new map would need a №529
    // row with two-circuit bounds BEFORE landing).
    let src = include_str!("../src/backends.rs");
    assert!(
        !src.contains("Lazy<Mutex"),
        "backends.rs must stay free of process-global maps (the №515 honest record: 0 new runtime state)"
    );
    let inv = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("scripts/ci/registry_bounds_inventory.txt"),
    )
    .expect("the №529 inventory exists");
    assert!(
        !inv.contains("BACKEND_REGISTRY"),
        "the static const table stays out of the runtime-state inventory (no drift either way)"
    );
}
